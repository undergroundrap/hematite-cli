use serde_json::Value;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "babel_tools",
        "description": "Parse, inspect, and validate Babel configuration files (babel.config.json, .babelrc, .babelrc.json, .babelrc.yaml, or 'babel' key in package.json) without external utilities. Auto-detects JSON vs YAML.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "presets", "plugins", "env", "validate"],
                    "description": "info: full config overview (default); presets: detailed preset listing; plugins: detailed plugin listing; env: environment-specific config; validate: common config issues"
                },
                "config": {
                    "type": "string",
                    "description": "Inline Babel config as JSON or YAML string, or package.json JSON containing a 'babel' key"
                },
                "file": {
                    "type": "string",
                    "description": "Path to babel.config.json, .babelrc, .babelrc.json, .babelrc.yaml, or package.json"
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
        return Err("Provide 'config' (inline) or 'file' (path to babel.config.json, .babelrc, or package.json).".to_string());
    };

    let trimmed = text.trim();
    let parsed: Value = if trimmed.starts_with('{') || trimmed.starts_with('[') {
        serde_json::from_str(trimmed).map_err(|e| format!("JSON parse error: {}", e))?
    } else {
        serde_yaml::from_str(trimmed).map_err(|e| format!("YAML parse error: {}", e))?
    };

    if parsed.get("name").is_some() && parsed.get("version").is_some() {
        parsed
            .get("babel")
            .cloned()
            .ok_or_else(|| "No 'babel' key found in package.json. Add a 'babel' config section or use babel.config.json.".to_string())
    } else {
        Ok(parsed)
    }
}

fn preset_name(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or("(unnamed)")
            .to_string(),
        _ => "(complex)".to_string(),
    }
}

fn plugin_name(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or("(unnamed)")
            .to_string(),
        _ => "(complex)".to_string(),
    }
}

fn action_info(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Babel config must be a JSON/YAML object.".to_string(),
    };

    let mut out = String::from("Babel Configuration\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    let preset_count = obj
        .get("presets")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let plugin_count = obj
        .get("plugins")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let env_count = obj
        .get("env")
        .and_then(|v| v.as_object())
        .map(|o| o.len())
        .unwrap_or(0);
    let override_count = obj
        .get("overrides")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    out.push_str(&format!("Presets:   {}\n", preset_count));
    out.push_str(&format!("Plugins:   {}\n", plugin_count));
    if env_count > 0 {
        out.push_str(&format!("Env overrides: {} environment(s)\n", env_count));
    }
    if override_count > 0 {
        out.push_str(&format!("Overrides: {} block(s)\n", override_count));
    }

    if let Some(source_type) = obj.get("sourceType").and_then(|v| v.as_str()) {
        out.push_str(&format!("sourceType: {}\n", source_type));
    }

    if let Some(targets) = obj.get("targets") {
        out.push_str(&format!("targets:   {}\n", targets));
    }

    if preset_count > 0 {
        out.push('\n');
        out.push_str("Presets\n");
        out.push_str(&"─".repeat(52));
        out.push('\n');
        if let Some(arr) = obj.get("presets").and_then(|v| v.as_array()) {
            for p in arr {
                let name = preset_name(p);
                if let Value::Array(a) = p {
                    if a.len() > 1 {
                        out.push_str(&format!("  {} (with options)\n", name));
                    } else {
                        out.push_str(&format!("  {}\n", name));
                    }
                } else {
                    out.push_str(&format!("  {}\n", name));
                }
            }
        }
    }

    if plugin_count > 0 {
        out.push('\n');
        out.push_str("Plugins\n");
        out.push_str(&"─".repeat(52));
        out.push('\n');
        if let Some(arr) = obj.get("plugins").and_then(|v| v.as_array()) {
            for p in arr {
                let name = plugin_name(p);
                if let Value::Array(a) = p {
                    if a.len() > 1 {
                        out.push_str(&format!("  {} (with options)\n", name));
                    } else {
                        out.push_str(&format!("  {}\n", name));
                    }
                } else {
                    out.push_str(&format!("  {}\n", name));
                }
            }
        }
    }

    out
}

fn action_presets(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Babel config must be a JSON/YAML object.".to_string(),
    };

    let mut out = String::from("Babel Presets\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    let presets = match obj.get("presets").and_then(|v| v.as_array()) {
        Some(a) if !a.is_empty() => a,
        _ => {
            out.push_str("No presets configured.\n");
            return out;
        }
    };

    for (i, preset) in presets.iter().enumerate() {
        out.push_str(&format!("\nPreset #{}\n", i + 1));
        match preset {
            Value::String(s) => {
                out.push_str(&format!("  Name:    {}\n", s));
                out.push_str("  Options: (none)\n");
            }
            Value::Array(a) => {
                if let Some(name) = a.first().and_then(|v| v.as_str()) {
                    out.push_str(&format!("  Name:    {}\n", name));
                    if let Some(opts) = a.get(1) {
                        if let Some(omap) = opts.as_object() {
                            out.push_str("  Options:\n");
                            for (k, v) in omap {
                                out.push_str(&format!("    {:<24} {}\n", k, v));
                            }
                        } else {
                            out.push_str(&format!("  Options: {}\n", opts));
                        }
                    } else {
                        out.push_str("  Options: (none)\n");
                    }
                }
            }
            _ => out.push_str(&format!("  (complex): {}\n", preset)),
        }
    }

    out
}

fn action_plugins(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Babel config must be a JSON/YAML object.".to_string(),
    };

    let mut out = String::from("Babel Plugins\n");
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
        out.push_str(&format!("\nPlugin #{}\n", i + 1));
        match plugin {
            Value::String(s) => {
                out.push_str(&format!("  Name:    {}\n", s));
                out.push_str("  Options: (none)\n");
            }
            Value::Array(a) => {
                if let Some(name) = a.first().and_then(|v| v.as_str()) {
                    out.push_str(&format!("  Name:    {}\n", name));
                    if let Some(opts) = a.get(1) {
                        if let Some(omap) = opts.as_object() {
                            out.push_str("  Options:\n");
                            for (k, v) in omap {
                                out.push_str(&format!("    {:<24} {}\n", k, v));
                            }
                        } else {
                            out.push_str(&format!("  Options: {}\n", opts));
                        }
                    } else {
                        out.push_str("  Options: (none)\n");
                    }
                }
            }
            _ => out.push_str(&format!("  (complex): {}\n", plugin)),
        }
    }

    out
}

fn action_env(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Babel config must be a JSON/YAML object.".to_string(),
    };

    let mut out = String::from("Babel Environment Config\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    let env = match obj.get("env").and_then(|v| v.as_object()) {
        Some(e) if !e.is_empty() => e,
        _ => {
            out.push_str("No environment-specific configuration.\n");
            return out;
        }
    };

    for (env_name, env_cfg) in env {
        out.push_str(&format!("\n[{}]\n", env_name));
        if let Some(env_obj) = env_cfg.as_object() {
            if let Some(presets) = env_obj.get("presets").and_then(|v| v.as_array()) {
                out.push_str(&format!("  Presets ({}):\n", presets.len()));
                for p in presets {
                    out.push_str(&format!("    {}\n", preset_name(p)));
                }
            }
            if let Some(plugins) = env_obj.get("plugins").and_then(|v| v.as_array()) {
                out.push_str(&format!("  Plugins ({}):\n", plugins.len()));
                for p in plugins {
                    out.push_str(&format!("    {}\n", plugin_name(p)));
                }
            }
            for (k, v) in env_obj {
                if k != "presets" && k != "plugins" {
                    out.push_str(&format!("  {}: {}\n", k, v));
                }
            }
        }
    }

    out
}

fn action_validate(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Babel config must be a JSON/YAML object.".to_string(),
    };

    let mut issues: Vec<String> = Vec::new();

    let preset_names: Vec<String> = obj
        .get("presets")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().map(preset_name).collect())
        .unwrap_or_default();

    let plugin_names: Vec<String> = obj
        .get("plugins")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().map(plugin_name).collect())
        .unwrap_or_default();

    // @babel/preset-env without targets warning
    let has_preset_env = preset_names
        .iter()
        .any(|n| n.contains("preset-env") || n == "@babel/preset-env");
    if has_preset_env {
        let has_targets = obj.contains_key("targets")
            || obj
                .get("presets")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter().any(|p| {
                        if let Value::Array(pa) = p {
                            if let Some(opts) = pa.get(1).and_then(|v| v.as_object()) {
                                return opts.contains_key("targets");
                            }
                        }
                        false
                    })
                })
                .unwrap_or(false);
        if !has_targets {
            issues.push("[WARN]  @babel/preset-env found without 'targets' — it will compile to ES5 for all browsers. Set targets to reduce output size.".to_string());
        }
    }

    // deprecated presets
    for name in &preset_names {
        if name == "babel-preset-es2015" || name == "es2015" {
            issues.push(format!(
                "[WARN]  Deprecated preset '{}': use @babel/preset-env instead.",
                name
            ));
        }
        if name == "babel-preset-react" {
            issues.push(
                "[WARN]  Deprecated preset 'babel-preset-react': use @babel/preset-react instead."
                    .to_string(),
            );
        }
        if name == "babel-preset-stage-0"
            || name == "babel-preset-stage-1"
            || name == "babel-preset-stage-2"
            || name == "babel-preset-stage-3"
        {
            issues.push(format!("[WARN]  Deprecated stage preset '{}': use specific @babel/plugin-proposal-* plugins instead.", name));
        }
    }

    // deprecated plugins
    for name in &plugin_names {
        if name.starts_with("babel-plugin-") && !name.starts_with("babel-plugin-transform") {
            // non-scoped older plugin naming
        }
        if name == "transform-class-properties" || name == "babel-plugin-transform-class-properties"
        {
            issues.push("[INFO]  Plugin 'transform-class-properties': use @babel/plugin-transform-class-properties instead.".to_string());
        }
        if name == "transform-object-rest-spread"
            || name == "babel-plugin-transform-object-rest-spread"
        {
            issues.push("[INFO]  Plugin 'transform-object-rest-spread': use @babel/plugin-proposal-object-rest-spread instead.".to_string());
        }
    }

    // duplicate plugin detection
    let mut seen_plugins: Vec<String> = Vec::new();
    for name in &plugin_names {
        if seen_plugins.contains(name) {
            issues.push(format!(
                "[WARN]  Duplicate plugin '{}' — remove one occurrence.",
                name
            ));
        } else {
            seen_plugins.push(name.clone());
        }
    }

    // duplicate preset detection
    let mut seen_presets: Vec<String> = Vec::new();
    for name in &preset_names {
        if seen_presets.contains(name) {
            issues.push(format!(
                "[WARN]  Duplicate preset '{}' — remove one occurrence.",
                name
            ));
        } else {
            seen_presets.push(name.clone());
        }
    }

    let mut out = String::from("Babel Config Validation\n");
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
        "presets" => Ok(action_presets(&cfg)),
        "plugins" => Ok(action_plugins(&cfg)),
        "env" => Ok(action_env(&cfg)),
        "validate" => Ok(action_validate(&cfg)),
        other => Err(format!(
            "Unknown action '{}'. Valid: info, presets, plugins, env, validate.",
            other
        )),
    }
}
