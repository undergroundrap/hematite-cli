use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 300;
const MAX_OUTPUT_BYTES: usize = 131_072;

pub async fn run_hematite_maintainer_workflow(args: &Value) -> Result<String, String> {
    let workflow = args
        .get("workflow")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Missing required argument: 'workflow'".to_string())?;
    let invocation = ScriptInvocation::from_args(workflow, args)?;
    let output = execute_powershell_file(
        &invocation.script_path,
        &invocation.file_args,
        invocation.timeout_secs,
    )
    .await?;

    Ok(format!(
        "Hematite maintainer workflow: {}\nScript: {}\nCommand: {}\n\n{}",
        invocation.workflow_label,
        invocation.script_path.display(),
        invocation.display_command,
        output.trim()
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScriptInvocation {
    workflow_label: &'static str,
    script_path: PathBuf,
    file_args: Vec<String>,
    display_command: String,
    timeout_secs: u64,
}

impl ScriptInvocation {
    fn from_args(workflow: &str, args: &Value) -> Result<Self, String> {
        match workflow {
            "clean" => build_clean_invocation(args),
            "package_windows" => build_package_windows_invocation(args),
            "release" => build_release_invocation(args),
            other => Err(format!(
                "Unknown workflow '{}'. Use one of: clean, package_windows, release.",
                other
            )),
        }
    }
}

fn build_clean_invocation(args: &Value) -> Result<ScriptInvocation, String> {
    let repo_root = require_repo_root()?;
    let mut file_args = Vec::new();
    if bool_arg(args, "deep") {
        file_args.push("-Deep".to_string());
    }
    if bool_arg(args, "reset") {
        file_args.push("-Reset".to_string());
    }
    if bool_arg(args, "prune_dist") {
        file_args.push("-PruneDist".to_string());
    }

    Ok(ScriptInvocation {
        workflow_label: "clean",
        script_path: repo_root.join("clean.ps1"),
        display_command: render_display_command(".\\clean.ps1", &file_args),
        file_args,
        timeout_secs: 180,
    })
}

fn build_package_windows_invocation(args: &Value) -> Result<ScriptInvocation, String> {
    ensure_windows("package_windows")?;
    let repo_root = require_repo_root()?;

    let mut file_args = Vec::new();
    if bool_arg(args, "installer") {
        file_args.push("-Installer".to_string());
    }
    if bool_arg(args, "add_to_path") {
        file_args.push("-AddToPath".to_string());
    }

    Ok(ScriptInvocation {
        workflow_label: "package_windows",
        script_path: repo_root.join("scripts").join("package-windows.ps1"),
        display_command: render_display_command(".\\scripts\\package-windows.ps1", &file_args),
        file_args,
        timeout_secs: 1800,
    })
}

fn build_release_invocation(args: &Value) -> Result<ScriptInvocation, String> {
    let repo_root = require_repo_root()?;
    let version = string_arg(args, "version");
    let bump = string_arg(args, "bump");
    if version.is_none() == bump.is_none() {
        return Err("workflow=release requires exactly one of: 'version' or 'bump'.".to_string());
    }

    let mut file_args = Vec::new();
    if let Some(version) = version {
        file_args.push("-Version".to_string());
        file_args.push(version);
    }
    if let Some(bump) = bump {
        match bump.as_str() {
            "patch" | "minor" | "major" => {
                file_args.push("-Bump".to_string());
                file_args.push(bump);
            }
            other => {
                return Err(format!(
                    "Invalid bump '{}'. Use one of: patch, minor, major.",
                    other
                ))
            }
        }
    }

    for (field, flag) in [
        ("push", "-Push"),
        ("add_to_path", "-AddToPath"),
        ("skip_installer", "-SkipInstaller"),
        ("publish_crates", "-PublishCrates"),
        ("publish_voice_crate", "-PublishVoiceCrate"),
    ] {
        if bool_arg(args, field) {
            file_args.push(flag.to_string());
        }
    }

    Ok(ScriptInvocation {
        workflow_label: "release",
        script_path: repo_root.join("release.ps1"),
        display_command: render_display_command(".\\release.ps1", &file_args),
        file_args,
        timeout_secs: 3600,
    })
}

fn bool_arg(args: &Value, key: &str) -> bool {
    args.get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn string_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn require_repo_root() -> Result<PathBuf, String> {
    find_hematite_repo_root().ok_or_else(|| {
        "Could not locate a Hematite source checkout for this maintainer workflow. Run Hematite from the Hematite repo, launch it from a portable that still lives under that repo's dist/ directory, or switch into the Hematite source workspace before retrying."
            .to_string()
    })
}

fn find_hematite_repo_root() -> Option<PathBuf> {
    let cwd_root = crate::tools::file_ops::workspace_root();
    if is_hematite_repo_root(&cwd_root) {
        return Some(cwd_root);
    }

    let exe = std::env::current_exe().ok()?;
    for ancestor in exe.ancestors() {
        let candidate = ancestor.to_path_buf();
        if is_hematite_repo_root(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn is_hematite_repo_root(path: &std::path::Path) -> bool {
    let cargo_toml = path.join("Cargo.toml");
    let clean = path.join("clean.ps1");
    let release = path.join("release.ps1");
    let package_windows = path.join("scripts").join("package-windows.ps1");
    if !cargo_toml.exists() || !clean.exists() || !release.exists() || !package_windows.exists() {
        return false;
    }

    let cargo_text = match fs::read_to_string(cargo_toml) {
        Ok(text) => text,
        Err(_) => return false,
    };

    cargo_text.contains("name = \"hematite-cli\"") || cargo_text.contains("name = \"hematite\"")
}

fn ensure_windows(workflow: &str) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        Ok(())
    } else {
        Err(format!(
            "workflow={} is Windows-only because it depends on scripts/package-windows.ps1.",
            workflow
        ))
    }
}

fn render_display_command(script: &str, args: &[String]) -> String {
    if args.is_empty() {
        format!("pwsh {}", script)
    } else {
        format!("pwsh {} {}", script, args.join(" "))
    }
}

async fn execute_powershell_file(
    script_path: &std::path::Path,
    file_args: &[String],
    timeout_secs: u64,
) -> Result<String, String> {
    let cwd = require_repo_root()?;
    let shell = resolve_powershell_binary().await;
    let mut command = tokio::process::Command::new(&shell);
    command
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(script_path)
        .args(file_args)
        .current_dir(&cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let child_future = command.output();
    let output = match tokio::time::timeout(
        Duration::from_secs(timeout_secs.max(DEFAULT_TIMEOUT_SECS)),
        child_future,
    )
    .await
    {
        Ok(Ok(output)) => output,
        Ok(Err(err)) => {
            return Err(format!(
                "Failed to execute {}: {err}",
                script_path.display()
            ))
        }
        Err(_) => {
            return Err(format!(
                "Repo workflow timed out after {} seconds: {}",
                timeout_secs.max(DEFAULT_TIMEOUT_SECS),
                script_path.display()
            ))
        }
    };

    let stdout = cap_bytes(&output.stdout, MAX_OUTPUT_BYTES / 2);
    let stderr = cap_bytes(&output.stderr, MAX_OUTPUT_BYTES / 2);
    let exit_info = match output.status.code() {
        Some(0) => String::new(),
        Some(code) => format!("\n[exit code: {code}]"),
        None => "\n[process terminated by signal]".to_string(),
    };

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str("[stderr]\n");
        result.push_str(&stderr);
    }
    if result.is_empty() {
        result.push_str("(no output)");
    }
    result.push_str(&exit_info);
    Ok(crate::agent::utils::strip_ansi(&result))
}

async fn resolve_powershell_binary() -> String {
    if cfg!(target_os = "windows") && command_exists("pwsh").await {
        "pwsh".to_string()
    } else if cfg!(target_os = "windows") {
        "powershell".to_string()
    } else {
        "pwsh".to_string()
    }
}

async fn command_exists(name: &str) -> bool {
    let locator = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    tokio::process::Command::new(locator)
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}

fn cap_bytes(bytes: &[u8], max: usize) -> String {
    if bytes.len() <= max {
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        let mut s = String::from_utf8_lossy(&bytes[..max]).into_owned();
        s.push_str(&format!("\n... [truncated - {} bytes total]", bytes.len()));
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_invocation_supports_deep_prune_dist() {
        let invocation = ScriptInvocation::from_args(
            "clean",
            &serde_json::json!({
                "workflow": "clean",
                "deep": true,
                "prune_dist": true
            }),
        )
        .expect("invocation");

        assert!(invocation.file_args.contains(&"-Deep".to_string()));
        assert!(invocation.file_args.contains(&"-PruneDist".to_string()));
        assert!(invocation.display_command.contains("clean.ps1"));
    }

    #[test]
    fn repo_root_detection_finds_the_hematite_checkout() {
        let root = require_repo_root().expect("repo root");
        assert!(root.join("Cargo.toml").exists());
        assert!(root.join("clean.ps1").exists());
    }

    #[test]
    fn release_invocation_requires_version_or_bump() {
        let err = ScriptInvocation::from_args(
            "release",
            &serde_json::json!({
                "workflow": "release"
            }),
        )
        .unwrap_err();
        assert!(err.contains("requires exactly one"));
    }

    #[test]
    fn release_invocation_builds_publish_flags() {
        let invocation = ScriptInvocation::from_args(
            "release",
            &serde_json::json!({
                "workflow": "release",
                "bump": "patch",
                "push": true,
                "add_to_path": true,
                "publish_crates": true
            }),
        )
        .expect("invocation");

        assert!(invocation.file_args.contains(&"-Bump".to_string()));
        assert!(invocation.file_args.contains(&"patch".to_string()));
        assert!(invocation.file_args.contains(&"-Push".to_string()));
        assert!(invocation.file_args.contains(&"-AddToPath".to_string()));
        assert!(invocation.file_args.contains(&"-PublishCrates".to_string()));
    }
}
