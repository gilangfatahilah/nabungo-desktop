//! Use case laporan (RPT-02, RPT-03): cashflow per periode, breakdown kategori, net worth over time.

use std::collections::HashMap;

use nabungo_core::{BudgetPeriod, CategoryKind, net_worth};
use nabungo_storage::{category, report};
use time::Date;
use uuid::Uuid;

use crate::ledger::{AppError, Ledger};

/// Batas jumlah periode untuk laporan dan riwayat alokasi.
pub const MAX_PERIODS: u8 = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashflowPoint {
    pub period: BudgetPeriod,
    pub from: Date,
    /// Inklusif.
    pub to: Date,
    pub income: i64,
    /// Positif = keluar; refund mengurangi, biaya admin transfer ikut.
    pub expense: i64,
    pub net: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryAmount {
    /// Selalu kategori root; child digulung ke induknya.
    pub category_id: Uuid,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetWorthPoint {
    pub period: BudgetPeriod,
    /// Hari terakhir periode.
    pub date: Date,
    pub assets: i64,
    pub liabilities: i64,
    pub net: i64,
}

/// `count` periode berakhir di `end`, terlama dulu; `count` di-clamp ke `1..=MAX_PERIODS`.
pub(crate) fn periods_ending(end: BudgetPeriod, count: u8) -> Vec<BudgetPeriod> {
    let mut periods = vec![end];
    for _ in 1..count.clamp(1, MAX_PERIODS) {
        let prev = periods.last().expect("non-empty").prev();
        periods.push(prev);
    }
    periods.reverse();
    periods
}

impl Ledger {
    /// Income/expense per periode dari posting berkategori; transfer, valuation, adjustment,
    /// dan opening tidak punya kategori sehingga tidak ikut. `count` di-clamp ke `1..=24`.
    pub fn cashflow_report(
        &self,
        end: BudgetPeriod,
        count: u8,
    ) -> Result<Vec<CashflowPoint>, AppError> {
        let start = self.period_start_day()?;
        let kinds = self.category_kinds()?;
        periods_ending(end, count)
            .into_iter()
            .map(|period| {
                let (from, to_exclusive) = period.date_range(start);
                let (mut income, mut expense) = (0i64, 0i64);
                for (id, amount) in report::category_totals(&self.conn, from, to_exclusive)? {
                    match kinds.get(&id) {
                        Some(CategoryKind::Income) => income = checked_add(income, amount)?,
                        Some(CategoryKind::Expense) => expense = checked_sub(expense, amount)?,
                        None => {}
                    }
                }
                Ok(CashflowPoint {
                    period,
                    from,
                    to: period.last_day(start),
                    income,
                    expense,
                    net: checked_sub(income, expense)?,
                })
            })
            .collect()
    }

    /// Total per kategori root berjenis `kind` dalam satu periode, bernilai positif
    /// (expense: terpakai; income: diterima). Root dengan total ≤ 0 dibuang; urut nominal
    /// turun lalu nama.
    pub fn category_breakdown(
        &self,
        period: BudgetPeriod,
        kind: CategoryKind,
    ) -> Result<Vec<CategoryAmount>, AppError> {
        let (from, to_exclusive) = period.date_range(self.period_start_day()?);
        let cats = category::list_categories(&self.conn)?;
        let by_id: HashMap<Uuid, _> = cats.iter().map(|c| (c.id, c)).collect();
        let mut totals: HashMap<Uuid, i64> = HashMap::new();
        for (id, amount) in report::category_totals(&self.conn, from, to_exclusive)? {
            let Some(c) = by_id.get(&id).filter(|c| c.kind == kind) else {
                continue;
            };
            let signed = match kind {
                CategoryKind::Income => amount,
                CategoryKind::Expense => amount.checked_neg().ok_or_else(overflow)?,
            };
            let root = totals.entry(c.parent_id.unwrap_or(c.id)).or_default();
            *root = checked_add(*root, signed)?;
        }
        let mut out: Vec<CategoryAmount> = totals
            .into_iter()
            .filter(|&(_, amount)| amount > 0)
            .map(|(category_id, amount)| CategoryAmount {
                category_id,
                amount,
            })
            .collect();
        let name = |id: &Uuid| by_id.get(id).map(|c| c.name.as_str()).unwrap_or_default();
        out.sort_by(|a, b| {
            b.amount
                .cmp(&a.amount)
                .then_with(|| name(&a.category_id).cmp(name(&b.category_id)))
        });
        Ok(out)
    }

    /// Net worth (K9) per hari terakhir tiap periode, dari akun `include_in_net_worth`
    /// (termasuk yang diarsip). Valuation ikut karena berupa posting. `count` di-clamp ke `1..=24`.
    pub fn net_worth_history(
        &self,
        end: BudgetPeriod,
        count: u8,
    ) -> Result<Vec<NetWorthPoint>, AppError> {
        self.period_points(end, count)?
            .into_iter()
            .map(|(period, date, balances)| {
                let nw = net_worth(
                    balances
                        .iter()
                        .filter(|b| b.include_in_net_worth)
                        .map(|b| (b.kind, b.balance)),
                )?;
                Ok(NetWorthPoint {
                    period,
                    date,
                    assets: nw.assets,
                    liabilities: nw.liabilities,
                    net: nw.net,
                })
            })
            .collect()
    }

    fn category_kinds(&self) -> Result<HashMap<Uuid, CategoryKind>, AppError> {
        Ok(category::list_categories(&self.conn)?
            .into_iter()
            .map(|c| (c.id, c.kind))
            .collect())
    }
}

fn overflow() -> AppError {
    nabungo_core::MoneyError::Overflow.into()
}

fn checked_add(a: i64, b: i64) -> Result<i64, AppError> {
    a.checked_add(b).ok_or_else(overflow)
}

fn checked_sub(a: i64, b: i64) -> Result<i64, AppError> {
    a.checked_sub(b).ok_or_else(overflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Kit;
    use crate::{AccountFields, AccountKind, CategoryKind, FormBody, SplitLine, TxnForm};
    use time::macros::date;

    fn period(key: &str) -> BudgetPeriod {
        BudgetPeriod::parse(key).unwrap()
    }

    #[test]
    fn cashflow_periods_are_contiguous() {
        let k = Kit::new(25, date!(2026 - 01 - 01));
        let r = k.ledger.cashflow_report(period("2026-11"), 12).unwrap();
        assert_eq!(r.len(), 12);
        assert_eq!(r[0].period.key(), "2025-12");
        assert_eq!(r[11].to, date!(2026 - 11 - 24));
        for w in r.windows(2) {
            assert_eq!(w[0].to.next_day().unwrap(), w[1].from);
        }
    }

    #[test]
    fn count_is_clamped() {
        let k = Kit::new(1, date!(2026 - 01 - 01));
        let oct = period("2026-10");
        assert_eq!(k.ledger.cashflow_report(oct, 0).unwrap().len(), 1);
        assert_eq!(k.ledger.cashflow_report(oct, 200).unwrap().len(), 24);
        assert_eq!(k.ledger.net_worth_history(oct, 0).unwrap().len(), 1);
        assert_eq!(k.ledger.allocation_history(oct, 200).unwrap().len(), 24);
    }

    #[test]
    fn cashflow_excludes_transfers_and_valuation() {
        let mut k = Kit::new(1, date!(2026 - 09 - 01));
        let rdn = k
            .ledger
            .create_account(
                AccountKind::Investment,
                &AccountFields::new("RDN", 0, date!(2026 - 09 - 01)),
            )
            .unwrap();
        k.income(date!(2026 - 10 - 01), "Gaji", 8_000_000);
        k.expense(date!(2026 - 10 - 02), "Kopi & jajan", 25_000);
        k.expense(date!(2026 - 10 - 03), "Kopi & jajan", -5_000);
        k.transfer(date!(2026 - 10 - 04), 200_000, Some(6_500));
        k.ledger
            .record_valuation(rdn, date!(2026 - 10 - 05), 750_000)
            .unwrap();
        let p = &k.ledger.cashflow_report(period("2026-10"), 1).unwrap()[0];
        assert_eq!((p.income, p.expense, p.net), (8_000_000, 26_500, 7_973_500));
        assert_eq!(
            (p.from, p.to),
            (date!(2026 - 10 - 01), date!(2026 - 10 - 31))
        );
    }

    #[test]
    fn breakdown_rolls_up_to_root_sorted() {
        let mut k = Kit::new(1, date!(2026 - 09 - 01));
        k.expense(date!(2026 - 10 - 02), "Restoran", 50_000);
        k.expense(date!(2026 - 10 - 03), "Kopi & jajan", 25_000);
        k.expense(date!(2026 - 10 - 04), "Transportasi", 60_000);
        k.expense(date!(2026 - 09 - 04), "Tagihan", 99_000);
        k.income(date!(2026 - 10 - 01), "Gaji", 8_000_000);
        let (makan, transport) = (k.cat("Makan & Minum"), k.cat("Transportasi"));
        let amounts = |b: Vec<CategoryAmount>| -> Vec<(Uuid, i64)> {
            b.into_iter().map(|c| (c.category_id, c.amount)).collect()
        };
        let oct = period("2026-10");
        let b = k
            .ledger
            .category_breakdown(oct, CategoryKind::Expense)
            .unwrap();
        assert_eq!(amounts(b), vec![(makan, 75_000), (transport, 60_000)]);
        let b = k
            .ledger
            .category_breakdown(oct, CategoryKind::Income)
            .unwrap();
        assert_eq!(amounts(b), vec![(k.cat("Gaji"), 8_000_000)]);
    }

    #[test]
    fn net_worth_history_follows_postings_and_valuation() {
        let mut k = Kit::new(1, date!(2026 - 09 - 01));
        let opening = date!(2026 - 09 - 01);
        let cc = k
            .ledger
            .create_account(
                AccountKind::CreditCard,
                &AccountFields::new("Kartu", 0, opening),
            )
            .unwrap();
        let rdn = k
            .ledger
            .create_account(
                AccountKind::Investment,
                &AccountFields::new("RDN", 0, opening),
            )
            .unwrap();
        let category_id = k.cat("Belanja");
        k.ledger
            .create_txn(&TxnForm {
                body: FormBody::Expense {
                    account_id: cc,
                    amount: 200_000,
                    splits: vec![SplitLine {
                        category_id,
                        amount: 200_000,
                        memo: None,
                    }],
                },
                date: date!(2026 - 10 - 10),
                payee: None,
                note: None,
                tags: vec![],
            })
            .unwrap();
        k.ledger
            .record_valuation(rdn, date!(2026 - 11 - 15), 500_000)
            .unwrap();
        let h = k.ledger.net_worth_history(period("2026-11"), 3).unwrap();
        let nets: Vec<i64> = h.iter().map(|p| p.net).collect();
        assert_eq!(nets, vec![1_000_000, 800_000, 1_300_000]);
        assert_eq!(h[1].liabilities, 200_000);
        assert_eq!(h[1].assets, 1_000_000);
        assert_eq!(h[2].date, date!(2026 - 11 - 30));
    }
}
