use serde_json::Value;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;

// Frames from these prefixes are noise — hide them unless the stack is short.
const NOISE_PREFIXES: &[&str] = &[
    "std::",
    "core::",
    "alloc::",
    "tokio::",
    "futures::",
    "backtrace::",
    "rust_begin_unwind",
    "rust_panic",
    "__rust_",
    "___rust_",
    "panic_handler",
    "start_thread",
    "<unknown>",
];

pub async fn execute(args: &Value) -> Result<String, String> {
    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or("run_with_backtrace: missing required 'command'")?
        .trim();

    let timeout_secs = args
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT_SECS);

    let output = run_with_backtrace(command, timeout_secs).await?;
    Ok(format_output(command, &output))
}

struct CapturedOutput {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    timed_out: bool,
}

async fn run_with_backtrace(command: &str, timeout_secs: u64) -> Result<CapturedOutput, String> {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = tokio::process::Command::new("cmd");
        c.args(["/C", command]);
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = tokio::process::Command::new("sh");
        c.args(["-c", command]);
        c
    };

    cmd.env("RUST_BACKTRACE", "full")
        .env("RUST_LOG", "error")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let child = cmd
        .spawn()
        .map_err(|e| format!("run_with_backtrace: failed to spawn process: {e}"))?;

    let result =
        tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait_with_output()).await;

    match result {
        Ok(Ok(out)) => Ok(CapturedOutput {
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            exit_code: out.status.code(),
            timed_out: false,
        }),
        Ok(Err(e)) => Err(format!("run_with_backtrace: process error: {e}")),
        Err(_) => Ok(CapturedOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            timed_out: true,
        }),
    }
}

fn format_output(command: &str, out: &CapturedOutput) -> String {
    let mut result = String::new();

    if out.timed_out {
        result.push_str(&format!(
            "TIMEOUT — process did not exit within the allowed window\ncommand: {command}\n"
        ));
        return result;
    }

    let combined = format!("{}\n{}", out.stdout, out.stderr);
    let exit_ok = out.exit_code == Some(0);

    if exit_ok {
        result.push_str(&format!(
            "EXIT OK [exit 0]\ncommand: {command}\n\n{}",
            combined.trim()
        ));
        return result;
    }

    // Non-zero exit — try to parse panic/backtrace.
    let panic_msg = extract_panic_message(&combined);
    let frames = extract_stack_frames(&combined);

    result.push_str(&format!(
        "EXIT FAILED [exit {}]\ncommand: {command}\n",
        out.exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string())
    ));

    if let Some(panic) = &panic_msg {
        result.push_str(&format!("\nPANIC: {panic}\n"));
    }

    if !frames.is_empty() {
        let signal = frames.iter().filter(|f| is_signal(f)).count();
        let noise = frames.iter().filter(|f| is_noise(f)).count();

        result.push_str("\nSTACK TRACE (filtered):\n");
        let mut shown = 0;
        for (i, frame) in frames.iter().enumerate() {
            if !is_noise(frame) {
                result.push_str(&format!("  {:>2}: {frame}\n", i));
                shown += 1;
            }
        }
        if noise > 0 {
            result.push_str(&format!(
                "  ... {noise} stdlib/runtime frames hidden ({signal} signal frames)\n"
            ));
        }
        if shown == 0 {
            // All frames were noise — show the top 5 unfiltered.
            result.push_str("  (all frames are runtime — showing top 5 raw)\n");
            for (i, frame) in frames.iter().take(5).enumerate() {
                result.push_str(&format!("  {:>2}: {frame}\n", i));
            }
        }
    } else if panic_msg.is_none() {
        // No panic, no frames — just show raw output.
        result.push_str("\nRAW OUTPUT:\n");
        let trimmed = combined.trim();
        let preview: String = trimmed.lines().take(40).collect::<Vec<_>>().join("\n");
        result.push_str(&preview);
        if trimmed.lines().count() > 40 {
            result.push_str(&format!(
                "\n... ({} more lines)",
                trimmed.lines().count() - 40
            ));
        }
    }

    result
}

fn extract_panic_message(output: &str) -> Option<String> {
    // "thread 'X' panicked at 'MESSAGE', file.rs:N:M"
    // "thread 'X' panicked at file.rs:N:M:\nMESSAGE"  (Rust 2021 format)
    for line in output.lines() {
        if line.contains("panicked at") {
            // Strip the leading timestamp/level prefix if present.
            let line = line.trim_start_matches(|c: char| !c.is_alphabetic());
            return Some(line.trim().to_string());
        }
    }
    // Windows access violation / Linux segfault summary lines.
    for line in output.lines() {
        let l = line.trim();
        if l.starts_with("Segmentation fault")
            || l.starts_with("Access violation")
            || l.starts_with("fatal runtime error")
        {
            return Some(l.to_string());
        }
    }
    None
}

fn extract_stack_frames(output: &str) -> Vec<String> {
    let mut frames: Vec<String> = Vec::new();
    let mut in_backtrace = false;

    for line in output.lines() {
        let trimmed = line.trim();

        // Detect the start of a Rust backtrace block.
        if trimmed.contains("stack backtrace:") || trimmed.contains("BACKTRACE:") {
            in_backtrace = true;
            continue;
        }

        if in_backtrace {
            // End markers.
            if trimmed.is_empty() && frames.len() > 2 {
                break;
            }
            if trimmed.starts_with("note:") || trimmed.starts_with("error[") {
                break;
            }

            // Rust backtrace frame: "   N: symbol_name" followed by "     at file:line"
            if let Some(rest) = trimmed
                .strip_prefix(|c: char| c.is_ascii_digit())
                .and_then(|s| s.strip_prefix(':'))
            {
                frames.push(rest.trim().to_string());
            } else if let Some(loc) = trimmed.strip_prefix("at ") {
                // Attach location to previous frame.
                if let Some(last) = frames.last_mut() {
                    last.push_str(&format!("  ← {loc}"));
                }
            }
        }
    }

    // Fallback: if no backtrace block found, look for numbered frames anywhere.
    if frames.is_empty() {
        for line in output.lines() {
            let trimmed = line.trim();
            // cdb/windbg style: "00 <addr> <module>!function+offset"
            if trimmed.len() > 4
                && trimmed.chars().take(2).all(|c| c.is_ascii_hexdigit())
                && trimmed.contains('!')
            {
                if let Some(sym) = trimmed.splitn(3, ' ').nth(2) {
                    frames.push(sym.to_string());
                }
            }
        }
    }

    frames
}

fn is_noise(frame: &str) -> bool {
    NOISE_PREFIXES.iter().any(|prefix| frame.contains(prefix))
}

fn is_signal(frame: &str) -> bool {
    frame.contains("signal") || frame.contains("raise") || frame.contains("abort")
}
