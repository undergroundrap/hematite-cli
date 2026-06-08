use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else {
        "parse".to_string()
    };
    match action.as_str() {
        "parse" => parse_action(args),
        "explain" => explain_action(args),
        "validate" => validate_action(args),
        "build" => build_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: parse, explain, validate, build",
            action
        )),
    }
}

fn get_policy(args: &Value) -> Result<String, String> {
    args.get("header")
        .or_else(|| args.get("policy"))
        .or_else(|| args.get("csp"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| {
            // Strip "Content-Security-Policy: " prefix if present
            let s = s.trim();
            let lower = s.to_lowercase();
            if lower.starts_with("content-security-policy-report-only:") {
                let offset = "content-security-policy-report-only:".len();
                s[offset..].trim().to_string()
            } else if lower.starts_with("content-security-policy:") {
                let offset = "content-security-policy:".len();
                s[offset..].trim().to_string()
            } else {
                s.to_string()
            }
        })
        .ok_or_else(|| {
            "Missing 'header' — pass a CSP header value like \"default-src 'self'; img-src *\""
                .to_string()
        })
}

struct CspDirective {
    name: String,
    values: Vec<String>,
}

fn parse_directives(policy: &str) -> Vec<CspDirective> {
    policy
        .split(';')
        .map(|d| d.trim())
        .filter(|d| !d.is_empty())
        .map(|d| {
            let mut parts = d.splitn(2, ' ');
            let name = parts.next().unwrap_or("").to_lowercase();
            let vals: Vec<String> = parts
                .next()
                .unwrap_or("")
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            CspDirective { name, values: vals }
        })
        .collect()
}

fn source_description(src: &str) -> &'static str {
    match src {
        "'self'" => "same origin only",
        "'none'" => "block all sources",
        "'unsafe-inline'" => "allow inline scripts/styles (unsafe)",
        "'unsafe-eval'" => "allow eval() and similar (unsafe)",
        "'unsafe-hashes'" => "allow inline event handlers matching hash (unsafe)",
        "'strict-dynamic'" => "trust scripts with nonce/hash to load further scripts",
        "*" => "any origin (wildcard — no restriction)",
        "'wasm-unsafe-eval'" => "allow WebAssembly evaluation",
        "'report-sample'" => "include sample of violating code in report",
        _ if src.starts_with("'nonce-") => "nonce-based allowlist (cryptographic)",
        _ if src.starts_with("'sha256-")
            || src.starts_with("'sha384-")
            || src.starts_with("'sha512-") =>
        {
            "hash-based allowlist (cryptographic)"
        }
        _ if src.starts_with("https:") => "any HTTPS source",
        _ if src.starts_with("http:") => "any HTTP source (insecure)",
        _ if src.starts_with("data:") => "data: URIs",
        _ if src.starts_with("blob:") => "blob: URIs",
        _ if src.starts_with("filesystem:") => "filesystem: URIs",
        _ if src.starts_with("ws:") => "WebSocket (insecure)",
        _ if src.starts_with("wss:") => "WebSocket Secure",
        _ => "specific host or pattern",
    }
}

fn directive_description(name: &str) -> &'static str {
    match name {
        "default-src" => "Fallback for fetch directives not otherwise specified",
        "script-src" => "Controls JavaScript sources",
        "script-src-elem" => "Controls <script> element sources",
        "script-src-attr" => "Controls inline event handler sources",
        "style-src" => "Controls CSS stylesheet sources",
        "style-src-elem" => "Controls <style> and <link rel=stylesheet> sources",
        "style-src-attr" => "Controls inline style= attribute sources",
        "img-src" => "Controls image and favicon sources",
        "font-src" => "Controls font sources (@font-face)",
        "connect-src" => "Controls XHR, fetch(), WebSocket, EventSource destinations",
        "media-src" => "Controls <audio> and <video> sources",
        "object-src" => "Controls <object>, <embed>, <applet> sources (legacy plugins)",
        "frame-src" => "Controls sources for nested browsing contexts (<frame>, <iframe>)",
        "frame-ancestors" => "Controls which pages may embed this page (clickjacking protection)",
        "child-src" => "Controls web workers and nested browsing contexts",
        "worker-src" => "Controls worker, SharedWorker, ServiceWorker sources",
        "manifest-src" => "Controls Web App Manifest sources",
        "base-uri" => "Restricts values for <base href=>",
        "form-action" => "Restricts which URLs can be used as <form action=>",
        "navigate-to" => "Restricts which URLs the page may navigate to",
        "sandbox" => "Applies sandboxing restrictions (like iframe sandbox)",
        "report-uri" => "Where to send violation reports (deprecated — use report-to)",
        "report-to" => "Reporting endpoint group name for violation reports",
        "upgrade-insecure-requests" => "Upgrades all HTTP requests to HTTPS automatically",
        "block-all-mixed-content" => "Blocks all mixed content (HTTPS page loading HTTP)",
        "require-trusted-types-for" => "Requires Trusted Types for DOM XSS sinks",
        "trusted-types" => "Defines allowed Trusted Types policy names",
        _ => "Custom or unknown directive",
    }
}

fn is_unsafe(src: &str) -> bool {
    matches!(
        src,
        "'unsafe-inline'" | "'unsafe-eval'" | "'unsafe-hashes'" | "*" | "http:"
    )
}

fn parse_action(args: &Value) -> Result<String, String> {
    let policy = get_policy(args)?;
    let directives = parse_directives(&policy);

    let mut out = format!("CSP Header\n{}\n\n", "=".repeat(44));
    out += &format!("{} directive(s)\n\n", directives.len());

    for d in &directives {
        out += &format!("{}\n", d.name);
        out += &format!("  Purpose:  {}\n", directive_description(&d.name));
        if d.values.is_empty() {
            out += "  Sources:  (none — directive present with no values)\n";
        } else {
            for src in &d.values {
                let desc = source_description(src);
                let unsafe_flag = if is_unsafe(src) { "  [UNSAFE]" } else { "" };
                out += &format!("  {:25} {}{}\n", src, desc, unsafe_flag);
            }
        }
        out += "\n";
    }
    Ok(out)
}

fn explain_action(args: &Value) -> Result<String, String> {
    let policy = get_policy(args)?;
    let directives = parse_directives(&policy);

    let mut out = format!("CSP Plain-English Explanation\n{}\n\n", "=".repeat(44));

    let has_default = directives.iter().any(|d| d.name == "default-src");
    if !has_default {
        out += "Note: No default-src directive — directives without explicit fallback allow anything.\n\n";
    }

    for d in &directives {
        out += &format!("{}: ", d.name);
        if d.values.is_empty() {
            out += "(no source list)\n";
            continue;
        }
        let has_none = d.values.iter().any(|v| v == "'none'");
        let has_self = d.values.iter().any(|v| v == "'self'");
        let has_star = d.values.iter().any(|v| v == "*");
        let has_unsafe_inline = d.values.iter().any(|v| v == "'unsafe-inline'");
        let has_unsafe_eval = d.values.iter().any(|v| v == "'unsafe-eval'");
        let has_nonce = d.values.iter().any(|v| v.starts_with("'nonce-"));
        let has_hash = d.values.iter().any(|v| {
            v.starts_with("'sha256-") || v.starts_with("'sha384-") || v.starts_with("'sha512-")
        });
        let hosts: Vec<&String> = d
            .values
            .iter()
            .filter(|v| {
                !v.starts_with('\'')
                    && !matches!(v.as_str(), "*" | "http:" | "https:" | "data:" | "blob:")
            })
            .collect();

        if has_none {
            out += "block all sources entirely";
        } else if has_star {
            out += "allow ANY source (no restriction)";
        } else {
            let mut parts: Vec<String> = Vec::new();
            if has_self {
                parts.push("same origin".to_string());
            }
            if has_nonce {
                parts.push("specific nonce-tagged resources".to_string());
            }
            if has_hash {
                parts.push("hash-verified resources".to_string());
            }
            if !hosts.is_empty() {
                parts.push(
                    hosts
                        .iter()
                        .map(|h| h.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }
            if has_unsafe_inline {
                parts.push("inline (UNSAFE)".to_string());
            }
            if has_unsafe_eval {
                parts.push("eval() (UNSAFE)".to_string());
            }
            out += &parts.join(" + ");
        }
        out += "\n";
    }
    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let policy = get_policy(args)?;
    let directives = parse_directives(&policy);

    let mut warnings: Vec<String> = Vec::new();
    let errors: Vec<String> = Vec::new();

    let directive_names: Vec<&str> = directives.iter().map(|d| d.name.as_str()).collect();

    // Check for unknown directives
    let known_directives = [
        "default-src",
        "script-src",
        "script-src-elem",
        "script-src-attr",
        "style-src",
        "style-src-elem",
        "style-src-attr",
        "img-src",
        "font-src",
        "connect-src",
        "media-src",
        "object-src",
        "frame-src",
        "frame-ancestors",
        "child-src",
        "worker-src",
        "manifest-src",
        "base-uri",
        "form-action",
        "navigate-to",
        "sandbox",
        "report-uri",
        "report-to",
        "upgrade-insecure-requests",
        "block-all-mixed-content",
        "require-trusted-types-for",
        "trusted-types",
    ];

    for d in &directives {
        if !known_directives.contains(&d.name.as_str()) {
            warnings.push(format!("Unknown directive: '{}'", d.name));
        }
        for src in &d.values {
            if src == "'unsafe-inline'" {
                warnings.push(format!(
                    "'unsafe-inline' in {} bypasses XSS protection",
                    d.name
                ));
            }
            if src == "'unsafe-eval'" {
                warnings.push(format!(
                    "'unsafe-eval' in {} allows dynamic code execution",
                    d.name
                ));
            }
            if src == "*" {
                warnings.push(format!("Wildcard * in {} allows any source", d.name));
            }
            if src == "http:" {
                warnings.push(format!("http: in {} allows insecure HTTP sources", d.name));
            }
        }
        // Nonce + unsafe-inline combo: unsafe-inline is ignored when nonce present, but flagging it
        let has_nonce = d.values.iter().any(|v| v.starts_with("'nonce-"));
        let has_unsafe_inline = d.values.iter().any(|v| v == "'unsafe-inline'");
        if has_nonce && has_unsafe_inline {
            warnings.push(format!(
                "'unsafe-inline' in {} is redundant when nonce is present (ignored by modern browsers)",
                d.name
            ));
        }
    }

    // Missing recommended directives
    if !directive_names.contains(&"object-src") && !directive_names.contains(&"default-src") {
        warnings.push(
            "No object-src or default-src — legacy plugins (<object>, <embed>) are unrestricted"
                .to_string(),
        );
    }
    if !directive_names.contains(&"base-uri") {
        warnings.push("No base-uri directive — <base> tag can be used for injection".to_string());
    }
    if directive_names.contains(&"report-uri") && !directive_names.contains(&"report-to") {
        warnings.push("report-uri is deprecated — consider adding report-to as well".to_string());
    }

    let verdict = if errors.is_empty() && warnings.is_empty() {
        "VALID — no issues found"
    } else if errors.is_empty() {
        "VALID with warnings"
    } else {
        "INVALID"
    };

    let mut out = format!("CSP Validation\n{}\n\n", "=".repeat(44));
    out += &format!("Result: {}\n\n", verdict);
    if !errors.is_empty() {
        out += "Errors:\n";
        for e in &errors {
            out += &format!("  [ERROR] {}\n", e);
        }
        out += "\n";
    }
    if !warnings.is_empty() {
        out += "Warnings:\n";
        for w in &warnings {
            out += &format!("  [WARN]  {}\n", w);
        }
        out += "\n";
    }
    if errors.is_empty() && warnings.is_empty() {
        out += "No errors or warnings.\n";
    }
    Ok(out)
}

fn build_action(args: &Value) -> Result<String, String> {
    // Build a CSP from an object of { directive: [sources] } or a preset name
    let preset = args.get("preset").and_then(|v| v.as_str());
    if let Some(p) = preset {
        let policy = match p {
            "strict" => {
                "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'self'; form-action 'self'; base-uri 'self'; frame-ancestors 'none'"
            }
            "moderate" => {
                "default-src 'self'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; frame-ancestors 'self'; base-uri 'self'; form-action 'self'"
            }
            "api" => {
                "default-src 'none'; frame-ancestors 'none'; base-uri 'none'"
            }
            _ => return Err(format!("Unknown preset '{}'. Valid: strict, moderate, api", p)),
        };
        let directives = parse_directives(policy);
        let mut out = format!("CSP Preset: {}\n{}\n\n", p, "=".repeat(44));
        out += &format!("Header value:\n{}\n\n", policy);
        out += "Directives:\n";
        for d in &directives {
            let vals = if d.values.is_empty() {
                "(no values)".to_string()
            } else {
                d.values.join(" ")
            };
            out += &format!("  {:<30} {}\n", d.name, vals);
        }
        return Ok(out);
    }

    // Build from directives object
    let directives_obj = args
        .get("directives")
        .and_then(|v| v.as_object())
        .ok_or("Missing 'directives' object or 'preset' (strict/moderate/api)")?;
    let mut parts: Vec<String> = Vec::new();
    for (k, v) in directives_obj {
        let sources = match v {
            Value::Array(arr) => arr
                .iter()
                .filter_map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            Value::String(s) => s.clone(),
            Value::Bool(true) => String::new(),
            _ => continue,
        };
        if sources.is_empty() {
            parts.push(k.clone());
        } else {
            parts.push(format!("{} {}", k, sources));
        }
    }
    let policy = parts.join("; ");
    let mut out = format!("CSP Built\n{}\n\n", "=".repeat(44));
    out += &format!("Header value:\n{}\n\n", policy);
    out += "Use as:\nContent-Security-Policy: ";
    out += &policy;
    out += "\n";
    Ok(out)
}
