use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "name": "multipart_tools",
        "description": "Parses, inspects, and builds multipart/form-data (RFC 2046) bodies without external utilities. \
Actions: parse (default — tabular part summary), parts (detailed per-part with headers and body preview), \
files (only file-upload parts with filename), form (only non-file form fields as name=value), \
validate (RFC 2046 compliance checks), build (generate a multipart body from a fields array). \
Input: body/text for inline content or file for a path; boundary explicit or auto-detected from \
content_type header string or first 1 KB of body. \
Example: multipart_tools(body: '...', boundary: 'abc123') or multipart_tools(action: 'files', file: 'upload.bin') \
or multipart_tools(action: 'build', boundary: 'abc', fields: [{name: 'user', value: 'alice'}, {name: 'data', value: '...', filename: 'data.csv', content_type: 'text/csv'}]).",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "parse|parts|files|form|validate|build" },
                "body": { "type": "string", "description": "Raw multipart body text" },
                "text": { "type": "string", "description": "Alias for body" },
                "file": { "type": "string", "description": "Path to a file containing the multipart body" },
                "boundary": { "type": "string", "description": "Boundary string (without --)" },
                "content_type": { "type": "string", "description": "Full Content-Type header to extract boundary from" },
                "fields": { "type": "array", "description": "For build: array of {name, value, filename?, content_type?} objects" }
            },
            "required": []
        }
    })
}

struct Part {
    headers: Vec<(String, String)>,
    name: Option<String>,
    filename: Option<String>,
    content_type: String,
    transfer_encoding: Option<String>,
    body: Vec<u8>,
}

fn extract_boundary(args: &Value) -> Option<String> {
    // Explicit boundary arg
    if let Some(b) = args.get("boundary").and_then(|v| v.as_str()) {
        return Some(b.trim().to_string());
    }
    // From content_type header
    if let Some(ct) = args.get("content_type").and_then(|v| v.as_str()) {
        if let Some(b) = parse_boundary_from_ct(ct) {
            return Some(b);
        }
    }
    // Auto-detect from body
    let body = get_body(args)?;
    let head = &body[..body.len().min(1024)];
    let s = String::from_utf8_lossy(head);
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("--") {
            let candidate = rest.trim_end_matches('-').trim();
            if !candidate.is_empty() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

fn parse_boundary_from_ct(ct: &str) -> Option<String> {
    for part in ct.split(';') {
        let p = part.trim();
        if let Some(b) = p.strip_prefix("boundary=") {
            return Some(b.trim_matches('"').to_string());
        }
    }
    None
}

fn get_body(args: &Value) -> Option<Vec<u8>> {
    if let Some(s) = args.get("body").or_else(|| args.get("text")).and_then(|v| v.as_str()) {
        return Some(s.as_bytes().to_vec());
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read(path).ok();
    }
    None
}

fn parse_parts(body: &[u8], boundary: &str) -> Vec<Part> {
    let delimiter = format!("--{}", boundary);
    let final_delimiter = format!("--{}--", boundary);
    let body_str = String::from_utf8_lossy(body);
    let mut parts = Vec::new();

    let segments: Vec<&str> = body_str.split(delimiter.as_str()).collect();
    // Skip first (preamble), last may be epilogue
    for seg in segments.iter().skip(1) {
        // Skip final delimiter segment
        let trimmed = seg.trim_start_matches("\r\n").trim_start_matches('\n');
        if trimmed.starts_with("--") {
            break;
        }
        if trimmed.starts_with("--") || *seg == "--\r\n" || *seg == "--\n" || seg.starts_with("--") {
            break;
        }
        let seg_bytes = trimmed.as_bytes();
        // Split headers from body on double CRLF or LF
        let split_pos = find_header_body_split(seg_bytes);
        if let Some(pos) = split_pos {
            let header_str = String::from_utf8_lossy(&seg_bytes[..pos]);
            let body_start = pos + if seg_bytes.get(pos..pos + 2) == Some(b"\r\n") { 4 } else { 2 };
            let body_bytes = if body_start <= seg_bytes.len() {
                seg_bytes[body_start..].to_vec()
            } else {
                vec![]
            };
            // Strip trailing CRLF from body
            let body_clean = strip_trailing_crlf(&body_bytes);
            let headers = parse_headers(&header_str);
            let name = extract_disposition_param(&headers, "name");
            let filename = extract_disposition_param(&headers, "filename");
            let content_type = headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                .map(|(_, v)| v.trim().to_string())
                .unwrap_or_else(|| "text/plain".to_string());
            let transfer_encoding = headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-transfer-encoding"))
                .map(|(_, v)| v.trim().to_string());
            parts.push(Part { headers, name, filename, content_type, transfer_encoding, body: body_clean });
        }
    }
    parts
}

fn find_header_body_split(data: &[u8]) -> Option<usize> {
    // Look for \r\n\r\n
    for i in 0..data.len().saturating_sub(3) {
        if &data[i..i + 4] == b"\r\n\r\n" {
            return Some(i + 2); // position of second \r\n
        }
    }
    // Fallback: \n\n
    for i in 0..data.len().saturating_sub(1) {
        if data[i] == b'\n' && data[i + 1] == b'\n' {
            return Some(i + 1);
        }
    }
    None
}

fn strip_trailing_crlf(data: &[u8]) -> Vec<u8> {
    let mut end = data.len();
    while end > 0 && (data[end - 1] == b'\r' || data[end - 1] == b'\n') {
        end -= 1;
    }
    data[..end].to_vec()
}

fn parse_headers(header_str: &str) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    for line in header_str.lines() {
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_string();
            let val = line[colon + 1..].trim().to_string();
            if !key.is_empty() {
                headers.push((key, val));
            }
        }
    }
    headers
}

fn extract_disposition_param(headers: &[(String, String)], param: &str) -> Option<String> {
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("content-disposition") {
            for part in v.split(';') {
                let p = part.trim();
                if let Some(rest) = p.strip_prefix(&format!("{}=", param)) {
                    return Some(rest.trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

fn body_preview(body: &[u8], limit: usize) -> String {
    let s = String::from_utf8_lossy(body);
    let preview: String = s.chars().take(limit).collect();
    if body.len() > limit {
        format!("{}…", preview.replace(['\r', '\n'], " "))
    } else {
        preview.replace(['\r', '\n'], " ")
    }
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("parse");

    if action == "build" {
        return Ok(build(args));
    }

    let body = match get_body(args) {
        Some(b) => b,
        None => return Ok("Error: provide body/text/file with multipart content.".to_string()),
    };

    let boundary = match extract_boundary(args) {
        Some(b) => b,
        None => return Ok("Error: could not detect boundary. Provide boundary= or content_type= with boundary parameter.".to_string()),
    };

    let parts = parse_parts(&body, &boundary);

    Ok(match action {
        "parts" => show_parts(&parts, &boundary),
        "files" => show_files(&parts),
        "form" => show_form(&parts),
        "validate" => validate(&parts, &body, &boundary),
        _ => show_parse(&parts, &boundary),
    })
}

fn show_parse(parts: &[Part], boundary: &str) -> String {
    let mut out = format!("Boundary: {}\nParts: {}\n\n", boundary, parts.len());
    out.push_str(&format!(
        "{:<4} {:<24} {:<20} {:<10} {}\n",
        "#", "Name", "Content-Type", "Size", "Filename"
    ));
    out.push_str(&"-".repeat(80));
    out.push('\n');
    for (i, p) in parts.iter().enumerate() {
        out.push_str(&format!(
            "{:<4} {:<24} {:<20} {:<10} {}\n",
            i + 1,
            p.name.as_deref().unwrap_or("(none)"),
            p.content_type.chars().take(20).collect::<String>(),
            format!("{} B", p.body.len()),
            p.filename.as_deref().unwrap_or("-"),
        ));
    }
    let file_count = parts.iter().filter(|p| p.filename.is_some()).count();
    let form_count = parts.iter().filter(|p| p.filename.is_none()).count();
    out.push('\n');
    out.push_str(&format!("File parts: {}  Form fields: {}\n", file_count, form_count));
    out
}

fn show_parts(parts: &[Part], boundary: &str) -> String {
    let mut out = format!("Boundary: {}\n\n", boundary);
    for (i, p) in parts.iter().enumerate() {
        out.push_str(&format!("─── Part {} ─────────────────────────────\n", i + 1));
        for (k, v) in &p.headers {
            out.push_str(&format!("  {}: {}\n", k, v));
        }
        out.push_str(&format!("  [name]          {}\n", p.name.as_deref().unwrap_or("(none)")));
        if let Some(f) = &p.filename {
            out.push_str(&format!("  [filename]      {}\n", f));
        }
        out.push_str(&format!("  [content-type]  {}\n", p.content_type));
        if let Some(te) = &p.transfer_encoding {
            out.push_str(&format!("  [encoding]      {}\n", te));
        }
        out.push_str(&format!("  [size]          {} bytes\n", p.body.len()));
        out.push_str(&format!("  [body preview]  {}\n\n", body_preview(&p.body, 120)));
    }
    out
}

fn show_files(parts: &[Part]) -> String {
    let files: Vec<&Part> = parts.iter().filter(|p| p.filename.is_some()).collect();
    if files.is_empty() {
        return "No file-upload parts found.".to_string();
    }
    let mut out = format!("File parts: {}\n\n", files.len());
    out.push_str(&format!("{:<4} {:<24} {:<20} {:<10} {}\n", "#", "Field Name", "Content-Type", "Size", "Filename"));
    out.push_str(&"-".repeat(80));
    out.push('\n');
    for (i, p) in files.iter().enumerate() {
        out.push_str(&format!(
            "{:<4} {:<24} {:<20} {:<10} {}\n",
            i + 1,
            p.name.as_deref().unwrap_or("(none)"),
            p.content_type.chars().take(20).collect::<String>(),
            format!("{} B", p.body.len()),
            p.filename.as_deref().unwrap_or("-"),
        ));
    }
    out
}

fn show_form(parts: &[Part]) -> String {
    let fields: Vec<&Part> = parts.iter().filter(|p| p.filename.is_none()).collect();
    if fields.is_empty() {
        return "No non-file form fields found.".to_string();
    }
    let mut out = format!("Form fields: {}\n\n", fields.len());
    for p in fields {
        let val = String::from_utf8_lossy(&p.body);
        out.push_str(&format!(
            "{} = {}\n",
            p.name.as_deref().unwrap_or("(none)"),
            val.chars().take(200).collect::<String>()
        ));
    }
    out
}

fn validate(parts: &[Part], body: &[u8], boundary: &str) -> String {
    let mut issues = Vec::new();
    let body_str = String::from_utf8_lossy(body);

    // Check boundary length
    if boundary.len() > 70 {
        issues.push(format!(
            "Boundary length {} exceeds RFC 2046 maximum of 70 characters",
            boundary.len()
        ));
    }

    // Check final delimiter
    let final_delim = format!("--{}--", boundary);
    if !body_str.contains(&final_delim) {
        issues.push("Missing final delimiter (--boundary--)".to_string());
    }

    // Check each part for Content-Disposition
    for (i, p) in parts.iter().enumerate() {
        let has_disp = p.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-disposition"));
        if !has_disp {
            issues.push(format!("Part {} missing Content-Disposition header", i + 1));
        }
        if p.name.is_none() && p.filename.is_none() {
            issues.push(format!("Part {} has no name or filename in Content-Disposition", i + 1));
        }
    }

    if parts.is_empty() {
        issues.push("No parts found — check boundary string matches actual body".to_string());
    }

    if issues.is_empty() {
        format!(
            "VALID  —  {} part(s), boundary length {}, final delimiter present, all parts have Content-Disposition\n",
            parts.len(),
            boundary.len()
        )
    } else {
        let mut out = format!("INVALID  —  {} issue(s) found:\n\n", issues.len());
        for (i, issue) in issues.iter().enumerate() {
            out.push_str(&format!("  {}. {}\n", i + 1, issue));
        }
        out
    }
}

fn build(args: &Value) -> String {
    let boundary = args
        .get("boundary")
        .and_then(|v| v.as_str())
        .unwrap_or("----FormBoundary7MA4YWxkTrZu0gW");

    let fields = match args.get("fields").and_then(|v| v.as_array()) {
        Some(f) => f,
        None => return "Error: provide fields array with {name, value, filename?, content_type?} objects.".to_string(),
    };

    let mut body = String::new();
    for field in fields {
        let name = field.get("name").and_then(|v| v.as_str()).unwrap_or("field");
        let value = field.get("value").and_then(|v| v.as_str()).unwrap_or("");
        let filename = field.get("filename").and_then(|v| v.as_str());
        let ct = field.get("content_type").and_then(|v| v.as_str());

        body.push_str(&format!("--{}\r\n", boundary));
        if let Some(fname) = filename {
            body.push_str(&format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                name, fname
            ));
            let content_type = ct.unwrap_or("application/octet-stream");
            body.push_str(&format!("Content-Type: {}\r\n", content_type));
        } else {
            body.push_str(&format!("Content-Disposition: form-data; name=\"{}\"\r\n", name));
            if let Some(c) = ct {
                body.push_str(&format!("Content-Type: {}\r\n", c));
            }
        }
        body.push_str("\r\n");
        body.push_str(value);
        body.push_str("\r\n");
    }
    body.push_str(&format!("--{}--\r\n", boundary));

    let mut out = format!(
        "Generated multipart/form-data body\nBoundary: {}\nParts: {}\nTotal size: {} bytes\n\n",
        boundary,
        fields.len(),
        body.len()
    );
    out.push_str("Content-Type header to use:\n");
    out.push_str(&format!("  Content-Type: multipart/form-data; boundary={}\n\n", boundary));
    out.push_str("Body:\n");
    out.push_str(&body);
    out
}
