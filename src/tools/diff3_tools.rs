use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "description": "conflicts (default) | merge3 | sides | resolve"
            },
            "text": {
                "type": "string",
                "description": "Text containing git-style conflict markers (<<<<<<< / ======= / >>>>>>>)"
            },
            "file": {
                "type": "string",
                "description": "Path to a file with conflict markers"
            },
            "base": {
                "type": "string",
                "description": "Common ancestor text for merge3 action"
            },
            "ours": {
                "type": "string",
                "description": "Our version for merge3 action"
            },
            "theirs": {
                "type": "string",
                "description": "Their version for merge3 action"
            },
            "side": {
                "type": "string",
                "description": "Which side to take for 'sides' action: ours | theirs"
            },
            "strategy": {
                "type": "string",
                "description": "Auto-resolve strategy for 'resolve': ours | theirs | both | union (default: smart)"
            },
            "label_ours": {
                "type": "string",
                "description": "Label for ours side in merge3 output (default: ours)"
            },
            "label_theirs": {
                "type": "string",
                "description": "Label for theirs side in merge3 output (default: theirs)"
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("conflicts");

    match action {
        "conflicts" | "parse" | "list" => do_conflicts(args),
        "merge3" | "merge" => do_merge3(args),
        "sides" | "side" | "extract" => do_sides(args),
        "resolve" | "auto" => do_resolve(args),
        _ => Ok(format!(
            "Unknown action '{}'. Use: conflicts, merge3, sides, resolve.",
            action
        )),
    }
}

// ── Conflict marker parsing ────────────────────────────────────────────────

struct Conflict {
    start_line: usize,
    ours_label: String,
    ours_lines: Vec<String>,
    base_lines: Option<Vec<String>>,
    theirs_label: String,
    theirs_lines: Vec<String>,
}

enum ConflictLine {
    Normal(String),
    ConflictStart { label: String },
    ConflictBase,
    ConflictSep,
    ConflictEnd { label: String },
}

fn classify_line(line: &str) -> ConflictLine {
    if line.starts_with("<<<<<<<") {
        ConflictLine::ConflictStart {
            label: line[7..].trim().to_string(),
        }
    } else if line.starts_with("|||||||") {
        ConflictLine::ConflictBase
    } else if line == "=======" {
        ConflictLine::ConflictSep
    } else if line.starts_with(">>>>>>>") {
        ConflictLine::ConflictEnd {
            label: line[7..].trim().to_string(),
        }
    } else {
        ConflictLine::Normal(line.to_string())
    }
}

fn parse_conflicts(text: &str) -> Vec<Conflict> {
    let lines: Vec<&str> = text.lines().collect();

    #[derive(PartialEq, Clone, Copy)]
    enum State {
        Normal,
        InOurs,
        InBase,
        InTheirs,
    }

    let mut state = State::Normal;
    let mut conflicts = Vec::new();
    let mut start_line = 0usize;
    let mut ours_label = String::new();
    let mut ours_buf: Vec<String> = Vec::new();
    let mut base_buf: Vec<String> = Vec::new();
    let mut theirs_buf: Vec<String> = Vec::new();
    let mut has_base = false;

    for (i, raw) in lines.iter().enumerate() {
        match classify_line(raw) {
            ConflictLine::ConflictStart { label } if state == State::Normal => {
                state = State::InOurs;
                start_line = i + 1;
                ours_label = if label.is_empty() {
                    "ours".to_string()
                } else {
                    label
                };
                ours_buf.clear();
                base_buf.clear();
                theirs_buf.clear();
                has_base = false;
            }
            ConflictLine::ConflictBase if state == State::InOurs => {
                state = State::InBase;
                has_base = true;
            }
            ConflictLine::ConflictSep if state == State::InOurs || state == State::InBase => {
                state = State::InTheirs;
            }
            ConflictLine::ConflictEnd { label } if state == State::InTheirs => {
                conflicts.push(Conflict {
                    start_line,
                    ours_label: ours_label.clone(),
                    ours_lines: ours_buf.clone(),
                    base_lines: if has_base {
                        Some(base_buf.clone())
                    } else {
                        None
                    },
                    theirs_label: if label.is_empty() {
                        "theirs".to_string()
                    } else {
                        label
                    },
                    theirs_lines: theirs_buf.clone(),
                });
                state = State::Normal;
            }
            ConflictLine::Normal(line) => match state {
                State::InOurs => ours_buf.push(line),
                State::InBase => base_buf.push(line),
                State::InTheirs => theirs_buf.push(line),
                State::Normal => {}
            },
            _ => {}
        }
    }
    conflicts
}

fn load_text(args: &Value) -> Result<String, String> {
    if let Some(text) = args.get("text").and_then(|v| v.as_str()) {
        return Ok(text.to_string());
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path).map_err(|e| format!("Cannot read '{}': {}", path, e));
    }
    Err("Pass 'text' with conflict-marked content, or 'file' with a path.".to_string())
}

// ── Action: conflicts ──────────────────────────────────────────────────────

fn do_conflicts(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let conflicts = parse_conflicts(&text);

    if conflicts.is_empty() {
        return Ok(
            "No conflict markers found.\nExpected: <<<<<<< / ======= / >>>>>>> markers."
                .to_string(),
        );
    }

    let mut out = format!(
        "Found {} conflict{}\n{}\n\n",
        conflicts.len(),
        if conflicts.len() == 1 { "" } else { "s" },
        "─".repeat(50)
    );

    for (idx, c) in conflicts.iter().enumerate() {
        out.push_str(&format!("Conflict #{} (line {})\n", idx + 1, c.start_line));
        out.push_str(&format!(
            "  Ours   ({}): {} line{}\n",
            c.ours_label,
            c.ours_lines.len(),
            if c.ours_lines.len() == 1 { "" } else { "s" }
        ));
        if let Some(base) = &c.base_lines {
            out.push_str(&format!(
                "  Base:  {} line{}\n",
                base.len(),
                if base.len() == 1 { "" } else { "s" }
            ));
        }
        out.push_str(&format!(
            "  Theirs ({}): {} line{}\n",
            c.theirs_label,
            c.theirs_lines.len(),
            if c.theirs_lines.len() == 1 { "" } else { "s" }
        ));

        if !c.ours_lines.is_empty() {
            out.push_str("  — Ours:\n");
            for line in c.ours_lines.iter().take(5) {
                out.push_str(&format!("      {}\n", line));
            }
            if c.ours_lines.len() > 5 {
                out.push_str(&format!(
                    "      ... ({} more lines)\n",
                    c.ours_lines.len() - 5
                ));
            }
        } else {
            out.push_str("  — Ours: (empty — deletion)\n");
        }

        if !c.theirs_lines.is_empty() {
            out.push_str("  — Theirs:\n");
            for line in c.theirs_lines.iter().take(5) {
                out.push_str(&format!("      {}\n", line));
            }
            if c.theirs_lines.len() > 5 {
                out.push_str(&format!(
                    "      ... ({} more lines)\n",
                    c.theirs_lines.len() - 5
                ));
            }
        } else {
            out.push_str("  — Theirs: (empty — deletion)\n");
        }

        let class = if c.ours_lines == c.theirs_lines {
            "Identical content (trivially resolvable)"
        } else if c.ours_lines.is_empty() && c.theirs_lines.is_empty() {
            "Both deleted (trivially resolvable)"
        } else if c.ours_lines.is_empty() {
            "Ours deleted, theirs added"
        } else if c.theirs_lines.is_empty() {
            "Ours added, theirs deleted"
        } else {
            "Divergent content"
        };
        out.push_str(&format!("  Type:  {}\n", class));

        if idx + 1 < conflicts.len() {
            out.push_str(&format!("\n{}\n\n", "─".repeat(40)));
        }
    }

    let trivial = conflicts
        .iter()
        .filter(|c| {
            c.ours_lines == c.theirs_lines || (c.ours_lines.is_empty() && c.theirs_lines.is_empty())
        })
        .count();
    let non_trivial = conflicts.len() - trivial;
    out.push_str(&format!("\n{}\n", "─".repeat(50)));
    out.push_str(&format!(
        "Summary: {} trivial (auto-resolvable), {} need manual review\n",
        trivial, non_trivial
    ));
    out.push_str("Use action='resolve' to auto-resolve, action='sides' to extract one side.\n");

    Ok(out)
}

// ── Action: sides ─────────────────────────────────────────────────────────

fn do_sides(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let side = args.get("side").and_then(|v| v.as_str()).unwrap_or("ours");

    if side != "ours" && side != "theirs" {
        return Ok("Pass side: \"ours\" or \"theirs\".".to_string());
    }

    #[derive(PartialEq, Clone, Copy)]
    enum State {
        Normal,
        InOurs,
        InBase,
        InTheirs,
    }

    let mut state = State::Normal;
    let mut result_lines: Vec<String> = Vec::new();
    let mut conflict_count = 0usize;

    for raw in text.lines() {
        match classify_line(raw) {
            ConflictLine::ConflictStart { .. } if state == State::Normal => {
                state = State::InOurs;
                conflict_count += 1;
            }
            ConflictLine::ConflictBase if state == State::InOurs => {
                state = State::InBase;
            }
            ConflictLine::ConflictSep if state == State::InOurs || state == State::InBase => {
                state = State::InTheirs;
            }
            ConflictLine::ConflictEnd { .. } if state == State::InTheirs => {
                state = State::Normal;
            }
            ConflictLine::Normal(line) => match state {
                State::Normal => result_lines.push(line),
                State::InOurs if side == "ours" => result_lines.push(line),
                State::InTheirs if side == "theirs" => result_lines.push(line),
                _ => {}
            },
            _ => {}
        }
    }

    if conflict_count == 0 {
        return Ok("No conflict markers found in input.".to_string());
    }

    Ok(format!(
        "Extracted '{}' side from {} conflict{}:\n\n{}",
        side,
        conflict_count,
        if conflict_count == 1 { "" } else { "s" },
        result_lines.join("\n")
    ))
}

// ── Action: resolve ────────────────────────────────────────────────────────

fn do_resolve(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let strategy = args
        .get("strategy")
        .and_then(|v| v.as_str())
        .unwrap_or("smart");

    #[derive(PartialEq, Clone, Copy)]
    enum State {
        Normal,
        InOurs,
        InBase,
        InTheirs,
    }

    let mut state = State::Normal;
    let mut result: Vec<String> = Vec::new();
    let mut ours_buf: Vec<String> = Vec::new();
    let mut base_buf: Vec<String> = Vec::new();
    let mut theirs_buf: Vec<String> = Vec::new();
    let mut has_base = false;
    let mut resolved = 0usize;
    let mut remaining = 0usize;

    for raw in text.lines() {
        match classify_line(raw) {
            ConflictLine::ConflictStart { .. } if state == State::Normal => {
                state = State::InOurs;
                ours_buf.clear();
                base_buf.clear();
                theirs_buf.clear();
                has_base = false;
            }
            ConflictLine::ConflictBase if state == State::InOurs => {
                state = State::InBase;
                has_base = true;
            }
            ConflictLine::ConflictSep if state == State::InOurs || state == State::InBase => {
                state = State::InTheirs;
            }
            ConflictLine::ConflictEnd { label } if state == State::InTheirs => {
                let lb = if label.is_empty() {
                    "theirs".to_string()
                } else {
                    label
                };

                let resolved_lines: Option<Vec<String>> = match strategy {
                    "ours" => Some(ours_buf.clone()),
                    "theirs" => Some(theirs_buf.clone()),
                    "both" | "union" => {
                        let mut combined = ours_buf.clone();
                        combined.extend_from_slice(&theirs_buf);
                        Some(combined)
                    }
                    _ => {
                        if ours_buf == theirs_buf {
                            Some(ours_buf.clone())
                        } else if ours_buf.is_empty() && theirs_buf.is_empty() {
                            Some(Vec::new())
                        } else if has_base && ours_buf == base_buf {
                            Some(theirs_buf.clone())
                        } else if has_base && theirs_buf == base_buf {
                            Some(ours_buf.clone())
                        } else {
                            None
                        }
                    }
                };

                if let Some(kept) = resolved_lines {
                    result.extend(kept);
                    resolved += 1;
                } else {
                    result.push("<<<<<<< ours".to_string());
                    result.extend_from_slice(&ours_buf);
                    if has_base {
                        result.push("||||||| merged common ancestors".to_string());
                        result.extend_from_slice(&base_buf);
                    }
                    result.push("=======".to_string());
                    result.extend_from_slice(&theirs_buf);
                    result.push(format!(">>>>>>> {}", lb));
                    remaining += 1;
                }
                state = State::Normal;
            }
            ConflictLine::Normal(line) => match state {
                State::Normal => result.push(line),
                State::InOurs => ours_buf.push(line),
                State::InBase => base_buf.push(line),
                State::InTheirs => theirs_buf.push(line),
            },
            _ => {}
        }
    }

    let total = resolved + remaining;
    if total == 0 {
        return Ok("No conflict markers found in input.".to_string());
    }

    let mut out = format!(
        "Resolved {}/{} conflicts using strategy='{}'\n",
        resolved, total, strategy
    );
    if remaining > 0 {
        out.push_str(&format!(
            "{} conflict{} still need manual review.\n",
            remaining,
            if remaining == 1 { "" } else { "s" }
        ));
    } else {
        out.push_str("All conflicts resolved — file is clean.\n");
    }
    out.push_str("\n─── Resolved content ─────────────────────────────────────\n\n");
    out.push_str(&result.join("\n"));

    Ok(out)
}

// ── Action: merge3 ────────────────────────────────────────────────────────

fn do_merge3(args: &Value) -> Result<String, String> {
    let base = args
        .get("base")
        .and_then(|v| v.as_str())
        .ok_or("Pass 'base', 'ours', and 'theirs' text for merge3 action.")?;
    let ours = args
        .get("ours")
        .and_then(|v| v.as_str())
        .ok_or("Pass 'ours' text for merge3 action.")?;
    let theirs = args
        .get("theirs")
        .and_then(|v| v.as_str())
        .ok_or("Pass 'theirs' text for merge3 action.")?;

    let label_a = args
        .get("label_ours")
        .and_then(|v| v.as_str())
        .unwrap_or("ours");
    let label_b = args
        .get("label_theirs")
        .and_then(|v| v.as_str())
        .unwrap_or("theirs");

    let base_lines: Vec<String> = base.lines().map(|s| s.to_string()).collect();
    let ours_lines: Vec<String> = ours.lines().map(|s| s.to_string()).collect();
    let theirs_lines: Vec<String> = theirs.lines().map(|s| s.to_string()).collect();

    let (merged, conflict_count) =
        merge3_lines(&base_lines, &ours_lines, &theirs_lines, label_a, label_b);

    let header = if conflict_count == 0 {
        "Merged successfully — 0 conflicts\n\n".to_string()
    } else {
        format!(
            "Merged with {} conflict{}. Review markers and edit manually.\n\n",
            conflict_count,
            if conflict_count == 1 { "" } else { "s" }
        )
    };

    Ok(format!(
        "{}─── Merged result ─────────────────────────────────────────\n\n{}",
        header, merged
    ))
}

// ── LCS-based three-way merge ──────────────────────────────────────────────

fn compute_lcs_dp(a: &[String], b: &[String]) -> Vec<Vec<usize>> {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }
    dp
}

struct SideChange {
    insertions: Vec<Vec<String>>,
    insertions_end: Vec<String>,
    kept: Vec<bool>,
}

fn compute_side_change(base: &[String], side: &[String]) -> SideChange {
    let dp = compute_lcs_dp(base, side);

    let mut matches: Vec<(usize, usize)> = Vec::new();
    let mut i = base.len();
    let mut j = side.len();
    while i > 0 && j > 0 {
        if base[i - 1] == side[j - 1] && dp[i][j] == dp[i - 1][j - 1] + 1 {
            matches.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    matches.reverse();

    let mut kept = vec![false; base.len()];
    let mut insertions: Vec<Vec<String>> = vec![Vec::new(); base.len() + 1];
    let match_map: std::collections::HashMap<usize, usize> = matches.iter().copied().collect();

    for base_idx in 0..base.len() {
        if match_map.contains_key(&base_idx) {
            kept[base_idx] = true;
        }
    }

    let mut prev_si: Option<usize> = None;
    for &(bi, si) in &matches {
        let start = prev_si.map(|x| x + 1).unwrap_or(0);
        for k in start..si {
            insertions[bi].push(side[k].clone());
        }
        prev_si = Some(si);
    }

    let side_start = prev_si.map(|x| x + 1).unwrap_or(0);
    let insertions_end: Vec<String> = (side_start..side.len()).map(|k| side[k].clone()).collect();

    SideChange {
        insertions,
        insertions_end,
        kept,
    }
}

fn emit_insertions(
    ins_a: &[String],
    ins_b: &[String],
    la: &str,
    lb: &str,
    out: &mut String,
    conflicts: &mut usize,
) {
    match (ins_a.is_empty(), ins_b.is_empty()) {
        (true, true) => {}
        (false, true) => {
            for l in ins_a {
                out.push_str(l);
                out.push('\n');
            }
        }
        (true, false) => {
            for l in ins_b {
                out.push_str(l);
                out.push('\n');
            }
        }
        (false, false) => {
            if ins_a == ins_b {
                for l in ins_a {
                    out.push_str(l);
                    out.push('\n');
                }
            } else {
                *conflicts += 1;
                out.push_str(&format!("<<<<<<< {}\n", la));
                for l in ins_a {
                    out.push_str(l);
                    out.push('\n');
                }
                out.push_str("=======\n");
                for l in ins_b {
                    out.push_str(l);
                    out.push('\n');
                }
                out.push_str(&format!(">>>>>>> {}\n", lb));
            }
        }
    }
}

fn merge3_lines(
    base: &[String],
    ours: &[String],
    theirs: &[String],
    la: &str,
    lb: &str,
) -> (String, usize) {
    let sa = compute_side_change(base, ours);
    let sb = compute_side_change(base, theirs);
    let n = base.len();

    let mut out = String::new();
    let mut conflicts = 0usize;

    for i in 0..n {
        emit_insertions(
            &sa.insertions[i],
            &sb.insertions[i],
            la,
            lb,
            &mut out,
            &mut conflicts,
        );

        match (sa.kept[i], sb.kept[i]) {
            (true, true) => {
                out.push_str(&base[i]);
                out.push('\n');
            }
            (false, false) => {} // both deleted
            (true, false) => {
                conflicts += 1;
                out.push_str(&format!("<<<<<<< {}\n", la));
                out.push_str(&base[i]);
                out.push('\n');
                out.push_str("=======\n");
                out.push_str(&format!(">>>>>>> {}\n", lb));
            }
            (false, true) => {
                conflicts += 1;
                out.push_str(&format!("<<<<<<< {}\n", la));
                out.push_str("=======\n");
                out.push_str(&base[i]);
                out.push('\n');
                out.push_str(&format!(">>>>>>> {}\n", lb));
            }
        }
    }

    emit_insertions(
        &sa.insertions_end,
        &sb.insertions_end,
        la,
        lb,
        &mut out,
        &mut conflicts,
    );

    (out, conflicts)
}
