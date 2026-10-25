//! Penjaga offline (PRD §8): konfigurasi tidak boleh membuka jalan ke jaringan.
//! Bukti runtime ada di `scripts/firewall-test.sh`; test ini menjaga konfigurasi di setiap PR.

use serde_json::Value;

const TAURI_CONF: &str = include_str!("../tauri.conf.json");
const CARGO_TOML: &str = include_str!("../Cargo.toml");
const CAPABILITY: &str = include_str!("../capabilities/default.json");

/// Origin non-`'self'` yang diizinkan: IPC dan protokol asset Tauri (lokal).
const ALLOWED_ORIGINS: [&str; 2] = ["http://ipc.localhost", "http://asset.localhost"];
const WILDCARD_SOURCES: [&str; 5] = ["*", "http:", "https:", "ws:", "wss:"];

/// Token CSP yang membuka origin eksternal.
fn csp_violations(csp: &str) -> Vec<String> {
    csp.split(';')
        .flat_map(|directive| directive.split_whitespace().skip(1))
        .filter(|src| {
            WILDCARD_SOURCES.contains(src)
                || (src.contains("://") && !ALLOWED_ORIGINS.contains(src))
        })
        .map(str::to_owned)
        .collect()
}

fn conf() -> Value {
    serde_json::from_str(TAURI_CONF).unwrap()
}

#[test]
fn csp_violations_detects_external_origins() {
    let csp = "default-src 'self'; font-src https://fonts.googleapis.com; connect-src ipc: http://ipc.localhost wss:; img-src *";
    assert_eq!(
        csp_violations(csp),
        ["https://fonts.googleapis.com", "wss:", "*"]
    );
    assert!(csp_violations("default-src 'self'; img-src 'self' data: blob:").is_empty());
}

#[test]
fn csp_has_no_external_origins() {
    let csp = conf()["app"]["security"]["csp"]
        .as_str()
        .expect("CSP wajib diisi")
        .to_owned();
    assert_eq!(csp_violations(&csp), Vec::<String>::new(), "{csp}");
}

#[test]
fn no_updater_configured() {
    let conf = conf();
    assert!(conf["plugins"]["updater"].is_null());
    assert_ne!(conf["bundle"]["createUpdaterArtifacts"], Value::Bool(true));
}

#[test]
fn no_network_plugins() {
    for name in ["tauri-plugin-updater", "tauri-plugin-http"] {
        assert!(!CARGO_TOML.contains(name), "{name}");
    }
}

#[test]
fn capabilities_have_no_network_permissions() {
    let cap: Value = serde_json::from_str(CAPABILITY).unwrap();
    for p in cap["permissions"].as_array().unwrap() {
        let id = p
            .as_str()
            .or_else(|| p["identifier"].as_str())
            .unwrap_or_default();
        assert!(
            !id.starts_with("http:") && !id.starts_with("updater:"),
            "{id}"
        );
    }
}
