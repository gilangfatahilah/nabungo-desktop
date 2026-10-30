//! Rilis GitHub (`/releases/latest`) dan pemilihan installer per format instalasi.

use std::collections::HashMap;

use serde::Deserialize;

/// Subset field rilis GitHub yang dipakai.
#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub body: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// Format instalasi yang sedang berjalan (dari bundler Tauri).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallKind {
    Deb,
    Rpm,
    AppImage,
    Nsis,
    Msi,
}

impl InstallKind {
    /// Akhiran nama file buatan tauri-bundler (x86_64), mis. `Nabungo_0.4.0_amd64.deb`,
    /// `Nabungo-0.4.0-1.x86_64.rpm`, `Nabungo_0.4.0_x64-setup.exe`, `Nabungo_0.4.0_x64_en-US.msi`.
    fn matches(self, name: &str) -> bool {
        match self {
            InstallKind::Deb => name.ends_with("_amd64.deb"),
            InstallKind::Rpm => name.ends_with(".x86_64.rpm"),
            InstallKind::AppImage => name.ends_with("_amd64.AppImage"),
            InstallKind::Nsis => name.ends_with("_x64-setup.exe"),
            InstallKind::Msi => name.contains("_x64") && name.ends_with(".msi"),
        }
    }
}

/// Installer untuk `kind`; `None` bila rilis tidak menyediakannya.
pub fn pick_asset(assets: &[Asset], kind: InstallKind) -> Option<&Asset> {
    assets.iter().find(|a| kind.matches(&a.name))
}

/// Isi `SHA256SUMS` (`<hash>  <nama>` per baris, format `sha256sum`) → nama → hash huruf kecil.
pub fn parse_sha256sums(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let (hash, name) = line.trim().split_once(char::is_whitespace)?;
            // `sha256sum -b` menandai file biner dengan `*` di depan nama.
            let name = name.trim_start().trim_start_matches('*');
            let valid = hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit());
            (valid && !name.is_empty()).then(|| (name.to_owned(), hash.to_ascii_lowercase()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release() -> Release {
        serde_json::from_str(include_str!("../tests/fixtures/release-v0.3.0.json")).unwrap()
    }

    #[test]
    fn parses_github_release_json() {
        let r = release();
        assert_eq!(r.tag_name, "v0.3.0");
        assert_eq!(r.assets.len(), 6);
        assert!(r.body.unwrap().contains("Email capture"));
    }

    #[test]
    fn picks_asset_for_each_kind() {
        let r = release();
        for (kind, name) in [
            (InstallKind::Deb, "Nabungo_0.3.0_amd64.deb"),
            (InstallKind::Rpm, "Nabungo-0.3.0-1.x86_64.rpm"),
            (InstallKind::AppImage, "Nabungo_0.3.0_amd64.AppImage"),
            (InstallKind::Nsis, "Nabungo_0.3.0_x64-setup.exe"),
            (InstallKind::Msi, "Nabungo_0.3.0_x64_en-US.msi"),
        ] {
            assert_eq!(pick_asset(&r.assets, kind).unwrap().name, name);
        }
        assert_eq!(pick_asset(&r.assets[5..], InstallKind::Deb), None);
    }

    #[test]
    fn parses_sha256sums() {
        let h = "a".repeat(64);
        let text = format!(
            "{h}  Nabungo_0.3.0_amd64.deb\n{}  *Nabungo_0.3.0_x64-setup.exe\nrusak  x\n\n",
            "B".repeat(64)
        );
        let sums = parse_sha256sums(&text);
        assert_eq!(sums.len(), 2);
        assert_eq!(sums["Nabungo_0.3.0_amd64.deb"], h);
        assert_eq!(sums["Nabungo_0.3.0_x64-setup.exe"], "b".repeat(64));
    }
}
