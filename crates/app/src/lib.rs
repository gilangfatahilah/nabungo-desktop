//! Use case / service layer: TransactionService, BudgetService, AllocationService,
//! ReportService, BackupService. Tidak tahu apa-apa soal Tauri.

/// Versi domain yang dipakai app (smoke test lintas layer).
pub fn version() -> &'static str {
    nabungo_core::version()
}

/// Tanggal lokal hari ini; jatuh ke UTC bila offset lokal tak bisa dibaca.
pub fn local_today() -> Date {
    OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .date()
}

mod accounts;
mod allocation;
mod backup;
mod budget;
mod budget_alerts;
mod categories;
mod defaults;
mod export;
mod ledger;
mod onboarding;
mod payee;
mod reports;
pub mod security;
mod settings;
#[cfg(test)]
mod testkit;
mod transactions;
mod valuation;
pub mod vault;

pub use accounts::{AccountFields, AccountsOverview};
pub use allocation::{AllocationOverview, AllocationPoint, BucketFields};
pub use backup::{BackupInfo, BackupKind, is_auto_due, list_backups, parse_backup_name};
pub use budget::{BudgetOverview, BudgetRow};
pub use budget_alerts::BudgetAlert;
pub use categories::CategoryFields;
pub use ledger::{AppError, Ledger, PostingInput, TxnInput};
pub use nabungo_core::Mapping;
pub use nabungo_core::date::{format_date, parse_date};
pub use nabungo_core::{
    AccountKind, AllocationError, AllocationReport, AmountParseError, BucketActual, BudgetError,
    BudgetLevel, BudgetPeriod, BudgetTotals, CatalogError, CategoryKind, Fee, FormBody, MoneyError,
    NetWorth, PeriodError, PeriodStartDay, SiblingWarning, SplitLine, TxnError, TxnKind,
    Unallocated, parse_amount,
};
pub use nabungo_storage::DbKey;
pub use nabungo_storage::account::{AccountRow, Opening};
pub use nabungo_storage::account_group::AccountGroupRow;
pub use nabungo_storage::allocation::BucketRow;
pub use nabungo_storage::category::CategoryRow;
pub use nabungo_storage::contact::ContactRow;
pub use nabungo_storage::format_utc as format_timestamp;
pub use nabungo_storage::txn::PostingRecord;
pub use nabungo_storage::txn_list::{TxnListRow, TxnPage, TxnQuery};
pub use onboarding::OnboardingInput;
pub use payee::PayeeSuggestion;
pub use reports::{CashflowPoint, CategoryAmount, MAX_PERIODS, NetWorthPoint};
pub use settings::{AUTO_LOCK_CHOICES, Locale, Theme, UiSettings};
pub use time::{Date, OffsetDateTime};
pub use transactions::{TxnDetail, TxnForm};
pub use uuid::Uuid;
pub use valuation::ValuationPoint;
