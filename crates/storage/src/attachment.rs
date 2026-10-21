//! Metadata lampiran transaksi. Isi file ada di folder data, dirujuk lewat sha256.

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};
use uuid::Uuid;

use crate::{Result, parse_uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentRecord {
    pub id: Uuid,
    pub txn_id: Uuid,
    pub file_name: String,
    /// Hex huruf kecil; nama file di folder lampiran.
    pub sha256: String,
    pub mime: String,
    pub size_bytes: u64,
    pub created_at: String,
}

const COLUMNS: &str = "id, txn_id, file_name, sha256, mime, size_bytes, created_at";

fn from_row(r: &Row) -> rusqlite::Result<(String, String, AttachmentRecord)> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        AttachmentRecord {
            id: Uuid::nil(),
            txn_id: Uuid::nil(),
            file_name: r.get(2)?,
            sha256: r.get(3)?,
            mime: r.get(4)?,
            size_bytes: u64::try_from(r.get::<_, i64>(5)?).unwrap_or(0),
            created_at: r.get(6)?,
        },
    ))
}

fn finish((id, txn, mut rec): (String, String, AttachmentRecord)) -> Result<AttachmentRecord> {
    rec.id = parse_uuid("attachment", &id)?;
    rec.txn_id = parse_uuid("attachment", &txn)?;
    Ok(rec)
}

fn query(
    conn: &Connection,
    sql: &str,
    args: impl rusqlite::Params,
) -> Result<Vec<AttachmentRecord>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let raw = stmt
        .query_map(args, from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter().map(finish).collect()
}

pub fn insert_attachment(tx: &Transaction, rec: &AttachmentRecord) -> Result<()> {
    tx.execute(
        &format!("INSERT INTO attachment ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"),
        params![
            rec.id.to_string(),
            rec.txn_id.to_string(),
            rec.file_name,
            rec.sha256,
            rec.mime,
            i64::try_from(rec.size_bytes).unwrap_or(i64::MAX),
            rec.created_at
        ],
    )?;
    Ok(())
}

/// Lampiran satu transaksi, urut waktu ditambahkan.
pub fn list_for_txn(conn: &Connection, txn_id: Uuid) -> Result<Vec<AttachmentRecord>> {
    query(
        conn,
        &format!("SELECT {COLUMNS} FROM attachment WHERE txn_id = ?1 ORDER BY created_at, id"),
        [txn_id.to_string()],
    )
}

pub fn get_attachment(conn: &Connection, id: Uuid) -> Result<Option<AttachmentRecord>> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM attachment WHERE id = ?1"),
        [id.to_string()],
        from_row,
    )
    .optional()?
    .map(finish)
    .transpose()
}

/// `false` bila tidak ada.
pub fn delete_attachment(tx: &Transaction, id: Uuid) -> Result<bool> {
    Ok(tx.execute("DELETE FROM attachment WHERE id = ?1", [id.to_string()])? > 0)
}

/// Jumlah baris (termasuk milik transaksi terhapus) yang merujuk isi `sha256`.
pub fn sha_ref_count(conn: &Connection, sha256: &str) -> Result<u32> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM attachment WHERE sha256 = ?1",
        [sha256],
        |r| r.get(0),
    )?)
}

/// Semua sha256 yang dirujuk, unik dan urut.
pub fn all_shas(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare_cached("SELECT DISTINCT sha256 FROM attachment ORDER BY sha256")?;
    let shas = stmt
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(shas)
}

/// Lampiran milik transaksi aktif, urut transaksi lalu waktu ditambahkan.
pub fn for_active_txns(conn: &Connection) -> Result<Vec<AttachmentRecord>> {
    query(
        conn,
        "SELECT a.id, a.txn_id, a.file_name, a.sha256, a.mime, a.size_bytes, a.created_at
         FROM attachment a JOIN txn t ON t.id = a.txn_id
         WHERE t.deleted_at IS NULL ORDER BY a.txn_id, a.created_at, a.id",
        [],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_in_memory;
    use rusqlite::Transaction;

    fn raw_txn(tx: &Transaction, deleted: bool) -> Uuid {
        let id = Uuid::now_v7();
        tx.execute(
            "INSERT INTO txn (id, kind, date, deleted_at, created_at, updated_at)
             VALUES (?1, 'adjustment', '2026-10-17', ?2, 'x', 'x')",
            rusqlite::params![id.to_string(), deleted.then_some("x")],
        )
        .unwrap();
        id
    }

    fn record(txn_id: Uuid, sha: &str, created_at: &str) -> AttachmentRecord {
        AttachmentRecord {
            id: Uuid::now_v7(),
            txn_id,
            file_name: "struk.png".into(),
            sha256: sha.into(),
            mime: "image/png".into(),
            size_bytes: 42,
            created_at: created_at.into(),
        }
    }

    #[test]
    fn attachment_crud_and_ref_count() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let (a, b, gone) = (raw_txn(&tx, false), raw_txn(&tx, false), raw_txn(&tx, true));
        let ra = record(a, "aa", "1");
        let ra2 = record(a, "cc", "2");
        let rb = record(b, "aa", "1");
        let rg = record(gone, "dd", "1");
        for r in [&ra, &ra2, &rb, &rg] {
            insert_attachment(&tx, r).unwrap();
        }
        assert_eq!(sha_ref_count(&tx, "aa").unwrap(), 2);
        assert_eq!(list_for_txn(&tx, a).unwrap(), vec![ra.clone(), ra2.clone()]);
        assert_eq!(get_attachment(&tx, rb.id).unwrap(), Some(rb.clone()));
        assert!(delete_attachment(&tx, rb.id).unwrap());
        assert!(!delete_attachment(&tx, rb.id).unwrap());
        assert_eq!(sha_ref_count(&tx, "aa").unwrap(), 1);
        assert_eq!(get_attachment(&tx, rb.id).unwrap(), None);
        assert_eq!(all_shas(&tx).unwrap(), vec!["aa", "cc", "dd"]);
        assert_eq!(for_active_txns(&tx).unwrap(), vec![ra, ra2]);
    }
}
