//! Command Tauri: tipis, hanya memanggil `Ledger`/core lalu memetakan DTO & error.

use std::sync::MutexGuard;

use nabungo_app::{Ledger, PeriodStartDay, Uuid};
use tauri::State;

use crate::dto::{
    AccountFieldsDto, AccountKindDto, AccountsOverviewDto, AllocationOverviewDto,
    AllocationPointDto, AppInfo, BucketFieldsDto, BudgetOverviewDto, CategoryDto,
    CategoryFieldsDto, CategoryKindDto, LocaleDto, OnboardingDto, PayeeSuggestionDto, ShareDto,
    ThemeDto, TxnDetailDto, TxnFormDefaultsDto, TxnFormDto, TxnPageDto, TxnQueryDto, UiSettingsDto,
    ValuationPointDto, parse_day, parse_id, parse_period,
};
use crate::error::{ErrorCode, IpcError};
use crate::state::AppState;

fn ledger<'a>(state: &'a State<'_, AppState>) -> Result<MutexGuard<'a, Ledger>, IpcError> {
    state.ledger.lock().map_err(|_| {
        tracing::error!("mutex ledger poisoned");
        IpcError {
            code: ErrorCode::Internal,
            entity: None,
        }
    })
}

#[tauri::command]
#[specta::specta]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: nabungo_app::version().to_owned(),
        data_dir: state.data_dir.path.display().to_string(),
        portable: state.data_dir.portable,
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_ui_settings(state: State<'_, AppState>) -> Result<UiSettingsDto, IpcError> {
    Ok(ledger(&state)?.ui_settings()?.into())
}

#[tauri::command]
#[specta::specta]
pub fn set_theme(state: State<'_, AppState>, theme: ThemeDto) -> Result<(), IpcError> {
    Ok(ledger(&state)?.set_theme(theme.into())?)
}

#[tauri::command]
#[specta::specta]
pub fn set_locale(state: State<'_, AppState>, locale: LocaleDto) -> Result<(), IpcError> {
    Ok(ledger(&state)?.set_locale(locale.into())?)
}

#[tauri::command]
#[specta::specta]
pub fn parse_amount(input: String) -> Result<i64, IpcError> {
    Ok(nabungo_app::parse_amount(&input)?)
}

fn ids(ids: &[String], entity: &str) -> Result<Vec<Uuid>, IpcError> {
    ids.iter().map(|id| parse_id(id, entity)).collect()
}

// --- Onboarding & periode budget ---

#[tauri::command]
#[specta::specta]
pub fn get_onboarding_status(state: State<'_, AppState>) -> Result<bool, IpcError> {
    Ok(ledger(&state)?.is_onboarded()?)
}

/// Mengembalikan ID akun pertama.
#[tauri::command]
#[specta::specta]
pub fn complete_onboarding(
    state: State<'_, AppState>,
    input: OnboardingDto,
) -> Result<String, IpcError> {
    let input = input.try_into()?;
    Ok(ledger(&state)?.complete_onboarding(&input)?.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn get_period_start_day(state: State<'_, AppState>) -> Result<u8, IpcError> {
    Ok(ledger(&state)?.period_start_day()?.day())
}

#[tauri::command]
#[specta::specta]
pub fn set_period_start_day(state: State<'_, AppState>, day: u8) -> Result<(), IpcError> {
    let day = PeriodStartDay::new(day)?;
    Ok(ledger(&state)?.set_period_start_day(day)?)
}

// --- Akun & account group ---

#[tauri::command]
#[specta::specta]
pub fn accounts_overview(state: State<'_, AppState>) -> Result<AccountsOverviewDto, IpcError> {
    Ok(ledger(&state)?.accounts_overview()?.into())
}

#[tauri::command]
#[specta::specta]
pub fn create_account(
    state: State<'_, AppState>,
    kind: AccountKindDto,
    fields: AccountFieldsDto,
) -> Result<String, IpcError> {
    let fields = fields.try_into()?;
    Ok(ledger(&state)?
        .create_account(kind.into(), &fields)?
        .to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_account(
    state: State<'_, AppState>,
    id: String,
    fields: AccountFieldsDto,
) -> Result<(), IpcError> {
    let id = parse_id(&id, "account")?;
    let fields = fields.try_into()?;
    Ok(ledger(&state)?.update_account(id, &fields)?)
}

#[tauri::command]
#[specta::specta]
pub fn archive_account(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "account")?;
    Ok(ledger(&state)?.archive_account(id)?)
}

#[tauri::command]
#[specta::specta]
pub fn unarchive_account(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "account")?;
    Ok(ledger(&state)?.unarchive_account(id)?)
}

#[tauri::command]
#[specta::specta]
pub fn delete_account(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "account")?;
    Ok(ledger(&state)?.delete_account(id)?)
}

#[tauri::command]
#[specta::specta]
pub fn reorder_accounts(
    state: State<'_, AppState>,
    group_id: Option<String>,
    ids: Vec<String>,
) -> Result<(), IpcError> {
    let group_id = group_id
        .map(|g| parse_id(&g, "account_group"))
        .transpose()?;
    let ids = self::ids(&ids, "account")?;
    Ok(ledger(&state)?.reorder_accounts(group_id, &ids)?)
}

#[tauri::command]
#[specta::specta]
pub fn create_account_group(state: State<'_, AppState>, name: String) -> Result<String, IpcError> {
    Ok(ledger(&state)?.create_group(&name)?.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn rename_account_group(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<(), IpcError> {
    let id = parse_id(&id, "account_group")?;
    Ok(ledger(&state)?.rename_group(id, &name)?)
}

#[tauri::command]
#[specta::specta]
pub fn delete_account_group(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "account_group")?;
    Ok(ledger(&state)?.delete_group(id)?)
}

#[tauri::command]
#[specta::specta]
pub fn reorder_account_groups(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<(), IpcError> {
    let ids = self::ids(&ids, "account_group")?;
    Ok(ledger(&state)?.reorder_groups(&ids)?)
}

// --- Kategori ---

#[tauri::command]
#[specta::specta]
pub fn list_categories(state: State<'_, AppState>) -> Result<Vec<CategoryDto>, IpcError> {
    Ok(ledger(&state)?
        .list_categories()?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn create_category(
    state: State<'_, AppState>,
    kind: CategoryKindDto,
    fields: CategoryFieldsDto,
) -> Result<String, IpcError> {
    let fields = fields.try_into()?;
    Ok(ledger(&state)?
        .create_category(kind.into(), &fields)?
        .to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_category(
    state: State<'_, AppState>,
    id: String,
    fields: CategoryFieldsDto,
) -> Result<(), IpcError> {
    let id = parse_id(&id, "category")?;
    let fields = fields.try_into()?;
    Ok(ledger(&state)?.update_category(id, &fields)?)
}

#[tauri::command]
#[specta::specta]
pub fn archive_category(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "category")?;
    Ok(ledger(&state)?.archive_category(id)?)
}

#[tauri::command]
#[specta::specta]
pub fn unarchive_category(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "category")?;
    Ok(ledger(&state)?.unarchive_category(id)?)
}

#[tauri::command]
#[specta::specta]
pub fn delete_category(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "category")?;
    Ok(ledger(&state)?.delete_category(id)?)
}

#[tauri::command]
#[specta::specta]
pub fn merge_category(
    state: State<'_, AppState>,
    source: String,
    target: String,
) -> Result<(), IpcError> {
    let source = parse_id(&source, "category")?;
    let target = parse_id(&target, "category")?;
    Ok(ledger(&state)?.merge_category(source, target)?)
}

// --- Transaksi ---

#[tauri::command]
#[specta::specta]
pub fn create_txn(state: State<'_, AppState>, form: TxnFormDto) -> Result<String, IpcError> {
    let form = form.try_into()?;
    Ok(ledger(&state)?.create_txn(&form)?.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_txn(
    state: State<'_, AppState>,
    id: String,
    form: TxnFormDto,
) -> Result<(), IpcError> {
    let id = parse_id(&id, "transaction")?;
    let form = form.try_into()?;
    Ok(ledger(&state)?.update_txn(id, &form)?)
}

#[tauri::command]
#[specta::specta]
pub fn get_txn(state: State<'_, AppState>, id: String) -> Result<TxnDetailDto, IpcError> {
    let id = parse_id(&id, "transaction")?;
    Ok(ledger(&state)?.get_txn(id)?.into())
}

#[tauri::command]
#[specta::specta]
pub fn list_txns(state: State<'_, AppState>, query: TxnQueryDto) -> Result<TxnPageDto, IpcError> {
    let query = query.try_into()?;
    Ok(ledger(&state)?.list_txns(&query)?.into())
}

#[tauri::command]
#[specta::specta]
pub fn delete_txn(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "transaction")?;
    Ok(ledger(&state)?.delete_txn(id)?)
}

#[tauri::command]
#[specta::specta]
pub fn undo_delete_txn(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "transaction")?;
    Ok(ledger(&state)?.undo_delete_txn(id)?)
}

#[tauri::command]
#[specta::specta]
pub fn payee_suggestion(
    state: State<'_, AppState>,
    name: String,
) -> Result<Option<PayeeSuggestionDto>, IpcError> {
    Ok(ledger(&state)?.payee_suggestion(&name)?.map(Into::into))
}

#[tauri::command]
#[specta::specta]
pub fn search_payees(state: State<'_, AppState>, query: String) -> Result<Vec<String>, IpcError> {
    Ok(ledger(&state)?.search_payees(&query)?)
}

#[tauri::command]
#[specta::specta]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<String>, IpcError> {
    Ok(ledger(&state)?.list_tags()?)
}

#[tauri::command]
#[specta::specta]
pub fn txn_form_defaults(state: State<'_, AppState>) -> Result<TxnFormDefaultsDto, IpcError> {
    let ledger = ledger(&state)?;
    Ok(TxnFormDefaultsDto {
        last_account_id: ledger.last_used_account()?.map(|a| a.to_string()),
        fee_category_id: ledger.default_fee_category()?.map(|c| c.to_string()),
    })
}

// --- Budget ---

/// Kunci periode budget (`YYYY-MM`) yang memuat `date`.
#[tauri::command]
#[specta::specta]
pub fn period_of(state: State<'_, AppState>, date: String) -> Result<String, IpcError> {
    let date = parse_day(&date)?;
    Ok(ledger(&state)?.period_of(date)?.key())
}

#[tauri::command]
#[specta::specta]
pub fn budget_overview(
    state: State<'_, AppState>,
    period: String,
) -> Result<BudgetOverviewDto, IpcError> {
    let period = parse_period(&period)?;
    Ok(ledger(&state)?.budget_overview(period)?.into())
}

/// `amount = null` menghapus budget kategori di periode itu.
#[tauri::command]
#[specta::specta]
pub fn set_budget(
    state: State<'_, AppState>,
    category_id: String,
    period: String,
    amount: Option<i64>,
) -> Result<(), IpcError> {
    let category_id = parse_id(&category_id, "category")?;
    let period = parse_period(&period)?;
    Ok(ledger(&state)?.set_budget(category_id, period, amount)?)
}

/// Salin budget periode sebelumnya ke kategori yang belum punya budget. Mengembalikan jumlah baris.
#[tauri::command]
#[specta::specta]
pub fn copy_budgets(state: State<'_, AppState>, period: String) -> Result<u32, IpcError> {
    let period = parse_period(&period)?;
    Ok(ledger(&state)?.copy_budgets(period)?)
}

// --- Valuation ---

/// Catat nilai akun investasi per akhir `date`; yang disimpan adalah selisih terhadap saldo.
#[tauri::command]
#[specta::specta]
pub fn record_valuation(
    state: State<'_, AppState>,
    account_id: String,
    date: String,
    value: i64,
) -> Result<String, IpcError> {
    let account_id = parse_id(&account_id, "account")?;
    let date = parse_day(&date)?;
    Ok(ledger(&state)?
        .record_valuation(account_id, date, value)?
        .to_string())
}

/// Riwayat valuation, terbaru dulu.
#[tauri::command]
#[specta::specta]
pub fn valuation_history(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<ValuationPointDto>, IpcError> {
    let account_id = parse_id(&account_id, "account")?;
    Ok(ledger(&state)?
        .valuation_history(account_id)?
        .into_iter()
        .map(Into::into)
        .collect())
}

// --- Alokasi modal ---

#[tauri::command]
#[specta::specta]
pub fn list_allocation(state: State<'_, AppState>) -> Result<AllocationOverviewDto, IpcError> {
    Ok(ledger(&state)?.allocation_overview()?.into())
}

#[tauri::command]
#[specta::specta]
pub fn create_bucket(
    state: State<'_, AppState>,
    fields: BucketFieldsDto,
) -> Result<String, IpcError> {
    let fields = fields.try_into()?;
    Ok(ledger(&state)?.create_bucket(&fields)?.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_bucket(
    state: State<'_, AppState>,
    id: String,
    fields: BucketFieldsDto,
) -> Result<(), IpcError> {
    let id = parse_id(&id, "bucket")?;
    let fields = fields.try_into()?;
    Ok(ledger(&state)?.update_bucket(id, &fields)?)
}

#[tauri::command]
#[specta::specta]
pub fn delete_bucket(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "bucket")?;
    Ok(ledger(&state)?.delete_bucket(id)?)
}

/// Mengganti seluruh porsi akun; daftar kosong = akun tidak dialokasikan.
#[tauri::command]
#[specta::specta]
pub fn set_account_allocation(
    state: State<'_, AppState>,
    account_id: String,
    shares: Vec<ShareDto>,
) -> Result<(), IpcError> {
    let account_id = parse_id(&account_id, "account")?;
    let shares = shares
        .iter()
        .map(ShareDto::parse)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ledger(&state)?.set_account_allocation(account_id, &shares)?)
}

/// `count` periode berakhir di `end_period`, terlama dulu.
#[tauri::command]
#[specta::specta]
pub fn allocation_history(
    state: State<'_, AppState>,
    end_period: String,
    count: u8,
) -> Result<Vec<AllocationPointDto>, IpcError> {
    let end = parse_period(&end_period)?;
    Ok(ledger(&state)?
        .allocation_history(end, count)?
        .into_iter()
        .map(Into::into)
        .collect())
}
