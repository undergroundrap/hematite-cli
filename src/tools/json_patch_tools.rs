use serde_json::{json, Map, Value};

pub fn json_patch_tools_schema() -> Value {
    json!({
        "name": "json_patch_tools",
        "description": "Apply and generate JSON Patch (RFC 6902) and JSON Merge Patch (RFC 7396) documents without external utilities. Actions: apply (apply a JSON Patch operation list to a JSON document), generate (generate a JSON Patch from two JSON documents), merge_apply (apply a JSON Merge Patch to a document), merge_generate (create a JSON Merge Patch from original and modified), test (validate 'test' operations in a patch document against the current document).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["apply", "generate", "merge_apply", "merge_generate", "test"],
                    "description": "Operation to perform (default: apply)"
                },
                "document": {
                    "description": "Source JSON document to patch or compare (inline JSON value or JSON string)"
                },
                "patch": {
                    "description": "JSON Patch array (RFC 6902) of {op, path, value?, from?} objects, or JSON Merge Patch object (RFC 7396)"
                },
                "original": {
                    "description": "Original JSON document for 'generate' and 'merge_generate' actions"
                },
                "modified": {
                    "description": "Modified JSON document for 'generate' and 'merge_generate' actions"
                }
            },
            "required": []
        }
    })
}

// ── JSON Pointer (RFC 6901) ────────────────────────────────────────────────────

fn parse_pointer(ptr: &str) -> Result<Vec<String>, String> {
    if ptr.is_empty() {
        return Ok(vec![]);
    }
    if !ptr.starts_with('/') {
        return Err(format!(
            "Invalid JSON Pointer: '{ptr}' — must start with '/'"
        ));
    }
    Ok(ptr[1..]
        .split('/')
        .map(|tok| tok.replace("~1", "/").replace("~0", "~"))
        .collect())
}

fn escape_token(s: &str) -> String {
    s.replace('~', "~0").replace('/', "~1")
}

fn ptr_get<'a>(doc: &'a Value, parts: &[String]) -> Option<&'a Value> {
    if parts.is_empty() {
        return Some(doc);
    }
    match doc {
        Value::Object(map) => ptr_get(map.get(&parts[0])?, &parts[1..]),
        Value::Array(arr) => {
            let idx: usize = parts[0].parse().ok()?;
            ptr_get(arr.get(idx)?, &parts[1..])
        }
        _ => None,
    }
}

fn ptr_set(doc: &mut Value, parts: &[String], val: Value) -> Result<(), String> {
    if parts.is_empty() {
        *doc = val;
        return Ok(());
    }
    let (head, tail) = (&parts[0], &parts[1..]);
    match doc {
        Value::Object(map) => {
            if tail.is_empty() {
                map.insert(head.clone(), val);
            } else {
                let child = map
                    .get_mut(head.as_str())
                    .ok_or_else(|| format!("Key not found: '{head}'"))?;
                ptr_set(child, tail, val)?;
            }
        }
        Value::Array(arr) => {
            if tail.is_empty() {
                if head == "-" {
                    arr.push(val);
                } else {
                    let idx: usize = head
                        .parse()
                        .map_err(|_| format!("Invalid array index: '{head}'"))?;
                    if idx > arr.len() {
                        return Err(format!("Index {idx} out of bounds (len {})", arr.len()));
                    }
                    arr.insert(idx, val);
                }
            } else {
                let idx: usize = head
                    .parse()
                    .map_err(|_| format!("Invalid array index: '{head}'"))?;
                let child = arr
                    .get_mut(idx)
                    .ok_or_else(|| format!("Index {idx} out of bounds"))?;
                ptr_set(child, tail, val)?;
            }
        }
        _ => return Err(format!("Cannot navigate into a scalar at '{head}'")),
    }
    Ok(())
}

fn ptr_remove(doc: &mut Value, parts: &[String]) -> Result<Value, String> {
    if parts.is_empty() {
        return Err("Cannot remove the root document".to_string());
    }
    let (head, tail) = (&parts[0], &parts[1..]);
    match doc {
        Value::Object(map) => {
            if tail.is_empty() {
                map.remove(head.as_str())
                    .ok_or_else(|| format!("Key not found: '{head}'"))
            } else {
                let child = map
                    .get_mut(head.as_str())
                    .ok_or_else(|| format!("Key not found: '{head}'"))?;
                ptr_remove(child, tail)
            }
        }
        Value::Array(arr) => {
            if tail.is_empty() {
                let idx: usize = head
                    .parse()
                    .map_err(|_| format!("Invalid array index: '{head}'"))?;
                if idx >= arr.len() {
                    return Err(format!("Index {idx} out of bounds (len {})", arr.len()));
                }
                Ok(arr.remove(idx))
            } else {
                let idx: usize = head
                    .parse()
                    .map_err(|_| format!("Invalid array index: '{head}'"))?;
                let child = arr
                    .get_mut(idx)
                    .ok_or_else(|| format!("Index {idx} out of bounds"))?;
                ptr_remove(child, tail)
            }
        }
        _ => Err(format!("Cannot navigate into a scalar at '{head}'")),
    }
}

// ── JSON Patch (RFC 6902) ─────────────────────────────────────────────────────

fn apply_patch(mut doc: Value, ops: &[Value]) -> Result<(Value, Vec<String>), String> {
    let mut log: Vec<String> = Vec::new();
    for (i, op_val) in ops.iter().enumerate() {
        let op = op_val
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Operation {i}: missing 'op'"))?;
        let path_str = op_val
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Operation {i}: missing 'path'"))?;
        let parts = parse_pointer(path_str)?;

        match op {
            "add" => {
                let val = op_val
                    .get("value")
                    .cloned()
                    .ok_or_else(|| format!("Operation {i} (add): missing 'value'"))?;
                log.push(format!("  add    {path_str}"));
                ptr_set(&mut doc, &parts, val)?;
            }
            "remove" => {
                log.push(format!("  remove {path_str}"));
                ptr_remove(&mut doc, &parts)?;
            }
            "replace" => {
                let val = op_val
                    .get("value")
                    .cloned()
                    .ok_or_else(|| format!("Operation {i} (replace): missing 'value'"))?;
                ptr_remove(&mut doc, &parts)?;
                log.push(format!("  replace {path_str}"));
                ptr_set(&mut doc, &parts, val)?;
            }
            "move" => {
                let from_str = op_val
                    .get("from")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("Operation {i} (move): missing 'from'"))?;
                let from_parts = parse_pointer(from_str)?;
                let moved = ptr_remove(&mut doc, &from_parts)?;
                log.push(format!("  move   {from_str} → {path_str}"));
                ptr_set(&mut doc, &parts, moved)?;
            }
            "copy" => {
                let from_str = op_val
                    .get("from")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("Operation {i} (copy): missing 'from'"))?;
                let from_parts = parse_pointer(from_str)?;
                let copied = ptr_get(&doc, &from_parts)
                    .cloned()
                    .ok_or_else(|| format!("Operation {i} (copy): 'from' path not found"))?;
                log.push(format!("  copy   {from_str} → {path_str}"));
                ptr_set(&mut doc, &parts, copied)?;
            }
            "test" => {
                let expected = op_val
                    .get("value")
                    .ok_or_else(|| format!("Operation {i} (test): missing 'value'"))?;
                let actual = ptr_get(&doc, &parts)
                    .ok_or_else(|| format!("Operation {i} (test): path '{path_str}' not found"))?;
                if actual != expected {
                    return Err(format!(
                        "Operation {i} (test): FAILED at '{path_str}'\n  expected: {expected}\n  actual:   {actual}"
                    ));
                }
                log.push(format!("  test   {path_str} ✓"));
            }
            _ => return Err(format!("Operation {i}: unknown op '{op}'")),
        }
    }
    Ok((doc, log))
}

// ── JSON Patch generation (diff) ──────────────────────────────────────────────

fn diff_values(a: &Value, b: &Value, path: &str, ops: &mut Vec<Value>) {
    if a == b {
        return;
    }
    match (a, b) {
        (Value::Object(ao), Value::Object(bo)) => {
            // Removals
            for k in ao.keys() {
                if !bo.contains_key(k.as_str()) {
                    ops.push(json!({
                        "op": "remove",
                        "path": format!("{path}/{}", escape_token(k))
                    }));
                }
            }
            // Additions and replacements
            for (k, bv) in bo.iter() {
                let child_path = format!("{path}/{}", escape_token(k));
                match ao.get(k.as_str()) {
                    Some(av) => diff_values(av, bv, &child_path, ops),
                    None => ops.push(json!({ "op": "add", "path": child_path, "value": bv })),
                }
            }
        }
        (Value::Array(aa), Value::Array(ba)) => {
            let min_len = aa.len().min(ba.len());
            for i in 0..min_len {
                diff_values(&aa[i], &ba[i], &format!("{path}/{i}"), ops);
            }
            // Extra elements in b → add
            for i in min_len..ba.len() {
                ops.push(json!({ "op": "add", "path": format!("{path}/{i}"), "value": &ba[i] }));
            }
            // Extra elements in a → remove (in reverse to keep indices stable)
            for i in (min_len..aa.len()).rev() {
                ops.push(json!({ "op": "remove", "path": format!("{path}/{i}") }));
            }
        }
        _ => {
            ops.push(json!({ "op": "replace", "path": path, "value": b }));
        }
    }
}

// ── JSON Merge Patch (RFC 7396) ───────────────────────────────────────────────

fn merge_apply(mut doc: Value, patch: &Value) -> Value {
    match (doc.as_object_mut(), patch.as_object()) {
        (Some(doc_map), Some(patch_map)) => {
            for (k, pv) in patch_map {
                if pv.is_null() {
                    doc_map.remove(k.as_str());
                } else {
                    let existing = doc_map.remove(k.as_str()).unwrap_or(Value::Null);
                    doc_map.insert(k.clone(), merge_apply(existing, pv));
                }
            }
            doc
        }
        _ => patch.clone(),
    }
}

fn merge_generate(original: &Value, modified: &Value) -> Value {
    match (original.as_object(), modified.as_object()) {
        (Some(oo), Some(mo)) => {
            let mut patch = Map::new();
            // Keys removed → null
            for k in oo.keys() {
                if !mo.contains_key(k.as_str()) {
                    patch.insert(k.clone(), Value::Null);
                }
            }
            // Keys added or changed
            for (k, mv) in mo {
                match oo.get(k.as_str()) {
                    Some(ov) if ov == mv => {} // unchanged
                    Some(ov) => {
                        patch.insert(k.clone(), merge_generate(ov, mv));
                    }
                    None => {
                        patch.insert(k.clone(), mv.clone());
                    }
                }
            }
            Value::Object(patch)
        }
        _ => {
            if original == modified {
                Value::Object(Map::new())
            } else {
                modified.clone()
            }
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn parse_json_arg(v: &Value) -> Result<Value, String> {
    match v {
        Value::String(s) => serde_json::from_str(s).map_err(|e| format!("Invalid JSON: {e}")),
        other => Ok(other.clone()),
    }
}

fn get_json(args: &Value, key: &str) -> Result<Value, String> {
    let raw = args
        .get(key)
        .ok_or_else(|| format!("'{key}' is required"))?;
    parse_json_arg(raw)
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_apply(args: &Value) -> Result<String, String> {
    let doc = get_json(args, "document")?;
    let patch_val = get_json(args, "patch")?;
    let ops = patch_val
        .as_array()
        .ok_or("'patch' must be a JSON array of operations")?;

    let (result, log) = apply_patch(doc, ops)?;

    let mut out = format!("Operations applied:  {}\n\n", ops.len());
    for entry in &log {
        out.push_str(entry);
        out.push('\n');
    }
    out.push_str("\nResult:\n");
    out.push_str(
        &serde_json::to_string_pretty(&result).map_err(|e| format!("Serialization error: {e}"))?,
    );
    out.push('\n');
    Ok(out)
}

fn action_generate(args: &Value) -> Result<String, String> {
    let original = get_json(args, "original")?;
    let modified = get_json(args, "modified")?;

    let mut ops: Vec<Value> = Vec::new();
    diff_values(&original, &modified, "", &mut ops);

    let mut out = format!("Generated {} operation(s)\n\n", ops.len());
    out.push_str("Patch:\n");
    out.push_str(
        &serde_json::to_string_pretty(&Value::Array(ops))
            .map_err(|e| format!("Serialization error: {e}"))?,
    );
    out.push('\n');
    Ok(out)
}

fn action_merge_apply(args: &Value) -> Result<String, String> {
    let doc = get_json(args, "document")?;
    let patch = get_json(args, "patch")?;
    let result = merge_apply(doc, &patch);
    let mut out = "JSON Merge Patch applied (RFC 7396)\n\nResult:\n".to_string();
    out.push_str(
        &serde_json::to_string_pretty(&result).map_err(|e| format!("Serialization error: {e}"))?,
    );
    out.push('\n');
    Ok(out)
}

fn action_merge_generate(args: &Value) -> Result<String, String> {
    let original = get_json(args, "original")?;
    let modified = get_json(args, "modified")?;
    let patch = merge_generate(&original, &modified);
    let mut out = "JSON Merge Patch (RFC 7396)\n\nPatch:\n".to_string();
    out.push_str(
        &serde_json::to_string_pretty(&patch).map_err(|e| format!("Serialization error: {e}"))?,
    );
    out.push('\n');
    Ok(out)
}

fn action_test(args: &Value) -> Result<String, String> {
    let doc = get_json(args, "document")?;
    let patch_val = get_json(args, "patch")?;
    let ops = patch_val
        .as_array()
        .ok_or("'patch' must be a JSON array of operations")?;

    // Filter to test-only operations
    let test_ops: Vec<&Value> = ops
        .iter()
        .filter(|op| op.get("op").and_then(|v| v.as_str()) == Some("test"))
        .collect();

    if test_ops.is_empty() {
        return Ok("No 'test' operations found in patch.\n".to_string());
    }

    let mut out = format!("Test operations: {}\n\n", test_ops.len());
    let mut all_pass = true;

    for (i, op) in test_ops.iter().enumerate() {
        let path_str = op.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let expected = op.get("value");
        let parts = parse_pointer(path_str)?;
        let actual = ptr_get(&doc, &parts);

        let pass = match (actual, expected) {
            (Some(a), Some(e)) => a == e,
            (None, Some(_)) => false,
            _ => false,
        };

        if !pass {
            all_pass = false;
        }

        out.push_str(&format!(
            "  [{}] {}  {}\n",
            i + 1,
            if pass { "PASS ✓" } else { "FAIL ✗" },
            path_str
        ));
        if !pass {
            if let Some(e) = expected {
                out.push_str(&format!("       expected: {e}\n"));
            }
            if let Some(a) = actual {
                out.push_str(&format!("       actual:   {a}\n"));
            } else {
                out.push_str("       actual:   (path not found)\n");
            }
        }
    }

    out.push_str(&format!(
        "\nVerdict: {}\n",
        if all_pass {
            "ALL PASS ✓"
        } else {
            "SOME FAILED ✗"
        }
    ));
    Ok(out)
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("apply");

    match action {
        "apply" => action_apply(args),
        "generate" => action_generate(args),
        "merge_apply" => action_merge_apply(args),
        "merge_generate" => action_merge_generate(args),
        "test" => action_test(args),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: apply, generate, merge_apply, merge_generate, test"
        )),
    }
}
