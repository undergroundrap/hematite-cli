use serde_json::Value;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "prettier_tools",
        "description": "Parse, inspect, validate, and explain Prettier configuration files (.prettierrc, .prettierrc.json, .prettierrc.yaml) without external utilities. Auto-detects JSON vs YAML format.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "validate", "explain", "overrides"],
                    "description": "info: all options with defaults (default); validate: deprecated/invalid options; explain: plain-English per-option; overrides: file-pattern override blocks"
                },
                "config": {
                    "type": "string",
                    "description": "Inline Prettier config as JSON or YAML string"
                },
                "file": {
                    "type": "string",
                    "description": "Path to .prettierrc, .prettierrc.json, .prettierrc.yaml, or .prettierrc.yml"
                }
            }
        }
    })
}

fn load_config(args: &Value) -> Result<Value, String> {
    let text = if let Some(c) = args.get("config").and_then(|v| v.as_str()) {
        c.to_string()
    } else if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(f).map_err(|e| format!("Cannot read '{}': {}", f, e))?
    } else {
        return Err("Provide 'config' (inline) or 'file' (path to .prettierrc).".to_string());
    };

    let trimmed = text.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        serde_json::from_str(trimmed).map_err(|e| format!("JSON parse error: {}", e))
    } else {
        serde_yaml::from_str(trimmed).map_err(|e| format!("YAML parse error: {}", e))
    }
}

struct OptionDef {
    name: &'static str,
    default: &'static str,
    description: &'static str,
    deprecated: bool,
}

fn option_defs() -> Vec<OptionDef> {
    vec![
        OptionDef {
            name: "printWidth",
            default: "80",
            description: "Line length before Prettier wraps",
            deprecated: false,
        },
        OptionDef {
            name: "tabWidth",
            default: "2",
            description: "Number of spaces per indent level",
            deprecated: false,
        },
        OptionDef {
            name: "useTabs",
            default: "false",
            description: "Indent with tabs instead of spaces",
            deprecated: false,
        },
        OptionDef {
            name: "semi",
            default: "true",
            description: "Add semicolons at end of statements",
            deprecated: false,
        },
        OptionDef {
            name: "singleQuote",
            default: "false",
            description: "Use single quotes instead of double quotes",
            deprecated: false,
        },
        OptionDef {
            name: "quoteProps",
            default: "\"as-needed\"",
            description: "Object property quoting: as-needed/consistent/preserve",
            deprecated: false,
        },
        OptionDef {
            name: "jsxSingleQuote",
            default: "false",
            description: "Use single quotes in JSX attributes",
            deprecated: false,
        },
        OptionDef {
            name: "trailingComma",
            default: "\"all\"",
            description: "Trailing commas: none/es5/all",
            deprecated: false,
        },
        OptionDef {
            name: "bracketSpacing",
            default: "true",
            description: "Spaces inside object braces: { foo: bar }",
            deprecated: false,
        },
        OptionDef {
            name: "bracketSameLine",
            default: "false",
            description: "Put JSX closing > on same line as last prop",
            deprecated: false,
        },
        OptionDef {
            name: "arrowParens",
            default: "\"always\"",
            description: "Arrow function parens: always/avoid",
            deprecated: false,
        },
        OptionDef {
            name: "proseWrap",
            default: "\"preserve\"",
            description: "Markdown prose wrapping: always/never/preserve",
            deprecated: false,
        },
        OptionDef {
            name: "htmlWhitespaceSensitivity",
            default: "\"css\"",
            description: "HTML whitespace sensitivity: css/strict/ignore",
            deprecated: false,
        },
        OptionDef {
            name: "vueIndentScriptAndStyle",
            default: "false",
            description: "Indent <script> and <style> tags in .vue files",
            deprecated: false,
        },
        OptionDef {
            name: "endOfLine",
            default: "\"lf\"",
            description: "Line endings: lf/crlf/cr/auto",
            deprecated: false,
        },
        OptionDef {
            name: "embeddedLanguageFormatting",
            default: "\"auto\"",
            description: "Format embedded code blocks: auto/off",
            deprecated: false,
        },
        OptionDef {
            name: "singleAttributePerLine",
            default: "false",
            description: "One HTML/JSX attribute per line",
            deprecated: false,
        },
        OptionDef {
            name: "requirePragma",
            default: "false",
            description: "Only format files with @prettier pragma comment",
            deprecated: false,
        },
        OptionDef {
            name: "insertPragma",
            default: "false",
            description: "Insert @prettier pragma at top of formatted files",
            deprecated: false,
        },
        OptionDef {
            name: "parser",
            default: "(auto-detect)",
            description: "Force a specific parser (e.g. babel, typescript, css)",
            deprecated: false,
        },
        OptionDef {
            name: "filepath",
            default: "(none)",
            description: "Infer parser from this file path",
            deprecated: false,
        },
        OptionDef {
            name: "rangeStart",
            default: "0",
            description: "Format only from this character offset",
            deprecated: false,
        },
        OptionDef {
            name: "rangeEnd",
            default: "Infinity",
            description: "Format only up to this character offset",
            deprecated: false,
        },
        OptionDef {
            name: "jsxBracketSameLine",
            default: "false",
            description: "DEPRECATED: use bracketSameLine instead",
            deprecated: true,
        },
        OptionDef {
            name: "experimentalTernaries",
            default: "false",
            description: "DEPRECATED: experimental ternary formatting",
            deprecated: true,
        },
    ]
}

fn compact_val(v: &Value) -> String {
    match v {
        Value::String(s) => format!("\"{}\"", s),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => format!("[{} items]", a.len()),
        Value::Object(o) => format!("{{{}  keys}}", o.len()),
    }
}

fn action_info(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Prettier config must be a JSON/YAML object.".to_string(),
    };

    let defs = option_defs();
    let mut out = String::from("Prettier Configuration\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    let override_count = obj
        .get("overrides")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let configured: Vec<_> = obj
        .keys()
        .filter(|k| k.as_str() != "overrides" && defs.iter().any(|d| d.name == k.as_str()))
        .collect();
    let unknown: Vec<_> = obj
        .keys()
        .filter(|k| k.as_str() != "overrides" && !defs.iter().any(|d| d.name == k.as_str()))
        .collect();

    out.push_str(&format!("Configured options: {}\n", configured.len()));
    out.push_str(&format!("Override blocks:    {}\n", override_count));
    if !unknown.is_empty() {
        out.push_str(&format!("Unknown keys:       {}\n", unknown.len()));
    }
    out.push('\n');

    out.push_str("Configured Options\n");
    out.push_str(&"─".repeat(52));
    out.push('\n');
    let mut any = false;
    for def in &defs {
        if let Some(val) = obj.get(def.name) {
            let dep = if def.deprecated { " [DEPRECATED]" } else { "" };
            out.push_str(&format!("  {:<32} {}{}\n", def.name, compact_val(val), dep));
            any = true;
        }
    }
    if !any {
        out.push_str("  (all defaults)\n");
    }

    if !unknown.is_empty() {
        out.push('\n');
        out.push_str("Unknown Options\n");
        out.push_str(&"─".repeat(52));
        out.push('\n');
        for k in unknown {
            if let Some(v) = obj.get(k) {
                out.push_str(&format!("  {:<32} {}\n", k, compact_val(v)));
            }
        }
    }

    out.push('\n');
    out.push_str("Defaults (not configured)\n");
    out.push_str(&"─".repeat(52));
    out.push('\n');
    for def in &defs {
        if !obj.contains_key(def.name) && !def.deprecated {
            out.push_str(&format!("  {:<32} {} (default)\n", def.name, def.default));
        }
    }

    out
}

fn action_validate(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Prettier config must be a JSON/YAML object.".to_string(),
    };

    let defs = option_defs();
    let mut issues: Vec<String> = Vec::new();

    for def in defs.iter().filter(|d| d.deprecated) {
        if obj.contains_key(def.name) {
            issues.push(format!(
                "[WARN]  Deprecated option '{}': {}",
                def.name, def.description
            ));
        }
    }

    if let (Some(rs), Some(re)) = (obj.get("rangeStart"), obj.get("rangeEnd")) {
        if let (Some(start), Some(end)) = (rs.as_u64(), re.as_u64()) {
            if start > end {
                issues.push("[ERROR] rangeStart is greater than rangeEnd".to_string());
            }
        }
    }

    if obj.get("insertPragma").and_then(|v| v.as_bool()) == Some(true)
        && obj.get("requirePragma").and_then(|v| v.as_bool()) == Some(true)
    {
        issues.push("[WARN]  Both insertPragma and requirePragma are true — redundant, files already have the pragma".to_string());
    }

    if let Some(eol) = obj.get("endOfLine").and_then(|v| v.as_str()) {
        if !["lf", "crlf", "cr", "auto"].contains(&eol) {
            issues.push(format!(
                "[ERROR] endOfLine: \"{}\" is invalid — use lf, crlf, cr, or auto",
                eol
            ));
        }
        if eol == "crlf" {
            issues.push(
                "[INFO]  endOfLine: \"crlf\" — consider \"lf\" for cross-platform compatibility"
                    .to_string(),
            );
        }
    }

    if let Some(tc) = obj.get("trailingComma").and_then(|v| v.as_str()) {
        if !["none", "es5", "all"].contains(&tc) {
            issues.push(format!(
                "[ERROR] trailingComma: \"{}\" is invalid — use none, es5, or all",
                tc
            ));
        }
    }

    if let Some(qp) = obj.get("quoteProps").and_then(|v| v.as_str()) {
        if !["as-needed", "consistent", "preserve"].contains(&qp) {
            issues.push(format!(
                "[ERROR] quoteProps: \"{}\" is invalid — use as-needed, consistent, or preserve",
                qp
            ));
        }
    }

    if let Some(ap) = obj.get("arrowParens").and_then(|v| v.as_str()) {
        if !["always", "avoid"].contains(&ap) {
            issues.push(format!(
                "[ERROR] arrowParens: \"{}\" is invalid — use always or avoid",
                ap
            ));
        }
    }

    if let Some(pw) = obj.get("proseWrap").and_then(|v| v.as_str()) {
        if !["always", "never", "preserve"].contains(&pw) {
            issues.push(format!(
                "[ERROR] proseWrap: \"{}\" is invalid — use always, never, or preserve",
                pw
            ));
        }
    }

    for key in obj.keys() {
        if key == "overrides" {
            continue;
        }
        if !defs.iter().any(|d| d.name == key.as_str()) {
            issues.push(format!(
                "[WARN]  Unknown option '{}' — may be a typo or plugin option not in Prettier core",
                key
            ));
        }
    }

    if let Some(overrides) = obj.get("overrides").and_then(|v| v.as_array()) {
        for (i, ov) in overrides.iter().enumerate() {
            if ov.get("files").is_none() {
                issues.push(format!(
                    "[ERROR] overrides[{}]: missing required 'files' field",
                    i
                ));
            }
            if ov.get("options").is_none() {
                issues.push(format!(
                    "[WARN]  overrides[{}]: no 'options' — override has no effect",
                    i
                ));
            }
        }
    }

    let mut out = String::from("Prettier Config Validation\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    if issues.is_empty() {
        out.push_str("VALID — No issues found.\n");
    } else {
        let errors = issues.iter().filter(|i| i.starts_with("[ERROR]")).count();
        let warns = issues.iter().filter(|i| i.starts_with("[WARN]")).count();
        let infos = issues.iter().filter(|i| i.starts_with("[INFO]")).count();
        out.push_str(if errors > 0 {
            "INVALID\n\n"
        } else {
            "VALID (with warnings)\n\n"
        });
        out.push_str(&format!(
            "Issues: {} error(s), {} warning(s), {} note(s)\n\n",
            errors, warns, infos
        ));
        for issue in &issues {
            out.push_str(&format!("  {}\n", issue));
        }
    }

    out
}

fn action_explain(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Prettier config must be a JSON/YAML object.".to_string(),
    };

    let defs = option_defs();
    let mut out = String::from("Prettier Option Explanations\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    let configured: Vec<_> = defs.iter().filter(|d| obj.contains_key(d.name)).collect();

    if configured.is_empty() {
        out.push_str("No options configured — all defaults apply.\n\n");
        out.push_str("Common options:\n");
        for def in defs.iter().take(10) {
            out.push_str(&format!(
                "  {:<28} (default: {}) — {}\n",
                def.name, def.default, def.description
            ));
        }
        return out;
    }

    for def in configured {
        let val = &obj[def.name];
        out.push_str(&format!("\n{}", def.name));
        if def.deprecated {
            out.push_str(" [DEPRECATED]");
        }
        out.push('\n');
        out.push_str(&format!("  Set to:  {}\n", compact_val(val)));
        out.push_str(&format!("  Default: {}\n", def.default));
        out.push_str(&format!("  Effect:  {}\n", def.description));
    }

    out
}

fn action_overrides(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Prettier config must be a JSON/YAML object.".to_string(),
    };

    let mut out = String::from("Prettier File Overrides\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    let overrides = match obj.get("overrides").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            out.push_str("No overrides configured.\n");
            return out;
        }
    };

    out.push_str(&format!("{} override block(s)\n\n", overrides.len()));

    for (i, ov) in overrides.iter().enumerate() {
        let files = match ov.get("files") {
            Some(Value::String(s)) => format!("\"{}\"", s),
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", "),
            _ => "(missing)".to_string(),
        };
        out.push_str(&format!("Override #{} — files: {}\n", i + 1, files));
        if let Some(opts) = ov.get("options").and_then(|v| v.as_object()) {
            for (k, v) in opts {
                out.push_str(&format!("  {:<28} {}\n", k, compact_val(v)));
            }
        } else {
            out.push_str("  (no options)\n");
        }
        out.push('\n');
    }

    out
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let cfg = load_config(args)?;
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => Ok(action_info(&cfg)),
        "validate" => Ok(action_validate(&cfg)),
        "explain" => Ok(action_explain(&cfg)),
        "overrides" => Ok(action_overrides(&cfg)),
        other => Err(format!(
            "Unknown action '{}'. Valid: info, validate, explain, overrides.",
            other
        )),
    }
}
