use base64::engine::general_purpose::STANDARD as B64_STANDARD;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64_URL;
use base64::Engine;

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("base64-encode");

    match action {
        "base64-encode" => base64_encode(args),
        "base64-decode" => base64_decode(args),
        "url-encode" => url_encode(args),
        "url-decode" => url_decode(args),
        "hex-encode" => hex_encode(args),
        "hex-decode" => hex_decode(args),
        "jwt-decode" => jwt_decode(args),
        "html-encode" => html_encode(args),
        "html-decode" => html_decode(args),
        other => Err(format!(
            "encode_tools: unknown action '{other}'. Valid: base64-encode, base64-decode, url-encode, url-decode, hex-encode, hex-decode, jwt-decode, html-encode, html-decode"
        )),
    }
}

fn get_input(args: &serde_json::Value) -> Result<String, String> {
    args.get("input")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "encode_tools: 'input' field is required".to_string())
}

fn base64_encode(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let url_safe = args
        .get("url_safe")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let encoded = if url_safe {
        B64_URL.encode(input.as_bytes())
    } else {
        B64_STANDARD.encode(input.as_bytes())
    };

    let variant = if url_safe { "URL-safe" } else { "standard" };
    Ok(format!(
        "BASE64 ENCODE ({variant})\n{}\n{encoded}",
        "─".repeat(50)
    ))
}

fn base64_decode(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let url_safe = args
        .get("url_safe")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let bytes = if url_safe {
        B64_URL
            .decode(input.trim())
            .map_err(|e| format!("encode_tools: base64 decode error: {e}"))?
    } else {
        B64_STANDARD
            .decode(input.trim())
            .map_err(|e| format!("encode_tools: base64 decode error: {e}"))?
    };

    match String::from_utf8(bytes.clone()) {
        Ok(text) => Ok(format!("BASE64 DECODE\n{}\n{text}", "─".repeat(50))),
        Err(_) => Ok(format!(
            "BASE64 DECODE (binary)\n{}\n{} bytes (not valid UTF-8)\nHex: {}",
            "─".repeat(50),
            bytes.len(),
            bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        )),
    }
}

fn url_encode(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let encoded = urlencoding::encode(&input).into_owned();
    Ok(format!("URL ENCODE\n{}\n{encoded}", "─".repeat(50)))
}

fn url_decode(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let decoded = urlencoding::decode(&input)
        .map_err(|e| format!("encode_tools: url decode error: {e}"))?
        .into_owned();
    Ok(format!("URL DECODE\n{}\n{decoded}", "─".repeat(50)))
}

fn hex_encode(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let hex: String = input.bytes().map(|b| format!("{b:02x}")).collect();
    Ok(format!("HEX ENCODE\n{}\n{hex}", "─".repeat(50)))
}

fn hex_decode(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let clean: String = input.chars().filter(|c| !c.is_whitespace()).collect();

    if clean.len() % 2 != 0 {
        return Err("encode_tools: hex string must have an even number of characters".to_string());
    }

    let bytes: Result<Vec<u8>, _> = (0..clean.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&clean[i..i + 2], 16))
        .collect();

    let bytes = bytes.map_err(|e| format!("encode_tools: hex decode error: {e}"))?;

    match String::from_utf8(bytes.clone()) {
        Ok(text) => Ok(format!("HEX DECODE\n{}\n{text}", "─".repeat(50))),
        Err(_) => Ok(format!(
            "HEX DECODE (binary)\n{}\n{} bytes (not valid UTF-8)",
            "─".repeat(50),
            bytes.len()
        )),
    }
}

fn jwt_decode(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let token = input.trim();

    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err(
            "encode_tools: JWT must have 3 parts separated by '.' (header.payload.signature)"
                .to_string(),
        );
    }

    let header_bytes = B64_URL
        .decode(parts[0])
        .map_err(|e| format!("encode_tools: JWT header decode error: {e}"))?;
    let payload_bytes = B64_URL
        .decode(parts[1])
        .map_err(|e| format!("encode_tools: JWT payload decode error: {e}"))?;

    let header_str = String::from_utf8(header_bytes)
        .map_err(|_| "encode_tools: JWT header is not valid UTF-8".to_string())?;
    let payload_str = String::from_utf8(payload_bytes)
        .map_err(|_| "encode_tools: JWT payload is not valid UTF-8".to_string())?;

    let header_pretty = pretty_json(&header_str);
    let payload_pretty = pretty_json(&payload_str);

    let sig_display = if parts[2].len() > 20 {
        format!("{}... ({} chars)", &parts[2][..20], parts[2].len())
    } else {
        parts[2].to_string()
    };

    let mut out = format!("JWT DECODE\n{}\n", "─".repeat(50));
    out.push_str("⚠  Signature NOT verified — decode only\n\n");
    out.push_str("HEADER:\n");
    out.push_str(&header_pretty);
    out.push('\n');
    out.push_str("\nPAYLOAD:\n");
    out.push_str(&payload_pretty);
    out.push('\n');

    // Check for exp claim and show human-readable expiry
    if let Ok(payload_val) = serde_json::from_str::<serde_json::Value>(&payload_str) {
        if let Some(exp) = payload_val.get("exp").and_then(|v| v.as_i64()) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let delta = exp - now;
            let status = if delta > 0 {
                format!("expires in {}s", delta)
            } else {
                format!("EXPIRED {}s ago", -delta)
            };
            out.push_str(&format!("\nexp: {exp} ({status})\n"));
        }
        if let Some(iat) = payload_val.get("iat").and_then(|v| v.as_i64()) {
            out.push_str(&format!("iat: {iat}\n"));
        }
    }

    out.push_str(&format!("\nSIGNATURE: {sig_display}\n"));
    Ok(out)
}

fn html_encode(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let encoded = input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;");
    Ok(format!("HTML ENCODE\n{}\n{encoded}", "─".repeat(50)))
}

fn html_decode(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let decoded = input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");
    Ok(format!("HTML DECODE\n{}\n{decoded}", "─".repeat(50)))
}

fn pretty_json(s: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| s.to_string()),
        Err(_) => s.to_string(),
    }
}
