use serde_json::{json, Value};

pub fn find_tools_schema() -> Value {
    json!({
        "name": "find_tools",
        "description": "Find files and directories matching criteria without external utilities (no `find` command needed). Actions: list (default — matching paths with metadata), count (how many match), sizes (size summary of matches), recent (recently modified files sorted newest first). Filter by name glob/substring, extension, size range, age, and type.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Root directory to search (default: '.')"
                },
                "action": {
                    "type": "string",
                    "enum": ["list", "count", "sizes", "recent"],
                    "description": "Action to perform (default: list)"
                },
                "name": {
                    "type": "string",
                    "description": "File name substring or glob-style pattern to match (e.g. '*.rs', 'main', 'test_*')"
                },
                "ext": {
                    "type": "string",
                    "description": "File extension filter, without dot (e.g. 'rs', 'json')"
                },
                "type": {
                    "type": "string",
                    "enum": ["file", "dir", "all"],
                    "description": "Entry type to find (default: file)"
                },
                "min_size": {
                    "type": "integer",
                    "description": "Minimum file size in bytes"
                },
                "max_size": {
                    "type": "integer",
                    "description": "Maximum file size in bytes"
                },
                "newer_than": {
                    "type": "integer",
                    "description": "Only show files modified in the last N days"
                },
                "older_than": {
                    "type": "integer",
                    "description": "Only show files not modified in the last N days"
                },
                "depth": {
                    "type": "integer",
                    "description": "Maximum recursion depth (default: 0 = unlimited)"
                },
                "show_hidden": {
                    "type": "boolean",
                    "description": "Include hidden files/dirs (default: false)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results to display (default: 200)"
                }
            },
            "required": []
        }
    })
}

// ── types ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct Found {
    path: std::path::PathBuf,
    is_dir: bool,
    size: u64,
    modified_secs: u64,
}

// ── helpers ───────────────────────────────────────────────────────────────────

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

fn glob_match(pattern: &str, name: &str) -> bool {
    if !pattern.contains('*') && !pattern.contains('?') {
        return name.to_lowercase().contains(&pattern.to_lowercase());
    }
    // Simple glob: * matches any sequence, ? matches one char
    let pat_bytes = pattern.as_bytes();
    let name_bytes = name.as_bytes();
    glob_match_bytes(pat_bytes, name_bytes)
}

fn glob_match_bytes(pat: &[u8], s: &[u8]) -> bool {
    match (pat.first(), s.first()) {
        (None, None) => true,
        (Some(b'*'), _) => {
            // Star: skip zero or more chars
            glob_match_bytes(&pat[1..], s) || (!s.is_empty() && glob_match_bytes(pat, &s[1..]))
        }
        (Some(b'?'), Some(_)) => glob_match_bytes(&pat[1..], &s[1..]),
        (Some(p), Some(c)) => p.eq_ignore_ascii_case(c) && glob_match_bytes(&pat[1..], &s[1..]),
        _ => false,
    }
}

fn unix_epoch_approx() -> u64 {
    // Approximate current Unix timestamp via SystemTime
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn walk(
    dir: &std::path::Path,
    depth: usize,
    max_depth: usize,
    show_hidden: bool,
    results: &mut Vec<Found>,
    filter: &Filter,
    now: u64,
) {
    if max_depth > 0 && depth > max_depth {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut paths: Vec<std::path::PathBuf> =
        entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();

    for path in paths {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if is_skip_dir(name) {
                continue;
            }
            if filter.entry_type != EntryType::File {
                let matches = filter.matches_dir(name);
                if matches {
                    results.push(Found {
                        path: path.clone(),
                        is_dir: true,
                        size: 0,
                        modified_secs: 0,
                    });
                }
            }
            walk(
                &path,
                depth + 1,
                max_depth,
                show_hidden,
                results,
                filter,
                now,
            );
        } else if path.is_file() {
            if filter.entry_type == EntryType::Dir {
                continue;
            }
            let meta = std::fs::metadata(&path).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified_secs = meta
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let age_days = if now > modified_secs {
                (now - modified_secs) / 86400
            } else {
                0
            };

            if !filter.matches_file(name, size, age_days) {
                continue;
            }
            results.push(Found {
                path,
                is_dir: false,
                size,
                modified_secs,
            });
        }
    }
}

#[derive(PartialEq)]
enum EntryType {
    File,
    Dir,
    All,
}

struct Filter {
    name: Option<String>,
    ext: Option<String>,
    entry_type: EntryType,
    min_size: Option<u64>,
    max_size: Option<u64>,
    newer_than_days: Option<u64>,
    older_than_days: Option<u64>,
}

impl Filter {
    fn matches_file(&self, name: &str, size: u64, age_days: u64) -> bool {
        if let Some(ref p) = self.name {
            if !glob_match(p, name) {
                return false;
            }
        }
        if let Some(ref e) = self.ext {
            let file_ext = std::path::Path::new(name)
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("");
            if file_ext.to_lowercase() != e.to_lowercase() {
                return false;
            }
        }
        if let Some(min) = self.min_size {
            if size < min {
                return false;
            }
        }
        if let Some(max) = self.max_size {
            if size > max {
                return false;
            }
        }
        if let Some(newer) = self.newer_than_days {
            if age_days > newer {
                return false;
            }
        }
        if let Some(older) = self.older_than_days {
            if age_days <= older {
                return false;
            }
        }
        true
    }

    fn matches_dir(&self, name: &str) -> bool {
        if let Some(ref p) = self.name {
            return glob_match(p, name);
        }
        true
    }
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_age(secs: u64, now: u64) -> String {
    if now <= secs {
        return "now".to_string();
    }
    let diff = now - secs;
    if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 86400 * 30 {
        format!("{}d ago", diff / 86400)
    } else if diff < 86400 * 365 {
        format!("{}mo ago", diff / (86400 * 30))
    } else {
        format!("{}y ago", diff / (86400 * 365))
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_list(results: &[Found], root: &str, limit: usize, now: u64) -> String {
    let mut out = format!("{} match(es)\n", results.len().min(limit));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for r in results.iter().take(limit) {
        let rel = r
            .path
            .strip_prefix(root)
            .unwrap_or(&r.path)
            .to_string_lossy();
        if r.is_dir {
            out.push_str(&format!("  {rel}/\n"));
        } else {
            let age = format_age(r.modified_secs, now);
            out.push_str(&format!(
                "  {:>10}  {:>10}  {rel}\n",
                human_size(r.size),
                age
            ));
        }
    }
    if results.len() > limit {
        out.push_str(&format!("\n... ({} more)\n", results.len() - limit));
    }
    out
}

fn action_count(results: &[Found]) -> String {
    let files = results.iter().filter(|r| !r.is_dir).count();
    let dirs = results.iter().filter(|r| r.is_dir).count();
    let total_size: u64 = results.iter().map(|r| r.size).sum();
    format!(
        "{} match(es): {} file(s)  {} dir(s)  total {}\n",
        results.len(),
        files,
        dirs,
        human_size(total_size)
    )
}

fn action_sizes(results: &[Found], root: &str) -> String {
    let mut files: Vec<_> = results.iter().filter(|r| !r.is_dir).collect();
    files.sort_by(|a, b| b.size.cmp(&a.size));

    let total: u64 = files.iter().map(|f| f.size).sum();
    let mut out = format!("{} file(s)  total {}\n", files.len(), human_size(total));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for f in files.iter().take(50) {
        let rel = f
            .path
            .strip_prefix(root)
            .unwrap_or(&f.path)
            .to_string_lossy();
        out.push_str(&format!("{:>12}  {rel}\n", human_size(f.size)));
    }
    if files.len() > 50 {
        out.push_str(&format!("\n... ({} more)\n", files.len() - 50));
    }
    out
}

fn action_recent(results: &[Found], root: &str, limit: usize, now: u64) -> String {
    let mut files: Vec<_> = results.iter().filter(|r| !r.is_dir).collect();
    files.sort_by(|a, b| b.modified_secs.cmp(&a.modified_secs));

    let mut out = format!(
        "{} file(s) sorted by modification time\n",
        files.len().min(limit)
    );
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for f in files.iter().take(limit) {
        let rel = f
            .path
            .strip_prefix(root)
            .unwrap_or(&f.path)
            .to_string_lossy();
        let age = format_age(f.modified_secs, now);
        out.push_str(&format!(
            "  {:>10}  {:>10}  {rel}\n",
            age,
            human_size(f.size)
        ));
    }
    if files.len() > limit {
        out.push_str(&format!("\n... ({} more)\n", files.len() - limit));
    }
    out
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let root = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("list");
    let max_depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let show_hidden = args
        .get("show_hidden")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize;

    let entry_type = match args.get("type").and_then(|v| v.as_str()).unwrap_or("file") {
        "dir" => EntryType::Dir,
        "all" => EntryType::All,
        _ => EntryType::File,
    };

    let filter = Filter {
        name: args
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        ext: args
            .get("ext")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        entry_type,
        min_size: args.get("min_size").and_then(|v| v.as_u64()),
        max_size: args.get("max_size").and_then(|v| v.as_u64()),
        newer_than_days: args.get("newer_than").and_then(|v| v.as_u64()),
        older_than_days: args.get("older_than").and_then(|v| v.as_u64()),
    };

    if !std::path::Path::new(root).is_dir() {
        return Err(format!("'{root}' is not a directory or does not exist"));
    }

    let now = unix_epoch_approx();
    let mut results: Vec<Found> = Vec::new();
    walk(
        std::path::Path::new(root),
        1,
        max_depth,
        show_hidden,
        &mut results,
        &filter,
        now,
    );

    if results.is_empty() {
        return Ok(format!("No matches found in '{root}'.\n"));
    }

    match action {
        "list" => Ok(action_list(&results, root, limit, now)),
        "count" => Ok(action_count(&results)),
        "sizes" => Ok(action_sizes(&results, root)),
        "recent" => Ok(action_recent(&results, root, limit, now)),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: list, count, sizes, recent"
        )),
    }
}
