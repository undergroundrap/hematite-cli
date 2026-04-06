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
    let candidate = std::path::Path::new(path);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    joined
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase()
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
        "write_file" | "edit_file" | "patch_hunk" | "multi_search_replace" => args
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
        assert!(is_mcp_workspace_read_tool("mcp__filesystem__list_directory"));
        assert!(!is_mcp_workspace_read_tool("mcp__filesystem__write_file"));
        assert!(!is_mcp_workspace_read_tool("mcp__custom__read_record"));
        assert!(!is_mcp_workspace_read_tool("grep_files"));
    }
}
