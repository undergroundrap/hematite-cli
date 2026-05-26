use regex::Regex;
use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("test");

    match action {
        "test" => test_pattern(args),
        "extract" => extract_matches(args),
        "replace" => replace_matches(args),
        "split" => split_on_pattern(args),
        "explain" => explain_pattern(args),
        "named-groups" => named_groups(args),
        other => Err(format!(
            "regex_tools: unknown action '{other}'. Valid: test, extract, replace, split, explain, named-groups"
        )),
    }
}

fn get_pattern(args: &Value) -> Result<(Regex, bool), String> {
    let pattern_str = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or("regex_tools: 'pattern' is required")?;

    let case_insensitive = args
        .get("case_insensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let multiline = args
        .get("multiline")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let dot_all = args
        .get("dot_all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut builder = regex::RegexBuilder::new(pattern_str);
    builder.case_insensitive(case_insensitive);
    builder.multi_line(multiline);
    builder.dot_matches_new_line(dot_all);

    let re = builder
        .build()
        .map_err(|e| format!("regex_tools: invalid pattern '{pattern_str}': {e}"))?;

    Ok((re, case_insensitive))
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "regex_tools: 'text' is required".into())
}

fn test_pattern(args: &Value) -> Result<String, String> {
    let (re, _case_insensitive) = get_pattern(args)?;

    // Accept multiple test strings as array or single string
    let texts: Vec<String> = if let Some(arr) = args.get("texts").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else {
        vec![get_text(args)?]
    };

    let pattern_str = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    let flags = build_flags_display(args);

    let mut out = format!(
        "REGEX TEST\nPattern  : /{pattern_str}/{flags}\n{}\n\n",
        "─".repeat(60)
    );

    let mut match_count = 0usize;
    let mut no_match_count = 0usize;

    for text in &texts {
        let is_match = re.is_match(text);
        if is_match {
            match_count += 1;
        } else {
            no_match_count += 1;
        }

        let display = if text.len() > 80 {
            format!("{}...", &text[..80])
        } else {
            text.clone()
        };

        let status = if is_match {
            "✓ MATCH"
        } else {
            "✗ NO MATCH"
        };
        out.push_str(&format!("{status}  \"{display}\"\n"));

        if is_match {
            // Show what matched
            for (i, m) in re.find_iter(text).enumerate().take(3) {
                out.push_str(&format!(
                    "         match[{i}]: {:?} at {}..{}\n",
                    m.as_str(),
                    m.start(),
                    m.end()
                ));
            }
            let total = re.find_iter(text).count();
            if total > 3 {
                out.push_str(&format!("         ... and {} more match(es)\n", total - 3));
            }
        }
    }

    out.push_str(&format!("\nSummary: {match_count} match(es), {no_match_count} no-match(es) out of {} test string(s)", texts.len()));

    Ok(out)
}

fn extract_matches(args: &Value) -> Result<String, String> {
    let (re, _) = get_pattern(args)?;
    let text = get_text(args)?;
    let pattern_str = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    let flags = build_flags_display(args);

    let has_groups = re.captures_len() > 1;
    let mut out = format!(
        "REGEX EXTRACT: /{pattern_str}/{flags}\n{}\n\n",
        "─".repeat(60)
    );

    if has_groups {
        let group_names: Vec<Option<&str>> = re.capture_names().collect();
        for (i, caps) in re.captures_iter(&text).enumerate().take(50) {
            out.push_str(&format!("Match {}:\n", i + 1));
            out.push_str(&format!("  full: {:?}\n", caps.get(0).map(|m| m.as_str())));
            for (g, name) in group_names.iter().enumerate().skip(1) {
                if let Some(m) = caps.get(g) {
                    let label = name
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| format!("group {g}"));
                    out.push_str(&format!("  {label}: {:?}\n", m.as_str()));
                }
            }
        }
    } else {
        let matches: Vec<&str> = re.find_iter(&text).map(|m| m.as_str()).collect();
        if matches.is_empty() {
            out.push_str("No matches found.\n");
        } else {
            for (i, m) in matches.iter().enumerate().take(50) {
                out.push_str(&format!("  [{i}] {m:?}\n"));
            }
            if matches.len() > 50 {
                out.push_str(&format!(
                    "  ... ({} total matches, showing first 50)\n",
                    matches.len()
                ));
            } else {
                out.push_str(&format!("\nTotal: {} match(es)\n", matches.len()));
            }
        }
    }

    Ok(out)
}

fn replace_matches(args: &Value) -> Result<String, String> {
    let (re, _) = get_pattern(args)?;
    let text = get_text(args)?;
    let replacement = args
        .get("replacement")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    let result = if limit == 0 || limit > 1 {
        if limit == 0 {
            re.replace_all(&text, replacement).to_string()
        } else {
            // Replace up to N occurrences
            let mut s = text.clone();
            let mut count = 0;
            while count < limit {
                let new = re.replacen(&s, 1, replacement).to_string();
                if new == s {
                    break;
                }
                s = new;
                count += 1;
            }
            s
        }
    } else {
        re.replace(&text, replacement).to_string()
    };

    let pattern_str = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    let count = re.find_iter(&text).count();

    Ok(format!(
        "REGEX REPLACE: /{pattern_str}/ → {replacement:?}\n\
         {}\n\
         Replacements : {} of {count} match(es) in original\n\n\
         Result:\n{result}",
        "─".repeat(60),
        if limit == 0 { count } else { limit.min(count) }
    ))
}

fn split_on_pattern(args: &Value) -> Result<String, String> {
    let (re, _) = get_pattern(args)?;
    let text = get_text(args)?;
    let pattern_str = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");

    let parts: Vec<&str> = re.split(&text).collect();
    let mut out = format!(
        "REGEX SPLIT: /{pattern_str}/\n{}\n{} part(s):\n\n",
        "─".repeat(60),
        parts.len()
    );
    for (i, part) in parts.iter().enumerate() {
        out.push_str(&format!("  [{i}] {part:?}\n"));
    }
    Ok(out)
}

fn explain_pattern(args: &Value) -> Result<String, String> {
    let pattern_str = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or("regex_tools explain: 'pattern' is required")?;

    // Validate the pattern compiles
    regex::Regex::new(pattern_str).map_err(|e| format!("regex_tools: invalid pattern: {e}"))?;

    let mut out = format!("REGEX EXPLAIN: /{pattern_str}/\n{}\n\n", "─".repeat(60));

    // Walk through the pattern and describe components
    let explanations = analyze_pattern(pattern_str);
    if explanations.is_empty() {
        out.push_str("Pattern components could not be broken down further.\n");
    } else {
        for line in &explanations {
            out.push_str(line);
            out.push('\n');
        }
    }

    // Show flags if any
    out.push_str("\nFlag options: case_insensitive, multiline, dot_all\n");

    Ok(out)
}

fn analyze_pattern(pattern: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = pattern.chars().collect();

    while i < chars.len() {
        let c = chars[i];
        let desc = match c {
            '^' => Some("^ — asserts start of line/string"),
            '$' => Some("$ — asserts end of line/string"),
            '.' => Some(". — matches any character except newline"),
            '*' => Some("* — repeats the previous element 0 or more times (greedy)"),
            '+' => Some("+ — repeats the previous element 1 or more times (greedy)"),
            '?' => Some("? — makes the previous element optional (0 or 1 time)"),
            '|' => Some("| — alternation: either the left or right side"),
            '\\' => {
                i += 1;
                if i < chars.len() {
                    match chars[i] {
                        'd' => Some("\\d — matches any digit [0-9]"),
                        'D' => Some("\\D — matches any non-digit"),
                        'w' => Some("\\w — matches any word character [a-zA-Z0-9_]"),
                        'W' => Some("\\W — matches any non-word character"),
                        's' => Some("\\s — matches any whitespace character"),
                        'S' => Some("\\S — matches any non-whitespace character"),
                        'b' => Some("\\b — word boundary (between \\w and \\W)"),
                        'B' => Some("\\B — not a word boundary"),
                        'n' => Some("\\n — newline character"),
                        't' => Some("\\t — tab character"),
                        'r' => Some("\\r — carriage return"),
                        '0'..='9' => Some("\\N — backreference to capture group N"),
                        _ => Some("\\? — escaped literal character"),
                    }
                } else {
                    None
                }
            }
            '[' => {
                // Find matching ]
                let start = i;
                while i < chars.len() && chars[i] != ']' {
                    i += 1;
                }
                let class: String = chars[start..=i.min(chars.len() - 1)].iter().collect();
                out.push(format!(
                    "{class} — character class (matches any one of the listed characters)"
                ));
                i += 1;
                continue;
            }
            '(' => {
                if i + 1 < chars.len() {
                    if chars[i + 1] == '?' {
                        if i + 2 < chars.len() {
                            match chars[i + 2] {
                                ':' => out.push("(?:...) — non-capturing group".to_string()),
                                '=' => out.push("(?=...) — positive lookahead".to_string()),
                                '!' => out.push("(?!...) — negative lookahead".to_string()),
                                '<' => {
                                    if i + 3 < chars.len() {
                                        match chars[i + 3] {
                                            '=' => out
                                                .push("(?<=...) — positive lookbehind".to_string()),
                                            '!' => out
                                                .push("(?<!...) — negative lookbehind".to_string()),
                                            _ => out.push(
                                                "(?<name>...) — named capture group".to_string(),
                                            ),
                                        }
                                    }
                                }
                                'P' => out.push(
                                    "(?P<name>...) — named capture group (Python syntax)"
                                        .to_string(),
                                ),
                                _ => {}
                            }
                        }
                    } else {
                        out.push(
                            "(...) — capture group (captured and numbered from 1)".to_string(),
                        );
                    }
                }
                i += 1;
                continue;
            }
            '{' => {
                // Find matching }
                let start = i;
                while i < chars.len() && chars[i] != '}' {
                    i += 1;
                }
                let quant: String = chars[start..=i.min(chars.len() - 1)].iter().collect();
                out.push(format!(
                    "{quant} — quantifier: repeat a specific number of times"
                ));
                i += 1;
                continue;
            }
            _ => None,
        };
        if let Some(d) = desc {
            out.push(d.to_string());
        }
        i += 1;
    }
    out
}

fn named_groups(args: &Value) -> Result<String, String> {
    let (re, _) = get_pattern(args)?;
    let text = get_text(args)?;
    let pattern_str = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");

    let names: Vec<Option<&str>> = re.capture_names().collect();
    let named: Vec<&str> = names.iter().filter_map(|n| *n).collect();

    if named.is_empty() {
        return Ok(format!(
            "regex_tools named-groups: pattern /{pattern_str}/ has no named capture groups.\n\
             Add named groups like (?P<name>...) or (?<name>...)."
        ));
    }

    let mut out = format!(
        "REGEX NAMED GROUPS: /{pattern_str}/\nGroups: {}\n{}\n\n",
        named.join(", "),
        "─".repeat(60)
    );

    for (i, caps) in re.captures_iter(&text).enumerate().take(20) {
        out.push_str(&format!("Match {}:\n", i + 1));
        for name in &named {
            if let Some(m) = caps.name(name) {
                out.push_str(&format!("  {name}: {:?}\n", m.as_str()));
            } else {
                out.push_str(&format!("  {name}: (not matched)\n"));
            }
        }
    }

    Ok(out)
}

fn build_flags_display(args: &Value) -> String {
    let mut flags = String::new();
    if args
        .get("case_insensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        flags.push('i');
    }
    if args
        .get("multiline")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        flags.push('m');
    }
    if args
        .get("dot_all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        flags.push('s');
    }
    flags
}
