use serde_json::Value;
use std::collections::HashMap;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "http_cache_tools",
        "description": "Parse, explain, and analyze HTTP Cache-Control and caching headers. Works offline â€” no network calls.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "analyze", "etag", "vary"],
                    "description": "parse (default â€” directive breakdown with conflict warnings), analyze (freshness calculation with age), etag (304 vs 200 verdict), vary (Vary field explanations)"
                },
                "header": { "type": "string", "description": "Cache-Control header value (prefix stripped automatically)" },
                "value": { "type": "string", "description": "Alias for 'header'" },
                "cache_control": { "type": "string", "description": "Alias for 'header'" },
                "cc": { "type": "string", "description": "Alias for 'header'" },
                "request": { "type": "boolean", "description": "Parse as request Cache-Control (not response)" },
                "age": { "type": "string", "description": "Current age in seconds (for freshness analysis)" },
                "etag": { "type": "string", "description": "Server ETag value (for etag action)" },
                "if_none_match": { "type": "string", "description": "Client If-None-Match header value (for etag action)" }
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "parse" | "explain" => action_parse(args),
        "analyze" | "freshness" => action_analyze(args),
        "etag" | "conditional" => action_etag(args),
        "vary" => action_vary(args),
        _ => Err(format!(
            "Unknown action '{}'. Use: parse, analyze, etag, vary.",
            action
        )),
    }
}

fn strip_header_name(raw: &str) -> &str {
    if let Some(pos) = raw.find(':') {
        let prefix = raw[..pos].trim();
        if prefix.eq_ignore_ascii_case("cache-control") {
            return raw[pos + 1..].trim();
        }
    }
    raw.trim()
}

fn get_cc_value(args: &Value) -> Option<&str> {
    args.get("header")
        .or_else(|| args.get("value"))
        .or_else(|| args.get("cache_control"))
        .or_else(|| args.get("cc"))
        .and_then(|v| v.as_str())
        .map(strip_header_name)
}

fn parse_directives(header: &str) -> Vec<(String, Option<String>)> {
    header
        .split(',')
        .map(|d| d.trim())
        .filter(|d| !d.is_empty())
        .map(|d| {
            if let Some((k, v)) = d.split_once('=') {
                (
                    k.trim().to_lowercase(),
                    Some(v.trim().trim_matches('"').to_string()),
                )
            } else {
                (d.trim().to_lowercase(), None)
            }
        })
        .collect()
}

fn secs_to_human(s: u64) -> String {
    if s == 0 {
        return "0s".to_string();
    }
    let mut parts = vec![];
    if s / 86400 > 0 {
        parts.push(format!("{}d", s / 86400));
    }
    if (s % 86400) / 3600 > 0 {
        parts.push(format!("{}h", (s % 86400) / 3600));
    }
    if (s % 3600) / 60 > 0 {
        parts.push(format!("{}m", (s % 3600) / 60));
    }
    if s % 60 > 0 {
        parts.push(format!("{}s", s % 60));
    }
    parts.join(" ")
}

fn explain_directive(name: &str, value: Option<&str>, is_request: bool) -> String {
    match name {
        "max-age" => {
            let secs = value.and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            if is_request {
                format!("Accept responses no older than {} ({} s)", secs_to_human(secs), secs)
            } else {
                format!("Fresh for {} ({} s)", secs_to_human(secs), secs)
            }
        }
        "s-maxage" => {
            let secs = value.and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            format!(
                "Shared caches (CDN/proxy) freshness: {} ({} s) â€” overrides max-age for shared caches",
                secs_to_human(secs),
                secs
            )
        }
        "no-cache" => {
            if is_request {
                "Force end-to-end reload â€” shared cache must not serve without revalidation".to_string()
            } else {
                "Must revalidate with origin before each use (cached but not served without validation)".to_string()
            }
        }
        "no-store" => {
            if is_request {
                "Ask caches not to store any part of this request or its response".to_string()
            } else {
                "Must NOT store response anywhere â€” not in memory or on disk".to_string()
            }
        }
        "no-transform" => "Intermediaries must not transform the content (e.g. compress images, modify encoding)".to_string(),
        "must-revalidate" => "Once stale, MUST revalidate with origin â€” cannot serve stale even if origin is down".to_string(),
        "proxy-revalidate" => "Like must-revalidate but only for shared (proxy/CDN) caches".to_string(),
        "public" => "May be stored by any cache including shared caches, even for authenticated responses".to_string(),
        "private" => {
            if let Some(fields) = value {
                format!("Browser-only cache. Fields '{}' must not be stored by shared caches", fields)
            } else {
                "Browser (private) cache only â€” CDNs and proxies must not store this response".to_string()
            }
        }
        "immutable" => "Content will not change during freshness window â€” skip conditional revalidation entirely".to_string(),
        "stale-while-revalidate" => {
            let secs = value.and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            format!(
                "Serve stale for {} while fetching a fresh copy in the background",
                secs_to_human(secs)
            )
        }
        "stale-if-error" => {
            let secs = value.and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            format!(
                "Serve stale for {} if origin returns 5xx or network error",
                secs_to_human(secs)
            )
        }
        "max-stale" => {
            if let Some(s) = value.and_then(|v| v.parse::<u64>().ok()) {
                format!(
                    "Accept responses up to {} past their expiry",
                    secs_to_human(s)
                )
            } else {
                "Accept any stale response regardless of age".to_string()
            }
        }
        "min-fresh" => {
            let secs = value.and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            format!("Only accept responses that will remain fresh for at least {} more seconds", secs)
        }
        "only-if-cached" => "Return cached response only â€” do not contact origin. Return 504 if nothing cached.".to_string(),
        other => format!(
            "Non-standard or extension directive: {}{}",
            other,
            value.map(|v| format!("={}", v)).unwrap_or_default()
        ),
    }
}

fn action_parse(args: &Value) -> Result<String, String> {
    let header = get_cc_value(args)
        .ok_or("Provide 'header' with a Cache-Control value.")?
        .to_string();
    let is_request = args
        .get("request")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let directives = parse_directives(&header);
    if directives.is_empty() {
        return Ok("No Cache-Control directives found.".to_string());
    }

    let ctx = if is_request { "REQUEST" } else { "RESPONSE" };
    let mut out = format!("Cache-Control [{}]: {}\n\n", ctx, header);

    let names: Vec<&str> = directives.iter().map(|(k, _)| k.as_str()).collect();
    let has_no_store = names.contains(&"no-store");
    let has_no_cache = names.contains(&"no-cache");
    let has_max_age = names.contains(&"max-age");
    let has_public = names.contains(&"public");
    let has_private = names.contains(&"private");
    let has_immutable = names.contains(&"immutable");

    let mut warnings: Vec<&str> = vec![];
    if has_no_store && has_max_age {
        warnings.push("no-store + max-age: max-age is meaningless when no-store is set");
    }
    if has_public && has_private {
        warnings.push("public + private: contradictory â€” private takes precedence");
    }
    if !is_request && has_immutable && !has_max_age {
        warnings
            .push("immutable without max-age: immutable only applies during the freshness window");
    }

    out.push_str(&format!("{} directive(s):\n", directives.len()));
    out.push_str(&"-".repeat(70));
    out.push('\n');
    for (name, value) in &directives {
        let display = if let Some(v) = value {
            format!("{}={}", name, v)
        } else {
            name.clone()
        };
        let explanation = explain_directive(name, value.as_deref(), is_request);
        out.push_str(&format!("  {:<32}  {}\n", display, explanation));
    }

    if !warnings.is_empty() {
        out.push_str(&format!("\n{}\n", "-".repeat(70)));
        for w in &warnings {
            out.push_str(&format!("  âš  {}\n", w));
        }
    }

    out.push_str(&format!("\n{}\n", "-".repeat(70)));
    if !is_request {
        if has_no_store {
            out.push_str("Summary: NOT CACHEABLE â€” no-store prevents all caching.\n");
        } else if has_no_cache {
            out.push_str("Summary: CONDITIONAL â€” cached but must revalidate on every use.\n");
        } else if has_max_age {
            let secs = directives
                .iter()
                .find(|(k, _)| k == "max-age")
                .and_then(|(_, v)| v.as_ref())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            out.push_str(&format!(
                "Summary: CACHEABLE for {} ({} seconds){}.\n",
                secs_to_human(secs),
                secs,
                if has_immutable {
                    " â€” immutable (no revalidation during window)"
                } else {
                    ""
                }
            ));
        } else {
            out.push_str("Summary: No explicit freshness â€” heuristic caching may apply.\n");
        }
    }
    Ok(out)
}

fn action_analyze(args: &Value) -> Result<String, String> {
    let response_cc = args
        .get("response")
        .or_else(|| args.get("header"))
        .and_then(|v| v.as_str())
        .map(strip_header_name)
        .ok_or("Provide 'response' with the response Cache-Control header.")?
        .to_string();

    let request_cc = args
        .get("request")
        .and_then(|v| v.as_str())
        .map(strip_header_name)
        .map(str::to_string);

    let age_secs: u64 = args.get("age").and_then(|v| v.as_u64()).unwrap_or(0);

    let resp_dirs = parse_directives(&response_cc);
    let req_dirs = request_cc
        .as_deref()
        .map(parse_directives)
        .unwrap_or_default();

    let resp: HashMap<&str, Option<&str>> = resp_dirs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_deref()))
        .collect();
    let req: HashMap<&str, Option<&str>> = req_dirs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_deref()))
        .collect();

    let mut out = String::from("Cache Freshness Analysis\n");
    out.push_str(&format!("{}\n\n", "â•".repeat(50)));
    out.push_str(&format!("Response CC : {}\n", response_cc));
    if let Some(rcc) = &request_cc {
        out.push_str(&format!("Request CC  : {}\n", rcc));
    }
    out.push_str(&format!(
        "Current Age : {} seconds ({})\n\n",
        age_secs,
        secs_to_human(age_secs)
    ));

    if resp.contains_key("no-store") {
        out.push_str("âŒ NOT CACHEABLE\n   no-store: response must not be stored in any cache.\n");
        return Ok(out);
    }

    if req.contains_key("no-cache") {
        out.push_str("ðŸ”„ REQUEST FORCED RELOAD (request no-cache)\n   Client demands fresh response from origin.\n\n");
    }

    if resp.contains_key("no-cache") {
        out.push_str("ðŸ”„ MUST REVALIDATE ON EVERY USE (response no-cache)\n   Cached but must be validated before every use.\n");
        return Ok(out);
    }

    let max_age: Option<u64> = resp
        .get("s-maxage")
        .and_then(|v| *v)
        .and_then(|v| v.parse().ok())
        .or_else(|| {
            resp.get("max-age")
                .and_then(|v| *v)
                .and_then(|v| v.parse().ok())
        });

    let stale_while_reval: Option<u64> = resp
        .get("stale-while-revalidate")
        .and_then(|v| *v)
        .and_then(|v| v.parse().ok());

    let stale_if_error: Option<u64> = resp
        .get("stale-if-error")
        .and_then(|v| *v)
        .and_then(|v| v.parse().ok());

    let must_reval = resp.contains_key("must-revalidate");
    let immutable = resp.contains_key("immutable");

    match max_age {
        Some(lifetime) => {
            let is_fresh = age_secs < lifetime;
            if is_fresh {
                let remaining = lifetime - age_secs;
                out.push_str("âœ… FRESH\n");
                out.push_str(&format!(
                    "   Freshness lifetime : {} ({} s)\n",
                    secs_to_human(lifetime),
                    lifetime
                ));
                out.push_str(&format!(
                    "   Current age        : {} ({} s)\n",
                    secs_to_human(age_secs),
                    age_secs
                ));
                out.push_str(&format!(
                    "   Remaining          : {} ({} s)\n",
                    secs_to_human(remaining),
                    remaining
                ));
                if immutable {
                    out.push_str(
                        "   Immutable: browser will NOT send revalidation requests during window\n",
                    );
                }
            } else {
                let stale_for = age_secs - lifetime;
                out.push_str("âš  STALE\n");
                out.push_str(&format!(
                    "   Freshness lifetime : {} ({} s)\n",
                    secs_to_human(lifetime),
                    lifetime
                ));
                out.push_str(&format!(
                    "   Current age        : {} ({} s)\n",
                    secs_to_human(age_secs),
                    age_secs
                ));
                out.push_str(&format!(
                    "   Stale by           : {} ({} s)\n",
                    secs_to_human(stale_for),
                    stale_for
                ));
                if let Some(swr) = stale_while_reval {
                    if stale_for <= swr {
                        out.push_str(&format!(
                            "   stale-while-revalidate: within grace ({} window) â€” can serve stale, background fetch active\n",
                            secs_to_human(swr)
                        ));
                    }
                }
                if must_reval {
                    out.push_str(
                        "   must-revalidate: CANNOT serve stale â€” must contact origin\n",
                    );
                }
                out.push_str(
                    "\n   Action: send conditional request (If-None-Match or If-Modified-Since)\n",
                );
            }
            if let Some(sie) = stale_if_error {
                out.push_str(&format!(
                    "\n   stale-if-error: can serve stale for {} on 5xx/network errors\n",
                    secs_to_human(sie)
                ));
            }
        }
        None => {
            out.push_str("âš  NO EXPLICIT FRESHNESS\n");
            out.push_str("   No max-age or s-maxage directive.\n");
            out.push_str(
                "   Heuristic caching may apply (typically ~10% of (Date âˆ’ Last-Modified)).\n",
            );
        }
    }

    Ok(out)
}

fn action_etag(args: &Value) -> Result<String, String> {
    let etag = args.get("etag").and_then(|v| v.as_str());
    let if_none_match = args
        .get("if_none_match")
        .or_else(|| args.get("if-none-match"))
        .and_then(|v| v.as_str());
    let last_modified = args
        .get("last_modified")
        .or_else(|| args.get("last-modified"))
        .and_then(|v| v.as_str());
    let if_modified_since = args
        .get("if_modified_since")
        .or_else(|| args.get("if-modified-since"))
        .and_then(|v| v.as_str());

    let mut out = String::from("Conditional Request Analysis\n");
    out.push_str(&format!("{}\n\n", "â•".repeat(50)));
    let mut has_input = false;

    if let (Some(et), Some(inm)) = (etag, if_none_match) {
        has_input = true;
        let is_weak = et.starts_with("W/");
        out.push_str(&format!("ETag          : {}\n", et));
        out.push_str(&format!("If-None-Match : {}\n", inm));
        out.push_str(&format!(
            "Type          : {} ETag\n\n",
            if is_weak {
                "Weak â€” semantically equivalent (minor differences allowed)"
            } else {
                "Strong â€” byte-for-byte identical"
            }
        ));

        let is_star = inm.trim() == "*";
        let matches = is_star
            || inm.split(',').map(|s| s.trim()).any(|candidate| {
                candidate == et
                    || (is_weak && candidate.strip_prefix("W/").is_some_and(|c| c == &et[2..]))
                    || (!is_weak && candidate.strip_prefix("W/").is_some_and(|c| c == et))
            });

        if matches {
            out.push_str(
                "Result: 304 NOT MODIFIED â€” client cache is up-to-date, no body transfer needed.\n",
            );
        } else {
            out.push_str(
                "Result: 200 OK â€” ETag does not match, server should return fresh content with new ETag.\n",
            );
        }
    }

    if let Some(et) = etag.filter(|_| if_none_match.is_none()) {
        has_input = true;
        let is_weak = et.starts_with("W/");
        out.push_str(&format!("ETag: {}\n\n", et));
        out.push_str(&format!(
            "Type: {} ETag\n",
            if is_weak { "Weak" } else { "Strong" }
        ));
        out.push_str("Provide 'if_none_match' to simulate the conditional request.\n\n");
        out.push_str("Usage flow:\n");
        out.push_str(&format!("  Server sends  ETag: {}\n", et));
        out.push_str(&format!("  Client sends  If-None-Match: {}\n", et));
        out.push_str(
            "  Server returns 304 if unchanged (no body), 200 with new ETag if changed.\n",
        );
    }

    if let (Some(lm), Some(ims)) = (last_modified, if_modified_since) {
        has_input = true;
        out.push('\n');
        out.push_str(&format!("Last-Modified    : {}\n", lm));
        out.push_str(&format!("If-Modified-Since: {}\n\n", ims));
        out.push_str("Note: Timestamp comparison â€” 304 if Last-Modified <= If-Modified-Since.\n");
        out.push_str(
            "If-Modified-Since precision is 1 second. Use ETag for sub-second change detection.\n",
        );
    }

    if !has_input {
        return Err("Provide 'etag' and/or 'if_none_match' (and optionally 'last_modified'/'if_modified_since') to analyze conditional requests.".to_string());
    }
    Ok(out)
}

fn action_vary(args: &Value) -> Result<String, String> {
    let vary = args
        .get("vary")
        .or_else(|| args.get("header"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'vary' with a Vary header value.")?;

    let mut out = format!("Vary: {}\n\n", vary);

    if vary.trim() == "*" {
        out.push_str(
            "Vary: * â€” response is unique per request. Effectively uncacheable by shared caches.\n",
        );
        return Ok(out);
    }

    let fields: Vec<&str> = vary.split(',').map(|s| s.trim()).collect();
    out.push_str(&format!(
        "{} field(s) included in cache key:\n\n",
        fields.len()
    ));

    let mut warnings: Vec<String> = vec![];
    for f in &fields {
        let lower = f.to_lowercase();
        let note = match lower.as_str() {
            "accept-encoding" => "Different encodings (gzip, br, identity) get separate entries â€” set this whenever you compress responses.",
            "accept-language" => "Different language preferences get separate entries â€” required for multi-language content negotiation.",
            "accept" => "Different content types (json, html, xml) get separate entries â€” common for REST APIs serving multiple formats.",
            "origin" => "CORS: different origins get separate entries â€” required when Access-Control-Allow-Origin varies.",
            "cookie" | "set-cookie" => {
                warnings.push(format!("âš  Vary on {} creates per-user cache entries â€” effectively private, shared caches usually won't store.", f));
                "Per-user cache key â€” shared caches typically won't cache at all."
            }
            "authorization" => {
                warnings.push("âš  Vary on Authorization makes responses private â€” CDNs generally skip these.".to_string());
                "Per-credential cache key â€” shared caches typically won't cache at all."
            }
            "user-agent" => {
                warnings.push("âš  Vary on User-Agent causes massive cache fragmentation â€” avoid unless truly necessary.".to_string());
                "Per-browser cache key â€” thousands of unique User-Agent strings = near-empty cache hit rate."
            }
            "save-data" => "Data-saving preference gets separate entries â€” useful for lite vs full page variants.",
            _ => "Non-standard or custom header â€” verify all shared caches support arbitrary Vary fields.",
        };
        out.push_str(&format!("  {:<25}  {}\n", f, note));
    }

    out.push_str(&format!("\nCache Key = URL + {}\n", fields.join(" + ")));
    out.push_str(
        "Each unique combination of these header values creates a separate cache entry.\n",
    );

    if !warnings.is_empty() {
        out.push('\n');
        for w in &warnings {
            out.push_str(&format!("{}\n", w));
        }
    }
    Ok(out)
}
