use serde_json::Value;
use std::path::PathBuf;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("query");
    match action {
        "query" | "get" | "select" => action_query(args),
        "keys" => action_keys(args),
        "values" => action_values(args),
        "flatten" => action_flatten(args),
        "map" => action_map(args),
        "filter" => action_filter(args),
        "count" => action_count(args),
        "type" => action_type(args),
        other => Err(format!(
            "jq_tools: unknown action '{other}'. \
             Valid: query, keys, values, flatten, map, filter, count, type"
        )),
    }
}

// ── input resolution ─────────────────────────────────────────────────────────

fn resolve_json(args: &Value) -> Result<Value, String> {
    // Inline JSON value (already parsed by caller — serde_json object/array)
    if let Some(v) = args.get("json") {
        // Could be an already-parsed Value or a string containing JSON
        if let Some(s) = v.as_str() {
            return serde_json::from_str(s)
                .map_err(|e| format!("jq_tools: invalid JSON in 'json': {e}"));
        }
        return Ok(v.clone());
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
        let content = std::fs::read_to_string(&full)
            .map_err(|e| format!("jq_tools: cannot read '{}': {e}", full.display()))?;
        return serde_json::from_str(&content)
            .map_err(|e| format!("jq_tools: invalid JSON in '{}': {e}", full.display()));
    }
    Err("jq_tools: provide 'json' (JSON value or string) or 'file' (path to JSON file)".into())
}

fn path_arg(args: &Value) -> Option<&str> {
    args.get("path")
        .or_else(|| args.get("q"))
        .and_then(|v| v.as_str())
}

// ── path evaluator ────────────────────────────────────────────────────────────

/// Evaluate a path expression against a JSON value.
/// Returns a Vec of results (multiple items come from `[]` iteration or `,` multi-path).
fn eval_path<'a>(root: &'a Value, path: &str) -> Result<Vec<&'a Value>, String> {
    // Handle multi-path: ".name, .age"
    if path.contains(',') {
        let mut results: Vec<&'a Value> = Vec::new();
        for sub in split_top_level_commas(path) {
            let sub = sub.trim();
            let sub_results = eval_single_path(root, sub)?;
            results.extend(sub_results);
        }
        return Ok(results);
    }
    eval_single_path(root, path)
}

/// Split a comma-separated path at the top level (not inside brackets).
fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

fn eval_single_path<'a>(root: &'a Value, path: &str) -> Result<Vec<&'a Value>, String> {
    let path = path.trim();

    // Handle built-in builtins on the root when there's no dot-path prefix
    match path {
        "" | "." => return Ok(vec![root]),
        "length" => {
            return Err(
                "jq_tools: 'length' is a builtin — use action: 'count' or path: '.' with no path"
                    .into(),
            );
        }
        "keys" | "type" | "values" | "first" | "last" | "reverse" | "sort" | "unique" | "min"
        | "max" | "add" => {
            return apply_builtin(root, path, root);
        }
        _ => {}
    }

    // Strip leading dot
    let path = path.strip_prefix('.').unwrap_or(path);

    if path.is_empty() {
        return Ok(vec![root]);
    }

    // Check for trailing builtin: ".field.sub | keys"
    if let Some(pipe_pos) = find_top_level_pipe(path) {
        let nav_path = path[..pipe_pos].trim();
        let builtin = path[pipe_pos + 1..].trim();
        let navigated = navigate(root, nav_path)?;
        return apply_builtin(root, builtin, navigated);
    }

    // Navigate step by step
    let current = navigate(root, path)?;
    Ok(vec![current])
}

/// Find a `|` at depth 0.
fn find_top_level_pipe(s: &str) -> Option<usize> {
    let mut depth: i32 = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            '|' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Apply a builtin function to a value.
fn apply_builtin<'a>(
    root: &'a Value,
    builtin: &str,
    target: &'a Value,
) -> Result<Vec<&'a Value>, String> {
    match builtin.trim() {
        "keys" => match target {
            Value::Object(map) => {
                // We can't return refs to temporary values, so we special-case and
                // signal the caller to handle formatting.
                let _ = (root, map);
                Err(format!(
                    "__builtin_keys:{}",
                    map.keys().cloned().collect::<Vec<_>>().join("\n")
                ))
            }
            Value::Array(arr) => Err(format!(
                "__builtin_keys:{}",
                (0..arr.len())
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            )),
            _ => Err("jq_tools: keys — input must be an object or array".into()),
        },
        "values" => match target {
            Value::Object(map) => {
                Err(format!("__builtin_values_object:{}", {
                    let vals: Vec<String> =
                        map.values().map(format_scalar).collect();
                    vals.join("\n")
                }))
            }
            Value::Array(_) => Ok(vec![target]),
            _ => Err("jq_tools: values — input must be an object or array".into()),
        },
        "type" => Err(format!("__builtin_type:{}", json_type_name(target))),
        "length" => Err(format!("__builtin_length:{}", json_length(target))),
        "first" => match target {
            Value::Array(arr) => arr
                .first()
                .map(|v| vec![v])
                .ok_or_else(|| "jq_tools: first — array is empty".into()),
            _ => Err("jq_tools: first — input must be an array".into()),
        },
        "last" => match target {
            Value::Array(arr) => arr
                .last()
                .map(|v| vec![v])
                .ok_or_else(|| "jq_tools: last — array is empty".into()),
            _ => Err("jq_tools: last — input must be an array".into()),
        },
        "reverse" => Err(format!(
            "__builtin_json:{}",
            serde_json::to_string_pretty(&{
                let arr = target
                    .as_array()
                    .ok_or("jq_tools: reverse — input must be an array")?;
                let rev: Vec<_> = arr.iter().rev().cloned().collect();
                Value::Array(rev)
            })
            .map_err(|e| e.to_string())?
        )),
        "sort" => Err(format!(
            "__builtin_json:{}",
            serde_json::to_string_pretty(&{
                let arr = target
                    .as_array()
                    .ok_or("jq_tools: sort — input must be an array")?;
                let mut sorted = arr.clone();
                sorted.sort_by(compare_values);
                Value::Array(sorted)
            })
            .map_err(|e| e.to_string())?
        )),
        "unique" => Err(format!(
            "__builtin_json:{}",
            serde_json::to_string_pretty(&{
                let arr = target
                    .as_array()
                    .ok_or("jq_tools: unique — input must be an array")?;
                let mut seen: Vec<String> = Vec::new();
                let mut out: Vec<Value> = Vec::new();
                for item in arr {
                    let key = item.to_string();
                    if !seen.contains(&key) {
                        seen.push(key);
                        out.push(item.clone());
                    }
                }
                Value::Array(out)
            })
            .map_err(|e| e.to_string())?
        )),
        "min" => {
            let arr = target
                .as_array()
                .ok_or("jq_tools: min — input must be an array")?;
            let min = arr
                .iter()
                .filter_map(|v| v.as_f64())
                .reduce(f64::min)
                .ok_or("jq_tools: min — array contains no numbers")?;
            Err(format!("__builtin_scalar:{}", format_float(min)))
        }
        "max" => {
            let arr = target
                .as_array()
                .ok_or("jq_tools: max — input must be an array")?;
            let max = arr
                .iter()
                .filter_map(|v| v.as_f64())
                .reduce(f64::max)
                .ok_or("jq_tools: max — array contains no numbers")?;
            Err(format!("__builtin_scalar:{}", format_float(max)))
        }
        "add" => {
            let arr = target
                .as_array()
                .ok_or("jq_tools: add — input must be an array")?;
            if arr.is_empty() {
                return Err("__builtin_scalar:null".into());
            }
            // Numbers: sum; strings: concatenate
            if arr.iter().all(|v| v.is_number()) {
                let sum: f64 = arr.iter().filter_map(|v| v.as_f64()).sum();
                return Err(format!("__builtin_scalar:{}", format_float(sum)));
            }
            if arr.iter().all(|v| v.is_string()) {
                let joined: String = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("");
                return Err(format!(
                    "__builtin_json:{}",
                    serde_json::to_string_pretty(&Value::String(joined)).unwrap_or_default()
                ));
            }
            Err("jq_tools: add — array must contain all numbers or all strings".into())
        }
        other => Err(format!(
            "jq_tools: unknown builtin '{}'. \
             Valid builtins: keys, values, type, length, first, last, reverse, sort, unique, min, max, add",
            other
        )),
    }
}

/// Navigate a dot-notation path (with `[N]` support) to a value.
fn navigate<'a>(root: &'a Value, path: &str) -> Result<&'a Value, String> {
    // Tokenize: split on '.' but keep bracket accesses attached to their segment
    let segments = tokenize_path(path);
    let mut current = root;

    for seg in segments {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }

        // Check for .field[] — iterate all elements (handled at caller level)
        if seg.ends_with("[]") {
            let key = seg.trim_end_matches("[]");
            if !key.is_empty() {
                current = current
                    .get(key)
                    .ok_or_else(|| format!("jq_tools: key '{}' not found", key))?;
            }
            // Return the array itself; the caller handles iteration
            return Ok(current);
        }

        // Check for bracket index: "field[0]" or "[0]" or "field[-1]"
        if let Some(bracket) = seg.find('[') {
            let key = &seg[..bracket];
            let idx_str = seg[bracket + 1..].trim_end_matches(']');

            if !key.is_empty() {
                current = current
                    .get(key)
                    .ok_or_else(|| format!("jq_tools: key '{}' not found", key))?;
            }

            let arr = current
                .as_array()
                .ok_or_else(|| format!("jq_tools: '[{}]' applied to non-array", idx_str))?;

            let idx: isize = idx_str
                .parse()
                .map_err(|_| format!("jq_tools: invalid array index '{}'", idx_str))?;

            let resolved = if idx < 0 {
                let pos = arr.len() as isize + idx;
                if pos < 0 {
                    return Err(format!(
                        "jq_tools: index [{}] out of bounds (len {})",
                        idx,
                        arr.len()
                    ));
                }
                pos as usize
            } else {
                idx as usize
            };

            current = arr.get(resolved).ok_or_else(|| {
                format!(
                    "jq_tools: index [{}] out of bounds (len {})",
                    idx,
                    arr.len()
                )
            })?;
        } else {
            // Plain key
            current = current
                .get(seg)
                .ok_or_else(|| format!("jq_tools: key '{}' not found", seg))?;
        }
    }

    Ok(current)
}

/// Tokenize a dot-separated path, keeping bracket expressions intact.
/// "a.b[0].c" → ["a", "b[0]", "c"]
fn tokenize_path(path: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = Vec::new();
    let mut start = 0;
    let mut in_bracket = false;
    for (i, ch) in path.char_indices() {
        match ch {
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            '.' if !in_bracket => {
                if i > start {
                    parts.push(&path[start..i]);
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < path.len() {
        parts.push(&path[start..]);
    }
    parts
}

// ── action handlers ───────────────────────────────────────────────────────────

fn action_query(args: &Value) -> Result<String, String> {
    let json = resolve_json(args)?;

    let path = path_arg(args).unwrap_or(".");

    // Detect `[]` iteration at the end (or in the middle)
    let iterate = path.ends_with("[]") || path == ".[]" || path == "[]";

    // Handle `[]` direct iteration on root or nested
    if iterate {
        let target = if path == ".[]" || path == "[]" {
            &json
        } else {
            let nav_path = path.trim_end_matches("[]").trim_end_matches('.');
            if nav_path.is_empty() {
                &json
            } else {
                navigate(&json, nav_path.trim_start_matches('.'))?
            }
        };
        let arr = target
            .as_array()
            .ok_or("jq_tools: '[]' requires an array value")?;
        if arr.is_empty() {
            return Ok("(empty array)\n\n0 results".to_string());
        }
        let mut out = String::new();
        for item in arr {
            out += &format_value(item);
            out += "\n";
        }
        out += &format!("\n{} result(s)", arr.len());
        return Ok(out);
    }

    match eval_path(&json, path) {
        Ok(results) => {
            if results.is_empty() {
                return Ok("(no results)".to_string());
            }
            if results.len() == 1 {
                return Ok(format_value(results[0]));
            }
            let mut out = String::new();
            for r in &results {
                out += &format_value(r);
                out += "\n";
            }
            out += &format!("\n{} result(s)", results.len());
            Ok(out)
        }
        Err(e) => {
            // Handle special builtin return signals
            if let Some(val) = e.strip_prefix("__builtin_keys:") {
                return Ok(format!(
                    "Keys:\n{}\n",
                    val.lines()
                        .map(|l| format!("  {}", l))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
            if let Some(val) = e.strip_prefix("__builtin_values_object:") {
                return Ok(format!(
                    "Values:\n{}\n",
                    val.lines()
                        .map(|l| format!("  {}", l))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
            if let Some(val) = e.strip_prefix("__builtin_type:") {
                return Ok(format!("Type: {}", val));
            }
            if let Some(val) = e.strip_prefix("__builtin_length:") {
                return Ok(format!("Length: {}", val));
            }
            if let Some(val) = e.strip_prefix("__builtin_scalar:") {
                return Ok(val.to_string());
            }
            if let Some(val) = e.strip_prefix("__builtin_json:") {
                return Ok(val.to_string());
            }
            Err(e)
        }
    }
}

fn action_keys(args: &Value) -> Result<String, String> {
    let json = resolve_json(args)?;
    let target = if let Some(p) = path_arg(args) {
        navigate(&json, p.trim_start_matches('.')).cloned()
    } else {
        Ok(json.clone())
    }?;

    match &target {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = format!("Keys ({}):\n", keys.len());
            for k in &keys {
                out += &format!("  {}\n", k);
            }
            Ok(out)
        }
        Value::Array(arr) => {
            let mut out = format!("Array indices ({} elements):\n", arr.len());
            for i in 0..arr.len() {
                out += &format!("  [{}]\n", i);
            }
            Ok(out)
        }
        other => Err(format!(
            "jq_tools keys: expected object or array, got {}",
            json_type_name(other)
        )),
    }
}

fn action_values(args: &Value) -> Result<String, String> {
    let json = resolve_json(args)?;
    let target = if let Some(p) = path_arg(args) {
        navigate(&json, p.trim_start_matches('.')).cloned()
    } else {
        Ok(json.clone())
    }?;

    match &target {
        Value::Object(map) => {
            let mut out = format!("Values ({}):\n", map.len());
            for (k, v) in map {
                out += &format!("  {}:  {}\n", k, format_scalar(v));
            }
            Ok(out)
        }
        Value::Array(arr) => {
            let mut out = format!("Array elements ({}):\n", arr.len());
            for (i, v) in arr.iter().enumerate() {
                out += &format!("  [{}]  {}\n", i, format_scalar(v));
            }
            Ok(out)
        }
        other => Err(format!(
            "jq_tools values: expected object or array, got {}",
            json_type_name(other)
        )),
    }
}

fn action_flatten(args: &Value) -> Result<String, String> {
    let json = resolve_json(args)?;
    let target = if let Some(p) = path_arg(args) {
        navigate(&json, p.trim_start_matches('.')).cloned()
    } else {
        Ok(json.clone())
    }?;

    let depth = args
        .get("depth")
        .and_then(|v| v.as_u64())
        .map(|d| d as usize);

    let arr = target
        .as_array()
        .ok_or("jq_tools flatten: input must be an array")?;

    let flat = flatten_recursive(arr, depth.unwrap_or(usize::MAX));
    let result = Value::Array(flat);
    let count = result.as_array().map(|a| a.len()).unwrap_or(0);

    let json_out = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
    Ok(format!("{}\n\n{} element(s)", json_out, count))
}

fn flatten_recursive(arr: &[Value], depth: usize) -> Vec<Value> {
    if depth == 0 {
        return arr.to_vec();
    }
    let mut out = Vec::new();
    for item in arr {
        match item {
            Value::Array(inner) => {
                out.extend(flatten_recursive(inner, depth - 1));
            }
            other => out.push(other.clone()),
        }
    }
    out
}

fn action_map(args: &Value) -> Result<String, String> {
    let json = resolve_json(args)?;
    let target = if let Some(p) = path_arg(args) {
        navigate(&json, p.trim_start_matches('.')).cloned()
    } else {
        Ok(json.clone())
    }?;

    let arr = target
        .as_array()
        .ok_or("jq_tools map: input must be an array")?;

    let field = args
        .get("field")
        .and_then(|v| v.as_str())
        .ok_or("jq_tools map: 'field' is required — the field to extract from each element")?;

    let mut out_arr: Vec<Value> = Vec::new();
    let mut missing = 0usize;
    for item in arr {
        if let Some(v) = item.get(field) {
            out_arr.push(v.clone());
        } else {
            out_arr.push(Value::Null);
            missing += 1;
        }
    }

    let result = Value::Array(out_arr);
    let json_out = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
    let mut out = json_out;
    out += &format!("\n\n{} element(s)", arr.len());
    if missing > 0 {
        out += &format!(", {} null (field '{}' not present)", missing, field);
    }
    Ok(out)
}

fn action_filter(args: &Value) -> Result<String, String> {
    let json = resolve_json(args)?;
    let target = if let Some(p) = path_arg(args) {
        navigate(&json, p.trim_start_matches('.')).cloned()
    } else {
        Ok(json.clone())
    }?;

    let arr = target
        .as_array()
        .ok_or("jq_tools filter: input must be an array")?;

    let field = args
        .get("field")
        .and_then(|v| v.as_str())
        .ok_or("jq_tools filter: 'field' is required")?;

    // Determine filter mode
    if let Some(exists_val) = args.get("exists") {
        // exists: true/false — check for field presence
        let want = exists_val.as_bool().unwrap_or(true);
        let filtered: Vec<Value> = arr
            .iter()
            .filter(|item| item.get(field).is_some() == want)
            .cloned()
            .collect();
        let total = arr.len();
        let count = filtered.len();
        let result = Value::Array(filtered);
        let json_out = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
        return Ok(format!(
            "{}\n\nFiltered {} of {} element(s)  (field '{}' {})",
            json_out,
            count,
            total,
            field,
            if want { "present" } else { "absent" }
        ));
    }

    if let Some(gt_val) = args.get("gt").and_then(|v| v.as_f64()) {
        let filtered: Vec<Value> = arr
            .iter()
            .filter(|item| {
                item.get(field)
                    .and_then(|v| v.as_f64())
                    .map(|n| n > gt_val)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        let (count, total) = (filtered.len(), arr.len());
        let json_out =
            serde_json::to_string_pretty(&Value::Array(filtered)).map_err(|e| e.to_string())?;
        return Ok(format!(
            "{}\n\nFiltered {} of {} element(s)  ({} > {})",
            json_out, count, total, field, gt_val
        ));
    }

    if let Some(lt_val) = args.get("lt").and_then(|v| v.as_f64()) {
        let filtered: Vec<Value> = arr
            .iter()
            .filter(|item| {
                item.get(field)
                    .and_then(|v| v.as_f64())
                    .map(|n| n < lt_val)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        let (count, total) = (filtered.len(), arr.len());
        let json_out =
            serde_json::to_string_pretty(&Value::Array(filtered)).map_err(|e| e.to_string())?;
        return Ok(format!(
            "{}\n\nFiltered {} of {} element(s)  ({} < {})",
            json_out, count, total, field, lt_val
        ));
    }

    if let Some(contains_val) = args.get("contains").and_then(|v| v.as_str()) {
        let filtered: Vec<Value> = arr
            .iter()
            .filter(|item| {
                item.get(field)
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains(contains_val))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        let (count, total) = (filtered.len(), arr.len());
        let json_out =
            serde_json::to_string_pretty(&Value::Array(filtered)).map_err(|e| e.to_string())?;
        return Ok(format!(
            "{}\n\nFiltered {} of {} element(s)  ({} contains \"{}\")",
            json_out, count, total, field, contains_val
        ));
    }

    // Default: equality match
    let value = args
        .get("value")
        .ok_or("jq_tools filter: 'value' or 'contains' or 'gt'/'lt' or 'exists' is required")?;

    let filtered: Vec<Value> = arr
        .iter()
        .filter(|item| item.get(field).map(|v| v == value).unwrap_or(false))
        .cloned()
        .collect();
    let (count, total) = (filtered.len(), arr.len());
    let json_out =
        serde_json::to_string_pretty(&Value::Array(filtered)).map_err(|e| e.to_string())?;
    Ok(format!(
        "{}\n\nFiltered {} of {} element(s)  ({} == {})",
        json_out, count, total, field, value
    ))
}

fn action_count(args: &Value) -> Result<String, String> {
    let json = resolve_json(args)?;
    let target = if let Some(p) = path_arg(args) {
        navigate(&json, p.trim_start_matches('.')).cloned()
    } else {
        Ok(json.clone())
    }?;

    match &target {
        Value::Array(arr) => Ok(format!("{} element(s)", arr.len())),
        Value::Object(map) => Ok(format!("{} key(s)", map.len())),
        Value::String(s) => Ok(format!("{} character(s)", s.len())),
        Value::Null => Ok("0 (null)".to_string()),
        other => Ok(format!("1 ({})", json_type_name(other))),
    }
}

fn action_type(args: &Value) -> Result<String, String> {
    let json = resolve_json(args)?;
    let target = if let Some(p) = path_arg(args) {
        navigate(&json, p.trim_start_matches('.')).cloned()
    } else {
        Ok(json.clone())
    }?;

    let type_name = json_type_name(&target);
    let mut out = format!("Type: {}\n", type_name);

    match &target {
        Value::Array(arr) => {
            out += &format!("Count: {} elements\n", arr.len());
            // Element type distribution
            let mut counts: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for item in arr {
                *counts.entry(json_type_name(item)).or_insert(0) += 1;
            }
            if !counts.is_empty() {
                out += "Element types:\n";
                for (t, n) in &counts {
                    out += &format!("  {}: {}\n", t, n);
                }
            }
        }
        Value::Object(map) => {
            out += &format!("Count: {} keys\n", map.len());
        }
        Value::String(s) => {
            out += &format!("Length: {} chars\n", s.len());
        }
        Value::Number(n) => {
            if n.is_f64() {
                out += "Subtype: float\n";
            } else {
                out += "Subtype: integer\n";
            }
        }
        _ => {}
    }

    Ok(out)
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn format_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => format_scalar(v),
        other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
    }
}

fn format_scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                format_float(f)
            } else {
                n.to_string()
            }
        }
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn format_float(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        format!("{}", f)
    }
}

fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn json_length(v: &Value) -> usize {
    match v {
        Value::Array(arr) => arr.len(),
        Value::Object(map) => map.len(),
        Value::String(s) => s.len(),
        Value::Null => 0,
        _ => 1,
    }
}

fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a.as_f64(), b.as_f64()) {
        (Some(fa), Some(fb)) => fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal),
        _ => a.to_string().cmp(&b.to_string()),
    }
}
