//! Pemetaan error domain ke `IpcError` yang dikirim ke UI.

use nabungo_app::AppError;
use nabungo_app::{AmountParseError, TxnError};
use serde::Serialize;

/// Error yang diterima UI. Detail teknis tidak dikirim; UI menerjemahkan `code`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct IpcError {
    pub code: ErrorCode,
    /// Nama entitas untuk `not_found`/`archived`.
    pub entity: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NotFound,
    Archived,
    OpeningNotAllowed,
    TxnNoPostings,
    TxnZeroAmount,
    TxnAmountTooLarge,
    TxnMissingCategory,
    TxnWrongCategoryKind,
    TxnUnexpectedCategory,
    TxnTransferLegCount,
    TxnTransferSameAccount,
    TxnCurrencyMismatch,
    TxnUnbalanced,
    TxnOverflow,
    TxnValuationNonInvestment,
    TxnOpeningPostingCount,
    AmountEmpty,
    AmountInvalid,
    AmountFractional,
    AmountTooLarge,
    Internal,
}

impl IpcError {
    fn code(code: ErrorCode) -> Self {
        Self { code, entity: None }
    }

    fn with_entity(code: ErrorCode, entity: &str) -> Self {
        Self {
            code,
            entity: Some(entity.to_owned()),
        }
    }
}

// Sengaja tanpa wildcard: varian baru wajib dipetakan di sini.
impl From<TxnError> for ErrorCode {
    fn from(e: TxnError) -> Self {
        match e {
            TxnError::NoPostings => ErrorCode::TxnNoPostings,
            TxnError::ZeroAmount => ErrorCode::TxnZeroAmount,
            TxnError::AmountTooLarge => ErrorCode::TxnAmountTooLarge,
            TxnError::MissingCategory => ErrorCode::TxnMissingCategory,
            TxnError::WrongCategoryKind { .. } => ErrorCode::TxnWrongCategoryKind,
            TxnError::UnexpectedCategory => ErrorCode::TxnUnexpectedCategory,
            TxnError::TransferLegCount(_) => ErrorCode::TxnTransferLegCount,
            TxnError::TransferSameAccount => ErrorCode::TxnTransferSameAccount,
            TxnError::CurrencyMismatch => ErrorCode::TxnCurrencyMismatch,
            TxnError::Unbalanced(_) => ErrorCode::TxnUnbalanced,
            TxnError::Overflow => ErrorCode::TxnOverflow,
            TxnError::ValuationNonInvestment => ErrorCode::TxnValuationNonInvestment,
            TxnError::OpeningPostingCount(_) => ErrorCode::TxnOpeningPostingCount,
        }
    }
}

impl From<AppError> for IpcError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::Storage(err) => {
                tracing::error!(error = %err, "storage error");
                Self::code(ErrorCode::Internal)
            }
            AppError::Sqlite(err) => {
                tracing::error!(error = %err, "sqlite error");
                Self::code(ErrorCode::Internal)
            }
            AppError::Txn(err) => Self::code(err.into()),
            AppError::NotFound(entity) => Self::with_entity(ErrorCode::NotFound, entity),
            AppError::Archived(entity) => Self::with_entity(ErrorCode::Archived, entity),
            AppError::OpeningNotAllowed => Self::code(ErrorCode::OpeningNotAllowed),
            // Sementara; kode spesifik ditambahkan bersama command akun/kategori.
            AppError::Catalog(_)
            | AppError::Money(_)
            | AppError::InUse(_)
            | AppError::Duplicate(_)
            | AppError::AlreadyOnboarded => Self::code(ErrorCode::Internal),
        }
    }
}

impl From<AmountParseError> for IpcError {
    fn from(e: AmountParseError) -> Self {
        Self::code(match e {
            AmountParseError::Empty => ErrorCode::AmountEmpty,
            AmountParseError::Invalid => ErrorCode::AmountInvalid,
            AmountParseError::Fractional => ErrorCode::AmountFractional,
            AmountParseError::TooLarge => ErrorCode::AmountTooLarge,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_maps_with_entity() {
        let e = IpcError::from(AppError::NotFound("account"));
        assert_eq!(e.code, ErrorCode::NotFound);
        assert_eq!(e.entity.as_deref(), Some("account"));
    }

    #[test]
    fn txn_error_maps_to_specific_code() {
        let e = IpcError::from(AppError::Txn(TxnError::TransferSameAccount));
        assert_eq!(e.code, ErrorCode::TxnTransferSameAccount);
        assert_eq!(e.entity, None);
    }

    #[test]
    fn amount_error_maps() {
        let e = IpcError::from(AmountParseError::Fractional);
        assert_eq!(e.code, ErrorCode::AmountFractional);
    }

    #[test]
    fn error_serializes_snake_case() {
        let e = IpcError::from(AppError::Txn(TxnError::ZeroAmount));
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            serde_json::json!({ "code": "txn_zero_amount", "entity": null })
        );
    }
}
