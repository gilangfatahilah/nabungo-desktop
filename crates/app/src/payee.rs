//! Payee & saran kategori dari transaksi terakhir (CAT-03).

use nabungo_core::normalize_name;
use nabungo_storage::rusqlite::{Connection, Transaction};
use nabungo_storage::{account, category, payee};
use uuid::Uuid;

use crate::ledger::{AppError, Ledger};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayeeSuggestion {
    pub account_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
}

/// Nama payee → ID (dipakai ulang tanpa membedakan huruf besar/kecil, atau dibuat).
/// Kosong/spasi → tanpa payee.
pub(crate) fn resolve_payee(
    tx: &Transaction,
    name: Option<&str>,
) -> Result<Option<Uuid>, AppError> {
    let Some(name) = name.filter(|n| !n.trim().is_empty()) else {
        return Ok(None);
    };
    let name = normalize_name(name)?;
    if let Some(id) = payee::find_payee(tx, &name)? {
        return Ok(Some(id));
    }
    Ok(Some(payee::insert_payee(tx, &name)?))
}

fn active_category(conn: &Connection, id: Uuid) -> Result<bool, AppError> {
    Ok(category::get_category_ref(conn, id)?.is_some_and(|c| !c.archived))
}

fn active_account(conn: &Connection, id: Uuid) -> Result<bool, AppError> {
    Ok(account::get_account_ref(conn, id)?.is_some_and(|a| !a.archived))
}

impl Ledger {
    /// Saran akun & kategori untuk payee; `None` bila payee belum dikenal.
    /// Kategori: default payee bila aktif, selain itu dari transaksi terakhir.
    pub fn payee_suggestion(&self, name: &str) -> Result<Option<PayeeSuggestion>, AppError> {
        let Some(id) = payee::find_payee(&self.conn, name.trim())? else {
            return Ok(None);
        };
        let last = payee::last_postings_for_payee(&self.conn, id)?;
        let mut category_id = None;
        if let Some(c) = payee::payee_default_category(&self.conn, id)?
            && active_category(&self.conn, c)?
        {
            category_id = Some(c);
        }
        if category_id.is_none() {
            for c in last.iter().filter_map(|(_, c)| *c) {
                if active_category(&self.conn, c)? {
                    category_id = Some(c);
                    break;
                }
            }
        }
        let mut account_id = None;
        for &(a, _) in &last {
            if active_account(&self.conn, a)? {
                account_id = Some(a);
                break;
            }
        }
        Ok(Some(PayeeSuggestion {
            account_id,
            category_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountFields, CategoryFields, Ledger, TxnInput};
    use nabungo_core::{AccountKind, CategoryKind, TxnKind};
    use time::Date;
    use time::macros::date;
    use uuid::Uuid;

    fn account(ledger: &mut Ledger, name: &str) -> Uuid {
        ledger
            .create_account(
                AccountKind::Bank,
                &AccountFields::new(name, 0, date!(2026 - 10 - 01)),
            )
            .unwrap()
    }

    fn expense_cat(ledger: &mut Ledger, name: &str) -> Uuid {
        ledger
            .create_category(CategoryKind::Expense, &CategoryFields::named(name))
            .unwrap()
    }

    fn spend(ledger: &mut Ledger, d: Date, acc: Uuid, cat: Uuid, payee: &str) -> Uuid {
        let mut input = TxnInput::single(TxnKind::Expense, d, acc, Some(cat), -5_000);
        input.payee = Some(payee.to_owned());
        ledger.record(&input).unwrap()
    }

    fn payee_of(ledger: &Ledger, txn: Uuid) -> Option<String> {
        ledger
            .conn
            .query_row(
                "SELECT payee_id FROM txn WHERE id = ?1",
                [txn.to_string()],
                |r| r.get(0),
            )
            .unwrap()
    }

    #[test]
    fn record_reuses_payee_case_insensitive() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = account(&mut ledger, "BCA");
        let food = expense_cat(&mut ledger, "Makan");
        let a = spend(&mut ledger, date!(2026 - 10 - 10), acc, food, "Indomaret");
        let b = spend(&mut ledger, date!(2026 - 10 - 11), acc, food, " indomaret ");
        let c = spend(&mut ledger, date!(2026 - 10 - 11), acc, food, "  ");
        let count: i64 = ledger
            .conn
            .query_row("SELECT count(*) FROM payee", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        assert!(payee_of(&ledger, a).is_some());
        assert_eq!(payee_of(&ledger, a), payee_of(&ledger, b));
        assert_eq!(payee_of(&ledger, c), None);
    }

    #[test]
    fn suggestion_from_last_txn() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let (a, b) = (account(&mut ledger, "A"), account(&mut ledger, "B"));
        let food = expense_cat(&mut ledger, "Makan");
        let shop = expense_cat(&mut ledger, "Belanja");
        spend(&mut ledger, date!(2026 - 10 - 10), a, food, "Indomaret");
        let last = spend(&mut ledger, date!(2026 - 10 - 12), b, shop, "Indomaret");
        assert_eq!(
            ledger.payee_suggestion("INDOMARET").unwrap(),
            Some(PayeeSuggestion {
                account_id: Some(b),
                category_id: Some(shop)
            })
        );
        ledger.delete_txn(last).unwrap();
        assert_eq!(
            ledger.payee_suggestion("Indomaret").unwrap(),
            Some(PayeeSuggestion {
                account_id: Some(a),
                category_id: Some(food)
            })
        );
    }

    #[test]
    fn default_category_wins_and_archived_skipped() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = account(&mut ledger, "BCA");
        let food = expense_cat(&mut ledger, "Makan");
        let snack = expense_cat(&mut ledger, "Jajan");
        spend(&mut ledger, date!(2026 - 10 - 10), acc, food, "Warung");
        ledger
            .conn
            .execute(
                "UPDATE payee SET default_category_id = ?1",
                [snack.to_string()],
            )
            .unwrap();
        let s = ledger.payee_suggestion("Warung").unwrap().unwrap();
        assert_eq!(s.category_id, Some(snack));

        ledger.archive_category(snack).unwrap();
        let s = ledger.payee_suggestion("Warung").unwrap().unwrap();
        assert_eq!(s.category_id, Some(food));

        ledger.archive_account(acc).unwrap();
        let s = ledger.payee_suggestion("Warung").unwrap().unwrap();
        assert_eq!(s.account_id, None);
    }

    #[test]
    fn unknown_payee_has_no_suggestion() {
        let ledger = Ledger::open_in_memory().unwrap();
        assert_eq!(ledger.payee_suggestion("Siapa").unwrap(), None);
    }
}
