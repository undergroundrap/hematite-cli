use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use md5::Digest as _;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "ssh_key_tools",
        "description": "Parse and inspect SSH public keys from authorized_keys or .pub files. Detects key type (RSA/ECDSA/Ed25519/DSA), computes SHA-256 and MD5 fingerprints, extracts key size in bits, and validates format. Distinct from pem_tools which handles X.509 certificates.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "fingerprint", "validate", "authorized_keys"],
                    "description": "info (default — full key detail per key), fingerprint (SHA-256 and MD5 fingerprints only), validate (format check with VALID/INVALID verdict per key), authorized_keys (parse a multi-key authorized_keys file showing all keys in a table)"
                },
                "key": {
                    "type": "string",
                    "description": "A single SSH public key in authorized_keys line format: 'type base64 [comment]'"
                },
                "file": {
                    "type": "string",
                    "description": "Path to a .pub file or authorized_keys file"
                },
                "text": {
                    "type": "string",
                    "description": "Raw content of a public key or authorized_keys file (multiple keys, one per line)"
                }
            }
        }
    })
}

struct WireReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> WireReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn read_u32(&mut self) -> Option<u32> {
        if self.remaining() < 4 {
            return None;
        }
        let v = u32::from_be_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Some(v)
    }

    fn read_bytes(&mut self) -> Option<Vec<u8>> {
        let len = self.read_u32()? as usize;
        if self.remaining() < len {
            return None;
        }
        let bytes = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Some(bytes)
    }

    fn read_string(&mut self) -> Option<String> {
        let bytes = self.read_bytes()?;
        String::from_utf8(bytes).ok()
    }

    fn read_mpint(&mut self) -> Option<Vec<u8>> {
        let bytes = self.read_bytes()?;
        // strip leading zero sign byte
        if bytes.first() == Some(&0x00) {
            Some(bytes[1..].to_vec())
        } else {
            Some(bytes)
        }
    }
}

fn mpint_bit_len(m: &[u8]) -> usize {
    if m.is_empty() {
        return 0;
    }
    m.len() * 8 - m[0].leading_zeros() as usize
}

struct ParsedKey {
    key_type: String,
    bits: Option<usize>,
    curve: Option<String>,
    comment: String,
    raw_bytes: Vec<u8>,
    security: &'static str,
}

fn is_key_type_token(s: &str) -> bool {
    matches!(
        s,
        "ssh-rsa"
            | "ssh-dss"
            | "ssh-ed25519"
            | "ssh-ed448"
            | "ecdsa-sha2-nistp256"
            | "ecdsa-sha2-nistp384"
            | "ecdsa-sha2-nistp521"
            | "sk-ssh-ed25519@openssh.com"
            | "sk-ecdsa-sha2-nistp256@openssh.com"
    )
}

fn parse_key_line(line: &str) -> Result<ParsedKey, String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Err("skip".to_string());
    }

    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() < 2 {
        return Err("too few fields".to_string());
    }

    // Find the key-type token (may be preceded by authorized_keys options)
    let (type_idx, _) = words
        .iter()
        .enumerate()
        .find(|(_, w)| is_key_type_token(w))
        .ok_or_else(|| "unrecognized key type".to_string())?;

    let key_type_str = words[type_idx];
    let b64_str = words
        .get(type_idx + 1)
        .copied()
        .ok_or("missing base64 field")?;
    let comment = if type_idx + 2 < words.len() {
        words[type_idx + 2..].join(" ")
    } else {
        String::new()
    };

    let raw_bytes = B64
        .decode(b64_str)
        .map_err(|e| format!("base64 error: {e}"))?;

    let mut r = WireReader::new(&raw_bytes);
    let wire_type = r
        .read_string()
        .ok_or("truncated wire format")?;

    let (bits, curve) = match wire_type.as_str() {
        "ssh-rsa" => {
            let _e = r.read_mpint();
            let modulus = r.read_mpint().unwrap_or_default();
            (Some(mpint_bit_len(&modulus)), None)
        }
        "ssh-dss" => {
            let p = r.read_mpint().unwrap_or_default();
            (Some(mpint_bit_len(&p)), None)
        }
        "ecdsa-sha2-nistp256" => (Some(256), Some("nistp256 (P-256)")),
        "ecdsa-sha2-nistp384" => (Some(384), Some("nistp384 (P-384)")),
        "ecdsa-sha2-nistp521" => (Some(521), Some("nistp521 (P-521)")),
        "ssh-ed25519" | "sk-ssh-ed25519@openssh.com" => (Some(255), None),
        "ssh-ed448" => (Some(448), None),
        _ => (None, None),
    };

    let security = key_security(wire_type.as_str(), bits);

    Ok(ParsedKey {
        key_type: wire_type,
        bits,
        curve: curve.map(|s| s.to_string()),
        comment,
        raw_bytes,
        security,
    })
}

fn key_security(key_type: &str, bits: Option<usize>) -> &'static str {
    match key_type {
        "ssh-dss" => "WEAK (DSA fixed 1024-bit, deprecated in OpenSSH 7.0)",
        "ssh-rsa" => match bits {
            Some(b) if b < 2048 => "WEAK (RSA < 2048 bits)",
            Some(b) if b < 3072 => "GOOD (RSA 2048-bit)",
            Some(_) => "STRONG (RSA >= 3072-bit)",
            None => "UNKNOWN",
        },
        "ecdsa-sha2-nistp256" => "GOOD (NIST P-256, potentially NIST-backdoored curves)",
        "ecdsa-sha2-nistp384" => "GOOD (NIST P-384, potentially NIST-backdoored curves)",
        "ecdsa-sha2-nistp521" => "GOOD (NIST P-521, potentially NIST-backdoored curves)",
        "ssh-ed25519" | "sk-ssh-ed25519@openssh.com" => "STRONG (Ed25519 / Curve25519)",
        "ssh-ed448" => "STRONG (Ed448 / Curve448)",
        _ => "UNKNOWN",
    }
}

fn sha256_fp(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    format!("SHA256:{}", B64.encode(hash))
}

fn md5_fp(bytes: &[u8]) -> String {
    let hash = md5::Md5::digest(bytes);
    let hex: String = hash
        .iter()
        .enumerate()
        .map(|(i, b)| {
            if i == 0 {
                format!("{:02x}", b)
            } else {
                format!(":{:02x}", b)
            }
        })
        .collect();
    hex
}

fn format_key_info(k: &ParsedKey, show_all: bool) -> String {
    let mut out = String::new();
    out.push_str(&format!("Type:        {}\n", k.key_type));
    if let Some(bits) = k.bits {
        out.push_str(&format!("Key size:    {} bits\n", bits));
    }
    if let Some(ref curve) = k.curve {
        out.push_str(&format!("Curve:       {}\n", curve));
    }
    out.push_str(&format!("Security:    {}\n", k.security));
    if show_all {
        out.push_str(&format!("Fingerprint: {}\n", sha256_fp(&k.raw_bytes)));
        out.push_str(&format!("MD5:         {}\n", md5_fp(&k.raw_bytes)));
    }
    if !k.comment.is_empty() {
        out.push_str(&format!("Comment:     {}\n", k.comment));
    }
    out
}

fn load_input(args: &Value) -> Result<String, String> {
    if let Some(k) = args["key"].as_str().or(args["text"].as_str()) {
        return Ok(k.to_string());
    }
    if let Some(path) = args["file"].as_str() {
        return std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read file '{path}': {e}"));
    }
    Err("provide 'key', 'text', or 'file'".to_string())
}

fn action_info(content: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut count = 0;
    let mut errors = 0;

    for (i, line) in content.lines().enumerate() {
        match parse_key_line(line) {
            Ok(k) => {
                count += 1;
                if count > 1 {
                    out.push_str(&format!("\n─── Key {} ───\n", count));
                }
                out.push_str(&format_key_info(&k, true));
            }
            Err(e) if e == "skip" => {}
            Err(e) => {
                errors += 1;
                out.push_str(&format!("Line {}: error — {}\n", i + 1, e));
            }
        }
    }

    if count == 0 && errors == 0 {
        return Err("no SSH public keys found — provide an authorized_keys line or .pub file content".to_string());
    }
    if count == 0 {
        return Err(format!("no valid keys found ({errors} parse error(s))"));
    }
    Ok(out)
}

fn action_fingerprint(content: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut count = 0;

    for line in content.lines() {
        match parse_key_line(line) {
            Ok(k) => {
                count += 1;
                let comment = if k.comment.is_empty() {
                    String::new()
                } else {
                    format!("  {}", k.comment)
                };
                out.push_str(&format!("{}{}\n", sha256_fp(&k.raw_bytes), comment));
                out.push_str(&format!("MD5:{}\n", md5_fp(&k.raw_bytes)));
            }
            Err(e) if e == "skip" => {}
            Err(_) => {}
        }
    }

    if count == 0 {
        return Err("no SSH public keys found".to_string());
    }
    Ok(out)
}

fn action_validate(content: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut valid = 0;
    let mut invalid = 0;

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        match parse_key_line(trimmed) {
            Ok(k) => {
                valid += 1;
                let comment = if k.comment.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", k.comment)
                };
                out.push_str(&format!(
                    "Line {:3}: VALID   {} {} bits{}\n",
                    i + 1,
                    k.key_type,
                    k.bits.map(|b| b.to_string()).unwrap_or_else(|| "?".to_string()),
                    comment
                ));
                let sec = k.security;
                if sec.starts_with("WEAK") {
                    out.push_str(&format!("          WARNING: {sec}\n"));
                }
            }
            Err(e) if e == "skip" => {}
            Err(e) => {
                invalid += 1;
                out.push_str(&format!("Line {:3}: INVALID — {e}\n", i + 1));
            }
        }
    }

    if valid == 0 && invalid == 0 {
        return Err("no SSH public keys found".to_string());
    }

    out.push('\n');
    out.push_str(&format!(
        "Result: {} valid, {} invalid\n",
        valid, invalid
    ));
    if invalid == 0 {
        out.push_str("Verdict: VALID\n");
    } else {
        out.push_str("Verdict: INVALID\n");
    }

    Ok(out)
}

fn action_authorized_keys(content: &str) -> Result<String, String> {
    let mut rows: Vec<(String, String, String, String)> = Vec::new();

    for line in content.lines() {
        match parse_key_line(line) {
            Ok(k) => {
                let bits = k
                    .bits
                    .map(|b| b.to_string())
                    .unwrap_or_else(|| "—".to_string());
                let fp = sha256_fp(&k.raw_bytes);
                let comment = if k.comment.is_empty() {
                    "—".to_string()
                } else {
                    k.comment.clone()
                };
                rows.push((k.key_type, bits, fp, comment));
            }
            Err(e) if e == "skip" => {}
            Err(_) => {}
        }
    }

    if rows.is_empty() {
        return Err("no SSH public keys found".to_string());
    }

    let w_type = rows.iter().map(|r| r.0.len()).max().unwrap_or(8).max(8);
    let w_bits = rows.iter().map(|r| r.1.len()).max().unwrap_or(4).max(4);
    let w_fp = 51; // SHA256: + 43 chars

    let header = format!(
        "{:<w_type$}  {:>w_bits$}  {:<w_fp$}  Comment",
        "Type", "Bits", "Fingerprint (SHA-256)"
    );
    let sep = "-".repeat(header.len());

    let mut out = format!("{}\n{}\n", header, sep);
    for (kt, bits, fp, cmt) in &rows {
        out.push_str(&format!(
            "{:<w_type$}  {:>w_bits$}  {:<w_fp$}  {}\n",
            kt, bits, fp, cmt
        ));
    }
    out.push_str(&format!("\n{} key(s)\n", rows.len()));

    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args["action"].as_str().unwrap_or("info");
    let content = load_input(args)?;

    match action {
        "info" => action_info(&content),
        "fingerprint" => action_fingerprint(&content),
        "validate" => action_validate(&content),
        "authorized_keys" => action_authorized_keys(&content),
        _ => Err(format!("unknown action '{action}'")),
    }
}
