//! Command Tauri: tipis, hanya memanggil `Ledger`/core lalu memetakan DTO & error.

use std::path::{Path, PathBuf};
use std::sync::MutexGuard;

use nabungo_app::{Ledger, OffsetDateTime, PeriodStartDay, Uuid};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::dto::{
    AccountFieldsDto, AccountKindDto, AccountsOverviewDto, AllocationOverviewDto,
    AllocationPointDto, AppInfo, BackupDto, BucketFieldsDto, BudgetOverviewDto, CashflowPointDto,
    CategoryAmountDto, CategoryDto, CategoryFieldsDto, CategoryKindDto, LocaleDto,
    NetWorthPointDto, OnboardingDto, PayeeSuggestionDto, ShareDto, StartupReportDto, ThemeDto,
    TxnDetailDto, TxnFormDefaultsDto, TxnFormDto, TxnPageDto, TxnQueryDto, UiSettingsDto,
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
    state.startup.into()
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
    ledger(&state)?.backup_to(&dest)?;
    Ok(Some(display(&dest)))
}

#[tauri::command]
#[specta::specta]
pub fn restore_backup(state: State<'_, AppState>, file_name: String) -> Result<(), IpcError> {
    let dir = state.data_dir.backups_path();
    Ok(ledger(&state)?.restore_named(&dir, &file_name, OffsetDateTime::now_utc())?)
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
    ledger(&state)?.restore_file(&src, &dir, OffsetDateTime::now_utc())?;
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
    let file = std::fs::File::create(&dest).map_err(nabungo_app::AppError::from)?;
    let mut w = std::io::BufWriter::new(file);
    ledger(&state)?.export_json(&mut w, OffsetDateTime::now_utc())?;
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
    let dir = ledger(&state)?.export_csv(&parent, nabungo_app::local_today())?;
    Ok(Some(display(&dir)))
}
