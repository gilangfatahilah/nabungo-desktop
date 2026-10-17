fn main() {
    let mut attrs = tauri_build::Attributes::new();
    // tauri-build hanya menanam manifest Windows (Common-Controls v6) ke binary app;
    // test binary tanpa manifest gagal load (STATUS_ENTRYPOINT_NOT_FOUND). Tanam lewat
    // linker agar berlaku untuk semua target, termasuk `cargo test`.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        attrs =
            attrs.windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
    tauri_build::try_build(attrs).expect("failed to run tauri-build");
}
