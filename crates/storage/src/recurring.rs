//! Jadwal transaksi berulang (TRX-06). Kejadian tidak disimpan: `next_date` adalah kejadian
//! tertua yang belum ditangani, dan transaksi hasil konfirmasi ditautkan lewat
//! `txn.recurring_rule_id`.

use nabungo_core::date::{format_date, parse_date};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};
use time::Date;
use uuid::Uuid;

use crate::{Result, StorageError, parse_uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleRecord {
    pub id: Uuid,
    /// JSON template transaksi; formatnya milik `nabungo-app`.
    pub template: String,
    pub rrule: String,
    pub next_date: Date,
    pub auto_post: bool,
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct RuleWrite<'a> {
    pub template: &'a str,
    pub rrule: &'a str,
    pub next_date: Date,
    pub auto_post: bool,
}

pub fn insert_rule(tx: &Transaction, w: &RuleWrite) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO recurring_rule (id, template, rrule, next_date, auto_post)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id.to_string(),
            w.template,
            w.rrule,
            format_date(w.next_date),
            w.auto_post
        ],
    )?;
    Ok(id)
}

/// Ganti isi jadwal dan aktifkan lagi (`ended_at = NULL`). `false` bila ID tidak ada.
pub fn update_rule(tx: &Transaction, id: Uuid, w: &RuleWrite) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE recurring_rule
         SET template = ?2, rrule = ?3, next_date = ?4, auto_post = ?5, ended_at = NULL
         WHERE id = ?1",
        params![
            id.to_string(),
            w.template,
            w.rrule,
            format_date(w.next_date),
            w.auto_post
        ],
    )?;
    Ok(changed == 1)
}

/// Majukan ke kejadian berikutnya; `next = None` mengakhiri jadwal (`next_date` tetap).
pub fn advance_rule(tx: &Transaction, id: Uuid, next: Option<Date>, now: &str) -> Result<()> {
    match next {
        Some(d) => tx.execute(
            "UPDATE recurring_rule SET next_date = ?2 WHERE id = ?1",
            params![id.to_string(), format_date(d)],
        )?,
        None => tx.execute(
            "UPDATE recurring_rule SET ended_at = ?2 WHERE id = ?1",
            params![id.to_string(), now],
        )?,
    };
    Ok(())
}

/// Hapus jadwal; transaksi yang sudah tercatat tetap ada, tautannya dilepas.
pub fn delete_rule(tx: &Transaction, id: Uuid) -> Result<bool> {
    tx.execute(
        "UPDATE txn SET recurring_rule_id = NULL WHERE recurring_rule_id = ?1",
        [id.to_string()],
    )?;
    let changed = tx.execute("DELETE FROM recurring_rule WHERE id = ?1", [id.to_string()])?;
    Ok(changed == 1)
}

pub fn link_txn(tx: &Transaction, txn_id: Uuid, rule_id: Uuid) -> Result<()> {
    tx.execute(
        "UPDATE txn SET recurring_rule_id = ?2 WHERE id = ?1",
        params![txn_id.to_string(), rule_id.to_string()],
    )?;
    Ok(())
}

const COLUMNS: &str = "id, template, rrule, next_date, auto_post, ended_at";

fn read_rule(r: &Row) -> rusqlite::Result<(String, String, String, String, bool, Option<String>)> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
    ))
}

fn to_record(
    (id, template, rrule, next_date, auto_post, ended_at): (
        String,
        String,
        String,
        String,
        bool,
        Option<String>,
    ),
) -> Result<RuleRecord> {
    let next_date = parse_date(&next_date).map_err(|e| {
        StorageError::Corrupt(format!("recurring_rule next_date {next_date:?}: {e}"))
    })?;
    Ok(RuleRecord {
        id: parse_uuid("recurring_rule", &id)?,
        template,
        rrule,
        next_date,
        auto_post,
        ended_at,
    })
}

pub fn get_rule(conn: &Connection, id: Uuid) -> Result<Option<RuleRecord>> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM recurring_rule WHERE id = ?1"),
        [id.to_string()],
        read_rule,
    )
    .optional()?
    .map(to_record)
    .transpose()
}

/// Jadwal aktif dulu, lalu menurut `next_date` dan ID.
pub fn list_rules(conn: &Connection) -> Result<Vec<RuleRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM recurring_rule
         ORDER BY ended_at IS NOT NULL, next_date, id"
    ))?;
    let rows = stmt.query_map([], read_rule)?;
    rows.map(|r| to_record(r?)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{NewAccount, get_account_ref, insert_account};
    use crate::txn::{get_txn, insert_txn};
    use crate::{now_utc, open_in_memory};
    use nabungo_core::{AccountKind, CurrencyCode, PostingDraft, TxnDraft, TxnKind, ValidTxn};
    use time::macros::date;

    fn write(next_date: Date) -> RuleWrite<'static> {
        RuleWrite {
            template: r#"{"v":1}"#,
            rrule: "FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=25",
            next_date,
            auto_post: true,
        }
    }

    #[test]
    fn insert_get_round_trip() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let id = insert_rule(&tx, &write(date!(2026 - 10 - 25))).unwrap();
        tx.commit().unwrap();
        let rec = get_rule(&conn, id).unwrap().unwrap();
        assert_eq!(rec.id, id);
        assert_eq!(rec.template, r#"{"v":1}"#);
        assert_eq!(rec.rrule, "FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=25");
        assert_eq!(rec.next_date, date!(2026 - 10 - 25));
        assert!(rec.auto_post);
        assert_eq!(rec.ended_at, None);
        assert_eq!(get_rule(&conn, Uuid::now_v7()).unwrap(), None);
    }

    #[test]
    fn advance_and_end() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let id = insert_rule(&tx, &write(date!(2026 - 09 - 25))).unwrap();
        advance_rule(&tx, id, Some(date!(2026 - 10 - 25)), &now_utc()).unwrap();
        assert_eq!(
            get_rule(&tx, id).unwrap().unwrap().next_date,
            date!(2026 - 10 - 25)
        );
        advance_rule(&tx, id, None, "2026-10-21T00:00:00Z").unwrap();
        let rec = get_rule(&tx, id).unwrap().unwrap();
        assert_eq!(rec.ended_at.as_deref(), Some("2026-10-21T00:00:00Z"));
        assert_eq!(rec.next_date, date!(2026 - 10 - 25));
        assert!(update_rule(&tx, id, &write(date!(2026 - 11 - 25))).unwrap());
        let rec = get_rule(&tx, id).unwrap().unwrap();
        assert_eq!(rec.ended_at, None);
        assert_eq!(rec.next_date, date!(2026 - 11 - 25));
        assert!(!update_rule(&tx, Uuid::now_v7(), &write(date!(2026 - 11 - 25))).unwrap());
    }

    #[test]
    fn list_orders_active_first() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let ended = insert_rule(&tx, &write(date!(2026 - 01 - 01))).unwrap();
        advance_rule(&tx, ended, None, &now_utc()).unwrap();
        let late = insert_rule(&tx, &write(date!(2026 - 12 - 01))).unwrap();
        let early = insert_rule(&tx, &write(date!(2026 - 11 - 01))).unwrap();
        tx.commit().unwrap();
        let ids: Vec<Uuid> = list_rules(&conn).unwrap().iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![early, late, ended]);
    }

    #[test]
    fn delete_unlinks_txns() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let acc = insert_account(
            &tx,
            &NewAccount::new("BCA", AccountKind::Bank, CurrencyCode::IDR),
            &now_utc(),
        )
        .unwrap();
        let account = get_account_ref(&tx, acc).unwrap().unwrap().value;
        let valid = ValidTxn::new(TxnDraft {
            kind: TxnKind::Adjustment,
            date: date!(2026 - 10 - 12),
            note: None,
            postings: vec![PostingDraft {
                account,
                category: None,
                amount: 1000,
                memo: None,
            }],
        })
        .unwrap();
        let txn = insert_txn(&tx, &valid, None, &now_utc()).unwrap();
        let rule = insert_rule(&tx, &write(date!(2026 - 10 - 25))).unwrap();
        link_txn(&tx, txn, rule).unwrap();
        let linked: Option<String> = tx
            .query_row(
                "SELECT recurring_rule_id FROM txn WHERE id = ?1",
                [txn.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(linked, Some(rule.to_string()));
        assert!(delete_rule(&tx, rule).unwrap());
        assert!(!delete_rule(&tx, Uuid::now_v7()).unwrap());
        assert_eq!(get_rule(&tx, rule).unwrap(), None);
        assert!(get_txn(&tx, txn).unwrap().is_some());
        let linked: Option<String> = tx
            .query_row(
                "SELECT recurring_rule_id FROM txn WHERE id = ?1",
                [txn.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(linked, None);
    }
}
