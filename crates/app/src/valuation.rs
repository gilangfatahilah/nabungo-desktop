//! Valuation akun investasi (CAP-02 prasyarat): catat nilai pasar sebagai selisih terhadap saldo.

use std::collections::HashSet;

use nabungo_core::{MAX_AMOUNT, TxnError, TxnKind};
use nabungo_storage::{account, report};
use time::Date;
use uuid::Uuid;

use crate::ledger::{AppError, Ledger, TxnInput, active};
use crate::transactions::write_txn;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuationPoint {
    pub txn_id: Uuid,
    pub date: Date,
    /// Selisih yang dicatat (posting valuation).
    pub change: i64,
    /// Saldo akun per akhir `date`.
    pub value: i64,
}

impl Ledger {
    /// Nilai akun per akhir `date` menjadi `value`; yang dicatat adalah selisihnya (K5).
    pub fn record_valuation(
        &mut self,
        account_id: Uuid,
        date: Date,
        value: i64,
    ) -> Result<Uuid, AppError> {
        if value.unsigned_abs() > MAX_AMOUNT.unsigned_abs() {
            return Err(TxnError::AmountTooLarge.into());
        }
        let tx = self.conn.transaction()?;
        active(account::get_account_ref(&tx, account_id)?, "account")?;
        let change = value
            .checked_sub(report::account_balance_as_of(&tx, account_id, date)?)
            .ok_or(TxnError::AmountTooLarge)?;
        if change == 0 {
            return Err(AppError::ValuationUnchanged);
        }
        let input = TxnInput::single(TxnKind::Valuation, date, account_id, None, change);
        let id = write_txn(&tx, None, &input, &HashSet::new())?;
        tx.commit()?;
        Ok(id)
    }

    /// Valuation aktif, terbaru dulu.
    pub fn valuation_history(&self, account_id: Uuid) -> Result<Vec<ValuationPoint>, AppError> {
        report::valuations(&self.conn, account_id)?
            .into_iter()
            .map(|(txn_id, date, change)| {
                Ok(ValuationPoint {
                    txn_id,
                    date,
                    change,
                    value: report::account_balance_as_of(&self.conn, account_id, date)?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use nabungo_core::{AccountKind, FormBody, SplitLine, TxnError};
    use time::macros::date;
    use uuid::Uuid;

    use crate::testkit::Kit;
    use crate::{AccountFields, AppError, TxnForm};

    fn kit() -> (Kit, Uuid) {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let rdn = k
            .ledger
            .create_account(
                AccountKind::Investment,
                &AccountFields::new("RDN", 10_000_000, date!(2026 - 10 - 01)),
            )
            .unwrap();
        (k, rdn)
    }

    #[test]
    fn valuation_records_delta_against_balance() {
        let (mut k, rdn) = kit();
        let id = k
            .ledger
            .record_valuation(rdn, date!(2026 - 10 - 31), 10_750_000)
            .unwrap();
        assert_eq!(k.ledger.balance(rdn).unwrap(), 10_750_000);
        let h = k.ledger.valuation_history(rdn).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(
            (h[0].txn_id, h[0].date, h[0].change, h[0].value),
            (id, date!(2026 - 10 - 31), 750_000, 10_750_000)
        );
    }

    #[test]
    fn valuation_in_past_uses_balance_as_of_date() {
        let (mut k, rdn) = kit();
        let bca = k.bca;
        k.ledger
            .create_txn(&TxnForm {
                body: FormBody::Transfer {
                    from: bca,
                    to: rdn,
                    amount: 1_000_000,
                    fee: None,
                },
                date: date!(2026 - 10 - 20),
                payee: None,
                note: None,
                tags: vec![],
            })
            .unwrap();
        k.ledger
            .record_valuation(rdn, date!(2026 - 10 - 10), 9_500_000)
            .unwrap();
        assert_eq!(k.ledger.balance(rdn).unwrap(), 10_500_000);
        k.ledger
            .record_valuation(rdn, date!(2026 - 10 - 31), 11_000_000)
            .unwrap();
        let h = k.ledger.valuation_history(rdn).unwrap();
        let got: Vec<_> = h.iter().map(|p| (p.date, p.change, p.value)).collect();
        assert_eq!(
            got,
            vec![
                (date!(2026 - 10 - 31), 500_000, 11_000_000),
                (date!(2026 - 10 - 10), -500_000, 9_500_000),
            ]
        );
    }

    #[test]
    fn valuation_rules() {
        let (mut k, rdn) = kit();
        let d = date!(2026 - 10 - 31);
        let bca = k.bca;
        assert!(matches!(
            k.ledger.record_valuation(bca, d, 5),
            Err(AppError::Txn(TxnError::ValuationNonInvestment))
        ));
        assert!(matches!(
            k.ledger.record_valuation(rdn, d, 10_000_000),
            Err(AppError::ValuationUnchanged)
        ));
        assert!(matches!(
            k.ledger
                .record_valuation(rdn, d, nabungo_core::MAX_AMOUNT + 1),
            Err(AppError::Txn(TxnError::AmountTooLarge))
        ));
        assert!(matches!(
            k.ledger.record_valuation(Uuid::now_v7(), d, 0),
            Err(AppError::NotFound("account"))
        ));
        k.ledger.archive_account(rdn).unwrap();
        assert!(matches!(
            k.ledger.record_valuation(rdn, d, 1),
            Err(AppError::Archived("account"))
        ));
        assert!(k.ledger.valuation_history(rdn).unwrap().is_empty());
    }

    #[test]
    fn valuation_not_editable_via_form() {
        let (mut k, rdn) = kit();
        let id = k
            .ledger
            .record_valuation(rdn, date!(2026 - 10 - 31), 12_000_000)
            .unwrap();
        let category_id = k.cat("Belanja");
        let bca = k.bca;
        let form = TxnForm {
            body: FormBody::Expense {
                account_id: bca,
                amount: 1,
                splits: vec![SplitLine {
                    category_id,
                    amount: 1,
                    memo: None,
                }],
            },
            date: date!(2026 - 10 - 31),
            payee: None,
            note: None,
            tags: vec![],
        };
        assert!(matches!(
            k.ledger.update_txn(id, &form),
            Err(AppError::NotEditable)
        ));
        k.ledger.delete_txn(id).unwrap();
        assert_eq!(k.ledger.balance(rdn).unwrap(), 10_000_000);
        assert!(k.ledger.valuation_history(rdn).unwrap().is_empty());
    }
}
