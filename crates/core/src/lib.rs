//! Domain murni Nabungo: Money, Account, Txn, Posting, validasi invariant,
//! kalkulasi budget & alokasi. Crate ini TIDAK boleh bergantung pada IO
//! (rusqlite, tauri, filesystem).
//!
//! Nominal uang selalu `i64` dalam satuan minor; floating point dilarang.
#![deny(clippy::float_arithmetic)]

pub mod date;
pub mod money;
pub mod period;

pub use money::{CurrencyCode, Money, MoneyError};
pub use period::{BudgetPeriod, PeriodError, PeriodStartDay, period_of};

/// Versi crate domain, dipakai untuk smoke test lintas layer.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_manifest() {
        assert_eq!(version(), "0.1.0");
    }
}
