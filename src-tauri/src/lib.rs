//! Lapisan Tauri: command tipis + DTO serde. Logika bisnis ada di `nabungo-app`.

mod commands;
mod dto;
mod error;
mod lifecycle;
mod logging;
mod paths;
mod state;

use anyhow::Context;

/// Registry command + konfigurasi binding; dipakai `run()` dan test ekspor.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            commands::app_info,
            commands::get_ui_settings,
            commands::set_theme,
            commands::set_locale,
            commands::parse_amount,
            commands::get_onboarding_status,
            commands::complete_onboarding,
            commands::get_period_start_day,
            commands::set_period_start_day,
            commands::accounts_overview,
            commands::create_account,
            commands::update_account,
            commands::archive_account,
            commands::unarchive_account,
            commands::delete_account,
            commands::reorder_accounts,
            commands::create_account_group,
            commands::rename_account_group,
            commands::delete_account_group,
            commands::reorder_account_groups,
            commands::list_categories,
            commands::create_category,
            commands::update_category,
            commands::archive_category,
            commands::unarchive_category,
            commands::delete_category,
            commands::merge_category,
            commands::create_txn,
            commands::update_txn,
            commands::get_txn,
            commands::list_txns,
            commands::delete_txn,
            commands::undo_delete_txn,
            commands::payee_suggestion,
            commands::search_payees,
            commands::list_tags,
            commands::txn_form_defaults,
            commands::period_of,
            commands::budget_overview,
            commands::set_budget,
            commands::set_budget_rollover,
            commands::copy_budgets,
            commands::record_valuation,
            commands::valuation_history,
            commands::list_allocation,
            commands::create_bucket,
            commands::update_bucket,
            commands::delete_bucket,
            commands::set_account_allocation,
            commands::allocation_history,
            commands::cashflow_report,
            commands::category_breakdown,
            commands::net_worth_history,
            commands::startup_report,
            commands::list_backups,
            commands::backup_now,
            commands::restore_backup,
            commands::restore_backup_from_file,
            commands::export_json,
            commands::export_csv,
            commands::set_auto_lock_minutes,
            commands::security_status,
            commands::unlock,
            commands::unlock_with_recovery,
            commands::lock,
            commands::enable_encryption,
            commands::disable_encryption,
            commands::change_password,
        ])
        // Aman: nominal dibatasi MAX_AMOUNT (10^12) < 2^53.
        .dangerously_cast_bigints_to_number()
}

fn bindings_exporter() -> specta_typescript::Typescript {
    specta_typescript::Typescript::default()
}

const BINDINGS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ui/src/ipc/bindings.ts");

fn init(app: &tauri::App) -> anyhow::Result<()> {
    use tauri::Manager;

    let exe = std::env::current_exe()?;
    let exe_dir = exe.parent().context("executable tanpa folder induk")?;
    let base = directories::BaseDirs::new();
    let data_dir = paths::resolve_data_dir(exe_dir, base.as_ref().map(|b| b.data_dir()))?;
    let guard = logging::init_logging(&data_dir.path)?;
    tracing::info!(path = %data_dir.path.display(), portable = data_dir.portable, "Nabungo mulai");

    let unclean_shutdown = lifecycle::SessionMarker::acquire(&data_dir.session_marker_path())
        .context("gagal menulis penanda sesi")?;
    if unclean_shutdown {
        tracing::warn!("sesi sebelumnya tidak berakhir normal");
    }
    let db_state = match nabungo_app::security::db_state(&data_dir.db_path()) {
        Ok(st) => st,
        Err(e) => {
            tracing::error!(error = %e, "gagal memeriksa file database");
            return Err(e.into());
        }
    };
    app.manage(state::LogGuard(guard));
    app.manage(state::AppState {
        ledger: std::sync::Mutex::new(None),
        data_dir,
        startup: std::sync::Mutex::new(lifecycle::StartupReport {
            unclean_shutdown,
            integrity: lifecycle::IntegrityStatus::Skipped,
        }),
        scheduler_started: std::sync::atomic::AtomicBool::new(false),
        unclean_shutdown,
    });
    match db_state {
        nabungo_app::security::DbState::Plain => {
            if let Err(e) = lifecycle::open_unlocked(app.handle(), None) {
                tracing::error!(error = %e, "gagal membuka database");
                eprintln!("gagal membuka database: {e}");
                return Err(e.into());
            }
        }
        nabungo_app::security::DbState::Encrypted { .. } => {
            tracing::info!("database terenkripsi; menunggu unlock");
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    if let Err(e) = builder.export(bindings_exporter(), BINDINGS_PATH) {
        // Logging belum diinisialisasi di titik ini.
        eprintln!("gagal ekspor bindings TypeScript: {e}");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| init(app).map_err(Into::into))
        .invoke_handler(builder.invoke_handler())
        .build(tauri::generate_context!())
        .expect("error while building Nabungo")
        .run(|app, event| {
            use tauri::Manager;
            if let tauri::RunEvent::Exit = event
                && let Some(state) = app.try_state::<state::AppState>()
            {
                lifecycle::SessionMarker::release(&state.data_dir.session_marker_path());
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_bindings() {
        specta_builder()
            .export(bindings_exporter(), BINDINGS_PATH)
            .expect("gagal ekspor bindings");
    }

    /// File-drop native Tauri menelan event drag HTML5 di WebView2, sehingga
    /// reorder akun (SortableJS) mati di Windows.
    #[test]
    fn native_file_drop_disabled() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        for w in conf["app"]["windows"].as_array().unwrap() {
            assert_eq!(w["dragDropEnabled"], serde_json::Value::Bool(false));
        }
    }
}
