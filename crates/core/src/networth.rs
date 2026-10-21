//! Net worth: aset dikurangi liability, diturunkan dari saldo akun.

use crate::{AccountKind, MoneyError};

/// `liabilities` bernilai positif untuk utang (saldo liability negatif).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NetWorth {
    pub assets: i64,
    pub liabilities: i64,
    pub net: i64,
}

/// Hitung net worth dari pasangan (jenis akun, saldo). Pemanggil yang
/// menyaring akun `include_in_net_worth`.
pub fn net_worth(
    items: impl IntoIterator<Item = (AccountKind, i64)>,
) -> Result<NetWorth, MoneyError> {
    let (mut assets, mut liabilities) = (0i128, 0i128);
    for (kind, balance) in items {
        if kind.counts_as_liability(balance) {
            liabilities -= i128::from(balance);
        } else {
            assets += i128::from(balance);
        }
    }
    let fit = |v: i128| i64::try_from(v).map_err(|_| MoneyError::Overflow);
    Ok(NetWorth {
        assets: fit(assets)?,
        liabilities: fit(liabilities)?,
        net: fit(assets - liabilities)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_minus_liabilities() {
        let nw = net_worth([
            (AccountKind::Bank, 1_000_000),
            (AccountKind::Ewallet, 250_000),
            (AccountKind::CreditCard, -400_000),
            (AccountKind::Loan, -1_000_000),
        ])
        .unwrap();
        assert_eq!(
            nw,
            NetWorth {
                assets: 1_250_000,
                liabilities: 1_400_000,
                net: -150_000
            }
        );
    }

    #[test]
    fn negative_receivable_is_liability() {
        let nw = net_worth([
            (AccountKind::Bank, 1_000),
            (AccountKind::Receivable, 300),
            (AccountKind::Receivable, -200),
        ])
        .unwrap();
        assert_eq!(
            nw,
            NetWorth {
                assets: 1_300,
                liabilities: 200,
                net: 1_100
            }
        );
    }

    #[test]
    fn overpaid_credit_card_reduces_liabilities() {
        let nw = net_worth([(AccountKind::Bank, 100), (AccountKind::CreditCard, 50)]).unwrap();
        assert_eq!(
            nw,
            NetWorth {
                assets: 100,
                liabilities: -50,
                net: 150
            }
        );
    }

    #[test]
    fn empty_is_zero() {
        assert_eq!(net_worth([]).unwrap(), NetWorth::default());
    }

    #[test]
    fn overflow_is_error() {
        let err = net_worth([(AccountKind::Bank, i64::MAX), (AccountKind::Bank, i64::MAX)]);
        assert_eq!(err, Err(MoneyError::Overflow));
    }
}
