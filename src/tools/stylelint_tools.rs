use serde_json::Value;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "stylelint_tools",
        "description": "Parse, inspect, and validate Stylelint configuration files (.stylelintrc, .stylelintrc.json, .stylelintrc.yaml, stylelint.config.js pattern, or 'stylelint' key in package.json) without external utilities. Auto-detects JSON vs YAML.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "rules", "plugins", "extends", "validate"],
                    "description": "info: full config overview (default); rules: detailed rule listing with severity; plugins: plugin list; extends: config inheritance chain; validate: common config issues"
                },
                "config": {
                    "type": "string",
                    "description": "Inline Stylelint config as JSON or YAML string, or package.json JSON containing a 'stylelint' key"
                },
                "file": {
                    "type": "string",
                    "description": "Path to .stylelintrc, .stylelintrc.json, .stylelintrc.yaml, stylelint.config.js, or package.json"
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
        return Err("Provide 'config' (inline) or 'file' (path to .stylelintrc, .stylelintrc.json, .stylelintrc.yaml, or package.json).".to_string());
    };

    let trimmed = text.trim();

    // stylelint.config.js — extract the exported object if possible (basic heuristic)
    // We can't actually run JS, but we can warn the user
    if args
        .get("file")
        .and_then(|v| v.as_str())
        .map(|f| f.ends_with(".js") || f.ends_with(".mjs") || f.ends_with(".cjs"))
        .unwrap_or(false)
    {
        return Err("JavaScript config files (.js/.mjs/.cjs) cannot be parsed without executing Node.js. Convert to .stylelintrc.json or use 'config' with the inline JSON object.".to_string());
    }

    let parsed: Value = if trimmed.starts_with('{') || trimmed.starts_with('[') {
        serde_json::from_str(trimmed).map_err(|e| format!("JSON parse error: {}", e))?
    } else {
        serde_yaml::from_str(trimmed).map_err(|e| format!("YAML parse error: {}", e))?
    };

    // package.json detection
    if parsed.get("name").is_some() && parsed.get("version").is_some() {
        return parsed
            .get("stylelint")
            .cloned()
            .ok_or_else(|| "No 'stylelint' key found in package.json. Add a 'stylelint' config section or use .stylelintrc.json.".to_string());
    }

    Ok(parsed)
}

fn rule_severity(v: &Value) -> &'static str {
    match v {
        Value::Null => "off",
        Value::Bool(false) => "off",
        Value::Bool(true) => "on",
        Value::String(s) => match s.as_str() {
            "error" | "2" => "error",
            "warning" | "warn" | "1" => "warning",
            "off" | "0" => "off",
            _ => "?",
        },
        Value::Number(n) => match n.as_u64() {
            Some(0) => "off",
            Some(1) => "warning",
            Some(2) => "error",
            _ => "?",
        },
        Value::Array(a) => {
            // [severity, options] form
            a.first().map(|first| rule_severity(first)).unwrap_or("?")
        }
        _ => "?",
    }
}

fn action_info(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Stylelint config must be a JSON/YAML object.".to_string(),
    };

    let mut out = String::from("Stylelint Configuration\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    let rule_count = obj
        .get("rules")
        .and_then(|v| v.as_object())
        .map(|o| o.len())
        .unwrap_or(0);
    let plugin_count = obj
        .get("plugins")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let extends_count = match obj.get("extends") {
        Some(Value::String(_)) => 1,
        Some(Value::Array(a)) => a.len(),
        _ => 0,
    };
    let override_count = obj
        .get("overrides")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    out.push_str(&format!("Rules:    {}\n", rule_count));
    out.push_str(&format!("Plugins:  {}\n", plugin_count));
    if extends_count > 0 {
        out.push_str(&format!("Extends:  {}\n", extends_count));
    }
    if override_count > 0 {
        out.push_str(&format!("Overrides: {} block(s)\n", override_count));
    }

    if let Some(ignore) = obj.get("ignoreFiles") {
        match ignore {
            Value::String(s) => out.push_str(&format!("ignoreFiles: {}\n", s)),
            Value::Array(a) => {
                out.push_str(&format!("ignoreFiles: {} pattern(s)\n", a.len()));
            }
            _ => {}
        }
    }

    if let Some(custom_syn) = obj.get("customSyntax") {
        out.push_str(&format!("customSyntax: {}\n", custom_syn));
    }

    // Extends summary
    if extends_count > 0 {
        out.push('\n');
        out.push_str("Extends\n");
        out.push_str(&"─".repeat(52));
        out.push('\n');
        match obj.get("extends") {
            Some(Value::String(s)) => out.push_str(&format!("  {}\n", s)),
            Some(Value::Array(a)) => {
                for e in a {
                    if let Some(s) = e.as_str() {
                        out.push_str(&format!("  {}\n", s));
                    }
                }
            }
            _ => {}
        }
    }

    // Rule severity summary
    if rule_count > 0 {
        let rules_obj = obj.get("rules").and_then(|v| v.as_object()).unwrap();
        let errors = rules_obj
            .values()
            .filter(|v| rule_severity(v) == "error")
            .count();
        let warnings = rules_obj
            .values()
            .filter(|v| rule_severity(v) == "warning")
            .count();
        let off = rules_obj
            .values()
            .filter(|v| rule_severity(v) == "off")
            .count();

        out.push('\n');
        out.push_str("Rules Summary\n");
        out.push_str(&"─".repeat(52));
        out.push('\n');
        out.push_str(&format!("  error:   {}\n", errors));
        out.push_str(&format!("  warning: {}\n", warnings));
        if off > 0 {
            out.push_str(&format!("  off:     {}\n", off));
        }
    }

    out
}

fn action_rules(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Stylelint config must be a JSON/YAML object.".to_string(),
    };

    let mut out = String::from("Stylelint Rules\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    let rules = match obj.get("rules").and_then(|v| v.as_object()) {
        Some(r) if !r.is_empty() => r,
        _ => {
            out.push_str("No rules configured.\n");
            return out;
        }
    };

    // Group by severity
    let mut errors: Vec<(&String, &Value)> = Vec::new();
    let mut warnings: Vec<(&String, &Value)> = Vec::new();
    let mut offs: Vec<(&String, &Value)> = Vec::new();
    let mut unknowns: Vec<(&String, &Value)> = Vec::new();

    for (name, val) in rules {
        match rule_severity(val) {
            "error" => errors.push((name, val)),
            "warning" => warnings.push((name, val)),
            "off" => offs.push((name, val)),
            _ => unknowns.push((name, val)),
        }
    }

    if !errors.is_empty() {
        out.push_str(&format!("\nErrors ({})\n", errors.len()));
        out.push_str(&"─".repeat(52));
        out.push('\n');
        for (name, val) in &errors {
            let opts = rule_options_str(val);
            if opts.is_empty() {
                out.push_str(&format!("  {}\n", name));
            } else {
                out.push_str(&format!("  {}  {}\n", name, opts));
            }
        }
    }

    if !warnings.is_empty() {
        out.push_str(&format!("\nWarnings ({})\n", warnings.len()));
        out.push_str(&"─".repeat(52));
        out.push('\n');
        for (name, val) in &warnings {
            let opts = rule_options_str(val);
            if opts.is_empty() {
                out.push_str(&format!("  {}\n", name));
            } else {
                out.push_str(&format!("  {}  {}\n", name, opts));
            }
        }
    }

    if !offs.is_empty() {
        out.push_str(&format!("\nDisabled ({})\n", offs.len()));
        out.push_str(&"─".repeat(52));
        out.push('\n');
        for (name, _) in &offs {
            out.push_str(&format!("  {}\n", name));
        }
    }

    if !unknowns.is_empty() {
        out.push_str(&format!("\nUnknown severity ({})\n", unknowns.len()));
        out.push_str(&"─".repeat(52));
        out.push('\n');
        for (name, val) in &unknowns {
            out.push_str(&format!("  {}  {}\n", name, val));
        }
    }

    out
}

fn rule_options_str(v: &Value) -> String {
    // For [severity, options] form, show the options
    if let Value::Array(a) = v {
        if a.len() > 1 {
            let opts: Vec<String> = a[1..].iter().map(|x| x.to_string()).collect();
            return format!("({})", opts.join(", "));
        }
    }
    String::new()
}

fn action_plugins(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Stylelint config must be a JSON/YAML object.".to_string(),
    };

    let mut out = String::from("Stylelint Plugins\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    let plugins = match obj.get("plugins").and_then(|v| v.as_array()) {
        Some(a) if !a.is_empty() => a,
        _ => {
            out.push_str("No plugins configured.\n");
            return out;
        }
    };

    for (i, plugin) in plugins.iter().enumerate() {
        if let Some(s) = plugin.as_str() {
            out.push_str(&format!("  {}. {}\n", i + 1, s));
        } else {
            out.push_str(&format!("  {}. {}\n", i + 1, plugin));
        }
    }

    out
}

fn action_extends(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Stylelint config must be a JSON/YAML object.".to_string(),
    };

    let mut out = String::from("Stylelint Config Inheritance\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    match obj.get("extends") {
        None => {
            out.push_str("No extends configured.\n");
        }
        Some(Value::String(s)) => {
            out.push_str(&format!("  1. {}\n", s));
        }
        Some(Value::Array(a)) if !a.is_empty() => {
            out.push_str(&format!(
                "Chain ({} configs, applied left-to-right)\n\n",
                a.len()
            ));
            for (i, e) in a.iter().enumerate() {
                let label = if i == 0 {
                    " (base)"
                } else if i == a.len() - 1 {
                    " (top)"
                } else {
                    ""
                };
                if let Some(s) = e.as_str() {
                    out.push_str(&format!("  {}. {}{}\n", i + 1, s, label));
                }
            }
        }
        _ => {
            out.push_str("No extends configured.\n");
        }
    }

    // Note about rule override order
    if obj
        .get("extends")
        .and_then(|v| v.as_array())
        .map(|a| a.len() > 1)
        .unwrap_or(false)
    {
        out.push('\n');
        out.push_str("Note: Later configs in the chain override earlier ones.\n");
        out.push_str("      Rules in this config override all extended configs.\n");
    }

    out
}

fn action_validate(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Stylelint config must be a JSON/YAML object.".to_string(),
    };

    let mut issues: Vec<String> = Vec::new();

    let rules = obj.get("rules").and_then(|v| v.as_object());
    let plugins: Vec<&str> = obj
        .get("plugins")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    // No rules and no extends — probably an empty config
    let extends_count = match obj.get("extends") {
        Some(Value::String(_)) => 1,
        Some(Value::Array(a)) => a.len(),
        _ => 0,
    };
    let rule_count = rules.map(|r| r.len()).unwrap_or(0);
    if rule_count == 0 && extends_count == 0 && plugins.is_empty() {
        issues.push("[WARN]  Config has no rules, extends, or plugins — Stylelint will not enforce any style constraints.".to_string());
    }

    // Deprecated rules (moved or renamed in Stylelint v15+)
    let deprecated_rules = [
        (
            "color-hex-case",
            "Removed in v15. Use a Prettier or similar formatter for case.",
        ),
        ("font-family-name-quotes", "Removed in v15."),
        (
            "function-calc-no-invalid",
            "Removed in v15; use function-calc-no-unspaced-operator.",
        ),
        ("shorthand-property-no-redundant-values", "Removed in v15."),
        (
            "unit-no-unknown",
            "Still valid but check for v15+ naming changes.",
        ),
        (
            "declaration-block-no-duplicate-properties",
            "Renamed in some configs — verify rule name.",
        ),
    ];

    if let Some(rule_obj) = rules {
        for (rule_name, note) in &deprecated_rules {
            if rule_obj.contains_key(*rule_name)
                && rule_severity(rule_obj.get(*rule_name).unwrap()) != "off"
            {
                issues.push(format!("[WARN]  Rule '{}': {}", rule_name, note));
            }
        }

        // Rules that need plugins but plugins aren't listed
        let scss_rules: Vec<&str> = rule_obj
            .keys()
            .filter(|k| k.starts_with("scss/"))
            .map(|k| k.as_str())
            .collect();
        if !scss_rules.is_empty() && !plugins.iter().any(|p| p.contains("scss")) {
            issues.push(format!(
                "[WARN]  {} SCSS rule(s) found (e.g. '{}') but no stylelint-scss plugin detected in 'plugins'.",
                scss_rules.len(),
                scss_rules[0]
            ));
        }

        let order_rules: Vec<&str> = rule_obj
            .keys()
            .filter(|k| k.starts_with("order/"))
            .map(|k| k.as_str())
            .collect();
        if !order_rules.is_empty() && !plugins.iter().any(|p| p.contains("order")) {
            issues.push(format!(
                "[WARN]  {} order rule(s) found (e.g. '{}') but no stylelint-order plugin detected in 'plugins'.",
                order_rules.len(),
                order_rules[0]
            ));
        }

        // Rules set to invalid severity values
        for (name, val) in rule_obj {
            if rule_severity(val) == "?" {
                issues.push(format!(
                    "[WARN]  Rule '{}' has an unrecognized severity value: {}",
                    name, val
                ));
            }
        }

        // Duplicate rule names (can't happen in a JSON object, but double-check for clarity)
        // (JSON parsers deduplicate keys, so this is informational at best)
    }

    // stylelint-config-standard is the common starting point; warn if missing and no extends
    if extends_count == 0 && rule_count > 0 {
        issues.push("[INFO]  No 'extends' configured. Consider extending 'stylelint-config-standard' or 'stylelint-config-recommended' as a baseline.".to_string());
    }

    // Unknown top-level keys
    let known_keys = [
        "rules",
        "plugins",
        "extends",
        "overrides",
        "ignoreFiles",
        "customSyntax",
        "processors",
        "defaultSeverity",
        "reportNeedlessDisables",
        "reportInvalidScopeDisables",
        "allowEmptyInput",
        "cache",
        "cacheLocation",
        "cacheStrategy",
        "fix",
        "formatter",
        "quiet",
        "quietDeprecationWarnings",
    ];
    for key in obj.keys() {
        if !known_keys.contains(&key.as_str()) {
            issues.push(format!(
                "[WARN]  Unknown top-level key '{}' — Stylelint may ignore it.",
                key
            ));
        }
    }

    let mut out = String::from("Stylelint Config Validation\n");
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
            "VALID (with notes)\n\n"
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

pub async fn execute(args: &Value) -> Result<String, String> {
    let cfg = load_config(args)?;
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => Ok(action_info(&cfg)),
        "rules" => Ok(action_rules(&cfg)),
        "plugins" => Ok(action_plugins(&cfg)),
        "extends" => Ok(action_extends(&cfg)),
        "validate" => Ok(action_validate(&cfg)),
        other => Err(format!(
            "Unknown action '{}'. Valid: info, rules, plugins, extends, validate.",
            other
        )),
    }
}
