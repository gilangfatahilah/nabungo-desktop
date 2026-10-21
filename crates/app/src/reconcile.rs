//! Rekonsiliasi akun (ACC-04): samakan saldo tercatat dengan saldo riil per tanggal,
//! selisihnya dicatat sebagai transaksi `adjustment`, lalu posting ditandai reconciled.

use std::collections::HashSet;

use nabungo_core::{MAX_AMOUNT, TxnError, TxnKind};
use nabungo_storage::{account, report, txn};
use time::Date;
use uuid::Uuid;

use crate::ledger::{AppError, Ledger, TxnInput, active};
use crate::transactions::write_txn;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconciliation {
    /// Transaksi penyesuaian; `None` bila saldo sudah cocok.
    pub adjustment: Option<Uuid>,
    /// Saldo riil − saldo tercatat per `date`.
    pub difference: i64,
    /// Jumlah posting akun yang ditandai reconciled.
    pub marked: usize,
}

impl Ledger {
    /// Saldo tercatat akun per akhir `date` (bertanda), untuk dialog rekonsiliasi bertanggal mundur.
    pub fn balance_as_of(&self, account_id: Uuid, date: Date) -> Result<i64, AppError> {
        account::get_account_ref(&self.conn, account_id)?.ok_or(AppError::NotFound("account"))?;
        Ok(report::account_balance_as_of(&self.conn, account_id, date)?)
    }

    /// Saldo riil akun per akhir `date` adalah `balance` (bertanda, liabilitas negatif).
    /// Selisih dicatat sebagai `adjustment`, lalu posting akun s.d. `date` ditandai (K5, K6).
    pub fn reconcile_account(
        &mut self,
        account_id: Uuid,
        date: Date,
        balance: i64,
    ) -> Result<Reconciliation, AppError> {
        if balance.unsigned_abs() > MAX_AMOUNT.unsigned_abs() {
            return Err(TxnError::AmountTooLarge.into());
        }
        let tx = self.conn.transaction()?;
        active(account::get_account_ref(&tx, account_id)?, "account")?;
        let difference = balance
            .checked_sub(report::account_balance_as_of(&tx, account_id, date)?)
            .ok_or(TxnError::AmountTooLarge)?;
        let adjustment = if difference == 0 {
            None
        } else {
            let input = TxnInput::single(TxnKind::Adjustment, date, account_id, None, difference);
            Some(write_txn(&tx, None, &input, &HashSet::new())?)
        };
        let marked = txn::mark_reconciled(&tx, account_id, date)?;
        tx.commit()?;
        Ok(Reconciliation {
            adjustment,
            difference,
            marked,
        })
    }
}

#[cfg(test)]
mod tests {
    use nabungo_core::{AccountKind, BudgetPeriod, TxnError, TxnKind};
    use nabungo_storage::txn_list::TxnQuery;
    use time::macros::date;
    use uuid::Uuid;

    use crate::testkit::Kit;
    use crate::{AccountFields, AppError};

    const D: time::Date = date!(2026 - 10 - 10);

    fn kit() -> Kit {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        k.expense(date!(2026 - 10 - 05), "Makan & Minum", 25_000);
        k
    }

    fn txn_count(k: &Kit) -> i64 {
        k.ledger
            .conn
            .query_row("SELECT count(*) FROM txn", [], |r| r.get(0))
            .unwrap()
    }

    fn through(k: &Kit, id: Uuid) -> Option<time::Date> {
        k.ledger
            .accounts_overview()
            .unwrap()
            .accounts
            .into_iter()
            .find(|a| a.id == id)
            .unwrap()
            .reconciled_through
    }

    #[test]
    fn difference_creates_adjustment_and_marks() {
        let mut k = kit();
        let bca = k.bca;
        let before = k.ledger.balance(bca).unwrap();
        let r = k.ledger.reconcile_account(bca, D, before - 15_000).unwrap();
        assert_eq!(r.difference, -15_000);
        let id = r.adjustment.unwrap();
        let t = k.ledger.get_txn(id).unwrap();
        assert_eq!(t.kind, TxnKind::Adjustment);
        assert_eq!(t.date, D);
        assert_eq!(t.postings.len(), 1);
        assert_eq!(t.postings[0].account_id, bca);
        assert_eq!(t.postings[0].category_id, None);
        assert_eq!(t.postings[0].amount, -15_000);
        assert_eq!(k.ledger.balance(bca).unwrap(), before - 15_000);
        // Opening, expense, adjustment.
        assert_eq!(r.marked, 3);
        assert_eq!(through(&k, bca), Some(D));
    }

    #[test]
    fn matching_balance_marks_without_adjustment() {
        let mut k = kit();
        let bca = k.bca;
        let count = txn_count(&k);
        let balance = k.ledger.balance(bca).unwrap();
        let r = k.ledger.reconcile_account(bca, D, balance).unwrap();
        assert_eq!(r.adjustment, None);
        assert_eq!(r.difference, 0);
        assert!(r.marked > 0);
        assert_eq!(txn_count(&k), count);
        assert_eq!(through(&k, bca), Some(date!(2026 - 10 - 05)));
    }

    #[test]
    fn balance_as_of_excludes_later_postings() {
        let mut k = kit();
        let bca = k.bca;
        let as_of = k.ledger.balance(bca).unwrap();
        k.expense(date!(2026 - 10 - 20), "Makan & Minum", 40_000);
        assert_eq!(k.ledger.balance_as_of(bca, D).unwrap(), as_of);
        assert_eq!(
            k.ledger.balance_as_of(bca, date!(2026 - 10 - 20)).unwrap(),
            as_of - 40_000
        );
        assert!(matches!(
            k.ledger.balance_as_of(Uuid::now_v7(), D),
            Err(AppError::NotFound("account"))
        ));
    }

    #[test]
    fn difference_uses_balance_as_of_date() {
        let mut k = kit();
        let bca = k.bca;
        let as_of = k.ledger.balance(bca).unwrap();
        k.expense(date!(2026 - 10 - 20), "Makan & Minum", 40_000);
        let r = k.ledger.reconcile_account(bca, D, as_of - 1_000).unwrap();
        assert_eq!(r.difference, -1_000);
        assert_eq!(k.ledger.balance(bca).unwrap(), as_of - 41_000);
    }

    #[test]
    fn later_postings_stay_unreconciled() {
        let mut k = kit();
        let bca = k.bca;
        let later = k.expense(date!(2026 - 10 - 20), "Makan & Minum", 40_000);
        let balance = k.ledger.balance(bca).unwrap() + 40_000;
        k.ledger.reconcile_account(bca, D, balance).unwrap();
        let page = k
            .ledger
            .list_txns(&TxnQuery {
                account_id: Some(bca),
                limit: 100,
                ..TxnQuery::default()
            })
            .unwrap();
        for row in &page.rows {
            assert_eq!(row.reconciled, Some(row.id != later), "{:?}", row.date);
        }
    }

    #[test]
    fn update_account_keeps_opening_reconciled() {
        let mut k = kit();
        let opening = date!(2026 - 10 - 01);
        let id = k
            .ledger
            .create_account(
                AccountKind::Cash,
                &AccountFields::new("Laci", 50_000, opening),
            )
            .unwrap();
        k.ledger.reconcile_account(id, D, 50_000).unwrap();
        assert_eq!(through(&k, id), Some(opening));
        let mut f = AccountFields::new("Laci meja", 50_000, opening);
        f.icon = Some("wallet".into());
        k.ledger.update_account(id, &f).unwrap();
        assert_eq!(through(&k, id), Some(opening));
        // Saldo awal yang benar-benar berubah tetap menghapus tanda (K8).
        k.ledger
            .update_account(id, &AccountFields::new("Laci meja", 60_000, opening))
            .unwrap();
        assert_eq!(through(&k, id), None);
    }

    #[test]
    fn liability_reconcile_uses_signed_balance() {
        let mut k = kit();
        let mut f = AccountFields::new("Kartu", -100_000, date!(2026 - 10 - 01));
        f.credit_limit = Some(1_000_000);
        f.statement_day = Some(17);
        let cc = k
            .ledger
            .create_account(AccountKind::CreditCard, &f)
            .unwrap();
        let r = k.ledger.reconcile_account(cc, D, -120_000).unwrap();
        assert_eq!(r.difference, -20_000);
        assert_eq!(k.ledger.balance(cc).unwrap(), -120_000);
    }

    #[test]
    fn rejects_archived_unknown_and_too_large() {
        let mut k = kit();
        let (bca, dompet) = (k.bca, k.dompet);
        let count = txn_count(&k);
        assert!(matches!(
            k.ledger
                .reconcile_account(bca, D, nabungo_core::MAX_AMOUNT + 1),
            Err(AppError::Txn(TxnError::AmountTooLarge))
        ));
        assert!(matches!(
            k.ledger.reconcile_account(Uuid::now_v7(), D, 0),
            Err(AppError::NotFound("account"))
        ));
        k.ledger.archive_account(dompet).unwrap();
        assert!(matches!(
            k.ledger.reconcile_account(dompet, D, 5),
            Err(AppError::Archived("account"))
        ));
        assert_eq!(txn_count(&k), count);
        assert_eq!(through(&k, bca), None);
    }

    #[test]
    fn adjustment_not_in_budget_or_cashflow() {
        let mut k = kit();
        let bca = k.bca;
        let period = BudgetPeriod::new(2026, 10).unwrap();
        let budget = k.ledger.budget_overview(period, D).unwrap();
        let cashflow = k.ledger.cashflow_report(period, 1).unwrap();
        let balance = k.ledger.balance(bca).unwrap();
        k.ledger
            .reconcile_account(bca, D, balance - 15_000)
            .unwrap();
        assert_eq!(k.ledger.budget_overview(period, D).unwrap(), budget);
        assert_eq!(k.ledger.cashflow_report(period, 1).unwrap(), cashflow);
    }
}
