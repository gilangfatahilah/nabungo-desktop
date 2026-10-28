//! Rule editor email capture (EML-04): CRUD rule, uji rule pada email contoh, saran rule dari
//! email yang gagal dibaca, dan re-parse lokal `email_unparsed` (tanpa jaringan).
//!
//! Subjek, pengirim, dan teks email tidak pernah masuk log (K13).

use nabungo_core::{CategoryKind, TxnError};
use nabungo_mail::message::{self, MailMessage};
use nabungo_mail::rule::{CompiledRules, Outcome, Rule, RuleField, RuleKind, RuleTrace, trace};
use nabungo_mail::suggest::suggest;
use nabungo_storage::email::{self as repo, CaptureWrite, RuleWrite, UnparsedRecord};
use nabungo_storage::rusqlite::{Connection, Transaction};
use nabungo_storage::{account, category};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

use crate::capture::{CURRENCY, kind_str, to_rule};
use crate::ledger::{AppError, Ledger, active};

/// Isian rule editor: [`Rule`] tanpa id, plus status aktif.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailRuleInput {
    pub name: String,
    pub sender_pattern: String,
    pub subject_regex: Option<String>,
    pub amount_regex: String,
    pub payee_regex: Option<String>,
    pub date_regex: Option<String>,
    pub date_format: Option<String>,
    pub kind: RuleKind,
    pub account_id: Option<Uuid>,
    pub counter_account_id: Option<Uuid>,
    pub default_category_id: Option<Uuid>,
    pub priority: i64,
    pub enabled: bool,
}

impl EmailRuleInput {
    pub fn from_rule(r: Rule, enabled: bool) -> Self {
        Self {
            name: r.name,
            sender_pattern: r.sender_pattern,
            subject_regex: r.subject_regex,
            amount_regex: r.amount_regex,
            payee_regex: r.payee_regex,
            date_regex: r.date_regex,
            date_format: r.date_format,
            kind: r.kind,
            account_id: r.account_id,
            counter_account_id: r.counter_account_id,
            default_category_id: r.default_category_id,
            priority: r.priority,
            enabled,
        }
    }

    /// Nama & pengirim di-trim; pola opsional yang kosong menjadi `None`.
    fn normalized(&self) -> Rule {
        let opt = |p: &Option<String>| p.clone().filter(|p| !p.trim().is_empty());
        Rule {
            id: None,
            name: self.name.trim().to_owned(),
            sender_pattern: self.sender_pattern.trim().to_owned(),
            subject_regex: opt(&self.subject_regex),
            amount_regex: self.amount_regex.clone(),
            payee_regex: opt(&self.payee_regex),
            date_regex: opt(&self.date_regex),
            date_format: opt(&self.date_format),
            kind: self.kind,
            account_id: self.account_id,
            counter_account_id: self.counter_account_id,
            default_category_id: self.default_category_id,
            priority: self.priority,
        }
    }
}

/// Baris tab Rule: rule (id terisi), status aktif, dan jumlah capture pending-nya.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailRuleItem {
    pub rule: Rule,
    pub enabled: bool,
    pub pending: u32,
}

/// Hasil re-parse: email yang pindah dari "Gagal dibaca" dan item capture barunya.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReparseSummary {
    pub emails: u32,
    pub items: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleSaved {
    pub id: Uuid,
    pub reparsed: ReparseSummary,
}

/// Email contoh untuk uji rule. Teks tempelan tidak disimpan (K13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleSample {
    /// `message_id` baris `email_unparsed`.
    Unparsed(String),
    Pasted {
        sender: String,
        subject: String,
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleTest {
    pub trace: RuleTrace,
    /// Email gagal dibaca yang lolos rule ini sendiri (tanpa rule lain).
    pub unparsed_matches: u32,
}

/// Email tersimpan sebagai pesan untuk rule engine. `None` = raw tidak terbaca saat sync.
fn stored_message(u: UnparsedRecord) -> Option<MailMessage> {
    if u.sender.is_empty() {
        return None;
    }
    Some(MailMessage {
        sent_at: u
            .received_at
            .as_deref()
            .and_then(|t| OffsetDateTime::parse(t, &Rfc3339).ok()),
        message_id: u.message_id,
        from: u.sender,
        subject: u.subject,
        text: u.raw_snippet,
    })
}

fn invalid(field: RuleField) -> AppError {
    AppError::EmailRuleInvalid(field)
}

/// K3: pola (lewat `nabungo_mail`) lalu referensi akun/kategori.
fn check(conn: &Connection, rule: &Rule) -> Result<(), AppError> {
    nabungo_mail::rule::validate(rule).map_err(invalid)?;
    if let Some(id) = rule.account_id {
        active(account::get_account_ref(conn, id)?, "account")?;
    }
    let transfer = matches!(rule.kind, RuleKind::TransferOut | RuleKind::TransferIn);
    if let Some(id) = rule.counter_account_id {
        if !transfer {
            return Err(invalid(RuleField::CounterAccount));
        }
        if rule.account_id == Some(id) {
            return Err(TxnError::TransferSameAccount.into());
        }
        active(account::get_account_ref(conn, id)?, "account")?;
    }
    if let Some(id) = rule.default_category_id {
        let cat = active(category::get_category_ref(conn, id)?, "category")?;
        let expected = match rule.kind {
            RuleKind::Income => CategoryKind::Income,
            RuleKind::Expense => CategoryKind::Expense,
            RuleKind::TransferOut | RuleKind::TransferIn => {
                return Err(TxnError::UnexpectedCategory.into());
            }
        };
        if cat.kind != expected {
            return Err(TxnError::WrongCategoryKind { expected }.into());
        }
    }
    Ok(())
}

fn write_of<'a>(r: &'a Rule, enabled: bool) -> RuleWrite<'a> {
    RuleWrite {
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
        enabled,
    }
}

const RULE_NOT_FOUND: AppError = AppError::NotFound("email rule");

/// K6: jalankan rule aktif pada `email_unparsed`; yang terbaca pindah ke staging.
fn reparse_in(tx: &Transaction, off: UtcOffset) -> Result<ReparseSummary, AppError> {
    let rules = repo::list_active_rules(tx)?
        .into_iter()
        .map(to_rule)
        .collect::<Result<Vec<_>, _>>()?;
    let rules = CompiledRules::new(&rules);
    let mut summary = ReparseSummary::default();
    for msg in repo::list_unparsed(tx)?
        .into_iter()
        .filter_map(stored_message)
    {
        let Outcome::Parsed { rule, items } = rules.apply(&msg, off) else {
            continue;
        };
        let rule_id = rules.rule(rule).id.ok_or(RULE_NOT_FOUND)?;
        for item in &items {
            let inserted = repo::insert_capture(
                tx,
                &CaptureWrite {
                    message_id: &msg.message_id,
                    idx: item.idx,
                    rule_id,
                    date: item.date,
                    amount: item.amount,
                    currency: CURRENCY,
                    payee: item.payee.as_deref(),
                    raw_snippet: &msg.text,
                },
            )?;
            summary.items += u32::from(inserted);
        }
        repo::delete_unparsed(tx, &msg.message_id)?;
        summary.emails += 1;
    }
    Ok(summary)
}

impl Ledger {
    /// Tab Rule (K2): semua rule yang tidak dihapus, aktif maupun tidak.
    pub fn email_rules_list(&self) -> Result<Vec<EmailRuleItem>, AppError> {
        repo::list_rules(&self.conn)?
            .into_iter()
            .map(|r| {
                let pending = repo::count_pending_for_rule(&self.conn, r.id)?;
                let enabled = r.enabled;
                Ok(EmailRuleItem {
                    rule: to_rule(r)?,
                    enabled,
                    pending,
                })
            })
            .collect()
    }

    /// Simpan rule baru lalu re-parse email gagal dibaca (K7), satu SQLite transaction.
    pub fn create_email_rule(
        &mut self,
        input: &EmailRuleInput,
        off: UtcOffset,
    ) -> Result<RuleSaved, AppError> {
        let rule = input.normalized();
        let tx = self.conn.transaction()?;
        check(&tx, &rule)?;
        let id = repo::insert_rule(&tx, &write_of(&rule, input.enabled))?;
        let reparsed = reparse_in(&tx, off)?;
        tx.commit()?;
        Ok(RuleSaved { id, reparsed })
    }

    pub fn update_email_rule(
        &mut self,
        id: Uuid,
        input: &EmailRuleInput,
        off: UtcOffset,
    ) -> Result<RuleSaved, AppError> {
        let rule = input.normalized();
        let tx = self.conn.transaction()?;
        check(&tx, &rule)?;
        if !repo::update_rule(&tx, id, &write_of(&rule, input.enabled))? {
            return Err(RULE_NOT_FOUND);
        }
        let reparsed = reparse_in(&tx, off)?;
        tx.commit()?;
        Ok(RuleSaved { id, reparsed })
    }

    /// Menonaktifkan tidak me-re-parse; mengaktifkan me-re-parse (K7).
    pub fn set_email_rule_enabled(
        &mut self,
        id: Uuid,
        enabled: bool,
        off: UtcOffset,
    ) -> Result<RuleSaved, AppError> {
        let tx = self.conn.transaction()?;
        if !repo::set_rule_enabled(&tx, id, enabled)? {
            return Err(RULE_NOT_FOUND);
        }
        let reparsed = if enabled {
            reparse_in(&tx, off)?
        } else {
            ReparseSummary::default()
        };
        tx.commit()?;
        Ok(RuleSaved { id, reparsed })
    }

    /// Soft delete (K8); capture lama tetap menampilkan nama rule.
    pub fn delete_email_rule(&mut self, id: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        if !repo::soft_delete_rule(&tx, id)? {
            return Err(RULE_NOT_FOUND);
        }
        tx.commit()?;
        Ok(())
    }

    /// Undo hapus; rule yang aktif langsung me-re-parse.
    pub fn restore_email_rule(&mut self, id: Uuid, off: UtcOffset) -> Result<RuleSaved, AppError> {
        let tx = self.conn.transaction()?;
        if !repo::restore_rule(&tx, id)? {
            return Err(RULE_NOT_FOUND);
        }
        let reparsed = reparse_in(&tx, off)?;
        tx.commit()?;
        Ok(RuleSaved { id, reparsed })
    }

    /// Uji rule (K5) tanpa menulis apa pun. Email tempelan dianggap baru diterima. `rule_id` =
    /// rule yang sedang diubah (versi tersimpannya tidak ikut dihitung).
    pub fn test_email_rule(
        &self,
        input: &EmailRuleInput,
        rule_id: Option<Uuid>,
        sample: &RuleSample,
        off: UtcOffset,
    ) -> Result<RuleTest, AppError> {
        let rule = input.normalized();
        nabungo_mail::rule::validate(&rule).map_err(invalid)?;
        let msg = match sample {
            RuleSample::Unparsed(message_id) => repo::get_unparsed(&self.conn, message_id)?
                .and_then(stored_message)
                .ok_or(AppError::NotFound("email"))?,
            RuleSample::Pasted {
                sender,
                subject,
                text,
            } => message::pasted(sender, subject, text, OffsetDateTime::now_utc()),
        };
        let trace = trace(&rule, &msg, off).map_err(invalid)?;
        // Sama dengan re-parse saat disimpan: rule aktif lain (urutan prioritas) bisa lebih dulu
        // menangkap atau menahan email. Rule ini dihitung seolah aktif.
        let mut rules = repo::list_active_rules(&self.conn)?
            .into_iter()
            .filter(|r| Some(r.id) != rule_id)
            .map(to_rule)
            .collect::<Result<Vec<_>, _>>()?;
        let at = rules
            .iter()
            .position(|r| (r.priority, &rule.name) < (rule.priority, &r.name))
            .unwrap_or(rules.len());
        rules.insert(at, rule);
        let rules = CompiledRules::new(&rules);
        let mut unparsed_matches = 0;
        for other in repo::list_unparsed(&self.conn)?
            .into_iter()
            .filter_map(stored_message)
        {
            if matches!(rules.apply(&other, off), Outcome::Parsed { rule, .. } if rule == at) {
                unparsed_matches += 1;
            }
        }
        Ok(RuleTest {
            trace,
            unparsed_matches,
        })
    }

    /// Saran rule dari email gagal dibaca (K9).
    pub fn suggest_email_rule(&self, message_id: &str) -> Result<EmailRuleInput, AppError> {
        let msg = repo::get_unparsed(&self.conn, message_id)?
            .and_then(stored_message)
            .ok_or(AppError::NotFound("email"))?;
        Ok(EmailRuleInput::from_rule(suggest(&msg), true))
    }

    /// Re-parse manual (K6).
    pub fn reparse_unparsed(&mut self, off: UtcOffset) -> Result<ReparseSummary, AppError> {
        let tx = self.conn.transaction()?;
        let summary = reparse_in(&tx, off)?;
        tx.commit()?;
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::tests::{
        QRIS, QRIS_AMOUNT, UNKNOWN, WIB, ingest, kit, pending, qris_capture,
    };
    use crate::{CaptureStatus, IngestOutcome};
    use nabungo_core::{CategoryKind, TxnError};
    use nabungo_mail::defaults::default_rules;
    use nabungo_mail::rule::RuleField;
    use time::macros::date;

    fn input(name: &str) -> EmailRuleInput {
        EmailRuleInput {
            name: name.into(),
            sender_pattern: "toko.example".into(),
            subject_regex: None,
            amount_regex: r"^Total Payment : IDR ([\d.,]+)$".into(),
            payee_regex: None,
            date_regex: None,
            date_format: None,
            kind: RuleKind::Expense,
            account_id: None,
            counter_account_id: None,
            default_category_id: None,
            priority: 10,
            enabled: true,
        }
    }

    /// Email dengan Message-ID lain (isi sama).
    fn renamed(raw: &[u8], id: &str) -> Vec<u8> {
        let s = String::from_utf8(raw.to_vec()).unwrap();
        let (_, rest) = s.split_once('\n').unwrap();
        format!("Message-ID: <{id}>\n{rest}").into_bytes()
    }

    fn rule_names(kit: &crate::testkit::Kit) -> Vec<String> {
        kit.ledger
            .email_rules_list()
            .unwrap()
            .into_iter()
            .map(|r| r.rule.name)
            .collect()
    }

    fn unparsed_ids(kit: &crate::testkit::Kit) -> Vec<String> {
        kit.ledger
            .list_unparsed()
            .unwrap()
            .into_iter()
            .map(|u| u.message_id)
            .collect()
    }

    #[test]
    fn list_includes_disabled_with_pending_count() {
        let mut kit = kit();
        qris_capture(&mut kit);
        let rules = kit.ledger.email_rules_list().unwrap();
        assert_eq!(rules.len(), default_rules().len());
        let bca = rules
            .iter()
            .find(|r| r.rule.name == "BCA Pembayaran (QRIS/VA)")
            .unwrap();
        assert_eq!(bca.pending, 1);
        assert!(bca.enabled);
        let id = bca.rule.id.unwrap();
        kit.ledger.set_email_rule_enabled(id, false, WIB).unwrap();
        let rules = kit.ledger.email_rules_list().unwrap();
        assert!(
            !rules
                .iter()
                .find(|r| r.rule.id == Some(id))
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn create_rejects_invalid_regex() {
        let mut kit = kit();
        let before = rule_names(&kit).len();
        let mut bad = input("Rusak");
        bad.amount_regex = "(".into();
        assert!(matches!(
            kit.ledger.create_email_rule(&bad, WIB),
            Err(AppError::EmailRuleInvalid(RuleField::Amount))
        ));
        assert_eq!(rule_names(&kit).len(), before);
    }

    #[test]
    fn create_trims_and_drops_empty_optionals() {
        let mut kit = kit();
        let mut i = input("  Toko  ");
        i.subject_regex = Some("  ".into());
        i.payee_regex = Some(String::new());
        let id = kit.ledger.create_email_rule(&i, WIB).unwrap().id;
        let r = kit
            .ledger
            .email_rules_list()
            .unwrap()
            .into_iter()
            .find(|r| r.rule.id == Some(id))
            .unwrap()
            .rule;
        assert_eq!(r.name, "Toko");
        assert_eq!(r.subject_regex, None);
        assert_eq!(r.payee_regex, None);
    }

    #[test]
    fn create_rejects_archived_account() {
        let mut kit = kit();
        kit.ledger.archive_account(kit.dompet).unwrap();
        let mut i = input("Toko");
        i.account_id = Some(kit.dompet);
        assert!(matches!(
            kit.ledger.create_email_rule(&i, WIB),
            Err(AppError::Archived("account"))
        ));
        i.account_id = Some(Uuid::now_v7());
        assert!(matches!(
            kit.ledger.create_email_rule(&i, WIB),
            Err(AppError::NotFound("account"))
        ));
    }

    #[test]
    fn create_rejects_category_kind_mismatch() {
        let mut kit = kit();
        let mut i = input("Toko");
        i.default_category_id = Some(kit.cat("Gaji"));
        assert!(matches!(
            kit.ledger.create_email_rule(&i, WIB),
            Err(AppError::Txn(TxnError::WrongCategoryKind {
                expected: CategoryKind::Expense
            }))
        ));
        i.kind = RuleKind::TransferOut;
        i.default_category_id = Some(kit.cat("Makan & Minum"));
        assert!(matches!(
            kit.ledger.create_email_rule(&i, WIB),
            Err(AppError::Txn(TxnError::UnexpectedCategory))
        ));
    }

    #[test]
    fn create_rejects_counter_on_expense() {
        let mut kit = kit();
        let mut i = input("Toko");
        i.counter_account_id = Some(kit.dompet);
        assert!(matches!(
            kit.ledger.create_email_rule(&i, WIB),
            Err(AppError::EmailRuleInvalid(RuleField::CounterAccount))
        ));
    }

    #[test]
    fn create_rejects_counter_equal_account() {
        let mut kit = kit();
        let mut i = input("Toko");
        i.kind = RuleKind::TransferOut;
        i.account_id = Some(kit.bca);
        i.counter_account_id = Some(kit.bca);
        assert!(matches!(
            kit.ledger.create_email_rule(&i, WIB),
            Err(AppError::Txn(TxnError::TransferSameAccount))
        ));
    }

    #[test]
    fn reparse_moves_matching_unparsed() {
        let mut kit = kit();
        assert_eq!(ingest(&mut kit, 1, UNKNOWN), IngestOutcome::Unparsed);
        let suggested = kit
            .ledger
            .suggest_email_rule("promo-0001@toko.example")
            .unwrap();
        let saved = kit.ledger.create_email_rule(&suggested, WIB).unwrap();
        assert_eq!(
            saved.reparsed,
            ReparseSummary {
                emails: 1,
                items: 1
            }
        );
        assert!(unparsed_ids(&kit).is_empty());
        let items = kit.ledger.pending_captures().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].draft.amount, 50_000);
        assert_eq!(items[0].draft.date, date!(2026 - 10 - 26));
        assert_eq!(
            items[0].draft.note.as_deref(),
            Some(suggested.name.as_str())
        );
    }

    #[test]
    fn reparse_twice_is_noop() {
        let mut kit = kit();
        ingest(&mut kit, 1, UNKNOWN);
        let suggested = kit
            .ledger
            .suggest_email_rule("promo-0001@toko.example")
            .unwrap();
        kit.ledger.create_email_rule(&suggested, WIB).unwrap();
        assert_eq!(
            kit.ledger.reparse_unparsed(WIB).unwrap(),
            ReparseSummary::default()
        );
        assert_eq!(pending(&kit).len(), 1);
    }

    #[test]
    fn reparse_skips_unreadable() {
        let mut kit = kit();
        ingest(&mut kit, 1, b"bukan email");
        assert_eq!(unparsed_ids(&kit).len(), 1);
        let mut catch_all = input("Semua");
        catch_all.sender_pattern = "@".into();
        catch_all.amount_regex = r"(\d+)".into();
        kit.ledger.create_email_rule(&catch_all, WIB).unwrap();
        assert_eq!(unparsed_ids(&kit).len(), 1);
    }

    #[test]
    fn invalid_rule_save_rolls_back() {
        let mut kit = kit();
        ingest(&mut kit, 1, UNKNOWN);
        let id = kit
            .ledger
            .create_email_rule(&input("Lain"), WIB)
            .unwrap()
            .id;
        // Rule "Lain" sudah membaca email tadi; buat email baru yang belum terbaca.
        ingest(&mut kit, 2, &renamed(UNKNOWN, "promo-0002@toko.example"));
        kit.ledger.set_email_rule_enabled(id, false, WIB).unwrap();
        ingest(&mut kit, 3, &renamed(UNKNOWN, "promo-0003@toko.example"));
        let before = unparsed_ids(&kit);
        assert_eq!(before.len(), 1);
        let mut i = input("Diubah");
        i.account_id = Some(Uuid::now_v7());
        assert!(matches!(
            kit.ledger.update_email_rule(id, &i, WIB),
            Err(AppError::NotFound("account"))
        ));
        assert!(rule_names(&kit).contains(&"Lain".to_owned()));
        assert_eq!(unparsed_ids(&kit), before);
    }

    #[test]
    fn update_missing_rule_is_not_found() {
        let mut kit = kit();
        assert!(matches!(
            kit.ledger
                .update_email_rule(Uuid::now_v7(), &input("x"), WIB),
            Err(AppError::NotFound("email rule"))
        ));
    }

    #[test]
    fn rule_account_change_updates_pending_draft() {
        let mut kit = kit();
        qris_capture(&mut kit);
        let rule = kit
            .ledger
            .email_rules_list()
            .unwrap()
            .into_iter()
            .find(|r| r.rule.name == "BCA Pembayaran (QRIS/VA)")
            .unwrap();
        let before = kit.ledger.pending_captures().unwrap()[0].draft.clone();
        let mut i = EmailRuleInput::from_rule(rule.rule.clone(), rule.enabled);
        i.account_id = Some(kit.bca);
        kit.ledger
            .update_email_rule(rule.rule.id.unwrap(), &i, WIB)
            .unwrap();
        let after = kit.ledger.pending_captures().unwrap()[0].draft.clone();
        assert_eq!(after.account_id, Some(kit.bca));
        assert_eq!(after.amount, QRIS_AMOUNT);
        assert_eq!(after.payee, before.payee);
    }

    #[test]
    fn disable_rule_excludes_from_sync() {
        let mut kit = kit();
        qris_capture(&mut kit);
        let id = kit
            .ledger
            .email_rules_list()
            .unwrap()
            .into_iter()
            .find(|r| r.rule.name == "BCA Pembayaran (QRIS/VA)")
            .unwrap()
            .rule
            .id
            .unwrap();
        let saved = kit.ledger.set_email_rule_enabled(id, false, WIB).unwrap();
        assert_eq!(saved.reparsed, ReparseSummary::default());
        assert_eq!(
            ingest(&mut kit, 11, &renamed(QRIS, "qris-0002@bca.co.id")),
            IngestOutcome::Unparsed
        );
        // Aktifkan lagi: email tadi langsung terbaca.
        let saved = kit.ledger.set_email_rule_enabled(id, true, WIB).unwrap();
        assert_eq!(
            saved.reparsed,
            ReparseSummary {
                emails: 1,
                items: 1
            }
        );
    }

    #[test]
    fn delete_keeps_capture_rule_name() {
        let mut kit = kit();
        qris_capture(&mut kit);
        let id = kit
            .ledger
            .email_rules_list()
            .unwrap()
            .into_iter()
            .find(|r| r.rule.name == "BCA Pembayaran (QRIS/VA)")
            .unwrap()
            .rule
            .id
            .unwrap();
        kit.ledger.delete_email_rule(id).unwrap();
        assert!(!rule_names(&kit).contains(&"BCA Pembayaran (QRIS/VA)".to_owned()));
        let items = kit.ledger.pending_captures().unwrap();
        assert_eq!(
            items[0].draft.note.as_deref(),
            Some("BCA Pembayaran (QRIS/VA)")
        );
        assert!(matches!(
            kit.ledger.delete_email_rule(id),
            Err(AppError::NotFound("email rule"))
        ));
    }

    #[test]
    fn restore_reactivates_rule() {
        let mut kit = kit();
        let id = kit
            .ledger
            .create_email_rule(&input("Toko"), WIB)
            .unwrap()
            .id;
        kit.ledger.delete_email_rule(id).unwrap();
        ingest(&mut kit, 1, UNKNOWN);
        assert_eq!(unparsed_ids(&kit).len(), 1);
        let saved = kit.ledger.restore_email_rule(id, WIB).unwrap();
        assert_eq!(
            saved.reparsed,
            ReparseSummary {
                emails: 1,
                items: 1
            }
        );
        assert!(rule_names(&kit).contains(&"Toko".to_owned()));
        assert!(matches!(
            kit.ledger.restore_email_rule(id, WIB),
            Err(AppError::NotFound("email rule"))
        ));
    }

    #[test]
    fn test_rule_counts_unparsed_matches() {
        let mut kit = kit();
        ingest(&mut kit, 1, UNKNOWN);
        ingest(&mut kit, 2, &renamed(UNKNOWN, "promo-0002@toko.example"));
        let id = kit
            .ledger
            .email_rules_list()
            .unwrap()
            .into_iter()
            .find(|r| r.rule.name == "BCA Pembayaran (QRIS/VA)")
            .unwrap()
            .rule
            .id
            .unwrap();
        kit.ledger.set_email_rule_enabled(id, false, WIB).unwrap();
        ingest(&mut kit, 3, QRIS);
        assert_eq!(unparsed_ids(&kit).len(), 3);

        let result = kit
            .ledger
            .test_email_rule(
                &input("Toko"),
                None,
                &RuleSample::Unparsed("promo-0001@toko.example".into()),
                WIB,
            )
            .unwrap();
        assert_eq!(result.unparsed_matches, 2);
        assert_eq!(result.trace.result.unwrap()[0].amount, 50_000);
        assert_eq!(unparsed_ids(&kit).len(), 3);
        assert!(
            kit.ledger
                .list_captures(CaptureStatus::Pending)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_rule_count_follows_rule_order() {
        let mut kit = kit();
        ingest(&mut kit, 1, UNKNOWN);
        ingest(&mut kit, 2, &renamed(UNKNOWN, "promo-0002@toko.example"));
        // Rule prioritas lebih tinggi untuk pengirim yang sama gagal di nominal → email tetap gagal dibaca.
        let mut blocker = input("Blok");
        blocker.priority = 50;
        blocker.amount_regex = r"Total (Payment)".into();
        let blocker_id = kit.ledger.create_email_rule(&blocker, WIB).unwrap().id;
        let sample = RuleSample::Unparsed("promo-0001@toko.example".into());
        let count = |kit: &crate::testkit::Kit, rule: &EmailRuleInput, id| {
            kit.ledger
                .test_email_rule(rule, id, &sample, WIB)
                .unwrap()
                .unparsed_matches
        };
        assert_eq!(count(&kit, &input("Toko"), None), 0);
        let mut above = input("Toko");
        above.priority = 60;
        assert_eq!(count(&kit, &above, None), 2);
        // Mengubah rule penghalang itu sendiri: versi lama tidak ikut dihitung.
        assert_eq!(count(&kit, &input("Blok"), Some(blocker_id)), 2);
        // Rule nonaktif dihitung seolah aktif (pratinjau isi rule).
        let mut off = input("Blok");
        off.enabled = false;
        assert_eq!(count(&kit, &off, Some(blocker_id)), 2);
    }

    #[test]
    fn test_rule_with_pasted_email() {
        let kit = kit();
        let result = kit
            .ledger
            .test_email_rule(
                &input("Toko"),
                None,
                &RuleSample::Pasted {
                    sender: "promo@toko.example".into(),
                    subject: "Bayar".into(),
                    text: "Total Payment : IDR 12,500.00".into(),
                },
                WIB,
            )
            .unwrap();
        assert!(result.trace.date_from_header);
        assert_eq!(result.trace.result.unwrap()[0].amount, 12_500);
        let mut bad = input("x");
        bad.amount_regex = "(".into();
        assert!(matches!(
            kit.ledger
                .test_email_rule(&bad, None, &RuleSample::Unparsed("x".into()), WIB),
            Err(AppError::EmailRuleInvalid(RuleField::Amount))
        ));
        assert!(matches!(
            kit.ledger
                .test_email_rule(&input("x"), None, &RuleSample::Unparsed("x".into()), WIB),
            Err(AppError::NotFound("email"))
        ));
    }

    #[test]
    fn test_rule_pasted_email_normalized_like_sync() {
        let kit = kit();
        let mut rule = input("Toko");
        rule.subject_regex = Some("^Bayar Toko$".into());
        // Tempelan dari klien email: spasi ganda, tab, NBSP, CRLF, alamat huruf besar.
        let result = kit
            .ledger
            .test_email_rule(
                &rule,
                None,
                &RuleSample::Pasted {
                    sender: " Promo@TOKO.example ".into(),
                    subject: "  Bayar   Toko ".into(),
                    text: "Halo\r\n\r\n\r\n  Total   Payment\u{a0}:\tIDR 12,500.00  \r\n".into(),
                },
                WIB,
            )
            .unwrap();
        assert_eq!(result.trace.subject_ok, Some(true));
        assert_eq!(result.trace.result.unwrap()[0].amount, 12_500);
    }

    #[test]
    fn seed_on_save_email_account() {
        let mut ledger = crate::Ledger::open_in_memory().unwrap();
        assert!(ledger.email_rules_list().unwrap().is_empty());
        ledger
            .save_email_account("saya@example.com", "Finance")
            .unwrap();
        assert_eq!(
            ledger.email_rules_list().unwrap().len(),
            default_rules().len()
        );
    }
}
