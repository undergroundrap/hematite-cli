use serde_json::Value;
use std::collections::VecDeque;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("compare");

    match action {
        "compare" => compare(args),
        "patch" => make_patch(args),
        "apply" => apply_patch(args),
        "word-diff" => word_diff(args),
        "stat" => diff_stat(args),
        other => Err(format!(
            "diff_tools: unknown action '{other}'. Valid: compare, patch, apply, word-diff, stat"
        )),
    }
}

fn get_sides(args: &Value) -> Result<(String, String, String, String), String> {
    // Resolve left side: text_a or file_a
    let (left, label_a) = if let Some(t) = args.get("text_a").and_then(|v| v.as_str()) {
        (t.to_string(), "<text_a>")
    } else if let Some(f) = args.get("file_a").and_then(|v| v.as_str()) {
        let path = resolve_path(args, f);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("diff_tools: cannot read '{f}': {e}"))?;
        (content, f)
    } else {
        return Err("diff_tools: provide 'text_a'/'file_a' for the left side".into());
    };

    // Resolve right side: text_b or file_b
    let (right, label_b) = if let Some(t) = args.get("text_b").and_then(|v| v.as_str()) {
        (t.to_string(), "<text_b>")
    } else if let Some(f) = args.get("file_b").and_then(|v| v.as_str()) {
        let path = resolve_path(args, f);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("diff_tools: cannot read '{f}': {e}"))?;
        (content, f)
    } else {
        return Err("diff_tools: provide 'text_b'/'file_b' for the right side".into());
    };

    Ok((left, label_a.to_string(), right, label_b.to_string()))
}

fn resolve_path(args: &Value, rel: &str) -> std::path::PathBuf {
    if let Some(root) = args.get("_root").and_then(|v| v.as_str()) {
        std::path::Path::new(root).join(rel)
    } else {
        crate::tools::file_ops::workspace_root().join(rel)
    }
}

// Myers diff algorithm producing Edit enum sequence
#[derive(Debug, Clone)]
enum Edit {
    Keep(String),
    Insert(String),
    Delete(String),
}

fn compute_diff(left_lines: &[&str], right_lines: &[&str]) -> Vec<Edit> {
    let n = left_lines.len();
    let m = right_lines.len();

    if n == 0 && m == 0 {
        return vec![];
    }
    if n == 0 {
        return right_lines.iter().map(|l| Edit::Insert(l.to_string())).collect();
    }
    if m == 0 {
        return left_lines.iter().map(|l| Edit::Delete(l.to_string())).collect();
    }

    // LCS-based diff via DP table
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            if left_lines[i] == right_lines[j] {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut edits = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && left_lines[i] == right_lines[j] {
            edits.push(Edit::Keep(left_lines[i].to_string()));
            i += 1;
            j += 1;
        } else if j < m && (i >= n || dp[i][j + 1] >= dp[i + 1][j]) {
            edits.push(Edit::Insert(right_lines[j].to_string()));
            j += 1;
        } else {
            edits.push(Edit::Delete(left_lines[i].to_string()));
            i += 1;
        }
    }
    edits
}

fn compare(args: &Value) -> Result<String, String> {
    let (left, label_a, right, label_b) = get_sides(args)?;
    let context = args
        .get("context")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;

    let left_lines: Vec<&str> = left.lines().collect();
    let right_lines: Vec<&str> = right.lines().collect();

    let edits = compute_diff(&left_lines, &right_lines);

    let changed = edits.iter().any(|e| !matches!(e, Edit::Keep(_)));
    if !changed {
        return Ok(format!(
            "DIFF: {} vs {}\n{}\nFiles are identical — no differences found.",
            label_a,
            label_b,
            "─".repeat(60)
        ));
    }

    let mut out = format!(
        "DIFF: {} vs {}\n{}\n--- {}\n+++ {}\n",
        label_a,
        label_b,
        "─".repeat(60),
        label_a,
        label_b
    );

    // Produce unified-style hunks
    let hunk_lines = build_hunk_lines(&edits, context);
    out.push_str(&hunk_lines);

    // Summary line
    let added: usize = edits.iter().filter(|e| matches!(e, Edit::Insert(_))).count();
    let deleted: usize = edits.iter().filter(|e| matches!(e, Edit::Delete(_))).count();
    out.push_str(&format!(
        "\n{} line(s) added, {} line(s) removed",
        added, deleted
    ));

    Ok(out)
}

fn build_hunk_lines(edits: &[Edit], context: usize) -> String {
    // Collect positions where changes occur
    let changed_positions: Vec<usize> = edits
        .iter()
        .enumerate()
        .filter(|(_, e)| !matches!(e, Edit::Keep(_)))
        .map(|(i, _)| i)
        .collect();

    if changed_positions.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let n = edits.len();

    // Merge nearby changed positions into hunks
    let mut hunk_ranges: Vec<(usize, usize)> = vec![];
    let mut start = changed_positions[0].saturating_sub(context);
    let mut end = (changed_positions[0] + context + 1).min(n);

    for &pos in &changed_positions[1..] {
        let new_start = pos.saturating_sub(context);
        if new_start <= end {
            end = (pos + context + 1).min(n);
        } else {
            hunk_ranges.push((start, end));
            start = new_start;
            end = (pos + context + 1).min(n);
        }
    }
    hunk_ranges.push((start, end));

    // Track line numbers per side
    let mut left_line = 1usize;
    let mut right_line = 1usize;
    let mut edit_pos = 0;

    for (hunk_start, hunk_end) in hunk_ranges {
        // Advance counters to hunk_start
        while edit_pos < hunk_start {
            match &edits[edit_pos] {
                Edit::Keep(_) => { left_line += 1; right_line += 1; }
                Edit::Delete(_) => { left_line += 1; }
                Edit::Insert(_) => { right_line += 1; }
            }
            edit_pos += 1;
        }

        let hunk_left_start = left_line;
        let hunk_right_start = right_line;

        // Collect hunk lines
        let mut hunk_buf = String::new();
        let mut tmp_left = left_line;
        let mut tmp_right = right_line;

        for e in &edits[hunk_start..hunk_end] {
            match e {
                Edit::Keep(l) => {
                    hunk_buf.push_str(&format!(" {l}\n"));
                    tmp_left += 1;
                    tmp_right += 1;
                }
                Edit::Delete(l) => {
                    hunk_buf.push_str(&format!("-{l}\n"));
                    tmp_left += 1;
                }
                Edit::Insert(l) => {
                    hunk_buf.push_str(&format!("+{l}\n"));
                    tmp_right += 1;
                }
            }
        }

        let left_count = tmp_left - hunk_left_start;
        let right_count = tmp_right - hunk_right_start;
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk_left_start, left_count, hunk_right_start, right_count
        ));
        out.push_str(&hunk_buf);

        left_line = tmp_left;
        right_line = tmp_right;
        edit_pos = hunk_end;
    }

    out
}

fn make_patch(args: &Value) -> Result<String, String> {
    let (left, label_a, right, label_b) = get_sides(args)?;
    let context = args
        .get("context")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;

    let left_lines: Vec<&str> = left.lines().collect();
    let right_lines: Vec<&str> = right.lines().collect();
    let edits = compute_diff(&left_lines, &right_lines);

    let mut patch = format!("--- {label_a}\n+++ {label_b}\n");
    patch.push_str(&build_hunk_lines(&edits, context));

    let added: usize = edits.iter().filter(|e| matches!(e, Edit::Insert(_))).count();
    let deleted: usize = edits.iter().filter(|e| matches!(e, Edit::Delete(_))).count();

    if added == 0 && deleted == 0 {
        return Ok(format!(
            "PATCH: {} vs {}\nNo differences — empty patch.\n",
            label_a, label_b
        ));
    }

    // Optionally write to file
    if let Some(out_path) = args.get("output").and_then(|v| v.as_str()) {
        let full = resolve_path(args, out_path);
        std::fs::write(&full, &patch)
            .map_err(|e| format!("diff_tools: cannot write patch to '{out_path}': {e}"))?;
        return Ok(format!(
            "PATCH written to: {out_path}\n({added} addition(s), {deleted} deletion(s))"
        ));
    }

    Ok(format!(
        "PATCH: {} vs {}\n{}\n{patch}\n({added} addition(s), {deleted} deletion(s))",
        label_a,
        label_b,
        "─".repeat(60)
    ))
}

fn apply_patch(args: &Value) -> Result<String, String> {
    let patch_text = if let Some(t) = args.get("patch").and_then(|v| v.as_str()) {
        t.to_string()
    } else if let Some(f) = args.get("patch_file").and_then(|v| v.as_str()) {
        let path = resolve_path(args, f);
        std::fs::read_to_string(&path)
            .map_err(|e| format!("diff_tools: cannot read patch '{f}': {e}"))?
    } else {
        return Err("diff_tools apply: provide 'patch' (inline text) or 'patch_file' (path)".into());
    };

    let target_text = if let Some(t) = args.get("text_a").and_then(|v| v.as_str()) {
        t.to_string()
    } else if let Some(f) = args.get("file_a").and_then(|v| v.as_str()) {
        let path = resolve_path(args, f);
        std::fs::read_to_string(&path)
            .map_err(|e| format!("diff_tools: cannot read '{f}': {e}"))?
    } else {
        return Err("diff_tools apply: provide 'text_a' or 'file_a' as the base to patch".into());
    };

    let result = apply_unified_patch(&target_text, &patch_text)
        .map_err(|e| format!("diff_tools apply: {e}"))?;

    // Write result if output specified
    if let Some(out_path) = args.get("output").and_then(|v| v.as_str()) {
        let full = resolve_path(args, out_path);
        std::fs::write(&full, &result)
            .map_err(|e| format!("diff_tools: cannot write to '{out_path}': {e}"))?;
        return Ok(format!("PATCH APPLIED → written to {out_path}"));
    }

    Ok(format!("PATCH APPLIED\n{}\n{result}", "─".repeat(60)))
}

fn apply_unified_patch(base: &str, patch: &str) -> Result<String, String> {
    let mut result: Vec<String> = base.lines().map(|l| l.to_string()).collect();
    let patch_lines: Vec<&str> = patch.lines().collect();

    let mut offset: i64 = 0;

    let mut i = 0;
    while i < patch_lines.len() {
        let line = patch_lines[i];
        if let Some(rest) = line.strip_prefix("@@ ") {
            // Parse @@ -left_start,left_count +right_start,right_count @@
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() < 2 {
                return Err(format!("malformed hunk header: {line}"));
            }
            let left_part = parts[0].trim_start_matches('-');
            let left_start: i64 = left_part
                .split(',')
                .next()
                .unwrap_or("1")
                .parse()
                .unwrap_or(1);

            let apply_at = (left_start - 1 + offset).max(0) as usize;
            i += 1;

            let mut del_lines = VecDeque::new();
            let mut add_lines = VecDeque::new();

            while i < patch_lines.len() {
                let pl = patch_lines[i];
                if pl.starts_with("@@") || pl.starts_with("---") || pl.starts_with("+++") {
                    break;
                }
                if let Some(stripped) = pl.strip_prefix('-') {
                    del_lines.push_back(stripped.to_string());
                } else if let Some(stripped) = pl.strip_prefix('+') {
                    add_lines.push_back(stripped.to_string());
                }
                i += 1;
            }

            // Remove deleted lines starting at apply_at
            let del_count = del_lines.len() as i64;
            let add_count = add_lines.len() as i64;

            if apply_at <= result.len() {
                let end = (apply_at + del_lines.len()).min(result.len());
                result.drain(apply_at..end);
                for (idx, line) in add_lines.into_iter().enumerate() {
                    result.insert(apply_at + idx, line);
                }
                offset += add_count - del_count;
            }
        } else {
            i += 1;
        }
    }

    Ok(result.join("\n"))
}

fn word_diff(args: &Value) -> Result<String, String> {
    let (left, label_a, right, label_b) = get_sides(args)?;

    let left_words: Vec<&str> = left.split_whitespace().collect();
    let right_words: Vec<&str> = right.split_whitespace().collect();

    let n = left_words.len();
    let m = right_words.len();

    // LCS on words
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            if left_words[i] == right_words[j] {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut out = format!(
        "WORD-DIFF: {} vs {}\n{}\n",
        label_a,
        label_b,
        "─".repeat(60)
    );

    let (mut i, mut j) = (0, 0);
    let mut buf = String::new();
    while i < n || j < m {
        if i < n && j < m && left_words[i] == right_words[j] {
            buf.push_str(left_words[i]);
            buf.push(' ');
            i += 1;
            j += 1;
        } else if j < m && (i >= n || dp[i][j + 1] >= dp[i + 1][j]) {
            buf.push_str(&format!("[+{}]", right_words[j]));
            buf.push(' ');
            j += 1;
        } else {
            buf.push_str(&format!("[-{}]", left_words[i]));
            buf.push(' ');
            i += 1;
        }
    }
    out.push_str(buf.trim());
    out.push('\n');

    let added = right_words.len() as i64 - left_words.len() as i64;
    let sign = if added >= 0 { "+" } else { "" };
    out.push_str(&format!("\n{sign}{added} word(s) net change"));

    Ok(out)
}

fn diff_stat(args: &Value) -> Result<String, String> {
    let (left, label_a, right, label_b) = get_sides(args)?;

    let left_lines: Vec<&str> = left.lines().collect();
    let right_lines: Vec<&str> = right.lines().collect();
    let edits = compute_diff(&left_lines, &right_lines);

    let added: usize = edits.iter().filter(|e| matches!(e, Edit::Insert(_))).count();
    let deleted: usize = edits.iter().filter(|e| matches!(e, Edit::Delete(_))).count();
    let kept: usize = edits.iter().filter(|e| matches!(e, Edit::Keep(_))).count();
    let changed = added > 0 || deleted > 0;

    let total_left = left_lines.len();
    let total_right = right_lines.len();
    let similarity = if total_left + total_right > 0 {
        (2 * kept * 100) / (total_left + total_right)
    } else {
        100
    };

    let bar_width = 40usize;
    let added_bar = if added + deleted > 0 {
        (added * bar_width) / (added + deleted).max(1)
    } else {
        0
    };
    let del_bar = bar_width.saturating_sub(added_bar);
    let bar = format!("{}{}", "+".repeat(added_bar), "-".repeat(del_bar));

    Ok(format!(
        "DIFF STAT: {} vs {}\n{}\n\
         Left  : {} line(s)\n\
         Right : {} line(s)\n\
         Added : {added}\n\
         Deleted: {deleted}\n\
         Unchanged: {kept}\n\
         Similarity: {similarity}%\n\
         Changed: {}\n\n\
         {bar}",
        label_a,
        label_b,
        "─".repeat(60),
        total_left,
        total_right,
        if changed { "yes" } else { "no (identical)" },
    ))
}
