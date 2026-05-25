use serde_json::Value;
use std::time::Duration;

const TIMEOUT: u64 = 60;

pub async fn execute(args: &Value) -> Result<String, String> {
    let check_only = args.get("check").and_then(|v| v.as_bool()).unwrap_or(false);
    let path_filter = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
        std::path::PathBuf::from(r)
    } else {
        crate::tools::file_ops::workspace_root()
    };

    // Detect workspace type and pick formatter
    if root.join("Cargo.toml").exists() {
        return format_rust(&root, check_only, path_filter).await;
    }
    if root.join("package.json").exists() {
        return format_node(&root, check_only).await;
    }
    if root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join(".ruff.toml").exists()
    {
        return format_python(&root, check_only).await;
    }

    Err("format_code: no recognized project root \
         (Cargo.toml / package.json / pyproject.toml)"
        .to_string())
}

async fn format_rust(
    root: &std::path::Path,
    check_only: bool,
    path_filter: &str,
) -> Result<String, String> {
    let mut args = vec!["fmt".to_string()];
    if check_only {
        args.push("--check".to_string());
    }
    if !path_filter.is_empty() {
        // fmt --  path does not exist; format a specific file via rustfmt directly
        return format_rust_file(root, path_filter, check_only).await;
    }

    let result = tokio::time::timeout(
        Duration::from_secs(TIMEOUT),
        tokio::process::Command::new("cargo")
            .args(&args)
            .current_dir(root)
            .output(),
    )
    .await;

    match result {
        Err(_) => Err(format!("format_code: cargo fmt timed out after {TIMEOUT}s")),
        Ok(Err(e)) => Err(format!("format_code: spawn cargo fmt: {e}")),
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            if check_only {
                if output.status.success() {
                    Ok("format_code [CHECK]: all files are correctly formatted.".to_string())
                } else {
                    // cargo fmt --check exits non-zero when files need formatting
                    let files = extract_unformatted_files(&stderr);
                    Ok(format!(
                        "format_code [CHECK]: {} file(s) need reformatting:\n{}\n\
                         Run format_code() without check=true to apply.",
                        files.len(),
                        files.join("\n")
                    ))
                }
            } else {
                // Apply mode: detect which files changed by running --check first, then applying
                let changed = detect_changed_files(root).await;
                if output.status.success() {
                    if changed.is_empty() {
                        Ok("format_code: no changes needed — already formatted.".to_string())
                    } else {
                        Ok(format!(
                            "format_code [APPLIED]: {} file(s) reformatted:\n{}",
                            changed.len(),
                            changed.join("\n")
                        ))
                    }
                } else {
                    Err(format!("format_code: cargo fmt failed:\n{stderr}"))
                }
            }
        }
    }
}

async fn format_rust_file(
    root: &std::path::Path,
    path: &str,
    check_only: bool,
) -> Result<String, String> {
    let file_path = root.join(path);
    if !file_path.exists() {
        return Err(format!("format_code: file not found: {path}"));
    }

    let mut args = vec![file_path.to_string_lossy().to_string()];
    if check_only {
        args.push("--check".to_string());
    }

    let result = tokio::time::timeout(
        Duration::from_secs(TIMEOUT),
        tokio::process::Command::new("rustfmt").args(&args).output(),
    )
    .await;

    match result {
        Err(_) => Err("format_code: rustfmt timed out".to_string()),
        Ok(Err(e)) => Err(format!("format_code: spawn rustfmt: {e}")),
        Ok(Ok(output)) => {
            if check_only {
                if output.status.success() {
                    Ok(format!(
                        "format_code [CHECK]: {path} is correctly formatted."
                    ))
                } else {
                    Ok(format!("format_code [CHECK]: {path} needs reformatting. Run without check=true to apply."))
                }
            } else if output.status.success() {
                Ok(format!("format_code [APPLIED]: {path} reformatted."))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                Err(format!("format_code: rustfmt failed on {path}:\n{stderr}"))
            }
        }
    }
}

async fn format_node(root: &std::path::Path, check_only: bool) -> Result<String, String> {
    // Try prettier, fall back to a clear message
    let prettier = if cfg!(windows) {
        "prettier.cmd"
    } else {
        "prettier"
    };
    let mut args = vec!["--write".to_string(), ".".to_string()];
    if check_only {
        args = vec!["--check".to_string(), ".".to_string()];
    }

    let result = tokio::time::timeout(
        Duration::from_secs(TIMEOUT),
        tokio::process::Command::new(prettier)
            .args(&args)
            .current_dir(root)
            .output(),
    )
    .await;

    match result {
        Err(_) => Err("format_code: prettier timed out".to_string()),
        Ok(Err(e)) => Err(format!(
            "format_code: prettier not found ({e}). Install with: npm install -g prettier"
        )),
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            if output.status.success() {
                let action = if check_only { "CHECK" } else { "APPLIED" };
                Ok(
                    format!("format_code [{action}] (prettier):\n{stdout}{stderr}")
                        .trim()
                        .to_string(),
                )
            } else {
                Err(format!("format_code: prettier failed:\n{stderr}"))
            }
        }
    }
}

async fn format_python(root: &std::path::Path, check_only: bool) -> Result<String, String> {
    // Try ruff format first, fall back to black
    for (tool, check_flag, apply_flag) in &[
        ("ruff", vec!["format", "--check", "."], vec!["format", "."]),
        ("black", vec!["--check", "."], vec!["."]),
    ] {
        let args: Vec<String> = if check_only {
            check_flag.iter().map(|s| s.to_string()).collect()
        } else {
            apply_flag.iter().map(|s| s.to_string()).collect()
        };

        let result = tokio::time::timeout(
            Duration::from_secs(TIMEOUT),
            tokio::process::Command::new(tool)
                .args(&args)
                .current_dir(root)
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let action = if check_only { "CHECK" } else { "APPLIED" };
                if output.status.success() || (!check_only && output.status.success()) {
                    return Ok(
                        format!("format_code [{action}] ({tool}):\n{stdout}{stderr}")
                            .trim()
                            .to_string(),
                    );
                } else if check_only {
                    return Ok(format!(
                        "format_code [CHECK] ({tool}): files need reformatting.\n{stdout}{stderr}\n\
                         Run without check=true to apply."
                    )
                    .trim()
                    .to_string());
                } else {
                    return Err(format!("format_code: {tool} failed:\n{stderr}"));
                }
            }
            Ok(Err(_)) => continue, // tool not found, try next
            Err(_) => return Err(format!("format_code: {tool} timed out")),
        }
    }

    Err("format_code: neither ruff nor black is installed. \
         Install: pip install ruff OR pip install black"
        .to_string())
}

fn extract_unformatted_files(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|l| l.contains("Diff in") || l.ends_with(".rs"))
        .map(|l| l.trim().to_string())
        .collect()
}

async fn detect_changed_files(root: &std::path::Path) -> Vec<String> {
    // Ask git which files changed after fmt was applied
    let result = tokio::process::Command::new("git")
        .args(["diff", "--name-only"])
        .current_dir(root)
        .output()
        .await;

    match result {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| format!("  {l}"))
            .collect(),
        _ => Vec::new(),
    }
}
