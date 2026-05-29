use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => info_action(args),
        "scripts" => scripts_action(args),
        "deps" => deps_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: info, scripts, deps, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("json"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the package.json content as a string".to_string())
}

fn parse_pkg(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|e| format!("Failed to parse package.json: {}", e))
}

fn str_field<'a>(pkg: &'a Value, key: &str) -> Option<&'a str> {
    pkg.get(key).and_then(|v| v.as_str())
}

fn obj_keys_count(pkg: &Value, key: &str) -> usize {
    pkg.get(key)
        .and_then(|v| v.as_object())
        .map(|m| m.len())
        .unwrap_or(0)
}

fn info_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let pkg = parse_pkg(&text)?;

    let name = str_field(&pkg, "name").unwrap_or("(unnamed)");
    let version = str_field(&pkg, "version").unwrap_or("(unset)");
    let description = str_field(&pkg, "description").unwrap_or("");
    let license = str_field(&pkg, "license").unwrap_or("(none)");
    let main = str_field(&pkg, "main").unwrap_or("");
    let module = str_field(&pkg, "module").unwrap_or("");
    let types = str_field(&pkg, "types")
        .or_else(|| str_field(&pkg, "typings"))
        .unwrap_or("");

    let mut out = format!("package.json\n{}\n\n", "=".repeat(44));
    out += &format!("Name:        {}\n", name);
    out += &format!("Version:     {}\n", version);
    if !description.is_empty() {
        let snippet: String = description.chars().take(80).collect();
        let ellipsis = if description.len() > 80 { "..." } else { "" };
        out += &format!("Description: {}{}\n", snippet, ellipsis);
    }
    out += &format!("License:     {}\n", license);

    if let Some(author) = pkg.get("author") {
        let author_str = match author {
            Value::String(s) => s.clone(),
            Value::Object(o) => {
                let n = o.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let e = o.get("email").and_then(|v| v.as_str()).unwrap_or("");
                if e.is_empty() {
                    n.to_string()
                } else {
                    format!("{} <{}>", n, e)
                }
            }
            _ => String::new(),
        };
        if !author_str.is_empty() {
            out += &format!("Author:      {}\n", author_str);
        }
    }

    if !main.is_empty() {
        out += &format!("Main:        {}\n", main);
    }
    if !module.is_empty() {
        out += &format!("Module:      {}\n", module);
    }
    if !types.is_empty() {
        out += &format!("Types:       {}\n", types);
    }

    // Private flag
    if pkg.get("private").and_then(|v| v.as_bool()) == Some(true) {
        out += "Private:     true\n";
    }

    // Workspaces
    if let Some(ws) = pkg.get("workspaces") {
        let count = match ws {
            Value::Array(a) => a.len(),
            _ => 0,
        };
        if count > 0 {
            out += &format!("Workspaces:  {} package(s)\n", count);
        }
    }

    // Engines
    if let Some(engines) = pkg.get("engines").and_then(|v| v.as_object()) {
        out += "Engines:\n";
        for (k, v) in engines.iter().take(4) {
            out += &format!("  {}: {}\n", k, v.as_str().unwrap_or("?"));
        }
    }

    out += "\n";
    let scripts_count = obj_keys_count(&pkg, "scripts");
    let deps_count = obj_keys_count(&pkg, "dependencies");
    let dev_deps_count = obj_keys_count(&pkg, "devDependencies");
    let peer_deps_count = obj_keys_count(&pkg, "peerDependencies");
    let optional_deps_count = obj_keys_count(&pkg, "optionalDependencies");

    out += &format!("Scripts:          {}\n", scripts_count);
    out += &format!("Dependencies:     {}\n", deps_count);
    out += &format!("DevDependencies:  {}\n", dev_deps_count);
    if peer_deps_count > 0 {
        out += &format!("PeerDependencies: {}\n", peer_deps_count);
    }
    if optional_deps_count > 0 {
        out += &format!("OptionalDeps:     {}\n", optional_deps_count);
    }

    // Keywords
    if let Some(keywords) = pkg.get("keywords").and_then(|v| v.as_array()) {
        let kws: Vec<&str> = keywords.iter().filter_map(|v| v.as_str()).take(6).collect();
        if !kws.is_empty() {
            out += &format!("\nKeywords: {}\n", kws.join(", "));
        }
    }

    // Repository
    if let Some(repo) = pkg.get("repository") {
        let url = match repo {
            Value::String(s) => s.clone(),
            Value::Object(o) => o
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            _ => String::new(),
        };
        if !url.is_empty() {
            out += &format!("Repository: {}\n", url);
        }
    }

    Ok(out)
}

fn scripts_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let pkg = parse_pkg(&text)?;

    let scripts = pkg
        .get("scripts")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "No 'scripts' field found in package.json".to_string())?;

    if scripts.is_empty() {
        return Ok("No scripts defined.\n".to_string());
    }

    let filter = args
        .get("filter")
        .or_else(|| args.get("query"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let mut out = format!("Scripts  [{}]\n{}\n\n", scripts.len(), "=".repeat(44));
    for (name, cmd) in scripts.iter() {
        let cmd_str = cmd.as_str().unwrap_or("");
        if let Some(ref f) = filter {
            if !name.to_lowercase().contains(f.as_str())
                && !cmd_str.to_lowercase().contains(f.as_str())
            {
                continue;
            }
        }
        let snippet: String = cmd_str.chars().take(72).collect();
        let ellipsis = if cmd_str.len() > 72 { "…" } else { "" };
        out += &format!("  {:<20} {}{}\n", name, snippet, ellipsis);
    }

    Ok(out)
}

fn deps_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let pkg = parse_pkg(&text)?;

    let kind = args
        .get("kind")
        .or_else(|| args.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("all");

    let filter = args
        .get("filter")
        .or_else(|| args.get("query"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let mut out = format!("Dependencies\n{}\n\n", "=".repeat(44));
    let sections: Vec<(&str, &str)> = match kind {
        "dev" => vec![("devDependencies", "Dev")],
        "peer" => vec![("peerDependencies", "Peer")],
        "optional" => vec![("optionalDependencies", "Optional")],
        "prod" => vec![("dependencies", "Prod")],
        _ => vec![
            ("dependencies", "Prod"),
            ("devDependencies", "Dev"),
            ("peerDependencies", "Peer"),
            ("optionalDependencies", "Optional"),
        ],
    };

    for (key, label) in &sections {
        if let Some(deps) = pkg.get(*key).and_then(|v| v.as_object()) {
            if deps.is_empty() {
                continue;
            }
            out += &format!("{} [{}]\n", label, deps.len());
            let mut entries: Vec<(&String, &Value)> = deps.iter().collect();
            entries.sort_by_key(|(k, _)| k.as_str());
            for (name, ver) in &entries {
                let ver_str = ver.as_str().unwrap_or("?");
                if let Some(ref f) = filter {
                    if !name.to_lowercase().contains(f.as_str())
                        && !ver_str.to_lowercase().contains(f.as_str())
                    {
                        continue;
                    }
                }
                let flag = dep_flag(ver_str);
                out += &format!("  {:<36} {}{}\n", name, ver_str, flag);
            }
            out += "\n";
        }
    }

    Ok(out)
}

fn dep_flag(ver: &str) -> &'static str {
    if ver == "*" || ver == "x" {
        " [WILDCARD]"
    } else if ver.starts_with("http://")
        || ver.starts_with("https://")
        || ver.starts_with("git+")
        || ver.starts_with("git://")
    {
        " [URL-DEP]"
    } else if ver.starts_with("file:") {
        " [LOCAL]"
    } else if ver.starts_with("github:") || (ver.contains('/') && !ver.contains('@')) {
        " [GIT]"
    } else {
        ""
    }
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let pkg = parse_pkg(&text)?;
    let mut warnings: Vec<String> = Vec::new();

    // Required fields
    if pkg.get("name").and_then(|v| v.as_str()).is_none() {
        warnings.push("Missing 'name' field".to_string());
    }
    if pkg.get("version").and_then(|v| v.as_str()).is_none() {
        warnings.push("Missing 'version' field".to_string());
    }
    if pkg.get("description").and_then(|v| v.as_str()).is_none()
        || pkg
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().is_empty())
            == Some(true)
    {
        warnings.push("Missing or empty 'description' field".to_string());
    }
    if pkg.get("license").and_then(|v| v.as_str()).is_none()
        || pkg
            .get("license")
            .and_then(|v| v.as_str())
            .map(|s| s.is_empty())
            == Some(true)
    {
        warnings.push("Missing 'license' field — required for npm publishing".to_string());
    }

    // engines should be set
    if pkg.get("engines").is_none() {
        warnings.push(
            "No 'engines' field — specifying node/npm engine requirements prevents version mismatch"
                .to_string(),
        );
    }

    // Version field format
    if let Some(ver) = pkg.get("version").and_then(|v| v.as_str()) {
        if ver.starts_with('^') || ver.starts_with('~') || ver.contains('*') {
            warnings.push(format!(
                "Package 'version' field '{}' should be an exact version, not a range",
                ver
            ));
        }
    }

    // Dependency checks
    for dep_key in &[
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        if let Some(deps) = pkg.get(*dep_key).and_then(|v| v.as_object()) {
            for (name, ver) in deps.iter() {
                let ver_str = ver.as_str().unwrap_or("");
                if ver_str == "*" || ver_str == "x" {
                    warnings.push(format!(
                        "[{}] '{}' uses wildcard version '{}' — pin to a specific range",
                        dep_key, name, ver_str
                    ));
                }
                if ver_str.starts_with("http://") {
                    warnings.push(format!(
                        "[{}] '{}' uses an http:// URL dep — prefer https:// or a registry version",
                        dep_key, name
                    ));
                }
                // Detect overly broad ranges like >=1.0.0 with no upper bound
                if (ver_str.starts_with(">=") || ver_str.starts_with(">"))
                    && !ver_str.contains('<')
                    && !ver_str.contains(' ')
                {
                    warnings.push(format!(
                        "[{}] '{}' range '{}' has no upper bound — use ^ or ~ for safer updates",
                        dep_key, name, ver_str
                    ));
                }
            }
        }
    }

    // Scripts: check for missing test/build
    if let Some(scripts) = pkg.get("scripts").and_then(|v| v.as_object()) {
        if !scripts.contains_key("test") {
            warnings.push("No 'test' script — add one so 'npm test' works in CI".to_string());
        }
        // Check for 'prepare' if this is a library (has main or module but no private:true)
        let has_main = pkg.get("main").is_some() || pkg.get("module").is_some();
        let is_private = pkg
            .get("private")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if has_main && !is_private && !scripts.contains_key("build") {
            warnings.push(
                "No 'build' script — libraries with 'main'/'module' typically need a build step"
                    .to_string(),
            );
        }
    } else {
        warnings.push("No 'scripts' field — at minimum add a 'test' script".to_string());
    }

    // Files field: if publishing, 'files' whitelist is safer than relying on .npmignore
    let is_private = pkg
        .get("private")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !is_private && pkg.get("files").is_none() {
        warnings.push(
            "No 'files' field — npm will publish all non-ignored files; add a 'files' whitelist for safety"
                .to_string(),
        );
    }

    // Duplicate dep in both deps and devDeps
    if let (Some(deps), Some(dev_deps)) = (
        pkg.get("dependencies").and_then(|v| v.as_object()),
        pkg.get("devDependencies").and_then(|v| v.as_object()),
    ) {
        for name in deps.keys() {
            if dev_deps.contains_key(name) {
                warnings.push(format!(
                    "'{}' appears in both 'dependencies' and 'devDependencies'",
                    name
                ));
            }
        }
    }

    // Finish
    let dep_count = obj_keys_count(&pkg, "dependencies");
    let dev_count = obj_keys_count(&pkg, "devDependencies");
    let script_count = obj_keys_count(&pkg, "scripts");

    let mut out = format!("package.json Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!(
        "{} dep(s), {} devDep(s), {} script(s).\n",
        dep_count, dev_count, script_count
    );
    if warnings.is_empty() {
        out += "No issues found.\n";
    } else {
        out += &format!("\n{} warning(s):\n", warnings.len());
        for w in &warnings {
            out += &format!("  [WARN] {}\n", w);
        }
    }
    Ok(out)
}
