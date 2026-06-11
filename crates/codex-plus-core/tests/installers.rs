use codex_plus_core::install::{
    InstallOptions, SILENT_BINARY, app_bundle_names, build_macos_app_bundle,
    build_windows_entrypoint_plan, companion_binary_path_from_exe, default_install_root_strategy,
    shortcut_names,
};

#[test]
fn windows_entrypoint_plan_contains_silent_and_manager_entrypoints() {
    let options = InstallOptions {
        install_root: Some("C:/Users/A/Desktop".into()),
        launcher_path: Some("C:/Tools/luoda-codex.exe".into()),
        manager_path: Some("C:/Tools/luoda-codex-manager.exe".into()),
        remove_owned_data: false,
    };

    let plan = build_windows_entrypoint_plan(&options);

    assert!(plan.silent_shortcut.ends_with("Luoda-Codex.lnk"));
    assert!(plan.manager_shortcut.ends_with("Luoda-Codex 管理工具.lnk"));
    assert_eq!(plan.launcher_path, "C:/Tools/luoda-codex.exe");
    assert_eq!(plan.manager_path, "C:/Tools/luoda-codex-manager.exe");
    assert_eq!(plan.silent_icon_path, "C:/Tools/luoda-codex.exe");
    assert_eq!(
        plan.manager_icon_path,
        "C:/Tools/luoda-codex-manager.exe"
    );
    assert_eq!(plan.uninstall_key, "Luoda-Codex");
    assert_eq!(plan.legacy_uninstall_key, "Luoda-Codex");
}

#[test]
fn windows_entrypoint_plan_can_request_owned_data_removal_without_shell_script() {
    let options = InstallOptions {
        install_root: Some("C:/Users/A/Desktop".into()),
        launcher_path: None,
        manager_path: None,
        remove_owned_data: true,
    };

    let plan = build_windows_entrypoint_plan(&options);

    assert!(plan.silent_shortcut.ends_with("Luoda-Codex.lnk"));
    assert!(plan.manager_shortcut.ends_with("Luoda-Codex 管理工具.lnk"));
    assert!(plan.remove_owned_data);
}

#[test]
fn macos_bundle_metadata_contains_silent_and_manager_apps() {
    let options = InstallOptions {
        install_root: Some("/Applications".into()),
        launcher_path: Some("/opt/Luoda-Codex/luoda-codex".into()),
        manager_path: Some("/opt/Luoda-Codex/luoda-codex-manager".into()),
        remove_owned_data: false,
    };

    let silent = build_macos_app_bundle(&options, false);
    let manager = build_macos_app_bundle(&options, true);

    assert!(silent.app_path.ends_with("Luoda-Codex.app"));
    assert!(manager.app_path.ends_with("Luoda-Codex 管理工具.app"));
    assert!(silent.info_plist.contains("<string>Luoda-Codex</string>"));
    assert!(
        manager
            .info_plist
            .contains("<string>Luoda-Codex 管理工具</string>")
    );
    assert!(silent.launch_script.contains("luoda-codex"));
    assert!(manager.launch_script.contains("luoda-codex-manager"));
}

#[test]
fn installer_exports_expected_two_entrypoint_names() {
    assert_eq!(shortcut_names(), ("Luoda-Codex.lnk", "Luoda-Codex 管理工具.lnk"));
    assert_eq!(app_bundle_names(), ("Luoda-Codex.app", "Luoda-Codex 管理工具.app"));
}

#[test]
fn companion_binary_path_resolves_macos_silent_app_next_to_manager_app() {
    let manager_exe = std::path::Path::new(
        "/Applications/Luoda-Codex 管理工具.app/Contents/MacOS/Luoda-CodexManager",
    );

    let companion = companion_binary_path_from_exe(manager_exe, SILENT_BINARY);

    assert_eq!(
        companion,
        std::path::PathBuf::from("/Applications/Luoda-Codex.app/Contents/MacOS/Luoda-Codex")
    );
    assert_ne!(
        companion,
        std::path::PathBuf::from(
            "/Applications/Luoda-Codex 管理工具.app/Contents/MacOS/luoda-codex"
        )
    );
}

#[test]
fn macos_bundle_does_not_wrap_the_bundle_executable_in_itself() {
    let options = InstallOptions {
        install_root: Some("/Applications".into()),
        launcher_path: Some("/Applications/Luoda-Codex.app/Contents/MacOS/Luoda-Codex".into()),
        manager_path: Some(
            "/Applications/Luoda-Codex 管理工具.app/Contents/MacOS/Luoda-CodexManager".into(),
        ),
        remove_owned_data: false,
    };

    let silent = build_macos_app_bundle(&options, false);
    let manager = build_macos_app_bundle(&options, true);

    assert!(!silent.launch_script.contains("Luoda-Codex\""));
    assert!(!manager.launch_script.contains("Luoda-CodexManager\""));
    assert!(silent.launch_script.contains("luoda-codex"));
    assert!(manager.launch_script.contains("luoda-codex-manager"));
}

#[test]
fn windows_default_install_root_uses_known_folder_before_userprofile_desktop() {
    let strategy = default_install_root_strategy();

    if cfg!(windows) {
        assert_eq!(strategy, "windows-known-folder");
    } else if cfg!(target_os = "macos") {
        assert_eq!(strategy, "macos-applications");
    } else {
        assert_eq!(strategy, "user-dirs-desktop");
    }
}
