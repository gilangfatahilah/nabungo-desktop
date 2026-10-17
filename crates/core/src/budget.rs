//! Budget per kategori expense: validasi nominal dan rollup realisasi (PRD §4.4).

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::{MAX_AMOUNT, MoneyError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BudgetError {
    #[error("budget amount cannot be negative")]
    Negative,
    #[error("budget amount exceeds the maximum of {MAX_AMOUNT}")]
    TooLarge,
    #[error("budgets are for expense categories only")]
    IncomeCategory,
}

/// Nominal budget valid: `0..=MAX_AMOUNT`. Nol berarti "tidak boleh belanja".
pub fn validate_budget_amount(amount: i64) -> Result<i64, BudgetError> {
    if amount < 0 {
        Err(BudgetError::Negative)
    } else if amount > MAX_AMOUNT {
        Err(BudgetError::TooLarge)
    } else {
        Ok(amount)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetCategory {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetLine {
    pub category_id: Uuid,
    pub budget: Option<i64>,
    pub spent: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BudgetTotals {
    pub budget: i64,
    pub spent: i64,
}

/// Realisasi (positif = terpakai) per kategori. `direct` berisi Σ posting
/// bertanda per kategori (expense negatif, refund positif); parent mencakup
/// child-nya. Memakai `saturating_*`: setiap posting dibatasi `MAX_AMOUNT`,
/// sehingga overflow i64 butuh ~9 juta posting maksimum dalam satu periode.
pub fn rollup_spent(
    categories: &[BudgetCategory],
    direct: &HashMap<Uuid, i64>,
) -> HashMap<Uuid, i64> {
    let mut spent: HashMap<Uuid, i64> = categories.iter().map(|c| (c.id, 0)).collect();
    for c in categories {
        let own = direct.get(&c.id).copied().unwrap_or(0).saturating_neg();
        for id in std::iter::once(c.id).chain(c.parent_id) {
            if let Some(v) = spent.get_mut(&id) {
                *v = v.saturating_add(own);
            }
        }
    }
    spent
}

/// Total budget & terpakai tanpa hitung ganda: hanya baris ber-budget yang
/// parent-nya tidak ber-budget yang dijumlahkan.
pub fn budget_totals(
    categories: &[BudgetCategory],
    lines: &[BudgetLine],
) -> Result<BudgetTotals, MoneyError> {
    let budgeted: HashSet<Uuid> = lines
        .iter()
        .filter(|l| l.budget.is_some())
        .map(|l| l.category_id)
        .collect();
    let parent: HashMap<Uuid, Option<Uuid>> =
        categories.iter().map(|c| (c.id, c.parent_id)).collect();
    let (mut budget, mut spent) = (0i128, 0i128);
    for l in lines {
        let Some(amount) = l.budget else { continue };
        let covered = parent
            .get(&l.category_id)
            .copied()
            .flatten()
            .is_some_and(|p| budgeted.contains(&p));
        if !covered {
            budget += i128::from(amount);
            spent += i128::from(l.spent);
        }
    }
    let fit = |v: i128| i64::try_from(v).map_err(|_| MoneyError::Overflow);
    Ok(BudgetTotals {
        budget: fit(budget)?,
        spent: fit(spent)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        cats: Vec<BudgetCategory>,
        food: Uuid,
        resto: Uuid,
        transport: Uuid,
        bills: Uuid,
    }

    fn fixture() -> Fixture {
        let [food, resto, transport, bills] = [(); 4].map(|_| Uuid::now_v7());
        let cat = |id, parent_id| BudgetCategory { id, parent_id };
        Fixture {
            cats: vec![
                cat(food, None),
                cat(resto, Some(food)),
                cat(transport, None),
                cat(bills, None),
            ],
            food,
            resto,
            transport,
            bills,
        }
    }

    fn line(category_id: Uuid, budget: Option<i64>, spent: i64) -> BudgetLine {
        BudgetLine {
            category_id,
            budget,
            spent,
        }
    }

    #[test]
    fn spent_rolls_up_children_and_refund_reduces() {
        let f = fixture();
        let direct = HashMap::from([
            (f.food, -10_000),
            (f.resto, -50_000),
            (f.transport, -30_000 + 5_000),
        ]);
        let s = rollup_spent(&f.cats, &direct);
        assert_eq!(s[&f.food], 60_000);
        assert_eq!(s[&f.resto], 50_000);
        assert_eq!(s[&f.transport], 25_000);
        assert_eq!(s[&f.bills], 0);
    }

    #[test]
    fn totals_do_not_double_count_parent_and_child_budgets() {
        let f = fixture();
        let lines = [
            line(f.food, Some(100_000), 60_000),
            line(f.resto, Some(70_000), 50_000),
            line(f.transport, None, 25_000),
            line(f.bills, Some(50_000), 0),
        ];
        assert_eq!(
            budget_totals(&f.cats, &lines).unwrap(),
            BudgetTotals {
                budget: 150_000,
                spent: 60_000
            }
        );
    }

    #[test]
    fn child_budget_counts_when_parent_has_none() {
        let f = fixture();
        let lines = [
            line(f.food, None, 60_000),
            line(f.resto, Some(70_000), 50_000),
        ];
        assert_eq!(
            budget_totals(&f.cats, &lines).unwrap(),
            BudgetTotals {
                budget: 70_000,
                spent: 50_000
            }
        );
    }

    #[test]
    fn budget_amount_bounds() {
        assert_eq!(validate_budget_amount(0), Ok(0));
        assert_eq!(validate_budget_amount(MAX_AMOUNT), Ok(MAX_AMOUNT));
        assert_eq!(validate_budget_amount(-1), Err(BudgetError::Negative));
        assert_eq!(
            validate_budget_amount(MAX_AMOUNT + 1),
            Err(BudgetError::TooLarge)
        );
    }
}
