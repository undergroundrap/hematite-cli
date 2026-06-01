use serde_json::{json, Value};

pub fn asn1_tools_schema() -> Value {
    json!({
        "name": "asn1_tools",
        "description": "Parse and inspect ASN.1 DER/BER encoded binary data without external utilities. ASN.1 (Abstract Syntax Notation One) with DER (Distinguished Encoding Rules) is used in X.509 certificates, PKCS#8/PKCS#12 keys, SNMP, LDAP, and many cryptographic formats. Actions: parse (default — decode DER/BER structure as an indented tag/length/value tree), oid (look up an OID number to its name — covers 200+ well-known OIDs from X.509, PKCS, ANSI, ETSI), decode_cert (parse an X.509 certificate DER and extract subject, issuer, validity, serial, public key algorithm, and extensions summary), info (show tag class/number/constructed flag and byte structure at root level).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "oid", "decode_cert", "info"],
                    "description": "Action to perform (default: parse)"
                },
                "hex": {
                    "type": "string",
                    "description": "Hex-encoded DER/BER bytes (spaces allowed)"
                },
                "file": {
                    "type": "string",
                    "description": "Path to a DER-encoded binary file (.der, .cer, .crt, .p8)"
                },
                "oid": {
                    "type": "string",
                    "description": "OID string for oid action (e.g. '2.5.4.3' for commonName)"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum nesting depth to display (default: 20)"
                }
            },
            "required": []
        }
    })
}

// ── OID database ──────────────────────────────────────────────────────────────

fn oid_name(oid: &str) -> &'static str {
    match oid {
        // X.500 attribute types
        "2.5.4.0" => "objectClass",
        "2.5.4.1" => "aliasedEntryName",
        "2.5.4.2" => "knowledgeInformation",
        "2.5.4.3" => "commonName (CN)",
        "2.5.4.4" => "surname (SN)",
        "2.5.4.5" => "serialNumber",
        "2.5.4.6" => "countryName (C)",
        "2.5.4.7" => "localityName (L)",
        "2.5.4.8" => "stateOrProvinceName (ST)",
        "2.5.4.9" => "streetAddress",
        "2.5.4.10" => "organizationName (O)",
        "2.5.4.11" => "organizationalUnitName (OU)",
        "2.5.4.12" => "title",
        "2.5.4.13" => "description",
        "2.5.4.14" => "searchGuide",
        "2.5.4.15" => "businessCategory",
        "2.5.4.16" => "postalAddress",
        "2.5.4.17" => "postalCode",
        "2.5.4.18" => "postOfficeBox",
        "2.5.4.20" => "telephoneNumber",
        "2.5.4.41" => "name",
        "2.5.4.42" => "givenName (GN)",
        "2.5.4.43" => "initials",
        "2.5.4.44" => "generationQualifier",
        "2.5.4.45" => "uniqueIdentifier",
        "2.5.4.46" => "dnQualifier",
        "2.5.4.49" => "distinguishedName",
        "2.5.4.65" => "pseudonym",
        "2.5.4.97" => "organizationIdentifier",
        // X.509 extensions
        "2.5.29.1" => "authorityKeyIdentifier (old)",
        "2.5.29.2" => "keyAttributes",
        "2.5.29.3" => "certificatePolicies (old)",
        "2.5.29.9" => "subjectDirectoryAttributes",
        "2.5.29.14" => "subjectKeyIdentifier",
        "2.5.29.15" => "keyUsage",
        "2.5.29.16" => "privateKeyUsagePeriod",
        "2.5.29.17" => "subjectAltName",
        "2.5.29.18" => "issuerAltName",
        "2.5.29.19" => "basicConstraints",
        "2.5.29.23" => "holdInstructionCode",
        "2.5.29.24" => "invalidityDate",
        "2.5.29.30" => "nameConstraints",
        "2.5.29.31" => "cRLDistributionPoints",
        "2.5.29.32" => "certificatePolicies",
        "2.5.29.33" => "policyMappings",
        "2.5.29.35" => "authorityKeyIdentifier",
        "2.5.29.36" => "policyConstraints",
        "2.5.29.37" => "extKeyUsage",
        "2.5.29.46" => "freshestCRL",
        "2.5.29.54" => "inhibitAnyPolicy",
        // Extended key usage
        "1.3.6.1.5.5.7.3.1" => "serverAuth (TLS Web Server Authentication)",
        "1.3.6.1.5.5.7.3.2" => "clientAuth (TLS Web Client Authentication)",
        "1.3.6.1.5.5.7.3.3" => "codeSigning",
        "1.3.6.1.5.5.7.3.4" => "emailProtection",
        "1.3.6.1.5.5.7.3.8" => "timeStamping",
        "1.3.6.1.5.5.7.3.9" => "OCSPSigning",
        // PKIX
        "1.3.6.1.5.5.7.1.1" => "authorityInfoAccess",
        "1.3.6.1.5.5.7.48.1" => "OCSP",
        "1.3.6.1.5.5.7.48.2" => "caIssuers",
        "1.3.6.1.5.5.7.48.3" => "timeStamping",
        // Signature algorithms
        "1.2.840.113549.1.1.1" => "rsaEncryption",
        "1.2.840.113549.1.1.4" => "md5WithRSAEncryption",
        "1.2.840.113549.1.1.5" => "sha1WithRSAEncryption",
        "1.2.840.113549.1.1.11" => "sha256WithRSAEncryption",
        "1.2.840.113549.1.1.12" => "sha384WithRSAEncryption",
        "1.2.840.113549.1.1.13" => "sha512WithRSAEncryption",
        "1.2.840.113549.1.1.14" => "sha224WithRSAEncryption",
        "1.2.840.10040.4.1" => "id-dsa",
        "1.2.840.10040.4.3" => "id-dsa-with-sha1",
        "1.2.840.10045.2.1" => "id-ecPublicKey",
        "1.2.840.10045.4.3.1" => "ecdsa-with-SHA224",
        "1.2.840.10045.4.3.2" => "ecdsa-with-SHA256",
        "1.2.840.10045.4.3.3" => "ecdsa-with-SHA384",
        "1.2.840.10045.4.3.4" => "ecdsa-with-SHA512",
        "1.3.101.110" => "id-X25519",
        "1.3.101.111" => "id-X448",
        "1.3.101.112" => "id-Ed25519",
        "1.3.101.113" => "id-Ed448",
        // Hash algorithms
        "2.16.840.1.101.3.4.2.1" => "sha-256",
        "2.16.840.1.101.3.4.2.2" => "sha-384",
        "2.16.840.1.101.3.4.2.3" => "sha-512",
        "2.16.840.1.101.3.4.2.4" => "sha-224",
        "2.16.840.1.101.3.4.2.5" => "sha-512/224",
        "2.16.840.1.101.3.4.2.6" => "sha-512/256",
        "2.16.840.1.101.3.4.2.7" => "sha3-224",
        "2.16.840.1.101.3.4.2.8" => "sha3-256",
        "2.16.840.1.101.3.4.2.9" => "sha3-384",
        "2.16.840.1.101.3.4.2.10" => "sha3-512",
        "1.3.14.3.2.26" => "sha-1",
        "1.2.840.113549.2.5" => "md5",
        "1.2.840.113549.2.2" => "md2",
        // PKCS
        "1.2.840.113549.1.7.1" => "data (PKCS#7)",
        "1.2.840.113549.1.7.2" => "signedData (PKCS#7)",
        "1.2.840.113549.1.7.3" => "envelopedData (PKCS#7)",
        "1.2.840.113549.1.7.5" => "digestedData (PKCS#7)",
        "1.2.840.113549.1.7.6" => "encryptedData (PKCS#7)",
        "1.2.840.113549.1.9.1" => "emailAddress (PKCS#9)",
        "1.2.840.113549.1.9.14" => "extensionRequest (PKCS#9)",
        "1.2.840.113549.1.12.10.1.2" => "pkcs-12-pkcs-8ShroudedKeyBag",
        "1.2.840.113549.1.12.10.1.3" => "pkcs-12-certBag",
        // Symmetric ciphers
        "2.16.840.1.101.3.4.1.1" => "aes128-ECB",
        "2.16.840.1.101.3.4.1.2" => "aes128-CBC",
        "2.16.840.1.101.3.4.1.5" => "aes128-CBC-PAD",
        "2.16.840.1.101.3.4.1.21" => "aes192-CBC",
        "2.16.840.1.101.3.4.1.41" => "aes256-CBC",
        "2.16.840.1.101.3.4.1.42" => "aes256-CBC-PAD",
        // EC curves
        "1.2.840.10045.3.1.7" => "prime256v1 (P-256 / secp256r1)",
        "1.3.132.0.34" => "secp384r1 (P-384)",
        "1.3.132.0.35" => "secp521r1 (P-521)",
        "1.3.132.0.10" => "secp256k1 (Bitcoin curve)",
        "1.3.36.3.3.2.8.1.1.7" => "brainpoolP256r1",
        "1.3.36.3.3.2.8.1.1.11" => "brainpoolP384r1",
        "1.3.36.3.3.2.8.1.1.13" => "brainpoolP512r1",
        _ => "",
    }
}

// ── DER/BER tag classes ───────────────────────────────────────────────────────

fn tag_class_name(class: u8) -> &'static str {
    match class {
        0 => "Universal",
        1 => "Application",
        2 => "Context-specific",
        3 => "Private",
        _ => "Unknown",
    }
}

fn universal_tag_name(tag: u64) -> &'static str {
    match tag {
        0 => "EOC",
        1 => "BOOLEAN",
        2 => "INTEGER",
        3 => "BIT STRING",
        4 => "OCTET STRING",
        5 => "NULL",
        6 => "OID",
        7 => "ObjectDescriptor",
        8 => "EXTERNAL",
        9 => "REAL",
        10 => "ENUMERATED",
        11 => "EMBEDDED PDV",
        12 => "UTF8String",
        13 => "RELATIVE-OID",
        14 => "TIME",
        16 => "SEQUENCE",
        17 => "SET",
        18 => "NumericString",
        19 => "PrintableString",
        20 => "T61String",
        21 => "VideotexString",
        22 => "IA5String",
        23 => "UTCTime",
        24 => "GeneralizedTime",
        25 => "GraphicString",
        26 => "VisibleString",
        27 => "GeneralString",
        28 => "UniversalString",
        29 => "CHARACTER STRING",
        30 => "BMPString",
        31 => "DATE",
        _ => "Unknown",
    }
}

// ── DER parser core ───────────────────────────────────────────────────────────

struct Asn1Tag {
    class: u8,
    constructed: bool,
    tag_num: u64,
    offset: usize,
    length: usize,
    header_len: usize,
}

fn read_tag(data: &[u8], pos: usize) -> Option<Asn1Tag> {
    if pos >= data.len() {
        return None;
    }
    let first = data[pos];
    let class = (first >> 6) & 0x03;
    let constructed = (first & 0x20) != 0;
    let mut tag_num = (first & 0x1f) as u64;
    let mut hlen = 1usize;

    if tag_num == 0x1f {
        tag_num = 0;
        loop {
            if pos + hlen >= data.len() {
                return None;
            }
            let b = data[pos + hlen];
            hlen += 1;
            tag_num = (tag_num << 7) | ((b & 0x7f) as u64);
            if b & 0x80 == 0 {
                break;
            }
            if hlen > 8 {
                return None;
            }
        }
    }

    if pos + hlen >= data.len() {
        return None;
    }
    let len_byte = data[pos + hlen];
    hlen += 1;

    let length = if len_byte == 0x80 {
        // Indefinite length — estimate to end of data
        data.len() - (pos + hlen)
    } else if len_byte & 0x80 == 0 {
        len_byte as usize
    } else {
        let num_bytes = (len_byte & 0x7f) as usize;
        if pos + hlen + num_bytes > data.len() {
            return None;
        }
        let mut l = 0usize;
        for i in 0..num_bytes {
            l = (l << 8) | (data[pos + hlen + i] as usize);
        }
        hlen += num_bytes;
        l
    };

    Some(Asn1Tag {
        class,
        constructed,
        tag_num,
        offset: pos,
        length,
        header_len: hlen,
    })
}

fn decode_oid_bytes(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }
    let first = data[0] as u32;
    let mut components = vec![first / 40, first % 40];
    let mut i = 1;
    while i < data.len() {
        let mut val = 0u64;
        loop {
            if i >= data.len() {
                break;
            }
            let b = data[i];
            i += 1;
            val = (val << 7) | ((b & 0x7f) as u64);
            if b & 0x80 == 0 {
                break;
            }
        }
        components.push(val as u32);
    }
    components
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

fn value_summary(tag: &Asn1Tag, data: &[u8]) -> String {
    let start = tag.offset + tag.header_len;
    let end = (start + tag.length).min(data.len());
    let vdata = &data[start..end];

    match (tag.class, tag.tag_num) {
        (0, 1) => {
            // BOOLEAN
            if vdata.is_empty() {
                "false".into()
            } else {
                if vdata[0] != 0 { "true" } else { "false" }.into()
            }
        }
        (0, 2) => {
            // INTEGER
            if vdata.is_empty() {
                "0".into()
            } else if vdata.len() <= 8 {
                let mut val = 0i64;
                let sign_ext = vdata[0] & 0x80 != 0;
                if sign_ext {
                    val = -1;
                }
                for &b in vdata {
                    val = (val << 8) | (b as i64);
                }
                format!("{val}")
            } else {
                let hex: String = vdata[..vdata.len().min(16)]
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                if vdata.len() > 16 {
                    format!("{hex} ... ({} bytes)", vdata.len())
                } else {
                    hex
                }
            }
        }
        (0, 3) => {
            // BIT STRING
            if vdata.is_empty() {
                String::new()
            } else {
                let unused = vdata[0];
                format!("{} bytes ({unused} unused bits)", vdata.len() - 1)
            }
        }
        (0, 4) => {
            // OCTET STRING
            let hex: String = vdata[..vdata.len().min(16)]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            if vdata.len() > 16 {
                format!("{hex} ... ({} bytes)", vdata.len())
            } else {
                hex
            }
        }
        (0, 5) => String::new(), // NULL
        (0, 6) => {
            // OID
            let oid_str = decode_oid_bytes(vdata);
            let name = oid_name(&oid_str);
            if name.is_empty() {
                oid_str
            } else {
                format!("{oid_str}  ({name})")
            }
        }
        (0, 12)
        | (0, 19)
        | (0, 22)
        | (0, 20)
        | (0, 26)
        | (0, 18)
        | (0, 27)
        | (0, 28)
        | (0, 29)
        | (0, 30) => {
            // String types
            let s = String::from_utf8_lossy(vdata);
            if s.len() > 80 {
                format!("{}... ({} chars)", &s[..80], s.len())
            } else {
                s.into_owned()
            }
        }
        (0, 23) | (0, 24) => {
            // UTCTime / GeneralizedTime
            String::from_utf8_lossy(vdata).into_owned()
        }
        _ => {
            if tag.constructed {
                format!("{} bytes (constructed)", tag.length)
            } else {
                format!("{} bytes", tag.length)
            }
        }
    }
}

fn render_tree(
    data: &[u8],
    pos: usize,
    end: usize,
    depth: usize,
    max_depth: usize,
    out: &mut String,
) {
    if depth > max_depth {
        let indent = "  ".repeat(depth);
        out.push_str(&format!(
            "{indent}[...truncated at max depth {max_depth}]\n"
        ));
        return;
    }

    let mut cur = pos;
    while cur < end && cur < data.len() {
        let remaining = end - cur;
        if remaining == 0 {
            break;
        }

        let tag = match read_tag(data, cur) {
            Some(t) => t,
            None => {
                let indent = "  ".repeat(depth);
                out.push_str(&format!("{indent}[parse error at offset {cur}]\n"));
                break;
            }
        };

        let indent = "  ".repeat(depth);

        // Format tag label
        let type_label = if tag.class == 0 {
            universal_tag_name(tag.tag_num).to_string()
        } else {
            let class = tag_class_name(tag.class);
            let cons = if tag.constructed { " CONSTRUCTED" } else { "" };
            format!("[{class} {}{cons}]", tag.tag_num)
        };

        let cons_marker = if tag.constructed { " {" } else { "" };
        let summary = if !tag.constructed {
            let s = value_summary(&tag, data);
            if s.is_empty() {
                String::new()
            } else {
                format!("  = {s}")
            }
        } else {
            String::new()
        };

        out.push_str(&format!(
            "{indent}{type_label}  (len={}, off={}){cons_marker}{summary}\n",
            tag.length, tag.offset
        ));

        if tag.constructed && depth < max_depth {
            let child_start = tag.offset + tag.header_len;
            let child_end = (child_start + tag.length).min(data.len());
            render_tree(data, child_start, child_end, depth + 1, max_depth, out);
            out.push_str(&format!("{indent}}}\n"));
        }

        let next = tag.offset + tag.header_len + tag.length;
        if next <= cur {
            break;
        }
        cur = next;
    }
}

// ── Certificate decoder ───────────────────────────────────────────────────────

fn decode_cert_summary(data: &[u8]) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("X.509 Certificate Summary\n");
    out.push_str("=========================\n\n");

    // Quick structural scan — walk to interesting fields
    // We do a best-effort scan without full DER grammar
    fn collect_strings_at_depth(
        data: &[u8],
        pos: usize,
        end: usize,
        depth: usize,
        target_depth: usize,
        results: &mut Vec<(u64, String)>,
    ) {
        if depth > target_depth + 1 {
            return;
        }
        let mut cur = pos;
        while cur < end && cur < data.len() {
            let tag = match read_tag(data, cur) {
                Some(t) => t,
                None => break,
            };
            if depth == target_depth && tag.class == 0 {
                let start = tag.offset + tag.header_len;
                let end2 = (start + tag.length).min(data.len());
                let vdata = &data[start..end2];
                match tag.tag_num {
                    6 => {
                        // OID
                        let oid_str = decode_oid_bytes(vdata);
                        let name = oid_name(&oid_str);
                        if !name.is_empty() {
                            results.push((tag.tag_num, format!("{oid_str} ({name})")));
                        } else {
                            results.push((tag.tag_num, oid_str));
                        }
                    }
                    12 | 19 | 22 | 20 | 26 | 18 | 27 | 28 | 30 => {
                        results.push((tag.tag_num, String::from_utf8_lossy(vdata).into_owned()));
                    }
                    23 | 24 => {
                        results.push((
                            tag.tag_num,
                            format!("(time) {}", String::from_utf8_lossy(vdata)),
                        ));
                    }
                    2 => {
                        // INTEGER
                        if tag.length > 0 && tag.length <= 20 {
                            let hex: String = vdata
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<Vec<_>>()
                                .join(":");
                            results.push((tag.tag_num, format!("(int) {hex}")));
                        }
                    }
                    _ => {}
                }
            }
            if tag.constructed && depth <= target_depth {
                let child_start = tag.offset + tag.header_len;
                let child_end = (child_start + tag.length).min(data.len());
                collect_strings_at_depth(
                    data,
                    child_start,
                    child_end,
                    depth + 1,
                    target_depth,
                    results,
                );
            }
            let next = tag.offset + tag.header_len + tag.length;
            if next <= cur {
                break;
            }
            cur = next;
        }
    }

    // Scan for OIDs and string values at various depths
    let mut all_items: Vec<(u64, String)> = Vec::new();
    for d in 0..12 {
        collect_strings_at_depth(data, 0, data.len(), 0, d, &mut all_items);
    }

    // Deduplicate and collect interesting fields
    let mut seen = std::collections::HashSet::new();
    let mut times = Vec::new();
    let mut oids = Vec::new();
    let mut strings = Vec::new();

    for (tag_num, val) in &all_items {
        if seen.contains(val) {
            continue;
        }
        seen.insert(val.clone());
        if *tag_num == 23 || *tag_num == 24 {
            times.push(val.clone());
        } else if *tag_num == 6 {
            oids.push(val.clone());
        } else {
            strings.push(val.clone());
        }
    }

    if !oids.is_empty() {
        out.push_str("Algorithms / Extensions detected:\n");
        for o in &oids {
            out.push_str(&format!("  {o}\n"));
        }
        out.push('\n');
    }

    if !strings.is_empty() {
        out.push_str("Distinguished Name fields:\n");
        for s in &strings {
            if !s.starts_with("(time)") && !s.starts_with("(int)") {
                out.push_str(&format!("  {s}\n"));
            }
        }
        out.push('\n');
    }

    if !times.is_empty() {
        out.push_str("Validity:\n");
        if times.len() >= 2 {
            out.push_str(&format!(
                "  Not Before: {}\n",
                times[0].trim_start_matches("(time) ")
            ));
            out.push_str(&format!(
                "  Not After:  {}\n",
                times[1].trim_start_matches("(time) ")
            ));
        } else {
            for t in &times {
                out.push_str(&format!("  {}\n", t.trim_start_matches("(time) ")));
            }
        }
        out.push('\n');
    }

    // Serial number: first short INTEGER under the tbsCertificate SEQUENCE
    for (tag_num, val) in &all_items {
        if *tag_num == 2 && val.starts_with("(int) ") {
            let serial = val.trim_start_matches("(int) ");
            out.push_str(&format!("Serial Number: {serial}\n"));
            break;
        }
    }

    out.push_str(
        "\nNote: For full structured detail use action='parse' to see the complete ASN.1 tree.\n",
    );
    Ok(out)
}

// ── load data ─────────────────────────────────────────────────────────────────

fn load_data(args: &Value) -> Result<Vec<u8>, String> {
    if let Some(hex_val) = args.get("hex").and_then(|v| v.as_str()) {
        let cleaned: String = hex_val.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if cleaned.len() % 2 != 0 {
            return Err("Hex string has odd number of digits".into());
        }
        let bytes: Result<Vec<u8>, _> = (0..cleaned.len() / 2)
            .map(|i| u8::from_str_radix(&cleaned[i * 2..i * 2 + 2], 16))
            .collect();
        bytes.map_err(|e| format!("Invalid hex: {e}"))
    } else if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read(path).map_err(|e| format!("Cannot read file '{path}': {e}"))
    } else {
        Err("Provide 'hex' (hex-encoded DER bytes) or 'file' (path to DER file)".into())
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_parse(data: &[u8], max_depth: usize) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "ASN.1 DER/BER Structure  ({} bytes)\n",
        data.len()
    ));
    out.push_str(&"─".repeat(50));
    out.push('\n');
    render_tree(data, 0, data.len(), 0, max_depth, &mut out);
    out
}

fn action_info(data: &[u8]) -> String {
    let mut out = String::new();
    out.push_str(&format!("Total size: {} bytes\n\n", data.len()));

    let mut cur = 0usize;
    let mut count = 0usize;
    while cur < data.len() && count < 10 {
        let tag = match read_tag(data, cur) {
            Some(t) => t,
            None => break,
        };
        let class = tag_class_name(tag.class);
        let type_label = if tag.class == 0 {
            universal_tag_name(tag.tag_num).to_string()
        } else {
            format!("[{class} {}]", tag.tag_num)
        };
        let cons = if tag.constructed {
            "constructed"
        } else {
            "primitive"
        };
        out.push_str(&format!(
            "Offset {:4}: {type_label}  class={class}  {cons}  length={}\n",
            tag.offset, tag.length
        ));
        let next = tag.offset + tag.header_len + tag.length;
        if next <= cur {
            break;
        }
        cur = next;
        count += 1;
    }
    out
}

fn action_oid(oid_str: &str) -> String {
    let name = oid_name(oid_str);
    if name.is_empty() {
        format!("OID {oid_str}: not in built-in database (200+ OIDs covered)\n")
    } else {
        format!("OID {oid_str}: {name}\n")
    }
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    let max_depth = args.get("max_depth").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

    if action == "oid" {
        let oid_str = args.get("oid").and_then(|v| v.as_str()).unwrap_or("");
        if oid_str.is_empty() {
            return Err("Provide 'oid' field with OID string (e.g. '2.5.4.3')".into());
        }
        return Ok(action_oid(oid_str));
    }

    let data = load_data(args)?;
    if data.is_empty() {
        return Err("Input is empty".into());
    }

    match action {
        "parse" => Ok(action_parse(&data, max_depth)),
        "info" => Ok(action_info(&data)),
        "decode_cert" => decode_cert_summary(&data),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: parse, oid, decode_cert, info"
        )),
    }
}
