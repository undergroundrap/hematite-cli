use serde_json::{json, Value};

pub fn jsonschema_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["validate", "info", "properties", "refs"],
                "description": "Operation: validate (check JSON instance against schema), info (schema summary), properties (list properties with types), refs (list $ref/$defs/$id references). Default: info."
            },
            "schema": {
                "type": "string",
                "description": "JSON Schema as inline JSON string or file path."
            },
            "schema_file": {
                "type": "string",
                "description": "Path to a JSON Schema file."
            },
            "instance": {
                "type": "string",
                "description": "JSON instance to validate (inline JSON or file path). Required for 'validate' action."
            },
            "instance_file": {
                "type": "string",
                "description": "Path to a JSON instance file to validate."
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    // Load schema
    let schema_val = load_json_arg(args, "schema", "schema_file")?;

    match action {
        "validate" => {
            let instance_val = load_json_arg(args, "instance", "instance_file").map_err(|_| {
                "validate action requires 'instance' or 'instance_file'".to_string()
            })?;
            action_validate(&schema_val, &instance_val)
        }
        "info" => action_info(&schema_val),
        "properties" => action_properties(&schema_val),
        "refs" => action_refs(&schema_val),
        _ => Err(format!(
            "Unknown action '{}'. Use: validate, info, properties, refs",
            action
        )),
    }
}

fn load_json_arg(args: &Value, inline_key: &str, file_key: &str) -> Result<Value, String> {
    // Try inline first
    if let Some(raw) = args.get(inline_key).and_then(|v| v.as_str()) {
        // Could be a file path or inline JSON
        if raw.trim_start().starts_with('{') || raw.trim_start().starts_with('[') {
            return serde_json::from_str(raw)
                .map_err(|e| format!("Invalid JSON in '{}': {}", inline_key, e));
        }
        // Treat as file path
        let content =
            std::fs::read_to_string(raw).map_err(|e| format!("Cannot read '{}': {}", raw, e))?;
        return serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON in file '{}': {}", raw, e));
    }
    // Try file key
    if let Some(path) = args.get(file_key).and_then(|v| v.as_str()) {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Cannot read '{}': {}", path, e))?;
        return serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON in file '{}': {}", path, e));
    }
    Err(format!(
        "Provide '{}' (inline JSON or file path) or '{}'",
        inline_key, file_key
    ))
}

// ---------------------------------------------------------------------------
// action: info
// ---------------------------------------------------------------------------

fn action_info(schema: &Value) -> Result<String, String> {
    let mut out = String::new();

    let schema_id = schema
        .get("$schema")
        .and_then(|v| v.as_str())
        .unwrap_or("(not specified)");
    let id = schema.get("$id").and_then(|v| v.as_str()).unwrap_or("");
    let title = schema
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("(untitled)");
    let description = schema
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let schema_type = type_label(schema);

    out.push_str("JSON Schema Info\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!("$schema:     {}\n", schema_id));
    if !id.is_empty() {
        out.push_str(&format!("$id:         {}\n", id));
    }
    out.push_str(&format!("title:       {}\n", title));
    if !description.is_empty() {
        let desc_short = if description.len() > 120 {
            format!("{}...", &description[..117])
        } else {
            description.to_string()
        };
        out.push_str(&format!("description: {}\n", desc_short));
    }
    out.push_str(&format!("type:        {}\n", schema_type));

    // Required fields
    if let Some(Value::Array(req)) = schema.get("required") {
        let names: Vec<&str> = req.iter().filter_map(|v| v.as_str()).collect();
        if !names.is_empty() {
            out.push_str(&format!(
                "required:    {} field(s): {}\n",
                names.len(),
                names.join(", ")
            ));
        }
    }

    // Properties count
    if let Some(Value::Object(props)) = schema.get("properties") {
        out.push_str(&format!("properties:  {} defined\n", props.len()));
    }

    // additionalProperties
    if let Some(ap) = schema.get("additionalProperties") {
        match ap {
            Value::Bool(b) => {
                out.push_str(&format!("additionalProperties: {}\n", b));
            }
            Value::Object(_) => {
                out.push_str("additionalProperties: (schema)\n");
            }
            _ => {}
        }
    }

    // enum values
    if let Some(Value::Array(enums)) = schema.get("enum") {
        let labels: Vec<String> = enums.iter().map(|v| v.to_string()).collect();
        out.push_str(&format!(
            "enum:        {} value(s): {}\n",
            labels.len(),
            labels.join(", ")
        ));
    }

    // $defs / definitions count
    for key in &["$defs", "definitions"] {
        if let Some(Value::Object(defs)) = schema.get(*key) {
            out.push_str(&format!("{}: {} definition(s)\n", key, defs.len()));
        }
    }

    // Numeric constraints
    for field in &[
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
    ] {
        if let Some(v) = schema.get(*field) {
            out.push_str(&format!("{}: {}\n", field, v));
        }
    }

    // String constraints
    for field in &["minLength", "maxLength", "pattern", "format"] {
        if let Some(v) = schema.get(*field) {
            out.push_str(&format!("{}: {}\n", field, v));
        }
    }

    // Array constraints
    for field in &["minItems", "maxItems", "uniqueItems"] {
        if let Some(v) = schema.get(*field) {
            out.push_str(&format!("{}: {}\n", field, v));
        }
    }

    // Combiners
    for kw in &["allOf", "anyOf", "oneOf"] {
        if let Some(Value::Array(arr)) = schema.get(*kw) {
            out.push_str(&format!("{}: {} branch(es)\n", kw, arr.len()));
        }
    }
    if schema.get("not").is_some() {
        out.push_str("not: (present)\n");
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// action: properties
// ---------------------------------------------------------------------------

fn action_properties(schema: &Value) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("Properties\n");
    out.push_str(&"─".repeat(70));
    out.push('\n');
    out.push_str(&format!(
        "{:<30} {:<20} {:<8} {}\n",
        "Name", "Type", "Req?", "Description"
    ));
    out.push_str(&"─".repeat(70));
    out.push('\n');

    let required_set: std::collections::HashSet<&str> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut count = 0usize;

    if let Some(Value::Object(props)) = schema.get("properties") {
        for (name, prop_schema) in props {
            let typ = type_label(prop_schema);
            let req = if required_set.contains(name.as_str()) {
                "yes"
            } else {
                "no"
            };
            let desc = prop_schema
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let desc_short = if desc.len() > 40 {
                format!("{}...", &desc[..37])
            } else {
                desc.to_string()
            };
            out.push_str(&format!(
                "{:<30} {:<20} {:<8} {}\n",
                truncate(name, 29),
                truncate(&typ, 19),
                req,
                desc_short
            ));
            count += 1;
        }
    }

    if count == 0 {
        out.push_str("(no properties defined at root level)\n");
        // Check if this is an array schema with items
        if let Some(items) = schema.get("items") {
            out.push_str("\nNote: this is an array schema. Items schema:\n");
            out.push_str(&format!("  type: {}\n", type_label(items)));
        }
    } else {
        out.push_str(&format!("\nTotal: {} properties\n", count));
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// action: refs
// ---------------------------------------------------------------------------

fn action_refs(schema: &Value) -> Result<String, String> {
    let mut refs: Vec<String> = Vec::new();
    let mut defs: Vec<String> = Vec::new();
    let mut ids: Vec<String> = Vec::new();

    collect_refs(schema, &mut refs, &mut defs, &mut ids);

    refs.sort();
    refs.dedup();
    defs.sort();
    ids.sort();
    ids.dedup();

    let mut out = String::new();
    out.push_str("Schema References\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');

    if !ids.is_empty() {
        out.push_str("\n$id anchors:\n");
        for id in &ids {
            out.push_str(&format!("  {}\n", id));
        }
    }

    if !defs.is_empty() {
        out.push_str("\n$defs / definitions:\n");
        for d in &defs {
            out.push_str(&format!("  {}\n", d));
        }
    }

    if !refs.is_empty() {
        out.push_str("\n$ref usages:\n");
        for r in &refs {
            out.push_str(&format!("  {}\n", r));
        }
    }

    if ids.is_empty() && defs.is_empty() && refs.is_empty() {
        out.push_str("(no $ref, $id, $defs, or definitions found)\n");
    } else {
        out.push_str(&format!(
            "\nSummary: {} $ref(s), {} def(s), {} $id(s)\n",
            refs.len(),
            defs.len(),
            ids.len()
        ));
    }

    Ok(out)
}

fn collect_refs(
    val: &Value,
    refs: &mut Vec<String>,
    defs: &mut Vec<String>,
    ids: &mut Vec<String>,
) {
    match val {
        Value::Object(map) => {
            if let Some(Value::String(r)) = map.get("$ref") {
                refs.push(r.clone());
            }
            if let Some(Value::String(id)) = map.get("$id") {
                if !id.is_empty() {
                    ids.push(id.clone());
                }
            }
            for key in &["$defs", "definitions"] {
                if let Some(Value::Object(d)) = map.get(*key) {
                    for k in d.keys() {
                        defs.push(format!("{}#{}", key, k));
                    }
                    for v in d.values() {
                        collect_refs(v, refs, defs, ids);
                    }
                }
            }
            for (k, v) in map {
                if k != "$defs" && k != "definitions" {
                    collect_refs(v, refs, defs, ids);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_refs(v, refs, defs, ids);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// action: validate
// ---------------------------------------------------------------------------

fn action_validate(schema: &Value, instance: &Value) -> Result<String, String> {
    let mut errors: Vec<String> = Vec::new();
    validate_value(schema, instance, "$", schema, &mut errors);

    let mut out = String::new();
    if errors.is_empty() {
        out.push_str("Validation: VALID\n");
        out.push_str("─────────────────────────────\n");
        out.push_str("The instance conforms to the schema.\n");
    } else {
        out.push_str("Validation: INVALID\n");
        out.push_str("─────────────────────────────\n");
        out.push_str(&format!("{} error(s) found:\n\n", errors.len()));
        for (i, e) in errors.iter().enumerate() {
            out.push_str(&format!("  {}. {}\n", i + 1, e));
        }
    }
    Ok(out)
}

fn validate_value(
    schema: &Value,
    instance: &Value,
    path: &str,
    root_schema: &Value,
    errors: &mut Vec<String>,
) {
    // $ref resolution (simple #/$defs/Name and #/definitions/Name)
    if let Some(Value::String(ref_str)) = schema.get("$ref") {
        if let Some(resolved) = resolve_ref(ref_str, root_schema) {
            validate_value(resolved, instance, path, root_schema, errors);
            return;
        }
        // Unresolvable $ref — skip validation for that branch
        return;
    }

    // const
    if let Some(const_val) = schema.get("const") {
        if instance != const_val {
            errors.push(format!(
                "{}: expected const value {}, got {}",
                path,
                const_val,
                type_name(instance)
            ));
        }
        return;
    }

    // enum
    if let Some(Value::Array(enum_vals)) = schema.get("enum") {
        if !enum_vals.contains(instance) {
            let labels: Vec<String> = enum_vals.iter().map(|v| v.to_string()).collect();
            errors.push(format!(
                "{}: value must be one of [{}]",
                path,
                labels.join(", ")
            ));
        }
        return;
    }

    // type check
    if let Some(type_val) = schema.get("type") {
        let ok = match type_val {
            Value::String(t) => check_type(instance, t.as_str()),
            Value::Array(types) => types
                .iter()
                .any(|t| t.as_str().map(|s| check_type(instance, s)).unwrap_or(false)),
            _ => true,
        };
        if !ok {
            let expected = match type_val {
                Value::String(t) => t.clone(),
                Value::Array(types) => types
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(" | "),
                _ => "unknown".to_string(),
            };
            errors.push(format!(
                "{}: type mismatch — expected {}, got {}",
                path,
                expected,
                type_name(instance)
            ));
            return; // further checks would be meaningless
        }
    }

    // Object validation
    if let Value::Object(obj) = instance {
        // required
        if let Some(Value::Array(req)) = schema.get("required") {
            for r in req {
                if let Some(field) = r.as_str() {
                    if !obj.contains_key(field) {
                        errors.push(format!("{}: missing required field '{}'", path, field));
                    }
                }
            }
        }
        // properties
        if let Some(Value::Object(props)) = schema.get("properties") {
            for (key, prop_schema) in props {
                if let Some(val) = obj.get(key) {
                    validate_value(
                        prop_schema,
                        val,
                        &format!("{}.{}", path, key),
                        root_schema,
                        errors,
                    );
                }
            }
        }
        // additionalProperties: false
        if let Some(Value::Bool(false)) = schema.get("additionalProperties") {
            let allowed: std::collections::HashSet<&str> = schema
                .get("properties")
                .and_then(|v| v.as_object())
                .map(|p| p.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();
            for key in obj.keys() {
                if !allowed.contains(key.as_str()) {
                    errors.push(format!(
                        "{}: additional property '{}' not allowed",
                        path, key
                    ));
                }
            }
        }
        // minProperties / maxProperties
        if let Some(min) = schema.get("minProperties").and_then(|v| v.as_u64()) {
            if (obj.len() as u64) < min {
                errors.push(format!(
                    "{}: object has {} properties, minimum is {}",
                    path,
                    obj.len(),
                    min
                ));
            }
        }
        if let Some(max) = schema.get("maxProperties").and_then(|v| v.as_u64()) {
            if (obj.len() as u64) > max {
                errors.push(format!(
                    "{}: object has {} properties, maximum is {}",
                    path,
                    obj.len(),
                    max
                ));
            }
        }
    }

    // Array validation
    if let Value::Array(arr) = instance {
        if let Some(items_schema) = schema.get("items") {
            for (i, item) in arr.iter().enumerate() {
                validate_value(
                    items_schema,
                    item,
                    &format!("{}[{}]", path, i),
                    root_schema,
                    errors,
                );
            }
        }
        if let Some(min) = schema.get("minItems").and_then(|v| v.as_u64()) {
            if (arr.len() as u64) < min {
                errors.push(format!(
                    "{}: array has {} item(s), minimum is {}",
                    path,
                    arr.len(),
                    min
                ));
            }
        }
        if let Some(max) = schema.get("maxItems").and_then(|v| v.as_u64()) {
            if (arr.len() as u64) > max {
                errors.push(format!(
                    "{}: array has {} item(s), maximum is {}",
                    path,
                    arr.len(),
                    max
                ));
            }
        }
        if let Some(Value::Bool(true)) = schema.get("uniqueItems") {
            let mut seen: Vec<String> = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                let s = item.to_string();
                if seen.contains(&s) {
                    errors.push(format!(
                        "{}[{}]: duplicate item (uniqueItems required)",
                        path, i
                    ));
                } else {
                    seen.push(s);
                }
            }
        }
    }

    // String validation
    if let Value::String(s) = instance {
        if let Some(min) = schema.get("minLength").and_then(|v| v.as_u64()) {
            if (s.chars().count() as u64) < min {
                errors.push(format!(
                    "{}: string length {} < minLength {}",
                    path,
                    s.chars().count(),
                    min
                ));
            }
        }
        if let Some(max) = schema.get("maxLength").and_then(|v| v.as_u64()) {
            if (s.chars().count() as u64) > max {
                errors.push(format!(
                    "{}: string length {} > maxLength {}",
                    path,
                    s.chars().count(),
                    max
                ));
            }
        }
        if let Some(Value::String(pattern)) = schema.get("pattern") {
            if let Ok(re) = regex::Regex::new(pattern) {
                if !re.is_match(s) {
                    errors.push(format!(
                        "{}: string '{}' does not match pattern '{}'",
                        path, s, pattern
                    ));
                }
            }
        }
    }

    // Number validation
    if let Some(n) = as_number(instance) {
        if let Some(min) = schema.get("minimum").and_then(as_f64_val) {
            if n < min {
                errors.push(format!("{}: {} < minimum {}", path, n, min));
            }
        }
        if let Some(max) = schema.get("maximum").and_then(as_f64_val) {
            if n > max {
                errors.push(format!("{}: {} > maximum {}", path, n, max));
            }
        }
        if let Some(emin) = schema.get("exclusiveMinimum").and_then(as_f64_val) {
            if n <= emin {
                errors.push(format!(
                    "{}: {} is not > exclusiveMinimum {}",
                    path, n, emin
                ));
            }
        }
        if let Some(emax) = schema.get("exclusiveMaximum").and_then(as_f64_val) {
            if n >= emax {
                errors.push(format!(
                    "{}: {} is not < exclusiveMaximum {}",
                    path, n, emax
                ));
            }
        }
        if let Some(mult) = schema.get("multipleOf").and_then(as_f64_val) {
            if mult > 0.0 {
                let remainder = n % mult;
                let tol = 1e-10;
                if remainder.abs() > tol && (mult - remainder).abs() > tol {
                    errors.push(format!("{}: {} is not a multiple of {}", path, n, mult));
                }
            }
        }
    }

    // allOf
    if let Some(Value::Array(all)) = schema.get("allOf") {
        for (i, sub) in all.iter().enumerate() {
            let mut sub_errors = Vec::new();
            validate_value(sub, instance, path, root_schema, &mut sub_errors);
            for e in sub_errors {
                errors.push(format!("allOf[{}]: {}", i, e));
            }
        }
    }

    // anyOf
    if let Some(Value::Array(any)) = schema.get("anyOf") {
        let passed = any.iter().any(|sub| {
            let mut sub_errors = Vec::new();
            validate_value(sub, instance, path, root_schema, &mut sub_errors);
            sub_errors.is_empty()
        });
        if !passed {
            errors.push(format!(
                "{}: value does not match any of the {} anyOf schemas",
                path,
                any.len()
            ));
        }
    }

    // oneOf
    if let Some(Value::Array(one)) = schema.get("oneOf") {
        let count = one
            .iter()
            .filter(|sub| {
                let mut sub_errors = Vec::new();
                validate_value(sub, instance, path, root_schema, &mut sub_errors);
                sub_errors.is_empty()
            })
            .count();
        if count != 1 {
            errors.push(format!(
                "{}: value matches {} of the {} oneOf schemas (exactly 1 required)",
                path,
                count,
                one.len()
            ));
        }
    }

    // not
    if let Some(not_schema) = schema.get("not") {
        let mut sub_errors = Vec::new();
        validate_value(not_schema, instance, path, root_schema, &mut sub_errors);
        if sub_errors.is_empty() {
            errors.push(format!("{}: value must NOT match the 'not' schema", path));
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_ref<'a>(ref_str: &str, root: &'a Value) -> Option<&'a Value> {
    // Support #/$defs/Name and #/definitions/Name
    let path = ref_str.strip_prefix("#/")?;
    let parts: Vec<&str> = path.split('/').collect();
    let mut cur = root;
    for part in parts {
        // JSON Pointer decoding (basic)
        let decoded = part.replace("~1", "/").replace("~0", "~");
        cur = cur.get(&decoded)?;
    }
    Some(cur)
}

fn check_type(instance: &Value, t: &str) -> bool {
    match t {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "number" => instance.is_number(),
        "integer" => instance.is_i64() || instance.is_u64(),
        "boolean" => instance.is_boolean(),
        "null" => instance.is_null(),
        _ => true, // unknown type — pass
    }
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

fn type_label(schema: &Value) -> String {
    if let Some(t) = schema.get("type") {
        match t {
            Value::String(s) => s.clone(),
            Value::Array(types) => types
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(" | "),
            _ => "unknown".to_string(),
        }
    } else if schema.get("$ref").is_some() {
        schema
            .get("$ref")
            .and_then(|v| v.as_str())
            .unwrap_or("$ref")
            .to_string()
    } else if schema.get("allOf").is_some() {
        "allOf".to_string()
    } else if schema.get("anyOf").is_some() {
        "anyOf".to_string()
    } else if schema.get("oneOf").is_some() {
        "oneOf".to_string()
    } else if schema.get("enum").is_some() {
        "enum".to_string()
    } else if schema.get("const").is_some() {
        "const".to_string()
    } else {
        "any".to_string()
    }
}

fn as_number(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn as_f64_val(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
