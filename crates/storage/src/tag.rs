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

/// Tag beserta jumlah transaksi aktif yang memakainya.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagUsage {
    pub id: Uuid,
    pub name: String,
    pub txn_count: u32,
}

/// Semua tag dengan jumlah pemakaian (transaksi aktif), urut nama (NOCASE).
pub fn list_tag_usage(conn: &Connection) -> Result<Vec<TagUsage>> {
    let mut stmt = conn.prepare_cached(
        "SELECT g.id, g.name, COUNT(t.id) FROM tag g
         LEFT JOIN txn_tag tt ON tt.tag_id = g.id
         LEFT JOIN txn t ON t.id = tt.txn_id AND t.deleted_at IS NULL
         GROUP BY g.id ORDER BY g.name COLLATE NOCASE",
    )?;
    let raw = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<Vec<(String, String, u32)>>>()?;
    raw.into_iter()
        .map(|(id, name, txn_count)| {
            Ok(TagUsage {
                id: parse_uuid("tag", &id)?,
                name,
                txn_count,
            })
        })
        .collect()
}

pub fn tag_exists(conn: &Connection, id: Uuid) -> Result<bool> {
    Ok(conn
        .query_row("SELECT 1 FROM tag WHERE id = ?1", [id.to_string()], |_| {
            Ok(())
        })
        .optional()?
        .is_some())
}

/// Sisipkan tag baru; nama yang sudah ada (NOCASE) → `Duplicate`.
pub fn insert_tag(tx: &Transaction, name: &str) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO tag (id, name) VALUES (?1, ?2)",
        params![id.to_string(), name],
    )
    .map_err(map_unique)?;
    Ok(id)
}

/// `false` bila tag tidak ada.
pub fn rename_tag(tx: &Transaction, id: Uuid, name: &str) -> Result<bool> {
    let n = tx
        .execute(
            "UPDATE tag SET name = ?2 WHERE id = ?1",
            params![id.to_string(), name],
        )
        .map_err(map_unique)?;
    Ok(n > 0)
}

/// Pindahkan semua pemakaian `source` ke `target` (tanpa duplikat), lalu hapus `source`.
pub fn merge_tag(tx: &Transaction, source: Uuid, target: Uuid) -> Result<()> {
    let (source, target) = (source.to_string(), target.to_string());
    tx.execute(
        "INSERT OR IGNORE INTO txn_tag (txn_id, tag_id)
         SELECT txn_id, ?2 FROM txn_tag WHERE tag_id = ?1",
        params![source, target],
    )?;
    tx.execute("DELETE FROM txn_tag WHERE tag_id = ?1", [&source])?;
    tx.execute("DELETE FROM tag WHERE id = ?1", [&source])?;
    Ok(())
}

/// Lepas tag dari semua transaksi (termasuk yang terhapus), lalu hapus. `false` bila tidak ada.
pub fn delete_tag(tx: &Transaction, id: Uuid) -> Result<bool> {
    tx.execute("DELETE FROM txn_tag WHERE tag_id = ?1", [id.to_string()])?;
    Ok(tx.execute("DELETE FROM tag WHERE id = ?1", [id.to_string()])? > 0)
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

    fn delete_txn(tx: &Transaction, id: Uuid) {
        tx.execute(
            "UPDATE txn SET deleted_at = 'x' WHERE id = ?1",
            [id.to_string()],
        )
        .unwrap();
    }

    #[test]
    fn tag_usage_counts_active_txns_only() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let kantor = find_or_insert_tag(&tx, "kantor").unwrap();
        find_or_insert_tag(&tx, "kosong").unwrap();
        let (t1, t2) = (raw_txn(&tx), raw_txn(&tx));
        set_txn_tags(&tx, t1, &[kantor]).unwrap();
        set_txn_tags(&tx, t2, &[kantor]).unwrap();
        delete_txn(&tx, t2);
        let usage: Vec<_> = list_tag_usage(&tx)
            .unwrap()
            .into_iter()
            .map(|u| (u.name, u.txn_count))
            .collect();
        assert_eq!(
            usage,
            vec![("kantor".to_owned(), 1), ("kosong".to_owned(), 0)]
        );
    }

    #[test]
    fn rename_merge_delete_tag() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let a = insert_tag(&tx, "kantor").unwrap();
        let b = insert_tag(&tx, "kerja").unwrap();
        assert!(rename_tag(&tx, a, "Kantor").unwrap());
        assert!(!rename_tag(&tx, Uuid::now_v7(), "x").unwrap());
        assert!(matches!(
            insert_tag(&tx, "KANTOR"),
            Err(crate::StorageError::Duplicate)
        ));
        let t = raw_txn(&tx);
        let only_a = raw_txn(&tx);
        set_txn_tags(&tx, t, &[a, b]).unwrap();
        set_txn_tags(&tx, only_a, &[a]).unwrap();
        merge_tag(&tx, a, b).unwrap();
        assert_eq!(txn_tags(&tx, t).unwrap(), vec!["kerja"]);
        assert_eq!(txn_tags(&tx, only_a).unwrap(), vec!["kerja"]);
        assert_eq!(list_tags(&tx).unwrap(), vec!["kerja"]);
        assert!(delete_tag(&tx, b).unwrap());
        assert!(!delete_tag(&tx, b).unwrap());
        assert!(txn_tags(&tx, t).unwrap().is_empty());
        assert!(list_tags(&tx).unwrap().is_empty());
    }
}
