//! Use case kategori: pohon 2 level, arsip, merge (CAT-01, CAT-02).

use nabungo_core::{
    CategoryKind, MergeSide, ParentInfo, check_merge, check_parent, normalize_name,
};
use nabungo_storage::category::{self, CategoryRow, NewCategory};
use nabungo_storage::now_utc;
use nabungo_storage::rusqlite::Connection;
use uuid::Uuid;

use crate::ledger::{AppError, Ledger};

/// Isian form kategori. Jenis terpisah karena tidak bisa diubah setelah dibuat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryFields {
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

impl CategoryFields {
    /// Kategori root tanpa ikon/warna.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent_id: None,
            icon: None,
            color: None,
        }
    }
}

fn get(conn: &Connection, id: Uuid) -> Result<CategoryRow, AppError> {
    category::get_category(conn, id)?.ok_or(AppError::NotFound("category"))
}

fn parent_info(conn: &Connection, parent: Option<Uuid>) -> Result<Option<ParentInfo>, AppError> {
    parent
        .map(|p| {
            let row = get(conn, p)?;
            Ok(ParentInfo {
                kind: row.kind,
                has_parent: row.parent_id.is_some(),
            })
        })
        .transpose()
}

impl Ledger {
    pub fn create_category(
        &mut self,
        kind: CategoryKind,
        fields: &CategoryFields,
    ) -> Result<Uuid, AppError> {
        let name = normalize_name(&fields.name)?;
        let tx = self.conn.transaction()?;
        check_parent(kind, parent_info(&tx, fields.parent_id)?, false)?;
        let new = NewCategory {
            kind,
            name,
            parent_id: fields.parent_id,
            icon: fields.icon.clone(),
            color: fields.color.clone(),
            sort_order: category::next_sort_order(&tx, kind, fields.parent_id)?,
        };
        let id =
            category::insert_category(&tx, &new).map_err(AppError::duplicate_as("category"))?;
        tx.commit()?;
        Ok(id)
    }

    /// Ubah nama/parent/ikon/warna. Kategori ber-child tidak bisa dipindah ke bawah parent.
    pub fn update_category(&mut self, id: Uuid, fields: &CategoryFields) -> Result<(), AppError> {
        let name = normalize_name(&fields.name)?;
        let tx = self.conn.transaction()?;
        let row = get(&tx, id)?;
        if fields.parent_id == Some(id) {
            return Err(nabungo_core::CatalogError::CategoryDepth.into());
        }
        let has_children = category::child_count(&tx, id)? > 0;
        check_parent(row.kind, parent_info(&tx, fields.parent_id)?, has_children)?;
        category::update_category(
            &tx,
            id,
            &name,
            fields.parent_id,
            fields.icon.as_deref(),
            fields.color.as_deref(),
        )
        .map_err(AppError::duplicate_as("category"))?;
        tx.commit()?;
        Ok(())
    }

    /// Child ikut diarsip.
    pub fn archive_category(&mut self, id: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        get(&tx, id)?;
        category::set_category_archived(&tx, id, true, &now_utc())?;
        tx.commit()?;
        Ok(())
    }

    /// Child dengan parent yang masih diarsip → `Archived("category")`.
    pub fn unarchive_category(&mut self, id: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        let row = get(&tx, id)?;
        if let Some(p) = row.parent_id
            && get(&tx, p)?.archived
        {
            return Err(AppError::Archived("category"));
        }
        category::set_category_archived(&tx, id, false, &now_utc())?;
        tx.commit()?;
        Ok(())
    }

    /// Hanya kategori yang tidak dipakai dan tanpa child; selain itu `InUse` (tawarkan arsip/merge).
    pub fn delete_category(&mut self, id: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        get(&tx, id)?;
        if category::is_referenced(&tx, id)? || category::child_count(&tx, id)? > 0 {
            return Err(AppError::InUse("category"));
        }
        category::delete_category(&tx, id)?;
        tx.commit()?;
        Ok(())
    }

    /// Pindahkan semua pemakaian `source` ke `target`, lalu hapus `source`.
    pub fn merge_category(&mut self, source: Uuid, target: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        let src = get(&tx, source)?;
        let dst = get(&tx, target)?;
        let side = |row: &CategoryRow| -> Result<MergeSide, AppError> {
            Ok(MergeSide {
                id: row.id,
                kind: row.kind,
                has_children: category::child_count(&tx, row.id)? > 0,
            })
        };
        check_merge(side(&src)?, side(&dst)?)?;
        if dst.archived {
            return Err(AppError::Archived("category"));
        }
        category::merge_category(&tx, source, target)?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_categories(&self) -> Result<Vec<CategoryRow>, AppError> {
        Ok(category::list_categories(&self.conn)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountFields, AppError, Ledger, TxnInput};
    use nabungo_core::{AccountKind, CatalogError, CategoryKind, TxnKind};
    use time::macros::date;
    use uuid::Uuid;

    const E: CategoryKind = CategoryKind::Expense;
    const I: CategoryKind = CategoryKind::Income;

    fn cat(ledger: &mut Ledger, kind: CategoryKind, name: &str, parent: Option<Uuid>) -> Uuid {
        let mut f = CategoryFields::named(name);
        f.parent_id = parent;
        ledger.create_category(kind, &f).unwrap()
    }

    fn row(ledger: &Ledger, id: Uuid) -> Option<CategoryRow> {
        ledger
            .list_categories()
            .unwrap()
            .into_iter()
            .find(|c| c.id == id)
    }

    fn bank(ledger: &mut Ledger) -> Uuid {
        ledger
            .create_account(
                AccountKind::Bank,
                &AccountFields::new("BCA", 0, date!(2026 - 10 - 01)),
            )
            .unwrap()
    }

    fn expense(ledger: &mut Ledger, acc: Uuid, cat: Uuid) -> Uuid {
        ledger
            .record(&TxnInput::single(
                TxnKind::Expense,
                date!(2026 - 10 - 12),
                acc,
                Some(cat),
                -10_000,
            ))
            .unwrap()
    }

    fn budget(ledger: &Ledger, cat: Uuid, period: &str, amount: i64) {
        ledger
            .conn
            .execute(
                "INSERT INTO budget (id, category_id, period, amount) VALUES (?1, ?2, ?3, ?4)",
                (Uuid::now_v7().to_string(), cat.to_string(), period, amount),
            )
            .unwrap();
    }

    fn budgets(ledger: &Ledger, cat: Uuid) -> Vec<(String, i64)> {
        let mut stmt = ledger
            .conn
            .prepare("SELECT period, amount FROM budget WHERE category_id = ?1 ORDER BY period")
            .unwrap();
        stmt.query_map([cat.to_string()], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn create_child_rules() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let food = cat(&mut ledger, E, "Makan", None);
        let coffee = cat(&mut ledger, E, "Kopi", Some(food));
        assert_eq!(row(&ledger, coffee).unwrap().parent_id, Some(food));

        let mut f = CategoryFields::named("Latte");
        f.parent_id = Some(coffee);
        assert!(matches!(
            ledger.create_category(E, &f),
            Err(AppError::Catalog(CatalogError::CategoryDepth))
        ));
        f.parent_id = Some(food);
        assert!(matches!(
            ledger.create_category(I, &f),
            Err(AppError::Catalog(CatalogError::CategoryKindMismatch))
        ));
        f.parent_id = Some(Uuid::now_v7());
        assert!(matches!(
            ledger.create_category(E, &f),
            Err(AppError::NotFound("category"))
        ));
    }

    #[test]
    fn sibling_names_unique_case_insensitive() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let food = cat(&mut ledger, E, "Makan", None);
        assert!(matches!(
            ledger.create_category(E, &CategoryFields::named(" makan ")),
            Err(AppError::Duplicate("category"))
        ));
        cat(&mut ledger, I, "Makan", None);
        let other = cat(&mut ledger, E, "Jajan", None);
        cat(&mut ledger, E, "Kopi", Some(food));
        cat(&mut ledger, E, "Kopi", Some(other));
        let mut f = CategoryFields::named("KOPI");
        f.parent_id = Some(food);
        assert!(matches!(
            ledger.create_category(E, &f),
            Err(AppError::Duplicate("category"))
        ));
    }

    #[test]
    fn move_parent_with_children_rejected() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let food = cat(&mut ledger, E, "Makan", None);
        cat(&mut ledger, E, "Kopi", Some(food));
        let other = cat(&mut ledger, E, "Belanja", None);
        let mut f = CategoryFields::named("Makan");
        f.parent_id = Some(other);
        assert!(matches!(
            ledger.update_category(food, &f),
            Err(AppError::Catalog(CatalogError::CategoryDepth))
        ));

        let mut g = CategoryFields::named("Pakaian & Sepatu");
        g.parent_id = Some(other);
        g.icon = Some("shirt".into());
        let lone = cat(&mut ledger, E, "Pakaian", None);
        ledger.update_category(lone, &g).unwrap();
        let r = row(&ledger, lone).unwrap();
        assert_eq!(r.name, "Pakaian & Sepatu");
        assert_eq!(r.parent_id, Some(other));
        assert_eq!(r.icon.as_deref(), Some("shirt"));
    }

    #[test]
    fn archive_cascades_and_unarchive_child_needs_parent() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let food = cat(&mut ledger, E, "Makan", None);
        let coffee = cat(&mut ledger, E, "Kopi", Some(food));
        ledger.archive_category(food).unwrap();
        assert!(row(&ledger, food).unwrap().archived);
        assert!(row(&ledger, coffee).unwrap().archived);
        assert!(matches!(
            ledger.unarchive_category(coffee),
            Err(AppError::Archived("category"))
        ));
        ledger.unarchive_category(food).unwrap();
        assert!(row(&ledger, coffee).unwrap().archived);
        ledger.unarchive_category(coffee).unwrap();
        assert!(!row(&ledger, coffee).unwrap().archived);
    }

    #[test]
    fn delete_rules() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let unused = cat(&mut ledger, E, "Rokok", None);
        ledger.delete_category(unused).unwrap();
        assert!(row(&ledger, unused).is_none());

        let acc = bank(&mut ledger);
        let food = cat(&mut ledger, E, "Makan", None);
        let txn = expense(&mut ledger, acc, food);
        assert!(row(&ledger, food).unwrap().in_use);
        ledger.delete_txn(txn).unwrap();
        assert!(matches!(
            ledger.delete_category(food),
            Err(AppError::InUse("category"))
        ));

        let parent = cat(&mut ledger, E, "Belanja", None);
        cat(&mut ledger, E, "Pakaian", Some(parent));
        assert!(matches!(
            ledger.delete_category(parent),
            Err(AppError::InUse("category"))
        ));
        assert!(matches!(
            ledger.delete_category(Uuid::now_v7()),
            Err(AppError::NotFound("category"))
        ));
    }

    #[test]
    fn merge_moves_everything() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = bank(&mut ledger);
        let source = cat(&mut ledger, E, "Jajan", None);
        let target = cat(&mut ledger, E, "Makan", None);
        let live = expense(&mut ledger, acc, source);
        let gone = expense(&mut ledger, acc, source);
        ledger.delete_txn(gone).unwrap();
        ledger
            .conn
            .execute(
                "INSERT INTO payee (id, name, default_category_id) VALUES ('p1', 'Warung', ?1)",
                [source.to_string()],
            )
            .unwrap();
        budget(&ledger, source, "2026-11", 100_000);
        budget(&ledger, target, "2026-11", 50_000);
        budget(&ledger, source, "2026-12", 30_000);

        ledger.merge_category(source, target).unwrap();

        assert!(row(&ledger, source).is_none());
        let on_target: i64 = ledger
            .conn
            .query_row(
                "SELECT count(*) FROM posting WHERE category_id = ?1",
                [target.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(on_target, 2);
        ledger.undo_delete_txn(gone).unwrap();
        let payee_default: String = ledger
            .conn
            .query_row("SELECT default_category_id FROM payee", [], |r| r.get(0))
            .unwrap();
        assert_eq!(payee_default, target.to_string());
        assert_eq!(
            budgets(&ledger, target),
            [
                ("2026-11".to_owned(), 150_000),
                ("2026-12".to_owned(), 30_000)
            ]
        );
        let _ = live;
    }

    #[test]
    fn merge_rules() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let food = cat(&mut ledger, E, "Makan", None);
        let coffee = cat(&mut ledger, E, "Kopi", Some(food));
        let salary = cat(&mut ledger, I, "Gaji", None);
        let old = cat(&mut ledger, E, "Lama", None);
        assert!(matches!(
            ledger.merge_category(coffee, coffee),
            Err(AppError::Catalog(CatalogError::MergeIntoSelf))
        ));
        assert!(matches!(
            ledger.merge_category(coffee, salary),
            Err(AppError::Catalog(CatalogError::CategoryKindMismatch))
        ));
        assert!(matches!(
            ledger.merge_category(food, old),
            Err(AppError::Catalog(CatalogError::CategoryHasChildren))
        ));
        ledger.archive_category(old).unwrap();
        assert!(matches!(
            ledger.merge_category(coffee, old),
            Err(AppError::Archived("category"))
        ));
        ledger.merge_category(coffee, food).unwrap();
        assert!(row(&ledger, coffee).is_none());
    }
}
