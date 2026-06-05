use serde_json::Value;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "eslint_tools",
        "description": "Parse, inspect, and validate ESLint configuration files (.eslintrc.json, eslint.config.js/mjs, or inline JSON) without external utilities.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "rules", "plugins", "extends", "validate"],
                    "description": "info (default — overview), rules (all configured rules with severity), plugins (plugin list), extends (preset/extends chain), validate (best-practice checks)"
                },
                "config": { "type": "string", "description": "Inline ESLint config JSON content (.eslintrc.json format)" },
                "file": { "type": "string", "description": "Path to .eslintrc.json or eslint.config.json" }
            }
        }
    })
}

#[derive(Debug)]
enum ConfigFormat {
    Legacy,  // .eslintrc.json style (root/env/extends/rules)
    Flat,    // eslint.config.js style array (objects with files/rules/plugins)
}

struct ParsedConfig {
    format: ConfigFormat,
    root: bool,
    parser: Option<String>,
    envs: Vec<String>,
    globals: Vec<String>,
    plugins: Vec<String>,
    extends: Vec<String>,
    rules: Vec<(String, String, String)>,  // (name, severity, options)
    ignore_patterns: Vec<String>,
    overrides_count: usize,
}

fn severity_label(v: &Value) -> String {
    match v {
        Value::Number(n) => match n.as_u64() {
            Some(0) => "off".into(),
            Some(1) => "warn".into(),
            Some(2) => "error".into(),
            _ => n.to_string(),
        },
        Value::String(s) => s.clone(),
        Value::Array(a) if !a.is_empty() => severity_label(&a[0]),
        _ => "?".into(),
    }
}

fn rule_options(v: &Value) -> String {
    match v {
        Value::Array(a) if a.len() > 1 => {
            let opts: Vec<String> = a[1..].iter().map(|x| {
                let s = x.to_string();
                if s.len() > 60 { format!("{}...", &s[..60]) } else { s }
            }).collect();
            opts.join(", ")
        }
        _ => String::new(),
    }
}

fn parse_legacy(cfg: &Value) -> ParsedConfig {
    let root = cfg.get("root").and_then(|v| v.as_bool()).unwrap_or(false);

    let parser = cfg.get("parser").and_then(|v| v.as_str()).map(String::from);

    let envs: Vec<String> = cfg.get("env")
        .and_then(|v| v.as_object())
        .map(|m| m.iter()
            .filter(|(_, v)| v.as_bool().unwrap_or(false))
            .map(|(k, _)| k.clone())
            .collect())
        .unwrap_or_default();

    let globals: Vec<String> = cfg.get("globals")
        .and_then(|v| v.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();

    let plugins: Vec<String> = match cfg.get("plugins") {
        Some(Value::Array(a)) => a.iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    };

    let extends: Vec<String> = match cfg.get("extends") {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(a)) => a.iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    };

    let mut rules: Vec<(String, String, String)> = Vec::new();
    if let Some(Value::Object(rmap)) = cfg.get("rules") {
        for (name, val) in rmap {
            let sev = severity_label(val);
            let opts = rule_options(val);
            rules.push((name.clone(), sev, opts));
        }
    }
    rules.sort_by(|a, b| {
        let ord = ["error", "warn", "off"];
        let ai = ord.iter().position(|&x| x == a.1).unwrap_or(3);
        let bi = ord.iter().position(|&x| x == b.1).unwrap_or(3);
        ai.cmp(&bi).then(a.0.cmp(&b.0))
    });

    let ignore_patterns: Vec<String> = match cfg.get("ignorePatterns") {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str()).map(String::from).collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    };

    let overrides_count = cfg.get("overrides")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    ParsedConfig {
        format: ConfigFormat::Legacy,
        root,
        parser,
        envs,
        globals,
        plugins,
        extends,
        rules,
        ignore_patterns,
        overrides_count,
    }
}

fn parse_flat(arr: &[Value]) -> ParsedConfig {
    let mut plugins: Vec<String> = Vec::new();
    let mut rules: Vec<(String, String, String)> = Vec::new();
    let mut globals: Vec<String> = Vec::new();
    let mut ignore_patterns: Vec<String> = Vec::new();

    for obj in arr {
        if let Some(m) = obj.as_object() {
            if let Some(Value::Object(pm)) = m.get("plugins") {
                for k in pm.keys() { if !plugins.contains(k) { plugins.push(k.clone()); } }
            }
            if let Some(Value::Object(rmap)) = m.get("rules") {
                for (name, val) in rmap {
                    let sev = severity_label(val);
                    let opts = rule_options(val);
                    if !rules.iter().any(|(n, _, _)| n == name) {
                        rules.push((name.clone(), sev, opts));
                    }
                }
            }
            if let Some(Value::Object(lg)) = m.get("languageOptions") {
                if let Some(Value::Object(gmap)) = lg.get("globals") {
                    for k in gmap.keys() { if !globals.contains(k) { globals.push(k.clone()); } }
                }
            }
            if let Some(Value::Array(ig)) = m.get("ignores") {
                for pat in ig {
                    if let Some(s) = pat.as_str() {
                        if !ignore_patterns.contains(&s.to_string()) { ignore_patterns.push(s.to_string()); }
                    }
                }
            }
        }
    }

    rules.sort_by(|a, b| {
        let ord = ["error", "warn", "off"];
        let ai = ord.iter().position(|&x| x == a.1).unwrap_or(3);
        let bi = ord.iter().position(|&x| x == b.1).unwrap_or(3);
        ai.cmp(&bi).then(a.0.cmp(&b.0))
    });

    ParsedConfig {
        format: ConfigFormat::Flat,
        root: true,
        parser: None,
        envs: Vec::new(),
        globals,
        plugins,
        extends: Vec::new(),
        rules,
        ignore_patterns,
        overrides_count: 0,
    }
}

fn load_and_parse(args: &Value) -> Result<ParsedConfig, String> {
    let text = if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(f)
            .map_err(|e| format!("Cannot read '{}': {}", f, e))?
    } else if let Some(t) = args.get("config").and_then(|v| v.as_str()) {
        t.to_string()
    } else {
        return Err("Provide 'file' (path to ESLint config) or 'config' (inline JSON content).".into());
    };

    let json: Value = serde_json::from_str(&text)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    if json.is_array() {
        Ok(parse_flat(json.as_array().unwrap()))
    } else {
        Ok(parse_legacy(&json))
    }
}

fn format_name(f: &ConfigFormat) -> &'static str {
    match f { ConfigFormat::Legacy => "Legacy (.eslintrc.json)", ConfigFormat::Flat => "Flat (eslint.config.js)" }
}

fn sev_icon(s: &str) -> &'static str {
    match s { "error" | "2" => "✗", "warn" | "1" => "⚠", "off" | "0" => "·", _ => "?" }
}

fn action_info(cfg: &ParsedConfig) -> String {
    let mut out = String::from("ESLint Configuration\n====================\n\n");
    out.push_str(&format!("Format:          {}\n", format_name(&cfg.format)));
    if let ConfigFormat::Legacy = cfg.format {
        out.push_str(&format!("root:            {}\n", cfg.root));
    }
    if let Some(p) = &cfg.parser {
        out.push_str(&format!("parser:          {}\n", p));
    }
    if !cfg.extends.is_empty() {
        out.push_str(&format!("extends:         {} preset(s)\n", cfg.extends.len()));
        for e in &cfg.extends { out.push_str(&format!("  - {}\n", e)); }
    }
    if !cfg.plugins.is_empty() {
        out.push_str(&format!("plugins:         {} plugin(s)\n", cfg.plugins.len()));
        for p in &cfg.plugins { out.push_str(&format!("  - {}\n", p)); }
    }
    if !cfg.envs.is_empty() {
        out.push_str(&format!("env:             {} environment(s)\n", cfg.envs.len()));
        for e in &cfg.envs { out.push_str(&format!("  - {}\n", e)); }
    }
    if !cfg.globals.is_empty() {
        out.push_str(&format!("globals:         {} global(s)\n", cfg.globals.len()));
    }

    let errors = cfg.rules.iter().filter(|(_, s, _)| s == "error" || s == "2").count();
    let warns  = cfg.rules.iter().filter(|(_, s, _)| s == "warn" || s == "1").count();
    let offs   = cfg.rules.iter().filter(|(_, s, _)| s == "off" || s == "0").count();

    out.push_str(&format!("rules:           {} total\n", cfg.rules.len()));
    if cfg.rules.is_empty() {
        out.push_str("  (no rules configured — relying entirely on extends/presets)\n");
    } else {
        if errors > 0 { out.push_str(&format!("  error:  {}\n", errors)); }
        if warns  > 0 { out.push_str(&format!("  warn:   {}\n", warns));  }
        if offs   > 0 { out.push_str(&format!("  off:    {}\n", offs));   }
    }
    if !cfg.ignore_patterns.is_empty() {
        out.push_str(&format!("ignorePatterns:  {} pattern(s)\n", cfg.ignore_patterns.len()));
    }
    if cfg.overrides_count > 0 {
        out.push_str(&format!("overrides:       {} block(s)\n", cfg.overrides_count));
    }
    out
}

fn action_rules(cfg: &ParsedConfig, filter: Option<&str>) -> String {
    if cfg.rules.is_empty() {
        return "No rules explicitly configured.".into();
    }

    let filtered: Vec<&(String, String, String)> = cfg.rules.iter()
        .filter(|(name, _, _)| filter.map(|f| name.to_lowercase().contains(f)).unwrap_or(true))
        .collect();

    if filtered.is_empty() {
        return format!("No rules match filter '{}'.", filter.unwrap_or(""));
    }

    let mut out = format!("Rules ({}{}):\n",
        filtered.len(),
        filter.map(|f| format!(" matching '{}'", f)).unwrap_or_default()
    );
    out.push_str(&format!("{:<4} {:<45} {}\n", "Sev", "Rule", "Options"));
    out.push_str(&format!("{} {} {}\n", "-".repeat(4), "-".repeat(45), "-".repeat(30)));

    for (name, sev, opts) in &filtered {
        let icon = sev_icon(sev);
        let opts_str = if opts.is_empty() { String::new() } else { format!(" {}", opts) };
        out.push_str(&format!("{:<4} {:<45}{}\n",
            format!("{} {}", icon, sev),
            name,
            opts_str
        ));
    }
    out
}

fn action_plugins(cfg: &ParsedConfig) -> String {
    if cfg.plugins.is_empty() {
        return "No plugins configured.".into();
    }
    let mut out = format!("Plugins ({}):\n", cfg.plugins.len());
    for p in &cfg.plugins { out.push_str(&format!("  - {}\n", p)); }
    out
}

fn action_extends(cfg: &ParsedConfig) -> String {
    if cfg.extends.is_empty() {
        match cfg.format {
            ConfigFormat::Flat => return "Flat config does not use extends — plugins are imported directly.".into(),
            ConfigFormat::Legacy => return "No extends configured.".into(),
        }
    }
    let mut out = format!("Extends Chain ({}):\n", cfg.extends.len());
    for (i, e) in cfg.extends.iter().enumerate() {
        out.push_str(&format!("  {}. {}\n", i + 1, e));
    }
    out.push_str("\nNote: extends are applied left-to-right; later entries override earlier ones.\n");
    out
}

fn action_validate(cfg: &ParsedConfig) -> String {
    let mut issues: Vec<String> = Vec::new();

    if cfg.rules.is_empty() && cfg.extends.is_empty() {
        issues.push("No rules and no extends — this config has no lint effect.".into());
    } else if cfg.rules.is_empty() && cfg.extends.is_empty() {
        issues.push("No rules configured and no extends presets — nothing will be enforced.".into());
    }

    if let ConfigFormat::Legacy = cfg.format {
        if !cfg.root {
            issues.push("root: true not set — ESLint will continue searching parent directories for more configs.".into());
        }
        for e in &cfg.extends {
            if e.starts_with("plugin:") && e.contains("flat/") {
                issues.push(format!("extends '{}' looks like a flat config preset — use flat config format (array) instead.", e));
            }
        }
        for e in &cfg.extends {
            if e == "eslint:all" {
                issues.push("extends 'eslint:all' enables every ESLint rule and will break on most projects; use 'eslint:recommended' instead.".into());
            }
        }
    }

    if let ConfigFormat::Flat = cfg.format {
        for (name, _, _) in &cfg.rules {
            if name.starts_with("react/") && !cfg.plugins.iter().any(|p| p == "react") {
                issues.push(format!("Rule '{}' uses 'react/' prefix but 'react' plugin is not configured.", name));
                break;
            }
            if name.starts_with("@typescript-eslint/") && !cfg.plugins.iter().any(|p| p.contains("typescript")) {
                issues.push(format!("Rule '{}' uses '@typescript-eslint/' prefix but the typescript plugin is not configured.", name));
                break;
            }
        }
    }

    for (name, sev, _) in &cfg.rules {
        if sev != "error" && sev != "warn" && sev != "off" &&
           sev != "0" && sev != "1" && sev != "2" {
            issues.push(format!("Rule '{}' has unexpected severity '{}' — valid values are 'error'/'2', 'warn'/'1', 'off'/'0'.", name, sev));
        }
    }

    let error_count = cfg.rules.iter().filter(|(_, s, _)| s == "error" || s == "2").count();
    let warn_count  = cfg.rules.iter().filter(|(_, s, _)| s == "warn" || s == "1").count();
    let _off_count  = cfg.rules.iter().filter(|(_, s, _)| s == "off" || s == "0").count();

    let mut out = String::from("ESLint Config Validation\n========================\n\n");
    if issues.is_empty() {
        out.push_str("VALID — no issues found.\n");
    } else {
        out.push_str(&format!("WARNINGS ({} issue(s)):\n\n", issues.len()));
        for (i, issue) in issues.iter().enumerate() {
            out.push_str(&format!("  {}. {}\n", i + 1, issue));
        }
    }
    out.push_str(&format!("\nSummary: {} error rule(s), {} warn rule(s), {} preset(s), {} plugin(s)\n",
        error_count, warn_count, cfg.extends.len(), cfg.plugins.len()));
    out
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let cfg = load_and_parse(args)?;
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("info");
    let filter = args.get("filter").and_then(|v| v.as_str());

    Ok(match action {
        "rules"   => action_rules(&cfg, filter),
        "plugins" => action_plugins(&cfg),
        "extends" => action_extends(&cfg),
        "validate" => action_validate(&cfg),
        _         => action_info(&cfg),
    })
}
