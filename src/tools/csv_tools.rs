use serde_json::Value;
use std::collections::HashMap;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("read");

    match action {
        "read" => read(args),
        "head" => head(args),
        "columns" => columns(args),
        "stats" => stats(args),
        "filter" => filter(args),
        "sort" => sort(args),
        "to-json" => to_json(args),
        "to-markdown" => to_markdown(args),
        "count" => count(args),
        other => Err(format!(
            "csv_tools: unknown action '{other}'. Valid: read, head, columns, stats, filter, sort, to-json, to-markdown, count"
        )),
    }
}

// ── CSV parser ────────────────────────────────────────────────────────────────

fn parse_csv_line(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '"' {
            if in_quotes {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cur.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                in_quotes = true;
            }
        } else if c == delimiter && !in_quotes {
            fields.push(cur.trim().to_string());
            cur = String::new();
        } else {
            cur.push(c);
        }
    }
    fields.push(cur.trim().to_string());
    fields
}

struct CsvData {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn resolve_input(args: &Value) -> Result<String, String> {
    if let Some(inline) = args.get("csv").and_then(|v| v.as_str()) {
        return Ok(inline.to_string());
    }
    if let Some(file) = args.get("file").and_then(|v| v.as_str()) {
        let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
            std::path::PathBuf::from(r)
        } else {
            crate::tools::file_ops::workspace_root()
        };
        let path = root.join(file);
        return std::fs::read_to_string(&path)
            .map_err(|e| format!("csv_tools: cannot read '{file}': {e}"));
    }
    Err("csv_tools: provide 'csv' (inline string) or 'file' (path)".into())
}

fn parse(args: &Value) -> Result<CsvData, String> {
    let src = resolve_input(args)?;
    let delimiter = args
        .get("delimiter")
        .and_then(|v| v.as_str())
        .and_then(|s| s.chars().next())
        .unwrap_or(',');

    let mut lines = src.lines().filter(|l| !l.trim().is_empty());

    let header_line = lines
        .next()
        .ok_or("csv_tools: empty input — no header row found")?;
    let headers = parse_csv_line(header_line, delimiter);

    let rows: Vec<Vec<String>> = lines
        .map(|l| {
            let mut row = parse_csv_line(l, delimiter);
            // Pad short rows
            while row.len() < headers.len() {
                row.push(String::new());
            }
            row.truncate(headers.len());
            row
        })
        .collect();

    Ok(CsvData { headers, rows })
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn read(args: &Value) -> Result<String, String> {
    let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let data = parse(args)?;

    let mut out = format!(
        "CSV READ — {} row(s), {} column(s)\n{}\n",
        data.rows.len(),
        data.headers.len(),
        "─".repeat(60)
    );
    out.push_str(&render_table(&data.headers, &data.rows, n));
    if data.rows.len() > n {
        out.push_str(&format!(
            "\n... {} more row(s) not shown (set n=N to see more)\n",
            data.rows.len() - n
        ));
    }
    Ok(out)
}

fn head(args: &Value) -> Result<String, String> {
    let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let data = parse(args)?;
    let mut out = format!(
        "CSV HEAD — first {} of {} row(s)\n{}\n",
        n.min(data.rows.len()),
        data.rows.len(),
        "─".repeat(60)
    );
    out.push_str(&render_table(&data.headers, &data.rows, n));
    Ok(out)
}

fn columns(args: &Value) -> Result<String, String> {
    let data = parse(args)?;
    let mut out = format!(
        "CSV COLUMNS — {} column(s)\n{}\n",
        data.headers.len(),
        "─".repeat(60)
    );
    for (i, h) in data.headers.iter().enumerate() {
        let sample: Vec<&str> = data
            .rows
            .iter()
            .filter_map(|r| r.get(i))
            .filter(|v| !v.is_empty())
            .take(3)
            .map(|s| s.as_str())
            .collect();
        let sample_str = if sample.is_empty() {
            "(empty)".into()
        } else {
            sample.join(", ")
        };
        out.push_str(&format!("  [{i}] {h}  →  e.g. {sample_str}\n"));
    }
    Ok(out)
}

fn stats(args: &Value) -> Result<String, String> {
    let data = parse(args)?;
    let mut out = format!(
        "CSV STATS — {} row(s), {} column(s)\n{}\n",
        data.rows.len(),
        data.headers.len(),
        "─".repeat(60)
    );

    for (i, header) in data.headers.iter().enumerate() {
        let vals: Vec<&str> = data
            .rows
            .iter()
            .filter_map(|r| r.get(i).map(|s| s.as_str()))
            .filter(|s| !s.is_empty())
            .collect();

        let non_empty = vals.len();
        let missing = data.rows.len() - non_empty;

        // Try numeric
        let nums: Vec<f64> = vals.iter().filter_map(|s| s.parse::<f64>().ok()).collect();

        out.push_str(&format!("\n{header}:\n"));
        out.push_str(&format!("  count: {non_empty}  missing: {missing}\n"));

        if nums.len() == non_empty && !nums.is_empty() {
            let min = nums.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let sum: f64 = nums.iter().sum();
            let mean = sum / nums.len() as f64;
            let mut sorted = nums.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = if sorted.len() % 2 == 0 {
                (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
            } else {
                sorted[sorted.len() / 2]
            };
            let variance = nums.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / nums.len() as f64;
            let stddev = variance.sqrt();
            out.push_str("  type: numeric\n");
            out.push_str(&format!("  min: {min}  max: {max}\n"));
            out.push_str(&format!(
                "  mean: {mean:.4}  median: {median:.4}  stddev: {stddev:.4}\n"
            ));
        } else {
            // Categorical
            let mut freq: HashMap<&str, usize> = HashMap::new();
            for v in &vals {
                *freq.entry(v).or_insert(0) += 1;
            }
            let unique = freq.len();
            out.push_str(&format!("  type: text  unique: {unique}\n"));
            let mut top: Vec<(&&str, &usize)> = freq.iter().collect();
            top.sort_by(|a, b| b.1.cmp(a.1));
            for (val, cnt) in top.iter().take(5) {
                out.push_str(&format!("  {:>6}x  {}\n", cnt, val));
            }
        }
    }
    Ok(out)
}

fn filter(args: &Value) -> Result<String, String> {
    let data = parse(args)?;
    let col = args
        .get("column")
        .and_then(|v| v.as_str())
        .ok_or("csv_tools filter: 'column' is required")?;
    let value = args
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or("csv_tools filter: 'value' is required")?;
    let op = args
        .get("op")
        .and_then(|v| v.as_str())
        .unwrap_or("contains");

    let col_idx = data
        .headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(col))
        .ok_or_else(|| {
            format!(
                "csv_tools filter: column '{col}' not found. Available: {}",
                data.headers.join(", ")
            )
        })?;

    let matched: Vec<&Vec<String>> = data
        .rows
        .iter()
        .filter(|row| {
            let cell = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
            match op {
                "eq" | "equals" => cell.eq_ignore_ascii_case(value),
                "ne" | "not-equals" => !cell.eq_ignore_ascii_case(value),
                "gt" => cell
                    .parse::<f64>()
                    .ok()
                    .zip(value.parse::<f64>().ok())
                    .map(|(a, b)| a > b)
                    .unwrap_or(false),
                "lt" => cell
                    .parse::<f64>()
                    .ok()
                    .zip(value.parse::<f64>().ok())
                    .map(|(a, b)| a < b)
                    .unwrap_or(false),
                "gte" => cell
                    .parse::<f64>()
                    .ok()
                    .zip(value.parse::<f64>().ok())
                    .map(|(a, b)| a >= b)
                    .unwrap_or(false),
                "lte" => cell
                    .parse::<f64>()
                    .ok()
                    .zip(value.parse::<f64>().ok())
                    .map(|(a, b)| a <= b)
                    .unwrap_or(false),
                "starts-with" => cell.to_lowercase().starts_with(&value.to_lowercase()),
                "ends-with" => cell.to_lowercase().ends_with(&value.to_lowercase()),
                _ => cell.to_lowercase().contains(&value.to_lowercase()),
            }
        })
        .collect();

    let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let mut out = format!(
        "CSV FILTER: {col} {op} {value:?} — {} match(es) of {} rows\n{}\n",
        matched.len(),
        data.rows.len(),
        "─".repeat(60)
    );
    let owned: Vec<Vec<String>> = matched.iter().map(|r| (*r).clone()).collect();
    out.push_str(&render_table(&data.headers, &owned, n));
    if matched.len() > n {
        out.push_str(&format!(
            "\n... {} more match(es) not shown\n",
            matched.len() - n
        ));
    }
    Ok(out)
}

fn sort(args: &Value) -> Result<String, String> {
    let mut data = parse(args)?;
    let col = args
        .get("column")
        .and_then(|v| v.as_str())
        .ok_or("csv_tools sort: 'column' is required")?;
    let descending = args
        .get("descending")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let col_idx = data
        .headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(col))
        .ok_or_else(|| format!("csv_tools sort: column '{col}' not found"))?;

    // Try numeric sort, fall back to lexicographic
    let all_numeric = data.rows.iter().all(|r| {
        r.get(col_idx)
            .map(|v| v.parse::<f64>().is_ok())
            .unwrap_or(true)
    });

    if all_numeric {
        data.rows.sort_by(|a, b| {
            let av: f64 = a.get(col_idx).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let bv: f64 = b.get(col_idx).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            av.partial_cmp(&bv).unwrap()
        });
    } else {
        data.rows
            .sort_by(|a, b| a.get(col_idx).cmp(&b.get(col_idx)));
    }
    if descending {
        data.rows.reverse();
    }

    let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let dir = if descending { "DESC" } else { "ASC" };
    let mut out = format!(
        "CSV SORT: {col} {dir} — {} row(s)\n{}\n",
        data.rows.len(),
        "─".repeat(60)
    );
    out.push_str(&render_table(&data.headers, &data.rows, n));
    if data.rows.len() > n {
        out.push_str(&format!("\n... {} more row(s)\n", data.rows.len() - n));
    }
    Ok(out)
}

fn to_json(args: &Value) -> Result<String, String> {
    let data = parse(args)?;
    let records: Vec<serde_json::Map<String, serde_json::Value>> = data
        .rows
        .iter()
        .map(|row| {
            let mut map = serde_json::Map::new();
            for (i, h) in data.headers.iter().enumerate() {
                let cell = row.get(i).cloned().unwrap_or_default();
                // Try to parse as number
                let val = if let Ok(n) = cell.parse::<f64>() {
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into()),
                    )
                } else if cell == "true" {
                    serde_json::Value::Bool(true)
                } else if cell == "false" {
                    serde_json::Value::Bool(false)
                } else if cell.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(cell)
                };
                map.insert(h.clone(), val);
            }
            map
        })
        .collect();

    let json = serde_json::to_string_pretty(&records)
        .map_err(|e| format!("csv_tools to-json: serialize error: {e}"))?;
    Ok(format!(
        "CSV → JSON — {} record(s)\n{}\n{json}",
        data.rows.len(),
        "─".repeat(60)
    ))
}

fn to_markdown(args: &Value) -> Result<String, String> {
    let data = parse(args)?;
    let n = args
        .get("n")
        .and_then(|v| v.as_u64())
        .unwrap_or(data.rows.len() as u64) as usize;

    let mut out = format!(
        "CSV → MARKDOWN — {} row(s)\n{}\n",
        data.rows.len(),
        "─".repeat(60)
    );

    // Header row
    out.push_str("| ");
    out.push_str(&data.headers.join(" | "));
    out.push_str(" |\n");

    // Separator
    out.push_str("| ");
    out.push_str(
        &data
            .headers
            .iter()
            .map(|h| "─".repeat(h.len().max(3)))
            .collect::<Vec<_>>()
            .join(" | "),
    );
    out.push_str(" |\n");

    // Rows
    for row in data.rows.iter().take(n) {
        out.push_str("| ");
        let cells: Vec<String> = data
            .headers
            .iter()
            .enumerate()
            .map(|(i, _)| row.get(i).cloned().unwrap_or_default())
            .collect();
        out.push_str(&cells.join(" | "));
        out.push_str(" |\n");
    }
    if data.rows.len() > n {
        out.push_str(&format!("\n*... {} more rows*\n", data.rows.len() - n));
    }
    Ok(out)
}

fn count(args: &Value) -> Result<String, String> {
    let data = parse(args)?;
    let col = args.get("column").and_then(|v| v.as_str());

    if let Some(col_name) = col {
        let col_idx = data
            .headers
            .iter()
            .position(|h| h.eq_ignore_ascii_case(col_name))
            .ok_or_else(|| format!("csv_tools count: column '{col_name}' not found"))?;

        let mut freq: HashMap<String, usize> = HashMap::new();
        for row in &data.rows {
            let val = row.get(col_idx).cloned().unwrap_or_default();
            *freq.entry(val).or_insert(0) += 1;
        }

        let mut sorted: Vec<(String, usize)> = freq.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let mut out = format!(
            "CSV COUNT BY {col_name} — {} unique value(s)\n{}\n",
            sorted.len(),
            "─".repeat(60)
        );
        for (val, cnt) in sorted.iter().take(n) {
            let pct = (cnt * 100) / data.rows.len().max(1);
            out.push_str(&format!("  {:>6}  ({pct:>2}%)  {val}\n", cnt));
        }
        if sorted.len() > n {
            out.push_str(&format!("\n... {} more values\n", sorted.len() - n));
        }
        Ok(out)
    } else {
        Ok(format!(
            "CSV COUNT\n{}\nRows   : {}\nColumns: {}\n",
            "─".repeat(60),
            data.rows.len(),
            data.headers.len()
        ))
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render_table(headers: &[String], rows: &[Vec<String>], limit: usize) -> String {
    // Compute column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows.iter().take(limit) {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len().min(30));
            }
        }
    }

    let sep: String = widths
        .iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("┼");
    let sep = format!("┼{sep}┼");

    let mut out = String::new();

    // Header
    out.push('┼');
    for (i, h) in headers.iter().enumerate() {
        let w = widths.get(i).copied().unwrap_or(10);
        out.push_str(&format!(" {:w$} ┼", truncate(h, w)));
    }
    out.push('\n');
    out.push_str(&sep);
    out.push('\n');

    // Rows
    for row in rows.iter().take(limit) {
        out.push('┼');
        for (i, _) in headers.iter().enumerate() {
            let w = widths.get(i).copied().unwrap_or(10);
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            out.push_str(&format!(" {:w$} ┼", truncate(cell, w)));
        }
        out.push('\n');
    }

    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
