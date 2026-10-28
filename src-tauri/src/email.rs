//! Email capture di desktop (EML-01): sync manual/terjadwal di thread `email-sync`, event
//! kemajuan ke UI. Logika ada di `nabungo_app::email_runner`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use nabungo_app::email_runner::{self, Connect};
use nabungo_app::mail::ImapSource;
use nabungo_app::secret::{KeyringStore, SecretStore};
use nabungo_app::{AppError, MailSource, SourceError, SyncError, SyncSummary};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::dto::{EmailSyncDone, EmailSyncProgress};
use crate::error::{ErrorCode, IpcError};
use crate::lifecycle::{FIRST_TICK_MAX_WAIT, READY_POLL, wait_until_ready};
use crate::state::{AppState, EmailRuntime};

const IMAP_HOST: &str = "imap.gmail.com";
const IMAP_PORT: u16 = 993;
/// Jeda antar-pemeriksaan jadwal sync (K7).
const TICK: Duration = Duration::from_secs(30);

fn imap_connect(address: &str, password: &str) -> Result<Box<dyn MailSource>, SourceError> {
    Ok(Box::new(ImapSource::connect(
        IMAP_HOST, IMAP_PORT, address, password,
    )?))
}

const CONNECT: Connect = &imap_connect;

/// Keychain OS yang baru dipasang saat pertama dipakai, agar D-Bus tidak disentuh selama email
/// belum dikonfigurasi (K7).
struct LazyKeyring;

impl SecretStore for LazyKeyring {
    fn get(&self, account: &str) -> Result<Option<String>, AppError> {
        KeyringStore::new()?.get(account)
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), AppError> {
        KeyringStore::new()?.set(account, secret)
    }

    fn delete(&self, account: &str) -> Result<(), AppError> {
        KeyringStore::new()?.delete(account)
    }
}

/// Memegang flag `running`; dilepas saat drop, termasuk saat panic (K9).
struct RunningGuard<'a>(&'a AtomicBool);

impl<'a> RunningGuard<'a> {
    fn acquire(flag: &'a AtomicBool) -> Result<Self, IpcError> {
        flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| Self(flag))
            .map_err(|_| IpcError::code(ErrorCode::EmailSyncRunning))
    }
}

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn emit<E: Event + serde::Serialize + Clone>(app: &AppHandle, event: E) {
    if let Err(e) = event.emit(app) {
        tracing::warn!(error = %e, "gagal mengirim event sync email");
    }
}

/// Satu putaran sync. Ledger hanya dikunci per email, bukan selama I/O jaringan.
pub fn run_sync(app: &AppHandle) -> Result<SyncSummary, IpcError> {
    let state = app.state::<AppState>();
    let rt = &state.email;
    let _running = RunningGuard::acquire(&rt.running)?;
    let started = rt.generation.load(Ordering::SeqCst);
    let result = email_runner::run_email_sync(
        &state.ledger,
        &LazyKeyring,
        CONNECT,
        rt.local_offset,
        &mut |p| emit(app, EmailSyncProgress::from(p)),
    );
    let result = record_outcome(rt, started, Instant::now(), result);
    match &result {
        Ok(summary) => tracing::info!(
            new_items = summary.new_items,
            unparsed = summary.unparsed,
            "sync email selesai"
        ),
        Err(e) => tracing::warn!(code = ?e.code, "sync email gagal"),
    }
    emit(
        app,
        EmailSyncDone {
            summary: result.as_ref().ok().map(|s| (*s).into()),
            error: result.as_ref().err().cloned(),
        },
    );
    result
}

/// Mencatat hasil ke jadwal dan `last_error`, kecuali akun diganti/diputus selama sync
/// berjalan (`started` ≠ generasi sekarang).
fn record_outcome(
    rt: &EmailRuntime,
    started: u64,
    now: Instant,
    result: Result<SyncSummary, SyncError>,
) -> Result<SyncSummary, IpcError> {
    if rt.generation.load(Ordering::SeqCst) != started {
        return result.map_err(IpcError::from);
    }
    rt.schedule
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .record(now, &result);
    // Terkunci di tengah sync bukan kegagalan sync; status sebelumnya dipertahankan.
    let locked = matches!(result, Err(SyncError::Locked));
    let result = result.map_err(IpcError::from);
    if !locked {
        *rt.last_error.lock().unwrap_or_else(|e| e.into_inner()) =
            result.as_ref().err().map(|e| e.code);
    }
    result
}

/// Verifikasi + simpan akun; sync berikutnya segera dijalankan thread.
pub fn connect(
    app: &AppHandle,
    address: &str,
    mailbox: &str,
    password: Option<&str>,
) -> Result<(), IpcError> {
    let state = app.state::<AppState>();
    email_runner::connect_email(
        &state.ledger,
        &LazyKeyring,
        CONNECT,
        address,
        mailbox,
        password,
    )?;
    reset_status(&state);
    tracing::info!("akun email dihubungkan");
    Ok(())
}

pub fn disconnect(app: &AppHandle) -> Result<(), IpcError> {
    let state = app.state::<AppState>();
    email_runner::disconnect_email(&state.ledger, &LazyKeyring)?;
    reset_status(&state);
    tracing::info!("akun email diputus");
    Ok(())
}

fn reset_status(state: &AppState) {
    account_changed(&state.email);
}

fn account_changed(rt: &EmailRuntime) {
    rt.generation.fetch_add(1, Ordering::SeqCst);
    rt.schedule
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .resume();
    *rt.last_error.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// DB baru terbuka (startup, unlock, restore): putaran segera, thread dijalankan sekali.
pub fn on_ledger_opened(app: &AppHandle) {
    let state = app.state::<AppState>();
    state
        .email
        .schedule
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .reset();
    if !state.email.thread_started.swap(true, Ordering::SeqCst) {
        spawn_email_scheduler(app.clone());
    }
}

/// Thread menjalankan putaran hanya bila email aktif, DB terbuka, bukan mode pemulihan, tidak
/// ada sync lain, dan jadwal jatuh tempo (K7, K9).
fn tick_should_run(
    enabled: bool,
    unlocked: bool,
    recovering: bool,
    running: bool,
    due: bool,
) -> bool {
    enabled && unlocked && !recovering && !running && due
}

fn tick(app: &AppHandle) {
    let state = app.state::<AppState>();
    let recovering = state
        .recovery
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some();
    let (unlocked, enabled) = {
        let slot = state.ledger.lock().unwrap_or_else(|e| e.into_inner());
        match slot.as_ref() {
            Some(l) => (true, l.email_settings().map(|s| s.enabled).unwrap_or(false)),
            None => (false, false),
        }
    };
    let running = state.email.running.load(Ordering::SeqCst);
    let due = state
        .email
        .schedule
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .due(Instant::now());
    if tick_should_run(enabled, unlocked, recovering, running, due) {
        // Hasil sudah dicatat dan dikirim sebagai event oleh `run_sync`.
        let _ = run_sync(app);
    }
}

fn spawn_email_scheduler(app: AppHandle) {
    let spawned = std::thread::Builder::new()
        .name("email-sync".into())
        .spawn(move || {
            wait_until_ready(
                &app.state::<AppState>().ready_logged,
                FIRST_TICK_MAX_WAIT,
                READY_POLL,
            );
            loop {
                tick(&app);
                std::thread::sleep(TICK);
            }
        });
    if let Err(e) = spawned {
        tracing::warn!(error = %e, "gagal menjalankan penjadwal sync email");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_skips_when_locked_recovering_disabled_or_running() {
        assert!(tick_should_run(true, true, false, false, true));
        assert!(!tick_should_run(false, true, false, false, true));
        assert!(!tick_should_run(true, false, false, false, true));
        assert!(!tick_should_run(true, true, true, false, true));
        assert!(!tick_should_run(true, true, false, true, true));
        assert!(!tick_should_run(true, true, false, false, false));
    }

    #[test]
    fn stale_sync_does_not_overwrite_status_after_connect() {
        let rt = EmailRuntime::new(nabungo_app::UtcOffset::UTC);
        let auth = || Err(SyncError::Source(SourceError::Auth));
        let started = rt.generation.load(Ordering::SeqCst);
        account_changed(&rt);
        let _ = record_outcome(&rt, started, Instant::now(), auth());
        assert!(!rt.schedule.lock().unwrap().paused);
        assert_eq!(*rt.last_error.lock().unwrap(), None);

        let current = rt.generation.load(Ordering::SeqCst);
        let _ = record_outcome(&rt, current, Instant::now(), auth());
        assert!(rt.schedule.lock().unwrap().paused);
        assert_eq!(
            *rt.last_error.lock().unwrap(),
            Some(ErrorCode::EmailAuthFailed)
        );
    }

    #[test]
    fn running_guard_rejects_second_and_releases_on_drop() {
        let flag = AtomicBool::new(false);
        let first = RunningGuard::acquire(&flag).unwrap();
        assert_eq!(
            RunningGuard::acquire(&flag).err().map(|e| e.code),
            Some(ErrorCode::EmailSyncRunning)
        );
        drop(first);
        assert!(!flag.load(Ordering::SeqCst));
        RunningGuard::acquire(&flag).unwrap();
    }
}
