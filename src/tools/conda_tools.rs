use serde_json::{json, Value};
use std::fs;

pub fn conda_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["info", "list", "compare", "validate", "export"],
                "description": "info: environment overview | list: list all dependencies | compare: diff two environments | validate: check for issues | export: convert to pip requirements.txt"
            },
            "file": {"type": "string", "description": "Path to environment.yml"},
            "yaml": {"type": "string", "description": "Inline environment YAML string"},
            "text": {"type": "string", "description": "Alias for yaml"},
            "file_a": {"type": "string", "description": "First environment file for compare"},
            "file_b": {"type": "string", "description": "Second environment file for compare"},
            "yaml_a": {"type": "string", "description": "First environment YAML for compare"},
            "yaml_b": {"type": "string", "description": "Second environment YAML for compare"},
            "type": {"type": "string", "description": "Filter by type: conda or pip"},
            "query": {"type": "string", "description": "Filter by package name substring"}
        },
        "required": []
    })
}

fn load_env(args: &Value, file_key: &str, yaml_key: &str) -> Result<Value, String> {
    if let Some(path) = args.get(file_key).and_then(|v| v.as_str()) {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Cannot read '{}': {}", path, e))?;
        serde_yaml::from_str::<Value>(&content)
            .map_err(|e| format!("Invalid YAML in '{}': {}", path, e))
    } else if let Some(raw) = args
        .get(yaml_key)
        .or_else(|| args.get("text"))
        .and_then(|v| v.as_str())
    {
        serde_yaml::from_str::<Value>(raw).map_err(|e| format!("Invalid YAML: {}", e))
    } else {
        Err(format!(
            "Provide '{}' (file path) or '{}' (inline YAML).",
            file_key, yaml_key
        ))
    }
}

struct Dep {
    name: String,
    version: String,
    dep_type: String, // "conda" or "pip"
}

fn parse_dep_string(s: &str, dep_type: &str) -> Dep {
    // Strip channel prefix: conda-forge::numpy -> numpy
    let s = if s.contains("::") {
        s.splitn(2, "::").nth(1).unwrap_or(s)
    } else {
        s
    };
    // Split on version operators
    let ops = [">=", "<=", "!=", "~=", "==", ">", "<", "="];
    for op in &ops {
        if let Some(pos) = s.find(op) {
            return Dep {
                name: s[..pos].trim().to_string(),
                version: format!("{}{}", op, s[pos + op.len()..].trim()),
                dep_type: dep_type.to_string(),
            };
        }
    }
    Dep {
        name: s.trim().to_string(),
        version: String::new(),
        dep_type: dep_type.to_string(),
    }
}

fn extract_deps(env: &Value) -> Vec<Dep> {
    let mut deps: Vec<Dep> = Vec::new();
    let dep_list = match env.get("dependencies").and_then(|d| d.as_array()) {
        Some(v) => v,
        None => return deps,
    };

    for item in dep_list {
        match item {
            Value::String(s) => {
                deps.push(parse_dep_string(s, "conda"));
            }
            Value::Object(m) => {
                // pip sub-list
                for (k, v) in m {
                    if k == "pip" {
                        if let Some(pip_list) = v.as_array() {
                            for pip_item in pip_list {
                                if let Some(s) = pip_item.as_str() {
                                    deps.push(parse_dep_string(s, "pip"));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    deps
}

fn action_info(env: &Value) -> String {
    let name = env
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("(unnamed)");
    let channels: Vec<&str> = env
        .get("channels")
        .and_then(|c| c.as_array())
        .map(|seq| seq.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let deps = extract_deps(env);
    let conda_count = deps.iter().filter(|d| d.dep_type == "conda").count();
    let pip_count = deps.iter().filter(|d| d.dep_type == "pip").count();

    let python_ver = deps
        .iter()
        .find(|d| d.name.to_lowercase() == "python")
        .map(|d| {
            if d.version.is_empty() {
                "any".to_string()
            } else {
                d.version.clone()
            }
        });

    let has_variables = env.get("variables").is_some();

    let mut out = format!("Conda Environment: {}\n{}\n\n", name, "=".repeat(40));
    out += &format!(
        "Channels:        {}\n",
        if channels.is_empty() {
            "(none)".to_string()
        } else {
            channels.join(", ")
        }
    );
    if let Some(pv) = python_ver {
        out += &format!("Python:          {}\n", pv);
    }
    out += &format!("Dependencies:    {} total\n", deps.len());
    out += &format!("  Conda:         {}\n", conda_count);
    out += &format!("  Pip:           {}\n", pip_count);
    if has_variables {
        if let Some(vars) = env.get("variables").and_then(|v| v.as_object()) {
            out += &format!("\nVariables ({}):\n", vars.len());
            for (k, v) in vars {
                let val = v.as_str().unwrap_or("?");
                out += &format!("  {}={}\n", k, val);
            }
        }
    }
    out
}

fn action_list(args: &Value, env: &Value) -> String {
    let type_filter = args.get("type").and_then(|v| v.as_str());
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let mut deps = extract_deps(env);
    deps.sort_by(|a, b| a.name.cmp(&b.name));

    let mut out = format!("{:<30} {:<20} {}\n", "Package", "Version", "Type");
    out += &format!("{}\n", "-".repeat(60));

    let mut shown = 0;
    for dep in &deps {
        if let Some(tf) = type_filter {
            if dep.dep_type != tf {
                continue;
            }
        }
        if let Some(ref q) = query {
            if !dep.name.to_lowercase().contains(q.as_str()) {
                continue;
            }
        }
        out += &format!(
            "{:<30} {:<20} {}\n",
            dep.name,
            if dep.version.is_empty() {
                "any"
            } else {
                &dep.version
            },
            dep.dep_type
        );
        shown += 1;
    }

    if shown == 0 {
        out += "(no matching packages)\n";
    } else {
        out += &format!("\n{} package(s)\n", shown);
    }
    out
}

fn action_compare(args: &Value) -> Result<String, String> {
    let env_a = if args.get("file_a").is_some() {
        load_env(args, "file_a", "yaml_a")?
    } else {
        load_env(args, "file_a", "yaml_a").or_else(|_| load_env(args, "file_a", "yaml_a"))?
    };
    let env_b = if args.get("file_b").is_some() {
        load_env(args, "file_b", "yaml_b")?
    } else {
        load_env(args, "file_b", "yaml_b")?
    };

    let deps_a = extract_deps(&env_a);
    let deps_b = extract_deps(&env_b);

    let name_a = env_a
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("env_a");
    let name_b = env_b
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("env_b");

    let set_a: std::collections::HashMap<&str, &str> = deps_a
        .iter()
        .map(|d| (d.name.as_str(), d.version.as_str()))
        .collect();
    let set_b: std::collections::HashMap<&str, &str> = deps_b
        .iter()
        .map(|d| (d.name.as_str(), d.version.as_str()))
        .collect();

    let mut only_a: Vec<(&str, &str)> = set_a
        .iter()
        .filter(|(k, _)| !set_b.contains_key(*k))
        .map(|(k, v)| (*k, *v))
        .collect();
    let mut only_b: Vec<(&str, &str)> = set_b
        .iter()
        .filter(|(k, _)| !set_a.contains_key(*k))
        .map(|(k, v)| (*k, *v))
        .collect();
    let mut common: Vec<(&str, &str, &str)> = set_a
        .iter()
        .filter(|(k, _)| set_b.contains_key(*k))
        .map(|(k, va)| {
            let vb = set_b[*k];
            (*k, *va, vb)
        })
        .collect();

    only_a.sort_by_key(|(k, _)| *k);
    only_b.sort_by_key(|(k, _)| *k);
    common.sort_by_key(|(k, _, _)| *k);

    let mut out = format!(
        "Environment Comparison: {} vs {}\n{}\n\n",
        name_a,
        name_b,
        "=".repeat(50)
    );

    out += &format!("Only in {} ({}):\n", name_a, only_a.len());
    for (k, v) in &only_a {
        out += &format!("  - {} {}\n", k, v);
    }

    out += &format!("\nOnly in {} ({}):\n", name_b, only_b.len());
    for (k, v) in &only_b {
        out += &format!("  + {} {}\n", k, v);
    }

    out += &format!("\nIn both ({}):\n", common.len());
    for (k, va, vb) in &common {
        if va != vb {
            out += &format!("  ~ {} ({} | {})\n", k, va, vb);
        } else {
            out += &format!("    {} {}\n", k, va);
        }
    }

    Ok(out)
}

fn action_validate(env: &Value) -> String {
    let mut issues: Vec<String> = Vec::new();

    if env.get("name").and_then(|v| v.as_str()).is_none() {
        issues.push("Missing 'name' field".to_string());
    }

    let channels: Vec<&str> = env
        .get("channels")
        .and_then(|c| c.as_array())
        .map(|seq| seq.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    if channels.is_empty() {
        issues.push("No channels specified".to_string());
    } else if !channels.contains(&"defaults") && !channels.contains(&"conda-forge") {
        issues.push(
            "Neither 'defaults' nor 'conda-forge' channel found — resolution may fail".to_string(),
        );
    }

    let deps = extract_deps(env);

    // Check for duplicates
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for dep in &deps {
        *seen.entry(dep.name.to_lowercase()).or_insert(0) += 1;
    }
    for (name, count) in &seen {
        if *count > 1 {
            issues.push(format!(
                "Duplicate package: '{}' appears {} times",
                name, count
            ));
        }
    }

    if env.get("dependencies").is_none() {
        issues.push("No 'dependencies' section found".to_string());
    }

    let mut out = if issues.is_empty() {
        "VALID\n\nNo issues found.\n".to_string()
    } else {
        format!("INVALID\n\n{} issue(s) found:\n", issues.len())
    };

    for issue in &issues {
        out += &format!("  - {}\n", issue);
    }

    out
}

fn action_export(env: &Value) -> String {
    let deps = extract_deps(env);
    let name = env
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("environment");

    let mut out = format!("# pip requirements.txt exported from conda env: {}\n", name);
    for dep in &deps {
        if dep.name.to_lowercase() == "python" {
            out += &format!("# python{}\n", dep.version);
            continue;
        }
        let ver = dep.version.trim_start_matches('=');
        if ver.is_empty() {
            out += &format!("{}\n", dep.name);
        } else {
            // Normalize conda =X.Y to ==X.Y for pip
            let pip_ver = if dep.version.starts_with('=') && !dep.version.starts_with("==") {
                format!("=={}", ver)
            } else {
                dep.version.clone()
            };
            out += &format!("{}{}\n", dep.name, pip_ver);
        }
    }
    out
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    Ok(match action {
        "compare" => action_compare(args)?,
        _ => {
            let env = load_env(args, "file", "yaml")?;
            match action {
                "info" => action_info(&env),
                "list" => action_list(args, &env),
                "validate" => action_validate(&env),
                "export" => action_export(&env),
                other => format!(
                    "Unknown action '{}'. Use: info, list, compare, validate, export",
                    other
                ),
            }
        }
    })
}
