//! Enkripsi DB (SEC-01): status file, unlock, recovery key, aktif/nonaktif, ganti password.
//!
//! Header file DB adalah sumber kebenaran status enkripsi. Urutan langkah di sini dipilih agar
//! crash di titik mana pun meninggalkan keadaan yang bisa dibuka: key file ditulis sebelum DB
//! terenkripsi menggantikan yang lama, dan baru dihapus setelah DB plaintext menggantikannya;
//! [`db_state`] membersihkan sisanya.

use std::path::{Path, PathBuf};

use nabungo_storage::rusqlite::Connection;
use nabungo_storage::{DbKey, StorageError, convert_file, is_encrypted_file};

use crate::backup::list_backups;
use crate::vault::{
    KdfParams, KeyFile, format_recovery_key, keyfile_path, parse_recovery_key, validate_password,
};
use crate::{AppError, Ledger};

/// Status file DB di disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbState {
    Plain,
    /// `has_keyfile == false` → hanya recovery key yang bisa membukanya.
    Encrypted {
        has_keyfile: bool,
    },
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn remove_if_exists(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

fn remove_logged(path: &Path) {
    if let Err(e) = remove_if_exists(path) {
        tracing::warn!(error = %e, path = %path.display(), "gagal menghapus file sementara");
    }
}

/// Hasil konversi sementara di samping DB (`nabungo.db.convert`).
fn convert_path(db: &Path) -> PathBuf {
    sibling(db, ".convert")
}

/// Status enkripsi `db` sekaligus membersihkan sisa enable/disable yang terputus: file
/// konversi sementara, dan key file di samping DB plaintext. DB yang belum ada = `Plain`.
pub fn db_state(db: &Path) -> Result<DbState, AppError> {
    let convert = convert_path(db);
    for path in [
        convert.clone(),
        sibling(&convert, ".partial"),
        sibling(&convert, ".partial-journal"),
        sibling(&convert, ".partial-wal"),
        sibling(&convert, ".partial-shm"),
    ] {
        remove_if_exists(&path)?;
    }
    let keyfile = keyfile_path(db);
    if is_encrypted_file(db)? {
        return Ok(DbState::Encrypted {
            has_keyfile: keyfile.is_file(),
        });
    }
    // Hanya bila DB-nya ada: DB yang dipindah sementara tidak boleh kehilangan key file-nya.
    if db.is_file() && keyfile.exists() {
        tracing::warn!("key file tersisa di samping DB plaintext; dihapus");
        remove_if_exists(&keyfile)?;
    }
    Ok(DbState::Plain)
}

fn read_keyfile(db: &Path) -> Result<KeyFile, AppError> {
    KeyFile::read(&keyfile_path(db)).map_err(|e| match e {
        AppError::Io(e) if e.kind() == std::io::ErrorKind::NotFound => AppError::KeyFileInvalid,
        e => e,
    })
}

/// Buka DEK dengan password lewat key file. Key file tidak ada → `KeyFileInvalid`.
pub fn unlock_key(db: &Path, password: &str) -> Result<DbKey, AppError> {
    read_keyfile(db)?.unseal(password)
}

/// Buka DB dengan recovery key lalu tulis key file baru untuk `new_password`.
pub fn recover_key(
    db: &Path,
    recovery: &str,
    new_password: &str,
    params: KdfParams,
) -> Result<DbKey, AppError> {
    validate_password(new_password)?;
    let key = parse_recovery_key(recovery)?;
    match nabungo_storage::open_file_read_only(db, Some(&key)) {
        Ok(conn) => drop(conn),
        Err(StorageError::WrongKey) => return Err(AppError::InvalidRecoveryKey),
        Err(e) => return Err(e.into()),
    }
    KeyFile::seal(&key, new_password, params)?.write_atomic(&keyfile_path(db))?;
    Ok(key)
}

/// Ubah status enkripsi tiap backup di `dir` ke status `to`; best-effort, kegagalan dicatat.
fn convert_backups(dir: &Path, from: Option<&DbKey>, to: Option<&DbKey>) {
    let list = match list_backups(dir) {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(error = %e, "gagal membaca daftar backup untuk konversi");
            return;
        }
    };
    for b in list {
        let path = dir.join(&b.file_name);
        let tmp = sibling(&path, ".convert");
        let result = (|| -> Result<(), AppError> {
            if is_encrypted_file(&path)? == to.is_some() {
                return Ok(());
            }
            convert_file(&path, from, &tmp, to)?;
            std::fs::rename(&tmp, &path)?;
            Ok(())
        })();
        if let Err(e) = result {
            tracing::warn!(error = %e, file = %b.file_name, "konversi enkripsi backup gagal");
            remove_logged(&tmp);
        }
    }
}

impl Ledger {
    pub fn is_encrypted(&self) -> bool {
        self.key.is_some()
    }

    fn file_path(&self) -> Result<PathBuf, AppError> {
        self.path.clone().ok_or(AppError::EncryptionState)
    }

    /// Tutup koneksi, ganti `db` dengan `converted`, lalu buka ulang dengan `new_key`. Bila
    /// rename gagal, DB lama dibuka kembali dan `converted` dihapus.
    fn swap_file(
        &mut self,
        db: &Path,
        converted: &Path,
        new_key: Option<DbKey>,
    ) -> Result<(), AppError> {
        // Koneksi sementara tanpa skema: bila buka ulang gagal, use case lain ikut gagal
        // alih-alih menulis ke tempat lain.
        let old = std::mem::replace(&mut self.conn, Connection::open_in_memory()?);
        if let Err((conn, e)) = old.close() {
            self.conn = conn;
            remove_logged(converted);
            return Err(e.into());
        }
        let swapped = (|| -> std::io::Result<()> {
            // Isi WAL sudah ada di hasil konversi; sisa WAL milik file lama tidak boleh
            // diterapkan ke file baru.
            remove_if_exists(&sibling(db, "-wal"))?;
            remove_if_exists(&sibling(db, "-shm"))?;
            std::fs::rename(converted, db)
        })();
        if let Err(e) = swapped {
            remove_logged(converted);
            self.conn = nabungo_storage::open(db, self.key.as_ref())?;
            return Err(e.into());
        }
        self.key = new_key;
        self.conn = nabungo_storage::open(db, self.key.as_ref())?;
        Ok(())
    }

    /// Enkripsi DB dengan key acak yang dibungkus `password`; mengembalikan recovery key.
    /// Backup di `backups` ikut dienkripsi (best-effort).
    pub fn enable_encryption(
        &mut self,
        password: &str,
        backups: &Path,
        params: KdfParams,
    ) -> Result<String, AppError> {
        if self.is_encrypted() {
            return Err(AppError::EncryptionState);
        }
        validate_password(password)?;
        let db = self.file_path()?;
        let dek = DbKey::generate();
        let converted = convert_path(&db);
        convert_file(&db, None, &converted, Some(&dek))?;
        let keyfile = keyfile_path(&db);
        if let Err(e) = KeyFile::seal(&dek, password, params)?.write_atomic(&keyfile) {
            remove_logged(&converted);
            return Err(e);
        }
        if let Err(e) = self.swap_file(&db, &converted, Some(dek.clone())) {
            if !is_encrypted_file(&db).unwrap_or(true) {
                remove_logged(&keyfile);
            }
            return Err(e);
        }
        convert_backups(backups, None, Some(&dek));
        Ok(format_recovery_key(&dek))
    }

    /// Kembalikan DB ke plaintext setelah password diverifikasi; key file dihapus setelah DB
    /// plaintext menggantikan yang terenkripsi. Backup ikut didekripsi (best-effort).
    pub fn disable_encryption(&mut self, password: &str, backups: &Path) -> Result<(), AppError> {
        let Some(key) = self.key.clone() else {
            return Err(AppError::EncryptionState);
        };
        let db = self.file_path()?;
        if unlock_key(&db, password)? != key {
            return Err(AppError::KeyFileInvalid);
        }
        let converted = convert_path(&db);
        convert_file(&db, Some(&key), &converted, None)?;
        self.swap_file(&db, &converted, None)?;
        // Gagal hapus tidak berbahaya: `db_state` membuang key file di samping DB plaintext.
        remove_logged(&keyfile_path(&db));
        convert_backups(backups, Some(&key), None);
        Ok(())
    }

    /// Bungkus ulang key yang sama dengan password baru.
    pub fn change_password(&self, old: &str, new: &str, params: KdfParams) -> Result<(), AppError> {
        let Some(key) = &self.key else {
            return Err(AppError::EncryptionState);
        };
        validate_password(new)?;
        let db = self.file_path()?;
        if &unlock_key(&db, old)? != key {
            return Err(AppError::KeyFileInvalid);
        }
        KeyFile::seal(key, new, params)?.write_atomic(&keyfile_path(&db))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AccountFields;
    use nabungo_core::AccountKind;
    use time::OffsetDateTime;
    use time::macros::{date, datetime};

    const NOW: OffsetDateTime = datetime!(2026-10-20 03:15:00 UTC);
    const PW: &str = "rahasia-123";
    const FAST: KdfParams = KdfParams::FAST;

    struct Env {
        _dir: tempfile::TempDir,
        db: PathBuf,
        backups: PathBuf,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("nabungo.db");
        let backups = dir.path().join("backups");
        Env {
            db,
            backups,
            _dir: dir,
        }
    }

    fn add_account(l: &mut Ledger, name: &str, balance: i64) {
        l.create_account(
            AccountKind::Cash,
            &AccountFields::new(name, balance, date!(2026 - 10 - 01)),
        )
        .unwrap();
    }

    fn names(l: &Ledger) -> Vec<(String, i64)> {
        l.accounts_overview()
            .unwrap()
            .accounts
            .into_iter()
            .map(|a| (a.name, a.balance))
            .collect()
    }

    /// Ledger file dengan satu akun, lalu dienkripsi.
    fn encrypted(e: &Env) -> (Ledger, String) {
        let mut l = Ledger::open_with_backups(&e.db, &e.backups, None, NOW).unwrap();
        add_account(&mut l, "Dompet", 50_000);
        let rk = l.enable_encryption(PW, &e.backups, FAST).unwrap();
        (l, rk)
    }

    fn reopen(e: &Env, key: &DbKey) -> Ledger {
        Ledger::open_with_backups(&e.db, &e.backups, Some(key), NOW).unwrap()
    }

    #[test]
    fn enable_encrypts_file_and_returns_recovery_key() {
        let e = env();
        let (l, rk) = encrypted(&e);
        assert!(l.is_encrypted());
        assert!(is_encrypted_file(&e.db).unwrap());
        assert!(keyfile_path(&e.db).is_file());
        assert_eq!(rk.len(), 64);
        assert!(!convert_path(&e.db).exists());
    }

    #[test]
    fn enable_keeps_latest_write_and_leaves_no_wal() {
        let e = env();
        let mut l = Ledger::open_with_backups(&e.db, &e.backups, None, NOW).unwrap();
        add_account(&mut l, "Dompet", 50_000);
        add_account(&mut l, "Terakhir", 7_000);
        l.enable_encryption(PW, &e.backups, FAST).unwrap();
        // Ledger yang sama tetap bisa menulis setelah swap.
        add_account(&mut l, "Sesudah", 1);
        let expected = names(&l);
        drop(l);
        assert!(!sibling(&e.db, "-wal").exists());
        let l = reopen(&e, &unlock_key(&e.db, PW).unwrap());
        assert_eq!(names(&l), expected);
        assert!(names(&l).contains(&("Terakhir".into(), 7_000)));
    }

    #[test]
    fn enable_twice_is_encryption_state_error() {
        let e = env();
        let (mut l, _) = encrypted(&e);
        assert!(matches!(
            l.enable_encryption(PW, &e.backups, FAST),
            Err(AppError::EncryptionState)
        ));
    }

    #[test]
    fn enable_rejects_short_password() {
        let e = env();
        let mut l = Ledger::open_with_backups(&e.db, &e.backups, None, NOW).unwrap();
        add_account(&mut l, "Dompet", 50_000);
        assert!(matches!(
            l.enable_encryption("pendek", &e.backups, FAST),
            Err(AppError::PasswordTooShort)
        ));
        assert!(!l.is_encrypted());
        assert!(!is_encrypted_file(&e.db).unwrap());
        assert!(!keyfile_path(&e.db).exists());
    }

    #[test]
    fn unlock_with_password_opens_ledger() {
        let e = env();
        let (l, _) = encrypted(&e);
        let before = names(&l);
        drop(l);
        assert_eq!(
            db_state(&e.db).unwrap(),
            DbState::Encrypted { has_keyfile: true }
        );
        let l = reopen(&e, &unlock_key(&e.db, PW).unwrap());
        assert!(l.is_encrypted());
        assert_eq!(names(&l), before);
        assert!(matches!(
            Ledger::open_with_backups(&e.db, &e.backups, None, NOW),
            Err(AppError::Locked)
        ));
    }

    #[test]
    fn unlock_wrong_password() {
        let e = env();
        drop(encrypted(&e));
        assert!(matches!(
            unlock_key(&e.db, "salah-1234"),
            Err(AppError::WrongPassword)
        ));
    }

    #[test]
    fn recover_with_recovery_key_sets_new_password() {
        let e = env();
        let (l, rk) = encrypted(&e);
        drop(l);
        let key = recover_key(&e.db, &rk.to_lowercase(), "baru-12345", FAST).unwrap();
        assert!(matches!(
            unlock_key(&e.db, PW),
            Err(AppError::WrongPassword)
        ));
        assert_eq!(unlock_key(&e.db, "baru-12345").unwrap(), key);
        assert_eq!(names(&reopen(&e, &key)).len(), 1);
    }

    #[test]
    fn recover_with_wrong_recovery_key() {
        let e = env();
        drop(encrypted(&e));
        let before = std::fs::read(keyfile_path(&e.db)).unwrap();
        let other = format_recovery_key(&DbKey::generate());
        assert!(matches!(
            recover_key(&e.db, &other, "baru-12345", FAST),
            Err(AppError::InvalidRecoveryKey)
        ));
        assert!(matches!(
            recover_key(&e.db, "bukan-key", "baru-12345", FAST),
            Err(AppError::InvalidRecoveryKey)
        ));
        assert_eq!(std::fs::read(keyfile_path(&e.db)).unwrap(), before);
    }

    #[test]
    fn change_password_keeps_data_readable() {
        let e = env();
        let (l, _) = encrypted(&e);
        assert!(matches!(
            l.change_password("salah-1234", "baru-12345", FAST),
            Err(AppError::WrongPassword)
        ));
        assert!(matches!(
            l.change_password(PW, "pendek", FAST),
            Err(AppError::PasswordTooShort)
        ));
        l.change_password(PW, "baru-12345", FAST).unwrap();
        drop(l);
        assert!(matches!(
            unlock_key(&e.db, PW),
            Err(AppError::WrongPassword)
        ));
        let l = reopen(&e, &unlock_key(&e.db, "baru-12345").unwrap());
        assert_eq!(names(&l), vec![("Dompet".into(), 50_000)]);
    }

    #[test]
    fn disable_restores_plaintext_and_removes_keyfile() {
        let e = env();
        let (mut l, _) = encrypted(&e);
        assert!(matches!(
            l.disable_encryption("salah-1234", &e.backups),
            Err(AppError::WrongPassword)
        ));
        assert!(is_encrypted_file(&e.db).unwrap());
        l.disable_encryption(PW, &e.backups).unwrap();
        assert!(!l.is_encrypted());
        add_account(&mut l, "Plain", 1);
        drop(l);
        assert!(!is_encrypted_file(&e.db).unwrap());
        assert!(!keyfile_path(&e.db).exists());
        assert_eq!(db_state(&e.db).unwrap(), DbState::Plain);
        let l = Ledger::open_with_backups(&e.db, &e.backups, None, NOW).unwrap();
        assert_eq!(names(&l).len(), 2);
        let mut l = l;
        assert!(matches!(
            l.disable_encryption(PW, &e.backups),
            Err(AppError::EncryptionState)
        ));
    }

    #[test]
    fn backups_follow_encryption_state() {
        let e = env();
        let mut l = Ledger::open_with_backups(&e.db, &e.backups, None, NOW).unwrap();
        add_account(&mut l, "Dompet", 50_000);
        let b = l.auto_backup_if_due(&e.backups, NOW).unwrap().unwrap();
        let path = e.backups.join(&b.file_name);
        assert!(!is_encrypted_file(&path).unwrap());
        l.enable_encryption(PW, &e.backups, FAST).unwrap();
        assert!(is_encrypted_file(&path).unwrap());
        // Backup baru selagi terenkripsi juga terenkripsi.
        let b2 = l
            .auto_backup_if_due(&e.backups, NOW + time::Duration::days(2))
            .unwrap()
            .unwrap();
        assert!(is_encrypted_file(&e.backups.join(&b2.file_name)).unwrap());
        l.disable_encryption(PW, &e.backups).unwrap();
        assert!(!is_encrypted_file(&path).unwrap());
        assert!(!is_encrypted_file(&e.backups.join(&b2.file_name)).unwrap());
        let names: Vec<_> = std::fs::read_dir(&e.backups)
            .unwrap()
            .map(|d| d.unwrap().file_name().into_string().unwrap())
            .collect();
        assert!(names.iter().all(|n| !n.contains(".convert")), "{names:?}");
    }

    #[test]
    fn restore_named_plain_backup_after_enable() {
        let e = env();
        let mut l = Ledger::open_with_backups(&e.db, &e.backups, None, NOW).unwrap();
        add_account(&mut l, "Lama", 10);
        let plain = e.db.with_file_name("salinan.db");
        l.backup_to(&plain).unwrap();
        add_account(&mut l, "Baru", 20);
        l.enable_encryption(PW, &e.backups, FAST).unwrap();
        std::fs::create_dir_all(&e.backups).unwrap();
        let name = "auto-20261020T031500Z.db";
        std::fs::copy(&plain, e.backups.join(name)).unwrap();
        l.restore_named(&e.backups, name, NOW).unwrap();
        assert_eq!(names(&l), vec![("Lama".into(), 10)]);
        drop(l);
        assert!(is_encrypted_file(&e.db).unwrap());
        let l = reopen(&e, &unlock_key(&e.db, PW).unwrap());
        assert_eq!(names(&l), vec![("Lama".into(), 10)]);
    }

    #[test]
    fn db_state_plain_with_stale_keyfile_cleans_up() {
        let e = env();
        drop(Ledger::open(&e.db).unwrap());
        std::fs::write(keyfile_path(&e.db), b"{}").unwrap();
        std::fs::write(convert_path(&e.db), b"sisa").unwrap();
        assert_eq!(db_state(&e.db).unwrap(), DbState::Plain);
        assert!(!keyfile_path(&e.db).exists());
        assert!(!convert_path(&e.db).exists());
        // DB belum ada: Plain, key file tidak disentuh.
        let missing = e.db.with_file_name("baru.db");
        std::fs::write(keyfile_path(&missing), b"{}").unwrap();
        assert_eq!(db_state(&missing).unwrap(), DbState::Plain);
        assert!(keyfile_path(&missing).exists());
    }

    #[test]
    fn db_state_encrypted_without_keyfile() {
        let e = env();
        let (l, rk) = encrypted(&e);
        drop(l);
        std::fs::remove_file(keyfile_path(&e.db)).unwrap();
        assert_eq!(
            db_state(&e.db).unwrap(),
            DbState::Encrypted { has_keyfile: false }
        );
        assert!(matches!(
            unlock_key(&e.db, PW),
            Err(AppError::KeyFileInvalid)
        ));
        let key = recover_key(&e.db, &rk, "baru-12345", FAST).unwrap();
        assert_eq!(names(&reopen(&e, &key)).len(), 1);
    }

    #[test]
    fn backup_target_keyfile_is_rejected() {
        let e = env();
        let (l, _) = encrypted(&e);
        assert!(matches!(
            l.backup_to(&keyfile_path(&e.db)),
            Err(AppError::BackupTargetInvalid)
        ));
    }
}
