use serde_json::Value;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const MAX_OUTPUT_BYTES: usize = 65_536; // 64 KB cap (higher for professional mode)

/// Execute a shell command and return its stdout/stderr combined as a String.
///
/// Unified Shell Adapter:
/// - Windows: Tries `pwsh` first, then `powershell.exe`, then `cmd /C`.
/// - Unix: Tries `sh -c`.
pub async fn execute(args: &Value) -> Result<String, String> {
    let mut command = args
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: 'command'".to_string())?
        .to_string();

    // Expand @path/to/file into the absolute workspace path before execution.
    if command.contains('@') {
        let root = crate::tools::file_ops::workspace_root();
        let root_str = root.to_string_lossy().to_string().replace("\\", "/");
        command = command.replace('@', &format!("{}/", root_str.trim_end_matches('/')));
    }

    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            args.get("timeout_secs")
                .and_then(|v| v.as_u64())
                .map(|s| s * 1000)
        })
        .unwrap_or(DEFAULT_TIMEOUT_SECS * 1000);

    let run_in_background = args
        .get("run_in_background")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let cwd =
        std::env::current_dir().map_err(|e| format!("Failed to get working directory: {e}"))?;

    execute_command_in_dir(&command, &cwd, timeout_ms, run_in_background).await
}

/// Like `execute`, but streams each stdout/stderr line to the TUI as a
/// `ShellLine` event while the command runs, so the operator sees live
/// progress instead of a blank screen until completion.
///
/// Falls back to plain `execute` for background tasks.
pub async fn execute_streaming(
    args: &Value,
    tx: mpsc::Sender<crate::agent::inference::InferenceEvent>,
) -> Result<String, String> {
    // Background tasks don't benefit from streaming — delegate to execute().
    if args
        .get("run_in_background")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return execute(args).await;
    }

    let mut command = args
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: 'command'".to_string())?
        .to_string();

    if command.contains('@') {
        let root = crate::tools::file_ops::workspace_root();
        let root_str = root.to_string_lossy().replace("\\", "/").to_string();
        command = command.replace('@', &format!("{}/", root_str.trim_end_matches('/')));
    }

    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            args.get("timeout_secs")
                .and_then(|v| v.as_u64())
                .map(|s| s * 1000)
        })
        .unwrap_or(DEFAULT_TIMEOUT_SECS * 1000);

    crate::tools::guard::bash_is_safe(&command)?;

    let cwd =
        std::env::current_dir().map_err(|e| format!("Failed to get working directory: {e}"))?;

    let mut tokio_cmd = build_command(&command).await;
    tokio_cmd
        .current_dir(&cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let sandbox_root = cwd.join(".hematite").join("sandbox");
    let _ = std::fs::create_dir_all(&sandbox_root);
    tokio_cmd.env("HOME", &sandbox_root);
    tokio_cmd.env("TMPDIR", &sandbox_root);

    let mut child = tokio_cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn process: {e}"))?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
    let mut stderr_lines = tokio::io::BufReader::new(stderr).lines();

    let mut out_buf = String::new();
    let mut err_buf = String::new();
    let mut stdout_done = false;
    let mut stderr_done = false;

    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);

    loop {
        if stdout_done && stderr_done {
            break;
        }
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => {
                let _ = child.kill().await;
                return Err(format!("Command timed out after {} ms: {}", timeout_ms, command));
            }
            line = stdout_lines.next_line(), if !stdout_done => {
                match line {
                    Ok(Some(l)) => {
                        let clean = l.trim_end_matches('\r').to_string();
                        let _ = tx
                            .send(crate::agent::inference::InferenceEvent::ShellLine(clean.clone()))
                            .await;
                        out_buf.push_str(&clean);
                        out_buf.push('\n');
                    }
                    _ => stdout_done = true,
                }
            }
            line = stderr_lines.next_line(), if !stderr_done => {
                match line {
                    Ok(Some(l)) => {
                        let clean = l.trim_end_matches('\r').to_string();
                        let _ = tx
                            .send(crate::agent::inference::InferenceEvent::ShellLine(
                                format!("[err] {}", clean),
                            ))
                            .await;
                        err_buf.push_str(&clean);
                        err_buf.push('\n');
                    }
                    _ => stderr_done = true,
                }
            }
        }
    }

    // Wait for exit, with a short grace period for cleanup.
    let status = tokio::time::timeout(Duration::from_millis(5_000), child.wait())
        .await
        .map_err(|_| "Process cleanup timed out".to_string())?
        .map_err(|e| format!("Failed to wait for process: {e}"))?;

    let stdout_capped = cap_bytes(out_buf.as_bytes(), MAX_OUTPUT_BYTES / 2);
    let stderr_capped = cap_bytes(err_buf.as_bytes(), MAX_OUTPUT_BYTES / 2);

    let exit_info = match status.code() {
        Some(0) => String::new(),
        Some(code) => format!("\n[exit code: {code}]"),
        None => "\n[process terminated by signal]".to_string(),
    };

    let mut result = String::new();
    if !stdout_capped.is_empty() {
        result.push_str(&stdout_capped);
    }
    if !stderr_capped.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str("[stderr]\n");
        result.push_str(&stderr_capped);
    }
    if result.is_empty() {
        result.push_str("(no output)");
    }
    result.push_str(&exit_info);

    Ok(crate::agent::utils::strip_ansi(&result))
}

pub async fn execute_command_in_dir(
    command: &str,
    cwd: &Path,
    timeout_ms: u64,
    run_in_background: bool,
) -> Result<String, String> {
    crate::tools::guard::bash_is_safe(command)?;

    let mut tokio_cmd = build_command(command).await;
    tokio_cmd
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let sandbox_root = cwd.join(".hematite").join("sandbox");
    let _ = std::fs::create_dir_all(&sandbox_root);
    tokio_cmd.env("HOME", &sandbox_root);
    tokio_cmd.env("TMPDIR", &sandbox_root);

    if run_in_background {
        let _child = tokio_cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn background process: {e}"))?;
        return Ok(
            "[background_task_id: spawned]\nCommand started in background. Use `ps` or `jobs` to monitor if available."
                .into(),
        );
    }

    let child_future = tokio_cmd.output();

    let output = match tokio::time::timeout(Duration::from_millis(timeout_ms), child_future).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(format!("Failed to execution process: {e}")),
        Err(_) => {
            return Err(format!(
                "Command timed out after {} ms: {}",
                timeout_ms, command
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

/// Build the platform-appropriate shell invocation.
async fn build_command(command: &str) -> tokio::process::Command {
    #[cfg(target_os = "windows")]
    {
        let normalized = command
            .replace("/dev/null", "$null")
            .replace("1>/dev/null", "2>$null")
            .replace("2>/dev/null", "2>$null");

        if which("pwsh").await {
            let mut cmd = tokio::process::Command::new("pwsh");
            cmd.args(["-NoProfile", "-NonInteractive", "-Command", &normalized]);
            cmd
        } else {
            let mut cmd = tokio::process::Command::new("powershell");
            cmd.args(["-NoProfile", "-NonInteractive", "-Command", &normalized]);
            cmd
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = tokio::process::Command::new("sh");
        cmd.args(["-c", command]);
        cmd
    }
}

#[allow(dead_code)]
async fn which(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    let check = format!("{}.exe", name);
    #[cfg(not(target_os = "windows"))]
    let check = name;

    tokio::process::Command::new("where")
        .arg(check)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
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
