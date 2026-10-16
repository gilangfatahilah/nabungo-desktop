//! Draft transaksi (header + postings) dan invariant PRD §6 no. 1–3.

use time::Date;
use uuid::Uuid;

use crate::{AccountKind, CategoryKind, CurrencyCode, TxnKind};

/// Batas nominal satu posting (10^12 satuan minor = 1 triliun rupiah). `SUM(amount)` di SQL
/// baru bisa overflow `i64` setelah ~9,2 juta posting bernilai maksimum di satu akun.
pub const MAX_AMOUNT: i64 = 1_000_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountRef {
    pub id: Uuid,
    pub kind: AccountKind,
    pub currency: CurrencyCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryRef {
    pub id: Uuid,
    pub kind: CategoryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostingDraft {
    pub account: AccountRef,
    pub category: Option<CategoryRef>,
    /// +masuk / -keluar, satuan minor currency akun.
    pub amount: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnDraft {
    pub kind: TxnKind,
    pub date: Date,
    pub note: Option<String>,
    pub postings: Vec<PostingDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TxnError {
    #[error("transaction has no postings")]
    NoPostings,
    #[error("posting amount must not be zero")]
    ZeroAmount,
    #[error("posting amount exceeds the maximum of {MAX_AMOUNT}")]
    AmountTooLarge,
    #[error("posting requires a category")]
    MissingCategory,
    #[error("posting category must be {expected:?}")]
    WrongCategoryKind { expected: CategoryKind },
    #[error("posting must not have a category")]
    UnexpectedCategory,
    #[error("transfer needs exactly 2 uncategorized postings, got {0}")]
    TransferLegCount(usize),
    #[error("transfer source and destination must differ")]
    TransferSameAccount,
    #[error("transfer between different currencies is not supported")]
    CurrencyMismatch,
    #[error("transfer is unbalanced by {0}")]
    Unbalanced(i64),
    #[error("amount overflow")]
    Overflow,
    #[error("valuation is only allowed on investment accounts")]
    ValuationNonInvestment,
    #[error("opening balance needs exactly 1 posting, got {0}")]
    OpeningPostingCount(usize),
}

/// Draft yang sudah lolos [`validate`]. Field privat: satu-satunya cara membuatnya adalah
/// [`ValidTxn::new`], sehingga storage tidak bisa menyimpan transaksi yang melanggar invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidTxn(TxnDraft);

impl ValidTxn {
    pub fn new(draft: TxnDraft) -> Result<Self, TxnError> {
        validate(&draft)?;
        Ok(Self(draft))
    }

    pub fn draft(&self) -> &TxnDraft {
        &self.0
    }
}

pub fn validate(draft: &TxnDraft) -> Result<(), TxnError> {
    let postings = &draft.postings;
    if postings.is_empty() {
        return Err(TxnError::NoPostings);
    }
    if postings.iter().any(|p| p.amount == 0) {
        return Err(TxnError::ZeroAmount);
    }
    if postings
        .iter()
        .any(|p| p.amount.unsigned_abs() > MAX_AMOUNT.unsigned_abs())
    {
        return Err(TxnError::AmountTooLarge);
    }
    match draft.kind {
        TxnKind::Income => require_category(postings, CategoryKind::Income),
        TxnKind::Expense => require_category(postings, CategoryKind::Expense),
        TxnKind::Transfer => validate_transfer(postings),
        TxnKind::Adjustment => forbid_category(postings),
        TxnKind::Valuation => {
            forbid_category(postings)?;
            if postings
                .iter()
                .all(|p| p.account.kind == AccountKind::Investment)
            {
                Ok(())
            } else {
                Err(TxnError::ValuationNonInvestment)
            }
        }
        TxnKind::Opening => {
            if postings.len() != 1 {
                return Err(TxnError::OpeningPostingCount(postings.len()));
            }
            forbid_category(postings)
        }
    }
}

fn require_category<'a>(
    postings: impl IntoIterator<Item = &'a PostingDraft>,
    expected: CategoryKind,
) -> Result<(), TxnError> {
    for p in postings {
        match p.category {
            None => return Err(TxnError::MissingCategory),
            Some(c) if c.kind != expected => return Err(TxnError::WrongCategoryKind { expected }),
            Some(_) => {}
        }
    }
    Ok(())
}

fn forbid_category(postings: &[PostingDraft]) -> Result<(), TxnError> {
    if postings.iter().any(|p| p.category.is_some()) {
        Err(TxnError::UnexpectedCategory)
    } else {
        Ok(())
    }
}

fn validate_transfer(postings: &[PostingDraft]) -> Result<(), TxnError> {
    let (legs, fees): (Vec<&PostingDraft>, Vec<&PostingDraft>) =
        postings.iter().partition(|p| p.category.is_none());
    let [from, to] = legs[..] else {
        return Err(TxnError::TransferLegCount(legs.len()));
    };
    if from.account.id == to.account.id {
        return Err(TxnError::TransferSameAccount);
    }
    if from.account.currency != to.account.currency {
        return Err(TxnError::CurrencyMismatch);
    }
    let sum = from
        .amount
        .checked_add(to.amount)
        .ok_or(TxnError::Overflow)?;
    if sum != 0 {
        return Err(TxnError::Unbalanced(sum));
    }
    require_category(fees, CategoryKind::Expense)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use time::macros::date;

    fn account(kind: AccountKind) -> AccountRef {
        AccountRef {
            id: Uuid::now_v7(),
            kind,
            currency: CurrencyCode::IDR,
        }
    }

    fn category(kind: CategoryKind) -> CategoryRef {
        CategoryRef {
            id: Uuid::now_v7(),
            kind,
        }
    }

    fn posting(account: AccountRef, category: Option<CategoryRef>, amount: i64) -> PostingDraft {
        PostingDraft {
            account,
            category,
            amount,
            memo: None,
        }
    }

    fn draft(kind: TxnKind, postings: Vec<PostingDraft>) -> TxnDraft {
        TxnDraft {
            kind,
            date: date!(2026 - 10 - 12),
            note: None,
            postings,
        }
    }

    fn transfer(a: AccountRef, b: AccountRef, out: i64, inc: i64) -> TxnDraft {
        draft(
            TxnKind::Transfer,
            vec![posting(a, None, out), posting(b, None, inc)],
        )
    }

    #[test]
    fn expense_with_expense_category_is_valid() {
        let d = draft(
            TxnKind::Expense,
            vec![posting(
                account(AccountKind::Ewallet),
                Some(category(CategoryKind::Expense)),
                -25_000,
            )],
        );
        assert_eq!(validate(&d), Ok(()));
    }

    #[test]
    fn refund_is_positive_expense() {
        let d = draft(
            TxnKind::Expense,
            vec![posting(
                account(AccountKind::Bank),
                Some(category(CategoryKind::Expense)),
                15_000,
            )],
        );
        assert_eq!(validate(&d), Ok(()));
    }

    #[test]
    fn split_expense_needs_category_on_every_posting() {
        let acc = account(AccountKind::Bank);
        let d = draft(
            TxnKind::Expense,
            vec![
                posting(acc, Some(category(CategoryKind::Expense)), -10_000),
                posting(acc, None, -5_000),
            ],
        );
        assert_eq!(validate(&d), Err(TxnError::MissingCategory));
    }

    #[test]
    fn income_rejects_expense_category() {
        let d = draft(
            TxnKind::Income,
            vec![posting(
                account(AccountKind::Bank),
                Some(category(CategoryKind::Expense)),
                1_000,
            )],
        );
        assert_eq!(
            validate(&d),
            Err(TxnError::WrongCategoryKind {
                expected: CategoryKind::Income
            })
        );
    }

    #[test]
    fn empty_and_zero_postings_rejected() {
        assert_eq!(
            validate(&draft(TxnKind::Adjustment, vec![])),
            Err(TxnError::NoPostings)
        );
        let zero = draft(
            TxnKind::Adjustment,
            vec![posting(account(AccountKind::Bank), None, 0)],
        );
        assert_eq!(validate(&zero), Err(TxnError::ZeroAmount));
    }

    #[test]
    fn balanced_transfer_is_valid() {
        let d = transfer(
            account(AccountKind::Bank),
            account(AccountKind::Ewallet),
            -100_000,
            100_000,
        );
        assert_eq!(validate(&d), Ok(()));
    }

    #[test]
    fn transfer_with_admin_fee_is_valid() {
        let bank = account(AccountKind::Bank);
        let mut d = transfer(bank, account(AccountKind::Ewallet), -100_000, 100_000);
        d.postings
            .push(posting(bank, Some(category(CategoryKind::Expense)), -1_000));
        assert_eq!(validate(&d), Ok(()));
    }

    #[test]
    fn transfer_fee_must_be_expense_category() {
        let bank = account(AccountKind::Bank);
        let mut d = transfer(bank, account(AccountKind::Ewallet), -100_000, 100_000);
        d.postings
            .push(posting(bank, Some(category(CategoryKind::Income)), -1_000));
        assert_eq!(
            validate(&d),
            Err(TxnError::WrongCategoryKind {
                expected: CategoryKind::Expense
            })
        );
    }

    #[test]
    fn unbalanced_transfer_rejected() {
        let d = transfer(
            account(AccountKind::Bank),
            account(AccountKind::Ewallet),
            -100_000,
            90_000,
        );
        assert_eq!(validate(&d), Err(TxnError::Unbalanced(-10_000)));
    }

    #[test]
    fn transfer_to_same_account_rejected() {
        let bank = account(AccountKind::Bank);
        assert_eq!(
            validate(&transfer(bank, bank, -1, 1)),
            Err(TxnError::TransferSameAccount)
        );
    }

    #[test]
    fn transfer_needs_exactly_two_legs() {
        let d = draft(
            TxnKind::Transfer,
            vec![posting(account(AccountKind::Bank), None, -1)],
        );
        assert_eq!(validate(&d), Err(TxnError::TransferLegCount(1)));
    }

    #[test]
    fn transfer_across_currencies_rejected_in_v1() {
        let usd = CurrencyCode::parse("USD").unwrap();
        let foreign = AccountRef {
            currency: usd,
            ..account(AccountKind::Bank)
        };
        let d = transfer(account(AccountKind::Bank), foreign, -1, 1);
        assert_eq!(validate(&d), Err(TxnError::CurrencyMismatch));
    }

    #[test]
    fn transfer_leg_above_max_is_rejected_not_overflowed() {
        let d = transfer(
            account(AccountKind::Bank),
            account(AccountKind::Ewallet),
            i64::MAX,
            1,
        );
        assert_eq!(validate(&d), Err(TxnError::AmountTooLarge));
    }

    #[test]
    fn valuation_only_for_investment() {
        let ok = draft(
            TxnKind::Valuation,
            vec![posting(account(AccountKind::Investment), None, 50_000)],
        );
        assert_eq!(validate(&ok), Ok(()));
        let bad = draft(
            TxnKind::Valuation,
            vec![posting(account(AccountKind::Bank), None, 50_000)],
        );
        assert_eq!(validate(&bad), Err(TxnError::ValuationNonInvestment));
    }

    #[test]
    fn adjustment_and_opening_have_no_category() {
        let cat = Some(category(CategoryKind::Expense));
        let adj = draft(
            TxnKind::Adjustment,
            vec![posting(account(AccountKind::Bank), cat, -1)],
        );
        assert_eq!(validate(&adj), Err(TxnError::UnexpectedCategory));
        let acc = account(AccountKind::Bank);
        let two = draft(
            TxnKind::Opening,
            vec![posting(acc, None, 1), posting(acc, None, 2)],
        );
        assert_eq!(validate(&two), Err(TxnError::OpeningPostingCount(2)));
    }

    #[test]
    fn amount_above_max_rejected() {
        let acc = account(AccountKind::Bank);
        let ok = draft(TxnKind::Adjustment, vec![posting(acc, None, -MAX_AMOUNT)]);
        assert_eq!(validate(&ok), Ok(()));
        for bad in [MAX_AMOUNT + 1, -MAX_AMOUNT - 1, i64::MAX, i64::MIN] {
            let d = draft(TxnKind::Adjustment, vec![posting(acc, None, bad)]);
            assert_eq!(validate(&d), Err(TxnError::AmountTooLarge), "{bad}");
        }
    }

    #[test]
    fn valid_txn_only_from_valid_draft() {
        let acc = account(AccountKind::Bank);
        let ok = draft(TxnKind::Adjustment, vec![posting(acc, None, 500)]);
        assert_eq!(ValidTxn::new(ok.clone()).unwrap().draft(), &ok);
        let bad = draft(TxnKind::Adjustment, vec![posting(acc, None, 0)]);
        assert_eq!(ValidTxn::new(bad), Err(TxnError::ZeroAmount));
    }

    proptest! {
        #[test]
        fn any_balanced_transfer_is_valid(x in 1i64..=MAX_AMOUNT) {
            let d = transfer(account(AccountKind::Bank), account(AccountKind::Ewallet), -x, x);
            prop_assert_eq!(validate(&d), Ok(()));
        }

        #[test]
        fn any_unbalanced_transfer_is_rejected(x in 1i64..=1_000_000_000_000, y in 1i64..=1_000_000_000_000) {
            prop_assume!(x != y);
            let d = transfer(account(AccountKind::Bank), account(AccountKind::Ewallet), -x, y);
            prop_assert_eq!(validate(&d), Err(TxnError::Unbalanced(y - x)));
        }
    }
}
