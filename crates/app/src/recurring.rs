//! Transaksi terjadwal (TRX-06). Kejadian tidak dimaterialisasi: `next_date` jadwal adalah
//! kejadian tertua yang belum ditangani, dan kejadian berikutnya dihitung dari aturan.

use std::collections::HashSet;

use nabungo_core::recurrence::{Freq, Recurrence};
use nabungo_core::{Fee, FormBody, SplitLine, ValidTxn};
use nabungo_storage::now_utc;
use nabungo_storage::recurring::{self as repo, RuleRecord, RuleWrite};
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use crate::ledger::{AppError, Ledger, resolve};
use crate::transactions::{TxnForm, form_input, normalize_tag, write_txn};

/// Batas kejadian jatuh tempo per jadwal per panggilan (daftar & konfirmasi semua).
pub const MAX_CATCH_UP: usize = 366;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleInput {
    /// `date` = tanggal mulai (buat) atau kejadian berikutnya (ubah).
    pub form: TxnForm,
    pub freq: Freq,
    pub interval: u8,
    pub until: Option<Date>,
    pub auto_post: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleRow {
    pub id: Uuid,
    /// Template dengan `date` = `next_date`.
    pub form: TxnForm,
    pub freq: Freq,
    pub interval: u8,
    pub until: Option<Date>,
    pub auto_post: bool,
    pub ended: bool,
    /// Kejadian ≤ hari ini, urut naik, maksimal [`MAX_CATCH_UP`].
    pub due: Vec<Date>,
}

/// Format JSON kolom `recurring_rule.template` (versi 1). ID disimpan sebagai teks.
#[derive(Debug, Serialize, Deserialize)]
struct TemplateJson {
    v: u8,
    kind: String,
    account_id: Uuid,
    to_account_id: Option<Uuid>,
    amount: i64,
    splits: Vec<SplitJson>,
    fee: Option<FeeJson>,
    payee: Option<String>,
    note: Option<String>,
    tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SplitJson {
    category_id: Uuid,
    amount: i64,
    memo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeeJson {
    amount: i64,
    category_id: Uuid,
}

fn to_template(form: &TxnForm) -> String {
    let splits = |s: &[SplitLine]| {
        s.iter()
            .map(|l| SplitJson {
                category_id: l.category_id,
                amount: l.amount,
                memo: l.memo.clone(),
            })
            .collect()
    };
    let (kind, account_id, to_account_id, amount, splits, fee) = match &form.body {
        FormBody::Income {
            account_id,
            amount,
            splits: s,
        } => ("income", *account_id, None, *amount, splits(s), None),
        FormBody::Expense {
            account_id,
            amount,
            splits: s,
        } => ("expense", *account_id, None, *amount, splits(s), None),
        FormBody::Transfer {
            from,
            to,
            amount,
            fee,
        } => (
            "transfer",
            *from,
            Some(*to),
            *amount,
            vec![],
            fee.as_ref().map(|f| FeeJson {
                amount: f.amount,
                category_id: f.category_id,
            }),
        ),
    };
    let json = TemplateJson {
        v: 1,
        kind: kind.into(),
        account_id,
        to_account_id,
        amount,
        splits,
        fee,
        payee: form.payee.clone(),
        note: form.note.clone(),
        tags: form.tags.clone(),
    };
    serde_json::to_string(&json).expect("template is always serializable")
}

/// Template jadwal sebagai form dengan tanggal `date`.
pub(crate) fn template_form(rec: &RuleRecord, date: Date) -> Result<TxnForm, AppError> {
    let corrupt = || {
        AppError::Storage(nabungo_storage::StorageError::Corrupt(format!(
            "recurring_rule {} template",
            rec.id
        )))
    };
    let t: TemplateJson = serde_json::from_str(&rec.template).map_err(|_| corrupt())?;
    if t.v != 1 {
        return Err(corrupt());
    }
    let splits = t
        .splits
        .into_iter()
        .map(|s| SplitLine {
            category_id: s.category_id,
            amount: s.amount,
            memo: s.memo,
        })
        .collect();
    let body = match (t.kind.as_str(), t.to_account_id) {
        ("income", _) => FormBody::Income {
            account_id: t.account_id,
            amount: t.amount,
            splits,
        },
        ("expense", _) => FormBody::Expense {
            account_id: t.account_id,
            amount: t.amount,
            splits,
        },
        ("transfer", Some(to)) => FormBody::Transfer {
            from: t.account_id,
            to,
            amount: t.amount,
            fee: t.fee.map(|f| Fee {
                amount: f.amount,
                category_id: f.category_id,
            }),
        },
        _ => return Err(corrupt()),
    };
    Ok(TxnForm {
        body,
        date,
        payee: t.payee,
        note: t.note,
        tags: t.tags,
    })
}

pub(crate) fn parse_rule(rec: &RuleRecord) -> Result<Recurrence, AppError> {
    Ok(Recurrence::parse(&rec.rrule)?)
}

/// Kejadian `from`, `next_after(from)`, … selama ≤ `today`, maksimal [`MAX_CATCH_UP`].
pub(crate) fn due_dates(rule: &Recurrence, from: Date, today: Date) -> Vec<Date> {
    let mut out = Vec::new();
    let mut cur = Some(from);
    while let Some(d) = cur.filter(|d| *d <= today && out.len() < MAX_CATCH_UP) {
        out.push(d);
        cur = rule.next_after(d);
    }
    out
}

impl Ledger {
    /// Validasi template (seperti mencatat transaksi di tanggal mulai, tanpa menyimpan).
    fn rule_write(
        tx: &nabungo_storage::rusqlite::Transaction,
        input: &RuleInput,
        old: Option<&Recurrence>,
    ) -> Result<(String, Recurrence), AppError> {
        let mut rule = Recurrence::new(input.freq, input.interval, input.form.date, input.until)?;
        if let Some(old) = old {
            rule = rule.keep_day_of(input.form.date, old);
        }
        let txn_input = form_input(&input.form)?;
        ValidTxn::new(resolve(tx, &txn_input, &HashSet::new())?)?;
        for t in &input.form.tags {
            normalize_tag(t)?;
        }
        Ok((to_template(&input.form), rule))
    }

    pub fn create_rule(&mut self, input: &RuleInput) -> Result<Uuid, AppError> {
        let tx = self.conn.transaction()?;
        let (template, rule) = Self::rule_write(&tx, input, None)?;
        let id = repo::insert_rule(
            &tx,
            &RuleWrite {
                template: &template,
                rrule: &rule.to_rrule(),
                next_date: input.form.date,
                auto_post: input.auto_post,
            },
        )?;
        tx.commit()?;
        Ok(id)
    }

    /// Ganti template/aturan; `next_date` = tanggal form dan jadwal aktif lagi.
    ///
    /// Form edit diisi dari `next_date`, jadi dua kasus memakai data jadwal lama: tanggal
    /// akhir bulan yang ter-clamp mempertahankan anchor lama (tgl 31 tetap 31), dan pada
    /// jadwal yang sudah selesai, `next_date` adalah kejadian terakhir yang sudah ditangani
    /// sehingga mulai dari kejadian sesudahnya (tidak dicatat dua kali).
    pub fn update_rule(&mut self, id: Uuid, input: &RuleInput) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        let old = repo::get_rule(&tx, id)?.ok_or(AppError::NotFound("recurring_rule"))?;
        let old_rule = parse_rule(&old).ok();
        let (template, rule) = Self::rule_write(&tx, input, old_rule.as_ref())?;
        let handled = old.ended_at.is_some() && input.form.date == old.next_date;
        let next = if handled {
            rule.next_after(input.form.date)
        } else {
            Some(input.form.date)
        };
        let w = RuleWrite {
            template: &template,
            rrule: &rule.to_rrule(),
            next_date: next.unwrap_or(input.form.date),
            auto_post: input.auto_post,
        };
        repo::update_rule(&tx, id, &w)?;
        if next.is_none() {
            repo::advance_rule(&tx, id, None, &now_utc())?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Transaksi yang sudah tercatat tetap ada.
    pub fn delete_rule(&mut self, id: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        if !repo::delete_rule(&tx, id)? {
            return Err(AppError::NotFound("recurring_rule"));
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_rules(&self, today: Date) -> Result<Vec<RuleRow>, AppError> {
        repo::list_rules(&self.conn)?
            .into_iter()
            .map(|rec| {
                let rule = parse_rule(&rec)?;
                let ended = rec.ended_at.is_some();
                Ok(RuleRow {
                    id: rec.id,
                    form: template_form(&rec, rec.next_date)?,
                    freq: rule.freq(),
                    interval: rule.interval(),
                    until: rule.until(),
                    auto_post: rec.auto_post,
                    ended,
                    due: if ended {
                        vec![]
                    } else {
                        due_dates(&rule, rec.next_date, today)
                    },
                })
            })
            .collect()
    }
}

/// Jadwal aktif dengan `next_date == date`; selain itu [`AppError::OccurrenceStale`].
fn pending(
    tx: &nabungo_storage::rusqlite::Transaction,
    id: Uuid,
    date: Date,
) -> Result<(RuleRecord, Recurrence), AppError> {
    let rec = repo::get_rule(tx, id)?.ok_or(AppError::NotFound("recurring_rule"))?;
    if rec.ended_at.is_some() || rec.next_date != date {
        return Err(AppError::OccurrenceStale);
    }
    let rule = parse_rule(&rec)?;
    Ok((rec, rule))
}

/// Catat kejadian `date` (form diedit atau template) lalu majukan jadwal.
fn post_occurrence(
    tx: &nabungo_storage::rusqlite::Transaction,
    id: Uuid,
    date: Date,
    form: Option<&TxnForm>,
) -> Result<Uuid, AppError> {
    let (rec, rule) = pending(tx, id, date)?;
    let form = match form {
        Some(f) => f.clone(),
        None => template_form(&rec, date)?,
    };
    let txn = write_txn(tx, None, &form_input(&form)?, &HashSet::new())?;
    repo::link_txn(tx, txn, id)?;
    repo::advance_rule(tx, id, rule.next_after(date), &now_utc())?;
    Ok(txn)
}

impl Ledger {
    /// `form = None` → template dengan `date`. Mensyaratkan jadwal aktif dan `date == next_date`.
    pub fn confirm_occurrence(
        &mut self,
        id: Uuid,
        date: Date,
        form: Option<&TxnForm>,
    ) -> Result<Uuid, AppError> {
        let tx = self.conn.transaction()?;
        let txn = post_occurrence(&tx, id, date, form)?;
        tx.commit()?;
        Ok(txn)
    }

    pub fn skip_occurrence(&mut self, id: Uuid, date: Date) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        let (_, rule) = pending(&tx, id, date)?;
        repo::advance_rule(&tx, id, rule.next_after(date), &now_utc())?;
        tx.commit()?;
        Ok(())
    }

    /// Semua kejadian ≤ `today` (maks [`MAX_CATCH_UP`]) dalam satu transaction: gagal satu, batal semua.
    pub fn confirm_all_due(&mut self, id: Uuid, today: Date) -> Result<u32, AppError> {
        let tx = self.conn.transaction()?;
        let n = post_all_due(&tx, id, today)?;
        tx.commit()?;
        Ok(n)
    }

    /// Satu transaction per jadwal; jadwal yang gagal di-log dan dilewati.
    pub fn post_auto_due(&mut self, today: Date) -> u32 {
        let rules = match repo::list_rules(&self.conn) {
            Ok(rules) => rules,
            Err(e) => {
                tracing::warn!(error = %e, "gagal memuat jadwal auto-post");
                return 0;
            }
        };
        let mut total = 0;
        for rec in rules
            .into_iter()
            .filter(|r| r.auto_post && r.ended_at.is_none() && r.next_date <= today)
        {
            let res = self
                .conn
                .transaction()
                .map_err(AppError::from)
                .and_then(|tx| {
                    let n = post_all_due(&tx, rec.id, today)?;
                    tx.commit()?;
                    Ok(n)
                });
            match res {
                Ok(n) => total += n,
                Err(e) => tracing::warn!(rule = %rec.id, error = %e, "auto-post jadwal gagal"),
            }
        }
        total
    }
}

fn post_all_due(
    tx: &nabungo_storage::rusqlite::Transaction,
    id: Uuid,
    today: Date,
) -> Result<u32, AppError> {
    let rec = repo::get_rule(tx, id)?.ok_or(AppError::NotFound("recurring_rule"))?;
    if rec.ended_at.is_some() {
        return Ok(0);
    }
    let dates = due_dates(&parse_rule(&rec)?, rec.next_date, today);
    for &d in &dates {
        post_occurrence(tx, id, d, None)?;
    }
    Ok(dates.len() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Kit;
    use nabungo_core::recurrence::RecurrenceError;
    use nabungo_core::{Fee, FormBody, SplitLine};
    use time::macros::date;

    const TODAY: Date = date!(2026 - 10 - 21);

    pub(super) fn expense_form(kit: &Kit, date: Date, amount: i64) -> TxnForm {
        TxnForm {
            body: FormBody::Expense {
                account_id: kit.bca,
                amount,
                splits: vec![SplitLine {
                    category_id: kit.cat("Listrik"),
                    amount,
                    memo: None,
                }],
            },
            date,
            payee: Some("PLN".into()),
            note: None,
            tags: vec![],
        }
    }

    pub(super) fn monthly(form: TxnForm) -> RuleInput {
        RuleInput {
            form,
            freq: Freq::Monthly,
            interval: 1,
            until: None,
            auto_post: false,
        }
    }

    #[test]
    fn template_round_trip() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let makan = kit.cat("Internet");
        let mut split = expense_form(&kit, date!(2026 - 11 - 01), 150_000);
        split.body = FormBody::Expense {
            account_id: kit.bca,
            amount: 150_000,
            splits: vec![
                SplitLine {
                    category_id: kit.cat("Listrik"),
                    amount: 100_000,
                    memo: Some("listrik".into()),
                },
                SplitLine {
                    category_id: makan,
                    amount: 50_000,
                    memo: None,
                },
            ],
        };
        split.tags = vec!["rumah".into()];
        split.note = Some("bulanan".into());
        let transfer = TxnForm {
            body: FormBody::Transfer {
                from: kit.bca,
                to: kit.dompet,
                amount: 200_000,
                fee: Some(Fee {
                    amount: 2_500,
                    category_id: kit.cat("Biaya admin"),
                }),
            },
            date: date!(2026 - 11 - 02),
            payee: None,
            note: None,
            tags: vec![],
        };
        for form in [split, transfer] {
            let id = kit.ledger.create_rule(&monthly(form.clone())).unwrap();
            let row = kit
                .ledger
                .list_rules(TODAY)
                .unwrap()
                .into_iter()
                .find(|r| r.id == id)
                .unwrap();
            assert_eq!(row.form, form);
            assert_eq!(row.freq, Freq::Monthly);
            assert_eq!(row.interval, 1);
            assert!(!row.ended);
        }
    }

    #[test]
    fn create_validates_template() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let form = expense_form(&kit, date!(2026 - 11 - 01), 10_000);
        let mut bad = monthly(form.clone());
        bad.interval = 0;
        assert!(matches!(
            kit.ledger.create_rule(&bad),
            Err(AppError::Recurrence(RecurrenceError::Interval))
        ));
        let mut bad = monthly(form.clone());
        bad.until = Some(date!(2026 - 10 - 31));
        assert!(matches!(
            kit.ledger.create_rule(&bad),
            Err(AppError::Recurrence(RecurrenceError::UntilBeforeStart))
        ));
        kit.ledger.archive_account(kit.bca).unwrap();
        assert!(matches!(
            kit.ledger.create_rule(&monthly(form)),
            Err(AppError::Archived("account"))
        ));
        assert!(kit.ledger.list_rules(TODAY).unwrap().is_empty());
    }

    #[test]
    fn due_lists_past_occurrences() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let past = expense_form(&kit, date!(2026 - 07 - 25), 10_000);
        let id = kit.ledger.create_rule(&monthly(past)).unwrap();
        let future = expense_form(&kit, date!(2026 - 11 - 01), 10_000);
        let later = kit.ledger.create_rule(&monthly(future)).unwrap();
        let rows = kit.ledger.list_rules(TODAY).unwrap();
        let row = rows.iter().find(|r| r.id == id).unwrap();
        assert_eq!(
            row.due,
            vec![
                date!(2026 - 07 - 25),
                date!(2026 - 08 - 25),
                date!(2026 - 09 - 25)
            ]
        );
        let row = rows.iter().find(|r| r.id == later).unwrap();
        assert!(row.due.is_empty());
        assert_eq!(row.form.date, date!(2026 - 11 - 01));
    }

    #[test]
    fn due_list_is_capped() {
        let mut kit = Kit::new(1, date!(2024 - 01 - 01));
        let mut input = monthly(expense_form(&kit, date!(2024 - 01 - 01), 1_000));
        input.freq = Freq::Daily;
        kit.ledger.create_rule(&input).unwrap();
        let rows = kit.ledger.list_rules(TODAY).unwrap();
        assert_eq!(rows[0].due.len(), MAX_CATCH_UP);
    }

    #[test]
    fn update_reactivates_ended_rule() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let mut input = monthly(expense_form(&kit, date!(2026 - 08 - 25), 10_000));
        input.until = Some(date!(2026 - 09 - 25));
        let id = kit.ledger.create_rule(&input).unwrap();
        assert_eq!(kit.ledger.confirm_all_due(id, TODAY).unwrap(), 2);
        let r = row(&kit, id);
        assert!(r.ended);
        assert!(r.due.is_empty());
        // Form edit diisi dari daftar: tanggal = next_date (kejadian terakhir yang sudah tercatat).
        input.form.date = r.form.date;
        kit.ledger.update_rule(id, &input).unwrap();
        assert!(
            row(&kit, id).ended,
            "UNTIL tidak diperpanjang: tetap selesai"
        );
        input.until = Some(date!(2026 - 12 - 31));
        kit.ledger.update_rule(id, &input).unwrap();
        let r = row(&kit, id);
        assert!(!r.ended);
        assert_eq!(r.form.date, date!(2026 - 10 - 25));
        assert!(r.due.is_empty());
        assert_eq!(r.until, Some(date!(2026 - 12 - 31)));
        assert_eq!(
            rule_txns(&kit, id),
            vec![date!(2026 - 08 - 25), date!(2026 - 09 - 25)]
        );
    }

    #[test]
    fn update_at_clamped_date_keeps_month_end_day() {
        let mut kit = Kit::new(1, date!(2025 - 01 - 01));
        let mut input = monthly(expense_form(&kit, date!(2026 - 01 - 31), 10_000));
        let id = kit.ledger.create_rule(&input).unwrap();
        kit.ledger
            .skip_occurrence(id, date!(2026 - 01 - 31))
            .unwrap();
        assert_eq!(row(&kit, id).form.date, date!(2026 - 02 - 28));
        // Ubah nominal saja; tanggal form = next_date yang ter-clamp.
        input.form = expense_form(&kit, date!(2026 - 02 - 28), 12_000);
        kit.ledger.update_rule(id, &input).unwrap();
        kit.ledger
            .skip_occurrence(id, date!(2026 - 02 - 28))
            .unwrap();
        assert_eq!(row(&kit, id).form.date, date!(2026 - 03 - 31));
        // Tanggal diubah sengaja (bukan akhir bulan) → anchor baru.
        input.form = expense_form(&kit, date!(2026 - 04 - 15), 12_000);
        kit.ledger.update_rule(id, &input).unwrap();
        kit.ledger
            .skip_occurrence(id, date!(2026 - 04 - 15))
            .unwrap();
        assert_eq!(row(&kit, id).form.date, date!(2026 - 05 - 15));
    }

    #[test]
    fn delete_missing_rule() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        assert!(matches!(
            kit.ledger.delete_rule(Uuid::now_v7()),
            Err(AppError::NotFound("recurring_rule"))
        ));
        let input = monthly(expense_form(&kit, date!(2026 - 11 - 01), 10_000));
        assert!(matches!(
            kit.ledger.update_rule(Uuid::now_v7(), &input),
            Err(AppError::NotFound("recurring_rule"))
        ));
        let id = kit.ledger.create_rule(&input).unwrap();
        kit.ledger.delete_rule(id).unwrap();
        assert!(kit.ledger.list_rules(TODAY).unwrap().is_empty());
    }

    fn rule_txns(kit: &Kit, id: Uuid) -> Vec<Date> {
        let mut stmt = kit
            .ledger
            .conn
            .prepare(
                "SELECT date FROM txn WHERE recurring_rule_id = ?1 AND deleted_at IS NULL ORDER BY date",
            )
            .unwrap();
        stmt.query_map([id.to_string()], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|d| crate::parse_date(&d.unwrap()).unwrap())
            .collect()
    }

    fn row(kit: &Kit, id: Uuid) -> RuleRow {
        kit.ledger
            .list_rules(TODAY)
            .unwrap()
            .into_iter()
            .find(|r| r.id == id)
            .unwrap()
    }

    #[test]
    fn confirm_posts_and_advances() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let before = kit.ledger.balance(kit.bca).unwrap();
        let id = kit
            .ledger
            .create_rule(&monthly(expense_form(&kit, date!(2026 - 09 - 25), 150_000)))
            .unwrap();
        let txn = kit
            .ledger
            .confirm_occurrence(id, date!(2026 - 09 - 25), None)
            .unwrap();
        assert_eq!(kit.ledger.get_txn(txn).unwrap().date, date!(2026 - 09 - 25));
        assert_eq!(rule_txns(&kit, id), vec![date!(2026 - 09 - 25)]);
        assert_eq!(kit.ledger.balance(kit.bca).unwrap(), before - 150_000);
        let r = row(&kit, id);
        assert!(r.due.is_empty());
        assert_eq!(r.form.date, date!(2026 - 10 - 25));
    }

    #[test]
    fn confirm_twice_is_stale() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let id = kit
            .ledger
            .create_rule(&monthly(expense_form(&kit, date!(2026 - 09 - 25), 10_000)))
            .unwrap();
        kit.ledger
            .confirm_occurrence(id, date!(2026 - 09 - 25), None)
            .unwrap();
        assert!(matches!(
            kit.ledger
                .confirm_occurrence(id, date!(2026 - 09 - 25), None),
            Err(AppError::OccurrenceStale)
        ));
        assert_eq!(rule_txns(&kit, id).len(), 1);
    }

    #[test]
    fn confirm_edited_form() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let before = kit.ledger.balance(kit.bca).unwrap();
        let id = kit
            .ledger
            .create_rule(&monthly(expense_form(&kit, date!(2026 - 09 - 25), 150_000)))
            .unwrap();
        let edited = expense_form(&kit, date!(2026 - 09 - 27), 175_000);
        let txn = kit
            .ledger
            .confirm_occurrence(id, date!(2026 - 09 - 25), Some(&edited))
            .unwrap();
        assert_eq!(kit.ledger.get_txn(txn).unwrap().date, date!(2026 - 09 - 27));
        assert_eq!(kit.ledger.balance(kit.bca).unwrap(), before - 175_000);
        let r = row(&kit, id);
        assert_eq!(r.form.date, date!(2026 - 10 - 25));
        assert_eq!(
            r.form,
            expense_form(&kit, date!(2026 - 10 - 25), 150_000),
            "template unchanged"
        );
    }

    #[test]
    fn skip_advances_without_txn() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let id = kit
            .ledger
            .create_rule(&monthly(expense_form(&kit, date!(2026 - 09 - 25), 10_000)))
            .unwrap();
        assert!(matches!(
            kit.ledger.skip_occurrence(id, date!(2026 - 09 - 26)),
            Err(AppError::OccurrenceStale)
        ));
        kit.ledger
            .skip_occurrence(id, date!(2026 - 09 - 25))
            .unwrap();
        assert!(rule_txns(&kit, id).is_empty());
        assert_eq!(row(&kit, id).form.date, date!(2026 - 10 - 25));
    }

    #[test]
    fn confirm_with_archived_account_keeps_next_date() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let id = kit
            .ledger
            .create_rule(&monthly(expense_form(&kit, date!(2026 - 09 - 25), 10_000)))
            .unwrap();
        kit.ledger.archive_account(kit.bca).unwrap();
        assert!(matches!(
            kit.ledger
                .confirm_occurrence(id, date!(2026 - 09 - 25), None),
            Err(AppError::Archived("account"))
        ));
        assert!(rule_txns(&kit, id).is_empty());
        assert_eq!(row(&kit, id).form.date, date!(2026 - 09 - 25));
    }

    #[test]
    fn last_occurrence_ends_rule() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let mut input = monthly(expense_form(&kit, date!(2026 - 09 - 25), 10_000));
        input.until = Some(date!(2026 - 10 - 25));
        let id = kit.ledger.create_rule(&input).unwrap();
        for d in [date!(2026 - 09 - 25), date!(2026 - 10 - 25)] {
            kit.ledger.confirm_occurrence(id, d, None).unwrap();
        }
        let r = row(&kit, id);
        assert!(r.ended);
        assert!(r.due.is_empty());
        assert!(matches!(
            kit.ledger
                .confirm_occurrence(id, date!(2026 - 11 - 25), None),
            Err(AppError::OccurrenceStale)
        ));
    }

    #[test]
    fn confirm_all_caps_and_continues() {
        let mut kit = Kit::new(1, date!(2025 - 01 - 01));
        let mut input = monthly(expense_form(&kit, date!(2025 - 01 - 01), 1_000));
        input.freq = Freq::Daily;
        let id = kit.ledger.create_rule(&input).unwrap();
        assert_eq!(kit.ledger.confirm_all_due(id, TODAY).unwrap(), 366);
        assert!(!row(&kit, id).due.is_empty());
        let rest = kit.ledger.confirm_all_due(id, TODAY).unwrap();
        let days = (TODAY - date!(2025 - 01 - 01)).whole_days() + 1;
        assert_eq!(i64::from(366 + rest), days);
        assert_eq!(rule_txns(&kit, id).len() as i64, days);
        assert_eq!(kit.ledger.confirm_all_due(id, TODAY).unwrap(), 0);
    }

    #[test]
    fn confirm_all_is_atomic() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let id = kit
            .ledger
            .create_rule(&monthly(expense_form(&kit, date!(2026 - 07 - 25), 10_000)))
            .unwrap();
        kit.ledger.archive_category(kit.cat("Listrik")).unwrap();
        assert!(kit.ledger.confirm_all_due(id, TODAY).is_err());
        assert!(rule_txns(&kit, id).is_empty());
        assert_eq!(row(&kit, id).due.len(), 3);
    }

    #[test]
    fn auto_post_skips_failing_rule() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let mut a = monthly(expense_form(&kit, date!(2026 - 08 - 25), 10_000));
        a.auto_post = true;
        let a = kit.ledger.create_rule(&a).unwrap();
        let mut b_form = expense_form(&kit, date!(2026 - 08 - 25), 10_000);
        b_form.body = FormBody::Expense {
            account_id: kit.dompet,
            amount: 10_000,
            splits: vec![SplitLine {
                category_id: kit.cat("Listrik"),
                amount: 10_000,
                memo: None,
            }],
        };
        let mut b = monthly(b_form);
        b.auto_post = true;
        let b = kit.ledger.create_rule(&b).unwrap();
        let c = kit
            .ledger
            .create_rule(&monthly(expense_form(&kit, date!(2026 - 08 - 25), 10_000)))
            .unwrap();
        kit.ledger.archive_account(kit.dompet).unwrap();
        assert_eq!(kit.ledger.post_auto_due(TODAY), 2);
        assert_eq!(rule_txns(&kit, a).len(), 2);
        assert!(row(&kit, a).due.is_empty());
        assert_eq!(row(&kit, b).due.len(), 2);
        assert_eq!(row(&kit, c).due.len(), 2);
    }

    #[test]
    fn confirm_alerts() {
        let mut kit = Kit::new(1, date!(2026 - 01 - 01));
        let listrik = kit.cat("Listrik");
        kit.ledger
            .set_budget(
                listrik,
                nabungo_core::BudgetPeriod::parse("2026-09").unwrap(),
                Some(100_000),
            )
            .unwrap();
        let id = kit
            .ledger
            .create_rule(&monthly(expense_form(&kit, date!(2026 - 09 - 25), 150_000)))
            .unwrap();
        let (_, alerts) = kit
            .ledger
            .confirm_occurrence_with_alerts(id, date!(2026 - 09 - 25), None)
            .unwrap();
        assert!(!alerts.is_empty());
    }
}
