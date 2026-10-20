//! Lapisan penyimpanan SQLite: koneksi, PRAGMA, migrasi, repository.

use std::path::Path;

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

pub mod account;
pub mod account_group;
pub mod allocation;
pub mod budget;
pub mod category;
mod file;
pub mod payee;
pub mod report;
pub mod setting;
pub mod tag;
pub mod txn;
pub mod txn_list;

pub use file::{
    backup_into, inspect_backup, integrity_check, migrate, migrate_to, open_file,
    open_file_read_only, restore_from, schema_version,
};
pub use rusqlite;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Migration(#[from] rusqlite_migration::Error),
    #[error("corrupt data: {0}")]
    Corrupt(String),
    #[error("duplicate name")]
    Duplicate,
    #[error("not a valid backup")]
    InvalidBackup,
    #[error("backup schema version {0} is newer than this app")]
    BackupTooNew(usize),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Pelanggaran indeks UNIQUE → [`StorageError::Duplicate`]; error lain apa adanya.
pub(crate) fn map_unique(e: rusqlite::Error) -> StorageError {
    match &e {
        rusqlite::Error::SqliteFailure(f, _)
            if f.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
        {
            StorageError::Duplicate
        }
        _ => StorageError::Sqlite(e),
    }
}

/// Pola `LIKE ... ESCAPE '\\'` untuk substring literal: `%`, `_`, `\\` di-escape.
pub(crate) fn like_contains(query: &str) -> String {
    let mut out = String::with_capacity(query.len() + 2);
    out.push('%');
    for c in query.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

fn parse_uuid(table: &str, s: &str) -> Result<uuid::Uuid> {
    uuid::Uuid::parse_str(s).map_err(|e| StorageError::Corrupt(format!("{table} id {s:?}: {e}")))
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// Hasil lookup entitas yang bisa diarsip (akun, kategori).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lookup<T> {
    pub value: T,
    pub archived: bool,
}

/// Timestamp audit ISO-8601 UTC, lebar tetap dengan presisi milidetik
/// (`2026-10-16T10:00:00.000Z`) supaya urutan teks = urutan waktu.
pub fn now_utc() -> String {
    format_utc(time::OffsetDateTime::now_utc())
}

fn format_utc(t: time::OffsetDateTime) -> String {
    let t = t.to_offset(time::UtcOffset::UTC);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second(),
        t.millisecond()
    )
}

/// Daftar migrasi ber-versi (`PRAGMA user_version`). Hanya boleh ditambah di akhir;
/// migrasi yang sudah dirilis tidak boleh diubah.
pub fn migrations() -> Migrations<'static> {
    Migrations::new(MIGRATIONS.iter().map(|sql| M::up(sql)).collect())
}

const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_init.sql"),
    include_str!("../migrations/0002_unique_root_category.sql"),
    include_str!("../migrations/0003_catalog_indexes.sql"),
    include_str!("../migrations/0004_txn_indexes.sql"),
    include_str!("../migrations/0005_allocation_names.sql"),
];

/// Versi skema terbaru (`user_version` setelah semua migrasi).
pub fn latest_version() -> usize {
    MIGRATIONS.len()
}

fn configure(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(())
}

/// Buka (atau buat) database di `path`, set PRAGMA, lalu jalankan migrasi.
pub fn open(path: &Path) -> Result<Connection> {
    let mut conn = open_file(path)?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// Database in-memory yang sudah dimigrasi penuh; dipakai di test.
pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_are_fixed_width_and_sort_chronologically() {
        use time::macros::datetime;
        let whole = format_utc(datetime!(2026-10-16 10:00:00 UTC));
        let later = format_utc(datetime!(2026-10-16 10:00:00.5 UTC));
        assert_eq!(whole, "2026-10-16T10:00:00.000Z");
        assert_eq!(later, "2026-10-16T10:00:00.500Z");
        assert!(whole < later);
        assert_eq!(
            format_utc(datetime!(2026-10-16 17:00:00 +07:00)),
            "2026-10-16T10:00:00.000Z"
        );
        assert_eq!(now_utc().len(), whole.len());
    }

    #[test]
    fn migrations_are_valid() {
        migrations().validate().unwrap();
    }

    #[test]
    fn foreign_keys_enabled() {
        let conn = open_in_memory().unwrap();
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn schema_has_core_tables() {
        let conn = open_in_memory().unwrap();
        for table in [
            "account",
            "txn",
            "posting",
            "category",
            "budget",
            "allocation_bucket",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "missing table {table}");
        }
    }

    #[test]
    fn migration_0003_indexes_exist() {
        let conn = open_in_memory().unwrap();
        for index in [
            "account_group_name_nocase",
            "payee_name_nocase",
            "category_child_name_nocase",
            "category_root_name_nocase",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE type = 'index' AND name = ?1",
                    [index],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "missing index {index}");
        }
    }

    #[test]
    fn migration_0005_indexes_exist() {
        let conn = open_in_memory().unwrap();
        for index in [
            "allocation_bucket_root_name_nocase",
            "allocation_bucket_child_name_nocase",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE type = 'index' AND name = ?1",
                    [index],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "missing index {index}");
        }
    }

    #[test]
    fn idr_is_seeded() {
        let conn = open_in_memory().unwrap();
        let decimals: i64 = conn
            .query_row(
                "SELECT decimals FROM currency WHERE code = 'IDR'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(decimals, 0);
    }
}
