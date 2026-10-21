use nabungo_core::date::parse_date;
use nabungo_core::{AccountKind, AccountRef, CurrencyCode};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};
use time::Date;
use uuid::Uuid;

use crate::{Lookup, Result, StorageError, parse_uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAccount {
    pub name: String,
    pub kind: AccountKind,
    pub currency: CurrencyCode,
    pub group_id: Option<Uuid>,
    pub include_in_net_worth: bool,
    pub credit_limit: Option<i64>,
    pub statement_day: Option<u8>,
    pub icon: Option<String>,
    pub color: Option<String>,
    /// Hanya untuk `receivable` (D10).
    pub contact_id: Option<Uuid>,
}

impl NewAccount {
    /// Akun tanpa group, ikut net worth, tanpa atribut opsional.
    pub fn new(name: impl Into<String>, kind: AccountKind, currency: CurrencyCode) -> Self {
        Self {
            name: name.into(),
            kind,
            currency,
            group_id: None,
            include_in_net_worth: true,
            credit_limit: None,
            statement_day: None,
            icon: None,
            color: None,
            contact_id: None,
        }
    }
}

/// Field akun yang boleh diubah setelah dibuat (jenis & currency tidak).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountPatch {
    pub name: String,
    pub group_id: Option<Uuid>,
    pub include_in_net_worth: bool,
    pub credit_limit: Option<i64>,
    pub statement_day: Option<u8>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub contact_id: Option<Uuid>,
}

/// Transaksi saldo awal aktif milik akun.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opening {
    pub txn_id: Uuid,
    pub amount: i64,
    pub date: Date,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRow {
    pub id: Uuid,
    pub name: String,
    pub kind: AccountKind,
    pub currency: CurrencyCode,
    pub group_id: Option<Uuid>,
    pub include_in_net_worth: bool,
    pub credit_limit: Option<i64>,
    pub statement_day: Option<u8>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
    pub archived: bool,
    pub balance: i64,
    pub opening: Option<Opening>,
    /// Nama kontak (akun piutang/hutang).
    pub contact: Option<String>,
    /// Tanggal txn terakhir yang posting akun ini sudah ditandai reconciled.
    pub reconciled_through: Option<Date>,
}

fn parse_kind(id: &str, kind: &str) -> Result<AccountKind> {
    AccountKind::parse(kind)
        .ok_or_else(|| StorageError::Corrupt(format!("account {id}: unknown kind {kind:?}")))
}

fn parse_currency(id: &str, currency: &str) -> Result<CurrencyCode> {
    CurrencyCode::parse(currency).map_err(|e| StorageError::Corrupt(format!("account {id}: {e}")))
}

pub fn insert_account(tx: &Transaction, new: &NewAccount, now: &str) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO account (id, group_id, name, kind, currency, include_in_net_worth,
                              credit_limit, statement_day, icon, color, sort_order,
                              created_at, updated_at, contact_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                 COALESCE((SELECT MAX(sort_order) + 1 FROM account WHERE group_id IS ?2), 0),
                 ?11, ?11, ?12)",
        params![
            id.to_string(),
            new.group_id.map(|g| g.to_string()),
            new.name,
            new.kind.as_str(),
            new.currency.as_str(),
            new.include_in_net_worth,
            new.credit_limit,
            new.statement_day,
            new.icon,
            new.color,
            now,
            new.contact_id.map(|c| c.to_string())
        ],
    )?;
    Ok(id)
}

pub fn get_account_ref(conn: &Connection, id: Uuid) -> Result<Option<Lookup<AccountRef>>> {
    let row = conn
        .query_row(
            "SELECT kind, currency, archived_at IS NOT NULL FROM account WHERE id = ?1",
            [id.to_string()],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, bool>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((kind, currency, archived)) = row else {
        return Ok(None);
    };
    let key = id.to_string();
    Ok(Some(Lookup {
        value: AccountRef {
            id,
            kind: parse_kind(&key, &kind)?,
            currency: parse_currency(&key, &currency)?,
        },
        archived,
    }))
}

/// Mentah dari SQL; diubah ke [`AccountRow`] di luar closure rusqlite.
struct RawAccount {
    id: String,
    name: String,
    kind: String,
    currency: String,
    group_id: Option<String>,
    include_in_net_worth: bool,
    credit_limit: Option<i64>,
    statement_day: Option<u8>,
    icon: Option<String>,
    color: Option<String>,
    sort_order: i64,
    archived: bool,
    balance: i64,
    opening: Option<(String, i64, String)>,
    contact: Option<String>,
    reconciled_through: Option<String>,
}

impl RawAccount {
    fn from_row(r: &Row) -> rusqlite::Result<Self> {
        let opening_id: Option<String> = r.get(13)?;
        Ok(Self {
            id: r.get(0)?,
            name: r.get(1)?,
            kind: r.get(2)?,
            currency: r.get(3)?,
            group_id: r.get(4)?,
            include_in_net_worth: r.get(5)?,
            credit_limit: r.get(6)?,
            statement_day: r.get(7)?,
            icon: r.get(8)?,
            color: r.get(9)?,
            sort_order: r.get(10)?,
            archived: r.get(11)?,
            balance: r.get(12)?,
            opening: match opening_id {
                Some(txn) => Some((txn, r.get(14)?, r.get(15)?)),
                None => None,
            },
            contact: r.get(16)?,
            reconciled_through: r.get(17)?,
        })
    }

    fn into_row(self) -> Result<AccountRow> {
        let opening = match self.opening {
            Some((txn, amount, date)) => Some(Opening {
                txn_id: parse_uuid("txn", &txn)?,
                amount,
                date: parse_date(&date)
                    .map_err(|e| StorageError::Corrupt(format!("txn {txn}: date {date:?}: {e}")))?,
            }),
            None => None,
        };
        let reconciled_through = self
            .reconciled_through
            .as_deref()
            .map(|d| {
                parse_date(d).map_err(|e| {
                    StorageError::Corrupt(format!(
                        "account {}: reconciled date {d:?}: {e}",
                        self.id
                    ))
                })
            })
            .transpose()?;
        Ok(AccountRow {
            id: parse_uuid("account", &self.id)?,
            kind: parse_kind(&self.id, &self.kind)?,
            currency: parse_currency(&self.id, &self.currency)?,
            group_id: self
                .group_id
                .as_deref()
                .map(|g| parse_uuid("account_group", g))
                .transpose()?,
            name: self.name,
            include_in_net_worth: self.include_in_net_worth,
            credit_limit: self.credit_limit,
            statement_day: self.statement_day,
            icon: self.icon,
            color: self.color,
            sort_order: self.sort_order,
            archived: self.archived,
            balance: self.balance,
            opening,
            contact: self.contact,
            reconciled_through,
        })
    }
}

/// Semua akun (termasuk diarsip) dengan saldo dan saldo awal, dalam satu query.
/// Saldo memakai aturan yang sama dengan [`crate::txn::account_balance`].
pub fn list_accounts(conn: &Connection) -> Result<Vec<AccountRow>> {
    let mut stmt = conn.prepare_cached(
        "SELECT a.id, a.name, a.kind, a.currency, a.group_id, a.include_in_net_worth,
                a.credit_limit, a.statement_day, a.icon, a.color, a.sort_order,
                a.archived_at IS NOT NULL, COALESCE(b.balance, 0),
                o.txn_id, o.amount, o.date, c.name,
                (SELECT MAX(t.date) FROM posting p JOIN txn t ON t.id = p.txn_id
                 WHERE p.account_id = a.id AND p.reconciled = 1
                   AND t.deleted_at IS NULL AND t.status = 'posted')
         FROM account a
         LEFT JOIN contact c ON c.id = a.contact_id
         LEFT JOIN (SELECT p.account_id, SUM(p.amount) AS balance
                    FROM posting p JOIN txn t ON t.id = p.txn_id
                    WHERE t.deleted_at IS NULL AND t.status = 'posted'
                    GROUP BY p.account_id) b ON b.account_id = a.id
         LEFT JOIN (SELECT p.account_id, t.id AS txn_id, p.amount, t.date
                    FROM posting p JOIN txn t ON t.id = p.txn_id
                    WHERE t.kind = 'opening' AND t.deleted_at IS NULL) o ON o.account_id = a.id
         ORDER BY a.sort_order, a.name",
    )?;
    let raw = stmt
        .query_map([], RawAccount::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter().map(RawAccount::into_row).collect()
}

pub fn update_account(tx: &Transaction, id: Uuid, patch: &AccountPatch, now: &str) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE account SET name = ?2, group_id = ?3, include_in_net_worth = ?4,
                            credit_limit = ?5, statement_day = ?6, icon = ?7, color = ?8,
                            updated_at = ?9, contact_id = ?10
         WHERE id = ?1",
        params![
            id.to_string(),
            patch.name,
            patch.group_id.map(|g| g.to_string()),
            patch.include_in_net_worth,
            patch.credit_limit,
            patch.statement_day,
            patch.icon,
            patch.color,
            now,
            patch.contact_id.map(|c| c.to_string())
        ],
    )?;
    Ok(changed == 1)
}

pub fn set_account_archived(tx: &Transaction, id: Uuid, archived: bool, now: &str) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE account SET archived_at = CASE WHEN ?2 THEN ?3 END, updated_at = ?3
         WHERE id = ?1",
        params![id.to_string(), archived, now],
    )?;
    Ok(changed == 1)
}

pub fn set_account_position(
    tx: &Transaction,
    id: Uuid,
    group_id: Option<Uuid>,
    sort_order: i64,
    now: &str,
) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE account SET group_id = ?2, sort_order = ?3, updated_at = ?4 WHERE id = ?1",
        params![
            id.to_string(),
            group_id.map(|g| g.to_string()),
            sort_order,
            now
        ],
    )?;
    Ok(changed == 1)
}

/// Akun dipakai transaksi selain saldo awal — termasuk transaksi yang di-soft-delete,
/// supaya undo hapus tetap menunjuk akun yang ada.
pub fn has_non_opening_postings(conn: &Connection, id: Uuid) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM posting p JOIN txn t ON t.id = p.txn_id
                        WHERE p.account_id = ?1 AND t.kind <> 'opening')",
        [id.to_string()],
        |r| r.get(0),
    )?)
}

/// Hard-delete transaksi saldo awal akun (postings ikut via CASCADE). Saldo awal bukan
/// transaksi user, jadi tidak lewat soft delete/undo.
pub fn delete_opening(tx: &Transaction, id: Uuid) -> Result<()> {
    tx.execute(
        "DELETE FROM txn WHERE kind = 'opening'
           AND id IN (SELECT txn_id FROM posting WHERE account_id = ?1)",
        [id.to_string()],
    )?;
    Ok(())
}

/// Hapus akun beserta transaksi saldo awal (postings ikut via CASCADE) dan
/// `allocation_mapping`-nya. Pemanggil wajib cek [`has_non_opening_postings`] dulu.
pub fn delete_account(tx: &Transaction, id: Uuid) -> Result<bool> {
    let key = id.to_string();
    delete_opening(tx, id)?;
    tx.execute(
        "DELETE FROM allocation_mapping WHERE account_id = ?1",
        [&key],
    )?;
    Ok(tx.execute("DELETE FROM account WHERE id = ?1", [&key])? == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account_group::insert_group;
    use crate::contact::ensure_contact;
    use crate::txn::{insert_txn, mark_reconciled, replace_txn, soft_delete};
    use crate::{now_utc, open_in_memory};
    use nabungo_core::{PostingDraft, TxnDraft, TxnKind, ValidTxn};
    use time::Date;
    use time::macros::date;

    fn bank(tx: &Transaction, name: &str) -> AccountRef {
        let new = NewAccount::new(name, AccountKind::Bank, CurrencyCode::IDR);
        let id = insert_account(tx, &new, &now_utc()).unwrap();
        get_account_ref(tx, id).unwrap().unwrap().value
    }

    fn record(tx: &Transaction, kind: TxnKind, on: Date, postings: &[(AccountRef, i64)]) -> Uuid {
        let txn = ValidTxn::new(TxnDraft {
            kind,
            date: on,
            note: None,
            postings: postings
                .iter()
                .map(|&(account, amount)| PostingDraft {
                    account,
                    category: None,
                    amount,
                    memo: None,
                })
                .collect(),
        })
        .unwrap();
        insert_txn(tx, &txn, None, &now_utc()).unwrap()
    }

    fn row(conn: &Connection, id: Uuid) -> AccountRow {
        list_accounts(conn)
            .unwrap()
            .into_iter()
            .find(|a| a.id == id)
            .unwrap()
    }

    #[test]
    fn list_accounts_has_balance_and_opening() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let (a, b, c) = (bank(&tx, "A"), bank(&tx, "B"), bank(&tx, "C"));
        record(
            &tx,
            TxnKind::Opening,
            date!(2026 - 10 - 01),
            &[(a, 500_000)],
        );
        record(
            &tx,
            TxnKind::Transfer,
            date!(2026 - 10 - 02),
            &[(a, -200_000), (b, 200_000)],
        );
        let gone = record(&tx, TxnKind::Adjustment, date!(2026 - 10 - 03), &[(a, -1)]);
        soft_delete(&tx, gone, &now_utc()).unwrap();

        let ra = row(&tx, a.id);
        assert_eq!(ra.balance, 300_000);
        let opening = ra.opening.unwrap();
        assert_eq!(opening.amount, 500_000);
        assert_eq!(opening.date, date!(2026 - 10 - 01));
        let rb = row(&tx, b.id);
        assert_eq!(rb.opening, None);
        assert_eq!(rb.balance, 200_000);
        assert_eq!(row(&tx, c.id).balance, 0);
    }

    fn draft(kind: TxnKind, on: Date, postings: &[(AccountRef, i64)]) -> ValidTxn {
        ValidTxn::new(TxnDraft {
            kind,
            date: on,
            note: None,
            postings: postings
                .iter()
                .map(|&(account, amount)| PostingDraft {
                    account,
                    category: None,
                    amount,
                    memo: None,
                })
                .collect(),
        })
        .unwrap()
    }

    #[test]
    fn contact_roundtrip() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let c = ensure_contact(&tx, "Budi", &now_utc()).unwrap();
        let new = NewAccount {
            contact_id: Some(c),
            ..NewAccount::new("Pinjaman Budi", AccountKind::Receivable, CurrencyCode::IDR)
        };
        let id = insert_account(&tx, &new, &now_utc()).unwrap();
        assert_eq!(row(&tx, id).contact.as_deref(), Some("Budi"));
        let patch = AccountPatch {
            name: "Pinjaman Budi".into(),
            group_id: None,
            include_in_net_worth: true,
            credit_limit: None,
            statement_day: None,
            icon: None,
            color: None,
            contact_id: None,
        };
        assert!(update_account(&tx, id, &patch, &now_utc()).unwrap());
        assert_eq!(row(&tx, id).contact, None);
    }

    #[test]
    fn mark_reconciled_respects_date_and_account() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let (a, b) = (bank(&tx, "A"), bank(&tx, "B"));
        record(&tx, TxnKind::Opening, date!(2026 - 10 - 01), &[(a, 100)]);
        record(&tx, TxnKind::Opening, date!(2026 - 10 - 01), &[(b, 100)]);
        let gone = record(&tx, TxnKind::Adjustment, date!(2026 - 10 - 03), &[(a, -1)]);
        soft_delete(&tx, gone, &now_utc()).unwrap();
        record(&tx, TxnKind::Adjustment, date!(2026 - 10 - 05), &[(a, -2)]);
        record(&tx, TxnKind::Adjustment, date!(2026 - 10 - 09), &[(a, -3)]);
        assert_eq!(row(&tx, a.id).reconciled_through, None);
        assert_eq!(
            mark_reconciled(&tx, a.id, date!(2026 - 10 - 05)).unwrap(),
            2
        );
        assert_eq!(
            row(&tx, a.id).reconciled_through,
            Some(date!(2026 - 10 - 05))
        );
        assert_eq!(row(&tx, b.id).reconciled_through, None);
    }

    #[test]
    fn replace_txn_clears_reconciled() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let a = bank(&tx, "A");
        record(&tx, TxnKind::Opening, date!(2026 - 10 - 01), &[(a, 100)]);
        let later = record(&tx, TxnKind::Adjustment, date!(2026 - 10 - 05), &[(a, -2)]);
        mark_reconciled(&tx, a.id, date!(2026 - 10 - 05)).unwrap();
        let edited = draft(TxnKind::Adjustment, date!(2026 - 10 - 05), &[(a, -4)]);
        assert!(replace_txn(&tx, later, &edited, None, &now_utc()).unwrap());
        assert_eq!(
            row(&tx, a.id).reconciled_through,
            Some(date!(2026 - 10 - 01))
        );
    }

    #[test]
    fn update_and_archive_roundtrip() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let a = bank(&tx, "A");
        let g = insert_group(&tx, "Harian").unwrap();
        let patch = AccountPatch {
            name: "BCA".into(),
            group_id: Some(g),
            include_in_net_worth: false,
            credit_limit: None,
            statement_day: None,
            icon: Some("banknote".into()),
            color: Some("blue".into()),
            contact_id: None,
        };
        assert!(update_account(&tx, a.id, &patch, &now_utc()).unwrap());
        let r = row(&tx, a.id);
        assert_eq!(r.name, "BCA");
        assert_eq!(r.group_id, Some(g));
        assert!(!r.include_in_net_worth);
        assert_eq!(r.icon.as_deref(), Some("banknote"));
        assert_eq!(r.color.as_deref(), Some("blue"));

        assert!(set_account_archived(&tx, a.id, true, &now_utc()).unwrap());
        assert!(row(&tx, a.id).archived);
        assert!(set_account_archived(&tx, a.id, false, &now_utc()).unwrap());
        assert!(!row(&tx, a.id).archived);
        assert!(!set_account_archived(&tx, Uuid::now_v7(), true, &now_utc()).unwrap());
        assert!(!update_account(&tx, Uuid::now_v7(), &patch, &now_utc()).unwrap());
    }

    #[test]
    fn delete_account_removes_opening_and_mapping() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let a = bank(&tx, "A");
        record(
            &tx,
            TxnKind::Opening,
            date!(2026 - 10 - 01),
            &[(a, 500_000)],
        );
        tx.execute(
            "INSERT INTO allocation_bucket (id, name, target_bp) VALUES ('b1', 'Kas', 10000)",
            [],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO allocation_mapping (account_id, bucket_id) VALUES (?1, 'b1')",
            [a.id.to_string()],
        )
        .unwrap();

        assert!(!has_non_opening_postings(&tx, a.id).unwrap());
        assert!(delete_account(&tx, a.id).unwrap());
        for table in ["txn", "posting", "allocation_mapping", "account"] {
            let n: i64 = tx
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table} not empty");
        }
        assert!(!delete_account(&tx, a.id).unwrap());
    }

    #[test]
    fn soft_deleted_txn_counts_as_usage() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let a = bank(&tx, "A");
        let id = record(&tx, TxnKind::Adjustment, date!(2026 - 10 - 01), &[(a, 10)]);
        soft_delete(&tx, id, &now_utc()).unwrap();
        assert!(has_non_opening_postings(&tx, a.id).unwrap());
    }
}
