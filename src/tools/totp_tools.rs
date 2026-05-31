use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("generate");
    match action {
        "generate" | "" => action_generate(args),
        "verify" => action_verify(args),
        "hotp" => action_hotp(args),
        "info" => action_info(args),
        "qr" => action_qr(args),
        _ => Err(format!(
            "Unknown action '{}'. Available: generate, verify, hotp, info, qr",
            action
        )),
    }
}

// ── SHA-1 (pure Rust, TOTP RFC 6238 uses HMAC-SHA1 by default) ──────────────

fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let bit_len = (data.len() as u64) * 8;

    // Padding
    let mut padded = data.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 64-byte block
    for block in padded.chunks(64) {
        let mut w = [0u32; 80];
        for t in 0..16 {
            w[t] = u32::from_be_bytes([
                block[t * 4],
                block[t * 4 + 1],
                block[t * 4 + 2],
                block[t * 4 + 3],
            ]);
        }
        for t in 16..80 {
            w[t] = (w[t - 3] ^ w[t - 8] ^ w[t - 14] ^ w[t - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for t in 0..80 {
            let (f, k) = match t {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[t]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for i in 0..5 {
        out[i * 4..i * 4 + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20] {
    let block_size = 64;
    let key_block: Vec<u8> = if key.len() > block_size {
        let mut hashed = sha1(key).to_vec();
        hashed.resize(block_size, 0);
        hashed
    } else {
        let mut k = key.to_vec();
        k.resize(block_size, 0);
        k
    };

    let ipad: Vec<u8> = key_block.iter().map(|&b| b ^ 0x36).collect();
    let opad: Vec<u8> = key_block.iter().map(|&b| b ^ 0x5C).collect();

    let inner: Vec<u8> = ipad.iter().chain(msg.iter()).cloned().collect();
    let inner_hash = sha1(&inner);
    let outer: Vec<u8> = opad.iter().chain(inner_hash.iter()).cloned().collect();
    sha1(&outer)
}

// ── Base32 decoder (RFC 4648, case-insensitive, padding optional) ───────────

fn base32_decode(s: &str) -> Result<Vec<u8>, String> {
    let alpha = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let clean: String = s
        .chars()
        .filter(|c| *c != '=' && !c.is_whitespace())
        .map(|c| c.to_uppercase().next().unwrap_or(c))
        .collect();
    let mut bits: u64 = 0;
    let mut bit_count: u32 = 0;
    let mut out = Vec::new();
    for ch in clean.chars() {
        let val = alpha
            .iter()
            .position(|&b| b == ch as u8)
            .ok_or_else(|| format!("Invalid base32 character: '{ch}'"))? as u64;
        bits = (bits << 5) | val;
        bit_count += 5;
        if bit_count >= 8 {
            bit_count -= 8;
            out.push(((bits >> bit_count) & 0xFF) as u8);
        }
    }
    Ok(out)
}

// ── HOTP core (RFC 4226) ────────────────────────────────────────────────────

fn hotp(key_bytes: &[u8], counter: u64, digits: u32) -> u32 {
    let msg = counter.to_be_bytes();
    let mac = hmac_sha1(key_bytes, &msg);
    let offset = (mac[19] & 0x0F) as usize;
    let code = ((mac[offset] as u32 & 0x7F) << 24)
        | ((mac[offset + 1] as u32 & 0xFF) << 16)
        | ((mac[offset + 2] as u32 & 0xFF) << 8)
        | (mac[offset + 3] as u32 & 0xFF);
    code % 10u32.pow(digits)
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn action_generate(args: &Value) -> Result<String, String> {
    let secret = args.get("secret").and_then(|v| v.as_str()).ok_or(
        "Pass 'secret' with the base32-encoded TOTP secret (from the QR code or app setup).",
    )?;

    let period = args.get("period").and_then(|v| v.as_u64()).unwrap_or(30);
    let digits = args.get("digits").and_then(|v| v.as_u64()).unwrap_or(6) as u32;
    let algorithm = args
        .get("algorithm")
        .and_then(|v| v.as_str())
        .unwrap_or("SHA1");

    if !matches!(algorithm.to_uppercase().as_str(), "SHA1") {
        return Err("Only SHA1 is supported for TOTP (RFC 6238 default). For TOTP-SHA256/SHA512, use a dedicated library.".into());
    }

    let key_bytes = base32_decode(secret)?;
    let now = args
        .get("time")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(current_unix_time);
    let counter = now / period;
    let code = hotp(&key_bytes, counter, digits);

    let remaining = period - (now % period);
    let window_start = counter * period;
    let window_end = window_start + period;

    let mut out = String::new();
    out.push_str(&format!("TOTP Code\n{}\n\n", "─".repeat(40)));
    out.push_str(&format!(
        "  Code:        {:0>width$}\n",
        code,
        width = digits as usize
    ));
    out.push_str(&format!("  Valid for:   {} seconds\n", remaining));
    out.push_str(&format!(
        "  Window:      {window_start}–{window_end} (UTC)\n"
    ));
    out.push_str(&format!("  Digits:      {digits}\n"));
    out.push_str(&format!("  Period:      {period}s\n"));
    out.push_str(&format!("  Algorithm:   {algorithm}\n"));
    out.push_str(&format!("  Counter:     {counter}\n\n"));

    // Show previous and next codes for context
    let prev = hotp(&key_bytes, counter.saturating_sub(1), digits);
    let next = hotp(&key_bytes, counter + 1, digits);
    out.push_str(&format!(
        "  Previous:    {:0>width$}  (expired)\n",
        prev,
        width = digits as usize
    ));
    out.push_str(&format!(
        "  Current:     {:0>width$}  ← use this\n",
        code,
        width = digits as usize
    ));
    out.push_str(&format!(
        "  Next:        {:0>width$}  (in {remaining}s)\n",
        next,
        width = digits as usize
    ));

    Ok(out)
}

fn action_verify(args: &Value) -> Result<String, String> {
    let secret = args
        .get("secret")
        .and_then(|v| v.as_str())
        .ok_or("Pass 'secret' and 'code' to verify a TOTP code.")?;
    let code_str = args
        .get("code")
        .and_then(|v| v.as_str())
        .or_else(|| args.get("code").and_then(|v| v.as_u64()).map(|_| ""))
        .ok_or("Pass 'code' with the 6-digit code to verify.")?;
    let code: u32 = if let Some(n) = args.get("code").and_then(|v| v.as_u64()) {
        n as u32
    } else {
        code_str
            .trim()
            .parse::<u32>()
            .map_err(|_| "Invalid code — must be numeric.".to_string())?
    };

    let period = args.get("period").and_then(|v| v.as_u64()).unwrap_or(30);
    let digits = args.get("digits").and_then(|v| v.as_u64()).unwrap_or(6) as u32;
    let window = args.get("window").and_then(|v| v.as_u64()).unwrap_or(1); // allow ±1 window

    let key_bytes = base32_decode(secret)?;
    let now = args
        .get("time")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(current_unix_time);
    let counter = now / period;

    let mut valid = false;
    let mut matched_window = 0i64;
    for w in -(window as i64)..=(window as i64) {
        let c = if w < 0 {
            counter.saturating_sub((-w) as u64)
        } else {
            counter + w as u64
        };
        if hotp(&key_bytes, c, digits) == code {
            valid = true;
            matched_window = w;
            break;
        }
    }

    let verdict = if valid { "VALID ✓" } else { "INVALID ✗" };
    let mut out = format!("TOTP Verify\n{}\n\n", "─".repeat(40));
    out.push_str(&format!(
        "  Code:     {:0>width$}\n",
        code,
        width = digits as usize
    ));
    out.push_str(&format!("  Result:   {verdict}\n"));
    if valid {
        let window_label = match matched_window {
            0 => "current window".into(),
            1 => "next window (clock drift)".into(),
            -1 => "previous window (clock drift)".into(),
            n => format!("window offset {n}"),
        };
        out.push_str(&format!("  Window:   {window_label}\n"));
    }
    out.push_str(&format!("  Period:   {period}s\n"));
    Ok(out)
}

fn action_hotp(args: &Value) -> Result<String, String> {
    let secret = args
        .get("secret")
        .and_then(|v| v.as_str())
        .ok_or("Pass 'secret' and 'counter' for HOTP generation.")?;
    let counter = args.get("counter").and_then(|v| v.as_u64()).unwrap_or(0);
    let digits = args.get("digits").and_then(|v| v.as_u64()).unwrap_or(6) as u32;
    let count = args
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .min(20);

    let key_bytes = base32_decode(secret)?;
    let mut out = format!(
        "HOTP Codes (counter={counter}, digits={digits})\n{}\n\n",
        "─".repeat(50)
    );
    for i in 0..count {
        let code = hotp(&key_bytes, counter + i, digits);
        out.push_str(&format!(
            "  Counter {:>4}:  {:0>width$}\n",
            counter + i,
            code,
            width = digits as usize
        ));
    }
    Ok(out)
}

fn action_info(args: &Value) -> Result<String, String> {
    let uri = args
        .get("uri")
        .or_else(|| args.get("url"))
        .and_then(|v| v.as_str());

    if let Some(uri_str) = uri {
        // Parse otpauth:// URI
        // otpauth://totp/label?secret=BASE32&issuer=Name&algorithm=SHA1&digits=6&period=30
        if !uri_str.starts_with("otpauth://") {
            return Err("URI must start with 'otpauth://totp/' or 'otpauth://hotp/'".into());
        }
        let rest = &uri_str[10..]; // strip "otpauth://"
        let (otp_type, rest) = rest.split_once('/').unwrap_or(("totp", rest));
        let (label_encoded, params_str) = rest.split_once('?').unwrap_or((rest, ""));
        let label = url_decode(label_encoded);
        let mut out = format!("OTP URI Info\n{}\n\n", "─".repeat(50));
        out.push_str(&format!("  Type:     {}\n", otp_type.to_uppercase()));
        out.push_str(&format!("  Label:    {label}\n"));
        for pair in params_str.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                let key_label = match k {
                    "secret" => "Secret",
                    "issuer" => "Issuer",
                    "algorithm" => "Algorithm",
                    "digits" => "Digits",
                    "period" => "Period",
                    "counter" => "Counter",
                    _ => k,
                };
                let val = if k == "secret" {
                    format!("{}... ({} chars)", &v[..v.len().min(8)], v.len())
                } else {
                    url_decode(v)
                };
                out.push_str(&format!("  {key_label:<12}{val}\n"));
            }
        }
        out.push_str("\n  Defaults: SHA1, 6 digits, 30-second period (if not specified)\n");
        Ok(out)
    } else {
        // General TOTP/HOTP info
        Ok(
            "TOTP / HOTP Reference\n────────────────────────────────────────────────────\n\
\n\
  TOTP (RFC 6238) — Time-based One-Time Password\n\
    Algorithm:   HMAC-SHA1 (default), SHA256, SHA512\n\
    Counter:     floor(unix_time / period)  — default 30s\n\
    Window:      ±1 period tolerance for clock drift\n\
    Formula:     code = HOTP(key, T) where T = floor(time/period)\n\
\n\
  HOTP (RFC 4226) — HMAC-based One-Time Password\n\
    Counter:     monotonically increasing integer\n\
    Formula:     hmac = HMAC-SHA1(key, counter_BE_8bytes)\n\
                 offset = hmac[19] & 0xF\n\
                 code = ((hmac[offset..offset+4] & 0x7FFFFFFF) % 10^digits)\n\
\n\
  Secret encoding: Base32 (RFC 4648), case-insensitive, padding optional\n\
  Common digits:   6 (default), 7, 8\n\
  Common period:   30s (default), 60s\n\
\n\
  otpauth URI format:\n\
    otpauth://totp/Issuer:user@example.com?secret=BASE32&issuer=Issuer\n\
              &algorithm=SHA1&digits=6&period=30\n"
                .into(),
        )
    }
}

fn action_qr(args: &Value) -> Result<String, String> {
    // Generate the otpauth:// URI string (not an actual QR code — that requires image rendering)
    let secret = args
        .get("secret")
        .and_then(|v| v.as_str())
        .ok_or("Pass 'secret' to generate an otpauth:// URI.")?;
    let label = args
        .get("label")
        .or_else(|| args.get("account"))
        .and_then(|v| v.as_str())
        .unwrap_or("account@example.com");
    let issuer = args
        .get("issuer")
        .and_then(|v| v.as_str())
        .unwrap_or("Service");
    let digits = args.get("digits").and_then(|v| v.as_u64()).unwrap_or(6);
    let period = args.get("period").and_then(|v| v.as_u64()).unwrap_or(30);
    let algorithm = args
        .get("algorithm")
        .and_then(|v| v.as_str())
        .unwrap_or("SHA1");

    let encoded_label = url_encode(&format!("{issuer}:{label}"));
    let encoded_issuer = url_encode(issuer);
    let secret_clean: String = secret
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_uppercase().next().unwrap_or(c))
        .collect();

    let uri = format!(
        "otpauth://totp/{encoded_label}?secret={secret_clean}&issuer={encoded_issuer}&algorithm={algorithm}&digits={digits}&period={period}"
    );

    let mut out = format!("OTP Auth URI\n{}\n\n", "─".repeat(60));
    out.push_str("  Scan the URI below with your authenticator app (Google\n");
    out.push_str("  Authenticator, Authy, 1Password, Bitwarden, etc.):\n\n");
    out.push_str(&format!("  {uri}\n\n"));
    out.push_str("  To generate a scannable QR code image from this URI:\n");
    out.push_str("    • qrencode -t PNG -o totp.png '<URI>'\n");
    out.push_str("    • python3 -c \"import qrcode; qrcode.make('<URI>').save('totp.png')\"\n");
    out.push_str("    • Paste the URI at https://qr.io or similar tool\n\n");
    out.push_str(&format!("  Label:     {label}\n"));
    out.push_str(&format!("  Issuer:    {issuer}\n"));
    out.push_str(&format!("  Algorithm: {algorithm}\n"));
    out.push_str(&format!("  Digits:    {digits}\n"));
    out.push_str(&format!("  Period:    {period}s\n"));
    Ok(out)
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
            let h2 = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
            let byte = ((h1 << 4) | h2) as u8;
            out.push(byte as char);
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for byte in s.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b':' | b'@') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
