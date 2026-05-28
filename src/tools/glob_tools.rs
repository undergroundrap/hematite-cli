use regex::Regex;
use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("paths").is_some() || args.get("list").is_some() {
        "filter".to_string()
    } else if args.get("path").is_some() || args.get("input").is_some() {
        "match".to_string()
    } else {
        "explain".to_string()
    };
    match action.as_str() {
        "match" => match_action(args),
        "filter" => filter_action(args),
        "explain" => explain_action(args),
        "convert" => convert_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: match, filter, explain, convert",
            action
        )),
    }
}

fn glob_to_regex_str(pattern: &str) -> String {
    let mut out = String::from("^");
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    // ** matches anything including path separators
                    out.push_str(".*");
                    i += 2;
                    // skip trailing slash after **
                    if i < chars.len() && (chars[i] == '/' || chars[i] == '\\') {
                        i += 1;
                    }
                    continue;
                } else {
                    out.push_str("[^/]*");
                }
            }
            '?' => out.push_str("[^/]"),
            '[' => {
                // character class — pass through but handle [!...] → [^...]
                out.push('[');
                i += 1;
                if i < chars.len() && chars[i] == '!' {
                    out.push('^');
                    i += 1;
                }
                while i < chars.len() && chars[i] != ']' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        out.push('\\');
                        out.push(chars[i + 1]);
                        i += 2;
                        continue;
                    }
                    out.push(chars[i]);
                    i += 1;
                }
                out.push(']');
            }
            '.' | '+' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
        i += 1;
    }
    out.push('$');
    out
}

fn get_pattern(args: &Value) -> Result<String, String> {
    args.get("pattern")
        .or_else(|| args.get("glob"))
        .or_else(|| args.get("pat"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'pattern' argument — pass a glob like '**/*.rs'".to_string())
}

fn match_action(args: &Value) -> Result<String, String> {
    let pattern = get_pattern(args)?;
    let path = args
        .get("path")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;
    let re_str = glob_to_regex_str(&pattern);
    let re = Regex::new(&re_str).map_err(|e| format!("Pattern compile error: {}", e))?;
    let matched = re.is_match(path);
    let mut out = format!("Glob Match\n{}\n\n", "=".repeat(44));
    out += &format!("Pattern: {}\n", pattern);
    out += &format!("Path:    {}\n", path);
    out += &format!("Regex:   {}\n\n", re_str);
    out += if matched {
        "Result:  MATCH\n"
    } else {
        "Result:  NO MATCH\n"
    };
    Ok(out)
}

fn filter_action(args: &Value) -> Result<String, String> {
    let pattern = get_pattern(args)?;
    let re_str = glob_to_regex_str(&pattern);
    let re = Regex::new(&re_str).map_err(|e| format!("Pattern compile error: {}", e))?;

    let paths: Vec<String> = if let Some(arr) = args.get("paths").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else if let Some(arr) = args.get("list").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else if let Some(text) = args
        .get("paths")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
        text.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    } else {
        return Err(
            "Missing 'paths' — pass an array or newline-separated string of paths".to_string(),
        );
    };

    let matches: Vec<&String> = paths.iter().filter(|p| re.is_match(p)).collect();
    let mut out = format!("Glob Filter: '{}'\n{}\n\n", pattern, "=".repeat(44));
    out += &format!("Matched {} of {} paths\n\n", matches.len(), paths.len());
    if matches.is_empty() {
        out += "No paths matched.\n";
    } else {
        for p in &matches {
            out += &format!("  {}\n", p);
        }
    }
    Ok(out)
}

struct GlobToken {
    raw: String,
    kind: &'static str,
    description: String,
}

fn tokenize_glob(pattern: &str) -> Vec<GlobToken> {
    let mut tokens: Vec<GlobToken> = Vec::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    let mut raw = String::from("**");
                    i += 2;
                    if i < chars.len() && (chars[i] == '/' || chars[i] == '\\') {
                        raw.push(chars[i]);
                        i += 1;
                    }
                    tokens.push(GlobToken {
                        raw,
                        kind: "globstar",
                        description: "Match any path — zero or more directories".to_string(),
                    });
                    continue;
                } else {
                    tokens.push(GlobToken {
                        raw: "*".to_string(),
                        kind: "wildcard",
                        description: "Match any sequence of characters (not a path separator)"
                            .to_string(),
                    });
                }
            }
            '?' => {
                tokens.push(GlobToken {
                    raw: "?".to_string(),
                    kind: "any-char",
                    description: "Match exactly one character (not a path separator)".to_string(),
                });
            }
            '[' => {
                let mut raw = String::from("[");
                let negated = i + 1 < chars.len() && chars[i + 1] == '!';
                if negated {
                    raw.push('!');
                    i += 1;
                }
                i += 1;
                while i < chars.len() && chars[i] != ']' {
                    raw.push(chars[i]);
                    i += 1;
                }
                raw.push(']');
                let desc = if negated {
                    format!(
                        "Match any character NOT in the set '{}'",
                        &raw[2..raw.len() - 1]
                    )
                } else {
                    format!(
                        "Match any character in the set '{}'",
                        &raw[1..raw.len() - 1]
                    )
                };
                tokens.push(GlobToken {
                    raw,
                    kind: "char-class",
                    description: desc,
                });
            }
            '/' | '\\' => {
                tokens.push(GlobToken {
                    raw: c.to_string(),
                    kind: "separator",
                    description: "Path separator".to_string(),
                });
            }
            '.' => {
                tokens.push(GlobToken {
                    raw: ".".to_string(),
                    kind: "literal",
                    description: "Literal dot (extension separator)".to_string(),
                });
            }
            _ => {
                // merge consecutive literal chars
                let mut raw = c.to_string();
                while i + 1 < chars.len() {
                    let next = chars[i + 1];
                    if !matches!(next, '*' | '?' | '[' | '/' | '\\' | '.') {
                        raw.push(next);
                        i += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(GlobToken {
                    raw: raw.clone(),
                    kind: "literal",
                    description: format!("Literal text '{}'", raw),
                });
            }
        }
        i += 1;
    }
    tokens
}

fn explain_action(args: &Value) -> Result<String, String> {
    let pattern = get_pattern(args)?;
    let tokens = tokenize_glob(&pattern);
    let re_str = glob_to_regex_str(&pattern);

    let mut out = format!("Glob Pattern: '{}'\n{}\n\n", pattern, "=".repeat(44));
    out += "Components:\n";
    for t in &tokens {
        out += &format!(
            "  {:12} {:<16} {}\n",
            format!("'{}'", t.raw),
            t.kind,
            t.description
        );
    }
    out += &format!("\nEquivalent regex: {}\n", re_str);

    // Plain-English summary
    out += "\nSummary: matches ";
    let has_globstar = tokens.iter().any(|t| t.kind == "globstar");
    let has_wildcard = tokens.iter().any(|t| t.kind == "wildcard");
    let last_literal = tokens.iter().rev().find(|t| t.kind == "literal");
    if has_globstar {
        out += "paths at any depth";
    } else if has_wildcard {
        out += "paths in a single directory";
    } else {
        out += "an exact path";
    }
    if let Some(lit) = last_literal {
        if lit.raw.starts_with(|c: char| !c.is_alphabetic()) || pattern.contains('.') {
            // probably an extension
        }
        out += &format!(" named like '{}'", lit.raw);
    }
    out += "\n";
    Ok(out)
}

fn convert_action(args: &Value) -> Result<String, String> {
    let pattern = get_pattern(args)?;
    let re_str = glob_to_regex_str(&pattern);
    Regex::new(&re_str).map_err(|e| format!("Pattern compile error: {}", e))?;
    let mut out = format!("Glob to Regex\n{}\n\n", "=".repeat(44));
    out += &format!("Glob:  {}\n", pattern);
    out += &format!("Regex: {}\n\n", re_str);
    out += "Conversion rules applied:\n";
    if pattern.contains("**") {
        out += "  **    -> .*         (match any path, including separators)\n";
    }
    if pattern.chars().filter(|&c| c == '*').count() > pattern.matches("**").count() * 2 {
        out += "  *     -> [^/]*      (match any filename segment)\n";
    }
    if pattern.contains('?') {
        out += "  ?     -> [^/]       (match any single character)\n";
    }
    if pattern.contains('[') {
        out += "  [!..] -> [^..]      (negated character class)\n";
    }
    out += "  ^ and $ anchors added (full-string match)\n";
    Ok(out)
}
