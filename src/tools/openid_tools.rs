use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "name": "openid_tools",
        "description": "Inspects OpenID Connect (OIDC) discovery documents, ID tokens, and userinfo responses without external utilities. \
    Actions: discover (default — parse OIDC discovery JSON: endpoints, response types, grant types, scopes, signing algorithms), \
    id_token (decode and explain an OIDC ID token JWT with claim-by-claim breakdown), \
    userinfo (parse userinfo JSON response, explain standard claims), \
    scopes (explain OIDC scopes: openid/profile/email/address/phone/offline_access with claim lists), \
    client (generate OAuth client config from discovery doc with Python authlib example). \
    Input: json/document for inline discovery JSON or file for a path; token/id_token for id_token action; \
    claims/userinfo for userinfo action. \
    Example: openid_tools(action: 'discover', file: 'openid-configuration.json') or \
    openid_tools(action: 'id_token', token: 'eyJ...') or openid_tools(action: 'scopes').",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "discover|id_token|userinfo|scopes|client" },
                "json": { "type": "string", "description": "Inline OIDC discovery document JSON or userinfo JSON" },
                "document": { "type": "string", "description": "Alias for json" },
                "file": { "type": "string", "description": "Path to discovery document or userinfo JSON file" },
                "token": { "type": "string", "description": "ID token JWT string (eyJ...)" },
                "id_token": { "type": "string", "description": "Alias for token" },
                "claims": { "type": "string", "description": "Userinfo JSON claims string" },
                "userinfo": { "type": "string", "description": "Alias for claims" }
            },
            "required": []
        }
    })
}

fn get_json_input(args: &Value) -> Option<Value> {
    let s = args
        .get("json")
        .or_else(|| args.get("document"))
        .or_else(|| args.get("claims"))
        .or_else(|| args.get("userinfo"))
        .and_then(|v| v.as_str());
    if let Some(s) = s {
        return serde_json::from_str(s).ok();
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        let content = std::fs::read_to_string(path).ok()?;
        return serde_json::from_str(&content).ok();
    }
    None
}

fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    let input = input.replace('-', "+").replace('_', "/");
    let padded = match input.len() % 4 {
        2 => format!("{}==", input),
        3 => format!("{}=", input),
        _ => input.to_string(),
    };
    let mut buf = Vec::new();
    let b64 = padded.as_bytes();
    let mut i = 0;
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut decode_map = [255u8; 256];
    for (idx, &c) in table.iter().enumerate() {
        decode_map[c as usize] = idx as u8;
    }
    while i + 3 < b64.len() {
        let a = decode_map[b64[i] as usize];
        let b = decode_map[b64[i + 1] as usize];
        let c = decode_map[b64[i + 2] as usize];
        let d = decode_map[b64[i + 3] as usize];
        if a == 255 || b == 255 {
            break;
        }
        buf.push((a << 2) | (b >> 4));
        if c != 255 {
            buf.push((b << 4) | (c >> 2));
        }
        if d != 255 {
            buf.push((c << 6) | d);
        }
        i += 4;
    }
    let _ = buf.iter().all(|_| true); // suppress unused warning
    Some(buf)
}

fn format_unix_ts(ts: i64) -> String {
    if ts <= 0 {
        return "N/A".to_string();
    }
    let secs = ts as u64;
    let days = secs / 86400;
    let rem = secs % 86400;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;
    let epoch_days = days as i64;
    let (y, mo, d) = epoch_days_to_ymd(epoch_days);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", y, mo, d, h, m, s)
}

fn epoch_days_to_ymd(mut days: i64) -> (i64, u32, u32) {
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

fn expiry_status(exp: i64, now_hint: i64) -> &'static str {
    if exp < now_hint {
        "EXPIRED"
    } else {
        "VALID"
    }
}

fn time_now_approx() -> i64 {
    // Use file mtime as a rough now — avoids SystemTime import issues with no-std
    std::fs::metadata(std::env::current_exe().unwrap_or_default())
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64
        })
        .unwrap_or(0)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("discover");

    Ok(match action {
        "id_token" => {
            let token = args
                .get("token")
                .or_else(|| args.get("id_token"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            decode_id_token(token)
        }
        "userinfo" => {
            let claims = get_json_input(args).or_else(|| {
                args.get("claims")
                    .or_else(|| args.get("userinfo"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| serde_json::from_str(s).ok())
            });
            match claims {
                Some(c) => explain_userinfo(&c),
                None => "Error: provide userinfo JSON via json/claims/file.".to_string(),
            }
        }
        "scopes" => explain_scopes(),
        "client" => {
            let doc = get_json_input(args);
            match doc {
                Some(d) => generate_client_config(&d),
                None => {
                    "Provide a discovery document via json= or file= to generate client config.\n\
                     Example: openid_tools(action: 'client', file: '.well-known/openid-configuration.json')\n\n\
                     OIDC discovery URL pattern: https://<issuer>/.well-known/openid-configuration".to_string()
                }
            }
        }
        _ => {
            let doc = get_json_input(args);
            match doc {
                Some(d) => discover_info(&d),
                None => "Error: provide OIDC discovery document via json/document/file.\n\
                         Typical URL: https://accounts.google.com/.well-known/openid-configuration\n\
                         Fetch with: curl https://<issuer>/.well-known/openid-configuration > discovery.json".to_string(),
            }
        }
    })
}

fn discover_info(doc: &Value) -> String {
    let get_str = |key: &str| doc.get(key).and_then(|v| v.as_str()).unwrap_or("(not set)");
    let get_arr_str = |key: &str| -> Vec<String> {
        doc.get(key)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    };

    let issuer = get_str("issuer");
    let mut out = "OpenID Connect Discovery Document\n".to_string();
    out.push_str(&format!("Issuer: {}\n\n", issuer));

    // Endpoints
    out.push_str("── Endpoints ───────────────────────────\n");
    let endpoints = [
        ("authorization_endpoint", "Authorization"),
        ("token_endpoint", "Token"),
        ("userinfo_endpoint", "UserInfo"),
        ("jwks_uri", "JWKS"),
        ("end_session_endpoint", "End Session"),
        ("revocation_endpoint", "Revocation"),
        ("introspection_endpoint", "Introspection"),
        ("device_authorization_endpoint", "Device Auth"),
        ("registration_endpoint", "Registration"),
    ];
    for (key, label) in &endpoints {
        if let Some(val) = doc.get(*key).and_then(|v| v.as_str()) {
            out.push_str(&format!("  {:<22} {}\n", label, val));
        }
    }

    // Capabilities
    out.push('\n');
    out.push_str("── Capabilities ────────────────────────\n");
    let response_types = get_arr_str("response_types_supported");
    if !response_types.is_empty() {
        out.push_str(&format!(
            "  Response types:   {}\n",
            response_types.join(", ")
        ));
    }
    let grant_types = get_arr_str("grant_types_supported");
    if !grant_types.is_empty() {
        out.push_str(&format!("  Grant types:      {}\n", grant_types.join(", ")));
    }
    let scopes = get_arr_str("scopes_supported");
    if !scopes.is_empty() {
        out.push_str(&format!("  Scopes:           {}\n", scopes.join(", ")));
    }
    let claims = get_arr_str("claims_supported");
    if !claims.is_empty() {
        out.push_str(&format!("  Claims:           {}\n", claims.join(", ")));
    }

    // Signing algorithms
    out.push('\n');
    out.push_str("── Algorithms ──────────────────────────\n");
    let id_token_algs = get_arr_str("id_token_signing_alg_values_supported");
    if !id_token_algs.is_empty() {
        out.push_str(&format!(
            "  ID token signing: {}\n",
            id_token_algs.join(", ")
        ));
    }
    let userinfo_algs = get_arr_str("userinfo_signing_alg_values_supported");
    if !userinfo_algs.is_empty() {
        out.push_str(&format!(
            "  UserInfo signing: {}\n",
            userinfo_algs.join(", ")
        ));
    }
    let token_endpoint_auth = get_arr_str("token_endpoint_auth_methods_supported");
    if !token_endpoint_auth.is_empty() {
        out.push_str(&format!(
            "  Token endpoint auth: {}\n",
            token_endpoint_auth.join(", ")
        ));
    }

    // PKCE
    let pkce_methods = get_arr_str("code_challenge_methods_supported");
    if !pkce_methods.is_empty() {
        out.push('\n');
        out.push_str(&format!(
            "PKCE supported: {}  (use S256 for new implementations)\n",
            pkce_methods.join(", ")
        ));
    } else {
        out.push('\n');
        out.push_str("PKCE: not advertised in discovery document\n");
    }

    // Subject types
    let subject_types = get_arr_str("subject_types_supported");
    if !subject_types.is_empty() {
        out.push_str(&format!("Subject types: {}\n", subject_types.join(", ")));
    }

    // Claims param support
    if let Some(true) = doc
        .get("claims_parameter_supported")
        .and_then(|v| v.as_bool())
    {
        out.push_str("Claims parameter: supported\n");
    }
    if let Some(true) = doc
        .get("request_parameter_supported")
        .and_then(|v| v.as_bool())
    {
        out.push_str("Request parameter (JAR): supported\n");
    }

    out
}

fn decode_id_token(token: &str) -> String {
    let token = token.trim();
    if token.is_empty() {
        return "Error: provide token= with the ID token JWT string.".to_string();
    }
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() < 2 {
        return "Error: not a valid JWT (expected header.payload.signature)".to_string();
    }

    let header_json = match base64url_decode(parts[0])
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
    {
        Some(h) => h,
        None => return "Error: could not decode JWT header".to_string(),
    };

    let payload_json = match base64url_decode(parts[1])
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
    {
        Some(p) => p,
        None => return "Error: could not decode JWT payload".to_string(),
    };

    let alg = header_json
        .get("alg")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let kid = header_json.get("kid").and_then(|v| v.as_str());

    let now = time_now_approx();

    let mut out = String::from("OIDC ID Token\n\n");
    out.push_str(&format!(
        "Algorithm: {}{}    [Signature NOT verified — decode only]\n",
        alg,
        if let Some(k) = kid {
            format!("  KID: {}", k)
        } else {
            String::new()
        }
    ));
    out.push('\n');

    // Core OIDC claims
    out.push_str("── Core Claims ─────────────────────────\n");
    let oidc_claims: &[(&str, &str)] = &[
        ("iss", "Issuer            "),
        ("sub", "Subject           "),
        ("aud", "Audience          "),
        ("azp", "Authorized Party  "),
        ("nonce", "Nonce             "),
        ("acr", "Auth Context Ref  "),
        ("amr", "Auth Methods Ref  "),
    ];
    for (key, label) in oidc_claims {
        if let Some(v) = payload_json.get(*key) {
            let display = match v {
                Value::String(s) => s.clone(),
                Value::Array(a) => a
                    .iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                other => other.to_string(),
            };
            out.push_str(&format!("  {} {}\n", label, display));
        }
    }

    // Time claims
    out.push('\n');
    out.push_str("── Time Claims ─────────────────────────\n");
    for key in &["iat", "exp", "auth_time", "nbf"] {
        if let Some(ts) = payload_json.get(*key).and_then(|v| v.as_i64()) {
            let formatted = format_unix_ts(ts);
            let status = match *key {
                "exp" => {
                    let s = expiry_status(ts, now);
                    format!("  [{}]", s)
                }
                "nbf" if now > 0 && ts > now => "  [NOT YET VALID]".to_string(),
                _ => String::new(),
            };
            let label = match *key {
                "iat" => "Issued At         ",
                "exp" => "Expires           ",
                "auth_time" => "Auth Time         ",
                "nbf" => "Not Before        ",
                _ => key,
            };
            out.push_str(&format!("  {} {}{}\n", label, formatted, status));
        }
    }

    // at_hash / c_hash
    for key in &["at_hash", "c_hash", "s_hash"] {
        if let Some(v) = payload_json.get(*key).and_then(|v| v.as_str()) {
            let label = match *key {
                "at_hash" => "Access Token Hash ",
                "c_hash" => "Code Hash         ",
                "s_hash" => "State Hash        ",
                _ => key,
            };
            out.push_str(&format!("  {} {}\n", label, v));
        }
    }

    // Standard profile claims
    let profile_claims = [
        ("name", "Name"),
        ("given_name", "Given Name"),
        ("family_name", "Family Name"),
        ("middle_name", "Middle Name"),
        ("nickname", "Nickname"),
        ("preferred_username", "Preferred Username"),
        ("profile", "Profile URL"),
        ("picture", "Picture URL"),
        ("website", "Website"),
        ("email", "Email"),
        ("email_verified", "Email Verified"),
        ("gender", "Gender"),
        ("birthdate", "Birthdate"),
        ("phone_number", "Phone"),
        ("phone_number_verified", "Phone Verified"),
        ("locale", "Locale"),
        ("zoneinfo", "Zoneinfo"),
        ("updated_at", "Updated At"),
    ];

    let has_profile = profile_claims
        .iter()
        .any(|(k, _)| payload_json.get(*k).is_some());
    if has_profile {
        out.push('\n');
        out.push_str("── Profile Claims ──────────────────────\n");
        for (key, label) in &profile_claims {
            if let Some(v) = payload_json.get(*key) {
                let display = match v {
                    Value::Bool(b) => {
                        if *b {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        }
                    }
                    Value::Number(n) => {
                        if key == &"updated_at" {
                            format_unix_ts(n.as_i64().unwrap_or(0))
                        } else {
                            n.to_string()
                        }
                    }
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                out.push_str(&format!("  {:<22} {}\n", label, display));
            }
        }
    }

    // Extra custom claims
    let known: std::collections::HashSet<&str> = [
        "iss",
        "sub",
        "aud",
        "azp",
        "nonce",
        "acr",
        "amr",
        "iat",
        "exp",
        "auth_time",
        "nbf",
        "at_hash",
        "c_hash",
        "s_hash",
        "name",
        "given_name",
        "family_name",
        "middle_name",
        "nickname",
        "preferred_username",
        "profile",
        "picture",
        "website",
        "email",
        "email_verified",
        "gender",
        "birthdate",
        "phone_number",
        "phone_number_verified",
        "locale",
        "zoneinfo",
        "updated_at",
    ]
    .iter()
    .cloned()
    .collect();

    let extras: Vec<(&String, &Value)> = payload_json
        .as_object()
        .map(|o| {
            o.iter()
                .filter(|(k, _)| !known.contains(k.as_str()))
                .collect()
        })
        .unwrap_or_default();

    if !extras.is_empty() {
        out.push('\n');
        out.push_str("── Additional Claims ───────────────────\n");
        for (k, v) in extras {
            out.push_str(&format!("  {:<22} {}\n", k, v));
        }
    }

    out
}

fn explain_userinfo(claims: &Value) -> String {
    let claim_explanations: &[(&str, &str, &str)] = &[
        ("sub", "Subject (unique user ID)", "openid"),
        ("name", "Full display name", "profile"),
        ("given_name", "First/given name", "profile"),
        ("family_name", "Last/family name", "profile"),
        ("middle_name", "Middle name", "profile"),
        ("nickname", "Casual name or alias", "profile"),
        ("preferred_username", "Preferred login username", "profile"),
        ("profile", "URL of user's profile page", "profile"),
        ("picture", "URL of user's profile photo", "profile"),
        ("website", "User's website URL", "profile"),
        ("gender", "User's gender", "profile"),
        ("birthdate", "Date of birth (YYYY-MM-DD)", "profile"),
        (
            "zoneinfo",
            "IANA timezone (e.g. America/Chicago)",
            "profile",
        ),
        ("locale", "BCP47 locale tag (e.g. en-US)", "profile"),
        (
            "updated_at",
            "Last profile update (Unix timestamp)",
            "profile",
        ),
        ("email", "Email address", "email"),
        (
            "email_verified",
            "Whether email is verified by IdP",
            "email",
        ),
        ("phone_number", "Phone number (E.164 format)", "phone"),
        (
            "phone_number_verified",
            "Whether phone is verified by IdP",
            "phone",
        ),
        ("address", "Structured postal address object", "address"),
    ];

    let mut out = String::from("OIDC UserInfo Claims\n\n");
    for (key, desc, scope) in claim_explanations {
        if let Some(v) = claims.get(*key) {
            let val = match v {
                Value::Bool(b) => if *b { "true ✓" } else { "false ✗" }.to_string(),
                Value::String(s) => s.clone(),
                Value::Number(n) => {
                    if *key == "updated_at" {
                        format_unix_ts(n.as_i64().unwrap_or(0))
                    } else {
                        n.to_string()
                    }
                }
                Value::Object(_) => format!("{}", v),
                other => other.to_string(),
            };
            out.push_str(&format!(
                "  {:<28} {}  [{}]\n  └ {}\n\n",
                key, val, scope, desc
            ));
        }
    }

    // Extra claims
    let known: std::collections::HashSet<&str> =
        claim_explanations.iter().map(|(k, _, _)| *k).collect();
    let extras: Vec<(&String, &Value)> = claims
        .as_object()
        .map(|o| {
            o.iter()
                .filter(|(k, _)| !known.contains(k.as_str()))
                .collect()
        })
        .unwrap_or_default();

    if !extras.is_empty() {
        out.push_str("── Non-standard Claims ─────────────────\n");
        for (k, v) in extras {
            out.push_str(&format!("  {:<28} {}\n", k, v));
        }
        out.push('\n');
    }

    out
}

fn explain_scopes() -> String {
    let mut out = String::from("OIDC Scope Reference\n\n");

    let scopes: &[(&str, &str, &[&str])] = &[
        ("openid", "Required for OIDC. Returns sub (subject identifier) in ID token. Without this, the flow is plain OAuth 2.0.",
         &["sub"]),
        ("profile", "Basic identity claims. Returns the user's display name, username, gender, birthday, and locale.",
         &["name", "given_name", "family_name", "middle_name", "nickname", "preferred_username", "profile", "picture", "website", "gender", "birthdate", "zoneinfo", "locale", "updated_at"]),
        ("email", "Returns email address and whether it is verified by the identity provider.",
         &["email", "email_verified"]),
        ("address", "Returns the user's preferred postal address as a structured object with street_address, locality, region, postal_code, country fields.",
         &["address"]),
        ("phone", "Returns phone number in E.164 format and verification status.",
         &["phone_number", "phone_number_verified"]),
        ("offline_access", "Requests a refresh token. Required when you need to access APIs after the user closes the browser. Subject to IdP policy.",
         &[]),
    ];

    for (scope, desc, claims) in scopes {
        out.push_str(&format!("▸ {}\n", scope));
        out.push_str(&format!("  {}\n", desc));
        if !claims.is_empty() {
            out.push_str(&format!("  Claims: {}\n", claims.join(", ")));
        }
        out.push('\n');
    }

    out.push_str("Notes:\n");
    out.push_str("  • Always include openid to trigger OIDC mode\n");
    out.push_str("  • Some IdPs require additional custom scopes (e.g. api, roles, groups)\n");
    out.push_str(
        "  • Consult the discovery document scopes_supported field for IdP-specific additions\n",
    );
    out
}

fn generate_client_config(doc: &Value) -> String {
    let get_str = |key: &str| doc.get(key).and_then(|v| v.as_str()).unwrap_or("(not set)");

    let issuer = get_str("issuer");
    let auth_ep = get_str("authorization_endpoint");
    let token_ep = get_str("token_endpoint");
    let userinfo_ep = get_str("userinfo_endpoint");
    let jwks_uri = get_str("jwks_uri");
    let end_session_ep = doc
        .get("end_session_endpoint")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let pkce_methods: Vec<String> = doc
        .get("code_challenge_methods_supported")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    let pkce_supported = pkce_methods.contains(&"S256".to_string())
        || pkce_methods.contains(&"s256".to_string())
        || !pkce_methods.is_empty();

    let mut out = String::from("OIDC Client Configuration\n\n");

    // Environment variables
    out.push_str("── Environment Variables ────────────────\n");
    out.push_str(&format!("  OIDC_ISSUER={}                  \n", issuer));
    out.push_str("  OIDC_CLIENT_ID=<your-client-id>\n");
    out.push_str("  OIDC_CLIENT_SECRET=<your-client-secret>\n");
    out.push_str("  OIDC_REDIRECT_URI=https://yourapp.example.com/auth/callback\n\n");

    // Python authlib
    out.push_str("── Python (authlib + requests) ─────────\n");
    out.push_str("from authlib.integrations.requests_client import OAuth2Session\nimport os\n\n");
    out.push_str("client = OAuth2Session(\n");
    out.push_str("    client_id=os.environ['OIDC_CLIENT_ID'],\n");
    out.push_str("    client_secret=os.environ['OIDC_CLIENT_SECRET'],\n");
    out.push_str("    redirect_uri=os.environ['OIDC_REDIRECT_URI'],\n");
    out.push_str("    scope='openid profile email',\n");
    out.push_str(")\n\n");
    out.push_str(&format!("# Authorization endpoint: {}\n", auth_ep));
    if pkce_supported {
        out.push_str("uri, state, code_verifier = client.create_authorization_url(\n");
        out.push_str(&format!("    '{}',\n", auth_ep));
        out.push_str("    code_challenge_method='S256',\n");
        out.push_str(")\n\n");
        out.push_str("# Exchange code:\n");
        out.push_str(&format!(
            "token = client.fetch_token('{}', code=request.args['code'],\n",
            token_ep
        ));
        out.push_str("                            code_verifier=code_verifier)\n\n");
    } else {
        out.push_str("uri, state = client.create_authorization_url(\n");
        out.push_str(&format!("    '{}',\n", auth_ep));
        out.push_str(")\n\n");
        out.push_str("# Exchange code:\n");
        out.push_str(&format!(
            "token = client.fetch_token('{}', code=request.args['code'])\n\n",
            token_ep
        ));
    }
    out.push_str(&format!("# UserInfo: GET {}\n", userinfo_ep));
    if !end_session_ep.is_empty() {
        out.push_str(&format!("# Logout: GET {}\n\n", end_session_ep));
    }

    // Discovery summary
    out.push_str("── Key Endpoints ───────────────────────\n");
    out.push_str(&format!("  Issuer:        {}\n", issuer));
    out.push_str(&format!("  Authorization: {}\n", auth_ep));
    out.push_str(&format!("  Token:         {}\n", token_ep));
    out.push_str(&format!("  UserInfo:      {}\n", userinfo_ep));
    out.push_str(&format!("  JWKS:          {}\n", jwks_uri));
    if !end_session_ep.is_empty() {
        out.push_str(&format!("  End Session:   {}\n", end_session_ep));
    }
    out.push('\n');
    if pkce_supported {
        out.push_str(&format!(
            "PKCE: supported ({}) — use S256\n",
            pkce_methods.join(", ")
        ));
    } else {
        out.push_str("PKCE: not advertised — use client_secret for public clients\n");
    }

    out
}
