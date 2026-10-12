//! Tanggal transaksi: `YYYY-MM-DD`, tanpa jam dan tanpa timezone.

use time::Date;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;

const FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[year]-[month]-[day]");

pub fn parse_date(s: &str) -> Result<Date, time::error::Parse> {
    Date::parse(s, FORMAT)
}

pub fn format_date(d: Date) -> String {
    d.format(FORMAT).expect("YYYY-MM-DD is always formattable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn roundtrip() {
        let d = parse_date("2026-10-12").unwrap();
        assert_eq!(d, date!(2026 - 10 - 12));
        assert_eq!(format_date(d), "2026-10-12");
    }

    #[test]
    fn rejects_other_formats() {
        for bad in ["2026-13-01", "12/10/2026", "2026-1-5", "2026-02-30", ""] {
            assert!(parse_date(bad).is_err(), "{bad} should be rejected");
        }
    }
}
