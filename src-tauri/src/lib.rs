//! Lapisan Tauri: command tipis + DTO serde. Logika bisnis ada di `nabungo-app`.

mod commands;
mod dto;
mod error;
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
        ])
        // Aman: nominal dibatasi MAX_AMOUNT (10^12) < 2^53.
        .dangerously_cast_bigints_to_number()
}

fn bindings_exporter() -> specta_typescript::Typescript {
    specta_typescript::Typescript::default()
}

const BINDINGS_PATH: &str = "../ui/src/ipc/bindings.ts";

fn init(app: &tauri::App) -> anyhow::Result<()> {
    use tauri::Manager;

    let exe = std::env::current_exe()?;
    let exe_dir = exe.parent().context("executable tanpa folder induk")?;
    let base = directories::BaseDirs::new();
    let data_dir = paths::resolve_data_dir(exe_dir, base.as_ref().map(|b| b.data_dir()))?;
    let guard = logging::init_logging(&data_dir.path)?;
    tracing::info!(path = %data_dir.path.display(), portable = data_dir.portable, "Nabungo mulai");

    let ledger = match nabungo_app::Ledger::open(&data_dir.db_path()) {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, "gagal membuka database");
            eprintln!("gagal membuka database: {e}");
            return Err(e.into());
        }
    };
    app.manage(state::LogGuard(guard));
    app.manage(state::AppState {
        ledger: std::sync::Mutex::new(ledger),
        data_dir,
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    builder
        .export(bindings_exporter(), BINDINGS_PATH)
        .expect("gagal ekspor bindings TypeScript");

    tauri::Builder::default()
        .setup(|app| init(app).map_err(Into::into))
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running Nabungo");
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
}
