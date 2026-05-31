use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    let raw = get_email(args)?;
    match action {
        "parse" | "" => action_parse(&raw, args),
        "headers" => action_headers(&raw, args),
        "structure" => action_structure(&raw),
        "trace" => action_trace(&raw),
        other => Err(format!(
            "email_tools: unknown action '{other}'. Valid: parse, headers, structure, trace"
        )),
    }
}

// ── Input resolution ─────────────────────────────────────────────────────────

fn get_email(args: &Value) -> Result<String, String> {
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path)
            .map_err(|e| format!("email_tools: cannot read '{path}': {e}"));
    }
    for field in &["text", "email", "eml", "raw", "input"] {
        if let Some(s) = args.get(field).and_then(|v| v.as_str()) {
            return Ok(s.to_string());
        }
    }
    Err("email_tools: provide 'file' (path to .eml) or 'text' (raw email content)".to_string())
}

// ── Header parsing ───────────────────────────────────────────────────────────

fn split_headers_body(raw: &str) -> (&str, &str) {
    // Try \r\n\r\n first (RFC 2822 canonical), then \n\n
    if let Some(pos) = raw.find("\r\n\r\n") {
        (&raw[..pos], &raw[pos + 4..])
    } else if let Some(pos) = raw.find("\n\n") {
        (&raw[..pos], &raw[pos + 2..])
    } else {
        (raw, "")
    }
}

fn parse_headers(header_section: &str) -> Vec<(String, String)> {
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut current_name = String::new();
    let mut current_value = String::new();

    for line in header_section.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with(' ') || line.starts_with('\t') {
            // Folded continuation (RFC 2822 §2.2.3)
            if !current_name.is_empty() {
                current_value.push(' ');
                current_value.push_str(line.trim());
            }
        } else if let Some(colon) = line.find(':') {
            // Save previous
            if !current_name.is_empty() {
                headers.push((current_name.clone(), current_value.trim().to_string()));
            }
            current_name = line[..colon].trim().to_string();
            current_value = line[colon + 1..].trim().to_string();
        }
    }
    if !current_name.is_empty() {
        headers.push((current_name, current_value.trim().to_string()));
    }
    headers
}

fn get_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    let name_lc = name.to_lowercase();
    headers
        .iter()
        .find(|(n, _)| n.to_lowercase() == name_lc)
        .map(|(_, v)| v.as_str())
}

fn get_all_headers<'a>(headers: &'a [(String, String)], name: &str) -> Vec<&'a str> {
    let name_lc = name.to_lowercase();
    headers
        .iter()
        .filter(|(n, _)| n.to_lowercase() == name_lc)
        .map(|(_, v)| v.as_str())
        .collect()
}

// ── RFC 2047 encoded-word decoding ───────────────────────────────────────────

fn decode_encoded_words(s: &str) -> String {
    let mut result = String::new();
    let mut remaining = s;

    while let Some(start) = remaining.find("=?") {
        result.push_str(&remaining[..start]);
        let tail = &remaining[start + 2..];

        // charset?encoding?text?=
        if let Some(q1) = tail.find('?') {
            let charset = &tail[..q1];
            let tail2 = &tail[q1 + 1..];
            if let Some(q2) = tail2.find('?') {
                let enc = &tail2[..q2];
                let tail3 = &tail2[q2 + 1..];
                if let Some(end) = tail3.find("?=") {
                    let encoded = &tail3[..end];
                    let decoded = match enc.to_uppercase().as_str() {
                        "B" => decode_b64_word(encoded),
                        "Q" => decode_qp_word(encoded),
                        _ => format!("=?{charset}?{enc}?{encoded}?="),
                    };
                    result.push_str(&decoded);
                    remaining = &tail3[end + 2..];
                    continue;
                }
            }
        }

        result.push_str("=?");
        remaining = &remaining[2..];
    }

    result.push_str(remaining);
    result
}

fn decode_b64_word(s: &str) -> String {
    let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    match decode_base64(clean.as_bytes()) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(_) => s.to_string(),
    }
}

fn decode_qp_word(s: &str) -> String {
    // Quoted-printable for encoded-words: _ = space, =XX = hex byte
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'_' {
            out.push(b' ');
            i += 1;
        } else if bytes[i] == b'=' && i + 2 < bytes.len() {
            if let Ok(hi) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(hi);
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn decode_base64(data: &[u8]) -> Result<Vec<u8>, ()> {
    const TABLE: [i8; 256] = {
        let mut t = [-1i8; 256];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0usize;
        while i < chars.len() {
            t[chars[i] as usize] = i as i8;
            i += 1;
        }
        t
    };

    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0u32;

    for &b in data {
        if b == b'=' {
            break;
        }
        let v = TABLE[b as usize];
        if v < 0 {
            continue;
        }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

// ── MIME helpers ─────────────────────────────────────────────────────────────

struct MimePart {
    content_type: String,
    encoding: String,
    disposition: String,
    filename: String,
    size: usize,
    is_attachment: bool,
    parts: Vec<MimePart>,
}

fn parse_content_type(value: &str) -> (String, Vec<(String, String)>) {
    let mut iter = value.splitn(2, ';');
    let media_type = iter.next().unwrap_or("").trim().to_lowercase();
    let mut params = Vec::new();
    if let Some(rest) = iter.next() {
        for param in rest.split(';') {
            let p = param.trim();
            if let Some(eq) = p.find('=') {
                let key = p[..eq].trim().to_lowercase();
                let val = p[eq + 1..].trim().trim_matches('"').to_string();
                params.push((key, val));
            }
        }
    }
    (media_type, params)
}

fn get_param<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn parse_mime_structure(raw: &str, depth: usize) -> Vec<MimePart> {
    if depth > 4 {
        return Vec::new();
    }

    let (header_sec, body) = split_headers_body(raw);
    let headers = parse_headers(header_sec);

    let ct_raw = get_header(&headers, "Content-Type").unwrap_or("text/plain");
    let (media_type, ct_params) = parse_content_type(ct_raw);
    let encoding = get_header(&headers, "Content-Transfer-Encoding")
        .unwrap_or("")
        .to_lowercase();
    let cd_raw = get_header(&headers, "Content-Disposition").unwrap_or("");
    let is_attachment = cd_raw.to_lowercase().starts_with("attachment");
    let filename = if let Some(pos) = cd_raw.to_lowercase().find("filename") {
        let tail = &cd_raw[pos + 8..];
        if let Some(eq) = tail.find('=') {
            tail[eq + 1..].trim().trim_matches('"').to_string()
        } else {
            String::new()
        }
    } else {
        get_param(&ct_params, "name").unwrap_or("").to_string()
    };

    if media_type.starts_with("multipart/") {
        let boundary = get_param(&ct_params, "boundary").unwrap_or("");
        let delimiter = format!("--{boundary}");
        let end_delimiter = format!("--{boundary}--");

        let mut sub_parts = Vec::new();
        let mut in_part = false;
        let mut part_lines: Vec<&str> = Vec::new();

        for line in body.lines() {
            let line_trimmed = line.trim_end_matches('\r');
            if line_trimmed == end_delimiter {
                if in_part && !part_lines.is_empty() {
                    let part_text = part_lines.join("\n");
                    sub_parts.extend(parse_mime_structure(&part_text, depth + 1));
                }
                break;
            } else if line_trimmed == delimiter {
                if in_part && !part_lines.is_empty() {
                    let part_text = part_lines.join("\n");
                    sub_parts.extend(parse_mime_structure(&part_text, depth + 1));
                    part_lines.clear();
                }
                in_part = true;
            } else if in_part {
                part_lines.push(line);
            }
        }

        return vec![MimePart {
            content_type: media_type,
            encoding,
            disposition: cd_raw.to_string(),
            filename,
            size: body.len(),
            is_attachment,
            parts: sub_parts,
        }];
    }

    vec![MimePart {
        content_type: media_type,
        encoding,
        disposition: cd_raw.to_string(),
        filename,
        size: body.len(),
        is_attachment,
        parts: Vec::new(),
    }]
}

fn render_mime_tree(parts: &[MimePart], indent: usize, out: &mut String) {
    for part in parts {
        let prefix = "  ".repeat(indent);
        let att = if part.is_attachment {
            " [attachment]"
        } else {
            ""
        };
        let fname = if !part.filename.is_empty() {
            format!(" ({})", part.filename)
        } else {
            String::new()
        };
        let enc = if !part.encoding.is_empty() {
            format!(", {}", part.encoding)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{prefix}├─ {}{fname}{att} [{} bytes{}]\n",
            part.content_type, part.size, enc,
        ));
        render_mime_tree(&part.parts, indent + 1, out);
    }
}

// ── Received header parsing ──────────────────────────────────────────────────

struct Hop {
    from: String,
    by: String,
    timestamp: String,
    delay_note: String,
}

fn parse_received(value: &str) -> Hop {
    // Extract 'from X', 'by Y', and the date (after the last semicolon)
    let lc = value.to_lowercase();

    let from = if let Some(start) = lc.find("from ") {
        let tail = &value[start + 5..];
        let end = tail
            .find(" by ")
            .or_else(|| tail.find(';'))
            .unwrap_or(tail.len().min(60));
        tail[..end].trim().to_string()
    } else {
        String::new()
    };

    let by = if let Some(start) = lc.find(" by ") {
        let tail = &value[start + 4..];
        let end = tail
            .find(" with ")
            .or_else(|| tail.find(" for "))
            .or_else(|| tail.find(';'))
            .unwrap_or(tail.len().min(60));
        tail[..end].trim().to_string()
    } else {
        String::new()
    };

    let timestamp = if let Some(semi) = value.rfind(';') {
        value[semi + 1..].trim().to_string()
    } else {
        String::new()
    };

    Hop {
        from,
        by,
        timestamp,
        delay_note: String::new(),
    }
}

// ── Actions ──────────────────────────────────────────────────────────────────

fn action_parse(raw: &str, args: &Value) -> Result<String, String> {
    let (header_sec, body) = split_headers_body(raw);
    let headers = parse_headers(header_sec);

    let preview_len = args.get("preview").and_then(|v| v.as_u64()).unwrap_or(300) as usize;

    let mut out = String::new();
    out.push_str("── Email Summary ───────────────────────────────────────────────\n");

    let key_fields = [
        "From",
        "To",
        "Cc",
        "Subject",
        "Date",
        "Message-ID",
        "Reply-To",
        "In-Reply-To",
        "References",
        "Content-Type",
        "MIME-Version",
        "X-Mailer",
    ];

    for field in &key_fields {
        if let Some(val) = get_header(&headers, field) {
            let decoded = decode_encoded_words(val);
            let truncated = if decoded.len() > 120 {
                format!("{}…", &decoded[..117])
            } else {
                decoded
            };
            out.push_str(&format!("{field:<16} {truncated}\n"));
        }
    }

    // Security headers
    let sec = [
        "DKIM-Signature",
        "Authentication-Results",
        "Received-SPF",
        "ARC-Authentication-Results",
    ];
    let has_sec = sec.iter().any(|h| get_header(&headers, h).is_some());
    if has_sec {
        out.push_str("\n── Authentication ──────────────────────────────────────────────\n");
        for h in &sec {
            if let Some(val) = get_header(&headers, h) {
                let short = if val.len() > 100 { &val[..97] } else { val };
                out.push_str(&format!("{h:<30} {short}…\n"));
            }
        }
    }

    let received = get_all_headers(&headers, "Received");
    if !received.is_empty() {
        out.push_str(&format!(
            "\nDelivery hops: {} (use action:'trace' for details)\n",
            received.len()
        ));
    }

    out.push_str(&format!("\nTotal headers: {}\n", headers.len()));
    out.push_str(&format!("Body size:     {} bytes\n", body.len()));

    if !body.is_empty() {
        let preview = if body.len() > preview_len {
            format!("{}…", &body[..preview_len].replace('\r', ""))
        } else {
            body.replace('\r', "")
        };
        out.push_str("\n── Body Preview ────────────────────────────────────────────────\n");
        out.push_str(&preview);
        out.push('\n');
    }

    Ok(out)
}

fn action_headers(raw: &str, args: &Value) -> Result<String, String> {
    let (header_sec, _) = split_headers_body(raw);
    let headers = parse_headers(header_sec);

    if let Some(name) = args
        .get("name")
        .or_else(|| args.get("header"))
        .and_then(|v| v.as_str())
    {
        let matches: Vec<_> = get_all_headers(&headers, name);
        if matches.is_empty() {
            return Ok(format!("Header '{name}' not found.\n"));
        }
        let mut out = String::new();
        for (i, val) in matches.iter().enumerate() {
            let decoded = decode_encoded_words(val);
            if matches.len() > 1 {
                out.push_str(&format!("[{}] {}\n", i + 1, decoded));
            } else {
                out.push_str(&format!("{decoded}\n"));
            }
        }
        return Ok(out);
    }

    let filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let mut out = String::new();
    out.push_str(&format!("{:<30} {}\n", "Header", "Value"));
    out.push_str(&"-".repeat(80));
    out.push('\n');

    for (name, value) in &headers {
        if let Some(ref f) = filter {
            if !name.to_lowercase().contains(f.as_str())
                && !value.to_lowercase().contains(f.as_str())
            {
                continue;
            }
        }
        let decoded = decode_encoded_words(value);
        let val_display = if decoded.len() > 80 {
            format!("{}…", &decoded[..77])
        } else {
            decoded
        };
        out.push_str(&format!("{name:<30} {val_display}\n"));
    }

    out.push_str(&format!("\nTotal: {} headers\n", headers.len()));
    Ok(out)
}

fn action_structure(raw: &str) -> Result<String, String> {
    let parts = parse_mime_structure(raw, 0);

    let mut out = String::new();
    out.push_str("── MIME Structure ──────────────────────────────────────────────\n");

    if parts.is_empty() {
        out.push_str("No MIME parts detected (plain text email)\n");
        return Ok(out);
    }

    render_mime_tree(&parts, 0, &mut out);

    // Summarize attachments
    fn collect_attachments(parts: &[MimePart]) -> Vec<String> {
        let mut found = Vec::new();
        for p in parts {
            if p.is_attachment || !p.filename.is_empty() {
                let name = if p.filename.is_empty() {
                    format!("[{}]", p.content_type)
                } else {
                    p.filename.clone()
                };
                found.push(format!("{} ({} bytes)", name, p.size));
            }
            found.extend(collect_attachments(&p.parts));
        }
        found
    }

    let attachments = collect_attachments(&parts);
    if !attachments.is_empty() {
        out.push_str("\n── Attachments ─────────────────────────────────────────────────\n");
        for a in &attachments {
            out.push_str(&format!("  • {a}\n"));
        }
    }

    Ok(out)
}

fn action_trace(raw: &str) -> Result<String, String> {
    let (header_sec, _) = split_headers_body(raw);
    let headers = parse_headers(header_sec);

    let received: Vec<_> = get_all_headers(&headers, "Received");
    if received.is_empty() {
        return Ok("No Received: headers found — cannot trace delivery path.\n".to_string());
    }

    let mut out = String::new();
    out.push_str("── Delivery Trace ──────────────────────────────────────────────\n");
    out.push_str(&format!("{} hop(s) detected\n\n", received.len()));

    // Received headers are in reverse-chronological order (newest first)
    for (i, &val) in received.iter().enumerate() {
        let hop_num = received.len() - i;
        let hop = parse_received(val);
        out.push_str(&format!("Hop {hop_num}:\n"));
        if !hop.from.is_empty() {
            out.push_str(&format!("  from : {}\n", hop.from));
        }
        if !hop.by.is_empty() {
            out.push_str(&format!("  by   : {}\n", hop.by));
        }
        if !hop.timestamp.is_empty() {
            out.push_str(&format!("  time : {}\n", hop.timestamp));
        }
        out.push('\n');
    }

    // Additional delivery headers
    let extra_headers = [
        ("Return-Path", "Return path"),
        ("X-Originating-IP", "Originating IP"),
        ("X-Forwarded-To", "Forwarded to"),
        ("X-Original-To", "Original to"),
        ("Delivered-To", "Delivered to"),
    ];
    let mut has_extra = false;
    for (h, label) in &extra_headers {
        if let Some(val) = get_header(&headers, h) {
            if !has_extra {
                out.push_str("── Additional delivery metadata ─────────────────────────────────\n");
                has_extra = true;
            }
            out.push_str(&format!("{label:<20} {val}\n"));
        }
    }

    Ok(out)
}

// ── Schema ───────────────────────────────────────────────────────────────────

pub fn email_tools_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["parse", "headers", "structure", "trace"],
                "description": "Operation: parse (default — key headers + body preview), headers (all headers or specific header), structure (MIME part tree and attachments), trace (delivery hop chain from Received: headers)"
            },
            "file": {
                "type": "string",
                "description": "Path to a .eml file to read"
            },
            "text": {
                "type": "string",
                "description": "Raw email content as a string"
            },
            "name": {
                "type": "string",
                "description": "headers: specific header name to retrieve (e.g. 'From', 'Subject')"
            },
            "filter": {
                "type": "string",
                "description": "headers: substring filter on header names or values"
            },
            "preview": {
                "type": "integer",
                "description": "parse: body preview length in characters (default 300)"
            }
        }
    })
}
