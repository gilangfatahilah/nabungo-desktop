//! Domain murni Nabungo: Money, Account, Txn, Posting, validasi invariant,
//! kalkulasi budget & alokasi. Crate ini TIDAK boleh bergantung pada IO
//! (rusqlite, tauri, filesystem).
//!
//! Nominal uang selalu `i64` dalam satuan minor; floating point dilarang.
#![deny(clippy::float_arithmetic)]

pub mod allocation;
pub mod amount;
pub mod budget;
pub mod catalog;
pub mod date;
pub mod form;
pub mod kinds;
pub mod money;
pub mod networth;
pub mod period;
pub mod recurrence;
pub mod txn;

pub use allocation::{
    AccountBalance, AllocationError, AllocationReport, AmountWarning, Bucket, BucketActual,
    Mapping, SiblingWarning, Target, Unallocated, allocation_report, check_bp, check_shares,
    check_sibling_targets, check_target,
};
pub use amount::{AmountParseError, parse_amount};
pub use budget::{
    BudgetCategory, BudgetEntry, BudgetError, BudgetLevel, BudgetLine, BudgetTotals, budget_level,
    budget_totals, days_left, per_day, rollover_chain, rollup_spent, spent_percent,
    validate_budget_amount,
};
pub use catalog::{
    CatalogError, MAX_NAME_LEN, MergeSide, ParentInfo, check_merge, check_parent, normalize_name,
    validate_credit_limit, validate_statement_day,
};
pub use form::{Fee, FormBody, PostingSpec, SplitLine, form_postings};
pub use kinds::{AccountKind, CategoryKind, TxnKind};
pub use money::{CurrencyCode, Money, MoneyError};
pub use networth::{NetWorth, net_worth};
pub use period::{BudgetPeriod, PeriodError, PeriodStartDay, period_of};
pub use txn::{
    AccountRef, CategoryRef, MAX_AMOUNT, PostingDraft, TxnDraft, TxnError, ValidTxn, validate,
};

/// Versi crate domain, dipakai untuk smoke test lintas layer.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_manifest() {
        assert_eq!(version(), "0.4.0");
    }
}
