//! State yang di-manage Tauri.

use std::sync::Mutex;

use nabungo_app::Ledger;

use crate::lifecycle::StartupReport;
use crate::paths::DataDir;

pub struct AppState {
    pub ledger: Mutex<Ledger>,
    pub data_dir: DataDir,
    pub startup: StartupReport,
}

/// Penahan `WorkerGuard` log selama app berjalan.
// Tidak pernah dibaca: cukup ditahan agar log di-flush saat app berhenti.
#[allow(dead_code)]
pub struct LogGuard(pub tracing_appender::non_blocking::WorkerGuard);
