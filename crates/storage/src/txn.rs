use nabungo_core::date::{format_date, parse_date};
use nabungo_core::{TxnKind, ValidTxn};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use time::Date;
use uuid::Uuid;

use crate::{Result, StorageError, parse_uuid};

/// Transaksi lengkap untuk form edit/duplikasi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnRecord {
    pub id: Uuid,
    pub kind: TxnKind,
    pub date: Date,
    pub payee: Option<String>,
    pub note: Option<String>,
    pub deleted: bool,
    pub postings: Vec<PostingRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostingRecord {
    pub account_id: Uuid,
    pub category_id: Option<Uuid>,
    pub amount: i64,
    pub memo: Option<String>,
}

/// Simpan header + postings. Hanya menerima [`ValidTxn`], jadi invariant sudah dicek di core.
pub fn insert_txn(
    tx: &Transaction,
    txn: &ValidTxn,
    payee_id: Option<Uuid>,
    now: &str,
) -> Result<Uuid> {
    let draft = txn.draft();
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO txn (id, kind, date, payee_id, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            id.to_string(),
            draft.kind.as_str(),
            format_date(draft.date),
            payee_id.map(|p| p.to_string()),
            draft.note,
            now
        ],
    )?;
    insert_postings(tx, id, txn)?;
    Ok(id)
}

/// Posting disimpan dengan ID UUIDv7 berurutan, jadi `ORDER BY id` = urutan input.
fn insert_postings(tx: &Transaction, txn_id: Uuid, txn: &ValidTxn) -> Result<()> {
    let mut stmt = tx.prepare_cached(
        "INSERT INTO posting (id, txn_id, account_id, category_id, amount, memo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for p in &txn.draft().postings {
        stmt.execute(params![
            Uuid::now_v7().to_string(),
            txn_id.to_string(),
            p.account.id.to_string(),
            p.category.map(|c| c.id.to_string()),
            p.amount,
            p.memo,
        ])?;
    }
    Ok(())
}

/// Tulis ulang header + postings transaksi aktif; `id` dan `created_at` tetap.
/// `false` bila ID tidak ada atau sudah terhapus.
pub fn replace_txn(
    tx: &Transaction,
    id: Uuid,
    txn: &ValidTxn,
    payee_id: Option<Uuid>,
    now: &str,
) -> Result<bool> {
    let draft = txn.draft();
    let changed = tx.execute(
        "UPDATE txn SET kind = ?2, date = ?3, payee_id = ?4, note = ?5, updated_at = ?6
         WHERE id = ?1 AND deleted_at IS NULL",
        params![
            id.to_string(),
            draft.kind.as_str(),
            format_date(draft.date),
            payee_id.map(|p| p.to_string()),
            draft.note,
            now
        ],
    )?;
    if changed == 0 {
        return Ok(false);
    }
    tx.execute("DELETE FROM posting WHERE txn_id = ?1", [id.to_string()])?;
    insert_postings(tx, id, txn)?;
    Ok(true)
}

/// Detail transaksi (termasuk yang terhapus; lihat `deleted`). Postings urut input.
pub fn get_txn(conn: &Connection, id: Uuid) -> Result<Option<TxnRecord>> {
    let header = conn
        .query_row(
            "SELECT t.kind, t.date, p.name, t.note, t.deleted_at IS NOT NULL
             FROM txn t LEFT JOIN payee p ON p.id = t.payee_id WHERE t.id = ?1",
            [id.to_string()],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, bool>(4)?,
                ))
            },
        )
        .optional()?;
    let Some((kind, date, payee, note, deleted)) = header else {
        return Ok(None);
    };
    let kind =
        TxnKind::parse(&kind).ok_or_else(|| StorageError::Corrupt(format!("txn kind {kind:?}")))?;
    let date = parse_date(&date).map_err(|e| StorageError::Corrupt(format!("txn date: {e}")))?;
    let mut stmt = conn.prepare_cached(
        "SELECT account_id, category_id, amount, memo FROM posting WHERE txn_id = ?1 ORDER BY id",
    )?;
    let raw = stmt
        .query_map([id.to_string()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let postings = raw
        .into_iter()
        .map(|(a, c, amount, memo)| {
            Ok(PostingRecord {
                account_id: parse_uuid("posting", &a)?,
                category_id: c.as_deref().map(|c| parse_uuid("posting", c)).transpose()?,
                amount,
                memo,
            })
        })
        .collect::<Result<_>>()?;
    Ok(Some(TxnRecord {
        id,
        kind,
        date,
        payee,
        note,
        deleted,
        postings,
    }))
}

/// Akun aktif dari transaksi income/expense terbaru (default akun form).
pub fn last_used_account(conn: &Connection) -> Result<Option<Uuid>> {
    let id: Option<String> = conn
        .query_row(
            "SELECT p.account_id FROM posting p
             JOIN txn t ON t.id = p.txn_id
             JOIN account a ON a.id = p.account_id
             WHERE t.kind IN ('income', 'expense') AND t.deleted_at IS NULL
               AND t.status = 'posted' AND a.archived_at IS NULL
             ORDER BY t.date DESC, t.created_at DESC, t.id DESC
             LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    id.map(|id| parse_uuid("posting", &id)).transpose()
}

/// Kategori biaya admin dari transfer berbiaya terbaru (boleh sudah diarsip; app menyaring).
pub fn last_fee_category(conn: &Connection) -> Result<Option<Uuid>> {
    let id: Option<String> = conn
        .query_row(
            "SELECT p.category_id FROM posting p JOIN txn t ON t.id = p.txn_id
             WHERE t.kind = 'transfer' AND p.category_id IS NOT NULL
               AND t.deleted_at IS NULL AND t.status = 'posted'
             ORDER BY t.date DESC, t.created_at DESC, t.id DESC
             LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    id.map(|id| parse_uuid("posting", &id)).transpose()
}

/// Soft delete transaksi aktif. `false` bila ID tidak ada atau sudah terhapus.
pub fn soft_delete(tx: &Transaction, id: Uuid, now: &str) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE txn SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
        params![id.to_string(), now],
    )?;
    Ok(changed == 1)
}

/// Kembalikan transaksi yang terhapus. `false` bila ID tidak ada atau tidak sedang terhapus.
pub fn restore(tx: &Transaction, id: Uuid, now: &str) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE txn SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NOT NULL",
        params![id.to_string(), now],
    )?;
    Ok(changed == 1)
}

/// Saldo = Σ postings dari transaksi aktif yang sudah `posted`.
pub fn account_balance(conn: &Connection, account_id: Uuid) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(p.amount), 0)
         FROM posting p JOIN txn t ON t.id = p.txn_id
         WHERE p.account_id = ?1 AND t.deleted_at IS NULL AND t.status = 'posted'",
        [account_id.to_string()],
        |r| r.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{NewAccount, get_account_ref, insert_account};
    use crate::category::{NewCategory, get_category_ref, insert_category};
    use crate::{now_utc, open_in_memory};
    use nabungo_core::{
        AccountKind, AccountRef, CategoryKind, CategoryRef, CurrencyCode, PostingDraft, TxnDraft,
        TxnKind,
    };
    use time::macros::date;

    fn bank(tx: &Transaction, name: &str) -> AccountRef {
        let new = NewAccount::new(name, AccountKind::Bank, CurrencyCode::IDR);
        let id = insert_account(tx, &new, &now_utc()).unwrap();
        get_account_ref(tx, id).unwrap().unwrap().value
    }

    fn draft(kind: TxnKind, postings: Vec<(AccountRef, Option<CategoryRef>, i64)>) -> ValidTxn {
        ValidTxn::new(TxnDraft {
            kind,
            date: date!(2026 - 10 - 12),
            note: None,
            postings: postings
                .into_iter()
                .map(|(account, category, amount)| PostingDraft {
                    account,
                    category,
                    amount,
                    memo: None,
                })
                .collect(),
        })
        .unwrap()
    }

    #[test]
    fn account_ref_roundtrip() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let acc = bank(&tx, "BCA");
        assert_eq!(acc.kind, AccountKind::Bank);
        assert_eq!(acc.currency, CurrencyCode::IDR);
        assert_eq!(get_account_ref(&tx, Uuid::now_v7()).unwrap(), None);
    }

    #[test]
    fn category_ref_roundtrip() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let parent = insert_category(
            &tx,
            &NewCategory::new(CategoryKind::Expense, "Makan & Minum", None),
        )
        .unwrap();
        let child = insert_category(
            &tx,
            &NewCategory::new(CategoryKind::Expense, "Kopi", Some(parent)),
        )
        .unwrap();
        let r = get_category_ref(&tx, child).unwrap().unwrap();
        assert_eq!(r.value.kind, CategoryKind::Expense);
        assert!(!r.archived);
    }

    #[test]
    fn lookup_reports_archived() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let acc = bank(&tx, "Kartu Lama");
        let cat =
            insert_category(&tx, &NewCategory::new(CategoryKind::Expense, "Rokok", None)).unwrap();
        tx.execute_batch("UPDATE account SET archived_at = '2026-10-16T00:00:00.000Z'; UPDATE category SET archived_at = '2026-10-16T00:00:00.000Z';")
            .unwrap();
        assert!(get_account_ref(&tx, acc.id).unwrap().unwrap().archived);
        assert!(get_category_ref(&tx, cat).unwrap().unwrap().archived);
    }

    #[test]
    fn root_category_names_unique_per_kind() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        insert_category(&tx, &NewCategory::new(CategoryKind::Expense, "Makan", None)).unwrap();
        assert!(
            insert_category(&tx, &NewCategory::new(CategoryKind::Expense, "Makan", None)).is_err()
        );
        insert_category(&tx, &NewCategory::new(CategoryKind::Income, "Makan", None)).unwrap();
    }

    #[test]
    fn balance_is_sum_of_postings() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let (a, b) = (bank(&tx, "BCA"), bank(&tx, "Jago"));
        let now = now_utc();
        insert_txn(
            &tx,
            &draft(TxnKind::Opening, vec![(a, None, 500_000)]),
            None,
            &now,
        )
        .unwrap();
        insert_txn(
            &tx,
            &draft(
                TxnKind::Transfer,
                vec![(a, None, -200_000), (b, None, 200_000)],
            ),
            None,
            &now,
        )
        .unwrap();
        assert_eq!(account_balance(&tx, a.id).unwrap(), 300_000);
        assert_eq!(account_balance(&tx, b.id).unwrap(), 200_000);
    }

    #[test]
    fn soft_deleted_txn_excluded_and_restorable() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let a = bank(&tx, "BCA");
        let now = now_utc();
        let id = insert_txn(
            &tx,
            &draft(TxnKind::Opening, vec![(a, None, 500_000)]),
            None,
            &now,
        )
        .unwrap();

        assert!(!restore(&tx, id, &now).unwrap(), "restore of a live txn");
        assert!(soft_delete(&tx, id, &now).unwrap());
        assert_eq!(account_balance(&tx, a.id).unwrap(), 0);
        assert!(!soft_delete(&tx, id, "2099-01-01T00:00:00.000Z").unwrap());
        let deleted_at: String = tx
            .query_row(
                "SELECT deleted_at FROM txn WHERE id = ?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            deleted_at, now,
            "second delete must not overwrite the timestamp"
        );

        assert!(restore(&tx, id, &now).unwrap());
        assert_eq!(account_balance(&tx, a.id).unwrap(), 500_000);

        assert!(!soft_delete(&tx, Uuid::now_v7(), &now).unwrap());
    }

    fn expense_cat(tx: &Transaction, name: &str) -> CategoryRef {
        let id = insert_category(tx, &NewCategory::new(CategoryKind::Expense, name, None)).unwrap();
        get_category_ref(tx, id).unwrap().unwrap().value
    }

    fn dated(mut t: ValidTxn, d: Date) -> ValidTxn {
        let mut draft = t.draft().clone();
        draft.date = d;
        t = ValidTxn::new(draft).unwrap();
        t
    }

    #[test]
    fn replace_keeps_id_and_created_at() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let a = bank(&tx, "BCA");
        let (makan, kopi) = (expense_cat(&tx, "Makan"), expense_cat(&tx, "Kopi"));
        let p1 = crate::payee::insert_payee(&tx, "Indomaret").unwrap();
        let p2 = crate::payee::insert_payee(&tx, "Alfamart").unwrap();
        let t0 = "2026-10-01T00:00:00.000Z";
        let id = insert_txn(
            &tx,
            &draft(TxnKind::Expense, vec![(a, Some(makan), -10_000)]),
            Some(p1),
            t0,
        )
        .unwrap();
        let t1 = "2026-10-02T00:00:00.000Z";
        let new = draft(
            TxnKind::Expense,
            vec![(a, Some(makan), -7_000), (a, Some(kopi), -3_000)],
        );
        assert!(replace_txn(&tx, id, &new, Some(p2), t1).unwrap());
        let rec = get_txn(&tx, id).unwrap().unwrap();
        assert_eq!(rec.id, id);
        assert_eq!(rec.kind, TxnKind::Expense);
        assert_eq!(rec.payee.as_deref(), Some("Alfamart"));
        assert!(!rec.deleted);
        let shape: Vec<_> = rec
            .postings
            .iter()
            .map(|p| (p.category_id, p.amount))
            .collect();
        assert_eq!(
            shape,
            vec![(Some(makan.id), -7_000), (Some(kopi.id), -3_000)]
        );
        let (created, updated): (String, String) = tx
            .query_row(
                "SELECT created_at, updated_at FROM txn WHERE id = ?1",
                [id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((created.as_str(), updated.as_str()), (t0, t1));
        assert_eq!(account_balance(&tx, a.id).unwrap(), -10_000);

        assert!(soft_delete(&tx, id, t1).unwrap());
        assert!(!replace_txn(&tx, id, &new, None, t1).unwrap());
        assert!(get_txn(&tx, id).unwrap().unwrap().deleted);
        assert_eq!(get_txn(&tx, Uuid::now_v7()).unwrap(), None);
    }

    #[test]
    fn last_used_account_ignores_transfer_and_deleted() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let (a, b, c, d) = (
            bank(&tx, "A"),
            bank(&tx, "B"),
            bank(&tx, "C"),
            bank(&tx, "D"),
        );
        let cat = expense_cat(&tx, "Makan");
        let now = now_utc();
        assert_eq!(last_used_account(&tx).unwrap(), None);
        let e = draft(TxnKind::Expense, vec![(a, Some(cat), -1)]);
        insert_txn(&tx, &dated(e, date!(2026 - 10 - 01)), None, &now).unwrap();
        let t = draft(TxnKind::Transfer, vec![(b, None, -1), (c, None, 1)]);
        insert_txn(&tx, &dated(t, date!(2026 - 10 - 02)), None, &now).unwrap();
        let del = draft(TxnKind::Expense, vec![(d, Some(cat), -1)]);
        let del = insert_txn(&tx, &dated(del, date!(2026 - 10 - 03)), None, &now).unwrap();
        soft_delete(&tx, del, &now).unwrap();
        assert_eq!(last_used_account(&tx).unwrap(), Some(a.id));
        tx.execute(
            "UPDATE account SET archived_at = ?1 WHERE id = ?2",
            params![now, a.id.to_string()],
        )
        .unwrap();
        assert_eq!(last_used_account(&tx).unwrap(), None);
    }

    #[test]
    fn last_fee_category_from_latest_transfer() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let (a, b) = (bank(&tx, "A"), bank(&tx, "B"));
        let (f1, f2) = (expense_cat(&tx, "Biaya admin"), expense_cat(&tx, "Bank"));
        let now = now_utc();
        assert_eq!(last_fee_category(&tx).unwrap(), None);
        let t1 = draft(
            TxnKind::Transfer,
            vec![(a, None, -10), (b, None, 10), (a, Some(f1), -1)],
        );
        insert_txn(&tx, &dated(t1, date!(2026 - 10 - 01)), None, &now).unwrap();
        let t2 = draft(
            TxnKind::Transfer,
            vec![(a, None, -10), (b, None, 10), (a, Some(f2), -1)],
        );
        insert_txn(&tx, &dated(t2, date!(2026 - 10 - 05)), None, &now).unwrap();
        // transfer tanpa biaya yang lebih baru tidak mengubah hasil
        let t3 = draft(TxnKind::Transfer, vec![(a, None, -10), (b, None, 10)]);
        insert_txn(&tx, &dated(t3, date!(2026 - 10 - 06)), None, &now).unwrap();
        assert_eq!(last_fee_category(&tx).unwrap(), Some(f2.id));
    }

    #[test]
    fn insert_txn_is_atomic_with_enclosing_transaction() {
        let mut conn = open_in_memory().unwrap();
        let a = {
            let tx = conn.transaction().unwrap();
            let a = bank(&tx, "BCA");
            tx.commit().unwrap();
            a
        };
        {
            let tx = conn.transaction().unwrap();
            insert_txn(
                &tx,
                &draft(TxnKind::Opening, vec![(a, None, 1)]),
                None,
                &now_utc(),
            )
            .unwrap();
            // drop tanpa commit = rollback
        }
        assert_eq!(account_balance(&conn, a.id).unwrap(), 0);
    }
}
