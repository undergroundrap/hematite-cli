use crate::agent::types::ChatMessage;
use serde_json::Value;

pub(crate) fn is_destructive_tool(name: &str) -> bool {
    crate::agent::inference::tool_metadata_for_name(name).mutates_workspace
}

#[allow(dead_code)]
pub(crate) fn is_path_safe(path: &str) -> bool {
    crate::agent::permission_enforcer::is_path_safe(path)
}

pub(crate) fn normalize_workspace_path(path: &str) -> String {
    let root = crate::tools::file_ops::workspace_root();
    let candidate = crate::tools::file_ops::resolve_candidate(path);
    let joined = if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    };
    joined.to_string_lossy().replace('\\', "/").to_lowercase()
}

pub(crate) fn is_sovereign_path_request(path: &str) -> bool {
    // Check for tokens that resolve to system directories outside the workspace.
    let upper = path.to_uppercase();
    upper.contains("@DESKTOP")
        || upper.contains("@DOWNLOADS")
        || upper.contains("@DOCUMENTS")
        || upper.contains("@PICTURES")
        || upper.contains("@IMAGES")
        || upper.contains("@VIDEOS")
        || upper.contains("@MUSIC")
        || upper.contains("@HOME")
        || upper.contains("@TEMP")
        || upper.contains("@TMP")
        || path.starts_with('~')
        || path.starts_with("/") // Absolute Linux paths are often sovereign or handled by sanitizer
}

fn prompt_explicitly_targets_docs(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    lower.contains("readme")
        || lower.contains("claude.md")
        || lower.contains("docs/")
        || lower.contains("documentation")
        || lower.contains("contributing.md")
}

pub(crate) fn is_docs_like_path(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_lowercase();

    // Internal agent metadata is NOT human documentation.
    if lower.contains("/.hematite/") || lower.contains(".hematite/") {
        return false;
    }

    lower.ends_with(".md")
        || lower.ends_with(".mdx")
        || lower.contains("/docs/")
        || lower.ends_with("/claude")
}

/// Block docs edits for any task unless the user explicitly asked for docs.
pub(crate) fn docs_edit_without_explicit_request(prompt: &str, normalized_target: &str) -> bool {
    is_docs_like_path(normalized_target) && !prompt_explicitly_targets_docs(prompt)
}

pub(crate) fn tool_path_argument(name: &str, args: &Value) -> Option<String> {
    match name {
        "read_file"
        | "inspect_lines"
        | "list_files"
        | "grep_files"
        | "lsp_get_diagnostics"
        | "lsp_hover"
        | "lsp_definitions"
        | "lsp_references"
        | "write_file"
        | "edit_file"
        | "patch_hunk"
        | "multi_search_replace" => args
            .get("path")
            .and_then(|v| v.as_str())
            .map(normalize_workspace_path),
        _ if is_mcp_mutating_tool(name) => args
            .get("path")
            .or_else(|| args.get("target"))
            .or_else(|| args.get("target_path"))
            .or_else(|| args.get("destination"))
            .or_else(|| args.get("destination_path"))
            .or_else(|| args.get("source"))
            .or_else(|| args.get("source_path"))
            .or_else(|| args.get("from"))
            .and_then(|v| v.as_str())
            .map(normalize_workspace_path),
        _ => None,
    }
}

pub(crate) fn is_mcp_mutating_tool(name: &str) -> bool {
    let metadata = crate::agent::inference::tool_metadata_for_name(name);
    metadata.external_surface && metadata.mutates_workspace
}

pub(crate) fn is_mcp_workspace_read_tool(name: &str) -> bool {
    let metadata = crate::agent::inference::tool_metadata_for_name(name);
    metadata.external_surface
        && !metadata.mutates_workspace
        && name.starts_with("mcp__filesystem__")
}

pub(crate) fn action_target_path(name: &str, args: &Value) -> Option<String> {
    match name {
        "read_file"
        | "inspect_lines"
        | "write_file"
        | "edit_file"
        | "patch_hunk"
        | "multi_search_replace"
        | "lsp_get_diagnostics"
        | "lsp_hover"
        | "lsp_definitions"
        | "lsp_references" => args
            .get("path")
            .and_then(|v| v.as_str())
            .map(normalize_workspace_path),
        _ if is_mcp_mutating_tool(name) => tool_path_argument(name, args),
        _ => None,
    }
}

#[allow(dead_code)]
pub(crate) fn requires_approval(
    name: &str,
    args: &Value,
    config: &crate::agent::config::HematiteConfig,
) -> bool {
    use crate::agent::config::{permission_for_shell, PermissionDecision};
    use crate::tools::RiskLevel;

    if name.starts_with("mcp__") {
        return true;
    }

    if name == "write_file" || name == "edit_file" {
        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
            if is_path_safe(path) {
                return false;
            }
        }
    }

    if name == "shell" {
        let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");

        match permission_for_shell(cmd, config) {
            PermissionDecision::Allow => return false,
            PermissionDecision::Deny | PermissionDecision::Ask => return true,
            PermissionDecision::UseRiskClassifier => {}
        }

        if crate::tools::guard::bash_is_safe(cmd).is_err() {
            return true;
        }

        return match crate::tools::guard::classify_bash_risk(cmd) {
            RiskLevel::High => true,
            RiskLevel::Moderate => true,
            RiskLevel::Safe => false,
        };
    }

    false
}

pub(crate) fn find_binary_in_path(name: &str) -> bool {
    let binary = name.split_whitespace().next().unwrap_or(name);
    which::which(binary).is_ok()
}

pub(crate) fn is_redundant_action(
    name: &str,
    args: &Value,
    history: &[ChatMessage],
) -> Option<String> {
    // 1. Double-Read Guard: Block reading a file immediately after writing it if no context was used.
    if name == "read_file" {
        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
            let normalized = normalize_workspace_path(path);
            if let Some(last_assistant) = history.iter().rev().find(|m| m.role == "assistant") {
                if let Some(calls) = &last_assistant.tool_calls {
                    if calls.iter().any(|c| {
                        (c.function.name == "write_file" || c.function.name == "edit_file")
                            && c.function
                                .arguments
                                .get("path")
                                .and_then(|v| v.as_str())
                                .map(normalize_workspace_path)
                                == Some(normalized.clone())
                    }) {
                        return Some(format!(
                            "STRICT: You just wrote to `{}` in your previous step. \
                             Do not read it again immediately. Assume your changes are present. \
                             Proceed with verification or the next file.",
                            path
                        ));
                    }
                }
            }
        }
    }

    // 2. Grep Persistence: If a search failed once this turn, don't repeat it with identical args.
    if name == "grep_files" || name == "grep_search" {
        if let Some(query) = args.get("query").and_then(|v| v.as_str()) {
            for m in history.iter().rev() {
                if m.role == "tool" && m.content.as_str().contains("0 matches found") {
                    // Check if this result belongs to a previous identical grep call
                    if let Some(_prev_assistant) = history.iter().rev().find(|prev| {
                        prev.role == "assistant"
                            && prev.tool_calls.as_ref().map_or(false, |calls| {
                                calls.iter().any(|c| {
                                    c.id == m.tool_call_id.clone().unwrap_or_default()
                                        && (c.function.name == "grep_files"
                                            || c.function.name == "grep_search")
                                        && c.function
                                            .arguments
                                            .get("query")
                                            .and_then(|v| v.as_str())
                                            == Some(query)
                                })
                            })
                    }) {
                        return Some(format!(
                            "STOP. You already searched for `{}` and got 0 matches. \
                             Do not repeat the same search. Try a broader term, \
                             check your spelling, or explore the directory structure instead.",
                            query
                        ));
                    }
                }
            }
        }
    }

    None
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ToolchainHeartbeat {
    pub node: Option<String>,
    pub npm: Option<String>,
    pub cargo: Option<String>,
    pub rustc: Option<String>,
}

impl ToolchainHeartbeat {
    pub fn capture() -> Self {
        fn get_version(cmd: &str, args: &[&str]) -> Option<String> {
            std::process::Command::new(cmd)
                .args(args)
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                    } else {
                        None
                    }
                })
        }

        Self {
            node: get_version("node", &["--version"]),
            npm: get_version("npm", &["--version"]),
            cargo: get_version("cargo", &["--version"]),
            rustc: get_version("rustc", &["--version"]),
        }
    }

    pub fn to_summary(&self) -> String {
        let mut lines = Vec::new();
        if let Some(v) = &self.node {
            lines.push(format!("Node: {}", v));
        }
        if let Some(v) = &self.npm {
            lines.push(format!("NPM: {}", v));
        }
        if let Some(v) = &self.cargo {
            lines.push(format!("Cargo: {}", v));
        }
        if let Some(v) = &self.rustc {
            lines.push(format!("Rustc: {}", v));
        }

        if lines.is_empty() {
            "No standard toolchains detected in PATH.".to_string()
        } else {
            format!(
                "[Authoritative Environment Heartbeat]\n{}",
                lines.join("\n")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_mutation_helper_uses_registry_metadata() {
        assert!(is_mcp_mutating_tool("mcp__filesystem__write_file"));
        assert!(is_mcp_mutating_tool("mcp__custom__rename_record"));
        assert!(!is_mcp_mutating_tool("read_file"));
        assert!(!is_mcp_mutating_tool("mcp__filesystem__read_file"));
    }

    #[test]
    fn mcp_workspace_read_helper_stays_filesystem_scoped_and_non_mutating() {
        assert!(is_mcp_workspace_read_tool("mcp__filesystem__read_file"));
        assert!(is_mcp_workspace_read_tool(
            "mcp__filesystem__list_directory"
        ));
        assert!(!is_mcp_workspace_read_tool("mcp__filesystem__write_file"));
        assert!(!is_mcp_workspace_read_tool("mcp__custom__read_record"));
        assert!(!is_mcp_workspace_read_tool("grep_files"));
    }

    #[test]
    fn tool_path_argument_handles_read_and_write_tools() {
        let read = serde_json::json!({ "path": "src/ui/tui.rs" });
        let edit = serde_json::json!({ "path": "src/ui/tui.rs" });
        let expected = normalize_workspace_path("src/ui/tui.rs");
        assert_eq!(
            tool_path_argument("read_file", &read),
            Some(expected.clone())
        );
        assert_eq!(tool_path_argument("edit_file", &edit), Some(expected));
    }

    #[test]
    fn normalize_handles_sovereign_tokens() {
        let normalized = normalize_workspace_path("@HOME/test");
        let home = dirs::home_dir().unwrap();
        let expected = home
            .join("test")
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase();
        assert_eq!(normalized, expected);
    }
}
