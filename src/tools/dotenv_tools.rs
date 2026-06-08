use std::collections::{HashMap, HashSet};

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "parse" => parse_action(args),
        "validate" => validate_action(args),
        "get" => get_action(args),
        "list" => list_action(args),
        "to-json" => to_json_action(args),
        "to-shell" => to_shell_action(args),
        "merge" => merge_action(args),
        other => Err(format!(
            "dotenv_tools: unknown action '{other}'. Valid: parse, validate, get, list, to-json, to-shell, merge"
        )),
    }
}

// ── Types ─────────────────────────────────────────────────────────────────────

struct EnvVar {
    key: String,
    value: String,
    line: usize,
}

// ── I/O ───────────────────────────────────────────────────────────────────────

fn get_text(args: &serde_json::Value) -> Result<String, String> {
    if let Some(s) = args
        .get("text")
        .or_else(|| args.get("env"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
        return Ok(s.to_string());
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path)
            .map_err(|e| format!("dotenv_tools: cannot read '{path}': {e}"));
    }
    Err("dotenv_tools: 'text' (inline .env content) or 'file' (path) is required".to_string())
}

// ── Parser ────────────────────────────────────────────────────────────────────

fn parse_dotenv_text(text: &str) -> Vec<EnvVar> {
    let mut vars = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_string();
            if key.is_empty() {
                continue;
            }
            let raw_value = &trimmed[eq_pos + 1..];
            let value = parse_env_value(raw_value);
            vars.push(EnvVar {
                key,
                value,
                line: i + 1,
            });
        }
    }
    vars
}

fn find_closing_quote(s: &str, quote: char) -> Option<usize> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 1; // skip opening quote
    while i < chars.len() {
        if chars[i] == '\\' && quote == '"' {
            i += 2; // skip escape
        } else if chars[i] == quote {
            return Some(i);
        } else {
            i += 1;
        }
    }
    None
}

fn unescape_double(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('$') => result.push('$'),
                Some(c) => {
                    result.push('\\');
                    result.push(c);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn parse_env_value(raw: &str) -> String {
    let raw = raw.trim();
    // Double-quoted
    if raw.starts_with('"') {
        if let Some(end) = find_closing_quote(raw, '"') {
            return unescape_double(&raw[1..end]);
        }
        return raw.to_string(); // unbalanced — return as-is
    }
    // Single-quoted (no escape processing)
    if raw.starts_with('\'') {
        if let Some(end) = find_closing_quote(raw, '\'') {
            return raw[1..end].to_string();
        }
        return raw.to_string();
    }
    // Bare value — return as-is (most .env parsers don't strip inline # from bare values)
    raw.to_string()
}

fn is_valid_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    key.chars()
        .next()
        .map(|c| c.is_ascii_alphabetic() || c == '_')
        .unwrap_or(false)
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn quote_for_merge(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    let needs_quotes = value
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '#' | '\\' | '$'));
    if needs_quotes {
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        value.to_string()
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn parse_action(args: &serde_json::Value) -> Result<String, String> {
    let text = get_text(args)?;
    let vars = parse_dotenv_text(&text);
    let show_values = args
        .get("show_values")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut out = format!("DOTENV PARSE\n{}\n", "─".repeat(50));
    if vars.is_empty() {
        out.push_str("(no variables found)\n");
        return Ok(out);
    }
    out.push_str(&format!("{} variable(s)\n\n", vars.len()));

    let key_w = vars.iter().map(|v| v.key.len()).max().unwrap_or(3).max(3);
    out.push_str(&format!("{:<key_w$}  LINE  VALUE\n", "KEY"));
    out.push_str(&format!("{}\n", "-".repeat(key_w + 2 + 6 + 40)));

    for var in &vars {
        let display = if show_values {
            if var.value.len() > 60 {
                format!("{}...", &var.value[..57])
            } else {
                var.value.clone()
            }
        } else if var.value.len() <= 4 {
            "*".repeat(var.value.len())
        } else {
            format!("{}...", &var.value[..2])
        };
        out.push_str(&format!(
            "{:<key_w$}  {:4}  {}\n",
            var.key, var.line, display
        ));
    }
    Ok(out)
}

fn validate_action(args: &serde_json::Value) -> Result<String, String> {
    let text = get_text(args)?;
    let mut issues: Vec<String> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut total_vars = 0usize;

    for (i, line) in text.lines().enumerate() {
        let line_no = i + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim();
            let raw_value = &trimmed[eq_pos + 1..];
            total_vars += 1;

            if !is_valid_key(key) {
                issues.push(format!(
                    "Line {line_no}: invalid key '{key}' — use A-Z, a-z, 0-9, _ (must start with letter or _)"
                ));
            }

            if let Some(prev) = seen.get(key) {
                issues.push(format!(
                    "Line {line_no}: duplicate key '{key}' (first seen on line {prev})"
                ));
            } else {
                seen.insert(key.to_string(), line_no);
            }

            let rv = raw_value.trim();
            if rv.starts_with('"') && find_closing_quote(rv, '"').is_none() {
                issues.push(format!(
                    "Line {line_no}: unbalanced double quote for key '{key}'"
                ));
            }
            if rv.starts_with('\'') && find_closing_quote(rv, '\'').is_none() {
                issues.push(format!(
                    "Line {line_no}: unbalanced single quote for key '{key}'"
                ));
            }
            // Whitespace before = (key side)
            if trimmed[..eq_pos].ends_with(' ') {
                issues.push(format!(
                    "Line {line_no}: whitespace before '=' for key '{key}' (may cause issues in some shells)"
                ));
            }
        } else {
            issues.push(format!(
                "Line {line_no}: missing '=' — '{}'",
                trimmed.chars().take(40).collect::<String>()
            ));
        }
    }

    let mut out = format!("DOTENV VALIDATE\n{}\n", "─".repeat(50));
    if issues.is_empty() {
        out.push_str(&format!("VALID — {total_vars} key(s) checked, no issues\n"));
    } else {
        out.push_str(&format!(
            "INVALID — {} issue(s) in {total_vars} variable(s)\n\n",
            issues.len()
        ));
        for issue in &issues {
            out.push_str(&format!("  x {issue}\n"));
        }
    }
    Ok(out)
}

fn get_action(args: &serde_json::Value) -> Result<String, String> {
    let text = get_text(args)?;
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("dotenv_tools get: 'key' is required")?;

    let vars = parse_dotenv_text(&text);
    let matches: Vec<&EnvVar> = vars.iter().filter(|v| v.key == key).collect();

    let mut out = format!("DOTENV GET\n{}\n", "─".repeat(50));
    out.push_str(&format!("Key   : {key}\n"));
    if matches.is_empty() {
        out.push_str("Value : (not found)\n");
    } else {
        let var = matches.last().unwrap();
        out.push_str(&format!("Value : {}\n", var.value));
        out.push_str(&format!("Line  : {}\n", var.line));
        if matches.len() > 1 {
            out.push_str(&format!(
                "Note  : defined {} times — last value used\n",
                matches.len()
            ));
        }
    }
    Ok(out)
}

fn list_action(args: &serde_json::Value) -> Result<String, String> {
    let text = get_text(args)?;
    let vars = parse_dotenv_text(&text);

    let mut out = format!("DOTENV LIST\n{}\n", "─".repeat(50));
    out.push_str(&format!("{} variable(s)\n\n", vars.len()));
    for (i, var) in vars.iter().enumerate() {
        out.push_str(&format!("{:3}. {}\n", i + 1, var.key));
    }
    Ok(out)
}

fn to_json_action(args: &serde_json::Value) -> Result<String, String> {
    let text = get_text(args)?;
    let vars = parse_dotenv_text(&text);

    let mut map = serde_json::Map::new();
    for var in &vars {
        map.insert(
            var.key.clone(),
            serde_json::Value::String(var.value.clone()),
        );
    }
    let json = serde_json::to_string_pretty(&serde_json::Value::Object(map))
        .map_err(|e| format!("dotenv_tools to-json: {e}"))?;

    let mut out = format!("DOTENV TO-JSON\n{}\n", "─".repeat(50));
    out.push_str(&json);
    out.push('\n');
    Ok(out)
}

fn to_shell_action(args: &serde_json::Value) -> Result<String, String> {
    let text = get_text(args)?;
    let vars = parse_dotenv_text(&text);
    let shell = args.get("shell").and_then(|v| v.as_str()).unwrap_or("bash");

    let mut out = format!("DOTENV TO-SHELL\n{}\n", "─".repeat(50));
    for var in &vars {
        let line = match shell.to_lowercase().as_str() {
            "powershell" | "ps" | "pwsh" => {
                let escaped = var.value.replace('\\', "\\\\").replace('"', "\\\"");
                format!("$env:{}=\"{}\"\n", var.key, escaped)
            }
            "cmd" => format!("SET {}={}\n", var.key, var.value),
            _ => {
                // bash/sh
                let escaped = var.value.replace('\\', "\\\\").replace('"', "\\\"");
                format!("export {}=\"{}\"\n", var.key, escaped)
            }
        };
        out.push_str(&line);
    }
    Ok(out)
}

fn merge_action(args: &serde_json::Value) -> Result<String, String> {
    let base_text = args
        .get("base")
        .and_then(|v| v.as_str())
        .ok_or("dotenv_tools merge: 'base' (.env content) is required")?;
    let overlay_text = args
        .get("overlay")
        .and_then(|v| v.as_str())
        .ok_or("dotenv_tools merge: 'overlay' (.env content) is required")?;

    let base_vars = parse_dotenv_text(base_text);
    let overlay_vars = parse_dotenv_text(overlay_text);

    let overlay_map: HashMap<String, &str> = overlay_vars
        .iter()
        .map(|v| (v.key.clone(), v.value.as_str()))
        .collect();

    let mut seen: HashSet<String> = HashSet::new();
    let mut result_lines: Vec<String> = Vec::new();
    let mut changed: Vec<String> = Vec::new();
    let mut added: Vec<String> = Vec::new();

    for var in &base_vars {
        seen.insert(var.key.clone());
        if let Some(&new_val) = overlay_map.get(&var.key) {
            if new_val != var.value {
                changed.push(var.key.clone());
            }
            result_lines.push(format!("{}={}", var.key, quote_for_merge(new_val)));
        } else {
            result_lines.push(format!("{}={}", var.key, quote_for_merge(&var.value)));
        }
    }

    for var in &overlay_vars {
        if !seen.contains(&var.key) {
            result_lines.push(format!("{}={}", var.key, quote_for_merge(&var.value)));
            added.push(var.key.clone());
        }
    }

    let merged = result_lines.join("\n");
    let mut out = format!("DOTENV MERGE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Base variables : {}\n", base_vars.len()));
    out.push_str(&format!("Overlay vars   : {}\n", overlay_vars.len()));
    if !changed.is_empty() {
        out.push_str(&format!("Changed        : {}\n", changed.join(", ")));
    }
    if !added.is_empty() {
        out.push_str(&format!("Added          : {}\n", added.join(", ")));
    }
    out.push_str(&format!(
        "Result vars    : {}\n\n",
        base_vars.len() + added.len()
    ));
    out.push_str("Merged .env:\n");
    out.push_str(&merged);
    out.push('\n');
    Ok(out)
}
