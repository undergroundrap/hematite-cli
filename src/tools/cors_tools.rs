use serde_json::{json, Value};

pub fn cors_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["parse", "validate", "generate", "explain", "preflight"],
                "description": "parse: decode CORS headers | validate: check config for issues | generate: build response headers | explain: plain-English per-header | preflight: simulate OPTIONS preflight"
            },
            "headers": {
                "type": "object",
                "description": "HTTP headers object (key-value string pairs) for parse/explain/validate"
            },
            "origin": {"type": "string", "description": "Requesting origin for preflight/generate"},
            "method": {"type": "string", "description": "HTTP method for preflight"},
            "request_headers": {"type": "string", "description": "Access-Control-Request-Headers value"},
            "allowed_origins": {
                "oneOf": [
                    {"type": "string"},
                    {"type": "array", "items": {"type": "string"}}
                ],
                "description": "Allowed origins: '*', a URL, or array of URLs"
            },
            "allowed_methods": {
                "type": "array", "items": {"type": "string"},
                "description": "Allowed HTTP methods"
            },
            "allowed_headers": {
                "type": "array", "items": {"type": "string"},
                "description": "Allowed request headers"
            },
            "expose_headers": {
                "type": "array", "items": {"type": "string"},
                "description": "Headers exposed to browser"
            },
            "allow_credentials": {"type": "boolean", "description": "Allow credentials (cookies, auth)"},
            "max_age": {"type": "integer", "description": "Preflight cache duration in seconds"}
        },
        "required": []
    })
}

struct CorsConfig {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>,
    expose_headers: Vec<String>,
    allow_credentials: bool,
    max_age: Option<u64>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        CorsConfig {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string(), "OPTIONS".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
            expose_headers: vec![],
            allow_credentials: false,
            max_age: Some(86400),
        }
    }
}

fn parse_cors_headers(headers: &Value) -> (Vec<(String, String)>, Vec<String>) {
    let cors_keys = [
        "access-control-allow-origin",
        "access-control-allow-methods",
        "access-control-allow-headers",
        "access-control-expose-headers",
        "access-control-allow-credentials",
        "access-control-max-age",
        "access-control-request-method",
        "access-control-request-headers",
        "origin",
        "vary",
    ];
    let mut found: Vec<(String, String)> = Vec::new();
    let mut other: Vec<String> = Vec::new();
    if let Some(obj) = headers.as_object() {
        for (k, v) in obj {
            let kl = k.to_lowercase();
            let val = v.as_str().unwrap_or("").to_string();
            if cors_keys.contains(&kl.as_str()) {
                found.push((k.clone(), val));
            } else {
                other.push(format!("{}: {}", k, val));
            }
        }
    }
    (found, other)
}

fn action_parse(args: &Value) -> Result<String, String> {
    let headers = args.get("headers").ok_or("'headers' object required")?;
    let (cors_headers, _) = parse_cors_headers(headers);

    if cors_headers.is_empty() {
        return Ok("No CORS headers found in the provided headers object.".to_string());
    }

    let mut out = String::from("CORS HEADERS\n============\n\n");
    for (name, value) in &cors_headers {
        let key_lower = name.to_lowercase();
        out.push_str(&format!("{}\n  Value : {}\n", name, value));
        let note = match key_lower.as_str() {
            "access-control-allow-origin" => {
                if value == "*" {
                    "  Note  : Wildcard — allows any origin (credentials must be false)".to_string()
                } else {
                    format!("  Note  : Restricts to origin: {}", value)
                }
            }
            "access-control-allow-methods" => {
                format!("  Note  : Methods: {}", value)
            }
            "access-control-allow-headers" => {
                format!("  Note  : Request headers allowed: {}", value)
            }
            "access-control-expose-headers" => {
                format!(
                    "  Note  : Response headers accessible to browser JS: {}",
                    value
                )
            }
            "access-control-allow-credentials" => {
                if value.to_lowercase() == "true" {
                    "  Note  : Cookies and auth headers forwarded — origin must not be *"
                        .to_string()
                } else {
                    "  Note  : Credentials not forwarded".to_string()
                }
            }
            "access-control-max-age" => {
                if let Ok(secs) = value.parse::<u64>() {
                    let h = secs / 3600;
                    let m = (secs % 3600) / 60;
                    format!("  Note  : Preflight cached for {}s ({h}h {m}m)", secs)
                } else {
                    "  Note  : Preflight cache duration".to_string()
                }
            }
            "access-control-request-method" => {
                "  Note  : Preflight request header — intended method".to_string()
            }
            "access-control-request-headers" => {
                "  Note  : Preflight request header — custom headers to be sent".to_string()
            }
            "origin" => format!("  Note  : Requesting origin: {}", value),
            "vary" => "  Note  : Tells caches to vary response by listed headers".to_string(),
            _ => String::new(),
        };
        if !note.is_empty() {
            out.push_str(&note);
            out.push('\n');
        }
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

fn action_validate(args: &Value) -> Result<String, String> {
    let headers = args.get("headers").ok_or("'headers' object required")?;
    let obj = headers.as_object().ok_or("'headers' must be an object")?;

    let get = |key: &str| -> Option<String> {
        obj.iter()
            .find(|(k, _)| k.to_lowercase() == key)
            .map(|(_, v)| v.as_str().unwrap_or("").to_string())
    };

    let mut warnings: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut info: Vec<String> = Vec::new();

    let allow_origin = get("access-control-allow-origin");
    let allow_credentials = get("access-control-allow-credentials");
    let allow_methods = get("access-control-allow-methods");
    let max_age = get("access-control-max-age");
    let expose = get("access-control-expose-headers");

    if allow_origin.is_none() {
        warnings.push(
            "Missing Access-Control-Allow-Origin — cross-origin requests will be blocked"
                .to_string(),
        );
    }

    if let (Some(origin), Some(creds)) = (&allow_origin, &allow_credentials) {
        if origin == "*" && creds.to_lowercase() == "true" {
            errors.push("Access-Control-Allow-Origin: * cannot be combined with Access-Control-Allow-Credentials: true — browsers will block the response".to_string());
        }
    }

    if let Some(origin) = &allow_origin {
        if origin == "*" {
            info.push("Wildcard origin (*) allows any site to make requests — acceptable for public APIs, risky for authenticated endpoints".to_string());
        }
    }

    if let Some(methods) = &allow_methods {
        let methods_upper: Vec<&str> = methods.split(',').map(|m| m.trim()).collect();
        for m in &methods_upper {
            if ![
                "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "CONNECT", "TRACE",
            ]
            .contains(m)
            {
                warnings.push(format!("Unrecognized HTTP method in Allow-Methods: {}", m));
            }
        }
        if methods_upper.contains(&"TRACE") {
            warnings.push(
                "TRACE method in Allow-Methods — potential XST (Cross-Site Tracing) risk"
                    .to_string(),
            );
        }
    }

    if let Some(age) = &max_age {
        if let Ok(secs) = age.parse::<u64>() {
            if secs > 86400 {
                warnings.push(format!(
                    "Access-Control-Max-Age {} seconds exceeds 24h — Chrome caps preflight cache at 7200s",
                    secs
                ));
            }
        } else {
            errors.push(format!(
                "Access-Control-Max-Age must be an integer (got '{}')",
                age
            ));
        }
    }

    if let Some(exp) = &expose {
        let forbidden = ["Set-Cookie", "Set-Cookie2"];
        for h in exp.split(',').map(|h| h.trim()) {
            if forbidden.contains(&h) {
                errors.push(format!(
                    "Exposing '{}' via Access-Control-Expose-Headers is forbidden by spec",
                    h
                ));
            }
        }
    }

    let mut out = String::from("CORS VALIDATION\n===============\n\n");
    if errors.is_empty() && warnings.is_empty() {
        out.push_str("VALID — no issues detected\n\n");
    } else if !errors.is_empty() {
        out.push_str("INVALID — errors present\n\n");
    } else {
        out.push_str("WARNINGS — check before deploying\n\n");
    }

    for e in &errors {
        out.push_str(&format!("ERROR   : {}\n", e));
    }
    for w in &warnings {
        out.push_str(&format!("WARNING : {}\n", w));
    }
    for i in &info {
        out.push_str(&format!("INFO    : {}\n", i));
    }
    Ok(out.trim_end().to_string())
}

fn action_generate(args: &Value) -> Result<String, String> {
    let origin_req = args
        .get("origin")
        .and_then(|v| v.as_str())
        .unwrap_or("https://example.com");

    let allowed_origins: Vec<String> = match args.get("allowed_origins") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => vec!["*".to_string()],
    };

    let allowed_methods: Vec<String> = match args.get("allowed_methods") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec!["GET".to_string(), "POST".to_string(), "OPTIONS".to_string()],
    };

    let allowed_headers: Vec<String> = match args.get("allowed_headers") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec!["Content-Type".to_string(), "Authorization".to_string()],
    };

    let expose_headers: Vec<String> = match args.get("expose_headers") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![],
    };

    let allow_credentials = args
        .get("allow_credentials")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let max_age = args
        .get("max_age")
        .and_then(|v| v.as_u64())
        .unwrap_or(86400);

    // Resolve origin
    let origin_value = if allowed_origins.contains(&"*".to_string()) {
        if allow_credentials {
            return Err(
                "Cannot use wildcard origin with allow_credentials: true — specify explicit origins".to_string(),
            );
        }
        "*".to_string()
    } else if allowed_origins.iter().any(|o| o == origin_req) {
        origin_req.to_string()
    } else {
        return Ok(format!(
            "Origin '{}' is NOT in the allowed origins list — no CORS headers would be sent.\nAllowed: {}",
            origin_req,
            allowed_origins.join(", ")
        ));
    };

    let mut out =
        String::from("GENERATED CORS RESPONSE HEADERS\n================================\n\n");
    out.push_str(&format!("Access-Control-Allow-Origin: {}\n", origin_value));
    if origin_value != "*" {
        out.push_str("Vary: Origin\n");
    }
    out.push_str(&format!(
        "Access-Control-Allow-Methods: {}\n",
        allowed_methods.join(", ")
    ));
    out.push_str(&format!(
        "Access-Control-Allow-Headers: {}\n",
        allowed_headers.join(", ")
    ));
    if !expose_headers.is_empty() {
        out.push_str(&format!(
            "Access-Control-Expose-Headers: {}\n",
            expose_headers.join(", ")
        ));
    }
    if allow_credentials {
        out.push_str("Access-Control-Allow-Credentials: true\n");
    }
    out.push_str(&format!("Access-Control-Max-Age: {}\n", max_age));

    out.push_str("\n--- Config Summary ---\n");
    out.push_str(&format!("Request origin  : {}\n", origin_req));
    out.push_str(&format!("Origin matched  : {}\n", origin_value));
    out.push_str(&format!("Credentials     : {}\n", allow_credentials));
    out.push_str(&format!("Preflight cache : {}s\n", max_age));
    Ok(out.trim_end().to_string())
}

fn action_explain(args: &Value) -> Result<String, String> {
    let headers = args.get("headers").ok_or("'headers' object required")?;
    let obj = headers.as_object().ok_or("'headers' must be an object")?;

    let explanations = [
        (
            "access-control-allow-origin",
            "Tells the browser which origin is allowed to read the response. '*' = any origin. \
             A specific URL (e.g. https://app.example.com) restricts access to that site only. \
             Must match the request's Origin header for non-simple requests."
        ),
        (
            "access-control-allow-methods",
            "Lists the HTTP methods (GET, POST, PUT, DELETE, etc.) the server permits for cross-origin \
             requests. Checked during preflight (OPTIONS). Only relevant when the request method is \
             non-simple (anything beyond GET/POST/HEAD with simple headers)."
        ),
        (
            "access-control-allow-headers",
            "Lists the request headers the server permits. Any header beyond the 'simple' set \
             (Accept, Accept-Language, Content-Language, Content-Type with certain values) must \
             appear here to be allowed in the actual request."
        ),
        (
            "access-control-expose-headers",
            "Lists response headers that browser JavaScript is allowed to read. By default the browser \
             only exposes Cache-Control, Content-Language, Content-Type, Expires, Last-Modified, Pragma. \
             Any other header (e.g. X-Request-Id) must be listed here."
        ),
        (
            "access-control-allow-credentials",
            "When 'true', tells the browser to include cookies, TLS client certificates, and HTTP auth \
             with cross-origin requests. The requesting code must also set credentials: 'include'. \
             Requires Allow-Origin to name a specific origin — wildcard (*) is forbidden when this is true."
        ),
        (
            "access-control-max-age",
            "Seconds the preflight response may be cached. Avoids a round-trip OPTIONS check before \
             every request during that window. Chrome caps this at 7200s (2h); Firefox allows up to 86400s (24h)."
        ),
        (
            "access-control-request-method",
            "Sent by the browser in preflight (OPTIONS) requests. Tells the server which HTTP method \
             the actual request will use. The server checks this against its Allow-Methods list."
        ),
        (
            "access-control-request-headers",
            "Sent by the browser in preflight (OPTIONS) requests. Lists any custom headers the actual \
             request will include. The server checks this against its Allow-Headers list."
        ),
        (
            "origin",
            "Sent by the browser with every cross-origin request. Contains the scheme, host, and port \
             of the requesting page. Cannot be set by JavaScript — the browser controls this value."
        ),
        (
            "vary",
            "Instructs CDNs and caches to store separate responses per listed header. \
             'Vary: Origin' is critical when Allow-Origin returns specific origins (not *), \
             otherwise a CDN may serve a cached response with the wrong origin to other callers."
        ),
    ];

    let mut out = String::from("CORS HEADER EXPLANATIONS\n========================\n\n");
    let mut found = false;
    for (k, v) in obj {
        let kl = k.to_lowercase();
        if let Some((_, explanation)) = explanations.iter().find(|(key, _)| *key == kl.as_str()) {
            found = true;
            let val = v.as_str().unwrap_or("");
            out.push_str(&format!(
                "{}\n  Value       : {}\n  Explanation : {}\n\n",
                k, val, explanation
            ));
        }
    }

    if !found {
        out.push_str("No recognized CORS headers found in the provided object.\n\n");
        out.push_str("CORS headers start with 'Access-Control-' or include 'Origin' / 'Vary'.\n");
    }
    Ok(out.trim_end().to_string())
}

fn action_preflight(args: &Value) -> Result<String, String> {
    let origin = args
        .get("origin")
        .and_then(|v| v.as_str())
        .unwrap_or("https://app.example.com");

    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("POST");

    let request_headers = args
        .get("request_headers")
        .and_then(|v| v.as_str())
        .unwrap_or("Content-Type, Authorization");

    let allowed_origins: Vec<String> = match args.get("allowed_origins") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => vec!["*".to_string()],
    };

    let allowed_methods: Vec<String> = match args.get("allowed_methods") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![
            "GET".to_string(),
            "POST".to_string(),
            "PUT".to_string(),
            "DELETE".to_string(),
            "OPTIONS".to_string(),
        ],
    };

    let allowed_headers: Vec<String> = match args.get("allowed_headers") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec!["Content-Type".to_string(), "Authorization".to_string()],
    };

    let allow_credentials = args
        .get("allow_credentials")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let max_age = args
        .get("max_age")
        .and_then(|v| v.as_u64())
        .unwrap_or(86400);

    let mut out = String::from("CORS PREFLIGHT SIMULATION\n=========================\n\n");
    out.push_str("--- Preflight Request (Browser sends) ---\n");
    out.push_str(&format!("OPTIONS /api/endpoint HTTP/1.1\n"));
    out.push_str(&format!("Origin: {}\n", origin));
    out.push_str(&format!("Access-Control-Request-Method: {}\n", method));
    out.push_str(&format!(
        "Access-Control-Request-Headers: {}\n\n",
        request_headers
    ));

    out.push_str("--- Server Checks ---\n");

    // Check origin
    let origin_ok =
        allowed_origins.contains(&"*".to_string()) || allowed_origins.iter().any(|o| o == origin);
    out.push_str(&format!(
        "Origin '{}': {}\n",
        origin,
        if origin_ok {
            "ALLOWED ✓"
        } else {
            "BLOCKED ✗"
        }
    ));

    // Check method
    let method_ok = allowed_methods
        .iter()
        .any(|m| m.eq_ignore_ascii_case(method));
    out.push_str(&format!(
        "Method '{}': {}\n",
        method,
        if method_ok {
            "ALLOWED ✓"
        } else {
            "BLOCKED ✗"
        }
    ));

    // Check headers
    let req_hdrs: Vec<&str> = request_headers.split(',').map(|h| h.trim()).collect();
    let mut headers_ok = true;
    for h in &req_hdrs {
        let allowed = allowed_headers.iter().any(|ah| ah.eq_ignore_ascii_case(h));
        out.push_str(&format!(
            "Header '{}': {}\n",
            h,
            if allowed {
                "ALLOWED ✓"
            } else {
                "BLOCKED ✗"
            }
        ));
        if !allowed {
            headers_ok = false;
        }
    }

    let overall_ok = origin_ok && method_ok && headers_ok;
    out.push_str(&format!(
        "\nPREFLIGHT RESULT: {}\n\n",
        if overall_ok {
            "PASS — actual request will proceed"
        } else {
            "FAIL — actual request blocked"
        }
    ));

    if overall_ok {
        out.push_str("--- Preflight Response (Server sends) ---\n");
        out.push_str("HTTP/1.1 204 No Content\n");
        let origin_val = if allowed_origins.contains(&"*".to_string()) && !allow_credentials {
            "*".to_string()
        } else {
            origin.to_string()
        };
        out.push_str(&format!("Access-Control-Allow-Origin: {}\n", origin_val));
        out.push_str(&format!(
            "Access-Control-Allow-Methods: {}\n",
            allowed_methods.join(", ")
        ));
        out.push_str(&format!(
            "Access-Control-Allow-Headers: {}\n",
            allowed_headers.join(", ")
        ));
        if allow_credentials {
            out.push_str("Access-Control-Allow-Credentials: true\n");
        }
        out.push_str(&format!("Access-Control-Max-Age: {}\n", max_age));
        if origin_val != "*" {
            out.push_str("Vary: Origin\n");
        }
    }

    Ok(out.trim_end().to_string())
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "parse" => action_parse(args),
        "validate" => action_validate(args),
        "generate" => action_generate(args),
        "explain" => action_explain(args),
        "preflight" => action_preflight(args),
        other => Err(format!(
            "Unknown action '{}'. Use: parse, validate, generate, explain, preflight",
            other
        )),
    }
}
