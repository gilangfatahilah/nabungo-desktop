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

/// Nominal satu kategori pada periode ini dan periode sebelumnya, bertanda seperti
/// [`CategoryAmount`] (positif = terpakai/diterima; refund bisa membuatnya negatif).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryDelta {
    pub category_id: Uuid,
    pub amount: i64,
    pub previous: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryComparison {
    /// Selalu kategori root; `amount`/`previous` termasuk child.
    pub category_id: Uuid,
    pub amount: i64,
    pub previous: i64,
    /// Rincian per child. Posting langsung di root muncul dengan id root, hanya bila
    /// root juga punya child yang terisi. Kosong bila tidak ada child terisi.
    pub children: Vec<CategoryDelta>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryTrendPoint {
    pub period: BudgetPeriod,
    pub from: Date,
    /// Inklusif.
    pub to: Date,
    pub amount: i64,
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
            let signed = sign(kind, amount)?;
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

    /// [`Self::category_breakdown`] periode `period` dibanding periode sebelumnya, dengan rincian
    /// child. Root ikut bila salah satu periode positif, jadi kategori yang turun ke 0 tetap
    /// terlihat. Urut nominal periode ini turun, lalu periode lalu, lalu nama; child juga.
    pub fn category_comparison(
        &self,
        period: BudgetPeriod,
        kind: CategoryKind,
    ) -> Result<Vec<CategoryComparison>, AppError> {
        let start = self.period_start_day()?;
        let cats = category::list_categories(&self.conn)?;
        let by_id: HashMap<Uuid, _> = cats.iter().map(|c| (c.id, c)).collect();
        let signed_totals = |p: BudgetPeriod| -> Result<HashMap<Uuid, i64>, AppError> {
            let (from, to_exclusive) = p.date_range(start);
            report::category_totals(&self.conn, from, to_exclusive)?
                .into_iter()
                .filter(|(id, _)| by_id.get(id).is_some_and(|c| c.kind == kind))
                .map(|(id, amount)| Ok((id, sign(kind, amount)?)))
                .collect()
        };
        let current = signed_totals(period)?;
        let previous = signed_totals(period.prev())?;

        // root → (total, child per kategori).
        let mut roots: HashMap<Uuid, (CategoryDelta, Vec<CategoryDelta>)> = HashMap::new();
        for id in current.keys().chain(previous.keys()) {
            let root_id = by_id[id].parent_id.unwrap_or(*id);
            let (total, children) = roots.entry(root_id).or_insert_with(|| {
                (
                    CategoryDelta {
                        category_id: root_id,
                        amount: 0,
                        previous: 0,
                    },
                    Vec::new(),
                )
            });
            if children.iter().any(|c| c.category_id == *id) {
                continue;
            }
            let child = CategoryDelta {
                category_id: *id,
                amount: current.get(id).copied().unwrap_or(0),
                previous: previous.get(id).copied().unwrap_or(0),
            };
            total.amount = checked_add(total.amount, child.amount)?;
            total.previous = checked_add(total.previous, child.previous)?;
            children.push(child);
        }

        let name = |id: &Uuid| by_id.get(id).map(|c| c.name.as_str()).unwrap_or_default();
        let order = |a: &CategoryDelta, b: &CategoryDelta| {
            b.amount
                .cmp(&a.amount)
                .then_with(|| b.previous.cmp(&a.previous))
                .then_with(|| name(&a.category_id).cmp(name(&b.category_id)))
        };
        let mut roots: Vec<_> = roots
            .into_values()
            .filter(|(total, _)| total.amount > 0 || total.previous > 0)
            .collect();
        roots.sort_by(|a, b| order(&a.0, &b.0));
        let out = roots
            .into_iter()
            .map(|(total, mut children)| {
                children.retain(|c| c.amount != 0 || c.previous != 0);
                // Hanya posting langsung di root: tidak ada rincian yang perlu ditampilkan.
                if children.iter().all(|c| c.category_id == total.category_id) {
                    children.clear();
                }
                children.sort_by(order);
                CategoryComparison {
                    category_id: total.category_id,
                    amount: total.amount,
                    previous: total.previous,
                    children,
                }
            })
            .collect();
        Ok(out)
    }

    /// Nominal satu kategori (root: termasuk child-nya) per periode, bertanda sesuai jenis
    /// kategorinya seperti [`Self::category_breakdown`]. `count` di-clamp ke `1..=24`.
    pub fn category_trend(
        &self,
        end: BudgetPeriod,
        count: u8,
        category_id: Uuid,
    ) -> Result<Vec<CategoryTrendPoint>, AppError> {
        let start = self.period_start_day()?;
        let cats = category::list_categories(&self.conn)?;
        let kind = cats
            .iter()
            .find(|c| c.id == category_id)
            .ok_or(AppError::NotFound("category"))?
            .kind;
        let ids: Vec<Uuid> = cats
            .iter()
            .filter(|c| c.id == category_id || c.parent_id == Some(category_id))
            .map(|c| c.id)
            .collect();
        periods_ending(end, count)
            .into_iter()
            .map(|period| {
                let (from, to_exclusive) = period.date_range(start);
                let mut amount = 0i64;
                for (_, total) in report::category_totals_in(&self.conn, from, to_exclusive, &ids)?
                {
                    amount = checked_add(amount, sign(kind, total)?)?;
                }
                Ok(CategoryTrendPoint {
                    period,
                    from,
                    to: period.last_day(start),
                    amount,
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

/// Posting expense bernilai negatif saat terpakai; dibalik agar laporan selalu positif.
fn sign(kind: CategoryKind, amount: i64) -> Result<i64, AppError> {
    match kind {
        CategoryKind::Income => Ok(amount),
        CategoryKind::Expense => amount.checked_neg().ok_or_else(overflow),
    }
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
    fn comparison_includes_previous_and_children() {
        let mut k = Kit::new(1, date!(2026 - 09 - 01));
        k.expense(date!(2026 - 09 - 02), "Restoran", 40_000);
        k.expense(date!(2026 - 09 - 03), "Tagihan", 99_000);
        k.expense(date!(2026 - 10 - 02), "Restoran", 50_000);
        k.expense(date!(2026 - 10 - 03), "Kopi & jajan", 25_000);
        k.expense(date!(2026 - 10 - 04), "Transportasi", 60_000);
        k.transfer(date!(2026 - 10 - 05), 100_000, None);
        k.income(date!(2026 - 10 - 01), "Gaji", 8_000_000);
        let c = k
            .ledger
            .category_comparison(period("2026-10"), CategoryKind::Expense)
            .unwrap();
        let rows: Vec<(Uuid, i64, i64)> = c
            .iter()
            .map(|r| (r.category_id, r.amount, r.previous))
            .collect();
        assert_eq!(
            rows,
            vec![
                (k.cat("Makan & Minum"), 75_000, 40_000),
                (k.cat("Transportasi"), 60_000, 0),
                // Turun ke 0 tetap tampil.
                (k.cat("Tagihan"), 0, 99_000),
            ]
        );
        let children: Vec<(Uuid, i64, i64)> = c[0]
            .children
            .iter()
            .map(|r| (r.category_id, r.amount, r.previous))
            .collect();
        assert_eq!(
            children,
            vec![
                (k.cat("Restoran"), 50_000, 40_000),
                (k.cat("Kopi & jajan"), 25_000, 0),
            ]
        );
        assert!(c[1].children.is_empty());
    }

    #[test]
    fn comparison_shows_direct_root_postings_beside_children() {
        let mut k = Kit::new(1, date!(2026 - 09 - 01));
        k.expense(date!(2026 - 10 - 02), "Makan & Minum", 10_000);
        k.expense(date!(2026 - 10 - 03), "Restoran", 30_000);
        k.expense(date!(2026 - 10 - 04), "Restoran", -5_000);
        let c = k
            .ledger
            .category_comparison(period("2026-10"), CategoryKind::Expense)
            .unwrap();
        assert_eq!(c[0].amount, 35_000);
        let ids: Vec<(Uuid, i64)> = c[0]
            .children
            .iter()
            .map(|r| (r.category_id, r.amount))
            .collect();
        assert_eq!(
            ids,
            vec![
                (k.cat("Restoran"), 25_000),
                (k.cat("Makan & Minum"), 10_000)
            ]
        );
    }

    #[test]
    fn trend_rolls_up_children_per_period() {
        let mut k = Kit::new(1, date!(2026 - 08 - 01));
        k.expense(date!(2026 - 09 - 02), "Restoran", 40_000);
        k.expense(date!(2026 - 10 - 02), "Restoran", 50_000);
        k.expense(date!(2026 - 10 - 03), "Kopi & jajan", 25_000);
        k.expense(date!(2026 - 10 - 04), "Transportasi", 60_000);
        let makan = k.cat("Makan & Minum");
        let t = k
            .ledger
            .category_trend(period("2026-10"), 3, makan)
            .unwrap();
        let amounts: Vec<i64> = t.iter().map(|p| p.amount).collect();
        assert_eq!(amounts, vec![0, 40_000, 75_000]);
        assert_eq!(t[2].to, date!(2026 - 10 - 31));
        let t = k
            .ledger
            .category_trend(period("2026-10"), 1, k.cat("Restoran"))
            .unwrap();
        assert_eq!(t[0].amount, 50_000);
        let gaji = k.cat("Gaji");
        k.income(date!(2026 - 10 - 01), "Gaji", 8_000_000);
        assert_eq!(
            k.ledger.category_trend(period("2026-10"), 1, gaji).unwrap()[0].amount,
            8_000_000
        );
        assert_eq!(
            k.ledger
                .category_trend(period("2026-10"), 200, gaji)
                .unwrap()
                .len(),
            24
        );
        assert!(matches!(
            k.ledger.category_trend(period("2026-10"), 1, Uuid::nil()),
            Err(AppError::NotFound("category"))
        ));
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
