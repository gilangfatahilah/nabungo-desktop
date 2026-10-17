//! Use case / service layer: TransactionService, BudgetService, AllocationService,
//! ReportService, BackupService. Tidak tahu apa-apa soal Tauri.

/// Versi domain yang dipakai app (smoke test lintas layer).
pub fn version() -> &'static str {
    nabungo_core::version()
}

mod accounts;
mod ledger;
mod settings;

pub use accounts::{AccountFields, AccountsOverview};
pub use ledger::{AppError, Ledger, PostingInput, TxnInput};
pub use nabungo_core::{
    AccountKind, AmountParseError, CatalogError, CategoryKind, MoneyError, NetWorth, TxnError,
    parse_amount,
};
pub use nabungo_storage::account::{AccountRow, Opening};
pub use nabungo_storage::account_group::AccountGroupRow;
pub use settings::{Locale, Theme, UiSettings};
