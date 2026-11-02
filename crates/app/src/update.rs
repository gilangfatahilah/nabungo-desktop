//! Cek & unduh update dari GitHub Releases (opt-in). Fungsi jaringan tidak memegang `Ledger`;
//! hanya setting `update.auto_check` yang disimpan di DB.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use nabungo_storage::setting;
use nabungo_update::{Client, UpdateError, Version, parse_sha256sums, pick_asset};

pub use nabungo_update::InstallKind;

use crate::{AppError, Ledger};

/// Repo publik sumber rilis.
pub const UPDATE_REPO: &str = "gilangfatahilah/nabungo-desktop";
/// Batas ukuran installer yang mau diunduh (installer < 15 MB, AppImage lebih besar).
pub const MAX_INSTALLER_BYTES: u64 = 200 * 1024 * 1024;

const KEY_AUTO_CHECK: &str = "update.auto_check";
const SUMS_ASSET: &str = "SHA256SUMS";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheck {
    pub current: String,
    pub latest: String,
    /// `latest` lebih baru dari `current`.
    pub available: bool,
    pub notes: String,
    pub published_at: Option<String>,
    pub html_url: String,
    /// Installer untuk format instalasi ini; `None` bila tidak ada versi baru, format tidak
    /// diketahui, atau rilis tidak menyediakannya (UI menawarkan halaman rilis).
    pub installer: Option<Installer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installer {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

impl From<UpdateError> for AppError {
    fn from(e: UpdateError) -> Self {
        match e {
            UpdateError::Network(detail) | UpdateError::InvalidResponse(detail) => {
                tracing::warn!(%detail, "cek update gagal");
                AppError::UpdateNetwork
            }
            UpdateError::NotFound => AppError::UpdateNotFound,
            UpdateError::NoAsset => AppError::UpdateNoAsset,
            UpdateError::ChecksumMismatch | UpdateError::TooLarge => AppError::UpdateChecksum,
            UpdateError::Cancelled => AppError::UpdateCancelled,
            UpdateError::Io(e) => AppError::Io(e),
        }
    }
}

fn user_agent(current: &str) -> String {
    format!("Nabungo/{current}")
}

/// Rilis terbaru dibanding `current`; installer + hash dicari hanya bila ada versi baru.
pub fn check_update(current: &str, kind: Option<InstallKind>) -> Result<UpdateCheck, AppError> {
    check_with(&Client::new(&user_agent(current)), current, kind)
}

fn check_with(
    client: &Client,
    current: &str,
    kind: Option<InstallKind>,
) -> Result<UpdateCheck, AppError> {
    let release = client.latest_release(UPDATE_REPO)?;
    let latest = Version::parse(&release.tag_name);
    let available = match (latest, Version::parse(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    };
    let installer = match kind {
        Some(kind) if available => find_installer(client, &release.assets, kind)?,
        _ => None,
    };
    tracing::info!(latest = %release.tag_name, available, "cek update");
    Ok(UpdateCheck {
        current: current.to_owned(),
        latest: latest.map_or(release.tag_name.clone(), |v| v.to_string()),
        available,
        notes: release.body.unwrap_or_default(),
        published_at: release.published_at,
        html_url: release.html_url,
        installer,
    })
}

fn find_installer(
    client: &Client,
    assets: &[nabungo_update::Asset],
    kind: InstallKind,
) -> Result<Option<Installer>, AppError> {
    let Some(asset) = pick_asset(assets, kind) else {
        return Ok(None);
    };
    let Some(sums) = assets.iter().find(|a| a.name == SUMS_ASSET) else {
        return Ok(None);
    };
    let hashes = parse_sha256sums(&client.fetch_text(&sums.browser_download_url)?);
    Ok(hashes.get(&asset.name).map(|sha256| Installer {
        name: asset.name.clone(),
        url: asset.browser_download_url.clone(),
        size: asset.size,
        sha256: sha256.clone(),
    }))
}

/// URL unduhan rilis repo ini (bukan sembarang URL dari respons).
fn trusted_download(url: &str) -> bool {
    url.starts_with(&format!(
        "https://github.com/{UPDATE_REPO}/releases/download/"
    ))
}

/// Halaman rilis repo ini, aman dibuka di browser.
pub fn trusted_release_page(url: &str) -> bool {
    url.starts_with(&format!("https://github.com/{UPDATE_REPO}/releases/"))
}

/// Unduh installer `check` ke `dir/<nama>` setelah SHA-256 cocok.
pub fn download_update(
    check: &UpdateCheck,
    dir: &Path,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<PathBuf, AppError> {
    let installer = check.installer.as_ref().ok_or(AppError::UpdateNoAsset)?;
    if !trusted_download(&installer.url) {
        tracing::error!(url = %installer.url, "URL installer di luar repo rilis");
        return Err(AppError::UpdateNoAsset);
    }
    download_with(
        &Client::new(&user_agent(&check.current)),
        installer,
        dir,
        progress,
        cancel,
    )
}

fn download_with(
    client: &Client,
    installer: &Installer,
    dir: &Path,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<PathBuf, AppError> {
    // Nama dari respons tidak boleh keluar dari `dir`.
    if installer.name.contains(['/', '\\']) || installer.name.starts_with('.') {
        return Err(AppError::UpdateNoAsset);
    }
    std::fs::create_dir_all(dir)?;
    let dest = dir.join(&installer.name);
    client.download(
        &installer.url,
        &dest,
        &installer.sha256,
        MAX_INSTALLER_BYTES,
        progress,
        cancel,
    )?;
    tracing::info!(path = %dest.display(), "installer update terunduh");
    Ok(dest)
}

impl Ledger {
    /// Cek update otomatis saat startup; mati secara default (0 koneksi keluar, PRD §8).
    pub fn update_auto_check(&self) -> Result<bool, AppError> {
        Ok(setting::get_setting(&self.conn, KEY_AUTO_CHECK)?.as_deref() == Some("1"))
    }

    pub fn set_update_auto_check(&mut self, enabled: bool) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        setting::set_setting(&tx, KEY_AUTO_CHECK, if enabled { "1" } else { "0" })?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    use super::*;

    /// Server HTTP lokal: path → body (200), selain itu 404. Rute dibuat dari base URL-nya.
    fn serve(routes: impl FnOnce(&str) -> Vec<(String, Vec<u8>)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let routes = routes(&base);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                loop {
                    let mut h = String::new();
                    if reader.read_line(&mut h).unwrap() == 0 || h == "\r\n" {
                        break;
                    }
                }
                let path = line.split_whitespace().nth(1).unwrap_or("");
                let found = routes.iter().find(|(p, _)| p == path);
                let (status, body) = found.map_or((404, &[][..]), |(_, b)| (200, &b[..]));
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(body);
            }
        });
        base
    }

    /// Rilis `tag` berisi installer rpm (isi `abc`) + `SHA256SUMS`.
    fn release_server(tag: &'static str) -> String {
        serve(|base| {
            let release = serde_json::json!({
                "tag_name": tag,
                "body": "Catatan",
                "html_url": format!("https://github.com/{UPDATE_REPO}/releases/tag/{tag}"),
                "published_at": "2026-10-30T00:00:00Z",
                "assets": [
                    { "name": "Nabungo-0.5.0-1.x86_64.rpm", "browser_download_url": format!("{base}/rpm"), "size": 3 },
                    { "name": "SHA256SUMS", "browser_download_url": format!("{base}/sums"), "size": 80 },
                ]
            });
            vec![
                (
                    format!("/repos/{UPDATE_REPO}/releases/latest"),
                    release.to_string().into_bytes(),
                ),
                (
                    "/sums".to_owned(),
                    format!("{ABC_SHA256}  Nabungo-0.5.0-1.x86_64.rpm\n").into_bytes(),
                ),
                ("/rpm".to_owned(), b"abc".to_vec()),
            ]
        })
    }

    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn newer_release_offers_installer_with_hash() {
        let client = Client::for_test(&release_server("v0.5.0"));
        let check = check_with(&client, "0.4.0", Some(InstallKind::Rpm)).unwrap();
        assert!(check.available);
        assert_eq!(check.latest, "0.5.0");
        assert_eq!(check.notes, "Catatan");
        let installer = check.installer.unwrap();
        assert_eq!(installer.name, "Nabungo-0.5.0-1.x86_64.rpm");
        assert_eq!(installer.sha256, ABC_SHA256);

        let dir = tempfile::tempdir().unwrap();
        let path = download_with(
            &client,
            &installer,
            dir.path(),
            &mut |_, _| {},
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"abc");
    }

    #[test]
    fn same_version_or_unknown_kind_has_no_installer() {
        let client = Client::for_test(&release_server("v0.5.0"));
        let check = check_with(&client, "0.5.0", Some(InstallKind::Rpm)).unwrap();
        assert!(!check.available);
        assert_eq!(check.installer, None);
        let check = check_with(&client, "0.4.0", None).unwrap();
        assert!(check.available);
        assert_eq!(check.installer, None);
        let check = check_with(&client, "0.4.0", Some(InstallKind::Msi)).unwrap();
        assert_eq!(check.installer, None);
    }

    #[test]
    fn missing_release_maps_to_not_found() {
        let client = Client::for_test(&serve(|_| Vec::new()));
        let err = check_with(&client, "0.4.0", None).unwrap_err();
        assert!(matches!(err, AppError::UpdateNotFound));
    }

    #[test]
    fn auto_check_defaults_off_and_roundtrips() {
        let mut l = Ledger::open_in_memory().unwrap();
        assert!(!l.update_auto_check().unwrap());
        l.set_update_auto_check(true).unwrap();
        assert!(l.update_auto_check().unwrap());
        l.set_update_auto_check(false).unwrap();
        assert!(!l.update_auto_check().unwrap());
    }

    #[test]
    fn trusted_urls_are_limited_to_repo() {
        assert!(trusted_download(&format!(
            "https://github.com/{UPDATE_REPO}/releases/download/v1.0.0/x.rpm"
        )));
        assert!(!trusted_download(
            "https://github.com/evil/repo/releases/download/x.rpm"
        ));
        assert!(!trusted_download(
            "http://github.com/gilangfatahilah/nabungo-desktop/releases/download/x"
        ));
        assert!(trusted_release_page(&format!(
            "https://github.com/{UPDATE_REPO}/releases/tag/v1.0.0"
        )));
        assert!(!trusted_release_page("https://example.com/"));
    }

    #[test]
    fn untrusted_or_missing_installer_is_rejected() {
        let mut check = UpdateCheck {
            current: "0.4.0".into(),
            latest: "0.5.0".into(),
            available: true,
            notes: String::new(),
            published_at: None,
            html_url: String::new(),
            installer: None,
        };
        let dir = tempfile::tempdir().unwrap();
        let none = AtomicBool::new(false);
        let err = download_update(&check, dir.path(), &mut |_, _| {}, &none).unwrap_err();
        assert!(matches!(err, AppError::UpdateNoAsset));
        check.installer = Some(Installer {
            name: "x.rpm".into(),
            url: "https://example.com/x.rpm".into(),
            size: 1,
            sha256: "0".repeat(64),
        });
        let err = download_update(&check, dir.path(), &mut |_, _| {}, &none).unwrap_err();
        assert!(matches!(err, AppError::UpdateNoAsset));
    }
}
