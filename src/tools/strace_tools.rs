use serde_json::{json, Value};
use std::collections::HashMap;

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["summary", "calls", "files", "network", "errors"],
                "description": "Action to perform (default: summary)"
            },
            "text": { "type": "string", "description": "strace output as inline text" },
            "file": { "type": "string", "description": "Path to strace output file" },
            "syscall": { "type": "string", "description": "Filter by syscall name (calls action)" },
            "pid": { "type": "integer", "description": "Filter by PID (calls/files/network action)" },
            "limit": { "type": "integer", "description": "Max results to show (default 50)" }
        }
    })
}

// ── data model ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
struct StraceCall {
    pid: Option<u32>,
    syscall: String,
    args_raw: String,
    result: String,
    errno: Option<String>,
    /// elapsed time in µs if -T was used
    elapsed_us: Option<u64>,
    is_error: bool,
}

// ── parser ───────────────────────────────────────────────────────────────────

fn parse_strace(text: &str) -> Vec<StraceCall> {
    let mut calls: Vec<StraceCall> = Vec::new();

    for line in text.lines() {
        let line = line.trim();

        // skip signal lines, exit lines, empty, and comments
        if line.is_empty()
            || line.starts_with("---")
            || line.starts_with("+++")
            || line.starts_with('#')
        {
            continue;
        }

        // skip unfinished lines (<unfinished ...>)
        if line.contains("<unfinished ...>") {
            continue;
        }

        let (rest, pid) = extract_pid(line);
        let rest = rest.trim();

        // strip leading timestamp (HH:MM:SS.ffffff or epoch.ffffff)
        let rest = strip_timestamp(rest);

        // parse: syscall_name(args) = result [errno] [<elapsed>]
        if let Some(call) = parse_call_line(rest, pid) {
            calls.push(call);
        }
    }

    calls
}

fn extract_pid(line: &str) -> (&str, Option<u32>) {
    // [pid NNNN] ... or NNNN  ... (with leading spaces/digits)
    if let Some(rest) = line.strip_prefix("[pid ") {
        if let Some(end) = rest.find(']') {
            let pid_str = &rest[..end];
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                return (&rest[end + 1..], Some(pid));
            }
        }
    }
    // Bare PID at start: "4242  openat(...)"
    let trimmed = line.trim_start();
    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() && digits.len() < 7 {
        let after = &trimmed[digits.len()..];
        if after.starts_with("  ") || after.starts_with('\t') {
            if let Ok(pid) = digits.parse::<u32>() {
                return (after.trim_start(), Some(pid));
            }
        }
    }
    (line, None)
}

fn strip_timestamp(s: &str) -> &str {
    // HH:MM:SS or HH:MM:SS.ffffff or epoch.ffffff
    let trimmed = s.trim_start();
    // find a space after what looks like a timestamp
    let end = trimmed
        .find(' ')
        .or_else(|| trimmed.find('\t'))
        .unwrap_or(0);
    let candidate = &trimmed[..end];
    // simple heuristic: has exactly 2 colons → HH:MM:SS
    // or is all digits + one dot → epoch timestamp
    let colon_count = candidate.chars().filter(|&c| c == ':').count();
    if colon_count == 2
        || (candidate.contains('.') && candidate.chars().all(|c| c.is_ascii_digit() || c == '.'))
    {
        trimmed[end..].trim_start()
    } else {
        trimmed
    }
}

fn parse_call_line(line: &str, pid: Option<u32>) -> Option<StraceCall> {
    // find the opening paren
    let paren_open = line.find('(')?;
    let syscall = line[..paren_open].trim().to_string();

    // syscall must be a valid identifier
    if syscall.is_empty() || !syscall.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    // find the = sign that marks the result (scan backwards from end)
    let eq_pos = find_result_eq(line)?;
    let args_raw = line[paren_open + 1..eq_pos]
        .trim()
        .trim_end_matches(')')
        .trim()
        .to_string();
    let result_part = line[eq_pos + 1..].trim();

    // strip <elapsed> from result: "0 <0.000042>"
    let (result_clean, elapsed_us) = extract_elapsed(result_part);

    // check for errno: "-1 ENOENT (No such file or directory)"
    let errno = extract_errno(result_clean);
    let is_error = errno.is_some() || result_clean.trim_start().starts_with("-1 ");

    Some(StraceCall {
        pid,
        syscall,
        args_raw,
        result: result_clean.to_string(),
        errno,
        elapsed_us,
        is_error,
    })
}

/// Find the `=` that separates args from result.
/// Skips `=` inside parentheses/brackets/braces.
fn find_result_eq(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_str = !in_str,
            b'(' | b'[' | b'{' if !in_str => depth += 1,
            b')' | b']' | b'}' if !in_str => depth -= 1,
            b'=' if !in_str && depth == 0 => {
                // make sure it's not == or !=
                let prev = if i > 0 { bytes[i - 1] } else { 0 };
                let next = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
                if prev != b'!' && prev != b'=' && prev != b'<' && prev != b'>' && next != b'=' {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn extract_elapsed(s: &str) -> (&str, Option<u64>) {
    // "<0.000042>" at end
    if let Some(start) = s.rfind('<') {
        if s.ends_with('>') {
            let inner = &s[start + 1..s.len() - 1];
            // parse as seconds float → µs
            if let Ok(secs) = inner.parse::<f64>() {
                let us = (secs * 1_000_000.0).round() as u64;
                return (s[..start].trim(), Some(us));
            }
        }
    }
    (s, None)
}

fn extract_errno(result: &str) -> Option<String> {
    // "-1 ENOENT (No such...)" → "ENOENT"
    let parts: Vec<&str> = result.splitn(3, ' ').collect();
    if parts.len() >= 2
        && (parts[0] == "-1" || parts[0].starts_with("-1"))
        && parts[1].starts_with('E')
        && parts[1]
            .chars()
            .skip(1)
            .all(|c| c.is_uppercase() || c.is_ascii_digit())
    {
        return Some(parts[1].to_string());
    }
    None
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn load_calls(args: &Value) -> Result<Vec<StraceCall>, String> {
    let raw = if let Some(t) = args.get("text").and_then(|v| v.as_str()) {
        t.to_string()
    } else if let Some(p) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(p).map_err(|e| format!("Cannot read '{}': {}", p, e))?
    } else {
        return Err("Pass 'text' with strace output or 'file' with a file path.".to_string());
    };
    Ok(parse_strace(&raw))
}

fn filter_pid<'a>(calls: &'a [StraceCall], args: &Value) -> Vec<&'a StraceCall> {
    let pid_filter: Option<u32> = args.get("pid").and_then(|v| v.as_u64()).map(|n| n as u32);
    calls
        .iter()
        .filter(|c| match pid_filter {
            Some(p) => c.pid == Some(p),
            None => true,
        })
        .collect()
}

// ── actions ──────────────────────────────────────────────────────────────────

fn do_summary(args: &Value) -> Result<String, String> {
    let calls = load_calls(args)?;
    if calls.is_empty() {
        return Ok("No syscall lines found.".to_string());
    }

    let mut freq: HashMap<String, (u64, u64, u64)> = HashMap::new(); // count, errors, total_us
    for c in &calls {
        let e = freq.entry(c.syscall.clone()).or_insert((0, 0, 0));
        e.0 += 1;
        if c.is_error {
            e.1 += 1;
        }
        if let Some(us) = c.elapsed_us {
            e.2 += us;
        }
    }

    let mut sorted: Vec<(String, u64, u64, u64)> = freq
        .into_iter()
        .map(|(k, (cnt, err, us))| (k, cnt, err, us))
        .collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(30) as usize;
    let total: u64 = sorted.iter().map(|r| r.1).sum();
    let errors: u64 = sorted.iter().map(|r| r.2).sum();

    let mut out = String::new();
    out.push_str(&format!(
        "strace Summary  ({} calls, {} errors, {} unique syscalls)\n",
        total,
        errors,
        sorted.len()
    ));
    out.push_str(&"─".repeat(62));
    out.push('\n');

    let has_time = sorted.iter().any(|r| r.3 > 0);
    if has_time {
        out.push_str(&format!(
            "{:<24} {:>8}  {:>6}  {:>8}  {:>10}\n",
            "Syscall", "Count", "Error", "Time(ms)", "% Calls"
        ));
    } else {
        out.push_str(&format!(
            "{:<24} {:>8}  {:>6}  {:>8}\n",
            "Syscall", "Count", "Errors", "% Calls"
        ));
    }
    out.push_str(&"─".repeat(62));
    out.push('\n');

    for (name, cnt, err, us) in sorted.iter().take(limit) {
        let pct = if total > 0 {
            *cnt as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        if has_time {
            let ms = *us as f64 / 1000.0;
            out.push_str(&format!(
                "{:<24} {:>8}  {:>6}  {:>8.2}  {:>9.1}%\n",
                name, cnt, err, ms, pct
            ));
        } else {
            out.push_str(&format!(
                "{:<24} {:>8}  {:>6}  {:>8.1}%\n",
                name, cnt, err, pct
            ));
        }
    }
    if sorted.len() > limit {
        out.push_str(&format!("... and {} more syscalls\n", sorted.len() - limit));
    }

    Ok(out)
}

fn do_calls(args: &Value) -> Result<String, String> {
    let calls = load_calls(args)?;
    let filtered = filter_pid(&calls, args);

    let syscall_filter = args
        .get("syscall")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let matched: Vec<_> = filtered
        .iter()
        .filter(|c| match &syscall_filter {
            Some(sf) => c.syscall.to_lowercase().contains(sf),
            None => true,
        })
        .take(limit)
        .collect();

    if matched.is_empty() {
        return Ok("No matching syscalls.".to_string());
    }

    let mut out = String::new();
    out.push_str(&format!("Syscall trace ({} shown):\n", matched.len()));
    out.push_str(&"─".repeat(70));
    out.push('\n');

    for c in matched {
        let pid_str = c.pid.map(|p| format!("[{}] ", p)).unwrap_or_default();
        let args_preview = if c.args_raw.len() > 60 {
            format!("{}…", &c.args_raw[..60])
        } else {
            c.args_raw.clone()
        };
        let errno_str = c
            .errno
            .as_deref()
            .map(|e| format!(" ({})", e))
            .unwrap_or_default();
        out.push_str(&format!(
            "{}{:<20} ({}) = {}{}\n",
            pid_str,
            c.syscall,
            args_preview,
            c.result.split_whitespace().next().unwrap_or("?"),
            errno_str
        ));
    }

    Ok(out)
}

fn do_files(args: &Value) -> Result<String, String> {
    const FILE_SYSCALLS: &[&str] = &[
        "open",
        "openat",
        "openat2",
        "creat",
        "close",
        "read",
        "write",
        "pread64",
        "pwrite64",
        "stat",
        "lstat",
        "fstat",
        "access",
        "faccessat",
        "unlink",
        "unlinkat",
        "rename",
        "renameat",
        "mkdir",
        "rmdir",
        "link",
        "symlink",
        "readlink",
        "chmod",
        "chown",
        "truncate",
        "ftruncate",
        "mmap",
    ];

    let calls = load_calls(args)?;
    let filtered = filter_pid(&calls, args);
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let file_calls: Vec<_> = filtered
        .iter()
        .filter(|c| FILE_SYSCALLS.iter().any(|&s| c.syscall == s))
        .take(limit)
        .collect();

    if file_calls.is_empty() {
        return Ok("No file-related syscalls found.".to_string());
    }

    // Count by path
    let mut path_counts: HashMap<String, (u64, u64)> = HashMap::new(); // (calls, errors)
    let mut rows: Vec<(&str, &str, &str, bool)> = Vec::new(); // syscall, path, result, is_error

    for c in &file_calls {
        let path = extract_first_string(&c.args_raw);
        let result_short = c.result.split_whitespace().next().unwrap_or("?");
        rows.push((&c.syscall, path, result_short, c.is_error));
        let e = path_counts.entry(path.to_string()).or_insert((0, 0));
        e.0 += 1;
        if c.is_error {
            e.1 += 1;
        }
    }

    let mut out = String::new();
    out.push_str(&format!("File operations ({} shown):\n", rows.len()));
    out.push_str(&"─".repeat(70));
    out.push('\n');

    for (syscall, path, result, is_err) in &rows {
        let flag = if *is_err { "✗" } else { "✓" };
        out.push_str(&format!(
            "{} {:<16} = {}  {}\n",
            flag,
            syscall,
            result,
            if path.is_empty() {
                "(no path extracted)"
            } else {
                path
            }
        ));
    }

    // Top-accessed paths
    let mut top: Vec<_> = path_counts.iter().filter(|(p, _)| !p.is_empty()).collect();
    top.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    if !top.is_empty() {
        out.push('\n');
        out.push_str("Most accessed paths:\n");
        for (path, (cnt, err)) in top.iter().take(10) {
            out.push_str(&format!("  {}×  {} ({}err)\n", cnt, path, err));
        }
    }

    Ok(out)
}

fn do_network(args: &Value) -> Result<String, String> {
    const NET_SYSCALLS: &[&str] = &[
        "socket",
        "connect",
        "bind",
        "listen",
        "accept",
        "accept4",
        "send",
        "sendto",
        "sendmsg",
        "recv",
        "recvfrom",
        "recvmsg",
        "shutdown",
        "setsockopt",
        "getsockopt",
        "getpeername",
        "getsockname",
        "poll",
        "epoll_wait",
    ];

    let calls = load_calls(args)?;
    let filtered = filter_pid(&calls, args);
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let net_calls: Vec<_> = filtered
        .iter()
        .filter(|c| NET_SYSCALLS.iter().any(|&s| c.syscall == s))
        .take(limit)
        .collect();

    if net_calls.is_empty() {
        return Ok("No network-related syscalls found.".to_string());
    }

    let mut freq: HashMap<&str, u64> = HashMap::new();

    let mut out = String::new();
    out.push_str(&format!(
        "Network operations ({} shown):\n",
        net_calls.len()
    ));
    out.push_str(&"─".repeat(70));
    out.push('\n');

    for c in &net_calls {
        *freq.entry(c.syscall.as_str()).or_insert(0) += 1;
        let result_short = c.result.split_whitespace().next().unwrap_or("?");
        let errno_str = c
            .errno
            .as_deref()
            .map(|e| format!(" {}", e))
            .unwrap_or_default();

        // try to extract address from connect/bind args: "sa_family=AF_INET, sin_addr=..."
        let addr = extract_addr(&c.args_raw);
        let flag = if c.is_error { "✗" } else { "✓" };
        out.push_str(&format!(
            "{} {:<16} = {}{}  {}\n",
            flag, c.syscall, result_short, errno_str, addr
        ));
    }

    if !freq.is_empty() {
        out.push('\n');
        out.push_str("Breakdown:\n");
        let mut pairs: Vec<_> = freq.iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(a.1));
        for (name, cnt) in pairs {
            out.push_str(&format!("  {}×  {}\n", cnt, name));
        }
    }

    Ok(out)
}

fn do_errors(args: &Value) -> Result<String, String> {
    let calls = load_calls(args)?;
    let filtered = filter_pid(&calls, args);
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let errors: Vec<_> = filtered.iter().filter(|c| c.is_error).take(limit).collect();

    if errors.is_empty() {
        return Ok("No failed syscalls found.".to_string());
    }

    // group by errno
    let mut by_errno: HashMap<String, u64> = HashMap::new();
    for c in &errors {
        let key = c.errno.clone().unwrap_or_else(|| "UNKNOWN".to_string());
        *by_errno.entry(key).or_insert(0) += 1;
    }

    let mut out = String::new();
    out.push_str(&format!("Failed syscalls ({} shown):\n", errors.len()));
    out.push('\n');

    out.push_str("By errno:\n");
    let mut pairs: Vec<_> = by_errno.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1));
    for (errno, cnt) in &pairs {
        out.push_str(&format!("  {:>6}×  {}\n", cnt, errno));
    }
    out.push('\n');

    out.push_str(&"─".repeat(70));
    out.push('\n');
    for c in &errors {
        let pid_str = c.pid.map(|p| format!("[{}] ", p)).unwrap_or_default();
        let errno_str = c
            .errno
            .as_deref()
            .map(|e| format!(" {}", e))
            .unwrap_or_default();
        let args_preview = if c.args_raw.len() > 50 {
            format!("{}…", &c.args_raw[..50])
        } else {
            c.args_raw.clone()
        };
        out.push_str(&format!(
            "{}{:<18} ({}) = -1{}\n",
            pid_str, c.syscall, args_preview, errno_str
        ));
    }

    Ok(out)
}

// ── string extraction helpers ────────────────────────────────────────────────

/// Extract the first double-quoted string from strace args, like `"/etc/passwd"`.
fn extract_first_string(args: &str) -> &str {
    let bytes = args.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() {
                if bytes[j] == b'\\' {
                    j += 2;
                } else if bytes[j] == b'"' {
                    return &args[start..j];
                } else {
                    j += 1;
                }
            }
        }
        i += 1;
    }
    ""
}

/// Try to extract a human-readable address from a sockaddr struct in strace output.
fn extract_addr(args: &str) -> String {
    // look for sin_addr=inet_addr("1.2.3.4") or sun_path="/foo/bar.sock"
    if let Some(pos) = args.find("inet_addr(\"") {
        let rest = &args[pos + 11..];
        if let Some(end) = rest.find('"') {
            let ip = &rest[..end];
            // find port
            let port = args
                .find("sin_port=htons(")
                .and_then(|p| {
                    let r = &args[p + 15..];
                    r.find(')').map(|e| r[..e].to_string())
                })
                .unwrap_or_default();
            if !port.is_empty() {
                return format!("{}:{}", ip, port);
            }
            return ip.to_string();
        }
    }
    if let Some(pos) = args.find("sun_path=\"") {
        let rest = &args[pos + 10..];
        if let Some(end) = rest.find('"') {
            return rest[..end].to_string();
        }
    }
    if args.contains("sin6_addr=") {
        return "[IPv6 addr]".to_string();
    }
    String::new()
}

// ── entry point ──────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("summary");
    match action {
        "summary" => do_summary(args),
        "calls" => do_calls(args),
        "files" => do_files(args),
        "network" => do_network(args),
        "errors" => do_errors(args),
        _ => Err(format!(
            "Unknown action '{}'. Use: summary / calls / files / network / errors.",
            action
        )),
    }
}
