use std::collections::{HashMap, HashSet};

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("render");
    match action {
        "render" => render_action(args),
        "list" => list_action(args),
        "validate" => validate_action(args),
        "preview" => preview_action(args),
        other => Err(format!(
            "template_tools: unknown action '{other}'. Valid: render, list, validate, preview"
        )),
    }
}

// ── Input helpers ─────────────────────────────────────────────────────────────

fn get_template(args: &serde_json::Value) -> Result<String, String> {
    if let Some(s) = args
        .get("template")
        .or_else(|| args.get("text"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
        return Ok(s.to_string());
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path)
            .map_err(|e| format!("template_tools: cannot read '{path}': {e}"));
    }
    Err("template_tools: 'template' (or 'text'/'file') is required".to_string())
}

fn get_vars(args: &serde_json::Value) -> HashMap<String, String> {
    args.get("vars")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| {
                    let val = if v.is_string() {
                        v.as_str().unwrap_or("").to_string()
                    } else {
                        v.to_string()
                    };
                    (k.clone(), val)
                })
                .collect()
        })
        .unwrap_or_default()
}

// ── Template engine ───────────────────────────────────────────────────────────

/// Extract all `{{VAR}}` and `{{VAR|default}}` placeholder names.
fn extract_placeholders(template: &str) -> Vec<(String, Option<String>)> {
    let mut result = Vec::new();
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'
            let mut inner = String::new();
            // Read until '}}'
            let mut closed = false;
            loop {
                match chars.next() {
                    Some('}') if chars.peek() == Some(&'}') => {
                        chars.next(); // consume second '}'
                        closed = true;
                        break;
                    }
                    Some(c) => inner.push(c),
                    None => break,
                }
            }
            if closed && !inner.trim().is_empty() {
                let inner = inner.trim();
                if let Some(pipe_pos) = inner.find('|') {
                    let var_name = inner[..pipe_pos].trim().to_string();
                    let default_val = inner[pipe_pos + 1..].trim().to_string();
                    if !var_name.is_empty() {
                        result.push((var_name, Some(default_val)));
                    }
                } else {
                    result.push((inner.to_string(), None));
                }
            }
        }
    }
    result
}

/// Render a template by substituting `{{VAR}}` and `{{VAR|default}}` placeholders.
fn render_template(
    template: &str,
    vars: &HashMap<String, String>,
    strict: bool,
) -> Result<(String, Vec<String>), Vec<String>> {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    let mut missing: Vec<String> = Vec::new();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'
            let mut inner = String::new();
            let mut closed = false;
            loop {
                match chars.next() {
                    Some('}') if chars.peek() == Some(&'}') => {
                        chars.next();
                        closed = true;
                        break;
                    }
                    Some(c) => inner.push(c),
                    None => break,
                }
            }
            if !closed {
                // Unclosed — emit as-is
                result.push_str("{{");
                result.push_str(&inner);
                continue;
            }
            let inner = inner.trim();
            if inner.is_empty() {
                result.push_str("{{}}");
                continue;
            }
            let (var_name, default_val) = if let Some(pipe_pos) = inner.find('|') {
                (inner[..pipe_pos].trim(), Some(inner[pipe_pos + 1..].trim()))
            } else {
                (inner, None)
            };

            if let Some(val) = vars.get(var_name) {
                result.push_str(val);
            } else if let Some(def) = default_val {
                result.push_str(def);
            } else {
                // Undefined with no default
                missing.push(var_name.to_string());
                if strict {
                    return Err(missing);
                }
                // Non-strict: leave placeholder as-is
                result.push_str("{{");
                result.push_str(inner);
                result.push_str("}}");
            }
        } else {
            result.push(ch);
        }
    }

    Ok((result, missing))
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn render_action(args: &serde_json::Value) -> Result<String, String> {
    let template = get_template(args)?;
    let vars = get_vars(args);
    let strict = args
        .get("strict")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match render_template(&template, &vars, strict) {
        Ok((rendered, missing)) => {
            let mut out = format!("TEMPLATE RENDER\n{}\n", "─".repeat(50));
            out.push_str(&format!("Variables provided : {}\n", vars.len()));
            if !missing.is_empty() {
                out.push_str(&format!(
                    "Undefined vars     : {} (left as placeholders)\n",
                    missing.join(", ")
                ));
            }
            out.push_str("\nResult:\n");
            out.push_str(&rendered);
            out.push('\n');
            Ok(out)
        }
        Err(missing) => Err(format!(
            "template_tools render: undefined variable(s): {} (use 'strict: false' to allow)",
            missing.join(", ")
        )),
    }
}

fn list_action(args: &serde_json::Value) -> Result<String, String> {
    let template = get_template(args)?;
    let placeholders = extract_placeholders(&template);

    // Deduplicate by name while preserving order
    let mut seen: HashSet<String> = HashSet::new();
    let mut unique: Vec<(String, Option<String>)> = Vec::new();
    for (name, default) in placeholders {
        if seen.insert(name.clone()) {
            unique.push((name, default));
        }
    }

    let mut out = format!("TEMPLATE LIST\n{}\n", "─".repeat(50));
    out.push_str(&format!("{} unique placeholder(s)\n\n", unique.len()));
    if unique.is_empty() {
        out.push_str("No {{VAR}} placeholders found.\n");
    } else {
        let name_w = unique
            .iter()
            .map(|(n, _)| n.len())
            .max()
            .unwrap_or(4)
            .max(4);
        out.push_str(&format!("{:<name_w$}  DEFAULT\n", "NAME"));
        out.push_str(&format!("{}\n", "-".repeat(name_w + 2 + 20)));
        for (name, default) in &unique {
            let def_str = default.as_deref().unwrap_or("(none)");
            out.push_str(&format!("{:<name_w$}  {def_str}\n", name));
        }
    }
    Ok(out)
}

fn validate_action(args: &serde_json::Value) -> Result<String, String> {
    let template = get_template(args)?;
    let vars = get_vars(args);
    let placeholders = extract_placeholders(&template);

    let mut issues: Vec<String> = Vec::new();

    // Check for unbalanced {{
    let open_count = template.match_indices("{{").count();
    let close_count = template.match_indices("}}").count();
    if open_count != close_count {
        issues.push(format!(
            "Unbalanced braces: {} opening '{{{{' vs {} closing '}}}}'",
            open_count, close_count
        ));
    }

    // Check for undefined vars (only if vars were provided)
    if !vars.is_empty() {
        let mut undefined: Vec<String> = Vec::new();
        for (name, default) in &placeholders {
            if default.is_none() && !vars.contains_key(name.as_str()) {
                undefined.push(name.clone());
            }
        }
        if !undefined.is_empty() {
            issues.push(format!(
                "Undefined variables (no default): {}",
                undefined.join(", ")
            ));
        }
    }

    // Check for empty placeholder names {{}}
    let empty_count = template.match_indices("{{}}").count();
    if empty_count > 0 {
        issues.push(format!(
            "{} empty placeholder(s) {{{{}}}} found",
            empty_count
        ));
    }

    let unique_names: HashSet<String> = placeholders.iter().map(|(n, _)| n.clone()).collect();
    let mut out = format!("TEMPLATE VALIDATE\n{}\n", "─".repeat(50));
    out.push_str(&format!(
        "Placeholders : {} total, {} unique\n",
        placeholders.len(),
        unique_names.len()
    ));
    if vars.is_empty() {
        out.push_str("Variables    : (none provided — skipping undefined check)\n");
    } else {
        out.push_str(&format!("Variables    : {} provided\n", vars.len()));
    }

    if issues.is_empty() {
        out.push_str("\nVALID — no issues found\n");
    } else {
        out.push_str(&format!("\nINVALID — {} issue(s):\n", issues.len()));
        for issue in &issues {
            out.push_str(&format!("  x {issue}\n"));
        }
    }
    Ok(out)
}

fn preview_action(args: &serde_json::Value) -> Result<String, String> {
    let template = get_template(args)?;
    let vars = get_vars(args);
    let placeholders = extract_placeholders(&template);

    let mut out = format!("TEMPLATE PREVIEW\n{}\n", "─".repeat(50));
    out.push_str(&format!(
        "{} placeholder(s), {} variable(s) provided\n\n",
        placeholders.len(),
        vars.len()
    ));

    // Show placeholder status table
    let name_w = placeholders
        .iter()
        .map(|(n, _)| n.len())
        .max()
        .unwrap_or(4)
        .max(4);
    out.push_str(&format!("{:<name_w$}  STATUS\n", "PLACEHOLDER"));
    out.push_str(&format!("{}\n", "-".repeat(name_w + 30)));
    let mut seen: HashSet<String> = HashSet::new();
    for (name, default) in &placeholders {
        if !seen.insert(name.clone()) {
            continue;
        }
        let status = if vars.contains_key(name.as_str()) {
            format!("DEFINED  → \"{}\"", vars[name.as_str()])
        } else if let Some(def) = default {
            format!("MISSING  → default \"{}\"", def)
        } else {
            "MISSING  (no default — will be left as placeholder)".to_string()
        };
        out.push_str(&format!("{:<name_w$}  {status}\n", name));
    }

    // Render preview with [MISSING] markers for undefined vars
    let preview_vars: HashMap<String, String> = {
        let mut m = vars.clone();
        for (name, default) in &placeholders {
            if !m.contains_key(name.as_str()) {
                let marker = match default {
                    Some(d) => d.clone(),
                    None => format!("[MISSING:{name}]"),
                };
                m.insert(name.clone(), marker);
            }
        }
        m
    };
    let (rendered, _) = render_template(&template, &preview_vars, false)
        .map_err(|e| format!("template_tools preview: {}", e.join(", ")))?;
    out.push_str("\nPreview:\n");
    out.push_str(&rendered);
    out.push('\n');
    Ok(out)
}
