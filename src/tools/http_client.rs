use reqwest::header::HeaderMap;
use serde_json::Value;
use std::time::Duration;

const MAX_BODY: usize = 8 * 1024;
const DEFAULT_TIMEOUT: u64 = 30;

const INTERESTING_HEADERS: &[&str] = &[
    "content-type",
    "content-length",
    "location",
    "x-request-id",
    "x-ratelimit-remaining",
    "x-ratelimit-limit",
    "retry-after",
    "server",
    "cache-control",
    "etag",
    "last-modified",
    "www-authenticate",
];

pub async fn execute(args: &Value) -> Result<String, String> {
    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("http_request: 'url' is required")?;
    let timeout_secs = args
        .get("timeout_seconds")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT);
    let body = args.get("body").and_then(|v| v.as_str());
    let bearer = args.get("bearer_token").and_then(|v| v.as_str());
    let basic = args.get("basic_auth").and_then(|v| v.as_str());
    let follow_redirects = args
        .get("follow_redirects")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let redirect_policy = if follow_redirects {
        reqwest::redirect::Policy::default()
    } else {
        reqwest::redirect::Policy::none()
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(redirect_policy)
        .build()
        .map_err(|e| format!("http_request: client build failed: {e}"))?;

    let mut req = match method.as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "PATCH" => client.patch(url),
        "HEAD" => client.head(url),
        other => return Err(format!("http_request: unsupported method '{other}'")),
    };

    if let Some(token) = bearer {
        req = req.header("Authorization", format!("Bearer {token}"));
    } else if let Some(creds) = basic {
        let mut parts = creds.splitn(2, ':');
        let user = parts.next().unwrap_or("");
        let pass = parts.next();
        req = req.basic_auth(user, pass);
    }

    if let Some(headers_obj) = args.get("headers").and_then(|v| v.as_object()) {
        for (k, v) in headers_obj {
            if let Some(val) = v.as_str() {
                req = req.header(k.as_str(), val);
            }
        }
    }

    if let Some(b) = body {
        let content_type = args
            .get("content_type")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                let trimmed = b.trim_start();
                if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    "application/json"
                } else {
                    "text/plain"
                }
            });
        req = req.header("Content-Type", content_type).body(b.to_string());
    }

    let result = tokio::time::timeout(
        Duration::from_secs(timeout_secs.saturating_add(2)),
        req.send(),
    )
    .await;

    let resp = match result {
        Err(_) => return Err(format!("http_request: timed out after {timeout_secs}s")),
        Ok(Err(e)) => return Err(format!("http_request: request failed: {e}")),
        Ok(Ok(r)) => r,
    };

    let status = resp.status();
    let headers_text = fmt_headers(resp.headers());
    let body_bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("http_request: failed reading body: {e}"))?;

    let body_str = String::from_utf8_lossy(&body_bytes);
    let body_display = if body_bytes.len() > MAX_BODY {
        format!(
            "{}...\n[truncated — {} bytes total]",
            &body_str[..MAX_BODY],
            body_bytes.len()
        )
    } else {
        try_pretty_json(&body_str)
    };

    let status_tag = if status.is_success() {
        "OK"
    } else if status.is_client_error() {
        "CLIENT ERROR"
    } else if status.is_server_error() {
        "SERVER ERROR"
    } else {
        "REDIRECT/INFO"
    };

    let mut out = format!(
        "http_request [{status_tag}]\n\
         {method} {url}\n\
         Status: {status}\n"
    );
    if !headers_text.is_empty() {
        out.push_str(&format!("Headers:\n{headers_text}"));
    }
    out.push_str("── Body ──\n");
    out.push_str(&body_display);
    Ok(out.trim_end().to_string())
}

fn fmt_headers(headers: &HeaderMap) -> String {
    let mut out = String::new();
    for key in INTERESTING_HEADERS {
        if let Some(val) = headers.get(*key) {
            if let Ok(s) = val.to_str() {
                out.push_str(&format!("  {key}: {s}\n"));
            }
        }
    }
    out
}

fn try_pretty_json(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(json) = serde_json::from_str::<Value>(trimmed) {
            if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                return pretty;
            }
        }
    }
    s.to_string()
}
