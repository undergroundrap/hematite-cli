use crate::agent::config;
use serde_json::Value;

const BUILD_TIMEOUT_SECS: u64 = 120;

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
    } else if cwd.join("package.json").exists() {
        match action {
            "build" => ("Node/npm", "npm run build --if-present".to_string()),
            "test" => ("Node/npm", "npm test --if-present".to_string()),
            "lint" => ("Node/npm", "npm run lint --if-present".to_string()),
            "fix" => return Err(missing_profile_msg("Node/npm", action)),
            _ => return Err(unknown_action(action)),
        }
    } else if cwd.join("pyproject.toml").exists() || cwd.join("setup.py").exists() {
        match action {
            "build" => ("Python", "python -m compileall .".to_string()),
            "test" => return Err(missing_profile_msg("Python", action)),
            "lint" => return Err(missing_profile_msg("Python", action)),
            "fix" => return Err(missing_profile_msg("Python", action)),
            _ => return Err(unknown_action(action)),
        }
    } else if cwd.join("go.mod").exists() {
        match action {
            "build" => ("Go", "go build ./...".to_string()),
            "test" => ("Go", "go test ./...".to_string()),
            "lint" => return Err(missing_profile_msg("Go", action)),
            "fix" => return Err(missing_profile_msg("Go", action)),
            _ => return Err(unknown_action(action)),
        }
    } else {
        return Err(
            "No recognized project root found.\n\
             Expected one of: Cargo.toml, package.json, pyproject.toml, go.mod\n\
             Ensure you are in the project root directory or configure `.hematite/settings.json` verify profiles."
                .into(),
        );
    };

    Ok((command.0, command.1, timeout_secs))
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
}
