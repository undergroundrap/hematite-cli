use serde_json::Value;
use std::collections::HashSet;
use std::process::Command;

/// Parse structured compiler errors from `cargo check --message-format=json`.
/// Returns a clean list of errors/warnings with file, line, code, and message.
pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("check");

    let include_warnings = args
        .get("warnings")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let explain = args
        .get("explain")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let package = args.get("package").and_then(|v| v.as_str());
    let test_mode = args.get("tests").and_then(|v| v.as_bool()).unwrap_or(false);

    run_structured(action, include_warnings, explain, package, test_mode).await
}

async fn run_structured(
    action: &str,
    include_warnings: bool,
    explain: bool,
    package: Option<&str>,
    test_mode: bool,
) -> Result<String, String> {
    let cargo_cmd = match action {
        "check" | "build" => "check",
        "test" => "test",
        "clippy" => "clippy",
        _ => "check",
    };

    let mut cmd_args: Vec<&str> = vec![cargo_cmd, "--message-format=json"];

    if test_mode || action == "test" {
        // --tests enables the test target without running tests
        if action != "test" {
            cmd_args.push("--tests");
        }
    }

    let pkg_flag;
    if let Some(pkg) = package {
        cmd_args.push("-p");
        pkg_flag = pkg.to_string();
        cmd_args.push(&pkg_flag);
    }

    let output = Command::new("cargo")
        .args(&cmd_args)
        .output()
        .map_err(|e| format!("cargo_errors: failed to run cargo: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    let mut errors: Vec<DiagEntry> = Vec::new();
    let mut warnings: Vec<DiagEntry> = Vec::new();

    for line in stdout.lines() {
        let Ok(msg) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if msg.get("reason").and_then(|r| r.as_str()) != Some("compiler-message") {
            continue;
        }
        let Some(message) = msg.get("message") else {
            continue;
        };
        let level = message
            .get("level")
            .and_then(|l| l.as_str())
            .unwrap_or("unknown");

        if level == "warning" && !include_warnings {
            continue;
        }
        if level != "error" && level != "warning" {
            continue;
        }

        let text = message
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("(no message)")
            .to_string();

        let code = message
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        // Find the primary span (is_primary: true).
        let primary_span = message
            .get("spans")
            .and_then(|s| s.as_array())
            .and_then(|spans| {
                spans
                    .iter()
                    .find(|s| {
                        s.get("is_primary")
                            .and_then(|p| p.as_bool())
                            .unwrap_or(false)
                    })
                    .or_else(|| spans.first())
            });

        let (file, line) = if let Some(span) = primary_span {
            let f = span
                .get("file_name")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let l = span.get("line_start").and_then(|v| v.as_u64()).unwrap_or(0);
            (f, l)
        } else {
            ("?".to_string(), 0)
        };

        // Collect help/note children as hints.
        let hints: Vec<String> = message
            .get("children")
            .and_then(|c| c.as_array())
            .map(|children| {
                children
                    .iter()
                    .filter_map(|child| {
                        let lvl = child.get("level").and_then(|l| l.as_str()).unwrap_or("");
                        if lvl == "help" || lvl == "note" {
                            child
                                .get("message")
                                .and_then(|m| m.as_str())
                                .map(|m| format!("  {lvl}: {m}"))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let entry = DiagEntry {
            file,
            line,
            code,
            message: text,
            hints,
        };

        if level == "error" {
            errors.push(entry);
        } else {
            warnings.push(entry);
        }
    }

    if errors.is_empty() && warnings.is_empty() {
        let status = if output.status.success() {
            "OK"
        } else {
            "FAILED (no structured messages — may be a linker or proc-macro error)"
        };
        return Ok(format!("cargo_errors [{cargo_cmd}]: {status}"));
    }

    let mut out = String::new();

    if !errors.is_empty() {
        out.push_str(&format!("ERRORS ({}) [{cargo_cmd}]:\n", errors.len()));
        for (i, e) in errors.iter().enumerate() {
            let code_str = e
                .code
                .as_deref()
                .map(|c| format!("[{c}] "))
                .unwrap_or_default();
            out.push_str(&format!(
                "\n  [{i}] {}:{} — {}{}\n",
                e.file, e.line, code_str, e.message
            ));
            for h in &e.hints {
                out.push_str(&format!("        {h}\n"));
            }
        }
    }

    if !warnings.is_empty() && include_warnings {
        out.push_str(&format!("\nWARNINGS ({}):\n", warnings.len()));
        for (i, w) in warnings.iter().enumerate() {
            let code_str = w
                .code
                .as_deref()
                .map(|c| format!("[{c}] "))
                .unwrap_or_default();
            out.push_str(&format!(
                "\n  [{i}] {}:{} — {}{}\n",
                w.file, w.line, code_str, w.message
            ));
        }
    }

    // Append `rustc --explain Exxxx` for each unique error code when requested.
    if explain && !errors.is_empty() {
        let mut seen: HashSet<String> = HashSet::new();
        let unique_codes: Vec<String> = errors
            .iter()
            .filter_map(|e| e.code.clone())
            .filter(|c| c.starts_with('E') && seen.insert(c.clone()))
            .collect();

        if !unique_codes.is_empty() {
            out.push_str("\nERROR EXPLANATIONS:\n");
            for code in &unique_codes {
                let explain_out = Command::new("rustc").args(["--explain", code]).output();
                match explain_out {
                    Ok(o) if o.status.success() => {
                        let text = String::from_utf8_lossy(&o.stdout);
                        // Cap each explanation at 60 lines to avoid flooding context.
                        let capped: String = text.lines().take(60).collect::<Vec<_>>().join("\n");
                        out.push_str(&format!("\n── {code} ──\n{capped}\n"));
                    }
                    _ => {
                        out.push_str(&format!("\n── {code} ── (explanation unavailable)\n"));
                    }
                }
            }
        }
    }

    Ok(out.trim_end().to_string())
}

struct DiagEntry {
    file: String,
    line: u64,
    code: Option<String>,
    message: String,
    hints: Vec<String>,
}
