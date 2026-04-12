use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

// ── Ghost Ledger ──────────────────────────────────────────────────────────────

const MAX_GHOST_BACKUPS: usize = 8;

fn prune_ghost_backups(ghost_dir: &Path) {
    let Ok(entries) = fs::read_dir(ghost_dir) else {
        return;
    };

    let mut backups: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("bak"))
                .unwrap_or(false)
        })
        .collect();

    backups.sort_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok());
    backups.reverse();

    let retained: std::collections::HashSet<String> = backups
        .iter()
        .take(MAX_GHOST_BACKUPS)
        .map(|entry| entry.path().to_string_lossy().replace('\\', "/"))
        .collect();

    for entry in backups.into_iter().skip(MAX_GHOST_BACKUPS) {
        let _ = fs::remove_file(entry.path());
    }

    let ledger_path = ghost_dir.join("ledger.txt");
    let Ok(content) = fs::read_to_string(&ledger_path) else {
        return;
    };

    let filtered_lines: Vec<String> = content
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(2, '|').collect();
            if parts.len() != 2 {
                return None;
            }

            let backup_path = parts[1].replace('\\', "/");
            if retained.contains(&backup_path) {
                Some(line.to_string())
            } else {
                None
            }
        })
        .collect();

    let rewritten = if filtered_lines.is_empty() {
        String::new()
    } else {
        filtered_lines.join("\n") + "\n"
    };
    let _ = fs::write(ledger_path, rewritten);
}

fn save_ghost_backup(target_path: &str, content: &str) {
    let ws = workspace_root();

    // Phase 1: Try Git Ghost Snapshot
    if crate::agent::git::is_git_repo(&ws) {
        let _ = crate::agent::git::create_ghost_snapshot(&ws);
    }

    // Phase 2: Fallback to local file backup (Ghost Ledger)
    let ghost_dir = ws.join(".hematite").join("ghost");
    let _ = fs::create_dir_all(&ghost_dir);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let safe_name = Path::new(target_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let backup_file = ghost_dir.join(format!("{}_{}.bak", ts, safe_name));

    if fs::write(&backup_file, content).is_ok() {
        use std::io::Write;
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(ghost_dir.join("ledger.txt"))
        {
            let _ = writeln!(f, "{}|{}", target_path, backup_file.display());
        }
        prune_ghost_backups(&ghost_dir);
    }
}

pub fn pop_ghost_ledger() -> Result<String, String> {
    let ws = workspace_root();
    let ghost_dir = ws.join(".hematite").join("ghost");
    let ledger_path = ghost_dir.join("ledger.txt");

    if !ledger_path.exists() {
        return Err("Ghost Ledger is empty — no edits to undo".into());
    }

    let content = fs::read_to_string(&ledger_path).map_err(|e| e.to_string())?;
    let mut lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

    if lines.is_empty() {
        return Err("Ghost Ledger is empty".into());
    }

    let last_line = lines.pop().unwrap();
    let parts: Vec<&str> = last_line.splitn(2, '|').collect();
    if parts.len() != 2 {
        return Err("Corrupted ledger entry".into());
    }

    let target_path = parts[0];
    let backup_path = parts[1];

    // Priority 1: Try Git Rollback
    if crate::agent::git::is_git_repo(&ws) {
        if let Ok(msg) = crate::agent::git::revert_from_ghost(&ws, target_path) {
            let _ = fs::remove_file(backup_path);
            let new_ledger = lines.join("\n");
            let _ = fs::write(
                &ledger_path,
                if new_ledger.is_empty() {
                    String::new()
                } else {
                    new_ledger + "\n"
                },
            );
            return Ok(msg);
        }
    }

    // Priority 2: Standard File Rollback
    let original_content =
        fs::read_to_string(backup_path).map_err(|e| format!("Failed to read backup: {e}"))?;
    let abs_target = ws.join(target_path);
    fs::write(&abs_target, original_content).map_err(|e| format!("Failed to restore file: {e}"))?;

    let new_ledger = lines.join("\n");
    let _ = fs::write(
        &ledger_path,
        if new_ledger.is_empty() {
            String::new()
        } else {
            new_ledger + "\n"
        },
    );
    let _ = fs::remove_file(backup_path);

    Ok(format!("Restored {} from Ghost Ledger", target_path))
}

// ── read_file ─────────────────────────────────────────────────────────────────

pub async fn read_file(args: &Value) -> Result<String, String> {
    let path = require_str(args, "path")?;
    let offset = get_usize_arg(args, "offset");
    let limit = get_usize_arg(args, "limit");

    let abs = safe_path(path)?;
    let raw = fs::read_to_string(&abs).map_err(|e| format!("read_file: {e} ({path})"))?;

    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();
    let start = offset.unwrap_or(0).min(total);
    let end = limit.map(|n| (start + n).min(total)).unwrap_or(total);

    let mut content = lines[start..end].join("\n");
    if end < total {
        content.push_str("\n\n--- [TRUNCATION WARNING] ---\n");
        content.push_str(&format!("This file has {} more lines below. ", total - end));
        content.push_str("To read more, use `read_file` with a higher `offset` OR use `inspect_lines` to find relevant blocks. \
                         Do NOT attempt to read the entire large file at once if it keeps truncating.");
    }

    Ok(format!(
        "[{path}  lines {}-{} of {}]\n{}",
        start + 1,
        end,
        total,
        content
    ))
}

// ── inspect_lines ─────────────────────────────────────────────────────────────

pub async fn inspect_lines(args: &Value) -> Result<String, String> {
    let path = require_str(args, "path")?;
    let start_line = get_usize_arg(args, "start_line").unwrap_or(1);
    let end_line = get_usize_arg(args, "end_line");

    let abs = safe_path(path)?;
    let raw = fs::read_to_string(&abs).map_err(|e| format!("inspect_lines: {e} ({path})"))?;

    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();

    let start = start_line.saturating_sub(1).min(total);
    let end = end_line.unwrap_or(total).min(total);

    if start >= end && total > 0 {
        return Err(format!(
            "inspect_lines: start_line ({start_line}) must be <= end_line ({})",
            end_line.unwrap_or(total)
        ));
    }

    let mut output = format!(
        "[inspect_lines: {path} lines {}-{} of {}]\n",
        start + 1,
        end,
        total
    );
    for i in start..end {
        output.push_str(&format!("[{:>4}] | {}\n", i + 1, lines[i]));
    }

    Ok(output)
}

// ── write_file ────────────────────────────────────────────────────────────────

pub async fn write_file(args: &Value) -> Result<String, String> {
    let path = require_str(args, "path")?;
    let content = require_str(args, "content")?;

    let abs = safe_path_allow_new(path)?;
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("write_file: could not create dirs: {e}"))?;
    }

    let existed = abs.exists();
    if existed {
        if let Ok(orig) = fs::read_to_string(&abs) {
            save_ghost_backup(path, &orig);
        }
    }

    fs::write(&abs, content).map_err(|e| format!("write_file: {e} ({path})"))?;

    let action = if existed { "Updated" } else { "Created" };
    Ok(format!("{action} {path}  ({} bytes)", content.len()))
}

// ── edit_file ─────────────────────────────────────────────────────────────────

pub async fn edit_file(args: &Value) -> Result<String, String> {
    let path = require_str(args, "path")?;
    let search = require_str(args, "search")?;
    let replace = require_str(args, "replace")?;
    let replace_all = args
        .get("replace_all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if search == replace {
        return Err("edit_file: 'search' and 'replace' are identical — no change needed".into());
    }

    let abs = safe_path(path)?;
    let raw = fs::read_to_string(&abs).map_err(|e| format!("edit_file: {e} ({path})"))?;
    // Normalize CRLF → LF so search strings from the model (always LF) match on Windows.
    let original = raw.replace("\r\n", "\n");

    save_ghost_backup(path, &original);

    let search_trimmed = search.trim();
    let search_non_ws_len = search_trimmed
        .chars()
        .filter(|c| !c.is_whitespace())
        .count();
    let search_line_count = search_trimmed.lines().count();
    if search_non_ws_len < 12 && search_line_count <= 1 {
        return Err(format!(
            "edit_file: search string is too short or generic for a safe mutation in {path}.\n\
             Provide a more specific anchor (prefer a full line, multiple lines, or use `inspect_lines` + `patch_hunk`)."
        ));
    }

    // ── Exact match first ────────────────────────────────────────────────────
    let (effective_search, was_repaired) = if original.contains(search) {
        let exact_match_count = original.matches(search).count();
        if exact_match_count > 1 && !replace_all {
            return Err(format!(
                "edit_file: search string matched {} times in {path}.\n\
                 Provide a more specific unique anchor or use `inspect_lines` + `patch_hunk`.",
                exact_match_count
            ));
        }
        (search.to_string(), false)
    } else {
        // ── Fuzzy repair: try whitespace-normalised match ─────────────────
        // Local models commonly produce search strings with wrong indentation,
        // trailing spaces, or CRLF/LF mismatches.  We normalise both sides and
        // find the real span in the file, then apply the replacement there.
        match fuzzy_find_span(&original, search) {
            Some(span) => {
                // Extract the exact slice from the file so we can replace it.
                let real_slice = original[span.clone()].to_string();
                (real_slice, true)
            }
            None => {
                let hint = nearest_lines(&original, search);
                return Err(format!(
                    "edit_file: search string not found in {path}.\n\
                     The 'search' value must match the file content exactly \
                     (including whitespace/indentation).\n\
                     {hint}"
                ));
            }
        }
    };

    let updated = if replace_all {
        original.replace(effective_search.as_str(), replace)
    } else {
        original.replacen(effective_search.as_str(), replace, 1)
    };

    fs::write(&abs, &updated).map_err(|e| format!("edit_file: write failed: {e}"))?;

    let removed = original.lines().count();
    let added = updated.lines().count();
    let repair_note = if was_repaired {
        "  [whitespace auto-corrected]"
    } else {
        ""
    };

    let mut diff_block = String::new();
    diff_block.push_str("\n--- DIFF \n");
    for line in effective_search.lines() {
        diff_block.push_str(&format!("- {}\n", line));
    }
    for line in replace.lines() {
        diff_block.push_str(&format!("+ {}\n", line));
    }

    Ok(format!(
        "Edited {path}  ({} -> {} lines){repair_note}{}",
        removed, added, diff_block
    ))
}

// ── patch_hunk ────────────────────────────────────────────────────────────────

pub async fn patch_hunk(args: &Value) -> Result<String, String> {
    let path = require_str(args, "path")?;
    let start_line = require_usize(args, "start_line")?;
    let end_line = require_usize(args, "end_line")?;
    let replacement = require_str(args, "replacement")?;

    let abs = safe_path(path)?;
    let original = fs::read_to_string(&abs).map_err(|e| format!("patch_hunk: {e} ({path})"))?;

    save_ghost_backup(path, &original);

    let lines: Vec<String> = original.lines().map(|s| s.to_string()).collect();
    let total = lines.len();

    if start_line < 1 || start_line > total || end_line < start_line || end_line > total {
        return Err(format!(
            "patch_hunk: invalid line range {}-{} for file with {} lines",
            start_line, end_line, total
        ));
    }

    let mut updated_lines = Vec::new();
    // 0-indexed adjustment
    let s_idx = start_line - 1;
    let e_idx = end_line; // inclusive in current logic from 1-based start_line..end_line

    // 1. Lines before the hunk
    updated_lines.extend_from_slice(&lines[0..s_idx]);

    // 2. The hunk replacement
    for line in replacement.lines() {
        updated_lines.push(line.to_string());
    }

    // 3. Lines after the hunk
    if e_idx < total {
        updated_lines.extend_from_slice(&lines[e_idx..total]);
    }

    let updated_content = updated_lines.join("\n");
    fs::write(&abs, &updated_content).map_err(|e| format!("patch_hunk: write failed: {e}"))?;

    let mut diff = String::new();
    diff.push_str("\n--- HUNK DIFF ---\n");
    for i in s_idx..e_idx {
        diff.push_str(&format!("- {}\n", lines[i].trim_end()));
    }
    for line in replacement.lines() {
        diff.push_str(&format!("+ {}\n", line.trim_end()));
    }

    Ok(format!(
        "Patched {path} lines {}-{} ({} -> {} lines){}",
        start_line,
        end_line,
        (e_idx - s_idx),
        replacement.lines().count(),
        diff
    ))
}

// ── multi_search_replace ──────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct SearchReplaceHunk {
    search: String,
    replace: String,
}

pub async fn multi_search_replace(args: &Value) -> Result<String, String> {
    let path = require_str(args, "path")?;
    let hunks_val = args
        .get("hunks")
        .ok_or_else(|| "multi_search_replace requires 'hunks' array".to_string())?;

    let hunks: Vec<SearchReplaceHunk> = serde_json::from_value(hunks_val.clone())
        .map_err(|e| format!("multi_search_replace: invalid hunks array: {e}"))?;

    if hunks.is_empty() {
        return Err("multi_search_replace: hunks array is empty".to_string());
    }

    let abs = safe_path(path)?;
    let raw =
        fs::read_to_string(&abs).map_err(|e| format!("multi_search_replace: {e} ({path})"))?;
    // Normalize CRLF → LF so search strings from the model (always LF) match on Windows.
    let original = raw.replace("\r\n", "\n");

    save_ghost_backup(path, &original);

    let mut current_content = original.clone();
    let mut diff = String::new();
    diff.push_str("\n--- SEARCH & REPLACE DIFF ---\n");

    let mut patched_hunks = 0;

    for (i, hunk) in hunks.iter().enumerate() {
        let match_count = current_content.matches(&hunk.search).count();
        if match_count == 0 {
            return Err(format!("multi_search_replace: hunk {} search string not found in file. Ensure exact whitespace match.", i));
        }
        if match_count > 1 {
            return Err(format!("multi_search_replace: hunk {} search string matched {} times. Provide more context to make it unique.", i, match_count));
        }

        diff.push_str(&format!("\n@@ Hunk {} @@\n", i + 1));
        for line in hunk.search.lines() {
            diff.push_str(&format!("- {}\n", line.trim_end()));
        }
        for line in hunk.replace.lines() {
            diff.push_str(&format!("+ {}\n", line.trim_end()));
        }

        current_content = current_content.replace(&hunk.search, &hunk.replace);
        patched_hunks += 1;
    }

    fs::write(&abs, &current_content)
        .map_err(|e| format!("multi_search_replace: write failed: {e}"))?;

    Ok(format!(
        "Modified {} hunks in {} using exact search-and-replace.{}",
        patched_hunks, path, diff
    ))
}

// ── list_files ────────────────────────────────────────────────────────────────

pub async fn list_files(args: &Value) -> Result<String, String> {
    let started = Instant::now();
    let base_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let ext_filter = args.get("extension").and_then(|v| v.as_str());

    let base = safe_path(base_str)?;

    let mut files: Vec<PathBuf> = Vec::new();
    let mut scanned_count = 0;
    for entry in WalkDir::new(&base).follow_links(false) {
        scanned_count += 1;
        if scanned_count > 25_000 {
            return Err("list_files: Too many files scanned (>25,000). The path is too broad. Narrow your search path or run Hematite directly in a project directory.".into());
        }
        let entry = entry.map_err(|e| format!("list_files: {e}"))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();

        // Skip hidden dirs / target / node_modules
        if path_has_hidden_segment(p) {
            continue;
        }

        if let Some(ext) = ext_filter {
            if p.extension().and_then(|s| s.to_str()) != Some(ext) {
                continue;
            }
        }
        files.push(p.to_path_buf());
    }

    // Sort by modification time (newest first).
    files.sort_by_key(|p| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .map(std::cmp::Reverse)
    });

    let total = files.len();
    const LIMIT: usize = 200;
    let truncated = total > LIMIT;
    let shown: Vec<String> = files
        .into_iter()
        .take(LIMIT)
        .map(|p| p.display().to_string())
        .collect();

    let ms = started.elapsed().as_millis();
    let mut out = format!(
        "{} file(s) in {}  ({ms}ms){}",
        total.min(LIMIT),
        base_str,
        if truncated {
            "  [truncated at 200]"
        } else {
            ""
        }
    );
    out.push('\n');
    out.push_str(&shown.join("\n"));
    Ok(out)
}

// ── grep_files ────────────────────────────────────────────────────────────────

pub async fn grep_files(args: &Value) -> Result<String, String> {
    let pattern = require_str(args, "pattern")?;
    let base_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let ext_filter = args.get("extension").and_then(|v| v.as_str());
    let case_insensitive = args
        .get("case_insensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let files_only = args.get("mode").and_then(|v| v.as_str()) == Some("files_only");
    let head_limit = get_usize_arg(args, "head_limit").unwrap_or(50);
    let offset = get_usize_arg(args, "offset").unwrap_or(0);

    // Context lines: `context` sets both before+after; `before`/`after` override individually.
    let ctx_default = get_usize_arg(args, "context").unwrap_or(0);
    let before = get_usize_arg(args, "before").unwrap_or(ctx_default);
    let after = get_usize_arg(args, "after").unwrap_or(ctx_default);

    let base = safe_path(base_str)?;

    let regex = regex::RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()
        .map_err(|e| format!("grep_files: invalid pattern '{pattern}': {e}"))?;

    // ── files_only mode ───────────────────────────────────────────────────────
    if files_only {
        let mut matched_files: Vec<String> = Vec::new();
        let mut scanned_count = 0;

        for entry in WalkDir::new(&base).follow_links(false) {
            scanned_count += 1;
            if scanned_count > 25_000 {
                return Err("grep_files: Too many files scanned (>25,000). The path is too broad. Narrow your search path or run Hematite directly in a project directory.".into());
            }
            let entry = entry.map_err(|e| format!("grep_files: {e}"))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let p = entry.path();
            if path_has_hidden_segment(p) {
                continue;
            }
            if let Some(ext) = ext_filter {
                if p.extension().and_then(|s| s.to_str()) != Some(ext) {
                    continue;
                }
            }
            let Ok(contents) = fs::read_to_string(p) else {
                continue;
            };
            if contents.lines().any(|line| regex.is_match(line)) {
                matched_files.push(p.display().to_string());
            }
        }

        if matched_files.is_empty() {
            return Ok(format!("No files matching '{pattern}' in {base_str}"));
        }

        let total = matched_files.len();
        let page: Vec<_> = matched_files
            .into_iter()
            .skip(offset)
            .take(head_limit)
            .collect();
        let showing = page.len();
        let mut out = format!("{total} file(s) match '{pattern}'");
        if offset > 0 || showing < total {
            out.push_str(&format!(
                " [showing {}-{} of {total}]",
                offset + 1,
                offset + showing
            ));
        }
        out.push('\n');
        out.push_str(&page.join("\n"));
        return Ok(out);
    }

    // ── content mode with optional context lines ──────────────────────────────

    // A "hunk" is a contiguous run of lines to display for one or more nearby matches.
    struct Hunk {
        path: String,
        /// (line_number_1_indexed, line_text, is_match)
        lines: Vec<(usize, String, bool)>,
    }

    let mut hunks: Vec<Hunk> = Vec::new();
    let mut total_matches = 0usize;
    let mut files_matched = 0usize;
    let mut scanned_count = 0;

    for entry in WalkDir::new(&base).follow_links(false) {
        scanned_count += 1;
        if scanned_count > 25_000 {
            return Err("grep_files: Too many files scanned (>25,000). The path is too broad. Narrow your search path or run Hematite directly in a project directory.".into());
        }
        let entry = entry.map_err(|e| format!("grep_files: {e}"))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        if path_has_hidden_segment(p) {
            continue;
        }
        if let Some(ext) = ext_filter {
            if p.extension().and_then(|s| s.to_str()) != Some(ext) {
                continue;
            }
        }
        let Ok(contents) = fs::read_to_string(p) else {
            continue;
        };
        let all_lines: Vec<&str> = contents.lines().collect();
        let n = all_lines.len();

        // Find all match indices in this file.
        let match_idxs: Vec<usize> = all_lines
            .iter()
            .enumerate()
            .filter(|(_, line)| regex.is_match(line))
            .map(|(i, _)| i)
            .collect();

        if match_idxs.is_empty() {
            continue;
        }
        files_matched += 1;
        total_matches += match_idxs.len();

        // Merge overlapping ranges into hunks.
        let path_str = p.display().to_string();
        let mut ranges: Vec<(usize, usize)> = match_idxs
            .iter()
            .map(|&i| {
                (
                    i.saturating_sub(before),
                    (i + after).min(n.saturating_sub(1)),
                )
            })
            .collect();

        // Sort and merge overlapping ranges.
        ranges.sort_unstable();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (s, e) in ranges {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 + 1 {
                    last.1 = last.1.max(e);
                    continue;
                }
            }
            merged.push((s, e));
        }

        // Build hunks from merged ranges.
        let match_set: std::collections::HashSet<usize> = match_idxs.into_iter().collect();
        for (start, end) in merged {
            let mut hunk_lines = Vec::new();
            for i in start..=end {
                hunk_lines.push((i + 1, all_lines[i].to_string(), match_set.contains(&i)));
            }
            hunks.push(Hunk {
                path: path_str.clone(),
                lines: hunk_lines,
            });
        }
    }

    if hunks.is_empty() {
        return Ok(format!("No matches for '{pattern}' in {base_str}"));
    }

    let total_hunks = hunks.len();
    let page_hunks: Vec<_> = hunks.into_iter().skip(offset).take(head_limit).collect();
    let showing = page_hunks.len();

    let mut out =
        format!("{total_matches} match(es) across {files_matched} file(s), {total_hunks} hunk(s)");
    if offset > 0 || showing < total_hunks {
        out.push_str(&format!(
            " [hunks {}-{} of {total_hunks}]",
            offset + 1,
            offset + showing
        ));
    }
    out.push('\n');

    for (i, hunk) in page_hunks.iter().enumerate() {
        if i > 0 {
            out.push_str("\n--\n");
        }
        for (lineno, text, is_match) in &hunk.lines {
            if *is_match {
                out.push_str(&format!("{}:{}:{}\n", hunk.path, lineno, text));
            } else {
                out.push_str(&format!("{}: {}-{}\n", hunk.path, lineno, text));
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── Argument helpers ──────────────────────────────────────────────────────────

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing required argument: '{key}'"))
}

fn get_usize_arg(args: &Value, key: &str) -> Option<usize> {
    args.get(key).and_then(value_as_usize)
}

fn require_usize(args: &Value, key: &str) -> Result<usize, String> {
    get_usize_arg(args, key).ok_or_else(|| format!("Missing required numeric argument: '{key}'"))
}

fn value_as_usize(value: &Value) -> Option<usize> {
    if let Some(v) = value.as_u64() {
        return usize::try_from(v).ok();
    }

    if let Some(v) = value.as_i64() {
        return if v >= 0 {
            usize::try_from(v as u64).ok()
        } else {
            None
        };
    }

    if let Some(v) = value.as_f64() {
        if v.is_finite() && v >= 0.0 && v.fract() == 0.0 && v <= (usize::MAX as f64) {
            return Some(v as usize);
        }
        return None;
    }

    value.as_str().and_then(|s| s.trim().parse::<usize>().ok())
}

// ── Path helpers ──────────────────────────────────────────────────────────────

/// Resolve a path that must already exist, and check it's inside the workspace.
fn safe_path(path: &str) -> Result<PathBuf, String> {
    let candidate = resolve_candidate(path);
    canonicalize_safe(&candidate, path)
}

/// Resolve a path that may not exist yet (for write_file).
fn safe_path_allow_new(path: &str) -> Result<PathBuf, String> {
    let candidate = resolve_candidate(path);

    // Try canonical first.
    if let Ok(abs) = candidate.canonicalize() {
        check_workspace_bounds(&abs, path)?;
        return Ok(abs);
    }

    // File doesn't exist yet — canonicalize the parent, append the filename.
    let parent = candidate.parent().unwrap_or(Path::new("."));
    let name = candidate
        .file_name()
        .ok_or_else(|| format!("invalid path: {path}"))?;
    let abs_parent = parent
        .canonicalize()
        .map_err(|_| format!("safe_path: parent dir doesn't exist for {path}"))?;
    let abs = abs_parent.join(name);
    check_workspace_bounds(&abs, path)?;
    Ok(abs)
}

fn resolve_candidate(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(p)
    }
}

fn canonicalize_safe(candidate: &Path, original: &str) -> Result<PathBuf, String> {
    let abs = candidate
        .canonicalize()
        .map_err(|e: io::Error| format!("safe_path: {e} ({original})"))?;
    check_workspace_bounds(&abs, original)?;
    Ok(abs)
}

fn check_workspace_bounds(abs: &Path, original: &str) -> Result<(), String> {
    // Delegate to the existing guard for blacklist + traversal checks.
    let workspace = std::env::current_dir().map_err(|e| format!("could not read cwd: {e}"))?;
    super::guard::path_is_safe(&workspace, abs)
        .map(|_| ())
        .map_err(|e| format!("file access denied for '{original}': {e}"))
}

/// Returns true if the path contains a segment that should be skipped (.git, target, node_modules, etc.)
fn path_has_hidden_segment(p: &Path) -> bool {
    p.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        s.starts_with('.') && s != "." && s != ".."
            || s == "target"
            || s == "node_modules"
            || s == "__pycache__"
    })
}

/// Show the lines nearest to where the search string *almost* matched,
/// so the model can see the real indentation/content and self-correct.
fn nearest_lines(content: &str, search: &str) -> String {
    // Try to find the best-matching line by the first non-empty search line.
    let first_search_line = search
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .unwrap_or("");

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return "(file is empty)".into();
    }

    // Find the line in the file that contains the most chars from the search line.
    let best_idx = if first_search_line.is_empty() {
        0
    } else {
        lines
            .iter()
            .enumerate()
            .max_by_key(|(_, l)| {
                let lt = l.trim();
                // Score: length of longest common prefix after trimming.
                first_search_line
                    .chars()
                    .zip(lt.chars())
                    .take_while(|(a, b)| a == b)
                    .count()
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    };

    let start = best_idx.saturating_sub(3);
    let end = (best_idx + 5).min(lines.len());
    let snippet = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, l)| format!("{:>4} | {}", start + i + 1, l))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Nearest matching lines ({}:{}):\n{}",
        best_idx + 1,
        end,
        snippet
    )
}

/// Fuzzy match: normalise both sides (trim trailing whitespace per line,
/// unify CRLF→LF) and return the byte range of the real match in `content`.
///
/// Only considers indentation-style differences — it does NOT tolerate
/// changed content, only changed surrounding whitespace.
fn fuzzy_find_span(content: &str, search: &str) -> Option<std::ops::Range<usize>> {
    // Normalise a string: CRLF→LF, trim both leading and trailing whitespace on each line.
    fn normalise(s: &str) -> String {
        s.lines().map(|l| l.trim()).collect::<Vec<_>>().join("\n")
    }

    let norm_content = normalise(content);
    let norm_search = normalise(search)
        .trim_start_matches('\n')
        .trim_end_matches('\n')
        .to_string();

    if norm_search.is_empty() {
        return None;
    }

    // Find where the normalised search appears in the normalised content.
    let norm_pos = norm_content.find(&norm_search)?;

    // Map the byte position back into the original (non-normalised) content.
    // We do this by counting newlines up to norm_pos and replaying through original.
    let lines_before = norm_content[..norm_pos]
        .as_bytes()
        .iter()
        .filter(|&&b| b == b'\n')
        .count();
    let search_lines = norm_search
        .as_bytes()
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1;

    let orig_lines: Vec<&str> = content.lines().collect();

    // Byte start of the first line in original.
    let mut current_pos = 0;
    for i in 0..lines_before {
        if i < orig_lines.len() {
            current_pos += orig_lines[i].len() + 1; // +1 for newline
        }
    }
    let byte_start = current_pos;

    // Byte end: sum of original line lengths for the matched span.
    let mut byte_len = 0;
    for i in 0..search_lines {
        let idx = lines_before + i;
        if idx < orig_lines.len() {
            byte_len += orig_lines[idx].len();
            if i < search_lines - 1 {
                byte_len += 1; // newline
            }
        }
    }

    // Validate: normalised forms must actually match (guards against false positives).
    if byte_start + byte_len > content.len() {
        return None;
    }

    let candidate = &content[byte_start..byte_start + byte_len];
    if normalise(candidate).trim_end_matches('\n') == norm_search.as_str() {
        Some(byte_start..byte_start + byte_len)
    } else {
        None
    }
}

// ── Diff preview helpers (read-only, no writes) ───────────────────────────────

/// Return a formatted diff string for an edit_file operation without applying it.
/// Lines prefixed "- " are removals, "+ " are additions.  Returns Err if the
/// search string cannot be located (caller falls through to normal tool dispatch).
pub fn compute_edit_file_diff(args: &Value) -> Result<String, String> {
    let path = require_str(args, "path")?;
    let search = require_str(args, "search")?;
    let replace = require_str(args, "replace")?;

    let abs = safe_path(path)?;
    let raw = fs::read_to_string(&abs).map_err(|e| format!("diff preview read: {e}"))?;
    let original = raw.replace("\r\n", "\n");

    let effective_search: String = if original.contains(search) {
        search.to_string()
    } else {
        match fuzzy_find_span(&original, search) {
            Some(span) => original[span].to_string(),
            None => return Err("search string not found — diff preview unavailable".into()),
        }
    };

    let mut diff = String::new();
    for line in effective_search.lines() {
        diff.push_str(&format!("- {}\n", line));
    }
    for line in replace.lines() {
        diff.push_str(&format!("+ {}\n", line));
    }
    Ok(diff)
}

/// Return a formatted diff string for a patch_hunk operation without applying it.
pub fn compute_patch_hunk_diff(args: &Value) -> Result<String, String> {
    let path = require_str(args, "path")?;
    let start_line = require_usize(args, "start_line")?;
    let end_line = require_usize(args, "end_line")?;
    let replacement = require_str(args, "replacement")?;

    let abs = safe_path(path)?;
    let original = fs::read_to_string(&abs).map_err(|e| format!("diff preview read: {e}"))?;
    let lines: Vec<&str> = original.lines().collect();
    let total = lines.len();

    if start_line < 1 || start_line > total || end_line < start_line || end_line > total {
        return Err(format!(
            "patch_hunk: invalid line range {}-{} for file with {} lines",
            start_line, end_line, total
        ));
    }

    let s_idx = start_line - 1;
    let e_idx = end_line;

    let mut diff = format!("@@ lines {}-{} @@\n", start_line, end_line);
    for i in s_idx..e_idx {
        diff.push_str(&format!("- {}\n", lines[i].trim_end()));
    }
    for line in replacement.lines() {
        diff.push_str(&format!("+ {}\n", line.trim_end()));
    }
    Ok(diff)
}

/// Return a formatted diff string for a multi_search_replace operation without applying it.
pub fn compute_msr_diff(args: &Value) -> Result<String, String> {
    let hunks_val = args
        .get("hunks")
        .ok_or_else(|| "multi_search_replace requires 'hunks' array".to_string())?;

    #[derive(serde::Deserialize)]
    struct PreviewHunk {
        search: String,
        replace: String,
    }
    let hunks: Vec<PreviewHunk> = serde_json::from_value(hunks_val.clone())
        .map_err(|e| format!("compute_msr_diff: invalid hunks: {e}"))?;

    let mut diff = String::new();
    for (i, hunk) in hunks.iter().enumerate() {
        if hunks.len() > 1 {
            diff.push_str(&format!("@@ hunk {} @@\n", i + 1));
        }
        for line in hunk.search.lines() {
            diff.push_str(&format!("- {}\n", line.trim_end()));
        }
        for line in hunk.replace.lines() {
            diff.push_str(&format!("+ {}\n", line.trim_end()));
        }
    }
    Ok(diff)
}

/// Resolve the workspace root by looking upward for common markers.
pub fn workspace_root() -> PathBuf {
    let mut current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if current.join(".git").exists()
            || current.join("Cargo.toml").exists()
            || current.join("package.json").exists()
        {
            return current;
        }
        if !current.pop() {
            break;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Returns true if the workspace root looks like a real project.
/// A bare `.git` alone (e.g. accidental `git init` in the home folder) doesn't
/// count — at least one explicit build/package marker must also be present.
pub fn is_project_workspace() -> bool {
    let root = workspace_root();
    let has_explicit_marker = root.join("Cargo.toml").exists()
        || root.join("package.json").exists()
        || root.join("pyproject.toml").exists()
        || root.join("go.mod").exists()
        || root.join("setup.py").exists()
        || root.join("pom.xml").exists()
        || root.join("build.gradle").exists()
        || root.join("CMakeLists.txt").exists();
    has_explicit_marker || (root.join(".git").exists() && root.join("src").exists())
}

/// A "Pre-Flight Scoping" tool that provides a high-level recursive map of the project.
/// Returns a directory tree and project configuration overview.
pub async fn map_project(_args: &Value) -> Result<String, String> {
    let root = workspace_root();
    let mut report = String::new();
    report.push_str(&format!("Project Root: {}\n", root.display()));

    // ── Layer 1: Configuration DNA ───────────────────────────────────────────
    report.push_str("\n── Configuration DNA ──\n");
    let markers = [
        "Cargo.toml",
        "package.json",
        "go.mod",
        "requirements.txt",
        "pyproject.toml",
        "README.md",
        "CLAUDE.md",
        "Taskfile.yml",
        ".env.example",
    ];
    for marker in &markers {
        let path = root.join(marker);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let snippet = &content[..content.len().min(800)];
                report.push_str(&format!("### File: {}\n```\n{}\n```\n", marker, snippet));
            }
        }
    }

    // ── Layer 2: Hierarchy Discovery ───────────────────────────────────────
    report.push_str("\n── Directory Structure ──\n");
    let mut lines = Vec::new();
    build_tree(&root, 0, &mut lines)?;
    report.push_str(&lines.join("\n"));

    Ok(report)
}

fn build_tree(dir: &PathBuf, depth: usize, lines: &mut Vec<String>) -> Result<(), String> {
    if depth > 4 {
        return Ok(());
    } // Cap depth to prevent token explosion

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read dir {dir:?}: {e}"))?
        .filter_map(Result::ok)
        .collect();

    entries.sort_by_key(|e| (e.file_type().unwrap().is_file(), e.file_name()));

    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "target" || name == "node_modules" || name == "vendor" {
            continue;
        }

        let indent = "  ".repeat(depth);
        let prefix = if entry.file_type().unwrap().is_dir() {
            "📁 "
        } else {
            "📄 "
        };
        lines.push(format!("{indent}{prefix}{name}"));

        if entry.file_type().unwrap().is_dir() {
            build_tree(&entry.path(), depth + 1, lines)?;
        }
    }
    Ok(())
}
