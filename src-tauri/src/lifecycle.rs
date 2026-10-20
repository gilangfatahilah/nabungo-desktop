//! Siklus hidup sesi: penanda shutdown tidak normal, laporan startup, backup terjadwal.

use std::path::Path;
use std::time::Duration;

use nabungo_app::OffsetDateTime;
use tauri::{AppHandle, Manager};

use crate::state::AppState;

/// Jeda antar-pemeriksaan backup otomatis selama app berjalan.
const SCHEDULER_TICK: Duration = Duration::from_secs(60 * 60);

/// File penanda sesi: ada saat startup = sesi sebelumnya tidak berakhir normal.
pub struct SessionMarker;

impl SessionMarker {
    /// Tulis penanda; `true` bila penanda sesi sebelumnya masih ada.
    pub fn acquire(path: &Path) -> std::io::Result<bool> {
        let existed = path.exists();
        std::fs::write(path, std::process::id().to_string())?;
        Ok(existed)
    }

    pub fn release(path: &Path) {
        if let Err(e) = std::fs::remove_file(path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(error = %e, "gagal menghapus penanda sesi");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityStatus {
    /// Sesi sebelumnya normal; pemeriksaan penuh tidak dijalankan.
    Skipped,
    Ok,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartupReport {
    pub unclean_shutdown: bool,
    pub integrity: IntegrityStatus,
}

impl StartupReport {
    /// DB baru saja diganti backup yang lolos `quick_check`: status gagal tidak berlaku lagi.
    pub fn after_restore(self) -> Self {
        if self.integrity == IntegrityStatus::Failed {
            Self {
                integrity: IntegrityStatus::Ok,
                ..self
            }
        } else {
            self
        }
    }
}

/// DB yang gagal integrity check tidak di-backup otomatis: jangan sampai backup sehat
/// tergeser retensi oleh salinan yang rusak.
pub fn auto_backup_allowed(r: &StartupReport) -> bool {
    r.integrity != IntegrityStatus::Failed
}

/// `integrity_check` penuh; pesan masalah dicatat ke log.
pub fn check_integrity(ledger: &nabungo_app::Ledger) -> IntegrityStatus {
    match ledger.integrity_check() {
        Ok(problems) if problems.is_empty() => IntegrityStatus::Ok,
        Ok(problems) => {
            tracing::error!(?problems, "integrity check gagal");
            IntegrityStatus::Failed
        }
        Err(e) => {
            tracing::error!(error = %e, "integrity check tidak bisa dijalankan");
            IntegrityStatus::Failed
        }
    }
}

/// Satu putaran backup otomatis; kesalahan hanya dicatat.
pub fn run_auto_backup(ledger: &nabungo_app::Ledger, dir: &Path) {
    match ledger.auto_backup_if_due(dir, OffsetDateTime::now_utc()) {
        Ok(Some(b)) => tracing::info!(file = %b.file_name, "backup otomatis dibuat"),
        Ok(None) => {}
        Err(e) => tracing::warn!(error = %e, "backup otomatis gagal"),
    }
}

/// Setelah restore sukses: perbarui laporan startup dan hidupkan backup otomatis bila tadi
/// dimatikan karena integrity check gagal.
pub fn on_restored(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut startup = state.startup.lock().unwrap_or_else(|e| e.into_inner());
    *startup = startup.after_restore();
    if auto_backup_allowed(&startup) {
        spawn_backup_scheduler(app.clone());
    }
}

/// Thread latar: tiap jam cek apakah backup otomatis jatuh tempo. Hanya satu per proses.
pub fn spawn_backup_scheduler(app: AppHandle) {
    if app
        .state::<AppState>()
        .scheduler_started
        .swap(true, std::sync::atomic::Ordering::SeqCst)
    {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("backup-scheduler".into())
        .spawn(move || {
            loop {
                std::thread::sleep(SCHEDULER_TICK);
                let state = app.state::<AppState>();
                let dir = state.data_dir.backups_path();
                match state.ledger.lock() {
                    Ok(ledger) => run_auto_backup(&ledger, &dir),
                    Err(_) => tracing::warn!("mutex ledger poisoned; backup otomatis dilewati"),
                };
            }
        });
    if let Err(e) = spawned {
        tracing::warn!(error = %e, "gagal menjalankan penjadwal backup");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_clears_failed_integrity() {
        let failed = StartupReport {
            unclean_shutdown: true,
            integrity: IntegrityStatus::Failed,
        };
        let r = failed.after_restore();
        assert_eq!(r.integrity, IntegrityStatus::Ok);
        assert!(r.unclean_shutdown);
        assert!(auto_backup_allowed(&r));
        let skipped = StartupReport {
            unclean_shutdown: false,
            integrity: IntegrityStatus::Skipped,
        };
        assert_eq!(skipped.after_restore(), skipped);
    }

    #[test]
    fn marker_detects_unclean_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let m = dir.path().join("session.lock");
        assert!(!SessionMarker::acquire(&m).unwrap());
        assert!(SessionMarker::acquire(&m).unwrap());
        SessionMarker::release(&m);
        assert!(!m.exists());
        assert!(!SessionMarker::acquire(&m).unwrap());
    }

    #[test]
    fn auto_backup_not_allowed_after_failed_integrity() {
        let r = |integrity| StartupReport {
            unclean_shutdown: true,
            integrity,
        };
        assert!(!auto_backup_allowed(&r(IntegrityStatus::Failed)));
        assert!(auto_backup_allowed(&r(IntegrityStatus::Ok)));
        assert!(auto_backup_allowed(&r(IntegrityStatus::Skipped)));
    }
}
