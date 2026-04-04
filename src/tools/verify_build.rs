use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

const BUILD_TIMEOUT_SECS: u64 = 120;

/// Auto-detect the project type from the current directory and run the
/// appropriate build/validation command.
///
/// Supports: Rust (Cargo.toml), Node (package.json), Python (pyproject.toml),
/// Go (go.mod). Returns "BUILD OK" or "BUILD FAILED:\n{error output}".
pub async fn execute(_args: &Value) -> Result<String, String> {
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Cannot determine working directory: {e}"))?;

    let (label, cmd, args): (&str, &str, Vec<&str>) =
        if cwd.join("Cargo.toml").exists() {
            ("Rust/Cargo", "cargo", vec!["build", "--color", "never"])
        } else if cwd.join("package.json").exists() {
            // npm install is non-destructive if node_modules already exists.
            ("Node/npm", "npm", vec!["run", "build", "--if-present"])
        } else if cwd.join("pyproject.toml").exists() || cwd.join("setup.py").exists() {
            ("Python", "python", vec!["-m", "py_compile"])
        } else if cwd.join("go.mod").exists() {
            ("Go", "go", vec!["build", "./..."])
        } else {
            return Err(
                "No recognized project root found.\n\
                 Expected one of: Cargo.toml, package.json, pyproject.toml, go.mod\n\
                 Ensure you are in the project root directory.".into()
            );
        };

    let child = Command::new(cmd)
        .args(&args)
        .current_dir(&cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn `{cmd}`: {e}"))?;

    let output = tokio::time::timeout(
        Duration::from_secs(BUILD_TIMEOUT_SECS),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| format!("[{label}] Build timed out after {BUILD_TIMEOUT_SECS}s"))?
    .map_err(|e| format!("Build process error: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Combine both streams for full diagnostics.
    let combined = {
        let mut s = String::new();
        if !stdout.is_empty() { s.push_str(&stdout); }
        if !stderr.is_empty() {
            if !s.is_empty() { s.push('\n'); }
            s.push_str(&stderr);
        }
        s
    };

    if output.status.success() {
        Ok(format!("BUILD OK [{label}]\n{}", combined.trim()))
    } else {
        let exit_code = output.status.code().map_or("?".to_string(), |c| c.to_string());
        Err(format!(
            "BUILD FAILED [{label}] (exit {exit_code})\n{}",
            combined.trim()
        ))
    }
}
