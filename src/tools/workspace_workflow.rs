use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub async fn run_workspace_workflow(args: &Value) -> Result<String, String> {
    let root = require_project_workspace_root()?;
    let invocation = WorkspaceInvocation::from_args(args, &root)?;
    let output = crate::tools::shell::execute_command_in_dir(
        &invocation.command,
        &root,
        invocation.timeout_ms,
        false,
    )
    .await?;

    Ok(format!(
        "Workspace workflow: {}\nWorkspace root: {}\nCommand: {}\n\n{}",
        invocation.workflow_label,
        root.display(),
        invocation.command,
        output.trim()
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceInvocation {
    workflow_label: String,
    command: String,
    timeout_ms: u64,
}

impl WorkspaceInvocation {
    fn from_args(args: &Value, root: &Path) -> Result<Self, String> {
        let workflow = args
            .get("workflow")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "Missing required argument: 'workflow'".to_string())?;
        let timeout_ms = args
            .get("timeout_ms")
            .and_then(|value| value.as_u64())
            .unwrap_or(default_timeout_ms(workflow));

        let command = match workflow {
            "build" => default_command_for_action(root, "build")?,
            "test" => default_command_for_action(root, "test")?,
            "lint" => default_command_for_action(root, "lint")?,
            "fix" => default_command_for_action(root, "fix")?,
            "package_script" => build_package_script_command(root, required_string(args, "name")?)?,
            "task" => format!("task {}", required_string(args, "name")?),
            "just" => format!("just {}", required_string(args, "name")?),
            "make" => format!("make {}", required_string(args, "name")?),
            "script_path" => build_script_path_command(root, required_string(args, "path")?)?,
            "command" => required_string(args, "command")?.to_string(),
            other => {
                return Err(format!(
                    "Unknown workflow '{}'. Use one of: build, test, lint, fix, package_script, task, just, make, script_path, command.",
                    other
                ))
            }
        };

        Ok(Self {
            workflow_label: workflow.to_string(),
            command,
            timeout_ms,
        })
    }
}

fn require_project_workspace_root() -> Result<PathBuf, String> {
    if !crate::tools::file_ops::is_project_workspace() {
        let root = crate::tools::file_ops::workspace_root();
        return Err(format!(
            "No project workspace is locked right now. Hematite is currently rooted at {}. Launch Hematite in the target project directory before asking it to run project-specific scripts or commands.",
            root.display()
        ));
    }
    Ok(crate::tools::file_ops::workspace_root())
}

fn required_string<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Missing required argument: '{}'", key))
}

fn default_timeout_ms(workflow: &str) -> u64 {
    match workflow {
        "build" | "test" | "lint" | "fix" => 1_800_000,
        _ => 600_000,
    }
}

fn default_command_for_action(root: &Path, action: &str) -> Result<String, String> {
    let profile = crate::agent::workspace_profile::load_workspace_profile(root)
        .unwrap_or_else(|| crate::agent::workspace_profile::detect_workspace_profile(root));

    match action {
        "build" => profile
            .build_hint
            .ok_or_else(|| missing_workspace_command_message(action, root)),
        "test" => profile
            .test_hint
            .ok_or_else(|| missing_workspace_command_message(action, root)),
        "lint" => detect_lint_command(root),
        "fix" => detect_fix_command(root),
        other => Err(format!("Unsupported workspace action '{}'.", other)),
    }
}

fn missing_workspace_command_message(action: &str, root: &Path) -> String {
    format!(
        "Hematite could not infer a `{}` command for the locked workspace at {}. Add a workspace verify profile in `.hematite/settings.json`, or ask for an explicit command/script instead.",
        action,
        root.display()
    )
}

fn detect_lint_command(root: &Path) -> Result<String, String> {
    if root.join("Cargo.toml").exists() {
        Ok("cargo clippy --all-targets --all-features -- -D warnings".to_string())
    } else if root.join("package.json").exists() {
        Ok(format!(
            "{} run lint --if-present",
            detect_node_package_manager(root)
        ))
    } else if root.join("go.mod").exists() {
        Err(missing_workspace_command_message("lint", root))
    } else {
        Err(missing_workspace_command_message("lint", root))
    }
}

fn detect_fix_command(root: &Path) -> Result<String, String> {
    if root.join("Cargo.toml").exists() {
        Ok("cargo fmt".to_string())
    } else if root.join("package.json").exists() {
        Ok(format!(
            "{} run fix --if-present",
            detect_node_package_manager(root)
        ))
    } else {
        Err(missing_workspace_command_message("fix", root))
    }
}

fn build_package_script_command(root: &Path, name: &str) -> Result<String, String> {
    let package_json = root.join("package.json");
    if !package_json.exists() {
        return Err(format!(
            "workflow=package_script requires package.json in the locked workspace root ({}).",
            root.display()
        ));
    }

    let content = fs::read_to_string(&package_json)
        .map_err(|e| format!("Failed to read {}: {}", package_json.display(), e))?;
    let package: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", package_json.display(), e))?;
    let has_script = package
        .get("scripts")
        .and_then(|value| value.get(name))
        .is_some();
    if !has_script {
        return Err(format!(
            "package.json does not define a script named `{}` in {}.",
            name,
            root.display()
        ));
    }

    let package_manager = detect_node_package_manager(root);
    let command = match package_manager.as_str() {
        "yarn" => format!("yarn {}", name),
        "bun" => format!("bun run {}", name),
        manager => format!("{} run {}", manager, name),
    };
    Ok(command)
}

fn build_script_path_command(root: &Path, relative_path: &str) -> Result<String, String> {
    let candidate = root.join(relative_path);
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("Failed to resolve workspace root {}: {}", root.display(), e))?;
    let canonical_path = candidate.canonicalize().map_err(|e| {
        format!(
            "Could not resolve script path `{}` from workspace root {}: {}",
            relative_path,
            root.display(),
            e
        )
    })?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(format!(
            "Script path `{}` resolves outside the locked workspace root {}.",
            relative_path,
            root.display()
        ));
    }

    let display_path = normalize_relative_path(&canonical_path, &canonical_root)?;
    let lower = display_path.to_ascii_lowercase();
    if lower.ends_with(".ps1") {
        Ok(format!(
            "pwsh -ExecutionPolicy Bypass -File {}",
            quote_command_arg(&display_path)
        ))
    } else if lower.ends_with(".cmd") || lower.ends_with(".bat") {
        Ok(format!("cmd /C {}", quote_command_arg(&display_path)))
    } else if lower.ends_with(".sh") {
        Ok(format!("bash {}", quote_command_arg(&display_path)))
    } else if lower.ends_with(".py") {
        Ok(format!("python {}", quote_command_arg(&display_path)))
    } else if lower.ends_with(".js") || lower.ends_with(".cjs") || lower.ends_with(".mjs") {
        Ok(format!("node {}", quote_command_arg(&display_path)))
    } else {
        Ok(display_path)
    }
}

fn normalize_relative_path(path: &Path, root: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|e| format!("Failed to normalize script path: {}", e))?;
    Ok(format!(
        ".{}{}",
        std::path::MAIN_SEPARATOR,
        relative.display()
    ))
}

fn quote_command_arg(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn detect_node_package_manager(root: &Path) -> String {
    if root.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if root.join("yarn.lock").exists() {
        "yarn".to_string()
    } else if root.join("bun.lockb").exists() || root.join("bun.lock").exists() {
        "bun".to_string()
    } else {
        "npm".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_script_uses_detected_package_manager() {
        let package_root = std::env::temp_dir().join(format!(
            "hematite-workspace-workflow-node-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&package_root).unwrap();
        std::fs::write(
            package_root.join("package.json"),
            r#"{ "scripts": { "dev": "vite" } }"#,
        )
        .unwrap();
        std::fs::write(package_root.join("pnpm-lock.yaml"), "").unwrap();

        let command = build_package_script_command(&package_root, "dev").unwrap();
        assert_eq!(command, "pnpm run dev");

        let _ = std::fs::remove_file(package_root.join("package.json"));
        let _ = std::fs::remove_file(package_root.join("pnpm-lock.yaml"));
        let _ = std::fs::remove_dir(package_root);
    }

    #[test]
    fn script_path_stays_inside_workspace_root() {
        let script_dir = std::env::temp_dir().join(format!(
            "hematite-workspace-workflow-scripts-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(script_dir.join("scripts")).unwrap();
        std::fs::write(script_dir.join("scripts").join("dev.ps1"), "Write-Host hi").unwrap();

        let command = build_script_path_command(&script_dir, "scripts/dev.ps1").unwrap();
        assert!(command.contains("pwsh -ExecutionPolicy Bypass -File"));

        let _ = std::fs::remove_file(script_dir.join("scripts").join("dev.ps1"));
        let _ = std::fs::remove_dir(script_dir.join("scripts"));
        let _ = std::fs::remove_dir(script_dir);
    }
}
