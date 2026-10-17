//! DTO serde/specta untuk command. Tanpa logika bisnis.

use nabungo_app::{
    AccountFields, AccountGroupRow, AccountKind, AccountRow, AccountsOverview, CategoryFields,
    CategoryKind, CategoryRow, Locale, NetWorth, OnboardingInput, PeriodStartDay, Theme,
    UiSettings, Uuid, format_date, parse_date,
};
use serde::{Deserialize, Serialize};

use crate::error::IpcError;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

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
}
