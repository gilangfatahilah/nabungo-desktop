//! Cek update dari GitHub Releases dan unduh installer yang cocok (opt-in, mati secara default).
//! Satu dari dua crate dengan kode jaringan (selain `nabungo-mail`); app hanya memanggilnya saat
//! user menekan "Periksa update" atau mengaktifkan cek otomatis.

pub mod client;
pub mod release;
pub mod version;

pub use client::{Client, Progress};
pub use release::{Asset, InstallKind, Release, parse_sha256sums, pick_asset};
pub use version::Version;

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// Tidak bisa terhubung / timeout / TLS gagal.
    #[error("network error: {0}")]
    Network(String),
    /// Repo atau rilis tidak ada (404), mis. repo privat atau belum ada rilis.
    #[error("release not found")]
    NotFound,
    /// Respons server tidak bisa dibaca.
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    /// Rilis tidak punya installer untuk format ini, atau tidak ada di `SHA256SUMS`.
    #[error("no installer for this platform")]
    NoAsset,
    #[error("checksum mismatch")]
    ChecksumMismatch,
    #[error("download too large")]
    TooLarge,
    #[error("download cancelled")]
    Cancelled,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
