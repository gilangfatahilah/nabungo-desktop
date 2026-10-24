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
mod attachments;
mod backup;
mod budget;
mod budget_alerts;
mod categories;
mod defaults;
mod export;
pub mod fixture;
mod ledger;
mod onboarding;
mod payee;
mod reconcile;
mod recurring;
mod reports;
pub mod security;
mod settings;
mod tags;
#[cfg(test)]
mod testkit;
mod transactions;
mod valuation;
pub mod vault;

pub use accounts::{AccountFields, AccountsOverview};
pub use allocation::{AllocationOverview, AllocationPoint, BucketFields};
pub use attachments::{AttachmentRow, MAX_ATTACHMENT_BYTES};
pub use backup::{
    BackupInfo, BackupKind, backup_needs_key, is_auto_due, list_backups, parse_backup_name,
};
pub use budget::{BudgetOverview, BudgetRow};
pub use budget_alerts::BudgetAlert;
pub use categories::CategoryFields;
pub use ledger::{AppError, Ledger, PostingInput, TxnInput};
pub use nabungo_core::Mapping;
pub use nabungo_core::date::{format_date, parse_date};
pub use nabungo_core::recurrence::{Freq, MAX_INTERVAL, Recurrence, RecurrenceError};
pub use nabungo_core::{
    AccountKind, AllocationError, AllocationReport, AmountParseError, AmountWarning, BucketActual,
    BudgetError, BudgetLevel, BudgetPeriod, BudgetTotals, CatalogError, CategoryKind, Fee,
    FormBody, MoneyError, NetWorth, PeriodError, PeriodStartDay, SiblingWarning, SplitLine, Target,
    TxnError, TxnKind, Unallocated, parse_amount,
};
pub use nabungo_storage::DbKey;
pub use nabungo_storage::account::{AccountRow, Opening};
pub use nabungo_storage::account_group::AccountGroupRow;
pub use nabungo_storage::allocation::BucketRow;
pub use nabungo_storage::category::CategoryRow;
pub use nabungo_storage::contact::ContactRow;
pub use nabungo_storage::format_utc as format_timestamp;
pub use nabungo_storage::payee::PayeeUsage;
pub use nabungo_storage::tag::TagUsage;
pub use nabungo_storage::txn::PostingRecord;
pub use nabungo_storage::txn_list::{TxnListRow, TxnPage, TxnQuery, TxnSort, TxnSortField};
pub use onboarding::OnboardingInput;
pub use payee::PayeeSuggestion;
pub use reconcile::Reconciliation;
pub use recurring::{MAX_CATCH_UP, RuleInput, RuleRow};
pub use reports::{CashflowPoint, CategoryAmount, MAX_PERIODS, NetWorthPoint};
pub use settings::{AUTO_LOCK_CHOICES, Locale, Theme, UiSettings};
pub use time::{Date, OffsetDateTime};
pub use transactions::{TxnDetail, TxnForm};
pub use uuid::Uuid;
pub use valuation::ValuationPoint;
