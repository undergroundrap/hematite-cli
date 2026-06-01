use regex::Regex;
use serde_json::{json, Value};

pub fn grep_tools_schema() -> Value {
    json!({
        "name": "grep_tools",
        "description": "Search files for patterns using regular expressions without external utilities. Actions: search (default — find all matching lines with file:line context, optional before/after context), count (match count per file), files (list files with at least one match), matches (flat list of every match with capture group extraction). Supports case-insensitive mode, multiline, fixed-string mode, and file extension filtering.",
        "parameters": {
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression pattern to search for (required)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search (default: '.')"
                },
                "action": {
                    "type": "string",
                    "enum": ["search", "count", "files", "matches"],
                    "description": "Action to perform (default: search)"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case-insensitive matching (default: false)"
                },
                "fixed": {
                    "type": "boolean",
                    "description": "Treat pattern as a fixed string, not a regex (default: false)"
                },
                "before": {
                    "type": "integer",
                    "description": "Lines of context before each match (default: 0)"
                },
                "after": {
                    "type": "integer",
                    "description": "Lines of context after each match (default: 0)"
                },
                "extensions": {
                    "type": "string",
                    "description": "Comma-separated file extensions to include (default: rs,py,js,ts,go,java,c,cpp,h,hpp,cs,rb,php,swift,kt,dart,scala,sh,md,txt,json,yaml,yml,toml,ini,cfg,conf)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum matches to display (default: 200)"
                },
                "invert": {
                    "type": "boolean",
                    "description": "Show lines that do NOT match the pattern (default: false)"
                },
                "whole_word": {
                    "type": "boolean",
                    "description": "Match whole words only — wraps pattern in \\b...\\b (default: false)"
                }
            },
            "required": ["pattern"]
        }
    })
}

// ── types ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct Match {
    file: String,
    line: usize,
    text: String,
    captures: Vec<String>,
}

// ── file walker ───────────────────────────────────────────────────────────────

fn default_extensions() -> Vec<String> {
    "rs,py,js,ts,go,java,c,cpp,h,hpp,cs,rb,php,swift,kt,dart,scala,sh,md,txt,json,yaml,yml,toml,ini,cfg,conf"
        .split(',')
        .map(|s| s.to_string())
        .collect()
}

fn should_include(path: &std::path::Path, extensions: &[String]) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    extensions.iter().any(|e| e == ext)
}

fn is_skip_dir(name: &str) -> bool {
    matches!(
        name,
        "target"
            | ".git"
            | "node_modules"
            | ".hematite"
            | "vendor"
            | "dist"
            | "build"
            | ".next"
            | "__pycache__"
            | ".venv"
            | "venv"
            | ".tox"
            | "coverage"
    )
}

fn walk(dir: &std::path::Path, extensions: &[String], out: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut paths: Vec<std::path::PathBuf> =
        entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !is_skip_dir(name) {
                walk(&path, extensions, out);
            }
        } else if path.is_file() && should_include(&path, extensions) {
            out.push(path);
        }
    }
}

fn collect_files(args: &Value) -> Vec<std::path::PathBuf> {
    let root = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let extensions: Vec<String> = args
        .get("extensions")
        .and_then(|v| v.as_str())
        .map(|s| s.split(',').map(|e| e.trim().to_string()).collect())
        .unwrap_or_else(default_extensions);

    let p = std::path::Path::new(root);
    if p.is_file() {
        vec![p.to_path_buf()]
    } else {
        let mut files = Vec::new();
        walk(p, &extensions, &mut files);
        files
    }
}

// ── search engine ─────────────────────────────────────────────────────────────

fn build_regex(args: &Value) -> Result<Regex, String> {
    let raw = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or("pattern is required")?;

    let fixed = args.get("fixed").and_then(|v| v.as_bool()).unwrap_or(false);
    let ci = args
        .get("case_insensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let ww = args
        .get("whole_word")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let escaped = if fixed {
        regex::escape(raw)
    } else {
        raw.to_string()
    };

    let wrapped = if ww {
        format!(r"\b{}\b", escaped)
    } else {
        escaped
    };

    let flags = if ci { "(?i)" } else { "" };
    Regex::new(&format!("{}{}", flags, wrapped)).map_err(|e| format!("Invalid regex: {e}"))
}

fn search_file(
    path: &std::path::Path,
    re: &Regex,
    invert: bool,
    before: usize,
    after: usize,
    matches: &mut Vec<Match>,
    limit: usize,
) {
    if matches.len() >= limit {
        return;
    }
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let lines: Vec<&str> = content.lines().collect();
    let file_str = path.to_string_lossy().to_string();

    // Collect matching line indices
    let mut hit_lines: Vec<usize> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let is_match = re.is_match(line);
        if is_match ^ invert {
            hit_lines.push(i);
        }
    }

    // Expand with context, deduplicate
    if before == 0 && after == 0 {
        for i in hit_lines {
            if matches.len() >= limit {
                break;
            }
            let caps: Vec<String> = re
                .captures(lines[i])
                .map(|c| {
                    (1..c.len())
                        .filter_map(|idx| c.get(idx).map(|m| m.as_str().to_string()))
                        .collect()
                })
                .unwrap_or_default();
            matches.push(Match {
                file: file_str.clone(),
                line: i + 1,
                text: lines[i].to_string(),
                captures: caps,
            });
        }
    } else {
        // Context mode: collect ranges and emit separator-aware output
        let mut covered = std::collections::HashSet::new();
        for &hit in &hit_lines {
            let start = hit.saturating_sub(before);
            let end = (hit + after + 1).min(lines.len());
            for idx in start..end {
                covered.insert(idx);
            }
        }
        let mut sorted: Vec<usize> = covered.into_iter().collect();
        sorted.sort_unstable();

        let hit_set: std::collections::HashSet<usize> = hit_lines.iter().copied().collect();

        let mut prev: Option<usize> = None;
        for idx in sorted {
            if matches.len() >= limit {
                break;
            }
            let gap = prev.map(|p| idx > p + 1).unwrap_or(false);
            let prefix = if hit_set.contains(&idx) { ":" } else { "-" };
            let sep = if gap { "--\n" } else { "" };
            let caps: Vec<String> = if hit_set.contains(&idx) {
                re.captures(lines[idx])
                    .map(|c| {
                        (1..c.len())
                            .filter_map(|i| c.get(i).map(|m| m.as_str().to_string()))
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                vec![]
            };
            matches.push(Match {
                file: file_str.clone(),
                line: idx + 1,
                text: format!("{sep}{prefix} {}", lines[idx]),
                captures: caps,
            });
            prev = Some(idx);
        }
    }
}

fn run_search(args: &Value) -> Result<(Vec<Match>, bool), String> {
    let re = build_regex(args)?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
    let invert = args
        .get("invert")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let before = args.get("before").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let after = args.get("after").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    let files = collect_files(args);
    let mut matches: Vec<Match> = Vec::new();
    let mut truncated = false;

    for f in &files {
        if matches.len() >= limit {
            truncated = true;
            break;
        }
        search_file(f, &re, invert, before, after, &mut matches, limit);
    }
    if matches.len() >= limit {
        truncated = true;
    }

    Ok((matches, truncated))
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_search(matches: &[Match], truncated: bool, limit: usize) -> String {
    if matches.is_empty() {
        return "No matches found.\n".to_string();
    }

    let mut unique_files: Vec<&str> = matches.iter().map(|m| m.file.as_str()).collect();
    unique_files.sort_unstable();
    unique_files.dedup();

    let mut out = format!(
        "{} matches in {} file(s)\n",
        matches
            .iter()
            .filter(|m| !m.text.starts_with("--\n") && !m.text.starts_with('-'))
            .count()
            + matches
                .iter()
                .filter(|m| !m.text.starts_with("--\n") && !m.text.starts_with('-'))
                .count()
                .min(0),
        unique_files.len()
    );

    // Recount properly
    let real_count = matches
        .iter()
        .filter(|m| !m.text.starts_with('-') && !m.text.starts_with("--"))
        .count();
    out = format!("{} matches in {} file(s)\n", real_count, unique_files.len());

    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut last_file = "";
    for m in matches {
        let short = m.file.trim_start_matches("./").trim_start_matches(".\\");
        if short != last_file {
            out.push_str(&format!("\n{short}\n"));
            last_file = short;
        }
        if m.text.starts_with("--\n") {
            out.push_str("  --\n");
            let rest = m.text.trim_start_matches("--\n");
            out.push_str(&format!("  {:4}  {rest}\n", m.line));
        } else {
            out.push_str(&format!("  {:4}  {}\n", m.line, m.text));
        }
    }

    if truncated {
        out.push_str(&format!("\n... limit reached ({limit} shown)\n"));
    }
    out
}

fn action_count(args: &Value, matches: &[Match]) -> String {
    use std::collections::HashMap;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for m in matches {
        *counts.entry(m.file.as_str()).or_insert(0) += 1;
    }

    let mut entries: Vec<_> = counts.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

    let total: usize = entries.iter().map(|(_, c)| c).sum();
    let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    let mut out = format!(
        "Pattern: {pattern}\n{total} matches in {} file(s)\n",
        entries.len()
    );
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for (file, count) in &entries {
        let short = file.trim_start_matches("./").trim_start_matches(".\\");
        out.push_str(&format!("{:>6}  {short}\n", count));
    }
    out
}

fn action_files(matches: &[Match]) -> String {
    let mut files: Vec<&str> = matches.iter().map(|m| m.file.as_str()).collect();
    files.sort_unstable();
    files.dedup();

    if files.is_empty() {
        return "No matching files.\n".to_string();
    }

    let mut out = format!("{} file(s) with matches\n", files.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');
    for f in files {
        let short = f.trim_start_matches("./").trim_start_matches(".\\");
        out.push_str(&format!("{short}\n"));
    }
    out
}

fn action_matches(matches: &[Match], truncated: bool, limit: usize) -> String {
    if matches.is_empty() {
        return "No matches found.\n".to_string();
    }

    let mut out = format!("{} matches\n", matches.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for m in matches {
        let short = m.file.trim_start_matches("./").trim_start_matches(".\\");
        if m.captures.is_empty() {
            out.push_str(&format!("{}:{}  {}\n", short, m.line, m.text.trim()));
        } else {
            let caps = m.captures.join(", ");
            out.push_str(&format!(
                "{}:{}  {}  [captures: {}]\n",
                short,
                m.line,
                m.text.trim(),
                caps
            ));
        }
    }

    if truncated {
        out.push_str(&format!("\n... limit reached ({limit} shown)\n"));
    }
    out
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("search");
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize;

    let (matches, truncated) = run_search(args)?;

    if matches.is_empty()
        && !args
            .get("invert")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    {
        let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        return Ok(format!("No matches for pattern '{pattern}' in '{path}'.\n"));
    }

    match action {
        "search" => Ok(action_search(&matches, truncated, limit)),
        "count" => Ok(action_count(args, &matches)),
        "files" => Ok(action_files(&matches)),
        "matches" => Ok(action_matches(&matches, truncated, limit)),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: search, count, files, matches"
        )),
    }
}
