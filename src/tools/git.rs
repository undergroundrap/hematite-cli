use serde_json::Value;
use std::process::Command;
use std::path::Path;
use crate::agent::git::is_git_repo;

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
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("list");
    let repo_path = Path::new(".");
    if !is_git_repo(repo_path) {
        return Err("Current directory is not a Git repository".to_string());
    }

    match action {
        "list" => {
            let output = Command::new("git").arg("remote").arg("-v").output()
                .map_err(|e| format!("Failed to list remotes: {e}"))?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        "add" => {
            let name = args.get("name").and_then(|v| v.as_str()).ok_or("Missing name for add")?;
            let url = args.get("url").and_then(|v| v.as_str()).ok_or("Missing url for add")?;
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
            let name = args.get("name").and_then(|v| v.as_str()).ok_or("Missing name for remove")?;
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
        _ => Err(format!("Unknown action: {}", action))
    }
}

/// tool: git_worktree
///
/// Manage Git worktrees — isolated working directories on separate branches.
/// Use this to do risky or experimental work without touching the main branch.
pub async fn execute_worktree(args: &Value) -> Result<String, String> {
    let action = args.get("action").and_then(|v| v.as_str())
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
            let out = String::from_utf8_lossy(&output.stdout).to_string();
            if out.trim().is_empty() {
                Ok("No worktrees (only main working tree)".to_string())
            } else {
                Ok(out)
            }
        }

        "add" => {
            let path = args.get("path").and_then(|v| v.as_str())
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
            let branch_exists = !String::from_utf8_lossy(&branch_check.stdout).trim().is_empty();

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
            let path = args.get("path").and_then(|v| v.as_str())
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
            let out = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(if out.trim().is_empty() { "Nothing to prune.".to_string() } else { out })
        }

        _ => Err(format!("Unknown worktree action '{action}'. Use: list | add | remove | prune"))
    }
}
