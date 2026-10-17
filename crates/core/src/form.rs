//! Isi form transaksi (Income/Expense/Transfer) → posting bertanda.
//!
//! User mengetik nominal positif; tanda ditentukan jenis transaksi. Refund = expense dengan
//! nominal negatif. Hasilnya tetap harus lolos [`crate::validate`] setelah ref di-resolve.

use uuid::Uuid;

use crate::{TxnError, TxnKind};

/// Satu baris kategori. Mode non-split = satu baris berisi seluruh nominal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitLine {
    pub category_id: Uuid,
    pub amount: i64,
    pub memo: Option<String>,
}

/// Biaya admin transfer, dibebankan ke akun asal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fee {
    pub amount: i64,
    pub category_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormBody {
    Income {
        account_id: Uuid,
        amount: i64,
        splits: Vec<SplitLine>,
    },
    Expense {
        account_id: Uuid,
        amount: i64,
        splits: Vec<SplitLine>,
    },
    Transfer {
        from: Uuid,
        to: Uuid,
        amount: i64,
        fee: Option<Fee>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostingSpec {
    pub account_id: Uuid,
    pub category_id: Option<Uuid>,
    pub amount: i64,
    pub memo: Option<String>,
}

impl FormBody {
    pub fn kind(&self) -> TxnKind {
        match self {
            FormBody::Income { .. } => TxnKind::Income,
            FormBody::Expense { .. } => TxnKind::Expense,
            FormBody::Transfer { .. } => TxnKind::Transfer,
        }
    }
}

pub fn form_postings(body: &FormBody) -> Result<Vec<PostingSpec>, TxnError> {
    match body {
        FormBody::Income {
            account_id,
            amount,
            splits,
        } => split_postings(*account_id, *amount, splits, 1),
        FormBody::Expense {
            account_id,
            amount,
            splits,
        } => split_postings(*account_id, *amount, splits, -1),
        FormBody::Transfer {
            from,
            to,
            amount,
            fee,
        } => {
            if *amount <= 0 {
                return Err(TxnError::NotPositive);
            }
            let leg = |account_id, amount| PostingSpec {
                account_id,
                category_id: None,
                amount,
                memo: None,
            };
            let mut postings = vec![leg(*from, -*amount), leg(*to, *amount)];
            if let Some(fee) = fee {
                if fee.amount <= 0 {
                    return Err(TxnError::NotPositive);
                }
                postings.push(PostingSpec {
                    account_id: *from,
                    category_id: Some(fee.category_id),
                    amount: -fee.amount,
                    memo: None,
                });
            }
            Ok(postings)
        }
    }
}

/// `sign` = +1 untuk income, -1 untuk expense.
fn split_postings(
    account_id: Uuid,
    amount: i64,
    splits: &[SplitLine],
    sign: i64,
) -> Result<Vec<PostingSpec>, TxnError> {
    if amount == 0 {
        return Err(TxnError::ZeroAmount);
    }
    if splits.is_empty() {
        return Err(TxnError::MissingCategory);
    }
    if splits.iter().any(|s| s.amount == 0) {
        return Err(TxnError::ZeroAmount);
    }
    let total = splits
        .iter()
        .try_fold(0i64, |acc, s| acc.checked_add(s.amount))
        .ok_or(TxnError::Overflow)?;
    if total != amount {
        let diff = amount.checked_sub(total).ok_or(TxnError::Overflow)?;
        return Err(TxnError::SplitMismatch(diff));
    }
    splits
        .iter()
        .map(|s| {
            Ok(PostingSpec {
                account_id,
                category_id: Some(s.category_id),
                amount: s.amount.checked_mul(sign).ok_or(TxnError::Overflow)?,
                memo: s.memo.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountKind, AccountRef, CategoryKind, CategoryRef, CurrencyCode, PostingDraft};
    use crate::{TxnDraft, validate};
    use proptest::prelude::*;
    use time::macros::date;

    fn line(category_id: Uuid, amount: i64) -> SplitLine {
        SplitLine {
            category_id,
            amount,
            memo: None,
        }
    }

    fn amounts(p: &[PostingSpec]) -> Vec<i64> {
        p.iter().map(|p| p.amount).collect()
    }

    #[test]
    fn expense_negates_each_split() {
        let (acc, a, b) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
        let body = FormBody::Expense {
            account_id: acc,
            amount: 75_000,
            splits: vec![line(a, 50_000), line(b, 25_000)],
        };
        let p = form_postings(&body).unwrap();
        assert_eq!(amounts(&p), vec![-50_000, -25_000]);
        assert_eq!(p[0].category_id, Some(a));
        assert_eq!(p[1].category_id, Some(b));
        assert!(p.iter().all(|p| p.account_id == acc));
        assert_eq!(body.kind(), TxnKind::Expense);
    }

    #[test]
    fn income_keeps_sign() {
        let c = Uuid::now_v7();
        let body = FormBody::Income {
            account_id: Uuid::now_v7(),
            amount: 1_000,
            splits: vec![line(c, 1_000)],
        };
        assert_eq!(amounts(&form_postings(&body).unwrap()), vec![1_000]);
    }

    #[test]
    fn refund_is_negative_expense() {
        let c = Uuid::now_v7();
        let body = FormBody::Expense {
            account_id: Uuid::now_v7(),
            amount: -20_000,
            splits: vec![line(c, -20_000)],
        };
        assert_eq!(amounts(&form_postings(&body).unwrap()), vec![20_000]);
    }

    #[test]
    fn split_total_must_match() {
        let body = FormBody::Expense {
            account_id: Uuid::now_v7(),
            amount: 10_000,
            splits: vec![line(Uuid::now_v7(), 6_000), line(Uuid::now_v7(), 3_999)],
        };
        assert_eq!(form_postings(&body), Err(TxnError::SplitMismatch(1)));
    }

    #[test]
    fn empty_splits_missing_category() {
        let body = FormBody::Expense {
            account_id: Uuid::now_v7(),
            amount: 10_000,
            splits: vec![],
        };
        assert_eq!(form_postings(&body), Err(TxnError::MissingCategory));
    }

    #[test]
    fn zero_split_line_rejected() {
        let body = FormBody::Expense {
            account_id: Uuid::now_v7(),
            amount: 10_000,
            splits: vec![line(Uuid::now_v7(), 10_000), line(Uuid::now_v7(), 0)],
        };
        assert_eq!(form_postings(&body), Err(TxnError::ZeroAmount));
        let zero = FormBody::Income {
            account_id: Uuid::now_v7(),
            amount: 0,
            splits: vec![line(Uuid::now_v7(), 0)],
        };
        assert_eq!(form_postings(&zero), Err(TxnError::ZeroAmount));
    }

    #[test]
    fn transfer_with_fee() {
        let (a, b, f) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
        let body = FormBody::Transfer {
            from: a,
            to: b,
            amount: 100_000,
            fee: Some(Fee {
                amount: 6_500,
                category_id: f,
            }),
        };
        let p = form_postings(&body).unwrap();
        let shape: Vec<_> = p
            .iter()
            .map(|p| (p.account_id, p.category_id, p.amount))
            .collect();
        assert_eq!(
            shape,
            vec![
                (a, None, -100_000),
                (b, None, 100_000),
                (a, Some(f), -6_500)
            ]
        );
    }

    #[test]
    fn transfer_requires_positive_amount_and_fee() {
        let (a, b) = (Uuid::now_v7(), Uuid::now_v7());
        for amount in [0, -5] {
            let body = FormBody::Transfer {
                from: a,
                to: b,
                amount,
                fee: None,
            };
            assert_eq!(form_postings(&body), Err(TxnError::NotPositive));
        }
        let body = FormBody::Transfer {
            from: a,
            to: b,
            amount: 1_000,
            fee: Some(Fee {
                amount: 0,
                category_id: Uuid::now_v7(),
            }),
        };
        assert_eq!(form_postings(&body), Err(TxnError::NotPositive));
    }

    #[test]
    fn split_sum_overflow_is_error() {
        let body = FormBody::Expense {
            account_id: Uuid::now_v7(),
            amount: 1,
            splits: vec![line(Uuid::now_v7(), i64::MAX), line(Uuid::now_v7(), 1)],
        };
        assert_eq!(form_postings(&body), Err(TxnError::Overflow));
    }

    proptest! {
        #[test]
        fn form_postings_validate(parts in prop::collection::vec(1i64..=10_000_000, 1..=5)) {
            let account = AccountRef { id: Uuid::now_v7(), kind: AccountKind::Bank, currency: CurrencyCode::IDR };
            let total: i64 = parts.iter().sum();
            let splits: Vec<_> = parts.iter().map(|&a| line(Uuid::now_v7(), a)).collect();
            let body = FormBody::Expense { account_id: account.id, amount: total, splits };
            let specs = form_postings(&body).unwrap();
            let draft = TxnDraft {
                kind: body.kind(),
                date: date!(2026 - 10 - 17),
                note: None,
                postings: specs.iter().map(|s| PostingDraft {
                    account,
                    category: s.category_id.map(|id| CategoryRef { id, kind: CategoryKind::Expense }),
                    amount: s.amount,
                    memo: s.memo.clone(),
                }).collect(),
            };
            prop_assert_eq!(validate(&draft), Ok(()));
            prop_assert_eq!(specs.iter().map(|s| s.amount).sum::<i64>(), -total);
        }
    }
}
