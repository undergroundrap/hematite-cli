use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["parse", "build", "convert"],
                "description": "Action: parse (default), build (build curl from parts), convert (to Python/Go/JS)"
            },
            "command": { "type": "string", "description": "Full curl command string to parse or convert" },
            "url":     { "type": "string", "description": "URL for build action" },
            "method":  { "type": "string", "description": "HTTP method (default GET)" },
            "headers": { "type": "object", "description": "Headers object {name: value} for build" },
            "data":    { "type": "string", "description": "Request body/data for build" },
            "form":    { "type": "object", "description": "Form fields {name: value} for build (sets -F flags)" },
            "params":  { "type": "object", "description": "Query parameters to append to URL" },
            "auth":    { "type": "string", "description": "Basic auth user:pass" },
            "insecure":{ "type": "boolean", "description": "Add -k / --insecure flag" },
            "follow_redirects": { "type": "boolean", "description": "Add -L flag" },
            "timeout": { "type": "integer", "description": "Timeout in seconds" },
            "output":  { "type": "string", "description": "Output file path (-o)" },
            "language":{ "type": "string", "enum": ["python", "go", "javascript", "js", "node"], "description": "Target language for convert action" }
        }
    })
}

#[derive(Debug, Default, Clone)]
struct CurlRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    data: Option<String>,
    data_binary: Option<String>,
    form: Vec<(String, String)>,
    auth: Option<String>,
    insecure: bool,
    follow_redirects: bool,
    verbose: bool,
    silent: bool,
    include_headers: bool,
    output: Option<String>,
    timeout: Option<u64>,
    max_time: Option<u64>,
    user_agent: Option<String>,
    compressed: bool,
    head_only: bool,
    cookie: Option<String>,
    cookie_jar: Option<String>,
    proxy: Option<String>,
}

fn parse_curl_command(cmd: &str) -> Result<CurlRequest, String> {
    let mut req = CurlRequest::default();
    // Tokenise respecting single/double quotes and backslash continuation
    let tokens = tokenize_shell(cmd)?;

    // Strip leading 'curl' token(s)
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i].eq_ignore_ascii_case("curl") {
            i += 1;
        } else {
            break;
        }
    }

    while i < tokens.len() {
        let tok = &tokens[i];
        match tok.as_str() {
            "-X" | "--request" => {
                i += 1;
                req.method = tokens.get(i).cloned().unwrap_or_default().to_uppercase();
            }
            "-H" | "--header" => {
                i += 1;
                if let Some(hdr) = tokens.get(i) {
                    if let Some(colon) = hdr.find(':') {
                        let name = hdr[..colon].trim().to_string();
                        let val = hdr[colon + 1..].trim().to_string();
                        req.headers.push((name, val));
                    }
                }
            }
            "-d" | "--data" | "--data-ascii" => {
                i += 1;
                req.data = tokens.get(i).cloned();
                if req.method.is_empty() {
                    req.method = "POST".to_string();
                }
            }
            "--data-binary" | "--data-raw" => {
                i += 1;
                req.data_binary = tokens.get(i).cloned();
                if req.method.is_empty() {
                    req.method = "POST".to_string();
                }
            }
            "-F" | "--form" => {
                i += 1;
                if let Some(field) = tokens.get(i) {
                    if let Some(eq) = field.find('=') {
                        req.form
                            .push((field[..eq].to_string(), field[eq + 1..].to_string()));
                    }
                }
                if req.method.is_empty() {
                    req.method = "POST".to_string();
                }
            }
            "-u" | "--user" => {
                i += 1;
                req.auth = tokens.get(i).cloned();
            }
            "-k" | "--insecure" => {
                req.insecure = true;
            }
            "-L" | "--location" => {
                req.follow_redirects = true;
            }
            "-v" | "--verbose" => {
                req.verbose = true;
            }
            "-s" | "--silent" => {
                req.silent = true;
            }
            "-i" | "--include" => {
                req.include_headers = true;
            }
            "-I" | "--head" => {
                req.head_only = true;
                req.method = "HEAD".to_string();
            }
            "--compressed" => {
                req.compressed = true;
            }
            "-o" | "--output" => {
                i += 1;
                req.output = tokens.get(i).cloned();
            }
            "--max-time" | "-m" => {
                i += 1;
                req.max_time = tokens.get(i).and_then(|s| s.parse().ok());
            }
            "--connect-timeout" => {
                i += 1;
                req.timeout = tokens.get(i).and_then(|s| s.parse().ok());
            }
            "-A" | "--user-agent" => {
                i += 1;
                req.user_agent = tokens.get(i).cloned();
            }
            "-b" | "--cookie" => {
                i += 1;
                req.cookie = tokens.get(i).cloned();
            }
            "-c" | "--cookie-jar" => {
                i += 1;
                req.cookie_jar = tokens.get(i).cloned();
            }
            "-x" | "--proxy" => {
                i += 1;
                req.proxy = tokens.get(i).cloned();
            }
            other if !other.starts_with('-') && req.url.is_empty() => {
                req.url = other.to_string();
            }
            _ => {}
        }
        i += 1;
    }

    if req.method.is_empty() {
        req.method = "GET".to_string();
    }

    Ok(req)
}

fn tokenize_shell(s: &str) -> Result<Vec<String>, String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(next) = chars.next() {
                    if next != '\n' {
                        cur.push(next);
                    }
                }
            }
            '\'' => loop {
                match chars.next() {
                    Some('\'') => break,
                    Some(ch) => cur.push(ch),
                    None => return Err("Unterminated single quote".to_string()),
                }
            },
            '"' => loop {
                match chars.next() {
                    Some('"') => break,
                    Some('\\') => {
                        if let Some(esc) = chars.next() {
                            cur.push(esc);
                        }
                    }
                    Some(ch) => cur.push(ch),
                    None => return Err("Unterminated double quote".to_string()),
                }
            },
            ' ' | '\t' | '\n' | '\r' => {
                if !cur.is_empty() {
                    tokens.push(cur.clone());
                    cur.clear();
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    Ok(tokens)
}

fn action_parse(args: &Value) -> Result<String, String> {
    let cmd = args
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or("Provide 'command' with a curl command string")?;
    let req = parse_curl_command(cmd)?;

    let mut out = String::new();
    out.push_str("## curl Request Breakdown\n\n");
    out.push_str(&format!("  Method:   {}\n", req.method));
    out.push_str(&format!("  URL:      {}\n", req.url));

    if !req.headers.is_empty() {
        out.push_str("\n## Headers\n\n");
        for (name, val) in &req.headers {
            // Redact Authorization values
            if name.to_lowercase() == "authorization" {
                let redacted = if val.len() > 8 {
                    format!("{}...[REDACTED]", &val[..8])
                } else {
                    "[REDACTED]".to_string()
                };
                out.push_str(&format!("  {}: {}\n", name, redacted));
            } else {
                out.push_str(&format!("  {}: {}\n", name, val));
            }
        }
    }

    if let Some(auth) = &req.auth {
        let parts: Vec<&str> = auth.splitn(2, ':').collect();
        if parts.len() == 2 {
            out.push_str(&format!(
                "\n## Auth\n\n  Basic auth — user: {}, password: [REDACTED]\n",
                parts[0]
            ));
        }
    }

    if let Some(data) = &req.data {
        let preview: String = data.chars().take(200).collect();
        let ellipsis = if data.len() > 200 { "..." } else { "" };
        out.push_str(&format!(
            "\n## Request Body\n\n  Mode:     form-data / raw\n  Preview:  {}{}\n",
            preview, ellipsis
        ));
        // Detect content type
        if data.starts_with('{') || data.starts_with('[') {
            out.push_str("  Detected: JSON body\n");
        } else if data.contains('&') || data.contains('=') {
            out.push_str("  Detected: URL-encoded form data\n");
        }
    }
    if let Some(data) = &req.data_binary {
        let preview: String = data.chars().take(200).collect();
        out.push_str(&format!(
            "\n## Request Body (binary/raw)\n\n  Preview: {}\n",
            preview
        ));
    }

    if !req.form.is_empty() {
        out.push_str("\n## Form Fields\n\n");
        for (k, v) in &req.form {
            out.push_str(&format!("  {} = {}\n", k, v));
        }
    }

    out.push_str("\n## Flags\n\n");
    if req.insecure {
        out.push_str("  -k (--insecure): SSL verification disabled\n");
    }
    if req.follow_redirects {
        out.push_str("  -L (--location): Follow redirects\n");
    }
    if req.verbose {
        out.push_str("  -v (--verbose): Verbose output\n");
    }
    if req.silent {
        out.push_str("  -s (--silent): Silent mode\n");
    }
    if req.include_headers {
        out.push_str("  -i (--include): Include response headers\n");
    }
    if req.head_only {
        out.push_str("  -I (--head): HEAD request only\n");
    }
    if req.compressed {
        out.push_str("  --compressed: Accept gzip/deflate\n");
    }
    if let Some(t) = req.max_time {
        out.push_str(&format!("  --max-time: {} seconds\n", t));
    }
    if let Some(t) = req.timeout {
        out.push_str(&format!("  --connect-timeout: {} seconds\n", t));
    }
    if let Some(ua) = &req.user_agent {
        out.push_str(&format!("  -A (--user-agent): {}\n", ua));
    }
    if let Some(p) = &req.proxy {
        out.push_str(&format!("  -x (--proxy): {}\n", p));
    }
    if let Some(o) = &req.output {
        out.push_str(&format!("  -o (--output): {}\n", o));
    }
    if let Some(c) = &req.cookie {
        out.push_str(&format!("  -b (--cookie): {}\n", c));
    }
    if let Some(c) = &req.cookie_jar {
        out.push_str(&format!("  -c (--cookie-jar): {}\n", c));
    }

    Ok(out)
}

fn action_build(args: &Value) -> Result<String, String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Provide 'url' for the build action")?;

    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let insecure = args
        .get("insecure")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let follow = args
        .get("follow_redirects")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let timeout = args.get("timeout").and_then(|v| v.as_u64());
    let output = args.get("output").and_then(|v| v.as_str());
    let auth = args.get("auth").and_then(|v| v.as_str());
    let data = args.get("data").and_then(|v| v.as_str());

    let mut parts: Vec<String> = vec!["curl".to_string()];

    if method != "GET" {
        parts.push(format!("-X {}", method));
    }

    // Auth
    if let Some(a) = auth {
        parts.push(format!("-u '{}'", a));
    }

    // Headers
    if let Some(hdrs) = args.get("headers").and_then(|v| v.as_object()) {
        for (k, v) in hdrs {
            let val = v.as_str().unwrap_or_default();
            parts.push(format!("-H '{}: {}'", k, val));
        }
    }

    // Query params
    let mut full_url = url.to_string();
    if let Some(params) = args.get("params").and_then(|v| v.as_object()) {
        if !params.is_empty() {
            let qs: Vec<String> = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v.as_str().unwrap_or_default()))
                .collect();
            let sep = if full_url.contains('?') { '&' } else { '?' };
            full_url.push(sep);
            full_url.push_str(&qs.join("&"));
        }
    }

    // Data / body
    if let Some(d) = data {
        parts.push(format!("--data '{}'", d));
    }

    // Form
    if let Some(form) = args.get("form").and_then(|v| v.as_object()) {
        for (k, v) in form {
            parts.push(format!("-F '{}={}'", k, v.as_str().unwrap_or_default()));
        }
    }

    if insecure {
        parts.push("-k".to_string());
    }
    if follow {
        parts.push("-L".to_string());
    }
    if let Some(t) = timeout {
        parts.push(format!("--max-time {}", t));
    }
    if let Some(o) = output {
        parts.push(format!("-o '{}'", o));
    }

    parts.push(format!("'{}'", full_url));

    let command = parts.join(" \\\n  ");
    Ok(format!("## Generated curl Command\n\n{}\n", command))
}

fn req_to_python(req: &CurlRequest) -> String {
    let mut lines: Vec<String> = vec!["import requests".to_string(), String::new()];

    if !req.headers.is_empty() {
        lines.push("headers = {".to_string());
        for (k, v) in &req.headers {
            lines.push(format!("    \"{}\": \"{}\",", k, v));
        }
        lines.push("}".to_string());
        lines.push(String::new());
    }

    let method = req.method.to_lowercase();
    let headers_arg = if req.headers.is_empty() {
        String::new()
    } else {
        ", headers=headers".to_string()
    };

    let data_arg = if let Some(d) = &req.data {
        if d.starts_with('{') {
            format!(", json={}", d)
        } else {
            format!(", data=\"{}\"", d)
        }
    } else if let Some(d) = &req.data_binary {
        format!(", data=\"{}\"", d)
    } else {
        String::new()
    };

    let verify_arg = if req.insecure {
        ", verify=False".to_string()
    } else {
        String::new()
    };
    let auth_arg = if let Some(a) = &req.auth {
        let parts: Vec<&str> = a.splitn(2, ':').collect();
        if parts.len() == 2 {
            format!(", auth=(\"{}\", \"{}\")", parts[0], parts[1])
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let allow_redirects = if req.follow_redirects {
        ", allow_redirects=True".to_string()
    } else {
        String::new()
    };
    let timeout_arg = if let Some(t) = req.max_time.or(req.timeout) {
        format!(", timeout={}", t)
    } else {
        String::new()
    };

    lines.push(format!(
        "response = requests.{}(\n    \"{}\"{}{}{}{}{}{}\n)",
        method, req.url, headers_arg, data_arg, verify_arg, auth_arg, allow_redirects, timeout_arg
    ));
    lines.push(String::new());
    lines.push("print(response.status_code)".to_string());
    lines.push("print(response.text)".to_string());
    lines.join("\n")
}

fn req_to_go(req: &CurlRequest) -> String {
    let mut lines = vec![
        "package main".to_string(),
        String::new(),
        "import (".to_string(),
        "\t\"fmt\"".to_string(),
        "\t\"net/http\"".to_string(),
    ];
    let has_body = req.data.is_some() || req.data_binary.is_some();
    if has_body {
        lines.push("\t\"strings\"".to_string());
    }
    lines.push(")".to_string());
    lines.push(String::new());
    lines.push("func main() {".to_string());

    if has_body {
        let body = req
            .data
            .as_deref()
            .or(req.data_binary.as_deref())
            .unwrap_or("");
        lines.push(format!("\tbody := strings.NewReader(`{}`)", body));
        lines.push(format!(
            "\treq, _ := http.NewRequest(\"{}\", \"{}\", body)",
            req.method, req.url
        ));
    } else {
        lines.push(format!(
            "\treq, _ := http.NewRequest(\"{}\", \"{}\", nil)",
            req.method, req.url
        ));
    }

    for (k, v) in &req.headers {
        lines.push(format!("\treq.Header.Set(\"{}\", \"{}\")", k, v));
    }
    if let Some(auth) = &req.auth {
        let parts: Vec<&str> = auth.splitn(2, ':').collect();
        if parts.len() == 2 {
            lines.push(format!(
                "\treq.SetBasicAuth(\"{}\", \"{}\")",
                parts[0], parts[1]
            ));
        }
    }

    lines.push(String::new());
    lines.push("\tclient := &http.Client{}".to_string());
    if req.insecure {
        lines.push("\t// Note: TLS verification disabled — unsafe for production".to_string());
    }
    lines.push("\tresp, err := client.Do(req)".to_string());
    lines.push("\tif err != nil { panic(err) }".to_string());
    lines.push("\tdefer resp.Body.Close()".to_string());
    lines.push("\tfmt.Println(resp.Status)".to_string());
    lines.push("}".to_string());
    lines.join("\n")
}

fn req_to_js(req: &CurlRequest) -> String {
    let mut lines: Vec<String> = Vec::new();
    let method = req.method.as_str();

    let mut options = vec![format!("  method: '{}'", method)];

    if !req.headers.is_empty() {
        let hdrs: Vec<String> = req
            .headers
            .iter()
            .map(|(k, v)| format!("    '{}': '{}'", k, v))
            .collect();
        options.push(format!("  headers: {{\n{}\n  }}", hdrs.join(",\n")));
    }

    if let Some(d) = &req.data {
        options.push(format!("  body: `{}`", d));
    }

    let opts = options.join(",\n");
    lines.push(format!(
        "const response = await fetch('{}', {{\n{}\n}});",
        req.url, opts
    ));
    lines.push(String::new());
    lines.push("const data = await response.json();".to_string());
    lines.push("console.log(response.status, data);".to_string());

    lines.join("\n")
}

fn action_convert(args: &Value) -> Result<String, String> {
    let cmd = args
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or("Provide 'command' with a curl command string")?;
    let lang = args
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("python");
    let req = parse_curl_command(cmd)?;

    let code = match lang {
        "python" => req_to_python(&req),
        "go" => req_to_go(&req),
        "javascript" | "js" | "node" => req_to_js(&req),
        other => {
            return Err(format!(
                "Unknown language '{}'. Supported: python, go, javascript",
                other
            ))
        }
    };

    let lang_label = match lang {
        "go" => "Go",
        "javascript" | "js" | "node" => "JavaScript (fetch)",
        _ => "Python (requests)",
    };

    Ok(format!(
        "## curl → {} Conversion\n\n```{}\n{}\n```\n",
        lang_label, lang, code
    ))
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            if args.get("url").is_some() {
                "build"
            } else if args.get("language").is_some() {
                "convert"
            } else {
                "parse"
            }
        });
    match action {
        "build" => action_build(args),
        "convert" => action_convert(args),
        _ => action_parse(args),
    }
}
