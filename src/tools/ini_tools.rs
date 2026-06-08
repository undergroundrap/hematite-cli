pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");

    match action {
        "parse" => parse(args),
        "get" => get(args),
        "sections" => sections(args),
        "keys" => keys(args),
        "validate" => validate(args),
        "to-json" => to_json(args),
        "to-toml" => to_toml(args),
        other => Err(format!(
            "ini_tools: unknown action '{other}'. Valid: parse, get, sections, keys, validate, to-json, to-toml"
        )),
    }
}

// ── Data structures ────────────────────────────────────────────────────────────

#[derive(Debug)]
struct IniDoc {
    global: Vec<(String, String)>,
    sections: Vec<IniSection>,
}

#[derive(Debug)]
struct IniSection {
    name: String,
    pairs: Vec<(String, String)>,
}

// ── Parser ─────────────────────────────────────────────────────────────────────

fn parse_ini(text: &str) -> IniDoc {
    let mut global: Vec<(String, String)> = Vec::new();
    let mut sections: Vec<IniSection> = Vec::new();
    let mut current: Option<usize> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();

        // Skip blank lines and full-line comments
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        // Section header
        if line.starts_with('[') {
            if let Some(end) = line.find(']') {
                let name = line[1..end].trim().to_string();
                sections.push(IniSection {
                    name,
                    pairs: Vec::new(),
                });
                current = Some(sections.len() - 1);
            }
            continue;
        }

        // Key = value (also accept key: value)
        let sep = line
            .find('=')
            .map(|p| ('=', p))
            .or_else(|| line.find(':').map(|p| (':', p)));

        if let Some((_, pos)) = sep {
            let key = line[..pos].trim().to_string();
            let raw_val = line[pos + 1..].trim();
            // Strip inline comments (semicolon or hash preceded by whitespace)
            let value = strip_inline_comment(raw_val).to_string();

            if !key.is_empty() {
                match current {
                    Some(idx) => sections[idx].pairs.push((key, value)),
                    None => global.push((key, value)),
                }
            }
        }
    }

    IniDoc { global, sections }
}

fn strip_inline_comment(s: &str) -> &str {
    for (i, c) in s.char_indices() {
        if (c == ';' || c == '#') && i > 0 && s.as_bytes()[i - 1] == b' ' {
            return s[..i].trim_end();
        }
    }
    s
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn load_ini_text(args: &serde_json::Value) -> Result<String, String> {
    if let Some(file) = args.get("file").and_then(|v| v.as_str()) {
        let path = if std::path::Path::new(file).is_absolute() {
            std::path::PathBuf::from(file)
        } else {
            let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
                std::path::PathBuf::from(r)
            } else {
                crate::tools::file_ops::workspace_root()
            };
            root.join(file)
        };
        if !path.exists() {
            return Err(format!("ini_tools: file not found: {}", path.display()));
        }
        return std::fs::read_to_string(&path)
            .map_err(|e| format!("ini_tools: cannot read '{}': {e}", path.display()));
    }
    args.get("text")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("ini"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "ini_tools: 'text', 'ini', or 'file' is required".to_string())
}

fn section_pairs<'a>(doc: &'a IniDoc, section: &str) -> Option<&'a [(String, String)]> {
    if section.is_empty() {
        return Some(&doc.global);
    }
    doc.sections
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(section))
        .map(|s| s.pairs.as_slice())
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn parse(args: &serde_json::Value) -> Result<String, String> {
    let text = load_ini_text(args)?;
    let doc = parse_ini(&text);
    let label = args
        .get("file")
        .and_then(|v| v.as_str())
        .unwrap_or("(inline)");

    let total_keys: usize =
        doc.global.len() + doc.sections.iter().map(|s| s.pairs.len()).sum::<usize>();
    let mut out = format!("INI PARSE: {label}\n{}\n", "─".repeat(50));
    out.push_str(&format!(
        "{} section(s), {} key(s) total\n\n",
        doc.sections.len(),
        total_keys
    ));

    if !doc.global.is_empty() {
        out.push_str("[global]\n");
        for (k, v) in &doc.global {
            out.push_str(&format!("  {k} = {v}\n"));
        }
        out.push('\n');
    }

    for sec in &doc.sections {
        out.push_str(&format!("[{}]\n", sec.name));
        if sec.pairs.is_empty() {
            out.push_str("  (empty)\n");
        } else {
            for (k, v) in &sec.pairs {
                out.push_str(&format!("  {k} = {v}\n"));
            }
        }
        out.push('\n');
    }

    Ok(out)
}

fn get(args: &serde_json::Value) -> Result<String, String> {
    let text = load_ini_text(args)?;
    let doc = parse_ini(&text);

    // Accept "section.key" via 'key' or separate 'section'/'key' args
    let (section, key) = if let Some(k) = args.get("key").and_then(|v| v.as_str()) {
        if let Some(dot) = k.find('.') {
            (k[..dot].to_string(), k[dot + 1..].to_string())
        } else {
            let sec = args
                .get("section")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (sec, k.to_string())
        }
    } else {
        return Err("ini_tools get: 'key' is required (use 'section.key' dot notation or pass 'section' separately)".to_string());
    };

    let pairs = section_pairs(&doc, &section).ok_or_else(|| {
        if section.is_empty() {
            "ini_tools get: no global keys found".to_string()
        } else {
            format!("ini_tools get: section '{section}' not found")
        }
    })?;

    let value = pairs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(&key))
        .map(|(_, v)| v.as_str())
        .ok_or_else(|| format!("ini_tools get: key '{key}' not found in section '{section}'"))?;

    let mut out = format!("INI GET\n{}\n", "─".repeat(50));
    let path = if section.is_empty() {
        key.clone()
    } else {
        format!("{section}.{key}")
    };
    out.push_str(&format!("{path} = {value}\n"));
    Ok(out)
}

fn sections(args: &serde_json::Value) -> Result<String, String> {
    let text = load_ini_text(args)?;
    let doc = parse_ini(&text);

    let mut out = format!("INI SECTIONS\n{}\n", "─".repeat(50));
    if !doc.global.is_empty() {
        out.push_str(&format!("  [global]  ({} key(s))\n", doc.global.len()));
    }
    if doc.sections.is_empty() && doc.global.is_empty() {
        out.push_str("  (no sections found)\n");
    }
    for sec in &doc.sections {
        out.push_str(&format!("  [{}]  ({} key(s))\n", sec.name, sec.pairs.len()));
    }
    out.push_str(&format!("\n{} section(s)\n", doc.sections.len()));
    Ok(out)
}

fn keys(args: &serde_json::Value) -> Result<String, String> {
    let text = load_ini_text(args)?;
    let doc = parse_ini(&text);

    let section_name = args.get("section").and_then(|v| v.as_str()).unwrap_or("");

    let pairs = if section_name.is_empty() && doc.sections.is_empty() {
        // No sections — show global
        doc.global.as_slice()
    } else {
        section_pairs(&doc, section_name)
            .ok_or_else(|| format!("ini_tools keys: section '{section_name}' not found"))?
    };

    let label = if section_name.is_empty() {
        "global"
    } else {
        section_name
    };
    let mut out = format!("INI KEYS: [{label}]\n{}\n", "─".repeat(50));
    if pairs.is_empty() {
        out.push_str("  (no keys)\n");
    } else {
        for (k, v) in pairs {
            out.push_str(&format!("  {k} = {v}\n"));
        }
        out.push_str(&format!("\n{} key(s)\n", pairs.len()));
    }
    Ok(out)
}

fn validate(args: &serde_json::Value) -> Result<String, String> {
    let text = load_ini_text(args)?;
    let label = args
        .get("file")
        .and_then(|v| v.as_str())
        .unwrap_or("(inline)");
    let doc = parse_ini(&text);

    let mut issues: Vec<String> = Vec::new();

    // Duplicate global keys
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (k, _) in &doc.global {
        let lk = k.to_lowercase();
        if !seen.insert(lk.clone()) {
            issues.push(format!("Duplicate global key: '{k}'"));
        }
    }

    // Per-section checks
    for sec in &doc.sections {
        if sec.name.is_empty() {
            issues.push("Empty section name '[]'".to_string());
        }
        if sec.pairs.is_empty() {
            issues.push(format!("Empty section: '[{}]'", sec.name));
        }
        let mut seen_in_sec: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (k, _) in &sec.pairs {
            let lk = k.to_lowercase();
            if !seen_in_sec.insert(lk) {
                issues.push(format!("Duplicate key '{k}' in section '[{}]'", sec.name));
            }
        }
    }

    // Duplicate section names
    let mut seen_secs: std::collections::HashSet<String> = std::collections::HashSet::new();
    for sec in &doc.sections {
        let ls = sec.name.to_lowercase();
        if !seen_secs.insert(ls) {
            issues.push(format!("Duplicate section: '[{}]'", sec.name));
        }
    }

    let total_keys: usize =
        doc.global.len() + doc.sections.iter().map(|s| s.pairs.len()).sum::<usize>();
    let mut out = format!("INI VALIDATE: {label}\n{}\n", "─".repeat(50));
    out.push_str(&format!(
        "{} section(s), {} key(s) total\n\n",
        doc.sections.len(),
        total_keys
    ));

    if issues.is_empty() {
        out.push_str("Result : VALID\n");
        out.push_str("No issues found.\n");
    } else {
        out.push_str("Result : ISSUES FOUND\n\n");
        for issue in &issues {
            out.push_str(&format!("  ⚠ {issue}\n"));
        }
        out.push_str(&format!("\n{} issue(s)\n", issues.len()));
    }
    Ok(out)
}

fn to_json(args: &serde_json::Value) -> Result<String, String> {
    let text = load_ini_text(args)?;
    let doc = parse_ini(&text);

    let mut root = serde_json::Map::new();

    if !doc.global.is_empty() {
        let mut global_obj = serde_json::Map::new();
        for (k, v) in &doc.global {
            global_obj.insert(k.clone(), serde_json::Value::String(v.clone()));
        }
        root.insert("global".to_string(), serde_json::Value::Object(global_obj));
    }

    for sec in &doc.sections {
        let mut sec_obj = serde_json::Map::new();
        for (k, v) in &sec.pairs {
            sec_obj.insert(k.clone(), serde_json::Value::String(v.clone()));
        }
        root.insert(sec.name.clone(), serde_json::Value::Object(sec_obj));
    }

    let json_val = serde_json::Value::Object(root);
    let pretty = serde_json::to_string_pretty(&json_val)
        .map_err(|e| format!("ini_tools to-json: serialization failed: {e}"))?;

    let mut out = format!("INI TO JSON\n{}\n", "─".repeat(50));
    out.push_str(&pretty);
    out.push('\n');
    Ok(out)
}

fn to_toml(args: &serde_json::Value) -> Result<String, String> {
    let text = load_ini_text(args)?;
    let doc = parse_ini(&text);

    let mut out_str = String::new();

    // Global keys go at the top level
    for (k, v) in &doc.global {
        out_str.push_str(&format!("{k} = \"{}\"\n", toml_escape(v)));
    }
    if !doc.global.is_empty() && !doc.sections.is_empty() {
        out_str.push('\n');
    }

    for sec in &doc.sections {
        out_str.push_str(&format!("[{}]\n", sec.name));
        for (k, v) in &sec.pairs {
            out_str.push_str(&format!("{k} = \"{}\"\n", toml_escape(v)));
        }
        out_str.push('\n');
    }

    let mut out = format!("INI TO TOML\n{}\n", "─".repeat(50));
    out.push_str(&out_str);
    Ok(out)
}

fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
