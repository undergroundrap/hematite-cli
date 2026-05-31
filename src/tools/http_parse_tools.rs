use serde_json::Value;
use std::path::PathBuf;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "parse" | "auto" => action_parse(args),
        "request" => action_request(args),
        "response" => action_response(args),
        "headers" => action_headers(args),
        "cookies" => action_cookies(args),
        "auth" => action_auth(args),
        other => Err(format!(
            "http_parse_tools: unknown action '{other}'. \
             Valid: parse, request, response, headers, cookies, auth"
        )),
    }
}

// ── input resolution ─────────────────────────────────────────────────────────

fn resolve_input(args: &Value) -> Result<String, String> {
    if let Some(s) = args
        .get("text")
        .or_else(|| args.get("http"))
        .or_else(|| args.get("message"))
        .and_then(|v| v.as_str())
    {
        return Ok(s.to_string());
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
            PathBuf::from(r)
        } else {
            crate::tools::file_ops::workspace_root()
        };
        let full = if std::path::Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            root.join(path)
        };
        return std::fs::read_to_string(&full)
            .map_err(|e| format!("http_parse_tools: cannot read '{}': {e}", full.display()));
    }
    Err(
        "http_parse_tools: provide 'text'/'http'/'message' (inline HTTP) or 'file' (path)"
            .to_string(),
    )
}

// ── HTTP message structures ───────────────────────────────────────────────────

#[derive(Debug)]
enum MessageKind {
    Request,
    Response,
}

#[derive(Debug)]
struct HttpMessage {
    kind: MessageKind,
    // request fields
    method: Option<String>,
    path: Option<String>,
    query: Vec<(String, String)>,
    // response fields
    status_code: Option<u16>,
    reason: Option<String>,
    // shared
    http_version: String,
    headers: Vec<(String, String)>,
    body: String,
}

const HTTP_METHODS: &[&str] = &[
    "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "CONNECT", "TRACE",
];

fn is_request_first_line(line: &str) -> bool {
    let upper = line.trim().to_uppercase();
    HTTP_METHODS.iter().any(|m| upper.starts_with(m))
}

fn parse_message(raw: &str, force_kind: Option<MessageKind>) -> Result<HttpMessage, String> {
    let mut lines = raw.lines();

    let first = lines.next().ok_or("http_parse_tools: empty input")?.trim();

    let kind = match force_kind {
        Some(k) => k,
        None => {
            if first.to_uppercase().starts_with("HTTP/") {
                MessageKind::Response
            } else if is_request_first_line(first) {
                MessageKind::Request
            } else {
                return Err(format!(
                    "http_parse_tools: cannot detect message type from first line: '{}'",
                    first
                ));
            }
        }
    };

    let mut method = None;
    let mut path_raw = None;
    let mut http_version = String::from("HTTP/1.1");
    let mut status_code = None;
    let mut reason = None;
    let mut query = Vec::new();

    match &kind {
        MessageKind::Request => {
            let parts: Vec<&str> = first.splitn(3, ' ').collect();
            if parts.len() < 2 {
                return Err("http_parse_tools: malformed request line".into());
            }
            method = Some(parts[0].to_uppercase());
            let full_path = parts[1].to_string();
            if let Some(q_pos) = full_path.find('?') {
                let q_str = &full_path[q_pos + 1..];
                for pair in q_str.split('&') {
                    if let Some(eq) = pair.find('=') {
                        let k = url_decode(&pair[..eq]);
                        let v = url_decode(&pair[eq + 1..]);
                        query.push((k, v));
                    } else if !pair.is_empty() {
                        query.push((url_decode(pair), String::new()));
                    }
                }
                path_raw = Some(full_path[..q_pos].to_string());
            } else {
                path_raw = Some(full_path);
            }
            if parts.len() >= 3 {
                http_version = parts[2].to_string();
            }
        }
        MessageKind::Response => {
            // HTTP/1.1 200 OK
            let parts: Vec<&str> = first.splitn(3, ' ').collect();
            if parts.len() < 2 {
                return Err("http_parse_tools: malformed status line".into());
            }
            http_version = parts[0].to_string();
            status_code = parts[1].parse::<u16>().ok().or(Some(0));
            reason = Some(if parts.len() >= 3 {
                parts[2].to_string()
            } else {
                reason_phrase(status_code.unwrap_or(0))
            });
        }
    }

    // Parse headers until blank line
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut body_start = false;
    let remaining: Vec<&str> = lines.collect();
    let mut body_lines: Vec<&str> = Vec::new();

    for (i, line) in remaining.iter().enumerate() {
        if line.trim().is_empty() && !body_start {
            body_start = true;
            body_lines = remaining[i + 1..].to_vec();
            break;
        }
        if !body_start {
            if let Some(colon) = line.find(':') {
                let name = line[..colon].trim().to_string();
                let value = line[colon + 1..].trim().to_string();
                headers.push((name, value));
            } else if line.starts_with(' ') || line.starts_with('\t') {
                // Header continuation (RFC 7230 folding — rare but valid)
                if let Some(last) = headers.last_mut() {
                    last.1.push(' ');
                    last.1.push_str(line.trim());
                }
            }
        }
    }
    if !body_start {
        body_lines = Vec::new();
    }

    let body = body_lines.join("\n");

    Ok(HttpMessage {
        kind,
        method,
        path: path_raw,
        query,
        status_code,
        reason,
        http_version,
        headers,
        body,
    })
}

// ── action handlers ───────────────────────────────────────────────────────────

fn action_parse(args: &Value) -> Result<String, String> {
    let raw = resolve_input(args)?;
    let msg = parse_message(&raw, None)?;
    match msg.kind {
        MessageKind::Request => format_request(&msg),
        MessageKind::Response => format_response(&msg),
    }
}

fn action_request(args: &Value) -> Result<String, String> {
    let raw = resolve_input(args)?;
    let msg = parse_message(&raw, Some(MessageKind::Request))?;
    format_request(&msg)
}

fn action_response(args: &Value) -> Result<String, String> {
    let raw = resolve_input(args)?;
    let msg = parse_message(&raw, Some(MessageKind::Response))?;
    format_response(&msg)
}

fn action_headers(args: &Value) -> Result<String, String> {
    let raw = resolve_input(args)?;
    let msg = parse_message(&raw, None)?;

    let is_response = matches!(msg.kind, MessageKind::Response);
    let kind_label = if is_response { "Response" } else { "Request" };

    let mut out = format!("HTTP {} Headers\n{}\n\n", kind_label, "─".repeat(50));

    out += &format!("{:<32} {}\n", "Header", "Value");
    out += &format!("{}\n", "─".repeat(70));

    for (name, value) in &msg.headers {
        let annotation = header_annotation(name, value);
        if annotation.is_empty() {
            out += &format!("{:<32} {}\n", name, truncate_str(value, 60));
        } else {
            out += &format!(
                "{:<32} {}  → {}\n",
                name,
                truncate_str(value, 40),
                annotation
            );
        }
    }

    out += "\n";

    // Security header check for responses
    if is_response {
        let header_names_lower: Vec<String> =
            msg.headers.iter().map(|(n, _)| n.to_lowercase()).collect();
        let security_headers = [
            ("x-content-type-options", "prevents MIME-sniffing attacks"),
            ("x-frame-options", "prevents clickjacking"),
            (
                "content-security-policy",
                "controls resource loading (XSS mitigation)",
            ),
            ("strict-transport-security", "enforces HTTPS (HSTS)"),
        ];
        let mut missing: Vec<(&str, &str)> = Vec::new();
        for (hdr, reason) in &security_headers {
            if !header_names_lower.iter().any(|n| n == hdr) {
                missing.push((hdr, reason));
            }
        }
        if missing.is_empty() {
            out += "Security headers: all common headers present\n";
        } else {
            out += "Missing security headers:\n";
            for (hdr, reason) in &missing {
                out += &format!("  ✗  {}  — {}\n", hdr, reason);
            }
        }
        out += "\n";
    }

    // Group by category
    out += &format!("Header categories\n{}\n", "─".repeat(50));
    let groups = categorize_headers(&msg.headers);
    for (cat, names) in groups {
        if !names.is_empty() {
            out += &format!("  {}:  {}\n", cat, names.join(", "));
        }
    }

    Ok(out)
}

fn action_cookies(args: &Value) -> Result<String, String> {
    let raw = resolve_input(args)?;
    let msg = parse_message(&raw, None)?;

    let mut out = format!("HTTP Cookies\n{}\n\n", "─".repeat(50));

    // Cookie: header (request)
    let cookie_hdrs: Vec<&str> = msg
        .headers
        .iter()
        .filter(|(n, _)| n.to_lowercase() == "cookie")
        .map(|(_, v)| v.as_str())
        .collect();

    if !cookie_hdrs.is_empty() {
        out += "Request Cookies (Cookie:)\n";
        out += &format!("{}\n", "─".repeat(40));
        for hdr_val in &cookie_hdrs {
            for pair in hdr_val.split(';') {
                let pair = pair.trim();
                if pair.is_empty() {
                    continue;
                }
                if let Some(eq) = pair.find('=') {
                    let name = &pair[..eq];
                    let value = &pair[eq + 1..];
                    out += &format!("  {:<24} = {}\n", name, value);
                } else {
                    out += &format!("  {}\n", pair);
                }
            }
        }
        out += "\n";
    }

    // Set-Cookie: headers (response)
    let set_cookie_hdrs: Vec<&str> = msg
        .headers
        .iter()
        .filter(|(n, _)| n.to_lowercase() == "set-cookie")
        .map(|(_, v)| v.as_str())
        .collect();

    if !set_cookie_hdrs.is_empty() {
        out += "Response Cookies (Set-Cookie:)\n";
        out += &format!("{}\n", "─".repeat(40));
        for hdr_val in &set_cookie_hdrs {
            let (name_val, attrs) = parse_set_cookie(hdr_val);
            out += &format!("  Cookie:  {}\n", name_val);
            for (attr, val) in &attrs {
                out += &format!("    {:<14} {}\n", format!("{}:", attr), val);
            }
            // Security flags
            let attr_names_lower: Vec<String> =
                attrs.iter().map(|(a, _)| a.to_lowercase()).collect();
            let has_httponly = attr_names_lower.contains(&"httponly".to_string());
            let has_secure = attr_names_lower.contains(&"secure".to_string());
            let samesite = attrs
                .iter()
                .find(|(a, _)| a.to_lowercase() == "samesite")
                .map(|(_, v)| v.to_lowercase());

            let mut flags: Vec<&str> = Vec::new();
            if !has_httponly {
                flags.push("Missing HttpOnly — XSS risk");
            }
            if !has_secure {
                flags.push("Missing Secure — sent over plain HTTP");
            }
            if samesite.as_deref() == Some("none") && !has_secure {
                flags.push("SameSite=None without Secure — CSRF risk");
            }
            for f in &flags {
                out += &format!("    ⚠  {}\n", f);
            }
            out += "\n";
        }
    }

    if cookie_hdrs.is_empty() && set_cookie_hdrs.is_empty() {
        out += "No cookie headers found in this message.\n";
    }

    Ok(out)
}

fn action_auth(args: &Value) -> Result<String, String> {
    let raw = resolve_input(args)?;
    let msg = parse_message(&raw, None)?;

    let mut out = format!("HTTP Authentication\n{}\n\n", "─".repeat(50));

    let auth_header_names = [
        "authorization",
        "www-authenticate",
        "x-api-key",
        "x-auth-token",
        "x-access-token",
        "proxy-authorization",
        "proxy-authenticate",
    ];

    let mut found_any = false;

    for (name, value) in &msg.headers {
        let name_lower = name.to_lowercase();
        if auth_header_names.contains(&name_lower.as_str()) {
            found_any = true;
            out += &format!("Header: {}\n", name);
            out += &format!("{}\n", "─".repeat(40));
            analyze_auth_header(&name_lower, value, &mut out);
            out += "\n";
        }
    }

    if !found_any {
        out += "No authentication headers found.\n";
    }

    Ok(out)
}

// ── formatters ────────────────────────────────────────────────────────────────

fn format_request(msg: &HttpMessage) -> Result<String, String> {
    let method = msg.method.as_deref().unwrap_or("UNKNOWN");
    let path = msg.path.as_deref().unwrap_or("/");

    let mut out = format!("HTTP Request\n{}\n\n", "─".repeat(50));

    out += &format!("Method:   {}\n", method);
    out += &format!("Path:     {}\n", path);
    if !msg.query.is_empty() {
        out += "Query parameters:\n";
        for (k, v) in &msg.query {
            out += &format!("  {}  =  {}\n", k, v);
        }
    }
    out += &format!("Version:  {}\n\n", msg.http_version);

    out += &format!("Headers ({}):\n", msg.headers.len());
    out += &format!("{}\n", "─".repeat(40));
    for (name, value) in &msg.headers {
        out += &format!(
            "  {:<30} {}\n",
            format!("{}:", name),
            truncate_str(value, 60)
        );
    }
    out += "\n";

    let body = &msg.body;
    if body.is_empty() {
        out += "Body: (empty)\n";
    } else {
        let ct = header_value(&msg.headers, "content-type");
        let hint = content_type_hint(ct.as_deref().unwrap_or(""));
        out += &format!("Body ({} bytes){}:\n", body.len(), hint);
        out += &format!("{}\n", "─".repeat(40));
        let preview: String = body.chars().take(500).collect();
        out += &preview;
        if body.len() > 500 {
            out += &format!("\n... ({} bytes total)", body.len());
        }
        out += "\n";
    }

    out += "\n";
    out += &format!(
        "Summary: {} headers, {} body bytes\n",
        msg.headers.len(),
        body.len()
    );

    Ok(out)
}

fn format_response(msg: &HttpMessage) -> Result<String, String> {
    let code = msg.status_code.unwrap_or(0);
    let reason = msg.reason.as_deref().unwrap_or_default();

    let mut out = format!("HTTP Response\n{}\n\n", "─".repeat(50));

    out += &format!("Version:     {}\n", msg.http_version);
    out += &format!("Status:      {} {}\n", code, reason);
    out += &format!("Meaning:     {}\n", status_meaning(code));
    out += "\n";

    out += &format!("Headers ({}):\n", msg.headers.len());
    out += &format!("{}\n", "─".repeat(40));
    for (name, value) in &msg.headers {
        out += &format!(
            "  {:<30} {}\n",
            format!("{}:", name),
            truncate_str(value, 60)
        );
    }
    out += "\n";

    // Response-time header if present
    let rt = header_value(&msg.headers, "x-response-time")
        .or_else(|| header_value(&msg.headers, "x-runtime"))
        .or_else(|| header_value(&msg.headers, "x-request-duration"));
    if let Some(rt_val) = rt {
        out += &format!("Response time:  {}\n\n", rt_val);
    }

    // Content analysis
    let ct = header_value(&msg.headers, "content-type");
    let ce = header_value(&msg.headers, "content-encoding");
    if ct.is_some() || ce.is_some() {
        out += "Content:\n";
        if let Some(ct_val) = &ct {
            out += &format!("  Type:      {}\n", ct_val);
        }
        if let Some(ce_val) = &ce {
            out += &format!("  Encoding:  {}\n", ce_val);
        }
        out += "\n";
    }

    let body = &msg.body;
    if body.is_empty() {
        out += "Body: (empty)\n";
    } else {
        let hint = content_type_hint(ct.as_deref().unwrap_or(""));
        out += &format!("Body ({} bytes){}:\n", body.len(), hint);
        out += &format!("{}\n", "─".repeat(40));
        let preview: String = body.chars().take(500).collect();
        out += &preview;
        if body.len() > 500 {
            out += &format!("\n... ({} bytes total)", body.len());
        }
        out += "\n";
    }

    out += "\n";
    let is_redirect = (300..400).contains(&code);
    let is_error = code >= 400;
    out += &format!(
        "Summary: {} headers, {} body bytes{}{}",
        msg.headers.len(),
        body.len(),
        if is_redirect { ", redirect" } else { "" },
        if is_error { ", error response" } else { "" }
    );
    out += "\n";

    Ok(out)
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn header_value(headers: &[(String, String)], name_lower: &str) -> Option<String> {
    headers
        .iter()
        .find(|(n, _)| n.to_lowercase() == name_lower)
        .map(|(_, v)| v.clone())
}

fn header_annotation(name: &str, value: &str) -> String {
    let name_lower = name.to_lowercase();
    let val_lower = value.to_lowercase();
    match name_lower.as_str() {
        "content-type" if val_lower.contains("application/json") => "JSON body".to_string(),
        "content-type" if val_lower.contains("application/x-www-form-urlencoded") => {
            "URL-encoded form data".to_string()
        }
        "content-type" if val_lower.contains("multipart/form-data") => {
            "multipart form upload".to_string()
        }
        "content-type" if val_lower.contains("text/html") => "HTML document".to_string(),
        "content-type" if val_lower.contains("text/plain") => "plain text".to_string(),
        "authorization" if val_lower.starts_with("bearer ") => {
            let token = &value[7..];
            let is_jwt = token.split('.').count() == 3 && token.split('.').all(|p| !p.is_empty());
            if is_jwt {
                "Bearer token (looks like JWT)".to_string()
            } else {
                "Bearer token auth".to_string()
            }
        }
        "authorization" if val_lower.starts_with("basic ") => "Basic auth (base64)".to_string(),
        "authorization" if val_lower.starts_with("digest ") => "Digest auth".to_string(),
        "cache-control" if val_lower.contains("no-cache") || val_lower.contains("no-store") => {
            "caching disabled".to_string()
        }
        "cache-control" if val_lower.contains("max-age=0") => "revalidate always".to_string(),
        "cache-control" if val_lower.contains("public") => "publicly cacheable".to_string(),
        "transfer-encoding" if val_lower == "chunked" => "chunked body streaming".to_string(),
        "content-encoding" if val_lower == "gzip" => "gzip-compressed body".to_string(),
        "content-encoding" if val_lower == "br" => "brotli-compressed body".to_string(),
        "connection" if val_lower == "keep-alive" => "persistent connection".to_string(),
        "connection" if val_lower == "close" => "connection will close after response".to_string(),
        "location" => "redirect target".to_string(),
        "strict-transport-security" => "HSTS — enforces HTTPS".to_string(),
        "x-frame-options" => "clickjacking protection".to_string(),
        "x-content-type-options" => "MIME-sniffing protection".to_string(),
        "access-control-allow-origin" => "CORS policy".to_string(),
        "content-security-policy" => "CSP — controls resource loading".to_string(),
        "set-cookie" => "sets a browser cookie".to_string(),
        "etag" => "cache validation token".to_string(),
        "last-modified" => "resource modification date".to_string(),
        "x-api-key" | "x-auth-token" | "x-access-token" => "API key / auth token".to_string(),
        _ => String::new(),
    }
}

fn categorize_headers(headers: &[(String, String)]) -> Vec<(&'static str, Vec<String>)> {
    let request_hdrs = [
        "accept",
        "accept-encoding",
        "accept-language",
        "authorization",
        "cookie",
        "expect",
        "from",
        "host",
        "if-match",
        "if-modified-since",
        "if-none-match",
        "if-range",
        "if-unmodified-since",
        "max-forwards",
        "proxy-authorization",
        "range",
        "referer",
        "te",
        "user-agent",
    ];
    let response_hdrs = [
        "accept-ranges",
        "age",
        "etag",
        "location",
        "proxy-authenticate",
        "retry-after",
        "server",
        "set-cookie",
        "vary",
        "www-authenticate",
        "x-content-type-options",
        "x-frame-options",
        "x-xss-protection",
        "strict-transport-security",
        "content-security-policy",
        "access-control-allow-origin",
    ];
    let entity_hdrs = [
        "allow",
        "content-encoding",
        "content-language",
        "content-length",
        "content-location",
        "content-md5",
        "content-range",
        "content-type",
        "expires",
        "last-modified",
    ];
    let general_hdrs = [
        "cache-control",
        "connection",
        "date",
        "pragma",
        "trailer",
        "transfer-encoding",
        "upgrade",
        "via",
        "warning",
    ];

    let mut req_names: Vec<String> = Vec::new();
    let mut resp_names: Vec<String> = Vec::new();
    let mut entity_names: Vec<String> = Vec::new();
    let mut general_names: Vec<String> = Vec::new();
    let mut other_names: Vec<String> = Vec::new();

    for (name, _) in headers {
        let nl = name.to_lowercase();
        if request_hdrs.contains(&nl.as_str()) {
            req_names.push(name.clone());
        } else if response_hdrs.contains(&nl.as_str()) {
            resp_names.push(name.clone());
        } else if entity_hdrs.contains(&nl.as_str()) {
            entity_names.push(name.clone());
        } else if general_hdrs.contains(&nl.as_str()) {
            general_names.push(name.clone());
        } else {
            other_names.push(name.clone());
        }
    }

    vec![
        ("Request", req_names),
        ("Response", resp_names),
        ("Entity", entity_names),
        ("General", general_names),
        ("Extension / Custom", other_names),
    ]
}

fn parse_set_cookie(header_val: &str) -> (String, Vec<(String, String)>) {
    let mut parts = header_val.splitn(2, ';');
    let name_val = parts.next().unwrap_or("").trim().to_string();
    let mut attrs: Vec<(String, String)> = Vec::new();

    if let Some(rest) = parts.next() {
        for attr in rest.split(';') {
            let attr = attr.trim();
            if attr.is_empty() {
                continue;
            }
            if let Some(eq) = attr.find('=') {
                attrs.push((
                    attr[..eq].trim().to_string(),
                    attr[eq + 1..].trim().to_string(),
                ));
            } else {
                // Boolean flag like Secure, HttpOnly
                attrs.push((attr.to_string(), "true".to_string()));
            }
        }
    }

    (name_val, attrs)
}

fn analyze_auth_header(name_lower: &str, value: &str, out: &mut String) {
    match name_lower {
        "authorization" => {
            if let Some(rest) = value
                .strip_prefix("Basic ")
                .or_else(|| value.strip_prefix("basic "))
            {
                *out += "Type:  Basic auth\n";
                // Decode base64 — stdlib only
                match simple_b64_decode(rest.trim()) {
                    Some(decoded) => {
                        if let Some(colon) = decoded.find(':') {
                            let username = &decoded[..colon];
                            *out += &format!("User:  {}\n", username);
                            *out += "Pass:  ***\n";
                        } else {
                            *out += &format!("Value: {} (no colon found)\n", decoded);
                        }
                    }
                    None => {
                        *out += &format!("Value: {} (base64 decode failed)\n", rest.trim());
                    }
                }
            } else if let Some(token) = value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("bearer "))
            {
                *out += "Type:  Bearer token\n";
                let is_jwt =
                    token.split('.').count() == 3 && token.split('.').all(|p| !p.is_empty());
                if is_jwt {
                    let preview = if token.len() > 20 {
                        format!("{}...", &token[..20])
                    } else {
                        token.to_string()
                    };
                    *out += &format!("Token: {} (JWT format)\n", preview);
                    let parts: Vec<&str> = token.splitn(3, '.').collect();
                    if let Some(header_b64) = parts.first() {
                        if let Some(decoded) = simple_b64_decode_url(header_b64) {
                            *out += &format!("JWT header: {}\n", decoded.replace('\n', " "));
                        }
                    }
                } else {
                    let preview = if token.len() > 20 {
                        format!("{}...", &token[..20])
                    } else {
                        token.to_string()
                    };
                    *out += &format!("Token: {}\n", preview);
                }
            } else if let Some(digest_val) = value
                .strip_prefix("Digest ")
                .or_else(|| value.strip_prefix("digest "))
            {
                *out += "Type:  Digest auth\n";
                for part in digest_val.split(',') {
                    let part = part.trim();
                    if let Some(eq) = part.find('=') {
                        let k = part[..eq].trim();
                        let v = part[eq + 1..].trim().trim_matches('"');
                        match k {
                            "realm" => *out += &format!("Realm:     {}\n", v),
                            "nonce" => *out += &format!("Nonce:     {}\n", v),
                            "algorithm" => *out += &format!("Algorithm: {}\n", v),
                            "qop" => *out += &format!("QoP:       {}\n", v),
                            _ => {}
                        }
                    }
                }
            } else {
                *out += &format!("Value: {}\n", truncate_str(value, 80));
            }
        }
        "www-authenticate" => {
            *out += "Type:  WWW-Authenticate challenge\n";
            *out += &format!("Value: {}\n", truncate_str(value, 80));
        }
        "x-api-key" | "x-auth-token" | "x-access-token" => {
            *out += "Type:  API key / access token\n";
            let preview = if value.len() > 12 {
                format!("{}...", &value[..12])
            } else {
                value.to_string()
            };
            *out += &format!("Token: {}\n", preview);
        }
        _ => {
            *out += &format!("Value: {}\n", truncate_str(value, 80));
        }
    }
}

fn content_type_hint(ct: &str) -> String {
    let lower = ct.to_lowercase();
    if lower.contains("application/json") {
        " [JSON]".to_string()
    } else if lower.contains("application/x-www-form-urlencoded") {
        " [form-urlencoded]".to_string()
    } else if lower.contains("multipart/form-data") {
        " [multipart]".to_string()
    } else if lower.contains("text/html") {
        " [HTML]".to_string()
    } else if lower.contains("text/xml") || lower.contains("application/xml") {
        " [XML]".to_string()
    } else {
        String::new()
    }
}

fn status_meaning(code: u16) -> &'static str {
    match code {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK — request succeeded",
        201 => "Created — resource created successfully",
        204 => "No Content — success, no body",
        206 => "Partial Content — range request",
        301 => "Moved Permanently — permanent redirect",
        302 => "Found — temporary redirect",
        304 => "Not Modified — use cached version",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request — malformed request syntax",
        401 => "Unauthorized — authentication required",
        403 => "Forbidden — authenticated but not authorized",
        404 => "Not Found — resource does not exist",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict — state conflict",
        410 => "Gone — resource permanently removed",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Entity — validation failed",
        429 => "Too Many Requests — rate limited",
        500 => "Internal Server Error — server-side failure",
        501 => "Not Implemented",
        502 => "Bad Gateway — upstream server error",
        503 => "Service Unavailable — overloaded or down",
        504 => "Gateway Timeout",
        _ if code < 200 => "Informational",
        _ if code < 300 => "Success",
        _ if code < 400 => "Redirect",
        _ if code < 500 => "Client Error",
        _ => "Server Error",
    }
}

fn reason_phrase(code: u16) -> String {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
    .to_string()
}

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte as char);
                    i += 3;
                    continue;
                }
            }
        } else if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Minimal standard-alphabet base64 decoder (no padding required). stdlib only.
fn simple_b64_decode(s: &str) -> Option<String> {
    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let clean: String = s.chars().filter(|c| *c != '=').collect();
    let mut bits: u32 = 0;
    let mut bit_count: u32 = 0;
    let mut bytes: Vec<u8> = Vec::new();
    for ch in clean.chars() {
        let val = alphabet.find(ch)? as u32;
        bits = (bits << 6) | val;
        bit_count += 6;
        if bit_count >= 8 {
            bit_count -= 8;
            bytes.push((bits >> bit_count) as u8);
            bits &= (1 << bit_count) - 1;
        }
    }
    String::from_utf8(bytes).ok()
}

/// URL-safe base64 decoder (- instead of +, _ instead of /).
fn simple_b64_decode_url(s: &str) -> Option<String> {
    let normalized = s.replace('-', "+").replace('_', "/");
    simple_b64_decode(&normalized)
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.min(s.len())])
    }
}
