//! Uang sebagai integer satuan minor. Tidak pernah floating point.

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    #[error("invalid currency code: {0:?}")]
    InvalidCurrency(String),
    #[error("currency mismatch: {0} vs {1}")]
    CurrencyMismatch(CurrencyCode, CurrencyCode),
    #[error("amount overflow")]
    Overflow,
}

/// Kode ISO 4217 tiga huruf kapital.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CurrencyCode([u8; 3]);

impl CurrencyCode {
    pub const IDR: CurrencyCode = CurrencyCode(*b"IDR");

    pub fn parse(s: &str) -> Result<Self, MoneyError> {
        match *s.as_bytes() {
            [a, b, c] if [a, b, c].iter().all(u8::is_ascii_uppercase) => Ok(Self([a, b, c])),
            _ => Err(MoneyError::InvalidCurrency(s.to_owned())),
        }
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("currency code is ASCII")
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<String> for CurrencyCode {
    type Error = MoneyError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl From<CurrencyCode> for String {
    fn from(c: CurrencyCode) -> Self {
        c.as_str().to_owned()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    pub amount: i64,
    pub currency: CurrencyCode,
}

impl Money {
    pub const fn new(amount: i64, currency: CurrencyCode) -> Self {
        Self { amount, currency }
    }

    pub const fn idr(amount: i64) -> Self {
        Self::new(amount, CurrencyCode::IDR)
    }

    pub fn checked_add(self, other: Money) -> Result<Money, MoneyError> {
        if self.currency != other.currency {
            return Err(MoneyError::CurrencyMismatch(self.currency, other.currency));
        }
        let amount = self
            .amount
            .checked_add(other.amount)
            .ok_or(MoneyError::Overflow)?;
        Ok(Money::new(amount, self.currency))
    }

    pub fn checked_neg(self) -> Result<Money, MoneyError> {
        let amount = self.amount.checked_neg().ok_or(MoneyError::Overflow)?;
        Ok(Money::new(amount, self.currency))
    }

    pub fn is_zero(&self) -> bool {
        self.amount == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_uppercase_iso_code() {
        assert_eq!(CurrencyCode::parse("IDR").unwrap(), CurrencyCode::IDR);
        assert_eq!(CurrencyCode::IDR.as_str(), "IDR");
    }

    #[test]
    fn parse_rejects_invalid_codes() {
        for bad in ["idr", "ID", "IDRX", "I1R", ""] {
            assert!(
                CurrencyCode::parse(bad).is_err(),
                "{bad} should be rejected"
            );
        }
    }

    #[test]
    fn add_same_currency() {
        let sum = Money::idr(25_000).checked_add(Money::idr(12_500)).unwrap();
        assert_eq!(sum, Money::idr(37_500));
    }

    #[test]
    fn add_different_currency_fails() {
        let usd = CurrencyCode::parse("USD").unwrap();
        let err = Money::idr(1).checked_add(Money::new(1, usd)).unwrap_err();
        assert_eq!(err, MoneyError::CurrencyMismatch(CurrencyCode::IDR, usd));
    }

    #[test]
    fn add_overflow_fails() {
        let err = Money::idr(i64::MAX).checked_add(Money::idr(1)).unwrap_err();
        assert_eq!(err, MoneyError::Overflow);
    }

    #[test]
    fn neg_min_overflows() {
        assert_eq!(
            Money::idr(i64::MIN).checked_neg().unwrap_err(),
            MoneyError::Overflow
        );
        assert_eq!(Money::idr(5).checked_neg().unwrap(), Money::idr(-5));
    }

    #[test]
    fn currency_serializes_as_string() {
        let json = serde_json::to_string(&Money::idr(10)).unwrap();
        assert_eq!(json, r#"{"amount":10,"currency":"IDR"}"#);
    }
}
