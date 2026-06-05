use serde_json::Value;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "jest_tools",
        "description": "Parse, inspect, and validate Jest configuration (jest.config.json, jest.config.js via JSON, or 'jest' key in package.json) without external utilities.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "testmatch", "transforms", "modules", "coverage", "validate"],
                    "description": "info: full config overview (default); testmatch: test file patterns; transforms: file transformer config; modules: moduleNameMapper and dirs; coverage: coverage settings and thresholds; validate: common config issues"
                },
                "config": {
                    "type": "string",
                    "description": "Inline Jest config as JSON string, or package.json JSON containing a 'jest' key"
                },
                "file": {
                    "type": "string",
                    "description": "Path to jest.config.json or package.json"
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
        return Err(
            "Provide 'config' (inline JSON) or 'file' (path to jest.config.json or package.json)."
                .to_string(),
        );
    };

    let parsed: Value =
        serde_json::from_str(text.trim()).map_err(|e| format!("JSON parse error: {}", e))?;

    if parsed.get("name").is_some() && parsed.get("version").is_some() {
        parsed
            .get("jest")
            .cloned()
            .ok_or_else(|| "No 'jest' key found in package.json. Add a 'jest' config section or use jest.config.json.".to_string())
    } else {
        Ok(parsed)
    }
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
        None => return "Error: Jest config must be a JSON object.".to_string(),
    };

    let mut out = String::from("Jest Configuration\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    if let Some(preset) = obj.get("preset").and_then(|v| v.as_str()) {
        out.push_str(&format!("Preset:           {}\n", preset));
    }
    if let Some(env) = obj.get("testEnvironment").and_then(|v| v.as_str()) {
        out.push_str(&format!("Test Environment: {}\n", env));
    } else {
        out.push_str("Test Environment: node (default)\n");
    }
    if let Some(roots) = obj.get("roots").and_then(|v| v.as_array()) {
        out.push_str(&format!(
            "Roots:            {:?}\n",
            roots.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>()
        ));
    }
    if let Some(timeout) = obj.get("testTimeout").and_then(|v| v.as_u64()) {
        out.push_str(&format!("Test Timeout:     {}ms\n", timeout));
    }

    let transform_count = obj
        .get("transform")
        .and_then(|v| v.as_object())
        .map(|o| o.len())
        .unwrap_or(0);
    let mapper_count = obj
        .get("moduleNameMapper")
        .and_then(|v| v.as_object())
        .map(|o| o.len())
        .unwrap_or(0);
    let setup_files = obj
        .get("setupFiles")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let setup_after = obj
        .get("setupFilesAfterFramework")
        .and_then(|v| v.as_array())
        .or_else(|| {
            obj.get("setupFilesAfterFramework")
                .and_then(|v| v.as_array())
        })
        .map(|a| a.len())
        .unwrap_or_else(|| {
            obj.get("setupFilesAfterFramework")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0)
        });
    let coverage_enabled = obj
        .get("collectCoverage")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    out.push('\n');
    out.push_str("Summary\n");
    out.push_str(&"─".repeat(52));
    out.push('\n');
    out.push_str(&format!("  Transforms:         {}\n", transform_count));
    out.push_str(&format!("  Module mappings:    {}\n", mapper_count));
    out.push_str(&format!("  Setup files:        {}\n", setup_files));
    out.push_str(&format!(
        "  Coverage:           {}\n",
        if coverage_enabled {
            "enabled"
        } else {
            "not auto-collected"
        }
    ));

    if let Some(proj_arr) = obj.get("projects").and_then(|v| v.as_array()) {
        out.push_str(&format!(
            "  Projects:           {} (multi-project)\n",
            proj_arr.len()
        ));
    }

    out.push('\n');
    out.push_str("Key Settings\n");
    out.push_str(&"─".repeat(52));
    out.push('\n');

    let display_keys = [
        "preset",
        "testEnvironment",
        "testTimeout",
        "bail",
        "verbose",
        "maxWorkers",
        "passWithNoTests",
        "testPathPattern",
        "rootDir",
        "globalSetup",
        "globalTeardown",
        "runner",
        "testRunner",
        "fakeTimers",
        "clearMocks",
        "restoreMocks",
        "resetMocks",
    ];
    for key in &display_keys {
        if let Some(val) = obj.get(*key) {
            out.push_str(&format!("  {:<28} {}\n", key, compact_val(val)));
        }
    }

    out
}

fn action_testmatch(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Jest config must be a JSON object.".to_string(),
    };

    let mut out = String::from("Jest Test File Patterns\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    let default_match = &[
        "**/__tests__/**/*.[jt]s?(x)",
        "**/?(*.)+(spec|test).[jt]s?(x)",
    ];

    let patterns: Vec<String> = if let Some(arr) = obj.get("testMatch").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect()
    } else if let Some(regex) = obj.get("testRegex") {
        match regex {
            Value::String(s) => vec![format!("(regex) {}", s)],
            Value::Array(a) => a
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| format!("(regex) {}", s))
                .collect(),
            _ => vec![],
        }
    } else {
        default_match.iter().map(|s| s.to_string()).collect()
    };

    let configured = obj.contains_key("testMatch") || obj.contains_key("testRegex");
    out.push_str(if configured {
        "Test Match (configured):\n"
    } else {
        "Test Match (defaults):\n"
    });
    for p in &patterns {
        out.push_str(&format!("  {}\n", p));
    }

    if let Some(ignore) = obj.get("testPathIgnorePatterns").and_then(|v| v.as_array()) {
        out.push('\n');
        out.push_str("Ignored Paths:\n");
        for p in ignore {
            if let Some(s) = p.as_str() {
                out.push_str(&format!("  {}\n", s));
            }
        }
    } else {
        out.push('\n');
        out.push_str("Ignored Paths (default): /node_modules/\n");
    }

    if let Some(ext) = obj.get("testPathPattern").and_then(|v| v.as_str()) {
        out.push('\n');
        out.push_str(&format!("testPathPattern filter: {}\n", ext));
    }

    if let Some(exts) = obj.get("moduleFileExtensions").and_then(|v| v.as_array()) {
        out.push('\n');
        out.push_str("Module File Extensions:\n");
        let list: Vec<_> = exts.iter().filter_map(|v| v.as_str()).collect();
        out.push_str(&format!("  {}\n", list.join(", ")));
    } else {
        out.push('\n');
        out.push_str("Module File Extensions (default): js, mjs, cjs, jsx, ts, tsx, json, node\n");
    }

    out
}

fn action_transforms(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Jest config must be a JSON object.".to_string(),
    };

    let mut out = String::from("Jest Transforms\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    if let Some(xform) = obj.get("transform").and_then(|v| v.as_object()) {
        if xform.is_empty() {
            out.push_str("Transform: {} (all transforms disabled)\n");
        } else {
            out.push_str(&format!("{} transform rule(s):\n\n", xform.len()));
            for (pattern, transformer) in xform {
                out.push_str(&format!("Pattern: {}\n", pattern));
                match transformer {
                    Value::String(s) => out.push_str(&format!("  Transformer: {}\n", s)),
                    Value::Array(a) if !a.is_empty() => {
                        if let Some(t) = a[0].as_str() {
                            out.push_str(&format!("  Transformer: {}\n", t));
                        }
                        if a.len() > 1 {
                            out.push_str(&format!("  Options:     {}\n", a[1]));
                        }
                    }
                    _ => out.push_str(&format!("  Transformer: {}\n", transformer)),
                }
                out.push('\n');
            }
        }
    } else {
        out.push_str(
            "No 'transform' configured — Jest uses babel-jest by default for .js/.ts files.\n",
        );
    }

    if let Some(ignore) = obj
        .get("transformIgnorePatterns")
        .and_then(|v| v.as_array())
    {
        out.push_str("transformIgnorePatterns:\n");
        for p in ignore {
            if let Some(s) = p.as_str() {
                out.push_str(&format!("  {}\n", s));
            }
        }
    } else {
        out.push_str("transformIgnorePatterns (default): /node_modules/, \\.pnp\\.[^\\\\]+$\n");
    }

    out
}

fn action_modules(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Jest config must be a JSON object.".to_string(),
    };

    let mut out = String::from("Jest Module Configuration\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    if let Some(mapper) = obj.get("moduleNameMapper").and_then(|v| v.as_object()) {
        if mapper.is_empty() {
            out.push_str("moduleNameMapper: {} (none)\n");
        } else {
            out.push_str(&format!("moduleNameMapper ({} entries):\n", mapper.len()));
            for (pattern, target) in mapper {
                out.push_str(&format!("  {:<40} → {}\n", pattern, compact_val(target)));
            }
        }
    } else {
        out.push_str("moduleNameMapper: (none configured)\n");
    }

    out.push('\n');

    if let Some(dirs) = obj.get("moduleDirectories").and_then(|v| v.as_array()) {
        let list: Vec<_> = dirs.iter().filter_map(|v| v.as_str()).collect();
        out.push_str(&format!("moduleDirectories: {}\n", list.join(", ")));
    } else {
        out.push_str("moduleDirectories (default): node_modules\n");
    }

    if let Some(paths) = obj.get("modulePaths").and_then(|v| v.as_array()) {
        out.push_str("modulePaths:\n");
        for p in paths {
            if let Some(s) = p.as_str() {
                out.push_str(&format!("  {}\n", s));
            }
        }
    }

    if let Some(exts) = obj.get("moduleFileExtensions").and_then(|v| v.as_array()) {
        let list: Vec<_> = exts.iter().filter_map(|v| v.as_str()).collect();
        out.push_str(&format!("moduleFileExtensions: {}\n", list.join(", ")));
    }

    if let Some(sf) = obj.get("setupFiles").and_then(|v| v.as_array()) {
        out.push('\n');
        out.push_str("setupFiles (before test framework):\n");
        for p in sf {
            if let Some(s) = p.as_str() {
                out.push_str(&format!("  {}\n", s));
            }
        }
    }

    if let Some(sf) = obj
        .get("setupFilesAfterFramework")
        .and_then(|v| v.as_array())
    {
        out.push('\n');
        out.push_str("setupFilesAfterFramework:\n");
        for p in sf {
            if let Some(s) = p.as_str() {
                out.push_str(&format!("  {}\n", s));
            }
        }
    }

    out
}

fn action_coverage(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Jest config must be a JSON object.".to_string(),
    };

    let mut out = String::from("Jest Coverage Configuration\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    let collect = obj
        .get("collectCoverage")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    out.push_str(&format!("collectCoverage:    {}\n", collect));

    if let Some(from) = obj.get("collectCoverageFrom").and_then(|v| v.as_array()) {
        out.push_str("collectCoverageFrom:\n");
        for p in from {
            if let Some(s) = p.as_str() {
                out.push_str(&format!("  {}\n", s));
            }
        }
    }

    if let Some(reporters) = obj.get("coverageReporters").and_then(|v| v.as_array()) {
        let list: Vec<_> = reporters.iter().filter_map(|v| v.as_str()).collect();
        out.push_str(&format!("coverageReporters:  {}\n", list.join(", ")));
    } else {
        out.push_str("coverageReporters (default): json, lcov, text, clover\n");
    }

    if let Some(dir) = obj.get("coverageDirectory").and_then(|v| v.as_str()) {
        out.push_str(&format!("coverageDirectory:  {}\n", dir));
    } else {
        out.push_str("coverageDirectory (default): coverage/\n");
    }

    if let Some(provider) = obj.get("coverageProvider").and_then(|v| v.as_str()) {
        out.push_str(&format!("coverageProvider:   {}\n", provider));
    } else {
        out.push_str("coverageProvider (default): babel\n");
    }

    if let Some(thresholds) = obj.get("coverageThreshold").and_then(|v| v.as_object()) {
        out.push('\n');
        out.push_str("Coverage Thresholds\n");
        out.push_str(&"─".repeat(52));
        out.push('\n');
        for (scope, limits) in thresholds {
            out.push_str(&format!("  [{}]\n", scope));
            if let Some(lobj) = limits.as_object() {
                for (metric, val) in lobj {
                    out.push_str(&format!("    {:<16} {}\n", metric, compact_val(val)));
                }
            }
        }
    } else {
        out.push('\n');
        out.push_str("Coverage Thresholds: (none — build will not fail on low coverage)\n");
    }

    out
}

fn action_validate(cfg: &Value) -> String {
    let obj = match cfg.as_object() {
        Some(o) => o,
        None => return "Error: Jest config must be a JSON object.".to_string(),
    };

    let mut issues: Vec<String> = Vec::new();

    if obj.contains_key("testMatch") && obj.contains_key("testRegex") {
        issues.push("[ERROR] Both 'testMatch' and 'testRegex' are set — Jest only uses one; remove the other.".to_string());
    }

    if let Some(env) = obj.get("testEnvironment").and_then(|v| v.as_str()) {
        if !["node", "jsdom", "happy-dom", "edge-runtime"].contains(&env) && !env.starts_with('<') {
            issues.push(format!("[WARN]  testEnvironment: '{}' is not a standard value — valid: node, jsdom, happy-dom, edge-runtime", env));
        }
    }

    if let Some(timeout) = obj.get("testTimeout").and_then(|v| v.as_u64()) {
        if timeout == 0 {
            issues.push("[ERROR] testTimeout is 0 — tests will time out immediately.".to_string());
        } else if timeout > 60_000 {
            issues.push(format!(
                "[WARN]  testTimeout is {}ms (>60s) — very long; ensure this is intentional.",
                timeout
            ));
        }
    }

    if let Some(workers) = obj.get("maxWorkers") {
        if workers.as_u64() == Some(0) {
            issues.push("[ERROR] maxWorkers: 0 — Jest needs at least 1 worker.".to_string());
        }
    }

    let has_thresholds = obj.get("coverageThreshold").is_some();
    let collects = obj
        .get("collectCoverage")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if has_thresholds && !collects {
        issues.push("[WARN]  coverageThreshold is set but collectCoverage is false — thresholds will not be enforced unless you run with --coverage.".to_string());
    }

    if let Some(xform) = obj.get("transform").and_then(|v| v.as_object()) {
        for (pattern, transformer) in xform {
            let t_name = match transformer {
                Value::String(s) => s.as_str(),
                Value::Array(a) => a.first().and_then(|v| v.as_str()).unwrap_or(""),
                _ => "",
            };
            if t_name.contains("ts-jest")
                && obj.get("preset").and_then(|v| v.as_str()) == Some("ts-jest")
            {
                issues.push("[WARN]  Both 'preset: ts-jest' and a ts-jest transform rule are present — the preset already includes the transform; one is redundant.".to_string());
            }
            if pattern.is_empty() {
                issues.push("[ERROR] Empty pattern key in 'transform' object.".to_string());
            }
        }
    }

    if obj.get("globals").and_then(|v| v.get("ts-jest")).is_some()
        && obj.get("transform").is_none()
        && obj.get("preset").and_then(|v| v.as_str()) != Some("ts-jest")
    {
        issues.push(
            "[WARN]  'globals.ts-jest' is configured but no ts-jest preset or transform is set."
                .to_string(),
        );
    }

    let unknown_keys = [
        "automock",
        "browser",
        "cacheDirectory",
        "clearMocks",
        "coveragePathIgnorePatterns",
        "coverageProvider",
        "coverageReporters",
        "coverageThreshold",
        "dependencyExtractor",
        "displayName",
        "errorOnDeprecated",
        "extensionsToTreatAsEsm",
        "fakeTimers",
        "forceCoverageMatch",
        "globalSetup",
        "globalTeardown",
        "globals",
        "haste",
        "injectGlobals",
        "maxConcurrency",
        "maxWorkers",
        "moduleDirectories",
        "moduleFileExtensions",
        "moduleNameMapper",
        "modulePaths",
        "modulePathIgnorePatterns",
        "notify",
        "notifyMode",
        "passWithNoTests",
        "preset",
        "prettierPath",
        "projects",
        "reporters",
        "resetMocks",
        "resetModules",
        "resolver",
        "restoreMocks",
        "rootDir",
        "roots",
        "runner",
        "runInBand",
        "sandboxInjectedGlobals",
        "setupFiles",
        "setupFilesAfterFramework",
        "slowTestThreshold",
        "snapshotFormat",
        "snapshotResolver",
        "snapshotSerializers",
        "testEnvironment",
        "testEnvironmentOptions",
        "testLocationInResults",
        "testMatch",
        "testNamePattern",
        "testPathIgnorePatterns",
        "testPathPattern",
        "testRegex",
        "testResultsProcessor",
        "testRunner",
        "testSequencer",
        "testTimeout",
        "testURL",
        "timers",
        "transform",
        "transformIgnorePatterns",
        "unmockedModulePathPatterns",
        "verbose",
        "watchPathIgnorePatterns",
        "watchPlugins",
        "watchman",
        "workerIdleMemoryLimit",
        "workerThreads",
        "collectCoverage",
        "collectCoverageFrom",
        "coverageDirectory",
    ];

    for key in obj.keys() {
        if !unknown_keys.contains(&key.as_str()) {
            issues.push(format!(
                "[WARN]  Unknown key '{}' — may be a typo or a custom Jest plugin option.",
                key
            ));
        }
    }

    let mut out = String::from("Jest Config Validation\n");
    out.push_str(&"═".repeat(52));
    out.push('\n');

    if issues.is_empty() {
        out.push_str("VALID — No issues found.\n");
    } else {
        let errors = issues.iter().filter(|i| i.starts_with("[ERROR]")).count();
        let warns = issues.iter().filter(|i| i.starts_with("[WARN]")).count();
        out.push_str(if errors > 0 {
            "INVALID\n\n"
        } else {
            "VALID (with warnings)\n\n"
        });
        out.push_str(&format!(
            "Issues: {} error(s), {} warning(s)\n\n",
            errors, warns
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
        "testmatch" => Ok(action_testmatch(&cfg)),
        "transforms" => Ok(action_transforms(&cfg)),
        "modules" => Ok(action_modules(&cfg)),
        "coverage" => Ok(action_coverage(&cfg)),
        "validate" => Ok(action_validate(&cfg)),
        other => Err(format!(
            "Unknown action '{}'. Valid: info, testmatch, transforms, modules, coverage, validate.",
            other
        )),
    }
}
