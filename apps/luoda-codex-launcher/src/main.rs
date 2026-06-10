#![cfg_attr(windows, windows_subsystem = "windows")]

use anyhow::{Context, Result};
use luoda_codex_core::launcher::{
    DefaultLaunchHooks, LaunchHooks, LaunchOptions, launch_and_inject_with_hooks,
};
use luoda_codex_core::models::{DeleteResult, ExportResult, SessionRef};
use luoda_codex_core::routes::{BridgeContext, BridgeDataService, BridgeRuntimeService};
use luoda_codex_core::user_scripts::UserScriptManager;
use serde_json::{Value, json};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct LauncherHooks {
    core: Arc<DefaultLaunchHooks>,
    data: Arc<LauncherDataService>,
    runtime: Arc<LauncherRuntimeService>,
}

impl Default for LauncherHooks {
    fn default() -> Self {
        Self {
            core: Arc::new(DefaultLaunchHooks::default()),
            data: Arc::new(LauncherDataService::default()),
            runtime: Arc::new(LauncherRuntimeService::new(
                9229,
                default_user_script_manager(),
            )),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let options = parse_launch_options(std::env::args().skip(1));
    let Some(_guard) = acquire_single_instance_guard(options.debug_port)? else {
        activate_existing_codex_app(&options).await?;
        return Ok(());
    };
    tokio::spawn(async {
        let _ = notify_manager_when_update_available().await;
    });
    let hooks = LauncherHooks::default();
    let handle = launch_and_inject_with_hooks(options, &hooks).await?;
    handle.wait_for_codex_exit().await?;
    Ok(())
}

fn acquire_single_instance_guard(
    debug_port: u16,
) -> anyhow::Result<Option<luoda_codex_core::ports::LoopbackPortGuard>> {
    acquire_single_instance_guard_with_retry(debug_port, true)
}

fn acquire_single_instance_guard_with_retry(
    debug_port: u16,
    allow_stale_recovery: bool,
) -> anyhow::Result<Option<luoda_codex_core::ports::LoopbackPortGuard>> {
    match try_acquire_single_instance_guard() {
        Ok(guard) => {
            if let Some(fallback_lock_path) = guard.fallback_path() {
                log_launcher_guard_fallback(fallback_lock_path);
            }
            Ok(Some(guard))
        }
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            log_launcher_already_running(debug_port);
            Ok(None)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            log_launcher_already_running(debug_port);
            if allow_stale_recovery && should_recover_stale_launcher(debug_port) {
                luoda_codex_core::watcher::stop_launcher_processes();
                std::thread::sleep(std::time::Duration::from_millis(250));
                return acquire_single_instance_guard_with_retry(debug_port, false);
            }
            Ok(None)
        }
        Err(error) => Err(error)
            .with_context(|| {
                format!(
                    "failed to acquire launcher guard port {}",
                    luoda_codex_core::ports::LAUNCHER_GUARD_PORT
                )
            })
            .map(Some),
    }
}

fn try_acquire_single_instance_guard() -> std::io::Result<luoda_codex_core::ports::LoopbackPortGuard>
{
    luoda_codex_core::ports::acquire_resilient_loopback_port_guard(
        luoda_codex_core::ports::LAUNCHER_GUARD_PORT,
    )
}

fn log_launcher_guard_fallback(fallback_lock_path: &Path) {
    let _ = luoda_codex_core::diagnostic_log::append_diagnostic_log(
        "launcher.guard_fallback",
        json!({
            "requested_guard_port": luoda_codex_core::ports::LAUNCHER_GUARD_PORT,
            "fallback_lock_path": fallback_lock_path
        }),
    );
}

fn should_recover_stale_launcher(debug_port: u16) -> bool {
    let has_codex_process = !luoda_codex_core::watcher::find_codex_processes().is_empty();
    let cdp_listening = luoda_codex_core::watcher::cdp_listening(debug_port);
    let recover =
        luoda_codex_core::watcher::should_recover_stale_launcher(has_codex_process, cdp_listening);
    let _ = luoda_codex_core::diagnostic_log::append_diagnostic_log(
        "launcher.stale_recovery_check",
        json!({
            "debug_port": debug_port,
            "has_codex_process": has_codex_process,
            "cdp_listening": cdp_listening,
            "recover": recover
        }),
    );
    recover
}

async fn activate_existing_codex_app(options: &LaunchOptions) -> anyhow::Result<()> {
    let hooks = LauncherHooks::default();
    let settings = hooks.load_settings().await?;
    let app_dir = hooks.resolve_app_dir(options.app_dir.as_deref(), &settings)?;
    let launch_result = hooks
        .launch_codex(&app_dir, options.debug_port, &settings.codex_extra_args)
        .await;
    if settings.enhancements_enabled {
        hooks.start_helper(options.helper_port).await?;
    }
    let process_ids = luoda_codex_core::watcher::find_codex_processes();
    let mut activated = false;
    #[cfg(windows)]
    {
        for process_id in &process_ids {
            if luoda_codex_core::windows_activate_process_window(*process_id) {
                activated = true;
                break;
            }
        }
    }
    let injection_ready = if settings.enhancements_enabled {
        hooks
            .ensure_injection(options.debug_port, options.helper_port, &app_dir)
            .await
    } else {
        false
    };
    if injection_ready {
        hooks
            .start_bridge_watchdog(options.debug_port, options.helper_port)
            .await?;
        hooks.write_status("running").await;
    } else if settings.enhancements_enabled {
        hooks.write_status("running_degraded").await;
    }
    let _ = luoda_codex_core::diagnostic_log::append_diagnostic_log(
        "launcher.activate_existing_codex",
        json!({
            "app_dir": app_dir.to_string_lossy(),
            "debug_port": options.debug_port,
            "helper_port": options.helper_port,
            "process_ids": process_ids,
            "activated": activated,
            "injection_ready": injection_ready,
            "launch_ok": launch_result.is_ok(),
            "launch_error": launch_result.as_ref().err().map(|error| error.to_string())
        }),
    );
    launch_result.map(|_| ())
}

fn log_launcher_already_running(debug_port: u16) {
    let _ = luoda_codex_core::diagnostic_log::append_diagnostic_log(
        "launcher.already_running",
        json!({
            "guard_port": luoda_codex_core::ports::LAUNCHER_GUARD_PORT,
            "debug_port": debug_port
        }),
    );
}

async fn notify_manager_when_update_available() -> anyhow::Result<bool> {
    let update =
        luoda_codex_core::update::check_for_update(luoda_codex_core::version::VERSION).await?;
    if !update.update_available {
        return Ok(false);
    }
    open_manager_with_update_prompt()?;
    Ok(true)
}

fn open_manager_with_update_prompt() -> anyhow::Result<()> {
    let manager_path = manager_exe_path();
    let mut command = std::process::Command::new(&manager_path);
    command.arg("--show-update");
    #[cfg(windows)]
    {
        command.creation_flags(luoda_codex_core::windows_create_no_window());
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| anyhow::anyhow!("?????????{error}"))?;
        Ok(())
    }

    async fn apply_active_relay_profile(
        &self,
        settings: &luoda_codex_core::settings::BackendSettings,
    ) -> anyhow::Result<()> {
        self.core.apply_active_relay_profile(settings).await
    }

    async fn start_helper(&self, helper_port: u16) -> anyhow::Result<()> {
        self.core.start_helper(helper_port).await
    }

    async fn launch_codex(
        &self,
        app_dir: &Path,
        debug_port: u16,
        extra_args: &[String],
    ) -> anyhow::Result<luoda_codex_core::launcher::CodexLaunch> {
        self.core
            .launch_codex(app_dir, debug_port, extra_args)
            .await
    }

    async fn bridge_context(
        &self,
        debug_port: u16,
        app_dir: &Path,
    ) -> anyhow::Result<Option<BridgeContext>> {
        self.runtime.set_debug_port(debug_port);
        Ok(Some(BridgeContext::core_with_data_and_app_dir(
            self.runtime.clone(),
            self.data.clone(),
            app_dir.to_path_buf(),
        )))
    }

    async fn inject_bridge(
        &self,
        debug_port: u16,
        helper_port: u16,
        ctx: BridgeContext,
    ) -> anyhow::Result<()> {
        inject_with_context(debug_port, helper_port, ctx, self.runtime.clone()).await
    }

    async fn inject(&self, debug_port: u16, helper_port: u16) -> anyhow::Result<()> {
        self.core.inject(debug_port, helper_port).await
    }

    async fn write_status(&self, status: &str) {
        self.core.write_status(status).await;
    }

    async fn wait_for_codex_exit(
        &self,
        launch: &luoda_codex_core::launcher::CodexLaunch,
    ) -> anyhow::Result<()> {
        self.core.wait_for_codex_exit(launch).await
    }

    async fn shutdown_helper(&self, helper_port: u16) {
        self.core.shutdown_helper(helper_port).await;
    }

    async fn terminate_codex(&self, launch: &luoda_codex_core::launcher::CodexLaunch) {
        self.core.terminate_codex(launch).await;
    }
}

#[derive(Debug, Clone)]
struct LauncherDataService {
    db_path: PathBuf,
    backup_dir: PathBuf,
}

impl Default for LauncherDataService {
    fn default() -> Self {
        Self {
            db_path: default_codex_db_path(),
            backup_dir: luoda_codex_core::paths::default_app_state_dir().join("backups"),
        }
    }
}

#[async_trait::async_trait]
impl BridgeDataService for LauncherDataService {
    async fn delete(&self, session: SessionRef) -> anyhow::Result<DeleteResult> {
        let adapter = self.storage_adapter();
        tokio::task::spawn_blocking(move || adapter.delete_local(&session))
            .await
            .map_err(|error| anyhow::anyhow!("delete task failed: {error}"))
    }

    async fn undo(&self, undo_token: String) -> anyhow::Result<DeleteResult> {
        let adapter = self.storage_adapter();
        tokio::task::spawn_blocking(move || adapter.undo(&undo_token))
            .await
            .map_err(|error| anyhow::anyhow!("undo task failed: {error}"))
    }

    async fn export_markdown(&self, session: SessionRef) -> anyhow::Result<ExportResult> {
        let export_service =
            luoda_codex_data::MarkdownExportService::new(Some(self.db_path.clone()));
        tokio::task::spawn_blocking(move || export_service.export(&session))
            .await
            .map_err(|error| anyhow::anyhow!("export markdown task failed: {error}"))
    }

    async fn thread_usage_history(&self, session: SessionRef) -> anyhow::Result<Value> {
        let adapter = self.storage_adapter();
        tokio::task::spawn_blocking(move || adapter.codex_thread_usage_history(&session))
            .await
            .map_err(|error| anyhow::anyhow!("thread usage history task failed: {error}"))
    }

    async fn find_archived_thread_by_title(
        &self,
        title: String,
    ) -> anyhow::Result<Option<SessionRef>> {
        let adapter = self.storage_adapter();
        tokio::task::spawn_blocking(move || adapter.find_archived_thread_by_title(&title))
            .await
            .map_err(|error| anyhow::anyhow!("archived lookup task failed: {error}"))
    }

    async fn move_thread_workspace(
        &self,
        session: SessionRef,
        target_cwd: String,
    ) -> anyhow::Result<Value> {
        let adapter = self.storage_adapter();
        tokio::task::spawn_blocking(move || {
            adapter.move_codex_thread_workspace(&session, &target_cwd)
        })
        .await
        .map_err(|error| anyhow::anyhow!("move thread workspace task failed: {error}"))
    }

    async fn thread_sort_key(&self, session: SessionRef) -> anyhow::Result<Value> {
        let adapter = self.storage_adapter();
        tokio::task::spawn_blocking(move || adapter.codex_thread_sort_key(&session))
            .await
            .map_err(|error| anyhow::anyhow!("thread sort key task failed: {error}"))
    }

    async fn thread_sort_keys(&self, sessions: Vec<SessionRef>) -> anyhow::Result<Value> {
        let adapter = self.storage_adapter();
        tokio::task::spawn_blocking(move || adapter.codex_thread_sort_keys(&sessions))
            .await
            .map_err(|error| anyhow::anyhow!("thread sort keys task failed: {error}"))
    }
}

impl LauncherDataService {
    fn storage_adapter(&self) -> luoda_codex_data::SQLiteStorageAdapter {
        luoda_codex_data::SQLiteStorageAdapter::new(
            self.db_path.clone(),
            luoda_codex_data::BackupStore::new(self.backup_dir.clone()),
        )
    }
}

struct LauncherRuntimeService {
    debug_port: Mutex<u16>,
    websocket_url: Mutex<Option<String>>,
    user_scripts: UserScriptManager,
}

impl LauncherRuntimeService {
    fn new(debug_port: u16, user_scripts: UserScriptManager) -> Self {
        Self {
            debug_port: Mutex::new(debug_port),
            websocket_url: Mutex::new(None),
            user_scripts,
        }
    }

    fn set_debug_port(&self, debug_port: u16) {
        *self.debug_port.lock().unwrap() = debug_port;
    }

    fn set_websocket_url(&self, websocket_url: &str) {
        *self.websocket_url.lock().unwrap() = Some(websocket_url.to_string());
    }
}

#[async_trait::async_trait]
impl BridgeRuntimeService for LauncherRuntimeService {
    async fn user_script_inventory(&self) -> anyhow::Result<Value> {
        self.user_scripts.inventory()
    }

    async fn set_user_scripts_enabled(&self, enabled: bool) -> anyhow::Result<Value> {
        self.user_scripts.set_global_enabled(enabled)?;
        self.user_scripts.inventory()
    }

    async fn set_user_script_enabled(&self, key: String, enabled: bool) -> anyhow::Result<Value> {
        self.user_scripts.set_script_enabled(&key, enabled)?;
        self.user_scripts.inventory()
    }

    async fn delete_user_script(&self, key: String) -> anyhow::Result<Value> {
        self.user_scripts.delete_user_script(&key)?;
        self.user_scripts.inventory()
    }

    async fn reload_user_scripts(&self) -> anyhow::Result<Value> {
        let bundle = self.user_scripts.build_enabled_bundle()?;
        let websocket_url = self.websocket_url.lock().unwrap().clone();
        if let Some(websocket_url) = websocket_url.filter(|_| !bundle.trim().is_empty()) {
            luoda_codex_core::bridge::evaluate_script(&websocket_url, &bundle).await?;
        }
        self.user_scripts.inventory()
    }

    async fn open_devtools(&self) -> anyhow::Result<Value> {
        let debug_port = *self.debug_port.lock().unwrap();
        let targets = luoda_codex_core::cdp::list_targets(debug_port).await?;
        let target = luoda_codex_core::cdp::pick_page_target(&targets)?;
        let url = luoda_codex_core::routes::devtools_url(debug_port, &target.id);
        open_url(&url)?;
        Ok(json!({
            "status": "ok",
            "target_id": target.id,
            "url": url
        }))
    }

    async fn open_manager(&self) -> anyhow::Result<Value> {
        let manager_path = manager_exe_path();
        #[cfg(windows)]
        {
            std::process::Command::new(&manager_path)
                .creation_flags(luoda_codex_core::windows_create_no_window())
                .spawn()
                .map_err(|error| anyhow::anyhow!("鍚启动管理工具失败：{error}"))
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| anyhow::anyhow!("failed to open DevTools URL: {error}"))
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| anyhow::anyhow!("failed to open DevTools URL: {error}"))
    }

    #[cfg(not(any(windows, target_os = "macos", unix)))]
    {
        let _ = url;
        anyhow::bail!("opening DevTools URL is not supported on this platform")
    }
}

fn manager_exe_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    dir.join(format!(
        "{}{}",
        luoda_codex_core::install::MANAGER_BINARY,
        suffix
    ))
}

fn default_user_script_manager() -> UserScriptManager {
    let config_dir = default_user_scripts_config_dir();
    UserScriptManager::new(
        builtin_user_scripts_dir(),
        config_dir.join("user_scripts"),
        config_dir.join("user_scripts.json"),
    )
}

fn default_user_scripts_config_dir() -> PathBuf {
    if cfg!(windows) {
        if let Some(roaming) = std::env::var_os("APPDATA") {
            return PathBuf::from(roaming).join("LuodaCodex");
        }
        if let Some(home) = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) {
            return home.join("AppData").join("Roaming").join("LuodaCodex");
        }
    }
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|dirs| dirs.home_dir().join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("LuodaCodex")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_launch_options_accepts_manager_forwarded_ports_and_app_path() {
        let options = parse_launch_options([
            "--app-path",
            "C:/Codex/App",
            "--debug-port",
            "9333",
            "--helper-port",
            "57322",
        ]);

        assert_eq!(options.app_dir, Some(PathBuf::from("C:/Codex/App")));
        assert_eq!(options.debug_port, 9333);
        assert_eq!(options.helper_port, 57322);
    }

    #[test]
    fn parse_launch_options_ignores_invalid_ports() {
        let options = parse_launch_options(["--debug-port", "nope", "--helper-port", "70000"]);

        assert_eq!(options.debug_port, LaunchOptions::default().debug_port);
        assert_eq!(options.helper_port, LaunchOptions::default().helper_port);
    }

    #[test]
    fn launcher_uses_single_instance_guard_before_launching() {
        let source = include_str!("main.rs");

        assert!(source.contains("acquire_single_instance_guard(options.debug_port)?"));
        assert!(source.contains("LAUNCHER_GUARD_PORT"));
        assert!(source.contains("launcher.already_running"));
    }

    #[test]
    fn existing_instance_path_starts_helper_and_ensures_injection() {
        let source = include_str!("main.rs").replace("\r\n", "\n");

        assert!(source.contains(
            "async fn activate_existing_codex_app(options: &LaunchOptions) -> anyhow::Result<()> {\n    let hooks = LauncherHooks::default();"
        ));
        assert!(source.contains("hooks.start_helper(options.helper_port).await?"));
        assert!(
            source
                .contains("hooks.ensure_injection(options.debug_port, options.helper_port).await")
        );
        assert!(source.contains("injection_ready"));
    }

    #[test]
    fn manager_update_prompt_uses_sidecar_manager_binary_name() {
        let path = manager_exe_path();

        assert!(
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(luoda_codex_core::install::MANAGER_BINARY))
        );
    }
}

fn builtin_user_scripts_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .map(|path| path.join("user_scripts"))
        .unwrap_or_else(|| PathBuf::from("user_scripts"))
}
