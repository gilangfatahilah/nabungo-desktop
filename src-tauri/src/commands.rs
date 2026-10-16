//! Command Tauri: tipis, hanya memanggil `Ledger`/core lalu memetakan DTO & error.

use std::sync::MutexGuard;

use nabungo_app::Ledger;
use tauri::State;

use crate::dto::{AppInfo, LocaleDto, ThemeDto, UiSettingsDto};
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
    Ok(nabungo_core::parse_amount(&input)?)
}
