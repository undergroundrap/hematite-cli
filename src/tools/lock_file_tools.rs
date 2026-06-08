use serde_json::Value;
use std::collections::HashMap;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let text = get_text(args)?;
    let format = detect_format(args, &text);

    match action {
        "info" => action_info(&text, &format),
        "list" => action_list(&text, &format, args),
        "search" => action_search(&text, &format, args),
        "duplicates" => action_duplicates(&text, &format),
        other => Err(format!(
            "Unknown action '{}'. Valid: info, list, search, duplicates",
            other
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    if let Some(p) = args
        .get("file")
        .or_else(|| args.get("path"))
        .and_then(|v| v.as_str())
    {
        return std::fs::read_to_string(p).map_err(|e| format!("Cannot read '{}': {}", p, e));
    }
    args.get("text")
        .or_else(|| args.get("content"))
        .or_else(|| args.get("lock"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            "Missing 'file' or 'text' — pass path to Cargo.lock / package-lock.json / yarn.lock / poetry.lock".to_string()
        })
}

fn detect_format(args: &Value, text: &str) -> String {
    if let Some(fmt) = args
        .get("format")
        .or_else(|| args.get("type"))
        .and_then(|v| v.as_str())
    {
        return fmt.to_lowercase();
    }
    if let Some(p) = args
        .get("file")
        .or_else(|| args.get("path"))
        .and_then(|v| v.as_str())
    {
        let name = p.split(['/', '\\']).next_back().unwrap_or(p).to_lowercase();
        if name == "cargo.lock" {
            return "cargo".to_string();
        }
        if name == "package-lock.json" {
            return "npm".to_string();
        }
        if name == "yarn.lock" {
            return "yarn".to_string();
        }
        if name == "poetry.lock" {
            return "poetry".to_string();
        }
    }
    // Heuristic from content
    if text.trim_start().starts_with('{') {
        return "npm".to_string();
    }
    if text.contains("# yarn lockfile") {
        return "yarn".to_string();
    }
    if text.contains("[metadata]") || text.contains("[[package]]") {
        if text.contains("content-hash") {
            return "poetry".to_string();
        }
        return "cargo".to_string();
    }
    "cargo".to_string()
}

// ── Package struct ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Package {
    name: String,
    version: String,
    source: Option<String>,
}

// ── Cargo.lock parser ────────────────────────────────────────────────────────

fn parse_cargo_lock(text: &str) -> Vec<Package> {
    let mut packages = Vec::new();
    let mut name = None::<String>;
    let mut version = None::<String>;
    let mut source = None::<String>;
    let mut in_package = false;

    for line in text.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            if in_package {
                if let (Some(n), Some(v)) = (name.take(), version.take()) {
                    packages.push(Package {
                        name: n,
                        version: v,
                        source: source.take(),
                    });
                }
            }
            in_package = true;
            name = None;
            version = None;
            source = None;
        } else if in_package {
            if let Some(val) = kv_value(line, "name") {
                name = Some(val);
            } else if let Some(val) = kv_value(line, "version") {
                version = Some(val);
            } else if let Some(val) = kv_value(line, "source") {
                source = Some(val);
            }
        }
    }
    if in_package {
        if let (Some(n), Some(v)) = (name, version) {
            packages.push(Package {
                name: n,
                version: v,
                source,
            });
        }
    }
    packages
}

fn kv_value(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{} = ", key);
    if line.starts_with(&prefix) {
        let val = line[prefix.len()..].trim();
        // strip surrounding quotes
        if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
            return Some(val[1..val.len() - 1].to_string());
        }
        return Some(val.to_string());
    }
    None
}

// ── package-lock.json parser (npm v2/v3) ─────────────────────────────────────

fn parse_npm_lock(text: &str) -> Result<Vec<Package>, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("JSON parse error: {}", e))?;

    let mut packages = Vec::new();

    // v3 format: root["packages"]["node_modules/..."]
    if let Some(pkgs) = root.get("packages").and_then(|v| v.as_object()) {
        for (path, pkg) in pkgs {
            // Skip the root entry "" and workspace entries
            if path.is_empty() || !path.starts_with("node_modules/") {
                continue;
            }
            let name = path
                .trim_start_matches("node_modules/")
                // Handle scoped packages: node_modules/@scope/pkg
                .to_string();
            let version = pkg
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let resolved = pkg
                .get("resolved")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            packages.push(Package {
                name,
                version,
                source: resolved,
            });
        }
    }
    // v1 fallback: root["dependencies"]
    if packages.is_empty() {
        if let Some(deps) = root.get("dependencies").and_then(|v| v.as_object()) {
            collect_npm_deps(deps, &mut packages);
        }
    }

    Ok(packages)
}

fn collect_npm_deps(deps: &serde_json::Map<String, Value>, out: &mut Vec<Package>) {
    for (name, pkg) in deps {
        let version = pkg
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let resolved = pkg
            .get("resolved")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        out.push(Package {
            name: name.clone(),
            version,
            source: resolved,
        });
        // Recurse into nested dependencies (v1 format)
        if let Some(nested) = pkg.get("dependencies").and_then(|v| v.as_object()) {
            collect_npm_deps(nested, out);
        }
    }
}

// ── yarn.lock parser ─────────────────────────────────────────────────────────

fn parse_yarn_lock(text: &str) -> Vec<Package> {
    let mut packages = Vec::new();
    let mut current_names: Vec<String> = Vec::new();
    let mut version = None::<String>;
    let mut resolved = None::<String>;

    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        // Entry header: "name@version, name@version:"
        if !line.starts_with(' ') && line.ends_with(':') {
            // Flush previous
            if let Some(v) = version.take() {
                for name in current_names.drain(..) {
                    packages.push(Package {
                        name,
                        version: v.clone(),
                        source: resolved.clone(),
                    });
                }
            }
            resolved = None;
            // Parse one or more "name@range" entries separated by ", "
            let header = line.trim_end_matches(':');
            for entry in header.split(", ") {
                let entry = entry.trim().trim_matches('"');
                // The name is everything before the last '@' (handles scoped packages)
                if let Some(at_pos) = entry.rfind('@') {
                    let name = entry[..at_pos].to_string();
                    if !name.is_empty() && !current_names.contains(&name) {
                        current_names.push(name);
                    }
                }
            }
        } else if line.starts_with("  version") {
            let val = line
                .trim()
                .trim_start_matches("version")
                .trim()
                .trim_matches('"');
            version = Some(val.to_string());
        } else if line.starts_with("  resolved") {
            let val = line
                .trim()
                .trim_start_matches("resolved")
                .trim()
                .trim_matches('"');
            resolved = Some(val.to_string());
        }
    }
    // Flush last
    if let Some(v) = version {
        for name in current_names {
            packages.push(Package {
                name,
                version: v.clone(),
                source: resolved.clone(),
            });
        }
    }
    packages
}

// ── poetry.lock parser ───────────────────────────────────────────────────────

fn parse_poetry_lock(text: &str) -> Vec<Package> {
    // poetry.lock is TOML with [[package]] blocks like Cargo.lock
    let mut packages = Vec::new();
    let mut name = None::<String>;
    let mut version = None::<String>;
    let mut category = None::<String>;
    let mut in_package = false;

    for line in text.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            if in_package {
                if let (Some(n), Some(v)) = (name.take(), version.take()) {
                    packages.push(Package {
                        name: n,
                        version: v,
                        source: category.take(),
                    });
                }
            }
            in_package = true;
            name = None;
            version = None;
            category = None;
        } else if in_package {
            if let Some(val) = kv_value(line, "name") {
                name = Some(val);
            } else if let Some(val) = kv_value(line, "version") {
                version = Some(val);
            } else if let Some(val) = kv_value(line, "category") {
                category = Some(val);
            }
        }
    }
    if in_package {
        if let (Some(n), Some(v)) = (name, version) {
            packages.push(Package {
                name: n,
                version: v,
                source: category,
            });
        }
    }
    packages
}

// ── Parse dispatch ───────────────────────────────────────────────────────────

fn parse(text: &str, format: &str) -> Result<(Vec<Package>, String), String> {
    match format {
        "cargo" => Ok((parse_cargo_lock(text), "Cargo.lock".to_string())),
        "npm" => Ok((parse_npm_lock(text)?, "package-lock.json".to_string())),
        "yarn" => Ok((parse_yarn_lock(text), "yarn.lock".to_string())),
        "poetry" => Ok((parse_poetry_lock(text), "poetry.lock".to_string())),
        other => Err(format!(
            "Unknown lock file format '{}'. Valid: cargo, npm, yarn, poetry",
            other
        )),
    }
}

fn lock_file_version(root: &Value) -> Option<u64> {
    root.get("lockfileVersion").and_then(|v| v.as_u64())
}

// ── Actions ──────────────────────────────────────────────────────────────────

fn action_info(text: &str, format: &str) -> Result<String, String> {
    let (packages, label) = parse(text, format)?;

    let mut out = format!("lock_file_tools — info\n\nFormat: {}\n", label);

    // npm-specific metadata
    if format == "npm" {
        if let Ok(root) = serde_json::from_str::<Value>(text) {
            if let Some(name) = root.get("name").and_then(|v| v.as_str()) {
                out.push_str(&format!("Project: {}\n", name));
            }
            if let Some(v) = lock_file_version(&root) {
                out.push_str(&format!("Lockfile version: {}\n", v));
            }
        }
    }

    let unique_names: std::collections::HashSet<&str> =
        packages.iter().map(|p| p.name.as_str()).collect();

    out.push_str(&format!("Packages:  {}\n", packages.len()));
    if packages.len() != unique_names.len() {
        out.push_str(&format!(
            "Unique:    {} ({} duplicate name(s))\n",
            unique_names.len(),
            packages.len() - unique_names.len()
        ));
    }

    // Count duplicates (same name, different version)
    let mut name_versions: HashMap<&str, Vec<&str>> = HashMap::new();
    for p in &packages {
        name_versions
            .entry(p.name.as_str())
            .or_default()
            .push(p.version.as_str());
    }
    let dup_count = name_versions.values().filter(|v| v.len() > 1).count();
    if dup_count > 0 {
        out.push_str(&format!(
            "Conflicts: {} package(s) at multiple versions\n",
            dup_count
        ));
    }

    // Format-specific extras
    if format == "cargo" {
        let registry: Vec<&Package> = packages
            .iter()
            .filter(|p| p.source.as_deref().unwrap_or("").contains("registry"))
            .collect();
        let git_deps: Vec<&Package> = packages
            .iter()
            .filter(|p| p.source.as_deref().unwrap_or("").starts_with("git"))
            .collect();
        let local_deps: Vec<&Package> = packages.iter().filter(|p| p.source.is_none()).collect();
        out.push_str(&format!(
            "Registry:  {}  Git: {}  Local/path: {}\n",
            registry.len(),
            git_deps.len(),
            local_deps.len()
        ));
    }

    Ok(out)
}

fn action_list(text: &str, format: &str, args: &Value) -> Result<String, String> {
    let (mut packages, label) = parse(text, format)?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

    // Sort by name
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    let mut out = format!("lock_file_tools — list ({})\n\n", label);
    let total = packages.len();
    let shown = packages.len().min(limit);

    out.push_str(&format!("{:<45} {}\n", "Package", "Version"));
    out.push_str(&format!("{}\n", "-".repeat(65)));

    for p in packages.iter().take(shown) {
        out.push_str(&format!("{:<45} {}\n", p.name, p.version));
    }

    if total > shown {
        out.push_str(&format!(
            "\n… {} more (pass 'limit' to show more)\n",
            total - shown
        ));
    } else {
        out.push_str(&format!("\nTotal: {} package(s)\n", total));
    }
    Ok(out)
}

fn action_search(text: &str, format: &str, args: &Value) -> Result<String, String> {
    let (packages, label) = parse(text, format)?;
    let query = args
        .get("query")
        .or_else(|| args.get("q"))
        .or_else(|| args.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'query' — pass the package name to search for".to_string())?;

    let q = query.to_lowercase();
    let matches: Vec<&Package> = packages
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&q))
        .collect();

    let mut out = format!(
        "lock_file_tools — search ({})\n\nQuery: {:?} — {} match(es)\n\n",
        label,
        query,
        matches.len()
    );

    if matches.is_empty() {
        out.push_str("No packages found.\n");
    } else {
        out.push_str(&format!("{:<45} {}\n", "Package", "Version"));
        out.push_str(&format!("{}\n", "-".repeat(65)));
        for p in &matches {
            let src = p
                .source
                .as_deref()
                .unwrap_or("")
                .split('/')
                .next_back()
                .unwrap_or("");
            if src.is_empty() {
                out.push_str(&format!("{:<45} {}\n", p.name, p.version));
            } else {
                out.push_str(&format!("{:<45} {}  ({})\n", p.name, p.version, src));
            }
        }
    }
    Ok(out)
}

fn action_duplicates(text: &str, format: &str) -> Result<String, String> {
    let (packages, label) = parse(text, format)?;

    let mut name_versions: HashMap<String, Vec<String>> = HashMap::new();
    for p in &packages {
        name_versions
            .entry(p.name.clone())
            .or_default()
            .push(p.version.clone());
    }

    let mut dupes: Vec<(&String, &Vec<String>)> = name_versions
        .iter()
        .filter(|(_, versions)| versions.len() > 1)
        .collect();
    dupes.sort_by_key(|(name, _)| name.as_str());

    let mut out = format!(
        "lock_file_tools — duplicates ({})\n\n{} package(s) at multiple versions\n\n",
        label,
        dupes.len()
    );

    if dupes.is_empty() {
        out.push_str("No duplicate packages found — dependency tree is clean.\n");
    } else {
        for (name, versions) in &dupes {
            out.push_str(&format!("{}\n", name));
            for v in versions.iter() {
                out.push_str(&format!("  {}\n", v));
            }
        }
        if !dupes.is_empty() {
            out.push_str(
                "\nDuplicate packages increase bundle size and can cause compatibility issues.\n",
            );
            out.push_str(
                "Use 'npm dedupe', 'yarn dedupe', or 'cargo update' to reduce conflicts.\n",
            );
        }
    }
    Ok(out)
}
