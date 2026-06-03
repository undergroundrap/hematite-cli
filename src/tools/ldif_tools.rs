use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["parse", "search", "attrs", "schema", "summary"],
                "description": "Action: parse (default), search, attrs, schema, summary"
            },
            "text": { "type": "string", "description": "Inline LDIF text" },
            "ldif": { "type": "string", "description": "Inline LDIF text (alias for text)" },
            "file": { "type": "string", "description": "Path to a .ldif file" },
            "query": { "type": "string", "description": "Filter entries by DN, objectClass, or attribute value substring" },
            "attr": { "type": "string", "description": "Focus on a specific attribute name" },
            "dn": { "type": "string", "description": "Filter entries by DN substring" },
            "limit": { "type": "integer", "description": "Max entries to show" }
        }
    })
}

fn load_text(args: &Value) -> Result<String, String> {
    if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(f).map_err(|e| format!("Cannot read '{}': {}", f, e))
    } else if let Some(t) = args.get("ldif").and_then(|v| v.as_str()) {
        Ok(t.to_string())
    } else if let Some(t) = args.get("text").and_then(|v| v.as_str()) {
        Ok(t.to_string())
    } else {
        Err("Provide 'ldif'/'text' (inline) or 'file' (path to .ldif).".to_string())
    }
}

#[derive(Debug, Default)]
struct LdifEntry {
    dn: String,
    changetype: Option<String>,
    attributes: Vec<(String, String)>,
}

fn decode_base64_value(val: &str) -> String {
    let bytes = val.trim();
    // decode base64 to UTF-8, fall back to hex on non-UTF-8
    let decoded: Vec<u8> = {
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut map = [255u8; 256];
        for (i, &c) in alphabet.iter().enumerate() {
            map[c as usize] = i as u8;
        }
        let mut out = Vec::new();
        let mut buf = 0u32;
        let mut bits = 0u8;
        for c in bytes.bytes() {
            if c == b'=' {
                break;
            }
            let v = map[c as usize];
            if v == 255 {
                continue;
            }
            buf = (buf << 6) | v as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((buf >> bits) as u8);
            }
        }
        out
    };
    String::from_utf8(decoded).unwrap_or_else(|e| format!("<binary {}>", e.into_bytes().len()))
}

fn parse_ldif(text: &str) -> Vec<LdifEntry> {
    // RFC 2849: unfold continuation lines (lines starting with space/tab)
    let mut unfolded = String::with_capacity(text.len());
    for line in text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            // continuation: append without newline prefix
            if let Some(last) = unfolded.strip_suffix('\n') {
                unfolded = last.to_string();
            }
            unfolded.push_str(line.trim_start());
        } else {
            if !unfolded.is_empty() {
                unfolded.push('\n');
            }
            unfolded.push_str(line);
        }
    }

    let mut entries: Vec<LdifEntry> = Vec::new();
    let mut current: Option<LdifEntry> = None;

    for line in unfolded.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.starts_with("version:") {
            continue;
        }
        if line.is_empty() {
            if let Some(e) = current.take() {
                if !e.dn.is_empty() {
                    entries.push(e);
                }
            }
            continue;
        }

        let (attr, value) = if let Some(pos) = line.find(':') {
            let a = line[..pos].trim().to_lowercase();
            let rest = &line[pos + 1..];
            if rest.starts_with(':') {
                // base64-encoded value
                let decoded = decode_base64_value(rest[1..].trim());
                (a, decoded)
            } else if rest.starts_with('<') {
                // URL reference
                (a, format!("<URL: {}>", rest[1..].trim()))
            } else {
                (a, rest.trim().to_string())
            }
        } else {
            continue;
        };

        if attr == "dn" {
            if let Some(e) = current.take() {
                if !e.dn.is_empty() {
                    entries.push(e);
                }
            }
            current = Some(LdifEntry {
                dn: value,
                ..Default::default()
            });
        } else if let Some(ref mut e) = current {
            if attr == "changetype" {
                e.changetype = Some(value);
            } else {
                e.attributes.push((attr, value));
            }
        }
    }
    if let Some(e) = current.take() {
        if !e.dn.is_empty() {
            entries.push(e);
        }
    }
    entries
}

fn entry_classes(e: &LdifEntry) -> Vec<String> {
    e.attributes
        .iter()
        .filter(|(k, _)| k == "objectclass")
        .map(|(_, v)| v.clone())
        .collect()
}

fn entry_attr<'a>(e: &'a LdifEntry, name: &str) -> Option<&'a str> {
    e.attributes
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

fn redact_if_sensitive(attr: &str, val: &str) -> String {
    let lower = attr.to_lowercase();
    if lower.contains("password")
        || lower.contains("passwd")
        || lower == "userpassword"
        || lower.contains("secret")
        || lower.contains("token")
    {
        "[REDACTED]".to_string()
    } else {
        val.to_string()
    }
}

fn action_parse(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let entries = parse_ldif(&text);
    let dn_filter = args.get("dn").and_then(|v| v.as_str());
    let query = args.get("query").and_then(|v| v.as_str());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20);

    let filtered: Vec<&LdifEntry> = entries
        .iter()
        .filter(|e| dn_filter.map_or(true, |d| e.dn.to_lowercase().contains(&d.to_lowercase())))
        .filter(|e| {
            query.map_or(true, |q| {
                let ql = q.to_lowercase();
                e.dn.to_lowercase().contains(&ql)
                    || e.attributes
                        .iter()
                        .any(|(_, v)| v.to_lowercase().contains(&ql))
                    || entry_classes(e)
                        .iter()
                        .any(|c| c.to_lowercase().contains(&ql))
            })
        })
        .take(limit as usize)
        .collect();

    if filtered.is_empty() {
        return Ok(format!("No entries found (total: {}).", entries.len()));
    }

    let mut out = String::new();
    for e in filtered {
        out.push_str(&format!("\n## {}\n", e.dn));
        if let Some(ct) = &e.changetype {
            out.push_str(&format!("   changetype: {}\n", ct));
        }
        let classes: Vec<String> = entry_classes(e);
        if !classes.is_empty() {
            out.push_str(&format!("   objectClass: {}\n", classes.join(", ")));
        }
        for (k, v) in &e.attributes {
            if k == "objectclass" {
                continue;
            }
            let display = redact_if_sensitive(k, v);
            out.push_str(&format!("   {}: {}\n", k, display));
        }
    }
    if entries.len() > limit as usize {
        out.push_str(&format!(
            "\n(Showing {} of {} entries — use limit= to see more)\n",
            limit,
            entries.len()
        ));
    }
    Ok(out.trim_start().to_string())
}

fn action_search(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let entries = parse_ldif(&text);
    let query = args
        .get("query")
        .or_else(|| args.get("q"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'query' to search.")?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20);
    let ql = query.to_lowercase();

    let matches: Vec<&LdifEntry> = entries
        .iter()
        .filter(|e| {
            e.dn.to_lowercase().contains(&ql)
                || e.attributes
                    .iter()
                    .any(|(k, v)| k.to_lowercase().contains(&ql) || v.to_lowercase().contains(&ql))
        })
        .take(limit as usize)
        .collect();

    if matches.is_empty() {
        return Ok(format!("No entries matched '{}'.", query));
    }

    let mut out = format!("Found {} match(es) for '{}':\n\n", matches.len(), query);
    for e in matches {
        let cn = entry_attr(e, "cn").unwrap_or("");
        let uid = entry_attr(e, "uid").unwrap_or("");
        let mail = entry_attr(e, "mail").unwrap_or("");
        out.push_str(&format!("  DN:   {}\n", e.dn));
        if !cn.is_empty() {
            out.push_str(&format!("  cn:   {}\n", cn));
        }
        if !uid.is_empty() {
            out.push_str(&format!("  uid:  {}\n", uid));
        }
        if !mail.is_empty() {
            out.push_str(&format!("  mail: {}\n", mail));
        }
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

fn action_attrs(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let entries = parse_ldif(&text);
    let attr_filter = args.get("attr").and_then(|v| v.as_str());

    let mut attr_map: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for e in &entries {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (k, _) in &e.attributes {
            if attr_filter.map_or(true, |f| k.to_lowercase().contains(&f.to_lowercase())) {
                if seen.insert(k.clone()) {
                    *attr_map.entry(k.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    if attr_map.is_empty() {
        return Ok("No attributes found.".to_string());
    }

    let total = entries.len().max(1);
    let mut out = format!("{:<30} {:<8} {}\n", "ATTRIBUTE", "COUNT", "COVERAGE");
    out.push_str(&format!("{}\n", "-".repeat(55)));
    let mut pairs: Vec<(&String, &usize)> = attr_map.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1));
    for (k, c) in pairs {
        let pct = (*c * 100) / total;
        let bar: String = "#".repeat(pct / 5);
        out.push_str(&format!("{:<30} {:<8} {}% {}\n", k, c, pct, bar));
    }
    Ok(out)
}

fn action_schema(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let entries = parse_ldif(&text);

    // detect schema entries (cn=schema subschema, objectclasses, attributetypes)
    let schema_entries: Vec<&LdifEntry> = entries
        .iter()
        .filter(|e| {
            e.dn.to_lowercase().contains("schema") || e.dn.to_lowercase().contains("subschema")
        })
        .collect();

    if schema_entries.is_empty() {
        // fall back to listing objectClasses found in data
        let mut oc_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for e in &entries {
            for c in entry_classes(e) {
                oc_set.insert(c);
            }
        }
        if oc_set.is_empty() {
            return Ok("No schema entries or objectClass attributes found.".to_string());
        }
        let mut out = "## Object Classes Found in Data\n\n".to_string();
        for oc in &oc_set {
            out.push_str(&format!("  {}\n", oc));
        }
        return Ok(out);
    }

    let mut out = String::new();
    for e in schema_entries {
        out.push_str(&format!("## {}\n", e.dn));
        for (k, v) in &e.attributes {
            out.push_str(&format!(
                "   {}: {}\n",
                k,
                v.chars().take(120).collect::<String>()
            ));
        }
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

fn action_summary(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let entries = parse_ldif(&text);

    let total = entries.len();
    let with_changetype = entries.iter().filter(|e| e.changetype.is_some()).count();
    let mut oc_map: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut attr_count = 0usize;

    for e in &entries {
        attr_count += e.attributes.len();
        for c in entry_classes(e) {
            *oc_map.entry(c).or_insert(0) += 1;
        }
    }

    let mut out = String::new();
    out.push_str("## LDIF Summary\n\n");
    out.push_str(&format!("  Entries:        {}\n", total));
    if with_changetype > 0 {
        out.push_str(&format!(
            "  With changetype: {} (modification records)\n",
            with_changetype
        ));
    }
    out.push_str(&format!(
        "  Avg attributes: {:.1}\n\n",
        if total > 0 {
            attr_count as f64 / total as f64
        } else {
            0.0
        }
    ));

    if !oc_map.is_empty() {
        out.push_str("## Object Classes\n\n");
        let mut oc_vec: Vec<(&String, &usize)> = oc_map.iter().collect();
        oc_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (oc, cnt) in oc_vec.iter().take(15) {
            out.push_str(&format!("  {:30} {}\n", oc, cnt));
        }
    }

    // DC hierarchy from DNs
    let mut dc_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for e in &entries {
        let dn_lower = e.dn.to_lowercase();
        for part in dn_lower.split(',') {
            let p = part.trim();
            if p.starts_with("dc=") {
                dc_set.insert(p[3..].to_string());
            }
        }
    }
    if !dc_set.is_empty() {
        out.push_str(&format!(
            "\n## Directory Domains\n\n  {}\n",
            dc_set.into_iter().collect::<Vec<_>>().join(".")
        ));
    }
    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "search" => action_search(args),
        "attrs" => action_attrs(args),
        "schema" => action_schema(args),
        "summary" => action_summary(args),
        _ => action_parse(args),
    }
}
