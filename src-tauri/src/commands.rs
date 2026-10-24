//! Command Tauri: tipis, hanya memanggil `Ledger`/core lalu memetakan DTO & error.

use std::path::{Path, PathBuf};

use nabungo_app::security::{self, DbState};
use nabungo_app::vault::KdfParams;
use nabungo_app::{AppError, OffsetDateTime, PeriodStartDay, Recurrence, Uuid};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::{DialogExt, FilePath};
use tauri_plugin_opener::OpenerExt;

use crate::dto::{
    AccountFieldsDto, AccountKindDto, AccountsOverviewDto, AllocationOverviewDto,
    AllocationPointDto, AppInfo, AttachmentDto, BackupDto, BucketFieldsDto, BudgetAlertDto,
    BudgetOverviewDto, CashflowPointDto, CategoryAmountDto, CategoryDto, CategoryFieldsDto,
    CategoryKindDto, ContactDto, LocaleDto, NetWorthPointDto, OnboardingDto, PayeeSuggestionDto,
    PayeeUsageDto, ReconciliationDto, RecurrenceDto, RuleDto, RuleInputDto, SavedTxnDto,
    SecurityStatusDto, ShareDto, StartupReportDto, TagUsageDto, ThemeDto, TxnDetailDto,
    TxnFormDefaultsDto, TxnFormDto, TxnPageDto, TxnQueryDto, UiSettingsDto, ValuationPointDto,
    parse_day, parse_id, parse_period,
};
use crate::error::{ErrorCode, IpcError};
use crate::state::{AppState, LedgerGuard, lock_ledger, lock_slot};

fn ledger<'a>(state: &'a State<'_, AppState>) -> Result<LedgerGuard<'a>, IpcError> {
    lock_ledger(&state.ledger)
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
pub fn set_auto_lock_minutes(state: State<'_, AppState>, minutes: u8) -> Result<(), IpcError> {
    Ok(ledger(&state)?.set_auto_lock_minutes(minutes)?)
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
pub fn create_txn(state: State<'_, AppState>, form: TxnFormDto) -> Result<SavedTxnDto, IpcError> {
    let form = form.try_into()?;
    let (id, alerts) = ledger(&state)?.create_txn_with_alerts(&form)?;
    Ok(SavedTxnDto {
        id: id.to_string(),
        alerts: alerts.into_iter().map(Into::into).collect(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn update_txn(
    state: State<'_, AppState>,
    id: String,
    form: TxnFormDto,
) -> Result<Vec<BudgetAlertDto>, IpcError> {
    let id = parse_id(&id, "transaction")?;
    let form = form.try_into()?;
    let alerts = ledger(&state)?.update_txn_with_alerts(id, &form)?;
    Ok(alerts.into_iter().map(Into::into).collect())
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
    let today = nabungo_app::local_today();
    Ok(ledger(&state)?.budget_overview(period, today)?.into())
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

/// Nyalakan/matikan rollover budget kategori di periode itu (budget harus ada).
#[tauri::command]
#[specta::specta]
pub fn set_budget_rollover(
    state: State<'_, AppState>,
    category_id: String,
    period: String,
    on: bool,
) -> Result<(), IpcError> {
    let category_id = parse_id(&category_id, "category")?;
    let period = parse_period(&period)?;
    Ok(ledger(&state)?.set_budget_rollover(category_id, period, on)?)
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

// --- Rekonsiliasi & kontak ---

/// Samakan saldo akun per akhir `date` dengan `balance` (ACC-04).
#[tauri::command]
#[specta::specta]
pub fn reconcile_account(
    state: State<'_, AppState>,
    account_id: String,
    date: String,
    balance: i64,
) -> Result<ReconciliationDto, IpcError> {
    let account_id = parse_id(&account_id, "account")?;
    let date = parse_day(&date)?;
    Ok(ledger(&state)?
        .reconcile_account(account_id, date, balance)?
        .into())
}

/// Saldo tercatat akun per akhir `date` (bertanda).
#[tauri::command]
#[specta::specta]
pub fn account_balance_as_of(
    state: State<'_, AppState>,
    account_id: String,
    date: String,
) -> Result<i64, IpcError> {
    let account_id = parse_id(&account_id, "account")?;
    let date = parse_day(&date)?;
    Ok(ledger(&state)?.balance_as_of(account_id, date)?)
}

#[tauri::command]
#[specta::specta]
pub fn list_contacts(state: State<'_, AppState>) -> Result<Vec<ContactDto>, IpcError> {
    Ok(ledger(&state)?
        .list_contacts()?
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

// --- Laporan ---

/// Income/expense per periode, `count` periode berakhir di `end_period` (terlama dulu, maks. 24).
#[tauri::command]
#[specta::specta]
pub fn cashflow_report(
    state: State<'_, AppState>,
    end_period: String,
    count: u8,
) -> Result<Vec<CashflowPointDto>, IpcError> {
    let end = parse_period(&end_period)?;
    Ok(ledger(&state)?
        .cashflow_report(end, count)?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// Total per kategori root dalam satu periode, nominal terbesar dulu.
#[tauri::command]
#[specta::specta]
pub fn category_breakdown(
    state: State<'_, AppState>,
    period: String,
    kind: CategoryKindDto,
) -> Result<Vec<CategoryAmountDto>, IpcError> {
    let period = parse_period(&period)?;
    Ok(ledger(&state)?
        .category_breakdown(period, kind.into())?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// Net worth per akhir periode, `count` periode berakhir di `end_period` (terlama dulu, maks. 24).
#[tauri::command]
#[specta::specta]
pub fn net_worth_history(
    state: State<'_, AppState>,
    end_period: String,
    count: u8,
) -> Result<Vec<NetWorthPointDto>, IpcError> {
    let end = parse_period(&end_period)?;
    Ok(ledger(&state)?
        .net_worth_history(end, count)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn startup_report(state: State<'_, AppState>) -> StartupReportDto {
    (*state.startup.lock().unwrap_or_else(|e| e.into_inner())).into()
}

/// Dasbor pertama selesai dimuat: catat waktu sejak proses mulai, sekali per proses (log lokal).
#[tauri::command]
#[specta::specta]
pub fn app_ready(state: State<'_, AppState>) {
    if !state
        .ready_logged
        .swap(true, std::sync::atomic::Ordering::SeqCst)
    {
        let startup_ms = state.started.elapsed().as_millis() as u64;
        tracing::info!(startup_ms, "dasbor pertama tampil");
    }
}

// --- Tag & payee (CAT-04) ---

fn parse_opt_id(id: Option<String>, entity: &str) -> Result<Option<Uuid>, IpcError> {
    id.map(|s| parse_id(&s, entity)).transpose()
}

#[tauri::command]
#[specta::specta]
pub fn list_tag_usage(state: State<'_, AppState>) -> Result<Vec<TagUsageDto>, IpcError> {
    let list = ledger(&state)?.list_tag_usage()?;
    Ok(list.into_iter().map(Into::into).collect())
}

#[tauri::command]
#[specta::specta]
pub fn create_tag(state: State<'_, AppState>, name: String) -> Result<String, IpcError> {
    Ok(ledger(&state)?.create_tag(&name)?.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn rename_tag(state: State<'_, AppState>, id: String, name: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "tag")?;
    Ok(ledger(&state)?.rename_tag(id, &name)?)
}

#[tauri::command]
#[specta::specta]
pub fn merge_tag(
    state: State<'_, AppState>,
    source_id: String,
    target_id: String,
) -> Result<(), IpcError> {
    let source = parse_id(&source_id, "tag")?;
    let target = parse_id(&target_id, "tag")?;
    Ok(ledger(&state)?.merge_tag(source, target)?)
}

#[tauri::command]
#[specta::specta]
pub fn delete_tag(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "tag")?;
    Ok(ledger(&state)?.delete_tag(id)?)
}

#[tauri::command]
#[specta::specta]
pub fn list_payee_usage(state: State<'_, AppState>) -> Result<Vec<PayeeUsageDto>, IpcError> {
    let list = ledger(&state)?.list_payee_usage()?;
    Ok(list.into_iter().map(Into::into).collect())
}

#[tauri::command]
#[specta::specta]
pub fn create_payee(
    state: State<'_, AppState>,
    name: String,
    default_category_id: Option<String>,
) -> Result<String, IpcError> {
    let category = parse_opt_id(default_category_id, "category")?;
    Ok(ledger(&state)?.create_payee(&name, category)?.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_payee(
    state: State<'_, AppState>,
    id: String,
    name: String,
    default_category_id: Option<String>,
) -> Result<(), IpcError> {
    let id = parse_id(&id, "payee")?;
    let category = parse_opt_id(default_category_id, "category")?;
    Ok(ledger(&state)?.update_payee(id, &name, category)?)
}

#[tauri::command]
#[specta::specta]
pub fn merge_payee(
    state: State<'_, AppState>,
    source_id: String,
    target_id: String,
) -> Result<(), IpcError> {
    let source = parse_id(&source_id, "payee")?;
    let target = parse_id(&target_id, "payee")?;
    Ok(ledger(&state)?.merge_payee(source, target)?)
}

#[tauri::command]
#[specta::specta]
pub fn delete_payee(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "payee")?;
    Ok(ledger(&state)?.delete_payee(id)?)
}

// --- Lampiran (TRX-08) ---

/// Pilih file struk (multi), lalu salin ke folder lampiran. Batal → `[]`.
#[tauri::command]
#[specta::specta]
pub async fn add_attachments(
    app: AppHandle,
    state: State<'_, AppState>,
    txn_id: String,
) -> Result<Vec<AttachmentDto>, IpcError> {
    let txn_id = parse_id(&txn_id, "transaction")?;
    let sources = pick_files(&app).await?;
    if sources.is_empty() {
        return Ok(vec![]);
    }
    let dir = state.data_dir.attachments_path();
    std::fs::create_dir_all(&dir).map_err(AppError::Io)?;
    let rows = ledger(&state)?.add_attachments(txn_id, &sources, &dir)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

#[tauri::command]
#[specta::specta]
pub fn list_attachments(
    state: State<'_, AppState>,
    txn_id: String,
) -> Result<Vec<AttachmentDto>, IpcError> {
    let txn_id = parse_id(&txn_id, "transaction")?;
    let rows = ledger(&state)?.list_attachments(txn_id, &state.data_dir.attachments_path())?;
    Ok(rows.into_iter().map(Into::into).collect())
}

#[tauri::command]
#[specta::specta]
pub fn remove_attachment(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "attachment")?;
    Ok(ledger(&state)?.remove_attachment(id, &state.data_dir.attachments_path())?)
}

/// Buka lampiran dengan viewer bawaan OS.
#[tauri::command]
#[specta::specta]
pub fn open_attachment(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), IpcError> {
    let id = parse_id(&id, "attachment")?;
    let path = ledger(&state)?.attachment_path(id, &state.data_dir.attachments_path())?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| {
            tracing::error!(error = %e, "gagal membuka lampiran");
            IpcError::code(ErrorCode::FileIo)
        })
}

#[tauri::command]
#[specta::specta]
pub fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupDto>, IpcError> {
    let list = nabungo_app::list_backups(&state.data_dir.backups_path())?;
    Ok(list.into_iter().map(Into::into).collect())
}

enum Dialog {
    Save { name: String, ext: &'static str },
    Open,
    Folder,
}

/// Dialog native dijalankan di thread blocking agar tidak menahan event loop;
/// mutex ledger baru dikunci setelah dialog ditutup.
async fn pick_path(app: &AppHandle, dialog: Dialog) -> Result<Option<PathBuf>, IpcError> {
    let app = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let file = app.dialog().file();
        match dialog {
            Dialog::Save { name, ext } => file
                .add_filter(ext, &[ext])
                .set_file_name(name)
                .blocking_save_file(),
            Dialog::Open => file.add_filter("db", &["db"]).blocking_pick_file(),
            Dialog::Folder => file.blocking_pick_folder(),
        }
    })
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "dialog task gagal");
        IpcError::code(ErrorCode::Internal)
    })?;
    picked.map(file_path).transpose()
}

/// Dialog multi-pilih untuk lampiran; batal → kosong.
async fn pick_files(app: &AppHandle) -> Result<Vec<PathBuf>, IpcError> {
    let app = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("struk", &["jpg", "jpeg", "png", "webp", "pdf"])
            .blocking_pick_files()
    })
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "dialog task gagal");
        IpcError::code(ErrorCode::Internal)
    })?;
    picked
        .unwrap_or_default()
        .into_iter()
        .map(file_path)
        .collect()
}

fn file_path(p: FilePath) -> Result<PathBuf, IpcError> {
    p.into_path().map_err(|e| {
        tracing::error!(error = %e, "path dialog tidak valid");
        IpcError::code(ErrorCode::FileIo)
    })
}

fn display(p: &Path) -> String {
    p.display().to_string()
}

#[tauri::command]
#[specta::specta]
pub async fn backup_now(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, IpcError> {
    let name = format!(
        "nabungo-backup-{}.db",
        nabungo_app::format_date(nabungo_app::local_today())
    );
    let Some(dest) = pick_path(&app, Dialog::Save { name, ext: "db" }).await? else {
        return Ok(None);
    };
    ledger(&state)?.backup_to(&dest, &state.data_dir.attachments_path())?;
    Ok(Some(display(&dest)))
}

#[tauri::command]
#[specta::specta]
pub fn restore_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    file_name: String,
) -> Result<(), IpcError> {
    let dir = state.data_dir.backups_path();
    ledger(&state)?.restore_named(&dir, &file_name, None, OffsetDateTime::now_utc())?;
    crate::lifecycle::on_restored(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn restore_backup_from_file(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, IpcError> {
    let Some(src) = pick_path(&app, Dialog::Open).await? else {
        return Ok(false);
    };
    let dir = state.data_dir.backups_path();
    let att = state.data_dir.attachments_path();
    ledger(&state)?.restore_file(&src, None, &dir, &att, OffsetDateTime::now_utc())?;
    crate::lifecycle::on_restored(&app);
    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub async fn export_json(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, IpcError> {
    let name = format!(
        "nabungo-export-{}.json",
        nabungo_app::format_date(nabungo_app::local_today())
    );
    let Some(dest) = pick_path(&app, Dialog::Save { name, ext: "json" }).await? else {
        return Ok(None);
    };
    ledger(&state)?.export_json_file(&dest, OffsetDateTime::now_utc())?;
    Ok(Some(display(&dest)))
}

#[tauri::command]
#[specta::specta]
pub async fn export_csv(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, IpcError> {
    let Some(parent) = pick_path(&app, Dialog::Folder).await? else {
        return Ok(None);
    };
    let att = state.data_dir.attachments_path();
    let dir = ledger(&state)?.export_csv(&parent, nabungo_app::local_today(), &att)?;
    Ok(Some(display(&dir)))
}

/// Kerja berat (Argon2id, konversi DB) di thread blocking agar event loop tetap jalan.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, IpcError> + Send + 'static,
) -> Result<T, IpcError> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| {
        tracing::error!(error = %e, "task keamanan gagal");
        IpcError::code(ErrorCode::Internal)
    })?
}

/// Status enkripsi; bisa dipanggil saat terkunci.
#[tauri::command]
#[specta::specta]
pub fn security_status(state: State<'_, AppState>) -> Result<SecurityStatusDto, IpcError> {
    let slot = lock_slot(&state.ledger)?;
    if let Some(l) = slot.as_ref() {
        let has_keyfile = l.is_encrypted()
            && nabungo_app::vault::keyfile_path(&state.data_dir.db_path()).is_file();
        return Ok(SecurityStatusDto {
            encrypted: l.is_encrypted(),
            locked: false,
            has_keyfile,
        });
    }
    let (encrypted, has_keyfile) = match security::db_state(&state.data_dir.db_path())? {
        DbState::Plain => (false, false),
        DbState::Encrypted { has_keyfile } => (true, has_keyfile),
    };
    Ok(SecurityStatusDto {
        encrypted,
        locked: true,
        has_keyfile,
    })
}

/// Sudah terbuka → tidak melakukan apa-apa.
fn already_unlocked(app: &AppHandle) -> Result<bool, IpcError> {
    let state = app.state::<AppState>();
    Ok(lock_slot(&state.ledger)?.is_some())
}

#[tauri::command]
#[specta::specta]
pub async fn unlock(app: AppHandle, password: String) -> Result<(), IpcError> {
    blocking(move || {
        if already_unlocked(&app)? {
            return Ok(());
        }
        let db = app.state::<AppState>().data_dir.db_path();
        let key = security::unlock_key(&db, &password)?;
        crate::lifecycle::open_unlocked(&app, Some(key))?;
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn unlock_with_recovery(
    app: AppHandle,
    recovery_key: String,
    new_password: String,
) -> Result<(), IpcError> {
    blocking(move || {
        if already_unlocked(&app)? {
            return Ok(());
        }
        let db = app.state::<AppState>().data_dir.db_path();
        let key = security::recover_key(&db, &recovery_key, &new_password, KdfParams::DEFAULT)?;
        crate::lifecycle::open_unlocked(&app, Some(key))?;
        Ok(())
    })
    .await
}

/// Tutup koneksi DB terenkripsi; kunci hilang dari memori sampai unlock berikutnya.
#[tauri::command]
#[specta::specta]
pub fn lock(state: State<'_, AppState>) -> Result<(), IpcError> {
    let mut slot = lock_slot(&state.ledger)?;
    match slot.as_ref() {
        Some(l) if l.is_encrypted() => {
            *slot = None;
            tracing::info!("aplikasi dikunci");
            Ok(())
        }
        Some(_) => Err(AppError::EncryptionState.into()),
        None => Ok(()),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn enable_encryption(app: AppHandle, password: String) -> Result<String, IpcError> {
    blocking(move || {
        let state = app.state::<AppState>();
        let backups = state.data_dir.backups_path();
        Ok(ledger(&state)?.enable_encryption(&password, &backups, KdfParams::DEFAULT)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn disable_encryption(app: AppHandle, password: String) -> Result<(), IpcError> {
    blocking(move || {
        let state = app.state::<AppState>();
        let backups = state.data_dir.backups_path();
        Ok(ledger(&state)?.disable_encryption(&password, &backups)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn change_password(
    app: AppHandle,
    old_password: String,
    new_password: String,
) -> Result<(), IpcError> {
    blocking(move || {
        let state = app.state::<AppState>();
        Ok(ledger(&state)?.change_password(&old_password, &new_password, KdfParams::DEFAULT)?)
    })
    .await
}

// --- Transaksi terjadwal ---

#[tauri::command]
#[specta::specta]
pub fn list_rules(state: State<'_, AppState>) -> Result<Vec<RuleDto>, IpcError> {
    let rows = ledger(&state)?.list_rules(nabungo_app::local_today())?;
    Ok(rows.into_iter().map(Into::into).collect())
}

#[tauri::command]
#[specta::specta]
pub fn create_rule(state: State<'_, AppState>, input: RuleInputDto) -> Result<String, IpcError> {
    let input = input.try_into()?;
    Ok(ledger(&state)?.create_rule(&input)?.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_rule(
    state: State<'_, AppState>,
    id: String,
    input: RuleInputDto,
) -> Result<(), IpcError> {
    let id = parse_id(&id, "recurring_rule")?;
    let input = input.try_into()?;
    Ok(ledger(&state)?.update_rule(id, &input)?)
}

#[tauri::command]
#[specta::specta]
pub fn delete_rule(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    let id = parse_id(&id, "recurring_rule")?;
    Ok(ledger(&state)?.delete_rule(id)?)
}

/// `form` = form yang diedit user; `None` → template apa adanya.
#[tauri::command]
#[specta::specta]
pub fn confirm_occurrence(
    state: State<'_, AppState>,
    id: String,
    date: String,
    form: Option<TxnFormDto>,
) -> Result<SavedTxnDto, IpcError> {
    let id = parse_id(&id, "recurring_rule")?;
    let date = parse_day(&date)?;
    let form = form.map(TryInto::try_into).transpose()?;
    let (txn, alerts) = ledger(&state)?.confirm_occurrence_with_alerts(id, date, form.as_ref())?;
    Ok(SavedTxnDto {
        id: txn.to_string(),
        alerts: alerts.into_iter().map(Into::into).collect(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn skip_occurrence(
    state: State<'_, AppState>,
    id: String,
    date: String,
) -> Result<(), IpcError> {
    let id = parse_id(&id, "recurring_rule")?;
    let date = parse_day(&date)?;
    Ok(ledger(&state)?.skip_occurrence(id, date)?)
}

#[tauri::command]
#[specta::specta]
pub fn confirm_all_due(state: State<'_, AppState>, id: String) -> Result<u32, IpcError> {
    let id = parse_id(&id, "recurring_rule")?;
    Ok(ledger(&state)?.confirm_all_due(id, nabungo_app::local_today())?)
}

/// Pratinjau kejadian (maks 12) untuk form jadwal.
#[tauri::command]
#[specta::specta]
pub fn preview_recurrence(
    recurrence: RecurrenceDto,
    start: String,
    count: u32,
) -> Result<Vec<String>, IpcError> {
    let start = parse_day(&start)?;
    let rule = Recurrence::new(
        recurrence.freq.into(),
        recurrence.interval,
        start,
        recurrence.until()?,
    )
    .map_err(AppError::from)?;
    Ok(rule
        .occurrences(start, count.min(12) as usize)
        .into_iter()
        .map(nabungo_app::format_date)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::FreqDto;

    #[test]
    fn preview_recurrence_monthly_31() {
        let rec = RecurrenceDto {
            freq: FreqDto::Monthly,
            interval: 1,
            until: None,
        };
        assert_eq!(
            preview_recurrence(rec.clone(), "2026-01-31".into(), 3).unwrap(),
            vec!["2026-01-31", "2026-02-28", "2026-03-31"]
        );
        assert!(
            preview_recurrence(rec, "2026-01-31".into(), 50)
                .unwrap()
                .len()
                <= 12
        );
    }
}
