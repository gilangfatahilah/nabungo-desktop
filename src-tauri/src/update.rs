//! Update aplikasi dari Pengaturan (opt-in): cek rilis, unduh installer terverifikasi, lalu
//! pasang sesuai format instalasi. Logika jaringan/verifikasi ada di `nabungo_app::update`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use nabungo_app::update::{self, InstallKind, UpdateCheck};
use tauri::utils::config::BundleType;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use tauri_specta::Event;

use crate::dto::{
    InstallOutcomeDto, UpdateAvailable, UpdateDownloadProgress, UpdateInfoDto, UpdateStatusDto,
};
use crate::error::{ErrorCode, IpcError};
use crate::lifecycle::{FIRST_TICK_MAX_WAIT, READY_POLL, wait_until_ready};
use crate::state::AppState;

/// Status update selama proses berjalan (tidak disimpan).
#[derive(Default)]
pub struct UpdateRuntime {
    /// Hasil cek terakhir; unduhan hanya memakai installer dari sini, bukan URL dari UI.
    pub last_check: Mutex<Option<UpdateCheck>>,
    /// Installer yang sudah terunduh dan terverifikasi.
    pub downloaded: Mutex<Option<PathBuf>>,
    pub downloading: AtomicBool,
    pub cancel: AtomicBool,
    /// Cek otomatis paling banyak sekali per proses.
    pub auto_checked: AtomicBool,
}

/// Format instalasi yang sedang berjalan; `None` untuk `pnpm dev`, binary mentah, dan mode
/// portable (installer akan memasang di lokasi lain).
pub fn install_kind(portable: bool) -> Option<InstallKind> {
    if portable {
        return None;
    }
    match tauri::utils::platform::bundle_type()? {
        BundleType::Deb => Some(InstallKind::Deb),
        BundleType::Rpm => Some(InstallKind::Rpm),
        BundleType::AppImage => Some(InstallKind::AppImage),
        BundleType::Nsis => Some(InstallKind::Nsis),
        BundleType::Msi => Some(InstallKind::Msi),
        _ => None,
    }
}

fn kind(app: &AppHandle) -> Option<InstallKind> {
    install_kind(app.state::<AppState>().data_dir.portable)
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Memegang flag `downloading`; dilepas saat drop.
struct DownloadGuard<'a>(&'a AtomicBool);

impl<'a> DownloadGuard<'a> {
    fn acquire(flag: &'a AtomicBool) -> Result<Self, IpcError> {
        flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| Self(flag))
            .map_err(|_| IpcError::code(ErrorCode::UpdateBusy))
    }
}

impl Drop for DownloadGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

pub fn status(app: &AppHandle) -> Result<UpdateStatusDto, IpcError> {
    let state = app.state::<AppState>();
    let auto_check = crate::state::lock_ledger(&state.ledger)?.update_auto_check()?;
    let rt = &state.update;
    Ok(UpdateStatusDto {
        current: nabungo_app::version().to_owned(),
        auto_check,
        install_kind: kind(app).map(Into::into),
        last_check: lock(&rt.last_check).as_ref().map(UpdateInfoDto::new),
        downloaded: lock(&rt.downloaded).is_some(),
        downloading: rt.downloading.load(Ordering::SeqCst),
    })
}

pub fn check(app: &AppHandle) -> Result<UpdateInfoDto, IpcError> {
    let kind = kind(app);
    let result = update::check_update(nabungo_app::version(), kind)?;
    let rt = &app.state::<AppState>().update;
    let dto = UpdateInfoDto::new(&result);
    *lock(&rt.downloaded) = None;
    *lock(&rt.last_check) = Some(result);
    Ok(dto)
}

/// Folder tujuan unduhan. AppImage: folder file AppImage yang berjalan (agar bisa diganti
/// dengan rename atomik); selain itu folder Downloads, atau `cache/update` di folder data.
fn download_dir(app: &AppHandle, kind: Option<InstallKind>) -> PathBuf {
    if kind == Some(InstallKind::AppImage)
        && let Some(dir) = running_appimage().and_then(|p| p.parent().map(Path::to_path_buf))
        && writable(&dir)
    {
        return dir;
    }
    directories::UserDirs::new()
        .and_then(|u| u.download_dir().map(Path::to_path_buf))
        .unwrap_or_else(|| {
            app.state::<AppState>()
                .data_dir
                .path
                .join("cache")
                .join("update")
        })
}

fn running_appimage() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE").map(PathBuf::from)
}

fn writable(dir: &Path) -> bool {
    let probe = dir.join(".nabungo-write-test");
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

pub fn download(app: &AppHandle) -> Result<(), IpcError> {
    let state = app.state::<AppState>();
    let rt = &state.update;
    let _guard = DownloadGuard::acquire(&rt.downloading)?;
    let check = lock(&rt.last_check)
        .clone()
        .ok_or(IpcError::code(ErrorCode::UpdateNoAsset))?;
    rt.cancel.store(false, Ordering::SeqCst);
    let dir = download_dir(app, kind(app));
    let mut last_pct = u64::MAX;
    let mut on_progress = |downloaded: u64, total: u64| {
        // Event per persen, bukan per chunk.
        let pct = downloaded
            .saturating_mul(100)
            .checked_div(total)
            .unwrap_or(0);
        if pct != last_pct || downloaded == total {
            last_pct = pct;
            emit(app, UpdateDownloadProgress { downloaded, total });
        }
    };
    let path = update::download_update(&check, &dir, &mut on_progress, &rt.cancel)?;
    *lock(&rt.downloaded) = Some(path);
    Ok(())
}

pub fn cancel(app: &AppHandle) {
    app.state::<AppState>()
        .update
        .cancel
        .store(true, Ordering::SeqCst);
}

pub fn install(app: &AppHandle) -> Result<InstallOutcomeDto, IpcError> {
    let path = lock(&app.state::<AppState>().update.downloaded)
        .clone()
        .ok_or(IpcError::code(ErrorCode::UpdateNoAsset))?;
    match kind(app) {
        Some(InstallKind::AppImage) => install_appimage(app, &path),
        Some(InstallKind::Nsis | InstallKind::Msi) => {
            open_path(app, &path)?;
            tracing::info!("installer Windows dibuka; aplikasi ditutup");
            // Installer mengganti file aplikasi: tutup lewat jalur exit normal (session lock
            // dilepas di `RunEvent::Exit`).
            app.exit(0);
            Ok(InstallOutcomeDto::Exiting)
        }
        Some(InstallKind::Deb | InstallKind::Rpm) => {
            open_path(app, &path)?;
            Ok(InstallOutcomeDto::InstallerOpened)
        }
        None => Err(IpcError::code(ErrorCode::UpdateNoAsset)),
    }
}

/// AppImage baru menggantikan file yang sedang berjalan (rename atomik; Linux tetap menjalankan
/// inode lama), lalu app dimulai ulang. Unduhan di luar folder AppImage → tampilkan di file
/// manager.
fn install_appimage(app: &AppHandle, path: &Path) -> Result<InstallOutcomeDto, IpcError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(io_error)?;
    }
    let target = running_appimage().filter(|t| t.parent() == path.parent());
    let Some(target) = target else {
        app.opener().reveal_item_in_dir(path).map_err(|e| {
            tracing::error!(error = %e, "gagal menampilkan installer");
            IpcError::code(ErrorCode::FileIo)
        })?;
        return Ok(InstallOutcomeDto::Revealed);
    };
    std::fs::rename(path, &target).map_err(io_error)?;
    tracing::info!(path = %target.display(), "AppImage diganti; memulai ulang");
    app.restart()
}

fn io_error(e: std::io::Error) -> IpcError {
    IpcError::from(nabungo_app::AppError::Io(e))
}

fn open_path(app: &AppHandle, path: &Path) -> Result<(), IpcError> {
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| {
            tracing::error!(error = %e, "gagal membuka installer");
            IpcError::code(ErrorCode::FileIo)
        })
}

pub fn open_release_page(app: &AppHandle) -> Result<(), IpcError> {
    let url = lock(&app.state::<AppState>().update.last_check)
        .as_ref()
        .map(|c| c.html_url.clone())
        .filter(|u| update::trusted_release_page(u))
        .unwrap_or_else(|| format!("https://github.com/{}/releases/latest", update::UPDATE_REPO));
    app.opener().open_url(url, None::<&str>).map_err(|e| {
        tracing::error!(error = %e, "gagal membuka halaman rilis");
        IpcError::code(ErrorCode::FileIo)
    })
}

fn emit<E: Event + serde::Serialize + Clone>(app: &AppHandle, event: E) {
    if let Err(e) = event.emit(app) {
        tracing::warn!(error = %e, "gagal mengirim event update");
    }
}

/// DB baru terbuka: cek otomatis sekali per proses bila `update.auto_check` aktif. Setting mati
/// → tidak ada koneksi sama sekali (firewall test).
pub fn on_ledger_opened(app: &AppHandle) {
    let state = app.state::<AppState>();
    let enabled = lock(&state.ledger)
        .as_ref()
        .is_some_and(|l| l.update_auto_check().unwrap_or(false));
    if !enabled || state.update.auto_checked.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("update-check".into())
        .spawn(move || {
            wait_until_ready(
                &app.state::<AppState>().ready_logged,
                FIRST_TICK_MAX_WAIT,
                READY_POLL,
            );
            match check(&app) {
                Ok(info) if info.available => emit(
                    &app,
                    UpdateAvailable {
                        latest: info.latest,
                    },
                ),
                Ok(_) => {}
                Err(e) => tracing::warn!(code = ?e.code, "cek update otomatis gagal"),
            }
        });
    if let Err(e) = spawned {
        tracing::warn!(error = %e, "gagal menjalankan cek update");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_and_unbundled_have_no_install_kind() {
        assert_eq!(install_kind(true), None);
        // Binary test tidak di-patch bundler.
        assert_eq!(install_kind(false), None);
    }

    #[test]
    fn download_guard_rejects_second_and_releases_on_drop() {
        let flag = AtomicBool::new(false);
        let g = DownloadGuard::acquire(&flag).unwrap();
        assert_eq!(
            DownloadGuard::acquire(&flag).err().map(|e| e.code),
            Some(ErrorCode::UpdateBusy)
        );
        drop(g);
        assert!(DownloadGuard::acquire(&flag).is_ok());
    }
}
