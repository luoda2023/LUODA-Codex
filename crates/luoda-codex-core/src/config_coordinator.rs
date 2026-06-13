use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::ccs_import::{self, CcsProviderImport};
use crate::relay_config::{self, RelayApplyResult};
use crate::settings::{BackendSettings, ConfigOwnership};

const WRITE_MARKER_FILE: &str = "config-write-marker.json";
const WRITER_luoda_codex_PLUS: &str = "codexplusplus";
const WRITER_CC_SWITCH: &str = "ccswitch";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveConfigFingerprint {
    pub config_hash: String,
    pub auth_hash: String,
    pub model_provider: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigWriteMarker {
    pub writer: String,
    pub fingerprint: LiveConfigFingerprint,
    pub written_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinationStatus {
    pub ccswitch_detected: bool,
    pub configured_ownership: ConfigOwnership,
    pub effective_ownership: ConfigOwnership,
    pub last_writer: Option<String>,
    pub conflict_detected: bool,
    pub conflict_message: String,
    pub ccswitch_current_provider_id: Option<String>,
    pub ccswitch_current_provider_name: Option<String>,
    pub live_model_provider: String,
    pub can_write_live_config: bool,
    pub guidance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveConfigWriteDecision {
    pub allowed: bool,
    pub message: String,
}

pub fn detect_ccswitch() -> bool {
    ccs_import::default_ccs_db_path().exists()
}

pub fn effective_ownership(settings: &BackendSettings) -> ConfigOwnership {
    match settings.config_ownership {
        ConfigOwnership::Auto => {
            if settings.ccs_link_enabled && detect_ccswitch() {
                ConfigOwnership::CcSwitch
            } else {
                ConfigOwnership::CodexPlusPlus
            }
        }
        other => other,
    }
}

pub fn write_marker_path() -> PathBuf {
    crate::paths::default_app_state_dir().join(WRITE_MARKER_FILE)
}

pub fn fingerprint_from_home(home: &std::path::Path) -> LiveConfigFingerprint {
    let config_path = home.join("config.toml");
    let auth_path = home.join("auth.json");
    let config_bytes = fs::read(&config_path).unwrap_or_default();
    let auth_bytes = fs::read(&auth_path).unwrap_or_default();
    let config_text = String::from_utf8_lossy(&config_bytes);
    LiveConfigFingerprint {
        config_hash: hash_bytes(&config_bytes),
        auth_hash: hash_bytes(&auth_bytes),
        model_provider: relay_config::root_key_string(&config_text, "model_provider")
            .unwrap_or_default(),
    }
}

pub fn read_write_marker() -> Option<ConfigWriteMarker> {
    let path = write_marker_path();
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn record_write_marker(writer: &str, home: &std::path::Path) -> anyhow::Result<()> {
    let marker = ConfigWriteMarker {
        writer: writer.to_string(),
        fingerprint: fingerprint_from_home(home),
        written_at_ms: now_ms(),
    };
    let path = write_marker_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::settings::atomic_write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&marker)?).as_bytes(),
    )
}

pub fn detect_external_modification(home: &std::path::Path) -> Option<String> {
    let marker = read_write_marker()?;
    let current = fingerprint_from_home(home);
    if marker.fingerprint == current {
        return None;
    }
    let writer = marker.writer.trim();
    if writer == WRITER_luoda_codex_PLUS {
        Some(
            "检测到 config.toml / auth.json �?Codex++ 上次写入后被外部修改。\
             这通常�?CC Switch 切换供应商引起�?
                .to_string(),
        )
    } else if writer == WRITER_CC_SWITCH {
        Some(
            "检测到 live 配置�?CC Switch 上次写入记录不一致。\
             请先�?CC Switch 中重新启用当前供应商，或刷新 Codex++ 联动状态�?
                .to_string(),
        )
    } else {
        Some(
            "检测到 config.toml / auth.json �?Codex++ 记录不一致，\
             可能已被其他工具改写�?
                .to_string(),
        )
    }
}

pub fn coordination_status(settings: &BackendSettings) -> CoordinationStatus {
    let home = relay_config::default_codex_home_dir();
    let ccswitch_detected = detect_ccswitch();
    let configured = settings.config_ownership;
    let effective = effective_ownership(settings);
    let live = fingerprint_from_home(&home);
    let marker = read_write_marker();
    let current_ccs = current_ccs_codex_provider();
    let conflict_message = detect_external_modification(&home).unwrap_or_default();
    let conflict_detected = !conflict_message.is_empty();
    let can_write = evaluate_live_write(settings, false).allowed;
    let guidance = coordination_guidance(
        settings,
        effective,
        ccswitch_detected,
        conflict_detected,
        &current_ccs,
        &live.model_provider,
    );

    CoordinationStatus {
        ccswitch_detected,
        configured_ownership: configured,
        effective_ownership: effective,
        last_writer: marker.map(|value| value.writer),
        conflict_detected,
        conflict_message,
        ccswitch_current_provider_id: current_ccs.as_ref().map(|provider| provider.source_id.clone()),
        ccswitch_current_provider_name: current_ccs.as_ref().map(|provider| provider.name.clone()),
        live_model_provider: live.model_provider,
        can_write_live_config: can_write,
        guidance,
    }
}

pub fn evaluate_live_write(settings: &BackendSettings, force: bool) -> LiveConfigWriteDecision {
    if !settings.relay_profiles_enabled {
        return LiveConfigWriteDecision {
            allowed: false,
            message: "供应商配置总开关已关闭，Codex++ 不会写入 config.toml / auth.json�?.to_string(),
        };
    }

    let effective = effective_ownership(settings);
    if effective == ConfigOwnership::CcSwitch && !settings.ccs_link_enabled {
        return LiveConfigWriteDecision {
            allowed: false,
            message: "当前配置所有权�?CC Switch，但未开启联�?cc-switch。\
                      请开启联动，或将配置所有权改为 Codex++�?
                .to_string(),
        };
    }

    if effective == ConfigOwnership::CcSwitch && !force {
        return LiveConfigWriteDecision {
            allowed: false,
            message: "当前�?CC Switch 管理 Codex 供应商配置。\
                      Codex++ 不会直接覆盖 live 配置；请通过联动供应商切换，或在 CC Switch 中切换后刷新�?
                .to_string(),
        };
    }

    if detect_ccswitch()
        && effective == ConfigOwnership::CodexPlusPlus
        && !force
        && let Some(conflict) = detect_external_modification(&relay_config::default_codex_home_dir())
    {
        return LiveConfigWriteDecision {
            allowed: false,
            message: format!(
                "{conflict} 若确认由 Codex++ 接管，请先将“配置所有权”设�?Codex++ 并强制切换�?
            ),
        };
    }

    LiveConfigWriteDecision {
        allowed: true,
        message: String::new(),
    }
}

pub fn apply_linked_ccs_provider_to_home(
    source_id: &str,
    common_config_contents: &str,
) -> anyhow::Result<RelayApplyResult> {
    let provider = current_ccs_codex_provider_by_id(source_id)
        .with_context(|| format!("未在 cc-switch 中找�?Codex 供应�?{source_id}"))?;
    apply_ccs_provider_import_to_home(&provider, common_config_contents)
}

pub fn apply_current_ccs_provider_to_home(
    common_config_contents: &str,
) -> anyhow::Result<RelayApplyResult> {
    let provider = current_ccs_codex_provider()
        .ok_or_else(|| anyhow::anyhow!("cc-switch 中未找到当前启用�?Codex 供应�?))?;
    apply_ccs_provider_import_to_home(&provider, common_config_contents)
}

pub fn sync_active_profile_from_ccs(settings: &mut BackendSettings) -> bool {
    if !settings.ccs_link_enabled {
        return false;
    }
    let Some(current) = current_ccs_codex_provider() else {
        return false;
    };
    if let Some(profile) = settings
        .relay_profiles
        .iter_mut()
        .find(|profile| profile.linked_ccs_provider_id == current.source_id)
    {
        settings.active_relay_id = profile.id.clone();
        ccs_import::apply_ccs_provider_to_profile(profile, &current);
        return true;
    }
    let existing_ids = settings
        .relay_profiles
        .iter()
        .map(|profile| profile.id.clone())
        .collect::<Vec<_>>();
    let mut profile = ccs_import::relay_profile_from_ccs(&current, &existing_ids);
    ccs_import::apply_ccs_provider_to_profile(&mut profile, &current);
    settings.active_relay_id = profile.id.clone();
    settings.relay_profiles.push(profile);
    true
}

fn apply_ccs_provider_import_to_home(
    provider: &CcsProviderImport,
    common_config_contents: &str,
) -> anyhow::Result<RelayApplyResult> {
    let home = relay_config::default_codex_home_dir();
    let config_with_common = relay_config::merge_common_config_into_config(
        &provider.config_contents,
        common_config_contents,
    )?;
    let result = relay_config::apply_relay_files_to_home(
        &home,
        &config_with_common,
        &provider.auth_contents,
    )?;
    record_write_marker(WRITER_CC_SWITCH, &home)?;
    Ok(result)
}

pub fn current_ccs_codex_provider() -> Option<CcsProviderImport> {
    ccs_import::current_codex_provider_from_db(&ccs_import::default_ccs_db_path())
        .ok()
        .flatten()
}

pub fn current_ccs_codex_provider_by_id(source_id: &str) -> Option<CcsProviderImport> {
    let source_id = source_id.trim();
    if source_id.is_empty() {
        return None;
    }
    ccs_import::list_codex_providers_from_db(&ccs_import::default_ccs_db_path())
        .ok()?
        .into_iter()
        .find(|provider| provider.source_id == source_id)
}

fn coordination_guidance(
    settings: &BackendSettings,
    effective: ConfigOwnership,
    ccswitch_detected: bool,
    conflict_detected: bool,
    current_ccs: &Option<CcsProviderImport>,
    live_model_provider: &str,
) -> String {
    if !ccswitch_detected {
        return "未检测到 CC Switch，Codex++ 将独立管�?~/.codex/config.toml�?.to_string();
    }
    if conflict_detected {
        return "检测到配置冲突：请统一只在一个工具中切换供应商，或明确设置“配置所有权”�?.to_string();
    }
    match effective {
        ConfigOwnership::CcSwitch => {
            let provider = current_ccs
                .as_ref()
                .map(|value| value.name.as_str())
                .unwrap_or("未知");
            format!(
                "当前�?CC Switch 管理供应商配置。Codex++ 会通过联动读取/回写 cc-switch 数据库，\
                 不会直接覆盖 live 配置。CC Switch 当前供应商：{provider}；live model_provider：{live_model_provider}�?
            )
        }
        ConfigOwnership::CodexPlusPlus => {
            if settings.ccs_link_enabled {
                "已开�?cc-switch 联动，但配置所有权仍为 Codex++。切换供应商�?Codex++ 会写�?live 配置，\
                 可能覆盖 CC Switch 的选择�?
                    .to_string()
            } else {
                "Codex++ 独立管理 live 配置。若同时使用 CC Switch，建议开启联动并设为 auto/ccswitch 所有权�?
                    .to_string()
            }
        }
        ConfigOwnership::Auto => "自动模式会根据联动开关和 CC Switch 是否存在选择所有权�?.to_string(),
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
