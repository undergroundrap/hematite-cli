use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "webhook_tools",
        "description": "Verify, generate, and explain webhook HMAC-SHA256 signatures for GitHub, Stripe, Slack, Shopify, and generic endpoints. Constant-time comparison prevents timing attacks.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["verify", "check", "generate", "sign", "explain", "parse"],
                    "description": "verify/check (default — validate signature; VALID/INVALID), generate/sign (produce header value), explain/parse (show each provider's format and algorithm)"
                },
                "provider": { "type": "string", "description": "Webhook provider: github, stripe, slack, shopify, or generic (default)" },
                "secret": { "type": "string", "description": "Webhook signing secret" },
                "body": { "type": "string", "description": "Raw request body string" },
                "signature": { "type": "string", "description": "Signature header value to verify against" },
                "sig": { "type": "string", "description": "Alias for 'signature'" },
                "timestamp": { "type": "string", "description": "Request timestamp (required for Stripe and Slack)" },
                "ts": { "type": "string", "description": "Alias for 'timestamp'" }
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("verify");
    match action {
        "verify" | "check" => action_verify(args),
        "generate" | "sign" => action_generate(args),
        "explain" | "parse" => action_explain(args),
        _ => Err(format!(
            "Unknown action '{}'. Use: verify, generate, explain.",
            action
        )),
    }
}

fn hmac_sha256(secret: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key size");
    mac.update(payload);
    mac.finalize().into_bytes().to_vec()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err("Hex string has odd length".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

enum Provider {
    GitHub,
    Stripe,
    Slack,
    Shopify,
    Generic,
}

fn detect_provider(args: &Value) -> Provider {
    match args
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "github" | "gh" => Provider::GitHub,
        "stripe" => Provider::Stripe,
        "slack" => Provider::Slack,
        "shopify" => Provider::Shopify,
        _ => Provider::Generic,
    }
}

fn action_verify(args: &Value) -> Result<String, String> {
    let secret = args
        .get("secret")
        .and_then(|v| v.as_str())
        .ok_or("Provide 'secret' (the webhook signing secret).")?;
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or("Provide 'body' (the raw request body).")?;
    let signature = args
        .get("signature")
        .or_else(|| args.get("sig"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'signature' (the webhook signature header value).")?;

    let provider = detect_provider(args);

    let mut out = String::from("Webhook Signature Verification\n");
    out.push_str(&format!("{}\n\n", "═".repeat(45)));

    match provider {
        Provider::GitHub => {
            out.push_str("Provider: GitHub (X-Hub-Signature-256)\n\n");
            // Signature format: "sha256=<hex>"
            let expected_hex = if let Some(h) = signature.strip_prefix("sha256=") {
                h
            } else {
                signature
            };
            let computed = hmac_sha256(secret.as_bytes(), body.as_bytes());
            let computed_hex = bytes_to_hex(&computed);
            let provided =
                hex_decode(expected_hex).map_err(|e| format!("Invalid signature hex: {e}"))?;
            if constant_time_eq(&computed, &provided) {
                out.push_str("✅ VALID — signature matches\n");
            } else {
                out.push_str("❌ INVALID — signature mismatch\n");
                out.push_str(&format!("   Expected : sha256={}\n", computed_hex));
                out.push_str(&format!("   Got      : sha256={}\n", expected_hex));
            }
        }
        Provider::Stripe => {
            out.push_str("Provider: Stripe (Stripe-Signature)\n\n");
            // Format: "t=<timestamp>,v1=<hex>[,v0=<old>]"
            let timestamp = args
                .get("timestamp")
                .or_else(|| args.get("ts"))
                .and_then(|v| v.as_str());

            let ts = timestamp.or_else(|| {
                signature
                    .split(',')
                    .find(|s| s.starts_with("t="))
                    .and_then(|s| s.strip_prefix("t="))
            });

            let v1_sig = signature
                .split(',')
                .find(|s| s.starts_with("v1="))
                .and_then(|s| s.strip_prefix("v1="));

            match (ts, v1_sig) {
                (Some(ts_val), Some(sig_hex)) => {
                    let payload = format!("{}.{}", ts_val, body);
                    let computed = hmac_sha256(secret.as_bytes(), payload.as_bytes());
                    let computed_hex = bytes_to_hex(&computed);
                    let provided =
                        hex_decode(sig_hex).map_err(|e| format!("Invalid v1 hex: {e}"))?;
                    out.push_str(&format!("Timestamp : {}\n", ts_val));
                    out.push_str(&format!(
                        "Payload   : {}.<body> ({} bytes)\n\n",
                        ts_val,
                        payload.len()
                    ));
                    if constant_time_eq(&computed, &provided) {
                        out.push_str("✅ VALID — v1 signature matches\n");
                    } else {
                        out.push_str("❌ INVALID — v1 signature mismatch\n");
                        out.push_str(&format!("   Expected: v1={}\n", computed_hex));
                        out.push_str(&format!("   Got     : v1={}\n", sig_hex));
                    }
                }
                (None, _) => {
                    return Err(
                        "Stripe verification requires 'timestamp' arg or t= in signature."
                            .to_string(),
                    );
                }
                (_, None) => {
                    return Err("No v1= component found in Stripe-Signature header.".to_string());
                }
            }
        }
        Provider::Slack => {
            out.push_str("Provider: Slack (X-Slack-Signature)\n\n");
            // Format: "v0=<hex>"
            let timestamp = args
                .get("timestamp")
                .or_else(|| args.get("ts"))
                .and_then(|v| v.as_str())
                .ok_or(
                    "Slack verification requires 'timestamp' (X-Slack-Request-Timestamp value).",
                )?;
            let sig_hex = signature.strip_prefix("v0=").unwrap_or(signature);
            let payload = format!("v0:{}:{}", timestamp, body);
            let computed = hmac_sha256(secret.as_bytes(), payload.as_bytes());
            let computed_hex = bytes_to_hex(&computed);
            let provided =
                hex_decode(sig_hex).map_err(|e| format!("Invalid signature hex: {e}"))?;
            out.push_str(&format!("Timestamp  : {}\n", timestamp));
            out.push_str(&format!("Payload    : v0:{}:<body>\n\n", timestamp));
            if constant_time_eq(&computed, &provided) {
                out.push_str("✅ VALID — signature matches\n");
            } else {
                out.push_str("❌ INVALID — signature mismatch\n");
                out.push_str(&format!("   Expected: v0={}\n", computed_hex));
                out.push_str(&format!("   Got     : v0={}\n", sig_hex));
            }
        }
        Provider::Shopify => {
            out.push_str("Provider: Shopify (X-Shopify-Hmac-SHA256)\n\n");
            // Format: base64(HMAC-SHA256(secret, body))
            let computed = hmac_sha256(secret.as_bytes(), body.as_bytes());
            let computed_b64 = B64.encode(&computed);
            let sig_trimmed = signature.trim();
            let provided = B64
                .decode(sig_trimmed)
                .map_err(|e| format!("Invalid base64 signature: {e}"))?;
            if constant_time_eq(&computed, &provided) {
                out.push_str("✅ VALID — signature matches\n");
            } else {
                out.push_str("❌ INVALID — signature mismatch\n");
                out.push_str(&format!("   Expected: {}\n", computed_b64));
                out.push_str(&format!("   Got     : {}\n", sig_trimmed));
            }
        }
        Provider::Generic => {
            out.push_str("Provider: Generic HMAC-SHA256\n\n");
            // Signature assumed to be hex of HMAC-SHA256(secret, body)
            let sig_hex = signature.trim();
            let computed = hmac_sha256(secret.as_bytes(), body.as_bytes());
            let computed_hex = bytes_to_hex(&computed);
            let provided =
                hex_decode(sig_hex).map_err(|e| format!("Invalid signature hex: {e}"))?;
            if constant_time_eq(&computed, &provided) {
                out.push_str("✅ VALID — HMAC-SHA256 matches\n");
            } else {
                out.push_str("❌ INVALID — HMAC-SHA256 mismatch\n");
                out.push_str(&format!("   Expected: {}\n", computed_hex));
                out.push_str(&format!("   Got     : {}\n", sig_hex));
            }
        }
    }
    Ok(out)
}

fn action_generate(args: &Value) -> Result<String, String> {
    let secret = args
        .get("secret")
        .and_then(|v| v.as_str())
        .ok_or("Provide 'secret' (the webhook signing secret).")?;
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or("Provide 'body' (the request body to sign).")?;

    let provider = detect_provider(args);
    let mut out = String::from("Webhook Signature Generation\n");
    out.push_str(&format!("{}\n\n", "═".repeat(45)));

    match provider {
        Provider::GitHub => {
            out.push_str("Provider: GitHub\n\n");
            let computed = hmac_sha256(secret.as_bytes(), body.as_bytes());
            let sig = format!("sha256={}", bytes_to_hex(&computed));
            out.push_str(&format!(
                "Signature header:\n  X-Hub-Signature-256: {}\n",
                sig
            ));
            out.push_str("\nAlgorithm: HMAC-SHA256(secret, body)\n");
        }
        Provider::Stripe => {
            let timestamp = args
                .get("timestamp")
                .or_else(|| args.get("ts"))
                .and_then(|v| v.as_str())
                .unwrap_or("1234567890");
            out.push_str(&format!("Provider: Stripe (timestamp={})\n\n", timestamp));
            let payload = format!("{}.{}", timestamp, body);
            let computed = hmac_sha256(secret.as_bytes(), payload.as_bytes());
            let sig_hex = bytes_to_hex(&computed);
            out.push_str(&format!(
                "Signature header:\n  Stripe-Signature: t={},v1={}\n",
                timestamp, sig_hex
            ));
            out.push_str("\nAlgorithm: HMAC-SHA256(secret, timestamp + \".\" + body)\n");
        }
        Provider::Slack => {
            let timestamp = args
                .get("timestamp")
                .or_else(|| args.get("ts"))
                .and_then(|v| v.as_str())
                .unwrap_or("1234567890");
            out.push_str(&format!("Provider: Slack (timestamp={})\n\n", timestamp));
            let payload = format!("v0:{}:{}", timestamp, body);
            let computed = hmac_sha256(secret.as_bytes(), payload.as_bytes());
            let sig_hex = bytes_to_hex(&computed);
            out.push_str(&format!(
                "Signature headers:\n  X-Slack-Signature: v0={}\n  X-Slack-Request-Timestamp: {}\n",
                sig_hex, timestamp
            ));
            out.push_str("\nAlgorithm: HMAC-SHA256(secret, \"v0:\" + timestamp + \":\" + body)\n");
        }
        Provider::Shopify => {
            out.push_str("Provider: Shopify\n\n");
            let computed = hmac_sha256(secret.as_bytes(), body.as_bytes());
            let sig_b64 = B64.encode(&computed);
            out.push_str(&format!(
                "Signature header:\n  X-Shopify-Hmac-SHA256: {}\n",
                sig_b64
            ));
            out.push_str("\nAlgorithm: BASE64(HMAC-SHA256(secret, body))\n");
        }
        Provider::Generic => {
            out.push_str("Provider: Generic\n\n");
            let computed = hmac_sha256(secret.as_bytes(), body.as_bytes());
            let sig_hex = bytes_to_hex(&computed);
            out.push_str(&format!("HMAC-SHA256 (hex): {}\n", sig_hex));
            out.push_str(&format!("HMAC-SHA256 (b64): {}\n", B64.encode(&computed)));
            out.push_str("\nAlgorithm: HMAC-SHA256(secret, body)\n");
        }
    }
    Ok(out)
}

fn action_explain(args: &Value) -> Result<String, String> {
    let provider_str = args
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("all")
        .to_lowercase();

    let mut out = String::from("Webhook Signature Formats\n");
    out.push_str(&format!("{}\n\n", "═".repeat(55)));

    let show_all = provider_str == "all" || provider_str.is_empty();

    if show_all || provider_str.contains("github") || provider_str.contains("gh") {
        out.push_str("── GitHub ──\n");
        out.push_str("  Header   : X-Hub-Signature-256: sha256=<hex>\n");
        out.push_str("  Payload  : raw request body (bytes)\n");
        out.push_str("  Algorithm: HMAC-SHA256(secret, body) → hex\n");
        out.push_str("  Timing   : check X-GitHub-Delivery for replay detection\n\n");
    }

    if show_all || provider_str.contains("stripe") {
        out.push_str("── Stripe ──\n");
        out.push_str("  Header   : Stripe-Signature: t=<timestamp>,v1=<hex>[,v1=<hex2>]\n");
        out.push_str("  Payload  : timestamp + \".\" + body\n");
        out.push_str("  Algorithm: HMAC-SHA256(secret, payload) → hex (v1 scheme)\n");
        out.push_str("  Tolerance: reject if |now - timestamp| > 300 seconds\n\n");
    }

    if show_all || provider_str.contains("slack") {
        out.push_str("── Slack ──\n");
        out.push_str("  Headers  : X-Slack-Signature: v0=<hex>\n");
        out.push_str("             X-Slack-Request-Timestamp: <unix-epoch>\n");
        out.push_str("  Payload  : \"v0:\" + timestamp + \":\" + body\n");
        out.push_str("  Algorithm: HMAC-SHA256(secret, payload) → hex with \"v0=\" prefix\n");
        out.push_str("  Tolerance: reject if |now - timestamp| > 300 seconds\n\n");
    }

    if show_all || provider_str.contains("shopify") {
        out.push_str("── Shopify ──\n");
        out.push_str("  Header   : X-Shopify-Hmac-SHA256: <base64>\n");
        out.push_str("  Payload  : raw request body (bytes)\n");
        out.push_str("  Algorithm: HMAC-SHA256(secret, body) → base64 (standard encoding)\n\n");
    }

    if show_all {
        out.push_str("── Security Notes ──\n");
        out.push_str("  • Always use constant-time comparison to prevent timing attacks\n");
        out.push_str("  • Validate timestamp freshness (±5 min) to prevent replay attacks\n");
        out.push_str("  • Verify before parsing body content — reject early on mismatch\n");
        out.push_str("  • Use the raw body bytes, not a re-serialized/parsed version\n");
    }

    Ok(out)
}
