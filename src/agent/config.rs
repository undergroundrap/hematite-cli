/// Hematite project-level configuration.
///
/// Read from `.hematite/settings.json` in the workspace root.
/// Re-loaded at the start of every turn so edits take effect without restart.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT_LM_STUDIO_API_URL: &str = "http://localhost:1234/v1";
pub const DEFAULT_OLLAMA_API_URL: &str = "http://localhost:11434/v1";

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq)]
pub enum PermissionMode {
    #[default]
    Developer,
    ReadOnly,
    SystemAdmin,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HematiteConfig {
    /// Active authority mode.
    #[serde(default)]
    pub mode: PermissionMode,
    /// Pattern-based permission overrides.
    pub permissions: Option<PermissionRules>,
    /// Workspace trust policy for the current project root.
    #[serde(default)]
    pub trust: WorkspaceTrustConfig,
    /// Override the primary model ID (e.g. "gemma-4-e4b").
    pub model: Option<String>,
    /// Override the fast model ID used for read-only tasks.
    pub fast_model: Option<String>,
    /// Override the think model ID used for complex tasks.
    pub think_model: Option<String>,
    /// Preferred embedding model to keep loaded for semantic search.
    pub embed_model: Option<String>,
    /// When true, Gemma 4 models enable native-formatting behavior automatically unless explicitly forced off.
    #[serde(default = "default_true")]
    pub gemma_native_auto: bool,
    /// Force Gemma-native request shaping on for Gemma 4 models.
    #[serde(default)]
    pub gemma_native_formatting: bool,
    /// Override the LLM provider base URL (e.g. "http://localhost:11434/v1" for Ollama).
    /// Defaults to "http://localhost:1234/v1" (LM Studio). Takes precedence over --url CLI flag.
    pub api_url: Option<String>,
    /// Voice ID for TTS. Use /voice in the TUI to list and select. Defaults to "af_sky".
    pub voice: Option<String>,
    /// TTS speech speed multiplier. 1.0 = normal, 0.8 = slower, 1.3 = faster. Defaults to 1.0.
    pub voice_speed: Option<f32>,
    /// TTS volume. 0.0 = silent, 1.0 = normal, 2.0 = louder. Defaults to 1.0.
    pub voice_volume: Option<f32>,
    /// Extra text appended verbatim to the system prompt (project notes, conventions, etc.).
    pub context_hint: Option<String>,
    /// Override path to the Deno executable for the run_code sandbox.
    /// If unset, Hematite checks LM Studio's bundled Deno, then system PATH.
    /// Example: "C:/Users/you/.deno/bin/deno.exe"
    pub deno_path: Option<String>,
    /// Per-project verification commands for build/test/lint/fix workflows.
    #[serde(default)]
    pub verify: VerifyProfilesConfig,
    /// Tool Lifecycle Hooks for automated pre/post scripts.
    #[serde(default)]
    pub hooks: crate::agent::hooks::RuntimeHookConfig,
    /// Optional local SearXNG URL (e.g. "http://localhost:8080") for private research.
    /// If set, research_web will prioritize this endpoint over external search proxies.
    pub searx_url: Option<String>,
    /// When true, Hematite will attempt to automatically start SearXNG on startup if it's offline.
    #[serde(default = "default_true")]
    pub auto_start_searx: bool,
    /// When true, Hematite stops a SearXNG stack on exit only if this session started it.
    #[serde(default)]
    pub auto_stop_searx: bool,
}

impl Default for HematiteConfig {
    fn default() -> Self {
        Self {
            mode: PermissionMode::Developer,
            permissions: None,
            trust: WorkspaceTrustConfig::default(),
            model: None,
            fast_model: None,
            think_model: None,
            embed_model: None,
            gemma_native_auto: true,
            gemma_native_formatting: false,
            api_url: None,
            voice: None,
            voice_speed: None,
            voice_volume: None,
            context_hint: None,
            deno_path: None,
            verify: VerifyProfilesConfig::default(),
            hooks: crate::agent::hooks::RuntimeHookConfig::default(),
            searx_url: None,
            auto_start_searx: true,
            auto_stop_searx: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorkspaceTrustConfig {
    /// Workspace roots trusted for normal destructive and external tool posture.
    #[serde(default = "default_trusted_workspace_roots")]
    pub allow: Vec<String>,
    /// Workspace roots explicitly denied for destructive and external tool posture.
    #[serde(default)]
    pub deny: Vec<String>,
}

impl Default for WorkspaceTrustConfig {
    fn default() -> Self {
        Self {
            allow: default_trusted_workspace_roots(),
            deny: Vec::new(),
        }
    }
}

fn default_trusted_workspace_roots() -> Vec<String> {
    vec![".".to_string()]
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct VerifyProfilesConfig {
    /// Optional default profile name to use when verify_build is called without an explicit profile.
    pub default_profile: Option<String>,
    /// Named verification profiles keyed by stack or workspace role.
    #[serde(default)]
    pub profiles: BTreeMap<String, VerifyProfile>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct VerifyProfile {
    /// Build/compile validation command.
    pub build: Option<String>,
    /// Test command.
    pub test: Option<String>,
    /// Lint/static analysis command.
    pub lint: Option<String>,
    /// Optional auto-fix command, typically lint --fix or formatter repair.
    pub fix: Option<String>,
    /// Optional timeout override for this profile.
    pub timeout_secs: Option<u64>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct PermissionRules {
    /// Always auto-approve these patterns (e.g. "cargo *", "git status").
    #[serde(default)]
    pub allow: Vec<String>,
    /// Always require approval for these patterns (e.g. "git push *").
    #[serde(default)]
    pub ask: Vec<String>,
    /// Always deny these patterns outright (e.g. "rm -rf *").
    #[serde(default)]
    pub deny: Vec<String>,
}

pub fn settings_path() -> std::path::PathBuf {
    crate::tools::file_ops::hematite_dir().join("settings.json")
}

/// Load global settings from `~/.hematite/settings.json` if present.
fn load_global_config() -> Option<HematiteConfig> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    let path = std::path::PathBuf::from(home)
        .join(".hematite")
        .join("settings.json");
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Load `.hematite/settings.json` from the workspace root, with global
/// `~/.hematite/settings.json` as a fallback for unset fields.
/// Workspace config always wins; global fills in what workspace doesn't set.
pub fn load_config() -> HematiteConfig {
    let path = settings_path();

    let workspace: Option<HematiteConfig> = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|d| serde_json::from_str(&d).ok())
    } else {
        write_default_config(&path);
        None
    };

    let global = load_global_config();

    match (workspace, global) {
        (Some(ws), Some(gb)) => {
            // Workspace wins on every field that isn't the zero/null default
            HematiteConfig {
                model: ws.model.or(gb.model),
                fast_model: ws.fast_model.or(gb.fast_model),
                think_model: ws.think_model.or(gb.think_model),
                embed_model: ws.embed_model.or(gb.embed_model),
                api_url: ws.api_url.or(gb.api_url),
                voice: if ws.voice != HematiteConfig::default().voice {
                    ws.voice
                } else {
                    gb.voice
                },
                voice_speed: ws.voice_speed.or(gb.voice_speed),
                voice_volume: ws.voice_volume.or(gb.voice_volume),
                context_hint: ws.context_hint.or(gb.context_hint),
                searx_url: ws.searx_url.or(gb.searx_url),
                auto_start_searx: ws.auto_start_searx, // Workspace setting always takes priority.
                auto_stop_searx: ws.auto_stop_searx,   // Workspace setting always takes priority.
                gemma_native_auto: ws.gemma_native_auto,
                gemma_native_formatting: ws.gemma_native_formatting,
                ..ws
            }
        }
        (Some(ws), None) => ws,
        (None, Some(gb)) => gb,
        (None, None) => HematiteConfig::default(),
    }
}

pub fn save_config(config: &HematiteConfig) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn provider_label_for_api_url(url: &str) -> &'static str {
    let normalized = url.trim().trim_end_matches('/').to_ascii_lowercase();
    if normalized.contains("11434") || normalized.contains("ollama") {
        "Ollama"
    } else if normalized.contains("1234") || normalized.contains("lmstudio") {
        "LM Studio"
    } else {
        "Custom"
    }
}

pub fn default_api_url_for_provider(provider_name: &str) -> &'static str {
    match provider_name {
        "Ollama" => DEFAULT_OLLAMA_API_URL,
        _ => DEFAULT_LM_STUDIO_API_URL,
    }
}

pub fn effective_api_url(config: &HematiteConfig, cli_default: &str) -> String {
    config
        .api_url
        .clone()
        .unwrap_or_else(|| cli_default.to_string())
}

pub fn set_api_url_override(url: Option<&str>) -> Result<(), String> {
    let mut config = load_config();
    config.api_url = url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    save_config(&config)
}

pub fn preferred_coding_model(config: &HematiteConfig) -> Option<String> {
    config
        .think_model
        .clone()
        .or(config.model.clone())
        .or(config.fast_model.clone())
}

pub fn set_preferred_coding_model(model_id: Option<&str>) -> Result<(), String> {
    let mut config = load_config();
    let normalized = model_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    config.think_model = normalized.clone();
    if normalized.is_some() {
        config.model = None;
    }
    save_config(&config)
}

pub fn set_preferred_embed_model(model_id: Option<&str>) -> Result<(), String> {
    let mut config = load_config();
    config.embed_model = model_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    save_config(&config)
}

pub fn set_gemma_native_formatting(enabled: bool) -> Result<(), String> {
    set_gemma_native_mode(if enabled { "on" } else { "off" })
}

pub fn set_gemma_native_mode(mode: &str) -> Result<(), String> {
    let mut config = load_config();
    match mode {
        "on" => {
            config.gemma_native_auto = false;
            config.gemma_native_formatting = true;
        }
        "off" => {
            config.gemma_native_auto = false;
            config.gemma_native_formatting = false;
        }
        "auto" => {
            config.gemma_native_auto = true;
            config.gemma_native_formatting = false;
        }
        _ => return Err(format!("Unknown gemma native mode: {}", mode)),
    }
    save_config(&config)
}

pub fn set_voice(voice_id: &str) -> Result<(), String> {
    let mut config = load_config();
    config.voice = Some(voice_id.to_string());
    save_config(&config)
}

pub fn effective_voice(config: &HematiteConfig) -> String {
    config.voice.clone().unwrap_or_else(|| "af_sky".to_string())
}

pub fn effective_voice_speed(config: &HematiteConfig) -> f32 {
    config.voice_speed.unwrap_or(1.0).clamp(0.5, 2.0)
}

pub fn effective_voice_volume(config: &HematiteConfig) -> f32 {
    config.voice_volume.unwrap_or(1.0).clamp(0.0, 3.0)
}

pub fn effective_gemma_native_formatting(config: &HematiteConfig, model_name: &str) -> bool {
    crate::agent::inference::is_hematite_native_model(model_name)
        && (config.gemma_native_formatting || config.gemma_native_auto)
}

pub fn gemma_native_mode_label(config: &HematiteConfig, model_name: &str) -> &'static str {
    if !crate::agent::inference::is_hematite_native_model(model_name) {
        "inactive"
    } else if config.gemma_native_formatting {
        "on"
    } else if config.gemma_native_auto {
        "auto"
    } else {
        "off"
    }
}

/// Write a commented default config on first run so users know what's available.
fn write_default_config(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let default = r#"{
  "_comment": "Hematite settings — edit and save, changes apply immediately without restart.",

  "permissions": {
    "allow": [
      "cargo *",
      "git status",
      "git log *",
      "git diff *",
      "git branch *"
    ],
    "ask": [],
    "deny": []
  },

  "trust": {
    "allow": ["."],
    "deny": []
  },

  "auto_approve_moderate": false,

  "api_url": null,
  "voice": null,
  "voice_speed": null,
  "voice_volume": null,
  "context_hint": null,
  "model": null,
  "fast_model": null,
  "think_model": null,
  "embed_model": null,
  "gemma_native_auto": true,
  "gemma_native_formatting": false,
  "searx_url": null,
  "auto_start_searx": true,
  "auto_stop_searx": false,

  "verify": {
    "default_profile": null,
    "profiles": {
      "rust": {
        "build": "cargo build --color never",
        "test": "cargo test --color never",
        "lint": "cargo clippy --all-targets --all-features -- -D warnings",
        "fix": "cargo fmt",
        "timeout_secs": 120
      }
    }
  },

  "hooks": {
    "pre_tool_use": [],
    "post_tool_use": []
  }
  }
"#;
    let _ = std::fs::write(path, default);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_label_for_api_url_detects_known_runtimes() {
        assert_eq!(
            provider_label_for_api_url("http://localhost:1234/v1"),
            "LM Studio"
        );
        assert_eq!(
            provider_label_for_api_url("http://localhost:11434/v1"),
            "Ollama"
        );
        assert_eq!(
            provider_label_for_api_url("https://ai.example.com/v1"),
            "Custom"
        );
    }

    #[test]
    fn default_api_url_for_provider_maps_presets() {
        assert_eq!(
            default_api_url_for_provider("LM Studio"),
            DEFAULT_LM_STUDIO_API_URL
        );
        assert_eq!(
            default_api_url_for_provider("Ollama"),
            DEFAULT_OLLAMA_API_URL
        );
        assert_eq!(
            default_api_url_for_provider("Custom"),
            DEFAULT_LM_STUDIO_API_URL
        );
    }

    #[test]
    fn preferred_coding_model_prefers_think_then_model_then_fast() {
        let mut config = HematiteConfig::default();
        config.fast_model = Some("fast".into());
        assert_eq!(preferred_coding_model(&config), Some("fast".to_string()));

        config.model = Some("main".into());
        assert_eq!(preferred_coding_model(&config), Some("main".to_string()));

        config.think_model = Some("think".into());
        assert_eq!(preferred_coding_model(&config), Some("think".to_string()));
    }
}

/// Returns the permission decision for a shell command given the loaded config.
///
/// Priority order (highest first):
/// 1. deny rules  → always block (return true = needs approval / will be rejected)
/// 2. allow rules → always approve (return false)
/// 3. ask rules   → always ask (return true)
/// 4. intrinsic risk classifier
pub fn permission_for_shell(cmd: &str, config: &HematiteConfig) -> PermissionDecision {
    if let Some(rules) = &config.permissions {
        for pattern in &rules.deny {
            if glob_matches(pattern, cmd) {
                return PermissionDecision::Deny;
            }
        }
        for pattern in &rules.allow {
            if glob_matches(pattern, cmd) {
                return PermissionDecision::Allow;
            }
        }
        for pattern in &rules.ask {
            if glob_matches(pattern, cmd) {
                return PermissionDecision::Ask;
            }
        }
    }
    PermissionDecision::UseRiskClassifier
}

#[derive(Debug, PartialEq)]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
    UseRiskClassifier,
}

/// Simple glob matcher: `*` is a wildcard, matching is case-insensitive.
/// `cargo *` matches `cargo build`, `cargo check --all-targets`, etc.
pub fn glob_matches(pattern: &str, text: &str) -> bool {
    let p = pattern.to_lowercase();
    let t = text.to_lowercase();
    if p == "*" {
        return true;
    }
    if let Some(star) = p.find('*') {
        let prefix = &p[..star];
        let suffix = &p[star + 1..];
        t.starts_with(prefix) && (suffix.is_empty() || t.ends_with(suffix))
    } else {
        t.contains(&p)
    }
}
