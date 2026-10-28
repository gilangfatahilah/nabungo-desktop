//! Peringatan budget (BUD-04): kategori ber-budget yang levelnya naik ke
//! `Warning` (≥ 80%) atau `Exceeded` (≥ 100%) karena transaksi yang disimpan.

use std::collections::{HashMap, HashSet};

use nabungo_core::{BudgetLevel, BudgetPeriod, form_postings};
use nabungo_storage::recurring as repo;
use time::Date;
use uuid::Uuid;

use crate::TxnForm;
use crate::budget::{budget_scope, overview_scoped};
use crate::ledger::{AppError, Ledger};
use crate::recurring::template_form;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetAlert {
    pub category_id: Uuid,
    pub period: BudgetPeriod,
    pub level: BudgetLevel,
    pub percent: Option<u32>,
}

type Levels = HashMap<(Uuid, BudgetPeriod), BudgetLevel>;

/// Kategori yang levelnya dihitung (lihat [`budget_scope`]) dan level sebelum simpan.
type Before = (HashSet<Uuid>, Levels);

/// Kategori posting form; form yang tidak valid tidak tersimpan, jadi tidak perlu alert.
fn form_categories(form: &TxnForm) -> Vec<Uuid> {
    form_postings(&form.body)
        .map(|ps| ps.into_iter().filter_map(|p| p.category_id).collect())
        .unwrap_or_default()
}

impl Ledger {
    pub fn create_txn_with_alerts(
        &mut self,
        form: &TxnForm,
    ) -> Result<(Uuid, Vec<BudgetAlert>), AppError> {
        let periods = vec![self.period_of(form.date)?];
        let before = self.budget_levels(&periods, form_categories(form));
        let id = self.create_txn(form)?;
        Ok((id, self.alerts_after_save(&periods, before)))
    }

    pub fn confirm_capture_with_alerts(
        &mut self,
        id: Uuid,
        form: &TxnForm,
    ) -> Result<(Uuid, Vec<BudgetAlert>), AppError> {
        let periods = vec![self.period_of(form.date)?];
        let before = self.budget_levels(&periods, form_categories(form));
        let txn = self.confirm_capture(id, form)?;
        Ok((txn, self.alerts_after_save(&periods, before)))
    }

    /// Periode = tanggal form (bila diedit) atau tanggal kejadian.
    pub fn confirm_occurrence_with_alerts(
        &mut self,
        id: Uuid,
        date: Date,
        form: Option<&TxnForm>,
    ) -> Result<(Uuid, Vec<BudgetAlert>), AppError> {
        let periods = vec![self.period_of(form.map_or(date, |f| f.date))?];
        let before = self
            .occurrence_categories(id, date, form)
            .and_then(|cats| self.budget_levels(&periods, cats));
        let txn = self.confirm_occurrence(id, date, form)?;
        Ok((txn, self.alerts_after_save(&periods, before)))
    }

    /// Memeriksa periode tanggal lama dan tanggal baru; periode yang realisasinya
    /// turun tidak memberi alert.
    pub fn update_txn_with_alerts(
        &mut self,
        id: Uuid,
        form: &TxnForm,
    ) -> Result<Vec<BudgetAlert>, AppError> {
        let old = self.get_txn(id)?;
        let mut periods = vec![self.period_of(old.date)?, self.period_of(form.date)?];
        periods.sort();
        periods.dedup();
        let touched = old
            .postings
            .iter()
            .filter_map(|p| p.category_id)
            .chain(form_categories(form));
        let before = self.budget_levels(&periods, touched);
        self.update_txn(id, form)?;
        Ok(self.alerts_after_save(&periods, before))
    }

    /// Transaksi sudah tersimpan: kegagalan menghitung alert tidak boleh membuat
    /// penyimpanan tampak gagal (user akan mengulang dan mendobel transaksi).
    fn alerts_after_save(
        &self,
        periods: &[BudgetPeriod],
        before: Result<Before, AppError>,
    ) -> Vec<BudgetAlert> {
        match before.and_then(|(scope, b)| self.budget_alerts(periods, &scope, &b)) {
            Ok(alerts) => alerts,
            Err(e) => {
                tracing::warn!(error = %e, "gagal menghitung peringatan budget");
                Vec::new()
            }
        }
    }

    /// Kategori template (atau form editan) kejadian yang akan dikonfirmasi.
    fn occurrence_categories(
        &self,
        id: Uuid,
        date: Date,
        form: Option<&TxnForm>,
    ) -> Result<Vec<Uuid>, AppError> {
        if let Some(f) = form {
            return Ok(form_categories(f));
        }
        match repo::get_rule(&self.conn, id)? {
            Some(rec) => Ok(form_categories(&template_form(&rec, date)?)),
            None => Ok(Vec::new()),
        }
    }

    /// Level sebelum simpan, hanya untuk subtree kategori yang disentuh: kategori lain
    /// tidak berubah realisasinya, jadi tidak mungkin naik level.
    fn budget_levels(
        &self,
        periods: &[BudgetPeriod],
        touched: impl IntoIterator<Item = Uuid>,
    ) -> Result<Before, AppError> {
        let scope = budget_scope(&self.conn, touched)?;
        let start = self.period_start_day()?;
        let mut out = HashMap::new();
        for &p in periods {
            if scope.is_empty() {
                break;
            }
            for r in overview_scoped(&self.conn, start, p, None, Some(&scope))?.rows {
                if let Some(level) = r.level {
                    out.insert((r.category_id, p), level);
                }
            }
        }
        Ok((scope, out))
    }

    fn budget_alerts(
        &self,
        periods: &[BudgetPeriod],
        scope: &HashSet<Uuid>,
        before: &Levels,
    ) -> Result<Vec<BudgetAlert>, AppError> {
        let mut out = Vec::new();
        if scope.is_empty() {
            return Ok(out);
        }
        let start = self.period_start_day()?;
        for &p in periods {
            for r in overview_scoped(&self.conn, start, p, None, Some(scope))?.rows {
                let Some(level) = r.level else { continue };
                let was = before
                    .get(&(r.category_id, p))
                    .copied()
                    .unwrap_or(BudgetLevel::Ok);
                if level > was && level != BudgetLevel::Ok {
                    out.push(BudgetAlert {
                        category_id: r.category_id,
                        period: p,
                        level,
                        percent: r.percent,
                    });
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::overview_on;
    use crate::testkit::Kit;
    use nabungo_core::{FormBody, SplitLine};
    use time::macros::date;

    fn period(key: &str) -> BudgetPeriod {
        BudgetPeriod::parse(key).unwrap()
    }

    fn expense(k: &Kit, date: Date, category: &str, amount: i64) -> TxnForm {
        TxnForm {
            body: FormBody::Expense {
                account_id: k.bca,
                amount,
                splits: vec![SplitLine {
                    category_id: k.cat(category),
                    amount,
                    memo: None,
                }],
            },
            date,
            payee: None,
            note: None,
            tags: vec![],
        }
    }

    fn spend(k: &mut Kit, category: &str, amount: i64) -> Vec<BudgetAlert> {
        let form = expense(k, date!(2026 - 10 - 10), category, amount);
        k.ledger.create_txn_with_alerts(&form).unwrap().1
    }

    fn alert(k: &Kit, name: &str, key: &str, level: BudgetLevel, pct: u32) -> BudgetAlert {
        BudgetAlert {
            category_id: k.cat(name),
            period: period(key),
            level,
            percent: Some(pct),
        }
    }

    fn kit_with_budget(amount: i64) -> Kit {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let makan = k.cat("Makan & Minum");
        k.ledger
            .set_budget(makan, period("2026-10"), Some(amount))
            .unwrap();
        k
    }

    #[test]
    fn crossing_80_alerts_once() {
        let mut k = kit_with_budget(100_000);
        assert_eq!(spend(&mut k, "Makan & Minum", 70_000), vec![]);
        assert_eq!(
            spend(&mut k, "Makan & Minum", 15_000),
            vec![alert(
                &k,
                "Makan & Minum",
                "2026-10",
                BudgetLevel::Warning,
                85
            )]
        );
        assert_eq!(spend(&mut k, "Makan & Minum", 5_000), vec![]);
    }

    #[test]
    fn crossing_100_from_warning_alerts_exceeded() {
        let mut k = kit_with_budget(100_000);
        spend(&mut k, "Makan & Minum", 85_000);
        assert_eq!(
            spend(&mut k, "Makan & Minum", 20_000),
            vec![alert(
                &k,
                "Makan & Minum",
                "2026-10",
                BudgetLevel::Exceeded,
                105
            )]
        );
    }

    #[test]
    fn jump_straight_to_exceeded_gives_one_alert() {
        let mut k = kit_with_budget(100_000);
        let alerts = spend(&mut k, "Makan & Minum", 120_000);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, BudgetLevel::Exceeded);
    }

    #[test]
    fn child_and_parent_both_alert() {
        let mut k = kit_with_budget(60_000);
        let resto = k.cat("Restoran");
        k.ledger
            .set_budget(resto, period("2026-10"), Some(50_000))
            .unwrap();
        assert_eq!(
            spend(&mut k, "Restoran", 50_000),
            vec![
                alert(&k, "Makan & Minum", "2026-10", BudgetLevel::Warning, 83),
                alert(&k, "Restoran", "2026-10", BudgetLevel::Exceeded, 100),
            ]
        );
    }

    /// Alert versi penuh (semua kategori), pembanding untuk versi ber-scope.
    fn full_levels(k: &Kit, p: BudgetPeriod) -> Levels {
        let start = k.ledger.period_start_day().unwrap();
        overview_on(&k.ledger.conn, start, p, None)
            .unwrap()
            .rows
            .into_iter()
            .filter_map(|r| r.level.map(|l| ((r.category_id, p), l)))
            .collect()
    }

    fn full_alerts(before: &Levels, after: &Levels) -> Vec<(Uuid, BudgetLevel)> {
        let mut out: Vec<_> = after
            .iter()
            .filter(|(key, level)| {
                **level > before.get(*key).copied().unwrap_or(BudgetLevel::Ok)
                    && **level != BudgetLevel::Ok
            })
            .map(|((id, _), level)| (*id, *level))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn alerts_scoped_to_touched_categories_match_full() {
        let mut k = kit_with_budget(100_000);
        let oct = period("2026-10");
        for (name, amount) in [
            ("Restoran", 50_000),
            ("Bensin", 50_000),
            ("Biaya admin", 10_000),
        ] {
            let c = k.cat(name);
            k.ledger.set_budget(c, oct, Some(amount)).unwrap();
        }
        spend(&mut k, "Biaya admin", 9_000);
        let before = full_levels(&k, oct);
        let form = TxnForm {
            body: FormBody::Expense {
                account_id: k.bca,
                amount: 100_000,
                splits: vec![
                    SplitLine {
                        category_id: k.cat("Restoran"),
                        amount: 45_000,
                        memo: None,
                    },
                    SplitLine {
                        category_id: k.cat("Bensin"),
                        amount: 55_000,
                        memo: None,
                    },
                ],
            },
            date: date!(2026 - 10 - 10),
            payee: None,
            note: None,
            tags: vec![],
        };
        let alerts = k.ledger.create_txn_with_alerts(&form).unwrap().1;
        let mut got: Vec<_> = alerts.iter().map(|a| (a.category_id, a.level)).collect();
        got.sort();
        assert_eq!(got, full_alerts(&before, &full_levels(&k, oct)));
        assert_eq!(
            alerts,
            vec![
                alert(&k, "Restoran", "2026-10", BudgetLevel::Warning, 90),
                alert(&k, "Bensin", "2026-10", BudgetLevel::Exceeded, 110),
            ]
        );
    }

    #[test]
    fn transfer_fee_alerts_fee_category() {
        let mut k = kit_with_budget(100_000);
        let admin = k.cat("Biaya admin");
        k.ledger
            .set_budget(admin, period("2026-10"), Some(10_000))
            .unwrap();
        let (from, to) = (k.bca, k.dompet);
        let form = TxnForm {
            body: FormBody::Transfer {
                from,
                to,
                amount: 50_000,
                fee: Some(nabungo_core::Fee {
                    amount: 9_000,
                    category_id: admin,
                }),
            },
            date: date!(2026 - 10 - 10),
            payee: None,
            note: None,
            tags: vec![],
        };
        assert_eq!(
            k.ledger.create_txn_with_alerts(&form).unwrap().1,
            vec![alert(
                &k,
                "Biaya admin",
                "2026-10",
                BudgetLevel::Warning,
                90
            )]
        );
    }

    #[test]
    fn update_changing_category_alerts_new_category() {
        let mut k = kit_with_budget(100_000);
        let bensin = k.cat("Bensin");
        k.ledger
            .set_budget(bensin, period("2026-10"), Some(100_000))
            .unwrap();
        let form = expense(&k, date!(2026 - 10 - 10), "Makan & Minum", 90_000);
        let (id, _) = k.ledger.create_txn_with_alerts(&form).unwrap();
        let moved = expense(&k, date!(2026 - 10 - 10), "Bensin", 90_000);
        assert_eq!(
            k.ledger.update_txn_with_alerts(id, &moved).unwrap(),
            vec![alert(&k, "Bensin", "2026-10", BudgetLevel::Warning, 90)]
        );
    }

    #[test]
    fn unbudgeted_and_transfer_never_alert() {
        let mut k = kit_with_budget(100);
        assert_eq!(spend(&mut k, "Bensin", 1_000_000), vec![]);
        let (from, to) = (k.bca, k.dompet);
        let form = TxnForm {
            body: FormBody::Transfer {
                from,
                to,
                amount: 50_000,
                fee: None,
            },
            date: date!(2026 - 10 - 10),
            payee: None,
            note: None,
            tags: vec![],
        };
        assert_eq!(k.ledger.create_txn_with_alerts(&form).unwrap().1, vec![]);
    }

    #[test]
    fn update_moving_period_alerts_new_period_only() {
        let mut k = kit_with_budget(100_000);
        let makan = k.cat("Makan & Minum");
        k.ledger
            .set_budget(makan, period("2026-11"), Some(100_000))
            .unwrap();
        let form = expense(&k, date!(2026 - 10 - 10), "Makan & Minum", 90_000);
        let (id, alerts) = k.ledger.create_txn_with_alerts(&form).unwrap();
        assert_eq!(alerts[0].period, period("2026-10"));
        let moved = expense(&k, date!(2026 - 11 - 10), "Makan & Minum", 90_000);
        assert_eq!(
            k.ledger.update_txn_with_alerts(id, &moved).unwrap(),
            vec![alert(
                &k,
                "Makan & Minum",
                "2026-11",
                BudgetLevel::Warning,
                90
            )]
        );
        let same = expense(&k, date!(2026 - 11 - 11), "Makan & Minum", 95_000);
        assert_eq!(k.ledger.update_txn_with_alerts(id, &same).unwrap(), vec![]);
    }

    #[test]
    fn failed_save_returns_error_without_side_effect() {
        let mut k = kit_with_budget(100_000);
        let before = k
            .ledger
            .budget_overview(period("2026-10"), date!(2026 - 10 - 10))
            .unwrap();
        let mut form = expense(&k, date!(2026 - 10 - 10), "Makan & Minum", 90_000);
        if let FormBody::Expense { account_id, .. } = &mut form.body {
            *account_id = Uuid::now_v7();
        }
        assert!(k.ledger.create_txn_with_alerts(&form).is_err());
        assert_eq!(
            k.ledger
                .budget_overview(period("2026-10"), date!(2026 - 10 - 10))
                .unwrap(),
            before
        );
    }

    #[test]
    fn alert_failure_after_write_still_reports_saved_txn() {
        let mut k = kit_with_budget(100_000);
        // Baris budget rusak membuat overview gagal; transaksi tetap harus sukses.
        k.ledger.conn.execute_batch("PRAGMA foreign_keys = OFF;
             INSERT INTO budget (id, category_id, period, amount) VALUES ('x', 'bukan-uuid', '2026-10', 1);").unwrap();
        let form = expense(&k, date!(2026 - 10 - 10), "Makan & Minum", 90_000);
        let (id, alerts) = k.ledger.create_txn_with_alerts(&form).unwrap();
        assert!(alerts.is_empty());
        assert!(k.ledger.get_txn(id).is_ok());
        let edit = expense(&k, date!(2026 - 10 - 11), "Makan & Minum", 95_000);
        assert_eq!(k.ledger.update_txn_with_alerts(id, &edit).unwrap(), vec![]);
    }

    #[test]
    fn confirm_capture_reports_alert() {
        use crate::CaptureStatus;
        let mut k = Kit::new(1, date!(2026 - 05 - 01));
        k.ledger
            .set_budget(k.cat("Makan & Minum"), period("2026-05"), Some(10_000))
            .unwrap();
        k.ledger.seed_email_rules().unwrap();
        let rules = k.ledger.email_rules().unwrap();
        let raw = include_bytes!("../../mail/tests/fixtures/bca_qris.eml");
        let cursor = crate::Cursor {
            uid_validity: 1,
            last_uid: 1,
        };
        let wib = time::UtcOffset::from_hms(7, 0, 0).unwrap();
        k.ledger
            .ingest_email("INBOX", cursor, raw, &rules, wib)
            .unwrap();
        let id = k.ledger.list_captures(CaptureStatus::Pending).unwrap()[0].id;
        let form = expense(&k, date!(2026 - 05 - 09), "Makan & Minum", 41_700);
        let (txn, alerts) = k.ledger.confirm_capture_with_alerts(id, &form).unwrap();
        assert_eq!(k.ledger.get_txn(txn).unwrap().date, date!(2026 - 05 - 09));
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, BudgetLevel::Exceeded);
    }
}
