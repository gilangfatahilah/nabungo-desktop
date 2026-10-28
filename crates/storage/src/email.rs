//! Auto-capture transaksi dari email notifikasi (EML-01..03): cursor sync IMAP, rule parsing,
//! staging review (`email_capture`), dan log email yang gagal di-parse. Tipe rule di sini
//! tidak bergantung pada `nabungo-mail`; konversinya di `nabungo-app`.

use nabungo_core::date::{format_date, parse_date};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};
use time::Date;
use uuid::Uuid;

use crate::{Result, StorageError, now_utc, parse_uuid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub uid_validity: u32,
    pub last_uid: u32,
}

pub fn get_cursor(conn: &Connection, mailbox: &str) -> Result<Option<Cursor>> {
    Ok(conn
        .query_row(
            "SELECT uid_validity, last_uid FROM email_sync_cursor WHERE mailbox = ?1",
            [mailbox],
            |r| {
                Ok(Cursor {
                    uid_validity: r.get(0)?,
                    last_uid: r.get(1)?,
                })
            },
        )
        .optional()?)
}

/// Simpan cursor dan `last_synced_at` sekarang.
pub fn set_cursor(tx: &Transaction, mailbox: &str, c: Cursor) -> Result<()> {
    tx.execute(
        "INSERT INTO email_sync_cursor (mailbox, uid_validity, last_uid, last_synced_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (mailbox) DO UPDATE SET uid_validity = ?2, last_uid = ?3, last_synced_at = ?4",
        params![mailbox, c.uid_validity, c.last_uid, now_utc()],
    )?;
    Ok(())
}

/// Buang semua cursor: sync berikutnya memindai ulang dari awal (dedup lewat Message-ID).
pub fn clear_cursors(tx: &Transaction) -> Result<()> {
    tx.execute("DELETE FROM email_sync_cursor", [])?;
    Ok(())
}

/// `kind` = `income` | `expense` | `transfer_out` | `transfer_in` (dijaga CHECK).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleWrite<'a> {
    pub name: &'a str,
    pub sender_pattern: &'a str,
    pub subject_regex: Option<&'a str>,
    pub amount_regex: &'a str,
    pub payee_regex: Option<&'a str>,
    pub date_regex: Option<&'a str>,
    pub date_format: Option<&'a str>,
    pub kind: &'a str,
    pub account_id: Option<Uuid>,
    pub counter_account_id: Option<Uuid>,
    pub default_category_id: Option<Uuid>,
    pub priority: i64,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleRecord {
    pub id: Uuid,
    pub name: String,
    pub sender_pattern: String,
    pub subject_regex: Option<String>,
    pub amount_regex: String,
    pub payee_regex: Option<String>,
    pub date_regex: Option<String>,
    pub date_format: Option<String>,
    pub kind: String,
    pub account_id: Option<Uuid>,
    pub counter_account_id: Option<Uuid>,
    pub default_category_id: Option<Uuid>,
    pub priority: i64,
    pub enabled: bool,
}

pub fn insert_rule(tx: &Transaction, r: &RuleWrite) -> Result<Uuid> {
    let id = Uuid::now_v7();
    let now = now_utc();
    tx.execute(
        "INSERT INTO email_rule (id, name, sender_pattern, subject_regex, amount_regex, payee_regex,
           date_regex, date_format, kind, account_id, counter_account_id, default_category_id,
           priority, enabled, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
        params![
            id.to_string(),
            r.name,
            r.sender_pattern,
            r.subject_regex,
            r.amount_regex,
            r.payee_regex,
            r.date_regex,
            r.date_format,
            r.kind,
            r.account_id.map(|u| u.to_string()),
            r.counter_account_id.map(|u| u.to_string()),
            r.default_category_id.map(|u| u.to_string()),
            r.priority,
            r.enabled,
            now,
        ],
    )?;
    Ok(id)
}

const RULE_COLUMNS: &str = "id, name, sender_pattern, subject_regex, amount_regex, payee_regex,
    date_regex, date_format, kind, account_id, counter_account_id, default_category_id,
    priority, enabled";

fn opt_uuid(table: &str, s: Option<String>) -> Result<Option<Uuid>> {
    s.map(|s| parse_uuid(table, &s)).transpose()
}

/// Baris mentah; UUID di-parse di [`to_rule`] agar error-nya `Corrupt`, bukan rusqlite.
type RuleRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    [Option<String>; 3],
    i64,
    bool,
);

fn read_rule(r: &Row) -> rusqlite::Result<RuleRow> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
        r.get(6)?,
        r.get(7)?,
        r.get(8)?,
        [r.get(9)?, r.get(10)?, r.get(11)?],
        r.get(12)?,
        r.get(13)?,
    ))
}

fn to_rule(row: RuleRow) -> Result<RuleRecord> {
    let (
        id,
        name,
        sender_pattern,
        subject_regex,
        amount_regex,
        payee_regex,
        date_regex,
        date_format,
        kind,
        [acc, counter, cat],
        priority,
        enabled,
    ) = row;
    Ok(RuleRecord {
        id: parse_uuid("email_rule", &id)?,
        name,
        sender_pattern,
        subject_regex,
        amount_regex,
        payee_regex,
        date_regex,
        date_format,
        kind,
        account_id: opt_uuid("account", acc)?,
        counter_account_id: opt_uuid("account", counter)?,
        default_category_id: opt_uuid("category", cat)?,
        priority,
        enabled,
    })
}

/// Rule aktif (enabled, tidak dihapus), urut `priority` DESC lalu `name`.
pub fn list_active_rules(conn: &Connection) -> Result<Vec<RuleRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {RULE_COLUMNS} FROM email_rule
         WHERE enabled = 1 AND deleted_at IS NULL
         ORDER BY priority DESC, name, id"
    ))?;
    let rows = stmt.query_map([], read_rule)?;
    rows.map(|r| to_rule(r?)).collect()
}

pub fn get_rule(conn: &Connection, id: Uuid) -> Result<Option<RuleRecord>> {
    conn.query_row(
        &format!("SELECT {RULE_COLUMNS} FROM email_rule WHERE id = ?1"),
        [id.to_string()],
        read_rule,
    )
    .optional()?
    .map(to_rule)
    .transpose()
}

/// Semua rule yang tidak dihapus (aktif & nonaktif), urut seperti [`list_active_rules`].
pub fn list_rules(conn: &Connection) -> Result<Vec<RuleRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {RULE_COLUMNS} FROM email_rule
         WHERE deleted_at IS NULL
         ORDER BY priority DESC, name, id"
    ))?;
    let rows = stmt.query_map([], read_rule)?;
    rows.map(|r| to_rule(r?)).collect()
}

/// `false` bila rule tidak ada atau sudah dihapus.
pub fn update_rule(tx: &Transaction, id: Uuid, r: &RuleWrite) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE email_rule SET name = ?2, sender_pattern = ?3, subject_regex = ?4,
           amount_regex = ?5, payee_regex = ?6, date_regex = ?7, date_format = ?8, kind = ?9,
           account_id = ?10, counter_account_id = ?11, default_category_id = ?12,
           priority = ?13, enabled = ?14, updated_at = ?15
         WHERE id = ?1 AND deleted_at IS NULL",
        params![
            id.to_string(),
            r.name,
            r.sender_pattern,
            r.subject_regex,
            r.amount_regex,
            r.payee_regex,
            r.date_regex,
            r.date_format,
            r.kind,
            r.account_id.map(|u| u.to_string()),
            r.counter_account_id.map(|u| u.to_string()),
            r.default_category_id.map(|u| u.to_string()),
            r.priority,
            r.enabled,
            now_utc(),
        ],
    )?;
    Ok(changed == 1)
}

/// `false` bila rule tidak ada atau sudah dihapus.
pub fn set_rule_enabled(tx: &Transaction, id: Uuid, enabled: bool) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE email_rule SET enabled = ?2, updated_at = ?3 WHERE id = ?1 AND deleted_at IS NULL",
        params![id.to_string(), enabled, now_utc()],
    )?;
    Ok(changed == 1)
}

/// `false` bila rule tidak ada atau sudah dihapus. Capture lama tetap menunjuk rule ini.
pub fn soft_delete_rule(tx: &Transaction, id: Uuid) -> Result<bool> {
    let now = now_utc();
    let changed = tx.execute(
        "UPDATE email_rule SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
        params![id.to_string(), now],
    )?;
    Ok(changed == 1)
}

/// `false` bila rule tidak ada atau tidak sedang dihapus.
pub fn restore_rule(tx: &Transaction, id: Uuid) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE email_rule SET deleted_at = NULL, updated_at = ?2
         WHERE id = ?1 AND deleted_at IS NOT NULL",
        params![id.to_string(), now_utc()],
    )?;
    Ok(changed == 1)
}

pub fn count_pending_for_rule(conn: &Connection, id: Uuid) -> Result<u32> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM email_capture WHERE rule_id = ?1 AND status = 'pending'",
        [id.to_string()],
        |r| r.get(0),
    )?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureWrite<'a> {
    pub message_id: &'a str,
    pub idx: u32,
    pub rule_id: Uuid,
    pub date: Date,
    pub amount: i64,
    pub currency: &'a str,
    pub payee: Option<&'a str>,
    pub raw_snippet: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureStatus {
    Pending,
    Confirmed,
    Ignored,
}

impl CaptureStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Confirmed => "confirmed",
            Self::Ignored => "ignored",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "confirmed" => Ok(Self::Confirmed),
            "ignored" => Ok(Self::Ignored),
            _ => Err(StorageError::Corrupt(format!("email_capture status {s:?}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureRecord {
    pub id: Uuid,
    pub message_id: String,
    pub idx: u32,
    pub rule_id: Option<Uuid>,
    pub date: Date,
    pub amount: i64,
    pub currency: String,
    pub payee: Option<String>,
    pub raw_snippet: Option<String>,
    pub status: CaptureStatus,
    pub txn_id: Option<Uuid>,
}

/// `false` bila `(message_id, idx)` sudah ada (sync ulang).
pub fn insert_capture(tx: &Transaction, w: &CaptureWrite) -> Result<bool> {
    let now = now_utc();
    let changed = tx.execute(
        "INSERT INTO email_capture (id, message_id, idx, rule_id, date, amount, currency, payee,
           raw_snippet, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
         ON CONFLICT (message_id, idx) DO NOTHING",
        params![
            Uuid::now_v7().to_string(),
            w.message_id,
            w.idx,
            w.rule_id.to_string(),
            format_date(w.date),
            w.amount,
            w.currency,
            w.payee,
            w.raw_snippet,
            now,
        ],
    )?;
    Ok(changed == 1)
}

const CAPTURE_COLUMNS: &str =
    "id, message_id, idx, rule_id, date, amount, currency, payee, raw_snippet, status, txn_id";

type CaptureRow = (
    String,
    String,
    u32,
    Option<String>,
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
);

fn read_capture(r: &Row) -> rusqlite::Result<CaptureRow> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
        r.get(6)?,
        r.get(7)?,
        r.get(8)?,
        r.get(9)?,
        r.get(10)?,
    ))
}

fn to_capture(row: CaptureRow) -> Result<CaptureRecord> {
    let (id, message_id, idx, rule_id, date, amount, currency, payee, raw_snippet, status, txn_id) =
        row;
    let parsed_date = parse_date(&date)
        .map_err(|e| StorageError::Corrupt(format!("email_capture date {date:?}: {e}")))?;
    Ok(CaptureRecord {
        id: parse_uuid("email_capture", &id)?,
        message_id,
        idx,
        rule_id: opt_uuid("email_rule", rule_id)?,
        date: parsed_date,
        amount,
        currency,
        payee,
        raw_snippet,
        status: CaptureStatus::parse(&status)?,
        txn_id: opt_uuid("txn", txn_id)?,
    })
}

pub fn get_capture(conn: &Connection, id: Uuid) -> Result<Option<CaptureRecord>> {
    conn.query_row(
        &format!("SELECT {CAPTURE_COLUMNS} FROM email_capture WHERE id = ?1"),
        [id.to_string()],
        read_capture,
    )
    .optional()?
    .map(to_capture)
    .transpose()
}

/// Urut `date` DESC lalu `id` DESC (terbaru di atas).
pub fn list_captures(conn: &Connection, status: CaptureStatus) -> Result<Vec<CaptureRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {CAPTURE_COLUMNS} FROM email_capture WHERE status = ?1 ORDER BY date DESC, id DESC"
    ))?;
    let rows = stmt.query_map([status.as_str()], read_capture)?;
    rows.map(|r| to_capture(r?)).collect()
}

pub fn count_captures(conn: &Connection, status: CaptureStatus) -> Result<u32> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM email_capture WHERE status = ?1",
        [status.as_str()],
        |r| r.get(0),
    )?)
}

/// Capture `pending` dengan `date <= until`, urut `date`, `id`.
pub fn pending_ids_until(conn: &Connection, until: Date) -> Result<Vec<Uuid>> {
    let mut stmt = conn.prepare(
        "SELECT id FROM email_capture WHERE status = 'pending' AND date <= ?1 ORDER BY date, id",
    )?;
    let ids = stmt.query_map([format_date(until)], |r| r.get::<_, String>(0))?;
    ids.map(|id| parse_uuid("email_capture", &id?)).collect()
}

pub fn set_capture_status(
    tx: &Transaction,
    id: Uuid,
    status: CaptureStatus,
    txn_id: Option<Uuid>,
) -> Result<()> {
    tx.execute(
        "UPDATE email_capture SET status = ?2, txn_id = ?3, updated_at = ?4 WHERE id = ?1",
        params![
            id.to_string(),
            status.as_str(),
            txn_id.map(|u| u.to_string()),
            now_utc()
        ],
    )?;
    Ok(())
}

/// Email ini sudah pernah diproses (masuk staging atau log unparsed).
pub fn capture_exists(conn: &Connection, message_id: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM email_capture WHERE message_id = ?1)
             OR EXISTS (SELECT 1 FROM email_unparsed WHERE message_id = ?1)",
        [message_id],
        |r| r.get(0),
    )?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnparsedWrite<'a> {
    pub message_id: &'a str,
    pub sender: &'a str,
    pub subject: &'a str,
    /// Header `Date` email, ISO-8601 UTC.
    pub received_at: Option<&'a str>,
    pub raw_snippet: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnparsedRecord {
    pub message_id: String,
    pub sender: String,
    pub subject: String,
    pub received_at: Option<String>,
    pub raw_snippet: String,
    pub created_at: String,
}

/// `false` bila email sudah tercatat.
pub fn insert_unparsed(tx: &Transaction, w: &UnparsedWrite) -> Result<bool> {
    let changed = tx.execute(
        "INSERT INTO email_unparsed (message_id, sender, subject, received_at, raw_snippet, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (message_id) DO NOTHING",
        params![w.message_id, w.sender, w.subject, w.received_at, w.raw_snippet, now_utc()],
    )?;
    Ok(changed == 1)
}

const UNPARSED_COLUMNS: &str = "message_id, COALESCE(sender, ''), COALESCE(subject, ''),
    received_at, COALESCE(raw_snippet, ''), created_at";

fn read_unparsed(r: &Row) -> rusqlite::Result<UnparsedRecord> {
    Ok(UnparsedRecord {
        message_id: r.get(0)?,
        sender: r.get(1)?,
        subject: r.get(2)?,
        received_at: r.get(3)?,
        raw_snippet: r.get(4)?,
        created_at: r.get(5)?,
    })
}

pub fn list_unparsed(conn: &Connection) -> Result<Vec<UnparsedRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {UNPARSED_COLUMNS} FROM email_unparsed ORDER BY created_at DESC, message_id"
    ))?;
    let rows = stmt.query_map([], read_unparsed)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn get_unparsed(conn: &Connection, message_id: &str) -> Result<Option<UnparsedRecord>> {
    Ok(conn
        .query_row(
            &format!("SELECT {UNPARSED_COLUMNS} FROM email_unparsed WHERE message_id = ?1"),
            [message_id],
            read_unparsed,
        )
        .optional()?)
}

/// Email sudah terbaca rule (re-parse): pindah ke `email_capture` oleh pemanggil.
pub fn delete_unparsed(tx: &Transaction, message_id: &str) -> Result<()> {
    tx.execute(
        "DELETE FROM email_unparsed WHERE message_id = ?1",
        [message_id],
    )?;
    Ok(())
}

pub fn count_unparsed(conn: &Connection) -> Result<u32> {
    Ok(conn.query_row("SELECT COUNT(*) FROM email_unparsed", [], |r| r.get(0))?)
}

/// Transaksi aktif `kind` bernominal `amount` (absolut; transfer = sisi masuk) dengan tanggal
/// ±1 hari dari `date`, opsional hanya yang menyentuh `account_id`. Urut selisih tanggal.
pub fn find_duplicate_txns(
    conn: &Connection,
    kind: &str,
    amount: i64,
    date: Date,
    account_id: Option<Uuid>,
    limit: u32,
) -> Result<Vec<Uuid>> {
    let mut stmt = conn.prepare(
        "SELECT t.id FROM txn t
         WHERE t.deleted_at IS NULL AND t.status = 'posted' AND t.kind = ?1
           AND t.date BETWEEN date(?2, '-1 day') AND date(?2, '+1 day')
           AND ABS(CASE WHEN t.kind = 'transfer'
                 THEN (SELECT MAX(amount) FROM posting WHERE txn_id = t.id AND category_id IS NULL)
                 ELSE (SELECT SUM(amount) FROM posting WHERE txn_id = t.id) END) = ?3
           AND (?4 IS NULL OR EXISTS
                 (SELECT 1 FROM posting p WHERE p.txn_id = t.id AND p.account_id = ?4))
         ORDER BY ABS(julianday(t.date) - julianday(?2)), t.date, t.id
         LIMIT ?5",
    )?;
    let ids = stmt.query_map(
        params![
            kind,
            format_date(date),
            amount,
            account_id.map(|u| u.to_string()),
            limit
        ],
        |r| r.get::<_, String>(0),
    )?;
    ids.map(|id| parse_uuid("txn", &id?)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{NewAccount, get_account_ref, insert_account};
    use crate::txn::insert_txn;
    use crate::{now_utc, open_in_memory};
    use nabungo_core::{AccountKind, CurrencyCode, PostingDraft, TxnDraft, TxnKind, ValidTxn};
    use time::macros::date;

    fn rule(name: &'static str, priority: i64, enabled: bool) -> RuleWrite<'static> {
        RuleWrite {
            name,
            sender_pattern: "bca.co.id",
            subject_regex: Some("^Internet Transaction Journal$"),
            amount_regex: r"^Total Payment : IDR ([\d.,]+)$",
            payee_regex: None,
            date_regex: None,
            date_format: None,
            kind: "expense",
            account_id: None,
            counter_account_id: None,
            default_category_id: None,
            priority,
            enabled,
        }
    }

    fn capture(message_id: &str, idx: u32, rule_id: Uuid) -> CaptureWrite<'_> {
        CaptureWrite {
            message_id,
            idx,
            rule_id,
            date: date!(2026 - 10 - 27),
            amount: 20_000,
            currency: "IDR",
            payee: Some("Warkop"),
            raw_snippet: "Total Payment : IDR 20,000.00",
        }
    }

    fn unparsed(message_id: &str) -> UnparsedWrite<'_> {
        UnparsedWrite {
            message_id,
            sender: "promo@toko.example",
            subject: "Diskon",
            received_at: Some("2026-10-27T03:00:00.000Z"),
            raw_snippet: "Diskon 50%",
        }
    }

    #[test]
    fn cursor_roundtrip() {
        let mut conn = open_in_memory().unwrap();
        assert_eq!(get_cursor(&conn, "Finance").unwrap(), None);
        let tx = conn.transaction().unwrap();
        set_cursor(
            &tx,
            "Finance",
            Cursor {
                uid_validity: 14,
                last_uid: 0,
            },
        )
        .unwrap();
        set_cursor(
            &tx,
            "Finance",
            Cursor {
                uid_validity: 14,
                last_uid: 479,
            },
        )
        .unwrap();
        tx.commit().unwrap();
        assert_eq!(
            get_cursor(&conn, "Finance").unwrap(),
            Some(Cursor {
                uid_validity: 14,
                last_uid: 479
            })
        );
        let synced: Option<String> = conn
            .query_row("SELECT last_synced_at FROM email_sync_cursor", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(synced.is_some());
        assert_eq!(get_cursor(&conn, "Lain").unwrap(), None);
    }

    #[test]
    fn rule_roundtrip() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let mut w = rule("BCA", 5, true);
        w.kind = "transfer_out";
        w.payee_regex = Some("^Payment to : (.+)$");
        let id = insert_rule(&tx, &w).unwrap();
        tx.commit().unwrap();
        let r = get_rule(&conn, id).unwrap().unwrap();
        assert_eq!(r.id, id);
        assert_eq!(r.name, "BCA");
        assert_eq!(r.kind, "transfer_out");
        assert_eq!(r.payee_regex.as_deref(), Some("^Payment to : (.+)$"));
        assert_eq!(r.date_regex, None);
        assert_eq!(r.priority, 5);
        assert!(r.enabled);
        assert_eq!(get_rule(&conn, Uuid::now_v7()).unwrap(), None);
    }

    #[test]
    fn invalid_rule_kind_is_rejected() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let mut w = rule("BCA", 0, true);
        w.kind = "transfer";
        assert!(insert_rule(&tx, &w).is_err());
    }

    #[test]
    fn list_active_rules_orders_and_filters() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let low = insert_rule(&tx, &rule("A rendah", 0, true)).unwrap();
        let high_b = insert_rule(&tx, &rule("B tinggi", 10, true)).unwrap();
        let high_a = insert_rule(&tx, &rule("A tinggi", 10, true)).unwrap();
        insert_rule(&tx, &rule("Mati", 99, false)).unwrap();
        let deleted = insert_rule(&tx, &rule("Terhapus", 99, true)).unwrap();
        tx.execute(
            "UPDATE email_rule SET deleted_at = ?2 WHERE id = ?1",
            [deleted.to_string(), now_utc()],
        )
        .unwrap();
        tx.commit().unwrap();
        let ids: Vec<Uuid> = list_active_rules(&conn)
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(ids, vec![high_a, high_b, low]);
    }

    #[test]
    fn insert_capture_is_idempotent() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let r = insert_rule(&tx, &rule("BCA", 0, true)).unwrap();
        assert!(insert_capture(&tx, &capture("m1@bca.co.id", 0, r)).unwrap());
        assert!(!insert_capture(&tx, &capture("m1@bca.co.id", 0, r)).unwrap());
        tx.commit().unwrap();
        let all = list_captures(&conn, CaptureStatus::Pending).unwrap();
        assert_eq!(all.len(), 1);
        let c = &all[0];
        assert_eq!(c.message_id, "m1@bca.co.id");
        assert_eq!(c.idx, 0);
        assert_eq!(c.rule_id, Some(r));
        assert_eq!(c.date, date!(2026 - 10 - 27));
        assert_eq!(c.amount, 20_000);
        assert_eq!(c.currency, "IDR");
        assert_eq!(c.payee.as_deref(), Some("Warkop"));
        assert_eq!(
            c.raw_snippet.as_deref(),
            Some("Total Payment : IDR 20,000.00")
        );
        assert_eq!(c.status, CaptureStatus::Pending);
        assert_eq!(c.txn_id, None);
        assert_eq!(get_capture(&conn, c.id).unwrap().as_ref(), Some(c));
    }

    #[test]
    fn same_message_different_idx_both_stored() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let r = insert_rule(&tx, &rule("BCA", 0, true)).unwrap();
        assert!(insert_capture(&tx, &capture("m1", 0, r)).unwrap());
        assert!(insert_capture(&tx, &capture("m1", 1, r)).unwrap());
        let idx: Vec<u32> = list_captures(&tx, CaptureStatus::Pending)
            .unwrap()
            .iter()
            .map(|c| c.idx)
            .collect();
        assert_eq!(idx.len(), 2);
        assert!(idx.contains(&0) && idx.contains(&1));
    }

    #[test]
    fn list_captures_orders_by_date_desc_and_filters_status() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let r = insert_rule(&tx, &rule("BCA", 0, true)).unwrap();
        let mut old = capture("lama", 0, r);
        old.date = date!(2026 - 10 - 01);
        insert_capture(&tx, &old).unwrap();
        insert_capture(&tx, &capture("baru", 0, r)).unwrap();
        insert_capture(&tx, &capture("abaikan", 0, r)).unwrap();
        let ignored = list_captures(&tx, CaptureStatus::Pending)
            .unwrap()
            .into_iter()
            .find(|c| c.message_id == "abaikan")
            .unwrap();
        set_capture_status(&tx, ignored.id, CaptureStatus::Ignored, None).unwrap();
        let pending: Vec<String> = list_captures(&tx, CaptureStatus::Pending)
            .unwrap()
            .into_iter()
            .map(|c| c.message_id)
            .collect();
        assert_eq!(pending, vec!["baru", "lama"]);
        assert_eq!(list_captures(&tx, CaptureStatus::Ignored).unwrap().len(), 1);
    }

    #[test]
    fn count_and_pending_until() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let r = insert_rule(&tx, &rule("BCA", 0, true)).unwrap();
        for (m, d) in [
            ("a", date!(2026 - 05 - 09)),
            ("b", date!(2026 - 05 - 11)),
            ("c", date!(2026 - 05 - 01)),
        ] {
            let mut w = capture(m, 0, r);
            w.date = d;
            insert_capture(&tx, &w).unwrap();
        }
        let find = |m: &str| {
            list_captures(&tx, CaptureStatus::Pending)
                .unwrap()
                .into_iter()
                .find(|c| c.message_id == m)
                .unwrap()
                .id
        };
        let (a, c) = (find("a"), find("c"));
        set_capture_status(&tx, c, CaptureStatus::Ignored, None).unwrap();
        insert_unparsed(&tx, &unparsed("u1")).unwrap();
        assert_eq!(count_captures(&tx, CaptureStatus::Pending).unwrap(), 2);
        assert_eq!(count_unparsed(&tx).unwrap(), 1);
        assert_eq!(
            pending_ids_until(&tx, date!(2026 - 05 - 10)).unwrap(),
            vec![a]
        );
    }

    #[test]
    fn status_update_sets_txn_id() {
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
            date: date!(2026 - 10 - 27),
            note: None,
            postings: vec![PostingDraft {
                account,
                category: None,
                amount: -20_000,
                memo: None,
            }],
        })
        .unwrap();
        let txn = insert_txn(&tx, &valid, None, &now_utc()).unwrap();
        let r = insert_rule(&tx, &rule("BCA", 0, true)).unwrap();
        insert_capture(&tx, &capture("m1", 0, r)).unwrap();
        let id = list_captures(&tx, CaptureStatus::Pending).unwrap()[0].id;
        set_capture_status(&tx, id, CaptureStatus::Confirmed, Some(txn)).unwrap();
        let c = get_capture(&tx, id).unwrap().unwrap();
        assert_eq!(c.status, CaptureStatus::Confirmed);
        assert_eq!(c.txn_id, Some(txn));
        assert!(
            list_captures(&tx, CaptureStatus::Pending)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn unparsed_is_idempotent() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        assert!(insert_unparsed(&tx, &unparsed("p1")).unwrap());
        assert!(!insert_unparsed(&tx, &unparsed("p1")).unwrap());
        tx.commit().unwrap();
        let all = list_unparsed(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].message_id, "p1");
        assert_eq!(all[0].sender, "promo@toko.example");
        assert_eq!(all[0].subject, "Diskon");
        assert_eq!(
            all[0].received_at.as_deref(),
            Some("2026-10-27T03:00:00.000Z")
        );
        assert_eq!(all[0].raw_snippet, "Diskon 50%");
    }

    #[test]
    fn capture_exists_checks_both_tables() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let r = insert_rule(&tx, &rule("BCA", 0, true)).unwrap();
        insert_capture(&tx, &capture("c1", 0, r)).unwrap();
        insert_unparsed(&tx, &unparsed("u1")).unwrap();
        assert!(capture_exists(&tx, "c1").unwrap());
        assert!(capture_exists(&tx, "u1").unwrap());
        assert!(!capture_exists(&tx, "x").unwrap());
    }

    #[test]
    fn rules_crud() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let low = insert_rule(&tx, &rule("Rendah", 0, true)).unwrap();
        let high = insert_rule(&tx, &rule("Tinggi", 5, true)).unwrap();
        let ids = |rs: Vec<RuleRecord>| rs.into_iter().map(|r| r.id).collect::<Vec<_>>();
        assert_eq!(ids(list_rules(&tx).unwrap()), vec![high, low]);

        assert!(set_rule_enabled(&tx, high, false).unwrap());
        assert_eq!(ids(list_active_rules(&tx).unwrap()), vec![low]);
        assert_eq!(ids(list_rules(&tx).unwrap()), vec![high, low]);
        assert!(!get_rule(&tx, high).unwrap().unwrap().enabled);

        assert!(soft_delete_rule(&tx, low).unwrap());
        assert!(!soft_delete_rule(&tx, low).unwrap());
        assert_eq!(ids(list_active_rules(&tx).unwrap()), Vec::<Uuid>::new());
        assert_eq!(ids(list_rules(&tx).unwrap()), vec![high]);
        // Rule terhapus tetap bisa dibaca (nama di capture lama).
        assert_eq!(get_rule(&tx, low).unwrap().unwrap().name, "Rendah");
        assert!(!update_rule(&tx, low, &rule("Baru", 1, true)).unwrap());
        assert!(!set_rule_enabled(&tx, low, true).unwrap());

        assert!(restore_rule(&tx, low).unwrap());
        assert!(!restore_rule(&tx, low).unwrap());
        assert_eq!(ids(list_active_rules(&tx).unwrap()), vec![low]);

        assert!(update_rule(&tx, low, &rule("Baru", 9, true)).unwrap());
        let r = get_rule(&tx, low).unwrap().unwrap();
        assert_eq!((r.name.as_str(), r.priority), ("Baru", 9));
        assert!(!update_rule(&tx, Uuid::now_v7(), &rule("x", 0, true)).unwrap());
    }

    #[test]
    fn pending_count_per_rule() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let r = insert_rule(&tx, &rule("QRIS", 0, true)).unwrap();
        let other = insert_rule(&tx, &rule("Lain", 0, true)).unwrap();
        insert_capture(&tx, &capture("a", 0, r)).unwrap();
        insert_capture(&tx, &capture("a", 1, r)).unwrap();
        let ignored = {
            insert_capture(&tx, &capture("b", 0, r)).unwrap();
            list_captures(&tx, CaptureStatus::Pending)
                .unwrap()
                .into_iter()
                .find(|c| c.message_id == "b")
                .unwrap()
                .id
        };
        set_capture_status(&tx, ignored, CaptureStatus::Ignored, None).unwrap();
        assert_eq!(count_pending_for_rule(&tx, r).unwrap(), 2);
        assert_eq!(count_pending_for_rule(&tx, other).unwrap(), 0);
    }

    #[test]
    fn unparsed_get_and_delete() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        insert_unparsed(&tx, &unparsed("u1")).unwrap();
        let got = get_unparsed(&tx, "u1").unwrap().unwrap();
        assert_eq!(got.subject, "Diskon");
        assert_eq!(get_unparsed(&tx, "x").unwrap(), None);
        delete_unparsed(&tx, "u1").unwrap();
        assert_eq!(get_unparsed(&tx, "u1").unwrap(), None);
        assert_eq!(count_unparsed(&tx).unwrap(), 0);
    }
}
