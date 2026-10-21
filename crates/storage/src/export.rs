//! Baca seluruh transaksi aktif untuk ekspor, tanpa memuat semuanya sekaligus.

use rusqlite::Connection;
use uuid::Uuid;

use crate::{Result, StorageError, parse_uuid};

/// Transaksi apa adanya di DB (kind/date/status berupa teks tersimpan).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportTxn {
    pub id: Uuid,
    pub kind: String,
    pub date: String,
    pub status: String,
    pub payee: Option<String>,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub postings: Vec<ExportPosting>,
    pub attachments: Vec<ExportAttachment>,
}

/// Metadata lampiran saja; isi file ada di folder lampiran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportAttachment {
    pub file_name: String,
    pub sha256: String,
    pub mime: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportPosting {
    pub account_id: Uuid,
    pub category_id: Option<Uuid>,
    pub amount: i64,
    pub currency: String,
    pub memo: Option<String>,
    pub reconciled: bool,
}

/// Panggil `f` untuk tiap transaksi aktif, urut `date, created_at`. Error dari `f`
/// menghentikan iterasi dan diteruskan.
pub fn for_each_txn<E: From<StorageError>>(
    conn: &Connection,
    mut f: impl FnMut(ExportTxn) -> std::result::Result<(), E>,
) -> std::result::Result<(), E> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.kind, t.date, t.status, py.name, t.note,
                    p.account_id, p.category_id, p.amount, a.currency, p.memo, p.reconciled
             FROM txn t
             LEFT JOIN payee py ON py.id = t.payee_id
             JOIN posting p ON p.txn_id = t.id
             JOIN account a ON a.id = p.account_id
             WHERE t.deleted_at IS NULL
             ORDER BY t.date, t.created_at, t.id, p.id",
        )
        .map_err(StorageError::from)?;
    let mut tags = conn
        .prepare(
            "SELECT g.name FROM txn_tag tt JOIN tag g ON g.id = tt.tag_id
             WHERE tt.txn_id = ?1 ORDER BY g.name",
        )
        .map_err(StorageError::from)?;
    let mut attachments = conn
        .prepare(
            "SELECT file_name, sha256, mime, size_bytes FROM attachment
             WHERE txn_id = ?1 ORDER BY created_at, id",
        )
        .map_err(StorageError::from)?;
    let mut rows = stmt.query([]).map_err(StorageError::from)?;
    let mut current: Option<ExportTxn> = None;
    while let Some(r) = rows.next().map_err(StorageError::from)? {
        let row = (|| -> Result<(Uuid, ExportPosting)> {
            let category: Option<String> = r.get(7)?;
            Ok((
                parse_uuid("txn", &r.get::<_, String>(0)?)?,
                ExportPosting {
                    account_id: parse_uuid("posting", &r.get::<_, String>(6)?)?,
                    category_id: category
                        .as_deref()
                        .map(|c| parse_uuid("posting", c))
                        .transpose()?,
                    amount: r.get(8)?,
                    currency: r.get(9)?,
                    memo: r.get(10)?,
                    reconciled: r.get(11)?,
                },
            ))
        })()?;
        let (id, posting) = row;
        if current.as_ref().is_some_and(|t| t.id != id) {
            f(current.take().expect("checked above"))?;
        }
        match &mut current {
            Some(t) => t.postings.push(posting),
            None => {
                let tag_names = tags
                    .query_map([id.to_string()], |r| r.get(0))
                    .and_then(|rows| rows.collect::<rusqlite::Result<Vec<String>>>())
                    .map_err(StorageError::from)?;
                let atts = attachments
                    .query_map([id.to_string()], |r| {
                        Ok(ExportAttachment {
                            file_name: r.get(0)?,
                            sha256: r.get(1)?,
                            mime: r.get(2)?,
                            size_bytes: u64::try_from(r.get::<_, i64>(3)?).unwrap_or(0),
                        })
                    })
                    .and_then(|rows| rows.collect::<rusqlite::Result<Vec<_>>>())
                    .map_err(StorageError::from)?;
                current = Some(ExportTxn {
                    id,
                    kind: r.get(1).map_err(StorageError::from)?,
                    date: r.get(2).map_err(StorageError::from)?,
                    status: r.get(3).map_err(StorageError::from)?,
                    payee: r.get(4).map_err(StorageError::from)?,
                    note: r.get(5).map_err(StorageError::from)?,
                    tags: tag_names,
                    postings: vec![posting],
                    attachments: atts,
                });
            }
        }
    }
    if let Some(t) = current {
        f(t)?;
    }
    Ok(())
}
