//! Use case budget (BUD-01..04): realisasi per periode budget, rollover, level
//! peringatan, sisa per hari, set nominal, salin dari periode lalu.

use std::collections::{BTreeMap, HashMap, HashSet};

use nabungo_core::{
    BudgetCategory, BudgetEntry, BudgetError, BudgetLevel, BudgetLine, BudgetPeriod, BudgetTotals,
    CategoryKind, MoneyError, PeriodStartDay, budget_level, budget_totals, days_left, per_day,
    period_of, rollover_chain, rollup_spent, spent_percent, validate_budget_amount,
};
use nabungo_storage::category::{self, CategoryRow};
use nabungo_storage::rusqlite::Connection;
use nabungo_storage::{budget, report};
use time::Date;
use uuid::Uuid;

use crate::ledger::{AppError, Ledger, active};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetRow {
    pub category_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub budget: Option<i64>,
    /// Sisa periode ini dibawa ke periode berikutnya.
    pub rollover: bool,
    /// Bawaan dari rantai rollover sebelumnya (bisa negatif); 0 tanpa budget.
    pub carry: i64,
    /// Positif = terpakai; parent mencakup child.
    pub spent: i64,
    /// `None` bila tanpa budget.
    pub level: Option<BudgetLevel>,
    pub percent: Option<u32>,
    /// `None` bila tanpa budget atau periode bukan periode berjalan.
    pub per_day: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetOverview {
    pub period: BudgetPeriod,
    pub from: Date,
    /// Inklusif.
    pub to: Date,
    pub rows: Vec<BudgetRow>,
    pub totals: BudgetTotals,
    /// Hari tersisa termasuk hari ini; `None` bila hari ini di luar periode.
    pub days_left: Option<u32>,
    pub total_per_day: Option<i64>,
}

impl Ledger {
    pub fn period_of(&self, date: Date) -> Result<BudgetPeriod, AppError> {
        Ok(period_of(date, self.period_start_day()?))
    }

    /// Baris = kategori expense aktif, plus yang diarsip tetapi punya budget atau
    /// realisasi di periode ini; urutan pohon (parent lalu child-nya). `today`
    /// hanya menentukan sisa per hari.
    pub fn budget_overview(
        &self,
        period: BudgetPeriod,
        today: Date,
    ) -> Result<BudgetOverview, AppError> {
        overview_on(&self.conn, self.period_start_day()?, period, Some(today))
    }

    /// `None` menghapus budget. Menghapus boleh untuk kategori yang diarsip; mengisi tidak.
    pub fn set_budget(
        &mut self,
        category_id: Uuid,
        period: BudgetPeriod,
        amount: Option<i64>,
    ) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        let found = category::get_category_ref(&tx, category_id)?;
        let kind = match (found, amount) {
            (None, _) => return Err(AppError::NotFound("category")),
            (Some(l), None) => l.value.kind,
            (found, Some(_)) => active(found, "category")?.kind,
        };
        if kind != CategoryKind::Expense {
            return Err(BudgetError::IncomeCategory.into());
        }
        match amount {
            Some(a) => {
                budget::upsert_budget(&tx, category_id, &period.key(), validate_budget_amount(a)?)?
            }
            None => {
                budget::delete_budget(&tx, category_id, &period.key())?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Menyalakan rollover butuh kategori expense aktif; mematikan boleh untuk
    /// kategori yang diarsip. Budget periode itu harus ada.
    pub fn set_budget_rollover(
        &mut self,
        category_id: Uuid,
        period: BudgetPeriod,
        on: bool,
    ) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        let found = category::get_category_ref(&tx, category_id)?;
        let kind = match (found, on) {
            (None, _) => return Err(AppError::NotFound("category")),
            (Some(l), false) => l.value.kind,
            (found, true) => active(found, "category")?.kind,
        };
        if kind != CategoryKind::Expense {
            return Err(BudgetError::IncomeCategory.into());
        }
        if !budget::set_rollover(&tx, category_id, &period.key(), on)? {
            return Err(AppError::NotFound("budget"));
        }
        tx.commit()?;
        Ok(())
    }

    /// Salin budget `to.prev()` (nominal + flag rollover) ke `to` untuk kategori
    /// yang belum punya budget di `to` dan tidak diarsip. Tidak menimpa.
    /// Mengembalikan jumlah baris yang disalin.
    pub fn copy_budgets(&mut self, to: BudgetPeriod) -> Result<u32, AppError> {
        let tx = self.conn.transaction()?;
        let existing: HashSet<Uuid> = budget::list_budgets(&tx, &to.key())?
            .into_iter()
            .map(|b| b.category_id)
            .collect();
        let mut copied = 0;
        for b in budget::list_budgets(&tx, &to.prev().key())? {
            if existing.contains(&b.category_id) {
                continue;
            }
            match category::get_category_ref(&tx, b.category_id)? {
                Some(l) if !l.archived && l.value.kind == CategoryKind::Expense => {
                    budget::upsert_budget(&tx, b.category_id, &to.key(), b.amount)?;
                    if b.rollover {
                        budget::set_rollover(&tx, b.category_id, &to.key(), true)?;
                    }
                    copied += 1;
                }
                _ => {}
            }
        }
        tx.commit()?;
        Ok(copied)
    }
}

/// Overview budget di atas koneksi apa pun (juga di dalam transaction tulis).
/// `today = None`: tanpa sisa per hari.
pub(crate) fn overview_on(
    conn: &Connection,
    start: PeriodStartDay,
    period: BudgetPeriod,
    today: Option<Date>,
) -> Result<BudgetOverview, AppError> {
    let (from, to) = period.date_range(start);
    let last = period.last_day(start);
    let cats: Vec<CategoryRow> = category::list_categories(conn)?
        .into_iter()
        .filter(|c| c.kind == CategoryKind::Expense)
        .collect();
    let tree: Vec<BudgetCategory> = cats
        .iter()
        .map(|c| BudgetCategory {
            id: c.id,
            parent_id: c.parent_id,
        })
        .collect();
    let direct: HashMap<Uuid, i64> = report::category_totals(conn, from, to)?
        .into_iter()
        .collect();
    let spent = rollup_spent(&tree, &direct);
    let budgets: HashMap<Uuid, budget::BudgetRow> = budget::list_budgets(conn, &period.key())?
        .into_iter()
        .map(|b| (b.category_id, b))
        .collect();
    let carry = carries(conn, start, period, &tree, &budgets)?;
    let days = today.and_then(|t| days_left(t, from, last));

    let mut rows = Vec::new();
    for root in cats.iter().filter(|c| c.parent_id.is_none()) {
        let children = cats.iter().filter(|c| c.parent_id == Some(root.id));
        for c in std::iter::once(root).chain(children) {
            let b = budgets.get(&c.id);
            let spent = spent.get(&c.id).copied().unwrap_or(0);
            let carry = carry.get(&c.id).copied().unwrap_or(0);
            let available = b
                .map(|b| b.amount.checked_add(carry).ok_or(MoneyError::Overflow))
                .transpose()?;
            let row = BudgetRow {
                category_id: c.id,
                parent_id: c.parent_id,
                budget: b.map(|b| b.amount),
                rollover: b.is_some_and(|b| b.rollover),
                carry,
                spent,
                level: available.map(|a| budget_level(a, spent)),
                percent: available.and_then(|a| spent_percent(a, spent)),
                per_day: available
                    .zip(days)
                    .map(|(a, d)| per_day(a.saturating_sub(spent), d)),
            };
            if !c.archived || row.budget.is_some() || row.spent != 0 {
                rows.push(row);
            }
        }
    }
    let lines: Vec<BudgetLine> = rows
        .iter()
        .map(|r| BudgetLine {
            category_id: r.category_id,
            budget: r.budget,
            carry: r.carry,
            spent: r.spent,
        })
        .collect();
    let totals = budget_totals(&tree, &lines)?;
    let total_per_day = days.map(|d| {
        per_day(
            totals
                .budget
                .saturating_add(totals.carry)
                .saturating_sub(totals.spent),
            d,
        )
    });
    Ok(BudgetOverview {
        period,
        from,
        to: last,
        rows,
        totals,
        days_left: days,
        total_per_day,
    })
}

/// Carry per kategori ber-budget di `period` (K2): Σ nominal rantai rollover
/// dikurangi realisasi rantai. Satu query realisasi per awal rantai unik.
fn carries(
    conn: &Connection,
    start: PeriodStartDay,
    period: BudgetPeriod,
    tree: &[BudgetCategory],
    budgets: &HashMap<Uuid, budget::BudgetRow>,
) -> Result<HashMap<Uuid, i64>, AppError> {
    let mut out = HashMap::new();
    if budgets.is_empty() {
        return Ok(out);
    }
    let mut history: HashMap<Uuid, BTreeMap<BudgetPeriod, BudgetEntry>> = HashMap::new();
    for b in budget::list_budgets_before(conn, &period.key())? {
        if budgets.contains_key(&b.category_id) {
            history.entry(b.category_id).or_default().insert(
                BudgetPeriod::parse(&b.period)?,
                BudgetEntry {
                    amount: b.amount,
                    rollover: b.rollover,
                },
            );
        }
    }
    let mut by_start: BTreeMap<BudgetPeriod, Vec<(Uuid, i64)>> = BTreeMap::new();
    for (id, entries) in &history {
        if let Some((first, sum)) = rollover_chain(entries, period)? {
            by_start.entry(first).or_default().push((*id, sum));
        }
    }
    let (until, _) = period.date_range(start);
    for (first, members) in by_start {
        let (chain_from, _) = first.date_range(start);
        let direct: HashMap<Uuid, i64> = report::category_totals(conn, chain_from, until)?
            .into_iter()
            .collect();
        let spent = rollup_spent(tree, &direct);
        for (id, sum) in members {
            let used = spent.get(&id).copied().unwrap_or(0);
            out.insert(id, sum.checked_sub(used).ok_or(MoneyError::Overflow)?);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Kit;
    use crate::{BudgetError, BudgetLevel, CategoryKind};
    use time::macros::date;

    /// "Hari ini" di luar semua periode test: kolom per hari tidak terisi.
    const FAR: Date = date!(2000 - 01 - 01);

    fn period(key: &str) -> BudgetPeriod {
        BudgetPeriod::parse(key).unwrap()
    }

    fn row(o: &BudgetOverview, id: Uuid) -> &BudgetRow {
        o.rows.iter().find(|r| r.category_id == id).expect("row")
    }

    #[test]
    fn transfer_does_not_affect_budget() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let (oct, makan) = (period("2026-10"), k.cat("Makan & Minum"));
        k.ledger.set_budget(makan, oct, Some(500_000)).unwrap();
        k.expense(date!(2026 - 10 - 02), "Kopi & jajan", 25_000);
        let before = k.ledger.budget_overview(oct, FAR).unwrap();
        assert_eq!(row(&before, makan).spent, 25_000);
        let t = k.transfer(date!(2026 - 10 - 03), 200_000, None);
        assert_eq!(k.ledger.budget_overview(oct, FAR).unwrap(), before);
        k.ledger.delete_txn(t).unwrap();
        assert_eq!(k.ledger.budget_overview(oct, FAR).unwrap(), before);
        k.income(date!(2026 - 10 - 04), "Gaji", 8_000_000);
        assert_eq!(k.ledger.budget_overview(oct, FAR).unwrap(), before);
    }

    #[test]
    fn custom_start_day_period_boundaries() {
        let mut k = Kit::new(25, date!(2026 - 09 - 01));
        k.expense(date!(2026 - 10 - 24), "Kopi & jajan", 10_000);
        k.expense(date!(2026 - 10 - 25), "Kopi & jajan", 20_000);
        let kopi = k.cat("Kopi & jajan");
        let oct = k.ledger.budget_overview(period("2026-10"), FAR).unwrap();
        let nov = k.ledger.budget_overview(period("2026-11"), FAR).unwrap();
        assert_eq!(
            (oct.from, oct.to),
            (date!(2026 - 09 - 25), date!(2026 - 10 - 24))
        );
        assert_eq!(
            (nov.from, nov.to),
            (date!(2026 - 10 - 25), date!(2026 - 11 - 24))
        );
        assert_eq!(row(&oct, kopi).spent, 10_000);
        assert_eq!(row(&nov, kopi).spent, 20_000);
        assert_eq!(
            k.ledger.period_of(date!(2026 - 10 - 26)).unwrap().key(),
            "2026-11"
        );
        assert_eq!(
            k.ledger.period_of(date!(2026 - 10 - 24)).unwrap().key(),
            "2026-10"
        );
    }

    #[test]
    fn rows_are_expense_categories_in_tree_order() {
        let k = Kit::new(1, date!(2026 - 10 - 01));
        let o = k.ledger.budget_overview(period("2026-10"), FAR).unwrap();
        let cats = k.ledger.list_categories().unwrap();
        assert!(o.rows.iter().all(|r| {
            cats.iter()
                .any(|c| c.id == r.category_id && c.kind == CategoryKind::Expense)
        }));
        let makan = o
            .rows
            .iter()
            .position(|r| r.category_id == k.cat("Makan & Minum"))
            .unwrap();
        let resto = o
            .rows
            .iter()
            .position(|r| r.category_id == k.cat("Restoran"))
            .unwrap();
        assert!(makan < resto);
        assert_eq!(o.rows[makan + 1].parent_id, Some(k.cat("Makan & Minum")));
        assert_eq!(o.rows[makan].parent_id, None);
    }

    #[test]
    fn archived_child_spending_rolls_up() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        k.expense(date!(2026 - 10 - 02), "Restoran", 50_000);
        let (resto, makan, kopi) = (
            k.cat("Restoran"),
            k.cat("Makan & Minum"),
            k.cat("Kopi & jajan"),
        );
        k.ledger.archive_category(resto).unwrap();
        let o = k.ledger.budget_overview(period("2026-10"), FAR).unwrap();
        assert_eq!(row(&o, resto).spent, 50_000);
        assert_eq!(row(&o, makan).spent, 50_000);
        k.ledger.archive_category(kopi).unwrap();
        let o = k.ledger.budget_overview(period("2026-10"), FAR).unwrap();
        assert!(
            !o.rows.iter().any(|r| r.category_id == kopi),
            "archived & unused hidden"
        );
        let nov = k.ledger.budget_overview(period("2026-11"), FAR).unwrap();
        assert!(!nov.rows.iter().any(|r| r.category_id == resto));
    }

    #[test]
    fn budget_follows_delete_and_undo() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let kopi = k.cat("Kopi & jajan");
        let t = k.expense(date!(2026 - 10 - 02), "Kopi & jajan", 25_000);
        k.ledger.delete_txn(t).unwrap();
        assert_eq!(
            row(
                &k.ledger.budget_overview(period("2026-10"), FAR).unwrap(),
                kopi
            )
            .spent,
            0
        );
        k.ledger.undo_delete_txn(t).unwrap();
        assert_eq!(
            row(
                &k.ledger.budget_overview(period("2026-10"), FAR).unwrap(),
                kopi
            )
            .spent,
            25_000
        );
    }

    #[test]
    fn fee_counts_refund_reduces() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let (oct, fee, kopi) = (
            period("2026-10"),
            k.cat("Biaya admin"),
            k.cat("Kopi & jajan"),
        );
        k.transfer(date!(2026 - 10 - 02), 200_000, Some(6_500));
        k.expense(date!(2026 - 10 - 03), "Kopi & jajan", 25_000);
        k.expense(date!(2026 - 10 - 04), "Kopi & jajan", -5_000);
        let o = k.ledger.budget_overview(oct, FAR).unwrap();
        assert_eq!(row(&o, fee).spent, 6_500);
        assert_eq!(row(&o, kopi).spent, 20_000);
    }

    #[test]
    fn totals_use_top_budgeted_rows() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let oct = period("2026-10");
        let (makan, resto, transport) = (
            k.cat("Makan & Minum"),
            k.cat("Restoran"),
            k.cat("Transportasi"),
        );
        k.ledger.set_budget(makan, oct, Some(500_000)).unwrap();
        k.ledger.set_budget(resto, oct, Some(200_000)).unwrap();
        k.ledger.set_budget(transport, oct, Some(300_000)).unwrap();
        k.expense(date!(2026 - 10 - 02), "Restoran", 50_000);
        k.expense(date!(2026 - 10 - 02), "Biaya admin", 1_000);
        let o = k.ledger.budget_overview(oct, FAR).unwrap();
        assert_eq!(
            o.totals,
            BudgetTotals {
                budget: 800_000,
                carry: 0,
                spent: 50_000
            }
        );
        assert_eq!(row(&o, resto).budget, Some(200_000));
    }

    #[test]
    fn set_budget_rules() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let oct = period("2026-10");
        let (gaji, makan, kopi) = (k.cat("Gaji"), k.cat("Makan & Minum"), k.cat("Kopi & jajan"));
        assert!(matches!(
            k.ledger.set_budget(gaji, oct, Some(1)),
            Err(AppError::Budget(BudgetError::IncomeCategory))
        ));
        assert!(matches!(
            k.ledger.set_budget(makan, oct, Some(-1)),
            Err(AppError::Budget(BudgetError::Negative))
        ));
        assert!(matches!(
            k.ledger.set_budget(Uuid::now_v7(), oct, Some(1)),
            Err(AppError::NotFound("category"))
        ));
        k.ledger.set_budget(kopi, oct, Some(10)).unwrap();
        k.ledger.archive_category(kopi).unwrap();
        assert!(matches!(
            k.ledger.set_budget(kopi, oct, Some(1)),
            Err(AppError::Archived("category"))
        ));
        k.ledger.set_budget(kopi, oct, None).unwrap();
        k.ledger.set_budget(makan, oct, Some(0)).unwrap();
        let o = k.ledger.budget_overview(oct, FAR).unwrap();
        assert_eq!(row(&o, makan).budget, Some(0));
        k.ledger.set_budget(makan, oct, None).unwrap();
        assert_eq!(
            row(&k.ledger.budget_overview(oct, FAR).unwrap(), makan).budget,
            None
        );
    }

    #[test]
    fn copy_budgets_fills_only_missing() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let (oct, nov) = (period("2026-10"), period("2026-11"));
        let (makan, transport, hiburan) = (
            k.cat("Makan & Minum"),
            k.cat("Transportasi"),
            k.cat("Hiburan"),
        );
        k.ledger.set_budget(makan, oct, Some(500_000)).unwrap();
        k.ledger.set_budget(transport, oct, Some(300_000)).unwrap();
        k.ledger.set_budget_rollover(transport, oct, true).unwrap();
        k.ledger.set_budget(hiburan, oct, Some(100_000)).unwrap();
        k.ledger.archive_category(hiburan).unwrap();
        k.ledger.set_budget(makan, nov, Some(450_000)).unwrap();
        assert_eq!(k.ledger.copy_budgets(nov).unwrap(), 1);
        let o = k.ledger.budget_overview(nov, FAR).unwrap();
        assert_eq!(row(&o, makan).budget, Some(450_000));
        assert_eq!(row(&o, transport).budget, Some(300_000));
        assert!(row(&o, transport).rollover);
        assert!(!row(&o, makan).rollover);
        assert!(!o.rows.iter().any(|r| r.category_id == hiburan));
        assert_eq!(k.ledger.copy_budgets(nov).unwrap(), 0);
    }

    fn set(k: &mut Kit, name: &str, key: &str, amount: i64, rollover: bool) {
        let (cat, p) = (k.cat(name), period(key));
        k.ledger.set_budget(cat, p, Some(amount)).unwrap();
        if rollover {
            k.ledger.set_budget_rollover(cat, p, true).unwrap();
        }
    }

    #[test]
    fn carry_chains_over_consecutive_rollover_periods() {
        let mut k = Kit::new(1, date!(2026 - 08 - 01));
        let makan = k.cat("Makan & Minum");
        set(&mut k, "Makan & Minum", "2026-08", 500_000, true);
        set(&mut k, "Makan & Minum", "2026-09", 500_000, true);
        set(&mut k, "Makan & Minum", "2026-10", 500_000, false);
        k.expense(date!(2026 - 08 - 10), "Makan & Minum", 300_000);
        k.expense(date!(2026 - 09 - 10), "Restoran", 600_000);
        let o = k.ledger.budget_overview(period("2026-10"), FAR).unwrap();
        let r = row(&o, makan);
        assert_eq!(
            (r.budget, r.carry, r.rollover),
            (Some(500_000), 100_000, false)
        );
        assert_eq!(o.totals.carry, 100_000);
        let sep = k.ledger.budget_overview(period("2026-09"), FAR).unwrap();
        assert_eq!(row(&sep, makan).carry, 200_000);
        assert!(row(&sep, makan).rollover);
        let nov = k.ledger.budget_overview(period("2026-11"), FAR).unwrap();
        assert_eq!(row(&nov, makan).carry, 0, "Oktober tanpa flag");
    }

    #[test]
    fn carry_resets_after_period_without_budget() {
        let mut k = Kit::new(1, date!(2026 - 08 - 01));
        let makan = k.cat("Makan & Minum");
        set(&mut k, "Makan & Minum", "2026-08", 500_000, true);
        set(&mut k, "Makan & Minum", "2026-10", 500_000, false);
        let o = k.ledger.budget_overview(period("2026-10"), FAR).unwrap();
        assert_eq!(row(&o, makan).carry, 0);
    }

    #[test]
    fn negative_carry_makes_available_negative() {
        let mut k = Kit::new(1, date!(2026 - 09 - 01));
        let makan = k.cat("Makan & Minum");
        set(&mut k, "Makan & Minum", "2026-09", 100_000, true);
        set(&mut k, "Makan & Minum", "2026-10", 100_000, false);
        k.expense(date!(2026 - 09 - 10), "Makan & Minum", 400_000);
        let o = k
            .ledger
            .budget_overview(period("2026-10"), date!(2026 - 10 - 15))
            .unwrap();
        let r = row(&o, makan);
        assert_eq!(r.carry, -300_000);
        assert_eq!(r.level, Some(BudgetLevel::Exceeded));
        assert_eq!(r.percent, None);
        assert_eq!(r.per_day, Some(0));
    }

    #[test]
    fn carry_uses_custom_period_boundaries() {
        let mut k = Kit::new(25, date!(2026 - 08 - 01));
        let makan = k.cat("Makan & Minum");
        set(&mut k, "Makan & Minum", "2026-10", 100_000, true);
        set(&mut k, "Makan & Minum", "2026-11", 100_000, false);
        k.expense(date!(2026 - 09 - 24), "Makan & Minum", 1_000);
        k.expense(date!(2026 - 09 - 25), "Makan & Minum", 10_000);
        k.expense(date!(2026 - 10 - 24), "Makan & Minum", 20_000);
        k.expense(date!(2026 - 10 - 25), "Makan & Minum", 40_000);
        let o = k.ledger.budget_overview(period("2026-11"), FAR).unwrap();
        assert_eq!(row(&o, makan).carry, 70_000);
    }

    #[test]
    fn transfer_does_not_affect_carry() {
        let mut k = Kit::new(1, date!(2026 - 09 - 01));
        let fee = k.cat("Biaya admin");
        set(&mut k, "Biaya admin", "2026-09", 10_000, true);
        set(&mut k, "Biaya admin", "2026-10", 10_000, false);
        k.transfer(date!(2026 - 09 - 05), 200_000, None);
        let o = k.ledger.budget_overview(period("2026-10"), FAR).unwrap();
        assert_eq!(row(&o, fee).carry, 10_000);
        k.transfer(date!(2026 - 09 - 06), 200_000, Some(2_500));
        let o = k.ledger.budget_overview(period("2026-10"), FAR).unwrap();
        assert_eq!(row(&o, fee).carry, 7_500);
    }

    #[test]
    fn per_day_only_for_current_period() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let (makan, kopi) = (k.cat("Makan & Minum"), k.cat("Kopi & jajan"));
        set(&mut k, "Makan & Minum", "2026-10", 310_000, false);
        k.expense(date!(2026 - 10 - 02), "Makan & Minum", 10_000);
        let today = date!(2026 - 10 - 22);
        let o = k.ledger.budget_overview(period("2026-10"), today).unwrap();
        assert_eq!(o.days_left, Some(10));
        assert_eq!(row(&o, makan).per_day, Some(30_000));
        assert_eq!(row(&o, kopi).per_day, None, "tanpa budget");
        assert_eq!(o.total_per_day, Some(30_000));
        let sep = k.ledger.budget_overview(period("2026-09"), today).unwrap();
        assert_eq!(sep.days_left, None);
        assert_eq!(sep.total_per_day, None);
        assert!(sep.rows.iter().all(|r| r.per_day.is_none()));
    }

    #[test]
    fn level_and_percent_follow_spending() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let (makan, bensin) = (k.cat("Makan & Minum"), k.cat("Bensin"));
        set(&mut k, "Makan & Minum", "2026-10", 100_000, false);
        k.expense(date!(2026 - 10 - 02), "Makan & Minum", 85_000);
        k.expense(date!(2026 - 10 - 02), "Bensin", 85_000);
        let o = k.ledger.budget_overview(period("2026-10"), FAR).unwrap();
        assert_eq!(row(&o, makan).level, Some(BudgetLevel::Warning));
        assert_eq!(row(&o, makan).percent, Some(85));
        assert_eq!(
            (row(&o, bensin).level, row(&o, bensin).percent),
            (None, None)
        );
    }

    #[test]
    fn set_budget_rollover_rules() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let oct = period("2026-10");
        let (gaji, kopi) = (k.cat("Gaji"), k.cat("Kopi & jajan"));
        assert!(matches!(
            k.ledger.set_budget_rollover(kopi, oct, true),
            Err(AppError::NotFound("budget"))
        ));
        assert!(matches!(
            k.ledger.set_budget_rollover(gaji, oct, true),
            Err(AppError::Budget(BudgetError::IncomeCategory))
        ));
        assert!(matches!(
            k.ledger.set_budget_rollover(Uuid::now_v7(), oct, true),
            Err(AppError::NotFound("category"))
        ));
        k.ledger.set_budget(kopi, oct, Some(10)).unwrap();
        k.ledger.set_budget_rollover(kopi, oct, true).unwrap();
        assert!(row(&k.ledger.budget_overview(oct, FAR).unwrap(), kopi).rollover);
        k.ledger.archive_category(kopi).unwrap();
        k.ledger.set_budget_rollover(kopi, oct, false).unwrap();
        assert!(!row(&k.ledger.budget_overview(oct, FAR).unwrap(), kopi).rollover);
        assert!(matches!(
            k.ledger.set_budget_rollover(kopi, oct, true),
            Err(AppError::Archived("category"))
        ));
    }
}
