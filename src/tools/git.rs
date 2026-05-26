use crate::agent::git::is_git_repo;
use serde_json::Value;
use std::path::Path;
use std::process::Command;

/// tool: git_commit
///
/// Action: Stage all changes (git add -A) and commit them using the 'Conventional Commits' style.
pub async fn execute(args: &Value) -> Result<String, String> {
    let message = args
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: 'message'".to_string())?;

    let repo_path = Path::new(".");
    if !is_git_repo(repo_path) {
        return Err("Current directory is not a Git repository".to_string());
    }

    // 1. Stage all changes
    let add_status = std::process::Command::new("git")
        .arg("add")
        .arg("-A")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("Failed to run git add: {e}"))?;

    if !add_status.success() {
        return Err("Git 'add' failed".to_string());
    }

    // 2. Commit
    let commit_status = std::process::Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(message)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("Failed to run git commit: {e}"))?;

    if commit_status.success() {
        Ok(format!("Successfully committed changes: '{message}'"))
    } else {
        Err("Git 'commit' failed (maybe nothing to commit or malformed message?)".to_string())
    }
}

/// tool: git_push
pub async fn execute_push(_args: &Value) -> Result<String, String> {
    let repo_path = Path::new(".");
    if !is_git_repo(repo_path) {
        return Err("Current directory is not a Git repository".to_string());
    }

    let output = Command::new("git")
        .args(["push", "origin", "HEAD"])
        .output()
        .map_err(|e| format!("Failed to execution git push: {e}"))?;

    if output.status.success() {
        Ok("Changes successfully pushed to remote origin.".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Git push failed: {}", stderr))
    }
}

/// tool: git_remote
pub async fn execute_remote(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("list");
    let repo_path = Path::new(".");
    if !is_git_repo(repo_path) {
        return Err("Current directory is not a Git repository".to_string());
    }

    match action {
        "list" => {
            let output = Command::new("git")
                .arg("remote")
                .arg("-v")
                .output()
                .map_err(|e| format!("Failed to list remotes: {e}"))?;
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        "add" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or("Missing name for add")?;
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("Missing url for add")?;
            let status = std::process::Command::new("git")
                .args(["remote", "add", name, url])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map_err(|e| format!("Failed to add remote: {e}"))?;
            if status.success() {
                Ok(format!("Successfully added remote '{}' -> {}", name, url))
            } else {
                Err("Failed to add remote (it might already exist)".to_string())
            }
        }
        "remove" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or("Missing name for remove")?;
            let status = std::process::Command::new("git")
                .args(["remote", "remove", name])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map_err(|e| format!("Failed to remove remote: {e}"))?;
            if status.success() {
                Ok(format!("Successfully removed remote '{}'", name))
            } else {
                Err("Failed to remove remote".to_string())
            }
        }
        _ => Err(format!("Unknown action: {}", action)),
    }
}

/// tool: git_worktree
///
/// Manage Git worktrees — isolated working directories on separate branches.
/// Use this to do risky or experimental work without touching the main branch.
pub async fn execute_worktree(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: 'action' (list|add|remove|prune)".to_string())?;

    let repo_path = Path::new(".");
    if !is_git_repo(repo_path) {
        return Err("Current directory is not a Git repository".to_string());
    }

    match action {
        "list" => {
            let output = Command::new("git")
                .args(["worktree", "list"])
                .output()
                .map_err(|e| format!("Failed to list worktrees: {e}"))?;
            let out = String::from_utf8_lossy(&output.stdout).into_owned();
            if out.trim().is_empty() {
                Ok("No worktrees (only main working tree)".to_string())
            } else {
                Ok(out)
            }
        }

        "add" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'path' for worktree add".to_string())?;

            // Derive branch name from path basename if not explicitly provided.
            let branch_arg = args.get("branch").and_then(|v| v.as_str());
            let branch = branch_arg.unwrap_or_else(|| {
                std::path::Path::new(path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(path)
            });

            // Check if the branch already exists.
            let branch_check = Command::new("git")
                .args(["branch", "--list", branch])
                .output()
                .map_err(|e| format!("Failed to check branch: {e}"))?;
            let branch_exists = !String::from_utf8_lossy(&branch_check.stdout)
                .trim()
                .is_empty();

            let output = if branch_exists {
                // Check out existing branch.
                Command::new("git")
                    .args(["worktree", "add", path, branch])
                    .output()
                    .map_err(|e| format!("Failed to add worktree: {e}"))?
            } else {
                // Create new branch.
                Command::new("git")
                    .args(["worktree", "add", path, "-b", branch])
                    .output()
                    .map_err(|e| format!("Failed to add worktree: {e}"))?
            };

            if output.status.success() {
                Ok(format!(
                    "Worktree created at '{path}' on branch '{branch}'.\n\
                     Work there independently, then commit and merge back when ready."
                ))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Failed to create worktree: {}", stderr.trim()))
            }
        }

        "remove" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'path' for worktree remove".to_string())?;

            let output = Command::new("git")
                .args(["worktree", "remove", path])
                .output()
                .map_err(|e| format!("Failed to remove worktree: {e}"))?;

            if output.status.success() {
                Ok(format!("Worktree '{path}' removed."))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // If it has uncommitted changes, suggest --force.
                if stderr.contains("contains modified or untracked files") {
                    Err(format!(
                        "Worktree '{path}' has uncommitted changes. \
                         Commit or stash them first, or use action=remove with force=true."
                    ))
                } else {
                    Err(format!("Failed to remove worktree: {}", stderr.trim()))
                }
            }
        }

        "prune" => {
            let output = Command::new("git")
                .args(["worktree", "prune", "-v"])
                .output()
                .map_err(|e| format!("Failed to prune worktrees: {e}"))?;
            let out = String::from_utf8_lossy(&output.stdout).into_owned();
            Ok(if out.trim().is_empty() {
                "Nothing to prune.".to_string()
            } else {
                out
            })
        }

        _ => Err(format!(
            "Unknown worktree action '{action}'. Use: list | add | remove | prune"
        )),
    }
}

/// tool: git_status
///
/// Returns a structured snapshot of the working tree: branch, ahead/behind remote,
/// staged files, modified files, and untracked files — all in one call.
pub async fn execute_status(_args: &Value) -> Result<String, String> {
    let repo_path = Path::new(".");
    if !is_git_repo(repo_path) {
        return Err("Current directory is not a Git repository".to_string());
    }

    // Branch + ahead/behind via --porcelain=v2 --branch
    let branch_out = Command::new("git")
        .args(["status", "--porcelain=v2", "--branch"])
        .output()
        .map_err(|e| format!("git_status: failed: {e}"))?;

    let raw = String::from_utf8_lossy(&branch_out.stdout).into_owned();

    // Parse porcelain v2 output.
    let mut branch = String::from("(unknown)");
    let mut upstream: Option<String> = None;
    let mut ahead: u32 = 0;
    let mut behind: u32 = 0;
    let mut staged: Vec<String> = Vec::new();
    let mut modified: Vec<String> = Vec::new();
    let mut untracked: Vec<String> = Vec::new();

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            branch = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            upstream = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            // format: +N -M
            for part in rest.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    behind = n.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("1 ") || line.starts_with("2 ") {
            // Tracked changed file: "1 XY N... file" or "2 XY ... file\torigfile"
            let parts: Vec<&str> = line.splitn(9, ' ').collect();
            if parts.len() < 9 {
                continue;
            }
            let xy = parts[1];
            let path = parts[8].split('\t').next().unwrap_or(parts[8]).trim();
            let x = xy.chars().next().unwrap_or('.');
            let y = xy.chars().nth(1).unwrap_or('.');
            if x != '.' && x != '?' {
                staged.push(path.to_string());
            }
            if y != '.' && y != '?' {
                modified.push(path.to_string());
            }
        } else if line.starts_with("? ") {
            // Untracked: "? path"
            let path = line[2..].trim();
            untracked.push(path.to_string());
        }
    }

    let mut out = String::new();
    out.push_str(&format!("branch  : {branch}\n"));

    if let Some(ref up) = upstream {
        out.push_str(&format!("upstream: {up}"));
        if ahead > 0 || behind > 0 {
            out.push_str(&format!(" (+{ahead} ahead, -{behind} behind)"));
        } else {
            out.push_str(" (up to date)");
        }
        out.push('\n');
    } else {
        out.push_str("upstream: (no tracking branch)\n");
    }

    if staged.is_empty() && modified.is_empty() && untracked.is_empty() {
        out.push_str("status  : clean\n");
        return Ok(out);
    }

    if !staged.is_empty() {
        out.push_str(&format!("\nstaged ({}):\n", staged.len()));
        for f in &staged {
            out.push_str(&format!("  + {f}\n"));
        }
    }
    if !modified.is_empty() {
        out.push_str(&format!("\nmodified ({}):\n", modified.len()));
        for f in &modified {
            out.push_str(&format!("  ~ {f}\n"));
        }
    }
    if !untracked.is_empty() {
        let shown = untracked.len().min(20);
        out.push_str(&format!("\nuntracked ({}):\n", untracked.len()));
        for f in untracked.iter().take(shown) {
            out.push_str(&format!("  ? {f}\n"));
        }
        if untracked.len() > shown {
            out.push_str(&format!(
                "  ... {} more untracked files\n",
                untracked.len() - shown
            ));
        }
    }

    Ok(out.trim_end().to_string())
}

/// tool: git_diff
///
/// Returns a structured diff of working-tree changes, staged changes, or between two refs.
/// Supports a compact stat summary (files changed + lines) or full unified diff with size cap.
pub async fn execute_diff(args: &Value) -> Result<String, String> {
    let repo_path = Path::new(".");
    if !is_git_repo(repo_path) {
        return Err("Current directory is not a Git repository".to_string());
    }

    let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("diff");

    let from = args.get("from").and_then(|v| v.as_str());
    let to = args.get("to").and_then(|v| v.as_str());
    let path_filter = args.get("path").and_then(|v| v.as_str());
    let staged = args
        .get("staged")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Build the git diff argument list.
    let mut git_args: Vec<&str> = vec!["diff"];

    if staged {
        git_args.push("--cached");
    }

    if mode == "stat" {
        git_args.push("--stat");
        git_args.push("--stat-width=100");
    }

    // Ref range: "from..to", or just "from" (compare from..HEAD), or nothing (working tree).
    let range_str;
    match (from, to) {
        (Some(f), Some(t)) => {
            range_str = format!("{f}..{t}");
            git_args.push(&range_str);
        }
        (Some(f), None) => {
            git_args.push(f);
        }
        _ => {}
    }

    if let Some(p) = path_filter {
        git_args.push("--");
        git_args.push(p);
    }

    let output = Command::new("git")
        .args(&git_args)
        .output()
        .map_err(|e| format!("git_diff: failed to run git: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(format!("git diff failed: {}", stderr.trim()));
    }

    if stdout.trim().is_empty() {
        return Ok(
            "No differences found (working tree is clean or refs are identical).".to_string(),
        );
    }

    // For full diff mode, cap at 12 KB and add a stat header so the model gets
    // a quick overview before the raw patch.
    if mode != "stat" {
        // Always prepend a --stat summary so the model sees the file list cheaply.
        let stat_range: Option<String> = from.zip(to).map(|(f, t)| format!("{f}..{t}"));
        let mut stat_args: Vec<&str> = vec!["diff", "--stat", "--stat-width=100"];
        if staged {
            stat_args.push("--cached");
        }
        if let Some(rs) = &stat_range {
            stat_args.push(rs.as_str());
        } else if let Some(f) = from {
            stat_args.push(f);
        }
        if let Some(p) = path_filter {
            stat_args.push("--");
            stat_args.push(p);
        }
        let stat_out = Command::new("git")
            .args(&stat_args)
            .output()
            .map_err(|e| format!("git_diff: stat pass failed: {e}"))?;
        let stat_text = String::from_utf8_lossy(&stat_out.stdout).into_owned();

        const CAP: usize = 12_000;
        let truncated = stdout.len() > CAP;
        let body = if truncated { &stdout[..CAP] } else { &stdout };

        let mut result = String::new();
        result.push_str("GIT DIFF STAT:\n");
        result.push_str(stat_text.trim());
        result.push_str("\n\nFULL DIFF:\n");
        result.push_str(body);
        if truncated {
            result.push_str(&format!(
                "\n\n... [{} bytes truncated — use path= to scope to a specific file]",
                stdout.len() - CAP
            ));
        }
        return Ok(result);
    }

    Ok(stdout)
}

/// tool: git_log
///
/// Returns structured commit history — hash, author, date, subject — for the current branch
/// or a specific file. Optionally scoped to a ref range.
pub async fn execute_log(args: &Value) -> Result<String, String> {
    let repo_path = Path::new(".");
    if !is_git_repo(repo_path) {
        return Err("Current directory is not a Git repository".to_string());
    }

    let n = args
        .get("n")
        .and_then(|v| v.as_u64())
        .unwrap_or(20)
        .min(200) as usize;

    let from = args.get("from").and_then(|v| v.as_str());
    let to = args.get("to").and_then(|v| v.as_str());
    let path_filter = args.get("path").and_then(|v| v.as_str());
    let oneline = args
        .get("oneline")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let format = if oneline {
        "%h %s"
    } else {
        "commit %H%nauthor  %an <%ae>%ndate    %ai%nsubject %s%n"
    };

    let n_str = format!("-{n}");
    let pretty = format!("--pretty=format:{format}");
    let mut git_args: Vec<&str> = vec!["log", &n_str, &pretty];

    let range_str;
    match (from, to) {
        (Some(f), Some(t)) => {
            range_str = format!("{f}..{t}");
            git_args.push(&range_str);
        }
        (Some(f), None) => {
            git_args.push(f);
        }
        _ => {}
    }

    if let Some(p) = path_filter {
        git_args.push("--");
        git_args.push(p);
    }

    let output = Command::new("git")
        .args(&git_args)
        .output()
        .map_err(|e| format!("git_log: failed to run git: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(format!("git log failed: {}", stderr.trim()));
    }

    if stdout.trim().is_empty() {
        return Ok("No commits found.".to_string());
    }

    Ok(stdout)
}

/// tool: changelog_gen
///
/// Generates a structured changelog from git commit history grouped by conventional commit type.
/// Supports scoping to a version range (tag..tag) or a fixed number of recent commits.
pub async fn execute_changelog(args: &Value) -> Result<String, String> {
    if !is_git_repo(Path::new(".")) {
        return Err("changelog_gen: not a git repository".to_string());
    }

    let n = args
        .get("n")
        .and_then(|v| v.as_u64())
        .unwrap_or(100)
        .min(500) as usize;
    let from = args.get("from").and_then(|v| v.as_str());
    let to = args.get("to").and_then(|v| v.as_str());
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Changelog");

    // Collect: hash\x1fsub\x1eauthor\x1edate
    let n_str = format!("-{n}");
    let mut git_args: Vec<&str> = vec!["log", "--pretty=format:%h\x1f%s\x1e%an\x1e%ai"];
    if from.is_some() || to.is_some() {
        // suppress -N when using a range
    } else {
        git_args.push(&n_str);
    }
    let range_str;
    match (from, to) {
        (Some(f), Some(t)) => {
            range_str = format!("{f}..{t}");
            git_args.push(&range_str);
        }
        (Some(f), None) => {
            git_args.push(f);
        }
        _ => {}
    }

    let output = Command::new("git")
        .args(&git_args)
        .output()
        .map_err(|e| format!("changelog_gen: git error: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("changelog_gen: git log failed: {}", err.trim()));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    if raw.trim().is_empty() {
        return Ok("changelog_gen: no commits found in the specified range.".to_string());
    }

    // Parse commits
    let mut sections: std::collections::BTreeMap<&str, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut uncategorized: Vec<String> = Vec::new();

    for entry in raw.split('\x1e').map(str::trim).filter(|s| !s.is_empty()) {
        let mut parts = entry.splitn(2, '\x1f');
        let hash = parts.next().unwrap_or("").trim();
        let rest = parts.next().unwrap_or("").trim();
        // rest = "subject\nauthor\ndate" — we only need subject
        let subject = rest.lines().next().unwrap_or("").trim();

        let (kind, scope, message) = parse_conventional_commit(subject);
        let display = if scope.is_empty() {
            format!("- {message} ({hash})")
        } else {
            format!("- **{scope}**: {message} ({hash})")
        };

        match kind {
            "feat" => sections.entry("Features").or_default().push(display),
            "fix" => sections.entry("Bug Fixes").or_default().push(display),
            "perf" => sections.entry("Performance").or_default().push(display),
            "refactor" => sections.entry("Refactoring").or_default().push(display),
            "docs" => sections.entry("Documentation").or_default().push(display),
            "test" => sections.entry("Tests").or_default().push(display),
            "chore" => sections.entry("Chores").or_default().push(display),
            "ci" => sections.entry("CI").or_default().push(display),
            "build" => sections.entry("Build").or_default().push(display),
            "style" => sections.entry("Style").or_default().push(display),
            _ => uncategorized.push(display),
        }
    }

    // Render — preferred section order
    let section_order = [
        "Features",
        "Bug Fixes",
        "Performance",
        "Refactoring",
        "Documentation",
        "Tests",
        "Build",
        "CI",
        "Style",
        "Chores",
    ];

    let mut out = format!("# {title}\n\n");
    for &name in &section_order {
        if let Some(items) = sections.get(name) {
            out.push_str(&format!("## {name}\n\n"));
            for item in items {
                out.push_str(item);
                out.push('\n');
            }
            out.push('\n');
        }
    }
    if !uncategorized.is_empty() {
        out.push_str("## Other\n\n");
        for item in &uncategorized {
            out.push_str(item);
            out.push('\n');
        }
        out.push('\n');
    }

    let total: usize = sections.values().map(|v| v.len()).sum::<usize>() + uncategorized.len();
    out.push_str(&format!("---\n_{total} commits included._"));

    Ok(out)
}

fn parse_conventional_commit<'a>(subject: &'a str) -> (&'a str, &'a str, &'a str) {
    // Matches: feat(scope): message  OR  feat: message  OR  feat!: message
    let s = subject.trim_start_matches("- ").trim();
    if let Some(colon_pos) = s.find(':') {
        let prefix = &s[..colon_pos].trim_end_matches('!');
        let message = s[colon_pos + 1..].trim();
        if let Some(paren) = prefix.find('(') {
            let kind = &prefix[..paren];
            let scope_end = prefix.rfind(')').unwrap_or(prefix.len());
            let scope = &prefix[paren + 1..scope_end];
            return (kind, scope, message);
        }
        // No scope
        return (prefix, "", message);
    }
    // Non-conventional commit
    ("other", "", s)
}
