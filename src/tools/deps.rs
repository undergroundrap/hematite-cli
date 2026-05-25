use serde_json::Value;
use std::time::Duration;

const TIMEOUT: u64 = 60;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or("manage_deps: missing required 'action' (add|remove|list|tree|outdated|audit)")?
        .trim();

    let root = crate::tools::file_ops::workspace_root();

    match action {
        "add" => {
            let name = require_str(args, "name", "manage_deps add")?;
            let version = args.get("version").and_then(|v| v.as_str()).unwrap_or("");
            let features = args.get("features").and_then(|v| v.as_str()).unwrap_or("");
            let dev = args.get("dev").and_then(|v| v.as_bool()).unwrap_or(false);

            let mut cmd_args = vec!["add".to_string(), name.to_string()];
            if !version.is_empty() {
                cmd_args.push(format!("@{version}"));
            }
            if dev {
                cmd_args.push("--dev".to_string());
            }
            if !features.is_empty() {
                cmd_args.extend(["--features".to_string(), features.to_string()]);
            }

            run_cargo(&cmd_args, &root, TIMEOUT).await
        }

        "remove" => {
            let name = require_str(args, "name", "manage_deps remove")?;
            run_cargo(&["remove".to_string(), name.to_string()], &root, TIMEOUT).await
        }

        "list" => {
            // Parse Cargo.toml directly for instant, dependency-free listing.
            let toml_path = root.join("Cargo.toml");
            if !toml_path.exists() {
                return Err("manage_deps list: no Cargo.toml in workspace root".to_string());
            }
            let content = std::fs::read_to_string(&toml_path)
                .map_err(|e| format!("manage_deps list: read Cargo.toml: {e}"))?;
            Ok(format_toml_deps(&content))
        }

        "tree" => {
            let package = args.get("package").and_then(|v| v.as_str()).unwrap_or("");
            let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(3);
            let mut cmd_args = vec!["tree".to_string(), format!("--depth={depth}")];
            if !package.is_empty() {
                cmd_args.extend(["-p".to_string(), package.to_string()]);
            }
            run_cargo(&cmd_args, &root, TIMEOUT).await
        }

        "outdated" => {
            // `cargo outdated` is an external subcommand — gracefully report if not installed.
            let out = run_cargo(&["outdated".to_string()], &root, TIMEOUT).await;
            match out {
                Ok(s) => Ok(s),
                Err(e) if e.contains("no such subcommand") || e.contains("is not a cargo command") => {
                    Err("manage_deps outdated: `cargo-outdated` is not installed. \
                         Install with: cargo install cargo-outdated".to_string())
                }
                Err(e) => Err(e),
            }
        }

        "audit" => {
            let out = run_cargo(&["audit".to_string()], &root, TIMEOUT).await;
            match out {
                Ok(s) => Ok(s),
                Err(e) if e.contains("no such subcommand") || e.contains("is not a cargo command") => {
                    Err("manage_deps audit: `cargo-audit` is not installed. \
                         Install with: cargo install cargo-audit".to_string())
                }
                Err(e) => Err(e),
            }
        }

        other => Err(format!(
            "manage_deps: unknown action '{other}'. Valid: add, remove, list, tree, outdated, audit"
        )),
    }
}

async fn run_cargo(args: &[String], root: &std::path::Path, timeout_secs: u64) -> Result<String, String> {
    let result = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        tokio::process::Command::new("cargo")
            .args(args)
            .current_dir(root)
            .output(),
    )
    .await;

    match result {
        Err(_) => Err(format!("manage_deps: timed out after {timeout_secs}s")),
        Ok(Err(e)) => Err(format!("manage_deps: failed to spawn cargo: {e}")),
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            if output.status.success() {
                let combined = format!("{stdout}{stderr}").trim().to_string();
                Ok(if combined.is_empty() {
                    format!("cargo {} completed successfully.", args.first().map(|s| s.as_str()).unwrap_or("?"))
                } else {
                    combined
                })
            } else {
                let msg = format!("{stderr}{stdout}").trim().to_string();
                Err(msg)
            }
        }
    }
}

fn require_str<'a>(args: &'a Value, key: &str, ctx: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{ctx}: missing required '{key}'"))
}

fn format_toml_deps(content: &str) -> String {
    let mut out = String::from("DEPENDENCIES (from Cargo.toml)\n");
    let mut section = "";
    let mut found_any = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            section = trimmed;
        }
        let is_dep_section = section == "[dependencies]"
            || section == "[dev-dependencies]"
            || section == "[build-dependencies]";
        if is_dep_section && !trimmed.is_empty() && !trimmed.starts_with('[') && !trimmed.starts_with('#') {
            if !found_any {
                found_any = true;
            }
            let label = match section {
                "[dev-dependencies]" => " [dev]",
                "[build-dependencies]" => " [build]",
                _ => "",
            };
            out.push_str(&format!("  {trimmed}{label}\n"));
        }
    }
    if !found_any {
        out.push_str("  (no dependencies found)\n");
    }
    out.trim_end().to_string()
}
