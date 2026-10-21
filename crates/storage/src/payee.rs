use nabungo_core::date::parse_date;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use time::Date;
use uuid::Uuid;

use crate::{Result, StorageError, like_contains, map_unique, parse_uuid};

/// Cari payee berdasarkan nama, tanpa membedakan huruf besar/kecil.
pub fn find_payee(conn: &Connection, name: &str) -> Result<Option<Uuid>> {
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM payee WHERE name = ?1 COLLATE NOCASE",
            [name],
            |r| r.get(0),
        )
        .optional()?;
    id.map(|id| parse_uuid("payee", &id)).transpose()
}

pub fn insert_payee(tx: &Transaction, name: &str) -> Result<Uuid> {
    insert_payee_with(tx, name, None)
}

pub fn insert_payee_with(
    tx: &Transaction,
    name: &str,
    default_category_id: Option<Uuid>,
) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO payee (id, name, default_category_id) VALUES (?1, ?2, ?3)",
        params![
            id.to_string(),
            name,
            default_category_id.map(|c| c.to_string())
        ],
    )
    .map_err(map_unique)?;
    Ok(id)
}

/// `false` bila payee tidak ada; nama milik payee lain (NOCASE) → `Duplicate`.
pub fn update_payee(
    tx: &Transaction,
    id: Uuid,
    name: &str,
    default_category_id: Option<Uuid>,
) -> Result<bool> {
    let n = tx
        .execute(
            "UPDATE payee SET name = ?2, default_category_id = ?3 WHERE id = ?1",
            params![
                id.to_string(),
                name,
                default_category_id.map(|c| c.to_string())
            ],
        )
        .map_err(map_unique)?;
    Ok(n > 0)
}

pub fn get_payee_name(conn: &Connection, id: Uuid) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT name FROM payee WHERE id = ?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?)
}

/// Semua transaksi `source` (termasuk yang terhapus, agar undo tetap konsisten) dipindah
/// ke `target`, lalu `source` dihapus.
pub fn merge_payee(tx: &Transaction, source: Uuid, target: Uuid) -> Result<()> {
    tx.execute(
        "UPDATE txn SET payee_id = ?2 WHERE payee_id = ?1",
        params![source.to_string(), target.to_string()],
    )?;
    tx.execute("DELETE FROM payee WHERE id = ?1", [source.to_string()])?;
    Ok(())
}

/// Kosongkan payee di semua transaksi (termasuk yang terhapus), lalu hapus. `false` bila
/// tidak ada.
pub fn delete_payee(tx: &Transaction, id: Uuid) -> Result<bool> {
    tx.execute(
        "UPDATE txn SET payee_id = NULL WHERE payee_id = ?1",
        [id.to_string()],
    )?;
    Ok(tx.execute("DELETE FROM payee WHERE id = ?1", [id.to_string()])? > 0)
}

/// Payee beserta pemakaian oleh transaksi aktif.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeeUsage {
    pub id: Uuid,
    pub name: String,
    pub default_category_id: Option<Uuid>,
    pub txn_count: u32,
    pub last_date: Option<Date>,
}

/// Semua payee, urut nama (NOCASE).
pub fn list_payee_usage(conn: &Connection) -> Result<Vec<PayeeUsage>> {
    let mut stmt = conn.prepare_cached(
        "SELECT p.id, p.name, p.default_category_id, COUNT(t.id), MAX(t.date)
         FROM payee p LEFT JOIN txn t ON t.payee_id = p.id AND t.deleted_at IS NULL
         GROUP BY p.id ORDER BY p.name COLLATE NOCASE",
    )?;
    let raw = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, u32>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter()
        .map(|(id, name, cat, txn_count, last)| {
            Ok(PayeeUsage {
                id: parse_uuid("payee", &id)?,
                name,
                default_category_id: cat
                    .as_deref()
                    .map(|c| parse_uuid("category", c))
                    .transpose()?,
                txn_count,
                last_date: last
                    .as_deref()
                    .map(|d| {
                        parse_date(d).map_err(|e| StorageError::Corrupt(format!("txn date: {e}")))
                    })
                    .transpose()?,
            })
        })
        .collect()
}

pub fn payee_default_category(conn: &Connection, id: Uuid) -> Result<Option<Uuid>> {
    let cat: Option<Option<String>> = conn
        .query_row(
            "SELECT default_category_id FROM payee WHERE id = ?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    cat.flatten()
        .map(|c| parse_uuid("category", &c))
        .transpose()
}

/// Posting (akun, kategori) dari transaksi aktif terbaru milik payee.
pub fn last_postings_for_payee(
    conn: &Connection,
    payee_id: Uuid,
) -> Result<Vec<(Uuid, Option<Uuid>)>> {
    let mut stmt = conn.prepare_cached(
        "SELECT p.account_id, p.category_id FROM posting p
         WHERE p.txn_id = (SELECT t.id FROM txn t
                           WHERE t.payee_id = ?1 AND t.deleted_at IS NULL
                             AND t.status = 'posted'
                           ORDER BY t.date DESC, t.created_at DESC, t.id DESC
                           LIMIT 1)
         ORDER BY p.id",
    )?;
    let raw = stmt
        .query_map([payee_id.to_string()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter()
        .map(|(a, c)| {
            Ok((
                parse_uuid("posting", &a)?,
                c.as_deref().map(|c| parse_uuid("posting", c)).transpose()?,
            ))
        })
        .collect()
}

/// Nama payee yang mengandung `query` (tanpa membedakan huruf, literal), urut transaksi
/// aktif terbaru lalu nama.
pub fn search_payees(conn: &Connection, query: &str, limit: u32) -> Result<Vec<String>> {
    let mut stmt = conn.prepare_cached(
        "SELECT p.name FROM payee p
         LEFT JOIN txn t ON t.payee_id = p.id AND t.deleted_at IS NULL
         WHERE p.name LIKE ?1 ESCAPE '\\'
         GROUP BY p.id
         ORDER BY MAX(t.date) IS NULL, MAX(t.date) DESC, p.name COLLATE NOCASE
         LIMIT ?2",
    )?;
    let names = stmt
        .query_map(params![like_contains(query), limit], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_in_memory;

    fn txn_on(tx: &Transaction, payee: Uuid, date: &str) {
        tx.execute(
            "INSERT INTO txn (id, kind, date, payee_id, created_at, updated_at)
             VALUES (?1, 'adjustment', ?2, ?3, 'x', 'x')",
            rusqlite::params![Uuid::now_v7().to_string(), date, payee.to_string()],
        )
        .unwrap();
    }

    #[test]
    fn search_payees_contains_nocase_recent_first() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let indo = insert_payee(&tx, "Indomaret").unwrap();
        let alfa = insert_payee(&tx, "Alfamart Indo").unwrap();
        insert_payee(&tx, "Indo Tanpa Transaksi").unwrap();
        insert_payee(&tx, "Shopee").unwrap();
        insert_payee(&tx, "Diskon 50%").unwrap();
        txn_on(&tx, indo, "2026-10-10");
        txn_on(&tx, alfa, "2026-10-12");
        assert_eq!(
            search_payees(&tx, "indo", 8).unwrap(),
            vec!["Alfamart Indo", "Indomaret", "Indo Tanpa Transaksi"]
        );
        assert_eq!(
            search_payees(&tx, "indo", 1).unwrap(),
            vec!["Alfamart Indo"]
        );
        assert_eq!(search_payees(&tx, "50%", 8).unwrap(), vec!["Diskon 50%"]);
        assert!(search_payees(&tx, "n_", 8).unwrap().is_empty());
    }

    #[test]
    fn find_is_case_insensitive_and_unique() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let id = insert_payee(&tx, "Indomaret").unwrap();
        assert_eq!(find_payee(&tx, "INDOMARET").unwrap(), Some(id));
        assert_eq!(find_payee(&tx, "Alfamart").unwrap(), None);
        assert!(insert_payee(&tx, "indomaret").is_err());
        assert_eq!(payee_default_category(&tx, id).unwrap(), None);
    }

    fn txn_id_on(tx: &Transaction, payee: Uuid, date: &str, deleted: bool) -> Uuid {
        let id = Uuid::now_v7();
        tx.execute(
            "INSERT INTO txn (id, kind, date, payee_id, deleted_at, created_at, updated_at)
             VALUES (?1, 'adjustment', ?2, ?3, ?4, 'x', 'x')",
            rusqlite::params![
                id.to_string(),
                date,
                payee.to_string(),
                deleted.then_some("x")
            ],
        )
        .unwrap();
        id
    }

    fn payee_of(tx: &Transaction, txn: Uuid) -> Option<String> {
        tx.query_row(
            "SELECT payee_id FROM txn WHERE id = ?1",
            [txn.to_string()],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn payee_usage_and_last_date() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let p = insert_payee(&tx, "Indomaret").unwrap();
        insert_payee_with(&tx, "Alfamart", None).unwrap();
        txn_id_on(&tx, p, "2026-10-10", false);
        txn_id_on(&tx, p, "2026-10-12", true);
        let usage = list_payee_usage(&tx).unwrap();
        assert_eq!(usage.len(), 2);
        assert_eq!(usage[0].name, "Alfamart");
        assert_eq!(usage[0].txn_count, 0);
        assert_eq!(usage[0].last_date, None);
        assert_eq!(usage[1].id, p);
        assert_eq!(usage[1].txn_count, 1);
        assert_eq!(
            usage[1].last_date,
            Some(time::macros::date!(2026 - 10 - 10))
        );
        assert_eq!(usage[1].default_category_id, None);
    }

    #[test]
    fn update_payee_name_and_default_category() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let p = insert_payee(&tx, "indomaret").unwrap();
        let other = insert_payee(&tx, "Alfamart").unwrap();
        let cat = crate::category::insert_category(
            &tx,
            &crate::category::NewCategory::new(
                nabungo_core::CategoryKind::Expense,
                "Belanja",
                None,
            ),
        )
        .unwrap();
        assert!(update_payee(&tx, p, "Indomaret", Some(cat)).unwrap());
        assert_eq!(payee_default_category(&tx, p).unwrap(), Some(cat));
        assert_eq!(
            get_payee_name(&tx, p).unwrap().as_deref(),
            Some("Indomaret")
        );
        assert!(matches!(
            update_payee(&tx, other, "INDOMARET", None),
            Err(crate::StorageError::Duplicate)
        ));
        assert!(!update_payee(&tx, Uuid::now_v7(), "x", None).unwrap());
        assert_eq!(get_payee_name(&tx, Uuid::now_v7()).unwrap(), None);
    }

    #[test]
    fn merge_and_delete_payee_touch_deleted_txns() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let source = insert_payee(&tx, "Indomart").unwrap();
        let target = insert_payee(&tx, "Indomaret").unwrap();
        let active = txn_id_on(&tx, source, "2026-10-10", false);
        let deleted = txn_id_on(&tx, source, "2026-10-11", true);
        merge_payee(&tx, source, target).unwrap();
        assert_eq!(payee_of(&tx, active), Some(target.to_string()));
        assert_eq!(payee_of(&tx, deleted), Some(target.to_string()));
        assert_eq!(get_payee_name(&tx, source).unwrap(), None);
        assert!(delete_payee(&tx, target).unwrap());
        assert!(!delete_payee(&tx, target).unwrap());
        assert_eq!(payee_of(&tx, active), None);
        assert_eq!(payee_of(&tx, deleted), None);
    }
}
