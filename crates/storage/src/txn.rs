use nabungo_core::TxnDraft;
use nabungo_core::date::format_date;
use rusqlite::{Connection, Transaction, params};
use uuid::Uuid;

use crate::Result;

/// Simpan header + postings. Pemanggil WAJIB sudah menjalankan `nabungo_core::validate`.
pub fn insert_txn(tx: &Transaction, draft: &TxnDraft, now: &str) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO txn (id, kind, date, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![
            id.to_string(),
            draft.kind.as_str(),
            format_date(draft.date),
            draft.note,
            now
        ],
    )?;
    let mut stmt = tx.prepare_cached(
        "INSERT INTO posting (id, txn_id, account_id, category_id, amount, memo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for p in &draft.postings {
        stmt.execute(params![
            Uuid::now_v7().to_string(),
            id.to_string(),
            p.account.id.to_string(),
            p.category.map(|c| c.id.to_string()),
            p.amount,
            p.memo,
        ])?;
    }
    Ok(id)
}

/// Soft delete (`Some(ts)`) atau restore (`None`). Mengembalikan `false` bila ID tidak ada.
pub fn set_deleted(
    tx: &Transaction,
    id: Uuid,
    deleted_at: Option<&str>,
    now: &str,
) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE txn SET deleted_at = ?2, updated_at = ?3 WHERE id = ?1",
        params![id.to_string(), deleted_at, now],
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
    use crate::category::{get_category_ref, insert_category};
    use crate::{now_utc, open_in_memory};
    use nabungo_core::{
        AccountKind, AccountRef, CategoryKind, CategoryRef, CurrencyCode, PostingDraft, TxnKind,
    };
    use time::macros::date;

    fn bank(tx: &Transaction, name: &str) -> AccountRef {
        let new = NewAccount {
            name: name.into(),
            kind: AccountKind::Bank,
            currency: CurrencyCode::IDR,
        };
        let id = insert_account(tx, &new, &now_utc()).unwrap();
        get_account_ref(tx, id).unwrap().unwrap()
    }

    fn draft(kind: TxnKind, postings: Vec<(AccountRef, Option<CategoryRef>, i64)>) -> TxnDraft {
        TxnDraft {
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
        }
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
        let parent = insert_category(&tx, CategoryKind::Expense, "Makan & Minum", None).unwrap();
        let child = insert_category(&tx, CategoryKind::Expense, "Kopi", Some(parent)).unwrap();
        let r = get_category_ref(&tx, child).unwrap().unwrap();
        assert_eq!(r.kind, CategoryKind::Expense);
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
            &now,
        )
        .unwrap();
        insert_txn(
            &tx,
            &draft(
                TxnKind::Transfer,
                vec![(a, None, -200_000), (b, None, 200_000)],
            ),
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
            &now,
        )
        .unwrap();

        assert!(set_deleted(&tx, id, Some(&now), &now).unwrap());
        assert_eq!(account_balance(&tx, a.id).unwrap(), 0);

        assert!(set_deleted(&tx, id, None, &now).unwrap());
        assert_eq!(account_balance(&tx, a.id).unwrap(), 500_000);

        assert!(!set_deleted(&tx, Uuid::now_v7(), Some(&now), &now).unwrap());
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
                &now_utc(),
            )
            .unwrap();
            // drop tanpa commit = rollback
        }
        assert_eq!(account_balance(&conn, a.id).unwrap(), 0);
    }
}
