use serde_json::Value;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const POLL_INTERVAL_MS: u64 = 500;

pub async fn execute(args: &Value) -> Result<String, String> {
    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or("profile_process: missing required 'command'")?
        .trim();

    let timeout_secs = args
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT_SECS);

    profile_command(command, timeout_secs).await
}

async fn profile_command(command: &str, timeout_secs: u64) -> Result<String, String> {
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

    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let start = Instant::now();
    let child = cmd
        .spawn()
        .map_err(|e| format!("profile_process: failed to spawn process: {e}"))?;

    let pid = child.id();

    // Spawn background poller — samples the process tree every POLL_INTERVAL_MS.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<PollSample>(256);
    let poll_pid = pid;
    let poll_timeout = timeout_secs;
    tokio::spawn(async move {
        let deadline = Instant::now() + Duration::from_secs(poll_timeout + 2);
        loop {
            if Instant::now() > deadline {
                break;
            }
            if let Some(sample) = sample_process(poll_pid) {
                if tx.send(sample).await.is_err() {
                    break;
                }
            } else {
                // Process gone.
                break;
            }
            tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        }
    });

    // Wait for the process to finish (with timeout).
    let result =
        tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait_with_output()).await;

    let wall_ms = start.elapsed().as_millis() as u64;

    // Drain all samples.
    rx.close();
    let mut samples: Vec<PollSample> = Vec::new();
    while let Ok(s) = rx.try_recv() {
        samples.push(s);
    }

    match result {
        Ok(Ok(out)) => {
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            let exit_code = out.status.code();
            format_profile(command, wall_ms, exit_code, false, &samples, &stdout, &stderr)
        }
        Ok(Err(e)) => Err(format!("profile_process: process error: {e}")),
        Err(_) => {
            // Timed out — still produce a partial profile.
            let (stdout, stderr) = (String::new(), String::new());
            format_profile(command, wall_ms, None, true, &samples, &stdout, &stderr)
        }
    }
}

#[derive(Debug)]
struct PollSample {
    cpu_percent: f64,
    ram_mb: f64,
}

fn sample_process(pid: Option<u32>) -> Option<PollSample> {
    let pid = pid?;

    #[cfg(target_os = "windows")]
    {
        sample_windows(pid)
    }

    #[cfg(not(target_os = "windows"))]
    {
        sample_linux(pid)
    }
}

#[cfg(target_os = "windows")]
fn sample_windows(pid: u32) -> Option<PollSample> {
    // Use Get-Process for a single-shot snapshot — no WMI overhead.
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "try {{ $p=Get-Process -Id {pid} -EA Stop; \
                 '{{}},{{}}' -f [math]::Round($p.CPU,2),[math]::Round($p.WorkingSet64/1MB,2) }} \
                 catch {{ 'gone' }}",
            ),
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let s = s.trim();
    if s == "gone" || s.is_empty() {
        return None;
    }
    // Format: "cpu_secs,ram_mb" — CPU here is cumulative seconds, not %; we use it
    // as a proxy for load between samples.
    let mut parts = s.split(',');
    let cpu: f64 = parts.next()?.trim().parse().ok()?;
    let ram: f64 = parts.next()?.trim().parse().ok()?;
    Some(PollSample {
        cpu_percent: cpu,
        ram_mb: ram,
    })
}

#[cfg(not(target_os = "windows"))]
fn sample_linux(pid: u32) -> Option<PollSample> {
    // Read /proc/<pid>/stat for CPU ticks and /proc/<pid>/status for RSS.
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;

    // stat fields: pid comm state ppid ... utime(14) stime(15) ...
    let fields: Vec<&str> = stat.split_whitespace().collect();
    let utime: u64 = fields.get(13)?.parse().ok()?;
    let stime: u64 = fields.get(14)?.parse().ok()?;
    let cpu_ticks = utime + stime;

    let ram_kb: f64 = status
        .lines()
        .find(|l| l.starts_with("VmRSS:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()?;

    Some(PollSample {
        cpu_percent: cpu_ticks as f64 / sysconf_clk_tck() * 100.0,
        ram_mb: ram_kb / 1024.0,
    })
}

#[cfg(not(target_os = "windows"))]
fn sysconf_clk_tck() -> f64 {
    // Typical value is 100 Hz; avoids unsafe libc call.
    100.0
}

fn format_profile(
    command: &str,
    wall_ms: u64,
    exit_code: Option<i32>,
    timed_out: bool,
    samples: &[PollSample],
    stdout: &str,
    stderr: &str,
) -> Result<String, String> {
    let mut out = String::new();

    let status = if timed_out {
        "TIMEOUT".to_string()
    } else {
        match exit_code {
            Some(0) => "EXIT OK".to_string(),
            Some(c) => format!("EXIT {c}"),
            None => "EXIT ?".to_string(),
        }
    };

    out.push_str(&format!("PROFILE RESULT [{status}]\n"));
    out.push_str(&format!("command : {command}\n"));
    out.push_str(&format!(
        "wall    : {}.{:03}s\n",
        wall_ms / 1000,
        wall_ms % 1000
    ));

    if !samples.is_empty() {
        let peak_ram = samples.iter().map(|s| s.ram_mb).fold(0.0_f64, f64::max);
        let peak_cpu = samples.iter().map(|s| s.cpu_percent).fold(0.0_f64, f64::max);
        let avg_ram =
            samples.iter().map(|s| s.ram_mb).sum::<f64>() / samples.len() as f64;

        out.push_str(&format!("samples : {}\n", samples.len()));
        out.push_str(&format!("peak RAM: {peak_ram:.1} MB\n"));
        out.push_str(&format!("avg RAM : {avg_ram:.1} MB\n"));
        out.push_str(&format!("peak CPU: {peak_cpu:.1} (cumulative CPU-s on Windows; %ticks on Linux)\n"));
    } else {
        out.push_str("samples : 0 (process exited before first poll or poller unavailable)\n");
    }

    let combined = format!("{}\n{}", stdout, stderr);
    let trimmed = combined.trim();
    if !trimmed.is_empty() {
        out.push_str("\nOUTPUT:\n");
        let preview: String = trimmed.lines().take(30).collect::<Vec<_>>().join("\n");
        out.push_str(&preview);
        let total = trimmed.lines().count();
        if total > 30 {
            out.push_str(&format!("\n... ({} more lines)", total - 30));
        }
    }

    Ok(out)
}
