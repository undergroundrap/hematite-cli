use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

const MAX_OUTPUT: usize = 12 * 1024;
const DEFAULT_TIMEOUT: u64 = 120;

pub async fn execute_run_tests(args: &Value) -> Result<String, String> {
    let filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let timeout_secs = args
        .get("timeout_seconds")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT);
    let dry_run = args
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let root = crate::tools::file_ops::workspace_root();

    let (program, cmd_args, label) = detect_runner(&root, &filter)?;

    if dry_run {
        let cmd_display = format!("{} {}", program, cmd_args.join(" "));
        return Ok(format!(
            "run_tests [DRY RUN]: would execute in {}\n  {cmd_display}\n  timeout: {timeout_secs}s",
            root.display()
        ));
    }

    let start = std::time::Instant::now();

    let result = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        tokio::process::Command::new(&program)
            .args(&cmd_args)
            .current_dir(&root)
            .output(),
    )
    .await;

    let elapsed = start.elapsed().as_secs();

    match result {
        Err(_) => Err(format!("run_tests: timed out after {timeout_secs}s")),
        Ok(Err(e)) => Err(format!("run_tests: failed to spawn `{program}`: {e}")),
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let combined = format!("{stdout}{stderr}");

            let (passed, failed, ignored) = parse_counts(&combined);
            let status = if output.status.success() { "PASSED" } else { "FAILED" };

            let mut out = format!(
                "TEST RUN [{status}] — {label} — {elapsed}s\n\
                 passed: {passed}  failed: {failed}  ignored: {ignored}\n"
            );
            if !filter.is_empty() {
                out.push_str(&format!("filter: {filter}\n"));
            }
            out.push('\n');

            if !output.status.success() {
                if let Some(failures) = extract_failures(&combined) {
                    out.push_str(&failures);
                    out.push('\n');
                }
            }

            // Append raw output, tail-preferring (failures are near the end)
            let remaining = MAX_OUTPUT.saturating_sub(out.len());
            out.push_str("── Output ──\n");
            if combined.len() > remaining {
                let tail_start = combined
                    .char_indices()
                    .rev()
                    .nth(remaining)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                out.push_str("...(truncated)...\n");
                out.push_str(&combined[tail_start..]);
            } else {
                out.push_str(&combined);
            }

            Ok(out.trim_end().to_string())
        }
    }
}

fn detect_runner(
    root: &PathBuf,
    filter: &str,
) -> Result<(String, Vec<String>, &'static str), String> {
    if root.join("Cargo.toml").exists() {
        let mut a = vec!["test".to_string()];
        if !filter.is_empty() {
            a.push(filter.to_string());
        }
        return Ok(("cargo".to_string(), a, "Rust/Cargo"));
    }
    if root.join("package.json").exists() {
        let mut a = vec!["test".to_string()];
        if !filter.is_empty() {
            a.extend(["--".to_string(), filter.to_string()]);
        }
        // On Windows `npm` is a .cmd file — needs shell expansion.
        // Use `npm.cmd` on Windows, `npm` elsewhere.
        let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
        return Ok((npm.to_string(), a, "Node/npm"));
    }
    if root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join("pytest.ini").exists()
        || root.join("setup.cfg").exists()
    {
        let py = if cfg!(windows) { "python" } else { "python3" };
        let mut a = vec!["-m".to_string(), "pytest".to_string(), "-v".to_string()];
        if !filter.is_empty() {
            a.extend(["-k".to_string(), filter.to_string()]);
        }
        return Ok((py.to_string(), a, "Python/pytest"));
    }
    Err(
        "run_tests: no recognized project root found \
         (Cargo.toml / package.json / pyproject.toml / setup.py / pytest.ini)"
            .to_string(),
    )
}

fn parse_counts(output: &str) -> (usize, usize, usize) {
    for line in output.lines().rev() {
        // Cargo: "test result: ok. 3 passed; 0 failed; 1 ignored;"
        if line.starts_with("test result:") {
            let passed = extract_num(line, "passed");
            let failed = extract_num(line, "failed");
            let ignored = extract_num(line, "ignored");
            return (passed, failed, ignored);
        }
        // pytest: "3 passed, 2 failed, 1 error" or "5 passed"
        if (line.contains("passed") || line.contains("failed") || line.contains("error"))
            && line.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
        {
            let passed = extract_num(line, "passed");
            let failed =
                extract_num(line, "failed") + extract_num(line, "error");
            let ignored = extract_num(line, "skipped") + extract_num(line, "deselected");
            return (passed, failed, ignored);
        }
    }
    (0, 0, 0)
}

fn extract_num(line: &str, label: &str) -> usize {
    let idx = match line.find(label) {
        Some(i) => i,
        None => return 0,
    };
    // "N passed" — look before the label
    let before = line[..idx].trim_end();
    before
        .split_whitespace()
        .last()
        .and_then(|s| s.trim_matches(';').trim_matches(',').parse().ok())
        .unwrap_or(0)
}

fn extract_failures(output: &str) -> Option<String> {
    // Cargo failure blocks: "---- test_name stdout ----" … "thread '…' panicked"
    let mut in_block = false;
    let mut buf = String::from("── Failures ──\n");
    let mut found = false;

    for line in output.lines() {
        if line.starts_with("---- ") && line.ends_with(" stdout ----") {
            in_block = true;
            found = true;
            buf.push_str(line);
            buf.push('\n');
        } else if in_block {
            if line.starts_with("failures:") || line.starts_with("test result:") {
                in_block = false;
            } else {
                buf.push_str(line);
                buf.push('\n');
                if buf.len() > 6 * 1024 {
                    buf.push_str("... (truncated)\n");
                    break;
                }
            }
        }
    }

    if found { Some(buf) } else { None }
}
