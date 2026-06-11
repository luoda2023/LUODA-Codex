fn main() {
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../luoda-codex-manager/src-tauri/icons/icon.ico");
        resource.set_manifest(include_str!(
            "../luoda-codex-manager/src-tauri/windows-app-manifest.xml"
        ));
        resource.compile().expect("compile launcher icon resource");
    }

    // Set custom cfg flags to avoid Rust 1.95 E0765 parser bug
    // when using target_os = "macos" in #[cfg()] attributes
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        println!("cargo:rustc-cfg=TARGET_OS_MACOS");
    }
}
