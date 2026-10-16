//! State yang di-manage Tauri.

use std::sync::Mutex;

use nabungo_app::Ledger;

use crate::paths::DataDir;

// Dibaca oleh command di Task 4.
#[allow(dead_code)]
pub struct AppState {
    pub ledger: Mutex<Ledger>,
    pub data_dir: DataDir,
}

/// Penahan `WorkerGuard` log selama app berjalan.
#[allow(dead_code)]
pub struct LogGuard(pub tracing_appender::non_blocking::WorkerGuard);
