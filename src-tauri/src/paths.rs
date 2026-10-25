//! Lokasi folder data: standar per-OS, atau portable di samping executable.

use std::path::{Path, PathBuf};

use anyhow::Context;

/// Folder data aktif. `portable` true bila dipilih lewat file penanda.
#[derive(Debug, Clone)]
pub struct DataDir {
    pub path: PathBuf,
    pub portable: bool,
}

impl DataDir {
    pub fn db_path(&self) -> PathBuf {
        self.path.join("nabungo.db")
    }

    pub fn backups_path(&self) -> PathBuf {
        self.path.join("backups")
    }

    pub fn attachments_path(&self) -> PathBuf {
        self.path.join("attachments")
    }

    /// Salinan lampiran berekstensi untuk viewer OS; dikosongkan saat start.
    pub fn open_cache_path(&self) -> PathBuf {
        self.path.join("cache").join("open")
    }

    pub fn session_marker_path(&self) -> PathBuf {
        self.path.join("session.lock")
    }
}

/// Portable bila `exe_dir/portable` ada (data di `exe_dir/data`); selain itu `base_data_dir/nabungo`.
pub fn resolve_data_dir(exe_dir: &Path, base_data_dir: Option<&Path>) -> anyhow::Result<DataDir> {
    let (path, portable) = if exe_dir.join("portable").exists() {
        (exe_dir.join("data"), true)
    } else {
        let base = base_data_dir.context("folder data standar OS tidak ditemukan")?;
        (base.join("nabungo"), false)
    };
    std::fs::create_dir_all(&path)
        .with_context(|| format!("gagal membuat folder data {}", path.display()))?;
    Ok(DataDir { path, portable })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_base_dir_and_creates_it() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("exe");
        let base = tmp.path().join("share");
        let d = resolve_data_dir(&exe, Some(&base)).unwrap();
        assert_eq!(d.path, base.join("nabungo"));
        assert!(!d.portable);
        assert!(d.path.is_dir());
    }

    #[test]
    fn portable_marker_wins() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("exe");
        std::fs::create_dir_all(&exe).unwrap();
        std::fs::write(exe.join("portable"), b"").unwrap();
        let d = resolve_data_dir(&exe, Some(&tmp.path().join("share"))).unwrap();
        assert_eq!(d.path, exe.join("data"));
        assert!(d.portable);
        assert!(d.path.is_dir());
    }

    #[test]
    fn no_base_dir_is_error() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(resolve_data_dir(tmp.path(), None).is_err());
    }

    #[test]
    fn attachments_dir_under_data() {
        let tmp = tempfile::tempdir().unwrap();
        let d = resolve_data_dir(tmp.path(), Some(&tmp.path().join("share"))).unwrap();
        assert_eq!(d.attachments_path(), d.path.join("attachments"));
    }

    #[test]
    fn open_cache_under_data() {
        let tmp = tempfile::tempdir().unwrap();
        let d = resolve_data_dir(tmp.path(), Some(&tmp.path().join("share"))).unwrap();
        assert_eq!(d.open_cache_path(), d.path.join("cache").join("open"));
    }
}
