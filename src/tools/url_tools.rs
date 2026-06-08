use url::Url;

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");

    match action {
        "parse" => parse(args),
        "build" => build(args),
        "params" => params(args),
        "encode" => encode(args),
        "decode" => decode(args),
        "normalize" => normalize(args),
        "validate" => validate(args),
        other => Err(format!(
            "url_tools: unknown action '{other}'. Valid: parse, build, params, encode, decode, normalize, validate"
        )),
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn get_url_str(args: &serde_json::Value) -> Result<String, String> {
    args.get("url")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "url_tools: 'url' is required".to_string())
}

fn parse_qs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|s| !s.is_empty())
        .map(|pair| {
            if let Some(eq) = pair.find('=') {
                let k = percent_decode(&pair[..eq]);
                let v = percent_decode(&pair[eq + 1..]);
                (k, v)
            } else {
                (percent_decode(pair), String::new())
            }
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .to_string()
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn parse(args: &serde_json::Value) -> Result<String, String> {
    let raw = get_url_str(args)?;
    let url = Url::parse(&raw).map_err(|e| format!("url_tools: invalid URL: {e}"))?;

    let mut out = format!("URL PARSE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input    : {raw}\n\n"));

    out.push_str(&format!("Scheme   : {}\n", url.scheme()));

    if let Some(host) = url.host_str() {
        out.push_str(&format!("Host     : {host}\n"));
    }
    if let Some(port) = url.port() {
        out.push_str(&format!("Port     : {port}\n"));
    } else if let Some(port) = url.port_or_known_default() {
        out.push_str(&format!("Port     : {port} (default)\n"));
    }

    if !url.username().is_empty() {
        out.push_str(&format!("Username : {}\n", url.username()));
    }
    if url.password().is_some() {
        out.push_str("Password : [set]\n");
    }

    out.push_str(&format!("Path     : {}\n", url.path()));

    if let Some(q) = url.query() {
        out.push_str(&format!("Query    : {q}\n"));
        let params = parse_qs(q);
        if !params.is_empty() {
            out.push_str("\nQuery Parameters:\n");
            for (k, v) in &params {
                out.push_str(&format!("  {k} = {v}\n"));
            }
        }
    }

    if let Some(frag) = url.fragment() {
        out.push_str(&format!("Fragment : {frag}\n"));
    }

    out.push_str(&format!(
        "\nOrigin   : {}",
        url.origin().unicode_serialization()
    ));
    out.push('\n');
    Ok(out)
}

fn build(args: &serde_json::Value) -> Result<String, String> {
    let scheme = args
        .get("scheme")
        .and_then(|v| v.as_str())
        .unwrap_or("https");
    let host = args
        .get("host")
        .and_then(|v| v.as_str())
        .ok_or("url_tools build: 'host' is required")?;
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("/");
    let port = args.get("port").and_then(|v| v.as_u64());
    let fragment = args.get("fragment").and_then(|v| v.as_str());

    // Build query string from params object or string
    let query_str = if let Some(q) = args.get("query").and_then(|v| v.as_str()) {
        q.to_string()
    } else if let Some(obj) = args.get("params").and_then(|v| v.as_object()) {
        obj.iter()
            .map(|(k, v)| {
                let val = v
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| v.to_string());
                format!(
                    "{}={}",
                    percent_encoding::utf8_percent_encode(k, percent_encoding::NON_ALPHANUMERIC),
                    percent_encoding::utf8_percent_encode(&val, percent_encoding::NON_ALPHANUMERIC)
                )
            })
            .collect::<Vec<_>>()
            .join("&")
    } else {
        String::new()
    };

    let port_part = port.map(|p| format!(":{p}")).unwrap_or_default();
    let query_part = if query_str.is_empty() {
        String::new()
    } else {
        format!("?{query_str}")
    };
    let frag_part = fragment.map(|f| format!("#{f}")).unwrap_or_default();

    let built = format!("{scheme}://{host}{port_part}{path}{query_part}{frag_part}");

    // Validate the result
    Url::parse(&built).map_err(|e| format!("url_tools build: constructed URL is invalid: {e}"))?;

    let mut out = format!("URL BUILD\n{}\n", "─".repeat(50));
    out.push_str(&format!("{built}\n"));
    Ok(out)
}

fn params(args: &serde_json::Value) -> Result<String, String> {
    let raw = get_url_str(args)?;
    let url = Url::parse(&raw).map_err(|e| format!("url_tools: invalid URL: {e}"))?;

    let action2 = args
        .get("op")
        .or_else(|| args.get("operation"))
        .and_then(|v| v.as_str())
        .unwrap_or("list");

    let mut pairs = parse_qs(url.query().unwrap_or(""));

    match action2 {
        "list" | "get" => {
            let mut out = format!("URL PARAMS\n{}\n", "─".repeat(50));
            out.push_str(&format!("URL: {raw}\n\n"));
            if pairs.is_empty() {
                out.push_str("No query parameters.\n");
            } else {
                out.push_str(&format!("{:<30} {}\n", "Parameter", "Value"));
                out.push_str(&format!("{}\n", "─".repeat(60)));
                for (k, v) in &pairs {
                    out.push_str(&format!("{:<30} {}\n", k, v));
                }
                out.push_str(&format!("\n{} parameter(s)\n", pairs.len()));
            }
            Ok(out)
        }
        "set" | "add" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("url_tools params set: 'key' is required")?;
            let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
            pairs.retain(|(k, _)| k != key);
            pairs.push((key.to_string(), value.to_string()));

            let qs = pairs
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}={}",
                        percent_encoding::utf8_percent_encode(
                            k,
                            percent_encoding::NON_ALPHANUMERIC
                        ),
                        percent_encoding::utf8_percent_encode(
                            v,
                            percent_encoding::NON_ALPHANUMERIC
                        )
                    )
                })
                .collect::<Vec<_>>()
                .join("&");

            let mut url2 = url.clone();
            url2.set_query(if qs.is_empty() { None } else { Some(&qs) });

            let mut out = format!("URL PARAMS SET\n{}\n", "─".repeat(50));
            out.push_str(&format!("Original : {raw}\n"));
            out.push_str(&format!("Modified : {url2}\n"));
            Ok(out)
        }
        "remove" | "delete" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("url_tools params remove: 'key' is required")?;
            pairs.retain(|(k, _)| k != key);

            let qs = pairs
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}={}",
                        percent_encoding::utf8_percent_encode(
                            k,
                            percent_encoding::NON_ALPHANUMERIC
                        ),
                        percent_encoding::utf8_percent_encode(
                            v,
                            percent_encoding::NON_ALPHANUMERIC
                        )
                    )
                })
                .collect::<Vec<_>>()
                .join("&");

            let mut url2 = url.clone();
            url2.set_query(if qs.is_empty() { None } else { Some(&qs) });

            let mut out = format!("URL PARAMS REMOVE\n{}\n", "─".repeat(50));
            out.push_str(&format!("Original : {raw}\n"));
            out.push_str(&format!("Modified : {url2}\n"));
            Ok(out)
        }
        other => Err(format!(
            "url_tools params: unknown op '{other}'. Valid: list, set, remove"
        )),
    }
}

fn encode(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .or_else(|| args.get("text"))
        .or_else(|| args.get("url"))
        .and_then(|v| v.as_str())
        .ok_or("url_tools encode: 'input' is required")?;

    let component = args
        .get("component")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let encoded = if component {
        percent_encoding::utf8_percent_encode(input, percent_encoding::NON_ALPHANUMERIC).to_string()
    } else {
        // URL-safe encoding that preserves path separators and common URL chars
        const QUERY_SAFE: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
            .add(b' ')
            .add(b'"')
            .add(b'#')
            .add(b'<')
            .add(b'>');
        percent_encoding::utf8_percent_encode(input, QUERY_SAFE).to_string()
    };

    let mut out = format!("URL ENCODE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input   : {input}\n"));
    out.push_str(&format!("Encoded : {encoded}\n"));
    Ok(out)
}

fn decode(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .or_else(|| args.get("text"))
        .or_else(|| args.get("url"))
        .and_then(|v| v.as_str())
        .ok_or("url_tools decode: 'input' is required")?;

    let decoded = percent_encoding::percent_decode_str(input)
        .decode_utf8()
        .map_err(|e| format!("url_tools decode: invalid UTF-8 after decoding: {e}"))?
        .to_string();

    let mut out = format!("URL DECODE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input   : {input}\n"));
    out.push_str(&format!("Decoded : {decoded}\n"));
    Ok(out)
}

fn normalize(args: &serde_json::Value) -> Result<String, String> {
    let raw = get_url_str(args)?;
    let url = Url::parse(&raw).map_err(|e| format!("url_tools: invalid URL: {e}"))?;

    // Url::parse already normalizes scheme/host to lowercase and resolves dot segments
    let normalized = url.to_string();

    let mut out = format!("URL NORMALIZE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input      : {raw}\n"));
    out.push_str(&format!("Normalized : {normalized}\n"));

    if raw == normalized {
        out.push_str("(already normalized)\n");
    } else {
        out.push_str("Changes applied: lowercase scheme/host, resolved path segments.\n");
    }
    Ok(out)
}

fn validate(args: &serde_json::Value) -> Result<String, String> {
    let raw = get_url_str(args)?;

    let mut out = format!("URL VALIDATE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input : {raw}\n\n"));

    match Url::parse(&raw) {
        Ok(url) => {
            out.push_str("Result : VALID\n\n");
            out.push_str(&format!("Scheme : {}\n", url.scheme()));
            if let Some(h) = url.host_str() {
                out.push_str(&format!("Host   : {h}\n"));
            }
            if url.path() != "/" && !url.path().is_empty() {
                out.push_str(&format!("Path   : {}\n", url.path()));
            }
            if url.query().is_some() {
                out.push_str(&format!(
                    "Params : {} parameter(s)\n",
                    parse_qs(url.query().unwrap_or("")).len()
                ));
            }

            // Warn about common issues
            if url.scheme() == "http" {
                out.push_str("\nNote: using HTTP (not HTTPS) — consider upgrading to HTTPS.\n");
            }
            if url
                .host_str()
                .is_some_and(|h| h == "localhost" || h == "127.0.0.1")
            {
                out.push_str("Note: localhost URL — likely a development endpoint.\n");
            }
        }
        Err(e) => {
            out.push_str("Result : INVALID\n\n");
            out.push_str(&format!("Error  : {e}\n"));

            // Suggest common fixes
            if !raw.contains("://") {
                out.push_str("\nHint: Missing scheme — try prefixing with https://\n");
            }
        }
    }
    Ok(out)
}
