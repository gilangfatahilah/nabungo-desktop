//! Pengulangan transaksi terjadwal (TRX-06): subset RRULE RFC 5545.
//!
//! Hanya `FREQ` (DAILY/WEEKLY/MONTHLY/YEARLY), `INTERVAL`, `BYDAY` (satu hari),
//! `BYMONTHDAY`, `BYMONTH`, dan `UNTIL` (tanggal). Berbeda dari RFC, tanggal yang tidak
//! ada di bulan pendek di-clamp ke hari terakhir bulan (tagihan tanggal 31 jatuh di
//! 28/29 Feb), lalu kembali ke tanggal semula di bulan berikutnya.

use time::{Date, Month, Weekday};

pub const MAX_INTERVAL: u8 = 99;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecurrenceError {
    #[error("interval must be 1..={MAX_INTERVAL}")]
    Interval,
    #[error("end date is before the start date")]
    UntilBeforeStart,
    #[error("invalid rrule: {0}")]
    Invalid(String),
}

/// Anchor (hari/tanggal/bulan) diturunkan dari tanggal mulai; fasenya dibawa oleh
/// tanggal kejadian terakhir yang diberikan ke `next_after`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Anchor {
    Daily,
    Weekly(Weekday),
    Monthly { day: u8 },
    Yearly { month: Month, day: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recurrence {
    anchor: Anchor,
    interval: u8,
    until: Option<Date>,
}

const DAYS: [(Weekday, &str); 7] = [
    (Weekday::Monday, "MO"),
    (Weekday::Tuesday, "TU"),
    (Weekday::Wednesday, "WE"),
    (Weekday::Thursday, "TH"),
    (Weekday::Friday, "FR"),
    (Weekday::Saturday, "SA"),
    (Weekday::Sunday, "SU"),
];

fn check_interval(interval: u8) -> Result<u8, RecurrenceError> {
    if (1..=MAX_INTERVAL).contains(&interval) {
        Ok(interval)
    } else {
        Err(RecurrenceError::Interval)
    }
}

/// Tanggal `day` di bulan itu, di-clamp ke hari terakhir bulan.
fn clamped(year: i32, month: Month, day: u8) -> Option<Date> {
    Date::from_calendar_date(year, month, day.min(month.length(year))).ok()
}

impl Recurrence {
    pub fn new(
        freq: Freq,
        interval: u8,
        anchor: Date,
        until: Option<Date>,
    ) -> Result<Self, RecurrenceError> {
        let interval = check_interval(interval)?;
        if until.is_some_and(|u| u < anchor) {
            return Err(RecurrenceError::UntilBeforeStart);
        }
        let anchor = match freq {
            Freq::Daily => Anchor::Daily,
            Freq::Weekly => Anchor::Weekly(anchor.weekday()),
            Freq::Monthly => Anchor::Monthly { day: anchor.day() },
            Freq::Yearly => Anchor::Yearly {
                month: anchor.month(),
                day: anchor.day(),
            },
        };
        Ok(Self {
            anchor,
            interval,
            until,
        })
    }

    pub fn parse(rrule: &str) -> Result<Self, RecurrenceError> {
        let bad = || RecurrenceError::Invalid(rrule.to_owned());
        let (mut freq, mut interval, mut byday, mut bymonthday, mut bymonth, mut until) =
            (None, None, None, None, None, None);
        for part in rrule.split(';') {
            let (key, value) = part.split_once('=').ok_or_else(bad)?;
            let num = |max: u8| {
                value
                    .parse::<u8>()
                    .ok()
                    .filter(|n| (1..=max).contains(n))
                    .ok_or_else(bad)
            };
            match key {
                "FREQ" => {
                    freq = Some(match value {
                        "DAILY" => Freq::Daily,
                        "WEEKLY" => Freq::Weekly,
                        "MONTHLY" => Freq::Monthly,
                        "YEARLY" => Freq::Yearly,
                        _ => return Err(bad()),
                    })
                }
                "INTERVAL" => interval = Some(num(MAX_INTERVAL)?),
                "BYDAY" => {
                    byday = Some(
                        DAYS.iter()
                            .find(|(_, code)| *code == value)
                            .map(|(d, _)| *d)
                            .ok_or_else(bad)?,
                    )
                }
                "BYMONTHDAY" => bymonthday = Some(num(31)?),
                "BYMONTH" => bymonth = Some(Month::try_from(num(12)?).map_err(|_| bad())?),
                "UNTIL" => until = Some(parse_until(value).ok_or_else(bad)?),
                _ => return Err(bad()),
            }
        }
        let anchor = match freq.ok_or_else(bad)? {
            Freq::Daily => Anchor::Daily,
            Freq::Weekly => Anchor::Weekly(byday.ok_or_else(bad)?),
            Freq::Monthly => Anchor::Monthly {
                day: bymonthday.ok_or_else(bad)?,
            },
            Freq::Yearly => Anchor::Yearly {
                month: bymonth.ok_or_else(bad)?,
                day: bymonthday.ok_or_else(bad)?,
            },
        };
        Ok(Self {
            anchor,
            interval: interval.unwrap_or(1),
            until,
        })
    }

    pub fn to_rrule(&self) -> String {
        let n = self.interval;
        let mut s = match self.anchor {
            Anchor::Daily => format!("FREQ=DAILY;INTERVAL={n}"),
            Anchor::Weekly(day) => {
                let code = DAYS
                    .iter()
                    .find(|(d, _)| *d == day)
                    .map_or("MO", |(_, c)| c);
                format!("FREQ=WEEKLY;INTERVAL={n};BYDAY={code}")
            }
            Anchor::Monthly { day } => format!("FREQ=MONTHLY;INTERVAL={n};BYMONTHDAY={day}"),
            Anchor::Yearly { month, day } => {
                format!(
                    "FREQ=YEARLY;INTERVAL={n};BYMONTH={};BYMONTHDAY={day}",
                    u8::from(month)
                )
            }
        };
        if let Some(u) = self.until {
            s.push_str(&format!(
                ";UNTIL={:04}{:02}{:02}",
                u.year(),
                u8::from(u.month()),
                u.day()
            ));
        }
        s
    }

    pub fn freq(&self) -> Freq {
        match self.anchor {
            Anchor::Daily => Freq::Daily,
            Anchor::Weekly(_) => Freq::Weekly,
            Anchor::Monthly { .. } => Freq::Monthly,
            Anchor::Yearly { .. } => Freq::Yearly,
        }
    }

    pub fn interval(&self) -> u8 {
        self.interval
    }

    pub fn until(&self) -> Option<Date> {
        self.until
    }

    /// Kejadian setelah `d` (kejadian sebelumnya); `None` bila melewati UNTIL atau
    /// di luar rentang tanggal yang didukung.
    pub fn next_after(&self, d: Date) -> Option<Date> {
        let n = i64::from(self.interval);
        let next = match self.anchor {
            Anchor::Daily => d.checked_add(time::Duration::days(n))?,
            Anchor::Weekly(_) => d.checked_add(time::Duration::weeks(n))?,
            Anchor::Monthly { day } => {
                let months = i64::from(d.year()) * 12 + i64::from(u8::from(d.month())) - 1 + n;
                let year = i32::try_from(months.div_euclid(12)).ok()?;
                let month = Month::try_from(u8::try_from(months.rem_euclid(12) + 1).ok()?).ok()?;
                clamped(year, month, day)?
            }
            Anchor::Yearly { month, day } => {
                clamped(d.year().checked_add(i32::from(self.interval))?, month, day)?
            }
        };
        match self.until {
            Some(u) if next > u => None,
            _ => Some(next),
        }
    }

    /// `start` lalu kejadian berikutnya, maksimal `n`, berhenti di UNTIL.
    pub fn occurrences(&self, start: Date, n: usize) -> Vec<Date> {
        let mut out = Vec::new();
        let mut cur = Some(start).filter(|d| self.until.is_none_or(|u| *d <= u));
        while let Some(d) = cur {
            if out.len() >= n {
                break;
            }
            out.push(d);
            cur = self.next_after(d);
        }
        out
    }
}

fn parse_until(s: &str) -> Option<Date> {
    if s.len() != 8 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let year = s[0..4].parse().ok()?;
    let month = Month::try_from(s[4..6].parse::<u8>().ok()?).ok()?;
    Date::from_calendar_date(year, month, s[6..8].parse().ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    fn rec(freq: Freq, interval: u8, anchor: Date, until: Option<Date>) -> Recurrence {
        Recurrence::new(freq, interval, anchor, until).unwrap()
    }

    #[test]
    fn rrule_round_trip() {
        let monthly = rec(Freq::Monthly, 1, date!(2026 - 01 - 31), None);
        assert_eq!(monthly.to_rrule(), "FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=31");
        let weekly = rec(
            Freq::Weekly,
            2,
            date!(2026 - 10 - 19),
            Some(date!(2026 - 12 - 31)),
        );
        assert_eq!(
            weekly.to_rrule(),
            "FREQ=WEEKLY;INTERVAL=2;BYDAY=MO;UNTIL=20261231"
        );
        let yearly = rec(Freq::Yearly, 1, date!(2028 - 02 - 29), None);
        assert_eq!(
            yearly.to_rrule(),
            "FREQ=YEARLY;INTERVAL=1;BYMONTH=2;BYMONTHDAY=29"
        );
        let daily = rec(Freq::Daily, 3, date!(2026 - 10 - 19), None);
        assert_eq!(daily.to_rrule(), "FREQ=DAILY;INTERVAL=3");
        for r in [monthly, weekly, yearly, daily] {
            assert_eq!(Recurrence::parse(&r.to_rrule()).unwrap(), r);
        }
        let any_order = Recurrence::parse("INTERVAL=1;FREQ=DAILY").unwrap();
        assert_eq!(any_order.freq(), Freq::Daily);
        assert_eq!(any_order.interval(), 1);
    }

    #[test]
    fn parse_rejects_unknown_and_bad() {
        for bad in [
            "FREQ=HOURLY",
            "FREQ=DAILY;COUNT=3",
            "FREQ=MONTHLY;INTERVAL=1",
            "FREQ=DAILY;INTERVAL=0",
            "FREQ=WEEKLY;INTERVAL=1;BYDAY=XX",
            "FREQ=YEARLY;INTERVAL=1;BYMONTH=13;BYMONTHDAY=1",
            "FREQ=DAILY;INTERVAL=1;UNTIL=2026-01-01",
            "",
        ] {
            assert!(Recurrence::parse(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn interval_bounds() {
        let d = date!(2026 - 01 - 01);
        assert_eq!(
            Recurrence::new(Freq::Daily, 0, d, None),
            Err(RecurrenceError::Interval)
        );
        assert_eq!(
            Recurrence::new(Freq::Daily, 100, d, None),
            Err(RecurrenceError::Interval)
        );
        assert!(Recurrence::new(Freq::Daily, MAX_INTERVAL, d, None).is_ok());
    }

    #[test]
    fn until_before_start() {
        assert_eq!(
            Recurrence::new(
                Freq::Daily,
                1,
                date!(2026 - 05 - 02),
                Some(date!(2026 - 05 - 01))
            ),
            Err(RecurrenceError::UntilBeforeStart)
        );
        let d = date!(2026 - 05 - 02);
        assert!(Recurrence::new(Freq::Daily, 1, d, Some(d)).is_ok());
    }

    #[test]
    fn daily_and_weekly_steps() {
        let daily = rec(Freq::Daily, 3, date!(2026 - 12 - 30), None);
        assert_eq!(
            daily.next_after(date!(2026 - 12 - 30)),
            Some(date!(2027 - 01 - 02))
        );
        let weekly = rec(Freq::Weekly, 2, date!(2026 - 10 - 19), None);
        assert_eq!(
            weekly.next_after(date!(2026 - 10 - 19)),
            Some(date!(2026 - 11 - 02))
        );
    }

    #[test]
    fn monthly_31_clamps_and_returns() {
        let r = rec(Freq::Monthly, 1, date!(2026 - 01 - 31), None);
        assert_eq!(
            r.occurrences(date!(2026 - 01 - 31), 4),
            vec![
                date!(2026 - 01 - 31),
                date!(2026 - 02 - 28),
                date!(2026 - 03 - 31),
                date!(2026 - 04 - 30)
            ]
        );
        let leap = rec(Freq::Monthly, 1, date!(2028 - 01 - 31), None);
        assert_eq!(
            leap.occurrences(date!(2028 - 01 - 31), 2)[1],
            date!(2028 - 02 - 29)
        );
        let every2 = rec(Freq::Monthly, 2, date!(2026 - 11 - 30), None);
        assert_eq!(
            every2.next_after(date!(2026 - 11 - 30)),
            Some(date!(2027 - 01 - 30))
        );
    }

    #[test]
    fn yearly_feb29_clamps() {
        let r = rec(Freq::Yearly, 1, date!(2028 - 02 - 29), None);
        assert_eq!(
            r.occurrences(date!(2028 - 02 - 29), 5),
            vec![
                date!(2028 - 02 - 29),
                date!(2029 - 02 - 28),
                date!(2030 - 02 - 28),
                date!(2031 - 02 - 28),
                date!(2032 - 02 - 29)
            ]
        );
    }

    #[test]
    fn until_stops() {
        let r = rec(
            Freq::Monthly,
            1,
            date!(2026 - 01 - 15),
            Some(date!(2026 - 03 - 15)),
        );
        assert_eq!(r.occurrences(date!(2026 - 01 - 15), 10).len(), 3);
        assert_eq!(r.next_after(date!(2026 - 03 - 15)), None);
        assert_eq!(r.until(), Some(date!(2026 - 03 - 15)));
    }
}
