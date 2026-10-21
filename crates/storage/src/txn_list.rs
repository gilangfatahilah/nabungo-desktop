//! Daftar transaksi: filter, paginasi di SQL, saldo berjalan per akun.

use std::collections::HashMap;

use nabungo_core::TxnKind;
use nabungo_core::date::{format_date, parse_date};
use rusqlite::types::Value;
use rusqlite::{Connection, params_from_iter};
use time::Date;
use uuid::Uuid;

use crate::{Result, StorageError, like_contains, parse_uuid};

/// Filter daftar. Semua field opsional digabung dengan AND.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TxnQuery {
    /// Inklusif.
    pub date_from: Option<Date>,
    /// Inklusif.
    pub date_to: Option<Date>,
    pub account_id: Option<Uuid>,
    /// Termasuk child kategori ini.
    pub category_id: Option<Uuid>,
    /// Nama tag (NOCASE).
    pub tag: Option<String>,
    pub payee_id: Option<Uuid>,
    /// Dibandingkan dengan `|amount|` baris, inklusif.
    pub amount_min: Option<i64>,
    pub amount_max: Option<i64>,
    /// Substring literal (tanpa membedakan huruf) pada payee, catatan, memo posting.
    pub search: Option<String>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnListRow {
    pub id: Uuid,
    pub kind: TxnKind,
    pub date: Date,
    pub payee: Option<String>,
    pub note: Option<String>,
    /// Σ posting bertanda; transfer = kaki positif (tanpa biaya).
    pub amount: i64,
    /// Akun utama; transfer = akun asal.
    pub account_id: Uuid,
    pub to_account_id: Option<Uuid>,
    /// Terisi hanya bila tepat satu posting berkategori (non-biaya).
    pub category_id: Option<Uuid>,
    pub split_count: u32,
    /// Biaya admin transfer (positif), 0 bila tidak ada.
    pub fee: i64,
    pub tags: Vec<String>,
    /// Hanya saat difilter per akun: Σ posting akun itu di transaksi ini.
    pub account_delta: Option<i64>,
    /// Hanya saat difilter per akun: saldo akun setelah transaksi ini (riwayat penuh).
    pub running_balance: Option<i64>,
    /// Terisi hanya bila difilter per akun: semua posting akun itu sudah reconciled.
    pub reconciled: Option<bool>,
    pub attachment_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnPage {
    pub rows: Vec<TxnListRow>,
    pub total: u64,
}

/// Nominal baris (lihat [`TxnListRow::amount`]) sebagai ekspresi SQL atas `t`.
const ROW_AMOUNT: &str = "CASE WHEN t.kind = 'transfer'
    THEN (SELECT MAX(amount) FROM posting WHERE txn_id = t.id AND category_id IS NULL)
    ELSE (SELECT SUM(amount) FROM posting WHERE txn_id = t.id) END";

pub fn list_txns(conn: &Connection, q: &TxnQuery) -> Result<TxnPage> {
    let (filter, args) = where_clause(q);
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM txn t LEFT JOIN payee py ON py.id = t.payee_id {filter}"),
        params_from_iter(&args),
        |r| r.get(0),
    )?;

    let mut page_args = args.clone();
    page_args.push(Value::Integer(q.limit.into()));
    page_args.push(Value::Integer(q.offset.into()));
    let mut stmt = conn.prepare(&format!(
        "SELECT t.id, t.kind, t.date, py.name, t.note,
                (SELECT COUNT(*) FROM attachment a WHERE a.txn_id = t.id)
         FROM txn t LEFT JOIN payee py ON py.id = t.payee_id {filter}
         ORDER BY t.date DESC, t.id DESC LIMIT ? OFFSET ?"
    ))?;
    let headers = stmt
        .query_map(params_from_iter(&page_args), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, u32>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if headers.is_empty() {
        return Ok(TxnPage {
            rows: vec![],
            total: total as u64,
        });
    }

    let ids: Vec<Value> = headers.iter().map(|h| Value::Text(h.0.clone())).collect();
    let marks = vec!["?"; ids.len()].join(",");
    let postings = page_postings(conn, &ids, &marks)?;
    let tags = page_tags(conn, &ids, &marks)?;
    let (running, reconciled) = match q.account_id {
        Some(acc) => (
            running_balances(conn, acc, &ids, &marks)?,
            reconciled_flags(conn, acc, &ids, &marks)?,
        ),
        None => (HashMap::new(), HashMap::new()),
    };

    let rows = headers
        .into_iter()
        .map(|(id_s, kind, date, payee, note, attachment_count)| {
            let id = parse_uuid("txn", &id_s)?;
            let kind = TxnKind::parse(&kind)
                .ok_or_else(|| StorageError::Corrupt(format!("txn kind {kind:?}")))?;
            let date =
                parse_date(&date).map_err(|e| StorageError::Corrupt(format!("txn date: {e}")))?;
            let ps = postings.get(&id_s).map(Vec::as_slice).unwrap_or_default();
            let mut row = shape_row(id, kind, date, payee, note, ps)?;
            row.tags = tags.get(&id_s).cloned().unwrap_or_default();
            row.attachment_count = attachment_count;
            if let Some(acc) = q.account_id {
                row.account_delta = Some(ps.iter().filter(|p| p.0 == acc).map(|p| p.2).sum());
                row.running_balance = running.get(&id_s).copied();
                row.reconciled = Some(reconciled.get(&id_s).copied().unwrap_or(false));
            }
            Ok(row)
        })
        .collect::<Result<_>>()?;
    Ok(TxnPage {
        rows,
        total: total as u64,
    })
}

fn where_clause(q: &TxnQuery) -> (String, Vec<Value>) {
    let mut conds = vec![
        "t.deleted_at IS NULL".to_owned(),
        "t.status = 'posted'".to_owned(),
    ];
    let mut args: Vec<Value> = vec![];
    let mut push = |cond: &str, vals: Vec<Value>| {
        conds.push(cond.to_owned());
        args.extend(vals);
    };
    if let Some(d) = q.date_from {
        push("t.date >= ?", vec![Value::Text(format_date(d))]);
    }
    if let Some(d) = q.date_to {
        push("t.date <= ?", vec![Value::Text(format_date(d))]);
    }
    if let Some(a) = q.account_id {
        push(
            "EXISTS (SELECT 1 FROM posting p WHERE p.txn_id = t.id AND p.account_id = ?)",
            vec![Value::Text(a.to_string())],
        );
    }
    if let Some(c) = q.category_id {
        let c = Value::Text(c.to_string());
        push(
            "EXISTS (SELECT 1 FROM posting p JOIN category c ON c.id = p.category_id
                     WHERE p.txn_id = t.id AND (c.id = ? OR c.parent_id = ?))",
            vec![c.clone(), c],
        );
    }
    if let Some(tag) = &q.tag {
        push(
            "EXISTS (SELECT 1 FROM txn_tag tt JOIN tag g ON g.id = tt.tag_id
                     WHERE tt.txn_id = t.id AND g.name = ? COLLATE NOCASE)",
            vec![Value::Text(tag.clone())],
        );
    }
    if let Some(p) = q.payee_id {
        push("t.payee_id = ?", vec![Value::Text(p.to_string())]);
    }
    if let Some(min) = q.amount_min {
        push(
            &format!("ABS({ROW_AMOUNT}) >= ?"),
            vec![Value::Integer(min)],
        );
    }
    if let Some(max) = q.amount_max {
        push(
            &format!("ABS({ROW_AMOUNT}) <= ?"),
            vec![Value::Integer(max)],
        );
    }
    if let Some(search) = q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let pat = Value::Text(like_contains(search));
        push(
            "(py.name LIKE ? ESCAPE '\\' OR t.note LIKE ? ESCAPE '\\'
              OR EXISTS (SELECT 1 FROM posting p WHERE p.txn_id = t.id
                         AND p.memo LIKE ? ESCAPE '\\'))",
            vec![pat.clone(), pat.clone(), pat],
        );
    }
    (format!("WHERE {}", conds.join(" AND ")), args)
}

type PostingRow = (Uuid, Option<Uuid>, i64);

/// Per txn di halaman: apakah semua posting `account_id` sudah reconciled.
fn reconciled_flags(
    conn: &Connection,
    account_id: Uuid,
    ids: &[Value],
    marks: &str,
) -> Result<HashMap<String, bool>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT txn_id, MIN(reconciled) FROM posting
         WHERE account_id = ? AND txn_id IN ({marks}) GROUP BY txn_id"
    ))?;
    let args = std::iter::once(Value::Text(account_id.to_string())).chain(ids.iter().cloned());
    let rows = stmt
        .query_map(params_from_iter(args), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?))
        })?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    Ok(rows)
}

fn page_postings(
    conn: &Connection,
    ids: &[Value],
    marks: &str,
) -> Result<HashMap<String, Vec<PostingRow>>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT txn_id, account_id, category_id, amount FROM posting
         WHERE txn_id IN ({marks}) ORDER BY id"
    ))?;
    let raw = stmt
        .query_map(params_from_iter(ids), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out: HashMap<String, Vec<PostingRow>> = HashMap::new();
    for (txn, acc, cat, amount) in raw {
        let acc = parse_uuid("posting", &acc)?;
        let cat = cat
            .as_deref()
            .map(|c| parse_uuid("posting", c))
            .transpose()?;
        out.entry(txn).or_default().push((acc, cat, amount));
    }
    Ok(out)
}

fn page_tags(
    conn: &Connection,
    ids: &[Value],
    marks: &str,
) -> Result<HashMap<String, Vec<String>>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT tt.txn_id, g.name FROM txn_tag tt JOIN tag g ON g.id = tt.tag_id
         WHERE tt.txn_id IN ({marks}) ORDER BY g.name COLLATE NOCASE"
    ))?;
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    let rows = stmt.query_map(params_from_iter(ids), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (txn, name) = row?;
        out.entry(txn).or_default().push(name);
    }
    Ok(out)
}

/// Saldo akun setelah tiap transaksi halaman ini, dihitung atas seluruh riwayat aktif akun
/// (urut `date, id`), bukan hanya baris yang lolos filter.
fn running_balances(
    conn: &Connection,
    account: Uuid,
    ids: &[Value],
    marks: &str,
) -> Result<HashMap<String, i64>> {
    let mut args = vec![Value::Text(account.to_string())];
    args.extend(ids.iter().cloned());
    let mut stmt = conn.prepare(&format!(
        "WITH d AS (
           SELECT t.id, t.date, SUM(p.amount) AS delta
           FROM posting p JOIN txn t ON t.id = p.txn_id
           WHERE p.account_id = ? AND t.deleted_at IS NULL AND t.status = 'posted'
           GROUP BY t.id
         ), r AS (
           SELECT id, SUM(delta) OVER (ORDER BY date, id ROWS UNBOUNDED PRECEDING) AS bal FROM d
         )
         SELECT id, bal FROM r WHERE id IN ({marks})"
    ))?;
    let rows = stmt
        .query_map(params_from_iter(&args), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

fn shape_row(
    id: Uuid,
    kind: TxnKind,
    date: Date,
    payee: Option<String>,
    note: Option<String>,
    postings: &[PostingRow],
) -> Result<TxnListRow> {
    let first = postings
        .first()
        .ok_or_else(|| StorageError::Corrupt(format!("txn {id} has no postings")))?;
    let mut row = TxnListRow {
        id,
        kind,
        date,
        payee,
        note,
        amount: 0,
        account_id: first.0,
        to_account_id: None,
        category_id: None,
        split_count: 0,
        fee: 0,
        tags: vec![],
        account_delta: None,
        running_balance: None,
        reconciled: None,
        attachment_count: 0,
    };
    if kind == TxnKind::Transfer {
        for &(acc, cat, amount) in postings {
            match cat {
                Some(_) => row.fee -= amount,
                None if amount < 0 => row.account_id = acc,
                None => {
                    row.to_account_id = Some(acc);
                    row.amount = amount;
                }
            }
        }
    } else {
        let cats: Vec<Uuid> = postings.iter().filter_map(|p| p.1).collect();
        row.amount = postings.iter().map(|p| p.2).sum();
        row.split_count = cats.len() as u32;
        if let [only] = cats[..] {
            row.category_id = Some(only);
        }
    }
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{NewAccount, get_account_ref, insert_account};
    use crate::category::{NewCategory, get_category_ref, insert_category};
    use crate::payee::insert_payee;
    use crate::tag::{find_or_insert_tag, set_txn_tags};
    use crate::txn::{insert_txn, soft_delete};
    use crate::{now_utc, open_in_memory};
    use nabungo_core::{
        AccountKind, AccountRef, CategoryKind, CategoryRef, CurrencyCode, PostingDraft, TxnDraft,
        ValidTxn,
    };
    use rusqlite::Transaction;
    use time::macros::date;

    struct Fx {
        conn: Connection,
        a: Uuid,
        b: Uuid,
        makan: Uuid,
        kopi: Uuid,
        biaya: Uuid,
        kopi_txn: Uuid,
        income_txn: Uuid,
        transfer_txn: Uuid,
        split_txn: Uuid,
        opening_txn: Uuid,
    }

    type P = (AccountRef, Option<CategoryRef>, i64);

    fn add(
        tx: &Transaction,
        kind: TxnKind,
        d: Date,
        note: Option<&str>,
        payee: Option<Uuid>,
        postings: Vec<P>,
    ) -> Uuid {
        let v = ValidTxn::new(TxnDraft {
            kind,
            date: d,
            note: note.map(str::to_owned),
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
        insert_txn(tx, &v, payee, &now_utc()).unwrap()
    }

    fn acc(tx: &Transaction, name: &str) -> AccountRef {
        let new = NewAccount::new(name, AccountKind::Bank, CurrencyCode::IDR);
        let id = insert_account(tx, &new, &now_utc()).unwrap();
        get_account_ref(tx, id).unwrap().unwrap().value
    }

    fn cat(tx: &Transaction, kind: CategoryKind, name: &str, parent: Option<Uuid>) -> CategoryRef {
        let id = insert_category(tx, &NewCategory::new(kind, name, parent)).unwrap();
        get_category_ref(tx, id).unwrap().unwrap().value
    }

    fn fixture() -> Fx {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let (a, b) = (acc(&tx, "A"), acc(&tx, "B"));
        let makan = cat(&tx, CategoryKind::Expense, "Makan", None);
        let kopi = cat(&tx, CategoryKind::Expense, "Kopi", Some(makan.id));
        let gaji = cat(&tx, CategoryKind::Income, "Gaji", None);
        let biaya = cat(&tx, CategoryKind::Expense, "Biaya", None);
        let kenangan = insert_payee(&tx, "Kopi Kenangan").unwrap();
        use TxnKind::*;
        let opening_txn = add(
            &tx,
            Opening,
            date!(2026 - 10 - 01),
            None,
            None,
            vec![(a, None, 1_000_000)],
        );
        let kopi_txn = add(
            &tx,
            Expense,
            date!(2026 - 10 - 02),
            None,
            Some(kenangan),
            vec![(a, Some(kopi), -25_000)],
        );
        let kantor = find_or_insert_tag(&tx, "kantor").unwrap();
        set_txn_tags(&tx, kopi_txn, &[kantor]).unwrap();
        let income_txn = add(
            &tx,
            Income,
            date!(2026 - 10 - 02),
            None,
            None,
            vec![(a, Some(gaji), 5_000_000)],
        );
        let transfer_txn = add(
            &tx,
            Transfer,
            date!(2026 - 10 - 03),
            None,
            None,
            vec![
                (a, None, -200_000),
                (b, None, 200_000),
                (a, Some(biaya), -6_500),
            ],
        );
        let split_txn = add(
            &tx,
            Expense,
            date!(2026 - 10 - 04),
            Some("makan 50% diskon"),
            None,
            vec![(a, Some(makan), -30_000), (a, Some(kopi), -20_000)],
        );
        let del = add(
            &tx,
            Expense,
            date!(2026 - 10 - 05),
            None,
            None,
            vec![(a, Some(makan), -99)],
        );
        soft_delete(&tx, del, &now_utc()).unwrap();
        tx.commit().unwrap();
        Fx {
            conn,
            a: a.id,
            b: b.id,
            makan: makan.id,
            kopi: kopi.id,
            biaya: biaya.id,
            kopi_txn,
            income_txn,
            transfer_txn,
            split_txn,
            opening_txn,
        }
    }

    fn q() -> TxnQuery {
        TxnQuery {
            limit: 100,
            ..TxnQuery::default()
        }
    }

    fn ids(page: &TxnPage) -> Vec<Uuid> {
        page.rows.iter().map(|r| r.id).collect()
    }

    fn row(page: &TxnPage, id: Uuid) -> &TxnListRow {
        page.rows.iter().find(|r| r.id == id).unwrap()
    }

    #[test]
    fn attachment_count_on_rows() {
        let mut f = fixture();
        let tx = f.conn.transaction().unwrap();
        for _ in 0..2 {
            crate::attachment::insert_attachment(
                &tx,
                &crate::attachment::AttachmentRecord {
                    id: Uuid::now_v7(),
                    txn_id: f.kopi_txn,
                    file_name: "s.png".into(),
                    sha256: "aa".into(),
                    mime: "image/png".into(),
                    size_bytes: 1,
                    created_at: "x".into(),
                },
            )
            .unwrap();
        }
        tx.commit().unwrap();
        let page = list_txns(&f.conn, &q()).unwrap();
        assert_eq!(row(&page, f.kopi_txn).attachment_count, 2);
        assert_eq!(row(&page, f.income_txn).attachment_count, 0);
    }

    #[test]
    fn default_order_and_total() {
        let f = fixture();
        let page = list_txns(&f.conn, &q()).unwrap();
        assert_eq!(page.total, 5);
        assert_eq!(
            ids(&page),
            vec![
                f.split_txn,
                f.transfer_txn,
                f.income_txn,
                f.kopi_txn,
                f.opening_txn
            ]
        );
        let kopi = row(&page, f.kopi_txn);
        assert_eq!(kopi.payee.as_deref(), Some("Kopi Kenangan"));
        assert_eq!(kopi.tags, vec!["kantor"]);
        assert_eq!(kopi.category_id, Some(f.kopi));
        assert_eq!(kopi.split_count, 1);
        assert_eq!(kopi.amount, -25_000);
        assert_eq!(kopi.account_delta, None);
        assert_eq!(kopi.running_balance, None);
        assert_eq!(row(&page, f.opening_txn).kind, TxnKind::Opening);
    }

    #[test]
    fn transfer_row_shape() {
        let f = fixture();
        let page = list_txns(&f.conn, &q()).unwrap();
        let t = row(&page, f.transfer_txn);
        assert_eq!(t.amount, 200_000);
        assert_eq!(t.account_id, f.a);
        assert_eq!(t.to_account_id, Some(f.b));
        assert_eq!(t.fee, 6_500);
        assert_eq!(t.category_id, None);
        assert_eq!(t.split_count, 0);
        let _ = f.biaya;
    }

    #[test]
    fn split_row_shape() {
        let f = fixture();
        let page = list_txns(&f.conn, &q()).unwrap();
        let s = row(&page, f.split_txn);
        assert_eq!(s.amount, -50_000);
        assert_eq!(s.category_id, None);
        assert_eq!(s.split_count, 2);
        assert_eq!(s.note.as_deref(), Some("makan 50% diskon"));
    }

    #[test]
    fn category_filter_includes_children() {
        let f = fixture();
        for c in [f.makan, f.kopi] {
            let page = list_txns(
                &f.conn,
                &TxnQuery {
                    category_id: Some(c),
                    ..q()
                },
            )
            .unwrap();
            assert_eq!(ids(&page), vec![f.split_txn, f.kopi_txn]);
            assert_eq!(page.total, 2);
        }
    }

    #[test]
    fn running_balance_full_history() {
        let f = fixture();
        let page = list_txns(
            &f.conn,
            &TxnQuery {
                account_id: Some(f.a),
                ..q()
            },
        )
        .unwrap();
        assert_eq!(page.total, 5);
        assert_eq!(row(&page, f.split_txn).running_balance, Some(5_718_500));
        let t = row(&page, f.transfer_txn);
        assert_eq!(t.account_delta, Some(-206_500));
        assert_eq!(t.running_balance, Some(5_768_500));
        assert_eq!(row(&page, f.opening_txn).running_balance, Some(1_000_000));
        let pb = list_txns(
            &f.conn,
            &TxnQuery {
                account_id: Some(f.b),
                ..q()
            },
        )
        .unwrap();
        assert_eq!(ids(&pb), vec![f.transfer_txn]);
        assert_eq!(pb.rows[0].account_delta, Some(200_000));
        assert_eq!(pb.rows[0].running_balance, Some(200_000));
    }

    #[test]
    fn reconciled_only_with_account_filter() {
        let mut f = fixture();
        let tx = f.conn.transaction().unwrap();
        crate::txn::mark_reconciled(&tx, f.a, date!(2026 - 10 - 02)).unwrap();
        tx.commit().unwrap();
        let by_a = list_txns(
            &f.conn,
            &TxnQuery {
                account_id: Some(f.a),
                ..q()
            },
        )
        .unwrap();
        assert_eq!(row(&by_a, f.opening_txn).reconciled, Some(true));
        assert_eq!(row(&by_a, f.kopi_txn).reconciled, Some(true));
        assert_eq!(row(&by_a, f.transfer_txn).reconciled, Some(false));
        let by_b = list_txns(
            &f.conn,
            &TxnQuery {
                account_id: Some(f.b),
                ..q()
            },
        )
        .unwrap();
        assert_eq!(by_b.rows[0].reconciled, Some(false));
        let all = list_txns(&f.conn, &q()).unwrap();
        assert!(all.rows.iter().all(|r| r.reconciled.is_none()));
    }

    #[test]
    fn running_balance_with_search_filter() {
        let f = fixture();
        let page = list_txns(
            &f.conn,
            &TxnQuery {
                account_id: Some(f.a),
                search: Some("diskon".into()),
                ..q()
            },
        )
        .unwrap();
        assert_eq!(ids(&page), vec![f.split_txn]);
        assert_eq!(page.rows[0].running_balance, Some(5_718_500));
    }

    #[test]
    fn search_literal_percent_and_case() {
        let f = fixture();
        let search = |s: &str| {
            list_txns(
                &f.conn,
                &TxnQuery {
                    search: Some(s.into()),
                    ..q()
                },
            )
            .unwrap()
        };
        assert_eq!(ids(&search("50%")), vec![f.split_txn]);
        assert!(search("50_").rows.is_empty());
        assert_eq!(ids(&search("KENANGAN")), vec![f.kopi_txn]);
    }

    #[test]
    fn amount_range_uses_abs() {
        let f = fixture();
        let page = list_txns(
            &f.conn,
            &TxnQuery {
                amount_min: Some(20_000),
                amount_max: Some(60_000),
                ..q()
            },
        )
        .unwrap();
        assert_eq!(ids(&page), vec![f.split_txn, f.kopi_txn]);
    }

    #[test]
    fn tag_and_payee_and_date_filters() {
        let f = fixture();
        let tag = list_txns(
            &f.conn,
            &TxnQuery {
                tag: Some("KANTOR".into()),
                ..q()
            },
        )
        .unwrap();
        assert_eq!(ids(&tag), vec![f.kopi_txn]);
        let payee: Uuid = f
            .conn
            .query_row("SELECT id FROM payee", [], |r| r.get::<_, String>(0))
            .unwrap()
            .parse()
            .unwrap();
        let by_payee = list_txns(
            &f.conn,
            &TxnQuery {
                payee_id: Some(payee),
                ..q()
            },
        )
        .unwrap();
        assert_eq!(ids(&by_payee), vec![f.kopi_txn]);
        let day = list_txns(
            &f.conn,
            &TxnQuery {
                date_from: Some(date!(2026 - 10 - 02)),
                date_to: Some(date!(2026 - 10 - 02)),
                ..q()
            },
        )
        .unwrap();
        assert_eq!(ids(&day), vec![f.income_txn, f.kopi_txn]);
    }

    #[test]
    fn pagination() {
        let f = fixture();
        let page = list_txns(
            &f.conn,
            &TxnQuery {
                limit: 2,
                offset: 2,
                ..q()
            },
        )
        .unwrap();
        assert_eq!(ids(&page), vec![f.income_txn, f.kopi_txn]);
        assert_eq!(page.total, 5);
    }

    /// `cargo test --release -p nabungo-storage -- --ignored perf_`
    #[test]
    #[ignore]
    fn perf_50k_first_page() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let accounts = [acc(&tx, "A"), acc(&tx, "B"), acc(&tx, "C")];
        let c = cat(&tx, CategoryKind::Expense, "Makan", None);
        let start = date!(2020 - 01 - 01);
        for i in 0..50_000i64 {
            let d = start + time::Duration::days(i % 2_000);
            let a = accounts[(i % 3) as usize];
            add(
                &tx,
                TxnKind::Expense,
                d,
                Some("catatan"),
                None,
                vec![(a, Some(c), -(i % 997 + 1))],
            );
        }
        tx.commit().unwrap();
        let t = std::time::Instant::now();
        let page = list_txns(
            &conn,
            &TxnQuery {
                account_id: Some(accounts[0].id),
                ..q()
            },
        )
        .unwrap();
        let elapsed = t.elapsed();
        assert_eq!(page.rows.len(), 100);
        let t2 = std::time::Instant::now();
        list_txns(&conn, &q()).unwrap();
        let unfiltered = t2.elapsed();
        eprintln!("list_txns 50k: account filter {elapsed:?}, unfiltered {unfiltered:?}");
        assert!(elapsed.as_millis() < 200, "{elapsed:?}");
        assert!(unfiltered.as_millis() < 200, "{unfiltered:?}");
    }
}
