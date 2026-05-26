use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha384, Sha512};

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("decode");

    match action {
        "decode" => decode(args),
        "verify" => verify(args),
        "sign" => sign(args),
        "inspect" => inspect(args),
        other => Err(format!(
            "jwt_tools: unknown action '{other}'. Valid: decode, verify, sign, inspect"
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn split_jwt(token: &str) -> Result<(&str, &str, &str), String> {
    let parts: Vec<&str> = token.trim().splitn(4, '.').collect();
    if parts.len() != 3 {
        return Err(format!(
            "jwt_tools: expected 3 dot-separated parts, got {}",
            parts.len()
        ));
    }
    Ok((parts[0], parts[1], parts[2]))
}

fn b64_decode_json(s: &str) -> Result<serde_json::Value, String> {
    let bytes = URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|e| format!("jwt_tools: base64 decode error: {e}"))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| format!("jwt_tools: JSON parse error in JWT segment: {e}"))
}

fn b64_encode(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

fn format_timestamp(ts: i64) -> String {
    // Simple UTC display without chrono for robustness
    let secs = ts;
    // Days since Unix epoch
    let days = secs / 86400;
    let rem = secs % 86400;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;
    // Gregorian calendar calculation
    let (y, mo, d) = days_to_ymd(days);
    format!("{y:04}-{mo:02}-{d:02} {:02}:{h:02}:{m:02}:{s:02} UTC", 0)
        .replace("00:", "")
        .trim()
        .to_string();
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02} UTC")
}

fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    // Algorithm from https://www.researchgate.net/publication/316558298
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn hmac_sign(alg: &str, key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    match alg {
        "HS256" => {
            let mut mac = Hmac::<Sha256>::new_from_slice(key)
                .map_err(|e| format!("jwt_tools: HMAC key error: {e}"))?;
            mac.update(data);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        "HS384" => {
            let mut mac = Hmac::<Sha384>::new_from_slice(key)
                .map_err(|e| format!("jwt_tools: HMAC key error: {e}"))?;
            mac.update(data);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        "HS512" => {
            let mut mac = Hmac::<Sha512>::new_from_slice(key)
                .map_err(|e| format!("jwt_tools: HMAC key error: {e}"))?;
            mac.update(data);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        other => Err(format!(
            "jwt_tools: unsupported algorithm '{other}'. Supported: HS256, HS384, HS512"
        )),
    }
}

fn pretty_claims(v: &serde_json::Value) -> String {
    let mut lines = Vec::new();
    if let Some(obj) = v.as_object() {
        let now = now_unix();
        for (k, val) in obj {
            let display = match k.as_str() {
                "exp" | "iat" | "nbf" => {
                    if let Some(ts) = val.as_i64() {
                        let human = format_timestamp(ts);
                        match k.as_str() {
                            "exp" => {
                                let diff = ts - now;
                                let state = if diff > 0 {
                                    format!("valid for {}s", diff)
                                } else {
                                    format!("EXPIRED {}s ago", -diff)
                                };
                                format!("{ts}  ({human})  [{state}]")
                            }
                            _ => format!("{ts}  ({human})"),
                        }
                    } else {
                        val.to_string()
                    }
                }
                _ => val.to_string(),
            };
            lines.push(format!("  {k:<12} : {display}"));
        }
    }
    lines.join("\n")
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn decode(args: &serde_json::Value) -> Result<String, String> {
    let token = args
        .get("token")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("jwt_tools decode: 'token' is required")?;

    let (header_b64, payload_b64, sig_b64) = split_jwt(token)?;
    let header: serde_json::Value = b64_decode_json(header_b64)?;
    let payload: serde_json::Value = b64_decode_json(payload_b64)?;

    let alg = header
        .get("alg")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let mut out = format!("JWT DECODE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Algorithm : {alg}\n"));
    if let Some(typ) = header.get("typ").and_then(|v| v.as_str()) {
        out.push_str(&format!("Type      : {typ}\n"));
    }
    if let Some(kid) = header.get("kid").and_then(|v| v.as_str()) {
        out.push_str(&format!("Key ID    : {kid}\n"));
    }
    out.push_str("\nHEADER\n");
    out.push_str(&format!(
        "{}\n",
        serde_json::to_string_pretty(&header).unwrap_or_default()
    ));
    out.push_str("\nPAYLOAD (claims)\n");
    out.push_str(&pretty_claims(&payload));
    out.push('\n');
    out.push_str("\nSIGNATURE\n");
    out.push_str(&format!("  {sig_b64}\n"));
    out.push_str("\nNote: signature not verified — use action='verify' with a secret.\n");
    Ok(out)
}

fn verify(args: &serde_json::Value) -> Result<String, String> {
    let token = args
        .get("token")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("jwt_tools verify: 'token' is required")?;
    let secret = args
        .get("secret")
        .or_else(|| args.get("key"))
        .and_then(|v| v.as_str())
        .ok_or("jwt_tools verify: 'secret' is required")?;

    let (header_b64, payload_b64, sig_b64) = split_jwt(token)?;
    let header: serde_json::Value = b64_decode_json(header_b64)?;
    let payload: serde_json::Value = b64_decode_json(payload_b64)?;

    let alg = header
        .get("alg")
        .and_then(|v| v.as_str())
        .unwrap_or("HS256");

    let signing_input = format!("{header_b64}.{payload_b64}");
    let expected = hmac_sign(alg, secret.as_bytes(), signing_input.as_bytes())?;
    let expected_b64 = b64_encode(&expected);

    let sig_valid = sig_b64 == expected_b64;

    // Time checks
    let now = now_unix();
    let exp = payload.get("exp").and_then(|v| v.as_i64());
    let nbf = payload.get("nbf").and_then(|v| v.as_i64());
    let time_valid = exp.map(|e| now < e).unwrap_or(true) && nbf.map(|n| now >= n).unwrap_or(true);

    let mut out = format!("JWT VERIFY\n{}\n", "─".repeat(50));
    out.push_str(&format!("Algorithm        : {alg}\n"));
    out.push_str(&format!(
        "Signature        : {}\n",
        if sig_valid {
            "VALID ✓"
        } else {
            "INVALID ✗"
        }
    ));

    if let Some(e) = exp {
        let diff = e - now;
        let state = if diff > 0 {
            format!("valid for {diff}s")
        } else {
            format!("EXPIRED {}s ago", -diff)
        };
        out.push_str(&format!(
            "Expiry           : {} ({state})\n",
            format_timestamp(e)
        ));
    } else {
        out.push_str("Expiry           : (none — token does not expire)\n");
    }

    if let Some(n) = nbf {
        let active = now >= n;
        out.push_str(&format!(
            "Not-before       : {}  [{}]\n",
            format_timestamp(n),
            if active { "active" } else { "NOT YET ACTIVE" }
        ));
    }

    out.push_str("\nClaims\n");
    out.push_str(&pretty_claims(&payload));
    out.push('\n');

    let overall = sig_valid && time_valid;
    out.push_str(&format!(
        "\nVerdict : {}\n",
        if overall {
            "VALID — signature correct and token is within its validity window"
        } else if !sig_valid {
            "INVALID — signature mismatch"
        } else {
            "INVALID — signature correct but token is outside its validity window"
        }
    ));
    Ok(out)
}

fn sign(args: &serde_json::Value) -> Result<String, String> {
    let secret = args
        .get("secret")
        .or_else(|| args.get("key"))
        .and_then(|v| v.as_str())
        .ok_or("jwt_tools sign: 'secret' is required")?;
    let alg = args
        .get("algorithm")
        .or_else(|| args.get("alg"))
        .and_then(|v| v.as_str())
        .unwrap_or("HS256");
    let claims = args
        .get("claims")
        .or_else(|| args.get("payload"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    // Build header
    let header = serde_json::json!({"alg": alg, "typ": "JWT"});
    let header_b64 = b64_encode(
        serde_json::to_string(&header)
            .map_err(|e| format!("jwt_tools sign: header serialize error: {e}"))?
            .as_bytes(),
    );
    let payload_str = serde_json::to_string(&claims)
        .map_err(|e| format!("jwt_tools sign: payload serialize error: {e}"))?;
    let payload_b64 = b64_encode(payload_str.as_bytes());

    let signing_input = format!("{header_b64}.{payload_b64}");
    let sig = hmac_sign(alg, secret.as_bytes(), signing_input.as_bytes())?;
    let sig_b64 = b64_encode(&sig);

    let token = format!("{signing_input}.{sig_b64}");

    let mut out = format!("JWT SIGN\n{}\n", "─".repeat(50));
    out.push_str(&format!("Algorithm : {alg}\n"));
    out.push_str("\nClaims\n");
    out.push_str(&pretty_claims(&claims));
    out.push('\n');
    out.push_str(&format!("\nToken\n  {token}\n"));
    Ok(out)
}

fn inspect(args: &serde_json::Value) -> Result<String, String> {
    let token = args
        .get("token")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("jwt_tools inspect: 'token' is required")?;

    let (header_b64, payload_b64, _sig_b64) = split_jwt(token)?;
    let header: serde_json::Value = b64_decode_json(header_b64)?;
    let payload: serde_json::Value = b64_decode_json(payload_b64)?;

    let now = now_unix();
    let alg = header
        .get("alg")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let exp = payload.get("exp").and_then(|v| v.as_i64());
    let iat = payload.get("iat").and_then(|v| v.as_i64());
    let nbf = payload.get("nbf").and_then(|v| v.as_i64());
    let sub = payload.get("sub").and_then(|v| v.as_str());
    let iss = payload.get("iss").and_then(|v| v.as_str());
    let aud = payload.get("aud");

    let expired = exp.map(|e| now >= e).unwrap_or(false);
    let not_yet_active = nbf.map(|n| now < n).unwrap_or(false);

    let status = if expired {
        "EXPIRED"
    } else if not_yet_active {
        "NOT YET ACTIVE"
    } else {
        "ACTIVE"
    };

    let mut out = format!("JWT INSPECT\n{}\n", "─".repeat(50));
    out.push_str(&format!("Status    : {status}\n"));
    out.push_str(&format!("Algorithm : {alg}\n"));

    if let Some(s) = sub {
        out.push_str(&format!("Subject   : {s}\n"));
    }
    if let Some(i) = iss {
        out.push_str(&format!("Issuer    : {i}\n"));
    }
    if let Some(a) = aud {
        out.push_str(&format!("Audience  : {a}\n"));
    }

    if let Some(e) = exp {
        let diff = e - now;
        if diff > 0 {
            let h = diff / 3600;
            let m = (diff % 3600) / 60;
            out.push_str(&format!(
                "Expires   : {} (in {h}h {m}m)\n",
                format_timestamp(e)
            ));
        } else {
            let ago = -diff;
            out.push_str(&format!(
                "Expired   : {} ({}s ago)\n",
                format_timestamp(e),
                ago
            ));
        }
    } else {
        out.push_str("Expires   : never\n");
    }

    if let Some(i) = iat {
        out.push_str(&format!("Issued at : {}\n", format_timestamp(i)));
    }
    if let Some(n) = nbf {
        out.push_str(&format!("Not before: {}\n", format_timestamp(n)));
    }

    // Claim count
    let claim_count = payload.as_object().map(|o| o.len()).unwrap_or(0);
    out.push_str(&format!("Claims    : {claim_count} total\n"));

    out.push_str("\nNote: use action='verify' with a secret to check the signature.\n");
    Ok(out)
}
