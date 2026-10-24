//! Pemulihan DB yang tidak bisa dibuka (P3–P5), tanpa `Ledger`: DB baru dibangun dari backup di
//! `nabungo.db.recover`, file lama dipindah ke samping (`.broken-<waktu>`, tidak pernah dihapus),
//! lalu `.recover` di-rename menjadi DB. [`crate::security::db_state`] menuntaskan rename yang
//! terputus dan membuang `.recover` yang belum selesai.

use std::path::{Path, PathBuf};

use nabungo_storage::{DbKey, StorageError, is_encrypted_file};
use time::OffsetDateTime;

use crate::AppError;
use crate::backup::{STAMP, ensure_not_live_path, restore_sidecar};
use crate::security::unlock_key;
use crate::vault::{KdfParams, KeyFile, keyfile_path, parse_recovery_key, validate_password};

/// Rahasia untuk membuka backup sumber (P5).
pub enum BackupSecret {
    None,
    /// Password instalasi ini: membuka `nabungo.key` di samping DB.
    Password(String),
    /// Recovery key milik backup; key file baru ditulis untuk `new_password`.
    RecoveryKey {
        key: String,
        new_password: String,
    },
}

pub struct Recovered {
    pub key: Option<DbKey>,
    /// File lama yang dipindah ke samping (P3), untuk ditampilkan dan di-log.
    pub moved_aside: Vec<PathBuf>,
}

/// `.broken-YYYYMMDDTHHMMSSZ`.
pub fn broken_suffix(now: OffsetDateTime) -> String {
    let stamp = now
        .to_offset(time::UtcOffset::UTC)
        .format(STAMP)
        .expect("format stempel waktu statis");
    format!(".broken-{stamp}")
}

/// Bangun DB baru dari backup `src` untuk menggantikan `db` yang rusak (P4). Semua validasi
/// (key sumber, isi backup) selesai sebelum file apa pun disentuh; gagal = tidak ada perubahan.
pub fn restore_into_fresh(
    db: &Path,
    src: &Path,
    attachments_dir: &Path,
    secret: &BackupSecret,
    params: KdfParams,
    now: OffsetDateTime,
) -> Result<Recovered, AppError> {
    ensure_not_live_path(db, src)?;
    let encrypted = is_encrypted_file(src)?;
    let (key, new_keyfile) = if encrypted {
        source_key(db, secret, params)?
    } else {
        (None, None)
    };
    nabungo_storage::inspect_backup(src, key.as_ref()).map_err(
        |e| match AppError::from_backup(e) {
            AppError::BackupKeyRequired if matches!(secret, BackupSecret::RecoveryKey { .. }) => {
                AppError::InvalidRecoveryKey
            }
            e => e,
        },
    )?;

    let recover = sibling(db, ".recover");
    remove_db_files(&recover)?;
    if let Err(e) = build(&recover, src, key.as_ref()) {
        if let Err(e) = remove_db_files(&recover) {
            tracing::warn!(error = %e, "gagal menghapus sisa pemulihan");
        }
        return Err(e);
    }

    // Key file instalasi ini tetap dipakai hanya bila backup-nya dibuka dengan password ini.
    let keep_keyfile = encrypted && matches!(secret, BackupSecret::Password(_));
    let suffix = broken_suffix(now);
    let mut moved_aside = move_db_aside(db, &suffix)?;
    if !keep_keyfile {
        moved_aside.extend(move_aside(&keyfile_path(db), &suffix)?);
    }
    // Key file baru ditulis sebelum rename final: crash di antaranya diselesaikan `db_state`.
    if let Some(keyfile) = new_keyfile {
        keyfile.write_atomic(&keyfile_path(db))?;
    }
    std::fs::rename(&recover, db)?;
    restore_sidecar(src, attachments_dir);
    tracing::info!(moved = ?moved_aside, encrypted, "DB dipulihkan dari backup");
    Ok(Recovered { key, moved_aside })
}

/// Pindahkan DB dan key file ke samping; DB baru (kosong, onboarding) dibuat oleh pemanggil.
pub fn start_fresh(db: &Path, now: OffsetDateTime) -> Result<Vec<PathBuf>, AppError> {
    let suffix = broken_suffix(now);
    let mut moved = move_db_aside(db, &suffix)?;
    moved.extend(move_aside(&keyfile_path(db), &suffix)?);
    tracing::info!(moved = ?moved, "mulai baru; DB lama dipindah ke samping");
    Ok(moved)
}

/// Key sumber terenkripsi dari `secret`, plus key file baru untuk `RecoveryKey`.
fn source_key(
    db: &Path,
    secret: &BackupSecret,
    params: KdfParams,
) -> Result<(Option<DbKey>, Option<KeyFile>), AppError> {
    match secret {
        BackupSecret::None => Err(AppError::BackupKeyRequired),
        BackupSecret::Password(pw) => Ok((Some(unlock_key(db, pw)?), None)),
        BackupSecret::RecoveryKey { key, new_password } => {
            validate_password(new_password)?;
            let key = parse_recovery_key(key)?;
            let keyfile = KeyFile::seal(&key, new_password, params)?;
            Ok((Some(key), Some(keyfile)))
        }
    }
}

/// Salin `src` ke `recover` (status enkripsi sama → backup API), lalu migrasi.
fn build(recover: &Path, src: &Path, key: Option<&DbKey>) -> Result<(), AppError> {
    let mut conn = nabungo_storage::open_file(recover, key)?;
    nabungo_storage::restore_from(&mut conn, src, key, key)?;
    nabungo_storage::migrate(&mut conn)?;
    conn.close().map_err(|(_, e)| StorageError::from(e))?;
    Ok(())
}

const DB_SIBLINGS: [&str; 4] = ["", "-wal", "-shm", "-journal"];

fn remove_db_files(db: &Path) -> std::io::Result<()> {
    for suffix in DB_SIBLINGS {
        match std::fs::remove_file(sibling(db, suffix)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    }
    Ok(())
}

fn move_db_aside(db: &Path, suffix: &str) -> Result<Vec<PathBuf>, AppError> {
    let mut moved = Vec::new();
    for s in DB_SIBLINGS {
        moved.extend(move_aside(&sibling(db, s), suffix)?);
    }
    Ok(moved)
}

/// Rename `path` → `path<suffix>` (atau `<suffix>-N` bila sudah ada); `None` bila tidak ada.
pub(crate) fn move_aside(path: &Path, suffix: &str) -> Result<Option<PathBuf>, AppError> {
    if !path.exists() {
        return Ok(None);
    }
    let mut target = sibling(path, suffix);
    let mut n = 2;
    while target.exists() {
        target = sibling(path, &format!("{suffix}-{n}"));
        n += 1;
    }
    std::fs::rename(path, &target)?;
    Ok(Some(target))
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{DbState, db_state, unlock_key};
    use crate::{AccountFields, Ledger};
    use nabungo_core::AccountKind;
    use time::macros::{date, datetime};

    const NOW: OffsetDateTime = datetime!(2026-10-24 10:15:00 UTC);
    const PW: &str = "rahasia-123";
    const NEW_PW: &str = "baru-456789";
    const FAST: KdfParams = KdfParams::FAST;

    fn garbage() -> Vec<u8> {
        (0..4096u32).map(|i| (i * 31 % 251) as u8).collect()
    }

    fn balances(l: &Ledger) -> Vec<(String, i64)> {
        let mut v: Vec<_> = l
            .accounts_overview()
            .unwrap()
            .accounts
            .into_iter()
            .map(|a| (a.name, a.balance))
            .collect();
        v.sort();
        v
    }

    /// Instalasi di `dir` dengan satu akun; terenkripsi bila `encrypt`. Mengembalikan backup
    /// `x.db`, saldo, dan recovery key.
    fn install(dir: &Path, encrypt: bool) -> (PathBuf, Vec<(String, i64)>, Option<String>) {
        let db = dir.join("nabungo.db");
        let backups = dir.join("backups");
        let mut l = Ledger::open_with_backups(&db, &backups, None, NOW).unwrap();
        l.create_account(
            AccountKind::Cash,
            &AccountFields::new("Dompet", 50_000, date!(2026 - 10 - 01)),
        )
        .unwrap();
        let rk = encrypt.then(|| l.enable_encryption(PW, &backups, FAST).unwrap());
        let x = dir.join("x.db");
        l.backup_to(&x, &dir.join("no-attachments")).unwrap();
        (x, balances(&l), rk)
    }

    /// Timpa DB (Ledger sudah ditutup) dengan sampah.
    fn break_db(db: &Path) {
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(sibling(db, suffix));
        }
        std::fs::write(db, garbage()).unwrap();
    }

    fn listing(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut v: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_file())
            .map(|p| {
                (
                    p.file_name().unwrap().to_string_lossy().into_owned(),
                    std::fs::read(&p).unwrap(),
                )
            })
            .collect();
        v.sort();
        v
    }

    fn broken(path: &Path) -> PathBuf {
        sibling(path, &broken_suffix(NOW))
    }

    fn no_att(dir: &Path) -> PathBuf {
        dir.join("attachments")
    }

    #[test]
    fn broken_suffix_format() {
        assert_eq!(broken_suffix(NOW), ".broken-20261024T101500Z");
    }

    #[test]
    fn restore_into_fresh_replaces_garbage_db() {
        let src_dir = tempfile::tempdir().unwrap();
        let (x, expected, _) = install(src_dir.path(), false);
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("nabungo.db");
        std::fs::write(&db, garbage()).unwrap();

        let r = restore_into_fresh(&db, &x, &no_att(dir.path()), &BackupSecret::None, FAST, NOW)
            .unwrap();
        assert!(r.key.is_none());
        assert_eq!(balances(&Ledger::open(&db).unwrap()), expected);
        assert_eq!(r.moved_aside, vec![broken(&db)]);
        assert_eq!(std::fs::read(broken(&db)).unwrap(), garbage());
    }

    #[test]
    fn plaintext_backup_moves_keyfile_aside() {
        let src_dir = tempfile::tempdir().unwrap();
        let (x, _, _) = install(src_dir.path(), false);
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("nabungo.db");
        std::fs::write(&db, garbage()).unwrap();
        std::fs::write(keyfile_path(&db), b"key lama").unwrap();

        let r = restore_into_fresh(&db, &x, &no_att(dir.path()), &BackupSecret::None, FAST, NOW)
            .unwrap();
        assert!(!keyfile_path(&db).exists());
        let aside = broken(&keyfile_path(&db));
        assert_eq!(std::fs::read(&aside).unwrap(), b"key lama");
        assert!(r.moved_aside.contains(&aside));
        assert_eq!(db_state(&db).unwrap(), DbState::Plain);
    }

    #[test]
    fn password_restores_same_installation_backup() {
        let dir = tempfile::tempdir().unwrap();
        let (x, expected, _) = install(dir.path(), true);
        let db = dir.path().join("nabungo.db");
        let dek = unlock_key(&db, PW).unwrap();
        let keyfile = std::fs::read(keyfile_path(&db)).unwrap();
        break_db(&db);

        let secret = BackupSecret::Password(PW.into());
        let r = restore_into_fresh(&db, &x, &no_att(dir.path()), &secret, FAST, NOW).unwrap();
        assert_eq!(r.key, Some(dek));
        assert_eq!(std::fs::read(keyfile_path(&db)).unwrap(), keyfile);
        let key = unlock_key(&db, PW).unwrap();
        let l =
            Ledger::open_with_backups(&db, &dir.path().join("backups"), Some(&key), NOW).unwrap();
        assert_eq!(balances(&l), expected);
    }

    #[test]
    fn recovery_key_restores_foreign_backup_and_writes_keyfile() {
        let a = tempfile::tempdir().unwrap();
        let (x, expected, rk) = install(a.path(), true);
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("nabungo.db");
        std::fs::write(&db, garbage()).unwrap();
        std::fs::write(keyfile_path(&db), b"key lama").unwrap();

        let secret = BackupSecret::RecoveryKey {
            key: rk.unwrap(),
            new_password: NEW_PW.into(),
        };
        let r = restore_into_fresh(&db, &x, &no_att(dir.path()), &secret, FAST, NOW).unwrap();
        let key = unlock_key(&db, NEW_PW).unwrap();
        assert_eq!(r.key, Some(key.clone()));
        assert_eq!(
            std::fs::read(broken(&keyfile_path(&db))).unwrap(),
            b"key lama"
        );
        let l =
            Ledger::open_with_backups(&db, &dir.path().join("backups"), Some(&key), NOW).unwrap();
        assert_eq!(balances(&l), expected);
    }

    #[test]
    fn failed_recovery_leaves_files_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let (x, _, _) = install(dir.path(), true);
        let db = dir.path().join("nabungo.db");
        break_db(&db);
        let junk = dir.path().join("junk.db");
        std::fs::write(&junk, garbage()).unwrap();
        let other_rk = {
            let b = tempfile::tempdir().unwrap();
            install(b.path(), true).2.unwrap()
        };
        let before = listing(dir.path());

        type Case<'a> = (&'a Path, BackupSecret, fn(&AppError) -> bool);
        let cases: Vec<Case> = vec![
            (&x, BackupSecret::Password("salah-sekali".into()), |e| {
                matches!(e, AppError::WrongPassword)
            }),
            (
                &x,
                BackupSecret::RecoveryKey {
                    key: other_rk.clone(),
                    new_password: NEW_PW.into(),
                },
                |e| matches!(e, AppError::InvalidRecoveryKey),
            ),
            (
                &x,
                BackupSecret::RecoveryKey {
                    key: "bukan recovery key".into(),
                    new_password: NEW_PW.into(),
                },
                |e| matches!(e, AppError::InvalidRecoveryKey),
            ),
            (
                &x,
                BackupSecret::RecoveryKey {
                    key: other_rk.clone(),
                    new_password: "pendek".into(),
                },
                |e| matches!(e, AppError::PasswordTooShort),
            ),
            (&junk, BackupSecret::None, |e| {
                matches!(e, AppError::BackupKeyRequired)
            }),
            (&x, BackupSecret::None, |e| {
                matches!(e, AppError::BackupKeyRequired)
            }),
        ];
        for (src, secret, expected) in cases {
            let err = restore_into_fresh(&db, src, &no_att(dir.path()), &secret, FAST, NOW)
                .err()
                .expect("pemulihan harus gagal");
            assert!(expected(&err), "{err:?}");
            assert_eq!(listing(dir.path()), before);
            assert!(!sibling(&db, ".recover").exists());
        }
    }

    #[test]
    fn password_without_keyfile_is_keyfile_invalid() {
        let a = tempfile::tempdir().unwrap();
        let (x, _, _) = install(a.path(), true);
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("nabungo.db");
        std::fs::write(&db, garbage()).unwrap();
        let secret = BackupSecret::Password(PW.into());
        let err = restore_into_fresh(&db, &x, &no_att(dir.path()), &secret, FAST, NOW)
            .err()
            .unwrap();
        assert!(matches!(err, AppError::KeyFileInvalid), "{err:?}");
    }

    #[test]
    fn recovery_rejects_live_db_as_source() {
        let dir = tempfile::tempdir().unwrap();
        install(dir.path(), false);
        let db = dir.path().join("nabungo.db");
        for src in [
            db.clone(),
            sibling(&db, "-wal"),
            dir.path().join(".").join("nabungo.db"),
        ] {
            let err = restore_into_fresh(
                &db,
                &src,
                &no_att(dir.path()),
                &BackupSecret::None,
                FAST,
                NOW,
            )
            .err()
            .unwrap();
            assert!(matches!(err, AppError::BackupTargetInvalid), "{err:?}");
        }
    }

    #[test]
    fn start_fresh_moves_db_and_keyfile_aside() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("nabungo.db");
        std::fs::write(&db, garbage()).unwrap();
        std::fs::write(keyfile_path(&db), b"key lama").unwrap();

        let moved = start_fresh(&db, NOW).unwrap();
        assert!(!db.exists());
        assert!(!keyfile_path(&db).exists());
        assert_eq!(std::fs::read(broken(&db)).unwrap(), garbage());
        assert_eq!(
            std::fs::read(broken(&keyfile_path(&db))).unwrap(),
            b"key lama"
        );
        assert_eq!(moved.len(), 2);
    }
}
