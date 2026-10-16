//! Lapisan penyimpanan SQLite: koneksi, PRAGMA, migrasi, repository.

use std::path::Path;

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

pub mod account;
pub mod category;
pub mod setting;
pub mod txn;

pub use rusqlite;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Migration(#[from] rusqlite_migration::Error),
    #[error("corrupt data: {0}")]
    Corrupt(String),
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
    Migrations::new(vec![
        M::up(include_str!("../migrations/0001_init.sql")),
        M::up(include_str!("../migrations/0002_unique_root_category.sql")),
    ])
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
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrations().to_latest(&mut conn)?;
    Ok(conn)
}

/// Database in-memory yang sudah dimigrasi penuh; dipakai di test.
pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrations().to_latest(&mut conn)?;
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
