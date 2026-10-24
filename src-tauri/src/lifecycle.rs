//! Siklus hidup sesi: penanda shutdown tidak normal, laporan startup, backup terjadwal.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use nabungo_app::{AppError, DbKey, Ledger, OffsetDateTime};
use tauri::{AppHandle, Manager};

use crate::state::AppState;

/// Jeda antar-pemeriksaan backup otomatis selama app berjalan.
const SCHEDULER_TICK: Duration = Duration::from_secs(60 * 60);
/// Batas tunggu putaran pertama bila dasbor tidak pernah melapor siap (mis. onboarding).
const FIRST_TICK_MAX_WAIT: Duration = Duration::from_secs(30);
const READY_POLL: Duration = Duration::from_millis(200);

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

/// Backup otomatis untuk sesi ini: `Immediate` = scheduler latar menjalankan putaran
/// pertama setelah dasbor tampil, sehingga backup jatuh tempo tidak menahan startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupPlan {
    Disabled,
    Immediate,
}

pub fn startup_backup_plan(r: &StartupReport) -> BackupPlan {
    if auto_backup_allowed(r) {
        BackupPlan::Immediate
    } else {
        BackupPlan::Disabled
    }
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

/// Catat kejadian terjadwal ber-auto-post yang jatuh tempo; kegagalan per jadwal hanya dicatat.
pub fn run_auto_post(ledger: &mut Ledger) {
    let n = ledger.post_auto_due(nabungo_app::local_today());
    if n > 0 {
        tracing::info!(count = n, "transaksi terjadwal dicatat otomatis");
    }
}

/// Alasan app masuk mode pemulihan (P1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryReason {
    Unreadable,
    TooNew,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryState {
    pub reason: RecoveryReason,
}

/// Hasil membuka DB saat startup → mode pemulihan (`None` = berhasil).
pub fn startup_mode(r: Result<(), AppError>) -> Option<RecoveryState> {
    let reason = match r.err()? {
        AppError::DatabaseTooNew => RecoveryReason::TooNew,
        _ => RecoveryReason::Unreadable,
    };
    Some(RecoveryState { reason })
}

/// Keluar dari mode pemulihan setelah DB diganti: buka DB baru, lalu perlakukan seperti restore.
pub fn leave_recovery(app: &AppHandle, key: Option<DbKey>) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    *state.recovery.lock().unwrap_or_else(|e| e.into_inner()) = None;
    if let Err(e) = open_unlocked(app, key) {
        // DB pengganti pun gagal dibuka: tetap di pemulihan agar user bisa mencoba lagi.
        let reason = if matches!(e, AppError::DatabaseTooNew) {
            RecoveryReason::TooNew
        } else {
            RecoveryReason::Unreadable
        };
        *state.recovery.lock().unwrap_or_else(|e| e.into_inner()) = Some(RecoveryState { reason });
        return Err(e);
    }
    on_restored(app);
    Ok(())
}

/// Buka DB (plaintext: `key = None`) dan pasang ke state. Integrity check penuh
/// dijalankan sekali per proses bila sesi sebelumnya tidak berakhir normal — untuk DB
/// terenkripsi baru bisa saat unlock pertama.
pub fn open_unlocked(app: &AppHandle, key: Option<DbKey>) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    let backups = state.data_dir.backups_path();
    let mut ledger = Ledger::open_with_backups(
        &state.data_dir.db_path(),
        &backups,
        key.as_ref(),
        OffsetDateTime::now_utc(),
    )?;
    let startup = {
        let mut startup = state.startup.lock().unwrap_or_else(|e| e.into_inner());
        if state.unclean_shutdown && startup.integrity == IntegrityStatus::Skipped {
            startup.integrity = check_integrity(&ledger);
        }
        *startup
    };
    let plan = startup_backup_plan(&startup);
    match plan {
        // Auto-post tetap sinkron: hasilnya harus sudah tampil di dasbor pertama.
        BackupPlan::Immediate => run_auto_post(&mut ledger),
        BackupPlan::Disabled => tracing::warn!("backup otomatis dimatikan untuk sesi ini"),
    }
    *state.ledger.lock().unwrap_or_else(|e| e.into_inner()) = Some(ledger);
    if plan == BackupPlan::Immediate {
        spawn_backup_scheduler(app.clone());
    }
    Ok(())
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

/// Tunggu sampai `ready` (dasbor pertama tampil) atau paling lama `max`.
fn wait_until_ready(ready: &AtomicBool, max: Duration, poll: Duration) {
    let start = std::time::Instant::now();
    while !ready.load(Ordering::SeqCst) && start.elapsed() < max {
        std::thread::sleep(poll);
    }
}

/// Thread latar: cek backup otomatis jatuh tempo setelah dasbor pertama tampil (backup
/// memegang mutex ledger, ±1 s untuk DB terenkripsi 50.000 transaksi), lalu tiap jam.
/// Hanya satu per proses.
pub fn spawn_backup_scheduler(app: AppHandle) {
    if app
        .state::<AppState>()
        .scheduler_started
        .swap(true, Ordering::SeqCst)
    {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("backup-scheduler".into())
        .spawn(move || {
            wait_until_ready(
                &app.state::<AppState>().ready_logged,
                FIRST_TICK_MAX_WAIT,
                READY_POLL,
            );
            loop {
                let state = app.state::<AppState>();
                let dir = state.data_dir.backups_path();
                match state.ledger.lock() {
                    Ok(mut slot) => {
                        // Terkunci: tidak ada yang bisa di-backup sampai unlock.
                        if let Some(ledger) = slot.as_mut() {
                            run_auto_backup(ledger, &dir);
                            run_auto_post(ledger);
                        }
                    }
                    Err(_) => tracing::warn!("mutex ledger poisoned; backup otomatis dilewati"),
                };
                std::thread::sleep(SCHEDULER_TICK);
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
    fn startup_mode_maps_errors() {
        assert!(startup_mode(Ok(())).is_none());
        assert_eq!(
            startup_mode(Err(AppError::DatabaseTooNew)).map(|r| r.reason),
            Some(RecoveryReason::TooNew)
        );
        assert_eq!(
            startup_mode(Err(AppError::DatabaseUnreadable)).map(|r| r.reason),
            Some(RecoveryReason::Unreadable)
        );
        assert_eq!(
            startup_mode(Err(AppError::Io(std::io::Error::other("disk")))).map(|r| r.reason),
            Some(RecoveryReason::Unreadable)
        );
    }

    #[test]
    fn first_tick_waits_until_ready() {
        let ready = std::sync::Arc::new(AtomicBool::new(false));
        let flag = ready.clone();
        let setter = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            flag.store(true, Ordering::SeqCst);
        });
        let t = std::time::Instant::now();
        wait_until_ready(&ready, Duration::from_secs(5), Duration::from_millis(5));
        let waited = t.elapsed();
        setter.join().unwrap();
        assert!(waited >= Duration::from_millis(50), "{waited:?}");
        assert!(waited < Duration::from_secs(1), "{waited:?}");
    }

    #[test]
    fn first_tick_gives_up_after_max() {
        let ready = AtomicBool::new(false);
        let t = std::time::Instant::now();
        wait_until_ready(&ready, Duration::from_millis(60), Duration::from_millis(5));
        assert!(t.elapsed() >= Duration::from_millis(60));
    }

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

    #[test]
    fn first_tick_respects_integrity() {
        let r = |integrity| StartupReport {
            unclean_shutdown: true,
            integrity,
        };
        assert_eq!(
            startup_backup_plan(&r(IntegrityStatus::Failed)),
            BackupPlan::Disabled
        );
        assert_eq!(
            startup_backup_plan(&r(IntegrityStatus::Ok)),
            BackupPlan::Immediate
        );
        assert_eq!(
            startup_backup_plan(&r(IntegrityStatus::Skipped)),
            BackupPlan::Immediate
        );
    }
}
