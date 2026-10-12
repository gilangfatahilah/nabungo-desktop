//! Lapisan Tauri: command tipis + DTO serde. Logika bisnis ada di `nabungo-app`.

#[tauri::command]
fn app_version() -> &'static str {
    nabungo_app::version()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_version])
        .run(tauri::generate_context!())
        .expect("error while running Nabungo");
}
