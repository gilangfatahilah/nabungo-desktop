//! State yang di-manage Tauri.

use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use nabungo_app::email_runner::SyncSchedule;
use nabungo_app::{Ledger, UtcOffset};

use crate::error::{ErrorCode, IpcError};
use crate::lifecycle::{RecoveryState, StartupReport};
use crate::paths::DataDir;

pub struct AppState {
    /// `None` selama DB terenkripsi belum dibuka (terkunci).
    pub ledger: Mutex<Option<Ledger>>,
    /// `Some` bila DB gagal dibuka saat startup (P1); ledger `None` selama itu.
    pub recovery: Mutex<Option<RecoveryState>>,
    pub data_dir: DataDir,
    /// Laporan startup; diperbarui setelah unlock dan restore sukses.
    pub startup: Mutex<StartupReport>,
    pub scheduler_started: AtomicBool,
    /// Sesi sebelumnya tidak berakhir normal; integrity check menunggu DB dibuka.
    pub unclean_shutdown: bool,
    /// Awal `run()`; acuan `startup_ms` saat dasbor pertama tampil.
    pub started: Instant,
    pub ready_logged: AtomicBool,
    pub email: EmailRuntime,
}

/// Status sync email selama proses berjalan (tidak disimpan).
pub struct EmailRuntime {
    /// Satu sync pada satu waktu (K9); dilepas lewat `email::RunningGuard`.
    pub running: AtomicBool,
    pub schedule: Mutex<SyncSchedule>,
    /// Error putaran terakhir; `None` setelah sukses atau connect/disconnect.
    pub last_error: Mutex<Option<ErrorCode>>,
    /// Dibaca sekali di awal `run()` (K10).
    pub local_offset: UtcOffset,
    pub thread_started: AtomicBool,
}

impl EmailRuntime {
    pub fn new(local_offset: UtcOffset) -> Self {
        Self {
            running: AtomicBool::new(false),
            schedule: Mutex::new(SyncSchedule::default()),
            last_error: Mutex::new(None),
            local_offset,
            thread_started: AtomicBool::new(false),
        }
    }
}

/// Ledger yang sedang terbuka; hanya dibuat oleh [`lock_ledger`] bila tidak terkunci.
pub struct LedgerGuard<'a>(MutexGuard<'a, Option<Ledger>>);

impl Deref for LedgerGuard<'_> {
    type Target = Ledger;

    fn deref(&self) -> &Ledger {
        self.0.as_ref().expect("LedgerGuard dibuat hanya bila Some")
    }
}

impl DerefMut for LedgerGuard<'_> {
    fn deref_mut(&mut self) -> &mut Ledger {
        self.0.as_mut().expect("LedgerGuard dibuat hanya bila Some")
    }
}

/// Kunci mutex ledger; terkunci (`None`) → `ErrorCode::Locked`.
pub fn lock_ledger(m: &Mutex<Option<Ledger>>) -> Result<LedgerGuard<'_>, IpcError> {
    let guard = lock_slot(m)?;
    if guard.is_none() {
        return Err(IpcError::code(ErrorCode::Locked));
    }
    Ok(LedgerGuard(guard))
}

/// Mutex ledger apa adanya (boleh `None`), untuk lock/unlock dan status.
pub fn lock_slot(m: &Mutex<Option<Ledger>>) -> Result<MutexGuard<'_, Option<Ledger>>, IpcError> {
    m.lock().map_err(|_| {
        tracing::error!("mutex ledger poisoned");
        IpcError::code(ErrorCode::Internal)
    })
}

/// Penahan `WorkerGuard` log selama app berjalan.
// Tidak pernah dibaca: cukup ditahan agar log di-flush saat app berhenti.
#[allow(dead_code)]
pub struct LogGuard(pub tracing_appender::non_blocking::WorkerGuard);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_state_returns_locked_error() {
        let locked: Mutex<Option<Ledger>> = Mutex::new(None);
        assert_eq!(
            lock_ledger(&locked).err().map(|e| e.code),
            Some(ErrorCode::Locked)
        );
        let open = Mutex::new(Some(Ledger::open_in_memory().unwrap()));
        let guard = lock_ledger(&open).unwrap();
        assert!(!guard.is_encrypted());
    }
}
