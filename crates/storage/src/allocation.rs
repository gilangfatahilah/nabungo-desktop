//! Bucket alokasi modal dan mapping akun → bucket (PRD §4.5). Porsi aktual dihitung di core.

use nabungo_core::{Mapping, Target};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use uuid::Uuid;

use crate::{Result, map_unique, parse_uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBucket {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub target: Target,
    pub color: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketRow {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub target: Target,
    pub color: Option<String>,
    pub sort_order: i64,
}

/// Kolom `(target_bp, target_amount)`: bucket nominal disimpan dengan `target_bp = 0`.
fn target_columns(target: Target) -> (i64, Option<i64>) {
    match target {
        Target::Percent(bp) => (bp, None),
        Target::Amount(n) => (0, Some(n)),
    }
}

pub fn insert_bucket(tx: &Transaction, new: &NewBucket) -> Result<Uuid> {
    let id = Uuid::now_v7();
    let (bp, amount) = target_columns(new.target);
    tx.execute(
        "INSERT INTO allocation_bucket
           (id, parent_id, name, target_bp, target_amount, color, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id.to_string(),
            new.parent_id.map(|p| p.to_string()),
            new.name,
            bp,
            amount,
            new.color,
            new.sort_order
        ],
    )
    .map_err(map_unique)?;
    Ok(id)
}

pub fn update_bucket(tx: &Transaction, id: Uuid, new: &NewBucket) -> Result<bool> {
    let (bp, amount) = target_columns(new.target);
    let changed = tx
        .execute(
            "UPDATE allocation_bucket
             SET parent_id = ?2, name = ?3, target_bp = ?4, target_amount = ?5, color = ?6,
                 sort_order = ?7
             WHERE id = ?1",
            params![
                id.to_string(),
                new.parent_id.map(|p| p.to_string()),
                new.name,
                bp,
                amount,
                new.color,
                new.sort_order
            ],
        )
        .map_err(map_unique)?;
    Ok(changed == 1)
}

/// Mapping ke bucket ini ikut dihapus. Pemanggil memastikan bucket tidak punya child.
pub fn delete_bucket(tx: &Transaction, id: Uuid) -> Result<bool> {
    let key = id.to_string();
    tx.execute(
        "DELETE FROM allocation_mapping WHERE bucket_id = ?1",
        [&key],
    )?;
    Ok(tx.execute("DELETE FROM allocation_bucket WHERE id = ?1", [&key])? == 1)
}

const ROW_SQL: &str = "SELECT id, parent_id, name, target_bp, target_amount, color, sort_order
     FROM allocation_bucket";

type RawBucket = (
    String,
    Option<String>,
    String,
    i64,
    Option<i64>,
    Option<String>,
    i64,
);

fn raw(r: &rusqlite::Row) -> rusqlite::Result<RawBucket> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
        r.get(6)?,
    ))
}

fn to_row(
    (id, parent_id, name, target_bp, target_amount, color, sort_order): RawBucket,
) -> Result<BucketRow> {
    Ok(BucketRow {
        id: parse_uuid("allocation_bucket", &id)?,
        parent_id: parent_id
            .as_deref()
            .map(|p| parse_uuid("allocation_bucket", p))
            .transpose()?,
        name,
        target: target_amount.map_or(Target::Percent(target_bp), Target::Amount),
        color,
        sort_order,
    })
}

/// Urutan pohon: root (sort_order, nama), masing-masing diikuti child-nya.
pub fn list_buckets(conn: &Connection) -> Result<Vec<BucketRow>> {
    let mut stmt = conn.prepare_cached(&format!("{ROW_SQL} ORDER BY sort_order, name"))?;
    let all = stmt
        .query_map([], raw)?
        .map(|r| to_row(r?))
        .collect::<Result<Vec<_>>>()?;
    let mut out = Vec::with_capacity(all.len());
    for root in all.iter().filter(|b| b.parent_id.is_none()) {
        out.push(root.clone());
        out.extend(all.iter().filter(|b| b.parent_id == Some(root.id)).cloned());
    }
    Ok(out)
}

pub fn get_bucket(conn: &Connection, id: Uuid) -> Result<Option<BucketRow>> {
    conn.query_row(&format!("{ROW_SQL} WHERE id = ?1"), [id.to_string()], raw)
        .optional()?
        .map(to_row)
        .transpose()
}

pub fn child_bucket_count(conn: &Connection, id: Uuid) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM allocation_bucket WHERE parent_id = ?1",
        [id.to_string()],
        |r| r.get(0),
    )?)
}

/// `sort_order` berikutnya di antara sesama root atau sesama child satu parent.
pub fn next_bucket_sort_order(conn: &Connection, parent: Option<Uuid>) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(sort_order) + 1, 0) FROM allocation_bucket WHERE parent_id IS ?1",
        [parent.map(|p| p.to_string())],
        |r| r.get(0),
    )?)
}

pub fn list_mappings(conn: &Connection) -> Result<Vec<Mapping>> {
    let mut stmt = conn.prepare_cached(
        "SELECT account_id, bucket_id, share_bp FROM allocation_mapping
         ORDER BY account_id, bucket_id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get(2)?))
    })?;
    rows.map(|r| {
        let (account_id, bucket_id, share_bp) = r?;
        Ok(Mapping {
            account_id: parse_uuid("account", &account_id)?,
            bucket_id: parse_uuid("allocation_bucket", &bucket_id)?,
            share_bp,
        })
    })
    .collect()
}

/// Ganti seluruh mapping satu akun; list kosong menghapus semuanya.
pub fn replace_account_mappings(
    tx: &Transaction,
    account_id: Uuid,
    shares: &[(Uuid, i64)],
) -> Result<()> {
    let key = account_id.to_string();
    tx.execute(
        "DELETE FROM allocation_mapping WHERE account_id = ?1",
        [&key],
    )?;
    let mut stmt = tx.prepare_cached(
        "INSERT INTO allocation_mapping (account_id, bucket_id, share_bp) VALUES (?1, ?2, ?3)",
    )?;
    for (bucket, bp) in shares {
        stmt.execute(params![key, bucket.to_string(), bp])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{NewAccount, insert_account};
    use crate::{StorageError, now_utc, open_in_memory};
    use nabungo_core::{AccountKind, CurrencyCode, Target};

    fn new(parent_id: Option<Uuid>, name: &str, target_bp: i64) -> NewBucket {
        NewBucket {
            parent_id,
            name: name.into(),
            target: Target::Percent(target_bp),
            color: None,
            sort_order: 0,
        }
    }

    #[test]
    fn bucket_names_unique_nocase_per_parent() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let aset = insert_bucket(&tx, &new(None, "Aset", 7000)).unwrap();
        let liquid = insert_bucket(&tx, &new(None, "Liquid", 3000)).unwrap();
        assert!(matches!(
            insert_bucket(&tx, &new(None, "ASET", 0)),
            Err(StorageError::Duplicate)
        ));
        insert_bucket(&tx, &new(Some(aset), "Saham", 6000)).unwrap();
        assert!(matches!(
            insert_bucket(&tx, &new(Some(aset), "saham", 0)),
            Err(StorageError::Duplicate)
        ));
        insert_bucket(&tx, &new(Some(liquid), "Saham", 0)).unwrap();
        let names: Vec<_> = list_buckets(&tx)
            .unwrap()
            .into_iter()
            .map(|b| b.name)
            .collect();
        assert_eq!(names, ["Aset", "Saham", "Liquid", "Saham"]);
        assert_eq!(child_bucket_count(&tx, aset).unwrap(), 1);
    }

    #[test]
    fn delete_bucket_removes_mappings() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let a = insert_account(
            &tx,
            &NewAccount::new("BCA", AccountKind::Bank, CurrencyCode::IDR),
            &now_utc(),
        )
        .unwrap();
        let x = insert_bucket(&tx, &new(None, "X", 5000)).unwrap();
        let y = insert_bucket(&tx, &new(None, "Y", 5000)).unwrap();
        replace_account_mappings(&tx, a, &[(x, 6000), (y, 4000)]).unwrap();
        assert_eq!(list_mappings(&tx).unwrap().len(), 2);
        assert!(delete_bucket(&tx, x).unwrap());
        assert_eq!(
            list_mappings(&tx).unwrap(),
            vec![Mapping {
                account_id: a,
                bucket_id: y,
                share_bp: 4000
            }]
        );
        replace_account_mappings(&tx, a, &[]).unwrap();
        assert!(list_mappings(&tx).unwrap().is_empty());
        assert!(get_bucket(&tx, x).unwrap().is_none());
    }

    #[test]
    fn bucket_target_roundtrip() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let mut b = NewBucket {
            target: Target::Amount(30_000_000),
            ..new(None, "Darurat", 0)
        };
        let id = insert_bucket(&tx, &b).unwrap();
        assert_eq!(
            get_bucket(&tx, id).unwrap().unwrap().target,
            Target::Amount(30_000_000)
        );
        b.target = Target::Percent(2500);
        assert!(update_bucket(&tx, id, &b).unwrap());
        assert_eq!(
            get_bucket(&tx, id).unwrap().unwrap().target,
            Target::Percent(2500)
        );
        let amount: Option<i64> = tx
            .query_row(
                "SELECT target_amount FROM allocation_bucket WHERE id = ?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(amount, None);
    }

    #[test]
    fn target_amount_check_rejects_zero() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let id = insert_bucket(&tx, &new(None, "X", 5000)).unwrap();
        assert!(
            tx.execute(
                "UPDATE allocation_bucket SET target_amount = 0 WHERE id = ?1",
                [id.to_string()],
            )
            .is_err()
        );
    }
}
