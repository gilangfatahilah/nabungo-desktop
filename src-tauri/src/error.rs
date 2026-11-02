//! Pemetaan error domain ke `IpcError` yang dikirim ke UI.

use nabungo_app::{
    AllocationError, AmountParseError, BudgetError, CatalogError, PeriodError, RecurrenceError,
    TxnError,
};
use nabungo_app::{AppError, RuleField, SourceError, SyncError};
use serde::Serialize;

/// Error yang diterima UI. Detail teknis tidak dikirim; UI menerjemahkan `code`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct IpcError {
    pub code: ErrorCode,
    /// Nama entitas untuk `not_found`/`archived`/`in_use`/`duplicate`/`invalid_id`.
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
    TxnSplitMismatch,
    TxnNotPositive,
    AmountEmpty,
    AmountInvalid,
    AmountFractional,
    AmountTooLarge,
    InUse,
    Duplicate,
    AlreadyOnboarded,
    InvalidId,
    InvalidDate,
    InvalidStartDay,
    NameEmpty,
    NameTooLong,
    InvalidStatementDay,
    InvalidCreditLimit,
    CategoryDepth,
    CategoryKindMismatch,
    CategoryHasChildren,
    MergeIntoSelf,
    InvalidPeriod,
    BudgetNegative,
    BudgetTooLarge,
    BudgetIncomeCategory,
    ValuationUnchanged,
    TxnNotEditable,
    AllocationInvalidBp,
    AllocationTargetsExceed,
    AllocationSharesExceed,
    AllocationDepth,
    AllocationLiability,
    AllocationDuplicateBucket,
    AllocationInvalidAmount,
    BackupInvalid,
    BackupTooNew,
    BackupTargetInvalid,
    BackupKeyRequired,
    AttachmentType,
    AttachmentTooLarge,
    AttachmentMissing,
    OccurrenceStale,
    CaptureNotPending,
    CaptureNotIgnored,
    /// `entity` = field rule yang salah (`name`, `sender`, `amount`, ...).
    EmailRuleInvalid,
    KeychainUnavailable,
    EmailAddressInvalid,
    EmailMailboxEmpty,
    EmailNotConfigured,
    EmailAuthFailed,
    EmailMailboxNotFound,
    EmailConnection,
    EmailSyncRunning,
    EmailSyncInterrupted,
    UpdateNetwork,
    UpdateNotFound,
    UpdateNoAsset,
    UpdateChecksum,
    UpdateCancelled,
    /// Unduhan update lain sedang berjalan.
    UpdateBusy,
    RecurrenceInterval,
    RecurrenceUntil,
    FileIo,
    Locked,
    WrongPassword,
    InvalidRecoveryKey,
    PasswordTooShort,
    KeyFileInvalid,
    EncryptionState,
    InvalidAutoLock,
    InvalidFontScale,
    DatabaseUnreadable,
    DatabaseTooNew,
    /// DB gagal dibuka saat startup; pulihkan dari backup dulu (P1).
    RecoveryRequired,
    /// Command pemulihan dipanggil saat DB sedang terbuka.
    RecoveryUnavailable,
    Internal,
}

impl IpcError {
    /// ID dari UI tidak bisa di-parse sebagai UUID.
    pub fn invalid_id(entity: &str) -> Self {
        Self::with_entity(ErrorCode::InvalidId, entity)
    }

    pub fn invalid_date() -> Self {
        Self::code(ErrorCode::InvalidDate)
    }

    pub fn code(code: ErrorCode) -> Self {
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
            TxnError::SplitMismatch(_) => ErrorCode::TxnSplitMismatch,
            TxnError::NotPositive => ErrorCode::TxnNotPositive,
        }
    }
}

impl From<CatalogError> for ErrorCode {
    fn from(e: CatalogError) -> Self {
        match e {
            CatalogError::EmptyName => ErrorCode::NameEmpty,
            CatalogError::NameTooLong => ErrorCode::NameTooLong,
            CatalogError::InvalidStatementDay(_) => ErrorCode::InvalidStatementDay,
            CatalogError::InvalidCreditLimit => ErrorCode::InvalidCreditLimit,
            CatalogError::CategoryDepth => ErrorCode::CategoryDepth,
            CatalogError::CategoryKindMismatch => ErrorCode::CategoryKindMismatch,
            CatalogError::CategoryHasChildren => ErrorCode::CategoryHasChildren,
            CatalogError::MergeIntoSelf => ErrorCode::MergeIntoSelf,
        }
    }
}

impl From<PeriodError> for ErrorCode {
    fn from(e: PeriodError) -> Self {
        match e {
            PeriodError::InvalidStartDay(_) => ErrorCode::InvalidStartDay,
            PeriodError::InvalidMonth(_) | PeriodError::InvalidKey => ErrorCode::InvalidPeriod,
        }
    }
}

impl From<PeriodError> for IpcError {
    fn from(e: PeriodError) -> Self {
        Self::code(e.into())
    }
}

impl From<BudgetError> for ErrorCode {
    fn from(e: BudgetError) -> Self {
        match e {
            BudgetError::Negative => ErrorCode::BudgetNegative,
            BudgetError::TooLarge => ErrorCode::BudgetTooLarge,
            BudgetError::IncomeCategory => ErrorCode::BudgetIncomeCategory,
        }
    }
}

impl From<AllocationError> for ErrorCode {
    fn from(e: AllocationError) -> Self {
        match e {
            AllocationError::InvalidBasisPoints => ErrorCode::AllocationInvalidBp,
            AllocationError::TargetsExceed { .. } => ErrorCode::AllocationTargetsExceed,
            AllocationError::SharesExceed { .. } => ErrorCode::AllocationSharesExceed,
            AllocationError::Depth => ErrorCode::AllocationDepth,
            AllocationError::LiabilityAccount => ErrorCode::AllocationLiability,
            AllocationError::DuplicateBucket => ErrorCode::AllocationDuplicateBucket,
            AllocationError::InvalidAmount => ErrorCode::AllocationInvalidAmount,
        }
    }
}

/// Nama field rule di form (snake_case, sama dengan `entity` di UI).
fn rule_field(field: RuleField) -> &'static str {
    match field {
        RuleField::Name => "name",
        RuleField::Sender => "sender",
        RuleField::Subject => "subject",
        RuleField::Amount => "amount",
        RuleField::Payee => "payee",
        RuleField::Date => "date",
        RuleField::DateFormat => "date_format",
        RuleField::CounterAccount => "counter_account",
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
            AppError::Catalog(err) => Self::code(err.into()),
            AppError::Money(err) => {
                tracing::error!(error = %err, "money error");
                Self::code(ErrorCode::Internal)
            }
            AppError::InUse(entity) => Self::with_entity(ErrorCode::InUse, entity),
            AppError::Duplicate(entity) => Self::with_entity(ErrorCode::Duplicate, entity),
            AppError::AlreadyOnboarded => Self::code(ErrorCode::AlreadyOnboarded),
            AppError::Budget(err) => Self::code(err.into()),
            AppError::Period(err) => Self::code(err.into()),
            AppError::ValuationUnchanged => Self::code(ErrorCode::ValuationUnchanged),
            AppError::NotEditable => Self::code(ErrorCode::TxnNotEditable),
            AppError::Allocation(err) => Self::code(err.into()),
            AppError::InvalidBackup => Self::code(ErrorCode::BackupInvalid),
            AppError::BackupKeyRequired => Self::code(ErrorCode::BackupKeyRequired),
            AppError::DatabaseTooNew => Self::code(ErrorCode::DatabaseTooNew),
            AppError::BackupTooNew => Self::code(ErrorCode::BackupTooNew),
            AppError::BackupTargetInvalid => Self::code(ErrorCode::BackupTargetInvalid),
            AppError::AttachmentType => Self::code(ErrorCode::AttachmentType),
            AppError::AttachmentTooLarge => Self::code(ErrorCode::AttachmentTooLarge),
            AppError::AttachmentMissing => Self::code(ErrorCode::AttachmentMissing),
            AppError::OccurrenceStale => Self::code(ErrorCode::OccurrenceStale),
            AppError::CaptureNotPending => Self::code(ErrorCode::CaptureNotPending),
            AppError::CaptureNotIgnored => Self::code(ErrorCode::CaptureNotIgnored),
            AppError::EmailRuleInvalid(field) => {
                Self::with_entity(ErrorCode::EmailRuleInvalid, rule_field(field))
            }
            AppError::KeychainUnavailable => Self::code(ErrorCode::KeychainUnavailable),
            AppError::EmailAddressInvalid => Self::code(ErrorCode::EmailAddressInvalid),
            AppError::EmailMailboxEmpty => Self::code(ErrorCode::EmailMailboxEmpty),
            AppError::EmailNotConfigured => Self::code(ErrorCode::EmailNotConfigured),
            AppError::Recurrence(err) => Self::code(match err {
                RecurrenceError::Interval => ErrorCode::RecurrenceInterval,
                RecurrenceError::UntilBeforeStart => ErrorCode::RecurrenceUntil,
                RecurrenceError::Invalid(rrule) => {
                    tracing::error!(%rrule, "invalid stored rrule");
                    ErrorCode::Internal
                }
            }),
            AppError::Io(err) => {
                tracing::error!(error = %err, "io error");
                Self::code(ErrorCode::FileIo)
            }
            AppError::Locked => Self::code(ErrorCode::Locked),
            AppError::WrongPassword => Self::code(ErrorCode::WrongPassword),
            AppError::InvalidRecoveryKey => Self::code(ErrorCode::InvalidRecoveryKey),
            AppError::PasswordTooShort => Self::code(ErrorCode::PasswordTooShort),
            AppError::KeyFileInvalid => Self::code(ErrorCode::KeyFileInvalid),
            AppError::EncryptionState => Self::code(ErrorCode::EncryptionState),
            AppError::InvalidAutoLock => Self::code(ErrorCode::InvalidAutoLock),
            AppError::InvalidFontScale => Self::code(ErrorCode::InvalidFontScale),
            AppError::DatabaseUnreadable => Self::code(ErrorCode::DatabaseUnreadable),
            AppError::UpdateNetwork => Self::code(ErrorCode::UpdateNetwork),
            AppError::UpdateNotFound => Self::code(ErrorCode::UpdateNotFound),
            AppError::UpdateNoAsset => Self::code(ErrorCode::UpdateNoAsset),
            AppError::UpdateChecksum => Self::code(ErrorCode::UpdateChecksum),
            AppError::UpdateCancelled => Self::code(ErrorCode::UpdateCancelled),
        }
    }
}

impl From<SyncError> for IpcError {
    fn from(e: SyncError) -> Self {
        match e {
            SyncError::Source(SourceError::Auth) => Self::code(ErrorCode::EmailAuthFailed),
            SyncError::Source(SourceError::Mailbox) => Self::code(ErrorCode::EmailMailboxNotFound),
            SyncError::Source(SourceError::Connection(detail)) => {
                tracing::warn!(%detail, "koneksi email gagal");
                Self::code(ErrorCode::EmailConnection)
            }
            SyncError::Locked => Self::code(ErrorCode::Locked),
            SyncError::Interrupted => Self::code(ErrorCode::EmailSyncInterrupted),
            SyncError::App(e) => e.into(),
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
    fn security_errors_map() {
        for (e, code) in [
            (AppError::Locked, ErrorCode::Locked),
            (AppError::WrongPassword, ErrorCode::WrongPassword),
            (AppError::InvalidRecoveryKey, ErrorCode::InvalidRecoveryKey),
            (AppError::PasswordTooShort, ErrorCode::PasswordTooShort),
            (AppError::KeyFileInvalid, ErrorCode::KeyFileInvalid),
            (AppError::EncryptionState, ErrorCode::EncryptionState),
            (AppError::InvalidAutoLock, ErrorCode::InvalidAutoLock),
            (AppError::InvalidFontScale, ErrorCode::InvalidFontScale),
            (AppError::DatabaseUnreadable, ErrorCode::DatabaseUnreadable),
        ] {
            assert_eq!(IpcError::from(e).code, code);
        }
    }

    #[test]
    fn backup_errors_map() {
        assert_eq!(
            IpcError::from(AppError::InvalidBackup).code,
            ErrorCode::BackupInvalid
        );
        assert_eq!(
            IpcError::from(AppError::BackupTooNew).code,
            ErrorCode::BackupTooNew
        );
        assert_eq!(
            IpcError::from(AppError::BackupTargetInvalid).code,
            ErrorCode::BackupTargetInvalid
        );
        let io = AppError::Io(std::io::Error::other("disk penuh"));
        assert_eq!(IpcError::from(io).code, ErrorCode::FileIo);
    }

    #[test]
    fn attachment_errors_map_to_codes() {
        let cases = [
            (
                AppError::AttachmentType,
                ErrorCode::AttachmentType,
                "attachment_type",
            ),
            (
                AppError::AttachmentTooLarge,
                ErrorCode::AttachmentTooLarge,
                "attachment_too_large",
            ),
            (
                AppError::AttachmentMissing,
                ErrorCode::AttachmentMissing,
                "attachment_missing",
            ),
        ];
        for (err, code, wire) in cases {
            assert_eq!(IpcError::from(err).code, code);
            assert_eq!(serde_json::to_value(code).unwrap(), wire);
        }
    }

    #[test]
    fn email_rule_invalid_carries_field() {
        let e = IpcError::from(AppError::EmailRuleInvalid(RuleField::DateFormat));
        assert_eq!(e.code, ErrorCode::EmailRuleInvalid);
        assert_eq!(e.entity.as_deref(), Some("date_format"));
        assert_eq!(serde_json::to_value(e.code).unwrap(), "email_rule_invalid");
    }

    #[test]
    fn capture_not_ignored_maps_to_code() {
        let e = IpcError::from(AppError::CaptureNotIgnored);
        assert_eq!(e.code, ErrorCode::CaptureNotIgnored);
        assert_eq!(serde_json::to_value(e.code).unwrap(), "capture_not_ignored");
    }

    #[test]
    fn capture_not_pending_maps_to_code() {
        let e = IpcError::from(AppError::CaptureNotPending);
        assert_eq!(e.code, ErrorCode::CaptureNotPending);
        assert_eq!(serde_json::to_value(e.code).unwrap(), "capture_not_pending");
    }

    #[test]
    fn sync_errors_map_to_codes() {
        for (e, code) in [
            (SyncError::Source(SourceError::Auth), "email_auth_failed"),
            (
                SyncError::Source(SourceError::Mailbox),
                "email_mailbox_not_found",
            ),
            (
                SyncError::Source(SourceError::Connection("timeout".into())),
                "email_connection",
            ),
            (SyncError::Locked, "locked"),
            (SyncError::Interrupted, "email_sync_interrupted"),
            (
                SyncError::App(AppError::EmailNotConfigured),
                "email_not_configured",
            ),
        ] {
            let e = IpcError::from(e);
            assert_eq!(serde_json::to_value(e.code).unwrap(), code);
        }
    }

    #[test]
    fn email_app_errors_map_to_codes() {
        for (e, code) in [
            (AppError::KeychainUnavailable, "keychain_unavailable"),
            (AppError::EmailAddressInvalid, "email_address_invalid"),
            (AppError::EmailMailboxEmpty, "email_mailbox_empty"),
            (AppError::EmailNotConfigured, "email_not_configured"),
        ] {
            let e = IpcError::from(e);
            assert_eq!(serde_json::to_value(e.code).unwrap(), code);
        }
    }

    #[test]
    fn update_errors_map_to_codes() {
        for (e, code) in [
            (AppError::UpdateNetwork, "update_network"),
            (AppError::UpdateNotFound, "update_not_found"),
            (AppError::UpdateNoAsset, "update_no_asset"),
            (AppError::UpdateChecksum, "update_checksum"),
            (AppError::UpdateCancelled, "update_cancelled"),
        ] {
            let e = IpcError::from(e);
            assert_eq!(serde_json::to_value(e.code).unwrap(), code);
        }
        assert_eq!(
            serde_json::to_value(ErrorCode::UpdateBusy).unwrap(),
            "update_busy"
        );
    }

    #[test]
    fn recurrence_errors_map_to_codes() {
        let cases = [
            (
                AppError::OccurrenceStale,
                ErrorCode::OccurrenceStale,
                "occurrence_stale",
            ),
            (
                AppError::Recurrence(RecurrenceError::Interval),
                ErrorCode::RecurrenceInterval,
                "recurrence_interval",
            ),
            (
                AppError::Recurrence(RecurrenceError::UntilBeforeStart),
                ErrorCode::RecurrenceUntil,
                "recurrence_until",
            ),
            (
                AppError::Recurrence(RecurrenceError::Invalid("x".into())),
                ErrorCode::Internal,
                "internal",
            ),
        ];
        for (err, code, wire) in cases {
            assert_eq!(IpcError::from(err).code, code);
            assert_eq!(serde_json::to_value(code).unwrap(), wire);
        }
    }

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
    fn catalog_errors_map() {
        let e = IpcError::from(AppError::Catalog(CatalogError::CategoryDepth));
        assert_eq!(e.code, ErrorCode::CategoryDepth);
        let e = IpcError::from(AppError::InUse("account"));
        assert_eq!(e.code, ErrorCode::InUse);
        assert_eq!(e.entity.as_deref(), Some("account"));
        let e = IpcError::from(AppError::Duplicate("category"));
        assert_eq!(e.code, ErrorCode::Duplicate);
        assert_eq!(e.entity.as_deref(), Some("category"));
    }

    #[test]
    fn split_mismatch_maps() {
        let e = IpcError::from(AppError::Txn(TxnError::SplitMismatch(1)));
        assert_eq!(e.code, ErrorCode::TxnSplitMismatch);
        let e = IpcError::from(AppError::Txn(TxnError::NotPositive));
        assert_eq!(e.code, ErrorCode::TxnNotPositive);
    }

    #[test]
    fn budget_and_period_errors_map() {
        let e = IpcError::from(AppError::Budget(BudgetError::IncomeCategory));
        assert_eq!(e.code, ErrorCode::BudgetIncomeCategory);
        let e = IpcError::from(AppError::Period(PeriodError::InvalidKey));
        assert_eq!(e.code, ErrorCode::InvalidPeriod);
        let e = IpcError::from(PeriodError::InvalidStartDay(29));
        assert_eq!(e.code, ErrorCode::InvalidStartDay);
    }

    #[test]
    fn valuation_errors_map() {
        let e = IpcError::from(AppError::ValuationUnchanged);
        assert_eq!(e.code, ErrorCode::ValuationUnchanged);
        let e = IpcError::from(AppError::NotEditable);
        assert_eq!(e.code, ErrorCode::TxnNotEditable);
    }

    #[test]
    fn allocation_errors_map() {
        let e = IpcError::from(AppError::Allocation(AllocationError::TargetsExceed {
            total: 10_001,
        }));
        assert_eq!(e.code, ErrorCode::AllocationTargetsExceed);
        let e = IpcError::from(AppError::Allocation(AllocationError::LiabilityAccount));
        assert_eq!(e.code, ErrorCode::AllocationLiability);
        let e = IpcError::from(AppError::Allocation(AllocationError::InvalidAmount));
        assert_eq!(e.code, ErrorCode::AllocationInvalidAmount);
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
        for (code, name) in [
            (ErrorCode::RecoveryRequired, "recovery_required"),
            (ErrorCode::RecoveryUnavailable, "recovery_unavailable"),
            (ErrorCode::BackupKeyRequired, "backup_key_required"),
            (ErrorCode::DatabaseTooNew, "database_too_new"),
        ] {
            assert_eq!(serde_json::to_value(code).unwrap(), serde_json::json!(name));
        }
        assert_eq!(
            IpcError::from(AppError::BackupKeyRequired).code,
            ErrorCode::BackupKeyRequired
        );
        assert_eq!(
            IpcError::from(AppError::DatabaseTooNew).code,
            ErrorCode::DatabaseTooNew
        );
    }
}
