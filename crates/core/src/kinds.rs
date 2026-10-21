//! Jenis akun, kategori, dan transaksi. String-nya harus sama dengan CHECK di skema SQL.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Cash,
    Bank,
    Ewallet,
    Investment,
    Receivable,
    CreditCard,
    Loan,
}

impl AccountKind {
    pub const ALL: [AccountKind; 7] = [
        Self::Cash,
        Self::Bank,
        Self::Ewallet,
        Self::Investment,
        Self::Receivable,
        Self::CreditCard,
        Self::Loan,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cash => "cash",
            Self::Bank => "bank",
            Self::Ewallet => "ewallet",
            Self::Investment => "investment",
            Self::Receivable => "receivable",
            Self::CreditCard => "credit_card",
            Self::Loan => "loan",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }

    pub fn is_liability(self) -> bool {
        matches!(self, Self::CreditCard | Self::Loan)
    }

    /// Klasifikasi saldo untuk net worth & alokasi: piutang/hutang bersaldo negatif
    /// berarti user berutang, jadi dihitung sebagai liabilitas (D10).
    pub fn counts_as_liability(self, balance: i64) -> bool {
        self.is_liability() || (self == Self::Receivable && balance < 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryKind {
    Income,
    Expense,
}

impl CategoryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Income => "income",
            Self::Expense => "expense",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [Self::Income, Self::Expense]
            .into_iter()
            .find(|k| k.as_str() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TxnKind {
    Income,
    Expense,
    Transfer,
    Adjustment,
    Valuation,
    Opening,
}

impl TxnKind {
    pub const ALL: [TxnKind; 6] = [
        Self::Income,
        Self::Expense,
        Self::Transfer,
        Self::Adjustment,
        Self::Valuation,
        Self::Opening,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Income => "income",
            Self::Expense => "expense",
            Self::Transfer => "transfer",
            Self::Adjustment => "adjustment",
            Self::Valuation => "valuation",
            Self::Opening => "opening",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_kind_roundtrip_matches_schema() {
        let names: Vec<_> = AccountKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(
            names,
            [
                "cash",
                "bank",
                "ewallet",
                "investment",
                "receivable",
                "credit_card",
                "loan"
            ]
        );
        for k in AccountKind::ALL {
            assert_eq!(AccountKind::parse(k.as_str()), Some(k));
        }
        assert_eq!(AccountKind::parse("savings"), None);
    }

    #[test]
    fn only_credit_card_and_loan_are_liabilities() {
        let liabilities: Vec<_> = AccountKind::ALL
            .into_iter()
            .filter(|k| k.is_liability())
            .collect();
        assert_eq!(liabilities, [AccountKind::CreditCard, AccountKind::Loan]);
    }

    #[test]
    fn counts_as_liability_by_kind_and_sign() {
        assert!(AccountKind::Loan.counts_as_liability(100));
        assert!(AccountKind::CreditCard.counts_as_liability(0));
        assert!(AccountKind::Receivable.counts_as_liability(-1));
        assert!(!AccountKind::Receivable.counts_as_liability(0));
        assert!(!AccountKind::Receivable.counts_as_liability(5));
        assert!(!AccountKind::Bank.counts_as_liability(-5));
    }

    #[test]
    fn txn_kind_roundtrip_matches_schema() {
        let names: Vec<_> = TxnKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(
            names,
            [
                "income",
                "expense",
                "transfer",
                "adjustment",
                "valuation",
                "opening"
            ]
        );
        for k in TxnKind::ALL {
            assert_eq!(TxnKind::parse(k.as_str()), Some(k));
        }
    }

    #[test]
    fn category_kind_roundtrip() {
        assert_eq!(CategoryKind::parse("income"), Some(CategoryKind::Income));
        assert_eq!(CategoryKind::parse("expense"), Some(CategoryKind::Expense));
        assert_eq!(CategoryKind::Expense.as_str(), "expense");
        assert_eq!(CategoryKind::parse("transfer"), None);
    }
}
