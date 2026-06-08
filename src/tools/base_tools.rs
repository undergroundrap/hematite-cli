use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("encode");

    match action {
        "encode" => action_encode(args),
        "decode" => action_decode(args),
        "identify" => action_identify(args),
        other => Err(format!(
            "Unknown action '{}'. Valid: encode, decode, identify",
            other
        )),
    }
}

fn get_input(args: &Value) -> Result<String, String> {
    args.get("input")
        .or_else(|| args.get("text"))
        .or_else(|| args.get("data"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'input' — pass the text or bytes to encode/decode".to_string())
}

fn get_encoding(args: &Value) -> Option<String> {
    args.get("encoding")
        .or_else(|| args.get("format"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase())
}

// ── Base16 ─────────────────────────────────────────────────────────────────

fn base16_encode(data: &[u8], upper: bool) -> String {
    data.iter()
        .map(|b| {
            if upper {
                format!("{:02X}", b)
            } else {
                format!("{:02x}", b)
            }
        })
        .collect()
}

fn base16_decode(s: &str) -> Result<Vec<u8>, String> {
    let s: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    if s.len() % 2 != 0 {
        return Err(format!("Base16 string length {} must be even", s.len()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|_| format!("Invalid hex pair '{}' at position {}", &s[i..i + 2], i))
        })
        .collect()
}

// ── Base32 (RFC 4648) ───────────────────────────────────────────────────────

const BASE32_ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn base32_encode(data: &[u8]) -> String {
    let mut result = String::new();
    let mut buf: u16 = 0;
    let mut bits: u8 = 0;

    for &byte in data {
        buf = (buf << 8) | byte as u16;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            result.push(BASE32_ALPHA[((buf >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        result.push(BASE32_ALPHA[((buf << (5 - bits)) & 0x1f) as usize] as char);
    }
    result
}

fn base32_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim().to_uppercase();
    let s = s.trim_end_matches('=');
    let mut result = Vec::new();
    let mut buf: u16 = 0;
    let mut bits: u8 = 0;

    for c in s.chars() {
        let val = BASE32_ALPHA
            .iter()
            .position(|&b| b == c as u8)
            .ok_or_else(|| format!("Invalid base32 character: '{}'", c))? as u16;
        buf = (buf << 5) | val;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            result.push((buf >> bits) as u8);
        }
    }
    Ok(result)
}

// ── Base58 (Bitcoin alphabet) ───────────────────────────────────────────────

const BASE58_ALPHA: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn base58_encode(data: &[u8]) -> String {
    let leading_zeros = data.iter().take_while(|&&b| b == 0).count();
    let mut digits: Vec<u32> = vec![0];

    for &byte in data {
        let mut carry = byte as u32;
        for d in digits.iter_mut() {
            carry += *d * 256;
            *d = carry % 58;
            carry /= 58;
        }
        while carry > 0 {
            digits.push(carry % 58);
            carry /= 58;
        }
    }

    let mut result = String::new();
    for _ in 0..leading_zeros {
        result.push('1');
    }
    for &d in digits.iter().rev().skip_while(|&&d| d == 0) {
        result.push(BASE58_ALPHA[d as usize] as char);
    }
    if result.len() == leading_zeros {
        result.push('1');
    }
    result
}

fn base58_decode(s: &str) -> Result<Vec<u8>, String> {
    let leading_ones = s.chars().take_while(|&c| c == '1').count();
    let mut bytes: Vec<u32> = vec![0];

    for c in s.chars() {
        let val = BASE58_ALPHA
            .iter()
            .position(|&b| b == c as u8)
            .ok_or_else(|| format!("Invalid base58 character: '{}'", c))? as u32;
        let mut carry = val;
        for b in bytes.iter_mut() {
            carry += *b * 58;
            *b = carry & 0xff;
            carry >>= 8;
        }
        while carry > 0 {
            bytes.push(carry & 0xff);
            carry >>= 8;
        }
    }

    let skip = bytes.iter().rev().take_while(|&&b| b == 0).count();
    let mut result: Vec<u8> = vec![0; leading_ones];
    let payload: Vec<u8> = bytes.iter().rev().skip(skip).map(|&b| b as u8).collect();
    result.extend_from_slice(&payload);
    Ok(result)
}

// ── Base85 / Z85 (ZeroMQ base85 alphabet) ──────────────────────────────────

const BASE85_ALPHA: &[u8] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#";

fn base85_encode(data: &[u8]) -> String {
    // Pad input to multiple of 4 bytes
    let pad = (4 - data.len() % 4) % 4;
    let mut padded = data.to_vec();
    padded.extend(vec![0u8; pad]);

    let mut result = String::new();
    for chunk in padded.chunks(4) {
        let n = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let mut tmp = [0u8; 5];
        let mut v = n;
        for b in tmp.iter_mut().rev() {
            *b = BASE85_ALPHA[(v % 85) as usize];
            v /= 85;
        }
        result.extend(tmp.iter().map(|&c| c as char));
    }
    // Drop the chars that represent padding bytes (pad chars encode as the last pad chars)
    let trim = if pad > 0 { pad } else { 0 };
    result.truncate(result.len() - trim);
    result
}

fn base85_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    let rem = s.len() % 5;
    // Pad encoded string to multiple of 5
    let pad = if rem == 0 { 0 } else { 5 - rem };
    let padded: String = s.chars().chain(std::iter::repeat_n('0', pad)).collect();

    let mut result = Vec::new();
    for chunk in padded.as_bytes().chunks(5) {
        let mut n: u64 = 0;
        for &c in chunk {
            let val = BASE85_ALPHA
                .iter()
                .position(|&b| b == c)
                .ok_or_else(|| format!("Invalid base85 character: '{}'", c as char))?
                as u64;
            n = n * 85 + val;
            if n > u32::MAX as u64 {
                return Err("Base85 chunk overflow".to_string());
            }
        }
        result.extend_from_slice(&(n as u32).to_be_bytes());
    }
    // Remove bytes that came from padding
    if pad > 0 {
        result.truncate(result.len() - pad);
    }
    Ok(result)
}

// ── Actions ─────────────────────────────────────────────────────────────────

fn action_encode(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let data = input.as_bytes();
    let encoding = get_encoding(args);

    let mut out = String::from("base_tools — encode\n\n");

    match encoding.as_deref() {
        Some("base16") | Some("hex") | Some("b16") => {
            out.push_str(&format!("Base16: {}\n", base16_encode(data, true)));
        }
        Some("base32") | Some("b32") => {
            out.push_str(&format!("Base32: {}\n", base32_encode(data)));
        }
        Some("base58") | Some("b58") => {
            out.push_str(&format!("Base58: {}\n", base58_encode(data)));
        }
        Some("base85") | Some("z85") | Some("b85") | Some("ascii85") => {
            out.push_str(&format!("Base85: {}\n", base85_encode(data)));
        }
        None | Some("all") => {
            let preview = if input.len() > 50 {
                format!("{}…", &input[..50])
            } else {
                input.clone()
            };
            out.push_str(&format!("Input:  {} bytes — {:?}\n\n", data.len(), preview));
            out.push_str(&format!("Base16: {}\n", base16_encode(data, true)));
            out.push_str(&format!("Base32: {}\n", base32_encode(data)));
            out.push_str(&format!("Base58: {}\n", base58_encode(data)));
            out.push_str(&format!("Base85: {}\n", base85_encode(data)));
        }
        Some(other) => {
            return Err(format!(
                "Unknown encoding '{}'. Valid: base16, base32, base58, base85",
                other
            ));
        }
    }
    Ok(out)
}

fn action_decode(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let encoding = get_encoding(args).ok_or_else(|| {
        "Missing 'encoding' — specify base16, base32, base58, or base85".to_string()
    })?;

    let decoded = match encoding.as_str() {
        "base16" | "hex" | "b16" => base16_decode(&input)?,
        "base32" | "b32" => base32_decode(&input)?,
        "base58" | "b58" => base58_decode(&input)?,
        "base85" | "z85" | "b85" | "ascii85" => base85_decode(&input)?,
        other => {
            return Err(format!(
                "Unknown encoding '{}'. Valid: base16, base32, base58, base85",
                other
            ))
        }
    };

    let mut out = format!("base_tools — decode ({})\n\n", encoding);
    out.push_str(&format!("Decoded {} byte(s)\n", decoded.len()));

    match std::str::from_utf8(&decoded) {
        Ok(t) => out.push_str(&format!("As UTF-8: {}\n", t)),
        Err(_) => {
            out.push_str("(not valid UTF-8)\n");
            out.push_str(&format!("As hex:   {}\n", base16_encode(&decoded, true)));
        }
    }
    Ok(out)
}

fn action_identify(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let s = input.trim();

    let mut out = String::from("base_tools — identify\n\n");
    out.push_str(&format!("Input: {} chars\n\n", s.len()));

    let is_b16 = s.len() % 2 == 0 && s.chars().all(|c| c.is_ascii_hexdigit());
    let is_b32 = !s.is_empty()
        && s.to_uppercase()
            .trim_end_matches('=')
            .chars()
            .all(|c| matches!(c, 'A'..='Z' | '2'..='7'));
    let b58_chars = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let is_b58 = !s.is_empty() && s.chars().all(|c| b58_chars.contains(c));
    let b85_chars =
        "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#";
    let is_b85 = !s.is_empty() && s.chars().all(|c| b85_chars.contains(c));

    let mut found = false;
    if is_b16 {
        out.push_str("  ✓ Base16 (hex) — all hex chars, even length\n");
        found = true;
    }
    if is_b32 {
        out.push_str("  ✓ Base32 — A–Z + 2–7 alphabet\n");
        found = true;
    }
    if is_b58 {
        out.push_str("  ✓ Base58 — Bitcoin/IPFS alphabet (no 0OIl)\n");
        found = true;
    }
    if is_b85 {
        out.push_str("  ✓ Base85 (Z85) — printable ASCII alphabet\n");
        found = true;
    }
    if !found {
        out.push_str("  No recognised encoding detected.\n");
    }

    Ok(out)
}
