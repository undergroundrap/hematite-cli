// Redaction audit trail — metadata-only JSONL log.
//
// Written to ~/.hematite/redact_audit.jsonl on every MCP tool call when
// edge redaction is active. Never logs raw output, original values, or
// summaries — only call metadata and redaction statistics.
//
// Each line is a self-contained JSON object (JSONL format).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug)]
pub struct AuditEntry {
    pub topic: String,
    pub mode: RedactMode,
    pub tier1_hits: BTreeMap<String, usize>,
    pub semantic_applied: bool,
    pub input_chars: usize,
    pub output_chars: usize,
    pub caller_pid: u32,
}

#[derive(Debug)]
pub enum RedactMode {
    None,
    Regex,
    Semantic,
}

impl RedactMode {
    fn as_str(&self) -> &'static str {
        match self {
            RedactMode::None => "none",
            RedactMode::Regex => "regex",
            RedactMode::Semantic => "semantic",
        }
    }
}

/// Append one audit entry to ~/.hematite/redact_audit.jsonl.
/// Failures are logged to stderr and silently ignored — the audit trail
/// must never block the main request path.
pub fn record(entry: &AuditEntry) {
    if let Err(e) = try_record(entry) {
        eprintln!("[hematite mcp] audit log write failed: {e}");
    }
}

fn try_record(entry: &AuditEntry) -> std::io::Result<()> {
    let path = audit_log_path()?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let shrink_ratio = if entry.input_chars > 0 {
        entry.output_chars as f64 / entry.input_chars as f64
    } else {
        1.0
    };

    // Build tier1_hits as a plain object
    let tier1_obj: serde_json::Value = entry
        .tier1_hits
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::from(*v)))
        .collect::<serde_json::Map<_, _>>()
        .into();

    let line = serde_json::json!({
        "ts": chrono_now_utc(),
        "topic": entry.topic,
        "mode": entry.mode.as_str(),
        "tier1_hits": tier1_obj,
        "semantic_applied": entry.semantic_applied,
        "input_chars": entry.input_chars,
        "output_chars": entry.output_chars,
        "shrink_ratio": (shrink_ratio * 1000.0).round() / 1000.0,
        "caller_pid": entry.caller_pid,
        "suspicious_low_shrink": shrink_ratio > 0.9 && entry.mode.as_str() == "semantic",
    });

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    let mut json_str = serde_json::to_string(&line)?;
    json_str.push('\n');
    file.write_all(json_str.as_bytes())?;
    Ok(())
}

fn audit_log_path() -> std::io::Result<PathBuf> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "HOME directory not found")
        })?;
    Ok(home.join(".hematite").join("redact_audit.jsonl"))
}

fn chrono_now_utc() -> String {
    // Use std::time to avoid a chrono dep; format as ISO 8601 manually.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Convert epoch seconds to UTC date-time string (good enough for audit logs)
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    // Days since 1970-01-01
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days = [
        31u64,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ymd_known_dates() {
        // 2026-04-19: days since epoch
        // 2026 - 1970 = 56 years; quick sanity check
        let (y, _m, _d) = days_to_ymd(20563);
        assert_eq!(y, 2026);
    }

    #[test]
    fn chrono_now_utc_format() {
        let ts = chrono_now_utc();
        assert!(ts.len() == 20, "expected ISO 8601 format, got: {ts}");
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
    }
}
