//! DTO serde/specta untuk command. Tanpa logika bisnis.

use nabungo_app::{
    AccountFields, AccountGroupRow, AccountKind, AccountRow, AccountsOverview, AllocationOverview,
    AllocationPoint, AttachmentRow, BackupInfo, BackupKind, BucketFields, BucketRow, BudgetAlert,
    BudgetLevel, BudgetOverview, BudgetPeriod, BudgetRow, CashflowPoint, CategoryAmount,
    CategoryFields, CategoryKind, CategoryRow, ContactRow, Date, Fee, FormBody, Freq, Locale,
    NetWorth, NetWorthPoint, OnboardingInput, PayeeSuggestion, PayeeUsage, PeriodStartDay,
    PostingRecord, Reconciliation, RuleInput, RuleRow, SplitLine, TagUsage, Target, Theme,
    TxnDetail, TxnForm, TxnKind, TxnListRow, TxnPage, TxnQuery, TxnSort, TxnSortField, UiSettings,
    Uuid, ValuationPoint, format_date, parse_date,
};
use serde::{Deserialize, Serialize};

use crate::error::{ErrorCode, IpcError};
use crate::lifecycle::{IntegrityStatus, StartupReport};

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub portable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ThemeDto {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum LocaleDto {
    Id,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UiSettingsDto {
    pub theme: ThemeDto,
    pub locale: LocaleDto,
    pub auto_lock_minutes: u8,
    pub font_scale: u16,
}

impl From<Theme> for ThemeDto {
    fn from(t: Theme) -> Self {
        match t {
            Theme::Light => ThemeDto::Light,
            Theme::Dark => ThemeDto::Dark,
            Theme::System => ThemeDto::System,
        }
    }
}

impl From<ThemeDto> for Theme {
    fn from(t: ThemeDto) -> Self {
        match t {
            ThemeDto::Light => Theme::Light,
            ThemeDto::Dark => Theme::Dark,
            ThemeDto::System => Theme::System,
        }
    }
}

impl From<Locale> for LocaleDto {
    fn from(l: Locale) -> Self {
        match l {
            Locale::Id => LocaleDto::Id,
            Locale::En => LocaleDto::En,
        }
    }
}

impl From<LocaleDto> for Locale {
    fn from(l: LocaleDto) -> Self {
        match l {
            LocaleDto::Id => Locale::Id,
            LocaleDto::En => Locale::En,
        }
    }
}

impl From<UiSettings> for UiSettingsDto {
    fn from(s: UiSettings) -> Self {
        Self {
            theme: s.theme.into(),
            locale: s.locale.into(),
            auto_lock_minutes: s.auto_lock_minutes,
            font_scale: s.font_scale,
        }
    }
}

/// UUID dari UI; gagal parse → `invalid_id` dengan nama entitas.
pub fn parse_id(s: &str, entity: &str) -> Result<Uuid, IpcError> {
    Uuid::parse_str(s).map_err(|_| IpcError::invalid_id(entity))
}

fn parse_opt_id(s: Option<&str>, entity: &str) -> Result<Option<Uuid>, IpcError> {
    s.map(|s| parse_id(s, entity)).transpose()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum AccountKindDto {
    Cash,
    Bank,
    Ewallet,
    Investment,
    Receivable,
    CreditCard,
    Loan,
}

impl From<AccountKind> for AccountKindDto {
    fn from(k: AccountKind) -> Self {
        match k {
            AccountKind::Cash => Self::Cash,
            AccountKind::Bank => Self::Bank,
            AccountKind::Ewallet => Self::Ewallet,
            AccountKind::Investment => Self::Investment,
            AccountKind::Receivable => Self::Receivable,
            AccountKind::CreditCard => Self::CreditCard,
            AccountKind::Loan => Self::Loan,
        }
    }
}

impl From<AccountKindDto> for AccountKind {
    fn from(k: AccountKindDto) -> Self {
        match k {
            AccountKindDto::Cash => Self::Cash,
            AccountKindDto::Bank => Self::Bank,
            AccountKindDto::Ewallet => Self::Ewallet,
            AccountKindDto::Investment => Self::Investment,
            AccountKindDto::Receivable => Self::Receivable,
            AccountKindDto::CreditCard => Self::CreditCard,
            AccountKindDto::Loan => Self::Loan,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CategoryKindDto {
    Income,
    Expense,
}

impl From<CategoryKind> for CategoryKindDto {
    fn from(k: CategoryKind) -> Self {
        match k {
            CategoryKind::Income => Self::Income,
            CategoryKind::Expense => Self::Expense,
        }
    }
}

impl From<CategoryKindDto> for CategoryKind {
    fn from(k: CategoryKindDto) -> Self {
        match k {
            CategoryKindDto::Income => Self::Income,
            CategoryKindDto::Expense => Self::Expense,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AccountFieldsDto {
    pub name: String,
    pub group_id: Option<String>,
    pub include_in_net_worth: bool,
    pub credit_limit: Option<i64>,
    pub statement_day: Option<u8>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub opening_balance: i64,
    /// `YYYY-MM-DD`
    pub opening_date: String,
    /// Hanya untuk akun piutang/hutang.
    pub contact: Option<String>,
}

impl TryFrom<AccountFieldsDto> for AccountFields {
    type Error = IpcError;

    fn try_from(d: AccountFieldsDto) -> Result<Self, IpcError> {
        Ok(Self {
            group_id: parse_opt_id(d.group_id.as_deref(), "account_group")?,
            opening_date: parse_date(&d.opening_date).map_err(|_| IpcError::invalid_date())?,
            name: d.name,
            include_in_net_worth: d.include_in_net_worth,
            credit_limit: d.credit_limit,
            statement_day: d.statement_day,
            icon: d.icon,
            color: d.color,
            opening_balance: d.opening_balance,
            contact: d.contact,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AccountDto {
    pub id: String,
    pub name: String,
    pub kind: AccountKindDto,
    pub group_id: Option<String>,
    pub include_in_net_worth: bool,
    pub credit_limit: Option<i64>,
    pub statement_day: Option<u8>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
    pub archived: bool,
    pub balance: i64,
    /// 0 bila akun tidak punya saldo awal.
    pub opening_balance: i64,
    pub opening_date: Option<String>,
    pub contact: Option<String>,
    /// `YYYY-MM-DD` txn terakhir yang sudah direkonsiliasi.
    pub reconciled_through: Option<String>,
}

impl From<AccountRow> for AccountDto {
    fn from(a: AccountRow) -> Self {
        Self {
            id: a.id.to_string(),
            name: a.name,
            kind: a.kind.into(),
            group_id: a.group_id.map(|g| g.to_string()),
            include_in_net_worth: a.include_in_net_worth,
            credit_limit: a.credit_limit,
            statement_day: a.statement_day,
            icon: a.icon,
            color: a.color,
            sort_order: a.sort_order,
            archived: a.archived,
            balance: a.balance,
            opening_balance: a.opening.as_ref().map_or(0, |o| o.amount),
            opening_date: a.opening.map(|o| format_date(o.date)),
            contact: a.contact,
            reconciled_through: a.reconciled_through.map(format_date),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AccountGroupDto {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
}

impl From<AccountGroupRow> for AccountGroupDto {
    fn from(g: AccountGroupRow) -> Self {
        Self {
            id: g.id.to_string(),
            name: g.name,
            sort_order: g.sort_order,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NetWorthDto {
    pub assets: i64,
    pub liabilities: i64,
    pub net: i64,
}

impl From<NetWorth> for NetWorthDto {
    fn from(n: NetWorth) -> Self {
        Self {
            assets: n.assets,
            liabilities: n.liabilities,
            net: n.net,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AccountsOverviewDto {
    pub groups: Vec<AccountGroupDto>,
    pub accounts: Vec<AccountDto>,
    pub net_worth: NetWorthDto,
}

impl From<AccountsOverview> for AccountsOverviewDto {
    fn from(o: AccountsOverview) -> Self {
        Self {
            groups: o.groups.into_iter().map(Into::into).collect(),
            accounts: o.accounts.into_iter().map(Into::into).collect(),
            net_worth: o.net_worth.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryFieldsDto {
    pub name: String,
    pub parent_id: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

impl TryFrom<CategoryFieldsDto> for CategoryFields {
    type Error = IpcError;

    fn try_from(d: CategoryFieldsDto) -> Result<Self, IpcError> {
        Ok(Self {
            parent_id: parse_opt_id(d.parent_id.as_deref(), "category")?,
            name: d.name,
            icon: d.icon,
            color: d.color,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: CategoryKindDto,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
    pub archived: bool,
    pub in_use: bool,
}

impl From<CategoryRow> for CategoryDto {
    fn from(c: CategoryRow) -> Self {
        Self {
            id: c.id.to_string(),
            parent_id: c.parent_id.map(|p| p.to_string()),
            kind: c.kind.into(),
            name: c.name,
            icon: c.icon,
            color: c.color,
            sort_order: c.sort_order,
            archived: c.archived,
            in_use: c.in_use,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingDto {
    pub locale: LocaleDto,
    pub period_start_day: u8,
    pub account_kind: AccountKindDto,
    pub account: AccountFieldsDto,
}

impl TryFrom<OnboardingDto> for OnboardingInput {
    type Error = IpcError;

    fn try_from(d: OnboardingDto) -> Result<Self, IpcError> {
        Ok(Self {
            locale: d.locale.into(),
            period_start_day: PeriodStartDay::new(d.period_start_day)?,
            account_kind: d.account_kind.into(),
            account: d.account.try_into()?,
        })
    }
}

pub fn parse_day(s: &str) -> Result<Date, IpcError> {
    parse_date(s).map_err(|_| IpcError::invalid_date())
}

// --- Transaksi ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TxnKindDto {
    Income,
    Expense,
    Transfer,
    Adjustment,
    Valuation,
    Opening,
}

impl From<TxnKind> for TxnKindDto {
    fn from(k: TxnKind) -> Self {
        match k {
            TxnKind::Income => Self::Income,
            TxnKind::Expense => Self::Expense,
            TxnKind::Transfer => Self::Transfer,
            TxnKind::Adjustment => Self::Adjustment,
            TxnKind::Valuation => Self::Valuation,
            TxnKind::Opening => Self::Opening,
        }
    }
}

/// Jenis yang bisa dibuat lewat form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TxnFormKindDto {
    Income,
    Expense,
    Transfer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SplitDto {
    pub category_id: String,
    pub amount: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FeeDto {
    pub amount: i64,
    /// `None` saat user menyalakan biaya tanpa memilih kategori → `txn_missing_category`.
    pub category_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TxnFormDto {
    pub kind: TxnFormKindDto,
    /// `YYYY-MM-DD`
    pub date: String,
    /// Transfer: akun asal.
    pub account_id: String,
    /// Wajib untuk transfer.
    pub to_account_id: Option<String>,
    /// Income/expense: total split (negatif = refund). Transfer: positif.
    pub amount: i64,
    /// Income/expense; diabaikan untuk transfer.
    pub splits: Vec<SplitDto>,
    /// Transfer saja.
    pub fee: Option<FeeDto>,
    pub payee: Option<String>,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

impl TryFrom<TxnFormDto> for TxnForm {
    type Error = IpcError;

    fn try_from(d: TxnFormDto) -> Result<Self, IpcError> {
        let account_id = parse_id(&d.account_id, "account")?;
        let splits = || {
            d.splits
                .iter()
                .map(|s| {
                    Ok(SplitLine {
                        category_id: parse_id(&s.category_id, "category")?,
                        amount: s.amount,
                        memo: s.memo.clone(),
                    })
                })
                .collect::<Result<Vec<_>, IpcError>>()
        };
        let body = match d.kind {
            TxnFormKindDto::Income => FormBody::Income {
                account_id,
                amount: d.amount,
                splits: splits()?,
            },
            TxnFormKindDto::Expense => FormBody::Expense {
                account_id,
                amount: d.amount,
                splits: splits()?,
            },
            TxnFormKindDto::Transfer => FormBody::Transfer {
                from: account_id,
                to: d
                    .to_account_id
                    .as_deref()
                    .ok_or_else(|| IpcError::invalid_id("account"))
                    .and_then(|t| parse_id(t, "account"))?,
                amount: d.amount,
                fee: d
                    .fee
                    .as_ref()
                    .map(|f| {
                        Ok::<_, IpcError>(Fee {
                            amount: f.amount,
                            category_id: f
                                .category_id
                                .as_deref()
                                .ok_or(IpcError::code(ErrorCode::TxnMissingCategory))
                                .and_then(|c| parse_id(c, "category"))?,
                        })
                    })
                    .transpose()?,
            },
        };
        Ok(Self {
            body,
            date: parse_day(&d.date)?,
            payee: d.payee,
            note: d.note,
            tags: d.tags,
        })
    }
}

impl From<&TxnForm> for TxnFormDto {
    fn from(f: &TxnForm) -> Self {
        let splits = |s: &[SplitLine]| {
            s.iter()
                .map(|l| SplitDto {
                    category_id: l.category_id.to_string(),
                    amount: l.amount,
                    memo: l.memo.clone(),
                })
                .collect()
        };
        let (kind, account_id, to_account_id, amount, splits, fee) = match &f.body {
            FormBody::Income {
                account_id,
                amount,
                splits: s,
            } => (
                TxnFormKindDto::Income,
                *account_id,
                None,
                *amount,
                splits(s),
                None,
            ),
            FormBody::Expense {
                account_id,
                amount,
                splits: s,
            } => (
                TxnFormKindDto::Expense,
                *account_id,
                None,
                *amount,
                splits(s),
                None,
            ),
            FormBody::Transfer {
                from,
                to,
                amount,
                fee,
            } => (
                TxnFormKindDto::Transfer,
                *from,
                Some(to.to_string()),
                *amount,
                vec![],
                fee.as_ref().map(|x| FeeDto {
                    amount: x.amount,
                    category_id: Some(x.category_id.to_string()),
                }),
            ),
        };
        Self {
            kind,
            date: format_date(f.date),
            account_id: account_id.to_string(),
            to_account_id,
            amount,
            splits,
            fee,
            payee: f.payee.clone(),
            note: f.note.clone(),
            tags: f.tags.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PostingDto {
    pub account_id: String,
    pub category_id: Option<String>,
    pub amount: i64,
    pub memo: Option<String>,
}

impl From<PostingRecord> for PostingDto {
    fn from(p: PostingRecord) -> Self {
        Self {
            account_id: p.account_id.to_string(),
            category_id: p.category_id.map(|c| c.to_string()),
            amount: p.amount,
            memo: p.memo,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TxnDetailDto {
    pub id: String,
    pub kind: TxnKindDto,
    pub date: String,
    pub payee: Option<String>,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub postings: Vec<PostingDto>,
}

impl From<TxnDetail> for TxnDetailDto {
    fn from(d: TxnDetail) -> Self {
        Self {
            id: d.id.to_string(),
            kind: d.kind.into(),
            date: format_date(d.date),
            payee: d.payee,
            note: d.note,
            tags: d.tags,
            postings: d.postings.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TxnQueryDto {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub account_id: Option<String>,
    pub category_id: Option<String>,
    pub tag: Option<String>,
    pub payee_id: Option<String>,
    pub amount_min: Option<i64>,
    pub amount_max: Option<i64>,
    pub search: Option<String>,
    /// `null` = tanggal terbaru dulu.
    pub sort: Option<TxnSortDto>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TxnSortFieldDto {
    Date,
    Amount,
    Payee,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TxnSortDto {
    pub field: TxnSortFieldDto,
    pub desc: bool,
}

impl From<TxnSortDto> for TxnSort {
    fn from(d: TxnSortDto) -> Self {
        let field = match d.field {
            TxnSortFieldDto::Date => TxnSortField::Date,
            TxnSortFieldDto::Amount => TxnSortField::Amount,
            TxnSortFieldDto::Payee => TxnSortField::Payee,
        };
        Self {
            field,
            desc: d.desc,
        }
    }
}

impl TryFrom<TxnQueryDto> for TxnQuery {
    type Error = IpcError;

    fn try_from(d: TxnQueryDto) -> Result<Self, IpcError> {
        let day = |s: Option<String>| s.as_deref().map(parse_day).transpose();
        Ok(Self {
            date_from: day(d.date_from)?,
            date_to: day(d.date_to)?,
            account_id: parse_opt_id(d.account_id.as_deref(), "account")?,
            category_id: parse_opt_id(d.category_id.as_deref(), "category")?,
            tag: d.tag,
            payee_id: parse_opt_id(d.payee_id.as_deref(), "payee")?,
            amount_min: d.amount_min,
            amount_max: d.amount_max,
            search: d.search,
            sort: d.sort.map(Into::into).unwrap_or_default(),
            limit: d.limit,
            offset: d.offset,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TxnRowDto {
    pub id: String,
    pub kind: TxnKindDto,
    pub date: String,
    pub payee: Option<String>,
    pub note: Option<String>,
    pub amount: i64,
    pub account_id: String,
    pub to_account_id: Option<String>,
    pub category_id: Option<String>,
    pub split_count: u32,
    pub fee: i64,
    pub tags: Vec<String>,
    pub account_delta: Option<i64>,
    pub running_balance: Option<i64>,
    /// Hanya saat difilter per akun.
    pub reconciled: Option<bool>,
    pub attachment_count: u32,
}

impl From<TxnListRow> for TxnRowDto {
    fn from(r: TxnListRow) -> Self {
        Self {
            id: r.id.to_string(),
            kind: r.kind.into(),
            date: format_date(r.date),
            payee: r.payee,
            note: r.note,
            amount: r.amount,
            account_id: r.account_id.to_string(),
            to_account_id: r.to_account_id.map(|a| a.to_string()),
            category_id: r.category_id.map(|c| c.to_string()),
            split_count: r.split_count,
            fee: r.fee,
            tags: r.tags,
            account_delta: r.account_delta,
            running_balance: r.running_balance,
            reconciled: r.reconciled,
            attachment_count: r.attachment_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TxnPageDto {
    pub rows: Vec<TxnRowDto>,
    // u64 → number aman: jumlah transaksi jauh di bawah 2^53.
    pub total: u64,
}

impl From<TxnPage> for TxnPageDto {
    fn from(p: TxnPage) -> Self {
        Self {
            rows: p.rows.into_iter().map(Into::into).collect(),
            total: p.total,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PayeeSuggestionDto {
    pub account_id: Option<String>,
    pub category_id: Option<String>,
}

impl From<PayeeSuggestion> for PayeeSuggestionDto {
    fn from(s: PayeeSuggestion) -> Self {
        Self {
            account_id: s.account_id.map(|a| a.to_string()),
            category_id: s.category_id.map(|c| c.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TxnFormDefaultsDto {
    pub last_account_id: Option<String>,
    pub fee_category_id: Option<String>,
}

// --- Budget ---

pub fn parse_period(s: &str) -> Result<BudgetPeriod, IpcError> {
    Ok(BudgetPeriod::parse(s)?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum BudgetLevelDto {
    Ok,
    Warning,
    Exceeded,
}

impl From<BudgetLevel> for BudgetLevelDto {
    fn from(l: BudgetLevel) -> Self {
        match l {
            BudgetLevel::Ok => Self::Ok,
            BudgetLevel::Warning => Self::Warning,
            BudgetLevel::Exceeded => Self::Exceeded,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetRowDto {
    pub category_id: String,
    pub parent_id: Option<String>,
    pub budget: Option<i64>,
    /// Sisa periode ini dibawa ke periode berikutnya.
    pub rollover: bool,
    /// Bawaan dari periode sebelumnya (bisa negatif).
    pub carry: i64,
    /// `budget + carry`; `null` tanpa budget.
    pub available: Option<i64>,
    /// Positif = terpakai; parent mencakup child.
    pub spent: i64,
    /// `available − spent`; negatif = lewat budget.
    pub remaining: Option<i64>,
    pub level: Option<BudgetLevelDto>,
    pub percent: Option<u32>,
    /// `null` tanpa budget atau di luar periode berjalan.
    pub per_day: Option<i64>,
}

impl From<BudgetRow> for BudgetRowDto {
    fn from(r: BudgetRow) -> Self {
        let available = r.budget.map(|b| b.saturating_add(r.carry));
        Self {
            category_id: r.category_id.to_string(),
            parent_id: r.parent_id.map(|p| p.to_string()),
            budget: r.budget,
            rollover: r.rollover,
            carry: r.carry,
            available,
            spent: r.spent,
            remaining: available.map(|a| a.saturating_sub(r.spent)),
            level: r.level.map(Into::into),
            percent: r.percent,
            per_day: r.per_day,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetTotalDto {
    pub budget: i64,
    pub carry: i64,
    pub available: i64,
    pub spent: i64,
    pub remaining: i64,
    pub per_day: Option<i64>,
}

/// `prev`/`next` adalah kunci periode agar UI tidak menghitung bulan sendiri.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetOverviewDto {
    pub period: String,
    pub prev: String,
    pub next: String,
    pub from: String,
    /// Inklusif.
    pub to: String,
    /// Hari tersisa termasuk hari ini; `null` bila hari ini di luar periode.
    pub days_left: Option<u32>,
    pub rows: Vec<BudgetRowDto>,
    pub total: BudgetTotalDto,
}

impl From<BudgetOverview> for BudgetOverviewDto {
    fn from(o: BudgetOverview) -> Self {
        let available = o.totals.budget.saturating_add(o.totals.carry);
        Self {
            period: o.period.key(),
            prev: o.period.prev().key(),
            next: o.period.next().key(),
            from: format_date(o.from),
            to: format_date(o.to),
            days_left: o.days_left,
            rows: o.rows.into_iter().map(Into::into).collect(),
            total: BudgetTotalDto {
                budget: o.totals.budget,
                carry: o.totals.carry,
                available,
                spent: o.totals.spent,
                remaining: available.saturating_sub(o.totals.spent),
                per_day: o.total_per_day,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetAlertDto {
    pub category_id: String,
    pub period: String,
    pub level: BudgetLevelDto,
    pub percent: Option<u32>,
}

impl From<BudgetAlert> for BudgetAlertDto {
    fn from(a: BudgetAlert) -> Self {
        Self {
            category_id: a.category_id.to_string(),
            period: a.period.key(),
            level: a.level.into(),
            percent: a.percent,
        }
    }
}

/// Hasil simpan transaksi baru: id dan peringatan budget yang baru terlewati.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedTxnDto {
    pub id: String,
    pub alerts: Vec<BudgetAlertDto>,
}

// --- Valuation ---

/// `value` = saldo akun per akhir `date`; `change` = selisih yang dicatat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ValuationPointDto {
    pub txn_id: String,
    pub date: String,
    pub change: i64,
    pub value: i64,
}

impl From<ValuationPoint> for ValuationPointDto {
    fn from(p: ValuationPoint) -> Self {
        Self {
            txn_id: p.txn_id.to_string(),
            date: format_date(p.date),
            change: p.change,
            value: p.value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationDto {
    pub adjustment_id: Option<String>,
    pub difference: i64,
    // u32 → number: jumlah posting satu akun jauh di bawah 2^32.
    pub marked: u32,
}

impl From<Reconciliation> for ReconciliationDto {
    fn from(r: Reconciliation) -> Self {
        Self {
            adjustment_id: r.adjustment.map(|a| a.to_string()),
            difference: r.difference,
            marked: u32::try_from(r.marked).unwrap_or(u32::MAX),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContactDto {
    pub id: String,
    pub name: String,
}

impl From<ContactRow> for ContactDto {
    fn from(c: ContactRow) -> Self {
        Self {
            id: c.id.to_string(),
            name: c.name,
        }
    }
}

// --- Tag, payee, lampiran ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TagUsageDto {
    pub id: String,
    pub name: String,
    pub txn_count: u32,
}

impl From<TagUsage> for TagUsageDto {
    fn from(t: TagUsage) -> Self {
        Self {
            id: t.id.to_string(),
            name: t.name,
            txn_count: t.txn_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PayeeUsageDto {
    pub id: String,
    pub name: String,
    pub default_category_id: Option<String>,
    pub txn_count: u32,
    pub last_date: Option<String>,
}

impl From<PayeeUsage> for PayeeUsageDto {
    fn from(p: PayeeUsage) -> Self {
        Self {
            id: p.id.to_string(),
            name: p.name,
            default_category_id: p.default_category_id.map(|c| c.to_string()),
            txn_count: p.txn_count,
            last_date: p.last_date.map(format_date),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentDto {
    pub id: String,
    pub file_name: String,
    pub mime: String,
    // u64 → number aman: dibatasi 10 MiB.
    pub size_bytes: u64,
    pub missing: bool,
}

impl From<AttachmentRow> for AttachmentDto {
    fn from(a: AttachmentRow) -> Self {
        Self {
            id: a.id.to_string(),
            file_name: a.file_name,
            mime: a.mime,
            size_bytes: a.size_bytes,
            missing: a.missing,
        }
    }
}

// --- Alokasi modal ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BucketDto {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    /// Relatif terhadap parent (child) atau total (root); 10000 = 100%. 0 bila nominal.
    pub target_bp: i64,
    /// Target nominal (satuan minor); `null` bila target persen.
    pub target_amount: Option<i64>,
    pub color: Option<String>,
    pub sort_order: i64,
}

impl From<BucketRow> for BucketDto {
    fn from(b: BucketRow) -> Self {
        Self {
            id: b.id.to_string(),
            parent_id: b.parent_id.map(|p| p.to_string()),
            name: b.name,
            target_bp: match b.target {
                Target::Percent(bp) => bp,
                Target::Amount(_) => 0,
            },
            target_amount: match b.target {
                Target::Amount(n) => Some(n),
                Target::Percent(_) => None,
            },
            color: b.color,
            sort_order: b.sort_order,
        }
    }
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BucketFieldsDto {
    pub name: String,
    pub parent_id: Option<String>,
    pub target_bp: i64,
    /// Non-null → target nominal (`target_bp` diabaikan).
    pub target_amount: Option<i64>,
    pub color: Option<String>,
}

impl TryFrom<BucketFieldsDto> for BucketFields {
    type Error = IpcError;

    fn try_from(f: BucketFieldsDto) -> Result<Self, IpcError> {
        Ok(Self {
            name: f.name,
            parent_id: parse_opt_id(f.parent_id.as_deref(), "bucket")?,
            target: f
                .target_amount
                .map_or(Target::Percent(f.target_bp), Target::Amount),
            color: f.color,
        })
    }
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShareDto {
    pub bucket_id: String,
    pub share_bp: i64,
}

impl ShareDto {
    pub fn parse(&self) -> Result<(Uuid, i64), IpcError> {
        Ok((parse_id(&self.bucket_id, "bucket")?, self.share_bp))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MappingDto {
    pub account_id: String,
    pub bucket_id: String,
    pub share_bp: i64,
}

/// Target efektif: `target_bp` global (porsi dari gross) dan `target_amount` (satuan minor);
/// `diff_* = aktual − target`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BucketActualDto {
    pub bucket_id: String,
    pub target_bp: i64,
    pub target_amount: i64,
    pub actual: i64,
    pub actual_bp: i64,
    pub diff_bp: i64,
    pub diff_amount: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UnallocatedDto {
    pub account_id: String,
    pub remaining_bp: i64,
    pub amount: i64,
}

/// Sibling (root bila `parent_id = null`) yang Σ target ≠ 100%.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WarningDto {
    pub parent_id: Option<String>,
    pub total_bp: i64,
}

/// Σ target nominal satu tingkat melebihi basisnya (total aset untuk root, target parent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AmountWarningDto {
    pub parent_id: Option<String>,
    pub total_amount: i64,
    pub base: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AllocationOverviewDto {
    /// Urutan pohon: root lalu child-nya.
    pub buckets: Vec<BucketDto>,
    pub mappings: Vec<MappingDto>,
    pub gross: i64,
    pub liabilities: i64,
    pub actuals: Vec<BucketActualDto>,
    pub unallocated: Vec<UnallocatedDto>,
    pub unallocated_total: i64,
    pub unallocated_bp: i64,
    pub warnings: Vec<WarningDto>,
    pub amount_warnings: Vec<AmountWarningDto>,
}

impl From<AllocationOverview> for AllocationOverviewDto {
    fn from(o: AllocationOverview) -> Self {
        let r = o.report;
        Self {
            buckets: o.buckets.into_iter().map(Into::into).collect(),
            mappings: o
                .mappings
                .into_iter()
                .map(|m| MappingDto {
                    account_id: m.account_id.to_string(),
                    bucket_id: m.bucket_id.to_string(),
                    share_bp: m.share_bp,
                })
                .collect(),
            gross: r.gross,
            liabilities: r.liabilities,
            actuals: r
                .buckets
                .into_iter()
                .map(|b| BucketActualDto {
                    bucket_id: b.bucket_id.to_string(),
                    target_bp: b.target_bp,
                    target_amount: b.target_amount,
                    actual: b.actual,
                    actual_bp: b.actual_bp,
                    diff_bp: b.diff_bp,
                    diff_amount: b.diff_amount,
                })
                .collect(),
            unallocated: r
                .unallocated
                .into_iter()
                .map(|u| UnallocatedDto {
                    account_id: u.account_id.to_string(),
                    remaining_bp: u.remaining_bp,
                    amount: u.amount,
                })
                .collect(),
            unallocated_total: r.unallocated_total,
            unallocated_bp: r.unallocated_bp,
            warnings: r
                .warnings
                .into_iter()
                .map(|w| WarningDto {
                    parent_id: w.parent_id.map(|p| p.to_string()),
                    total_bp: w.total_bp,
                })
                .collect(),
            amount_warnings: r
                .amount_warnings
                .into_iter()
                .map(|w| AmountWarningDto {
                    parent_id: w.parent_id.map(|p| p.to_string()),
                    total_amount: w.total_amount,
                    base: w.base,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BucketBpDto {
    pub bucket_id: String,
    pub actual_bp: i64,
}

/// Porsi bucket root per hari terakhir `period` (`date`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AllocationPointDto {
    pub period: String,
    pub date: String,
    pub gross: i64,
    pub buckets: Vec<BucketBpDto>,
}

impl From<AllocationPoint> for AllocationPointDto {
    fn from(p: AllocationPoint) -> Self {
        Self {
            period: p.period.key(),
            date: format_date(p.date),
            gross: p.gross,
            buckets: p
                .buckets
                .into_iter()
                .map(|(id, bp)| BucketBpDto {
                    bucket_id: id.to_string(),
                    actual_bp: bp,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CashflowPointDto {
    pub period: String,
    pub from: String,
    /// Inklusif.
    pub to: String,
    pub income: i64,
    /// Positif = keluar.
    pub expense: i64,
    pub net: i64,
}

impl From<CashflowPoint> for CashflowPointDto {
    fn from(p: CashflowPoint) -> Self {
        Self {
            period: p.period.key(),
            from: format_date(p.from),
            to: format_date(p.to),
            income: p.income,
            expense: p.expense,
            net: p.net,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CategoryAmountDto {
    /// Kategori root.
    pub category_id: String,
    pub amount: i64,
}

impl From<CategoryAmount> for CategoryAmountDto {
    fn from(c: CategoryAmount) -> Self {
        Self {
            category_id: c.category_id.to_string(),
            amount: c.amount,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NetWorthPointDto {
    pub period: String,
    pub date: String,
    pub assets: i64,
    pub liabilities: i64,
    pub net: i64,
}

impl From<NetWorthPoint> for NetWorthPointDto {
    fn from(p: NetWorthPoint) -> Self {
        Self {
            period: p.period.key(),
            date: format_date(p.date),
            assets: p.assets,
            liabilities: p.liabilities,
            net: p.net,
        }
    }
}

// --- Transaksi terjadwal ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum FreqDto {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl From<Freq> for FreqDto {
    fn from(f: Freq) -> Self {
        match f {
            Freq::Daily => Self::Daily,
            Freq::Weekly => Self::Weekly,
            Freq::Monthly => Self::Monthly,
            Freq::Yearly => Self::Yearly,
        }
    }
}

impl From<FreqDto> for Freq {
    fn from(f: FreqDto) -> Self {
        match f {
            FreqDto::Daily => Self::Daily,
            FreqDto::Weekly => Self::Weekly,
            FreqDto::Monthly => Self::Monthly,
            FreqDto::Yearly => Self::Yearly,
        }
    }
}

/// Anchor (hari/tanggal) diambil dari tanggal mulai/kejadian berikutnya.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceDto {
    pub freq: FreqDto,
    pub interval: u8,
    /// `YYYY-MM-DD`, inklusif.
    pub until: Option<String>,
}

impl RecurrenceDto {
    pub fn until(&self) -> Result<Option<Date>, IpcError> {
        self.until.as_deref().map(parse_day).transpose()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RuleInputDto {
    /// `date` = tanggal mulai (buat) atau kejadian berikutnya (ubah).
    pub form: TxnFormDto,
    pub recurrence: RecurrenceDto,
    pub auto_post: bool,
}

impl TryFrom<RuleInputDto> for RuleInput {
    type Error = IpcError;

    fn try_from(d: RuleInputDto) -> Result<Self, IpcError> {
        Ok(Self {
            until: d.recurrence.until()?,
            freq: d.recurrence.freq.into(),
            interval: d.recurrence.interval,
            form: d.form.try_into()?,
            auto_post: d.auto_post,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RuleDto {
    pub id: String,
    /// Template; `date` = kejadian berikutnya.
    pub form: TxnFormDto,
    pub recurrence: RecurrenceDto,
    pub auto_post: bool,
    pub ended: bool,
    /// Kejadian jatuh tempo (≤ hari ini), urut naik.
    pub due: Vec<String>,
}

impl From<RuleRow> for RuleDto {
    fn from(r: RuleRow) -> Self {
        Self {
            id: r.id.to_string(),
            form: (&r.form).into(),
            recurrence: RecurrenceDto {
                freq: r.freq.into(),
                interval: r.interval,
                until: r.until.map(format_date),
            },
            auto_post: r.auto_post,
            ended: r.ended,
            due: r.due.into_iter().map(format_date).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_fields_amount_wins() {
        let dto = |target_amount| BucketFieldsDto {
            name: "Darurat".into(),
            parent_id: None,
            target_bp: 7000,
            target_amount,
            color: None,
        };
        let f = BucketFields::try_from(dto(Some(5))).unwrap();
        assert_eq!(f.target, Target::Amount(5));
        let f = BucketFields::try_from(dto(None)).unwrap();
        assert_eq!(f.target, Target::Percent(7000));
    }

    #[test]
    fn txn_form_dto_round_trip() {
        let (a, b, c, d) = (
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
        );
        let date = parse_date("2026-10-21").unwrap();
        let forms = [
            TxnForm {
                body: FormBody::Expense {
                    account_id: a,
                    amount: 150_000,
                    splits: vec![
                        SplitLine {
                            category_id: c,
                            amount: 100_000,
                            memo: Some("listrik".into()),
                        },
                        SplitLine {
                            category_id: d,
                            amount: 50_000,
                            memo: None,
                        },
                    ],
                },
                date,
                payee: Some("PLN".into()),
                note: Some("bulanan".into()),
                tags: vec!["rumah".into()],
            },
            TxnForm {
                body: FormBody::Transfer {
                    from: a,
                    to: b,
                    amount: 200_000,
                    fee: Some(Fee {
                        amount: 2_500,
                        category_id: c,
                    }),
                },
                date,
                payee: None,
                note: None,
                tags: vec![],
            },
        ];
        for form in forms {
            let back = TxnForm::try_from(TxnFormDto::from(&form)).unwrap();
            assert_eq!(back, form);
        }
    }

    fn fields() -> AccountFieldsDto {
        AccountFieldsDto {
            name: "BCA".into(),
            group_id: None,
            include_in_net_worth: true,
            credit_limit: None,
            statement_day: None,
            icon: None,
            color: None,
            opening_balance: 1_000,
            opening_date: "2026-10-17".into(),
            contact: Some("Budi".into()),
        }
    }

    #[test]
    fn reconciliation_and_contact_dto_serialize_camel_case() {
        let id = Uuid::now_v7();
        let dto = ReconciliationDto::from(Reconciliation {
            adjustment: Some(id),
            difference: -15_000,
            marked: 3,
        });
        assert_eq!(
            serde_json::to_value(&dto).unwrap(),
            serde_json::json!({ "adjustmentId": id.to_string(), "difference": -15_000, "marked": 3 })
        );
        let c = ContactDto::from(ContactRow {
            id,
            name: "Budi".into(),
        });
        assert_eq!(
            serde_json::to_value(&c).unwrap(),
            serde_json::json!({ "id": id.to_string(), "name": "Budi" })
        );
    }

    #[test]
    fn cashflow_point_serializes_camel_case() {
        let p = CashflowPoint {
            period: BudgetPeriod::new(2026, 10).unwrap(),
            from: parse_date("2026-09-25").unwrap(),
            to: parse_date("2026-10-24").unwrap(),
            income: 10,
            expense: 3,
            net: 7,
        };
        assert_eq!(
            serde_json::to_value(CashflowPointDto::from(p)).unwrap(),
            serde_json::json!({ "period": "2026-10", "from": "2026-09-25", "to": "2026-10-24",
                "income": 10, "expense": 3, "net": 7 })
        );
    }

    #[test]
    fn bucket_fields_dto_rejects_bad_parent_id() {
        let f = BucketFieldsDto {
            name: "Saham".into(),
            parent_id: Some("x".into()),
            target_bp: 6000,
            target_amount: None,
            color: None,
        };
        let e = BucketFields::try_from(f).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidId);
        assert_eq!(e.entity.as_deref(), Some("bucket"));
    }

    #[test]
    fn allocation_point_dto_serializes_camel_case() {
        let id = Uuid::now_v7();
        let dto = AllocationPointDto::from(AllocationPoint {
            period: BudgetPeriod::parse("2026-10").unwrap(),
            date: parse_date("2026-10-31").unwrap(),
            gross: 1_000,
            buckets: vec![(id, 2500)],
        });
        assert_eq!(
            serde_json::to_value(&dto).unwrap(),
            serde_json::json!({
                "period": "2026-10", "date": "2026-10-31", "gross": 1000,
                "buckets": [{ "bucketId": id.to_string(), "actualBp": 2500 }],
            })
        );
    }

    #[test]
    fn valuation_point_dto_serializes_camel_case() {
        let id = Uuid::now_v7();
        let dto = ValuationPointDto::from(ValuationPoint {
            txn_id: id,
            date: nabungo_app::parse_date("2026-10-31").unwrap(),
            change: -500,
            value: 9_500,
        });
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "txnId": id.to_string(), "date": "2026-10-31", "change": -500, "value": 9_500 })
        );
    }

    #[test]
    fn budget_overview_dto_has_neighbours_and_remaining() {
        let period = BudgetPeriod::parse("2026-01").unwrap();
        let cat = Uuid::now_v7();
        let dto = BudgetOverviewDto::from(BudgetOverview {
            period,
            from: parse_date("2025-12-25").unwrap(),
            to: parse_date("2026-01-24").unwrap(),
            rows: vec![
                BudgetRow {
                    category_id: cat,
                    parent_id: None,
                    budget: Some(100),
                    rollover: true,
                    carry: -30,
                    spent: 50,
                    level: Some(BudgetLevel::Warning),
                    percent: Some(71),
                    per_day: Some(2),
                },
                BudgetRow {
                    category_id: Uuid::now_v7(),
                    parent_id: Some(cat),
                    budget: None,
                    rollover: false,
                    carry: 0,
                    spent: 5,
                    level: None,
                    percent: None,
                    per_day: None,
                },
            ],
            totals: nabungo_app::BudgetTotals {
                budget: 100,
                carry: -30,
                spent: 50,
            },
            days_left: Some(10),
            total_per_day: Some(2),
        });
        assert_eq!(
            (dto.prev.as_str(), dto.next.as_str()),
            ("2025-12", "2026-02")
        );
        assert_eq!(
            (dto.from.as_str(), dto.to.as_str()),
            ("2025-12-25", "2026-01-24")
        );
        let r = &dto.rows[0];
        assert_eq!((r.available, r.remaining), (Some(70), Some(20)));
        assert_eq!((r.carry, r.rollover), (-30, true));
        assert_eq!(r.level, Some(BudgetLevelDto::Warning));
        assert_eq!((r.percent, r.per_day), (Some(71), Some(2)));
        assert_eq!(dto.rows[1].remaining, None);
        assert_eq!(dto.rows[1].available, None);
        assert_eq!(dto.rows[1].parent_id, Some(cat.to_string()));
        assert_eq!((dto.total.available, dto.total.remaining), (70, 20));
        assert_eq!((dto.total.carry, dto.total.per_day), (-30, Some(2)));
        assert_eq!(dto.days_left, Some(10));
        assert_eq!(
            parse_period("2026-13").unwrap_err().code,
            ErrorCode::InvalidPeriod
        );
    }

    #[test]
    fn budget_level_serializes_snake_case() {
        let v = serde_json::to_value(BudgetLevelDto::from(BudgetLevel::Exceeded)).unwrap();
        assert_eq!(v, "exceeded");
    }

    #[test]
    fn budget_alert_dto_maps_period_key() {
        let cat = Uuid::now_v7();
        let dto = BudgetAlertDto::from(BudgetAlert {
            category_id: cat,
            period: BudgetPeriod::parse("2026-10").unwrap(),
            level: BudgetLevel::Warning,
            percent: Some(85),
        });
        assert_eq!(
            dto,
            BudgetAlertDto {
                category_id: cat.to_string(),
                period: "2026-10".into(),
                level: BudgetLevelDto::Warning,
                percent: Some(85),
            }
        );
    }

    #[test]
    fn account_fields_parse() {
        let mut bad = fields();
        bad.opening_date = "2026-13-01".into();
        let e = AccountFields::try_from(bad).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidDate);

        let mut bad = fields();
        bad.group_id = Some("nope".into());
        let e = AccountFields::try_from(bad).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidId);
        assert_eq!(e.entity.as_deref(), Some("account_group"));

        let g = Uuid::now_v7();
        let mut ok = fields();
        ok.group_id = Some(g.to_string());
        let f = AccountFields::try_from(ok).unwrap();
        assert_eq!(f.group_id, Some(g));
        assert_eq!(format_date(f.opening_date), "2026-10-17");
        assert_eq!(f.opening_balance, 1_000);
        assert_eq!(f.contact.as_deref(), Some("Budi"));
    }

    #[test]
    fn account_kind_dto_serializes_snake_case() {
        assert_eq!(
            serde_json::to_value(AccountKindDto::CreditCard).unwrap(),
            serde_json::json!("credit_card")
        );
    }

    fn transfer_dto() -> TxnFormDto {
        TxnFormDto {
            kind: TxnFormKindDto::Transfer,
            date: "2026-10-17".into(),
            account_id: Uuid::now_v7().to_string(),
            to_account_id: Some(Uuid::now_v7().to_string()),
            amount: 200_000,
            splits: vec![],
            fee: Some(FeeDto {
                amount: 6_500,
                category_id: Some(Uuid::now_v7().to_string()),
            }),
            payee: None,
            note: Some("pindah".into()),
            tags: vec!["kantor".into()],
        }
    }

    #[test]
    fn txn_form_dto_maps_transfer_with_fee() {
        let d = transfer_dto();
        let (from, to) = (d.account_id.clone(), d.to_account_id.clone().unwrap());
        let fee_cat = d.fee.as_ref().unwrap().category_id.clone().unwrap();
        let f = TxnForm::try_from(d).unwrap();
        assert_eq!(
            f.body,
            FormBody::Transfer {
                from: from.parse().unwrap(),
                to: to.parse().unwrap(),
                amount: 200_000,
                fee: Some(Fee {
                    amount: 6_500,
                    category_id: fee_cat.parse().unwrap()
                }),
            }
        );
        assert_eq!(format_date(f.date), "2026-10-17");
        assert_eq!(f.note.as_deref(), Some("pindah"));
        assert_eq!(f.tags, vec!["kantor"]);
    }

    #[test]
    fn txn_form_dto_transfer_without_to_is_invalid_id() {
        let mut d = transfer_dto();
        d.to_account_id = None;
        let e = TxnForm::try_from(d).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidId);
        assert_eq!(e.entity.as_deref(), Some("account"));

        let mut d = transfer_dto();
        d.date = "17-10-2026".into();
        assert_eq!(
            TxnForm::try_from(d).unwrap_err().code,
            ErrorCode::InvalidDate
        );
    }

    #[test]
    fn txn_form_dto_fee_without_category_is_missing_category() {
        let mut d = transfer_dto();
        d.fee.as_mut().unwrap().category_id = None;
        let e = TxnForm::try_from(d).unwrap_err();
        assert_eq!(e.code, ErrorCode::TxnMissingCategory);
    }

    #[test]
    fn txn_form_dto_maps_expense_splits() {
        let cat = Uuid::now_v7();
        let mut d = transfer_dto();
        d.kind = TxnFormKindDto::Expense;
        d.amount = 50_000;
        d.splits = vec![SplitDto {
            category_id: cat.to_string(),
            amount: 50_000,
            memo: Some("m".into()),
        }];
        let account = d.account_id.parse().unwrap();
        let f = TxnForm::try_from(d).unwrap();
        assert_eq!(
            f.body,
            FormBody::Expense {
                account_id: account,
                amount: 50_000,
                splits: vec![SplitLine {
                    category_id: cat,
                    amount: 50_000,
                    memo: Some("m".into())
                }],
            }
        );
    }

    #[test]
    fn txn_query_dto_parses() {
        let q = TxnQuery::try_from(TxnQueryDto {
            date_from: Some("2026-10-01".into()),
            limit: 100,
            ..TxnQueryDto::default()
        })
        .unwrap();
        assert_eq!(q.date_from.map(format_date).as_deref(), Some("2026-10-01"));
        assert_eq!(q.limit, 100);
        assert_eq!(q.sort, TxnSort::default());
        let sorted = TxnQuery::try_from(TxnQueryDto {
            sort: Some(TxnSortDto {
                field: TxnSortFieldDto::Amount,
                desc: false,
            }),
            ..TxnQueryDto::default()
        })
        .unwrap();
        assert_eq!(
            sorted.sort,
            TxnSort {
                field: TxnSortField::Amount,
                desc: false
            }
        );
        let e = TxnQuery::try_from(TxnQueryDto {
            payee_id: Some("x".into()),
            ..TxnQueryDto::default()
        })
        .unwrap_err();
        assert_eq!(e.entity.as_deref(), Some("payee"));
    }

    #[test]
    fn txn_row_serializes_camel_case() {
        let a = Uuid::now_v7();
        let row = TxnListRow {
            id: Uuid::now_v7(),
            kind: TxnKind::Transfer,
            date: parse_date("2026-10-03").unwrap(),
            payee: None,
            note: None,
            amount: 200_000,
            account_id: a,
            to_account_id: Some(a),
            category_id: None,
            split_count: 0,
            fee: 6_500,
            tags: vec![],
            account_delta: Some(-206_500),
            running_balance: Some(5_768_500),
            reconciled: None,
            attachment_count: 0,
        };
        let v = serde_json::to_value(TxnRowDto::from(row)).unwrap();
        assert_eq!(v["kind"], "transfer");
        assert_eq!(v["date"], "2026-10-03");
        assert_eq!(v["runningBalance"], 5_768_500);
        assert_eq!(v["accountDelta"], -206_500);
        assert_eq!(v["toAccountId"], a.to_string());
        assert_eq!(v["splitCount"], 0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum IntegrityStatusDto {
    Skipped,
    Ok,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StartupReportDto {
    pub unclean_shutdown: bool,
    pub integrity: IntegrityStatusDto,
}

impl From<StartupReport> for StartupReportDto {
    fn from(r: StartupReport) -> Self {
        Self {
            unclean_shutdown: r.unclean_shutdown,
            integrity: match r.integrity {
                IntegrityStatus::Skipped => IntegrityStatusDto::Skipped,
                IntegrityStatus::Ok => IntegrityStatusDto::Ok,
                IntegrityStatus::Failed => IntegrityStatusDto::Failed,
            },
        }
    }
}

/// Sumber restore/pemulihan (P9): backup app-managed (nama file) atau file pilihan user.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BackupSourceDto {
    #[serde(rename_all = "camelCase")]
    Named {
        file_name: String,
    },
    Path {
        path: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackupSourceInfoDto {
    /// Sumber terenkripsi dengan key lain (atau bukan SQLite): butuh recovery key-nya.
    pub needs_key: bool,
}

/// Rahasia pembuka backup saat pemulihan (P5).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BackupSecretDto {
    None,
    Password {
        password: String,
    },
    #[serde(rename_all = "camelCase")]
    RecoveryKey {
        recovery_key: String,
        new_password: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryReasonDto {
    Unreadable,
    TooNew,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryStatusDto {
    pub active: bool,
    pub reason: Option<RecoveryReasonDto>,
    pub data_dir: String,
    /// `nabungo.key` ada di samping DB (diperiksa langsung, bukan lewat `db_state`).
    pub has_keyfile: bool,
}

/// Status enkripsi untuk UI; tersedia juga saat terkunci.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SecurityStatusDto {
    pub encrypted: bool,
    pub locked: bool,
    pub has_keyfile: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum BackupKindDto {
    Auto,
    PreMigration,
    PreRestore,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackupDto {
    pub file_name: String,
    pub kind: BackupKindDto,
    /// ISO-8601 UTC.
    pub created_at: String,
    pub size_bytes: i64,
}

impl From<BackupInfo> for BackupDto {
    fn from(b: BackupInfo) -> Self {
        Self {
            file_name: b.file_name,
            kind: match b.kind {
                BackupKind::Auto => BackupKindDto::Auto,
                BackupKind::PreMigration => BackupKindDto::PreMigration,
                BackupKind::PreRestore => BackupKindDto::PreRestore,
            },
            created_at: nabungo_app::format_timestamp(b.created_at),
            size_bytes: i64::try_from(b.size_bytes).unwrap_or(i64::MAX),
        }
    }
}

// --- Email capture ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EmailStatusDto {
    pub enabled: bool,
    pub address: Option<String>,
    pub mailbox: String,
    pub last_sync_at: Option<String>,
    pub running: bool,
    pub last_error: Option<ErrorCode>,
    /// Sync otomatis dijeda karena login gagal (K8).
    pub paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncSummaryDto {
    pub new_items: u32,
    pub unparsed: u32,
    pub seen: u32,
}

impl From<nabungo_app::SyncSummary> for SyncSummaryDto {
    fn from(s: nabungo_app::SyncSummary) -> Self {
        Self {
            new_items: s.new_items,
            unparsed: s.unparsed,
            seen: s.seen,
        }
    }
}

/// Kemajuan kumulatif sync, dikirim setelah tiap batch.
#[derive(Debug, Clone, Copy, Serialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct EmailSyncProgress {
    pub processed: u32,
    pub new_items: u32,
    pub unparsed: u32,
}

impl From<nabungo_app::SyncProgress> for EmailSyncProgress {
    fn from(p: nabungo_app::SyncProgress) -> Self {
        Self {
            processed: p.processed,
            new_items: p.new_items,
            unparsed: p.unparsed,
        }
    }
}

/// Akhir satu putaran sync (manual maupun terjadwal).
#[derive(Debug, Clone, Serialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct EmailSyncDone {
    pub summary: Option<SyncSummaryDto>,
    pub error: Option<IpcError>,
}
