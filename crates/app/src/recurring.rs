//! Transaksi terjadwal (TRX-06). Kejadian tidak dimaterialisasi: `next_date` jadwal adalah
//! kejadian tertua yang belum ditangani, dan kejadian berikutnya dihitung dari aturan.

use std::collections::HashSet;

use nabungo_core::recurrence::{Freq, Recurrence};
use nabungo_core::{Fee, FormBody, SplitLine, ValidTxn};
use nabungo_storage::recurring::{self as repo, RuleRecord, RuleWrite};
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use crate::ledger::{AppError, Ledger, resolve};
use crate::transactions::{TxnForm, form_input, normalize_tag};

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
    ) -> Result<(String, String), AppError> {
        let rule = Recurrence::new(input.freq, input.interval, input.form.date, input.until)?;
        let txn_input = form_input(&input.form)?;
        ValidTxn::new(resolve(tx, &txn_input, &HashSet::new())?)?;
        for t in &input.form.tags {
            normalize_tag(t)?;
        }
        Ok((to_template(&input.form), rule.to_rrule()))
    }

    pub fn create_rule(&mut self, input: &RuleInput) -> Result<Uuid, AppError> {
        let tx = self.conn.transaction()?;
        let (template, rrule) = Self::rule_write(&tx, input)?;
        let id = repo::insert_rule(
            &tx,
            &RuleWrite {
                template: &template,
                rrule: &rrule,
                next_date: input.form.date,
                auto_post: input.auto_post,
            },
        )?;
        tx.commit()?;
        Ok(id)
    }

    /// Ganti template/aturan; `next_date` = tanggal form dan jadwal aktif lagi.
    pub fn update_rule(&mut self, id: Uuid, input: &RuleInput) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        let (template, rrule) = Self::rule_write(&tx, input)?;
        let w = RuleWrite {
            template: &template,
            rrule: &rrule,
            next_date: input.form.date,
            auto_post: input.auto_post,
        };
        if !repo::update_rule(&tx, id, &w)? {
            return Err(AppError::NotFound("recurring_rule"));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Kit;
    use nabungo_core::recurrence::RecurrenceError;
    use nabungo_core::{Fee, FormBody, SplitLine};
    use nabungo_storage::{now_utc, recurring as repo};
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
        let mut input = monthly(expense_form(&kit, date!(2026 - 09 - 25), 10_000));
        input.until = Some(date!(2026 - 09 - 25));
        let id = kit.ledger.create_rule(&input).unwrap();
        {
            let tx = kit.ledger.conn.transaction().unwrap();
            repo::advance_rule(&tx, id, None, &now_utc()).unwrap();
            tx.commit().unwrap();
        }
        let row = &kit.ledger.list_rules(TODAY).unwrap()[0];
        assert!(row.ended);
        assert!(row.due.is_empty());
        input.until = Some(date!(2026 - 12 - 31));
        kit.ledger.update_rule(id, &input).unwrap();
        let row = &kit.ledger.list_rules(TODAY).unwrap()[0];
        assert!(!row.ended);
        assert_eq!(row.due, vec![date!(2026 - 09 - 25)]);
        assert_eq!(row.until, Some(date!(2026 - 12 - 31)));
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
}
