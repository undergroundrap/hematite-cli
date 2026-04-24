use serde_json::Value;
use std::process::Command;

fn gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_gh(args: &[&str]) -> Result<String, String> {
    if !gh_available() {
        return Err(
            "GitHub CLI (`gh`) is not installed or not on PATH. \
             Install it from https://cli.github.com/ and run `gh auth login`."
                .to_string(),
        );
    }
    let out = Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| format!("gh exec failed: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if out.status.success() {
        Ok(stdout)
    } else if !stderr.is_empty() {
        Err(stderr)
    } else {
        Err(format!("gh exited with status {}", out.status))
    }
}

fn current_branch() -> String {
    Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "HEAD".to_string())
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or("Missing required argument: 'action'")?;

    match action {
        // ── Pull Requests ───────────────────────────────────────────────────
        "pr_list" => {
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(10)
                .to_string();
            run_gh(&[
                "pr",
                "list",
                "--limit",
                &limit,
                "--json",
                "number,title,state,author,headRefName,createdAt",
                "--template",
                "{{range .}}#{{.number}}\t{{.state}}\t{{.headRefName}}\t{{.title}}\n{{end}}",
            ])
        }

        "pr_view" => {
            let pr = args.get("pr").and_then(|v| v.as_str()).unwrap_or("");
            if pr.is_empty() {
                // default: current branch's PR
                run_gh(&["pr", "view", "--json", "number,title,state,body,reviews,url"])
            } else {
                run_gh(&[
                    "pr",
                    "view",
                    pr,
                    "--json",
                    "number,title,state,body,reviews,url",
                ])
            }
        }

        "pr_create" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'title' for pr_create")?;
            let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let base = args.get("base").and_then(|v| v.as_str()).unwrap_or("main");
            let draft = args
                .get("draft")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let mut gh_args = vec!["pr", "create", "--title", title, "--body", body, "--base", base];
            if draft {
                gh_args.push("--draft");
            }
            run_gh(&gh_args)
        }

        "pr_status" => run_gh(&["pr", "status"]),

        "pr_checks" => {
            let pr = args.get("pr").and_then(|v| v.as_str()).unwrap_or("");
            if pr.is_empty() {
                run_gh(&["pr", "checks"])
            } else {
                run_gh(&["pr", "checks", pr])
            }
        }

        "pr_merge" => {
            let pr = args.get("pr").and_then(|v| v.as_str()).unwrap_or("");
            let strategy = args.get("strategy").and_then(|v| v.as_str()).unwrap_or("merge");
            let flag = match strategy {
                "squash" => "--squash",
                "rebase" => "--rebase",
                _ => "--merge",
            };
            if pr.is_empty() {
                run_gh(&["pr", "merge", flag])
            } else {
                run_gh(&["pr", "merge", pr, flag])
            }
        }

        // ── Issues ──────────────────────────────────────────────────────────
        "issue_list" => {
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(10)
                .to_string();
            let state = args.get("state").and_then(|v| v.as_str()).unwrap_or("open");
            run_gh(&[
                "issue",
                "list",
                "--limit",
                &limit,
                "--state",
                state,
                "--json",
                "number,title,state,labels,createdAt",
                "--template",
                "{{range .}}#{{.number}}\t{{.state}}\t{{.title}}\n{{end}}",
            ])
        }

        "issue_view" => {
            let number = args
                .get("number")
                .and_then(|v| v.as_u64())
                .map(|n| n.to_string())
                .or_else(|| args.get("number").and_then(|v| v.as_str()).map(str::to_string))
                .ok_or("Missing 'number' for issue_view")?;
            run_gh(&["issue", "view", &number])
        }

        "issue_create" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'title' for issue_create")?;
            let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
            run_gh(&["issue", "create", "--title", title, "--body", body])
        }

        // ── CI / Actions ────────────────────────────────────────────────────
        "ci_status" => {
            let branch = args
                .get("branch")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(current_branch);
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(5)
                .to_string();
            run_gh(&[
                "run",
                "list",
                "--branch",
                &branch,
                "--limit",
                &limit,
                "--json",
                "status,conclusion,name,headBranch,createdAt,url",
                "--template",
                "{{range .}}{{.name}}\t{{.status}}\t{{.conclusion}}\t{{.headBranch}}\n{{end}}",
            ])
        }

        "run_view" => {
            let run_id = args
                .get("run_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'run_id' for run_view")?;
            run_gh(&["run", "view", run_id])
        }

        // ── Repo ─────────────────────────────────────────────────────────────
        "repo_view" => run_gh(&["repo", "view"]),

        "release_list" => {
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(5)
                .to_string();
            run_gh(&["release", "list", "--limit", &limit])
        }

        other => Err(format!(
            "Unknown github_ops action: '{}'. Valid actions: \
             pr_list, pr_view, pr_create, pr_status, pr_checks, pr_merge, \
             issue_list, issue_view, issue_create, \
             ci_status, run_view, repo_view, release_list",
            other
        )),
    }
}

/// Harness-side PR creation: gathers context, calls gh, returns formatted result.
/// Used by the `/pr` slash command without model involvement.
pub fn create_pr_from_context(title: Option<&str>, draft: bool) -> Result<String, String> {
    if !gh_available() {
        return Err(
            "`gh` not installed. Install from https://cli.github.com/ and run `gh auth login`."
                .to_string(),
        );
    }

    let branch = current_branch();
    if branch.is_empty() || branch == "HEAD" {
        return Err("Not on a named branch. Check out a branch first.".to_string());
    }

    // Build title from last commit if not supplied
    let auto_title = if title.is_none() {
        Command::new("git")
            .args(["log", "-1", "--format=%s"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    };
    let pr_title = title
        .map(str::to_string)
        .or(auto_title)
        .unwrap_or_else(|| branch.replace('-', " ").replace('_', " "));

    // Gather commit log for body
    let commits = Command::new("git")
        .args(["log", "main..HEAD", "--oneline"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let body = if commits.is_empty() {
        String::new()
    } else {
        format!("## Commits\n\n```\n{}\n```", commits)
    };

    let mut gh_args = vec![
        "pr", "create",
        "--title", &pr_title,
        "--body", &body,
        "--base", "main",
    ];
    if draft {
        gh_args.push("--draft");
    }

    let out = Command::new("gh")
        .args(&gh_args)
        .output()
        .map_err(|e| format!("gh exec failed: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if out.status.success() {
        Ok(format!("PR created: {}", stdout))
    } else {
        Err(if !stderr.is_empty() { stderr } else { stdout })
    }
}

/// Quick CI status for the current branch — used by `/ci` slash command.
pub fn ci_status_current() -> Result<String, String> {
    let branch = current_branch();
    run_gh(&[
        "run",
        "list",
        "--branch",
        &branch,
        "--limit",
        "5",
        "--json",
        "status,conclusion,name,headBranch,createdAt",
        "--template",
        "{{range .}}{{.name}}\t{{.status}}\t{{.conclusion}}\n{{end}}",
    ])
}
