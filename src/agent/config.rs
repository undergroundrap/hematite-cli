/// Hematite project-level configuration.
///
/// Read from `.hematite/settings.json` in the workspace root.
/// Re-loaded at the start of every turn so edits take effect without restart.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn default_true() -> bool { true }

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq)]
pub enum PermissionMode {
    #[default]
    Developer,
    ReadOnly,
    SystemAdmin,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
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
    /// When true, Gemma 4 models enable native-formatting behavior automatically unless explicitly forced off.
    #[serde(default = "default_true")]
    pub gemma_native_auto: bool,
    /// Force Gemma-native request shaping on for Gemma 4 models.
    #[serde(default)]
    pub gemma_native_formatting: bool,
    /// Override the LLM provider base URL (e.g. "http://localhost:11434/v1" for Ollama).
    /// Defaults to "http://localhost:1234/v1" (LM Studio). Takes precedence over --url CLI flag.
    pub api_url: Option<String>,
    /// Extra text appended verbatim to the system prompt (project notes, conventions, etc.).
    pub context_hint: Option<String>,
    /// Per-project verification commands for build/test/lint/fix workflows.
    #[serde(default)]
    pub verify: VerifyProfilesConfig,
    /// Tool Lifecycle Hooks for automated pre/post scripts.
    #[serde(default)]
    pub hooks: crate::agent::hooks::RuntimeHookConfig,
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
    crate::tools::file_ops::workspace_root()
        .join(".hematite")
        .join("settings.json")
}

/// Load `.hematite/settings.json` from the workspace root.
/// Returns default config if the file doesn't exist or can't be parsed.
pub fn load_config() -> HematiteConfig {
    let path = settings_path();

    if !path.exists() {
        write_default_config(&path);
        return HematiteConfig::default();
    }

    let Ok(data) = std::fs::read_to_string(&path) else {
        return HematiteConfig::default();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_config(config: &HematiteConfig) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
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

pub fn effective_gemma_native_formatting(
    config: &HematiteConfig,
    model_name: &str,
) -> bool {
    crate::agent::inference::is_gemma4_model_name(model_name)
        && (config.gemma_native_formatting || config.gemma_native_auto)
}

pub fn gemma_native_mode_label(config: &HematiteConfig, model_name: &str) -> &'static str {
    if !crate::agent::inference::is_gemma4_model_name(model_name) {
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
  "context_hint": null,
  "model": null,
  "fast_model": null,
  "think_model": null,
  "gemma_native_auto": true,
  "gemma_native_formatting": false,

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
