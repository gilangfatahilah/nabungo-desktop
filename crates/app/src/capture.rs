//! Review queue email capture (EML-02/03): seed rule, ingest email, draft, confirm/link/ignore.

use std::collections::HashSet;

use nabungo_core::TxnKind;
use nabungo_mail::defaults::default_rules;
use nabungo_mail::message::{parse_message, raw_message_id, snippet};
use nabungo_mail::rule::{CompiledRules, Outcome, Rule, RuleKind};
use nabungo_storage::email::{
    self as repo, CaptureRecord, CaptureStatus, CaptureWrite, Cursor, RuleRecord, RuleWrite,
    UnparsedRecord, UnparsedWrite,
};
use nabungo_storage::rusqlite::Transaction;
use nabungo_storage::{StorageError, format_utc, setting, txn};
use time::{Date, UtcOffset};
use uuid::Uuid;

use crate::ledger::{AppError, Ledger};
use crate::transactions::{TxnForm, form_input, write_txn};

/// Setting penanda rule bawaan sudah disalin (rule yang dihapus user tidak kembali).
const RULES_SEEDED: &str = "email.rules_seeded";
/// v1 hanya IDR (D4).
const CURRENCY: &str = "IDR";
const MAX_DUPLICATES: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestOutcome {
    /// Jumlah item baru di staging.
    Captured(u32),
    /// Email sudah pernah diproses.
    AlreadySeen,
    Unparsed,
}

/// Isian awal form review. Transfer: `account_id` = akun asal, `counter_account_id` = tujuan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureDraft {
    pub capture_id: Uuid,
    pub kind: TxnKind,
    pub date: Date,
    pub amount: i64,
    pub payee: Option<String>,
    pub account_id: Option<Uuid>,
    pub counter_account_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    /// Nama rule.
    pub note: Option<String>,
}

fn kind_str(kind: RuleKind) -> &'static str {
    match kind {
        RuleKind::Income => "income",
        RuleKind::Expense => "expense",
        RuleKind::TransferOut => "transfer_out",
        RuleKind::TransferIn => "transfer_in",
    }
}

fn parse_kind(s: &str) -> Result<RuleKind, AppError> {
    Ok(match s {
        "income" => RuleKind::Income,
        "expense" => RuleKind::Expense,
        "transfer_out" => RuleKind::TransferOut,
        "transfer_in" => RuleKind::TransferIn,
        _ => return Err(StorageError::Corrupt(format!("email_rule kind {s:?}")).into()),
    })
}

fn to_rule(r: RuleRecord) -> Result<Rule, AppError> {
    Ok(Rule {
        id: Some(r.id),
        kind: parse_kind(&r.kind)?,
        name: r.name,
        sender_pattern: r.sender_pattern,
        subject_regex: r.subject_regex,
        amount_regex: r.amount_regex,
        payee_regex: r.payee_regex,
        date_regex: r.date_regex,
        date_format: r.date_format,
        account_id: r.account_id,
        counter_account_id: r.counter_account_id,
        default_category_id: r.default_category_id,
        priority: r.priority,
    })
}

/// Capture yang masih `pending`; selain itu `CaptureNotPending`.
fn pending_capture(tx: &Transaction, id: Uuid) -> Result<CaptureRecord, AppError> {
    match repo::get_capture(tx, id)? {
        None => Err(AppError::NotFound("email capture")),
        Some(c) if c.status != CaptureStatus::Pending => Err(AppError::CaptureNotPending),
        Some(c) => Ok(c),
    }
}

impl Ledger {
    /// Salin rule bawaan ke DB sekali saja (K8).
    pub fn seed_email_rules(&mut self) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        if setting::get_setting(&tx, RULES_SEEDED)?.is_some() {
            return Ok(());
        }
        for r in default_rules() {
            repo::insert_rule(
                &tx,
                &RuleWrite {
                    name: &r.name,
                    sender_pattern: &r.sender_pattern,
                    subject_regex: r.subject_regex.as_deref(),
                    amount_regex: &r.amount_regex,
                    payee_regex: r.payee_regex.as_deref(),
                    date_regex: r.date_regex.as_deref(),
                    date_format: r.date_format.as_deref(),
                    kind: kind_str(r.kind),
                    account_id: r.account_id,
                    counter_account_id: r.counter_account_id,
                    default_category_id: r.default_category_id,
                    priority: r.priority,
                    enabled: true,
                },
            )?;
        }
        setting::set_setting(&tx, RULES_SEEDED, "1")?;
        tx.commit()?;
        Ok(())
    }

    pub fn email_cursor(&self, mailbox: &str) -> Result<Option<Cursor>, AppError> {
        Ok(repo::get_cursor(&self.conn, mailbox)?)
    }

    /// Rule aktif yang sudah di-compile, urut prioritas.
    pub fn email_rules(&self) -> Result<CompiledRules, AppError> {
        let rules = repo::list_active_rules(&self.conn)?
            .into_iter()
            .map(to_rule)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CompiledRules::new(&rules))
    }

    /// Satu email = satu SQLite transaction: capture/unparsed + cursor (K4).
    pub fn ingest_email(
        &mut self,
        mailbox: &str,
        cursor: Cursor,
        raw: &[u8],
        rules: &CompiledRules,
        local_offset: UtcOffset,
    ) -> Result<IngestOutcome, AppError> {
        let tx = self.conn.transaction()?;
        let outcome = match parse_message(raw) {
            None => {
                let message_id = raw_message_id(raw);
                let fresh = repo::insert_unparsed(
                    &tx,
                    &UnparsedWrite {
                        message_id: &message_id,
                        sender: "",
                        subject: "",
                        received_at: None,
                        raw_snippet: "",
                    },
                )?;
                if fresh {
                    IngestOutcome::Unparsed
                } else {
                    IngestOutcome::AlreadySeen
                }
            }
            Some(msg) if repo::capture_exists(&tx, &msg.message_id)? => IngestOutcome::AlreadySeen,
            Some(msg) => match rules.apply(&msg, local_offset) {
                Outcome::Parsed { rule, items } => {
                    let rule = rules.rule(rule);
                    let rule_id = rule.id.ok_or(AppError::NotFound("email rule"))?;
                    let raw_snippet = snippet(&msg.text);
                    let mut fresh = 0;
                    for item in &items {
                        let inserted = repo::insert_capture(
                            &tx,
                            &CaptureWrite {
                                message_id: &msg.message_id,
                                idx: item.idx,
                                rule_id,
                                date: item.date,
                                amount: item.amount,
                                currency: CURRENCY,
                                payee: item.payee.as_deref(),
                                raw_snippet: &raw_snippet,
                            },
                        )?;
                        fresh += u32::from(inserted);
                    }
                    if fresh == 0 {
                        IngestOutcome::AlreadySeen
                    } else {
                        IngestOutcome::Captured(fresh)
                    }
                }
                Outcome::Unparsed => {
                    repo::insert_unparsed(
                        &tx,
                        &UnparsedWrite {
                            message_id: &msg.message_id,
                            sender: &msg.from,
                            subject: &msg.subject,
                            received_at: msg.sent_at.map(format_utc).as_deref(),
                            raw_snippet: &snippet(&msg.text),
                        },
                    )?;
                    IngestOutcome::Unparsed
                }
            },
        };
        repo::set_cursor(&tx, mailbox, cursor)?;
        tx.commit()?;
        Ok(outcome)
    }

    pub fn list_captures(&self, status: CaptureStatus) -> Result<Vec<CaptureRecord>, AppError> {
        Ok(repo::list_captures(&self.conn, status)?)
    }

    pub fn list_unparsed(&self) -> Result<Vec<UnparsedRecord>, AppError> {
        Ok(repo::list_unparsed(&self.conn)?)
    }

    /// Isian form dari rule, lalu saran payee (K9).
    pub fn capture_draft(&self, id: Uuid) -> Result<CaptureDraft, AppError> {
        let capture =
            repo::get_capture(&self.conn, id)?.ok_or(AppError::NotFound("email capture"))?;
        let rule = match capture.rule_id {
            Some(rule_id) => repo::get_rule(&self.conn, rule_id)?
                .map(to_rule)
                .transpose()?,
            None => None,
        };
        let kind = rule.as_ref().map_or(RuleKind::Expense, |r| r.kind);
        let suggestion = match &capture.payee {
            Some(p) => self.payee_suggestion(p)?,
            None => None,
        };
        let rule_account = rule.as_ref().and_then(|r| r.account_id);
        let counter = rule.as_ref().and_then(|r| r.counter_account_id);
        let (txn_kind, account_id, counter_account_id, category_id) = match kind {
            RuleKind::Income | RuleKind::Expense => (
                if kind == RuleKind::Income {
                    TxnKind::Income
                } else {
                    TxnKind::Expense
                },
                rule_account.or(suggestion.and_then(|s| s.account_id)),
                None,
                rule.as_ref()
                    .and_then(|r| r.default_category_id)
                    .or(suggestion.and_then(|s| s.category_id)),
            ),
            RuleKind::TransferOut => (TxnKind::Transfer, rule_account, counter, None),
            RuleKind::TransferIn => (TxnKind::Transfer, counter, rule_account, None),
        };
        Ok(CaptureDraft {
            capture_id: capture.id,
            kind: txn_kind,
            date: capture.date,
            amount: capture.amount,
            payee: capture.payee,
            account_id,
            counter_account_id,
            category_id,
            note: rule.map(|r| r.name),
        })
    }

    /// Catat transaksi dari form review dan tandai capture confirmed (K10).
    pub fn confirm_capture(&mut self, id: Uuid, form: &TxnForm) -> Result<Uuid, AppError> {
        let input = form_input(form)?;
        let tx = self.conn.transaction()?;
        pending_capture(&tx, id)?;
        let txn_id = write_txn(&tx, None, &input, &HashSet::new())?;
        repo::set_capture_status(&tx, id, CaptureStatus::Confirmed, Some(txn_id))?;
        tx.commit()?;
        Ok(txn_id)
    }

    /// Hubungkan ke transaksi yang sudah ada (duplikat) tanpa transaksi baru.
    pub fn link_capture(&mut self, id: Uuid, txn_id: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        pending_capture(&tx, id)?;
        match txn::get_txn(&tx, txn_id)? {
            Some(t) if !t.deleted => {}
            _ => return Err(AppError::NotFound("transaction")),
        }
        repo::set_capture_status(&tx, id, CaptureStatus::Confirmed, Some(txn_id))?;
        tx.commit()?;
        Ok(())
    }

    pub fn ignore_capture(&mut self, id: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        pending_capture(&tx, id)?;
        repo::set_capture_status(&tx, id, CaptureStatus::Ignored, None)?;
        tx.commit()?;
        Ok(())
    }

    /// Transaksi sejenis bernominal sama, ±1 hari, di akun draft bila diketahui (K11).
    pub fn duplicate_candidates(&self, id: Uuid) -> Result<Vec<Uuid>, AppError> {
        let draft = self.capture_draft(id)?;
        Ok(repo::find_duplicate_txns(
            &self.conn,
            draft.kind.as_str(),
            draft.amount,
            draft.date,
            draft.account_id,
            MAX_DUPLICATES,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Kit;
    use nabungo_core::{FormBody, SplitLine};
    use time::macros::{date, offset};

    const WIB: UtcOffset = offset!(+7);
    const MAILBOX: &str = "notion-synced";
    const QRIS: &[u8] = include_bytes!("../../mail/tests/fixtures/bca_qris.eml");
    const TARIK: &[u8] = include_bytes!("../../mail/tests/fixtures/bca_tarik_tunai.eml");
    const UNKNOWN: &[u8] = include_bytes!("../../mail/tests/fixtures/unknown_sender.eml");
    /// bca_qris: 41.700 ke "IDM INDOMARET D" tanggal 2026-05-09.
    const QRIS_AMOUNT: i64 = 41_700;
    const QRIS_DATE: Date = date!(2026 - 05 - 09);

    fn kit() -> Kit {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        kit.ledger.seed_email_rules().unwrap();
        kit
    }

    fn cursor(last_uid: u32) -> Cursor {
        Cursor {
            uid_validity: 7,
            last_uid,
        }
    }

    fn ingest(kit: &mut Kit, uid: u32, raw: &[u8]) -> IngestOutcome {
        let rules = kit.ledger.email_rules().unwrap();
        kit.ledger
            .ingest_email(MAILBOX, cursor(uid), raw, &rules, WIB)
            .unwrap()
    }

    fn pending(kit: &Kit) -> Vec<CaptureRecord> {
        kit.ledger.list_captures(CaptureStatus::Pending).unwrap()
    }

    /// Capture QRIS yang baru di-ingest.
    fn qris_capture(kit: &mut Kit) -> Uuid {
        assert_eq!(ingest(kit, 10, QRIS), IngestOutcome::Captured(1));
        pending(kit)[0].id
    }

    fn expense(kit: &Kit, account_id: Uuid, date: Date, amount: i64) -> TxnForm {
        TxnForm {
            body: FormBody::Expense {
                account_id,
                amount,
                splits: vec![SplitLine {
                    category_id: kit.cat("Makan & Minum"),
                    amount,
                    memo: None,
                }],
            },
            date,
            payee: Some("IDM INDOMARET D".into()),
            note: None,
            tags: vec![],
        }
    }

    fn txn_count(kit: &Kit) -> i64 {
        kit.ledger
            .conn
            .query_row(
                "SELECT COUNT(*) FROM txn WHERE deleted_at IS NULL AND kind = 'expense'",
                [],
                |r| r.get(0),
            )
            .unwrap()
    }

    fn rule_count(kit: &Kit) -> usize {
        nabungo_storage::email::list_active_rules(&kit.ledger.conn)
            .unwrap()
            .len()
    }

    #[test]
    fn seed_is_once() {
        let mut kit = kit();
        let total = rule_count(&kit);
        assert_eq!(total, nabungo_mail::defaults::default_rules().len());
        kit.ledger
            .conn
            .execute(
                "UPDATE email_rule SET deleted_at = '2026-10-27T00:00:00.000Z' WHERE name = 'BCA Transfer'",
                [],
            )
            .unwrap();
        kit.ledger.seed_email_rules().unwrap();
        assert_eq!(rule_count(&kit), total - 1);
    }

    #[test]
    fn ingest_creates_pending_and_advances_cursor() {
        let mut kit = kit();
        assert_eq!(ingest(&mut kit, 10, QRIS), IngestOutcome::Captured(1));
        let caps = pending(&kit);
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].amount, QRIS_AMOUNT);
        assert_eq!(caps[0].date, QRIS_DATE);
        assert_eq!(caps[0].payee.as_deref(), Some("IDM INDOMARET D"));
        assert!(caps[0].rule_id.is_some());
        assert_eq!(kit.ledger.email_cursor(MAILBOX).unwrap(), Some(cursor(10)));
    }

    #[test]
    fn ingest_same_email_twice_is_already_seen() {
        let mut kit = kit();
        ingest(&mut kit, 10, QRIS);
        assert_eq!(ingest(&mut kit, 11, QRIS), IngestOutcome::AlreadySeen);
        assert_eq!(pending(&kit).len(), 1);
        assert_eq!(kit.ledger.email_cursor(MAILBOX).unwrap(), Some(cursor(11)));
    }

    #[test]
    fn ingest_unknown_goes_unparsed() {
        let mut kit = kit();
        assert_eq!(ingest(&mut kit, 3, UNKNOWN), IngestOutcome::Unparsed);
        assert!(pending(&kit).is_empty());
        let unparsed = kit.ledger.list_unparsed().unwrap();
        assert_eq!(unparsed.len(), 1);
        assert!(!unparsed[0].sender.is_empty());
        assert_eq!(ingest(&mut kit, 4, UNKNOWN), IngestOutcome::AlreadySeen);
        assert_eq!(kit.ledger.email_cursor(MAILBOX).unwrap(), Some(cursor(4)));
    }

    #[test]
    fn unreadable_raw_goes_unparsed_with_hash_id() {
        let mut kit = kit();
        assert_eq!(ingest(&mut kit, 5, b""), IngestOutcome::Unparsed);
        let unparsed = kit.ledger.list_unparsed().unwrap();
        assert!(unparsed[0].message_id.starts_with("sha256:"));
        assert_eq!(unparsed[0].sender, "");
        assert_eq!(kit.ledger.email_cursor(MAILBOX).unwrap(), Some(cursor(5)));
    }

    #[test]
    fn draft_uses_payee_suggestion() {
        let mut kit = kit();
        let form = expense(&kit, kit.dompet, date!(2026 - 04 - 01), 10_000);
        kit.ledger.create_txn(&form).unwrap();
        let id = qris_capture(&mut kit);
        let draft = kit.ledger.capture_draft(id).unwrap();
        assert_eq!(draft.kind, TxnKind::Expense);
        assert_eq!(draft.amount, QRIS_AMOUNT);
        assert_eq!(draft.date, QRIS_DATE);
        assert_eq!(draft.payee.as_deref(), Some("IDM INDOMARET D"));
        assert_eq!(draft.account_id, Some(kit.dompet));
        assert_eq!(draft.category_id, Some(kit.cat("Makan & Minum")));
        assert_eq!(draft.note.as_deref(), Some("BCA Pembayaran (QRIS/VA)"));
    }

    #[test]
    fn draft_uses_rule_accounts_for_transfer() {
        let mut kit = kit();
        kit.ledger
            .conn
            .execute(
                "UPDATE email_rule SET account_id = ?1, counter_account_id = ?2 WHERE name = 'BCA Tarik Tunai'",
                [kit.bca.to_string(), kit.dompet.to_string()],
            )
            .unwrap();
        assert_eq!(ingest(&mut kit, 1, TARIK), IngestOutcome::Captured(1));
        let draft = kit.ledger.capture_draft(pending(&kit)[0].id).unwrap();
        assert_eq!(draft.kind, TxnKind::Transfer);
        assert_eq!(draft.account_id, Some(kit.bca));
        assert_eq!(draft.counter_account_id, Some(kit.dompet));
        assert_eq!(draft.category_id, None);
    }

    #[test]
    fn confirm_creates_txn_and_marks_confirmed() {
        let mut kit = kit();
        let id = qris_capture(&mut kit);
        let form = expense(&kit, kit.bca, QRIS_DATE, QRIS_AMOUNT);
        let txn_id = kit.ledger.confirm_capture(id, &form).unwrap();
        assert_eq!(
            kit.ledger.balance(kit.bca).unwrap(),
            1_000_000 - QRIS_AMOUNT
        );
        let caps = kit.ledger.list_captures(CaptureStatus::Confirmed).unwrap();
        assert_eq!(caps[0].txn_id, Some(txn_id));
        assert!(pending(&kit).is_empty());
    }

    #[test]
    fn confirm_twice_is_rejected() {
        let mut kit = kit();
        let id = qris_capture(&mut kit);
        let form = expense(&kit, kit.bca, QRIS_DATE, QRIS_AMOUNT);
        kit.ledger.confirm_capture(id, &form).unwrap();
        let err = kit.ledger.confirm_capture(id, &form).unwrap_err();
        assert!(matches!(err, AppError::CaptureNotPending), "{err:?}");
        assert_eq!(txn_count(&kit), 1);
    }

    #[test]
    fn ignore_then_confirm_rejected() {
        let mut kit = kit();
        let id = qris_capture(&mut kit);
        kit.ledger.ignore_capture(id).unwrap();
        let form = expense(&kit, kit.bca, QRIS_DATE, QRIS_AMOUNT);
        let err = kit.ledger.confirm_capture(id, &form).unwrap_err();
        assert!(matches!(err, AppError::CaptureNotPending), "{err:?}");
        assert_eq!(txn_count(&kit), 0);
        assert_eq!(
            kit.ledger
                .list_captures(CaptureStatus::Ignored)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn missing_capture_is_not_found() {
        let mut kit = kit();
        let err = kit.ledger.ignore_capture(Uuid::now_v7()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }

    #[test]
    fn link_marks_confirmed_without_new_txn() {
        let mut kit = kit();
        let manual = kit
            .ledger
            .create_txn(&expense(&kit, kit.bca, QRIS_DATE, QRIS_AMOUNT))
            .unwrap();
        let id = qris_capture(&mut kit);
        kit.ledger.link_capture(id, manual).unwrap();
        assert_eq!(txn_count(&kit), 1);
        let caps = kit.ledger.list_captures(CaptureStatus::Confirmed).unwrap();
        assert_eq!(caps[0].txn_id, Some(manual));
        let err = kit.ledger.link_capture(id, manual).unwrap_err();
        assert!(matches!(err, AppError::CaptureNotPending), "{err:?}");
    }

    #[test]
    fn link_to_deleted_txn_is_not_found() {
        let mut kit = kit();
        let manual = kit
            .ledger
            .create_txn(&expense(&kit, kit.bca, QRIS_DATE, QRIS_AMOUNT))
            .unwrap();
        kit.ledger.delete_txn(manual).unwrap();
        let id = qris_capture(&mut kit);
        let err = kit.ledger.link_capture(id, manual).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
        assert_eq!(pending(&kit).len(), 1);
    }

    #[test]
    fn duplicate_candidates_within_one_day() {
        let mut kit = kit();
        let near = kit
            .ledger
            .create_txn(&expense(&kit, kit.bca, date!(2026 - 05 - 10), QRIS_AMOUNT))
            .unwrap();
        for form in [
            expense(&kit, kit.bca, date!(2026 - 05 - 11), QRIS_AMOUNT),
            expense(&kit, kit.bca, QRIS_DATE, QRIS_AMOUNT + 1),
        ] {
            kit.ledger.create_txn(&form).unwrap();
        }
        let id = qris_capture(&mut kit);
        assert_eq!(kit.ledger.duplicate_candidates(id).unwrap(), vec![near]);
    }

    #[test]
    fn duplicate_candidates_are_capped_and_sorted_by_distance() {
        let mut kit = kit();
        let exact = kit
            .ledger
            .create_txn(&expense(&kit, kit.bca, QRIS_DATE, QRIS_AMOUNT))
            .unwrap();
        for _ in 0..6 {
            let form = expense(&kit, kit.bca, date!(2026 - 05 - 08), QRIS_AMOUNT);
            kit.ledger.create_txn(&form).unwrap();
        }
        let id = qris_capture(&mut kit);
        let found = kit.ledger.duplicate_candidates(id).unwrap();
        assert_eq!(found.len(), 5);
        assert_eq!(found[0], exact);
    }

    #[test]
    fn confirm_failure_keeps_pending() {
        let mut kit = kit();
        let id = qris_capture(&mut kit);
        kit.ledger.archive_account(kit.dompet).unwrap();
        let form = expense(&kit, kit.dompet, QRIS_DATE, QRIS_AMOUNT);
        let err = kit.ledger.confirm_capture(id, &form).unwrap_err();
        assert!(matches!(err, AppError::Archived(_)), "{err:?}");
        assert_eq!(pending(&kit).len(), 1);
    }
}
