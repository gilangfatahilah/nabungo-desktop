//! Use case budget (BUD-01): realisasi per periode budget, set nominal, salin dari periode lalu.

use std::collections::{HashMap, HashSet};

use nabungo_core::{
    BudgetCategory, BudgetError, BudgetLine, BudgetPeriod, BudgetTotals, CategoryKind,
    budget_totals, period_of, rollup_spent, validate_budget_amount,
};
use nabungo_storage::category::{self, CategoryRow};
use nabungo_storage::{budget, report};
use time::Date;
use uuid::Uuid;

use crate::ledger::{AppError, Ledger, active};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetRow {
    pub category_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub budget: Option<i64>,
    /// Positif = terpakai; parent mencakup child.
    pub spent: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetOverview {
    pub period: BudgetPeriod,
    pub from: Date,
    /// Inklusif.
    pub to: Date,
    pub rows: Vec<BudgetRow>,
    pub totals: BudgetTotals,
}

impl Ledger {
    pub fn period_of(&self, date: Date) -> Result<BudgetPeriod, AppError> {
        Ok(period_of(date, self.period_start_day()?))
    }

    /// Baris = kategori expense aktif, plus yang diarsip tetapi punya budget atau
    /// realisasi di periode ini; urutan pohon (parent lalu child-nya).
    pub fn budget_overview(&self, period: BudgetPeriod) -> Result<BudgetOverview, AppError> {
        let start = self.period_start_day()?;
        let (from, to) = period.date_range(start);
        let cats: Vec<CategoryRow> = category::list_categories(&self.conn)?
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
        let direct: HashMap<Uuid, i64> = report::category_totals(&self.conn, from, to)?
            .into_iter()
            .collect();
        let spent = rollup_spent(&tree, &direct);
        let budgets: HashMap<Uuid, i64> = budget::list_budgets(&self.conn, &period.key())?
            .into_iter()
            .map(|b| (b.category_id, b.amount))
            .collect();

        let mut rows = Vec::new();
        for root in cats.iter().filter(|c| c.parent_id.is_none()) {
            let children = cats.iter().filter(|c| c.parent_id == Some(root.id));
            for c in std::iter::once(root).chain(children) {
                let row = BudgetRow {
                    category_id: c.id,
                    parent_id: c.parent_id,
                    budget: budgets.get(&c.id).copied(),
                    spent: spent.get(&c.id).copied().unwrap_or(0),
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
                spent: r.spent,
            })
            .collect();
        Ok(BudgetOverview {
            period,
            from,
            to: period.last_day(start),
            totals: budget_totals(&tree, &lines)?,
            rows,
        })
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

    /// Salin budget `to.prev()` ke `to` untuk kategori yang belum punya budget di `to`
    /// dan tidak diarsip. Tidak menimpa. Mengembalikan jumlah baris yang disalin.
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
                    copied += 1;
                }
                _ => {}
            }
        }
        tx.commit()?;
        Ok(copied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Kit;
    use crate::{BudgetError, CategoryKind};
    use time::macros::date;

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
        let before = k.ledger.budget_overview(oct).unwrap();
        assert_eq!(row(&before, makan).spent, 25_000);
        let t = k.transfer(date!(2026 - 10 - 03), 200_000, None);
        assert_eq!(k.ledger.budget_overview(oct).unwrap(), before);
        k.ledger.delete_txn(t).unwrap();
        assert_eq!(k.ledger.budget_overview(oct).unwrap(), before);
        k.income(date!(2026 - 10 - 04), "Gaji", 8_000_000);
        assert_eq!(k.ledger.budget_overview(oct).unwrap(), before);
    }

    #[test]
    fn custom_start_day_period_boundaries() {
        let mut k = Kit::new(25, date!(2026 - 09 - 01));
        k.expense(date!(2026 - 10 - 24), "Kopi & jajan", 10_000);
        k.expense(date!(2026 - 10 - 25), "Kopi & jajan", 20_000);
        let kopi = k.cat("Kopi & jajan");
        let oct = k.ledger.budget_overview(period("2026-10")).unwrap();
        let nov = k.ledger.budget_overview(period("2026-11")).unwrap();
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
        let o = k.ledger.budget_overview(period("2026-10")).unwrap();
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
        let o = k.ledger.budget_overview(period("2026-10")).unwrap();
        assert_eq!(row(&o, resto).spent, 50_000);
        assert_eq!(row(&o, makan).spent, 50_000);
        k.ledger.archive_category(kopi).unwrap();
        let o = k.ledger.budget_overview(period("2026-10")).unwrap();
        assert!(
            !o.rows.iter().any(|r| r.category_id == kopi),
            "archived & unused hidden"
        );
        let nov = k.ledger.budget_overview(period("2026-11")).unwrap();
        assert!(!nov.rows.iter().any(|r| r.category_id == resto));
    }

    #[test]
    fn budget_follows_delete_and_undo() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let kopi = k.cat("Kopi & jajan");
        let t = k.expense(date!(2026 - 10 - 02), "Kopi & jajan", 25_000);
        k.ledger.delete_txn(t).unwrap();
        assert_eq!(
            row(&k.ledger.budget_overview(period("2026-10")).unwrap(), kopi).spent,
            0
        );
        k.ledger.undo_delete_txn(t).unwrap();
        assert_eq!(
            row(&k.ledger.budget_overview(period("2026-10")).unwrap(), kopi).spent,
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
        let o = k.ledger.budget_overview(oct).unwrap();
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
        let o = k.ledger.budget_overview(oct).unwrap();
        assert_eq!(
            o.totals,
            BudgetTotals {
                budget: 800_000,
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
        let o = k.ledger.budget_overview(oct).unwrap();
        assert_eq!(row(&o, makan).budget, Some(0));
        k.ledger.set_budget(makan, oct, None).unwrap();
        assert_eq!(
            row(&k.ledger.budget_overview(oct).unwrap(), makan).budget,
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
        k.ledger.set_budget(hiburan, oct, Some(100_000)).unwrap();
        k.ledger.archive_category(hiburan).unwrap();
        k.ledger.set_budget(makan, nov, Some(450_000)).unwrap();
        assert_eq!(k.ledger.copy_budgets(nov).unwrap(), 1);
        let o = k.ledger.budget_overview(nov).unwrap();
        assert_eq!(row(&o, makan).budget, Some(450_000));
        assert_eq!(row(&o, transport).budget, Some(300_000));
        assert!(!o.rows.iter().any(|r| r.category_id == hiburan));
        assert_eq!(k.ledger.copy_budgets(nov).unwrap(), 0);
    }
}
