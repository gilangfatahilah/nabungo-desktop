//! File database: buka tanpa migrasi, backup/restore via online backup API, konversi
//! plaintext ↔ terenkripsi, inspeksi, integrity.
//!
//! Semua file DB (utama, tujuan backup, sumber restore) dibuka lewat modul ini; di sinilah key
//! SQLCipher dipasang.

use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};
use std::io::Read;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

use crate::key::DbKey;
use crate::{Result, StorageError, configure, latest_version, migrations};

/// 16 byte pertama file SQLite yang tidak terenkripsi.
const PLAIN_HEADER: &[u8; 16] = b"SQLite format 3\0";

/// `true` bila file ada dan 16 byte pertamanya bukan header SQLite plaintext. File yang tidak
/// ada, kosong, atau lebih pendek dari header dianggap tidak terenkripsi.
pub fn is_encrypted_file(path: &Path) -> std::io::Result<bool> {
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    let mut buf = [0u8; 16];
    let mut n = 0;
    while n < buf.len() {
        match f.read(&mut buf[n..])? {
            0 => break,
            k => n += k,
        }
    }
    Ok(n == buf.len() && &buf != PLAIN_HEADER)
}

/// Satu-satunya tempat koneksi ke file dibuat (tanpa PRAGMA selain key).
///
/// Tanpa key pada file terenkripsi → [`StorageError::KeyRequired`]; key yang tidak cocok
/// (termasuk key pada file plaintext) → [`StorageError::WrongKey`].
fn open_raw(path: &Path, flags: OpenFlags, key: Option<&DbKey>) -> Result<Connection> {
    if key.is_none() && is_encrypted_file(path)? {
        return Err(StorageError::KeyRequired);
    }
    let conn = Connection::open_with_flags(path, flags)?;
    if let Some(key) = key {
        conn.execute_batch(&Zeroizing::new(format!(
            "PRAGMA key = {};",
            key.sql_literal().as_str()
        )))?;
        // SQLCipher baru memverifikasi key saat halaman pertama dibaca.
        match conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        }) {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::NotADatabase =>
            {
                return Err(StorageError::WrongKey);
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(conn)
}

/// Buka (atau buat) file DB dan set PRAGMA, tanpa migrasi. `key` `Some` pada file baru
/// membuat file terenkripsi.
pub fn open_file(path: &Path, key: Option<&DbKey>) -> Result<Connection> {
    let conn = open_raw(path, OpenFlags::default(), key)?;
    configure(&conn)?;
    Ok(conn)
}

/// Buka file DB yang sudah ada hanya untuk dibaca; file tidak dibuat bila belum ada.
pub fn open_file_read_only(path: &Path, key: Option<&DbKey>) -> Result<Connection> {
    open_raw(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        key,
    )
}

pub fn migrate(conn: &mut Connection) -> Result<()> {
    migrations().to_latest(conn)?;
    Ok(())
}

/// Migrasi (naik/turun) ke versi tertentu; untuk test dan tool.
pub fn migrate_to(conn: &mut Connection, version: usize) -> Result<()> {
    migrations().to_version(conn, version)?;
    Ok(())
}

pub fn schema_version(conn: &Connection) -> Result<usize> {
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    usize::try_from(v).map_err(|_| StorageError::Corrupt(format!("user_version {v}")))
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn remove_partial(partial: &Path) {
    for p in [
        partial.to_path_buf(),
        sibling(partial, "-wal"),
        sibling(partial, "-shm"),
        sibling(partial, "-journal"),
    ] {
        let _ = std::fs::remove_file(p);
    }
}

/// Salin semua halaman dalam satu step: tujuan dikunci dan di-commit sekali (atomik).
fn copy_all(backup: &Backup<'_, '_>) -> Result<()> {
    match backup.step(-1)? {
        StepResult::Done => Ok(()),
        other => Err(StorageError::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some(format!("backup step: {other:?}")),
        ))),
    }
}

fn quick_check(conn: &Connection) -> Result<bool> {
    let first: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    Ok(first == "ok")
}

/// Salin isi `conn` ke `dest`: tulis ke `dest.partial`, `quick_check`, lalu rename
/// (menimpa `dest` bila ada). Gagal → tidak ada file yang tertinggal. `key` harus key DB
/// sumber: backup API tidak bisa menyalin lintas status enkripsi.
pub fn backup_into(conn: &Connection, dest: &Path, key: Option<&DbKey>) -> Result<()> {
    copy_into(conn, dest, key, true)
}

/// Seperti [`backup_into`] tanpa `quick_check`: salinan pengaman dari DB yang mungkin
/// rusak (pra-restore) tetap harus bisa dibuat.
pub fn backup_into_unverified(conn: &Connection, dest: &Path, key: Option<&DbKey>) -> Result<()> {
    copy_into(conn, dest, key, false)
}

fn copy_into(conn: &Connection, dest: &Path, key: Option<&DbKey>, verify: bool) -> Result<()> {
    let partial = sibling(dest, ".partial");
    remove_partial(&partial);
    let result = (|| {
        let mut dst = open_raw(&partial, OpenFlags::default(), key)?;
        copy_all(&Backup::new(conn, &mut dst)?)?;
        // Header ikut sumber (WAL); jadikan file tunggal agar bisa disalin/dibuka apa adanya.
        dst.pragma_update(None, "journal_mode", "DELETE")?;
        if verify && !quick_check(&dst)? {
            return Err(StorageError::Corrupt("backup failed quick_check".into()));
        }
        drop(dst);
        std::fs::rename(&partial, dest)?;
        Ok(())
    })();
    if result.is_err() {
        remove_partial(&partial);
    }
    result
}

/// Ganti isi `conn` (DB aktif dengan key `live_key`) dengan isi file `src` dalam satu langkah
/// backup API (atomik di sisi tujuan). Sumber terenkripsi dibuka dengan `live_key`; bila status
/// enkripsi sumber berbeda dari DB aktif, sumber lebih dulu dikonversi ke file sementara di
/// samping DB aktif.
pub fn restore_from(conn: &mut Connection, src: &Path, live_key: Option<&DbKey>) -> Result<()> {
    let src_encrypted = is_encrypted_file(src)?;
    if src_encrypted == live_key.is_some() {
        let src = open_file_read_only(src, live_key)?;
        return copy_all(&Backup::new(&src, conn)?);
    }
    let base = conn
        .path()
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| src.to_path_buf());
    let tmp = sibling(&base, ".restore-partial");
    remove_partial(&tmp);
    let src_key = if src_encrypted { live_key } else { None };
    let result = convert_file(src, src_key, &tmp, live_key).and_then(|()| {
        let tmp_conn = open_file_read_only(&tmp, live_key)?;
        copy_all(&Backup::new(&tmp_conn, conn)?)
    });
    remove_partial(&tmp);
    result
}

/// Salin `src` ke `dest` dengan status enkripsi baru lewat `sqlcipher_export` (backup API tidak
/// bisa lintas enkripsi). Ditulis ke `dest.partial`, diverifikasi, lalu di-rename.
pub fn convert_file(
    src: &Path,
    src_key: Option<&DbKey>,
    dest: &Path,
    dest_key: Option<&DbKey>,
) -> Result<()> {
    let partial = sibling(dest, ".partial");
    remove_partial(&partial);
    let result = (|| {
        // DB yang di-ATTACH mewarisi flag koneksi utama, jadi CREATE wajib agar `.partial`
        // bisa dibuat; sumber yang tidak ada dicek manual supaya tidak ikut dibuat.
        if !src.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} not found", src.display()),
            )
            .into());
        }
        let conn = open_raw(
            src,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            src_key,
        )?;
        let dest_literal = match dest_key {
            Some(k) => k.sql_literal(),
            None => Zeroizing::new("''".to_owned()),
        };
        conn.execute(
            &Zeroizing::new(format!(
                "ATTACH DATABASE ?1 AS nabungo_dest KEY {}",
                dest_literal.as_str()
            )),
            [partial.to_string_lossy()],
        )?;
        conn.execute_batch("SELECT sqlcipher_export('nabungo_dest');")?;
        // `sqlcipher_export` tidak menjamin `user_version` ikut tersalin.
        let version = schema_version(&conn)?;
        conn.execute_batch(&format!(
            "PRAGMA nabungo_dest.user_version = {version}; DETACH DATABASE nabungo_dest;"
        ))?;
        drop(conn);
        let check = open_file_read_only(&partial, dest_key)?;
        if !quick_check(&check)? {
            return Err(StorageError::Corrupt(
                "converted file failed quick_check".into(),
            ));
        }
        drop(check);
        std::fs::rename(&partial, dest)?;
        Ok(())
    })();
    if result.is_err() {
        remove_partial(&partial);
    }
    result
}

/// Validasi file backup sebelum restore; mengembalikan versi skemanya. `key` dipakai hanya bila
/// file terenkripsi.
pub fn inspect_backup(path: &Path, key: Option<&DbKey>) -> Result<usize> {
    let check = || -> Result<usize> {
        let key = if is_encrypted_file(path)? { key } else { None };
        let conn = open_file_read_only(path, key)?;
        if !quick_check(&conn)? {
            return Err(StorageError::InvalidBackup);
        }
        let has_posting: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = 'posting'",
            [],
            |r| r.get(0),
        )?;
        if has_posting != 1 {
            return Err(StorageError::InvalidBackup);
        }
        schema_version(&conn)
    };
    let version = match check() {
        Ok(v) => v,
        Err(StorageError::Sqlite(_) | StorageError::KeyRequired | StorageError::WrongKey) => {
            return Err(StorageError::InvalidBackup);
        }
        Err(e) => return Err(e),
    };
    if version > latest_version() {
        return Err(StorageError::BackupTooNew(version));
    }
    Ok(version)
}

/// Masalah yang dilaporkan `PRAGMA integrity_check`; kosong bila database sehat.
pub fn integrity_check(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("PRAGMA integrity_check")?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(if rows == ["ok"] { Vec::new() } else { rows })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::DbKey;
    use crate::{open, open_in_memory, setting};

    fn put(conn: &mut Connection, value: &str) {
        let tx = conn.transaction().unwrap();
        setting::set_setting(&tx, "k", value).unwrap();
        tx.commit().unwrap();
    }

    fn get(conn: &Connection) -> Option<String> {
        setting::get_setting(conn, "k").unwrap()
    }

    #[test]
    fn backup_roundtrip_preserves_rows() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("main.db"), None).unwrap();
        put(&mut conn, "v");
        let dest = dir.path().join("b.db");
        backup_into(&conn, &dest, None).unwrap();
        let mut names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| !n.starts_with("main.db"))
            .collect();
        names.sort();
        assert_eq!(names, ["b.db"]);
        assert_eq!(get(&open(&dest, None).unwrap()).as_deref(), Some("v"));
    }

    #[test]
    fn backup_includes_committed_wal_frames() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("main.db");
        let mut conn = open(&main, None).unwrap();
        put(&mut conn, "wal");
        // Belum checkpoint: data masih di -wal.
        assert!(dir.path().join("main.db-wal").exists());
        let dest = dir.path().join("b.db");
        backup_into(&conn, &dest, None).unwrap();
        let b = open_file_read_only(&dest, None).unwrap();
        assert_eq!(get(&b).as_deref(), Some("wal"));
        // File backup mandiri: mode rollback, tidak menumbuhkan -wal/-shm saat dibuka.
        let mode: String = b
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "delete");
        drop(b);
        assert!(!dir.path().join("b.db-wal").exists());
        assert!(!dir.path().join("b.db-shm").exists());
    }

    #[test]
    fn backup_failure_leaves_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("main.db"), None).unwrap();
        let dest = dir.path().join("missing").join("b.db");
        assert!(backup_into(&conn, &dest, None).is_err());
        assert!(!dest.exists());
        assert!(!dir.path().join("missing").join("b.db.partial").exists());
    }

    #[test]
    fn restore_replaces_contents() {
        let dir = tempfile::tempdir().unwrap();
        let mut a = open(&dir.path().join("a.db"), None).unwrap();
        put(&mut a, "a");
        let b_path = dir.path().join("b.db");
        let mut b = open(&b_path, None).unwrap();
        put(&mut b, "b");
        drop(b);
        restore_from(&mut a, &b_path, None).unwrap();
        assert_eq!(get(&a).as_deref(), Some("b"));
    }

    #[test]
    fn inspect_rejects_non_backups() {
        let dir = tempfile::tempdir().unwrap();
        let text = dir.path().join("t.db");
        std::fs::write(
            &text,
            "bukan database sama sekali, cukup panjang untuk header",
        )
        .unwrap();
        assert!(matches!(
            inspect_backup(&text, None),
            Err(StorageError::InvalidBackup)
        ));

        let empty = dir.path().join("e.db");
        std::fs::write(&empty, b"").unwrap();
        assert!(matches!(
            inspect_backup(&empty, None),
            Err(StorageError::InvalidBackup)
        ));

        let missing = dir.path().join("nope.db");
        assert!(matches!(
            inspect_backup(&missing, None),
            Err(StorageError::InvalidBackup)
        ));
        assert!(!missing.exists());

        let foreign = dir.path().join("f.db");
        Connection::open(&foreign)
            .unwrap()
            .execute_batch("CREATE TABLE x (a)")
            .unwrap();
        assert!(matches!(
            inspect_backup(&foreign, None),
            Err(StorageError::InvalidBackup)
        ));

        let newer = dir.path().join("n.db");
        let conn = open(&newer, None).unwrap();
        conn.pragma_update(None, "user_version", latest_version() as i64 + 1)
            .unwrap();
        drop(conn);
        assert!(matches!(
            inspect_backup(&newer, None),
            Err(StorageError::BackupTooNew(v)) if v == latest_version() + 1
        ));

        let good = dir.path().join("g.db");
        drop(open(&good, None).unwrap());
        assert_eq!(inspect_backup(&good, None).unwrap(), latest_version());
    }

    #[test]
    fn integrity_check_ok_on_fresh_db() {
        assert!(
            integrity_check(&open_in_memory().unwrap())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn migrate_to_then_migrate_reaches_latest() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open_file(&dir.path().join("m.db"), None).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), 0);
        migrate_to(&mut conn, 3).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), 3);
        migrate(&mut conn).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), latest_version());
        assert_eq!(latest_version(), 8);
    }

    fn key(b: u8) -> DbKey {
        DbKey::from_bytes([b; 32])
    }

    #[test]
    fn plaintext_file_is_not_encrypted() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("p.db");
        drop(open(&p, None).unwrap());
        assert!(!is_encrypted_file(&p).unwrap());
    }

    #[test]
    fn missing_or_empty_file_is_not_encrypted() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_encrypted_file(&dir.path().join("nope.db")).unwrap());
        let empty = dir.path().join("e.db");
        std::fs::write(&empty, b"").unwrap();
        assert!(!is_encrypted_file(&empty).unwrap());
    }

    #[test]
    fn keyed_file_roundtrip_and_is_encrypted() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("k.db");
        let mut conn = open(&p, Some(&key(1))).unwrap();
        put(&mut conn, "rahasia");
        drop(conn);
        assert!(is_encrypted_file(&p).unwrap());
        let raw = std::fs::read(&p).unwrap();
        assert!(!raw.windows(7).any(|w| w == b"rahasia"));
        assert_eq!(
            get(&open(&p, Some(&key(1))).unwrap()).as_deref(),
            Some("rahasia")
        );
    }

    #[test]
    fn encrypted_without_key_is_key_required() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("k.db");
        drop(open(&p, Some(&key(1))).unwrap());
        assert!(matches!(
            open_file(&p, None),
            Err(StorageError::KeyRequired)
        ));
        assert!(matches!(
            open_file_read_only(&p, None),
            Err(StorageError::KeyRequired)
        ));
    }

    #[test]
    fn encrypted_with_wrong_key_is_wrong_key() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("k.db");
        drop(open(&p, Some(&key(1))).unwrap());
        assert!(matches!(
            open_file(&p, Some(&key(2))),
            Err(StorageError::WrongKey)
        ));
        assert!(matches!(
            open_file_read_only(&p, Some(&key(2))),
            Err(StorageError::WrongKey)
        ));
    }

    #[test]
    fn convert_plain_to_encrypted_and_back() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("p.db");
        let mut conn = open(&plain, None).unwrap();
        put(&mut conn, "v");
        drop(conn);
        let enc = dir.path().join("e.db");
        convert_file(&plain, None, &enc, Some(&key(3))).unwrap();
        assert!(is_encrypted_file(&enc).unwrap());
        assert!(!sibling(&enc, ".partial").exists());
        let c = open_file(&enc, Some(&key(3))).unwrap();
        assert_eq!(get(&c).as_deref(), Some("v"));
        assert_eq!(schema_version(&c).unwrap(), latest_version());
        drop(c);
        let back = dir.path().join("b.db");
        convert_file(&enc, Some(&key(3)), &back, None).unwrap();
        assert!(!is_encrypted_file(&back).unwrap());
        let c = open_file(&back, None).unwrap();
        assert_eq!(get(&c).as_deref(), Some("v"));
        assert_eq!(schema_version(&c).unwrap(), latest_version());
    }

    #[test]
    fn backup_of_encrypted_db_is_encrypted() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("m.db"), Some(&key(4))).unwrap();
        put(&mut conn, "v");
        let dest = dir.path().join("b.db");
        backup_into(&conn, &dest, Some(&key(4))).unwrap();
        assert!(is_encrypted_file(&dest).unwrap());
        assert_eq!(
            inspect_backup(&dest, Some(&key(4))).unwrap(),
            latest_version()
        );
        assert!(matches!(
            inspect_backup(&dest, None),
            Err(StorageError::InvalidBackup)
        ));
    }

    #[test]
    fn restore_plain_backup_into_encrypted_live() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("plain.db");
        let mut s = open(&src, None).unwrap();
        put(&mut s, "dari-plain");
        drop(s);
        let live_path = dir.path().join("live.db");
        let mut live = open(&live_path, Some(&key(5))).unwrap();
        put(&mut live, "lama");
        restore_from(&mut live, &src, Some(&key(5))).unwrap();
        assert_eq!(get(&live).as_deref(), Some("dari-plain"));
        drop(live);
        assert!(is_encrypted_file(&live_path).unwrap());
        assert!(!sibling(&src, ".restore-partial").exists());
        assert_eq!(
            get(&open(&live_path, Some(&key(5))).unwrap()).as_deref(),
            Some("dari-plain")
        );
    }

    #[test]
    fn restore_encrypted_backup_into_plain_live() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("enc.db");
        let mut s = open(&src, Some(&key(6))).unwrap();
        put(&mut s, "dari-enc");
        drop(s);
        let live_path = dir.path().join("live.db");
        let mut live = open(&live_path, None).unwrap();
        // Sumber terenkripsi dengan key yang tidak dimiliki live → tidak bisa dibaca.
        assert!(restore_from(&mut live, &src, None).is_err());
        assert_eq!(get(&live), None);
    }
}
