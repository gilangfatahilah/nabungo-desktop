//! Lapisan Tauri: command tipis + DTO serde. Logika bisnis ada di `nabungo-app`.

mod commands;
mod dto;
mod email;
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
            commands::list_rules,
            commands::create_rule,
            commands::update_rule,
            commands::delete_rule,
            commands::confirm_occurrence,
            commands::skip_occurrence,
            commands::confirm_all_due,
            commands::preview_recurrence,
            commands::update_txn,
            commands::get_txn,
            commands::list_txns,
            commands::delete_txn,
            commands::undo_delete_txn,
            commands::payee_suggestion,
            commands::search_payees,
            commands::list_tag_usage,
            commands::create_tag,
            commands::rename_tag,
            commands::merge_tag,
            commands::delete_tag,
            commands::list_payee_usage,
            commands::create_payee,
            commands::update_payee,
            commands::merge_payee,
            commands::delete_payee,
            commands::add_attachments,
            commands::list_attachments,
            commands::remove_attachment,
            commands::open_attachment,
            commands::list_tags,
            commands::txn_form_defaults,
            commands::period_of,
            commands::budget_overview,
            commands::set_budget,
            commands::set_budget_rollover,
            commands::copy_budgets,
            commands::record_valuation,
            commands::valuation_history,
            commands::reconcile_account,
            commands::account_balance_as_of,
            commands::list_contacts,
            commands::list_allocation,
            commands::create_bucket,
            commands::update_bucket,
            commands::delete_bucket,
            commands::set_account_allocation,
            commands::allocation_history,
            commands::cashflow_report,
            commands::category_breakdown,
            commands::category_comparison,
            commands::category_trend,
            commands::net_worth_history,
            commands::startup_report,
            commands::app_ready,
            commands::list_backups,
            commands::backup_now,
            commands::restore_backup,
            commands::pick_backup_file,
            commands::inspect_backup_source,
            commands::recovery_status,
            commands::recover_from_backup,
            commands::recovery_start_fresh,
            commands::export_json,
            commands::export_csv,
            commands::set_auto_lock_minutes,
            commands::set_font_scale,
            commands::security_status,
            commands::unlock,
            commands::unlock_with_recovery,
            commands::lock,
            commands::enable_encryption,
            commands::disable_encryption,
            commands::change_password,
            commands::email_status,
            commands::connect_email,
            commands::disconnect_email,
            commands::sync_email,
            commands::inbox_summary,
            commands::list_pending_captures,
            commands::list_unparsed_emails,
            commands::confirm_capture,
            commands::link_capture,
            commands::ignore_capture,
            commands::ignore_captures_until,
            commands::unignore_captures,
            commands::list_email_rules,
            commands::create_email_rule,
            commands::update_email_rule,
            commands::set_email_rule_enabled,
            commands::delete_email_rule,
            commands::restore_email_rule,
            commands::test_email_rule,
            commands::suggest_email_rule,
            commands::reparse_unparsed,
        ])
        .events(tauri_specta::collect_events![
            dto::EmailSyncProgress,
            dto::EmailSyncDone
        ])
        // Aman: nominal dibatasi MAX_AMOUNT (10^12) < 2^53.
        .dangerously_cast_bigints_to_number()
}

fn bindings_exporter() -> specta_typescript::Typescript {
    specta_typescript::Typescript::default()
}

const BINDINGS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ui/src/ipc/bindings.ts");

fn init(
    app: &tauri::App,
    started: std::time::Instant,
    local_offset: nabungo_app::UtcOffset,
) -> anyhow::Result<()> {
    use tauri::Manager;

    let exe = std::env::current_exe()?;
    let exe_dir = exe.parent().context("executable tanpa folder induk")?;
    let base = directories::BaseDirs::new();
    let data_dir = paths::resolve_data_dir(exe_dir, base.as_ref().map(|b| b.data_dir()))?;
    let guard = logging::init_logging(&data_dir.path)?;
    tracing::info!(path = %data_dir.path.display(), portable = data_dir.portable, "Nabungo mulai");
    nabungo_app::clear_open_cache(&data_dir.open_cache_path());

    let unclean_shutdown = lifecycle::SessionMarker::acquire(&data_dir.session_marker_path())
        .context("gagal menulis penanda sesi")?;
    if unclean_shutdown {
        tracing::warn!("sesi sebelumnya tidak berakhir normal");
    }
    // DB yang gagal dibuka tidak fatal: app masuk mode pemulihan agar restore terjangkau (P1).
    let db_state = nabungo_app::security::db_state(&data_dir.db_path());
    let recovery = match &db_state {
        Ok(_) => None,
        Err(e) => {
            tracing::error!(error = %e, "gagal memeriksa file database; mode pemulihan");
            lifecycle::startup_mode(Err(nabungo_app::AppError::DatabaseUnreadable))
        }
    };
    app.manage(state::LogGuard(guard));
    app.manage(state::AppState {
        ledger: std::sync::Mutex::new(None),
        recovery: std::sync::Mutex::new(recovery),
        data_dir,
        startup: std::sync::Mutex::new(lifecycle::StartupReport {
            unclean_shutdown,
            integrity: lifecycle::IntegrityStatus::Skipped,
        }),
        scheduler_started: std::sync::atomic::AtomicBool::new(false),
        unclean_shutdown,
        started,
        ready_logged: std::sync::atomic::AtomicBool::new(false),
        email: state::EmailRuntime::new(local_offset),
    });
    match db_state {
        Ok(nabungo_app::security::DbState::Plain) => {
            let opened = lifecycle::open_unlocked(app.handle(), None);
            if let Err(e) = &opened {
                tracing::error!(error = %e, "gagal membuka database; mode pemulihan");
            }
            let state = app.state::<state::AppState>();
            *state.recovery.lock().unwrap_or_else(|e| e.into_inner()) =
                lifecycle::startup_mode(opened);
        }
        Ok(nabungo_app::security::DbState::Encrypted { .. }) => {
            tracing::info!("database terenkripsi; menunggu unlock");
        }
        Err(_) => {}
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let started = std::time::Instant::now();
    // Sebelum thread lain ada: `time` menolak membaca offset lokal di proses multi-thread (K10).
    let local_offset = nabungo_app::UtcOffset::current_local_offset()
        .unwrap_or(nabungo_app::UtcOffset::from_hms(7, 0, 0).expect("offset valid"));
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    if let Err(e) = builder.export(bindings_exporter(), BINDINGS_PATH) {
        // Logging belum diinisialisasi di titik ini.
        eprintln!("gagal ekspor bindings TypeScript: {e}");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            init(app, started, local_offset).map_err(Into::into)
        })
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
