use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("validate");

    match action {
        "validate" => validate(args),
        "format" => format_toml(args),
        "get" => get_path(args),
        "keys" => list_keys(args),
        "to-json" => to_json(args),
        "from-json" => from_json(args),
        other => Err(format!(
            "toml_tools: unknown action '{other}'. Valid: validate, format, get, keys, to-json, from-json"
        )),
    }
}

fn resolve_input(args: &serde_json::Value) -> Result<String, String> {
    if let Some(inline) = args.get("toml").and_then(|v| v.as_str()) {
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
            .map_err(|e| format!("toml_tools: cannot read '{file}': {e}"));
    }
    Err("toml_tools: provide 'toml' (inline string) or 'file' (path)".into())
}

fn parse(src: &str) -> Result<TomlValue, String> {
    toml::from_str(src).map_err(|e| format!("toml_tools: invalid TOML: {e}"))
}

fn validate(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let val = parse(&src)?;

    let kind = toml_kind(&val);
    let key_count = if let TomlValue::Table(t) = &val {
        t.len()
    } else {
        0
    };
    let item_count = if let TomlValue::Array(a) = &val {
        a.len()
    } else {
        0
    };

    let mut out = format!("TOML VALID\n{}\n", "─".repeat(50));
    out.push_str(&format!("Root type : {kind}\n"));
    if key_count > 0 {
        out.push_str(&format!("Top-level keys: {key_count}\n"));
        if let TomlValue::Table(t) = &val {
            for (k, v) in t.iter().take(10) {
                out.push_str(&format!("  - {} ({})\n", k, toml_kind(v)));
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

fn format_toml(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let val = parse(&src)?;
    let formatted =
        toml::to_string_pretty(&val).map_err(|e| format!("toml_tools: serialize error: {e}"))?;
    Ok(format!("TOML FORMAT\n{}\n{formatted}", "─".repeat(50)))
}

fn get_path(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let path = args.get("path").and_then(|v| v.as_str()).ok_or(
        "toml_tools get: 'path' is required (e.g. 'package.name' or 'dependencies.serde')",
    )?;

    let val = parse(&src)?;
    let result =
        navigate_toml(&val, path).ok_or_else(|| format!("toml_tools: path '{path}' not found"))?;

    let repr = toml::to_string(result).unwrap_or_else(|_| format!("{result:?}"));
    Ok(format!("TOML GET: {path}\n{}\n{repr}", "─".repeat(50)))
}

fn navigate_toml<'a>(val: &'a TomlValue, path: &str) -> Option<&'a TomlValue> {
    let mut cur = val;
    for part in path.split('.') {
        if let Some(bracket) = part.find('[') {
            let key = &part[..bracket];
            let idx_str = part[bracket + 1..].trim_end_matches(']');
            let idx: usize = idx_str.parse().ok()?;
            if !key.is_empty() {
                cur = cur.get(key)?;
            }
            cur = cur.as_array()?.get(idx)?;
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
        navigate_toml(&val, p)
            .ok_or_else(|| format!("toml_tools: path '{p}' not found"))?
            .clone()
    } else {
        val
    };

    let mut out = format!(
        "TOML KEYS{}\n{}\n",
        path.map(|p| format!(": {p}")).unwrap_or_default(),
        "─".repeat(50)
    );

    match &target {
        TomlValue::Table(t) => {
            for (k, v) in t {
                out.push_str(&format!("  {} ({})\n", k, toml_kind(v)));
            }
            out.push_str(&format!("\nTotal: {} key(s)\n", t.len()));
        }
        TomlValue::Array(a) => {
            out.push_str(&format!("Array with {} element(s):\n", a.len()));
            for (i, v) in a.iter().enumerate().take(20) {
                out.push_str(&format!("  [{}] {}\n", i, toml_kind(v)));
            }
            if a.len() > 20 {
                out.push_str(&format!("  ... and {} more\n", a.len() - 20));
            }
        }
        other => {
            out.push_str(&format!("Scalar: {}\n", toml_kind(other)));
        }
    }
    Ok(out)
}

fn to_json(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let toml_val: TomlValue = parse(&src)?;
    let json_val: JsonValue = serde_json::to_value(&toml_val)
        .map_err(|e| format!("toml_tools to-json: conversion error: {e}"))?;
    let pretty = serde_json::to_string_pretty(&json_val)
        .map_err(|e| format!("toml_tools to-json: serialize error: {e}"))?;
    Ok(format!("TOML → JSON\n{}\n{pretty}", "─".repeat(50)))
}

fn from_json(args: &serde_json::Value) -> Result<String, String> {
    let json_src = args
        .get("json")
        .and_then(|v| v.as_str())
        .ok_or("toml_tools from-json: provide 'json' arg with inline JSON string")?;

    let json_val: JsonValue = serde_json::from_str(json_src)
        .map_err(|e| format!("toml_tools from-json: invalid JSON: {e}"))?;
    let toml_val: TomlValue = serde_json::from_value(json_val)
        .map_err(|e| format!("toml_tools from-json: conversion error: {e}"))?;
    let toml_str = toml::to_string_pretty(&toml_val)
        .map_err(|e| format!("toml_tools from-json: serialize error: {e}"))?;

    Ok(format!("JSON → TOML\n{}\n{toml_str}", "─".repeat(50)))
}

fn toml_kind(v: &TomlValue) -> &'static str {
    match v {
        TomlValue::String(_) => "string",
        TomlValue::Integer(_) => "integer",
        TomlValue::Float(_) => "float",
        TomlValue::Boolean(_) => "bool",
        TomlValue::Datetime(_) => "datetime",
        TomlValue::Array(_) => "array",
        TomlValue::Table(_) => "table",
    }
}
