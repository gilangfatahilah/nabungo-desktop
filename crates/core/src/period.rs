//! Periode budget. Satu-satunya tempat aturan "tanggal gajian" didefinisikan (PRD §10).

use time::{Date, Month};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PeriodError {
    #[error("period start day must be 1..=28, got {0}")]
    InvalidStartDay(u8),
    #[error("month must be 1..=12, got {0}")]
    InvalidMonth(u8),
}

/// Tanggal awal periode budget. Dibatasi 1..=28 agar ada di setiap bulan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodStartDay(u8);

impl PeriodStartDay {
    pub const FIRST: PeriodStartDay = PeriodStartDay(1);

    pub fn new(day: u8) -> Result<Self, PeriodError> {
        if (1..=28).contains(&day) {
            Ok(Self(day))
        } else {
            Err(PeriodError::InvalidStartDay(day))
        }
    }

    pub fn day(self) -> u8 {
        self.0
    }
}

/// Periode budget dinamai menurut bulan tempat periode itu **berakhir**:
/// dengan start day 25, periode "2026-11" = 25 Okt s/d 24 Nov.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BudgetPeriod {
    pub year: i32,
    pub month: u8,
}

impl BudgetPeriod {
    pub fn new(year: i32, month: u8) -> Result<Self, PeriodError> {
        if (1..=12).contains(&month) {
            Ok(Self { year, month })
        } else {
            Err(PeriodError::InvalidMonth(month))
        }
    }

    pub fn next(self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }

    pub fn prev(self) -> Self {
        if self.month == 1 {
            Self {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Self {
                year: self.year,
                month: self.month - 1,
            }
        }
    }

    /// Kunci yang disimpan di kolom `budget.period`, mis. `"2026-11"`.
    pub fn key(&self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }

    /// Rentang tanggal setengah terbuka `[start, end)`.
    pub fn date_range(self, start: PeriodStartDay) -> (Date, Date) {
        if start.0 == 1 {
            let next = self.next();
            (ymd(self.year, self.month, 1), ymd(next.year, next.month, 1))
        } else {
            let prev = self.prev();
            (
                ymd(prev.year, prev.month, start.0),
                ymd(self.year, self.month, start.0),
            )
        }
    }
}

pub fn period_of(date: Date, start: PeriodStartDay) -> BudgetPeriod {
    let current = BudgetPeriod {
        year: date.year(),
        month: u8::from(date.month()),
    };
    if start.0 == 1 || date.day() < start.0 {
        current
    } else {
        current.next()
    }
}

fn ymd(year: i32, month: u8, day: u8) -> Date {
    let month = Month::try_from(month).expect("month validated by BudgetPeriod");
    Date::from_calendar_date(year, month, day).expect("day <= 28 exists in every month")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use time::macros::date;

    fn p(year: i32, month: u8) -> BudgetPeriod {
        BudgetPeriod::new(year, month).unwrap()
    }

    #[test]
    fn calendar_month_when_start_day_is_first() {
        let s = PeriodStartDay::FIRST;
        assert_eq!(period_of(date!(2026 - 10 - 01), s), p(2026, 10));
        assert_eq!(period_of(date!(2026 - 10 - 31), s), p(2026, 10));
    }

    #[test]
    fn payday_start_day_25() {
        // PRD §5: awal bulan budget = 25 → transaksi 26 Oktober masuk periode November.
        let s = PeriodStartDay::new(25).unwrap();
        assert_eq!(period_of(date!(2026 - 10 - 24), s), p(2026, 10));
        assert_eq!(period_of(date!(2026 - 10 - 25), s), p(2026, 11));
        assert_eq!(period_of(date!(2026 - 10 - 26), s), p(2026, 11));
        assert_eq!(period_of(date!(2026 - 12 - 25), s), p(2027, 1));
    }

    #[test]
    fn date_range_of_payday_period() {
        let s = PeriodStartDay::new(25).unwrap();
        assert_eq!(
            p(2026, 11).date_range(s),
            (date!(2026 - 10 - 25), date!(2026 - 11 - 25))
        );
        assert_eq!(
            p(2027, 1).date_range(s),
            (date!(2026 - 12 - 25), date!(2027 - 01 - 25))
        );
        assert_eq!(
            p(2026, 12).date_range(PeriodStartDay::FIRST),
            (date!(2026 - 12 - 01), date!(2027 - 01 - 01))
        );
    }

    #[test]
    fn start_day_must_be_1_to_28() {
        for bad in [0, 29, 30, 31] {
            assert_eq!(
                PeriodStartDay::new(bad),
                Err(PeriodError::InvalidStartDay(bad))
            );
        }
        assert!(PeriodStartDay::new(28).is_ok());
    }

    #[test]
    fn key_is_zero_padded() {
        assert_eq!(p(2026, 3).key(), "2026-03");
        assert!(BudgetPeriod::new(2026, 13).is_err());
    }

    proptest! {
        #[test]
        fn date_falls_inside_its_period_range(jd in 2_451_545i32..2_488_070, start in 1u8..=28) {
            let d = time::Date::from_julian_day(jd).unwrap();
            let s = PeriodStartDay::new(start).unwrap();
            let (from, to) = period_of(d, s).date_range(s);
            prop_assert!(from <= d && d < to, "{d} not in [{from}, {to})");
        }
    }
}
