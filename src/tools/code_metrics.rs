use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024; // 2 MB
const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "vendor",
    ".venv",
    "venv",
    "__pycache__",
    "dist",
    ".next",
    ".nuxt",
    "build",
    "out",
    ".hematite",
];
const SKIP_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "svg", "woff", "woff2", "ttf", "otf", "eot", "mp3", "mp4",
    "wav", "ogg", "pdf", "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "exe", "dll", "so", "dylib",
    "pdb", "lib", "a", "bin", "onnx",
];

struct FileStats {
    path: String,
    ext: String,
    lines: usize,
    code_lines: usize,
    comment_lines: usize,
    blank_lines: usize,
    todos: usize,
    fixmes: usize,
    is_test: bool,
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let scan_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

    let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
        PathBuf::from(r)
    } else {
        crate::tools::file_ops::workspace_root()
    };

    let target = if scan_path == "." {
        root.clone()
    } else {
        root.join(scan_path)
    };

    if !target.exists() {
        return Err(format!(
            "code_metrics: path not found: {}",
            target.display()
        ));
    }

    let mut all_files: Vec<FileStats> = Vec::new();
    collect_files(&target, &mut all_files);

    if all_files.is_empty() {
        return Ok("code_metrics: no source files found.".to_string());
    }

    format_report(&all_files, &target)
}

fn collect_files(dir: &Path, out: &mut Vec<FileStats>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if path.is_dir() {
            if SKIP_DIRS.contains(&name) {
                continue;
            }
            collect_files(&path, out);
        } else if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if SKIP_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.len() > MAX_FILE_SIZE {
                    continue;
                }
            }
            if let Some(stats) = analyze_file(&path, &ext) {
                out.push(stats);
            }
        }
    }
}

fn analyze_file(path: &Path, ext: &str) -> Option<FileStats> {
    let content = std::fs::read(path).ok()?;
    // Skip binary files
    let probe = &content[..content.len().min(8192)];
    if probe.contains(&0u8) {
        return None;
    }
    let text = std::str::from_utf8(&content).ok()?;

    let is_test = is_test_file(path, text, ext);
    let mut lines = 0usize;
    let mut code_lines = 0usize;
    let mut comment_lines = 0usize;
    let mut blank_lines = 0usize;
    let mut todos = 0usize;
    let mut fixmes = 0usize;

    for line in text.lines() {
        lines += 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_lines += 1;
            continue;
        }
        let lower = trimmed.to_lowercase();
        if is_comment_line(trimmed, ext) {
            comment_lines += 1;
        } else {
            code_lines += 1;
        }
        if lower.contains("todo") || lower.contains("to-do") {
            todos += 1;
        }
        if lower.contains("fixme")
            || lower.contains("fix me")
            || lower.contains("hack")
            || lower.contains("xxx")
        {
            fixmes += 1;
        }
    }

    Some(FileStats {
        path: path.to_string_lossy().to_string(),
        ext: ext.to_string(),
        lines,
        code_lines,
        comment_lines,
        blank_lines,
        todos,
        fixmes,
        is_test,
    })
}

fn is_comment_line(trimmed: &str, ext: &str) -> bool {
    match ext {
        "rs" | "c" | "cpp" | "cc" | "h" | "hpp" | "java" | "js" | "ts" | "jsx" | "tsx" | "cs"
        | "go" | "swift" | "kt" | "scala" | "dart" => {
            trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*")
        }
        "py" | "rb" | "sh" | "bash" | "zsh" | "fish" | "yaml" | "yml" | "toml" | "ini" | "conf"
        | "cfg" | "r" => trimmed.starts_with('#'),
        "html" | "xml" | "svg" => trimmed.starts_with("<!--"),
        "css" | "scss" | "sass" => trimmed.starts_with("/*") || trimmed.starts_with("*"),
        "lua" => trimmed.starts_with("--"),
        "sql" => trimmed.starts_with("--"),
        _ => false,
    }
}

fn is_test_file(path: &Path, content: &str, ext: &str) -> bool {
    let name = path.to_string_lossy().to_lowercase();
    if name.contains("test") || name.contains("spec") || name.contains("__tests__") {
        return true;
    }
    match ext {
        "rs" => content.contains("#[test]") || content.contains("#[cfg(test)]"),
        "py" => {
            name.contains("test_") || content.contains("def test_") || content.contains("unittest")
        }
        "js" | "ts" | "jsx" | "tsx" => {
            name.ends_with(".test.js")
                || name.ends_with(".spec.js")
                || name.ends_with(".test.ts")
                || name.ends_with(".spec.ts")
                || content.contains("describe(") && content.contains("it(")
        }
        "go" => name.ends_with("_test.go"),
        "java" => content.contains("@Test"),
        _ => false,
    }
}

fn format_report(files: &[FileStats], root: &Path) -> Result<String, String> {
    // Aggregate by extension
    let mut by_ext: BTreeMap<&str, (usize, usize, usize, usize)> = BTreeMap::new(); // ext -> (files, lines, code, test_files)
    let mut total_lines = 0usize;
    let mut total_code = 0usize;
    let mut total_comments = 0usize;
    let mut total_blanks = 0usize;
    let mut total_todos = 0usize;
    let mut total_fixmes = 0usize;
    let mut test_files = 0usize;
    let mut test_lines = 0usize;
    let mut src_lines = 0usize;

    for f in files {
        total_lines += f.lines;
        total_code += f.code_lines;
        total_comments += f.comment_lines;
        total_blanks += f.blank_lines;
        total_todos += f.todos;
        total_fixmes += f.fixmes;
        if f.is_test {
            test_files += 1;
            test_lines += f.code_lines;
        } else {
            src_lines += f.code_lines;
        }
        let e = by_ext.entry(f.ext.as_str()).or_default();
        e.0 += 1;
        e.1 += f.lines;
        e.2 += f.code_lines;
        if f.is_test {
            e.3 += 1;
        }
    }

    // Top 10 largest files by line count
    let mut sorted: Vec<&FileStats> = files.iter().collect();
    sorted.sort_by(|a, b| b.lines.cmp(&a.lines));
    let top_files = &sorted[..sorted.len().min(10)];

    let root_str = root.to_string_lossy();
    let test_ratio = if src_lines + test_lines > 0 {
        (test_lines as f64 / (src_lines + test_lines) as f64) * 100.0
    } else {
        0.0
    };

    let mut out = format!(
        "code_metrics [{}]\n{}\n\n",
        root.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&root_str),
        "─".repeat(60)
    );

    out.push_str(&format!(
        "SUMMARY\n\
         Files scanned   : {:>8}\n\
         Total lines     : {:>8}\n\
         Code lines      : {:>8}\n\
         Comment lines   : {:>8}\n\
         Blank lines     : {:>8}\n\
         TODOs / FIXMEs  : {:>8} / {:>4}\n\
         Test files      : {:>8}  ({test_files} of {} files)\n\
         Test line ratio : {:>7.1}%\n",
        files.len(),
        total_lines,
        total_code,
        total_comments,
        total_blanks,
        total_todos,
        total_fixmes,
        test_files,
        files.len(),
        test_ratio,
    ));

    out.push_str("\nBY LANGUAGE (top 12 by code lines)\n");
    type ExtEntry<'a> = (&'a str, (usize, usize, usize, usize));
    let mut ext_vec: Vec<ExtEntry<'_>> = by_ext.into_iter().collect();
    ext_vec.sort_by(|a, b| b.1 .2.cmp(&a.1 .2));
    for (ext, (file_count, lines, code, _)) in ext_vec.iter().take(12) {
        out.push_str(&format!(
            "  {:12}  {:>5} files  {:>8} lines  {:>8} code\n",
            if ext.is_empty() { "(no ext)" } else { ext },
            file_count,
            lines,
            code
        ));
    }

    out.push_str("\nLARGEST FILES (by total lines)\n");
    for f in top_files {
        let rel = f
            .path
            .strip_prefix(root_str.as_ref())
            .unwrap_or(&f.path)
            .trim_start_matches(['/', '\\']);
        let test_tag = if f.is_test { " [test]" } else { "" };
        out.push_str(&format!("  {:>6} lines  {}{}\n", f.lines, rel, test_tag));
    }

    if total_todos > 0 || total_fixmes > 0 {
        out.push_str(&format!(
            "\nATTENTION ITEMS\n\
             TODOs  : {total_todos}\n\
             FIXMEs : {total_fixmes}\n\
             Tip: run grep_files with pattern 'TODO|FIXME|HACK|XXX' to locate them.\n"
        ));
    }

    out.push_str(&format!(
        "\nTest coverage proxy: {:.1}% of code lines are in test files.",
        test_ratio
    ));

    Ok(out)
}
