//! Use case / service layer: TransactionService, BudgetService, AllocationService,
//! ReportService, BackupService. Tidak tahu apa-apa soal Tauri.

/// Versi domain yang dipakai app (smoke test lintas layer).
pub fn version() -> &'static str {
    nabungo_core::version()
}

mod ledger;
mod settings;

pub use ledger::{AppError, Ledger, PostingInput, TxnInput};
pub use nabungo_core::{AmountParseError, TxnError, parse_amount};
pub use settings::{Locale, Theme, UiSettings};
