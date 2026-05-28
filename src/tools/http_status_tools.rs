use serde_json::Value;

// (code, reason, description, category)
static HTTP_STATUSES: &[(u16, &str, &str, &str)] = &[
    // 1xx Informational
    (
        100,
        "Continue",
        "Client should continue with request; used for Expect: 100-continue",
        "1xx",
    ),
    (
        101,
        "Switching Protocols",
        "Server agrees to switch protocol specified in Upgrade header",
        "1xx",
    ),
    (
        102,
        "Processing",
        "Server received request and is processing; no response yet (WebDAV)",
        "1xx",
    ),
    (
        103,
        "Early Hints",
        "Preload resources via Link headers before the final response is ready",
        "1xx",
    ),
    // 2xx Success
    (
        200,
        "OK",
        "Request succeeded; response body contains the result",
        "2xx",
    ),
    (
        201,
        "Created",
        "Request succeeded and a new resource was created; Location header gives URL",
        "2xx",
    ),
    (
        202,
        "Accepted",
        "Request accepted for processing but processing is not yet complete",
        "2xx",
    ),
    (
        203,
        "Non-Authoritative Information",
        "Success but returned metadata is from a third-party copy",
        "2xx",
    ),
    (
        204,
        "No Content",
        "Request succeeded; no response body — common for DELETE and PUT",
        "2xx",
    ),
    (
        205,
        "Reset Content",
        "Request succeeded; client should reset the document view",
        "2xx",
    ),
    (
        206,
        "Partial Content",
        "Partial GET fulfilled; used with Range header for resumable downloads",
        "2xx",
    ),
    (
        207,
        "Multi-Status",
        "Multiple independent operations; body is XML with per-item status (WebDAV)",
        "2xx",
    ),
    (
        208,
        "Already Reported",
        "Members already enumerated in a prior 207 response (WebDAV)",
        "2xx",
    ),
    (
        226,
        "IM Used",
        "Server fulfilled GET using instance-manipulations; delta encoding (RFC 3229)",
        "2xx",
    ),
    // 3xx Redirection
    (
        300,
        "Multiple Choices",
        "Multiple representations available; client or user should choose one",
        "3xx",
    ),
    (
        301,
        "Moved Permanently",
        "Resource permanently moved to new URL; update bookmarks and links",
        "3xx",
    ),
    (
        302,
        "Found",
        "Resource temporarily at different URL; keep using the original URL in future",
        "3xx",
    ),
    (
        303,
        "See Other",
        "Redirect to another URL using GET; used after POST/PUT to prevent re-submit",
        "3xx",
    ),
    (
        304,
        "Not Modified",
        "Conditional GET: cached version still valid, use cached response",
        "3xx",
    ),
    (
        305,
        "Use Proxy",
        "Must access resource through the specified proxy (deprecated; security risk)",
        "3xx",
    ),
    (
        307,
        "Temporary Redirect",
        "Temporary redirect; repeat the request with the same method and body",
        "3xx",
    ),
    (
        308,
        "Permanent Redirect",
        "Permanent redirect; repeat with same method and body; update all links",
        "3xx",
    ),
    // 4xx Client Errors
    (
        400,
        "Bad Request",
        "Malformed request syntax, invalid parameters, or missing required fields",
        "4xx",
    ),
    (
        401,
        "Unauthorized",
        "Authentication required or credentials invalid; check Authorization header",
        "4xx",
    ),
    (
        402,
        "Payment Required",
        "Reserved; used by some APIs for quota enforcement or payment walls",
        "4xx",
    ),
    (
        403,
        "Forbidden",
        "Server understood request but refuses to authorize it; not an auth issue",
        "4xx",
    ),
    (
        404,
        "Not Found",
        "Resource does not exist at this URL; check the path or ID",
        "4xx",
    ),
    (
        405,
        "Method Not Allowed",
        "HTTP method not supported for this endpoint; check the Allow header",
        "4xx",
    ),
    (
        406,
        "Not Acceptable",
        "Server cannot produce a response matching the Accept headers sent",
        "4xx",
    ),
    (
        407,
        "Proxy Authentication Required",
        "Must authenticate with the intermediate proxy first",
        "4xx",
    ),
    (
        408,
        "Request Timeout",
        "Server timed out waiting for the client to send the complete request",
        "4xx",
    ),
    (
        409,
        "Conflict",
        "Request conflicts with the current state of the resource; resolve conflict first",
        "4xx",
    ),
    (
        410,
        "Gone",
        "Resource permanently deleted; no forwarding address — remove all references",
        "4xx",
    ),
    (
        411,
        "Length Required",
        "Content-Length header is required for this request",
        "4xx",
    ),
    (
        412,
        "Precondition Failed",
        "Conditional request precondition (If-Match, If-None-Match) not met",
        "4xx",
    ),
    (
        413,
        "Content Too Large",
        "Request body exceeds the server maximum; reduce payload size",
        "4xx",
    ),
    (
        414,
        "URI Too Long",
        "Request URI is longer than the server will process",
        "4xx",
    ),
    (
        415,
        "Unsupported Media Type",
        "Content-Type is not supported for this endpoint",
        "4xx",
    ),
    (
        416,
        "Range Not Satisfiable",
        "Range header cannot be fulfilled; range is outside the content bounds",
        "4xx",
    ),
    (
        417,
        "Expectation Failed",
        "Expect request-header value cannot be satisfied by the server",
        "4xx",
    ),
    (
        418,
        "I'm a Teapot",
        "Server is a teapot and refuses to brew coffee (RFC 2324, April Fools)",
        "4xx",
    ),
    (
        421,
        "Misdirected Request",
        "Request directed at a server that cannot produce a response for it",
        "4xx",
    ),
    (
        422,
        "Unprocessable Content",
        "Request is syntactically valid but semantically invalid (validation errors)",
        "4xx",
    ),
    (
        423,
        "Locked",
        "The resource is currently locked (WebDAV)",
        "4xx",
    ),
    (
        424,
        "Failed Dependency",
        "Request failed because a prior dependent request failed (WebDAV)",
        "4xx",
    ),
    (
        425,
        "Too Early",
        "Server unwilling to risk processing a request that might be replayed (TLS 1.3)",
        "4xx",
    ),
    (
        426,
        "Upgrade Required",
        "Client must upgrade to the protocol specified in the Upgrade header",
        "4xx",
    ),
    (
        428,
        "Precondition Required",
        "Request must be conditional to prevent lost-update conflicts",
        "4xx",
    ),
    (
        429,
        "Too Many Requests",
        "Rate limit exceeded; check Retry-After header for when to try again",
        "4xx",
    ),
    (
        431,
        "Request Header Fields Too Large",
        "One or more request header fields are too large",
        "4xx",
    ),
    (
        451,
        "Unavailable For Legal Reasons",
        "Resource blocked for legal reasons such as censorship or court order",
        "4xx",
    ),
    // 5xx Server Errors
    (
        500,
        "Internal Server Error",
        "Unexpected server-side error; check server logs for the root cause",
        "5xx",
    ),
    (
        501,
        "Not Implemented",
        "Server does not support the HTTP method used in the request",
        "5xx",
    ),
    (
        502,
        "Bad Gateway",
        "Server acting as gateway received invalid response from upstream server",
        "5xx",
    ),
    (
        503,
        "Service Unavailable",
        "Server temporarily unavailable due to overload or maintenance; check Retry-After",
        "5xx",
    ),
    (
        504,
        "Gateway Timeout",
        "Server acting as gateway did not receive timely response from upstream",
        "5xx",
    ),
    (
        505,
        "HTTP Version Not Supported",
        "Server does not support the HTTP protocol version in the request",
        "5xx",
    ),
    (
        506,
        "Variant Also Negotiates",
        "Server configuration error in content negotiation (circular reference)",
        "5xx",
    ),
    (
        507,
        "Insufficient Storage",
        "Server cannot store the representation needed to complete request (WebDAV)",
        "5xx",
    ),
    (
        508,
        "Loop Detected",
        "Server detected an infinite loop while processing the request (WebDAV)",
        "5xx",
    ),
    (
        510,
        "Not Extended",
        "Further extensions to the request are required before the server fulfills it",
        "5xx",
    ),
    (
        511,
        "Network Authentication Required",
        "Client must authenticate to gain network access (captive portal)",
        "5xx",
    ),
];

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("code").is_some() || args.get("status").is_some() {
        "lookup".to_string()
    } else if args.get("query").is_some() || args.get("q").is_some() {
        "search".to_string()
    } else if args.get("category").is_some() || args.get("cat").is_some() {
        "category".to_string()
    } else {
        "lookup".to_string()
    };
    match action.as_str() {
        "lookup" => lookup_action(args),
        "search" => search_action(args),
        "category" => category_action(args),
        "list" => list_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: lookup, search, category, list",
            action
        )),
    }
}

fn cat_name(cat: &str) -> &'static str {
    match cat {
        "1xx" => "Informational",
        "2xx" => "Success",
        "3xx" => "Redirection",
        "4xx" => "Client Error",
        "5xx" => "Server Error",
        _ => "Unknown",
    }
}

fn lookup_action(args: &Value) -> Result<String, String> {
    let val = args
        .get("code")
        .or_else(|| args.get("status"))
        .or_else(|| args.get("input"))
        .ok_or("Missing 'code' — pass an HTTP status code like 404")?;
    let code = val
        .as_u64()
        .map(|n| n as u16)
        .or_else(|| val.as_str().and_then(|s| s.parse::<u16>().ok()))
        .ok_or("'code' must be a number like 404 or a string like \"404\"")?;

    match HTTP_STATUSES.iter().find(|(c, _, _, _)| *c == code) {
        Some((c, reason, desc, cat)) => {
            let mut out = format!("HTTP {} {}\n{}\n\n", c, reason, "=".repeat(44));
            out += &format!("Category:    {} ({})\n", cat, cat_name(cat));
            out += &format!("Description: {}\n", desc);
            Ok(out)
        }
        None => Ok(format!(
            "HTTP {} is not a registered standard status code.\n",
            code
        )),
    }
}

fn search_action(args: &Value) -> Result<String, String> {
    let query = args
        .get("query")
        .or_else(|| args.get("q"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'query' argument")?;
    let q = query.to_lowercase();

    let matches: Vec<_> = HTTP_STATUSES
        .iter()
        .filter(|(_, reason, desc, _)| {
            reason.to_lowercase().contains(q.as_str()) || desc.to_lowercase().contains(q.as_str())
        })
        .collect();

    if matches.is_empty() {
        return Ok(format!("No HTTP status codes matching '{}'\n", query));
    }

    let mut out = format!(
        "HTTP Status Search: '{}' ({} results)\n{}\n\n",
        query,
        matches.len(),
        "=".repeat(44)
    );
    out += &format!("{:>5}  {:<34} {}\n", "Code", "Reason", "Category");
    out += &format!("{}\n", "-".repeat(55));
    for (c, reason, _, cat) in &matches {
        out += &format!("{:>5}  {:<34} {}\n", c, reason, cat);
    }
    Ok(out)
}

fn normalize_cat(cat: &str) -> Option<&'static str> {
    let lower = cat.to_lowercase();
    if lower == "1" || lower == "1xx" || lower.contains("informational") {
        Some("1xx")
    } else if lower == "2" || lower == "2xx" || lower.contains("success") {
        Some("2xx")
    } else if lower == "3" || lower == "3xx" || lower.contains("redirect") {
        Some("3xx")
    } else if lower == "4" || lower == "4xx" || lower.contains("client") {
        Some("4xx")
    } else if lower == "5" || lower == "5xx" || lower.contains("server") {
        Some("5xx")
    } else {
        None
    }
}

fn category_action(args: &Value) -> Result<String, String> {
    let cat_arg = args
        .get("category")
        .or_else(|| args.get("cat"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str());

    if cat_arg.is_none() {
        let mut out = format!("HTTP Status Categories\n{}\n\n", "=".repeat(44));
        for cat in ["1xx", "2xx", "3xx", "4xx", "5xx"] {
            let count = HTTP_STATUSES
                .iter()
                .filter(|(_, _, _, c)| *c == cat)
                .count();
            out += &format!("  {}  {}  ({} codes)\n", cat, cat_name(cat), count);
        }
        out += "\nPass category: \"4xx\" to list codes in that category.\n";
        return Ok(out);
    }

    let cat = cat_arg.unwrap();
    let normalized = normalize_cat(cat)
        .ok_or_else(|| format!("Unknown category '{}'. Valid: 1xx, 2xx, 3xx, 4xx, 5xx", cat))?;

    let matches: Vec<_> = HTTP_STATUSES
        .iter()
        .filter(|(_, _, _, c)| *c == normalized)
        .collect();

    let mut out = format!(
        "HTTP {} — {} ({} codes)\n{}\n\n",
        normalized,
        cat_name(normalized),
        matches.len(),
        "=".repeat(44)
    );
    for (c, reason, desc, _) in &matches {
        let short: String = desc.chars().take(50).collect();
        let short = if desc.len() > 50 {
            format!("{}...", short)
        } else {
            short.to_string()
        };
        out += &format!("{:>5}  {:<34} {}\n", c, reason, short);
    }
    Ok(out)
}

fn list_action(args: &Value) -> Result<String, String> {
    let filter = args
        .get("category")
        .or_else(|| args.get("cat"))
        .and_then(|v| v.as_str())
        .and_then(normalize_cat);

    let statuses: Vec<_> = HTTP_STATUSES
        .iter()
        .filter(|(_, _, _, c)| filter.map(|f| *c == f).unwrap_or(true))
        .collect();

    let mut out = format!(
        "HTTP Status Codes ({} total)\n{}\n\n",
        statuses.len(),
        "=".repeat(44)
    );
    out += &format!("{:>5}  {:<34} {}\n", "Code", "Reason", "Cat");
    out += &format!("{}\n", "-".repeat(50));
    for (c, reason, _, cat) in &statuses {
        out += &format!("{:>5}  {:<34} {}\n", c, reason, cat);
    }
    Ok(out)
}
