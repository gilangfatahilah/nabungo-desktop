//! Budget per kategori expense: validasi nominal, rollup realisasi, rollover,
//! level peringatan, dan sisa per hari (PRD §4.4, BUD-02..04).

use std::collections::{BTreeMap, HashMap, HashSet};

use time::Date;
use uuid::Uuid;

use crate::{BudgetPeriod, MAX_AMOUNT, MoneyError};

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
    /// Bawaan rollover dari periode sebelumnya; 0 bila tanpa budget/rantai.
    pub carry: i64,
    pub spent: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BudgetTotals {
    pub budget: i64,
    pub carry: i64,
    pub spent: i64,
}

/// Tingkat realisasi terhadap budget tersedia; urutan dipakai untuk mendeteksi
/// kenaikan level (peringatan hanya saat melewati ambang).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BudgetLevel {
    Ok,
    Warning,
    Exceeded,
}

/// `Warning` mulai 80%, `Exceeded` mulai 100% dari `available` (= budget + carry).
/// `available <= 0`: setiap realisasi (atau carry negatif) sudah terlampaui.
pub fn budget_level(available: i64, spent: i64) -> BudgetLevel {
    if available > 0 {
        let (a, s) = (i128::from(available), i128::from(spent));
        if s >= a {
            BudgetLevel::Exceeded
        } else if s * 100 >= a * 80 {
            BudgetLevel::Warning
        } else {
            BudgetLevel::Ok
        }
    } else if spent > 0 || available < 0 {
        BudgetLevel::Exceeded
    } else {
        BudgetLevel::Ok
    }
}

/// Persen terpakai dibulatkan ke bawah; `None` bila tidak ada budget positif.
pub fn spent_percent(available: i64, spent: i64) -> Option<u32> {
    if available <= 0 {
        return None;
    }
    let pct = i128::from(spent.max(0)) * 100 / i128::from(available);
    Some(u32::try_from(pct).unwrap_or(u32::MAX))
}

/// Jumlah hari dari `today` sampai `last_day` (inklusif) bila `today` ada di
/// `[from, last_day]`; selain itu `None`.
pub fn days_left(today: Date, from: Date, last_day: Date) -> Option<u32> {
    if today < from || today > last_day {
        return None;
    }
    u32::try_from((last_day - today).whole_days() + 1).ok()
}

/// Sisa per hari, dibulatkan ke bawah; sisa negatif menjadi 0.
pub fn per_day(remaining: i64, days_left: u32) -> i64 {
    remaining.max(0) / i64::from(days_left.max(1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetEntry {
    pub amount: i64,
    pub rollover: bool,
}

/// Rantai rollover yang berakhir di `period - 1`: periode berurutan mundur yang
/// punya budget dengan `rollover`. Mengembalikan awal rantai dan Σ nominalnya;
/// `None` bila periode sebelumnya tanpa budget atau tanpa flag.
pub fn rollover_chain(
    entries: &BTreeMap<BudgetPeriod, BudgetEntry>,
    period: BudgetPeriod,
) -> Result<Option<(BudgetPeriod, i64)>, MoneyError> {
    let mut chain = None;
    let mut sum = 0i64;
    let mut p = period.prev();
    while let Some(e) = entries.get(&p).filter(|e| e.rollover) {
        sum = sum.checked_add(e.amount).ok_or(MoneyError::Overflow)?;
        chain = Some(p);
        p = p.prev();
    }
    Ok(chain.map(|start| (start, sum)))
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
    let (mut budget, mut carry, mut spent) = (0i128, 0i128, 0i128);
    for l in lines {
        let Some(amount) = l.budget else { continue };
        let covered = parent
            .get(&l.category_id)
            .copied()
            .flatten()
            .is_some_and(|p| budgeted.contains(&p));
        if !covered {
            budget += i128::from(amount);
            carry += i128::from(l.carry);
            spent += i128::from(l.spent);
        }
    }
    let fit = |v: i128| i64::try_from(v).map_err(|_| MoneyError::Overflow);
    Ok(BudgetTotals {
        budget: fit(budget)?,
        carry: fit(carry)?,
        spent: fit(spent)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PeriodStartDay;
    use time::macros::date;

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
            carry: 0,
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
                carry: 0,
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
                carry: 0,
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

    #[test]
    fn totals_include_carry_of_counted_rows() {
        let f = fixture();
        let lines = [
            BudgetLine {
                carry: 20,
                ..line(f.food, Some(100), 0)
            },
            BudgetLine {
                carry: 5,
                ..line(f.resto, Some(50), 0)
            },
        ];
        assert_eq!(budget_totals(&f.cats, &lines).unwrap().carry, 20);
    }

    #[test]
    fn level_table() {
        use BudgetLevel::*;
        for (available, spent, want) in [
            (100, 0, Ok),
            (100, 79, Ok),
            (100, 80, Warning),
            (100, 99, Warning),
            (100, 100, Exceeded),
            (100, 150, Exceeded),
            (0, 0, Ok),
            (0, 1, Exceeded),
            (-50, 0, Exceeded),
            (100, -20, Ok),
            (MAX_AMOUNT, MAX_AMOUNT, Exceeded),
        ] {
            assert_eq!(budget_level(available, spent), want, "{available}/{spent}");
        }
        assert!(Ok < Warning && Warning < Exceeded);
    }

    #[test]
    fn percent_table() {
        for (available, spent, want) in [
            (100, 85, Some(85)),
            (300, 100, Some(33)),
            (100, -5, Some(0)),
            (0, 10, None),
            (-1, 0, None),
            (100, 250, Some(250)),
        ] {
            assert_eq!(spent_percent(available, spent), want, "{available}/{spent}");
        }
    }

    #[test]
    fn days_left_inclusive_to_period_end() {
        let start = PeriodStartDay::new(25).unwrap();
        let p = BudgetPeriod::new(2026, 11).unwrap();
        let (from, _) = p.date_range(start);
        let last = p.last_day(start);
        assert_eq!((from, last), (date!(2026 - 10 - 25), date!(2026 - 11 - 24)));
        assert_eq!(days_left(date!(2026 - 11 - 24), from, last), Some(1));
        assert_eq!(days_left(date!(2026 - 10 - 25), from, last), Some(31));
        assert_eq!(days_left(date!(2026 - 11 - 25), from, last), None);
        assert_eq!(days_left(date!(2026 - 10 - 24), from, last), None);
    }

    #[test]
    fn per_day_never_negative() {
        assert_eq!(per_day(310_000, 31), 10_000);
        assert_eq!(per_day(100, 3), 33);
        assert_eq!(per_day(-5_000, 4), 0);
    }

    fn p(month: u8) -> BudgetPeriod {
        BudgetPeriod::new(2026, month).unwrap()
    }

    fn entry(amount: i64, rollover: bool) -> BudgetEntry {
        BudgetEntry { amount, rollover }
    }

    #[test]
    fn rollover_chain_sums_consecutive_flagged_periods() {
        let entries = BTreeMap::from([
            (p(8), entry(100, true)),
            (p(9), entry(200, true)),
            (p(10), entry(300, false)),
        ]);
        assert_eq!(rollover_chain(&entries, p(10)), Ok(Some((p(8), 300))));
        assert_eq!(rollover_chain(&entries, p(9)), Ok(Some((p(8), 100))));
        assert_eq!(rollover_chain(&entries, p(11)), Ok(None));
        assert_eq!(rollover_chain(&entries, p(8)), Ok(None));
    }

    #[test]
    fn rollover_chain_stops_at_gap() {
        let entries = BTreeMap::from([(p(8), entry(100, true))]);
        assert_eq!(rollover_chain(&entries, p(10)), Ok(None));
        let entries = BTreeMap::from([(p(7), entry(50, true)), (p(8), entry(100, true))]);
        assert_eq!(rollover_chain(&entries, p(10)), Ok(None));
        let entries = BTreeMap::from([
            (p(7), entry(50, true)),
            (p(8), entry(100, false)),
            (p(9), entry(10, true)),
        ]);
        assert_eq!(rollover_chain(&entries, p(10)), Ok(Some((p(9), 10))));
    }

    #[test]
    fn rollover_chain_overflow_is_error() {
        let entries = BTreeMap::from([(p(8), entry(i64::MAX, true)), (p(9), entry(1, true))]);
        assert_eq!(rollover_chain(&entries, p(10)), Err(MoneyError::Overflow));
    }
}
