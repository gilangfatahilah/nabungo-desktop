//! Tag: label bebas pada transaksi, unik tanpa membedakan huruf besar/kecil.

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use uuid::Uuid;

use crate::{Result, map_unique, parse_uuid};

/// ID tag bernama `name` (NOCASE); dibuat bila belum ada. `name` sudah dinormalisasi app.
pub fn find_or_insert_tag(tx: &Transaction, name: &str) -> Result<Uuid> {
    let existing: Option<String> = tx
        .query_row(
            "SELECT id FROM tag WHERE name = ?1 COLLATE NOCASE",
            [name],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return parse_uuid("tag", &id);
    }
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO tag (id, name) VALUES (?1, ?2)",
        params![id.to_string(), name],
    )
    .map_err(map_unique)?;
    Ok(id)
}

/// Ganti seluruh tag transaksi dengan `tags`.
pub fn set_txn_tags(tx: &Transaction, txn_id: Uuid, tags: &[Uuid]) -> Result<()> {
    tx.execute(
        "DELETE FROM txn_tag WHERE txn_id = ?1",
        [txn_id.to_string()],
    )?;
    let mut stmt =
        tx.prepare_cached("INSERT OR IGNORE INTO txn_tag (txn_id, tag_id) VALUES (?1, ?2)")?;
    for tag in tags {
        stmt.execute(params![txn_id.to_string(), tag.to_string()])?;
    }
    Ok(())
}

/// Nama tag transaksi, urut nama (NOCASE).
pub fn txn_tags(conn: &Connection, txn_id: Uuid) -> Result<Vec<String>> {
    let mut stmt = conn.prepare_cached(
        "SELECT g.name FROM txn_tag tt JOIN tag g ON g.id = tt.tag_id
         WHERE tt.txn_id = ?1 ORDER BY g.name COLLATE NOCASE",
    )?;
    let names = stmt
        .query_map([txn_id.to_string()], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(names)
}

/// Semua tag, urut nama (NOCASE).
pub fn list_tags(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare_cached("SELECT name FROM tag ORDER BY name COLLATE NOCASE")?;
    let names = stmt
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{now_utc, open_in_memory};

    fn raw_txn(tx: &Transaction) -> Uuid {
        let id = Uuid::now_v7();
        tx.execute(
            "INSERT INTO txn (id, kind, date, created_at, updated_at)
             VALUES (?1, 'adjustment', '2026-10-17', ?2, ?2)",
            params![id.to_string(), now_utc()],
        )
        .unwrap();
        id
    }

    #[test]
    fn tags_reuse_case_insensitive() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let a = find_or_insert_tag(&tx, "Liburan").unwrap();
        let b = find_or_insert_tag(&tx, "liburan").unwrap();
        assert_eq!(a, b);
        assert_eq!(list_tags(&tx).unwrap(), vec!["Liburan"]);
    }

    #[test]
    fn set_txn_tags_replaces() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let t = raw_txn(&tx);
        let a = find_or_insert_tag(&tx, "a").unwrap();
        let b = find_or_insert_tag(&tx, "b").unwrap();
        set_txn_tags(&tx, t, &[b, a]).unwrap();
        assert_eq!(txn_tags(&tx, t).unwrap(), vec!["a", "b"]);
        set_txn_tags(&tx, t, &[b]).unwrap();
        assert_eq!(txn_tags(&tx, t).unwrap(), vec!["b"]);
        assert_eq!(list_tags(&tx).unwrap(), vec!["a", "b"]);
    }
}
