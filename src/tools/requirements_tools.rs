use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "description": "info (default) | list | validate | extras | export"
            },
            "requirements": {
                "type": "string",
                "description": "Inline requirements.txt or pyproject.toml content"
            },
            "file": {
                "type": "string",
                "description": "Path to requirements.txt or pyproject.toml"
            },
            "group": {
                "type": "string",
                "description": "For list/extras: filter by group name (e.g. dev, test, all)"
            },
            "filter": {
                "type": "string",
                "description": "Substring filter on package name"
            }
        }
    })
}

#[derive(Debug, Clone)]
struct PkgEntry {
    name: String,
    extras: Vec<String>,
    specifiers: String,
    marker: String,
    url: String,
    editable: bool,
    group: String,
}

impl PkgEntry {
    fn pin_type(&self) -> &'static str {
        if self.editable {
            return "editable";
        }
        if !self.url.is_empty() {
            return "url";
        }
        if self.specifiers.is_empty() {
            return "unpinned";
        }
        if self.specifiers.contains("==") {
            return "pinned";
        }
        "loose"
    }
}

#[derive(Debug)]
enum Format {
    Requirements,
    PyprojectPep621,
    PyprojectPoetry,
    Unknown,
}

fn detect_format(text: &str) -> Format {
    if text.contains("[project]") && text.contains("dependencies") {
        return Format::PyprojectPep621;
    }
    if text.contains("[tool.poetry") {
        return Format::PyprojectPoetry;
    }
    // requirements.txt style: lines with == or -r or package names
    let req_lines = text
        .lines()
        .filter(|l| {
            let l = l.trim();
            !l.is_empty()
                && !l.starts_with('#')
                && (l.contains("==")
                    || l.starts_with('-')
                    || l.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false))
        })
        .count();
    if req_lines > 0 {
        return Format::Requirements;
    }
    Format::Unknown
}

// Parse a single requirement line from requirements.txt
fn parse_req_line(line: &str, group: &str) -> Option<PkgEntry> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    // Skip options like -r, --index-url, etc. (but keep -e)
    if line.starts_with('-') && !line.starts_with("-e ") && !line.starts_with("--editable") {
        return None;
    }

    let editable = line.starts_with("-e ") || line.starts_with("--editable");
    let rest = if editable {
        line.trim_start_matches("--editable")
            .trim_start_matches("-e")
            .trim()
    } else {
        line
    };

    // Split on environment marker (;)
    let (pkg_part, marker) = if let Some(idx) = rest.find(';') {
        (rest[..idx].trim(), rest[idx + 1..].trim().to_string())
    } else {
        (rest, String::new())
    };

    // URL deps: contains @
    if pkg_part.contains(" @ ") {
        let parts: Vec<&str> = pkg_part.splitn(2, " @ ").collect();
        let name = parse_pkg_name(parts[0]).unwrap_or_default();
        return Some(PkgEntry {
            name,
            extras: Vec::new(),
            specifiers: String::new(),
            url: parts.get(1).unwrap_or(&"").to_string(),
            marker,
            editable,
            group: group.to_string(),
        });
    }

    // Standard: name[extras]specifiers
    let (name_extras, specifiers) = split_name_specifiers(pkg_part);
    let (name, extras) = parse_name_extras(&name_extras);

    if name.is_empty() {
        return None;
    }

    Some(PkgEntry {
        name,
        extras,
        specifiers,
        marker,
        url: String::new(),
        editable,
        group: group.to_string(),
    })
}

fn split_name_specifiers(s: &str) -> (String, String) {
    // Specifiers start at first occurrence of ==, !=, >=, <=, ~=, >, <
    let ops = ["==", "!=", ">=", "<=", "~=", ">", "<"];
    let mut spec_start = s.len();
    // Find earliest specifier that's not inside []
    let mut in_bracket = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            _ => {
                if !in_bracket {
                    let remaining = &s[i..];
                    for op in &ops {
                        if remaining.starts_with(op) {
                            spec_start = i;
                            break;
                        }
                    }
                    if spec_start < s.len() {
                        break;
                    }
                }
            }
        }
        i += 1;
    }
    (s[..spec_start].to_string(), s[spec_start..].to_string())
}

fn parse_name_extras(s: &str) -> (String, Vec<String>) {
    let s = s.trim();
    if let Some(bracket_start) = s.find('[') {
        let name = s[..bracket_start].trim().to_string();
        let bracket_end = s.rfind(']').unwrap_or(s.len() - 1);
        let extras_str = &s[bracket_start + 1..bracket_end];
        let extras: Vec<String> = extras_str
            .split(',')
            .map(|e| e.trim().to_string())
            .filter(|e| !e.is_empty())
            .collect();
        (name, extras)
    } else {
        (s.to_string(), Vec::new())
    }
}

fn parse_pkg_name(s: &str) -> Option<String> {
    let s = s.trim();
    // Name is letters, digits, -, _, .
    let name: String = s
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn parse_requirements_txt(text: &str) -> Vec<PkgEntry> {
    text.lines()
        .filter_map(|l| parse_req_line(l, "main"))
        .collect()
}

// Parse pyproject.toml PEP 621 format
fn parse_pep621(text: &str) -> Vec<PkgEntry> {
    let mut entries = Vec::new();
    let mut in_project_deps = false;
    let mut in_optional = false;
    let mut current_extra = String::new();

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed == "[project.dependencies]" || trimmed == "dependencies = [" {
            // This is inside [project] block
        }

        // Detect section headers
        if trimmed.starts_with('[') {
            in_project_deps = false;
            in_optional = false;
            current_extra.clear();

            if trimmed == "[project]" {
                // handled below with key detection
            } else if trimmed == "[project.optional-dependencies]" {
                in_optional = true;
            }
            continue;
        }

        if in_optional {
            // extra group header: name = [
            if trimmed.ends_with(" = [") || trimmed.contains(" = [") {
                let key = trimmed.split('=').next().unwrap_or("").trim().to_string();
                current_extra = key;
                continue;
            }
        }

        // Parse dependency lines inside [project] dependencies = [...] block
        if trimmed.starts_with("dependencies") && trimmed.contains('=') {
            in_project_deps = true;
            continue;
        }

        if in_project_deps || in_optional {
            // Lines like "    \"requests>=2.0\","
            if trimmed == "]" {
                in_project_deps = false;
                if !current_extra.is_empty() {
                    current_extra.clear();
                }
                continue;
            }
            let dep_str = trimmed
                .trim_matches(',')
                .trim_matches('"')
                .trim_matches('\'')
                .trim();
            if !dep_str.is_empty() && !dep_str.starts_with('#') {
                let group = if current_extra.is_empty() {
                    "main".to_string()
                } else {
                    current_extra.clone()
                };
                if let Some(entry) = parse_req_line(dep_str, &group) {
                    entries.push(entry);
                }
            }
        }
    }
    entries
}

// Parse pyproject.toml Poetry format
fn parse_poetry(text: &str) -> Vec<PkgEntry> {
    let mut entries = Vec::new();
    let mut current_group = String::new();
    let mut in_deps = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') {
            in_deps = false;
            if trimmed == "[tool.poetry.dependencies]" {
                current_group = "main".to_string();
                in_deps = true;
            } else if trimmed.starts_with("[tool.poetry.dev-dependencies]") {
                current_group = "dev".to_string();
                in_deps = true;
            } else if trimmed.starts_with("[tool.poetry.group.") {
                // [tool.poetry.group.X.dependencies]
                let g: String = trimmed
                    .trim_start_matches("[tool.poetry.group.")
                    .chars()
                    .take_while(|c| *c != '.')
                    .collect();
                current_group = g;
                in_deps = trimmed.ends_with(".dependencies]");
            }
            continue;
        }

        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Poetry format: name = "version" or name = {version = "...", extras = [...]}
        if let Some(eq_idx) = trimmed.find('=') {
            let name = trimmed[..eq_idx].trim().to_string();
            if name == "python" {
                continue;
            }
            let val = trimmed[eq_idx + 1..].trim();

            // Inline table: { version = "...", ... }
            if val.starts_with('{') {
                let specifiers = extract_poetry_version(val);
                let extras = extract_poetry_extras(val);
                let optional = val.contains("optional = true") || val.contains("optional=true");
                let group = if optional {
                    format!("{}-optional", current_group)
                } else {
                    current_group.clone()
                };
                entries.push(PkgEntry {
                    name,
                    extras,
                    specifiers,
                    marker: String::new(),
                    url: String::new(),
                    editable: false,
                    group,
                });
            } else {
                // Simple string version: name = "^1.0"
                let ver = val.trim_matches('"').trim_matches('\'').to_string();
                let specifiers = normalize_poetry_version(&ver);
                entries.push(PkgEntry {
                    name,
                    extras: Vec::new(),
                    specifiers,
                    marker: String::new(),
                    url: String::new(),
                    editable: false,
                    group: current_group.clone(),
                });
            }
        }
    }
    entries
}

fn extract_poetry_version(val: &str) -> String {
    // { version = "^1.0", ... }
    if let Some(start) = val.find("version") {
        let after = &val[start + 7..];
        if let Some(eq) = after.find('=') {
            let v = after[eq + 1..].trim();
            let v = v.trim_start_matches('"').trim_start_matches('\'');
            let end = v.find('"').or_else(|| v.find('\'')).unwrap_or(v.len());
            return normalize_poetry_version(&v[..end]);
        }
    }
    String::new()
}

fn extract_poetry_extras(val: &str) -> Vec<String> {
    if let Some(start) = val.find("extras") {
        let after = &val[start + 6..];
        if let Some(bracket) = after.find('[') {
            let inner = &after[bracket + 1..];
            if let Some(end) = inner.find(']') {
                return inner[..end]
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }
    Vec::new()
}

fn normalize_poetry_version(ver: &str) -> String {
    // Convert Poetry ^ and ~ to PEP 440 approximately
    let ver = ver.trim();
    if ver == "*" || ver.is_empty() {
        return String::new();
    }
    ver.to_string()
}

fn load_text(args: &Value) -> Result<String, String> {
    if let Some(t) = args
        .get("requirements")
        .or_else(|| args.get("text"))
        .and_then(|v| v.as_str())
    {
        return Ok(t.to_string());
    }
    if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(f).map_err(|e| format!("Cannot read '{}': {}", f, e));
    }
    Err("Provide 'requirements' (inline text) or 'file' (path to requirements.txt or pyproject.toml).".to_string())
}

fn parse_all(text: &str) -> (Format, Vec<PkgEntry>) {
    let fmt = detect_format(text);
    let entries = match &fmt {
        Format::PyprojectPoetry => parse_poetry(text),
        Format::PyprojectPep621 => parse_pep621(text),
        Format::Requirements | Format::Unknown => parse_requirements_txt(text),
    };
    (fmt, entries)
}

fn format_name(fmt: &Format) -> &'static str {
    match fmt {
        Format::Requirements => "requirements.txt",
        Format::PyprojectPep621 => "pyproject.toml (PEP 621)",
        Format::PyprojectPoetry => "pyproject.toml (Poetry)",
        Format::Unknown => "unknown",
    }
}

fn do_info(text: &str) -> String {
    let (fmt, entries) = parse_all(text);
    if entries.is_empty() {
        return "No packages found.".to_string();
    }

    let total = entries.len();
    let pinned = entries.iter().filter(|e| e.pin_type() == "pinned").count();
    let loose = entries.iter().filter(|e| e.pin_type() == "loose").count();
    let unpinned = entries
        .iter()
        .filter(|e| e.pin_type() == "unpinned")
        .count();
    let editable = entries.iter().filter(|e| e.editable).count();
    let url_deps = entries.iter().filter(|e| !e.url.is_empty()).count();
    let with_extras = entries.iter().filter(|e| !e.extras.is_empty()).count();
    let with_markers = entries.iter().filter(|e| !e.marker.is_empty()).count();

    // Group summary
    let mut groups: Vec<String> = Vec::new();
    for e in &entries {
        if !groups.contains(&e.group) {
            groups.push(e.group.clone());
        }
    }

    let mut out = String::new();
    out.push_str(&format!("Format      : {}\n", format_name(&fmt)));
    out.push_str(&format!(
        "Total       : {} package{}\n",
        total,
        if total == 1 { "" } else { "s" }
    ));
    out.push_str(&format!(
        "Pinned      : {}  Loose: {}  Unpinned: {}\n",
        pinned, loose, unpinned
    ));
    if editable > 0 {
        out.push_str(&format!("Editable    : {}\n", editable));
    }
    if url_deps > 0 {
        out.push_str(&format!("URL deps    : {}\n", url_deps));
    }
    if with_extras > 0 {
        out.push_str(&format!("With extras : {}\n", with_extras));
    }
    if with_markers > 0 {
        out.push_str(&format!("With markers: {}\n", with_markers));
    }
    if groups.len() > 1 {
        out.push_str(&format!("Groups      : {}\n", groups.join(", ")));
    }

    out.push_str("\nPackages:\n");
    let name_w = entries
        .iter()
        .map(|e| e.name.len())
        .max()
        .unwrap_or(10)
        .max(10);
    out.push_str(&format!(
        "{:<width$}  {:<8}  {}\n",
        "Name",
        "Type",
        "Specifiers",
        width = name_w
    ));
    out.push_str(&format!("{}\n", "-".repeat(name_w + 24)));

    for e in &entries {
        let pin = e.pin_type();
        let spec = if e.editable {
            "[editable]".to_string()
        } else if !e.url.is_empty() {
            format!("@ {}", &e.url[..e.url.len().min(40)])
        } else if e.specifiers.is_empty() {
            "(any)".to_string()
        } else {
            e.specifiers.clone()
        };
        let name_str = if e.extras.is_empty() {
            e.name.clone()
        } else {
            format!("{}[{}]", e.name, e.extras.join(","))
        };
        out.push_str(&format!(
            "{:<width$}  {:<8}  {}\n",
            name_str,
            pin,
            spec,
            width = name_w
        ));
    }

    out
}

fn do_list(args: &Value, text: &str) -> String {
    let (fmt, entries) = parse_all(text);
    if entries.is_empty() {
        return "No packages found.".to_string();
    }

    let group_filter = args.get("group").and_then(|v| v.as_str()).unwrap_or("all");
    let name_filter = args.get("filter").and_then(|v| v.as_str()).unwrap_or("");

    let filtered: Vec<&PkgEntry> = entries
        .iter()
        .filter(|e| {
            (group_filter == "all" || e.group == group_filter)
                && (name_filter.is_empty()
                    || e.name.to_lowercase().contains(&name_filter.to_lowercase()))
        })
        .collect();

    if filtered.is_empty() {
        return format!(
            "No packages match (group={}, filter={}).",
            group_filter, name_filter
        );
    }

    let name_w = filtered
        .iter()
        .map(|e| e.name.len() + e.extras.iter().map(|x| x.len() + 1).sum::<usize>())
        .max()
        .unwrap_or(10)
        .max(10);

    let mut out = format!("Format: {}\n\n", format_name(&fmt));
    out.push_str(&format!(
        "{:<width$}  {:<8}  {:<6}  {}\n",
        "Package",
        "Type",
        "Group",
        "Specifiers",
        width = name_w
    ));
    out.push_str(&format!("{}\n", "-".repeat(name_w + 32)));

    for e in &filtered {
        let name_str = if e.extras.is_empty() {
            e.name.clone()
        } else {
            format!("{}[{}]", e.name, e.extras.join(","))
        };
        let spec = if e.editable {
            "[editable]".to_string()
        } else if !e.url.is_empty() {
            format!("@ {}", &e.url[..e.url.len().min(30)])
        } else if e.specifiers.is_empty() {
            "(any)".to_string()
        } else {
            e.specifiers.clone()
        };
        out.push_str(&format!(
            "{:<width$}  {:<8}  {:<6}  {}\n",
            name_str,
            e.pin_type(),
            e.group,
            spec,
            width = name_w
        ));
        if !e.marker.is_empty() {
            out.push_str(&format!(
                "{:<width$}  ; {}\n",
                "",
                e.marker,
                width = name_w + 18
            ));
        }
    }

    out
}

fn do_extras(text: &str) -> String {
    let (fmt, entries) = parse_all(text);

    // Collect all groups
    let mut groups: Vec<String> = Vec::new();
    for e in &entries {
        if !groups.contains(&e.group) {
            groups.push(e.group.clone());
        }
    }

    if groups.len() <= 1 && groups.get(0).map(|s| s.as_str()) == Some("main") {
        // Check for package-level extras
        let with_extras: Vec<&PkgEntry> = entries.iter().filter(|e| !e.extras.is_empty()).collect();
        if with_extras.is_empty() {
            return format!(
                "Format: {}\n\nNo dependency groups or package extras found.",
                format_name(&fmt)
            );
        }
        let mut out = format!("Format: {}\n\nPackage extras:\n", format_name(&fmt));
        for e in with_extras {
            out.push_str(&format!("  {}[{}]\n", e.name, e.extras.join(", ")));
        }
        return out;
    }

    let mut out = format!("Format: {}\n\nDependency groups:\n", format_name(&fmt));
    for group in &groups {
        let pkgs: Vec<&PkgEntry> = entries.iter().filter(|e| &e.group == group).collect();
        out.push_str(&format!(
            "\n[{}] ({} package{})\n",
            group,
            pkgs.len(),
            if pkgs.len() == 1 { "" } else { "s" }
        ));
        for p in pkgs {
            let spec = if p.specifiers.is_empty() {
                "(any)".to_string()
            } else {
                p.specifiers.clone()
            };
            let extras_str = if p.extras.is_empty() {
                String::new()
            } else {
                format!("[{}]", p.extras.join(","))
            };
            out.push_str(&format!("  {}{} {}\n", p.name, extras_str, spec));
        }
    }

    out
}

fn do_validate(text: &str) -> String {
    let (fmt, entries) = parse_all(text);

    let mut issues: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    if entries.is_empty() {
        return format!(
            "Format: {}\n\nNo packages found — empty or unrecognized file.\n",
            format_name(&fmt)
        );
    }

    // Check for unpinned packages (could be a problem in production)
    let unpinned: Vec<&str> = entries
        .iter()
        .filter(|e| e.pin_type() == "unpinned" && e.group == "main")
        .map(|e| e.name.as_str())
        .collect();
    if !unpinned.is_empty() {
        warnings.push(format!(
            "Unpinned main deps (no version specifier): {}",
            unpinned.join(", ")
        ));
    }

    // Check for loose specifiers in main deps
    let loose: Vec<&str> = entries
        .iter()
        .filter(|e| e.pin_type() == "loose" && e.group == "main")
        .map(|e| e.name.as_str())
        .collect();
    if !loose.is_empty() {
        warnings.push(format!(
            "Loose main deps (>=, ~= but not pinned ==): {}",
            loose.join(", ")
        ));
    }

    // Check for duplicate package names
    let mut seen_names: Vec<String> = Vec::new();
    let mut duplicates: Vec<String> = Vec::new();
    for e in &entries {
        let normalized = e.name.to_lowercase().replace('-', "_");
        if seen_names.contains(&normalized) && !duplicates.contains(&e.name) {
            duplicates.push(e.name.clone());
        } else {
            seen_names.push(normalized);
        }
    }
    if !duplicates.is_empty() {
        issues.push(format!("Duplicate packages: {}", duplicates.join(", ")));
    }

    // Editable deps in non-dev groups
    let editable_main: Vec<&str> = entries
        .iter()
        .filter(|e| e.editable && e.group == "main")
        .map(|e| e.name.as_str())
        .collect();
    if !editable_main.is_empty() {
        issues.push(format!(
            "Editable installs in main group (not suitable for production): {}",
            editable_main.join(", ")
        ));
    }

    // URL deps
    let url_deps: Vec<&str> = entries
        .iter()
        .filter(|e| !e.url.is_empty())
        .map(|e| e.name.as_str())
        .collect();
    if !url_deps.is_empty() {
        warnings.push(format!(
            "URL/VCS dependencies (not reproducible without network): {}",
            url_deps.join(", ")
        ));
    }

    // Mixed pinning in main group
    let main_count = entries.iter().filter(|e| e.group == "main").count();
    let main_pinned = entries
        .iter()
        .filter(|e| e.group == "main" && e.pin_type() == "pinned")
        .count();
    if main_count > 0 && main_pinned > 0 && main_pinned < main_count {
        warnings.push(format!(
            "Mixed pinning in main group: {} of {} packages have exact pins — inconsistent lockfile",
            main_pinned, main_count
        ));
    }

    let verdict = if !issues.is_empty() {
        "INVALID"
    } else if !warnings.is_empty() {
        "WARNINGS"
    } else {
        "VALID"
    };

    let mut out = format!("Format : {}\n", format_name(&fmt));
    out.push_str(&format!("Verdict: {}\n", verdict));
    out.push_str(&format!(
        "Packages: {} total ({} main)\n\n",
        entries.len(),
        entries.iter().filter(|e| e.group == "main").count()
    ));

    if issues.is_empty() && warnings.is_empty() {
        out.push_str("No issues found.\n");
    } else {
        for issue in &issues {
            out.push_str(&format!("[ERROR]   {}\n", issue));
        }
        for w in &warnings {
            out.push_str(&format!("[WARN]    {}\n", w));
        }
    }

    out
}

fn do_export(text: &str) -> String {
    let (fmt, entries) = parse_all(text);

    let mut out = format!("# Exported from {} format\n", format_name(&fmt));
    for e in &entries {
        if e.editable {
            out.push_str(&format!(
                "-e {}\n",
                if e.url.is_empty() { &e.name } else { &e.url }
            ));
            continue;
        }
        let extras_str = if e.extras.is_empty() {
            String::new()
        } else {
            format!("[{}]", e.extras.join(","))
        };
        if !e.url.is_empty() {
            out.push_str(&format!("{}{} @ {}", e.name, extras_str, e.url));
        } else {
            out.push_str(&format!("{}{}{}", e.name, extras_str, e.specifiers));
        }
        if !e.marker.is_empty() {
            out.push_str(&format!(" ; {}", e.marker));
        }
        out.push('\n');
    }

    out
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let result = match action {
        "info" => do_info(&text),
        "list" => do_list(args, &text),
        "validate" => do_validate(&text),
        "extras" => do_extras(&text),
        "export" => do_export(&text),
        other => {
            return Err(format!(
                "Unknown action '{}'. Valid: info, list, validate, extras, export.",
                other
            ))
        }
    };

    Ok(result)
}
