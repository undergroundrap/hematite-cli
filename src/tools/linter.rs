use serde_json::Value;
use std::time::Duration;

const TIMEOUT: u64 = 90;
const MAX_OUTPUT: usize = 12 * 1024;

#[derive(Debug)]
struct Lint {
    file: String,
    line: u32,
    col: u32,
    level: String, // "warning" | "error"
    code: String,  // e.g. "clippy::needless_range_loop"
    message: String,
    suggestion: String, // machine-applicable fix text, if any
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let fix = args.get("fix").and_then(|v| v.as_bool()).unwrap_or(false);
    let allow_dirty = args
        .get("allow_dirty")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let filter = args.get("filter").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let workspace = args.get("workspace").and_then(|v| v.as_bool()).unwrap_or(false);

    let root = crate::tools::file_ops::workspace_root();

    if fix {
        return run_fix(&root, allow_dirty, workspace).await;
    }

    // ── Run cargo clippy --message-format=json ────────────────────────────────
    let mut cmd_args = vec![
        "clippy".to_string(),
        "--message-format=json".to_string(),
        "--".to_string(),
        "-D".to_string(),
        "warnings".to_string(), // treat warnings as errors in the output (still shows them)
    ];
    if workspace {
        cmd_args.insert(1, "--workspace".to_string());
    }
    // Undo the -D warnings we added (just for structured listing, not for failing build)
    // Actually, to get warnings without failing, we don't add -D warnings.
    cmd_args = vec!["clippy".to_string(), "--message-format=json".to_string()];
    if workspace {
        cmd_args.push("--workspace".to_string());
    }

    let result = tokio::time::timeout(
        Duration::from_secs(TIMEOUT),
        tokio::process::Command::new("cargo")
            .args(&cmd_args)
            .current_dir(&root)
            .output(),
    )
    .await;

    let output = match result {
        Err(_) => return Err(format!("lint_code: timed out after {TIMEOUT}s")),
        Ok(Err(e)) => return Err(format!("lint_code: failed to spawn cargo: {e}")),
        Ok(Ok(o)) => o,
    };

    // cargo clippy --message-format=json writes JSON to stdout (compiler messages)
    // and status info to stderr
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lints: Vec<Lint> = Vec::new();

    for line in stdout.lines() {
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
            if msg.get("reason").and_then(|v| v.as_str()) != Some("compiler-message") {
                continue;
            }
            let inner = match msg.get("message") {
                Some(m) => m,
                None => continue,
            };
            let level = inner
                .get("level")
                .and_then(|v| v.as_str())
                .unwrap_or("warning")
                .to_string();
            if level == "note" || level == "help" {
                continue;
            }
            let message_text = inner
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if message_text.is_empty() || message_text.starts_with("unused import") && level == "warning" {
                // Still show unused imports
            }

            let code = inner
                .get("code")
                .and_then(|c| c.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Apply filter
            if !filter.is_empty()
                && !code.contains(&filter)
                && !message_text.to_lowercase().contains(&filter.to_lowercase())
            {
                continue;
            }

            // Extract primary span
            let spans = inner.get("spans").and_then(|v| v.as_array());
            let primary_span = spans.and_then(|arr| {
                arr.iter()
                    .find(|s| s.get("is_primary").and_then(|v| v.as_bool()).unwrap_or(false))
            });

            let (file, line_num, col_num) = if let Some(span) = primary_span {
                let f = span
                    .get("file_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string();
                let l = span
                    .get("line_start")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let c = span
                    .get("column_start")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                (f, l, c)
            } else {
                ("?".to_string(), 0, 0)
            };

            // Extract machine-applicable suggestion from children
            let suggestion = extract_suggestion(inner);

            lints.push(Lint {
                file,
                line: line_num,
                col: col_num,
                level,
                code,
                message: message_text,
                suggestion,
            });
        }
    }

    format_result(&lints, fix)
}

async fn run_fix(root: &std::path::Path, allow_dirty: bool, workspace: bool) -> Result<String, String> {
    let mut cmd_args = vec!["clippy".to_string(), "--fix".to_string()];
    if allow_dirty {
        cmd_args.push("--allow-dirty".to_string());
    }
    if workspace {
        cmd_args.push("--workspace".to_string());
    }

    let result = tokio::time::timeout(
        Duration::from_secs(TIMEOUT),
        tokio::process::Command::new("cargo")
            .args(&cmd_args)
            .current_dir(root)
            .output(),
    )
    .await;

    match result {
        Err(_) => Err(format!("lint_code fix: timed out after {TIMEOUT}s")),
        Ok(Err(e)) => Err(format!("lint_code fix: spawn failed: {e}")),
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let combined = format!("{stderr}{stdout}").trim().to_string();

            if output.status.success() {
                let fixed = count_fixed(&combined);
                Ok(format!(
                    "lint_code [FIX APPLIED]: {fixed} automatic fix(es) applied.\n\
                     Run lint_code again to verify remaining warnings.\n\n\
                     {combined}"
                ))
            } else {
                if combined.contains("no changes") || combined.contains("0 warnings") {
                    Ok("lint_code [FIX]: no machine-applicable fixes needed.".to_string())
                } else if combined.contains("uncommitted") || combined.contains("dirty") {
                    Err(
                        "lint_code fix: working tree has uncommitted changes. \
                         Either commit first or pass allow_dirty=true."
                            .to_string(),
                    )
                } else {
                    Err(format!("lint_code fix failed:\n{combined}"))
                }
            }
        }
    }
}

fn extract_suggestion(msg: &serde_json::Value) -> String {
    let children = match msg.get("children").and_then(|v| v.as_array()) {
        Some(c) => c,
        None => return String::new(),
    };
    for child in children {
        if child.get("level").and_then(|v| v.as_str()) == Some("help") {
            if let Some(spans) = child.get("spans").and_then(|v| v.as_array()) {
                for span in spans {
                    if let Some(suggested) = span
                        .get("suggested_replacement")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                    {
                        return format!("→ {suggested}");
                    }
                }
            }
        }
    }
    String::new()
}

fn count_fixed(output: &str) -> usize {
    // "Fixed N warnings in ..." or look for "warning" lines with "fixed"
    for line in output.lines() {
        if line.contains("Fixed") {
            let digits: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<usize>() {
                return n;
            }
        }
    }
    0
}

fn format_result(lints: &[Lint], _fix: bool) -> Result<String, String> {
    if lints.is_empty() {
        return Ok("lint_code: no warnings or errors — clean!".to_string());
    }

    let errors = lints.iter().filter(|l| l.level == "error").count();
    let warnings = lints.iter().filter(|l| l.level == "warning").count();

    let mut out = format!(
        "LINT RESULTS: {errors} error(s), {warnings} warning(s)\n\
         (run with fix=true to auto-apply machine-fixable suggestions)\n"
    );

    const MAX_SHOWN: usize = 40;
    let shown = lints.len().min(MAX_SHOWN);

    for lint in lints.iter().take(shown) {
        let loc = if lint.line > 0 {
            format!("{}:{}", lint.line, lint.col)
        } else {
            "?".to_string()
        };
        let level_tag = if lint.level == "error" { "[E]" } else { "[W]" };
        out.push_str(&format!(
            "\n{level_tag} {}:{loc}\n    {}\n",
            lint.file,
            lint.message
        ));
        if !lint.code.is_empty() {
            out.push_str(&format!("    code: {}\n", lint.code));
        }
        if !lint.suggestion.is_empty() {
            out.push_str(&format!("    {}\n", lint.suggestion));
        }
        if out.len() > MAX_OUTPUT {
            out.push_str(&format!(
                "\n... {} more lint(s) truncated\n",
                lints.len() - shown
            ));
            break;
        }
    }

    if lints.len() > shown {
        out.push_str(&format!("\n... {} more lint(s)\n", lints.len() - shown));
    }

    Ok(out.trim_end().to_string())
}
