use serde_json::{json, Value};

pub fn file_tree_tools_schema() -> Value {
    json!({
        "name": "file_tree_tools",
        "description": "Generate visual file tree representations and directory statistics without external utilities (no `tree` command needed). Actions: tree (default — ASCII directory tree with optional depth limit), flat (sorted flat file list), stats (file/directory/size breakdown by extension), json (structured JSON tree), sizes (files sorted by size, largest first).",
        "parameters": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to visualize (default: '.')"
                },
                "action": {
                    "type": "string",
                    "enum": ["tree", "flat", "stats", "json", "sizes"],
                    "description": "Action to perform (default: tree)"
                },
                "depth": {
                    "type": "integer",
                    "description": "Maximum depth to traverse (default: 4; 0 = unlimited)"
                },
                "show_hidden": {
                    "type": "boolean",
                    "description": "Include hidden files and directories starting with '.' (default: false)"
                },
                "extensions": {
                    "type": "string",
                    "description": "Comma-separated extensions to include — omit to show all (e.g. 'rs,toml')"
                },
                "skip_dirs": {
                    "type": "string",
                    "description": "Comma-separated directory names to skip in addition to defaults (target,.git,node_modules)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum entries to display (default: 500)"
                },
                "top": {
                    "type": "integer",
                    "description": "For 'sizes' action: top N largest files (default: 30)"
                }
            },
            "required": []
        }
    })
}

// ── types ─────────────────────────────────────────────────────────────────────

struct Entry {
    path: std::path::PathBuf,
    is_dir: bool,
    size: u64,
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn default_skip_dirs() -> Vec<String> {
    vec![
        "target",
        ".git",
        "node_modules",
        ".hematite",
        "vendor",
        "dist",
        "build",
        ".next",
        "__pycache__",
        ".venv",
        "venv",
        ".tox",
        "coverage",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}

fn should_skip(name: &str, skip: &[String], show_hidden: bool) -> bool {
    if !show_hidden && name.starts_with('.') {
        return true;
    }
    skip.iter().any(|s| s == name)
}

fn ext_match(path: &std::path::Path, exts: &Option<Vec<String>>) -> bool {
    match exts {
        None => true,
        Some(list) => {
            let e = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            list.iter().any(|x| x == e)
        }
    }
}

fn collect_entries(
    dir: &std::path::Path,
    depth: usize,
    max_depth: usize,
    skip: &[String],
    show_hidden: bool,
    exts: &Option<Vec<String>>,
    out: &mut Vec<Entry>,
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
        if should_skip(name, skip, show_hidden) {
            continue;
        }
        if path.is_dir() {
            out.push(Entry {
                path: path.clone(),
                is_dir: true,
                size: 0,
            });
            collect_entries(&path, depth + 1, max_depth, skip, show_hidden, exts, out);
        } else if path.is_file() {
            if !ext_match(&path, exts) {
                continue;
            }
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            out.push(Entry {
                path,
                is_dir: false,
                size,
            });
        }
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

// ── ASCII tree builder ────────────────────────────────────────────────────────

fn build_tree_lines(
    dir: &std::path::Path,
    prefix: &str,
    depth: usize,
    max_depth: usize,
    skip: &[String],
    show_hidden: bool,
    exts: &Option<Vec<String>>,
    out: &mut Vec<String>,
    count: &mut usize,
    limit: usize,
) {
    if *count >= limit {
        return;
    }
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

    // Filter
    let paths: Vec<_> = paths
        .into_iter()
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if should_skip(name, skip, show_hidden) {
                return false;
            }
            if p.is_file() && !ext_match(p, exts) {
                return false;
            }
            true
        })
        .collect();

    let last_idx = paths.len().saturating_sub(1);
    for (i, path) in paths.iter().enumerate() {
        if *count >= limit {
            out.push(format!("{prefix}... (limit reached)"));
            break;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_last = i == last_idx;
        let (branch, child_prefix) = if is_last {
            ("└── ", format!("{prefix}    "))
        } else {
            ("├── ", format!("{prefix}│   "))
        };

        if path.is_dir() {
            out.push(format!("{prefix}{branch}{name}/"));
            *count += 1;
            build_tree_lines(
                path,
                &child_prefix,
                depth + 1,
                max_depth,
                skip,
                show_hidden,
                exts,
                out,
                count,
                limit,
            );
        } else {
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            out.push(format!("{prefix}{branch}{name}  ({})", human_size(size)));
            *count += 1;
        }
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_tree(
    root: &str,
    max_depth: usize,
    skip: &[String],
    show_hidden: bool,
    exts: &Option<Vec<String>>,
    limit: usize,
) -> String {
    let p = std::path::Path::new(root);
    let display_root = p.file_name().and_then(|n| n.to_str()).unwrap_or(root);

    let mut lines = vec![format!("{display_root}/")];
    let mut count = 0usize;
    build_tree_lines(
        p,
        "",
        1,
        max_depth,
        skip,
        show_hidden,
        exts,
        &mut lines,
        &mut count,
        limit,
    );
    lines.push(String::new());
    lines.push(format!("{count} entries shown"));
    lines.join("\n")
}

fn action_flat(entries: &[Entry], root: &str, limit: usize) -> String {
    let mut out = String::new();
    let files: Vec<_> = entries.iter().filter(|e| !e.is_dir).collect();
    out.push_str(&format!("{} files\n", files.len().min(limit)));
    out.push_str(&"─".repeat(60));
    out.push('\n');
    for e in files.iter().take(limit) {
        let rel = e
            .path
            .strip_prefix(root)
            .unwrap_or(&e.path)
            .to_string_lossy();
        out.push_str(&format!("{rel}  ({})\n", human_size(e.size)));
    }
    if files.len() > limit {
        out.push_str(&format!("\n... ({} more)\n", files.len() - limit));
    }
    out
}

fn action_stats(entries: &[Entry]) -> String {
    use std::collections::HashMap;

    let total_files = entries.iter().filter(|e| !e.is_dir).count();
    let total_dirs = entries.iter().filter(|e| e.is_dir).count();
    let total_size: u64 = entries.iter().map(|e| e.size).sum();

    let mut by_ext: HashMap<String, (usize, u64)> = HashMap::new();
    for e in entries.iter().filter(|e| !e.is_dir) {
        let ext = e
            .path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("(no ext)")
            .to_string();
        let entry = by_ext.entry(ext).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += e.size;
    }

    let mut ext_list: Vec<_> = by_ext.into_iter().collect();
    ext_list.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    let mut out = format!(
        "Directory stats\n{:─<40}\n{} files  {}  |  {} dirs\n\nBy extension:\n",
        "",
        total_files,
        human_size(total_size),
        total_dirs
    );

    for (ext, (count, size)) in ext_list.iter().take(30) {
        out.push_str(&format!(
            "  .{ext:<12}  {:>6} files  {}\n",
            count,
            human_size(*size)
        ));
    }
    out
}

fn action_json(
    dir: &std::path::Path,
    depth: usize,
    max_depth: usize,
    skip: &[String],
    show_hidden: bool,
    exts: &Option<Vec<String>>,
) -> Value {
    if max_depth > 0 && depth > max_depth {
        return json!(null);
    }
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or(".");
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return json!({ "name": name, "type": "dir", "children": [] }),
    };
    let mut paths: Vec<std::path::PathBuf> =
        entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();

    let mut children: Vec<Value> = Vec::new();
    for path in paths {
        let child_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if should_skip(child_name, skip, show_hidden) {
            continue;
        }
        if path.is_dir() {
            children.push(action_json(
                &path,
                depth + 1,
                max_depth,
                skip,
                show_hidden,
                exts,
            ));
        } else if ext_match(&path, exts) {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            children.push(json!({ "name": child_name, "type": "file", "size": size }));
        }
    }

    json!({ "name": name, "type": "dir", "children": children })
}

fn action_sizes(entries: &[Entry], root: &str, top: usize) -> String {
    let mut files: Vec<_> = entries.iter().filter(|e| !e.is_dir).collect();
    files.sort_by(|a, b| b.size.cmp(&a.size));

    let total: u64 = files.iter().map(|e| e.size).sum();
    let mut out = format!("Largest files (top {top}) — total {}\n", human_size(total));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for e in files.iter().take(top) {
        let rel = e
            .path
            .strip_prefix(root)
            .unwrap_or(&e.path)
            .to_string_lossy();
        let pct = if total > 0 {
            (e.size as f64 / total as f64 * 100.0) as usize
        } else {
            0
        };
        let bar = "█".repeat((pct / 4).max(1).min(25));
        out.push_str(&format!(
            "  {:>10}  {:>3}%  {bar:<25}  {rel}\n",
            human_size(e.size),
            pct
        ));
    }
    out
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let root = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("tree");
    let max_depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(4) as usize;
    let show_hidden = args
        .get("show_hidden")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(500) as usize;
    let top = args.get("top").and_then(|v| v.as_u64()).unwrap_or(30) as usize;

    let exts: Option<Vec<String>> = args
        .get("extensions")
        .and_then(|v| v.as_str())
        .map(|s| s.split(',').map(|e| e.trim().to_string()).collect());

    let mut skip_dirs = default_skip_dirs();
    if let Some(extra) = args.get("skip_dirs").and_then(|v| v.as_str()) {
        for s in extra.split(',') {
            let t = s.trim().to_string();
            if !t.is_empty() {
                skip_dirs.push(t);
            }
        }
    }

    if !std::path::Path::new(root).is_dir() {
        return Err(format!("'{root}' is not a directory or does not exist"));
    }

    match action {
        "tree" => Ok(action_tree(
            root,
            max_depth,
            &skip_dirs,
            show_hidden,
            &exts,
            limit,
        )),
        "flat" | "sizes" | "stats" => {
            let mut entries = Vec::new();
            collect_entries(
                std::path::Path::new(root),
                1,
                max_depth,
                &skip_dirs,
                show_hidden,
                &exts,
                &mut entries,
            );
            match action {
                "flat" => Ok(action_flat(&entries, root, limit)),
                "sizes" => Ok(action_sizes(&entries, root, top)),
                _ => Ok(action_stats(&entries)),
            }
        }
        "json" => {
            let tree = action_json(
                std::path::Path::new(root),
                1,
                max_depth,
                &skip_dirs,
                show_hidden,
                &exts,
            );
            Ok(serde_json::to_string_pretty(&tree).unwrap_or_else(|_| "{}".to_string()))
        }
        _ => Err(format!(
            "Unknown action '{action}'. Valid: tree, flat, stats, json, sizes"
        )),
    }
}
