use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "jwk_tools",
        "description": "Parse, validate, and compute RFC 7638 thumbprints for JSON Web Keys (JWK) and JWKS sets. Works offline — no network calls.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "parse", "validate", "check", "thumbprint", "tp", "list"],
                    "description": "info/parse (default — key type, size, alg, use, thumbprint), validate/check (field checks + weak key warnings), thumbprint/tp (RFC 7638 SHA-256 thumbprint), list (tabular JWKS summary)"
                },
                "jwk": { "type": "string", "description": "JWK JSON object string or JWKS {\"keys\":[...]} object" },
                "jwks": { "type": "string", "description": "Alias for 'jwk'" },
                "key": { "type": "string", "description": "Alias for 'jwk'" },
                "json": { "type": "string", "description": "Alias for 'jwk'" }
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let input = args
        .get("jwk")
        .or_else(|| args.get("jwks"))
        .or_else(|| args.get("key"))
        .or_else(|| args.get("json"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'jwk' with a JWK or JWKS JSON object.")?;

    let parsed: Value = serde_json::from_str(input).map_err(|e| format!("Invalid JSON: {e}"))?;

    // Collect individual keys — handle both single JWK and JWKS {"keys":[...]}
    let keys: Vec<&Value> = if let Some(arr) = parsed.get("keys").and_then(|k| k.as_array()) {
        arr.iter().collect()
    } else {
        vec![&parsed]
    };

    let is_jwks = parsed.get("keys").is_some();

    match action {
        "info" | "parse" => action_info(&keys, is_jwks),
        "validate" | "check" => action_validate(&keys, is_jwks),
        "thumbprint" | "tp" => action_thumbprint(&keys),
        "list" => action_list(&keys),
        _ => Err(format!(
            "Unknown action '{}'. Use: info, validate, thumbprint, list.",
            action
        )),
    }
}

struct JwkInfo {
    kty: String,
    kid: Option<String>,
    alg: Option<String>,
    key_use: Option<String>,
    key_ops: Option<Vec<String>>,
    crv: Option<String>,
    bit_size: Option<usize>,
    has_private: bool,
    has_symmetric: bool,
    raw: Value,
}

fn parse_jwk(key: &Value) -> Result<JwkInfo, String> {
    let kty = key
        .get("kty")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'kty' field")?
        .to_string();

    let kid = key
        .get("kid")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let alg = key
        .get("alg")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let key_use = key
        .get("use")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let key_ops = key.get("key_ops").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect()
    });

    let crv = key
        .get("crv")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Determine bit size from RSA modulus
    let bit_size = match kty.as_str() {
        "RSA" => key
            .get("n")
            .and_then(|v| v.as_str())
            .and_then(|n| URL_SAFE_NO_PAD.decode(n).ok())
            .map(|bytes| {
                // RSA modulus may have leading 0x00 byte for sign; strip it
                let len = if bytes.first() == Some(&0) {
                    bytes.len() - 1
                } else {
                    bytes.len()
                };
                len * 8
            }),
        "EC" => crv.as_deref().map(|c| match c {
            "P-256" => 256,
            "P-384" => 384,
            "P-521" => 521,
            _ => 0,
        }),
        "OKP" => crv.as_deref().map(|c| match c {
            "Ed25519" | "X25519" => 255,
            "Ed448" | "X448" => 448,
            _ => 0,
        }),
        "oct" => key
            .get("k")
            .and_then(|v| v.as_str())
            .and_then(|k| URL_SAFE_NO_PAD.decode(k).ok())
            .map(|bytes| bytes.len() * 8),
        _ => None,
    };

    // Private key indicators per key type
    let has_private = match kty.as_str() {
        "RSA" => key.get("d").is_some(),
        "EC" | "OKP" => key.get("d").is_some(),
        _ => false,
    };

    let has_symmetric = kty == "oct";

    Ok(JwkInfo {
        kty,
        kid,
        alg,
        key_use,
        key_ops,
        crv,
        bit_size,
        has_private,
        has_symmetric,
        raw: key.clone(),
    })
}

fn key_type_label(info: &JwkInfo) -> String {
    match info.kty.as_str() {
        "RSA" => {
            let bits = info
                .bit_size
                .map(|b| format!(" {}-bit", b))
                .unwrap_or_default();
            if info.has_private {
                format!("RSA Private Key{}", bits)
            } else {
                format!("RSA Public Key{}", bits)
            }
        }
        "EC" => {
            let curve = info.crv.as_deref().unwrap_or("unknown curve");
            if info.has_private {
                format!("EC Private Key ({})", curve)
            } else {
                format!("EC Public Key ({})", curve)
            }
        }
        "OKP" => {
            let curve = info.crv.as_deref().unwrap_or("unknown curve");
            if info.has_private {
                format!("OKP Private Key ({})", curve)
            } else {
                format!("OKP Public Key ({})", curve)
            }
        }
        "oct" => {
            let bits = info
                .bit_size
                .map(|b| format!(" {}-bit", b))
                .unwrap_or_default();
            format!("Symmetric Key (oct{})", bits)
        }
        other => format!("{} Key", other),
    }
}

fn use_label(u: &str) -> &str {
    match u {
        "sig" => "Signature (sig)",
        "enc" => "Encryption (enc)",
        _ => u,
    }
}

fn action_info(keys: &[&Value], is_jwks: bool) -> Result<String, String> {
    let mut out = if is_jwks {
        format!("JWKS — {} key(s)\n{}\n\n", keys.len(), "═".repeat(40))
    } else {
        format!("JWK Analysis\n{}\n\n", "═".repeat(40))
    };

    for (i, &key) in keys.iter().enumerate() {
        if is_jwks {
            out.push_str(&format!("── Key {} ──\n", i + 1));
        }

        match parse_jwk(key) {
            Ok(info) => {
                out.push_str(&format!("  Type      : {}\n", key_type_label(&info)));
                if let Some(kid) = &info.kid {
                    out.push_str(&format!("  Key ID    : {}\n", kid));
                }
                if let Some(alg) = &info.alg {
                    out.push_str(&format!("  Algorithm : {}\n", alg));
                }
                if let Some(u) = &info.key_use {
                    out.push_str(&format!("  Use       : {}\n", use_label(u)));
                }
                if let Some(ops) = &info.key_ops {
                    out.push_str(&format!("  Key Ops   : {}\n", ops.join(", ")));
                }
                if let Some(bits) = info.bit_size {
                    out.push_str(&format!("  Key Size  : {} bits\n", bits));
                }
                // Thumbprint inline
                match rfc7638_thumbprint(key) {
                    Ok(tp) => out.push_str(&format!("  Thumbprint: {}\n", tp)),
                    Err(e) => out.push_str(&format!("  Thumbprint: (unavailable: {})\n", e)),
                }
            }
            Err(e) => {
                out.push_str(&format!("  Parse error: {}\n", e));
            }
        }

        out.push('\n');
    }

    Ok(out)
}

fn action_validate(keys: &[&Value], is_jwks: bool) -> Result<String, String> {
    let mut out = if is_jwks {
        format!("JWKS Validation\n{}\n\n", "═".repeat(40))
    } else {
        format!("JWK Validation\n{}\n\n", "═".repeat(40))
    };

    let mut all_valid = true;

    for (i, &key) in keys.iter().enumerate() {
        if is_jwks {
            out.push_str(&format!("── Key {} ──\n", i + 1));
        }

        let mut issues: Vec<String> = vec![];

        match parse_jwk(key) {
            Ok(info) => {
                // kty present — already checked by parse_jwk
                // Check required fields per kty
                match info.kty.as_str() {
                    "RSA" => {
                        if key.get("n").is_none() {
                            issues.push("Missing 'n' (RSA modulus)".into());
                        }
                        if key.get("e").is_none() {
                            issues.push("Missing 'e' (RSA public exponent)".into());
                        }
                        // Warn on small key size
                        if let Some(bits) = info.bit_size {
                            if bits < 2048 {
                                issues.push(format!(
                                    "RSA key size {} bits is below recommended 2048 bits",
                                    bits
                                ));
                            }
                        }
                    }
                    "EC" => {
                        if key.get("crv").is_none() {
                            issues.push("Missing 'crv' (EC curve name)".into());
                        }
                        if key.get("x").is_none() {
                            issues.push("Missing 'x' (EC x-coordinate)".into());
                        }
                        if key.get("y").is_none() {
                            issues.push("Missing 'y' (EC y-coordinate)".into());
                        }
                        if let Some(crv) = &info.crv {
                            if !["P-256", "P-384", "P-521"].contains(&crv.as_str()) {
                                issues.push(format!("Non-standard EC curve: {}", crv));
                            }
                        }
                    }
                    "OKP" => {
                        if key.get("crv").is_none() {
                            issues.push("Missing 'crv' (OKP curve name)".into());
                        }
                        if key.get("x").is_none() {
                            issues.push("Missing 'x' (OKP public key)".into());
                        }
                    }
                    "oct" => {
                        if key.get("k").is_none() {
                            issues.push("Missing 'k' (symmetric key material)".into());
                        }
                        if let Some(bits) = info.bit_size {
                            if bits < 128 {
                                issues.push(format!(
                                    "Symmetric key {} bits is below recommended 128 bits",
                                    bits
                                ));
                            }
                        }
                    }
                    other => {
                        issues.push(format!("Unknown key type: {}", other));
                    }
                }

                // Warn on conflicting use+key_ops
                if let (Some(u), Some(ops)) = (&info.key_use, &info.key_ops) {
                    let sig_ops = ["sign", "verify"];
                    let enc_ops = ["encrypt", "decrypt", "wrapKey", "unwrapKey"];
                    let uses_sig = sig_ops.iter().any(|&op| ops.iter().any(|o| o == op));
                    let uses_enc = enc_ops.iter().any(|&op| ops.iter().any(|o| o == op));
                    if u == "sig" && uses_enc {
                        issues.push("'use': sig but key_ops includes encryption operations".into());
                    }
                    if u == "enc" && uses_sig {
                        issues.push("'use': enc but key_ops includes signing operations".into());
                    }
                }

                // Warn: private key without intended use hint
                if info.has_private && info.key_use.is_none() && info.key_ops.is_none() {
                    issues.push(
                        "Private key without 'use' or 'key_ops' — consider specifying intended use"
                            .into(),
                    );
                }

                if issues.is_empty() {
                    out.push_str("  ✅ VALID\n");
                } else {
                    all_valid = false;
                    out.push_str("  ⚠️  WARNINGS\n");
                    for issue in &issues {
                        out.push_str(&format!("    • {}\n", issue));
                    }
                }
            }
            Err(e) => {
                all_valid = false;
                out.push_str(&format!("  ❌ INVALID — {}\n", e));
            }
        }

        out.push('\n');
    }

    if is_jwks && keys.len() > 1 {
        if all_valid {
            out.push_str("Overall: ✅ All keys valid\n");
        } else {
            out.push_str("Overall: ⚠️  Some keys have issues\n");
        }
    }

    Ok(out)
}

fn rfc7638_thumbprint(key: &Value) -> Result<String, String> {
    let kty = key
        .get("kty")
        .and_then(|v| v.as_str())
        .ok_or("Missing kty")?;

    // RFC 7638 §3.2: canonical JSON — only required members, alphabetical, no whitespace
    let canonical = match kty {
        "RSA" => {
            let e = key
                .get("e")
                .and_then(|v| v.as_str())
                .ok_or("Missing RSA 'e'")?;
            let n = key
                .get("n")
                .and_then(|v| v.as_str())
                .ok_or("Missing RSA 'n'")?;
            format!(r#"{{"e":"{}","kty":"RSA","n":"{}"}}"#, e, n)
        }
        "EC" => {
            let crv = key
                .get("crv")
                .and_then(|v| v.as_str())
                .ok_or("Missing EC 'crv'")?;
            let x = key
                .get("x")
                .and_then(|v| v.as_str())
                .ok_or("Missing EC 'x'")?;
            let y = key
                .get("y")
                .and_then(|v| v.as_str())
                .ok_or("Missing EC 'y'")?;
            format!(r#"{{"crv":"{}","kty":"EC","x":"{}","y":"{}"}}"#, crv, x, y)
        }
        "OKP" => {
            let crv = key
                .get("crv")
                .and_then(|v| v.as_str())
                .ok_or("Missing OKP 'crv'")?;
            let x = key
                .get("x")
                .and_then(|v| v.as_str())
                .ok_or("Missing OKP 'x'")?;
            format!(r#"{{"crv":"{}","kty":"OKP","x":"{}"}}"#, crv, x)
        }
        "oct" => {
            let k = key
                .get("k")
                .and_then(|v| v.as_str())
                .ok_or("Missing oct 'k'")?;
            format!(r#"{{"k":"{}","kty":"oct"}}"#, k)
        }
        other => return Err(format!("Unknown kty for thumbprint: {}", other)),
    };

    let digest = Sha256::digest(canonical.as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(digest.as_slice()))
}

fn action_thumbprint(keys: &[&Value]) -> Result<String, String> {
    let mut out = String::from("RFC 7638 JWK Thumbprint (SHA-256)\n");
    out.push_str(&format!("{}\n\n", "═".repeat(45)));

    for (i, &key) in keys.iter().enumerate() {
        if keys.len() > 1 {
            out.push_str(&format!("Key {}:\n", i + 1));
        }

        match rfc7638_thumbprint(key) {
            Ok(tp) => {
                out.push_str(&format!("  {}\n", tp));
                // Also decode as hex for readability
                if let Ok(bytes) = URL_SAFE_NO_PAD.decode(&tp) {
                    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
                    out.push_str(&format!("  hex: {}\n", hex));
                }
            }
            Err(e) => {
                out.push_str(&format!("  Error: {}\n", e));
            }
        }

        out.push('\n');
    }

    out.push_str("Algorithm: SHA-256(UTF8(sorted required JWK members JSON))\n");
    out.push_str("Encoding : Base64url (no padding)\n");
    out.push_str("RFC      : https://www.rfc-editor.org/rfc/rfc7638\n");

    Ok(out)
}

fn action_list(keys: &[&Value]) -> Result<String, String> {
    if keys.is_empty() {
        return Ok("No keys found.".to_string());
    }

    let mut out = format!("{} key(s)\n\n", keys.len());
    out.push_str(&format!(
        "{:<4} {:<22} {:<12} {:<8} {:<8} {}\n",
        "#", "Type", "Algorithm", "Use", "Key ID", "Thumbprint"
    ));
    out.push_str(&"-".repeat(90));
    out.push('\n');

    for (i, &key) in keys.iter().enumerate() {
        match parse_jwk(key) {
            Ok(info) => {
                let label = key_type_label(&info);
                let alg = info.alg.as_deref().unwrap_or("—");
                let u = info.key_use.as_deref().unwrap_or("—");
                let kid = info.kid.as_deref().unwrap_or("—");
                let tp = rfc7638_thumbprint(key)
                    .map(|t| t.chars().take(16).collect::<String>() + "…")
                    .unwrap_or_else(|_| "—".into());
                out.push_str(&format!(
                    "{:<4} {:<22} {:<12} {:<8} {:<8} {}\n",
                    i + 1,
                    &label[..label.len().min(22)],
                    &alg[..alg.len().min(12)],
                    u,
                    &kid[..kid.len().min(8)],
                    tp
                ));
            }
            Err(e) => {
                out.push_str(&format!("{:<4} Parse error: {}\n", i + 1, e));
            }
        }
    }

    Ok(out)
}
