//! DTO serde/specta untuk command. Tanpa logika bisnis.

use nabungo_app::{
    AccountFields, AccountGroupRow, AccountKind, AccountRow, AccountsOverview, BudgetOverview,
    BudgetPeriod, BudgetRow, CategoryFields, CategoryKind, CategoryRow, Date, Fee, FormBody,
    Locale, NetWorth, OnboardingInput, PayeeSuggestion, PeriodStartDay, PostingRecord, SplitLine,
    Theme, TxnDetail, TxnForm, TxnKind, TxnListRow, TxnPage, TxnQuery, UiSettings, Uuid,
    ValuationPoint, format_date, parse_date,
};
use serde::{Deserialize, Serialize};

use crate::error::{ErrorCode, IpcError};

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
pub struct UiSettingsDto {
    pub theme: ThemeDto,
    pub locale: LocaleDto,
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
    pub limit: u32,
    pub offset: u32,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetRowDto {
    pub category_id: String,
    pub parent_id: Option<String>,
    pub budget: Option<i64>,
    /// Positif = terpakai; parent mencakup child.
    pub spent: i64,
    /// `budget − spent`; negatif = lewat budget.
    pub remaining: Option<i64>,
}

impl From<BudgetRow> for BudgetRowDto {
    fn from(r: BudgetRow) -> Self {
        Self {
            category_id: r.category_id.to_string(),
            parent_id: r.parent_id.map(|p| p.to_string()),
            budget: r.budget,
            spent: r.spent,
            remaining: r.budget.map(|b| b.saturating_sub(r.spent)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetTotalDto {
    pub budget: i64,
    pub spent: i64,
    pub remaining: i64,
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
    pub rows: Vec<BudgetRowDto>,
    pub total: BudgetTotalDto,
}

impl From<BudgetOverview> for BudgetOverviewDto {
    fn from(o: BudgetOverview) -> Self {
        Self {
            period: o.period.key(),
            prev: o.period.prev().key(),
            next: o.period.next().key(),
            from: format_date(o.from),
            to: format_date(o.to),
            rows: o.rows.into_iter().map(Into::into).collect(),
            total: BudgetTotalDto {
                budget: o.totals.budget,
                spent: o.totals.spent,
                remaining: o.totals.budget.saturating_sub(o.totals.spent),
            },
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
        }
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
                    spent: 130,
                },
                BudgetRow {
                    category_id: Uuid::now_v7(),
                    parent_id: Some(cat),
                    budget: None,
                    spent: 5,
                },
            ],
            totals: nabungo_app::BudgetTotals {
                budget: 100,
                spent: 130,
            },
        });
        assert_eq!(
            (dto.prev.as_str(), dto.next.as_str()),
            ("2025-12", "2026-02")
        );
        assert_eq!(
            (dto.from.as_str(), dto.to.as_str()),
            ("2025-12-25", "2026-01-24")
        );
        assert_eq!(dto.rows[0].remaining, Some(-30));
        assert_eq!(dto.rows[1].remaining, None);
        assert_eq!(dto.rows[1].parent_id, Some(cat.to_string()));
        assert_eq!(dto.total.remaining, -30);
        assert_eq!(
            parse_period("2026-13").unwrap_err().code,
            ErrorCode::InvalidPeriod
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
