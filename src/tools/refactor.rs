use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::Path;

const MAX_FILES: usize = 4_000;

pub async fn execute_rename(args: &Value) -> Result<String, String> {
    let old_name = args
        .get("old_name")
        .and_then(|v| v.as_str())
        .ok_or("refactor_rename: missing required 'old_name'")?
        .trim();

    let new_name = args
        .get("new_name")
        .and_then(|v| v.as_str())
        .ok_or("refactor_rename: missing required 'new_name'")?
        .trim();

    if old_name.is_empty() || new_name.is_empty() {
        return Err("refactor_rename: old_name and new_name must not be empty".to_string());
    }
    if old_name == new_name {
        return Ok(
            "refactor_rename: old_name and new_name are identical — nothing to do.".to_string(),
        );
    }

    let dry_run = args
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let extensions: Vec<&str> = args
        .get("extensions")
        .and_then(|v| v.as_str())
        .map(|s| s.split(',').map(|e| e.trim()).collect())
        .unwrap_or_else(|| vec!["rs"]);

    let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
        std::path::PathBuf::from(r)
    } else {
        crate::tools::file_ops::workspace_root()
    };

    // Build whole-word regex: \bold_name\b
    let pattern = format!(r"\b{}\b", regex::escape(old_name));
    let re = Regex::new(&pattern)
        .map_err(|e| format!("refactor_rename: invalid symbol pattern: {e}"))?;

    let mut file_results: Vec<FileRenameResult> = Vec::new();
    let mut file_count = 0;

    walk_files(&root, &extensions, &mut |path: &Path| {
        if file_count >= MAX_FILES {
            return;
        }
        file_count += 1;

        let Ok(content) = fs::read_to_string(path) else {
            return;
        };

        if !re.is_match(&content) {
            return;
        }

        let rel_path = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let mut hits: Vec<RenameHit> = Vec::new();
        let mut new_lines: Vec<String> = Vec::new();

        for (idx, line) in content.lines().enumerate() {
            if re.is_match(line) {
                let replaced = re.replace_all(line, new_name).into_owned();
                let count = re.find_iter(line).count();
                hits.push(RenameHit {
                    line: idx + 1,
                    count,
                    before: line.trim().to_string(),
                    after: replaced.trim().to_string(),
                });
                new_lines.push(replaced);
            } else {
                new_lines.push(line.to_string());
            }
        }

        if hits.is_empty() {
            return;
        }

        if !dry_run {
            // Preserve original line ending style.
            let uses_crlf = content.contains("\r\n");
            let new_content = if uses_crlf {
                new_lines.join("\r\n")
            } else {
                new_lines.join("\n")
            };
            // Preserve trailing newline if original had one.
            let new_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
                format!("{new_content}\n")
            } else {
                new_content
            };
            let _ = fs::write(path, new_content.as_bytes());
        }

        let total: usize = hits.iter().map(|h| h.count).sum();
        file_results.push(FileRenameResult {
            path: rel_path,
            total_replacements: total,
            hits,
        });
    });

    format_result(old_name, new_name, dry_run, &file_results)
}

struct RenameHit {
    line: usize,
    count: usize,
    before: String,
    after: String,
}

struct FileRenameResult {
    path: String,
    total_replacements: usize,
    hits: Vec<RenameHit>,
}

fn format_result(
    old_name: &str,
    new_name: &str,
    dry_run: bool,
    results: &[FileRenameResult],
) -> Result<String, String> {
    if results.is_empty() {
        return Ok(format!(
            "refactor_rename: \"{old_name}\" not found in any tracked source file."
        ));
    }

    let total_files = results.len();
    let total_replacements: usize = results.iter().map(|r| r.total_replacements).sum();

    let action = if dry_run { "DRY RUN" } else { "APPLIED" };
    let mut out = format!(
        "REFACTOR RENAME [{action}]: \"{old_name}\" → \"{new_name}\"\n\
         {total_files} file(s), {total_replacements} replacement(s) total\n"
    );

    if dry_run {
        out.push_str("(call with dry_run=false to apply)\n");
    }

    // Show per-file breakdown — cap individual hit display to keep output readable.
    const MAX_HITS_PER_FILE: usize = 15;
    for r in results {
        out.push_str(&format!(
            "\n── {} ({} replacement(s)) ──\n",
            r.path, r.total_replacements
        ));
        let shown = r.hits.len().min(MAX_HITS_PER_FILE);
        for h in r.hits.iter().take(shown) {
            out.push_str(&format!(
                "  line {:>4}: - {}\n           + {}\n",
                h.line, h.before, h.after
            ));
        }
        if r.hits.len() > shown {
            out.push_str(&format!("  ... {} more lines\n", r.hits.len() - shown));
        }
    }

    Ok(out.trim_end().to_string())
}

fn walk_files(root: &Path, extensions: &[&str], cb: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if path.is_dir() {
            if matches!(
                name,
                "target"
                    | ".git"
                    | "node_modules"
                    | "dist"
                    | ".hematite"
                    | "__pycache__"
                    | ".venv"
                    | "build"
            ) {
                continue;
            }
            dirs.push(path);
        } else {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if extensions.contains(&ext) {
                cb(&path);
            }
        }
    }

    for dir in dirs {
        walk_files(&dir, extensions, cb);
    }
}
