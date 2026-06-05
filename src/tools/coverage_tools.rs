use serde_json::{json, Value};
use std::collections::HashMap;

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["summary", "files", "uncovered", "compare"],
                "description": "Action to perform (default: summary)"
            },
            "text": { "type": "string", "description": "LCOV or Istanbul JSON coverage data inline" },
            "file": { "type": "string", "description": "Path to .lcov / .info / coverage-summary.json file" },
            "text_b": { "type": "string", "description": "Second report for compare action" },
            "file_b": { "type": "string", "description": "Path to second report for compare action" },
            "threshold": {
                "type": "number",
                "description": "Show only files below this coverage % (files action)"
            },
            "limit": { "type": "integer", "description": "Max files to show (default 50)" }
        }
    })
}

// ── data model ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
struct FileCov {
    path: String,
    lines_found: u64,
    lines_hit: u64,
    funcs_found: u64,
    funcs_hit: u64,
    branches_found: u64,
    branches_hit: u64,
    uncovered_lines: Vec<u32>,
}

impl FileCov {
    fn line_pct(&self) -> f64 {
        pct(self.lines_hit, self.lines_found)
    }
    fn func_pct(&self) -> f64 {
        pct(self.funcs_hit, self.funcs_found)
    }
    fn branch_pct(&self) -> f64 {
        pct(self.branches_hit, self.branches_found)
    }
}

fn pct(hit: u64, found: u64) -> f64 {
    if found == 0 {
        100.0
    } else {
        (hit as f64 / found as f64) * 100.0
    }
}

// ── parsers ─────────────────────────────────────────────────────────────────

fn parse_lcov(text: &str) -> Vec<FileCov> {
    let mut records: Vec<FileCov> = Vec::new();
    let mut cur = FileCov::default();
    let mut in_record = false;
    // line DA entries before end_of_record
    let mut da_map: HashMap<u32, u64> = HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("SF:") {
            cur = FileCov::default();
            cur.path = line[3..].to_string();
            da_map.clear();
            in_record = true;
        } else if line == "end_of_record" {
            if in_record {
                // collect uncovered lines from da_map
                let mut uncov: Vec<u32> = da_map
                    .iter()
                    .filter(|(_, &c)| c == 0)
                    .map(|(&l, _)| l)
                    .collect();
                uncov.sort_unstable();
                cur.uncovered_lines = uncov;
                records.push(cur.clone());
            }
            in_record = false;
        } else if in_record {
            if let Some(rest) = line.strip_prefix("DA:") {
                // DA:line_number,execution_count[,checksum]
                let parts: Vec<&str> = rest.splitn(3, ',').collect();
                if parts.len() >= 2 {
                    if let (Ok(lineno), Ok(count)) =
                        (parts[0].parse::<u32>(), parts[1].parse::<u64>())
                    {
                        da_map.insert(lineno, count);
                    }
                }
            } else if let Some(rest) = line.strip_prefix("LH:") {
                cur.lines_hit = rest.trim().parse().unwrap_or(0);
            } else if let Some(rest) = line.strip_prefix("LF:") {
                cur.lines_found = rest.trim().parse().unwrap_or(0);
            } else if let Some(rest) = line.strip_prefix("FNH:") {
                cur.funcs_hit = rest.trim().parse().unwrap_or(0);
            } else if let Some(rest) = line.strip_prefix("FNF:") {
                cur.funcs_found = rest.trim().parse().unwrap_or(0);
            } else if let Some(rest) = line.strip_prefix("BRH:") {
                cur.branches_hit = rest.trim().parse().unwrap_or(0);
            } else if let Some(rest) = line.strip_prefix("BRF:") {
                cur.branches_found = rest.trim().parse().unwrap_or(0);
            }
        }
    }

    records
}

fn parse_istanbul(text: &str) -> Result<Vec<FileCov>, String> {
    let v: Value =
        serde_json::from_str(text).map_err(|e| format!("Invalid Istanbul JSON: {}", e))?;
    let obj = v
        .as_object()
        .ok_or_else(|| "Expected an object at root level.".to_string())?;

    let mut records = Vec::new();

    // top-level may be {"total":{...}, "path":{...}} or just {"path":{...}}
    for (path, data) in obj {
        if path == "total" {
            continue;
        }
        let mut fc = FileCov {
            path: path.clone(),
            ..Default::default()
        };
        if let Some(lines) = data.get("lines").and_then(|v| v.as_object()) {
            fc.lines_found = lines.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
            fc.lines_hit = lines.get("covered").and_then(|v| v.as_u64()).unwrap_or(0);
        }
        if let Some(fns) = data.get("functions").and_then(|v| v.as_object()) {
            fc.funcs_found = fns.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
            fc.funcs_hit = fns.get("covered").and_then(|v| v.as_u64()).unwrap_or(0);
        }
        if let Some(br) = data.get("branches").and_then(|v| v.as_object()) {
            fc.branches_found = br.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
            fc.branches_hit = br.get("covered").and_then(|v| v.as_u64()).unwrap_or(0);
        }
        records.push(fc);
    }

    Ok(records)
}

fn load_coverage(args: &Value, text_key: &str, file_key: &str) -> Result<Vec<FileCov>, String> {
    let raw = if let Some(t) = args.get(text_key).and_then(|v| v.as_str()) {
        t.to_string()
    } else if let Some(p) = args.get(file_key).and_then(|v| v.as_str()) {
        std::fs::read_to_string(p).map_err(|e| format!("Cannot read '{}': {}", p, e))?
    } else {
        return Err(format!(
            "Pass '{}' with inline text or '{}' with a file path.",
            text_key, file_key
        ));
    };

    // auto-detect: Istanbul JSON starts with {, LCOV starts with SF: or TN:
    let trimmed = raw.trim();
    if trimmed.starts_with('{') {
        parse_istanbul(trimmed)
    } else {
        Ok(parse_lcov(trimmed))
    }
}

// ── aggregate helpers ────────────────────────────────────────────────────────

struct Totals {
    lines_hit: u64,
    lines_found: u64,
    funcs_hit: u64,
    funcs_found: u64,
    branches_hit: u64,
    branches_found: u64,
}

fn aggregate(records: &[FileCov]) -> Totals {
    let mut t = Totals {
        lines_hit: 0,
        lines_found: 0,
        funcs_hit: 0,
        funcs_found: 0,
        branches_hit: 0,
        branches_found: 0,
    };
    for r in records {
        t.lines_hit += r.lines_hit;
        t.lines_found += r.lines_found;
        t.funcs_hit += r.funcs_hit;
        t.funcs_found += r.funcs_found;
        t.branches_hit += r.branches_hit;
        t.branches_found += r.branches_found;
    }
    t
}

fn bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    let color = if pct >= 80.0 {
        "█"
    } else if pct >= 50.0 {
        "▓"
    } else {
        "░"
    };
    format!(
        "[{}{}] {:5.1}%",
        color.repeat(filled),
        " ".repeat(empty),
        pct
    )
}

fn grade(pct: f64) -> &'static str {
    if pct >= 90.0 {
        "A"
    } else if pct >= 80.0 {
        "B"
    } else if pct >= 70.0 {
        "C"
    } else if pct >= 50.0 {
        "D"
    } else {
        "F"
    }
}

fn short_path(path: &str, max: usize) -> String {
    if path.len() <= max {
        path.to_string()
    } else {
        format!("…{}", &path[path.len() - (max - 1)..])
    }
}

// ── actions ─────────────────────────────────────────────────────────────────

fn do_summary(args: &Value) -> Result<String, String> {
    let records = load_coverage(args, "text", "file")?;
    if records.is_empty() {
        return Ok("No coverage records found.".to_string());
    }

    let t = aggregate(&records);
    let l_pct = pct(t.lines_hit, t.lines_found);
    let f_pct = pct(t.funcs_hit, t.funcs_found);
    let b_pct = pct(t.branches_hit, t.branches_found);

    let mut out = String::new();
    out.push_str(&format!("Coverage Summary ({} files)\n", records.len()));
    out.push_str(&"─".repeat(52));
    out.push('\n');

    out.push_str(&format!(
        "Lines      {}/{:>7}  {}\n",
        t.lines_hit,
        t.lines_found,
        bar(l_pct, 25)
    ));
    out.push_str(&format!(
        "Functions  {}/{:>7}  {}\n",
        t.funcs_hit,
        t.funcs_found,
        bar(f_pct, 25)
    ));
    out.push_str(&format!(
        "Branches   {}/{:>7}  {}\n",
        t.branches_hit,
        t.branches_found,
        bar(b_pct, 25)
    ));

    out.push('\n');
    let overall = (l_pct + f_pct + b_pct) / 3.0;
    out.push_str(&format!(
        "Overall grade: {}  ({:.1}% average across all metrics)\n",
        grade(overall),
        overall
    ));

    // worst 5 files
    let mut sorted = records.clone();
    sorted.sort_by(|a, b| a.line_pct().partial_cmp(&b.line_pct()).unwrap());
    let worst: Vec<_> = sorted.iter().take(5).collect();
    if !worst.is_empty() {
        out.push('\n');
        out.push_str("Lowest-coverage files:\n");
        for fc in worst {
            out.push_str(&format!(
                "  {:5.1}%  {}\n",
                fc.line_pct(),
                short_path(&fc.path, 60)
            ));
        }
    }

    Ok(out)
}

fn do_files(args: &Value) -> Result<String, String> {
    let records = load_coverage(args, "text", "file")?;
    if records.is_empty() {
        return Ok("No coverage records found.".to_string());
    }

    let threshold: Option<f64> = args.get("threshold").and_then(|v| v.as_f64());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let mut sorted = records.clone();
    sorted.sort_by(|a, b| a.line_pct().partial_cmp(&b.line_pct()).unwrap());

    let filtered: Vec<_> = sorted
        .iter()
        .filter(|fc| match threshold {
            Some(t) => fc.line_pct() < t,
            None => true,
        })
        .take(limit)
        .collect();

    if filtered.is_empty() {
        return Ok(format!(
            "No files{}.",
            threshold
                .map(|t| format!(" below {:.0}% coverage", t))
                .unwrap_or_default()
        ));
    }

    let mut out = String::new();
    if let Some(t) = threshold {
        out.push_str(&format!(
            "Files below {:.0}% line coverage ({} of {}):\n",
            t,
            filtered.len(),
            records.len()
        ));
    } else {
        out.push_str(&format!(
            "All files by line coverage ({}):\n",
            filtered.len()
        ));
    }
    out.push_str(&format!(
        "{:<55} {:>7}  {:>7}  {:>7}\n",
        "File", "Lines%", "Funcs%", "Branch%"
    ));
    out.push_str(&"─".repeat(82));
    out.push('\n');

    for fc in filtered {
        out.push_str(&format!(
            "{:<55} {:>6.1}%  {:>6.1}%  {:>6.1}%\n",
            short_path(&fc.path, 55),
            fc.line_pct(),
            fc.func_pct(),
            fc.branch_pct()
        ));
    }

    Ok(out)
}

fn do_uncovered(args: &Value) -> Result<String, String> {
    let records = load_coverage(args, "text", "file")?;
    if records.is_empty() {
        return Ok("No coverage records found.".to_string());
    }

    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let mut out = String::new();
    let mut count = 0;
    let mut total_uncovered = 0u64;

    for fc in &records {
        total_uncovered += fc.uncovered_lines.len() as u64;
        if !fc.uncovered_lines.is_empty() && count < limit {
            // group consecutive line numbers into ranges
            let ranges = to_ranges(&fc.uncovered_lines);
            out.push_str(&format!(
                "{} ({} uncovered lines)\n",
                short_path(&fc.path, 70),
                fc.uncovered_lines.len()
            ));
            // show up to first 10 ranges
            let shown: Vec<_> = ranges.iter().take(10).collect();
            for (s, e) in &shown {
                if s == e {
                    out.push_str(&format!("  line {}\n", s));
                } else {
                    out.push_str(&format!("  lines {}–{}\n", s, e));
                }
            }
            if ranges.len() > 10 {
                out.push_str(&format!("  ... and {} more ranges\n", ranges.len() - 10));
            }
            count += 1;
        }
    }

    if out.is_empty() {
        return Ok("All instrumented lines are covered.".to_string());
    }

    let mut header = format!(
        "Uncovered lines ({} total uncovered across {} files)\n",
        total_uncovered,
        records
            .iter()
            .filter(|f| !f.uncovered_lines.is_empty())
            .count()
    );
    if count >= limit {
        header.push_str(&format!("(showing first {} files)\n", limit));
    }
    header.push_str(&"─".repeat(50));
    header.push('\n');
    header.push_str(&out);
    Ok(header)
}

fn to_ranges(lines: &[u32]) -> Vec<(u32, u32)> {
    if lines.is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let mut start = lines[0];
    let mut prev = lines[0];
    for &l in &lines[1..] {
        if l == prev + 1 {
            prev = l;
        } else {
            ranges.push((start, prev));
            start = l;
            prev = l;
        }
    }
    ranges.push((start, prev));
    ranges
}

fn do_compare(args: &Value) -> Result<String, String> {
    let a = load_coverage(args, "text", "file")?;
    let b = load_coverage(args, "text_b", "file_b")?;

    let ta = aggregate(&a);
    let tb = aggregate(&b);

    let la = pct(ta.lines_hit, ta.lines_found);
    let lb = pct(tb.lines_hit, tb.lines_found);
    let fa = pct(ta.funcs_hit, ta.funcs_found);
    let fb = pct(tb.funcs_hit, tb.funcs_found);
    let ba_pct = pct(ta.branches_hit, ta.branches_found);
    let bb_pct = pct(tb.branches_hit, tb.branches_found);

    let mut out = String::new();
    out.push_str("Coverage Comparison\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "{:<12} {:>8}  {:>8}  {:>8}\n",
        "Metric", "Before", "After", "Delta"
    ));
    out.push_str(&"─".repeat(42));
    out.push('\n');

    let diff = |a: f64, b: f64| -> String {
        let d = b - a;
        if d > 0.05 {
            format!("+{:.1}% ▲", d)
        } else if d < -0.05 {
            format!("{:.1}% ▼", d)
        } else {
            " 0.0%  ".to_string()
        }
    };

    out.push_str(&format!(
        "{:<12} {:>7.1}%  {:>7.1}%  {}\n",
        "Lines",
        la,
        lb,
        diff(la, lb)
    ));
    out.push_str(&format!(
        "{:<12} {:>7.1}%  {:>7.1}%  {}\n",
        "Functions",
        fa,
        fb,
        diff(fa, fb)
    ));
    out.push_str(&format!(
        "{:<12} {:>7.1}%  {:>7.1}%  {}\n",
        "Branches",
        ba_pct,
        bb_pct,
        diff(ba_pct, bb_pct)
    ));

    // files that changed
    let a_map: HashMap<&str, &FileCov> = a.iter().map(|f| (f.path.as_str(), f)).collect();
    let b_map: HashMap<&str, &FileCov> = b.iter().map(|f| (f.path.as_str(), f)).collect();

    let mut changed: Vec<(&str, f64, f64)> = a_map
        .iter()
        .filter_map(|(&path, &fa)| {
            b_map.get(path).map(|fb| {
                let _d = fb.line_pct() - fa.line_pct();
                (path, fa.line_pct(), fb.line_pct())
            })
        })
        .filter(|(_, pa, pb)| (pb - pa).abs() > 0.1)
        .collect();
    changed.sort_by(|x, y| (y.2 - y.1).abs().partial_cmp(&(x.2 - x.1).abs()).unwrap());

    let new_files: Vec<_> = b_map
        .keys()
        .filter(|&&p| !a_map.contains_key(p))
        .take(5)
        .collect();
    let removed_files: Vec<_> = a_map
        .keys()
        .filter(|&&p| !b_map.contains_key(p))
        .take(5)
        .collect();

    if !changed.is_empty() {
        out.push('\n');
        out.push_str("Most changed files (by line coverage):\n");
        for (path, pa, pb) in changed.iter().take(10) {
            let d = pb - pa;
            let arrow = if d > 0.0 { "▲" } else { "▼" };
            out.push_str(&format!(
                "  {:+.1}% {}  {}\n",
                d,
                arrow,
                short_path(path, 60)
            ));
        }
    }
    if !new_files.is_empty() {
        out.push('\n');
        out.push_str("New files in report B:\n");
        for p in new_files {
            out.push_str(&format!("  + {}\n", p));
        }
    }
    if !removed_files.is_empty() {
        out.push('\n');
        out.push_str("Files only in report A (removed or renamed):\n");
        for p in removed_files {
            out.push_str(&format!("  - {}\n", p));
        }
    }

    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("summary");
    match action {
        "summary" => do_summary(args),
        "files" => do_files(args),
        "uncovered" => do_uncovered(args),
        "compare" => do_compare(args),
        _ => Err(format!(
            "Unknown action '{}'. Use: summary / files / uncovered / compare.",
            action
        )),
    }
}
