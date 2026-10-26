//! Koneksi baca read-only ke file DB yang sama dengan [`Ledger`]: query baca berat (dasbor,
//! laporan) berjalan paralel di luar mutex ledger. WAL: pembaca melihat commit terakhir.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use nabungo_storage::DbKey;

use crate::{AppError, Ledger};

/// Koneksi idle yang disimpan; sisanya ditutup setelah dipakai.
const MAX_IDLE: usize = 4;

/// Pool koneksi baca; clone berbagi pool yang sama.
#[derive(Clone)]
pub struct ReadPool(Arc<Inner>);

struct Inner {
    path: PathBuf,
    key: Option<DbKey>,
    /// `true` setelah ditutup. Pembaca memegang read lock selama query, jadi `close` menunggu
    /// pembaca yang sedang berjalan sebelum file DB boleh diganti.
    closed: RwLock<bool>,
    /// Pembaca berupa `Ledger` tanpa pool, agar use case baca (`&self`) bisa dipakai apa adanya.
    idle: Mutex<Vec<Ledger>>,
}

impl ReadPool {
    fn new(path: &Path, key: Option<DbKey>) -> Self {
        Self(Arc::new(Inner {
            path: path.to_path_buf(),
            key,
            closed: RwLock::new(false),
            idle: Mutex::new(Vec::new()),
        }))
    }

    /// Jalankan `f` pada koneksi baca. `None` bila pool sudah ditutup atau koneksi baca gagal
    /// dibuka: pemanggil memakai koneksi tulis.
    pub fn with_reader<T, E>(
        &self,
        f: impl FnOnce(&Ledger) -> Result<T, E>,
    ) -> Option<Result<T, E>> {
        let closed = self.0.closed.read().unwrap_or_else(|e| e.into_inner());
        if *closed {
            return None;
        }
        let reader = match self.take() {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "gagal membuka koneksi baca");
                return None;
            }
        };
        let result = f(&reader);
        let mut idle = self.0.idle.lock().unwrap_or_else(|e| e.into_inner());
        if idle.len() < MAX_IDLE {
            idle.push(reader);
        }
        Some(result)
    }

    fn take(&self) -> Result<Ledger, AppError> {
        if let Some(r) = self.0.idle.lock().unwrap_or_else(|e| e.into_inner()).pop() {
            return Ok(r);
        }
        Ok(Ledger {
            conn: nabungo_storage::open_reader(&self.0.path, self.0.key.as_ref())?,
            path: Some(self.0.path.clone()),
            key: self.0.key.clone(),
            readers: None,
        })
    }

    /// Tutup semua koneksi baca setelah pembaca yang sedang berjalan selesai.
    fn close(&self) {
        let mut closed = self.0.closed.write().unwrap_or_else(|e| e.into_inner());
        *closed = true;
        self.0
            .idle
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
}

/// Pemilik pool di dalam `Ledger`: pool ditutup saat ledger ditutup atau dikunci, termasuk
/// clone yang sedang dipegang command.
pub(crate) struct Readers(ReadPool);

impl Readers {
    /// `None` untuk DB in-memory (tidak bisa dibuka koneksi kedua).
    pub(crate) fn for_file(path: Option<&Path>, key: Option<&DbKey>) -> Option<Self> {
        path.map(|p| Self(ReadPool::new(p, key.cloned())))
    }
}

impl Drop for Readers {
    fn drop(&mut self) {
        self.0.close();
    }
}

impl Ledger {
    /// Pool koneksi baca; `None` untuk DB in-memory dan untuk pembaca itu sendiri.
    pub fn read_pool(&self) -> Option<ReadPool> {
        self.readers.as_ref().map(|r| r.0.clone())
    }

    /// Jalankan `f` tanpa koneksi baca terbuka (file DB diganti atau ditimpa), lalu buka pool
    /// baru untuk file dan key sesudahnya, apa pun hasil `f`.
    pub(crate) fn without_readers<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        self.readers = None;
        let result = f(self);
        self.readers = Readers::for_file(self.path.as_deref(), self.key.as_ref());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::KdfParams;
    use crate::{AccountFields, TxnInput};
    use nabungo_core::AccountKind;
    use time::macros::date;

    fn file_ledger() -> (tempfile::TempDir, Ledger, uuid::Uuid) {
        let dir = tempfile::tempdir().unwrap();
        let mut ledger = Ledger::open(&dir.path().join("nabungo.db")).unwrap();
        let id = ledger
            .create_account(
                AccountKind::Bank,
                &AccountFields::new("BCA", 1_000_000, date!(2026 - 01 - 01)),
            )
            .unwrap();
        (dir, ledger, id)
    }

    fn reader_balance(ledger: &Ledger, id: uuid::Uuid) -> Option<i64> {
        ledger
            .read_pool()
            .unwrap()
            .with_reader(|r| r.balance(id))
            .map(Result::unwrap)
    }

    /// Use case yang dijalankan command lewat koneksi baca tidak boleh menulis.
    #[test]
    fn dashboard_reads_work_on_reader() {
        use crate::fixture::{FixtureSpec, seed};
        use crate::{CategoryKind, TxnQuery, local_today};

        let dir = tempfile::tempdir().unwrap();
        let mut ledger = Ledger::open(&dir.path().join("nabungo.db")).unwrap();
        let today = local_today();
        let spec = FixtureSpec {
            txns: 300,
            ..FixtureSpec::standard(today)
        };
        seed(&mut ledger, &spec).unwrap();
        let period = ledger.period_of(today).unwrap();
        let pool = ledger.read_pool().unwrap();
        let read = pool
            .with_reader(|r| -> Result<_, AppError> {
                Ok((
                    r.accounts_overview()?,
                    r.list_txns(&TxnQuery {
                        limit: 8,
                        ..TxnQuery::default()
                    })?,
                    r.budget_overview(period, today)?,
                    r.category_breakdown(period, CategoryKind::Expense)?,
                    r.allocation_overview()?,
                    r.allocation_history(period, 12)?,
                    r.cashflow_report(period, 12)?,
                    r.net_worth_history(period, 12)?,
                ))
            })
            .unwrap()
            .unwrap();
        assert_eq!(read.0, ledger.accounts_overview().unwrap());
        assert_eq!(read.7, ledger.net_worth_history(period, 12).unwrap());
    }

    #[test]
    fn restore_replaces_pool() {
        let (dir, mut ledger, id) = file_ledger();
        let backup = dir.path().join("manual.db");
        let att = dir.path().join("attachments");
        ledger.backup_to(&backup, &att).unwrap();
        let old = ledger.read_pool().unwrap();
        ledger
            .restore_file(
                &backup,
                None,
                &dir.path().join("backups"),
                &att,
                time::OffsetDateTime::now_utc(),
            )
            .unwrap();
        assert!(
            old.with_reader(|r| r.balance(id)).is_none(),
            "pool lama ditutup"
        );
        assert_eq!(reader_balance(&ledger, id), Some(1_000_000));
    }

    #[test]
    fn in_memory_has_no_pool() {
        assert!(Ledger::open_in_memory().unwrap().read_pool().is_none());
    }

    #[test]
    fn reader_sees_latest_commit() {
        let (_dir, mut ledger, id) = file_ledger();
        assert_eq!(reader_balance(&ledger, id), Some(1_000_000));
        let dompet = ledger
            .create_account(
                AccountKind::Cash,
                &AccountFields::new("Dompet", 0, date!(2026 - 01 - 01)),
            )
            .unwrap();
        ledger
            .record(&TxnInput::transfer(
                date!(2026 - 01 - 02),
                id,
                dompet,
                250_000,
            ))
            .unwrap();
        // Koneksi idle dari baca pertama dipakai ulang dan tetap melihat commit baru.
        assert_eq!(reader_balance(&ledger, id), Some(750_000));
    }

    #[test]
    fn reader_is_read_only() {
        let (_dir, ledger, _) = file_ledger();
        let written = ledger
            .read_pool()
            .unwrap()
            .with_reader(|r| r.conn.execute("DELETE FROM account", []))
            .unwrap();
        assert!(written.is_err());
    }

    #[test]
    fn dropping_ledger_closes_pool() {
        let (_dir, ledger, id) = file_ledger();
        let pool = ledger.read_pool().unwrap();
        drop(ledger);
        assert!(pool.with_reader(|r| r.balance(id)).is_none());
    }

    #[test]
    fn encryption_swap_replaces_pool() {
        let (dir, mut ledger, id) = file_ledger();
        let old = ledger.read_pool().unwrap();
        assert!(old.with_reader(|r| r.balance(id)).is_some());
        ledger
            .enable_encryption("rahasia-123", &dir.path().join("backups"), KdfParams::FAST)
            .unwrap();
        assert!(
            old.with_reader(|r| r.balance(id)).is_none(),
            "pool lama ditutup"
        );
        assert_eq!(reader_balance(&ledger, id), Some(1_000_000));
        assert!(
            ledger
                .read_pool()
                .unwrap()
                .with_reader(|r| Ok::<_, AppError>(r.is_encrypted()))
                .unwrap()
                .unwrap()
        );
    }
}
