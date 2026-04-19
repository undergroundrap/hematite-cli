// Redaction policy — loaded from .hematite/redact_policy.json (workspace)
// or ~/.hematite/redact_policy.json (global). Workspace overrides global.
//
// Controls which inspect_host topics the MCP server will serve, and at what
// redaction level. An absent policy file means "allow all, Tier 1 regex only".
//
// Example policy:
// {
//   "blocked_topics": ["user_accounts", "credentials", "audit_policy"],
//   "allowed_topics": [],
//   "topic_redaction_level": { "network": "semantic", "hardware": "regex" },
//   "default_redaction_level": "regex"
// }

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RedactPolicy {
    /// Topics that are hard-blocked — MCP returns an error, never runs the inspection.
    #[serde(default)]
    pub blocked_topics: HashSet<String>,

    /// If non-empty, only these topics are allowed (whitelist mode).
    /// An empty vec means all topics are allowed (subject to blocked_topics).
    #[serde(default)]
    pub allowed_topics: Vec<String>,

    /// Per-topic redaction level override.
    /// Values: "none" | "regex" | "semantic"
    #[serde(default)]
    pub topic_redaction_level: HashMap<String, RedactionLevel>,

    /// Fallback level when no per-topic override exists.
    /// Defaults to "regex" when edge_redact is active, "none" otherwise.
    #[serde(default)]
    pub default_redaction_level: Option<RedactionLevel>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RedactionLevel {
    /// Pass through unchanged.
    None,
    /// Apply Tier 1 regex patterns only (fast, deterministic).
    Regex,
    /// Route through local model semantic summarizer, then Tier 1 as safety net.
    Semantic,
}

impl RedactPolicy {
    /// Check whether a topic is blocked by policy.
    pub fn is_blocked(&self, topic: &str) -> bool {
        let t = topic.to_lowercase();
        if self.blocked_topics.contains(&t) {
            return true;
        }
        // Whitelist mode: if allowed_topics is set and this topic isn't in it, block it.
        if !self.allowed_topics.is_empty() {
            return !self.allowed_topics.iter().any(|a| a.to_lowercase() == t);
        }
        false
    }

    /// Effective redaction level for a topic, given whether --edge-redact was passed.
    pub fn redaction_level(&self, topic: &str, edge_redact_active: bool) -> RedactionLevel {
        let t = topic.to_lowercase();
        if let Some(level) = self.topic_redaction_level.get(&t) {
            return level.clone();
        }
        if let Some(ref default) = self.default_redaction_level {
            return default.clone();
        }
        if edge_redact_active {
            RedactionLevel::Regex
        } else {
            RedactionLevel::None
        }
    }
}

/// Load policy from workspace then global, workspace wins.
pub fn load_policy() -> RedactPolicy {
    // Workspace: .hematite/redact_policy.json
    let workspace_path = Path::new(".hematite").join("redact_policy.json");
    if let Some(policy) = try_load(&workspace_path) {
        eprintln!(
            "[hematite mcp] loaded redact policy from {}",
            workspace_path.display()
        );
        return policy;
    }

    // Global: ~/.hematite/redact_policy.json
    if let Some(home) = home_dir() {
        let global_path = home.join(".hematite").join("redact_policy.json");
        if let Some(policy) = try_load(&global_path) {
            eprintln!(
                "[hematite mcp] loaded redact policy from {}",
                global_path.display()
            );
            return policy;
        }
    }

    RedactPolicy::default()
}

fn try_load(path: &Path) -> Option<RedactPolicy> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<RedactPolicy>(&text) {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!(
                "[hematite mcp] redact_policy parse error at {}: {e}",
                path.display()
            );
            None
        }
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy_with_blocked(topics: &[&str]) -> RedactPolicy {
        RedactPolicy {
            blocked_topics: topics.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn blocks_exact_topic() {
        let p = policy_with_blocked(&["user_accounts", "credentials"]);
        assert!(p.is_blocked("user_accounts"));
        assert!(p.is_blocked("credentials"));
        assert!(!p.is_blocked("network"));
    }

    #[test]
    fn block_check_is_case_insensitive() {
        let p = policy_with_blocked(&["user_accounts"]);
        assert!(p.is_blocked("User_Accounts"));
        assert!(p.is_blocked("USER_ACCOUNTS"));
    }

    #[test]
    fn whitelist_mode_blocks_unlisted_topics() {
        let p = RedactPolicy {
            allowed_topics: vec!["network".into(), "storage".into()],
            ..Default::default()
        };
        assert!(!p.is_blocked("network"));
        assert!(!p.is_blocked("storage"));
        assert!(p.is_blocked("user_accounts"));
        assert!(p.is_blocked("credentials"));
    }

    #[test]
    fn default_redaction_level_follows_edge_redact_flag() {
        let p = RedactPolicy::default();
        assert_eq!(p.redaction_level("network", true), RedactionLevel::Regex);
        assert_eq!(p.redaction_level("network", false), RedactionLevel::None);
    }

    #[test]
    fn per_topic_override_takes_precedence() {
        let mut p = RedactPolicy::default();
        p.topic_redaction_level
            .insert("network".into(), RedactionLevel::Semantic);
        assert_eq!(
            p.redaction_level("network", false),
            RedactionLevel::Semantic
        );
        assert_eq!(p.redaction_level("storage", true), RedactionLevel::Regex);
    }
}
