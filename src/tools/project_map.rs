use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
struct ArchitectureSketch {
    relative_path: String,
    role: &'static str,
    score: i32,
    symbols: Vec<String>,
}

pub async fn map_project(args: &Value) -> Result<String, String> {
    let root = crate::tools::file_ops::workspace_root();
    let focus = args.get("focus").and_then(|v| v.as_str()).unwrap_or(".");
    let include_symbols = args
        .get("include_symbols")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let max_depth = args
        .get("max_depth")
        .and_then(value_as_usize)
        .unwrap_or(4)
        .min(6);
    let focus_root = resolve_focus_root(&root, focus)?;

    let mut report = String::new();
    report.push_str(&format!("Project Root: {}\n", root.display()));
    if focus != "." {
        report.push_str(&format!("Focus Path: {}\n", focus_root.display()));
    }

    report.push_str("\n-- Configuration DNA --\n");
    append_configuration_dna(&root, &mut report);

    let sketches = collect_architecture_sketches(&root, &focus_root, include_symbols)?;
    if !sketches.is_empty() {
        append_architecture_map(&sketches, &mut report);
    }

    report.push_str("\n-- Directory Structure --\n");
    let mut lines = Vec::new();
    build_tree(&root, &focus_root, 0, max_depth, &mut lines)?;
    report.push_str(&lines.join("\n"));

    Ok(report)
}

fn resolve_focus_root(root: &Path, focus: &str) -> Result<PathBuf, String> {
    if focus == "." {
        return Ok(root.to_path_buf());
    }

    let candidate = if Path::new(focus).is_absolute() {
        PathBuf::from(focus)
    } else {
        root.join(focus)
    };

    let canonical = candidate
        .canonicalize()
        .map_err(|e| format!("map_project: could not resolve focus '{}': {}", focus, e))?;
    crate::tools::guard::path_is_safe(root, &canonical)
        .map_err(|e| format!("map_project: invalid focus '{}': {}", focus, e))?;
    if canonical.is_file() {
        return canonical
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| format!("map_project: focus '{}' has no readable parent directory", focus));
    }
    Ok(canonical)
}

fn append_configuration_dna(root: &Path, report: &mut String) {
    let markers = [
        "Cargo.toml",
        "package.json",
        "go.mod",
        "requirements.txt",
        "pyproject.toml",
        "README.md",
        "CLAUDE.md",
        "CAPABILITIES.md",
        "Taskfile.yml",
        ".env.example",
    ];

    for marker in markers {
        let path = root.join(marker);
        if !path.exists() {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            let snippet = content.chars().take(600).collect::<String>();
            report.push_str(&format!("### File: {}\n```\n{}\n```\n", marker, snippet));
        }
    }
}

fn append_architecture_map(sketches: &[ArchitectureSketch], report: &mut String) {
    report.push_str("\n-- Architecture Map --\n");

    let entrypoints: Vec<_> = sketches
        .iter()
        .filter(|s| is_entrypoint_path(&s.relative_path))
        .collect();
    if !entrypoints.is_empty() {
        report.push_str("Likely entrypoints\n");
        for sketch in entrypoints.iter().take(4) {
            report.push_str(&format!("- {} [{}]\n", sketch.relative_path, sketch.role));
            if !sketch.symbols.is_empty() {
                report.push_str(&format!("  symbols: {}\n", sketch.symbols.join(", ")));
            }
        }
    }

    report.push_str("Core owner files\n");
    for sketch in sketches.iter().take(12) {
        report.push_str(&format!("- {} [{}]\n", sketch.relative_path, sketch.role));
        if !sketch.symbols.is_empty() {
            report.push_str(&format!("  symbols: {}\n", sketch.symbols.join(", ")));
        }
    }
}

fn collect_architecture_sketches(
    root: &Path,
    focus_root: &Path,
    include_symbols: bool,
) -> Result<Vec<ArchitectureSketch>, String> {
    let mut sketches = Vec::new();

    for entry in WalkDir::new(focus_root).follow_links(false).max_depth(4) {
        let entry = entry.map_err(|e| format!("map_project: {}", e))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if path_has_hidden_segment(path) || !is_architecture_candidate(path) {
            continue;
        }

        let relative_path = to_relative_display(root, path);
        let role = classify_file_role(&relative_path);
        let score = score_architecture_file(&relative_path);
        let symbols = if include_symbols {
            extract_top_symbols(path).unwrap_or_default()
        } else {
            Vec::new()
        };

        sketches.push(ArchitectureSketch {
            relative_path,
            role,
            score,
            symbols,
        });
    }

    sketches.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });
    sketches.truncate(18);
    Ok(sketches)
}

fn build_tree(
    root: &Path,
    dir: &Path,
    depth: usize,
    max_depth: usize,
    lines: &mut Vec<String>,
) -> Result<(), String> {
    if depth > max_depth {
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read dir {dir:?}: {}", e))?
        .filter_map(Result::ok)
        .collect();

    entries.sort_by_key(|e| {
        (
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false),
            e.file_name(),
        )
    });

    for entry in entries {
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect entry {:?}: {}", entry.path(), e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "target" || name == "node_modules" || name == "vendor" {
            continue;
        }

        let indent = "  ".repeat(depth);
        let path = entry.path();
        let rel = to_relative_display(root, &path);
        let prefix = if file_type.is_dir() { "[D]" } else { "[F]" };
        lines.push(format!("{indent}{prefix} {rel}"));

        if file_type.is_dir() {
            build_tree(root, &path, depth + 1, max_depth, lines)?;
        }
    }
    Ok(())
}

fn path_has_hidden_segment(path: &Path) -> bool {
    path.components().any(|component| {
        let segment = component.as_os_str().to_string_lossy();
        (segment.starts_with('.') && segment != "." && segment != "..")
            || segment == "target"
            || segment == "node_modules"
            || segment == "__pycache__"
    })
}

fn is_architecture_candidate(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return false;
    };
    matches!(
        ext,
        "rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "go" | "cs" | "java" | "kt"
    )
}

fn to_relative_display(root: &Path, path: &Path) -> String {
    let root_display = normalize_display_path(root);
    let path_display = normalize_display_path(path);

    if let Some(stripped) = path_display.strip_prefix(&root_display) {
        return stripped.trim_start_matches('/').to_string();
    }

    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .trim_start_matches("\\\\?\\")
        .to_string()
}

fn normalize_display_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .trim_start_matches("\\\\?\\")
        .trim_end_matches('/')
        .to_string()
}

fn value_as_usize(value: &Value) -> Option<usize> {
    if let Some(v) = value.as_u64() {
        return usize::try_from(v).ok();
    }

    if let Some(v) = value.as_f64() {
        if v.is_finite() && v >= 0.0 && v.fract() == 0.0 && v <= (usize::MAX as f64) {
            return Some(v as usize);
        }
    }

    value
        .as_str()
        .and_then(|s| s.trim().parse::<usize>().ok())
}

fn is_core_library_path(relative_path: &str) -> bool {
    relative_path.to_lowercase().ends_with("lib.rs")
}

fn is_entrypoint_path(relative_path: &str) -> bool {
    let lower = relative_path.to_lowercase();
    lower.ends_with("main.rs")
        || (lower.contains("/bin/") && lower.ends_with(".rs"))
        || lower.ends_with("app.rs")
        || lower.ends_with("server.rs")
        || lower.ends_with("cli.rs")
        || lower.ends_with("__main__.py")
        || lower.ends_with("main.py")
        || lower.ends_with("index.ts")
        || lower.ends_with("index.js")
}

fn classify_file_role(relative_path: &str) -> &'static str {
    let lower = relative_path.to_lowercase();
    if is_entrypoint_path(relative_path) {
        "entrypoint"
    } else if is_core_library_path(relative_path) {
        "core library"
    } else if lower.contains("/ui/") || lower.contains("tui") || lower.contains("voice") {
        "ui / operator surface"
    } else if lower.contains("/agent/") || lower.contains("conversation") || lower.contains("inference") {
        "agent orchestration"
    } else if lower.contains("/tools/") || lower.contains("shell") || lower.contains("git") {
        "tooling layer"
    } else if lower.contains("/memory/") || lower.contains("vein") || lower.contains("compaction") {
        "memory / retrieval"
    } else if lower.contains("/lsp/") {
        "language-server integration"
    } else {
        "workspace code"
    }
}

fn score_architecture_file(relative_path: &str) -> i32 {
    let lower = relative_path.to_lowercase();
    let mut score = 0;

    if lower.starts_with("src/") {
        score += 5;
    }
    if is_entrypoint_path(relative_path) {
        score += 12;
    } else if is_core_library_path(relative_path) {
        score += 7;
    }
    for needle in [
        "/agent/",
        "/ui/",
        "/tools/",
        "/memory/",
        "/lsp/",
        "conversation",
        "inference",
        "prompt",
        "voice",
        "tui",
        "main",
    ] {
        if lower.contains(needle) {
            score += 4;
        }
    }

    score
}

fn extract_top_symbols(path: &Path) -> Result<Vec<String>, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("symbol scan failed for {:?}: {}", path, e))?;
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or_default();
    let mut symbols = Vec::new();

    let patterns: &[&str] = match ext {
        "rs" => &[
            r"(?m)^\s*pub\s+struct\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*struct\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*pub\s+enum\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*enum\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*pub\s+trait\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*trait\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*pub\s+async\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*pub\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*async\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)",
        ],
        "py" => &[
            r"(?m)^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)",
        ],
        "ts" | "tsx" | "js" | "jsx" => &[
            r"(?m)^\s*export\s+class\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*export\s+function\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*function\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"(?m)^\s*export\s+const\s+([A-Za-z_][A-Za-z0-9_]*)",
        ],
        "go" => &[
            r"(?m)^\s*type\s+([A-Za-z_][A-Za-z0-9_]*)\s+struct",
            r"(?m)^\s*func\s+([A-Za-z_][A-Za-z0-9_]*)",
        ],
        _ => &[],
    };

    for pattern in patterns {
        let regex = regex::Regex::new(pattern).map_err(|e| format!("invalid symbol regex: {}", e))?;
        for capture in regex.captures_iter(&content) {
            let Some(name) = capture.get(1).map(|m| m.as_str().to_string()) else {
                continue;
            };
            if !symbols.contains(&name) {
                symbols.push(name);
            }
            if symbols.len() >= 4 {
                return Ok(symbols);
            }
        }
    }

    Ok(symbols)
}
