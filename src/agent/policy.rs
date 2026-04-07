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

pub(crate) fn looks_like_startup_ui_copy_request(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let startup_signal = lower.contains("startup banner")
        || lower.contains("startup wording")
        || lower.contains("startup copy")
        || lower.contains("first-time users")
        || lower.contains("initial screen")
        || lower.contains("splash screen")
        || lower.contains("welcome screen")
        || lower.contains("greeting copy")
        || lower.contains("landing copy");
    let ui_signal = lower.contains("startup")
        || lower.contains("banner")
        || lower.contains("tone")
        || lower.contains("first-time");
    startup_signal && ui_signal
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
    lower.ends_with(".md")
        || lower.ends_with(".mdx")
        || lower.contains("/docs/")
        || lower.ends_with("/claude")
}

#[cfg(test)]
pub(crate) fn docs_target_conflicts_with_startup_ui_request(
    prompt: &str,
    normalized_target: &str,
) -> bool {
    looks_like_startup_ui_copy_request(prompt)
        && is_docs_like_path(normalized_target)
        && !prompt_explicitly_targets_docs(prompt)
}

/// Block docs edits for any task unless the user explicitly asked for docs.
pub(crate) fn docs_edit_without_explicit_request(
    prompt: &str,
    normalized_target: &str,
) -> bool {
    is_docs_like_path(normalized_target) && !prompt_explicitly_targets_docs(prompt)
}

fn prompt_explicitly_targets_path(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    lower.contains("src/")
        || lower.contains("ui/")
        || lower.contains("frontend/")
        || lower.contains("app/")
        || lower.contains(".rs")
        || lower.contains(".tsx")
        || lower.contains(".jsx")
        || lower.contains(".vue")
        || lower.contains(".svelte")
        || lower.contains(".html")
        || lower.contains(".css")
        || lower.contains("main.rs")
}

fn startup_ui_path_score(normalized_target: &str) -> i32 {
    let lower = normalized_target.replace('\\', "/").to_lowercase();
    if is_docs_like_path(&lower)
        || lower.contains("/target/")
        || lower.contains("/node_modules/")
        || lower.contains("/.git/")
        || lower.contains("/tests/")
        || lower.contains("/test/")
    {
        return 0;
    }

    let mut score = 0;
    if lower.contains("/src/ui/")
        || lower.contains("/ui/")
        || lower.contains("/frontend/")
        || lower.contains("/app/")
        || lower.contains("/pages/")
        || lower.contains("/components/")
    {
        score += 6;
    }

    if lower.contains("splash")
        || lower.contains("welcome")
        || lower.contains("banner")
        || lower.contains("landing")
        || lower.contains("intro")
        || lower.contains("home")
    {
        score += 4;
    }

    if lower.ends_with(".tsx")
        || lower.ends_with(".jsx")
        || lower.ends_with(".vue")
        || lower.ends_with(".svelte")
        || lower.ends_with(".html")
        || lower.ends_with(".css")
        || lower.ends_with(".scss")
    {
        score += 3;
    }

    if lower.ends_with("/app.rs")
        || lower.ends_with("/main.rs")
        || lower.ends_with("/lib.rs")
        || lower.ends_with("/mod.rs")
    {
        score -= 2;
    }

    score.max(0)
}

fn collect_repo_files(
    dir: &std::path::Path,
    files: &mut Vec<String>,
    depth: usize,
    budget: &mut usize,
) {
    if depth > 6 || *budget == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        if *budget == 0 {
            break;
        }
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy().to_lowercase();
        if name == ".git"
            || name == "target"
            || name == "node_modules"
            || name == ".hematite"
            || name == ".hematite_logs"
        {
            continue;
        }
        if path.is_dir() {
            collect_repo_files(&path, files, depth + 1, budget);
        } else if path.is_file() {
            files.push(normalize_workspace_path(&path.to_string_lossy()));
            *budget = budget.saturating_sub(1);
        }
    }
}

pub(crate) fn startup_ui_owner_paths() -> Vec<String> {
    let root = crate::tools::file_ops::workspace_root();
    let mut files = Vec::new();
    let mut budget = 512usize;
    collect_repo_files(&root, &mut files, 0, &mut budget);

    let mut scored = files
        .into_iter()
        .map(|path| {
            let score = startup_ui_path_score(&path);
            (score, path)
        })
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();

    scored.sort_by(|(score_a, path_a), (score_b, path_b)| {
        score_b
            .cmp(score_a)
            .then_with(|| path_a.len().cmp(&path_b.len()))
            .then_with(|| path_a.cmp(path_b))
    });
    scored.dedup_by(|(_, a), (_, b)| a == b);
    scored.into_iter().take(4).map(|(_, path)| path).collect()
}

pub(crate) fn is_startup_ui_owner_path(normalized_target: &str) -> bool {
    startup_ui_path_score(normalized_target) > 0
}

pub(crate) fn startup_ui_target_conflicts_with_owner_discovery(
    prompt: &str,
    normalized_target: &str,
) -> bool {
    looks_like_startup_ui_copy_request(prompt)
        && !prompt_explicitly_targets_path(prompt)
        && !is_startup_ui_owner_path(normalized_target)
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
        "write_file" | "edit_file" | "patch_hunk" | "multi_search_replace" => args
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

    #[test]
    fn startup_ui_request_rejects_docs_targets_by_default() {
        assert!(docs_target_conflicts_with_startup_ui_request(
            "Update the startup banner wording so it reads more clearly for first-time users, keep the existing visual tone, and verify that the build still passes.",
            "c:/repo/claude.md"
        ));
        assert!(docs_target_conflicts_with_startup_ui_request(
            "Update the startup banner wording so it reads more clearly for first-time users.",
            "c:/repo/readme.md"
        ));
    }

    #[test]
    fn startup_ui_request_allows_docs_when_user_explicitly_asks_for_docs() {
        assert!(!docs_target_conflicts_with_startup_ui_request(
            "Update the startup banner wording in README.md so first-time users understand it better.",
            "c:/repo/readme.md"
        ));
    }

    #[test]
    fn startup_ui_request_blocks_non_owner_runtime_targets() {
        assert!(startup_ui_target_conflicts_with_owner_discovery(
            "Update the startup banner wording so it reads more clearly for first-time users, keep the existing visual tone, and verify that the build still passes.",
            "c:/repo/src/main.rs"
        ));
        assert!(!startup_ui_target_conflicts_with_owner_discovery(
            "Update the startup banner wording so it reads more clearly for first-time users, keep the existing visual tone, and verify that the build still passes.",
            "c:/repo/src/ui/tui.rs"
        ));
        assert!(!startup_ui_target_conflicts_with_owner_discovery(
            "Update the startup banner wording so it reads more clearly for first-time users, keep the existing visual tone, and verify that the build still passes.",
            "c:/repo/frontend/src/App.tsx"
        ));
        assert!(!startup_ui_target_conflicts_with_owner_discovery(
            "Update the startup banner wording in src/main.rs.",
            "c:/repo/src/main.rs"
        ));
    }

    #[test]
    fn startup_ui_scoring_stays_repo_shape_based() {
        assert!(startup_ui_path_score("c:/repo/src/ui/tui.rs") > 0);
        assert!(startup_ui_path_score("c:/repo/frontend/src/App.tsx") > 0);
        assert!(startup_ui_path_score("c:/repo/src/pages/home.vue") > 0);
        assert_eq!(startup_ui_path_score("c:/repo/src/main.rs"), 0);
        assert_eq!(startup_ui_path_score("c:/repo/README.md"), 0);
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
        assert_eq!(
            tool_path_argument("edit_file", &edit),
            Some(expected)
        );
    }
}
