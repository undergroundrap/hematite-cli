use serde_json::{json, Value};
use std::collections::HashMap;

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["parse", "units", "errors", "filter", "summary"],
                "description": "Action to perform (default: parse)"
            },
            "log": { "type": "string", "description": "journalctl -o json output as inline text" },
            "file": { "type": "string", "description": "Path to a saved journalctl JSON file" },
            "unit": { "type": "string", "description": "Filter by systemd unit name (filter action)" },
            "priority": {
                "type": "integer",
                "description": "Filter by max priority level 0–7 (0=EMERG, 3=ERR, 4=WARNING, 6=INFO)"
            },
            "limit": { "type": "integer", "description": "Max entries to show (default 40)" }
        }
    })
}

// ── priority helpers ──────────────────────────────────────────────────────────

const PRIORITY_NAMES: [&str; 8] = [
    "EMERG", "ALERT", "CRIT", "ERR", "WARNING", "NOTICE", "INFO", "DEBUG",
];

fn priority_label(p: u8) -> &'static str {
    PRIORITY_NAMES.get(p as usize).copied().unwrap_or("?")
}

fn priority_icon(p: u8) -> &'static str {
    match p {
        0 => "🔴",
        1 => "🔴",
        2 => "🔴",
        3 => "🟠",
        4 => "🟡",
        5 => "🔵",
        _ => "  ",
    }
}

// ── data model ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
struct JournalEntry {
    timestamp_us: u64,
    unit: String,
    comm: String,
    hostname: String,
    pid: u32,
    priority: u8,
    message: String,
}

fn format_ts(us: u64) -> String {
    if us == 0 {
        return "—".into();
    }
    let secs = us / 1_000_000;
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    // Date portion: days since unix epoch
    let days = secs / 86400;
    let (y, mo, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo, d, h, m, s)
}

// Simple Gregorian date from days since 1970-01-01
fn days_to_ymd(mut days: u64) -> (u32, u32, u32) {
    let mut y = 1970u32;
    loop {
        let ly = is_leap(y);
        let days_in_year = if ly { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        y += 1;
    }
    let months = [
        31u32,
        if is_leap(y) { 29 } else { 28 },
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
    let mut mo = 1u32;
    for &dim in &months {
        if days < dim as u64 {
            break;
        }
        days -= dim as u64;
        mo += 1;
    }
    (y, mo, days as u32 + 1)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

// ── parser ────────────────────────────────────────────────────────────────────

/// Parse journalctl -o json output (one JSON object per line = NDJSON).
fn parse_journal(text: &str) -> Vec<JournalEntry> {
    let mut entries: Vec<JournalEntry> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('{') {
            continue;
        }
        let obj: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Timestamp: __REALTIME_TIMESTAMP is microseconds since epoch as string
        let ts_us: u64 = obj
            .get("__REALTIME_TIMESTAMP")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .or_else(|| obj.get("__REALTIME_TIMESTAMP").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let unit = obj
            .get("_SYSTEMD_UNIT")
            .or_else(|| obj.get("SYSLOG_IDENTIFIER"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let comm = obj
            .get("_COMM")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let hostname = obj
            .get("_HOSTNAME")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let pid: u32 = obj
            .get("_PID")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .or_else(|| obj.get("_PID").and_then(|v| v.as_u64()).map(|n| n as u32))
            .unwrap_or(0);

        let priority: u8 = obj
            .get("PRIORITY")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .or_else(|| {
                obj.get("PRIORITY")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u8)
            })
            .unwrap_or(6); // default INFO

        // MESSAGE may be a string or an array of bytes
        let message = obj
            .get("MESSAGE")
            .map(|v| {
                if let Some(s) = v.as_str() {
                    s.to_string()
                } else if let Some(arr) = v.as_array() {
                    // byte array — try to decode as UTF-8
                    let bytes: Vec<u8> = arr
                        .iter()
                        .filter_map(|b| b.as_u64().map(|n| n as u8))
                        .collect();
                    String::from_utf8_lossy(&bytes).to_string()
                } else {
                    v.to_string()
                }
            })
            .unwrap_or_default();

        entries.push(JournalEntry {
            timestamp_us: ts_us,
            unit,
            comm,
            hostname,
            pid,
            priority,
            message,
        });
    }

    entries
}

// ── actions ───────────────────────────────────────────────────────────────────

fn unit_label(e: &JournalEntry) -> String {
    if !e.unit.is_empty() {
        e.unit.clone()
    } else if !e.comm.is_empty() {
        format!("[{}]", e.comm)
    } else {
        "kernel".into()
    }
}

fn action_parse(entries: &[JournalEntry], limit: usize) -> String {
    if entries.is_empty() {
        return "No journal entries found. Pass journalctl -o json output via 'log' or 'file'.\n\
                Example: journalctl -o json -n 500 > journal.json"
            .into();
    }
    let mut out = format!("─── journalctl — {} entries ───\n\n", entries.len());
    out.push_str(&format!(
        "{:<21} {:<6} {:<35} {}\n",
        "TIMESTAMP", "PRI", "UNIT", "MESSAGE"
    ));
    out.push_str(&"─".repeat(100));
    out.push('\n');
    for e in entries.iter().take(limit) {
        let ts = format_ts(e.timestamp_us);
        let label = unit_label(e);
        let unit_short = if label.len() > 33 {
            format!("{}..", &label[..31])
        } else {
            label
        };
        let msg = e.message.lines().next().unwrap_or("").trim();
        let msg_short = if msg.len() > 60 {
            format!("{}..", &msg[..58])
        } else {
            msg.to_string()
        };
        let pri = format!(
            "{} {}",
            priority_icon(e.priority),
            priority_label(e.priority)
        );
        out.push_str(&format!(
            "{:<21} {:<9} {:<35} {}\n",
            ts, pri, unit_short, msg_short
        ));
    }
    if entries.len() > limit {
        out.push_str(&format!(
            "\n… {} more (increase 'limit' to show more)\n",
            entries.len() - limit
        ));
    }
    out
}

fn action_units(entries: &[JournalEntry]) -> String {
    if entries.is_empty() {
        return "No journal entries found.".into();
    }
    let mut counts: HashMap<String, (u32, u8)> = HashMap::new();
    for e in entries {
        let label = unit_label(e);
        let entry = counts.entry(label).or_insert((0, 7));
        entry.0 += 1;
        if e.priority < entry.1 {
            entry.1 = e.priority; // track worst priority seen for this unit
        }
    }
    let mut ranked: Vec<(String, u32, u8)> =
        counts.into_iter().map(|(u, (n, p))| (u, n, p)).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));

    let total = entries.len() as f64;
    let mut out = format!(
        "─── Unit Frequency — {} entries, {} units ───\n\n",
        entries.len(),
        ranked.len()
    );
    out.push_str(&format!(
        "{:<4} {:<40} {:>7}  {:>5}  WORST  BAR\n",
        "#", "UNIT", "COUNT", "PCT"
    ));
    out.push_str(&"─".repeat(80));
    out.push('\n');
    for (i, (unit, n, worst_pri)) in ranked.iter().take(40).enumerate() {
        let pct = (*n as f64 / total * 100.0) as usize;
        let bar_len = pct.min(30);
        let bar = "█".repeat(bar_len);
        let unit_short = if unit.len() > 38 {
            format!("{}..", &unit[..36])
        } else {
            unit.clone()
        };
        out.push_str(&format!(
            "{:<4} {:<40} {:>7}  {:>4}%  {:<7}  {}\n",
            i + 1,
            unit_short,
            n,
            pct,
            priority_label(*worst_pri),
            bar,
        ));
    }
    out
}

fn action_errors(entries: &[JournalEntry], limit: usize) -> String {
    let errs: Vec<&JournalEntry> = entries.iter().filter(|e| e.priority <= 3).collect();
    if errs.is_empty() {
        return "No error-level entries found (priority ≤ 3: ERR/CRIT/ALERT/EMERG).\n\
                System appears healthy from this log sample."
            .into();
    }
    let mut out = format!(
        "─── Error/Critical Entries — {} of {} total ───\n\n",
        errs.len(),
        entries.len()
    );
    out.push_str(&format!(
        "{:<21} {:<6} {:<35} {}\n",
        "TIMESTAMP", "PRI", "UNIT", "MESSAGE"
    ));
    out.push_str(&"─".repeat(100));
    out.push('\n');
    for e in errs.iter().take(limit) {
        let ts = format_ts(e.timestamp_us);
        let label = unit_label(e);
        let unit_short = if label.len() > 33 {
            format!("{}..", &label[..31])
        } else {
            label
        };
        let msg = e.message.lines().next().unwrap_or("").trim();
        let msg_short = if msg.len() > 60 {
            format!("{}..", &msg[..58])
        } else {
            msg.to_string()
        };
        let pri = format!(
            "{} {}",
            priority_icon(e.priority),
            priority_label(e.priority)
        );
        out.push_str(&format!(
            "{:<21} {:<9} {:<35} {}\n",
            ts, pri, unit_short, msg_short
        ));
    }
    if errs.len() > limit {
        out.push_str(&format!("\n… {} more errors\n", errs.len() - limit));
    }
    out
}

fn action_filter(
    entries: &[JournalEntry],
    unit_filter: Option<&str>,
    pri_filter: Option<u8>,
    limit: usize,
) -> String {
    let filtered: Vec<&JournalEntry> = entries
        .iter()
        .filter(|e| {
            let unit_ok = unit_filter
                .map(|uf| unit_label(e).to_lowercase().contains(&uf.to_lowercase()))
                .unwrap_or(true);
            let pri_ok = pri_filter.map(|p| e.priority <= p).unwrap_or(true);
            unit_ok && pri_ok
        })
        .collect();

    if filtered.is_empty() {
        return format!(
            "No entries match filter (unit={:?}, priority≤{:?}).",
            unit_filter, pri_filter
        );
    }

    let mut out = format!(
        "─── Filtered — {} of {} entries ───\n\n",
        filtered.len(),
        entries.len()
    );
    out.push_str(&format!(
        "{:<21} {:<9} {:<35} {}\n",
        "TIMESTAMP", "PRI", "UNIT", "MESSAGE"
    ));
    out.push_str(&"─".repeat(100));
    out.push('\n');
    for e in filtered.iter().take(limit) {
        let ts = format_ts(e.timestamp_us);
        let label = unit_label(e);
        let unit_short = if label.len() > 33 {
            format!("{}..", &label[..31])
        } else {
            label
        };
        let msg = e.message.lines().next().unwrap_or("").trim();
        let msg_short = if msg.len() > 60 {
            format!("{}..", &msg[..58])
        } else {
            msg.to_string()
        };
        let pri = format!(
            "{} {}",
            priority_icon(e.priority),
            priority_label(e.priority)
        );
        out.push_str(&format!(
            "{:<21} {:<9} {:<35} {}\n",
            ts, pri, unit_short, msg_short
        ));
    }
    if filtered.len() > limit {
        out.push_str(&format!("\n… {} more\n", filtered.len() - limit));
    }
    out
}

fn action_summary(entries: &[JournalEntry]) -> String {
    if entries.is_empty() {
        return "No journal entries found.".into();
    }
    let mut pri_dist = [0u32; 8];
    for e in entries {
        let p = (e.priority as usize).min(7);
        pri_dist[p] += 1;
    }

    let hosts: std::collections::HashSet<&str> = entries
        .iter()
        .map(|e| e.hostname.as_str())
        .filter(|h| !h.is_empty())
        .collect();

    let first_ts = entries
        .iter()
        .map(|e| e.timestamp_us)
        .filter(|&t| t > 0)
        .min();
    let last_ts = entries
        .iter()
        .map(|e| e.timestamp_us)
        .filter(|&t| t > 0)
        .max();

    let mut unit_counts: HashMap<String, u32> = HashMap::new();
    for e in entries {
        *unit_counts.entry(unit_label(e)).or_default() += 1;
    }
    let mut top_units: Vec<(&str, u32)> =
        unit_counts.iter().map(|(u, &n)| (u.as_str(), n)).collect();
    top_units.sort_by(|a, b| b.1.cmp(&a.1));

    let mut out = "─── journald Summary ───\n\n".to_string();
    out.push_str(&format!("  Total entries : {}\n", entries.len()));
    if let Some(h) = hosts.iter().next() {
        out.push_str(&format!("  Host          : {}\n", h));
    }
    if let (Some(f), Some(l)) = (first_ts, last_ts) {
        out.push_str(&format!(
            "  Time range    : {} → {}\n",
            format_ts(f),
            format_ts(l)
        ));
    }

    out.push_str("\n  Priority Distribution:\n");
    for (p, &n) in pri_dist.iter().enumerate() {
        if n == 0 {
            continue;
        }
        let bar = "█".repeat(((n as usize * 20) / entries.len()).max(1));
        out.push_str(&format!(
            "    {} {:<8}  {:>5}  {}\n",
            priority_icon(p as u8),
            priority_label(p as u8),
            n,
            bar,
        ));
    }

    out.push_str("\n  Top Units:\n");
    for (u, n) in top_units.iter().take(10) {
        out.push_str(&format!("    {:<38} {}\n", u, n));
    }

    let error_count = pri_dist[..4].iter().sum::<u32>();
    if error_count > 0 {
        out.push_str(&format!(
            "\n  ⚠  {} error-level entries (use action='errors' to inspect)\n",
            error_count
        ));
    }
    out
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(40);
    let unit_filter = args.get("unit").and_then(|v| v.as_str());
    let pri_filter: Option<u8> = args
        .get("priority")
        .and_then(|v| v.as_u64())
        .map(|n| n.min(7) as u8);

    let raw = if let Some(t) = args.get("log").and_then(|v| v.as_str()) {
        t.to_string()
    } else if let Some(p) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(p).map_err(|e| format!("Cannot read '{}': {}", p, e))?
    } else {
        return Err(
            "Provide 'log' (inline journalctl JSON text) or 'file' (path to saved file).\n\
             Generate with: journalctl -o json -n 1000 > journal.json\n\
             Or filter:     journalctl -o json -u nginx.service --since '1 hour ago'"
                .into(),
        );
    };

    let entries = parse_journal(&raw);

    let result = match action {
        "parse" => action_parse(&entries, limit),
        "units" => action_units(&entries),
        "errors" => action_errors(&entries, limit),
        "filter" => action_filter(&entries, unit_filter, pri_filter, limit),
        "summary" => action_summary(&entries),
        other => {
            return Err(format!(
                "Unknown action '{}'. Choose: parse, units, errors, filter, summary.",
                other
            ))
        }
    };

    Ok(result)
}
