use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::Path;

const MAX_FILES: usize = 4_000;
const MAX_RESULTS: usize = 40;

pub async fn execute(args: &Value) -> Result<String, String> {
    let symbol = args
        .get("symbol")
        .and_then(|v| v.as_str())
        .ok_or("find_symbol: missing required 'symbol'")?
        .trim();

    if symbol.is_empty() {
        return Err("find_symbol: 'symbol' must not be empty".to_string());
    }

    let kind_filter = args.get("kind").and_then(|v| v.as_str());
    let defs_only = args
        .get("definitions_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Allow tests or callers to inject a specific root path without touching CWD.
    let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
        std::path::PathBuf::from(r)
    } else {
        crate::tools::file_ops::workspace_root()
    };

    let results = search_workspace(&root, symbol, kind_filter, defs_only)?;

    if results.is_empty() {
        return Ok(format!(
            "find_symbol: no {} found for \"{symbol}\"{}",
            if defs_only { "definitions" } else { "matches" },
            kind_filter
                .map(|k| format!(" (kind={k})"))
                .unwrap_or_default()
        ));
    }

    let mut out = format!(
        "SYMBOL SEARCH: \"{symbol}\" — {} result(s)\n\n",
        results.len()
    );
    for r in &results {
        out.push_str(&format!(
            "{}:{} [{}]\n  {}\n\n",
            r.path,
            r.line,
            r.kind,
            r.snippet.trim()
        ));
    }
    Ok(out.trim_end().to_string())
}

struct SymbolHit {
    path: String,
    line: usize,
    kind: String,
    snippet: String,
}

/// Declaration patterns for each Rust item kind.
/// Each pattern captures visibility + keyword + optional generics + the symbol name.
/// We require a word boundary after the symbol so `run` doesn't match `run_turn`.
struct KindPattern {
    kind: &'static str,
    /// Applied to each trimmed source line.
    re: Regex,
}

fn build_patterns(symbol: &str) -> Vec<KindPattern> {
    // Escape the symbol for use in a regex.
    let sym = regex::escape(symbol);

    let defs: &[(&str, String)] = &[
        (
            "fn",
            format!(r"(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+{sym}\s*(?:<|[\(\s{{])"),
        ),
        (
            "struct",
            format!(r"(?:pub(?:\([^)]*\))?\s+)?struct\s+{sym}\s*(?:<|\{{|\()"),
        ),
        (
            "enum",
            format!(r"(?:pub(?:\([^)]*\))?\s+)?enum\s+{sym}\s*(?:<|\{{)"),
        ),
        (
            "trait",
            format!(r"(?:pub(?:\([^)]*\))?\s+)?trait\s+{sym}\s*(?:<|\{{|:|\s)"),
        ),
        (
            "impl",
            format!(r"impl(?:<[^>]*>)?\s+(?:[A-Za-z0-9_]+\s+for\s+)?{sym}\s*(?:<|\{{)"),
        ),
        (
            "type",
            format!(r"(?:pub(?:\([^)]*\))?\s+)?type\s+{sym}\s*(?:=|<)"),
        ),
        (
            "const",
            format!(r"(?:pub(?:\([^)]*\))?\s+)?const\s+{sym}\s*:"),
        ),
        (
            "static",
            format!(r"(?:pub(?:\([^)]*\))?\s+)?static\s+(?:mut\s+)?{sym}\s*:"),
        ),
        (
            "mod",
            format!(r"(?:pub(?:\([^)]*\))?\s+)?mod\s+{sym}\s*(?:\{{|;)"),
        ),
        ("macro", format!(r"macro_rules!\s+{sym}\s*\{{")),
    ];

    defs.iter()
        .filter_map(|(kind, pat)| Regex::new(pat).ok().map(|re| KindPattern { kind, re }))
        .collect()
}

fn build_usage_pattern(symbol: &str) -> Option<Regex> {
    let sym = regex::escape(symbol);
    // Match the symbol as a standalone word — avoids substring false positives.
    Regex::new(&format!(r"\b{sym}\b")).ok()
}

fn search_workspace(
    root: &Path,
    symbol: &str,
    kind_filter: Option<&str>,
    defs_only: bool,
) -> Result<Vec<SymbolHit>, String> {
    let patterns = build_patterns(symbol);
    let usage_re = if !defs_only {
        build_usage_pattern(symbol)
    } else {
        None
    };

    let mut results: Vec<SymbolHit> = Vec::new();
    let mut file_count = 0;

    walk_rs_files(root, &mut |path: &Path| {
        if file_count >= MAX_FILES || results.len() >= MAX_RESULTS {
            return;
        }
        file_count += 1;

        let Ok(content) = fs::read_to_string(path) else {
            return;
        };
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Skip comments and string literals (best-effort).
            if trimmed.starts_with("//") || trimmed.starts_with("*") {
                continue;
            }

            if results.len() >= MAX_RESULTS {
                break;
            }

            // Try definition patterns.
            let mut matched_kind: Option<&str> = None;
            for kp in &patterns {
                if let Some(f) = kind_filter {
                    if kp.kind != f {
                        continue;
                    }
                }
                if kp.re.is_match(trimmed) {
                    matched_kind = Some(kp.kind);
                    break;
                }
            }

            if let Some(kind) = matched_kind {
                results.push(SymbolHit {
                    path: rel_path.clone(),
                    line: line_idx + 1,
                    kind: kind.to_string(),
                    snippet: line.to_string(),
                });
                continue;
            }

            // Usage scan (only when defs_only=false and no definition matched).
            if !defs_only {
                if let Some(ref re) = usage_re {
                    if re.is_match(trimmed) {
                        results.push(SymbolHit {
                            path: rel_path.clone(),
                            line: line_idx + 1,
                            kind: "usage".to_string(),
                            snippet: line.to_string(),
                        });
                    }
                }
            }
        }
    });

    // Sort: definitions before usages, then by path + line.
    results.sort_by(|a, b| {
        let a_def = a.kind != "usage";
        let b_def = b.kind != "usage";
        b_def
            .cmp(&a_def)
            .then(a.path.cmp(&b.path))
            .then(a.line.cmp(&b.line))
    });

    Ok(results)
}

fn walk_rs_files(root: &Path, cb: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip known noise directories.
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
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            cb(&path);
        }
    }

    for dir in dirs {
        walk_rs_files(&dir, cb);
    }
}
