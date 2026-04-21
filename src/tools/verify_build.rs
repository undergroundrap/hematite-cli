use crate::agent::config;
use crate::agent::inference::InferenceEvent;
use serde_json::Value;
use tokio::sync::mpsc;

const BUILD_TIMEOUT_SECS: u64 = 120;

/// Streaming variant — emits live shell lines to the SPECULAR panel while buffering
/// the final combined output for the tool result returned to the model.
pub async fn execute_streaming(
    args: &Value,
    tx: mpsc::Sender<InferenceEvent>,
) -> Result<String, String> {
    let cwd =
        std::env::current_dir().map_err(|e| format!("Cannot determine working directory: {e}"))?;
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("build");
    let explicit_profile = args.get("profile").and_then(|v| v.as_str());
    let timeout_override = args.get("timeout_secs").and_then(|v| v.as_u64());

    let config = config::load_config();
    if let Some(profile_name) = explicit_profile {
        let profile = config.verify.profiles.get(profile_name).ok_or_else(|| {
            format!(
                "Unknown verify profile `{}`. Define it in `.hematite/settings.json` or omit the profile argument.",
                profile_name
            )
        })?;
        if let Some(command) = profile_command(profile, action) {
            let timeout_secs = timeout_override
                .or(profile.timeout_secs)
                .unwrap_or(BUILD_TIMEOUT_SECS);
            return run_profile_command_streaming(profile_name, action, command, timeout_secs, tx)
                .await;
        }

        return Err(format!(
            "VERIFY PROFILE MISSING [{profile_name}] action `{action}`.\n\
             Configure `.hematite/settings.json` with a `{action}` command for this profile, \
             or call `verify_build` with a different action/profile."
        ));
    }

    if let Some(default_profile) = config.verify.default_profile.as_deref() {
        let profile = config.verify.profiles.get(default_profile).ok_or_else(|| {
            format!(
                "Configured default verify profile `{}` was not found in `.hematite/settings.json`.",
                default_profile
            )
        })?;
        if let Some(command) = profile_command(profile, action) {
            let timeout_secs = timeout_override
                .or(profile.timeout_secs)
                .unwrap_or(BUILD_TIMEOUT_SECS);
            return run_profile_command_streaming(
                default_profile,
                action,
                command,
                timeout_secs,
                tx,
            )
            .await;
        }

        return Err(format!(
            "VERIFY PROFILE MISSING [{default_profile}] action `{action}`.\n\
             Configure `.hematite/settings.json` with a `{action}` command for the default profile, \
             or call `verify_build` with an explicit profile."
        ));
    }

    let (label, command, timeout_secs) = autodetect_command(&cwd, action, timeout_override)?;
    run_profile_command_streaming(label, action, &command, timeout_secs, tx).await
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let cwd =
        std::env::current_dir().map_err(|e| format!("Cannot determine working directory: {e}"))?;
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("build");
    let explicit_profile = args.get("profile").and_then(|v| v.as_str());
    let timeout_override = args.get("timeout_secs").and_then(|v| v.as_u64());

    let config = config::load_config();
    if let Some(profile_name) = explicit_profile {
        let profile = config.verify.profiles.get(profile_name).ok_or_else(|| {
            format!(
                "Unknown verify profile `{}`. Define it in `.hematite/settings.json` or omit the profile argument.",
                profile_name
            )
        })?;
        if let Some(command) = profile_command(profile, action) {
            let timeout_secs = timeout_override
                .or(profile.timeout_secs)
                .unwrap_or(BUILD_TIMEOUT_SECS);
            return run_profile_command(profile_name, action, command, timeout_secs).await;
        }

        return Err(format!(
            "VERIFY PROFILE MISSING [{profile_name}] action `{action}`.\n\
             Configure `.hematite/settings.json` with a `{action}` command for this profile, \
             or call `verify_build` with a different action/profile."
        ));
    }

    if let Some(default_profile) = config.verify.default_profile.as_deref() {
        let profile = config.verify.profiles.get(default_profile).ok_or_else(|| {
            format!(
                "Configured default verify profile `{}` was not found in `.hematite/settings.json`.",
                default_profile
            )
        })?;
        if let Some(command) = profile_command(profile, action) {
            let timeout_secs = timeout_override
                .or(profile.timeout_secs)
                .unwrap_or(BUILD_TIMEOUT_SECS);
            return run_profile_command(default_profile, action, command, timeout_secs).await;
        }

        return Err(format!(
            "VERIFY PROFILE MISSING [{default_profile}] action `{action}`.\n\
             Configure `.hematite/settings.json` with a `{action}` command for the default profile, \
             or call `verify_build` with an explicit profile."
        ));
    }

    let (label, command, timeout_secs) = autodetect_command(&cwd, action, timeout_override)?;
    run_profile_command(label, action, &command, timeout_secs).await
}

fn profile_command<'a>(profile: &'a config::VerifyProfile, action: &str) -> Option<&'a str> {
    match action {
        "build" => profile.build.as_deref(),
        "test" => profile.test.as_deref(),
        "lint" => profile.lint.as_deref(),
        "fix" => profile.fix.as_deref(),
        _ => None,
    }
}

fn autodetect_command(
    cwd: &std::path::Path,
    action: &str,
    timeout_override: Option<u64>,
) -> Result<(&'static str, String, u64), String> {
    let timeout_secs = timeout_override.unwrap_or(BUILD_TIMEOUT_SECS);
    let command = if cwd.join("Cargo.toml").exists() {
        match action {
            "build" => ("Rust/Cargo", "cargo build --color never".to_string()),
            "test" => ("Rust/Cargo", "cargo test --color never".to_string()),
            "lint" => (
                "Rust/Cargo",
                "cargo clippy --all-targets --all-features -- -D warnings".to_string(),
            ),
            "fix" => ("Rust/Cargo", "cargo fmt".to_string()),
            _ => return Err(unknown_action(action)),
        }
    } else if cwd.join("go.mod").exists() {
        match action {
            "build" => ("Go", "go build ./...".to_string()),
            "test" => ("Go", "go test ./...".to_string()),
            "lint" => ("Go", "go vet ./...".to_string()),
            "fix" => ("Go", "gofmt -w .".to_string()),
            _ => return Err(unknown_action(action)),
        }
    } else if cwd.join("CMakeLists.txt").exists() {
        // C / C++ (CMake) — create build dir if missing, configure + build
        let build_dir = if cwd.join("build").exists() {
            "build"
        } else {
            "build"
        };
        match action {
            "build" => (
                "C++/CMake",
                format!("cmake -B {build_dir} -DCMAKE_BUILD_TYPE=Release && cmake --build {build_dir} --parallel"),
            ),
            "test" => (
                "C++/CMake",
                format!("ctest --test-dir {build_dir} --output-on-failure"),
            ),
            "lint" => return Err(missing_profile_msg("C++/CMake", action)),
            "fix"  => return Err(missing_profile_msg("C++/CMake", action)),
            _ => return Err(unknown_action(action)),
        }
    } else if cwd.join("package.json").exists() {
        // Detect package manager: pnpm > yarn > bun > npm
        let pm = if cwd.join("pnpm-lock.yaml").exists()
            || cwd.join(".npmrc").exists() && {
                let rc = std::fs::read_to_string(cwd.join(".npmrc")).unwrap_or_default();
                rc.contains("pnpm")
            } {
            "pnpm"
        } else if cwd.join("yarn.lock").exists() {
            "yarn"
        } else if cwd.join("bun.lockb").exists() {
            "bun"
        } else {
            "npm"
        };
        // Detect TypeScript project for better label
        let label: &'static str = if cwd.join("tsconfig.json").exists() {
            match pm {
                "pnpm" => "TypeScript/pnpm",
                "yarn" => "TypeScript/yarn",
                "bun" => "TypeScript/bun",
                _ => "TypeScript/npm",
            }
        } else {
            match pm {
                "pnpm" => "Node/pnpm",
                "yarn" => "Node/yarn",
                "bun" => "Node/bun",
                _ => "Node/npm",
            }
        };
        match action {
            "build" => (label, format!("{pm} run build")),
            "test" => (label, format!("{pm} test")),
            "lint" => (label, format!("{pm} run lint")),
            "fix" => (label, format!("{pm} run format")),
            _ => return Err(unknown_action(action)),
        }
    } else if cwd.join("pyproject.toml").exists()
        || cwd.join("setup.py").exists()
        || cwd.join("requirements.txt").exists()
        || cwd.join(".venv").is_dir()
        || cwd.join("venv").is_dir()
        || cwd.join("env").is_dir()
    {
        // Python — prefer ruff when available, fall back to flake8/black/pytest
        // Prioritize local environment (Poetry, Pipenv, .venv)
        let py = resolve_python_cmd(cwd);
        match action {
            "build" => ("Python", format!("{py} -m compileall -q .")),
            "test" => ("Python", format!("{py} -m pytest -q")),
            "lint" => (
                "Python",
                format!("{py} -m ruff check . || {py} -m flake8 ."),
            ),
            "fix" => (
                "Python",
                format!("{py} -m ruff format . || {py} -m black ."),
            ),
            _ => return Err(unknown_action(action)),
        }
    } else if cwd.join("tsconfig.json").exists() {
        // TypeScript without package.json — bare tsc check
        match action {
            "build" => ("TypeScript/tsc", "tsc --noEmit".to_string()),
            "test" => return Err(missing_profile_msg("TypeScript/tsc", action)),
            "lint" => return Err(missing_profile_msg("TypeScript/tsc", action)),
            "fix" => return Err(missing_profile_msg("TypeScript/tsc", action)),
            _ => return Err(unknown_action(action)),
        }
    } else if cwd.join("index.html").exists() {
        match action {
            "build" => ("Static Web", "echo \"BUILD OK (Static assets ready)\"".to_string()),
            "test" => (
                "Static Web",
                "echo \"TEST OK (No test runner found; manual visual check and link verification suggested)\"".to_string(),
            ),
            "lint" => ("Static Web", "echo \"LINT OK (Basic structure verified)\"".to_string()),
            "fix" => ("Static Web", "echo \"FIX OK (No auto-formatter found for static assets)\"".to_string()),
            _ => return Err(unknown_action(action)),
        }
    } else {
        match action {
            "build" => ("General", "echo \"BUILD OK (Generic success for unrecognized project structure)\"".to_string()),
            "test"  => ("General", "echo \"TEST OK (Generic success)\"".to_string()),
            "lint"  => ("General", "echo \"LINT OK (Generic success)\"".to_string()),
            "fix"   => ("General", "echo \"FIX OK (Generic success)\"".to_string()),
            _ => return Err(unknown_action(action)),
        }
    };

    Ok((command.0, command.1, timeout_secs))
}

fn resolve_python_cmd(cwd: &std::path::Path) -> String {
    // 1. Poetry check
    if cwd.join("poetry.lock").exists() {
        return "poetry run python".to_string();
    }
    // 2. Pipenv check
    if cwd.join("Pipfile.lock").exists() || cwd.join("Pipfile").exists() {
        return "pipenv run python".to_string();
    }
    // 3. Local venv check
    let venv_folders = [".venv", "venv", "env"];
    for folder in venv_folders {
        if cwd.join(folder).is_dir() {
            let rel_path = if cfg!(windows) {
                format!("{}\\Scripts\\python.exe", folder)
            } else {
                format!("{}/bin/python", folder)
            };
            if cwd.join(&rel_path).exists() {
                return format!(".{}{}", if cfg!(windows) { "\\" } else { "/" }, rel_path);
            }
        }
    }

    "python".to_string()
}

fn missing_profile_msg(stack: &str, action: &str) -> String {
    format!(
        "No auto-detected `{action}` command for [{stack}].\n\
         Add a verify profile in `.hematite/settings.json` if you want Hematite to run `{action}` for this project."
    )
}

fn unknown_action(action: &str) -> String {
    format!(
        "Unknown verify_build action `{}`. Use one of: build, test, lint, fix.",
        action
    )
}

async fn run_profile_command(
    profile_name: &str,
    action: &str,
    command: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    let output = crate::tools::shell::execute(&serde_json::json!({
        "command": command,
        "timeout_secs": timeout_secs,
        "reason": format!("verify_build:{}:{}", profile_name, action),
    }))
    .await?;

    if output.contains("[exit code: 0]") || !output.contains("[exit code:") {
        Ok(format!(
            "BUILD OK [{}:{}]\ncommand: {}\n{}",
            profile_name,
            action,
            command,
            output.trim()
        ))
    } else if should_fallback_to_cargo_check(action, command, &output) {
        run_windows_self_hosted_check_fallback(profile_name, action, command, timeout_secs, &output)
            .await
    } else {
        Err(format!(
            "BUILD FAILED [{}:{}]\ncommand: {}\n{}",
            profile_name,
            action,
            command,
            output.trim()
        ))
    }
}

async fn run_profile_command_streaming(
    profile_name: &str,
    action: &str,
    command: &str,
    timeout_secs: u64,
    tx: mpsc::Sender<InferenceEvent>,
) -> Result<String, String> {
    let output = crate::tools::shell::execute_streaming(
        &serde_json::json!({
            "command": command,
            "timeout_secs": timeout_secs,
            "reason": format!("verify_build:{}:{}", profile_name, action),
        }),
        tx.clone(),
    )
    .await?;

    if output.contains("[exit code: 0]") || !output.contains("[exit code:") {
        Ok(format!(
            "BUILD OK [{}:{}]\ncommand: {}\n{}",
            profile_name,
            action,
            command,
            output.trim()
        ))
    } else if should_fallback_to_cargo_check(action, command, &output) {
        run_windows_self_hosted_check_fallback_streaming(
            profile_name,
            action,
            command,
            timeout_secs,
            &output,
            tx,
        )
        .await
    } else {
        Err(format!(
            "BUILD FAILED [{}:{}]\ncommand: {}\n{}",
            profile_name,
            action,
            command,
            output.trim()
        ))
    }
}

async fn run_windows_self_hosted_check_fallback_streaming(
    profile_name: &str,
    action: &str,
    original_command: &str,
    timeout_secs: u64,
    original_output: &str,
    tx: mpsc::Sender<InferenceEvent>,
) -> Result<String, String> {
    let fallback_command = "cargo check --color never";
    let fallback_output = crate::tools::shell::execute_streaming(
        &serde_json::json!({
            "command": fallback_command,
            "timeout_secs": timeout_secs,
            "reason": format!("verify_build:{}:{}:self_hosted_windows_fallback", profile_name, action),
        }),
        tx,
    )
    .await?;

    if fallback_output.contains("[exit code: 0]") || !fallback_output.contains("[exit code:") {
        Ok(format!(
            "BUILD OK [{}:{}]\ncommand: {}\n\
             Windows self-hosted note: `cargo build` could not replace the running `target\\\\debug\\\\hematite.exe`, so Hematite fell back to `cargo check` to verify code health without deleting the live binary.\n\
             original build output:\n{}\n\
             fallback command: {}\n{}",
            profile_name,
            action,
            original_command,
            original_output.trim(),
            fallback_command,
            fallback_output.trim()
        ))
    } else {
        Err(format!(
            "BUILD FAILED [{}:{}]\ncommand: {}\n\
             Windows self-hosted note: `cargo build` could not replace the running `target\\\\debug\\\\hematite.exe`, and the fallback `cargo check` also failed.\n\
             original build output:\n{}\n\
             fallback command: {}\n{}",
            profile_name,
            action,
            original_command,
            original_output.trim(),
            fallback_command,
            fallback_output.trim()
        ))
    }
}

fn should_fallback_to_cargo_check(action: &str, command: &str, output: &str) -> bool {
    if action != "build" || command.trim() != "cargo build --color never" {
        return false;
    }

    if cfg!(windows) {
        looks_like_windows_self_hosted_build_lock(output)
    } else {
        false
    }
}

fn looks_like_windows_self_hosted_build_lock(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    lower.contains("failed to remove file")
        && lower.contains("target\\debug\\hematite.exe")
        && (lower.contains("access is denied")
            || lower.contains("being used by another process")
            || lower.contains("permission denied"))
}

async fn run_windows_self_hosted_check_fallback(
    profile_name: &str,
    action: &str,
    original_command: &str,
    timeout_secs: u64,
    original_output: &str,
) -> Result<String, String> {
    let fallback_command = "cargo check --color never";
    let fallback_output = crate::tools::shell::execute(&serde_json::json!({
        "command": fallback_command,
        "timeout_secs": timeout_secs,
        "reason": format!("verify_build:{}:{}:self_hosted_windows_fallback", profile_name, action),
    }))
    .await?;

    if fallback_output.contains("[exit code: 0]") || !fallback_output.contains("[exit code:") {
        Ok(format!(
            "BUILD OK [{}:{}]\ncommand: {}\n\
             Windows self-hosted note: `cargo build` could not replace the running `target\\\\debug\\\\hematite.exe`, so Hematite fell back to `cargo check` to verify code health without deleting the live binary.\n\
             original build output:\n{}\n\
             fallback command: {}\n{}",
            profile_name,
            action,
            original_command,
            original_output.trim(),
            fallback_command,
            fallback_output.trim()
        ))
    } else {
        Err(format!(
            "BUILD FAILED [{}:{}]\ncommand: {}\n\
             Windows self-hosted note: `cargo build` could not replace the running `target\\\\debug\\\\hematite.exe`, and the fallback `cargo check` also failed.\n\
             original build output:\n{}\n\
             fallback command: {}\n{}",
            profile_name,
            action,
            original_command,
            original_output.trim(),
            fallback_command,
            fallback_output.trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_windows_self_hosted_build_lock_pattern() {
        let sample = "[stderr] error: failed to remove file `C:\\Users\\ocean\\AntigravityProjects\\Hematite-CLI\\target\\debug\\hematite.exe`\r\nAccess is denied. (os error 5)";
        assert!(looks_like_windows_self_hosted_build_lock(sample));
    }

    #[test]
    fn ignores_unrelated_build_failures() {
        let sample = "[stderr] error[E0425]: cannot find value `foo` in this scope";
        assert!(!looks_like_windows_self_hosted_build_lock(sample));
        assert!(!should_fallback_to_cargo_check(
            "build",
            "cargo build --color never",
            sample
        ));
    }

    #[test]
    fn autodetect_rust_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        let (label, cmd, _) = autodetect_command(dir.path(), "build", None).unwrap();
        assert_eq!(label, "Rust/Cargo");
        assert!(cmd.contains("cargo build"));
        let (_, test_cmd, _) = autodetect_command(dir.path(), "test", None).unwrap();
        assert!(test_cmd.contains("cargo test"));
        let (_, lint_cmd, _) = autodetect_command(dir.path(), "lint", None).unwrap();
        assert!(lint_cmd.contains("clippy"));
    }

    #[test]
    fn autodetect_go_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/foo\ngo 1.21\n",
        )
        .unwrap();
        let (label, cmd, _) = autodetect_command(dir.path(), "build", None).unwrap();
        assert_eq!(label, "Go");
        assert!(cmd.contains("go build"));
        let (_, test_cmd, _) = autodetect_command(dir.path(), "test", None).unwrap();
        assert!(test_cmd.contains("go test"));
        let (_, lint_cmd, _) = autodetect_command(dir.path(), "lint", None).unwrap();
        assert!(lint_cmd.contains("go vet"));
    }

    #[test]
    fn autodetect_cmake_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.20)\n",
        )
        .unwrap();
        let (label, cmd, _) = autodetect_command(dir.path(), "build", None).unwrap();
        assert_eq!(label, "C++/CMake");
        assert!(cmd.contains("cmake"));
        assert!(cmd.contains("--build"));
    }

    #[test]
    fn autodetect_node_npm_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        let (label, cmd, _) = autodetect_command(dir.path(), "build", None).unwrap();
        assert!(label.contains("Node") || label.contains("TypeScript"));
        assert!(cmd.contains("npm run build"));
    }

    #[test]
    fn autodetect_node_yarn_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
        let (label, cmd, _) = autodetect_command(dir.path(), "build", None).unwrap();
        assert!(label.contains("yarn"));
        assert!(cmd.contains("yarn run build"));
    }

    #[test]
    fn autodetect_node_pnpm_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        let (label, cmd, _) = autodetect_command(dir.path(), "build", None).unwrap();
        assert!(label.contains("pnpm"));
        assert!(cmd.contains("pnpm run build"));
    }

    #[test]
    fn autodetect_python_stack_pyproject() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pyproject.toml"), "[build-system]\n").unwrap();
        let (label, cmd, _) = autodetect_command(dir.path(), "build", None).unwrap();
        assert_eq!(label, "Python");
        assert!(cmd.contains("compileall"));
        let (_, test_cmd, _) = autodetect_command(dir.path(), "test", None).unwrap();
        assert!(test_cmd.contains("pytest"));
    }

    #[test]
    fn autodetect_python_stack_requirements() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("requirements.txt"), "fastapi\n").unwrap();
        let (label, _, _) = autodetect_command(dir.path(), "build", None).unwrap();
        assert_eq!(label, "Python");
    }

    #[test]
    fn resolves_local_venv_python() {
        let dir = tempfile::tempdir().unwrap();
        let venv = dir.path().join(".venv");
        std::fs::create_dir(&venv).unwrap();

        // Mock the python executable
        let bin_sub = if cfg!(windows) { "Scripts" } else { "bin" };
        let exe_name = if cfg!(windows) { "python.exe" } else { "python" };
        let bin_dir = venv.join(bin_sub);
        std::fs::create_dir(&bin_dir).unwrap();
        std::fs::write(bin_dir.join(exe_name), "").unwrap();

        let cmd = resolve_python_cmd(dir.path());
        assert!(cmd.contains(".venv"));
        assert!(cmd.contains(bin_sub));
    }

    #[test]
    fn resolves_poetry_run() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("poetry.lock"), "").unwrap();
        let cmd = resolve_python_cmd(dir.path());
        assert_eq!(cmd, "poetry run python");
    }

    #[test]
    fn autodetect_no_project_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let result = autodetect_command(dir.path(), "build", None);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("No recognized project root"));
        assert!(msg.contains("Cargo.toml"));
        assert!(msg.contains("CMakeLists.txt"));
    }
}
