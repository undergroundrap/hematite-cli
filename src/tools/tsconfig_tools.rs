use serde_json::Value;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "tsconfig_tools",
        "description": "Parse, inspect, and validate TypeScript configuration files (tsconfig.json) without external utilities.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "compiler", "includes", "references", "validate"],
                    "description": "info (default — overview), compiler (compilerOptions detail), includes (include/exclude/files patterns), references (project references), validate (best-practice checks)"
                },
                "tsconfig": { "type": "string", "description": "Inline tsconfig.json content" },
                "file": { "type": "string", "description": "Path to tsconfig.json file" }
            }
        }
    })
}

fn load_input(args: &Value) -> Result<Value, String> {
    if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        let text = std::fs::read_to_string(f)
            .map_err(|e| format!("Cannot read '{}': {}", f, e))?;
        let stripped = strip_comments(&text);
        serde_json::from_str(&stripped)
            .map_err(|e| format!("JSON parse error in '{}': {}", f, e))
    } else if let Some(t) = args.get("tsconfig").and_then(|v| v.as_str()) {
        let stripped = strip_comments(t);
        serde_json::from_str(&stripped).map_err(|e| format!("JSON parse error: {}", e))
    } else {
        Err("Provide 'file' (path to tsconfig.json) or 'tsconfig' (inline JSON content).".into())
    }
}

fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut in_string = false;
    let mut escape_next = false;
    while let Some(c) = chars.next() {
        if escape_next { out.push(c); escape_next = false; continue; }
        if in_string {
            if c == '\\' { escape_next = true; out.push(c); continue; }
            if c == '"' { in_string = false; }
            out.push(c); continue;
        }
        if c == '"' { in_string = true; out.push(c); continue; }
        if c == '/' {
            if chars.peek() == Some(&'/') {
                chars.next();
                for nc in chars.by_ref() { if nc == '\n' { out.push('\n'); break; } }
                continue;
            } else if chars.peek() == Some(&'*') {
                chars.next();
                let mut prev = ' ';
                for nc in chars.by_ref() { if prev == '*' && nc == '/' { break; } prev = nc; }
                continue;
            }
        }
        out.push(c);
    }
    out
}

fn action_info(cfg: &Value) -> String {
    let mut out = String::from("TypeScript Configuration\n========================\n");

    if let Some(ext) = cfg.get("extends").and_then(|v| v.as_str()) {
        out.push_str(&format!("Extends:        {}\n", ext));
    }

    let co = cfg.get("compilerOptions");
    let co_count = co.and_then(|v| v.as_object()).map(|m| m.len()).unwrap_or(0);
    out.push_str(&format!("compilerOptions: {} settings\n", co_count));

    if let Some(co) = co {
        if let Some(target) = co.get("target").and_then(|v| v.as_str()) {
            out.push_str(&format!("  target:       {}\n", target));
        }
        if let Some(module) = co.get("module").and_then(|v| v.as_str()) {
            out.push_str(&format!("  module:       {}\n", module));
        }
        if let Some(mr) = co.get("moduleResolution").and_then(|v| v.as_str()) {
            out.push_str(&format!("  moduleResol:  {}\n", mr));
        }
        let strict = co.get("strict").and_then(|v| v.as_bool()).unwrap_or(false);
        out.push_str(&format!("  strict:       {}\n", if strict { "true ✓" } else { "false ✗" }));

        if let Some(od) = co.get("outDir").and_then(|v| v.as_str()) {
            out.push_str(&format!("  outDir:       {}\n", od));
        }
        if let Some(rd) = co.get("rootDir").and_then(|v| v.as_str()) {
            out.push_str(&format!("  rootDir:      {}\n", rd));
        }
        let noEmit = co.get("noEmit").and_then(|v| v.as_bool()).unwrap_or(false);
        if noEmit { out.push_str("  noEmit:       true\n"); }
        let composite = co.get("composite").and_then(|v| v.as_bool()).unwrap_or(false);
        if composite { out.push_str("  composite:    true\n"); }
        let jsx = co.get("jsx").and_then(|v| v.as_str());
        if let Some(j) = jsx { out.push_str(&format!("  jsx:          {}\n", j)); }
        let decl_maps = co.get("declaration").and_then(|v| v.as_bool()).unwrap_or(false);
        if decl_maps { out.push_str("  declaration:  true\n"); }
        if let Some(paths) = co.get("paths").and_then(|v| v.as_object()) {
            out.push_str(&format!("  paths:        {} alias(es)\n", paths.len()));
        }
    }

    let include_count = cfg.get("include")
        .and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let exclude_count = cfg.get("exclude")
        .and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let files_count = cfg.get("files")
        .and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let refs_count = cfg.get("references")
        .and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);

    out.push_str(&format!("\ninclude:         {} pattern(s)\n", include_count));
    out.push_str(&format!("exclude:         {} pattern(s)\n", exclude_count));
    if files_count > 0 { out.push_str(&format!("files:           {} explicit file(s)\n", files_count)); }
    if refs_count > 0 { out.push_str(&format!("references:      {} project reference(s)\n", refs_count)); }
    out
}

fn action_compiler(cfg: &Value) -> String {
    let co = match cfg.get("compilerOptions").and_then(|v| v.as_object()) {
        Some(m) => m,
        None => return "No compilerOptions found.".into(),
    };

    let mut out = String::from("compilerOptions\n===============\n\n");

    let categories: &[(&str, &[&str])] = &[
        ("Language & Target", &["target", "module", "moduleResolution", "lib", "jsx", "jsxFactory", "jsxFragmentFactory", "jsxImportSource"]),
        ("Emit", &["outDir", "outFile", "rootDir", "declaration", "declarationDir", "declarationMap", "sourceMap", "inlineSourceMap", "inlineSources", "noEmit", "removeComments", "emitDeclarationOnly"]),
        ("Strict Checks", &["strict", "noImplicitAny", "strictNullChecks", "strictFunctionTypes", "strictBindCallApply", "strictPropertyInitialization", "noImplicitThis", "useUnknownInCatchVariables", "alwaysStrict"]),
        ("Module Resolution", &["baseUrl", "paths", "rootDirs", "typeRoots", "types", "resolveJsonModule", "esModuleInterop", "allowSyntheticDefaultImports", "moduleDetection"]),
        ("Code Quality", &["noUnusedLocals", "noUnusedParameters", "noImplicitReturns", "noFallthroughCasesInSwitch", "noUncheckedIndexedAccess", "exactOptionalPropertyTypes"]),
        ("Project", &["composite", "incremental", "tsBuildInfoFile"]),
        ("Decorators", &["experimentalDecorators", "emitDecoratorMetadata"]),
        ("Other", &[]),
    ];

    let mut shown = std::collections::HashSet::new();

    for (cat, keys) in categories {
        let mut section = String::new();
        if *cat == "Other" {
            for (k, v) in co {
                if !shown.contains(k.as_str()) {
                    section.push_str(&format!("  {:35} {}\n", k, compact_val(v)));
                }
            }
        } else {
            for &k in *keys {
                if let Some(v) = co.get(k) {
                    section.push_str(&format!("  {:35} {}\n", k, compact_val(v)));
                    shown.insert(k);
                }
            }
        }
        if !section.is_empty() {
            out.push_str(&format!("{}:\n", cat));
            out.push_str(&section);
            out.push('\n');
        }
    }
    out
}

fn compact_val(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => {
            let items: Vec<String> = a.iter().map(compact_val).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(m) => format!("{{...{} keys}}", m.len()),
        Value::Null => "null".into(),
    }
}

fn action_includes(cfg: &Value) -> String {
    let mut out = String::from("Include / Exclude / Files\n=========================\n\n");

    for section in &["include", "exclude", "files"] {
        if let Some(arr) = cfg.get(section).and_then(|v| v.as_array()) {
            out.push_str(&format!("{}:\n", section));
            for item in arr {
                if let Some(s) = item.as_str() {
                    out.push_str(&format!("  {}\n", s));
                }
            }
            out.push('\n');
        }
    }

    let co = cfg.get("compilerOptions");
    if let Some(rd) = co.and_then(|c| c.get("rootDirs")).and_then(|v| v.as_array()) {
        out.push_str("rootDirs:\n");
        for d in rd { if let Some(s) = d.as_str() { out.push_str(&format!("  {}\n", s)); } }
        out.push('\n');
    }

    if let Some(paths) = co.and_then(|c| c.get("paths")).and_then(|v| v.as_object()) {
        out.push_str("paths (aliases):\n");
        for (alias, targets) in paths {
            let t = match targets {
                Value::Array(a) => a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "),
                _ => compact_val(targets),
            };
            out.push_str(&format!("  {:30} -> {}\n", alias, t));
        }
        out.push('\n');
    }

    if out.ends_with("=========================\n\n") {
        out.push_str("No include/exclude/files/paths configured (TypeScript will include all .ts files by default).\n");
    }
    out
}

fn action_references(cfg: &Value) -> String {
    let refs = match cfg.get("references").and_then(|v| v.as_array()) {
        Some(a) if !a.is_empty() => a,
        _ => return "No project references configured.".into(),
    };

    let mut out = format!("Project References ({} total)\n", refs.len());
    out.push_str(&"=".repeat(30));
    out.push('\n');

    for r in refs {
        let path = r.get("path").and_then(|v| v.as_str()).unwrap_or("(no path)");
        let prepend = r.get("prepend").and_then(|v| v.as_bool()).unwrap_or(false);
        out.push_str(&format!("  {}", path));
        if prepend { out.push_str(" [prepend]"); }
        out.push('\n');
    }

    if let Some(co) = cfg.get("compilerOptions") {
        let composite = co.get("composite").and_then(|v| v.as_bool()).unwrap_or(false);
        out.push_str(&format!("\nThis project composite: {}\n", if composite { "true ✓" } else { "false — required for referenced projects" }));
    }
    out
}

fn action_validate(cfg: &Value) -> String {
    let mut issues: Vec<String> = Vec::new();

    let co = cfg.get("compilerOptions");

    let strict = co.and_then(|c| c.get("strict")).and_then(|v| v.as_bool()).unwrap_or(false);
    if !strict {
        issues.push("strict is not enabled — enables strict null checks, noImplicitAny, and other safety checks".into());
    }

    let no_emit = co.and_then(|c| c.get("noEmit")).and_then(|v| v.as_bool()).unwrap_or(false);
    let out_dir = co.and_then(|c| c.get("outDir")).and_then(|v| v.as_str());
    if no_emit && out_dir.is_some() {
        issues.push("noEmit: true but outDir is also set — outDir has no effect when noEmit is true".into());
    }
    if !no_emit && out_dir.is_none() {
        issues.push("No outDir set — compiled output will be written next to source files".into());
    }

    let paths = co.and_then(|c| c.get("paths"));
    let base_url = co.and_then(|c| c.get("baseUrl"));
    if paths.is_some() && base_url.is_none() {
        let mr = co.and_then(|c| c.get("moduleResolution")).and_then(|v| v.as_str()).unwrap_or("");
        let bundler_mode = mr.eq_ignore_ascii_case("bundler");
        if !bundler_mode {
            issues.push("paths configured but no baseUrl — paths requires baseUrl (or moduleResolution: 'bundler')".into());
        }
    }

    let mr = co.and_then(|c| c.get("moduleResolution")).and_then(|v| v.as_str()).unwrap_or("");
    if mr.eq_ignore_ascii_case("node") {
        issues.push("moduleResolution: 'node' is deprecated in TypeScript 5 — prefer 'node16', 'nodenext', or 'bundler'".into());
    }

    let exp_dec = co.and_then(|c| c.get("experimentalDecorators")).and_then(|v| v.as_bool()).unwrap_or(false);
    let emit_meta = co.and_then(|c| c.get("emitDecoratorMetadata")).and_then(|v| v.as_bool()).unwrap_or(false);
    if exp_dec && !emit_meta {
        issues.push("experimentalDecorators enabled but emitDecoratorMetadata is not — most decorator frameworks (NestJS, TypeORM) need both".into());
    }

    if let Some(refs) = cfg.get("references").and_then(|v| v.as_array()) {
        if !refs.is_empty() {
            let composite = co.and_then(|c| c.get("composite")).and_then(|v| v.as_bool()).unwrap_or(false);
            if !composite {
                issues.push("Project has references but composite: true is not set — referenced projects require composite".into());
            }
        }
    }

    let target = co.and_then(|c| c.get("target")).and_then(|v| v.as_str()).unwrap_or("ES3");
    if target.eq_ignore_ascii_case("es3") || target.eq_ignore_ascii_case("es5") {
        issues.push(format!("target: '{}' — very old target; consider ES2018 or later for modern environments", target));
    }

    let mut out = String::from("tsconfig.json Validation\n========================\n\n");
    if issues.is_empty() {
        out.push_str("VALID — no issues found.\n");
    } else {
        out.push_str(&format!("WARNINGS ({} issue(s) found):\n\n", issues.len()));
        for (i, issue) in issues.iter().enumerate() {
            out.push_str(&format!("  {}. {}\n", i + 1, issue));
        }
    }
    out
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let cfg = load_input(args)?;
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("info");

    Ok(match action {
        "compiler"   => action_compiler(&cfg),
        "includes"   => action_includes(&cfg),
        "references" => action_references(&cfg),
        "validate"   => action_validate(&cfg),
        _            => action_info(&cfg),
    })
}
