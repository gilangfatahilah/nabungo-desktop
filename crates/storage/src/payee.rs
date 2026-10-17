use rusqlite::{Connection, OptionalExtension, Transaction, params};
use uuid::Uuid;

use crate::{Result, like_contains, map_unique, parse_uuid};

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
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO payee (id, name) VALUES (?1, ?2)",
        [id.to_string(), name.to_owned()],
    )
    .map_err(map_unique)?;
    Ok(id)
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
}
