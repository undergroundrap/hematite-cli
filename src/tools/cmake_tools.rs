use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["info", "targets", "options", "deps", "validate"],
                "description": "Action to perform (default: info)"
            },
            "file": { "type": "string", "description": "Path to CMakeLists.txt or .cmake file" },
            "text": { "type": "string", "description": "Inline CMake script content" },
            "cmake": { "type": "string", "description": "Alias for text" }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => do_info(args),
        "targets" => do_targets(args),
        "options" => do_options(args),
        "deps" => do_deps(args),
        "validate" => do_validate(args),
        other => Err(format!(
            "Unknown action '{other}'. Choose: info, targets, options, deps, validate"
        )),
    }
}

// ── input loading ─────────────────────────────────────────────────────────────

fn load_text(args: &Value) -> Result<String, String> {
    if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(f).map_err(|e| format!("Cannot read '{f}': {e}"));
    }
    args.get("text")
        .or_else(|| args.get("cmake"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Provide 'file' or 'text'.".into())
}

// ── CMake tokenizer ───────────────────────────────────────────────────────────

/// Strip line comments (# ...) and return logical lines with their content.
fn strip_comments(text: &str) -> String {
    text.lines()
        .map(|line| {
            // Strip # comments not inside quotes
            let mut in_q = false;
            let mut result = String::new();
            let chars = line.chars();
            for c in chars {
                match c {
                    '"' => {
                        in_q = !in_q;
                        result.push(c);
                    }
                    '#' if !in_q => break,
                    _ => result.push(c),
                }
            }
            result
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract the command name and arguments from a CMake call.
/// Returns (command_lower, Vec<arg>) for the first call starting at `text`.
fn parse_call(text: &str) -> Option<(String, Vec<String>)> {
    let text = text.trim();
    let paren = text.find('(')?;
    let name = text[..paren].trim().to_lowercase();
    let rest = &text[paren + 1..];
    // Find matching close paren (depth-aware)
    let mut depth = 1usize;
    let mut end = 0;
    for (i, c) in rest.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let inner = &rest[..end];
    let args = tokenize_args(inner);
    Some((name, args))
}

/// Split CMake argument list by whitespace, treating quoted strings as single tokens.
fn tokenize_args(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    for c in s.chars() {
        match c {
            '"' => in_q = !in_q,
            ' ' | '\t' | '\n' | '\r' if !in_q => {
                let t = cur.trim().to_string();
                if !t.is_empty() {
                    tokens.push(t);
                }
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    let t = cur.trim().to_string();
    if !t.is_empty() {
        tokens.push(t);
    }
    tokens
}

/// Collect all CMake command calls from the script text.
fn collect_calls(text: &str) -> Vec<(String, Vec<String>)> {
    let cleaned = strip_comments(text);
    let mut calls = Vec::new();
    let mut rest = cleaned.as_str();
    // Find each `identifier(` sequence
    while let Some(pos) = find_command_start(rest) {
        let slice = &rest[pos..];
        if let Some(call) = parse_call(slice) {
            // advance past this call
            if let Some(paren) = slice.find('(') {
                let after_open = &slice[paren + 1..];
                let skip = find_end_paren(after_open).unwrap_or(after_open.len());
                rest = &slice[paren + 1 + skip + 1..];
                calls.push(call);
            } else {
                rest = &rest[pos + 1..];
            }
        } else {
            rest = &rest[pos + 1..];
        }
    }
    calls
}

fn find_command_start(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Skip whitespace
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        // Skip if not an identifier start
        if !bytes[i].is_ascii_alphabetic() && bytes[i] != b'_' {
            i += 1;
            continue;
        }
        // Read identifier
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        // Check for opening paren
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'(' {
            return Some(start);
        }
    }
    None
}

fn find_end_paren(text: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

// ── CMake data extraction ─────────────────────────────────────────────────────

struct CmakeInfo {
    min_version: Option<String>,
    project_name: Option<String>,
    project_version: Option<String>,
    languages: Vec<String>,
    executables: Vec<(String, Vec<String>)>, // (name, sources)
    libraries: Vec<(String, String, Vec<String>)>, // (name, type, sources)
    options: Vec<(String, String, String)>,  // (var, description, default)
    set_vars: Vec<(String, String)>,
    subdirs: Vec<String>,
    find_pkgs: Vec<String>,
    includes: Vec<String>, // include() calls
    custom_targets: Vec<String>,
    install_targets: Vec<String>,
}

impl CmakeInfo {
    fn new() -> Self {
        CmakeInfo {
            min_version: None,
            project_name: None,
            project_version: None,
            languages: Vec::new(),
            executables: Vec::new(),
            libraries: Vec::new(),
            options: Vec::new(),
            set_vars: Vec::new(),
            subdirs: Vec::new(),
            find_pkgs: Vec::new(),
            includes: Vec::new(),
            custom_targets: Vec::new(),
            install_targets: Vec::new(),
        }
    }
}

fn parse_cmake(text: &str) -> CmakeInfo {
    let calls = collect_calls(text);
    let mut info = CmakeInfo::new();
    for (cmd, args) in &calls {
        match cmd.as_str() {
            "cmake_minimum_required" => {
                // cmake_minimum_required(VERSION X.Y)
                if let Some(pos) = args.iter().position(|a| a.eq_ignore_ascii_case("VERSION")) {
                    if let Some(ver) = args.get(pos + 1) {
                        info.min_version = Some(ver.clone());
                    }
                }
            }
            "project" => {
                if let Some(name) = args.first() {
                    info.project_name = Some(name.clone());
                }
                if let Some(pos) = args.iter().position(|a| a.eq_ignore_ascii_case("VERSION")) {
                    if let Some(ver) = args.get(pos + 1) {
                        info.project_version = Some(ver.clone());
                    }
                }
                if let Some(pos) = args
                    .iter()
                    .position(|a| a.eq_ignore_ascii_case("LANGUAGES"))
                {
                    info.languages.extend(args[pos + 1..].iter().cloned());
                }
            }
            "add_executable" => {
                if let Some(name) = args.first() {
                    let sources: Vec<String> = args[1..]
                        .iter()
                        .filter(|a| {
                            !a.eq_ignore_ascii_case("WIN32")
                                && !a.eq_ignore_ascii_case("MACOSX_BUNDLE")
                        })
                        .cloned()
                        .collect();
                    info.executables.push((name.clone(), sources));
                }
            }
            "add_library" => {
                if let Some(name) = args.first() {
                    let lib_type = args
                        .iter()
                        .skip(1)
                        .find(|a| {
                            matches!(
                                a.to_uppercase().as_str(),
                                "STATIC" | "SHARED" | "MODULE" | "INTERFACE" | "OBJECT" | "ALIAS"
                            )
                        })
                        .cloned()
                        .unwrap_or_else(|| "STATIC".into());
                    let sources: Vec<String> = args[1..]
                        .iter()
                        .filter(|a| {
                            !matches!(
                                a.to_uppercase().as_str(),
                                "STATIC" | "SHARED" | "MODULE" | "INTERFACE" | "OBJECT" | "ALIAS"
                            )
                        })
                        .cloned()
                        .collect();
                    info.libraries.push((name.clone(), lib_type, sources));
                }
            }
            "option" => {
                let var = args.first().cloned().unwrap_or_default();
                let desc = args.get(1).cloned().unwrap_or_default();
                let def = args.get(2).cloned().unwrap_or_else(|| "OFF".into());
                if !var.is_empty() {
                    info.options.push((var, desc, def));
                }
            }
            "set" => {
                if let Some(var) = args.first() {
                    let val = args.get(1).cloned().unwrap_or_default();
                    info.set_vars.push((var.clone(), val));
                }
            }
            "add_subdirectory" => {
                if let Some(dir) = args.first() {
                    info.subdirs.push(dir.clone());
                }
            }
            "find_package" => {
                if let Some(pkg) = args.first() {
                    info.find_pkgs.push(pkg.clone());
                }
            }
            "include" => {
                if let Some(f) = args.first() {
                    info.includes.push(f.clone());
                }
            }
            "add_custom_target" => {
                if let Some(name) = args.first() {
                    info.custom_targets.push(name.clone());
                }
            }
            "install" => {
                if let Some(pos) = args.iter().position(|a| a.eq_ignore_ascii_case("TARGETS")) {
                    let names: Vec<String> = args[pos + 1..]
                        .iter()
                        .take_while(|a| {
                            !a.eq_ignore_ascii_case("DESTINATION")
                                && !a.eq_ignore_ascii_case("RUNTIME")
                                && !a.eq_ignore_ascii_case("LIBRARY")
                                && !a.eq_ignore_ascii_case("ARCHIVE")
                        })
                        .cloned()
                        .collect();
                    info.install_targets.extend(names);
                }
            }
            _ => {}
        }
    }
    info
}

// ── actions ───────────────────────────────────────────────────────────────────

fn do_info(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let info = parse_cmake(&text);
    let mut out = String::new();
    out.push_str("CMake Project Info\n");
    out.push_str(&"─".repeat(40));
    out.push('\n');
    if let Some(v) = &info.min_version {
        out.push_str(&format!("Minimum CMake  : {v}\n"));
    }
    if let Some(n) = &info.project_name {
        out.push_str(&format!("Project Name   : {n}\n"));
    }
    if let Some(v) = &info.project_version {
        out.push_str(&format!("Project Version: {v}\n"));
    }
    if !info.languages.is_empty() {
        out.push_str(&format!("Languages      : {}\n", info.languages.join(", ")));
    }
    out.push_str(&format!("Executables    : {}\n", info.executables.len()));
    out.push_str(&format!("Libraries      : {}\n", info.libraries.len()));
    out.push_str(&format!("Subdirectories : {}\n", info.subdirs.len()));
    out.push_str(&format!("find_package() : {}\n", info.find_pkgs.len()));
    out.push_str(&format!("Options (option()): {}\n", info.options.len()));
    out.push_str(&format!("Custom targets : {}\n", info.custom_targets.len()));

    if !info.executables.is_empty() {
        out.push_str("\nExecutables\n");
        for (name, srcs) in &info.executables {
            out.push_str(&format!("  {name}  ({} source file(s))\n", srcs.len()));
        }
    }
    if !info.libraries.is_empty() {
        out.push_str("\nLibraries\n");
        for (name, lib_type, srcs) in &info.libraries {
            out.push_str(&format!(
                "  {name}  [{lib_type}]  ({} source file(s))\n",
                srcs.len()
            ));
        }
    }
    if !info.subdirs.is_empty() {
        out.push_str("\nSubdirectories\n");
        for d in &info.subdirs {
            out.push_str(&format!("  {d}\n"));
        }
    }
    if !info.find_pkgs.is_empty() {
        out.push_str("\nDependencies (find_package)\n");
        for p in &info.find_pkgs {
            out.push_str(&format!("  {p}\n"));
        }
    }
    Ok(out)
}

fn do_targets(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let info = parse_cmake(&text);
    let mut out = String::from("Build Targets\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    if info.executables.is_empty() && info.libraries.is_empty() && info.custom_targets.is_empty() {
        return Ok("No targets found.\n".into());
    }
    if !info.executables.is_empty() {
        out.push_str("\nExecutables\n");
        for (name, srcs) in &info.executables {
            let src_preview = if srcs.is_empty() {
                String::new()
            } else {
                format!(
                    "  ({})",
                    srcs.iter()
                        .take(3)
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                        + if srcs.len() > 3 { " …" } else { "" }
                )
            };
            out.push_str(&format!("  {name}{src_preview}\n"));
        }
    }
    if !info.libraries.is_empty() {
        out.push_str("\nLibraries\n");
        for (name, lib_type, srcs) in &info.libraries {
            let src_preview = if srcs.is_empty() {
                String::new()
            } else {
                format!(
                    "  ({})",
                    srcs.iter()
                        .take(3)
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                        + if srcs.len() > 3 { " …" } else { "" }
                )
            };
            out.push_str(&format!("  {name}  [{lib_type}]{src_preview}\n"));
        }
    }
    if !info.custom_targets.is_empty() {
        out.push_str("\nCustom Targets\n");
        for t in &info.custom_targets {
            out.push_str(&format!("  {t}\n"));
        }
    }
    if !info.install_targets.is_empty() {
        out.push_str("\nInstall Targets\n");
        for t in &info.install_targets {
            out.push_str(&format!("  {t}\n"));
        }
    }
    Ok(out)
}

fn do_options(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let info = parse_cmake(&text);
    if info.options.is_empty() && info.set_vars.is_empty() {
        return Ok("No option() or set() entries found.\n".into());
    }
    let mut out = String::new();
    if !info.options.is_empty() {
        let w = info
            .options
            .iter()
            .map(|(v, _, _)| v.len())
            .max()
            .unwrap_or(10)
            .max(8);
        out.push_str(&format!("CMake Options ({} total)\n", info.options.len()));
        out.push_str(&"─".repeat(50));
        out.push('\n');
        out.push_str(&format!("{:<w$}  Default  Description\n", "Variable"));
        out.push_str(&format!("{:<w$}  ───────  ───────────\n", "─".repeat(w)));
        for (var, desc, def) in &info.options {
            let d = if desc.len() > 50 {
                format!("{}…", &desc[..47])
            } else {
                desc.clone()
            };
            out.push_str(&format!("{:<w$}  {:<7}  {d}\n", var, def));
        }
    }
    if !info.set_vars.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!(
            "set() Variables ({} total)\n",
            info.set_vars.len()
        ));
        out.push_str(&"─".repeat(50));
        out.push('\n');
        for (var, val) in &info.set_vars {
            let v = if val.len() > 60 {
                format!("{}…", &val[..57])
            } else {
                val.clone()
            };
            out.push_str(&format!("  {var} = {v}\n"));
        }
    }
    Ok(out)
}

fn do_deps(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let info = parse_cmake(&text);
    let calls = collect_calls(&text);
    let mut out = String::from("CMake Dependencies\n");
    out.push_str(&"─".repeat(40));
    out.push('\n');
    if !info.find_pkgs.is_empty() {
        out.push_str(&format!(
            "\nfind_package() ({} total)\n",
            info.find_pkgs.len()
        ));
        for pkg in &info.find_pkgs {
            out.push_str(&format!("  {pkg}\n"));
        }
    }
    // target_link_libraries
    let link_calls: Vec<&(String, Vec<String>)> = calls
        .iter()
        .filter(|(cmd, _)| cmd == "target_link_libraries")
        .collect();
    if !link_calls.is_empty() {
        out.push_str("\ntarget_link_libraries()\n");
        for (_, args) in &link_calls {
            if let Some(target) = args.first() {
                let libs: Vec<&str> = args[1..]
                    .iter()
                    .filter(|a| {
                        !matches!(
                            a.to_uppercase().as_str(),
                            "PUBLIC" | "PRIVATE" | "INTERFACE"
                        )
                    })
                    .map(|s| s.as_str())
                    .collect();
                out.push_str(&format!("  {target}  ←  {}\n", libs.join(", ")));
            }
        }
    }
    // include_directories
    let include_dirs: Vec<&Vec<String>> = calls
        .iter()
        .filter(|(cmd, _)| cmd == "include_directories" || cmd == "target_include_directories")
        .map(|(_, a)| a)
        .collect();
    if !include_dirs.is_empty() {
        out.push_str("\nInclude Directories\n");
        for dirs in &include_dirs {
            for d in dirs.iter().filter(|a| {
                !matches!(
                    a.to_uppercase().as_str(),
                    "PUBLIC" | "PRIVATE" | "INTERFACE"
                )
            }) {
                out.push_str(&format!("  {d}\n"));
            }
        }
    }
    if !info.subdirs.is_empty() {
        out.push_str("\nadd_subdirectory()\n");
        for d in &info.subdirs {
            out.push_str(&format!("  {d}\n"));
        }
    }
    if !info.includes.is_empty() {
        out.push_str("\ninclude() modules\n");
        for inc in &info.includes {
            out.push_str(&format!("  {inc}\n"));
        }
    }
    if out.trim_end() == "CMake Dependencies\n───────────────────────────────────────"
    {
        out.push_str("  No external dependencies found.\n");
    }
    Ok(out)
}

fn do_validate(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let info = parse_cmake(&text);
    let calls = collect_calls(&text);
    let mut issues: Vec<String> = Vec::new();

    if info.min_version.is_none() {
        issues.push(
            "Missing cmake_minimum_required() — always specify to avoid CMake policy warnings"
                .into(),
        );
    }
    if info.project_name.is_none() {
        issues.push("Missing project() call".into());
    }
    if info.executables.is_empty() && info.libraries.is_empty() && info.subdirs.is_empty() {
        issues.push(
            "No add_executable(), add_library(), or add_subdirectory() — nothing to build".into(),
        );
    }

    // Warn about quoted variable references that might be empty
    let set_names: std::collections::HashSet<String> =
        info.set_vars.iter().map(|(v, _)| v.clone()).collect();
    for (cmd, args) in &calls {
        if cmd == "target_link_libraries" {
            for a in args.iter().skip(1) {
                if a.starts_with("${") {
                    let var = a.trim_start_matches("${").trim_end_matches('}');
                    if !set_names.contains(var)
                        && !var.starts_with("CMAKE_")
                        && !var.ends_with("_LIBRARIES")
                        && !var.ends_with("_LIBRARY")
                    {
                        issues.push(format!("target_link_libraries references '${{{var}}}' which may be unset — did you call find_package first?"));
                    }
                }
            }
        }
    }

    // Check for deprecated cmake_policy or old-style include_directories mixed with modern targets
    let has_modern_targets = calls.iter().any(|(cmd, _)| {
        cmd == "target_include_directories"
            || cmd == "target_link_libraries"
            || cmd == "target_compile_options"
    });
    let has_old_style = calls
        .iter()
        .any(|(cmd, _)| cmd == "include_directories" || cmd == "link_libraries");
    if has_modern_targets && has_old_style {
        issues.push("Mixing modern target_*() commands with old-style include_directories()/link_libraries() — prefer target_*() consistently".into());
    }

    // GLOB for sources is fragile
    let uses_glob = calls.iter().any(|(cmd, args)| {
        cmd == "file"
            && args
                .first()
                .map(|a| a.eq_ignore_ascii_case("GLOB") || a.eq_ignore_ascii_case("GLOB_RECURSE"))
                .unwrap_or(false)
    });
    if uses_glob {
        issues.push("file(GLOB ...) for source files won't re-run CMake when files are added/removed — prefer explicit source lists".into());
    }

    // Hardcoded absolute paths
    let lower = text.to_lowercase();
    if lower.contains("/usr/local/")
        || lower.contains("/home/")
        || lower.contains("c:\\\\users\\\\")
        || lower.contains("/root/")
    {
        issues.push("Hardcoded absolute user/local paths found — use variables or CMake find modules for portability".into());
    }

    let mut out = String::new();
    if issues.is_empty() {
        out.push_str("✓ VALID — no issues found\n");
    } else {
        out.push_str(&format!("WARNINGS — {} issue(s) found\n", issues.len()));
        out.push_str(&"─".repeat(40));
        out.push('\n');
        for issue in &issues {
            out.push_str(&format!("  ⚠ {issue}\n"));
        }
    }
    Ok(out)
}
