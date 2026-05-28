use serde_json::{Map, Value};

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("format").is_some() {
        "parse".to_string()
    } else {
        "parse".to_string()
    };
    match action.as_str() {
        "parse" => parse_action(args),
        "detect" => detect_action(args),
        "filter" => filter_action(args),
        "stats" => stats_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: parse, detect, filter, stats",
            action
        )),
    }
}

fn get_input(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("log"))
        .or_else(|| args.get("lines"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' argument — pass log lines as a string".to_string())
}

#[derive(Debug, Clone, PartialEq)]
enum LogFormat {
    JsonLines,
    KeyValue,
    ApacheCommon,
    ApacheCombined,
    Nginx,
    Syslog,
    Unknown,
}

fn detect_format(line: &str) -> LogFormat {
    let trimmed = line.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return LogFormat::JsonLines;
    }
    // Apache Combined: 1.2.3.4 - user [10/Oct/2000:13:55:36 -0700] "GET / HTTP/1.1" 200 2326 "ref" "ua"
    if is_apache_combined(trimmed) {
        return LogFormat::ApacheCombined;
    }
    // Apache Common: same but no referer/ua fields
    if is_apache_common(trimmed) {
        return LogFormat::ApacheCommon;
    }
    // Syslog: Nov 10 01:02:03 hostname process[pid]: message
    if looks_like_syslog(trimmed) {
        return LogFormat::Syslog;
    }
    // key=value / key="value" pairs
    if trimmed.contains('=') && !trimmed.starts_with('<') {
        let pairs: usize = trimmed
            .split_whitespace()
            .filter(|t| t.contains('='))
            .count();
        if pairs >= 2 {
            return LogFormat::KeyValue;
        }
    }
    LogFormat::Unknown
}

fn is_apache_combined(line: &str) -> bool {
    // Quick heuristic: IP - ident [date] "method" status bytes "ref" "ua"
    let parts: Vec<&str> = line.splitn(10, ' ').collect();
    parts.len() >= 9
        && parts[1] == "-"
        && parts[3].starts_with('[')
        && parts[4].starts_with('"')
        && parts[6].chars().all(|c| c.is_ascii_digit() || c == '-')
        && parts[8].starts_with('"')
}

fn is_apache_common(line: &str) -> bool {
    let parts: Vec<&str> = line.splitn(8, ' ').collect();
    parts.len() >= 7
        && parts[1] == "-"
        && parts[3].starts_with('[')
        && parts[4].starts_with('"')
        && parts[6].chars().all(|c| c.is_ascii_digit() || c == '-')
}

fn looks_like_syslog(line: &str) -> bool {
    // "Nov 10 01:02:03" or "2025-01-02T03:04:05" at start
    let parts: Vec<&str> = line.splitn(4, ' ').collect();
    if parts.len() >= 3 && parts[2].contains(':') {
        return true;
    }
    line.len() > 20 && line.chars().nth(4) == Some('-') && line.chars().nth(7) == Some('-')
}

fn parse_json_line(line: &str) -> Option<Vec<(String, String)>> {
    let v: Value = serde_json::from_str(line).ok()?;
    let obj = v.as_object()?;
    let pairs: Vec<(String, String)> = obj
        .iter()
        .map(|(k, v)| {
            let val = match v {
                Value::String(s) => s.clone(),
                Value::Null => "null".to_string(),
                other => other.to_string(),
            };
            (k.clone(), val)
        })
        .collect();
    Some(pairs)
}

fn parse_kv_line(line: &str) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut remaining = line.trim();
    while !remaining.is_empty() {
        if let Some(eq_pos) = remaining.find('=') {
            let key = remaining[..eq_pos]
                .trim()
                .trim_end_matches(|c: char| c == ' ');
            // handle key with spaces (take last word before =)
            let key = key.split_whitespace().last().unwrap_or(key);
            remaining = &remaining[eq_pos + 1..];
            let (val, rest) = if remaining.starts_with('"') {
                parse_quoted_value(&remaining[1..])
            } else {
                let end = remaining.find(' ').unwrap_or(remaining.len());
                (&remaining[..end], &remaining[end..])
            };
            pairs.push((key.to_string(), val.to_string()));
            remaining = rest.trim_start();
        } else {
            break;
        }
    }
    pairs
}

fn parse_quoted_value(s: &str) -> (&str, &str) {
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            return (&s[..i], &s[i + 1..]);
        }
    }
    (s, "")
}

fn parse_apache_line(line: &str) -> Vec<(String, String)> {
    // "1.2.3.4 - ident [10/Oct/2000:13:55:36 -0700] \"GET /path HTTP/1.1\" 200 2326"
    let mut fields: Vec<(String, String)> = Vec::new();
    let parts: Vec<&str> = line.splitn(8, ' ').collect();
    if parts.is_empty() {
        return fields;
    }
    fields.push(("client".to_string(), parts[0].to_string()));
    if parts.len() > 2 {
        fields.push(("ident".to_string(), parts[2].to_string()));
    }
    // extract [date]
    if let Some(d_start) = line.find('[') {
        if let Some(d_end) = line.find(']') {
            fields.push(("time".to_string(), line[d_start + 1..d_end].to_string()));
            let after_date = &line[d_end + 1..].trim_start();
            // extract "METHOD path PROTO"
            if after_date.starts_with('"') {
                let (req, rest) = parse_quoted_value(&after_date[1..]);
                {
                    let req_parts: Vec<&str> = req.splitn(3, ' ').collect();
                    if req_parts.len() >= 1 {
                        fields.push(("method".to_string(), req_parts[0].to_string()));
                    }
                    if req_parts.len() >= 2 {
                        fields.push(("path".to_string(), req_parts[1].to_string()));
                    }
                    if req_parts.len() >= 3 {
                        fields.push(("protocol".to_string(), req_parts[2].to_string()));
                    }
                    let status_part = rest.trim();
                    let status_fields: Vec<&str> = status_part.split_whitespace().collect();
                    if !status_fields.is_empty() {
                        fields.push(("status".to_string(), status_fields[0].to_string()));
                    }
                    if status_fields.len() > 1 {
                        fields.push(("bytes".to_string(), status_fields[1].to_string()));
                    }
                }
            }
        }
    }
    fields
}

fn parse_syslog_line(line: &str) -> Vec<(String, String)> {
    let mut fields: Vec<(String, String)> = Vec::new();
    // Try ISO 8601 first: 2025-01-02T03:04:05Z hostname proc[pid]: msg
    if line.len() > 20 && line.chars().nth(4) == Some('-') {
        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        if parts.len() >= 4 {
            fields.push(("timestamp".to_string(), parts[0].to_string()));
            fields.push(("host".to_string(), parts[1].to_string()));
            let proc_msg = parts[2];
            if let Some(colon) = proc_msg.find(':') {
                fields.push(("process".to_string(), proc_msg[..colon].to_string()));
                fields.push(("message".to_string(), parts[3].to_string()));
            } else {
                fields.push(("process".to_string(), proc_msg.to_string()));
                fields.push(("message".to_string(), parts[3].to_string()));
            }
            return fields;
        }
    }
    // Traditional: "Nov 10 01:02:03 hostname proc[pid]: msg"
    let parts: Vec<&str> = line.splitn(6, ' ').collect();
    if parts.len() >= 6 {
        let ts = format!("{} {} {}", parts[0], parts[1], parts[2]);
        fields.push(("timestamp".to_string(), ts));
        fields.push(("host".to_string(), parts[3].to_string()));
        let proc_msg = parts[4];
        if let Some(colon) = proc_msg.find(':') {
            fields.push(("process".to_string(), proc_msg[..colon].to_string()));
        } else {
            fields.push(("process".to_string(), proc_msg.to_string()));
        }
        fields.push(("message".to_string(), parts[5].to_string()));
    } else {
        fields.push(("raw".to_string(), line.to_string()));
    }
    fields
}

fn format_fields(fields: &[(String, String)]) -> String {
    let max_key = fields.iter().map(|(k, _)| k.len()).max().unwrap_or(8);
    fields
        .iter()
        .map(|(k, v)| format!("  {:<width$}  {}", k, v, width = max_key))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_action(args: &Value) -> Result<String, String> {
    let text = get_input(args)?;
    let fmt_override = args
        .get("format")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());
    let max_lines = args.get("max").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let sample = lines.first().unwrap_or(&"");
    let detected = match fmt_override.as_deref() {
        Some("json") | Some("jsonl") => LogFormat::JsonLines,
        Some("kv") | Some("keyvalue") | Some("key_value") => LogFormat::KeyValue,
        Some("apache") | Some("common") => LogFormat::ApacheCommon,
        Some("combined") => LogFormat::ApacheCombined,
        Some("syslog") => LogFormat::Syslog,
        _ => detect_format(sample),
    };
    let fmt_name = match &detected {
        LogFormat::JsonLines => "JSON Lines",
        LogFormat::KeyValue => "key=value",
        LogFormat::ApacheCommon => "Apache Common Log",
        LogFormat::ApacheCombined => "Apache Combined Log",
        LogFormat::Nginx => "Nginx",
        LogFormat::Syslog => "Syslog",
        LogFormat::Unknown => "Unknown (showing raw)",
    };

    let mut out = format!(
        "Log Parse  [{} — {} lines]\n{}\n\n",
        fmt_name,
        lines.len(),
        "=".repeat(44)
    );

    for (idx, line) in lines.iter().enumerate().take(max_lines) {
        out += &format!("Line {}\n", idx + 1);
        let fields: Vec<(String, String)> = match &detected {
            LogFormat::JsonLines => {
                if let Some(f) = parse_json_line(line) {
                    f
                } else {
                    vec![("raw".to_string(), line.to_string())]
                }
            }
            LogFormat::KeyValue => parse_kv_line(line),
            LogFormat::ApacheCommon | LogFormat::ApacheCombined | LogFormat::Nginx => {
                parse_apache_line(line)
            }
            LogFormat::Syslog => parse_syslog_line(line),
            LogFormat::Unknown => vec![("raw".to_string(), line.to_string())],
        };
        out += &format_fields(&fields);
        out += "\n\n";
    }

    if lines.len() > max_lines {
        out += &format!(
            "... {} more lines (pass 'max' to increase limit)\n",
            lines.len() - max_lines
        );
    }
    Ok(out)
}

fn detect_action(args: &Value) -> Result<String, String> {
    let text = get_input(args)?;
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut counts: std::collections::HashMap<&'static str, usize> =
        std::collections::HashMap::new();
    for line in &lines {
        let fmt = detect_format(line);
        let label = match fmt {
            LogFormat::JsonLines => "JSON Lines",
            LogFormat::KeyValue => "key=value",
            LogFormat::ApacheCommon => "Apache Common",
            LogFormat::ApacheCombined => "Apache Combined",
            LogFormat::Nginx => "Nginx",
            LogFormat::Syslog => "Syslog",
            LogFormat::Unknown => "Unknown",
        };
        *counts.entry(label).or_insert(0) += 1;
    }
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    let best = sorted.first().map(|(k, _)| **k).unwrap_or("Unknown");
    let mut out = format!("Log Format Detection\n{}\n\n", "=".repeat(44));
    out += &format!("Detected: {}\n\n", best);
    out += &format!("Distribution ({} lines):\n", lines.len());
    for (label, count) in &sorted {
        let pct = *count * 100 / lines.len().max(1);
        out += &format!("  {:<20} {} lines ({:.0}%)\n", label, count, pct);
    }
    Ok(out)
}

fn filter_action(args: &Value) -> Result<String, String> {
    let text = get_input(args)?;
    let field = args
        .get("field")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'field' — field name to filter on (e.g. 'status', 'level')")?;
    let value = args
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'value' — value to match")?;
    let fmt_override = args
        .get("format")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let sample = lines.first().unwrap_or(&"");
    let detected = match fmt_override.as_deref() {
        Some("json") | Some("jsonl") => LogFormat::JsonLines,
        Some("kv") | Some("keyvalue") => LogFormat::KeyValue,
        Some("apache") | Some("common") => LogFormat::ApacheCommon,
        Some("combined") => LogFormat::ApacheCombined,
        Some("syslog") => LogFormat::Syslog,
        _ => detect_format(sample),
    };

    let mut matched: Vec<&str> = Vec::new();
    for line in &lines {
        let fields: Vec<(String, String)> = match &detected {
            LogFormat::JsonLines => {
                if let Some(f) = parse_json_line(line) {
                    f
                } else {
                    vec![]
                }
            }
            LogFormat::KeyValue => parse_kv_line(line),
            LogFormat::ApacheCommon | LogFormat::ApacheCombined | LogFormat::Nginx => {
                parse_apache_line(line)
            }
            LogFormat::Syslog => parse_syslog_line(line),
            LogFormat::Unknown => vec![],
        };
        let field_lower = field.to_lowercase();
        let value_lower = value.to_lowercase();
        if fields.iter().any(|(k, v)| {
            k.to_lowercase() == field_lower && v.to_lowercase().contains(&value_lower)
        }) {
            matched.push(line);
        }
    }

    let mut out = format!("Log Filter: {}={}\n{}\n\n", field, value, "=".repeat(44));
    out += &format!("{} of {} lines matched\n\n", matched.len(), lines.len());
    for line in &matched {
        out += line;
        out += "\n";
    }
    Ok(out)
}

fn stats_action(args: &Value) -> Result<String, String> {
    let text = get_input(args)?;
    let fmt_override = args
        .get("format")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let sample = lines.first().unwrap_or(&"");
    let detected = match fmt_override.as_deref() {
        Some("json") | Some("jsonl") => LogFormat::JsonLines,
        Some("kv") | Some("keyvalue") => LogFormat::KeyValue,
        Some("apache") | Some("common") => LogFormat::ApacheCommon,
        Some("combined") => LogFormat::ApacheCombined,
        Some("syslog") => LogFormat::Syslog,
        _ => detect_format(sample),
    };

    let field = args
        .get("field")
        .or_else(|| args.get("key"))
        .and_then(|v| v.as_str());

    // Determine the field to aggregate
    let agg_field = field.unwrap_or_else(|| match &detected {
        LogFormat::ApacheCommon | LogFormat::ApacheCombined | LogFormat::Nginx => "status",
        LogFormat::Syslog => "process",
        _ => "level",
    });

    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total = 0usize;
    for line in &lines {
        let fields: Vec<(String, String)> = match &detected {
            LogFormat::JsonLines => {
                if let Some(f) = parse_json_line(line) {
                    f
                } else {
                    vec![]
                }
            }
            LogFormat::KeyValue => parse_kv_line(line),
            LogFormat::ApacheCommon | LogFormat::ApacheCombined | LogFormat::Nginx => {
                parse_apache_line(line)
            }
            LogFormat::Syslog => parse_syslog_line(line),
            LogFormat::Unknown => vec![],
        };
        let agg_lower = agg_field.to_lowercase();
        if let Some((_, v)) = fields.iter().find(|(k, _)| k.to_lowercase() == agg_lower) {
            *counts.entry(v.clone()).or_insert(0) += 1;
            total += 1;
        }
    }

    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    let mut out = format!("Log Stats: '{}' field\n{}\n\n", agg_field, "=".repeat(44));
    out += &format!("Total lines: {}  Matched: {}\n\n", lines.len(), total);
    out += &format!("{:<30} {:>8}  {}\n", "Value", "Count", "Share");
    out += &format!("{}\n", "-".repeat(52));
    for (val, count) in sorted {
        let pct = *count as f64 / total.max(1) as f64 * 100.0;
        let short = if val.len() > 28 {
            format!("{}...", &val[..25])
        } else {
            val.to_string()
        };
        out += &format!("{:<30} {:>8}  {:.1}%\n", short, count, pct);
    }
    Ok(out)
}

// Expose parse_kv_line for tests
pub fn parse_kv(line: &str) -> Map<String, Value> {
    let pairs = parse_kv_line(line);
    pairs
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect()
}
