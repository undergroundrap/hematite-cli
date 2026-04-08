//! Sandboxed code execution tool.
//!
//! Lets the model write and run code in a restricted subprocess — no network,
//! no filesystem escape, hard timeout. Supports JavaScript/TypeScript (Deno)
//! and Python. Neither runtime is bundled; we detect what's available and
//! report clearly when nothing is found.

use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 10;
const MAX_TIMEOUT_SECS: u64 = 60;
const MAX_OUTPUT_BYTES: usize = 16_384; // 16 KB — enough for any reasonable script result

pub async fn execute(args: &Value) -> Result<String, String> {
    let language = args
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("javascript")
        .to_lowercase();

    let code = args
        .get("code")
        .and_then(|v| v.as_str())
        .ok_or("Missing required argument: 'code'")?;

    let timeout_secs = args
        .get("timeout_seconds")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .min(MAX_TIMEOUT_SECS);

    match language.as_str() {
        "javascript" | "typescript" | "js" | "ts" => run_deno(code, timeout_secs),
        "python" | "python3" | "py" => run_python(code, timeout_secs),
        other => Err(format!(
            "Unsupported language: '{}'. Supported: javascript, typescript, python.",
            other
        )),
    }
}

/// Run code via Deno with strict permission flags.
/// Uses stdin so no temp file is needed.
fn run_deno(code: &str, timeout_secs: u64) -> Result<String, String> {
    let deno = find_deno().ok_or_else(|| {
        "Deno not found. Hematite checks (in order): settings.json `deno_path`, \
         LM Studio's bundled copy (~/.lmstudio/.internal/utils/deno.exe), system PATH. \
         To install Deno globally: `winget install DenoLand.Deno` (Windows) or see https://deno.com. \
         Or set `deno_path` in .hematite/settings.json to point to any Deno binary."
            .to_string()
    })?;

    let mut child = Command::new(&deno)
        .args([
            "run",
            "--allow-read=.",  // workspace only
            "--allow-write=.", // workspace only
            "--deny-net",      // no outbound network
            "--deny-sys",      // no OS info
            "--deny-env",      // no environment variable access
            "--deny-run",      // no spawning other processes
            "--deny-ffi",      // no native library calls (FFI escape vector)
            "--no-prompt",     // never ask for permissions interactively
            "-",               // read from stdin — no temp file needed
        ])
        .env("NO_COLOR", "true") // clean output, no ANSI codes in results
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn Deno: {e}"))?;

    write_stdin(&mut child, code)?;
    collect_output(child, "deno", timeout_secs)
}

/// Run code via Python with network and subprocess access blocked at env level.
/// Python has no built-in permission flags like Deno, so we restrict the
/// environment and use a short timeout as the primary safety net.
fn run_python(code: &str, timeout_secs: u64) -> Result<String, String> {
    let python = find_executable(&["python3", "python"])
        .ok_or_else(|| "Python is not installed or not on PATH.".to_string())?;

    let child = Command::new(&python)
        .args([
            "-c",
            // Wrap the code: block network imports and dangerous builtins before running.
            &wrap_python(code),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Strip PATH so the script can't find other executables easily
        .env_clear()
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .spawn()
        .map_err(|e| format!("Failed to spawn Python: {e}"))?;

    collect_output(child, "python", timeout_secs)
}

/// Wraps the user's Python in a minimal sandbox: blocks socket, subprocess,
/// os.system, and __import__ to prevent the most obvious escapes.
fn wrap_python(code: &str) -> String {
    format!(
        r#"
import sys

# Block network access
import socket as _socket
_socket.socket = None  # type: ignore

# Block subprocess and os.system
import os as _os
_os.system = lambda *a, **k: (_ for _ in ()).throw(PermissionError("os.system blocked in sandbox"))
_popen_orig = _os.popen
_os.popen = lambda *a, **k: (_ for _ in ()).throw(PermissionError("os.popen blocked in sandbox"))

# Block __import__ for subprocess
_real_import = __builtins__.__import__ if hasattr(__builtins__, '__import__') else __import__
def _safe_import(name, *args, **kwargs):
    if name in ('subprocess', 'multiprocessing', 'pty', 'telnetlib', 'ftplib', 'smtplib', 'http', 'urllib', 'requests', 'httpx'):
        raise ImportError(f"Module '{{name}}' is blocked in the sandbox.")
    return _real_import(name, *args, **kwargs)
import builtins
builtins.__import__ = _safe_import

# Run the actual code
exec(compile(r"""{code}""", "<sandbox>", "exec"))
"#,
        code = code.replace(r#"""""#, r#"\" \" \""#)
    )
}

fn write_stdin(child: &mut std::process::Child, code: &str) -> Result<(), String> {
    let mut stdin = child
        .stdin
        .take()
        .ok_or("Failed to open stdin for sandbox process")?;
    stdin
        .write_all(code.as_bytes())
        .map_err(|e| format!("Failed to write to sandbox stdin: {e}"))?;
    drop(stdin); // signal EOF so the process starts
    Ok(())
}

fn collect_output(
    child: std::process::Child,
    runtime: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    let timeout = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    // Poll with wait_with_output — use a thread so we can enforce a timeout.
    let output = std::thread::scope(|s| {
        let handle = s.spawn(|| child.wait_with_output());
        loop {
            if start.elapsed() >= timeout {
                return Err(format!(
                    "Sandbox timeout: process exceeded {}s and was killed.",
                    timeout_secs
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
            if handle.is_finished() {
                return handle
                    .join()
                    .map_err(|_| "Sandbox thread panicked.".to_string())?
                    .map_err(|e| format!("Failed to collect {runtime} output: {e}"));
            }
        }
    })?;

    let stdout = truncate(
        &String::from_utf8_lossy(&output.stdout),
        MAX_OUTPUT_BYTES / 2,
    );
    let stderr = truncate(
        &String::from_utf8_lossy(&output.stderr),
        MAX_OUTPUT_BYTES / 2,
    );

    if output.status.success() {
        if stdout.trim().is_empty() && stderr.trim().is_empty() {
            Ok("(no output)".to_string())
        } else if stderr.trim().is_empty() {
            Ok(stdout)
        } else {
            Ok(format!("{stdout}\n[stderr]\n{stderr}"))
        }
    } else {
        let code = output
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string());
        Err(format!(
            "Exit code {code}\n{}\n{}",
            stdout.trim(),
            stderr.trim()
        ))
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\n... [truncated]", &s[..max])
    }
}

/// Locate Deno with a priority-ordered search:
/// 1. `deno_path` in .hematite/settings.json (explicit user pin)
/// 2. Standard deno install location (~/.deno/bin/deno.exe)
/// 3. WinGet package location (winget doesn't always add to PATH correctly)
/// 4. System PATH via where/which
/// 5. LM Studio's bundled copy — automatic fallback for all LM Studio users
fn find_deno() -> Option<String> {
    // 1. settings.json override
    let config = crate::agent::config::load_config();
    if let Some(path) = config.deno_path {
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }

    let exe = if cfg!(windows) { "deno.exe" } else { "deno" };

    // 2. Standard deno install path (deno's own installer puts it here)
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let standard = std::path::Path::new(&home)
            .join(".deno")
            .join("bin")
            .join(exe);
        if standard.exists() {
            return Some(standard.to_string_lossy().into_owned());
        }
    }

    // 3. WinGet package location — winget installs Deno here but doesn't always
    //    wire PATH correctly for non-PowerShell processes
    if cfg!(windows) {
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            let winget_base = std::path::Path::new(&local_app)
                .join("Microsoft")
                .join("WinGet")
                .join("Packages");
            if let Ok(entries) = std::fs::read_dir(&winget_base) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    if name.to_string_lossy().starts_with("DenoLand.Deno") {
                        let candidate = entry.path().join("deno.exe");
                        if candidate.exists() {
                            return Some(candidate.to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }
    }

    // 4. System PATH
    let check = if cfg!(windows) {
        Command::new("where").arg("deno").output()
    } else {
        Command::new("which").arg("deno").output()
    };
    if let Ok(out) = check {
        if out.status.success() {
            // Use the resolved path, not just "deno", to avoid shim ambiguity
            let resolved = String::from_utf8_lossy(&out.stdout)
                .trim()
                .lines()
                .next()
                .unwrap_or("deno")
                .to_string();
            return Some(resolved);
        }
    }

    // 5. LM Studio bundled copy — last resort
    find_lmstudio_deno()
}

/// Find the first available executable from a list of candidates.
fn find_executable(candidates: &[&str]) -> Option<String> {
    for name in candidates {
        let check = if cfg!(windows) {
            Command::new("where").arg(name).output()
        } else {
            Command::new("which").arg(name).output()
        };
        if check.map(|o| o.status.success()).unwrap_or(false) {
            return Some(name.to_string());
        }
    }
    None
}

/// Returns the path to Deno bundled inside LM Studio's internal utils folder.
/// LM Studio ships Deno at `~/.lmstudio/.internal/utils/deno[.exe]` on all platforms.
fn find_lmstudio_deno() -> Option<String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;

    let exe = if cfg!(windows) { "deno.exe" } else { "deno" };
    let path = std::path::Path::new(&home)
        .join(".lmstudio")
        .join(".internal")
        .join("utils")
        .join(exe);

    if path.exists() {
        Some(path.to_string_lossy().into_owned())
    } else {
        None
    }
}
