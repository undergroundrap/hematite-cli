use serde_json::{json, Value};

pub fn jsonl_tools_schema() -> Value {
    json!({
        "name": "jsonl_tools",
        "description": "Process JSONL (JSON Lines / NDJSON) files and streams without external utilities. Each line is a separate JSON object. Actions: parse (default — display all records with index, pretty-printed; optional 'limit'), filter (keep records where a field matches a value; 'field' dot-path and 'value'), map (extract a single field from every record; 'field'), aggregate (count, sum, avg, min, max on a numeric field; 'field'), keys (union of all keys across all records with type distribution), stats (record count, key coverage %, null rate, type distribution per key), to_csv (convert to CSV — headers from first record), group (group records by a field value and count per group; 'field'), sort (sort records by a field; 'field'; optional 'order': asc/desc).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "filter", "map", "aggregate", "keys", "stats", "to_csv", "group", "sort"],
                    "description": "Action to perform (default: parse)"
                },
                "text": {
                    "type": "string",
                    "description": "JSONL content as a string (newline-separated JSON objects)"
                },
                "jsonl": {
                    "type": "string",
                    "description": "Alias for 'text'"
                },
                "file": {
                    "type": "string",
                    "description": "Path to a .jsonl or .ndjson file"
                },
                "field": {
                    "type": "string",
                    "description": "Field path for filter/map/aggregate/group/sort (dot notation: 'user.name', 'items[0]')"
                },
                "value": {
                    "type": "string",
                    "description": "Value to match for filter action (string comparison)"
                },
                "op": {
                    "type": "string",
                    "enum": ["eq", "ne", "gt", "lt", "gte", "lte", "contains", "exists", "missing"],
                    "description": "Filter operator (default: eq)"
                },
                "agg": {
                    "type": "string",
                    "enum": ["count", "sum", "avg", "min", "max", "distinct"],
                    "description": "Aggregation function for aggregate action (default: count)"
                },
                "order": {
                    "type": "string",
                    "enum": ["asc", "desc"],
                    "description": "Sort order (default: asc)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of records to display (default: 20 for parse)"
                }
            },
            "required": []
        }
    })
}

// ── field path navigation ─────────────────────────────────────────────────────

fn get_field<'a>(record: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = record;
    for part in path.split('.') {
        // Handle array indexing like items[0]
        if let Some(bracket_pos) = part.find('[') {
            let key = &part[..bracket_pos];
            let idx_str = part[bracket_pos + 1..part.len() - 1].trim();
            if !key.is_empty() {
                cur = cur.get(key)?;
            }
            let idx: usize = idx_str.parse().ok()?;
            cur = cur.get(idx)?;
        } else {
            cur = cur.get(part)?;
        }
    }
    Some(cur)
}

fn value_as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn value_as_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".into(),
        Value::Array(a) => format!("[{} items]", a.len()),
        Value::Object(o) => format!("{{...{} keys}}", o.len()),
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "bool",
        Value::Null => "null",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ── JSONL loader ──────────────────────────────────────────────────────────────

fn load_records(args: &Value) -> Result<Vec<Value>, String> {
    let content = if let Some(t) = args
        .get("text")
        .or_else(|| args.get("jsonl"))
        .and_then(|v| v.as_str())
    {
        t.to_string()
    } else if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(path).map_err(|e| format!("Cannot read '{path}': {e}"))?
    } else {
        return Err(
            "Provide 'text'/'jsonl' (inline JSONL string) or 'file' (path to .jsonl/.ndjson file)"
                .into(),
        );
    };

    let mut records = Vec::new();
    let mut errors = 0usize;

    for (lineno, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(v) => records.push(v),
            Err(_) => {
                errors += 1;
                if errors <= 3 {
                    eprintln!(
                        "jsonl_tools: parse error at line {}: {trimmed:.60}",
                        lineno + 1
                    );
                }
            }
        }
    }

    if records.is_empty() && errors > 0 {
        return Err(format!(
            "No valid JSON records found ({errors} parse errors). Ensure each line is a valid JSON object."
        ));
    }

    Ok(records)
}

// ── filter predicate ──────────────────────────────────────────────────────────

fn record_matches(record: &Value, field: &str, op: &str, value: &str) -> bool {
    match op {
        "exists" => get_field(record, field).is_some(),
        "missing" => get_field(record, field).is_none(),
        _ => {
            let fval = match get_field(record, field) {
                Some(v) => v,
                None => return false,
            };
            let fstr = value_as_string(fval);
            match op {
                "eq" | "" => fstr == value || value_as_string(fval).eq_ignore_ascii_case(value),
                "ne" => fstr != value,
                "contains" => fstr.to_lowercase().contains(&value.to_lowercase()),
                "gt" => value_as_f64(fval)
                    .map(|f| f > value.parse::<f64>().unwrap_or(0.0))
                    .unwrap_or(false),
                "lt" => value_as_f64(fval)
                    .map(|f| f < value.parse::<f64>().unwrap_or(0.0))
                    .unwrap_or(false),
                "gte" => value_as_f64(fval)
                    .map(|f| f >= value.parse::<f64>().unwrap_or(0.0))
                    .unwrap_or(false),
                "lte" => value_as_f64(fval)
                    .map(|f| f <= value.parse::<f64>().unwrap_or(0.0))
                    .unwrap_or(false),
                _ => fstr == value,
            }
        }
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_parse(records: &[Value], limit: usize) -> String {
    let mut out = String::new();
    let total = records.len();
    let display = records.len().min(limit);

    out.push_str(&format!("JSONL: {total} records\n"));
    out.push_str(&"─".repeat(50));
    out.push('\n');

    for (i, rec) in records[..display].iter().enumerate() {
        out.push_str(&format!("\n[{i}] "));
        match serde_json::to_string_pretty(rec) {
            Ok(s) => out.push_str(&s),
            Err(_) => out.push_str(&rec.to_string()),
        }
        out.push('\n');
    }

    if total > display {
        out.push_str(&format!(
            "\n... ({} more records not shown)\n",
            total - display
        ));
    }
    out
}

fn action_filter(records: &[Value], field: &str, op: &str, value: &str) -> String {
    let matched: Vec<&Value> = records
        .iter()
        .filter(|r| record_matches(r, field, op, value))
        .collect();

    let mut out = String::new();
    out.push_str(&format!(
        "Filter: {field} {op} {value:?} → {} / {} records match\n",
        matched.len(),
        records.len()
    ));
    out.push_str(&"─".repeat(50));
    out.push('\n');

    for (i, rec) in matched.iter().enumerate().take(50) {
        out.push_str(&format!("\n[{i}] "));
        match serde_json::to_string_pretty(rec) {
            Ok(s) => out.push_str(&s),
            Err(_) => out.push_str(&rec.to_string()),
        }
        out.push('\n');
    }

    if matched.len() > 50 {
        out.push_str(&format!("\n... ({} more matches)\n", matched.len() - 50));
    }
    out
}

fn action_map(records: &[Value], field: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("Map: .{field}  ({} records)\n", records.len()));
    out.push_str(&"─".repeat(40));
    out.push('\n');

    let mut missing = 0usize;
    for (i, rec) in records.iter().enumerate() {
        match get_field(rec, field) {
            Some(v) => out.push_str(&format!("[{i}]  {}\n", value_as_string(v))),
            None => {
                missing += 1;
                out.push_str(&format!("[{i}]  (missing)\n"));
            }
        }
        if i >= 199 {
            out.push_str(&format!("... ({} more)\n", records.len() - 200));
            break;
        }
    }

    if missing > 0 {
        out.push_str(&format!("\n{missing} records missing field '{field}'\n"));
    }
    out
}

fn action_aggregate(records: &[Value], field: &str, agg: &str) -> String {
    let mut out = String::new();

    match agg {
        "count" => {
            let present = records
                .iter()
                .filter(|r| get_field(r, field).is_some())
                .count();
            out.push_str(&format!(
                "count(.{field}) = {present} / {} records have this field\n",
                records.len()
            ));
        }
        "distinct" => {
            let mut seen = std::collections::HashSet::new();
            for rec in records {
                if let Some(v) = get_field(rec, field) {
                    seen.insert(value_as_string(v));
                }
            }
            let mut vals: Vec<_> = seen.into_iter().collect();
            vals.sort();
            out.push_str(&format!(
                "distinct(.{field}) = {} unique values\n",
                vals.len()
            ));
            for v in &vals {
                out.push_str(&format!("  {v}\n"));
            }
        }
        _ => {
            let nums: Vec<f64> = records
                .iter()
                .filter_map(|r| get_field(r, field))
                .filter_map(value_as_f64)
                .collect();

            if nums.is_empty() {
                out.push_str(&format!("No numeric values found for field '{field}'\n"));
                return out;
            }

            match agg {
                "sum" => {
                    let s: f64 = nums.iter().sum();
                    out.push_str(&format!("sum(.{field}) = {s}\n"));
                }
                "avg" => {
                    let s: f64 = nums.iter().sum();
                    let avg = s / nums.len() as f64;
                    out.push_str(&format!(
                        "avg(.{field}) = {avg:.4}  (from {} values)\n",
                        nums.len()
                    ));
                }
                "min" => {
                    let m = nums.iter().cloned().fold(f64::INFINITY, f64::min);
                    out.push_str(&format!("min(.{field}) = {m}\n"));
                }
                "max" => {
                    let m = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    out.push_str(&format!("max(.{field}) = {m}\n"));
                }
                _ => {
                    out.push_str(&format!(
                        "Unknown agg '{agg}'. Valid: count, sum, avg, min, max, distinct\n"
                    ));
                }
            }
        }
    }
    out
}

fn action_keys(records: &[Value]) -> String {
    use std::collections::HashMap;
    let mut key_types: HashMap<String, HashMap<&'static str, usize>> = HashMap::new();
    let mut key_count: HashMap<String, usize> = HashMap::new();

    for rec in records {
        if let Value::Object(map) = rec {
            for (k, v) in map {
                *key_count.entry(k.clone()).or_insert(0) += 1;
                *key_types
                    .entry(k.clone())
                    .or_default()
                    .entry(type_name(v))
                    .or_insert(0) += 1;
            }
        }
    }

    let total = records.len();
    let mut keys: Vec<(String, usize)> = key_count.into_iter().collect();
    keys.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut out = String::new();
    out.push_str(&format!("Keys across {} records:\n", total));
    out.push_str(&format!("{:<40} {:>8}  {}\n", "Key", "Present", "Types"));
    out.push_str(&"─".repeat(70));
    out.push('\n');

    for (k, count) in &keys {
        let coverage = count * 100 / total.max(1);
        let types = key_types
            .get(k.as_str())
            .map(|t| {
                let mut v: Vec<_> = t.iter().collect();
                v.sort_by(|a, b| b.1.cmp(a.1));
                v.iter()
                    .map(|(t, c)| format!("{t}({c})"))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        out.push_str(&format!("{:<40} {:>6}%  {types}\n", k, coverage));
    }
    out
}

fn action_stats(records: &[Value]) -> String {
    use std::collections::HashMap;

    let total = records.len();
    if total == 0 {
        return "No records\n".into();
    }

    let mut key_info: HashMap<String, (usize, usize, HashMap<&'static str, usize>)> =
        HashMap::new();

    for rec in records {
        if let Value::Object(map) = rec {
            for (k, v) in map {
                let e = key_info.entry(k.clone()).or_insert((0, 0, HashMap::new()));
                e.0 += 1;
                if *v == Value::Null {
                    e.1 += 1;
                }
                *e.2.entry(type_name(v)).or_insert(0) += 1;
            }
        }
    }

    let mut keys: Vec<_> = key_info.iter().collect();
    keys.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.0.cmp(b.0)));

    let mut out = String::new();
    out.push_str(&format!("JSONL Stats: {total} records\n"));
    out.push_str(&"─".repeat(70));
    out.push('\n');
    out.push_str(&format!(
        "{:<38} {:>7} {:>7}  {}\n",
        "Field", "Present%", "Null%", "Types"
    ));
    out.push_str(&"─".repeat(70));
    out.push('\n');

    for (k, (present, null_count, type_map)) in &keys {
        let pct = *present * 100 / total;
        let null_pct = if *present > 0 {
            null_count * 100 / present
        } else {
            0
        };
        let mut tv: Vec<_> = type_map.iter().collect();
        tv.sort_by(|a, b| b.1.cmp(a.1));
        let types = tv
            .iter()
            .map(|(t, c)| format!("{t}({c})"))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "{:<38} {:>6}%  {:>5}%  {types}\n",
            k, pct, null_pct
        ));
    }
    out
}

fn action_to_csv(records: &[Value]) -> String {
    if records.is_empty() {
        return "No records to convert\n".into();
    }

    // Collect all keys in order of first appearance
    let mut headers: Vec<String> = Vec::new();
    let mut seen_headers = std::collections::HashSet::new();
    for rec in records {
        if let Value::Object(map) = rec {
            for k in map.keys() {
                if seen_headers.insert(k.clone()) {
                    headers.push(k.clone());
                }
            }
        }
    }

    let mut out = String::new();
    // Header row
    let header_row: Vec<String> = headers.iter().map(|h| csv_escape(h)).collect();
    out.push_str(&header_row.join(","));
    out.push('\n');

    for rec in records {
        let row: Vec<String> = headers
            .iter()
            .map(|h| match rec.get(h) {
                None => String::new(),
                Some(v) => csv_escape(&value_as_string(v)),
            })
            .collect();
        out.push_str(&row.join(","));
        out.push('\n');
    }

    out.push_str(&format!(
        "\n({} rows, {} columns)\n",
        records.len(),
        headers.len()
    ));
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn action_group(records: &[Value], field: &str) -> String {
    use std::collections::HashMap;
    let mut groups: HashMap<String, usize> = HashMap::new();

    for rec in records {
        let key = match get_field(rec, field) {
            Some(v) => value_as_string(v),
            None => "(missing)".into(),
        };
        *groups.entry(key).or_insert(0) += 1;
    }

    let mut entries: Vec<_> = groups.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut out = String::new();
    out.push_str(&format!(
        "Group by .{field}  ({} records, {} groups)\n",
        records.len(),
        entries.len()
    ));
    out.push_str(&"─".repeat(50));
    out.push('\n');
    for (k, count) in &entries {
        let bar_len = (*count * 30) / records.len().max(1);
        let bar = "█".repeat(bar_len);
        out.push_str(&format!("{:<35}  {:>6}  {bar}\n", k, count));
    }
    out
}

fn action_sort(records: &[Value], field: &str, order: &str) -> String {
    let mut indexed: Vec<(usize, &Value)> = records.iter().enumerate().collect();

    indexed.sort_by(|(_, a), (_, b)| {
        let av = get_field(a, field).map(value_as_string).unwrap_or_default();
        let bv = get_field(b, field).map(value_as_string).unwrap_or_default();
        // Try numeric first
        let num_a = av.parse::<f64>();
        let num_b = bv.parse::<f64>();
        let cmp = match (num_a, num_b) {
            (Ok(na), Ok(nb)) => na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal),
            _ => av.cmp(&bv),
        };
        if order == "desc" {
            cmp.reverse()
        } else {
            cmp
        }
    });

    let mut out = String::new();
    out.push_str(&format!(
        "Sort by .{field} {order}  ({} records)\n",
        records.len()
    ));
    out.push_str(&"─".repeat(50));
    out.push('\n');

    for (i, (orig_idx, rec)) in indexed.iter().enumerate().take(50) {
        let fval = get_field(rec, field)
            .map(value_as_string)
            .unwrap_or_else(|| "(missing)".into());
        out.push_str(&format!("[{i}] (orig {orig_idx}) .{field} = {fval}\n"));
    }
    if records.len() > 50 {
        out.push_str(&format!("... ({} more)\n", records.len() - 50));
    }
    out
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");

    let records = load_records(args)?;

    let field = args.get("field").and_then(|v| v.as_str()).unwrap_or("");
    let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
    let op = args.get("op").and_then(|v| v.as_str()).unwrap_or("eq");
    let agg = args.get("agg").and_then(|v| v.as_str()).unwrap_or("count");
    let order = args.get("order").and_then(|v| v.as_str()).unwrap_or("asc");
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

    match action {
        "parse" => Ok(action_parse(&records, limit)),
        "filter" => {
            if field.is_empty() {
                return Err("filter requires 'field' parameter".into());
            }
            Ok(action_filter(&records, field, op, value))
        }
        "map" => {
            if field.is_empty() {
                return Err("map requires 'field' parameter".into());
            }
            Ok(action_map(&records, field))
        }
        "aggregate" => {
            if field.is_empty() {
                return Err("aggregate requires 'field' parameter".into());
            }
            Ok(action_aggregate(&records, field, agg))
        }
        "keys" => Ok(action_keys(&records)),
        "stats" => Ok(action_stats(&records)),
        "to_csv" => Ok(action_to_csv(&records)),
        "group" => {
            if field.is_empty() {
                return Err("group requires 'field' parameter".into());
            }
            Ok(action_group(&records, field))
        }
        "sort" => {
            if field.is_empty() {
                return Err("sort requires 'field' parameter".into());
            }
            Ok(action_sort(&records, field, order))
        }
        _ => Err(format!(
            "Unknown action '{action}'. Valid: parse, filter, map, aggregate, keys, stats, to_csv, group, sort"
        )),
    }
}
