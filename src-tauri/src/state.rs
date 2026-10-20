//! State yang di-manage Tauri.

use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use nabungo_app::Ledger;

use crate::lifecycle::StartupReport;
use crate::paths::DataDir;

pub struct AppState {
    pub ledger: Mutex<Ledger>,
    pub data_dir: DataDir,
    /// Laporan startup; diperbarui setelah restore sukses.
    pub startup: Mutex<StartupReport>,
    pub scheduler_started: AtomicBool,
}

/// Penahan `WorkerGuard` log selama app berjalan.
// Tidak pernah dibaca: cukup ditahan agar log di-flush saat app berhenti.
#[allow(dead_code)]
pub struct LogGuard(pub tracing_appender::non_blocking::WorkerGuard);
