//! Use case / service layer: TransactionService, BudgetService, AllocationService,
//! ReportService, BackupService. Tidak tahu apa-apa soal Tauri.

/// Versi domain yang dipakai app (smoke test lintas layer).
pub fn version() -> &'static str {
    nabungo_core::version()
}

mod accounts;
mod budget;
mod categories;
mod defaults;
mod ledger;
mod onboarding;
mod payee;
mod settings;
#[cfg(test)]
mod testkit;
mod transactions;
mod valuation;

pub use accounts::{AccountFields, AccountsOverview};
pub use budget::{BudgetOverview, BudgetRow};
pub use categories::CategoryFields;
pub use ledger::{AppError, Ledger, PostingInput, TxnInput};
pub use nabungo_core::date::{format_date, parse_date};
pub use nabungo_core::{
    AccountKind, AmountParseError, BudgetError, BudgetPeriod, BudgetTotals, CatalogError,
    CategoryKind, Fee, FormBody, MoneyError, NetWorth, PeriodError, PeriodStartDay, SplitLine,
    TxnError, TxnKind, parse_amount,
};
pub use nabungo_storage::account::{AccountRow, Opening};
pub use nabungo_storage::account_group::AccountGroupRow;
pub use nabungo_storage::category::CategoryRow;
pub use nabungo_storage::txn::PostingRecord;
pub use nabungo_storage::txn_list::{TxnListRow, TxnPage, TxnQuery};
pub use onboarding::OnboardingInput;
pub use payee::PayeeSuggestion;
pub use settings::{Locale, Theme, UiSettings};
pub use time::Date;
pub use transactions::{TxnDetail, TxnForm};
pub use uuid::Uuid;
pub use valuation::ValuationPoint;
