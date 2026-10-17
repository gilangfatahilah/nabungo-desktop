//! Use case pencatatan: resolve input berbasis ID → validasi domain → simpan atomik.

use std::path::Path;

use nabungo_core::{AccountRef, CategoryKind, PostingDraft, TxnDraft, TxnError, TxnKind, ValidTxn};
use nabungo_storage::account::{self, NewAccount};
use nabungo_storage::rusqlite::{self, Connection};
use nabungo_storage::{Lookup, StorageError, category, now_utc, txn};
use time::Date;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Txn(#[from] TxnError),
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0} is archived")]
    Archived(&'static str),
    /// Saldo awal hanya dibuat oleh `create_account`, tidak lewat `record`.
    #[error("opening balance can only be set when creating an account")]
    OpeningNotAllowed,
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
    pub note: Option<String>,
    pub postings: Vec<PostingInput>,
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
            note: None,
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
            note: None,
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
}

impl Ledger {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            conn: nabungo_storage::open(path)?,
        })
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        Ok(Self {
            conn: nabungo_storage::open_in_memory()?,
        })
    }

    /// Buat akun; saldo awal ≠ 0 dicatat sebagai transaksi `opening`.
    pub fn create_account(
        &mut self,
        new: NewAccount,
        opening_balance: i64,
        opening_date: Date,
    ) -> Result<Uuid, AppError> {
        let now = now_utc();
        let tx = self.conn.transaction()?;
        let id = account::insert_account(&tx, &new, &now)?;
        if opening_balance != 0 {
            let draft = TxnDraft {
                kind: TxnKind::Opening,
                date: opening_date,
                note: None,
                postings: vec![PostingDraft {
                    account: AccountRef {
                        id,
                        kind: new.kind,
                        currency: new.currency,
                    },
                    category: None,
                    amount: opening_balance,
                    memo: None,
                }],
            };
            txn::insert_txn(&tx, &ValidTxn::new(draft)?, &now)?;
        }
        tx.commit()?;
        Ok(id)
    }

    pub fn create_category(&mut self, kind: CategoryKind, name: &str) -> Result<Uuid, AppError> {
        let tx = self.conn.transaction()?;
        let id = category::insert_category(&tx, kind, name, None)?;
        tx.commit()?;
        Ok(id)
    }

    pub fn record(&mut self, input: &TxnInput) -> Result<Uuid, AppError> {
        if input.kind == TxnKind::Opening {
            return Err(AppError::OpeningNotAllowed);
        }
        let now = now_utc();
        let tx = self.conn.transaction()?;
        let valid = ValidTxn::new(resolve(&tx, input)?)?;
        let id = txn::insert_txn(&tx, &valid, &now)?;
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

fn resolve(conn: &Connection, input: &TxnInput) -> Result<TxnDraft, AppError> {
    let postings = input
        .postings
        .iter()
        .map(|p| {
            let account = active(account::get_account_ref(conn, p.account_id)?, "account")?;
            let category = match p.category_id {
                Some(id) => Some(active(category::get_category_ref(conn, id)?, "category")?),
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

/// Transaksi baru tidak boleh menyentuh akun/kategori yang sudah diarsip.
fn active<T>(found: Option<Lookup<T>>, what: &'static str) -> Result<T, AppError> {
    match found {
        None => Err(AppError::NotFound(what)),
        Some(l) if l.archived => Err(AppError::Archived(what)),
        Some(l) => Ok(l.value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nabungo_core::{AccountKind, CurrencyCode};
    use time::macros::date;

    const D: Date = date!(2026 - 10 - 12);

    fn bank(ledger: &mut Ledger, opening: i64) -> Uuid {
        let new = NewAccount::new("BCA", AccountKind::Bank, CurrencyCode::IDR);
        ledger.create_account(new, opening, D).unwrap()
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
            .create_category(CategoryKind::Expense, "Makan")
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
            .create_category(CategoryKind::Expense, "Makan")
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
