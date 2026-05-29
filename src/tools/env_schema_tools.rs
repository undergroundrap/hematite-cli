use serde_json::Value;
use std::collections::HashMap;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("validate");

    let example_text = get_file_or_inline(args, &["example", "example_text"], "example_file")?;
    let env_text = match action {
        "required" | "info"
            if args.get("env").is_none()
                && args.get("env_text").is_none()
                && args.get("env_file").is_none() =>
        {
            None
        }
        _ => Some(get_file_or_inline(args, &["env", "env_text"], "env_file")?),
    };

    match action {
        "validate" => {
            let env = env_text
                .ok_or("validate requires 'env' (.env content) in addition to 'example'")?;
            action_validate(&example_text, &env)
        }
        "diff" => {
            let env =
                env_text.ok_or("diff requires 'env' (.env content) in addition to 'example'")?;
            action_diff(&example_text, &env)
        }
        "required" => action_required(&example_text),
        "info" => {
            let env = env_text.as_deref().unwrap_or("");
            action_info(&example_text, env)
        }
        other => Err(format!(
            "Unknown action '{}'. Use: validate, diff, required, info.",
            other
        )),
    }
}

// ── Parsing ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct EnvEntry {
    key: String,
    value: String,
    comment: Option<String>,
    line: usize,
}

fn parse_env(text: &str) -> Vec<EnvEntry> {
    let mut entries = Vec::new();
    let mut pending_comment: Option<String> = None;

    for (idx, raw) in text.lines().enumerate() {
        let line = idx + 1;
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            pending_comment = None;
            continue;
        }

        if trimmed.starts_with('#') {
            let comment = trimmed.trim_start_matches('#').trim().to_string();
            if !comment.is_empty() {
                pending_comment = Some(comment);
            }
            continue;
        }

        if let Some(eq) = trimmed.find('=') {
            let key = trimmed[..eq].trim().to_string();
            let rest = &trimmed[eq + 1..];
            let value = strip_inline_comment(rest);
            if !key.is_empty() {
                entries.push(EnvEntry {
                    key,
                    value,
                    comment: pending_comment.take(),
                    line,
                });
                continue;
            }
        }
        pending_comment = None;
    }
    entries
}

fn strip_inline_comment(s: &str) -> String {
    let mut in_double = false;
    let mut in_single = false;
    let mut chars = s.chars().peekable();
    let mut result = String::new();

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_single => in_double = !in_double,
            '\'' if !in_double => in_single = !in_single,
            '#' if !in_double && !in_single => break,
            _ => {}
        }
        result.push(c);
    }
    unquote(result.trim())
}

fn unquote(s: &str) -> String {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn entries_to_map(entries: &[EnvEntry]) -> HashMap<String, &EnvEntry> {
    entries.iter().map(|e| (e.key.clone(), e)).collect()
}

/// Heuristic: a key is "required" in .env.example when its value is empty
/// or looks like a placeholder (CHANGE_ME, <…>, YOUR_…, xxx, etc.)
fn is_placeholder(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    let v = value.to_lowercase();
    v.starts_with('<')
        || v.starts_with("your_")
        || v.starts_with("your-")
        || v.starts_with("change")
        || v.starts_with("xxx")
        || v.starts_with("todo")
        || v.starts_with("replace")
        || v.starts_with("fill")
        || v.starts_with("insert")
        || v == "example"
        || v == "placeholder"
        || v == "required"
        || v == "secret"
        || v == "token"
        || v == "key"
        || v == "password"
        || v == "api_key"
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_validate(example_text: &str, env_text: &str) -> Result<String, String> {
    let example_entries = parse_env(example_text);
    let env_entries = parse_env(env_text);
    let example_map = entries_to_map(&example_entries);
    let env_map = entries_to_map(&env_entries);

    let mut findings: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut empty_required: Vec<String> = Vec::new();
    let mut extra: Vec<String> = Vec::new();

    for ex_entry in &example_entries {
        match env_map.get(&ex_entry.key) {
            None => {
                if is_placeholder(&ex_entry.value) {
                    missing.push(format!(
                        "  MISSING (required): {} — no default in .env.example",
                        ex_entry.key
                    ));
                } else {
                    missing.push(format!(
                        "  MISSING (optional): {} — .env.example default: {}",
                        ex_entry.key,
                        redact_if_secret(&ex_entry.key, &ex_entry.value)
                    ));
                }
            }
            Some(env_entry) => {
                if is_placeholder(&ex_entry.value) && env_entry.value.is_empty() {
                    empty_required.push(format!(
                        "  EMPTY (required): {} — must be set before deployment",
                        ex_entry.key
                    ));
                }
            }
        }
    }

    for env_entry in &env_entries {
        if !example_map.contains_key(&env_entry.key) {
            extra.push(format!("  EXTRA: {} (not in .env.example)", env_entry.key));
        }
    }

    if !missing.is_empty() {
        findings.push(format!("Missing keys ({}):", missing.len()));
        findings.extend(missing.clone());
    }
    if !empty_required.is_empty() {
        findings.push(format!(
            "\nEmpty required values ({}):",
            empty_required.len()
        ));
        findings.extend(empty_required.clone());
    }
    if !extra.is_empty() {
        findings.push(format!("\nExtra keys ({}):", extra.len()));
        findings.extend(extra);
    }

    let required_count = example_entries
        .iter()
        .filter(|e| is_placeholder(&e.value))
        .count();
    let env_required_set = example_entries
        .iter()
        .filter(|e| is_placeholder(&e.value))
        .filter(|e| {
            env_map
                .get(&e.key)
                .map(|ev| !ev.value.is_empty())
                .unwrap_or(false)
        })
        .count();

    let verdict = if findings.is_empty() {
        "VALID"
    } else {
        "INVALID"
    };
    let coverage = if required_count > 0 {
        format!("{}/{} required keys set", env_required_set, required_count)
    } else {
        format!("{} keys (all optional)", example_entries.len())
    };

    let mut out = format!(
        "env_schema_tools — validate\n\
         Status: {} | {} | .env.example: {} keys | .env: {} keys\n",
        verdict,
        coverage,
        example_entries.len(),
        env_entries.len()
    );

    if findings.is_empty() {
        out.push_str("\nAll required keys are present and non-empty. No extra keys.");
    } else {
        out.push('\n');
        out.push_str(&findings.join("\n"));
    }

    Ok(out)
}

fn action_diff(example_text: &str, env_text: &str) -> Result<String, String> {
    let example_entries = parse_env(example_text);
    let env_entries = parse_env(env_text);
    let env_map = entries_to_map(&env_entries);

    let absent: Vec<&EnvEntry> = example_entries
        .iter()
        .filter(|e| !env_map.contains_key(&e.key))
        .collect();

    if absent.is_empty() {
        return Ok(
            "env_schema_tools — diff\n\nAll .env.example keys are present in .env.".to_string(),
        );
    }

    let mut out = format!(
        "env_schema_tools — diff\n\
         {} key(s) in .env.example but absent from .env:\n\n",
        absent.len()
    );
    for entry in &absent {
        let req = if is_placeholder(&entry.value) {
            "[REQUIRED]"
        } else {
            "[optional]"
        };
        let desc = entry
            .comment
            .as_deref()
            .map(|c| format!(" — {}", c))
            .unwrap_or_default();
        out.push_str(&format!("  {} {}{}\n", req, entry.key, desc));
    }

    Ok(out)
}

fn action_required(example_text: &str) -> Result<String, String> {
    let example_entries = parse_env(example_text);

    let required: Vec<&EnvEntry> = example_entries
        .iter()
        .filter(|e| is_placeholder(&e.value))
        .collect();
    let optional: Vec<&EnvEntry> = example_entries
        .iter()
        .filter(|e| !is_placeholder(&e.value))
        .collect();

    let mut out = format!(
        "env_schema_tools — required\n\
         .env.example: {} total keys | {} required | {} optional\n",
        example_entries.len(),
        required.len(),
        optional.len()
    );

    if !required.is_empty() {
        out.push_str("\nRequired (must be set — no default placeholder):\n");
        for e in &required {
            let desc = e
                .comment
                .as_deref()
                .map(|c| format!(" — {}", c))
                .unwrap_or_default();
            out.push_str(&format!("  {}{}\n", e.key, desc));
        }
    }

    if !optional.is_empty() {
        out.push_str("\nOptional (has default placeholder in .env.example):\n");
        for e in &optional {
            let desc = e
                .comment
                .as_deref()
                .map(|c| format!(" — {}", c))
                .unwrap_or_default();
            out.push_str(&format!(
                "  {}={}{}\n",
                e.key,
                redact_if_secret(&e.key, &e.value),
                desc
            ));
        }
    }

    Ok(out)
}

fn action_info(example_text: &str, env_text: &str) -> Result<String, String> {
    let example_entries = parse_env(example_text);
    let env_entries = parse_env(env_text);
    let env_map = entries_to_map(&env_entries);

    let required_count = example_entries
        .iter()
        .filter(|e| is_placeholder(&e.value))
        .count();
    let optional_count = example_entries.len() - required_count;
    let present_count = example_entries
        .iter()
        .filter(|e| env_map.contains_key(&e.key))
        .count();
    let coverage_pct = if example_entries.is_empty() {
        0
    } else {
        present_count * 100 / example_entries.len()
    };
    let extra_count = env_entries
        .iter()
        .filter(|e| !example_entries.iter().any(|ex| ex.key == e.key))
        .count();

    let mut out = format!(
        "env_schema_tools — info\n\
         .env.example: {} keys ({} required, {} optional)\n\
         .env:         {} keys ({} present from schema, {} extra)\n\
         Coverage:     {}% ({}/{} schema keys present)\n",
        example_entries.len(),
        required_count,
        optional_count,
        env_entries.len(),
        present_count,
        extra_count,
        coverage_pct,
        present_count,
        example_entries.len()
    );

    let missing_required: Vec<&EnvEntry> = example_entries
        .iter()
        .filter(|e| is_placeholder(&e.value) && !env_map.contains_key(&e.key))
        .collect();
    if !missing_required.is_empty() {
        out.push_str(&format!(
            "\nMissing required keys ({}):\n",
            missing_required.len()
        ));
        for e in missing_required {
            out.push_str(&format!("  {}\n", e.key));
        }
    }

    Ok(out)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn redact_if_secret(key: &str, value: &str) -> String {
    let k = key.to_lowercase();
    if k.contains("key") || k.contains("secret") || k.contains("token") || k.contains("pass") {
        "[REDACTED]".to_string()
    } else {
        value.to_string()
    }
}

fn get_file_or_inline(
    args: &Value,
    inline_keys: &[&str],
    file_key: &str,
) -> Result<String, String> {
    for key in inline_keys {
        if let Some(v) = args.get(key).and_then(|v| v.as_str()) {
            return Ok(v.to_string());
        }
    }
    if let Some(path) = args.get(file_key).and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path).map_err(|e| format!("Cannot read '{}': {}", path, e));
    }
    Err(format!(
        "Provide '{}' (inline text) or '{}' (file path).",
        inline_keys[0], file_key
    ))
}
