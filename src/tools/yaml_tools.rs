use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("validate");

    match action {
        "validate" => validate(args),
        "format" => format_yaml(args),
        "get" => get_path(args),
        "keys" => list_keys(args),
        "to-json" => to_json(args),
        "from-json" => from_json(args),
        "merge" => merge(args),
        "diff" => diff(args),
        other => Err(format!(
            "yaml_tools: unknown action '{other}'. Valid: validate, format, get, keys, to-json, from-json, merge, diff"
        )),
    }
}

fn resolve_input(args: &serde_json::Value) -> Result<String, String> {
    if let Some(inline) = args.get("yaml").and_then(|v| v.as_str()) {
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
            .map_err(|e| format!("yaml_tools: cannot read '{file}': {e}"));
    }
    Err("yaml_tools: provide 'yaml' (inline string) or 'file' (path)".into())
}

fn parse(src: &str) -> Result<YamlValue, String> {
    serde_yaml::from_str(src).map_err(|e| format!("yaml_tools: invalid YAML: {e}"))
}

fn validate(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let val = parse(&src)?;

    let kind = yaml_kind(&val);
    let depth = yaml_depth(&val);
    let key_count = if let YamlValue::Mapping(m) = &val {
        m.len()
    } else {
        0
    };
    let item_count = if let YamlValue::Sequence(s) = &val {
        s.len()
    } else {
        0
    };

    let mut out = format!("YAML VALID\n{}\n", "─".repeat(50));
    out.push_str(&format!("Root type : {kind}\n"));
    out.push_str(&format!("Depth     : {depth}\n"));
    if key_count > 0 {
        out.push_str(&format!("Top-level keys: {key_count}\n"));
        if let YamlValue::Mapping(m) = &val {
            for (k, _) in m.iter().take(10) {
                out.push_str(&format!("  - {}\n", yaml_key_str(k)));
            }
            if key_count > 10 {
                out.push_str(&format!("  ... and {} more\n", key_count - 10));
            }
        }
    }
    if item_count > 0 {
        out.push_str(&format!("Array length: {item_count}\n"));
    }
    Ok(out)
}

fn format_yaml(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let val = parse(&src)?;
    let formatted =
        serde_yaml::to_string(&val).map_err(|e| format!("yaml_tools: serialize error: {e}"))?;
    Ok(format!("YAML FORMAT\n{}\n{formatted}", "─".repeat(50)))
}

fn get_path(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let path = args.get("path").and_then(|v| v.as_str()).ok_or(
        "yaml_tools get: 'path' is required (e.g. 'metadata.name' or 'spec.containers[0].image')",
    )?;

    let val = parse(&src)?;
    let result =
        navigate_yaml(&val, path).ok_or_else(|| format!("yaml_tools: path '{path}' not found"))?;

    let repr = serde_yaml::to_string(result).map_err(|e| format!("yaml_tools: serialize: {e}"))?;
    Ok(format!("YAML GET: {path}\n{}\n{repr}", "─".repeat(50)))
}

fn navigate_yaml<'a>(val: &'a YamlValue, path: &str) -> Option<&'a YamlValue> {
    let mut cur = val;
    for part in path.split('.') {
        // Handle array index like "containers[0]"
        if let Some(bracket) = part.find('[') {
            let key = &part[..bracket];
            let idx_str = part[bracket + 1..].trim_end_matches(']');
            let idx: usize = idx_str.parse().ok()?;
            if !key.is_empty() {
                cur = cur.get(key)?;
            }
            cur = cur.get(idx)?;
        } else {
            cur = cur.get(part)?;
        }
    }
    Some(cur)
}

fn list_keys(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let path = args.get("path").and_then(|v| v.as_str());
    let val = parse(&src)?;

    let target = if let Some(p) = path {
        navigate_yaml(&val, p)
            .ok_or_else(|| format!("yaml_tools: path '{p}' not found"))?
            .clone()
    } else {
        val
    };

    let mut out = format!(
        "YAML KEYS{}\n{}\n",
        path.map(|p| format!(": {p}")).unwrap_or_default(),
        "─".repeat(50)
    );

    match &target {
        YamlValue::Mapping(m) => {
            for (k, v) in m {
                let kind = yaml_kind(v);
                out.push_str(&format!("  {} ({kind})\n", yaml_key_str(k)));
            }
            out.push_str(&format!("\nTotal: {} key(s)\n", m.len()));
        }
        YamlValue::Sequence(s) => {
            out.push_str(&format!("Array with {} element(s):\n", s.len()));
            for (i, v) in s.iter().enumerate().take(20) {
                out.push_str(&format!("  [{}] {}\n", i, yaml_kind(v)));
            }
            if s.len() > 20 {
                out.push_str(&format!("  ... and {} more\n", s.len() - 20));
            }
        }
        other => {
            out.push_str(&format!("Scalar: {}\n", yaml_kind(other)));
        }
    }
    Ok(out)
}

fn to_json(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let yaml_val: YamlValue = parse(&src)?;
    let json_val: JsonValue = serde_json::to_value(&yaml_val)
        .map_err(|e| format!("yaml_tools to-json: conversion error: {e}"))?;
    let pretty = serde_json::to_string_pretty(&json_val)
        .map_err(|e| format!("yaml_tools to-json: serialize error: {e}"))?;
    Ok(format!("YAML → JSON\n{}\n{pretty}", "─".repeat(50)))
}

fn from_json(args: &serde_json::Value) -> Result<String, String> {
    let json_src = args
        .get("json")
        .and_then(|v| v.as_str())
        .ok_or("yaml_tools from-json: provide 'json' arg with inline JSON string")?;

    let json_val: JsonValue = serde_json::from_str(json_src)
        .map_err(|e| format!("yaml_tools from-json: invalid JSON: {e}"))?;
    let yaml_val: YamlValue = serde_json::from_value(json_val)
        .map_err(|e| format!("yaml_tools from-json: conversion error: {e}"))?;
    let yaml_str = serde_yaml::to_string(&yaml_val)
        .map_err(|e| format!("yaml_tools from-json: serialize error: {e}"))?;

    Ok(format!("JSON → YAML\n{}\n{yaml_str}", "─".repeat(50)))
}

fn merge(args: &serde_json::Value) -> Result<String, String> {
    let base_src = resolve_input(args)?;
    let with_src = args
        .get("with")
        .and_then(|v| v.as_str())
        .ok_or("yaml_tools merge: provide 'with' arg with the YAML to merge in")?;

    let mut base: YamlValue = parse(&base_src)?;
    let overlay: YamlValue = serde_yaml::from_str(with_src)
        .map_err(|e| format!("yaml_tools merge: invalid 'with' YAML: {e}"))?;

    merge_yaml(&mut base, overlay);

    let result =
        serde_yaml::to_string(&base).map_err(|e| format!("yaml_tools merge: serialize: {e}"))?;
    Ok(format!("YAML MERGE\n{}\n{result}", "─".repeat(50)))
}

fn merge_yaml(base: &mut YamlValue, overlay: YamlValue) {
    match (base, overlay) {
        (YamlValue::Mapping(b), YamlValue::Mapping(o)) => {
            for (k, v) in o {
                let entry = b.entry(k).or_insert(YamlValue::Null);
                merge_yaml(entry, v);
            }
        }
        (base, overlay) => *base = overlay,
    }
}

fn diff(args: &serde_json::Value) -> Result<String, String> {
    let a_src = resolve_input(args)?;
    let b_src = args
        .get("with")
        .and_then(|v| v.as_str())
        .ok_or("yaml_tools diff: provide 'with' arg with the second YAML document")?;

    let a: YamlValue = parse(&a_src)?;
    let b: YamlValue = serde_yaml::from_str(b_src)
        .map_err(|e| format!("yaml_tools diff: invalid 'with' YAML: {e}"))?;

    let mut out = format!("YAML DIFF\n{}\n", "─".repeat(50));
    let mut changes = Vec::new();
    diff_yaml(&a, &b, "", &mut changes);

    if changes.is_empty() {
        out.push_str("Documents are identical — no differences found.\n");
    } else {
        for line in &changes {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str(&format!("\n{} difference(s)\n", changes.len()));
    }
    Ok(out)
}

fn diff_yaml(a: &YamlValue, b: &YamlValue, path: &str, changes: &mut Vec<String>) {
    match (a, b) {
        (YamlValue::Mapping(ma), YamlValue::Mapping(mb)) => {
            for (k, av) in ma {
                let key = yaml_key_str(k);
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match mb.get(k) {
                    Some(bv) => diff_yaml(av, bv, &child_path, changes),
                    None => changes.push(format!("  - {child_path}: (removed)")),
                }
            }
            for (k, bv) in mb {
                if !ma.contains_key(k) {
                    let key = yaml_key_str(k);
                    let child_path = if path.is_empty() {
                        key
                    } else {
                        format!("{path}.{key}")
                    };
                    let repr = scalar_repr(bv);
                    changes.push(format!("  + {child_path}: {repr}"));
                }
            }
        }
        (av, bv) if av != bv => {
            changes.push(format!(
                "  ~ {path}: {} → {}",
                scalar_repr(av),
                scalar_repr(bv)
            ));
        }
        _ => {}
    }
}

fn yaml_kind(v: &YamlValue) -> &'static str {
    match v {
        YamlValue::Null => "null",
        YamlValue::Bool(_) => "bool",
        YamlValue::Number(_) => "number",
        YamlValue::String(_) => "string",
        YamlValue::Sequence(_) => "array",
        YamlValue::Mapping(_) => "object",
        YamlValue::Tagged(_) => "tagged",
    }
}

fn yaml_depth(v: &YamlValue) -> usize {
    match v {
        YamlValue::Mapping(m) => 1 + m.values().map(yaml_depth).max().unwrap_or(0),
        YamlValue::Sequence(s) => 1 + s.iter().map(yaml_depth).max().unwrap_or(0),
        _ => 0,
    }
}

fn yaml_key_str(k: &YamlValue) -> String {
    match k {
        YamlValue::String(s) => s.clone(),
        other => format!("{other:?}"),
    }
}

fn scalar_repr(v: &YamlValue) -> String {
    match v {
        YamlValue::Null => "null".into(),
        YamlValue::Bool(b) => b.to_string(),
        YamlValue::Number(n) => n.to_string(),
        YamlValue::String(s) => {
            if s.len() > 60 {
                format!("\"{}...\"", &s[..60])
            } else {
                format!("\"{s}\"")
            }
        }
        YamlValue::Sequence(s) => format!("[{} item(s)]", s.len()),
        YamlValue::Mapping(m) => format!("{{{} key(s)}}", m.len()),
        YamlValue::Tagged(_) => "<tagged>".into(),
    }
}
