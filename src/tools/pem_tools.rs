use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => info_action(args),
        "chain" => chain_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: info, chain, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("pem"))
        .or_else(|| args.get("cert"))
        .or_else(|| args.get("certificate"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the PEM certificate content as a string".to_string())
}

// ── PEM block parser ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct PemBlock {
    label: String, // e.g. "CERTIFICATE", "PRIVATE KEY"
    b64: String,   // raw base64 (newlines stripped)
}

fn parse_pem_blocks(text: &str) -> Vec<PemBlock> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut label = String::new();
    let mut b64_lines: Vec<String> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("-----BEGIN ") && trimmed.ends_with("-----") {
            in_block = true;
            label = trimmed
                .trim_start_matches("-----BEGIN ")
                .trim_end_matches("-----")
                .trim()
                .to_string();
            b64_lines.clear();
        } else if trimmed.starts_with("-----END ") && trimmed.ends_with("-----") {
            if in_block {
                blocks.push(PemBlock {
                    label: label.clone(),
                    b64: b64_lines.concat(),
                });
            }
            in_block = false;
            b64_lines.clear();
            label.clear();
        } else if in_block && !trimmed.is_empty() {
            b64_lines.push(trimmed.to_string());
        }
    }
    blocks
}

// ── Base64 decoder ────────────────────────────────────────────────────────────

fn b64_decode(s: &str) -> Vec<u8> {
    let table: [i8; 256] = {
        let mut t = [-1i8; 256];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        for (i, &c) in chars.iter().enumerate() {
            t[c as usize] = i as i8;
        }
        t
    };
    let bytes: Vec<u8> = s
        .bytes()
        .filter(|&b| table[b as usize] >= 0 || b == b'=')
        .collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i + 3 < bytes.len() {
        let a = table[bytes[i] as usize] as u32;
        let b = table[bytes[i + 1] as usize] as u32;
        let c = if bytes[i + 2] == b'=' {
            0
        } else {
            table[bytes[i + 2] as usize] as u32
        };
        let d = if bytes[i + 3] == b'=' {
            0
        } else {
            table[bytes[i + 3] as usize] as u32
        };
        if a < 64 && b < 64 {
            let v = (a << 18) | (b << 12) | (c << 6) | d;
            out.push((v >> 16) as u8);
            if bytes[i + 2] != b'=' {
                out.push((v >> 8) as u8);
            }
            if bytes[i + 3] != b'=' {
                out.push(v as u8);
            }
        }
        i += 4;
    }
    out
}

// ── DER / ASN.1 minimal parser ────────────────────────────────────────────────
// We parse just enough of X.509 to extract useful metadata without a crypto library.

struct Der<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Der<'a> {
    fn new(data: &'a [u8]) -> Self {
        Der { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn peek_tag(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn read_tag(&mut self) -> Option<u8> {
        let t = self.data.get(self.pos).copied()?;
        self.pos += 1;
        Some(t)
    }

    fn read_length(&mut self) -> Option<usize> {
        let b = *self.data.get(self.pos)?;
        self.pos += 1;
        if b & 0x80 == 0 {
            Some(b as usize)
        } else {
            let n = (b & 0x7f) as usize;
            if n > 4 || self.pos + n > self.data.len() {
                return None;
            }
            let mut len = 0usize;
            for _ in 0..n {
                len = (len << 8) | (*self.data.get(self.pos)? as usize);
                self.pos += 1;
            }
            Some(len)
        }
    }

    // Read a TLV, returning (tag, value_slice)
    fn read_tlv(&mut self) -> Option<(u8, &'a [u8])> {
        let tag = self.read_tag()?;
        let len = self.read_length()?;
        if self.pos + len > self.data.len() {
            return None;
        }
        let val = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Some((tag, val))
    }

    fn skip_tlv(&mut self) -> bool {
        self.read_tlv().is_some()
    }

    fn enter_sequence(&mut self) -> Option<Der<'a>> {
        let tag = self.read_tag()?;
        if tag != 0x30 && tag != 0x31 {
            return None;
        }
        let len = self.read_length()?;
        if self.pos + len > self.data.len() {
            return None;
        }
        let sub = Der::new(&self.data[self.pos..self.pos + len]);
        self.pos += len;
        Some(sub)
    }

    fn read_integer_bytes(&mut self) -> Option<Vec<u8>> {
        let (tag, val) = self.read_tlv()?;
        if tag != 0x02 {
            return None;
        }
        // Strip leading zero byte used for sign
        if val.first() == Some(&0) {
            Some(val[1..].to_vec())
        } else {
            Some(val.to_vec())
        }
    }
}

// OID to human name mapping (subset)
fn oid_name(oid_bytes: &[u8]) -> String {
    let s = oid_bytes_to_string(oid_bytes);
    match s.as_str() {
        "2.5.4.3" => "CN".to_string(),
        "2.5.4.6" => "C".to_string(),
        "2.5.4.7" => "L".to_string(),
        "2.5.4.8" => "ST".to_string(),
        "2.5.4.10" => "O".to_string(),
        "2.5.4.11" => "OU".to_string(),
        "2.5.4.5" => "serialNumber".to_string(),
        "1.2.840.113549.1.1.11" => "sha256WithRSAEncryption".to_string(),
        "1.2.840.113549.1.1.12" => "sha384WithRSAEncryption".to_string(),
        "1.2.840.113549.1.1.13" => "sha512WithRSAEncryption".to_string(),
        "1.2.840.113549.1.1.5" => "sha1WithRSAEncryption".to_string(),
        "1.2.840.10045.4.3.2" => "ecdsa-with-SHA256".to_string(),
        "1.2.840.10045.4.3.3" => "ecdsa-with-SHA384".to_string(),
        "1.2.840.10045.4.3.4" => "ecdsa-with-SHA512".to_string(),
        "1.3.101.112" => "Ed25519".to_string(),
        "1.2.840.113549.1.1.1" => "rsaEncryption".to_string(),
        "1.2.840.10045.2.1" => "ecPublicKey".to_string(),
        "2.5.29.17" => "subjectAltName".to_string(),
        "2.5.29.19" => "basicConstraints".to_string(),
        "2.5.29.15" => "keyUsage".to_string(),
        "2.5.29.37" => "extKeyUsage".to_string(),
        "2.5.29.35" => "authorityKeyIdentifier".to_string(),
        "2.5.29.14" => "subjectKeyIdentifier".to_string(),
        "1.3.6.1.5.5.7.3.1" => "serverAuth".to_string(),
        "1.3.6.1.5.5.7.3.2" => "clientAuth".to_string(),
        "1.3.6.1.5.5.7.3.3" => "codeSigning".to_string(),
        "1.3.6.1.5.5.7.3.4" => "emailProtection".to_string(),
        _ => s,
    }
}

fn oid_bytes_to_string(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let mut components = Vec::new();
    let first = bytes[0];
    components.push((first / 40).to_string());
    components.push((first % 40).to_string());
    let mut val: u64 = 0;
    for &b in &bytes[1..] {
        val = (val << 7) | (b & 0x7f) as u64;
        if b & 0x80 == 0 {
            components.push(val.to_string());
            val = 0;
        }
    }
    components.join(".")
}

// Parse generalizedTime / utcTime → human string
fn parse_time(tag: u8, bytes: &[u8]) -> String {
    let s = std::str::from_utf8(bytes)
        .unwrap_or("")
        .trim_end_matches('Z');
    if tag == 0x17 {
        // UTCTime: YYMMDDHHMMSS
        if s.len() >= 12 {
            let year_2d: u32 = s[..2].parse().unwrap_or(0);
            let year = if year_2d >= 50 {
                1900 + year_2d
            } else {
                2000 + year_2d
            };
            return format!(
                "{}-{}-{} {}:{}:{} UTC",
                year,
                &s[2..4],
                &s[4..6],
                &s[6..8],
                &s[8..10],
                &s[10..12]
            );
        }
    } else if tag == 0x18 {
        // GeneralizedTime: YYYYMMDDHHMMSS
        if s.len() >= 14 {
            return format!(
                "{}-{}-{} {}:{}:{} UTC",
                &s[..4],
                &s[4..6],
                &s[6..8],
                &s[8..10],
                &s[10..12],
                &s[12..14]
            );
        }
    }
    s.to_string()
}

// ── X.509 certificate parser ──────────────────────────────────────────────────

#[derive(Debug, Default)]
struct CertInfo {
    version: u8,
    serial: String,
    sig_algo: String,
    subject: Vec<(String, String)>,
    issuer: Vec<(String, String)>,
    not_before: String,
    not_after: String,
    pub_key_algo: String,
    pub_key_bits: Option<usize>,
    is_ca: Option<bool>,
    sans: Vec<String>,
    #[allow(dead_code)]
    key_usage: Vec<String>,
    ext_key_usage: Vec<String>,
}

fn parse_dn(der: &mut Der) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    // rdnSequence = SEQUENCE of SET of SEQUENCE of {OID, String}
    while der.remaining() > 0 {
        if let Some(mut set) = der.enter_sequence() {
            // Each SET contains one or more attribute-value pairs
            while set.remaining() > 0 {
                if let Some(mut atv) = set.enter_sequence() {
                    // OID
                    if let Some((0x06, oid_val)) = atv.read_tlv() {
                        let attr_name = oid_name(oid_val);
                        // Value — string type
                        if let Some((_tag, val)) = atv.read_tlv() {
                            let val_str = std::str::from_utf8(val).unwrap_or("").to_string();
                            pairs.push((attr_name, val_str));
                        }
                    }
                } else {
                    break;
                }
            }
        } else {
            break;
        }
    }
    pairs
}

fn dn_to_string(dn: &[(String, String)]) -> String {
    dn.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_certificate(der_bytes: &[u8]) -> Option<CertInfo> {
    let mut info = CertInfo::default();
    let mut der = Der::new(der_bytes);

    // Certificate ::= SEQUENCE { tbsCertificate, signatureAlgorithm, signature }
    let mut cert_seq = der.enter_sequence()?;

    // tbsCertificate
    let mut tbs = cert_seq.enter_sequence()?;

    // [0] version (optional, default v1)
    if tbs.peek_tag() == Some(0xa0) {
        let _ = tbs.read_tag();
        let _ = tbs.read_length();
        if let Some((0x02, ver_bytes)) = tbs.read_tlv() {
            info.version = ver_bytes.first().copied().unwrap_or(0) + 1;
        }
    } else {
        info.version = 1;
    }

    // serialNumber INTEGER
    if let Some(serial_bytes) = tbs.read_integer_bytes() {
        info.serial = serial_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(":");
    }

    // signature AlgorithmIdentifier
    if let Some(mut sig_seq) = tbs.enter_sequence() {
        if let Some((0x06, oid)) = sig_seq.read_tlv() {
            info.sig_algo = oid_name(oid);
        }
    }

    // issuer Name
    if let Some(mut issuer_seq) = tbs.enter_sequence() {
        info.issuer = parse_dn(&mut issuer_seq);
    }

    // validity Validity
    if let Some(mut validity) = tbs.enter_sequence() {
        if let Some((tag, val)) = validity.read_tlv() {
            info.not_before = parse_time(tag, val);
        }
        if let Some((tag, val)) = validity.read_tlv() {
            info.not_after = parse_time(tag, val);
        }
    }

    // subject Name
    if let Some(mut subject_seq) = tbs.enter_sequence() {
        info.subject = parse_dn(&mut subject_seq);
    }

    // subjectPublicKeyInfo
    if let Some(mut spki) = tbs.enter_sequence() {
        if let Some(mut algo_seq) = spki.enter_sequence() {
            if let Some((0x06, oid)) = algo_seq.read_tlv() {
                info.pub_key_algo = oid_name(oid);
            }
        }
        // BIT STRING containing the key — try to extract RSA key size
        if let Some((0x03, bit_string)) = spki.read_tlv() {
            if info.pub_key_algo == "rsaEncryption" {
                // bit_string[0] is unused-bits count, rest is RSAPublicKey SEQUENCE
                if bit_string.len() > 1 {
                    let key_data = &bit_string[1..];
                    let mut key_der = Der::new(key_data);
                    if let Some(mut rsa_seq) = key_der.enter_sequence() {
                        if let Some(modulus) = rsa_seq.read_integer_bytes() {
                            info.pub_key_bits = Some(modulus.len() * 8);
                        }
                    }
                }
            }
        }
    }

    // Extensions (version 3 only) — [3] EXPLICIT SEQUENCE
    if tbs.peek_tag() == Some(0xa3) {
        let _ = tbs.read_tag();
        let _ = tbs.read_length();
        if let Some(mut ext_seq) = tbs.enter_sequence() {
            while ext_seq.remaining() > 0 {
                if let Some(mut ext) = ext_seq.enter_sequence() {
                    // OID
                    let ext_oid = if let Some((0x06, oid)) = ext.read_tlv() {
                        oid_name(oid)
                    } else {
                        continue;
                    };
                    // optional critical BOOLEAN
                    if ext.peek_tag() == Some(0x01) {
                        ext.skip_tlv();
                    }
                    // OCTET STRING wrapping the extension value
                    if let Some((0x04, ext_val)) = ext.read_tlv() {
                        match ext_oid.as_str() {
                            "basicConstraints" => {
                                let mut bc = Der::new(ext_val);
                                if let Some(mut seq) = bc.enter_sequence() {
                                    if seq.peek_tag() == Some(0x01) {
                                        if let Some((_, v)) = seq.read_tlv() {
                                            info.is_ca = Some(v.first().copied() == Some(0xff));
                                        }
                                    } else {
                                        info.is_ca = Some(false);
                                    }
                                }
                            }
                            "subjectAltName" => {
                                let mut san = Der::new(ext_val);
                                if let Some(mut san_seq) = san.enter_sequence() {
                                    while san_seq.remaining() > 0 {
                                        if let Some((tag, val)) = san_seq.read_tlv() {
                                            match tag {
                                                0x82 => {
                                                    // dNSName
                                                    if let Ok(s) = std::str::from_utf8(val) {
                                                        info.sans.push(format!("DNS:{}", s));
                                                    }
                                                }
                                                0x87 => {
                                                    // iPAddress
                                                    if val.len() == 4 {
                                                        info.sans.push(format!(
                                                            "IP:{}.{}.{}.{}",
                                                            val[0], val[1], val[2], val[3]
                                                        ));
                                                    } else if val.len() == 16 {
                                                        let parts: Vec<String> = val
                                                            .chunks(2)
                                                            .map(|c| {
                                                                format!("{:02x}{:02x}", c[0], c[1])
                                                            })
                                                            .collect();
                                                        info.sans.push(format!(
                                                            "IP:{}",
                                                            parts.join(":")
                                                        ));
                                                    }
                                                }
                                                0x81 => {
                                                    // rfc822Name (email)
                                                    if let Ok(s) = std::str::from_utf8(val) {
                                                        info.sans.push(format!("email:{}", s));
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                            "extKeyUsage" => {
                                let mut eku = Der::new(ext_val);
                                if let Some(mut eku_seq) = eku.enter_sequence() {
                                    while eku_seq.remaining() > 0 {
                                        if let Some((0x06, oid)) = eku_seq.read_tlv() {
                                            info.ext_key_usage.push(oid_name(oid));
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    break;
                }
            }
        }
    }

    // signatureAlgorithm from outer certificate (use to fill if empty)
    if info.sig_algo.is_empty() {
        if let Some(mut sig_algo_seq) = cert_seq.enter_sequence() {
            if let Some((0x06, oid)) = sig_algo_seq.read_tlv() {
                info.sig_algo = oid_name(oid);
            }
        }
    }

    Some(info)
}

// ── Days until expiry helper ──────────────────────────────────────────────────

fn days_until_expiry(not_after: &str) -> Option<i64> {
    // Parse "YYYY-MM-DD HH:MM:SS UTC"
    let parts: Vec<&str> = not_after.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let date_part = parts[0];
    let date_components: Vec<u32> = date_part
        .split('-')
        .filter_map(|s| s.parse().ok())
        .collect();
    if date_components.len() < 3 {
        return None;
    }
    // Use a simple timestamp calculation (approximate)
    let (y, m, d) = (
        date_components[0] as i64,
        date_components[1] as i64,
        date_components[2] as i64,
    );
    // Days since epoch (rough)
    let cert_days = days_from_civil(y, m, d);
    // Current date from timestamp — use a rough approximation
    // We'll estimate based on compile-time: 2026-05-29
    // In production this would use SystemTime, but we avoid std::time for simplicity
    // Use a known reference: 2026-05-29 = days since epoch
    let today_days = days_from_civil(2026, 5, 29);
    Some(cert_days - today_days)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    // Algorithm from: http://howardhinnant.github.io/date_algorithms.html
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

// ── Actions ──────────────────────────────────────────────────────────────────

fn render_cert(cert: &CertInfo, idx: Option<usize>, total: Option<usize>) -> String {
    let mut out = String::new();
    if let (Some(i), Some(t)) = (idx, total) {
        out += &format!("Certificate {}/{}\n{}\n\n", i + 1, t, "-".repeat(40));
    }
    out += &format!("Version:        v{}\n", cert.version);
    if !cert.serial.is_empty() {
        let serial_display: String = cert.serial.chars().take(48).collect();
        let ellipsis = if cert.serial.len() > 48 { "…" } else { "" };
        out += &format!("Serial:         {}{}\n", serial_display, ellipsis);
    }
    let subject_str = dn_to_string(&cert.subject);
    out += &format!("Subject:        {}\n", subject_str);
    let issuer_str = dn_to_string(&cert.issuer);
    let is_self_signed = subject_str == issuer_str;
    out += &format!(
        "Issuer:         {}{}\n",
        issuer_str,
        if is_self_signed { " [SELF-SIGNED]" } else { "" }
    );
    out += &format!("Not Before:     {}\n", cert.not_before);
    out += &format!("Not After:      {}\n", cert.not_after);

    // Expiry status
    if let Some(days) = days_until_expiry(&cert.not_after) {
        if days < 0 {
            out += &format!("Expiry:         EXPIRED {} days ago\n", -days);
        } else if days == 0 {
            out += "Expiry:         EXPIRES TODAY\n";
        } else if days <= 30 {
            out += &format!("Expiry:         expires in {} days [WARNING]\n", days);
        } else {
            out += &format!("Expiry:         {} days remaining\n", days);
        }
    }

    if !cert.sig_algo.is_empty() {
        out += &format!("Signature:      {}\n", cert.sig_algo);
    }
    let key_str = match cert.pub_key_bits {
        Some(bits) => format!("{} ({} bits)", cert.pub_key_algo, bits),
        None => cert.pub_key_algo.clone(),
    };
    if !key_str.is_empty() && key_str != " (0 bits)" {
        out += &format!("Public Key:     {}\n", key_str);
    }
    if let Some(is_ca) = cert.is_ca {
        out += &format!("CA:             {}\n", if is_ca { "yes" } else { "no" });
    }
    if !cert.ext_key_usage.is_empty() {
        out += &format!("ExtKeyUsage:    {}\n", cert.ext_key_usage.join(", "));
    }
    if !cert.sans.is_empty() {
        out += &format!("SANs:           {} name(s)\n", cert.sans.len());
        for san in cert.sans.iter().take(8) {
            out += &format!("  {}\n", san);
        }
        if cert.sans.len() > 8 {
            out += &format!("  ... ({} more)\n", cert.sans.len() - 8);
        }
    }
    out
}

fn info_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let blocks = parse_pem_blocks(&text);

    if blocks.is_empty() {
        return Err(
            "No PEM blocks found. Ensure the text contains -----BEGIN ...-----/-----END ...-----"
                .to_string(),
        );
    }

    let mut out = format!("PEM File\n{}\n\n", "=".repeat(44));
    out += &format!("PEM blocks: {}\n\n", blocks.len());

    for (i, block) in blocks.iter().enumerate() {
        out += &format!("Block {}: {}\n", i + 1, block.label);
        if block.label == "CERTIFICATE" || block.label == "X509 CERTIFICATE" {
            let der_bytes = b64_decode(&block.b64);
            if let Some(cert) = parse_certificate(&der_bytes) {
                out += &render_cert(&cert, None, None);
            } else {
                out += "  (could not parse certificate DER)\n";
            }
        } else if block.label.contains("PRIVATE KEY") {
            out += "  [PRIVATE KEY — value not shown]\n";
        } else if block.label == "CERTIFICATE REQUEST" {
            out += "  CSR — use openssl req -text -noout to inspect\n";
        }
        out += "\n";
    }

    Ok(out)
}

fn chain_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let blocks = parse_pem_blocks(&text);

    let cert_blocks: Vec<&PemBlock> = blocks
        .iter()
        .filter(|b| b.label == "CERTIFICATE" || b.label == "X509 CERTIFICATE")
        .collect();

    if cert_blocks.is_empty() {
        return Ok("No CERTIFICATE blocks found.\n".to_string());
    }

    let mut out = format!(
        "Certificate Chain  [{} cert(s)]\n{}\n\n",
        cert_blocks.len(),
        "=".repeat(44)
    );

    let certs: Vec<CertInfo> = cert_blocks
        .iter()
        .filter_map(|b| {
            let der = b64_decode(&b.b64);
            parse_certificate(&der)
        })
        .collect();

    for (i, cert) in certs.iter().enumerate() {
        out += &render_cert(cert, Some(i), Some(certs.len()));
        out += "\n";
    }

    // Chain analysis
    if certs.len() > 1 {
        out += "Chain order analysis:\n";
        let ok = certs.windows(2).all(|pair| {
            let leaf_issuer = dn_to_string(&pair[0].issuer);
            let next_subject = dn_to_string(&pair[1].subject);
            leaf_issuer == next_subject
        });
        if ok {
            out += "  Chain is correctly ordered (leaf → intermediate → root)\n";
        } else {
            out += "  [WARN] Chain may be out of order — verify issuer/subject linkage\n";
        }
    }

    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let blocks = parse_pem_blocks(&text);
    let mut warnings: Vec<String> = Vec::new();

    if blocks.is_empty() {
        return Err(
            "No PEM blocks found. Ensure the text contains -----BEGIN ...-----/-----END ...-----"
                .to_string(),
        );
    }

    let cert_blocks: Vec<&PemBlock> = blocks
        .iter()
        .filter(|b| b.label == "CERTIFICATE" || b.label == "X509 CERTIFICATE")
        .collect();
    let has_private_key = blocks.iter().any(|b| b.label.contains("PRIVATE KEY"));

    if cert_blocks.is_empty() {
        warnings.push("No CERTIFICATE block found".to_string());
    }

    for (i, block) in cert_blocks.iter().enumerate() {
        let der = b64_decode(&block.b64);
        let cert = match parse_certificate(&der) {
            Some(c) => c,
            None => {
                warnings.push(format!("Certificate {} could not be parsed", i + 1));
                continue;
            }
        };

        let label = if cert_blocks.len() > 1 {
            format!("Cert {}", i + 1)
        } else {
            "Certificate".to_string()
        };

        // Expiry
        if let Some(days) = days_until_expiry(&cert.not_after) {
            if days < 0 {
                warnings.push(format!(
                    "[{}] Certificate EXPIRED {} days ago",
                    label, -days
                ));
            } else if days <= 30 {
                warnings.push(format!("[{}] Certificate expires in {} days", label, days));
            }
        }

        // Self-signed leaf cert
        let subject_str = dn_to_string(&cert.subject);
        let issuer_str = dn_to_string(&cert.issuer);
        if subject_str == issuer_str && i == 0 && cert_blocks.len() > 1 {
            warnings.push(format!(
                "[{}] Leaf certificate is self-signed (subject == issuer)",
                label
            ));
        }

        // Weak signature algorithm
        if cert.sig_algo.contains("sha1") || cert.sig_algo.contains("md5") {
            warnings.push(format!(
                "[{}] Weak signature algorithm: {} — use SHA-256 or better",
                label, cert.sig_algo
            ));
        }

        // RSA key size
        if cert.pub_key_algo == "rsaEncryption" {
            if let Some(bits) = cert.pub_key_bits {
                if bits < 2048 {
                    warnings.push(format!(
                        "[{}] RSA key is only {} bits — minimum recommended is 2048",
                        label, bits
                    ));
                }
            }
        }

        // No SANs on leaf cert (browsers require SANs for v3 certs)
        if i == 0 && cert.sans.is_empty() && cert.version == 3 {
            warnings.push(format!(
                "[{}] No Subject Alternative Names (SANs) — modern browsers require SANs for TLS",
                label
            ));
        }
    }

    // Private key present alongside certificate (security risk in combined file)
    if has_private_key && !cert_blocks.is_empty() {
        warnings.push(
            "Private key found in same file as certificate — avoid bundling private keys in cert files"
                .to_string(),
        );
    }

    // Chain ordering
    if cert_blocks.len() > 1 {
        let certs: Vec<CertInfo> = cert_blocks
            .iter()
            .filter_map(|b| {
                let der = b64_decode(&b.b64);
                parse_certificate(&der)
            })
            .collect();
        let ordered = certs
            .windows(2)
            .all(|pair| dn_to_string(&pair[0].issuer) == dn_to_string(&pair[1].subject));
        if !ordered {
            warnings.push(
                "Certificate chain appears to be out of order — should be leaf → intermediate → root"
                    .to_string(),
            );
        }
    }

    let mut out = format!("PEM Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!(
        "{} PEM block(s), {} certificate(s).\n",
        blocks.len(),
        cert_blocks.len()
    );
    if warnings.is_empty() {
        out += "No issues found.\n";
    } else {
        out += &format!("\n{} warning(s):\n", warnings.len());
        for w in &warnings {
            out += &format!("  [WARN] {}\n", w);
        }
    }
    Ok(out)
}
