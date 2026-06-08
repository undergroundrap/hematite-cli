use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;

type VProp = (String, HashMap<String, Vec<String>>, String);

pub fn vcf_tools_schema() -> Value {
    json!({
        "name": "vcf_tools",
        "description": "Parse and analyze vCard contact files (.vcf). Supports vCard 2.1, 3.0, and 4.0. Actions: parse (all contacts with full detail), list (names + primary email/phone summary), search (filter contacts by keyword), to_json (structured JSON output), to_csv (tabular CSV export). Pass 'vcf'/'text' for inline content or 'file' for a path.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "list", "search", "to_json", "to_csv"],
                    "description": "parse (default — full contact detail), list (summary table — name/email/phone), search (filter by keyword in any field; pass 'query'), to_json (JSON array of contacts), to_csv (CSV with standard columns)"
                },
                "vcf": {
                    "type": "string",
                    "description": "Inline vCard content"
                },
                "text": {
                    "type": "string",
                    "description": "Alias for vcf"
                },
                "file": {
                    "type": "string",
                    "description": "Path to a .vcf file"
                },
                "query": {
                    "type": "string",
                    "description": "Search keyword for the 'search' action"
                },
                "q": {
                    "type": "string",
                    "description": "Alias for query"
                }
            },
            "required": []
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");

    let raw = if let Some(p) = args.get("file").and_then(|v| v.as_str()) {
        fs::read_to_string(p).map_err(|e| format!("Cannot read file: {e}"))?
    } else if let Some(t) = args
        .get("vcf")
        .or_else(|| args.get("text"))
        .and_then(|v| v.as_str())
    {
        t.to_string()
    } else {
        return Err("Pass 'vcf', 'text', or 'file'.".into());
    };

    let contacts = parse_vcards(&raw)?;

    if contacts.is_empty() {
        return Ok("No vCard records found.".into());
    }

    match action {
        "list" => action_list(&contacts),
        "search" => {
            let query = args
                .get("query")
                .or_else(|| args.get("q"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            action_search(&contacts, query)
        }
        "to_json" => action_to_json(&contacts),
        "to_csv" => action_to_csv(&contacts),
        _ => action_parse(&contacts),
    }
}

// ── vCard data model ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct VCard {
    version: String,
    name: StructuredName,
    full_name: String,
    emails: Vec<TypedValue>,
    phones: Vec<TypedValue>,
    addresses: Vec<TypedValue>,
    organization: String,
    title: String,
    urls: Vec<TypedValue>,
    notes: Vec<String>,
    birthdate: String,
    photo: Option<String>,
    uid: String,
    categories: Vec<String>,
    extra: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
struct StructuredName {
    family: String,
    given: String,
    additional: String,
    prefix: String,
    suffix: String,
}

#[derive(Debug, Clone)]
struct TypedValue {
    types: Vec<String>,
    value: String,
}

// ── Parser ────────────────────────────────────────────────────────────────────

fn parse_vcards(raw: &str) -> Result<Vec<VCard>, String> {
    // unfold lines per RFC 6350 §3.2 (CRLF followed by whitespace = continuation)
    let unfolded = unfold_lines(raw);
    let mut contacts = Vec::new();
    let mut in_card = false;
    let mut props: Vec<(String, HashMap<String, Vec<String>>, String)> = Vec::new();

    for line in unfolded.lines() {
        let line = line.trim_end_matches('\r');
        let upper = line.to_uppercase();
        if upper == "BEGIN:VCARD" {
            in_card = true;
            props.clear();
        } else if upper == "END:VCARD" {
            if in_card {
                contacts.push(build_vcard(&props));
            }
            in_card = false;
        } else if in_card && !line.is_empty() {
            if let Some((name_params, value)) = split_property_line(line) {
                let (name, params) = parse_name_params(name_params);
                props.push((name.to_uppercase(), params, value));
            }
        }
    }

    Ok(contacts)
}

fn unfold_lines(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut first = true;
    for line in raw.lines() {
        if !first && (line.starts_with(' ') || line.starts_with('\t')) {
            // continuation line: drop the leading whitespace
            out.push_str(line.trim_start_matches([' ', '\t']));
        } else {
            if !first {
                out.push('\n');
            }
            out.push_str(line);
            first = false;
        }
    }
    out
}

fn split_property_line(line: &str) -> Option<(&str, String)> {
    // format: NAME[;params]:VALUE
    // careful: colons can appear inside base64 values, params, etc.
    // find first ':' not preceded by escaped chars — simple scan
    let bytes = line.as_bytes();
    let mut in_quote = false;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'"' {
            in_quote = !in_quote;
        }
        if b == b':' && !in_quote {
            let name_params = &line[..i];
            let value = decode_value(&line[i + 1..]);
            return Some((name_params, value));
        }
    }
    None
}

fn decode_value(v: &str) -> String {
    // decode common vCard escaping: \n \N \, \; \\
    v.replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

fn parse_name_params(s: &str) -> (&str, HashMap<String, Vec<String>>) {
    let mut parts = s.splitn(2, ';');
    let name = parts.next().unwrap_or(s);
    let rest = parts.next().unwrap_or("");
    let mut params: HashMap<String, Vec<String>> = HashMap::new();
    for param in rest.split(';') {
        if param.is_empty() {
            continue;
        }
        if let Some((k, v)) = param.split_once('=') {
            let key = k.trim().to_uppercase();
            let vals: Vec<String> = v
                .trim_matches('"')
                .split(',')
                .map(|s| s.trim().to_uppercase())
                .collect();
            params.entry(key).or_default().extend(vals);
        } else {
            // bare type value like ;WORK or ;VOICE
            params
                .entry("TYPE".into())
                .or_default()
                .push(param.trim().to_uppercase());
        }
    }
    (name, params)
}

fn extract_types(params: &HashMap<String, Vec<String>>) -> Vec<String> {
    params.get("TYPE").cloned().unwrap_or_default()
}

fn build_vcard(props: &[VProp]) -> VCard {
    let mut vc = VCard {
        version: String::new(),
        name: StructuredName::default(),
        full_name: String::new(),
        emails: Vec::new(),
        phones: Vec::new(),
        addresses: Vec::new(),
        organization: String::new(),
        title: String::new(),
        urls: Vec::new(),
        notes: Vec::new(),
        birthdate: String::new(),
        photo: None,
        uid: String::new(),
        categories: Vec::new(),
        extra: Vec::new(),
    };

    for (name, params, value) in props {
        match name.as_str() {
            "VERSION" => vc.version = value.trim().to_string(),
            "FN" => vc.full_name = value.trim().to_string(),
            "N" => {
                let parts: Vec<&str> = value.splitn(5, ';').collect();
                vc.name = StructuredName {
                    family: parts.first().unwrap_or(&"").trim().to_string(),
                    given: parts.get(1).unwrap_or(&"").trim().to_string(),
                    additional: parts.get(2).unwrap_or(&"").trim().to_string(),
                    prefix: parts.get(3).unwrap_or(&"").trim().to_string(),
                    suffix: parts.get(4).unwrap_or(&"").trim().to_string(),
                };
            }
            "EMAIL" => vc.emails.push(TypedValue {
                types: extract_types(params),
                value: value.trim().to_string(),
            }),
            "TEL" => vc.phones.push(TypedValue {
                types: extract_types(params),
                value: value.trim().to_string(),
            }),
            "ADR" => {
                // structured: PO Box;Extended;Street;City;Region;PostalCode;Country
                let parts: Vec<&str> = value.splitn(7, ';').collect();
                let street = parts.get(2).unwrap_or(&"").trim();
                let city = parts.get(3).unwrap_or(&"").trim();
                let region = parts.get(4).unwrap_or(&"").trim();
                let postal = parts.get(5).unwrap_or(&"").trim();
                let country = parts.get(6).unwrap_or(&"").trim();
                let formatted = [street, city, region, postal, country]
                    .iter()
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                vc.addresses.push(TypedValue {
                    types: extract_types(params),
                    value: formatted,
                });
            }
            "ORG" => {
                // can be semicolon separated (org;department)
                vc.organization = value.replace(';', " / ").trim().to_string();
            }
            "TITLE" => vc.title = value.trim().to_string(),
            "URL" => vc.urls.push(TypedValue {
                types: extract_types(params),
                value: value.trim().to_string(),
            }),
            "NOTE" => vc.notes.push(value.trim().to_string()),
            "BDAY" => vc.birthdate = value.trim().to_string(),
            "UID" => vc.uid = value.trim().to_string(),
            "CATEGORIES" => {
                vc.categories
                    .extend(value.split(',').map(|s| s.trim().to_string()));
            }
            "PHOTO" => {
                let is_base64 = params
                    .get("ENCODING")
                    .map(|v| v.iter().any(|e| e == "BASE64" || e == "B"))
                    .unwrap_or(false);
                if is_base64 {
                    vc.photo = Some(format!("[embedded photo, {} bytes base64]", value.len()));
                } else {
                    vc.photo = Some(value.trim().to_string());
                }
            }
            _ => {
                // store unrecognized properties as extra (skip noisy internal properties)
                match name.as_str() {
                    "PRODID" | "REV" | "CLASS" | "MAILER" | "SORT-STRING" | "SOUND" | "KEY"
                    | "AGENT" | "LOGO" => {}
                    _ => {
                        let snip = value.chars().take(80).collect::<String>();
                        vc.extra.push((name.clone(), snip));
                    }
                }
            }
        }
    }

    // synthesize full_name from N if FN is missing
    if vc.full_name.is_empty() {
        let parts = [
            vc.name.prefix.as_str(),
            vc.name.given.as_str(),
            vc.name.additional.as_str(),
            vc.name.family.as_str(),
            vc.name.suffix.as_str(),
        ];
        vc.full_name = parts
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
    }

    vc
}

// ── Helper for primary values ──────────────────────────────────────────────────

fn primary_or_first<'a>(items: &'a [TypedValue], prefer_type: &str) -> Option<&'a str> {
    items
        .iter()
        .find(|tv| {
            tv.types
                .iter()
                .any(|t| t.to_uppercase() == prefer_type.to_uppercase())
        })
        .or_else(|| items.first())
        .map(|tv| tv.value.as_str())
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_parse(contacts: &[VCard]) -> Result<String, String> {
    let mut out = format!("VCARDS ({} contact(s))\n", contacts.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for (i, vc) in contacts.iter().enumerate() {
        out.push_str(&format!(
            "\n╔ Contact #{} ─────────────────────────────────\n",
            i + 1
        ));
        out.push_str(&format!(
            "  Name       {}\n",
            if vc.full_name.is_empty() {
                "(unnamed)"
            } else {
                &vc.full_name
            }
        ));
        if !vc.name.family.is_empty() || !vc.name.given.is_empty() {
            let n = format!(
                "{} {} {} {} {}",
                vc.name.prefix, vc.name.given, vc.name.additional, vc.name.family, vc.name.suffix
            )
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
            if n != vc.full_name {
                out.push_str(&format!("  Structured {n}\n"));
            }
        }
        if !vc.organization.is_empty() {
            out.push_str(&format!("  Org        {}\n", vc.organization));
        }
        if !vc.title.is_empty() {
            out.push_str(&format!("  Title      {}\n", vc.title));
        }
        for e in &vc.emails {
            let t = if e.types.is_empty() {
                String::new()
            } else {
                format!(" [{}]", e.types.join(",").to_lowercase())
            };
            out.push_str(&format!("  Email      {}{t}\n", e.value));
        }
        for p in &vc.phones {
            let t = if p.types.is_empty() {
                String::new()
            } else {
                format!(" [{}]", p.types.join(",").to_lowercase())
            };
            out.push_str(&format!("  Phone      {}{t}\n", p.value));
        }
        for a in &vc.addresses {
            let t = if a.types.is_empty() {
                String::new()
            } else {
                format!(" [{}]", a.types.join(",").to_lowercase())
            };
            out.push_str(&format!("  Address    {}{t}\n", a.value));
        }
        for u in &vc.urls {
            let t = if u.types.is_empty() {
                String::new()
            } else {
                format!(" [{}]", u.types.join(",").to_lowercase())
            };
            out.push_str(&format!("  URL        {}{t}\n", u.value));
        }
        if !vc.birthdate.is_empty() {
            out.push_str(&format!("  Birthday   {}\n", vc.birthdate));
        }
        if !vc.categories.is_empty() {
            out.push_str(&format!("  Categories {}\n", vc.categories.join(", ")));
        }
        for note in &vc.notes {
            let snip = note.chars().take(80).collect::<String>();
            out.push_str(&format!("  Note       {snip}\n"));
        }
        if let Some(ph) = &vc.photo {
            out.push_str(&format!("  Photo      {ph}\n"));
        }
        if !vc.uid.is_empty() {
            out.push_str(&format!("  UID        {}\n", vc.uid));
        }
        if !vc.version.is_empty() {
            out.push_str(&format!("  vCard      v{}\n", vc.version));
        }
        for (k, v) in &vc.extra {
            let vs = v.chars().take(60).collect::<String>();
            out.push_str(&format!("  {k:<11}{vs}\n"));
        }
    }
    Ok(out)
}

fn action_list(contacts: &[VCard]) -> Result<String, String> {
    let mut out = format!("{:<30} {:<35} {:<20}\n", "Name", "Email", "Phone");
    out.push_str(&"─".repeat(87));
    out.push('\n');

    for vc in contacts {
        let name = if vc.full_name.is_empty() {
            "(unnamed)".to_string()
        } else {
            vc.full_name.chars().take(28).collect::<String>()
        };
        let email = primary_or_first(&vc.emails, "WORK")
            .or_else(|| primary_or_first(&vc.emails, "HOME"))
            .unwrap_or("")
            .chars()
            .take(33)
            .collect::<String>();
        let phone = primary_or_first(&vc.phones, "CELL")
            .or_else(|| primary_or_first(&vc.phones, "VOICE"))
            .or_else(|| primary_or_first(&vc.phones, "WORK"))
            .unwrap_or("")
            .chars()
            .take(18)
            .collect::<String>();
        out.push_str(&format!("{name:<30} {email:<35} {phone:<20}\n"));
    }
    Ok(out)
}

fn action_search(contacts: &[VCard], query: &str) -> Result<String, String> {
    if query.is_empty() {
        return Err("Pass 'query' for search.".into());
    }
    let ql = query.to_lowercase();
    let matches: Vec<&VCard> = contacts
        .iter()
        .filter(|vc| {
            let haystack = format!(
                "{} {} {} {} {} {} {}",
                vc.full_name,
                vc.organization,
                vc.title,
                vc.emails
                    .iter()
                    .map(|e| e.value.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                vc.phones
                    .iter()
                    .map(|p| p.value.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                vc.addresses
                    .iter()
                    .map(|a| a.value.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                vc.notes.join(" "),
            );
            haystack.to_lowercase().contains(&ql)
        })
        .collect();

    if matches.is_empty() {
        return Ok(format!("No contacts matching \"{query}\"."));
    }
    action_parse(&matches.into_iter().cloned().collect::<Vec<_>>())
}

fn action_to_json(contacts: &[VCard]) -> Result<String, String> {
    let arr: Vec<Value> = contacts
        .iter()
        .map(|vc| {
            json!({
                "full_name": vc.full_name,
                "name": {
                    "family": vc.name.family,
                    "given": vc.name.given,
                    "additional": vc.name.additional,
                    "prefix": vc.name.prefix,
                    "suffix": vc.name.suffix
                },
                "organization": vc.organization,
                "title": vc.title,
                "emails": vc.emails.iter().map(|e| json!({"types": e.types, "value": e.value})).collect::<Vec<_>>(),
                "phones": vc.phones.iter().map(|p| json!({"types": p.types, "value": p.value})).collect::<Vec<_>>(),
                "addresses": vc.addresses.iter().map(|a| json!({"types": a.types, "value": a.value})).collect::<Vec<_>>(),
                "urls": vc.urls.iter().map(|u| json!({"types": u.types, "value": u.value})).collect::<Vec<_>>(),
                "birthday": vc.birthdate,
                "categories": vc.categories,
                "notes": vc.notes,
                "uid": vc.uid,
                "version": vc.version
            })
        })
        .collect();
    Ok(serde_json::to_string_pretty(&arr).unwrap_or_default())
}

fn action_to_csv(contacts: &[VCard]) -> Result<String, String> {
    let mut out = "Full Name,Given,Family,Organization,Title,Email (primary),Phone (primary),Address (primary),Birthday,Categories,URL\n".to_string();
    for vc in contacts {
        let email = primary_or_first(&vc.emails, "WORK")
            .or_else(|| primary_or_first(&vc.emails, "HOME"))
            .unwrap_or("")
            .to_string();
        let phone = primary_or_first(&vc.phones, "CELL")
            .or_else(|| primary_or_first(&vc.phones, "VOICE"))
            .or_else(|| primary_or_first(&vc.phones, "WORK"))
            .unwrap_or("")
            .to_string();
        let addr = primary_or_first(&vc.addresses, "WORK")
            .or_else(|| primary_or_first(&vc.addresses, "HOME"))
            .unwrap_or("")
            .to_string();
        let url = vc.urls.first().map(|u| u.value.as_str()).unwrap_or("");
        let cats = vc.categories.join(";");
        let row = [
            csv_escape(&vc.full_name),
            csv_escape(&vc.name.given),
            csv_escape(&vc.name.family),
            csv_escape(&vc.organization),
            csv_escape(&vc.title),
            csv_escape(&email),
            csv_escape(&phone),
            csv_escape(&addr),
            csv_escape(&vc.birthdate),
            csv_escape(&cats),
            csv_escape(url),
        ];
        out.push_str(&row.join(","));
        out.push('\n');
    }
    Ok(out)
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
