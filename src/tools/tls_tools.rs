use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "name": "tls_tools",
        "description": "Parse and decode TLS records, ClientHello/ServerHello handshake messages, cipher suites, and extensions from raw hex bytes without external tools. Flags Heartbleed extension, weak/broken cipher suites, and missing PFS. Complements pcap_tools and network_header_tools for TLS debugging and security review.",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "client_hello", "server_hello", "cipher_suites", "extensions"],
                    "description": "parse (default) = auto-detect and decode TLS record(s), client_hello = detailed ClientHello breakdown, server_hello = detailed ServerHello breakdown, cipher_suites = enumerate and grade cipher suites from ClientHello, extensions = list all extensions with explanations"
                },
                "hex": {
                    "type": "string",
                    "description": "Hex-encoded TLS record or handshake bytes (spaces, colons, and newlines stripped automatically)"
                },
                "file": {
                    "type": "string",
                    "description": "Path to a binary file containing raw TLS bytes"
                }
            }
        }
    })
}

// ── Reader ────────────────────────────────────────────────────────────────────

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }
    fn read_u8(&mut self) -> Result<u8, String> {
        if self.pos >= self.data.len() {
            return Err("unexpected end of data".into());
        }
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }
    fn read_u16be(&mut self) -> Result<u16, String> {
        Ok(((self.read_u8()? as u16) << 8) | self.read_u8()? as u16)
    }
    fn read_u24be(&mut self) -> Result<u32, String> {
        Ok(((self.read_u8()? as u32) << 16)
            | ((self.read_u8()? as u32) << 8)
            | self.read_u8()? as u32)
    }
    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, String> {
        if self.pos + n > self.data.len() {
            return Err(format!(
                "need {} bytes, only {} remaining",
                n,
                self.remaining()
            ));
        }
        let v = self.data[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(v)
    }
    fn read_vec8(&mut self) -> Result<Vec<u8>, String> {
        let len = self.read_u8()? as usize;
        self.read_bytes(len)
    }
    fn read_vec16(&mut self) -> Result<Vec<u8>, String> {
        let len = self.read_u16be()? as usize;
        self.read_bytes(len)
    }
}

// ── Name tables ───────────────────────────────────────────────────────────────

fn content_type_name(t: u8) -> &'static str {
    match t {
        20 => "ChangeCipherSpec",
        21 => "Alert",
        22 => "Handshake",
        23 => "ApplicationData",
        24 => "Heartbeat",
        _ => "Unknown",
    }
}

fn tls_version_name(v: u16) -> &'static str {
    match v {
        0x0300 => "SSL 3.0",
        0x0301 => "TLS 1.0",
        0x0302 => "TLS 1.1",
        0x0303 => "TLS 1.2",
        0x0304 => "TLS 1.3",
        _ => "Unknown",
    }
}

fn handshake_type_name(t: u8) -> &'static str {
    match t {
        0 => "HelloRequest",
        1 => "ClientHello",
        2 => "ServerHello",
        4 => "NewSessionTicket",
        5 => "EndOfEarlyData",
        8 => "EncryptedExtensions",
        11 => "Certificate",
        12 => "ServerKeyExchange",
        13 => "CertificateRequest",
        14 => "ServerHelloDone",
        15 => "CertificateVerify",
        16 => "ClientKeyExchange",
        20 => "Finished",
        24 => "KeyUpdate",
        254 => "MessageHash",
        _ => "Unknown",
    }
}

// Returns (name, grade)
fn cipher_suite_info(code: u16) -> (&'static str, &'static str) {
    match code {
        // TLS 1.3 AEAD suites — always STRONG
        0x1301 => ("TLS_AES_128_GCM_SHA256", "STRONG"),
        0x1302 => ("TLS_AES_256_GCM_SHA384", "STRONG"),
        0x1303 => ("TLS_CHACHA20_POLY1305_SHA256", "STRONG"),
        0x1304 => ("TLS_AES_128_CCM_SHA256", "STRONG"),
        0x1305 => ("TLS_AES_128_CCM_8_SHA256", "GOOD"),
        // ECDHE-ECDSA AEAD
        0xc02b => ("TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256", "STRONG"),
        0xc02c => ("TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384", "STRONG"),
        0xcca9 => ("TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256", "STRONG"),
        0xc023 => ("TLS_ECDHE_ECDSA_WITH_AES_128_CBC_SHA256", "GOOD"),
        0xc024 => ("TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA384", "GOOD"),
        0xc009 => ("TLS_ECDHE_ECDSA_WITH_AES_128_CBC_SHA", "GOOD"),
        0xc00a => ("TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA", "GOOD"),
        // ECDHE-RSA AEAD
        0xc02f => ("TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256", "STRONG"),
        0xc030 => ("TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384", "STRONG"),
        0xcca8 => ("TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256", "STRONG"),
        0xc027 => ("TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA256", "GOOD"),
        0xc028 => ("TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA384", "GOOD"),
        0xc013 => ("TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA", "GOOD"),
        0xc014 => ("TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA", "GOOD"),
        // DHE-RSA AEAD
        0x009e => ("TLS_DHE_RSA_WITH_AES_128_GCM_SHA256", "STRONG"),
        0x009f => ("TLS_DHE_RSA_WITH_AES_256_GCM_SHA384", "STRONG"),
        0xccaa => ("TLS_DHE_RSA_WITH_CHACHA20_POLY1305_SHA256", "STRONG"),
        0x0067 => ("TLS_DHE_RSA_WITH_AES_128_CBC_SHA256", "GOOD"),
        0x006b => ("TLS_DHE_RSA_WITH_AES_256_CBC_SHA256", "GOOD"),
        // RSA key exchange — no PFS
        0x002f => ("TLS_RSA_WITH_AES_128_CBC_SHA", "WEAK"),
        0x0035 => ("TLS_RSA_WITH_AES_256_CBC_SHA", "WEAK"),
        0x003c => ("TLS_RSA_WITH_AES_128_CBC_SHA256", "WEAK"),
        0x003d => ("TLS_RSA_WITH_AES_256_CBC_SHA256", "WEAK"),
        0x009c => ("TLS_RSA_WITH_AES_128_GCM_SHA256", "WEAK"),
        0x009d => ("TLS_RSA_WITH_AES_256_GCM_SHA384", "WEAK"),
        // Broken
        0x000a => ("TLS_RSA_WITH_3DES_EDE_CBC_SHA", "BROKEN"),
        0xc012 => ("TLS_ECDHE_RSA_WITH_3DES_EDE_CBC_SHA", "WEAK"),
        0x0004 => ("TLS_RSA_WITH_RC4_128_MD5", "BROKEN"),
        0x0005 => ("TLS_RSA_WITH_RC4_128_SHA", "BROKEN"),
        0xc007 => ("TLS_ECDHE_ECDSA_WITH_RC4_128_SHA", "BROKEN"),
        0xc011 => ("TLS_ECDHE_RSA_WITH_RC4_128_SHA", "BROKEN"),
        // NULL / ANON
        0x0000 => ("TLS_NULL_WITH_NULL_NULL", "BROKEN"),
        0x0001 => ("TLS_RSA_WITH_NULL_MD5", "BROKEN"),
        0x0002 => ("TLS_RSA_WITH_NULL_SHA", "BROKEN"),
        // Signaling
        0x00ff => ("TLS_EMPTY_RENEGOTIATION_INFO_SCSV", "INFO"),
        0x5600 => ("TLS_FALLBACK_SCSV", "INFO"),
        _ => ("(unknown)", "UNKNOWN"),
    }
}

fn extension_name(code: u16) -> &'static str {
    match code {
        0x0000 => "server_name (SNI)",
        0x0001 => "max_fragment_length",
        0x0005 => "status_request (OCSP stapling)",
        0x000a => "supported_groups (elliptic_curves)",
        0x000b => "ec_point_formats",
        0x000d => "signature_algorithms",
        0x000e => "use_srtp",
        0x000f => "heartbeat [CVE-2014-0160 Heartbleed]",
        0x0010 => "application_layer_protocol_negotiation (ALPN)",
        0x0012 => "signed_certificate_timestamp (SCT)",
        0x0015 => "padding",
        0x0016 => "encrypt_then_mac",
        0x0017 => "extended_master_secret",
        0x001c => "record_size_limit",
        0x0023 => "session_ticket",
        0x0029 => "pre_shared_key",
        0x002a => "early_data (0-RTT)",
        0x002b => "supported_versions",
        0x002c => "cookie",
        0x002d => "psk_key_exchange_modes",
        0x002f => "certificate_authorities",
        0x0031 => "post_handshake_auth",
        0x0032 => "signature_algorithms_cert",
        0x0033 => "key_share",
        0xff01 => "renegotiation_info",
        _ => "(unknown extension)",
    }
}

fn named_group_name(code: u16) -> &'static str {
    match code {
        0x0017 => "secp256r1 (P-256)",
        0x0018 => "secp384r1 (P-384)",
        0x0019 => "secp521r1 (P-521)",
        0x001d => "x25519",
        0x001e => "x448",
        0x0100 => "ffdhe2048",
        0x0101 => "ffdhe3072",
        0x0102 => "ffdhe4096",
        0x0103 => "ffdhe6144",
        0x0104 => "ffdhe8192",
        _ => "(unknown group)",
    }
}

fn sig_alg_name(code: u16) -> &'static str {
    match code {
        0x0401 => "rsa_pkcs1_sha256",
        0x0501 => "rsa_pkcs1_sha384",
        0x0601 => "rsa_pkcs1_sha512",
        0x0403 => "ecdsa_secp256r1_sha256",
        0x0503 => "ecdsa_secp384r1_sha384",
        0x0603 => "ecdsa_secp521r1_sha512",
        0x0804 => "rsa_pss_rsae_sha256",
        0x0805 => "rsa_pss_rsae_sha384",
        0x0806 => "rsa_pss_rsae_sha512",
        0x0807 => "ed25519",
        0x0808 => "ed448",
        0x0809 => "rsa_pss_pss_sha256",
        0x080a => "rsa_pss_pss_sha384",
        0x080b => "rsa_pss_pss_sha512",
        0x0201 => "rsa_pkcs1_sha1 (deprecated)",
        0x0203 => "ecdsa_sha1 (deprecated)",
        _ => "(unknown sig alg)",
    }
}

fn alert_description_name(code: u8) -> &'static str {
    match code {
        0 => "close_notify",
        10 => "unexpected_message",
        20 => "bad_record_mac",
        22 => "record_overflow",
        40 => "handshake_failure",
        42 => "bad_certificate",
        43 => "unsupported_certificate",
        44 => "certificate_revoked",
        45 => "certificate_expired",
        46 => "certificate_unknown",
        47 => "illegal_parameter",
        48 => "unknown_ca",
        49 => "access_denied",
        50 => "decode_error",
        51 => "decrypt_error",
        70 => "protocol_version",
        71 => "insufficient_security",
        80 => "internal_error",
        86 => "inappropriate_fallback",
        90 => "user_canceled",
        112 => "unrecognized_name",
        116 => "certificate_required",
        120 => "no_application_protocol",
        _ => "unknown",
    }
}

fn grade_icon(grade: &str) -> &'static str {
    match grade {
        "STRONG" => "[+]",
        "GOOD" => "[~]",
        "WEAK" => "[!]",
        "BROKEN" => "[X]",
        _ => "[ ]",
    }
}

fn is_grease(v: u16) -> bool {
    (v & 0x0F0F == 0x0A0A) && ((v >> 8) == (v & 0xFF))
}

fn hex_str(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ── Extension decoding ────────────────────────────────────────────────────────

fn decode_sni(data: &[u8]) -> String {
    let mut r = Reader::new(data);
    if r.read_u16be().is_err() {
        return "(parse error)".into();
    }
    if let Ok(name_type) = r.read_u8() {
        if name_type == 0 {
            if let Ok(name_bytes) = r.read_vec16() {
                if let Ok(s) = std::str::from_utf8(&name_bytes) {
                    return s.to_string();
                }
            }
        }
    }
    "(parse error)".into()
}

fn decode_alpn(data: &[u8]) -> Vec<String> {
    let mut r = Reader::new(data);
    let mut protos = Vec::new();
    if let Ok(list_bytes) = r.read_vec16() {
        let mut lr = Reader::new(&list_bytes);
        while lr.remaining() > 0 {
            match lr.read_vec8() {
                Ok(b) => {
                    if let Ok(s) = std::str::from_utf8(&b) {
                        protos.push(s.to_string());
                    }
                }
                Err(_) => break,
            }
        }
    }
    protos
}

fn decode_supported_versions(data: &[u8], is_client: bool) -> Vec<String> {
    let mut r = Reader::new(data);
    let mut versions = Vec::new();
    if is_client {
        if let Ok(list_bytes) = r.read_vec8() {
            let mut lr = Reader::new(&list_bytes);
            while lr.remaining() >= 2 {
                if let Ok(v) = lr.read_u16be() {
                    versions.push(format!("{} (0x{:04x})", tls_version_name(v), v));
                }
            }
        }
    } else if let Ok(v) = r.read_u16be() {
        versions.push(format!("{} (0x{:04x})", tls_version_name(v), v));
    }
    versions
}

fn decode_supported_groups(data: &[u8]) -> Vec<String> {
    let mut r = Reader::new(data);
    let mut groups = Vec::new();
    if let Ok(list_bytes) = r.read_vec16() {
        let mut lr = Reader::new(&list_bytes);
        while lr.remaining() >= 2 {
            if let Ok(g) = lr.read_u16be() {
                groups.push(named_group_name(g).to_string());
            }
        }
    }
    groups
}

fn decode_sig_algs(data: &[u8]) -> Vec<String> {
    let mut r = Reader::new(data);
    let mut algs = Vec::new();
    if let Ok(list_bytes) = r.read_vec16() {
        let mut lr = Reader::new(&list_bytes);
        while lr.remaining() >= 2 {
            if let Ok(a) = lr.read_u16be() {
                algs.push(sig_alg_name(a).to_string());
            }
        }
    }
    algs
}

fn format_extension_value(ext_type: u16, data: &[u8], is_client: bool) -> String {
    match ext_type {
        0x0000 => format!("SNI = {}", decode_sni(data)),
        0x0010 => {
            let protos = decode_alpn(data);
            format!("protocols = [{}]", protos.join(", "))
        }
        0x002b => {
            let vs = decode_supported_versions(data, is_client);
            format!("[{}]", vs.join(", "))
        }
        0x000a => {
            let groups = decode_supported_groups(data);
            format!("[{}]", groups.join(", "))
        }
        0x000d => {
            let algs = decode_sig_algs(data);
            format!("[{}]", algs.join(", "))
        }
        0x0023 => format!("{} bytes", data.len()),
        0x0015 => format!("{} zero bytes", data.len()),
        0x0017 => "enabled".into(),
        0x0016 => "enabled".into(),
        0x0031 => "enabled".into(),
        0x002a => "requested".into(),
        0x000f => {
            let mode = data.first().copied().unwrap_or(0);
            format!("mode=0x{:02x}  *** CVE-2014-0160 HEARTBLEED ***", mode)
        }
        _ => {
            if data.is_empty() {
                "(empty)".into()
            } else if data.len() <= 6 {
                hex_str(data)
            } else {
                format!("{} bytes", data.len())
            }
        }
    }
}

// ── Handshake structures ──────────────────────────────────────────────────────

struct ClientHello {
    legacy_version: u16,
    random: Vec<u8>,
    session_id: Vec<u8>,
    cipher_suites: Vec<u16>,
    compression_methods: Vec<u8>,
    extensions: Vec<(u16, Vec<u8>)>,
}

fn parse_client_hello(data: &[u8]) -> Result<ClientHello, String> {
    let mut r = Reader::new(data);
    let legacy_version = r.read_u16be()?;
    let random = r.read_bytes(32)?;
    let session_id = r.read_vec8()?;
    let cs_bytes = r.read_vec16()?;
    let mut cipher_suites = Vec::new();
    let mut cr = Reader::new(&cs_bytes);
    while cr.remaining() >= 2 {
        cipher_suites.push(cr.read_u16be()?);
    }
    let compression_methods = r.read_vec8()?;
    let mut extensions = Vec::new();
    if r.remaining() >= 2 {
        let ext_bytes = r.read_vec16()?;
        let mut er = Reader::new(&ext_bytes);
        while er.remaining() >= 4 {
            let et = er.read_u16be()?;
            let ed = er.read_vec16()?;
            extensions.push((et, ed));
        }
    }
    Ok(ClientHello {
        legacy_version,
        random,
        session_id,
        cipher_suites,
        compression_methods,
        extensions,
    })
}

struct ServerHello {
    legacy_version: u16,
    random: Vec<u8>,
    session_id: Vec<u8>,
    cipher_suite: u16,
    compression_method: u8,
    extensions: Vec<(u16, Vec<u8>)>,
}

fn parse_server_hello(data: &[u8]) -> Result<ServerHello, String> {
    let mut r = Reader::new(data);
    let legacy_version = r.read_u16be()?;
    let random = r.read_bytes(32)?;
    let session_id = r.read_vec8()?;
    let cipher_suite = r.read_u16be()?;
    let compression_method = r.read_u8()?;
    let mut extensions = Vec::new();
    if r.remaining() >= 2 {
        let ext_bytes = r.read_vec16()?;
        let mut er = Reader::new(&ext_bytes);
        while er.remaining() >= 4 {
            let et = er.read_u16be()?;
            let ed = er.read_vec16()?;
            extensions.push((et, ed));
        }
    }
    Ok(ServerHello {
        legacy_version,
        random,
        session_id,
        cipher_suite,
        compression_method,
        extensions,
    })
}

// Strip TLS record header and handshake framing to get raw handshake body
fn extract_handshake_body(data: &[u8], expected_hs_type: u8) -> Result<Vec<u8>, String> {
    if data.is_empty() {
        return Err("empty input".into());
    }
    let mut offset = 0;

    // Skip TLS record header (content-type=22, 2-byte version, 2-byte length)
    if data[0] == 22 && data.len() >= 5 {
        let rec_len = ((data[3] as usize) << 8) | (data[4] as usize);
        if 5 + rec_len <= data.len() {
            offset = 5;
        }
    }

    // Skip handshake header (type=expected, 3-byte length)
    if offset < data.len() && data[offset] == expected_hs_type && data.len() >= offset + 4 {
        let hs_len = ((data[offset + 1] as usize) << 16)
            | ((data[offset + 2] as usize) << 8)
            | (data[offset + 3] as usize);
        if offset + 4 + hs_len <= data.len() {
            return Ok(data[offset + 4..offset + 4 + hs_len].to_vec());
        }
        // Body runs to end
        return Ok(data[offset + 4..].to_vec());
    }

    Ok(data[offset..].to_vec())
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_parse(data: &[u8]) -> Result<String, String> {
    let mut r = Reader::new(data);
    let mut out = String::new();
    let mut idx = 0;

    while r.remaining() >= 5 {
        let content_type = r.read_u8()?;
        let version = r.read_u16be()?;
        let len = r.read_u16be()? as usize;
        if len > r.remaining() {
            break;
        }
        let body = r.read_bytes(len)?;
        idx += 1;
        out.push_str(&format!(
            "── TLS Record #{} ─────────────────────────────────────\n",
            idx
        ));
        out.push_str(&format!(
            "  Content-Type : 0x{:02x} ({})\n",
            content_type,
            content_type_name(content_type)
        ));
        out.push_str(&format!(
            "  Version      : 0x{:04x} ({})\n",
            version,
            tls_version_name(version)
        ));
        out.push_str(&format!("  Length       : {} bytes\n", len));

        if content_type == 22 {
            let mut hr = Reader::new(&body);
            while hr.remaining() >= 4 {
                let hs_type = hr.read_u8()?;
                let hs_len = hr.read_u24be()? as usize;
                if hs_len > hr.remaining() {
                    break;
                }
                let hs_data = hr.read_bytes(hs_len)?;
                out.push_str(&format!(
                    "  Handshake    : {} (0x{:02x}, {} bytes)\n",
                    handshake_type_name(hs_type),
                    hs_type,
                    hs_len
                ));
                match hs_type {
                    1 => {
                        if let Ok(ch) = parse_client_hello(&hs_data) {
                            out.push_str(&format!(
                                "    Legacy-version : {}\n",
                                tls_version_name(ch.legacy_version)
                            ));
                            out.push_str(&format!(
                                "    Session-ID     : {} bytes\n",
                                ch.session_id.len()
                            ));
                            out.push_str(&format!(
                                "    Cipher suites  : {} offered\n",
                                ch.cipher_suites.len()
                            ));
                            out.push_str(&format!(
                                "    Extensions     : {}\n",
                                ch.extensions.len()
                            ));
                            for (et, ed) in &ch.extensions {
                                if *et == 0x0000 {
                                    out.push_str(&format!(
                                        "    SNI            : {}\n",
                                        decode_sni(ed)
                                    ));
                                }
                                if *et == 0x0010 {
                                    out.push_str(&format!(
                                        "    ALPN           : [{}]\n",
                                        decode_alpn(ed).join(", ")
                                    ));
                                }
                            }
                        }
                    }
                    2 => {
                        if let Ok(sh) = parse_server_hello(&hs_data) {
                            out.push_str(&format!(
                                "    Legacy-version : {}\n",
                                tls_version_name(sh.legacy_version)
                            ));
                            let (cs_name, cs_grade) = cipher_suite_info(sh.cipher_suite);
                            out.push_str(&format!(
                                "    Chosen cipher  : {} {} [{}]\n",
                                grade_icon(cs_grade),
                                cs_name,
                                cs_grade
                            ));
                        }
                    }
                    _ => {}
                }
            }
        } else if content_type == 21 && body.len() >= 2 {
            let level_str = match body[0] {
                1 => "warning",
                2 => "fatal",
                _ => "unknown",
            };
            out.push_str(&format!(
                "  Alert        : {} / {}\n",
                level_str,
                alert_description_name(body[1])
            ));
        } else if content_type == 20 {
            out.push_str("  (ChangeCipherSpec — encrypted traffic follows)\n");
        } else if content_type == 23 {
            out.push_str("  (ApplicationData — encrypted payload)\n");
        }
        out.push('\n');
    }

    if out.is_empty() {
        return Err("No complete TLS records found — provide a full TLS record starting with content-type byte (0x16 for Handshake)".into());
    }
    Ok(out)
}

fn action_client_hello(data: &[u8]) -> Result<String, String> {
    let body = extract_handshake_body(data, 1)?;
    let ch = parse_client_hello(&body).map_err(|e| format!("ClientHello parse error: {}", e))?;

    let mut out = String::new();
    out.push_str("── ClientHello ─────────────────────────────────────────\n");
    out.push_str(&format!(
        "  Legacy version : 0x{:04x} ({})\n",
        ch.legacy_version,
        tls_version_name(ch.legacy_version)
    ));
    out.push_str(&format!("  Random         : {}\n", hex_str(&ch.random)));
    if ch.session_id.is_empty() {
        out.push_str("  Session ID     : (empty — new session)\n");
    } else {
        out.push_str(&format!(
            "  Session ID     : {} bytes\n",
            ch.session_id.len()
        ));
    }
    out.push_str(&format!(
        "  Compression    : {:?}\n",
        ch.compression_methods
    ));

    out.push_str(&format!(
        "\n── Cipher Suites ({} offered) ───────────────────────────\n",
        ch.cipher_suites.len()
    ));
    let mut n_strong = 0usize;
    let mut n_grease = 0usize;
    for cs in &ch.cipher_suites {
        if is_grease(*cs) {
            n_grease += 1;
            out.push_str(&format!("  0x{:04x}  [GREASE]\n", cs));
            continue;
        }
        let (name, grade) = cipher_suite_info(*cs);
        if grade == "STRONG" {
            n_strong += 1;
        }
        out.push_str(&format!(
            "  0x{:04x}  {} {:<52} [{}]\n",
            cs,
            grade_icon(grade),
            name,
            grade
        ));
    }
    if n_grease > 0 {
        out.push_str(&format!(
            "  ({} GREASE values — modern browser/library detected)\n",
            n_grease
        ));
    }

    out.push_str(&format!(
        "\n── Extensions ({} present) ──────────────────────────────\n",
        ch.extensions.len()
    ));
    let mut heartbleed = false;
    for (et, ed) in &ch.extensions {
        if *et == 0x000f {
            heartbleed = true;
        }
        let name = extension_name(*et);
        let val = format_extension_value(*et, ed, true);
        out.push_str(&format!("  0x{:04x}  {:<50} {}\n", et, name, val));
    }

    out.push_str("\n── Security Assessment ──────────────────────────────────\n");
    if n_strong > 0 {
        out.push_str(&format!(
            "  [+] {} STRONG cipher suites (AEAD + PFS) offered\n",
            n_strong
        ));
    } else {
        out.push_str("  [!] No STRONG cipher suites — configuration is outdated\n");
    }
    if heartbleed {
        out.push_str("  [X] Heartbeat extension present — CVE-2014-0160 (Heartbleed) risk\n");
        out.push_str("      Fix: Upgrade OpenSSL >= 1.0.1g or disable Heartbeat\n");
    }

    Ok(out)
}

fn action_server_hello(data: &[u8]) -> Result<String, String> {
    let body = extract_handshake_body(data, 2)?;
    let sh = parse_server_hello(&body).map_err(|e| format!("ServerHello parse error: {}", e))?;

    let mut out = String::new();
    out.push_str("── ServerHello ─────────────────────────────────────────\n");
    out.push_str(&format!(
        "  Legacy version : 0x{:04x} ({})\n",
        sh.legacy_version,
        tls_version_name(sh.legacy_version)
    ));
    out.push_str(&format!("  Random         : {}\n", hex_str(&sh.random)));
    if !sh.session_id.is_empty() {
        out.push_str(&format!(
            "  Session ID     : {} bytes\n",
            sh.session_id.len()
        ));
    }
    out.push_str(&format!(
        "  Compression    : 0x{:02x}\n",
        sh.compression_method
    ));

    let (cs_name, cs_grade) = cipher_suite_info(sh.cipher_suite);
    out.push_str("\n── Chosen Cipher Suite ──────────────────────────────────\n");
    out.push_str(&format!(
        "  0x{:04x}  {} {} [{}]\n",
        sh.cipher_suite,
        grade_icon(cs_grade),
        cs_name,
        cs_grade
    ));

    out.push_str(&format!(
        "\n── Extensions ({} present) ──────────────────────────────\n",
        sh.extensions.len()
    ));
    for (et, ed) in &sh.extensions {
        let name = extension_name(*et);
        let val = format_extension_value(*et, ed, false);
        out.push_str(&format!("  0x{:04x}  {:<50} {}\n", et, name, val));
    }

    // Detect actual TLS version from supported_versions extension
    for (et, ed) in &sh.extensions {
        if *et == 0x002b {
            let vs = decode_supported_versions(ed, false);
            out.push_str(&format!("\n  Negotiated TLS: {}\n", vs.join(", ")));
        }
    }

    out.push_str("\n── Security Assessment ──────────────────────────────────\n");
    match cs_grade {
        "STRONG" => out.push_str("  [+] Chosen cipher is STRONG (AEAD + PFS)\n"),
        "GOOD" => out.push_str("  [~] Chosen cipher is GOOD (PFS but not AEAD)\n"),
        "WEAK" => out.push_str("  [!] Chosen cipher is WEAK (no PFS — RSA key exchange)\n"),
        "BROKEN" => out.push_str("  [X] Chosen cipher is BROKEN — do not use\n"),
        _ => {}
    }

    Ok(out)
}

fn action_cipher_suites(data: &[u8]) -> Result<String, String> {
    let body = extract_handshake_body(data, 1)?;
    let ch = parse_client_hello(&body).map_err(|e| format!("ClientHello parse error: {}", e))?;

    let mut strong = Vec::new();
    let mut good = Vec::new();
    let mut weak = Vec::new();
    let mut broken = Vec::new();
    let mut info_list = Vec::new();
    let mut grease_n = 0;

    for cs in &ch.cipher_suites {
        if is_grease(*cs) {
            grease_n += 1;
            continue;
        }
        let (name, grade) = cipher_suite_info(*cs);
        match grade {
            "STRONG" => strong.push((*cs, name)),
            "GOOD" => good.push((*cs, name)),
            "WEAK" => weak.push((*cs, name)),
            "BROKEN" => broken.push((*cs, name)),
            _ => info_list.push((*cs, name)),
        }
    }

    let mut out = String::new();
    out.push_str(&format!(
        "── Cipher Suite Analysis ({} total) ─────────────────────\n",
        ch.cipher_suites.len()
    ));
    out.push_str(&format!(
        "  [+] STRONG: {:3}   [~] GOOD: {:3}   [!] WEAK: {:3}   [X] BROKEN: {:3}\n\n",
        strong.len(),
        good.len(),
        weak.len(),
        broken.len()
    ));

    if !strong.is_empty() {
        out.push_str("[+] STRONG — AEAD + PFS:\n");
        for (code, name) in &strong {
            out.push_str(&format!("  0x{:04x}  {}\n", code, name));
        }
        out.push('\n');
    }
    if !good.is_empty() {
        out.push_str("[~] GOOD — PFS present:\n");
        for (code, name) in &good {
            out.push_str(&format!("  0x{:04x}  {}\n", code, name));
        }
        out.push('\n');
    }
    if !weak.is_empty() {
        out.push_str("[!] WEAK — no PFS (RSA key exchange):\n");
        for (code, name) in &weak {
            out.push_str(&format!("  0x{:04x}  {}\n", code, name));
        }
        out.push('\n');
    }
    if !broken.is_empty() {
        out.push_str("[X] BROKEN — RC4 / NULL / 3DES / EXPORT:\n");
        for (code, name) in &broken {
            out.push_str(&format!("  0x{:04x}  {}\n", code, name));
        }
        out.push('\n');
    }
    if !info_list.is_empty() {
        out.push_str("[ ] SIGNALING:\n");
        for (code, name) in &info_list {
            out.push_str(&format!("  0x{:04x}  {}\n", code, name));
        }
        out.push('\n');
    }
    if grease_n > 0 {
        out.push_str(&format!(
            "[GREASE: {} values — confirms modern client (Chrome/Firefox/Edge)]\n",
            grease_n
        ));
    }

    Ok(out)
}

fn action_extensions(data: &[u8]) -> Result<String, String> {
    // Try ClientHello first, then ServerHello
    let (extensions, is_client) = extract_handshake_body(data, 1)
        .and_then(|b| parse_client_hello(&b).map(|ch| (ch.extensions, true)))
        .or_else(|_| {
            extract_handshake_body(data, 2)
                .and_then(|b| parse_server_hello(&b).map(|sh| (sh.extensions, false)))
        })
        .map_err(|_| {
            "Could not parse extensions — provide ClientHello or ServerHello bytes".to_string()
        })?;

    let role = if is_client {
        "ClientHello"
    } else {
        "ServerHello"
    };
    let mut out = String::new();
    out.push_str(&format!(
        "── {} Extensions ({}) ───────────────────────────────\n",
        role,
        extensions.len()
    ));

    let mut heartbleed = false;
    for (et, ed) in &extensions {
        if *et == 0x000f {
            heartbleed = true;
        }
        let name = extension_name(*et);
        let val = format_extension_value(*et, ed, is_client);
        out.push_str(&format!("\n  [0x{:04x}] {}\n", et, name));
        out.push_str(&format!("    Value  : {}\n", val));
        out.push_str(&format!("    Size   : {} bytes\n", ed.len()));
    }

    if heartbleed {
        out.push_str("\n\n*** CVE-2014-0160 HEARTBLEED DETECTED ***\n");
        out.push_str("The Heartbeat extension (0x000f) is present.\n");
        out.push_str(
            "OpenSSL < 1.0.1g with Heartbeat enabled is vulnerable to memory disclosure.\n",
        );
        out.push_str("Fix: upgrade OpenSSL >= 1.0.1g, or compile with -DOPENSSL_NO_HEARTBEATS\n");
    }

    Ok(out)
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn load_input(args: &Value) -> Result<Vec<u8>, String> {
    if let Some(hex) = args.get("hex").and_then(Value::as_str) {
        let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if clean.len() % 2 != 0 {
            return Err("odd-length hex string".into());
        }
        (0..clean.len() / 2)
            .map(|i| u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string()))
            .collect()
    } else if let Some(path) = args.get("file").and_then(Value::as_str) {
        std::fs::read(path).map_err(|e| format!("Cannot read {}: {}", path, e))
    } else {
        Err("Provide 'hex' or 'file' input".into())
    }
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("parse");
    let data = load_input(args)?;
    match action {
        "parse" => action_parse(&data),
        "client_hello" => action_client_hello(&data),
        "server_hello" => action_server_hello(&data),
        "cipher_suites" => action_cipher_suites(&data),
        "extensions" => action_extensions(&data),
        other => Err(format!("Unknown action '{}'. Use: parse, client_hello, server_hello, cipher_suites, extensions", other)),
    }
}
