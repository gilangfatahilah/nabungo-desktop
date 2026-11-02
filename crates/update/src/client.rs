//! HTTP ke GitHub: info rilis terbaru, `SHA256SUMS`, dan unduhan installer dengan verifikasi
//! SHA-256 sebelum file muncul di tujuan.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};
use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::{Release, UpdateError};

const API_TIMEOUT: Duration = Duration::from_secs(20);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Batas total satu unduhan installer (± 15 MB di jaringan lambat masih jauh di bawahnya).
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
/// `SHA256SUMS` dan JSON rilis jauh di bawah ini.
const SMALL_BODY_LIMIT: u64 = 1024 * 1024;
const CHUNK: usize = 64 * 1024;

/// Kemajuan unduhan: byte diterima, total (dari ukuran asset).
pub type Progress<'a> = &'a mut dyn FnMut(u64, u64);

pub struct Client {
    api: Agent,
    download: Agent,
    api_base: String,
}

impl Client {
    /// Klien HTTPS ke `api.github.com`; `user_agent` wajib untuk API GitHub.
    pub fn new(user_agent: &str) -> Self {
        Self::build(user_agent, "https://api.github.com", true)
    }

    /// Klien HTTP polos ke server lokal, khusus test.
    #[doc(hidden)]
    pub fn for_test(api_base: &str) -> Self {
        Self::build("nabungo-test", api_base, false)
    }

    fn build(user_agent: &str, api_base: &str, https_only: bool) -> Self {
        let agent = |global: Duration| -> Agent {
            Agent::config_builder()
                .tls_config(
                    TlsConfig::builder()
                        .provider(TlsProvider::NativeTls)
                        .root_certs(RootCerts::PlatformVerifier)
                        .build(),
                )
                .https_only(https_only)
                .user_agent(user_agent)
                .timeout_connect(Some(CONNECT_TIMEOUT))
                .timeout_global(Some(global))
                .build()
                .into()
        };
        Self {
            api: agent(API_TIMEOUT),
            download: agent(DOWNLOAD_TIMEOUT),
            api_base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    /// Rilis terbaru (bukan draft/pre-release) dari `owner/repo`.
    pub fn latest_release(&self, repo: &str) -> Result<Release, UpdateError> {
        let url = format!("{}/repos/{repo}/releases/latest", self.api_base);
        let mut resp = self
            .api
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(map_err)?;
        let text = resp
            .body_mut()
            .with_config()
            .limit(SMALL_BODY_LIMIT)
            .read_to_string()
            .map_err(map_err)?;
        serde_json::from_str(&text).map_err(|e| UpdateError::InvalidResponse(e.to_string()))
    }

    /// Isi teks kecil (mis. `SHA256SUMS`).
    pub fn fetch_text(&self, url: &str) -> Result<String, UpdateError> {
        let mut resp = self.download.get(url).call().map_err(map_err)?;
        resp.body_mut()
            .with_config()
            .limit(SMALL_BODY_LIMIT)
            .read_to_string()
            .map_err(map_err)
    }

    /// Unduh `url` ke `dest`. Data ditulis ke `<dest>.part` sambil di-hash; hanya bila SHA-256
    /// cocok dengan `sha256` file di-rename ke `dest`. Gagal/batal → `.part` dihapus.
    pub fn download(
        &self,
        url: &str,
        dest: &Path,
        sha256: &str,
        max_bytes: u64,
        progress: Progress<'_>,
        cancel: &AtomicBool,
    ) -> Result<(), UpdateError> {
        let part = part_path(dest);
        let result = self.download_to(url, &part, sha256, max_bytes, progress, cancel);
        match result {
            Ok(()) => fs::rename(&part, dest).map_err(|e| {
                let _ = fs::remove_file(&part);
                e.into()
            }),
            Err(e) => {
                let _ = fs::remove_file(&part);
                Err(e)
            }
        }
    }

    fn download_to(
        &self,
        url: &str,
        part: &Path,
        sha256: &str,
        max_bytes: u64,
        progress: Progress<'_>,
        cancel: &AtomicBool,
    ) -> Result<(), UpdateError> {
        let mut resp = self.download.get(url).call().map_err(map_err)?;
        let total = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        if total > max_bytes {
            return Err(UpdateError::TooLarge);
        }
        let mut reader = resp.body_mut().with_config().limit(u64::MAX).reader();
        let mut file = File::create(part)?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; CHUNK];
        let mut received = 0u64;
        progress(0, total);
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(UpdateError::Cancelled);
            }
            let n = reader
                .read(&mut buf)
                .map_err(|e| UpdateError::Network(e.to_string()))?;
            if n == 0 {
                break;
            }
            received += n as u64;
            if received > max_bytes {
                return Err(UpdateError::TooLarge);
            }
            hasher.update(&buf[..n]);
            file.write_all(&buf[..n])?;
            progress(received, total);
        }
        file.sync_all()?;
        drop(file);
        let actual = hex(&hasher.finalize());
        if !actual.eq_ignore_ascii_case(sha256) {
            tracing::warn!(expected = sha256, %actual, "checksum installer tidak cocok");
            return Err(UpdateError::ChecksumMismatch);
        }
        Ok(())
    }
}

fn part_path(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    dest.with_file_name(name)
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn map_err(e: ureq::Error) -> UpdateError {
    match e {
        ureq::Error::StatusCode(404) => UpdateError::NotFound,
        ureq::Error::StatusCode(code) => UpdateError::InvalidResponse(format!("HTTP {code}")),
        ureq::Error::BodyExceedsLimit(_) => UpdateError::TooLarge,
        ureq::Error::Io(e) => UpdateError::Network(e.to_string()),
        other => UpdateError::Network(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    /// Server HTTP satu-permintaan-per-koneksi: path → (status, body).
    fn serve(routes: Vec<(&'static str, u16, Vec<u8>)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut line = String::new();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                reader.read_line(&mut line).unwrap();
                loop {
                    let mut h = String::new();
                    if reader.read_line(&mut h).unwrap() == 0 || h == "\r\n" {
                        break;
                    }
                }
                let path = line.split_whitespace().nth(1).unwrap_or("").to_owned();
                let (status, body) = routes
                    .iter()
                    .find(|(p, _, _)| *p == path)
                    .map(|(_, s, b)| (*s, b.clone()))
                    .unwrap_or((404, Vec::new()));
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        format!("http://{addr}")
    }

    fn sha(data: &[u8]) -> String {
        hex(&Sha256::digest(data))
    }

    #[test]
    fn latest_release_parses_and_maps_404() {
        let json = include_bytes!("../tests/fixtures/release-v0.3.0.json").to_vec();
        let base = serve(vec![("/repos/o/r/releases/latest", 200, json)]);
        let client = Client::for_test(&base);
        assert_eq!(client.latest_release("o/r").unwrap().tag_name, "v0.3.0");
        assert!(matches!(
            client.latest_release("o/missing"),
            Err(UpdateError::NotFound)
        ));
    }

    #[test]
    fn download_verifies_checksum_and_reports_progress() {
        let data = vec![7u8; 200_000];
        let base = serve(vec![("/f", 200, data.clone())]);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("Nabungo.rpm");
        let mut last = (0, 0);
        Client::for_test(&base)
            .download(
                &format!("{base}/f"),
                &dest,
                &sha(&data).to_uppercase(),
                1 << 20,
                &mut |r, t| last = (r, t),
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(fs::read(&dest).unwrap(), data);
        assert_eq!(last, (200_000, 200_000));
        assert!(!part_path(&dest).exists());
    }

    #[test]
    fn download_failures_leave_no_files() {
        let data = vec![1u8; 10_000];
        let base = serve(vec![("/f", 200, data.clone())]);
        let client = Client::for_test(&base);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("x.deb");
        let url = format!("{base}/f");
        let no_cancel = AtomicBool::new(false);
        let cases: [(&str, u64, &AtomicBool); 3] = [
            (&"0".repeat(64), 1 << 20, &no_cancel),
            (&sha(&data), 100, &no_cancel),
            (&sha(&data), 1 << 20, &AtomicBool::new(true)),
        ];
        for (i, (hash, max, cancel)) in cases.into_iter().enumerate() {
            let err = client
                .download(&url, &dest, hash, max, &mut |_, _| {}, cancel)
                .unwrap_err();
            match i {
                0 => assert!(matches!(err, UpdateError::ChecksumMismatch)),
                1 => assert!(matches!(err, UpdateError::TooLarge)),
                _ => assert!(matches!(err, UpdateError::Cancelled)),
            }
            assert!(!dest.exists());
            assert!(!part_path(&dest).exists());
        }
    }

    #[test]
    fn fetch_text_reads_sums() {
        let base = serve(vec![("/SHA256SUMS", 200, b"abc".to_vec())]);
        let text = Client::for_test(&base)
            .fetch_text(&format!("{base}/SHA256SUMS"))
            .unwrap();
        assert_eq!(text, "abc");
    }
}
