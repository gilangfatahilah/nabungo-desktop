//! File database: buka tanpa migrasi, backup/restore via online backup API, inspeksi, integrity.
//!
//! Semua file DB (utama, tujuan backup, sumber restore) dibuka lewat modul ini; di sinilah key
//! SQLCipher akan dipasang (Fase 6b).

use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

use crate::{Result, StorageError, configure, latest_version, migrations};

/// Satu-satunya tempat koneksi ke file dibuat (tanpa PRAGMA).
fn open_raw(path: &Path, flags: OpenFlags) -> Result<Connection> {
    Ok(Connection::open_with_flags(path, flags)?)
}

/// Buka (atau buat) file DB dan set PRAGMA, tanpa migrasi.
pub fn open_file(path: &Path) -> Result<Connection> {
    let conn = open_raw(path, OpenFlags::default())?;
    configure(&conn)?;
    Ok(conn)
}

/// Buka file DB yang sudah ada hanya untuk dibaca; file tidak dibuat bila belum ada.
pub fn open_file_read_only(path: &Path) -> Result<Connection> {
    open_raw(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
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
/// (menimpa `dest` bila ada). Gagal → tidak ada file yang tertinggal.
pub fn backup_into(conn: &Connection, dest: &Path) -> Result<()> {
    let partial = sibling(dest, ".partial");
    remove_partial(&partial);
    let result = (|| {
        let mut dst = open_raw(&partial, OpenFlags::default())?;
        copy_all(&Backup::new(conn, &mut dst)?)?;
        if !quick_check(&dst)? {
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

/// Ganti isi `conn` dengan isi file `src` dalam satu langkah backup API (atomik di sisi tujuan).
pub fn restore_from(conn: &mut Connection, src: &Path) -> Result<()> {
    let src = open_file_read_only(src)?;
    copy_all(&Backup::new(&src, conn)?)?;
    Ok(())
}

/// Validasi file backup sebelum restore; mengembalikan versi skemanya.
pub fn inspect_backup(path: &Path) -> Result<usize> {
    let check = || -> Result<usize> {
        let conn = open_file_read_only(path)?;
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
        Err(StorageError::Sqlite(_)) => return Err(StorageError::InvalidBackup),
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
        let mut conn = open(&dir.path().join("main.db")).unwrap();
        put(&mut conn, "v");
        let dest = dir.path().join("b.db");
        backup_into(&conn, &dest).unwrap();
        let mut names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| !n.starts_with("main.db"))
            .collect();
        names.sort();
        assert_eq!(names, ["b.db"]);
        assert_eq!(get(&open(&dest).unwrap()).as_deref(), Some("v"));
    }

    #[test]
    fn backup_includes_committed_wal_frames() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("main.db");
        let mut conn = open(&main).unwrap();
        put(&mut conn, "wal");
        // Belum checkpoint: data masih di -wal.
        assert!(dir.path().join("main.db-wal").exists());
        let dest = dir.path().join("b.db");
        backup_into(&conn, &dest).unwrap();
        assert_eq!(
            get(&open_file_read_only(&dest).unwrap()).as_deref(),
            Some("wal")
        );
    }

    #[test]
    fn backup_failure_leaves_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("main.db")).unwrap();
        let dest = dir.path().join("missing").join("b.db");
        assert!(backup_into(&conn, &dest).is_err());
        assert!(!dest.exists());
        assert!(!dir.path().join("missing").join("b.db.partial").exists());
    }

    #[test]
    fn restore_replaces_contents() {
        let dir = tempfile::tempdir().unwrap();
        let mut a = open(&dir.path().join("a.db")).unwrap();
        put(&mut a, "a");
        let b_path = dir.path().join("b.db");
        let mut b = open(&b_path).unwrap();
        put(&mut b, "b");
        drop(b);
        restore_from(&mut a, &b_path).unwrap();
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
            inspect_backup(&text),
            Err(StorageError::InvalidBackup)
        ));

        let empty = dir.path().join("e.db");
        std::fs::write(&empty, b"").unwrap();
        assert!(matches!(
            inspect_backup(&empty),
            Err(StorageError::InvalidBackup)
        ));

        let missing = dir.path().join("nope.db");
        assert!(matches!(
            inspect_backup(&missing),
            Err(StorageError::InvalidBackup)
        ));
        assert!(!missing.exists());

        let foreign = dir.path().join("f.db");
        Connection::open(&foreign)
            .unwrap()
            .execute_batch("CREATE TABLE x (a)")
            .unwrap();
        assert!(matches!(
            inspect_backup(&foreign),
            Err(StorageError::InvalidBackup)
        ));

        let newer = dir.path().join("n.db");
        let conn = open(&newer).unwrap();
        conn.pragma_update(None, "user_version", latest_version() as i64 + 1)
            .unwrap();
        drop(conn);
        assert!(matches!(
            inspect_backup(&newer),
            Err(StorageError::BackupTooNew(v)) if v == latest_version() + 1
        ));

        let good = dir.path().join("g.db");
        drop(open(&good).unwrap());
        assert_eq!(inspect_backup(&good).unwrap(), latest_version());
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
        let mut conn = open_file(&dir.path().join("m.db")).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), 0);
        migrate_to(&mut conn, 3).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), 3);
        migrate(&mut conn).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), latest_version());
        assert_eq!(latest_version(), 5);
    }
}
