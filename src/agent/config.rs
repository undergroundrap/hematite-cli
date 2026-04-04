/// Hematite project-level configuration.
///
/// Read from `.hematite/settings.json` in the workspace root.
/// Re-loaded at the start of every turn so edits take effect without restart.
use serde::{Deserialize, Serialize};

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
    /// Override the primary model ID (e.g. "gemma-4-e4b").
    pub model: Option<String>,
    /// Override the fast model ID used for read-only tasks.
    pub fast_model: Option<String>,
    /// Override the think model ID used for complex tasks.
    pub think_model: Option<String>,
    /// Extra text appended verbatim to the system prompt (project notes, conventions, etc.).
    pub context_hint: Option<String>,
    /// Tool Lifecycle Hooks for automated pre/post scripts.
    #[serde(default)]
    pub hooks: crate::agent::hooks::RuntimeHookConfig,
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

/// Load `.hematite/settings.json` from the workspace root.
/// Returns default config if the file doesn't exist or can't be parsed.
pub fn load_config() -> HematiteConfig {
    let path = crate::tools::file_ops::workspace_root()
        .join(".hematite")
        .join("settings.json");

    if !path.exists() {
        write_default_config(&path);
        return HematiteConfig::default();
    }

    let Ok(data) = std::fs::read_to_string(&path) else {
        return HematiteConfig::default();
    };
    serde_json::from_str(&data).unwrap_or_default()
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

  "auto_approve_moderate": false,

  "context_hint": null,
  "model": null,
  "fast_model": null,
  "think_model": null,

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
