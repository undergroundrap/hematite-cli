use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["pkce", "grant", "url", "token", "explain"],
                "description": "Action: pkce (generate PKCE pair), grant (explain grant type), url (build auth URL), token (decode access token), explain (explain OAuth 2.0 concepts)"
            },
            "grant_type": {
                "type": "string",
                "description": "OAuth grant type to explain or use (authorization_code, client_credentials, device_code, refresh_token, implicit, password)"
            },
            "client_id":       { "type": "string", "description": "OAuth client ID for URL builder" },
            "redirect_uri":    { "type": "string", "description": "Redirect URI for URL builder" },
            "scope":           { "type": "string", "description": "Space-separated scopes" },
            "authorization_endpoint": { "type": "string", "description": "Authorization server URL" },
            "state":           { "type": "string", "description": "State parameter (random if omitted)" },
            "code_challenge_method": {
                "type": "string",
                "enum": ["S256", "plain"],
                "description": "PKCE method (default S256)"
            },
            "code_verifier":   { "type": "string", "description": "Existing PKCE code_verifier to derive challenge from" },
            "token":           { "type": "string", "description": "Access token or JWT to decode/inspect" },
            "topic":           { "type": "string", "description": "Topic to explain: scopes, tokens, pkce, grants, flows, security" }
        }
    })
}

// ── PKCE ──────────────────────────────────────────────────────────────────────

fn base64url_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as FmtWrite;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut i = 0;
    while i + 2 < bytes.len() {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8) | (bytes[i + 2] as u32);
        let _ = write!(
            out,
            "{}{}{}{}",
            CHARS[((n >> 18) & 63) as usize] as char,
            CHARS[((n >> 12) & 63) as usize] as char,
            CHARS[((n >> 6) & 63) as usize] as char,
            CHARS[(n & 63) as usize] as char,
        );
        i += 3;
    }
    if i + 1 == bytes.len() {
        let n = (bytes[i] as u32) << 16;
        let _ = write!(
            out,
            "{}{}",
            CHARS[((n >> 18) & 63) as usize] as char,
            CHARS[((n >> 12) & 63) as usize] as char,
        );
    } else if i + 2 == bytes.len() {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8);
        let _ = write!(
            out,
            "{}{}{}",
            CHARS[((n >> 18) & 63) as usize] as char,
            CHARS[((n >> 12) & 63) as usize] as char,
            CHARS[((n >> 6) & 63) as usize] as char,
        );
    }
    out
}

fn sha256_bytes(input: &[u8]) -> [u8; 32] {
    // SHA-256 using sha2 crate (already in Cargo.toml)
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(input);
    hasher.finalize().into()
}

fn random_bytes_32() -> Vec<u8> {
    // Use rand crate (already in Cargo.toml)
    use rand::RngCore;
    let mut bytes = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

fn pkce_pair(verifier_override: Option<&str>, method: &str) -> Result<(String, String), String> {
    let verifier = if let Some(v) = verifier_override {
        v.to_string()
    } else {
        base64url_encode(&random_bytes_32())
    };

    let challenge = match method {
        "plain" => verifier.clone(),
        _ /* S256 */ => {
            let hash = sha256_bytes(verifier.as_bytes());
            base64url_encode(&hash)
        }
    };

    Ok((verifier, challenge))
}

fn action_pkce(args: &Value) -> Result<String, String> {
    let method = args
        .get("code_challenge_method")
        .and_then(|v| v.as_str())
        .unwrap_or("S256");
    let verifier_in = args.get("code_verifier").and_then(|v| v.as_str());

    let (verifier, challenge) = pkce_pair(verifier_in, method)?;

    let mut out = String::new();
    out.push_str("## PKCE Code Pair (RFC 7636)\n\n");
    out.push_str(&format!("  Method:          {}\n\n", method));
    out.push_str(&format!("  code_verifier:   {}\n", verifier));
    out.push_str(&format!("  code_challenge:  {}\n\n", challenge));
    out.push_str("## Usage\n\n");
    out.push_str("  1. Store code_verifier securely on the client (never send to auth server).\n");
    out.push_str(
        "  2. Include code_challenge and code_challenge_method=S256 in the /authorize request.\n",
    );
    out.push_str("  3. When exchanging the auth code for a token, send the code_verifier.\n");
    out.push_str("  4. The auth server computes SHA256(code_verifier) and verifies it matches code_challenge.\n\n");
    if method == "plain" {
        out.push_str(
            "  ⚠  Plain method transmits the verifier directly — use S256 for production.\n",
        );
    }
    out.push_str(&format!(
        "  Verifier length: {} chars (RFC 7636 requires 43–128)\n",
        verifier.len()
    ));
    Ok(out)
}

// ── Grant Types ───────────────────────────────────────────────────────────────

struct GrantInfo {
    id: &'static str,
    name: &'static str,
    use_case: &'static str,
    requires_user: bool,
    recommended: bool,
    description: &'static str,
    flow: &'static [&'static str],
    security: &'static str,
}

static GRANTS: &[GrantInfo] = &[
    GrantInfo {
        id: "authorization_code",
        name: "Authorization Code",
        use_case: "Web apps, mobile apps with PKCE",
        requires_user: true,
        recommended: true,
        description: "User is redirected to the auth server, authenticates, and the server returns a short-lived code. The client exchanges the code for tokens. With PKCE this is the gold standard for all app types.",
        flow: &[
            "1. Client redirects user to /authorize with response_type=code",
            "2. User authenticates and consents",
            "3. Auth server redirects back with ?code=XXXX&state=YYY",
            "4. Client POSTs to /token: grant_type=authorization_code&code=XXXX&code_verifier=ZZZ",
            "5. Auth server returns access_token, refresh_token, id_token",
        ],
        security: "Use PKCE for all public clients. Validate state to prevent CSRF. Keep client_secret server-side only.",
    },
    GrantInfo {
        id: "client_credentials",
        name: "Client Credentials",
        use_case: "Machine-to-machine / service accounts",
        requires_user: false,
        recommended: true,
        description: "No user is involved. The client authenticates with its own credentials to get an access token for accessing resources owned by the client itself.",
        flow: &[
            "1. Client POSTs to /token: grant_type=client_credentials",
            "2. Auth server validates client_id + client_secret",
            "3. Auth server returns access_token (no refresh_token)",
        ],
        security: "Store client_secret in a secrets manager, never in code or env files committed to VCS. Rotate regularly.",
    },
    GrantInfo {
        id: "device_code",
        name: "Device Authorization (Device Code)",
        use_case: "Devices with no browser or limited input (TV, CLI tools)",
        requires_user: true,
        recommended: true,
        description: "Device obtains a device_code and shows a short URL + user_code. User visits the URL on another device, authenticates, and enters the code. Device polls for tokens.",
        flow: &[
            "1. Device POSTs to /device_authorization: client_id + scope",
            "2. Auth server returns device_code, user_code, verification_uri, expires_in, interval",
            "3. Device shows verification_uri and user_code to user",
            "4. Device polls /token with grant_type=urn:ietf:params:oauth:grant-type:device_code",
            "5. On auth completion, poll returns access_token + refresh_token",
        ],
        security: "Honor polling interval. Handle authorization_pending and slow_down responses gracefully.",
    },
    GrantInfo {
        id: "refresh_token",
        name: "Refresh Token",
        use_case: "Renewing expired access tokens without re-authenticating the user",
        requires_user: false,
        recommended: true,
        description: "When an access_token expires, the client uses a stored refresh_token to obtain a new access_token without user interaction.",
        flow: &[
            "1. Client detects access_token expiry (check exp claim or 401 response)",
            "2. Client POSTs to /token: grant_type=refresh_token&refresh_token=XXXX",
            "3. Auth server returns new access_token (and sometimes a new refresh_token)",
        ],
        security: "Refresh tokens must be stored securely. Rotate them on each use if the server supports it. Use short-lived access tokens (15 min) with longer refresh tokens (hours–days).",
    },
    GrantInfo {
        id: "implicit",
        name: "Implicit (Deprecated)",
        use_case: "Legacy SPAs — do not use",
        requires_user: true,
        recommended: false,
        description: "Tokens are returned directly in the URL fragment. Deprecated by RFC 9700. Replaced by Authorization Code + PKCE for SPAs.",
        flow: &[
            "1. Client redirects to /authorize with response_type=token",
            "2. Auth server returns access_token in the URL fragment (#access_token=...)",
        ],
        security: "⚠  DEPRECATED. Tokens in URL fragments are exposed to browser history, referrer headers, and JS. Use Authorization Code + PKCE instead.",
    },
    GrantInfo {
        id: "password",
        name: "Resource Owner Password Credentials (ROPC, Deprecated)",
        use_case: "Legacy integrations only — do not use",
        requires_user: true,
        recommended: false,
        description: "User's credentials are passed directly to the client, which then exchanges them for tokens. Requires users to trust the client with their password. Deprecated by RFC 9700.",
        flow: &[
            "1. Client collects username + password from user",
            "2. Client POSTs to /token: grant_type=password&username=X&password=Y",
            "3. Auth server returns tokens",
        ],
        security: "⚠  DEPRECATED. Never build new integrations using ROPC. The client handles raw credentials, undermining the separation OAuth was designed to provide.",
    },
];

fn action_grant(args: &Value) -> Result<String, String> {
    let gt = args
        .get("grant_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if gt.is_empty() {
        // List all
        let mut out = format!("{:<30} {:<15} {}\n", "GRANT TYPE", "USER?", "STATUS");
        out.push_str(&format!("{}\n", "-".repeat(70)));
        for g in GRANTS {
            let user = if g.requires_user { "Yes" } else { "No" };
            let status = if g.recommended {
                "✓ Recommended"
            } else {
                "⚠ Deprecated"
            };
            out.push_str(&format!("{:<30} {:<15} {}\n", g.id, user, status));
        }
        out.push_str("\nUse grant_type='<id>' for detailed information.\n");
        return Ok(out);
    }

    let info = GRANTS
        .iter()
        .find(|g| {
            g.id.eq_ignore_ascii_case(gt) || g.name.to_lowercase().contains(&gt.to_lowercase())
        })
        .ok_or_else(|| {
            format!(
                "Unknown grant type '{}'. Known: {}",
                gt,
                GRANTS.iter().map(|g| g.id).collect::<Vec<_>>().join(", ")
            )
        })?;

    let mut out = format!("## OAuth 2.0 Grant: {}\n\n", info.name);
    out.push_str(&format!("  Grant type:    {}\n", info.id));
    out.push_str(&format!("  Use case:      {}\n", info.use_case));
    out.push_str(&format!(
        "  Requires user: {}\n",
        if info.requires_user { "Yes" } else { "No" }
    ));
    out.push_str(&format!(
        "  Status:        {}\n\n",
        if info.recommended {
            "✓ Recommended"
        } else {
            "⚠ Deprecated — do not use for new integrations"
        }
    ));
    out.push_str(&format!("## Description\n\n  {}\n\n", info.description));
    out.push_str("## Flow\n\n");
    for step in info.flow {
        out.push_str(&format!("  {}\n", step));
    }
    out.push_str(&format!("\n## Security Notes\n\n  {}\n", info.security));
    Ok(out)
}

// ── Auth URL Builder ───────────────────────────────────────────────────────────

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn action_url(args: &Value) -> Result<String, String> {
    let endpoint = args
        .get("authorization_endpoint")
        .and_then(|v| v.as_str())
        .ok_or(
            "Provide 'authorization_endpoint' (e.g. https://accounts.google.com/o/oauth2/auth)",
        )?;
    let client_id = args
        .get("client_id")
        .and_then(|v| v.as_str())
        .ok_or("Provide 'client_id'")?;
    let redirect_uri = args
        .get("redirect_uri")
        .and_then(|v| v.as_str())
        .ok_or("Provide 'redirect_uri'")?;
    let scope = args
        .get("scope")
        .and_then(|v| v.as_str())
        .unwrap_or("openid profile email");
    let state = args
        .get("state")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| base64url_encode(&random_bytes_32())[..16].to_string());

    let use_pkce = args.get("code_challenge_method").is_some()
        || args.get("code_verifier").is_some()
        || args
            .get("grant_type")
            .and_then(|v| v.as_str())
            .unwrap_or("authorization_code")
            == "authorization_code";

    let method = args
        .get("code_challenge_method")
        .and_then(|v| v.as_str())
        .unwrap_or("S256");
    let (verifier, challenge) = if use_pkce {
        let verifier_in = args.get("code_verifier").and_then(|v| v.as_str());
        let (v, c) = pkce_pair(verifier_in, method)?;
        (Some(v), Some(c))
    } else {
        (None, None)
    };

    let mut params: Vec<(String, String)> = vec![
        ("response_type".to_string(), "code".to_string()),
        ("client_id".to_string(), client_id.to_string()),
        ("redirect_uri".to_string(), redirect_uri.to_string()),
        ("scope".to_string(), scope.to_string()),
        ("state".to_string(), state.clone()),
    ];
    if let (Some(_), Some(ref c)) = (&verifier, &challenge) {
        params.push(("code_challenge".to_string(), c.clone()));
        params.push(("code_challenge_method".to_string(), method.to_string()));
    }

    let qs: Vec<String> = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, url_encode(v)))
        .collect();
    let full_url = format!("{}?{}", endpoint, qs.join("&"));

    let mut out = String::new();
    out.push_str("## OAuth 2.0 Authorization URL\n\n");
    out.push_str(&format!("{}\n\n", full_url));
    out.push_str("## Parameters\n\n");
    for (k, v) in &params {
        out.push_str(&format!("  {:<30} {}\n", k, v));
    }

    if let Some(ref v) = verifier {
        out.push_str("\n## PKCE Values (store securely)\n\n");
        out.push_str(&format!("  code_verifier:  {}\n", v));
        out.push_str(&format!(
            "  code_challenge: {}\n",
            challenge.as_deref().unwrap_or("")
        ));
        out.push_str(&format!("  method:         {}\n", method));
        out.push_str("\n  ⚠  Keep code_verifier secret — send it with the token exchange, never with the /authorize request.\n");
    }

    out.push_str(&format!(
        "\n## State\n\n  {}\n  Validate state on redirect to prevent CSRF.\n",
        state
    ));
    Ok(out)
}

// ── Token Inspect ─────────────────────────────────────────────────────────────

fn base64url_decode(s: &str) -> Option<Vec<u8>> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let padded: String = {
        let mut p = s.replace('-', "+").replace('_', "/");
        while p.len() % 4 != 0 {
            p.push('=');
        }
        p
    };
    let mut buf: Vec<u8> = Vec::new();
    let bytes = padded.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        let a = bytes[i];
        let b = bytes[i + 1];
        let c = bytes[i + 2];
        let d = bytes[i + 3];
        let decode_char = |ch: u8| -> Option<u8> {
            CHARS
                .iter()
                .position(|&x| x == ch)
                .map(|p| p as u8)
                .or_else(|| {
                    if ch == b'+' {
                        Some(62)
                    } else if ch == b'/' {
                        Some(63)
                    } else if ch == b'=' {
                        Some(0)
                    } else {
                        None
                    }
                })
        };
        let av = decode_char(a)?;
        let bv = decode_char(b)?;
        let cv = decode_char(c)?;
        let dv = decode_char(d)?;
        buf.push((av << 2) | (bv >> 4));
        if c != b'=' {
            buf.push((bv << 4) | (cv >> 2));
        }
        if d != b'=' {
            buf.push((cv << 2) | dv);
        }
        i += 4;
    }
    Some(buf)
}

fn action_token(args: &Value) -> Result<String, String> {
    let token = args
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or("Provide 'token' with an access token or JWT")?;

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        // Opaque token
        let mut out = format!("## Access Token Inspection\n\n");
        out.push_str("  Format:  Opaque (not a JWT)\n");
        out.push_str(&format!("  Length:  {} characters\n", token.len()));
        out.push_str(&format!("  Prefix:  {}\n", &token[..token.len().min(8)]));
        out.push_str("\n  This token cannot be decoded locally — only the authorization server can validate it.\n");
        return Ok(out);
    }

    let header_bytes = base64url_decode(parts[0]).ok_or("Cannot decode JWT header")?;
    let payload_bytes = base64url_decode(parts[1]).ok_or("Cannot decode JWT payload")?;

    let header_str =
        String::from_utf8(header_bytes).map_err(|_| "JWT header is not valid UTF-8")?;
    let payload_str =
        String::from_utf8(payload_bytes).map_err(|_| "JWT payload is not valid UTF-8")?;

    let header: Value =
        serde_json::from_str(&header_str).map_err(|e| format!("JWT header JSON error: {}", e))?;
    let payload: Value =
        serde_json::from_str(&payload_str).map_err(|e| format!("JWT payload JSON error: {}", e))?;

    let alg = header.get("alg").and_then(|v| v.as_str()).unwrap_or("?");
    let typ = header.get("typ").and_then(|v| v.as_str()).unwrap_or("JWT");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut out = format!("## JWT Inspection\n\n");
    out.push_str(&format!("  Type:      {}\n", typ));
    out.push_str(&format!("  Algorithm: {}\n\n", alg));

    out.push_str("## Key Claims\n\n");
    let print_claim = |out: &mut String, key: &str, label: &str| {
        if let Some(v) = payload.get(key) {
            if key == "exp" || key == "iat" || key == "nbf" {
                if let Some(ts) = v.as_u64() {
                    let dt = format_unix_ts(ts);
                    let status = if key == "exp" {
                        if ts < now {
                            format!(" ⚠  EXPIRED {} seconds ago", now - ts)
                        } else {
                            format!(" ✓ expires in {} seconds", ts - now)
                        }
                    } else {
                        String::new()
                    };
                    out.push_str(&format!("  {:<14} {}{}\n", label, dt, status));
                }
            } else {
                let s = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                let preview: String = s.chars().take(80).collect();
                let ellipsis = if s.len() > 80 { "..." } else { "" };
                out.push_str(&format!("  {:<14} {}{}\n", label, preview, ellipsis));
            }
        }
    };

    {
        let out = &mut out;
        print_claim(out, "sub", "sub (subject)");
        print_claim(out, "iss", "iss (issuer)");
        print_claim(out, "aud", "aud (audience)");
        print_claim(out, "iat", "iat (issued)");
        print_claim(out, "exp", "exp (expires)");
        print_claim(out, "nbf", "nbf (not before)");
        print_claim(out, "scope", "scope");
        print_claim(out, "scp", "scp");
        print_claim(out, "email", "email");
        print_claim(out, "name", "name");
        print_claim(out, "roles", "roles");
    }

    out.push_str("\n## All Claims\n\n");
    if let Value::Object(map) = &payload {
        for (k, v) in map {
            out.push_str(&format!("  {}: {}\n", k, v));
        }
    }

    out.push_str("\n## Signature\n\n");
    out.push_str("  (not verified — local inspection only)\n");
    out.push_str(&format!(
        "  Signature: {}...\n",
        &parts[2][..parts[2].len().min(16)]
    ));

    Ok(out)
}

fn format_unix_ts(ts: u64) -> String {
    // Simple UTC display without chrono
    let secs = ts;
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    // Gregorian calendar approximation
    let (y, mo, d) = epoch_to_ymd(days_since_epoch);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", y, mo, d, h, m, s)
}

fn epoch_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut d = days + 719468;
    let era = d / 146097;
    let doe = d % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    let day = doy - (153 * mp + 2) / 5 + 1;
    (year, month, day)
}

// ── Explain ───────────────────────────────────────────────────────────────────

fn action_explain(args: &Value) -> Result<String, String> {
    let topic = args
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("flows");

    let text = match topic {
        "pkce" => concat!(
            "## PKCE — Proof Key for Code Exchange (RFC 7636)\n\n",
            "PKCE prevents authorization code interception attacks in public clients (mobile apps, SPAs, CLIs).\n\n",
            "  1. Client generates a random 32-byte secret → code_verifier (base64url, 43 chars)\n",
            "  2. Client computes code_challenge = BASE64URL(SHA256(code_verifier))\n",
            "  3. code_challenge is sent with /authorize — safe to intercept, useless without verifier\n",
            "  4. code_verifier is sent with /token — server verifies SHA256(verifier) == challenge\n\n",
            "Use action='pkce' to generate a PKCE pair.\n",
        ),
        "tokens" => concat!(
            "## OAuth 2.0 Token Types\n\n",
            "  access_token   Short-lived (minutes–hours). Sent as Bearer token in Authorization header.\n",
            "                 May be opaque or a JWT. Do not store permanently.\n\n",
            "  refresh_token  Long-lived (hours–days). Used to get new access tokens without user login.\n",
            "                 Must be stored securely — treat like a password.\n\n",
            "  id_token       OpenID Connect (OIDC) JWT with user identity claims (sub, email, name).\n",
            "                 Signed by IdP. Verify signature before trusting. Do not use as API credential.\n\n",
            "  device_code    Short-lived code for device flow. Not an access token.\n\n",
            "Token Validation (access/id tokens as JWTs):\n",
            "  - Verify signature against IdP's JWKS endpoint\n",
            "  - Verify iss (issuer) matches expected value\n",
            "  - Verify aud (audience) contains your client_id\n",
            "  - Verify exp is in the future\n",
            "  - Verify nbf is in the past (if present)\n",
        ),
        "scopes" => concat!(
            "## OAuth 2.0 Scopes\n\n",
            "Scopes define what access the client requests. Users see them on the consent screen.\n\n",
            "Common OIDC scopes:\n",
            "  openid   Required for OIDC — tells the server to return an id_token\n",
            "  profile  name, given_name, family_name, picture, locale\n",
            "  email    email, email_verified\n",
            "  address  address claim\n",
            "  phone    phone_number, phone_number_verified\n\n",
            "Common API scopes (provider-specific):\n",
            "  Google: https://www.googleapis.com/auth/gmail.readonly\n",
            "  GitHub: repo, read:user, user:email\n",
            "  Stripe: read_write, read_only\n",
            "  Microsoft: https://graph.microsoft.com/User.Read\n\n",
            "Best practices:\n",
            "  - Request minimum necessary scopes (principle of least privilege)\n",
            "  - Request incrementally when additional access is needed\n",
            "  - Show users why each scope is needed\n",
        ),
        "security" => concat!(
            "## OAuth 2.0 Security Best Practices\n\n",
            "  ✓ Always use PKCE for public clients (SPAs, mobile, CLI)\n",
            "  ✓ Validate state parameter on redirect to prevent CSRF\n",
            "  ✓ Use short-lived access tokens (15 min) + refresh tokens\n",
            "  ✓ Store tokens in secure, httpOnly cookies or OS keychain — never localStorage\n",
            "  ✓ Use exact redirect_uri matching, not prefix/wildcard\n",
            "  ✓ Validate JWT signature, iss, aud, exp on the server\n",
            "  ✓ Keep client_secret server-side only — never in browser code or mobile binaries\n",
            "  ✓ Rotate refresh tokens on each use when server supports it\n\n",
            "  ✗ Never use implicit flow for new projects\n",
            "  ✗ Never use ROPC (password grant) for new projects\n",
            "  ✗ Never log or expose tokens in URLs, console, or error messages\n",
            "  ✗ Never skip JWT validation — expired or forged tokens must be rejected\n",
        ),
        _ /* flows */ => concat!(
            "## OAuth 2.0 Grant Flow Decision Guide\n\n",
            "  Has a user logging in?\n",
            "  ├─ Yes → Use Authorization Code + PKCE\n",
            "  │        (web app, SPA, mobile, desktop, CLI)\n",
            "  │\n",
            "  └─ No (machine-to-machine) → Use Client Credentials\n",
            "           (backend service, cron job, microservice)\n\n",
            "  Special cases:\n",
            "  Device with no browser → Device Authorization Grant\n",
            "  Renewing after expiry  → Refresh Token Grant\n\n",
            "  Deprecated (do not use):\n",
            "  Implicit, Resource Owner Password Credentials (ROPC)\n\n",
            "Use action='grant' with grant_type='<name>' for detailed flow diagrams.\n",
            "Use action='url' to build an authorization URL.\n",
            "Use action='pkce' to generate a PKCE code pair.\n",
        ),
    };

    Ok(text.to_string())
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            if args.get("token").is_some() {
                "token"
            } else if args.get("grant_type").is_some()
                && args.get("authorization_endpoint").is_none()
            {
                "grant"
            } else if args.get("authorization_endpoint").is_some() {
                "url"
            } else if args.get("code_verifier").is_some()
                || args.get("code_challenge_method").is_some()
            {
                "pkce"
            } else if args.get("topic").is_some() {
                "explain"
            } else {
                "pkce"
            }
        });
    match action {
        "pkce" => action_pkce(args),
        "grant" => action_grant(args),
        "url" => action_url(args),
        "token" => action_token(args),
        "explain" => action_explain(args),
        _ => action_pkce(args),
    }
}
