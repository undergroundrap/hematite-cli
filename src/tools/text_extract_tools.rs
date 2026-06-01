use regex::Regex;
use serde_json::{json, Value};

pub fn text_extract_tools_schema() -> Value {
    json!({
        "name": "text_extract_tools",
        "description": "Extract structured entities from unstructured text without external utilities. Actions: emails, urls, ips (IPv4 and IPv6), phones (US/international), dates (ISO/US/EU formats), uuids, hashes (MD5/SHA-1/SHA-256), all (extract every entity type at once), custom (user-supplied regex pattern). Each action returns a deduplicated list with occurrence counts.",
        "parameters": {
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Input text to extract entities from (required)"
                },
                "action": {
                    "type": "string",
                    "enum": ["emails", "urls", "ips", "phones", "dates", "uuids", "hashes", "all", "custom"],
                    "description": "Entity type to extract (default: all)"
                },
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern for 'custom' action"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case-insensitive matching for 'custom' action (default: false)"
                },
                "unique": {
                    "type": "boolean",
                    "description": "Return only unique matches (default: true)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum matches to return per type (default: 100)"
                }
            },
            "required": ["text"]
        }
    })
}

// ── regex patterns ────────────────────────────────────────────────────────────

fn re_email() -> Regex {
    Regex::new(r"(?i)[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").unwrap()
}

fn re_url() -> Regex {
    Regex::new(r#"(?i)https?://[^\s<>"')\]]+|ftp://[^\s<>"')\]]+"#).unwrap()
}

fn re_ipv4() -> Regex {
    Regex::new(r"\b(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b")
        .unwrap()
}

fn re_ipv6() -> Regex {
    Regex::new(
        r"(?i)(?:[0-9a-f]{1,4}:){7}[0-9a-f]{1,4}|(?:[0-9a-f]{1,4}:){1,7}:|(?:[0-9a-f]{1,4}:){1,6}:[0-9a-f]{1,4}|::(?:[0-9a-f]{1,4}:){0,5}[0-9a-f]{1,4}|::1|::"
    ).unwrap()
}

fn re_phone() -> Regex {
    // US/international: +1-800-555-1234, (800) 555-1234, 800.555.1234, +44 20 7946 0958
    Regex::new(
        r"(?:\+\d{1,3}[\s\-]?)?(?:\(\d{1,4}\)[\s\-]?)?\d{2,5}[\s\-\.]\d{2,5}[\s\-\.]?\d{2,5}",
    )
    .unwrap()
}

fn re_date_iso() -> Regex {
    Regex::new(r"\b\d{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12]\d|3[01])\b").unwrap()
}

fn re_date_us() -> Regex {
    Regex::new(r"\b(?:0?[1-9]|1[0-2])/(?:0?[1-9]|[12]\d|3[01])/(?:\d{2}|\d{4})\b").unwrap()
}

fn re_date_eu() -> Regex {
    Regex::new(r"\b(?:0?[1-9]|[12]\d|3[01])\.(?:0[1-9]|1[0-2])\.(?:\d{2}|\d{4})\b").unwrap()
}

fn re_uuid() -> Regex {
    Regex::new(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b").unwrap()
}

fn re_md5() -> Regex {
    Regex::new(r"(?i)\b[0-9a-f]{32}\b").unwrap()
}

fn re_sha1() -> Regex {
    Regex::new(r"(?i)\b[0-9a-f]{40}\b").unwrap()
}

fn re_sha256() -> Regex {
    Regex::new(r"(?i)\b[0-9a-f]{64}\b").unwrap()
}

// ── extraction helpers ────────────────────────────────────────────────────────

fn extract(re: &Regex, text: &str, dedup: bool, limit: usize) -> Vec<(String, usize)> {
    use std::collections::HashMap;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for m in re.find_iter(text) {
        *counts.entry(m.as_str().to_string()).or_insert(0) += 1;
    }
    if dedup {
        let mut v: Vec<_> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(limit);
        v
    } else {
        // Return in order of first occurrence, capped at limit
        let mut seen = std::collections::HashSet::new();
        let mut v = Vec::new();
        for m in re.find_iter(text) {
            if v.len() >= limit {
                break;
            }
            let s = m.as_str().to_string();
            if seen.insert(s.clone()) {
                let count = *counts.get(&s).unwrap_or(&1);
                v.push((s, count));
            }
        }
        v
    }
}

fn format_section(title: &str, items: &[(String, usize)], total_in_text: usize) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut out = format!("{title} ({total_in_text} found, {} unique)\n", items.len());
    out.push_str(&"─".repeat(50));
    out.push('\n');
    for (val, count) in items {
        if *count > 1 {
            out.push_str(&format!("  {val}  ×{count}\n"));
        } else {
            out.push_str(&format!("  {val}\n"));
        }
    }
    out
}

fn total_matches(re: &Regex, text: &str) -> usize {
    re.find_iter(text).count()
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_emails(text: &str, dedup: bool, limit: usize) -> String {
    let re = re_email();
    let total = total_matches(&re, text);
    let items = extract(&re, text, dedup, limit);
    if items.is_empty() {
        return "No email addresses found.\n".to_string();
    }
    format_section("Email Addresses", &items, total)
}

fn action_urls(text: &str, dedup: bool, limit: usize) -> String {
    let re = re_url();
    let total = total_matches(&re, text);
    let items = extract(&re, text, dedup, limit);
    if items.is_empty() {
        return "No URLs found.\n".to_string();
    }
    format_section("URLs", &items, total)
}

fn action_ips(text: &str, dedup: bool, limit: usize) -> String {
    let re4 = re_ipv4();
    let re6 = re_ipv6();
    let total4 = total_matches(&re4, text);
    let total6 = total_matches(&re6, text);
    let v4 = extract(&re4, text, dedup, limit);
    let v6 = extract(&re6, text, dedup, limit);
    let mut out = String::new();
    if !v4.is_empty() {
        out.push_str(&format_section("IPv4 Addresses", &v4, total4));
    }
    if !v6.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format_section("IPv6 Addresses", &v6, total6));
    }
    if out.is_empty() {
        out.push_str("No IP addresses found.\n");
    }
    out
}

fn action_phones(text: &str, dedup: bool, limit: usize) -> String {
    let re = re_phone();
    let total = total_matches(&re, text);
    let items = extract(&re, text, dedup, limit);
    if items.is_empty() {
        return "No phone numbers found.\n".to_string();
    }
    format_section("Phone Numbers", &items, total)
}

fn action_dates(text: &str, dedup: bool, limit: usize) -> String {
    let re_iso = re_date_iso();
    let re_us = re_date_us();
    let re_eu = re_date_eu();
    let t_iso = total_matches(&re_iso, text);
    let t_us = total_matches(&re_us, text);
    let t_eu = total_matches(&re_eu, text);
    let iso = extract(&re_iso, text, dedup, limit);
    let us = extract(&re_us, text, dedup, limit);
    let eu = extract(&re_eu, text, dedup, limit);
    let mut out = String::new();
    if !iso.is_empty() {
        out.push_str(&format_section("ISO Dates (YYYY-MM-DD)", &iso, t_iso));
    }
    if !us.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format_section("US Dates (MM/DD/YY)", &us, t_us));
    }
    if !eu.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format_section("EU Dates (DD.MM.YY)", &eu, t_eu));
    }
    if out.is_empty() {
        out.push_str("No dates found.\n");
    }
    out
}

fn action_uuids(text: &str, dedup: bool, limit: usize) -> String {
    let re = re_uuid();
    let total = total_matches(&re, text);
    let items = extract(&re, text, dedup, limit);
    if items.is_empty() {
        return "No UUIDs found.\n".to_string();
    }
    format_section("UUIDs", &items, total)
}

fn action_hashes(text: &str, dedup: bool, limit: usize) -> String {
    // Order: SHA-256 (64 hex) > SHA-1 (40 hex) > MD5 (32 hex)
    // Use separate extraction to avoid SHA-1 matching on substrings of SHA-256
    let re256 = re_sha256();
    let re1 = re_sha1();
    let re5 = re_md5();

    // Collect SHA-256 positions to exclude from shorter matches
    let mut sha256_spans: Vec<(usize, usize)> = re256
        .find_iter(text)
        .map(|m| (m.start(), m.end()))
        .collect();

    // For SHA-1, skip matches that are within a SHA-256 span
    let sha1_text: String = {
        let mut t = text.to_string();
        for (start, end) in sha256_spans.iter().rev() {
            let replacement = " ".repeat(end - start);
            t.replace_range(start..end, &replacement);
        }
        t
    };

    // For MD5, skip matches within SHA-256 or SHA-1 spans
    let sha1_spans: Vec<(usize, usize)> = re1
        .find_iter(&sha1_text)
        .map(|m| (m.start(), m.end()))
        .collect();

    let md5_text: String = {
        let mut t = sha1_text.clone();
        let mut all_spans = sha256_spans.clone();
        all_spans.extend_from_slice(&sha1_spans);
        all_spans.sort_by_key(|s| s.0);
        for (start, end) in all_spans.iter().rev() {
            if *end <= t.len() {
                let replacement = " ".repeat(end - start);
                t.replace_range(start..end, &replacement);
            }
        }
        t
    };

    let t256 = total_matches(&re256, text);
    let t1 = total_matches(&re1, &sha1_text);
    let t5 = total_matches(&re5, &md5_text);
    let v256 = extract(&re256, text, dedup, limit);
    let v1 = extract(&re1, &sha1_text, dedup, limit);
    let v5 = extract(&re5, &md5_text, dedup, limit);

    let mut out = String::new();
    if !v256.is_empty() {
        out.push_str(&format_section(
            "SHA-256 Hashes (64 hex chars)",
            &v256,
            t256,
        ));
    }
    if !v1.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format_section("SHA-1 Hashes (40 hex chars)", &v1, t1));
    }
    if !v5.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format_section("MD5 Hashes (32 hex chars)", &v5, t5));
    }
    if out.is_empty() {
        out.push_str("No hashes found.\n");
    }
    out
}

fn action_all(text: &str, dedup: bool, limit: usize) -> String {
    let sections: Vec<String> = vec![
        action_emails(text, dedup, limit),
        action_urls(text, dedup, limit),
        action_ips(text, dedup, limit),
        action_uuids(text, dedup, limit),
        action_dates(text, dedup, limit),
        action_hashes(text, dedup, limit),
    ];

    let non_empty: Vec<&String> = sections.iter().filter(|s| !s.starts_with("No ")).collect();

    if non_empty.is_empty() {
        return "No structured entities found.\n".to_string();
    }

    non_empty
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn action_custom(
    text: &str,
    pattern: &str,
    ci: bool,
    dedup: bool,
    limit: usize,
) -> Result<String, String> {
    let flags = if ci { "(?i)" } else { "" };
    let re =
        Regex::new(&format!("{}{}", flags, pattern)).map_err(|e| format!("Invalid regex: {e}"))?;
    let total = total_matches(&re, text);
    let items = extract(&re, text, dedup, limit);
    if items.is_empty() {
        return Ok(format!("No matches for pattern '{pattern}'.\n"));
    }
    Ok(format_section(
        &format!("Pattern '{pattern}'"),
        &items,
        total,
    ))
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("'text' field is required")?;

    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("all");
    let dedup = args.get("unique").and_then(|v| v.as_bool()).unwrap_or(true);
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

    match action {
        "emails" => Ok(action_emails(text, dedup, limit)),
        "urls" => Ok(action_urls(text, dedup, limit)),
        "ips" => Ok(action_ips(text, dedup, limit)),
        "phones" => Ok(action_phones(text, dedup, limit)),
        "dates" => Ok(action_dates(text, dedup, limit)),
        "uuids" => Ok(action_uuids(text, dedup, limit)),
        "hashes" => Ok(action_hashes(text, dedup, limit)),
        "all" => Ok(action_all(text, dedup, limit)),
        "custom" => {
            let pat = args
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or("'pattern' field is required for custom action")?;
            let ci = args
                .get("case_insensitive")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            action_custom(text, pat, ci, dedup, limit)
        }
        _ => Err(format!(
            "Unknown action '{action}'. Valid: emails, urls, ips, phones, dates, uuids, hashes, all, custom"
        )),
    }
}
