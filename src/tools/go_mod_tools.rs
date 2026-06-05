use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "description": "info (default) | require | replace | exclude | validate"
            },
            "gomod": {
                "type": "string",
                "description": "Inline go.mod content"
            },
            "file": {
                "type": "string",
                "description": "Path to go.mod"
            },
            "filter": {
                "type": "string",
                "description": "For require: substring filter on module path"
            },
            "indirect": {
                "type": "boolean",
                "description": "For require: true = indirect only, false = direct only"
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let (text, filename) = if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        let content =
            std::fs::read_to_string(f).map_err(|e| format!("Cannot read '{}': {}", f, e))?;
        let fname = std::path::Path::new(f)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(f)
            .to_string();
        (content, fname)
    } else if let Some(t) = args
        .get("gomod")
        .or_else(|| args.get("text"))
        .and_then(|v| v.as_str())
    {
        (t.to_string(), "go.mod".to_string())
    } else {
        return Err("Provide 'file' (path to go.mod) or 'gomod' (inline content).".into());
    };

    match action {
        "require" | "deps" | "dependencies" => do_require(&text, args),
        "replace" => do_replace(&text),
        "exclude" => do_exclude(&text),
        "validate" | "check" => do_validate(&text),
        _ => do_info(&text, &filename),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn strip_comment(s: &str) -> &str {
    if let Some(idx) = s.find("//") {
        s[..idx].trim()
    } else {
        s.trim()
    }
}

fn comment_text(s: &str) -> Option<String> {
    s.find("//")
        .map(|idx| s[idx + 2..].trim().to_string())
        .filter(|t| !t.is_empty())
}

fn split_module_version(s: &str) -> (String, String) {
    let parts: Vec<&str> = s.splitn(2, char::is_whitespace).collect();
    let module = parts[0].trim().to_string();
    let version = parts.get(1).map(|v| v.trim().to_string()).unwrap_or_default();
    (module, version)
}

// ── Struct types ──────────────────────────────────────────────────────────────

struct GoRequire {
    module: String,
    version: String,
    indirect: bool,
}

struct GoReplace {
    old_module: String,
    old_version: String,
    new_path: String,
    new_version: String,
}

struct GoExclude {
    module: String,
    version: String,
}

// ── Parsers ───────────────────────────────────────────────────────────────────

fn parse_module(text: &str) -> Option<String> {
    for line in text.lines() {
        let t = strip_comment(line.trim());
        if let Some(rest) = t.strip_prefix("module ") {
            let m = rest.trim();
            if !m.is_empty() {
                return Some(m.to_string());
            }
        }
    }
    None
}

fn parse_go_version(text: &str) -> Option<String> {
    for line in text.lines() {
        let t = strip_comment(line.trim());
        if let Some(rest) = t.strip_prefix("go ") {
            let v = rest.trim();
            if !v.is_empty() && v.chars().next().map(|c| c.is_numeric()).unwrap_or(false) {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn parse_toolchain(text: &str) -> Option<String> {
    for line in text.lines() {
        let t = strip_comment(line.trim());
        if let Some(rest) = t.strip_prefix("toolchain ") {
            let v = rest.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn parse_require_line(s: &str) -> Option<GoRequire> {
    let indirect = s.contains("// indirect");
    let clean = strip_comment(s).trim();
    let (module, version) = split_module_version(clean);
    if module.is_empty() || version.is_empty() {
        return None;
    }
    Some(GoRequire { module, version, indirect })
}

fn parse_block<T, F>(text: &str, keyword: &str, parse_line: F) -> Vec<T>
where
    F: Fn(&str) -> Option<T>,
{
    let mut results = Vec::new();
    let mut in_block = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if !in_block {
            if trimmed == &format!("{} (", keyword) {
                in_block = true;
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix(&format!("{} (", keyword)) {
                // Inline opening paren with content
                in_block = true;
                let r = rest.trim().trim_end_matches(')').trim();
                if !r.is_empty() {
                    if let Some(v) = parse_line(r) {
                        results.push(v);
                    }
                }
                if trimmed.ends_with(')') {
                    in_block = false;
                }
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix(&format!("{} ", keyword)) {
                let r = rest.trim();
                if !r.is_empty() && !r.starts_with("//") {
                    if let Some(v) = parse_line(r) {
                        results.push(v);
                    }
                }
            }
        } else {
            if trimmed == ")" {
                in_block = false;
                continue;
            }
            // Pass full trimmed line so parse_line can check comments (e.g. // indirect)
            if !trimmed.is_empty() && !trimmed.starts_with("//") {
                if let Some(v) = parse_line(trimmed) {
                    results.push(v);
                }
            }
        }
    }
    results
}

fn parse_requires(text: &str) -> Vec<GoRequire> {
    parse_block(text, "require", parse_require_line)
}

fn parse_replace_line(s: &str) -> Option<GoReplace> {
    let parts: Vec<&str> = s.splitn(2, "=>").collect();
    if parts.len() != 2 {
        return None;
    }
    let (old_module, old_version) = split_module_version(parts[0].trim());
    let (new_path, new_version) = split_module_version(parts[1].trim());
    Some(GoReplace { old_module, old_version, new_path, new_version })
}

fn parse_replaces(text: &str) -> Vec<GoReplace> {
    parse_block(text, "replace", parse_replace_line)
}

fn parse_exclude_line(s: &str) -> Option<GoExclude> {
    let (module, version) = split_module_version(s);
    if module.is_empty() {
        return None;
    }
    Some(GoExclude { module, version })
}

fn parse_excludes(text: &str) -> Vec<GoExclude> {
    parse_block(text, "exclude", parse_exclude_line)
}

fn parse_retracts(text: &str) -> Vec<(String, Option<String>)> {
    let mut retracts = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("retract ") {
            let cmt = comment_text(rest);
            let ver_part = strip_comment(rest).trim().trim_matches('[').trim_matches(']');
            for v in ver_part.split(',') {
                let v = v.trim();
                if !v.is_empty() {
                    retracts.push((v.to_string(), cmt.clone()));
                }
            }
        }
    }
    retracts
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn do_info(text: &str, filename: &str) -> Result<String, String> {
    let module = parse_module(text).unwrap_or_else(|| "(missing)".into());
    let go_ver = parse_go_version(text).unwrap_or_else(|| "(not set)".into());
    let toolchain = parse_toolchain(text);
    let requires = parse_requires(text);
    let replaces = parse_replaces(text);
    let excludes = parse_excludes(text);
    let retracts = parse_retracts(text);

    let direct: Vec<_> = requires.iter().filter(|r| !r.indirect).collect();
    let indirect: Vec<_> = requires.iter().filter(|r| r.indirect).collect();

    let mut out = String::new();
    out.push_str(&format!("go.mod — {}\n", filename));
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!("Module      : {}\n", module));
    out.push_str(&format!("Go version  : {}\n", go_ver));
    if let Some(tc) = toolchain {
        out.push_str(&format!("Toolchain   : {}\n", tc));
    }
    out.push('\n');
    out.push_str(&format!(
        "Dependencies: {} total  ({} direct / {} indirect)\n",
        requires.len(),
        direct.len(),
        indirect.len()
    ));
    if !replaces.is_empty() {
        out.push_str(&format!("Replaces    : {}\n", replaces.len()));
    }
    if !excludes.is_empty() {
        out.push_str(&format!("Excludes    : {}\n", excludes.len()));
    }
    if !retracts.is_empty() {
        out.push_str(&format!("Retracts    : {}\n", retracts.len()));
    }

    if !direct.is_empty() {
        out.push_str("\nDirect Dependencies\n");
        out.push_str(&"─".repeat(60));
        out.push('\n');
        let w = direct.iter().map(|r| r.module.len()).max().unwrap_or(20).min(60);
        for r in &direct {
            out.push_str(&format!("  {:<w$}  {}\n", r.module, r.version, w = w));
        }
    }

    Ok(out)
}

fn do_require(text: &str, args: &Value) -> Result<String, String> {
    let filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let indirect_only = args.get("indirect").and_then(|v| v.as_bool());

    let requires = parse_requires(text);
    let filtered: Vec<_> = requires
        .iter()
        .filter(|r| {
            let name_ok = filter.is_empty() || r.module.to_lowercase().contains(&filter);
            let ind_ok = match indirect_only {
                Some(true) => r.indirect,
                Some(false) => !r.indirect,
                None => true,
            };
            name_ok && ind_ok
        })
        .collect();

    if filtered.is_empty() {
        return Ok("No matching dependencies.\n".into());
    }

    let mut out = String::new();
    out.push_str(&format!("Dependencies ({})\n", filtered.len()));
    out.push_str(&"─".repeat(80));
    out.push('\n');

    let direct: Vec<_> = filtered.iter().filter(|r| !r.indirect).collect();
    let indirect: Vec<_> = filtered.iter().filter(|r| r.indirect).collect();

    if !direct.is_empty() {
        out.push_str(&format!("\nDirect ({})\n", direct.len()));
        out.push_str(&"─".repeat(50));
        out.push('\n');
        let w = direct.iter().map(|r| r.module.len()).max().unwrap_or(20).min(60);
        for r in direct {
            out.push_str(&format!("  {:<w$}  {}\n", r.module, r.version, w = w));
        }
    }

    if !indirect.is_empty() {
        out.push_str(&format!("\nIndirect ({})\n", indirect.len()));
        out.push_str(&"─".repeat(50));
        out.push('\n');
        let w = indirect.iter().map(|r| r.module.len()).max().unwrap_or(20).min(60);
        for r in indirect {
            out.push_str(&format!(
                "  {:<w$}  {}  // indirect\n",
                r.module,
                r.version,
                w = w
            ));
        }
    }

    Ok(out)
}

fn do_replace(text: &str) -> Result<String, String> {
    let replaces = parse_replaces(text);
    if replaces.is_empty() {
        return Ok("No replace directives found.\n".into());
    }

    let mut out = String::new();
    out.push_str(&format!("Replace Directives ({})\n", replaces.len()));
    out.push_str(&"─".repeat(80));
    out.push('\n');

    let local: Vec<_> = replaces
        .iter()
        .filter(|r| r.new_path.starts_with('.') || r.new_path.starts_with('/'))
        .collect();
    let remote: Vec<_> = replaces
        .iter()
        .filter(|r| !r.new_path.starts_with('.') && !r.new_path.starts_with('/'))
        .collect();

    if !remote.is_empty() {
        out.push_str(&format!("\nModule Replacements ({})\n", remote.len()));
        out.push_str(&"─".repeat(50));
        out.push('\n');
        for r in &remote {
            let old = if r.old_version.is_empty() {
                r.old_module.clone()
            } else {
                format!("{} {}", r.old_module, r.old_version)
            };
            let new = if r.new_version.is_empty() {
                r.new_path.clone()
            } else {
                format!("{} {}", r.new_path, r.new_version)
            };
            out.push_str(&format!("  {} => {}\n", old, new));
        }
    }

    if !local.is_empty() {
        out.push_str(&format!("\nLocal Path Replacements ({})  [CI risk]\n", local.len()));
        out.push_str(&"─".repeat(50));
        out.push('\n');
        for r in &local {
            let old = if r.old_version.is_empty() {
                r.old_module.clone()
            } else {
                format!("{} {}", r.old_module, r.old_version)
            };
            out.push_str(&format!("  {} => {}\n", old, r.new_path));
        }
    }

    Ok(out)
}

fn do_exclude(text: &str) -> Result<String, String> {
    let excludes = parse_excludes(text);
    if excludes.is_empty() {
        return Ok("No exclude directives found.\n".into());
    }
    let mut out = String::new();
    out.push_str(&format!("Excluded Versions ({})\n", excludes.len()));
    out.push_str(&"─".repeat(60));
    out.push('\n');
    let w = excludes
        .iter()
        .map(|e| e.module.len())
        .max()
        .unwrap_or(20)
        .min(60);
    for e in &excludes {
        out.push_str(&format!("  {:<w$}  {}\n", e.module, e.version, w = w));
    }
    Ok(out)
}

fn do_validate(text: &str) -> Result<String, String> {
    let mut issues = Vec::new();

    if parse_module(text).is_none() {
        issues.push("CRITICAL: Missing `module` directive".to_string());
    }

    let go_ver = parse_go_version(text);
    if go_ver.is_none() {
        issues.push(
            "WARNING: Missing `go` version directive — add `go 1.21` or newer".to_string(),
        );
    } else if let Some(ref v) = go_ver {
        let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
        if parts.len() >= 2 && parts[0] == 1 && parts[1] < 16 {
            issues.push(format!(
                "WARNING: go {} is very old — consider upgrading to 1.21+",
                v
            ));
        }
    }

    let requires = parse_requires(text);
    let replaces = parse_replaces(text);
    let retracts = parse_retracts(text);

    // Direct deps using pseudo-versions
    for r in requires.iter().filter(|r| !r.indirect && r.version.contains("v0.0.0-")) {
        issues.push(format!(
            "WARNING: Direct dep {} uses pseudo-version {} — prefer a tagged release",
            r.module, r.version
        ));
    }

    // Local replace directives
    for r in replaces
        .iter()
        .filter(|r| r.new_path.starts_with('.') || r.new_path.starts_with('/'))
    {
        issues.push(format!(
            "WARNING: replace {} => {} uses a local path — will break in CI unless the path exists",
            r.old_module, r.new_path
        ));
    }

    // Retracted versions in use
    for req in &requires {
        for (ver, _) in &retracts {
            if req.version == *ver {
                issues.push(format!(
                    "WARNING: {} {} is retracted — update to a newer version",
                    req.module, req.version
                ));
            }
        }
    }

    // Multiple major versions of same base module
    let mut base_map: std::collections::HashMap<String, Vec<&str>> =
        std::collections::HashMap::new();
    for r in &requires {
        let base = if let Some(slash) = r.module.rfind('/') {
            let suffix = &r.module[slash + 1..];
            if suffix.len() > 1
                && suffix.starts_with('v')
                && suffix[1..].parse::<u32>().is_ok()
            {
                r.module[..slash].to_string()
            } else {
                r.module.clone()
            }
        } else {
            r.module.clone()
        };
        base_map.entry(base).or_default().push(r.module.as_str());
    }
    for (base, mods) in &base_map {
        if mods.len() > 1 {
            issues.push(format!(
                "INFO: Multiple major versions of {}: {}",
                base,
                mods.join(", ")
            ));
        }
    }

    let module = parse_module(text);
    let mut out = String::new();
    if issues.is_empty() {
        out.push_str("VALID — no issues detected\n");
        if let Some(m) = module {
            out.push_str(&format!("Module : {}\n", m));
        }
        if let Some(v) = go_ver {
            out.push_str(&format!("Go     : {}\n", v));
        }
        out.push_str(&format!(
            "Deps   : {} ({} direct)\n",
            requires.len(),
            requires.iter().filter(|r| !r.indirect).count()
        ));
    } else {
        let critical = issues.iter().filter(|i| i.starts_with("CRITICAL")).count();
        let warnings = issues.iter().filter(|i| i.starts_with("WARNING")).count();
        let verdict = if critical > 0 {
            "INVALID"
        } else if warnings > 0 {
            "WARNINGS"
        } else {
            "VALID"
        };
        out.push_str(&format!("{} — {} issue(s)\n", verdict, issues.len()));
        if critical > 0 {
            out.push_str(&format!("  {} critical\n", critical));
        }
        if warnings > 0 {
            out.push_str(&format!("  {} warnings\n", warnings));
        }
        out.push('\n');
        for issue in &issues {
            out.push_str(&format!("• {}\n", issue));
        }
    }

    Ok(out)
}
