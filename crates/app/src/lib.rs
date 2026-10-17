//! Use case / service layer: TransactionService, BudgetService, AllocationService,
//! ReportService, BackupService. Tidak tahu apa-apa soal Tauri.

/// Versi domain yang dipakai app (smoke test lintas layer).
pub fn version() -> &'static str {
    nabungo_core::version()
}

mod accounts;
mod categories;
mod defaults;
mod ledger;
mod onboarding;
mod payee;
mod settings;

pub use accounts::{AccountFields, AccountsOverview};
pub use categories::CategoryFields;
pub use ledger::{AppError, Ledger, PostingInput, TxnInput};
pub use nabungo_core::{
    AccountKind, AmountParseError, CatalogError, CategoryKind, MoneyError, NetWorth, PeriodError,
    PeriodStartDay, TxnError, parse_amount,
};
pub use nabungo_storage::account::{AccountRow, Opening};
pub use nabungo_storage::account_group::AccountGroupRow;
pub use nabungo_storage::category::CategoryRow;
pub use onboarding::OnboardingInput;
pub use payee::PayeeSuggestion;
pub use settings::{Locale, Theme, UiSettings};
