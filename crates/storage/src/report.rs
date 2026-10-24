//! Agregasi baca untuk budget, alokasi, dan laporan. Semua angka diturunkan dari
//! `posting` milik transaksi aktif (`deleted_at IS NULL`) yang sudah `posted`.

use std::collections::BTreeMap;

use nabungo_core::date::{format_date, parse_date};
use rusqlite::{Connection, params};
use time::Date;
use uuid::Uuid;

use crate::{Result, StorageError, parse_uuid};

/// Σ `posting.amount` per kategori untuk transaksi dengan `from <= date < to`.
/// Posting tanpa kategori (transfer, opening, valuation) tidak ikut.
pub fn category_totals(
    conn: &Connection,
    from: Date,
    to_exclusive: Date,
) -> Result<Vec<(Uuid, i64)>> {
    let mut stmt = conn.prepare_cached(
        "SELECT p.category_id, SUM(p.amount)
         FROM posting p JOIN txn t ON t.id = p.txn_id
         WHERE p.category_id IS NOT NULL AND t.deleted_at IS NULL AND t.status = 'posted'
           AND t.date >= ?1 AND t.date < ?2
         GROUP BY p.category_id ORDER BY p.category_id",
    )?;
    collect_pairs(
        stmt.query(params![format_date(from), format_date(to_exclusive)])?,
        "category",
    )
}

/// [`category_totals`] hanya untuk `categories`. `CROSS JOIN` memaksa SQLite mulai dari
/// `idx_posting_cat`: rentang tanggal panjang (rantai rollover) tidak lagi memindai semua
/// transaksi di rentang itu.
pub fn category_totals_in(
    conn: &Connection,
    from: Date,
    to_exclusive: Date,
    categories: &[Uuid],
) -> Result<Vec<(Uuid, i64)>> {
    if categories.is_empty() {
        return Ok(Vec::new());
    }
    let marks = vec!["?"; categories.len()].join(", ");
    let mut stmt = conn.prepare(&format!(
        "SELECT p.category_id, SUM(p.amount)
         FROM posting p CROSS JOIN txn t ON t.id = p.txn_id
         WHERE p.category_id IN ({marks}) AND t.deleted_at IS NULL AND t.status = 'posted'
           AND t.date >= ? AND t.date < ?
         GROUP BY p.category_id ORDER BY p.category_id"
    ))?;
    let args = categories
        .iter()
        .map(|c| c.to_string())
        .chain([format_date(from), format_date(to_exclusive)]);
    collect_pairs(stmt.query(rusqlite::params_from_iter(args))?, "category")
}

/// Saldo per akun pada akhir `date` (inklusif). Akun tanpa posting tidak muncul.
pub fn balances_as_of(conn: &Connection, date: Date) -> Result<Vec<(Uuid, i64)>> {
    let mut stmt = conn.prepare_cached(
        "SELECT p.account_id, SUM(p.amount)
         FROM posting p JOIN txn t ON t.id = p.txn_id
         WHERE t.deleted_at IS NULL AND t.status = 'posted' AND t.date <= ?1
         GROUP BY p.account_id ORDER BY p.account_id",
    )?;
    collect_pairs(stmt.query([format_date(date)])?, "account")
}

/// [`balances_as_of`] untuk banyak tanggal (urut naik) sekaligus: satu scan posting,
/// lalu akumulasi per akun. `hasil[i] == balances_as_of(conn, dates[i])`.
pub fn balances_at_dates(conn: &Connection, dates: &[Date]) -> Result<Vec<Vec<(Uuid, i64)>>> {
    let Some(last) = dates.last() else {
        return Ok(Vec::new());
    };
    let keys: Vec<String> = dates.iter().map(|d| format_date(*d)).collect();
    let mut stmt = conn.prepare_cached(
        "SELECT p.account_id, t.date, SUM(p.amount)
         FROM posting p JOIN txn t ON t.id = p.txn_id
         WHERE t.deleted_at IS NULL AND t.status = 'posted' AND t.date <= ?1
         GROUP BY p.account_id, t.date",
    )?;
    // Per tanggal: Σ posting yang jatuh di (dates[i-1], dates[i]], urut id akun (teks) seperti
    // `ORDER BY p.account_id` di `balances_as_of`.
    let mut buckets = vec![BTreeMap::<String, i64>::new(); dates.len()];
    let mut rows = stmt.query([format_date(*last)])?;
    while let Some(r) = rows.next()? {
        let date: String = r.get(1)?;
        let i = keys.partition_point(|k| k.as_str() < date.as_str());
        *buckets[i].entry(r.get(0)?).or_default() += r.get::<_, i64>(2)?;
    }
    let mut running = BTreeMap::<String, i64>::new();
    buckets
        .into_iter()
        .map(|bucket| {
            for (id, sum) in bucket {
                *running.entry(id).or_default() += sum;
            }
            running
                .iter()
                .map(|(id, sum)| Ok((parse_uuid("account", id)?, *sum)))
                .collect()
        })
        .collect()
}

pub fn account_balance_as_of(conn: &Connection, account_id: Uuid, date: Date) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(p.amount), 0)
         FROM posting p JOIN txn t ON t.id = p.txn_id
         WHERE p.account_id = ?1 AND t.deleted_at IS NULL AND t.status = 'posted'
           AND t.date <= ?2",
        params![account_id.to_string(), format_date(date)],
        |r| r.get(0),
    )?)
}

/// Valuation aktif untuk satu akun: `(txn_id, tanggal, selisih)`, terbaru dulu.
pub fn valuations(conn: &Connection, account_id: Uuid) -> Result<Vec<(Uuid, Date, i64)>> {
    let mut stmt = conn.prepare_cached(
        "SELECT t.id, t.date, SUM(p.amount)
         FROM posting p JOIN txn t ON t.id = p.txn_id
         WHERE p.account_id = ?1 AND t.kind = 'valuation'
           AND t.deleted_at IS NULL AND t.status = 'posted'
         GROUP BY t.id ORDER BY t.date DESC, t.id DESC",
    )?;
    let mut rows = stmt.query([account_id.to_string()])?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        let id: String = r.get(0)?;
        let date: String = r.get(1)?;
        let date =
            parse_date(&date).map_err(|e| StorageError::Corrupt(format!("txn date: {e}")))?;
        out.push((parse_uuid("txn", &id)?, date, r.get(2)?));
    }
    Ok(out)
}

fn collect_pairs(mut rows: rusqlite::Rows<'_>, table: &str) -> Result<Vec<(Uuid, i64)>> {
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        let id: String = r.get(0)?;
        out.push((parse_uuid(table, &id)?, r.get(1)?));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{NewAccount, get_account_ref, insert_account};
    use crate::category::{NewCategory, get_category_ref, insert_category};
    use crate::txn::{insert_txn, soft_delete};
    use crate::{now_utc, open_in_memory};
    use nabungo_core::{
        AccountKind, AccountRef, CategoryKind, CategoryRef, CurrencyCode, PostingDraft, TxnDraft,
        TxnKind, ValidTxn,
    };
    use rusqlite::Transaction;
    use time::macros::date;

    fn bank(tx: &Transaction, name: &str) -> AccountRef {
        let new = NewAccount::new(name, AccountKind::Bank, CurrencyCode::IDR);
        let id = insert_account(tx, &new, &now_utc()).unwrap();
        get_account_ref(tx, id).unwrap().unwrap().value
    }

    fn expense_cat(tx: &Transaction, name: &str) -> CategoryRef {
        let id = insert_category(tx, &NewCategory::new(CategoryKind::Expense, name, None)).unwrap();
        get_category_ref(tx, id).unwrap().unwrap().value
    }

    fn add(
        tx: &Transaction,
        kind: TxnKind,
        date: Date,
        postings: Vec<(AccountRef, Option<CategoryRef>, i64)>,
    ) -> Uuid {
        let t = ValidTxn::new(TxnDraft {
            kind,
            date,
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
        .unwrap();
        insert_txn(tx, &t, None, &now_utc()).unwrap()
    }

    #[test]
    fn category_totals_ignores_deleted_and_transfers() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let (bca, dompet) = (bank(&tx, "BCA"), bank(&tx, "Dompet"));
        let kopi = expense_cat(&tx, "Kopi");
        add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 10 - 02),
            vec![(bca, Some(kopi), -25_000)],
        );
        add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 10 - 03),
            vec![(bca, Some(kopi), 5_000)],
        );
        add(
            &tx,
            TxnKind::Transfer,
            date!(2026 - 10 - 04),
            vec![(bca, None, -100_000), (dompet, None, 100_000)],
        );
        let gone = add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 10 - 05),
            vec![(bca, Some(kopi), -9_000)],
        );
        soft_delete(&tx, gone, &now_utc()).unwrap();
        add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 11 - 01),
            vec![(bca, Some(kopi), -7_000)],
        );
        add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 10 - 01),
            vec![(bca, Some(kopi), -1)],
        );

        let totals = category_totals(&tx, date!(2026 - 10 - 01), date!(2026 - 11 - 01)).unwrap();
        assert_eq!(totals, vec![(kopi.id, -20_001)]);
    }

    #[test]
    fn category_totals_in_filters_categories() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let bca = bank(&tx, "BCA");
        let (kopi, teh, roti) = (
            expense_cat(&tx, "Kopi"),
            expense_cat(&tx, "Teh"),
            expense_cat(&tx, "Roti"),
        );
        add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 10 - 02),
            vec![(bca, Some(kopi), -25_000), (bca, Some(teh), -5_000)],
        );
        add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 10 - 03),
            vec![(bca, Some(roti), -9_000)],
        );
        let gone = add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 10 - 04),
            vec![(bca, Some(kopi), -1_000)],
        );
        soft_delete(&tx, gone, &now_utc()).unwrap();
        add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 11 - 01),
            vec![(bca, Some(teh), -7_000)],
        );
        let (from, to) = (date!(2026 - 10 - 01), date!(2026 - 11 - 01));
        let mut ids = [kopi.id, teh.id];
        ids.sort();
        let mut expected: Vec<_> = category_totals(&tx, from, to)
            .unwrap()
            .into_iter()
            .filter(|(id, _)| ids.contains(id))
            .collect();
        expected.sort();
        assert_eq!(category_totals_in(&tx, from, to, &ids).unwrap(), expected);
        assert_eq!(category_totals_in(&tx, from, to, &[]).unwrap(), vec![]);
    }

    #[test]
    fn balances_as_of_is_inclusive_and_skips_deleted() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let bca = bank(&tx, "BCA");
        bank(&tx, "Kosong");
        let kopi = expense_cat(&tx, "Kopi");
        add(
            &tx,
            TxnKind::Opening,
            date!(2026 - 10 - 01),
            vec![(bca, None, 1_000_000)],
        );
        add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 10 - 05),
            vec![(bca, Some(kopi), -25_000)],
        );
        let gone = add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 10 - 06),
            vec![(bca, Some(kopi), -1_000)],
        );
        soft_delete(&tx, gone, &now_utc()).unwrap();

        assert_eq!(
            account_balance_as_of(&tx, bca.id, date!(2026 - 10 - 04)).unwrap(),
            1_000_000
        );
        assert_eq!(
            account_balance_as_of(&tx, bca.id, date!(2026 - 10 - 05)).unwrap(),
            975_000
        );
        assert_eq!(
            account_balance_as_of(&tx, bca.id, date!(2026 - 09 - 30)).unwrap(),
            0
        );
        assert_eq!(
            balances_as_of(&tx, date!(2026 - 10 - 31)).unwrap(),
            vec![(bca.id, 975_000)]
        );
    }

    #[test]
    fn period_balances_match_balances_as_of() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let (bca, dompet) = (bank(&tx, "BCA"), bank(&tx, "Dompet"));
        bank(&tx, "Kosong");
        let kopi = expense_cat(&tx, "Kopi");
        add(
            &tx,
            TxnKind::Opening,
            date!(2026 - 08 - 01),
            vec![(bca, None, 1_000_000)],
        );
        add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 08 - 24),
            vec![(bca, Some(kopi), -25_000)],
        );
        // Dompet baru muncul di periode ketiga; saldo 0 tetap muncul sebagai baris.
        add(
            &tx,
            TxnKind::Transfer,
            date!(2026 - 10 - 24),
            vec![(bca, None, -50_000), (dompet, None, 50_000)],
        );
        add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 10 - 24),
            vec![(dompet, Some(kopi), -50_000)],
        );
        let gone = add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 08 - 10),
            vec![(bca, Some(kopi), -1_000)],
        );
        soft_delete(&tx, gone, &now_utc()).unwrap();
        // Setelah tanggal terakhir: tidak boleh ikut.
        add(
            &tx,
            TxnKind::Expense,
            date!(2026 - 12 - 01),
            vec![(bca, Some(kopi), -7_000)],
        );

        let dates = [
            date!(2026 - 07 - 24),
            date!(2026 - 08 - 24),
            date!(2026 - 09 - 24),
            date!(2026 - 10 - 24),
            date!(2026 - 11 - 24),
        ];
        let got = balances_at_dates(&tx, &dates).unwrap();
        assert_eq!(got.len(), dates.len());
        for (i, d) in dates.iter().enumerate() {
            assert_eq!(got[i], balances_as_of(&tx, *d).unwrap(), "{d}");
        }
        assert!(got[0].is_empty());
        assert_eq!(got[3], vec![(bca.id, 925_000), (dompet.id, 0)]);
    }
}
