//! Use case pencatatan: resolve input berbasis ID → validasi domain → simpan atomik.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use nabungo_core::recurrence::RecurrenceError;
use nabungo_core::{
    AllocationError, BudgetError, CatalogError, MoneyError, PeriodError, PostingDraft, TxnDraft,
    TxnError, TxnKind,
};
use nabungo_storage::account;
use nabungo_storage::rusqlite::{self, Connection};
use nabungo_storage::{DbKey, Lookup, StorageError, category, now_utc, txn};
use time::Date;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Storage(StorageError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Txn(#[from] TxnError),
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0} is archived")]
    Archived(&'static str),
    /// Saldo awal hanya diatur lewat form akun, tidak lewat `record`.
    #[error("opening balance can only be set from the account form")]
    OpeningNotAllowed,
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error(transparent)]
    Money(#[from] MoneyError),
    /// Akun/kategori masih dipakai; tawarkan arsip/merge.
    #[error("{0} is in use")]
    InUse(&'static str),
    #[error("{0} name already exists")]
    Duplicate(&'static str),
    #[error("onboarding already completed")]
    AlreadyOnboarded,
    #[error(transparent)]
    Budget(#[from] BudgetError),
    #[error(transparent)]
    Period(#[from] PeriodError),
    /// Nilai valuation sama dengan saldo pada tanggal itu.
    #[error("valuation equals the current balance")]
    ValuationUnchanged,
    /// Valuation/adjustment tidak diedit lewat form; hapus lalu catat ulang.
    #[error("transaction can't be edited with the form")]
    NotEditable,
    #[error(transparent)]
    Allocation(#[from] AllocationError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// File bukan backup Nabungo yang utuh.
    #[error("file is not a valid Nabungo backup")]
    InvalidBackup,
    /// Backup dibuat oleh versi aplikasi yang lebih baru.
    #[error("backup was made by a newer app version")]
    BackupTooNew,
    /// Backup terenkripsi dengan key lain (atau bukan file SQLite): butuh recovery key-nya.
    #[error("backup needs its recovery key")]
    BackupKeyRequired,
    /// DB aktif dibuat oleh versi aplikasi yang lebih baru.
    #[error("database was made by a newer app version")]
    DatabaseTooNew,
    /// Tujuan/sumber backup adalah DB yang sedang dipakai.
    #[error("backup target is the live database")]
    BackupTargetInvalid,
    /// Database terkunci: belum dibuka dengan password/recovery key.
    #[error("database is locked")]
    Locked,
    #[error("wrong password")]
    WrongPassword,
    #[error("invalid recovery key")]
    InvalidRecoveryKey,
    #[error("password is too short")]
    PasswordTooShort,
    /// `nabungo.key` rusak atau versinya tidak dikenal.
    #[error("key file is invalid")]
    KeyFileInvalid,
    /// Enable saat sudah terenkripsi / disable saat belum.
    #[error("encryption is already in that state")]
    EncryptionState,
    #[error("invalid auto-lock duration")]
    InvalidAutoLock,
    #[error("invalid font scale")]
    InvalidFontScale,
    /// Key sudah terverifikasi (key file terbuka) tapi file DB tidak cocok atau rusak.
    #[error("database file cannot be read with this key")]
    DatabaseUnreadable,
    /// Lampiran hanya gambar JPEG/PNG/WebP atau PDF.
    #[error("unsupported attachment type")]
    AttachmentType,
    #[error("attachment is too large")]
    AttachmentTooLarge,
    /// Baris lampiran ada, tapi filenya tidak ada di folder data.
    #[error("attachment file is missing")]
    AttachmentMissing,
    /// Kejadian terjadwal sudah ditangani (bukan `next_date`) atau jadwal sudah selesai.
    #[error("scheduled occurrence was already handled")]
    OccurrenceStale,
    #[error(transparent)]
    Recurrence(#[from] RecurrenceError),
    /// Capture email sudah dikonfirmasi/diabaikan.
    #[error("email capture was already processed")]
    CaptureNotPending,
    /// OS keychain (Secret Service / Credential Manager) tidak bisa dipakai.
    #[error("system keychain unavailable")]
    KeychainUnavailable,
}

impl From<StorageError> for AppError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::WrongKey => AppError::WrongPassword,
            StorageError::KeyRequired => AppError::Locked,
            e => AppError::Storage(e),
        }
    }
}

impl AppError {
    /// `StorageError::Duplicate` → `AppError::Duplicate(entity)`; selain itu apa adanya.
    pub(crate) fn duplicate_as(entity: &'static str) -> impl Fn(StorageError) -> AppError {
        move |e| match e {
            StorageError::Duplicate => AppError::Duplicate(entity),
            e => e.into(),
        }
    }

    /// Kesalahan isi file backup → varian khusus; selain itu apa adanya.
    pub(crate) fn from_backup(e: StorageError) -> AppError {
        match e {
            StorageError::InvalidBackup => AppError::InvalidBackup,
            StorageError::BackupKeyRequired => AppError::BackupKeyRequired,
            StorageError::BackupTooNew(_) => AppError::BackupTooNew,
            e => e.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostingInput {
    pub account_id: Uuid,
    pub category_id: Option<Uuid>,
    pub amount: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnInput {
    pub kind: TxnKind,
    pub date: Date,
    /// Nama payee bebas; kosong = tanpa payee. Dicocokkan tanpa membedakan huruf besar/kecil.
    pub payee: Option<String>,
    pub note: Option<String>,
    pub postings: Vec<PostingInput>,
    /// Nama tag; dinormalisasi (trim, tanpa `#` di depan) dan di-dedupe saat disimpan.
    pub tags: Vec<String>,
}

impl TxnInput {
    pub fn single(
        kind: TxnKind,
        date: Date,
        account_id: Uuid,
        category_id: Option<Uuid>,
        amount: i64,
    ) -> Self {
        Self {
            kind,
            date,
            payee: None,
            note: None,
            tags: vec![],
            postings: vec![PostingInput {
                account_id,
                category_id,
                amount,
                memo: None,
            }],
        }
    }

    /// `amount` positif: keluar dari `from`, masuk ke `to`.
    pub fn transfer(date: Date, from: Uuid, to: Uuid, amount: i64) -> Self {
        // saturating: i64::MIN tidak punya negasi; hasilnya tetap ditolak `AmountTooLarge`.
        Self {
            kind: TxnKind::Transfer,
            date,
            payee: None,
            note: None,
            tags: vec![],
            postings: vec![
                PostingInput {
                    account_id: from,
                    category_id: None,
                    amount: amount.saturating_neg(),
                    memo: None,
                },
                PostingInput {
                    account_id: to,
                    category_id: None,
                    amount,
                    memo: None,
                },
            ],
        }
    }
}

pub struct Ledger {
    pub(crate) conn: Connection,
    /// Path file DB; `None` untuk DB in-memory.
    pub(crate) path: Option<PathBuf>,
    /// Key SQLCipher bila DB terenkripsi.
    pub(crate) key: Option<DbKey>,
    /// Koneksi baca; `None` untuk DB in-memory dan untuk pembaca itu sendiri.
    pub(crate) readers: Option<crate::readers::Readers>,
}

impl Ledger {
    /// Buka + migrasi tanpa backup pra-migrasi (test & alat).
    pub fn open(path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            conn: nabungo_storage::open(path, None)?,
            path: Some(path.to_path_buf()),
            key: None,
            readers: crate::readers::Readers::for_file(Some(path), None),
        })
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        Ok(Self {
            conn: nabungo_storage::open_in_memory()?,
            path: None,
            key: None,
            readers: None,
        })
    }

    pub fn record(&mut self, input: &TxnInput) -> Result<Uuid, AppError> {
        if input.kind == TxnKind::Opening {
            return Err(AppError::OpeningNotAllowed);
        }
        let tx = self.conn.transaction()?;
        let id = crate::transactions::write_txn(&tx, None, input, &HashSet::new())?;
        tx.commit()?;
        Ok(id)
    }

    /// `NotFound` bila ID tidak dikenal atau transaksi sudah terhapus.
    pub fn delete_txn(&mut self, id: Uuid) -> Result<(), AppError> {
        self.change_deleted(id, txn::soft_delete)
    }

    /// `NotFound` bila ID tidak dikenal atau transaksi tidak sedang terhapus.
    pub fn undo_delete_txn(&mut self, id: Uuid) -> Result<(), AppError> {
        self.change_deleted(id, txn::restore)
    }

    pub fn balance(&self, account_id: Uuid) -> Result<i64, AppError> {
        Ok(txn::account_balance(&self.conn, account_id)?)
    }

    fn change_deleted(
        &mut self,
        id: Uuid,
        op: fn(&rusqlite::Transaction, Uuid, &str) -> nabungo_storage::Result<bool>,
    ) -> Result<(), AppError> {
        let now = now_utc();
        let tx = self.conn.transaction()?;
        if !op(&tx, id, &now)? {
            return Err(AppError::NotFound("transaction"));
        }
        tx.commit()?;
        Ok(())
    }
}

/// `archived_ok`: ID akun/kategori yang boleh sudah diarsip (dipakai versi lama transaksi
/// yang sedang diedit).
pub(crate) fn resolve(
    conn: &Connection,
    input: &TxnInput,
    archived_ok: &HashSet<Uuid>,
) -> Result<TxnDraft, AppError> {
    let postings = input
        .postings
        .iter()
        .map(|p| {
            let account = usable(
                account::get_account_ref(conn, p.account_id)?,
                archived_ok.contains(&p.account_id),
                "account",
            )?;
            let category = match p.category_id {
                Some(id) => Some(usable(
                    category::get_category_ref(conn, id)?,
                    archived_ok.contains(&id),
                    "category",
                )?),
                None => None,
            };
            Ok(PostingDraft {
                account,
                category,
                amount: p.amount,
                memo: p.memo.clone(),
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(TxnDraft {
        kind: input.kind,
        date: input.date,
        note: input.note.clone(),
        postings,
    })
}

fn usable<T>(
    found: Option<Lookup<T>>,
    archived_ok: bool,
    what: &'static str,
) -> Result<T, AppError> {
    match found {
        Some(l) if archived_ok => Ok(l.value),
        found => active(found, what),
    }
}

/// Transaksi baru tidak boleh menyentuh akun/kategori yang sudah diarsip.
pub(crate) fn active<T>(found: Option<Lookup<T>>, what: &'static str) -> Result<T, AppError> {
    match found {
        None => Err(AppError::NotFound(what)),
        Some(l) if l.archived => Err(AppError::Archived(what)),
        Some(l) => Ok(l.value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountFields, CategoryFields};
    use nabungo_core::{AccountKind, CategoryKind};
    use time::macros::date;

    const D: Date = date!(2026 - 10 - 12);

    fn bank(ledger: &mut Ledger, opening: i64) -> Uuid {
        ledger
            .create_account(AccountKind::Bank, &AccountFields::new("BCA", opening, D))
            .unwrap()
    }

    fn txn_count(ledger: &Ledger) -> i64 {
        ledger
            .conn
            .query_row("SELECT count(*) FROM txn", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn opening_balance_creates_opening_txn() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = bank(&mut ledger, 1_000_000);
        assert_eq!(ledger.balance(acc).unwrap(), 1_000_000);
        assert_eq!(txn_count(&ledger), 1);
    }

    #[test]
    fn zero_opening_balance_creates_no_txn() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = bank(&mut ledger, 0);
        assert_eq!(ledger.balance(acc).unwrap(), 0);
        assert_eq!(txn_count(&ledger), 0);
    }

    #[test]
    fn invalid_txn_is_rejected_and_not_stored() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = bank(&mut ledger, 0);
        let err = ledger
            .record(&TxnInput::single(TxnKind::Expense, D, acc, None, -5_000))
            .unwrap_err();
        assert!(matches!(err, AppError::Txn(TxnError::MissingCategory)));
        assert_eq!(txn_count(&ledger), 0);
    }

    #[test]
    fn unknown_account_is_not_found() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let err = ledger
            .record(&TxnInput::single(
                TxnKind::Adjustment,
                D,
                Uuid::now_v7(),
                None,
                1,
            ))
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound("account")));
    }

    #[test]
    fn delete_and_undo() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = bank(&mut ledger, 0);
        let food = ledger
            .create_category(CategoryKind::Expense, &CategoryFields::named("Makan"))
            .unwrap();
        let id = ledger
            .record(&TxnInput::single(
                TxnKind::Expense,
                D,
                acc,
                Some(food),
                -25_000,
            ))
            .unwrap();
        ledger.delete_txn(id).unwrap();
        assert_eq!(ledger.balance(acc).unwrap(), 0);
        ledger.undo_delete_txn(id).unwrap();
        assert_eq!(ledger.balance(acc).unwrap(), -25_000);
        assert!(matches!(
            ledger.delete_txn(Uuid::now_v7()),
            Err(AppError::NotFound("transaction"))
        ));
        assert!(matches!(
            ledger.undo_delete_txn(id),
            Err(AppError::NotFound("transaction"))
        ));
    }

    #[test]
    fn transfer_of_i64_min_is_rejected_not_panicking() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let (a, b) = (bank(&mut ledger, 0), bank(&mut ledger, 0));
        let err = ledger
            .record(&TxnInput::transfer(D, a, b, i64::MIN))
            .unwrap_err();
        assert!(matches!(err, AppError::Txn(TxnError::AmountTooLarge)));
    }

    #[test]
    fn record_rejects_opening() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = bank(&mut ledger, 1_000_000);
        let err = ledger
            .record(&TxnInput::single(TxnKind::Opening, D, acc, None, 5))
            .unwrap_err();
        assert!(matches!(err, AppError::OpeningNotAllowed));
        assert_eq!(txn_count(&ledger), 1);
    }

    #[test]
    fn archived_account_and_category_reject_new_txn() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = bank(&mut ledger, 0);
        let food = ledger
            .create_category(CategoryKind::Expense, &CategoryFields::named("Makan"))
            .unwrap();
        ledger
            .conn
            .execute(
                "UPDATE category SET archived_at = '2026-10-16T00:00:00.000Z'",
                [],
            )
            .unwrap();
        let expense = TxnInput::single(TxnKind::Expense, D, acc, Some(food), -1_000);
        assert!(matches!(
            ledger.record(&expense),
            Err(AppError::Archived("category"))
        ));
        ledger
            .conn
            .execute(
                "UPDATE account SET archived_at = '2026-10-16T00:00:00.000Z'",
                [],
            )
            .unwrap();
        let adj = TxnInput::single(TxnKind::Adjustment, D, acc, None, 1);
        assert!(matches!(
            ledger.record(&adj),
            Err(AppError::Archived("account"))
        ));
        assert_eq!(txn_count(&ledger), 0);
    }
}
