//! Lapisan Tauri: command tipis + DTO serde. Logika bisnis ada di `nabungo-app`.

mod logging;
mod paths;
mod state;

use anyhow::Context;

#[tauri::command]
fn app_version() -> &'static str {
    nabungo_app::version()
}

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
    tauri::Builder::default()
        .setup(|app| init(app).map_err(Into::into))
        .invoke_handler(tauri::generate_handler![app_version])
        .run(tauri::generate_context!())
        .expect("error while running Nabungo");
}
