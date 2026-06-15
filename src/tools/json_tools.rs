use serde_json::{Map, Value};
use std::path::PathBuf;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("pretty");

    let input = resolve_input(args)?;
    let json: Value = serde_json::from_str(&input)
        .map_err(|e| format!("json_tools: failed to parse JSON: {e}"))?;

    match action {
        "pretty" => pretty(&json),
        "compact" => compact(&json),
        "keys" => keys(&json, args),
        "get" => get_path(&json, args),
        "filter" => filter_array(&json, args),
        "pluck" => pluck_array(&json, args),
        "flatten" => flatten_array(&json, args),
        "count" => count(&json),
        "sort" => sort_array(&json, args),
        "unique" => unique_array(&json, args),
        "merge" => merge(args),
        "diff" => json_diff(args),
        "validate" => validate(&json),
        "schema" => infer_schema(&json),
        "stats" => array_stats(&json),
        "to-csv" => to_csv(&json),
        other => Err(format!(
            "json_tools: unknown action '{other}'. Valid: pretty, compact, keys, get, filter, \
             pluck, flatten, count, sort, unique, merge, diff, validate, schema, stats, to-csv"
        )),
    }
}

fn resolve_input(args: &Value) -> Result<String, String> {
    // Inline JSON
    if let Some(s) = args.get("json").and_then(|v| v.as_str()) {
        return Ok(s.to_string());
    }
    // File path
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
            PathBuf::from(r)
        } else {
            crate::tools::file_ops::workspace_root()
        };
        let full = if std::path::Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            root.join(path)
        };
        return std::fs::read_to_string(&full)
            .map_err(|e| format!("json_tools: cannot read '{}': {e}", full.display()));
    }
    Err("json_tools: provide 'json' (inline JSON string) or 'file' (path to JSON file)".into())
}

fn pretty(json: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(json).map_err(|e| format!("json_tools pretty: {e}"))
}

fn compact(json: &Value) -> Result<String, String> {
    serde_json::to_string(json).map_err(|e| format!("json_tools compact: {e}"))
}

fn keys(json: &Value, args: &Value) -> Result<String, String> {
    let path = args.get("path").and_then(|v| v.as_str());
    let target = if let Some(p) = path {
        navigate(json, p)?
    } else {
        json
    };
    match target {
        Value::Object(map) => {
            let mut keys: Vec<&str> = map.keys().map(|s| s.as_str()).collect();
            keys.sort_unstable();
            Ok(keys.join("\n"))
        }
        Value::Array(arr) => Ok(format!("[array of {} elements]", arr.len())),
        other => Ok(format!("[{}]", type_name(other))),
    }
}

fn get_path(json: &Value, args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("json_tools get: 'path' is required (e.g. 'user.name' or 'items[0].id')")?;
    let val = navigate(json, path)?;
    serde_json::to_string_pretty(val).map_err(|e| e.to_string())
}

fn filter_array(json: &Value, args: &Value) -> Result<String, String> {
    let arr = require_array(json, "filter")?;
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("json_tools filter: 'key' is required (field name to match on)")?;
    let value = args
        .get("value")
        .ok_or("json_tools filter: 'value' is required")?;
    let op = args.get("op").and_then(|v| v.as_str()).unwrap_or("eq");

    let filtered: Vec<&Value> = arr
        .iter()
        .filter(|item| {
            if let Some(field) = item.get(key) {
                match op {
                    "eq" => field == value,
                    "ne" => field != value,
                    "gt" => cmp_numeric(field, value) == Some(std::cmp::Ordering::Greater),
                    "lt" => cmp_numeric(field, value) == Some(std::cmp::Ordering::Less),
                    "gte" => matches!(
                        cmp_numeric(field, value),
                        Some(std::cmp::Ordering::Greater) | Some(std::cmp::Ordering::Equal)
                    ),
                    "lte" => matches!(
                        cmp_numeric(field, value),
                        Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal)
                    ),
                    "contains" => {
                        let needle = value.as_str().unwrap_or("");
                        field.as_str().map(|s| s.contains(needle)).unwrap_or(false)
                    }
                    "starts_with" => {
                        let needle = value.as_str().unwrap_or("");
                        field
                            .as_str()
                            .map(|s| s.starts_with(needle))
                            .unwrap_or(false)
                    }
                    _ => false,
                }
            } else {
                false
            }
        })
        .collect();

    let result = Value::Array(filtered.into_iter().cloned().collect());
    serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
}

fn pluck_array(json: &Value, args: &Value) -> Result<String, String> {
    let arr = require_array(json, "pluck")?;
    let fields_raw = args
        .get("fields")
        .and_then(|v| v.as_str())
        .ok_or("json_tools pluck: 'fields' is required (comma-separated field names)")?;
    let fields: Vec<&str> = fields_raw.split(',').map(str::trim).collect();

    let result: Vec<Value> = arr
        .iter()
        .map(|item| match item {
            Value::Object(map) => {
                let mut out = Map::new();
                for f in &fields {
                    if let Some(v) = map.get(*f) {
                        out.insert(f.to_string(), v.clone());
                    }
                }
                Value::Object(out)
            }
            other => other.clone(),
        })
        .collect();

    serde_json::to_string_pretty(&Value::Array(result)).map_err(|e| e.to_string())
}

fn flatten_array(json: &Value, args: &Value) -> Result<String, String> {
    let arr = require_array(json, "flatten")?;
    let key = args.get("key").and_then(|v| v.as_str());

    let mut out: Vec<Value> = Vec::new();
    for item in arr {
        if let Some(k) = key {
            // Flatten nested array at this key
            if let Some(nested) = item.get(k).and_then(|v| v.as_array()) {
                out.extend(nested.iter().cloned());
            }
        } else {
            // Flatten one level
            match item {
                Value::Array(inner) => out.extend(inner.iter().cloned()),
                other => out.push(other.clone()),
            }
        }
    }
    serde_json::to_string_pretty(&Value::Array(out)).map_err(|e| e.to_string())
}

fn count(json: &Value) -> Result<String, String> {
    match json {
        Value::Array(arr) => Ok(format!("{}", arr.len())),
        Value::Object(map) => Ok(format!("{} keys", map.len())),
        Value::String(s) => Ok(format!("{} chars", s.len())),
        _ => Ok("1".to_string()),
    }
}

fn sort_array(json: &Value, args: &Value) -> Result<String, String> {
    let arr = require_array(json, "sort")?;
    let key = args.get("key").and_then(|v| v.as_str());
    let reverse = args
        .get("reverse")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut items = arr.to_vec();
    items.sort_by(|a, b| {
        let va = if let Some(k) = key {
            a.get(k).unwrap_or(a)
        } else {
            a
        };
        let vb = if let Some(k) = key {
            b.get(k).unwrap_or(b)
        } else {
            b
        };
        compare_values(va, vb)
    });
    if reverse {
        items.reverse();
    }
    serde_json::to_string_pretty(&Value::Array(items)).map_err(|e| e.to_string())
}

fn unique_array(json: &Value, args: &Value) -> Result<String, String> {
    let arr = require_array(json, "unique")?;
    let key = args.get("key").and_then(|v| v.as_str());

    let mut seen: Vec<String> = Vec::new();
    let mut out: Vec<Value> = Vec::new();
    for item in arr {
        let fingerprint = if let Some(k) = key {
            item.get(k).map(|v| v.to_string()).unwrap_or_default()
        } else {
            item.to_string()
        };
        if !seen.contains(&fingerprint) {
            seen.push(fingerprint);
            out.push(item.clone());
        }
    }
    serde_json::to_string_pretty(&Value::Array(out)).map_err(|e| e.to_string())
}

fn merge(args: &Value) -> Result<String, String> {
    let a_str = args
        .get("json")
        .and_then(|v| v.as_str())
        .ok_or("json_tools merge: 'json' (first object) is required")?;
    let b_str = args
        .get("with")
        .and_then(|v| v.as_str())
        .ok_or("json_tools merge: 'with' (second object) is required")?;

    let mut a: Map<String, Value> = serde_json::from_str(a_str)
        .map_err(|e| format!("json_tools merge: invalid JSON in 'json': {e}"))?;
    let b: Map<String, Value> = serde_json::from_str(b_str)
        .map_err(|e| format!("json_tools merge: invalid JSON in 'with': {e}"))?;

    for (k, v) in b {
        a.insert(k, v);
    }
    serde_json::to_string_pretty(&Value::Object(a)).map_err(|e| e.to_string())
}

fn json_diff(args: &Value) -> Result<String, String> {
    let a_str = args
        .get("json")
        .and_then(|v| v.as_str())
        .ok_or("json_tools diff: 'json' (first object) is required")?;
    let b_str = args
        .get("with")
        .and_then(|v| v.as_str())
        .ok_or("json_tools diff: 'with' (second object) is required")?;

    let a: Value = serde_json::from_str(a_str)
        .map_err(|e| format!("json_tools diff: invalid JSON in 'json': {e}"))?;
    let b: Value = serde_json::from_str(b_str)
        .map_err(|e| format!("json_tools diff: invalid JSON in 'with': {e}"))?;

    let mut changes: Vec<String> = Vec::new();
    diff_values("$", &a, &b, &mut changes);

    if changes.is_empty() {
        Ok("No differences found — the two JSON values are identical.".to_string())
    } else {
        Ok(format!(
            "JSON DIFF ({} change(s)):\n{}",
            changes.len(),
            changes.join("\n")
        ))
    }
}

fn validate(json: &Value) -> Result<String, String> {
    let summary = match json {
        Value::Object(map) => format!("Valid JSON object with {} keys.", map.len()),
        Value::Array(arr) => format!("Valid JSON array with {} elements.", arr.len()),
        Value::String(s) => format!("Valid JSON string ({} chars).", s.len()),
        Value::Number(n) => format!("Valid JSON number: {n}."),
        Value::Bool(b) => format!("Valid JSON boolean: {b}."),
        Value::Null => "Valid JSON null.".to_string(),
    };
    Ok(format!("VALID: {summary}"))
}

fn infer_schema(json: &Value) -> Result<String, String> {
    let schema = build_schema(json, 0);
    Ok(schema)
}

fn build_schema(val: &Value, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    match val {
        Value::Object(map) => {
            if map.is_empty() {
                return format!("{indent}{{}}");
            }
            let mut lines = vec![format!("{indent}{{")];
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in &keys {
                let v = &map[*k];
                let child = build_schema(v, depth + 1);
                lines.push(format!("{}  \"{k}\": {}", indent, child.trim()));
            }
            lines.push(format!("{indent}}}"));
            lines.join("\n")
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                return format!("{indent}[]");
            }
            let sample = build_schema(&arr[0], depth + 1);
            format!("{indent}[{}] ({} items)", sample.trim(), arr.len())
        }
        Value::String(_) => format!("{indent}<string>"),
        Value::Number(n) => {
            if n.is_f64() {
                format!("{indent}<float>")
            } else {
                format!("{indent}<integer>")
            }
        }
        Value::Bool(_) => format!("{indent}<boolean>"),
        Value::Null => format!("{indent}<null>"),
    }
}

fn array_stats(json: &Value) -> Result<String, String> {
    let arr = require_array(json, "stats")?;
    if arr.is_empty() {
        return Ok("Empty array — no statistics to compute.".to_string());
    }

    // Try numeric stats
    let nums: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
    let mut out = format!("Array length: {}\n", arr.len());

    if !nums.is_empty() && nums.len() == arr.len() {
        let sum: f64 = nums.iter().sum();
        let mean = sum / nums.len() as f64;
        let mut sorted = nums.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = if sorted.len().is_multiple_of(2) {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };
        let variance = nums.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / nums.len() as f64;
        let std_dev = variance.sqrt();
        out.push_str(&format!(
            "Type   : numeric\n\
             Min    : {:.4}\n\
             Max    : {:.4}\n\
             Sum    : {:.4}\n\
             Mean   : {:.4}\n\
             Median : {:.4}\n\
             StdDev : {:.4}\n",
            sorted[0],
            sorted[sorted.len() - 1],
            sum,
            mean,
            median,
            std_dev
        ));
    } else {
        // String or mixed stats
        let types: std::collections::BTreeMap<&str, usize> =
            arr.iter()
                .fold(std::collections::BTreeMap::new(), |mut m, v| {
                    *m.entry(type_name(v)).or_default() += 1;
                    m
                });
        for (t, n) in &types {
            out.push_str(&format!("  {t}: {n} item(s)\n"));
        }
    }

    Ok(out)
}

fn to_csv(json: &Value) -> Result<String, String> {
    let arr = require_array(json, "to-csv")?;
    if arr.is_empty() {
        return Ok("(empty array)".to_string());
    }

    // Collect all keys from all objects
    let mut all_keys: Vec<String> = Vec::new();
    for item in arr {
        if let Value::Object(map) = item {
            for k in map.keys() {
                if !all_keys.contains(k) {
                    all_keys.push(k.clone());
                }
            }
        }
    }
    if all_keys.is_empty() {
        return Err("json_tools to-csv: array must contain objects with string keys".into());
    }

    let mut lines = vec![all_keys.join(",")];
    for item in arr {
        let row: Vec<String> = all_keys
            .iter()
            .map(|k| match item.get(k) {
                Some(Value::String(s)) => csv_escape(s),
                Some(v) => v.to_string(),
                None => String::new(),
            })
            .collect();
        lines.push(row.join(","));
    }
    Ok(lines.join("\n"))
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn navigate<'a>(json: &'a Value, path: &str) -> Result<&'a Value, String> {
    let mut current = json;
    for segment in path.split('.') {
        if segment.is_empty() {
            continue;
        }
        // Handle array index: "items[0]" or "[0]"
        if let Some(bracket) = segment.find('[') {
            let key = &segment[..bracket];
            let idx_str = segment[bracket + 1..].trim_end_matches(']');
            let idx: usize = idx_str
                .parse()
                .map_err(|_| format!("json_tools get: invalid array index '{idx_str}'"))?;
            if !key.is_empty() {
                current = current
                    .get(key)
                    .ok_or_else(|| format!("json_tools get: key '{key}' not found"))?;
            }
            current = current
                .get(idx)
                .ok_or_else(|| format!("json_tools get: index [{idx}] out of bounds"))?;
        } else {
            current = current
                .get(segment)
                .ok_or_else(|| format!("json_tools get: key '{segment}' not found"))?;
        }
    }
    Ok(current)
}

fn require_array<'a>(json: &'a Value, action: &str) -> Result<&'a Vec<Value>, String> {
    json.as_array()
        .ok_or_else(|| format!("json_tools {action}: input must be a JSON array"))
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Object(_) => "object",
        Value::Array(_) => "array",
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Null => "null",
    }
}

fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a.as_f64(), b.as_f64()) {
        (Some(fa), Some(fb)) => fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal),
        _ => a.to_string().cmp(&b.to_string()),
    }
}

fn cmp_numeric(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    a.as_f64()?.partial_cmp(&b.as_f64()?)
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn diff_values(path: &str, a: &Value, b: &Value, changes: &mut Vec<String>) {
    if a == b {
        return;
    }
    match (a, b) {
        (Value::Object(ma), Value::Object(mb)) => {
            let mut all_keys: Vec<&String> = ma.keys().chain(mb.keys()).collect();
            all_keys.sort();
            all_keys.dedup();
            for k in all_keys {
                let child_path = format!("{path}.{k}");
                match (ma.get(k), mb.get(k)) {
                    (Some(va), Some(vb)) => diff_values(&child_path, va, vb, changes),
                    (Some(va), None) => changes.push(format!("  - {child_path}: {va}")),
                    (None, Some(vb)) => changes.push(format!("  + {child_path}: {vb}")),
                    _ => {}
                }
            }
        }
        _ => {
            changes.push(format!("  ~ {path}: {a} → {b}"));
        }
    }
}
