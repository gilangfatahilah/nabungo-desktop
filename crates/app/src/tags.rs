//! Kelola tag: buat, ganti nama, gabung, hapus (CAT-04).

use nabungo_core::CatalogError;
use nabungo_storage::rusqlite::Transaction;
use nabungo_storage::tag::{self, TagUsage};
use uuid::Uuid;

use crate::ledger::{AppError, Ledger};
use crate::transactions::normalize_tag;

fn ensure(tx: &Transaction, id: Uuid) -> Result<(), AppError> {
    if tag::tag_exists(tx, id)? {
        Ok(())
    } else {
        Err(AppError::NotFound("tag"))
    }
}

impl Ledger {
    pub fn list_tag_usage(&self) -> Result<Vec<TagUsage>, AppError> {
        Ok(tag::list_tag_usage(&self.conn)?)
    }

    /// Nama dinormalisasi seperti tag di form (`#` di depan dibuang).
    pub fn create_tag(&mut self, name: &str) -> Result<Uuid, AppError> {
        let name = normalize_tag(name)?;
        let tx = self.conn.transaction()?;
        let id = tag::insert_tag(&tx, &name).map_err(AppError::duplicate_as("tag"))?;
        tx.commit()?;
        Ok(id)
    }

    pub fn rename_tag(&mut self, id: Uuid, name: &str) -> Result<(), AppError> {
        let name = normalize_tag(name)?;
        let tx = self.conn.transaction()?;
        if !tag::rename_tag(&tx, id, &name).map_err(AppError::duplicate_as("tag"))? {
            return Err(AppError::NotFound("tag"));
        }
        tx.commit()?;
        Ok(())
    }

    /// Semua transaksi bertag `source` (termasuk yang terhapus) dipindah ke `target`.
    pub fn merge_tag(&mut self, source: Uuid, target: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        ensure(&tx, source)?;
        ensure(&tx, target)?;
        if source == target {
            return Err(CatalogError::MergeIntoSelf.into());
        }
        tag::merge_tag(&tx, source, target)?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_tag(&mut self, id: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        if !tag::delete_tag(&tx, id)? {
            return Err(AppError::NotFound("tag"));
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::testkit::Kit;
    use crate::{AppError, CatalogError, FormBody, SplitLine, TxnForm};
    use time::macros::date;
    use uuid::Uuid;

    fn tagged(kit: &mut Kit, tags: &[&str]) -> Uuid {
        let category_id = kit.cat("Kopi & jajan");
        kit.ledger
            .create_txn(&TxnForm {
                body: FormBody::Expense {
                    account_id: kit.bca,
                    amount: 10_000,
                    splits: vec![SplitLine {
                        category_id,
                        amount: 10_000,
                        memo: None,
                    }],
                },
                date: date!(2026 - 10 - 10),
                payee: None,
                note: None,
                tags: tags.iter().map(|t| (*t).to_owned()).collect(),
            })
            .unwrap()
    }

    fn kit() -> Kit {
        Kit::new(1, date!(2026 - 10 - 01))
    }

    fn names(kit: &Kit) -> Vec<String> {
        kit.ledger
            .list_tag_usage()
            .unwrap()
            .into_iter()
            .map(|t| t.name)
            .collect()
    }

    #[test]
    fn create_tag_normalizes_and_rejects_duplicate() {
        let mut kit = kit();
        kit.ledger.create_tag(" #Liburan ").unwrap();
        assert_eq!(names(&kit), vec!["Liburan"]);
        assert!(matches!(
            kit.ledger.create_tag("liburan"),
            Err(AppError::Duplicate("tag"))
        ));
        assert!(matches!(
            kit.ledger.create_tag("  "),
            Err(AppError::Catalog(CatalogError::EmptyName))
        ));
    }

    #[test]
    fn rename_tag_case_only_change() {
        let mut kit = kit();
        let id = kit.ledger.create_tag("kantor").unwrap();
        kit.ledger.rename_tag(id, "Kantor").unwrap();
        assert_eq!(names(&kit), vec!["Kantor"]);
    }

    #[test]
    fn rename_tag_to_other_name_is_duplicate() {
        let mut kit = kit();
        let id = kit.ledger.create_tag("kantor").unwrap();
        kit.ledger.create_tag("kerja").unwrap();
        assert!(matches!(
            kit.ledger.rename_tag(id, "#KERJA"),
            Err(AppError::Duplicate("tag"))
        ));
    }

    #[test]
    fn merge_tag_into_self_rejected() {
        let mut kit = kit();
        let id = kit.ledger.create_tag("kantor").unwrap();
        assert!(matches!(
            kit.ledger.merge_tag(id, id),
            Err(AppError::Catalog(CatalogError::MergeIntoSelf))
        ));
    }

    #[test]
    fn merge_tag_moves_txns() {
        let mut kit = kit();
        let t = tagged(&mut kit, &["a"]);
        let both = tagged(&mut kit, &["a", "b"]);
        let usage = kit.ledger.list_tag_usage().unwrap();
        let (a, b) = (usage[0].id, usage[1].id);
        kit.ledger.merge_tag(a, b).unwrap();
        assert_eq!(kit.ledger.get_txn(t).unwrap().tags, vec!["b"]);
        assert_eq!(kit.ledger.get_txn(both).unwrap().tags, vec!["b"]);
        let usage = kit.ledger.list_tag_usage().unwrap();
        assert_eq!(usage.len(), 1);
        assert_eq!(usage[0].txn_count, 2);
    }

    #[test]
    fn delete_tag_removes_from_txns() {
        let mut kit = kit();
        let t = tagged(&mut kit, &["a"]);
        let a = kit.ledger.list_tag_usage().unwrap()[0].id;
        kit.ledger.delete_tag(a).unwrap();
        assert!(kit.ledger.get_txn(t).unwrap().tags.is_empty());
        assert!(names(&kit).is_empty());
    }

    #[test]
    fn unknown_tag_not_found() {
        let mut kit = kit();
        let known = kit.ledger.create_tag("kantor").unwrap();
        let x = Uuid::now_v7();
        assert!(matches!(
            kit.ledger.rename_tag(x, "y"),
            Err(AppError::NotFound("tag"))
        ));
        assert!(matches!(
            kit.ledger.delete_tag(x),
            Err(AppError::NotFound("tag"))
        ));
        assert!(matches!(
            kit.ledger.merge_tag(x, known),
            Err(AppError::NotFound("tag"))
        ));
        assert!(matches!(
            kit.ledger.merge_tag(known, x),
            Err(AppError::NotFound("tag"))
        ));
    }
}
