//! Peringatan budget (BUD-04): kategori ber-budget yang levelnya naik ke
//! `Warning` (≥ 80%) atau `Exceeded` (≥ 100%) karena transaksi yang disimpan.

use std::collections::HashMap;

use nabungo_core::{BudgetLevel, BudgetPeriod};
use uuid::Uuid;

use crate::TxnForm;
use crate::budget::overview_on;
use crate::ledger::{AppError, Ledger};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetAlert {
    pub category_id: Uuid,
    pub period: BudgetPeriod,
    pub level: BudgetLevel,
    pub percent: Option<u32>,
}

type Levels = HashMap<(Uuid, BudgetPeriod), BudgetLevel>;

impl Ledger {
    pub fn create_txn_with_alerts(
        &mut self,
        form: &TxnForm,
    ) -> Result<(Uuid, Vec<BudgetAlert>), AppError> {
        let periods = vec![self.period_of(form.date)?];
        let before = self.budget_levels(&periods);
        let id = self.create_txn(form)?;
        Ok((id, self.alerts_after_save(&periods, before)))
    }

    /// Memeriksa periode tanggal lama dan tanggal baru; periode yang realisasinya
    /// turun tidak memberi alert.
    pub fn update_txn_with_alerts(
        &mut self,
        id: Uuid,
        form: &TxnForm,
    ) -> Result<Vec<BudgetAlert>, AppError> {
        let old = self.get_txn(id)?.date;
        let mut periods = vec![self.period_of(old)?, self.period_of(form.date)?];
        periods.sort();
        periods.dedup();
        let before = self.budget_levels(&periods);
        self.update_txn(id, form)?;
        Ok(self.alerts_after_save(&periods, before))
    }

    /// Transaksi sudah tersimpan: kegagalan menghitung alert tidak boleh membuat
    /// penyimpanan tampak gagal (user akan mengulang dan mendobel transaksi).
    fn alerts_after_save(
        &self,
        periods: &[BudgetPeriod],
        before: Result<Levels, AppError>,
    ) -> Vec<BudgetAlert> {
        match before.and_then(|b| self.budget_alerts(periods, &b)) {
            Ok(alerts) => alerts,
            Err(e) => {
                tracing::warn!(error = %e, "gagal menghitung peringatan budget");
                Vec::new()
            }
        }
    }

    fn budget_levels(&self, periods: &[BudgetPeriod]) -> Result<Levels, AppError> {
        let start = self.period_start_day()?;
        let mut out = HashMap::new();
        for &p in periods {
            for r in overview_on(&self.conn, start, p, None)?.rows {
                if let Some(level) = r.level {
                    out.insert((r.category_id, p), level);
                }
            }
        }
        Ok(out)
    }

    fn budget_alerts(
        &self,
        periods: &[BudgetPeriod],
        before: &Levels,
    ) -> Result<Vec<BudgetAlert>, AppError> {
        let start = self.period_start_day()?;
        let mut out = Vec::new();
        for &p in periods {
            for r in overview_on(&self.conn, start, p, None)?.rows {
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
    use crate::testkit::Kit;
    use nabungo_core::{FormBody, SplitLine};
    use time::Date;
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
}
