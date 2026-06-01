use serde_json::{json, Value};

pub fn todo_tools_schema() -> Value {
    json!({
        "name": "todo_tools",
        "description": "Scan source files for TODO, FIXME, HACK, XXX, NOTE, DEPRECATED, BUG, and OPTIMIZE comments without external utilities. Actions: scan (default — find all annotated comments in the workspace or a path, grouped by label with file:line context), stats (count summary per label across all files), list (flat list of all findings with file/line/label/text), filter (show only a specific label; pass 'label'), files (which files have the most annotations; top N files).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["scan", "stats", "list", "filter", "files"],
                    "description": "Action to perform (default: scan)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to scan (default: workspace root '.')"
                },
                "label": {
                    "type": "string",
                    "description": "Label to filter by for 'filter' action (e.g. TODO, FIXME, HACK)"
                },
                "extensions": {
                    "type": "string",
                    "description": "Comma-separated file extensions to include (default: rs,py,js,ts,go,java,c,cpp,h,hpp,cs,rb,php,swift,kt,dart,scala,sh,md)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum findings to display (default: 100)"
                },
                "top": {
                    "type": "integer",
                    "description": "Top N files to show for 'files' action (default: 20)"
                }
            },
            "required": []
        }
    })
}

// ── types ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct Finding {
    file: String,
    line: usize,
    label: String,
    text: String,
}

// ── scanner ───────────────────────────────────────────────────────────────────

const LABELS: &[&str] = &[
    "TODO",
    "FIXME",
    "HACK",
    "XXX",
    "NOTE",
    "DEPRECATED",
    "BUG",
    "OPTIMIZE",
    "WORKAROUND",
    "TEMP",
    "KLUDGE",
    "NB",
];

fn scan_file(path: &str, findings: &mut Vec<Finding>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for (lineno, line) in content.lines().enumerate() {
        let upper = line.to_uppercase();
        for &label in LABELS {
            // Match label followed by optional : or space, but not as part of a longer word
            // e.g. "TODO:" or "TODO " or "TODO(" or "// TODO" — but not "TODOLIST"
            if let Some(idx) = upper.find(label) {
                // Check that the character before (if any) is not alphanumeric
                let before_ok = idx == 0 || {
                    let before_char = upper
                        .as_bytes()
                        .get(idx.saturating_sub(1))
                        .copied()
                        .unwrap_or(0);
                    !before_char.is_ascii_alphanumeric()
                };
                // Check that the character after (if any) is not alphanumeric (except :, (, space, !)
                let after_pos = idx + label.len();
                let after_ok = after_pos >= upper.len() || {
                    let after_char = upper.as_bytes()[after_pos];
                    !after_char.is_ascii_alphabetic() || after_char == b'_'
                };

                if before_ok && after_ok {
                    // Extract the text after the label
                    let raw = &line[idx..];
                    let text = raw
                        .trim_start_matches(label)
                        .trim_start_matches(':')
                        .trim_start_matches('(')
                        .trim();
                    let text = if text.len() > 120 {
                        format!("{}...", &text[..120])
                    } else {
                        text.to_string()
                    };

                    findings.push(Finding {
                        file: path.to_string(),
                        line: lineno + 1,
                        label: label.to_string(),
                        text,
                    });
                    break; // Only one label per line
                }
            }
        }
    }
}

fn default_extensions() -> Vec<String> {
    "rs,py,js,ts,go,java,c,cpp,h,hpp,cs,rb,php,swift,kt,dart,scala,sh,md"
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

fn walk_and_scan(dir: &std::path::Path, extensions: &[String], findings: &mut Vec<Finding>) {
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
                walk_and_scan(&path, extensions, findings);
            }
        } else if path.is_file() && should_include(&path, extensions) {
            if let Some(s) = path.to_str() {
                scan_file(s, findings);
            }
        }
    }
}

fn collect_findings(args: &Value) -> Vec<Finding> {
    let root = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let extensions: Vec<String> = args
        .get("extensions")
        .and_then(|v| v.as_str())
        .map(|s| s.split(',').map(|e| e.trim().to_string()).collect())
        .unwrap_or_else(default_extensions);

    let mut findings = Vec::new();
    let p = std::path::Path::new(root);
    if p.is_file() {
        if let Some(s) = p.to_str() {
            scan_file(s, &mut findings);
        }
    } else {
        walk_and_scan(p, &extensions, &mut findings);
    }
    findings
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_scan(findings: &[Finding], limit: usize) -> String {
    use std::collections::HashMap;
    let mut by_label: HashMap<&str, Vec<&Finding>> = HashMap::new();
    for f in findings {
        by_label.entry(f.label.as_str()).or_default().push(f);
    }

    let mut out = String::new();
    out.push_str(&format!(
        "Found {} annotations across {} files\n",
        findings.len(),
        {
            let mut files: Vec<&str> = findings.iter().map(|f| f.file.as_str()).collect();
            files.sort_unstable();
            files.dedup();
            files.len()
        }
    ));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut labels: Vec<&&str> = by_label.keys().collect();
    labels.sort();

    let mut shown = 0usize;
    for label in labels {
        let items = &by_label[*label];
        out.push_str(&format!("\n{label} ({})\n", items.len()));
        out.push_str(&"─".repeat(40));
        out.push('\n');
        for f in items.iter().take(limit.saturating_sub(shown)) {
            let short_file = f.file.trim_start_matches("./").trim_start_matches(".\\");
            out.push_str(&format!(
                "  {:4}  {}:{}\n",
                format!("L{}", f.line),
                short_file,
                if f.text.is_empty() {
                    String::new()
                } else {
                    format!("  {}", f.text)
                }
            ));
            shown += 1;
        }
        if shown >= limit {
            out.push_str(&format!("\n... limit reached ({limit} shown)\n"));
            break;
        }
    }
    out
}

fn action_stats(findings: &[Finding]) -> String {
    use std::collections::HashMap;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for f in findings {
        *counts.entry(f.label.as_str()).or_insert(0) += 1;
    }

    let mut entries: Vec<_> = counts.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));

    let total: usize = entries.iter().map(|(_, c)| c).sum();
    let mut files: Vec<&str> = findings.iter().map(|f| f.file.as_str()).collect();
    files.sort_unstable();
    files.dedup();

    let mut out = String::new();
    out.push_str(&format!(
        "Annotation Stats: {} total in {} files\n",
        total,
        files.len()
    ));
    out.push_str(&"─".repeat(40));
    out.push('\n');
    for (label, count) in &entries {
        let bar_len = count * 30 / total.max(1);
        let bar = "█".repeat(bar_len);
        out.push_str(&format!("{:<14}  {:>5}  {bar}\n", label, count));
    }
    out
}

fn action_list(findings: &[Finding], limit: usize) -> String {
    let mut out = String::new();
    out.push_str(&format!("{} annotations\n", findings.len()));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for f in findings.iter().take(limit) {
        let short_file = f.file.trim_start_matches("./").trim_start_matches(".\\");
        out.push_str(&format!(
            "{:<12}  {}:{}  {}\n",
            f.label,
            short_file,
            f.line,
            if f.text.is_empty() {
                "(no text)"
            } else {
                &f.text
            }
        ));
    }
    if findings.len() > limit {
        out.push_str(&format!("\n... ({} more)\n", findings.len() - limit));
    }
    out
}

fn action_filter(findings: &[Finding], label: &str, limit: usize) -> String {
    let label_upper = label.to_uppercase();
    let matched: Vec<&Finding> = findings.iter().filter(|f| f.label == label_upper).collect();

    let mut out = String::new();
    out.push_str(&format!("{}: {} occurrences\n", label_upper, matched.len()));
    out.push_str(&"─".repeat(60));
    out.push('\n');
    for f in matched.iter().take(limit) {
        let short_file = f.file.trim_start_matches("./").trim_start_matches(".\\");
        out.push_str(&format!(
            "  {}:{}  {}\n",
            short_file,
            f.line,
            if f.text.is_empty() {
                "(no text)"
            } else {
                &f.text
            }
        ));
    }
    if matched.len() > limit {
        out.push_str(&format!("\n... ({} more)\n", matched.len() - limit));
    }
    out
}

fn action_files(findings: &[Finding], top: usize) -> String {
    use std::collections::HashMap;
    let mut file_counts: HashMap<&str, usize> = HashMap::new();
    for f in findings {
        *file_counts.entry(f.file.as_str()).or_insert(0) += 1;
    }

    let mut entries: Vec<_> = file_counts.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));

    let mut out = String::new();
    out.push_str(&format!(
        "Top files by annotation count (showing {}):\n",
        top
    ));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for (file, count) in entries.iter().take(top) {
        let short_file = file.trim_start_matches("./").trim_start_matches(".\\");
        out.push_str(&format!("{:>5}  {short_file}\n", count));
    }
    out
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("scan");
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
    let top = args.get("top").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

    let findings = collect_findings(args);

    if findings.is_empty() {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        return Ok(format!(
            "No TODO/FIXME/HACK/XXX/NOTE/DEPRECATED/BUG/OPTIMIZE annotations found in '{path}'.\n"
        ));
    }

    match action {
        "scan" => Ok(action_scan(&findings, limit)),
        "stats" => Ok(action_stats(&findings)),
        "list" => Ok(action_list(&findings, limit)),
        "filter" => {
            let label = args.get("label").and_then(|v| v.as_str()).unwrap_or("TODO");
            Ok(action_filter(&findings, label, limit))
        }
        "files" => Ok(action_files(&findings, top)),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: scan, stats, list, filter, files"
        )),
    }
}
