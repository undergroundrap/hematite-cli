use serde_json::{json, Value};

pub fn tlv_tools_schema() -> Value {
    json!({
        "name": "tlv_tools",
        "description": "Parse, decode, and build Type-Length-Value (TLV) encoded binary data without external tools. Supports generic TLV with configurable field sizes, ASN.1 BER/DER, DHCP options (RFC 2132), and 802.11 Wi-Fi information elements. Actions: parse (generic TLV), ber (ASN.1 BER/DER), dhcp (DHCP options), wifi (802.11 IEs), build (assemble TLV bytes). Pass 'hex' with raw bytes.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "ber", "dhcp", "wifi", "build"],
                    "description": "parse (generic TLV — configurable type_size/length_size/endian), ber (ASN.1 BER/DER — variable-length tag and length), dhcp (DHCP options with known type names), wifi (802.11 information elements), build (assemble TLV from JSON spec)"
                },
                "hex": {
                    "type": "string",
                    "description": "Hex-encoded raw bytes (spaces and colons ignored)"
                },
                "type_size": {
                    "type": "integer",
                    "description": "Bytes per type/tag field: 1, 2, or 4 (default 1). Only used for 'parse'."
                },
                "length_size": {
                    "type": "integer",
                    "description": "Bytes per length field: 1, 2, or 4 (default 1). Only used for 'parse'."
                },
                "endian": {
                    "type": "string",
                    "description": "Byte order for multi-byte type/length fields: big (default) or little"
                },
                "items": {
                    "type": "array",
                    "description": "For 'build': array of {type, value_hex?, value_string?} objects to assemble into TLV bytes"
                }
            },
            "required": []
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");

    match action {
        "ber" => {
            let hex_raw = args
                .get("hex")
                .and_then(|v| v.as_str())
                .ok_or("Pass 'hex' with raw BER/DER bytes.")?;
            let data = parse_hex(hex_raw)?;
            action_ber(&data)
        }
        "dhcp" => {
            let hex_raw = args
                .get("hex")
                .and_then(|v| v.as_str())
                .ok_or("Pass 'hex' with DHCP options bytes.")?;
            let data = parse_hex(hex_raw)?;
            action_dhcp(&data)
        }
        "wifi" => {
            let hex_raw = args
                .get("hex")
                .and_then(|v| v.as_str())
                .ok_or("Pass 'hex' with 802.11 information elements.")?;
            let data = parse_hex(hex_raw)?;
            action_wifi(&data)
        }
        "build" => {
            let items = args
                .get("items")
                .and_then(|v| v.as_array())
                .ok_or("Pass 'items' as an array of {type, value_hex?, value_string?} objects.")?;
            let type_size = args.get("type_size").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let length_size = args
                .get("length_size")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as usize;
            let big_endian = args
                .get("endian")
                .and_then(|v| v.as_str())
                .map(|s| s != "little")
                .unwrap_or(true);
            action_build(items, type_size, length_size, big_endian)
        }
        _ => {
            // generic parse
            let hex_raw = args
                .get("hex")
                .and_then(|v| v.as_str())
                .ok_or("Pass 'hex' with raw TLV bytes.")?;
            let data = parse_hex(hex_raw)?;
            let type_size = args.get("type_size").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let length_size = args
                .get("length_size")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as usize;
            let big_endian = args
                .get("endian")
                .and_then(|v| v.as_str())
                .map(|s| s != "little")
                .unwrap_or(true);
            action_parse(&data, type_size, length_size, big_endian)
        }
    }
}

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() % 2 != 0 {
        return Err(format!(
            "Odd hex digit count ({}). Provide an even number of hex digits.",
            clean.len()
        ));
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&clean[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex byte at position {}: {}", i, e))
        })
        .collect()
}

fn read_uint(data: &[u8], offset: usize, size: usize, big_endian: bool) -> Option<u64> {
    if offset + size > data.len() {
        return None;
    }
    let slice = &data[offset..offset + size];
    let mut v: u64 = 0;
    if big_endian {
        for &b in slice {
            v = (v << 8) | b as u64;
        }
    } else {
        for (i, &b) in slice.iter().enumerate() {
            v |= (b as u64) << (i * 8);
        }
    }
    Some(v)
}

fn hex_str(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn printable_str(data: &[u8]) -> String {
    data.iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

// ── Generic TLV parse ────────────────────────────────────────────────────────

fn action_parse(
    data: &[u8],
    type_size: usize,
    length_size: usize,
    big_endian: bool,
) -> Result<String, String> {
    if type_size == 0 || type_size > 4 {
        return Err("type_size must be 1, 2, or 4.".into());
    }
    if length_size == 0 || length_size > 4 {
        return Err("length_size must be 1, 2, or 4.".into());
    }

    let endian_label = if big_endian {
        "big-endian"
    } else {
        "little-endian"
    };
    let mut out = format!(
        "TLV Parse — {} bytes, type_size={}, length_size={}, {}\n",
        data.len(),
        type_size,
        length_size,
        endian_label
    );
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut offset = 0;
    let mut record = 0usize;
    let mut errors = 0usize;

    while offset < data.len() {
        let remaining = data.len() - offset;

        if remaining < type_size + length_size {
            out.push_str(&format!(
                "\n[!] Truncated at offset {}: need {} bytes for header, {} remaining\n",
                offset,
                type_size + length_size,
                remaining
            ));
            errors += 1;
            break;
        }

        let tag = read_uint(data, offset, type_size, big_endian).unwrap();
        let length = read_uint(data, offset + type_size, length_size, big_endian).unwrap() as usize;
        let val_offset = offset + type_size + length_size;

        if val_offset + length > data.len() {
            out.push_str(&format!(
                "\n[!] Truncated at offset {}: tag=0x{:x}, length={}, but only {} value bytes remain\n",
                offset,
                tag,
                length,
                data.len().saturating_sub(val_offset)
            ));
            errors += 1;
            break;
        }

        let value = &data[val_offset..val_offset + length];
        record += 1;

        let tag_hex = match type_size {
            1 => format!("0x{:02x}", tag),
            2 => format!("0x{:04x}", tag),
            _ => format!("0x{:08x}", tag),
        };

        out.push_str(&format!("\nRecord #{} @ offset {}:\n", record, offset));
        out.push_str(&format!("  Tag   : {} ({})\n", tag_hex, tag));
        out.push_str(&format!("  Length: {} bytes\n", length));

        if length == 0 {
            out.push_str("  Value : (empty)\n");
        } else {
            let hex_preview = hex_str(&value[..value.len().min(32)]);
            let more = if value.len() > 32 {
                format!(" +{} more bytes", value.len() - 32)
            } else {
                String::new()
            };
            out.push_str(&format!("  Hex   : {}{}\n", hex_preview, more));
            let printable = printable_str(&value[..value.len().min(32)]);
            out.push_str(&format!("  ASCII : {}\n", printable));

            // Try interpreting as a number for small values
            if length <= 8 {
                let n = read_uint(value, 0, length, big_endian).unwrap_or(0);
                out.push_str(&format!("  As int: {} (0x{:x})\n", n, n));
            }
        }

        offset = val_offset + length;
    }

    out.push('\n');
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!("Total records: {}", record));
    if errors > 0 {
        out.push_str(&format!("  Errors: {}", errors));
    }
    out.push('\n');

    Ok(out)
}

// ── ASN.1 BER/DER ───────────────────────────────────────────────────────────

struct BerTag {
    class: u8, // 0=universal, 1=application, 2=context, 3=private
    constructed: bool,
    number: u32,
    byte_len: usize,
}

fn read_ber_tag(data: &[u8], offset: usize) -> Option<BerTag> {
    if offset >= data.len() {
        return None;
    }
    let b0 = data[offset];
    let class = (b0 >> 6) & 0x03;
    let constructed = (b0 & 0x20) != 0;
    let tag_low = b0 & 0x1f;

    if tag_low < 0x1f {
        Some(BerTag {
            class,
            constructed,
            number: tag_low as u32,
            byte_len: 1,
        })
    } else {
        // Long form
        let mut number: u32 = 0;
        let mut i = offset + 1;
        loop {
            if i >= data.len() {
                return None;
            }
            let b = data[i];
            number = (number << 7) | (b & 0x7f) as u32;
            i += 1;
            if (b & 0x80) == 0 {
                break;
            }
            if i - offset > 5 {
                return None;
            } // guard
        }
        Some(BerTag {
            class,
            constructed,
            number,
            byte_len: i - offset,
        })
    }
}

fn read_ber_length(data: &[u8], offset: usize) -> Option<(usize, usize)> {
    if offset >= data.len() {
        return None;
    }
    let b0 = data[offset];
    if b0 == 0x80 {
        // Indefinite form — scan for 0x00 0x00
        let mut i = offset + 1;
        while i + 1 < data.len() {
            if data[i] == 0x00 && data[i + 1] == 0x00 {
                return Some((i - offset - 1, i + 2 - offset));
            }
            i += 1;
        }
        return None;
    }
    if (b0 & 0x80) == 0 {
        Some((b0 as usize, 1))
    } else {
        let n_bytes = (b0 & 0x7f) as usize;
        if n_bytes == 0 || n_bytes > 4 || offset + 1 + n_bytes > data.len() {
            return None;
        }
        let mut length: usize = 0;
        for i in 0..n_bytes {
            length = (length << 8) | data[offset + 1 + i] as usize;
        }
        Some((length, 1 + n_bytes))
    }
}

fn ber_universal_name(tag: u32) -> &'static str {
    match tag {
        0x00 => "EOC",
        0x01 => "BOOLEAN",
        0x02 => "INTEGER",
        0x03 => "BIT STRING",
        0x04 => "OCTET STRING",
        0x05 => "NULL",
        0x06 => "OBJECT IDENTIFIER",
        0x07 => "ObjectDescriptor",
        0x08 => "EXTERNAL",
        0x09 => "REAL",
        0x0A => "ENUMERATED",
        0x0B => "EMBEDDED PDV",
        0x0C => "UTF8String",
        0x0D => "RELATIVE-OID",
        0x10 => "SEQUENCE",
        0x11 => "SET",
        0x12 => "NumericString",
        0x13 => "PrintableString",
        0x14 => "TeletexString",
        0x15 => "VideotexString",
        0x16 => "IA5String",
        0x17 => "UTCTime",
        0x18 => "GeneralizedTime",
        0x19 => "GraphicString",
        0x1A => "VisibleString",
        0x1B => "GeneralString",
        0x1C => "UniversalString",
        0x1D => "CHARACTER STRING",
        0x1E => "BMPString",
        _ => "UNKNOWN",
    }
}

fn ber_class_name(class: u8) -> &'static str {
    match class {
        0 => "UNIVERSAL",
        1 => "APPLICATION",
        2 => "CONTEXT",
        3 => "PRIVATE",
        _ => "UNKNOWN",
    }
}

fn decode_ber_value(tag_number: u32, class: u8, data: &[u8]) -> String {
    if class == 0 {
        match tag_number {
            0x01 => {
                if data.is_empty() {
                    return "(missing)".into();
                }
                return if data[0] == 0x00 {
                    "FALSE".into()
                } else {
                    "TRUE".into()
                };
            }
            0x02 => {
                // INTEGER
                let mut n: i64 = 0;
                let negative = !data.is_empty() && (data[0] & 0x80) != 0;
                for &b in data {
                    n = (n << 8) | b as i64;
                }
                if negative && !data.is_empty() {
                    // sign extend
                    let shift = (8 - data.len() % 8) * 8 % 64;
                    n = (n << shift) >> shift;
                }
                if data.len() <= 8 {
                    return format!("{} (0x{})", n, hex_str(data).replace(' ', ""));
                }
                return format!("(large integer, {} bytes)", data.len());
            }
            0x05 => return "(NULL)".into(),
            0x06 => return decode_oid(data),
            0x0C | 0x12 | 0x13 | 0x14 | 0x16 | 0x1A => {
                return String::from_utf8_lossy(data).to_string();
            }
            0x17 | 0x18 => {
                return String::from_utf8_lossy(data).to_string();
            }
            _ => {}
        }
    }
    // Default: hex preview
    if data.is_empty() {
        "(empty)".into()
    } else {
        let hex = hex_str(&data[..data.len().min(16)]);
        let more = if data.len() > 16 {
            format!(" +{} more", data.len() - 16)
        } else {
            String::new()
        };
        format!("{}{}", hex, more)
    }
}

fn decode_oid(data: &[u8]) -> String {
    if data.is_empty() {
        return "(empty OID)".into();
    }
    let mut parts: Vec<u64> = Vec::new();
    let first = data[0];
    parts.push((first / 40) as u64);
    parts.push((first % 40) as u64);

    let mut i = 1;
    while i < data.len() {
        let mut value: u64 = 0;
        loop {
            if i >= data.len() {
                break;
            }
            let b = data[i];
            i += 1;
            value = (value << 7) | (b & 0x7f) as u64;
            if (b & 0x80) == 0 {
                break;
            }
        }
        parts.push(value);
    }

    parts
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

fn decode_ber_recursive(data: &[u8], depth: usize, out: &mut String, offset_base: usize) {
    let indent = "  ".repeat(depth);
    let mut pos = 0;

    while pos < data.len() {
        let abs_offset = offset_base + pos;

        let tag = match read_ber_tag(data, pos) {
            Some(t) => t,
            None => {
                out.push_str(&format!(
                    "{}[!] Cannot read tag at offset {}\n",
                    indent, abs_offset
                ));
                return;
            }
        };

        let len_offset = pos + tag.byte_len;
        let (length, len_bytes) = match read_ber_length(data, len_offset) {
            Some(pair) => pair,
            None => {
                out.push_str(&format!(
                    "{}[!] Cannot read length at offset {}\n",
                    indent,
                    abs_offset + tag.byte_len
                ));
                return;
            }
        };

        let val_offset = len_offset + len_bytes;
        if val_offset + length > data.len() {
            out.push_str(&format!(
                "{}[!] Truncated: need {} value bytes, {} remain at offset {}\n",
                indent,
                length,
                data.len().saturating_sub(val_offset),
                abs_offset
            ));
            return;
        }

        let value_data = &data[val_offset..val_offset + length];

        // Build tag label
        let tag_label = if tag.class == 0 {
            ber_universal_name(tag.number).to_string()
        } else {
            format!(
                "[{}] {} {}",
                tag.number,
                ber_class_name(tag.class),
                if tag.constructed {
                    "CONSTRUCTED"
                } else {
                    "PRIMITIVE"
                }
            )
        };

        let constructed_marker = if tag.constructed { " {" } else { "" };
        out.push_str(&format!(
            "{}@ +{}: {} (tag=0x{:02x}, len={}){}\n",
            indent, abs_offset, tag_label, tag.number, length, constructed_marker
        ));

        if tag.constructed {
            decode_ber_recursive(value_data, depth + 1, out, offset_base + val_offset);
            out.push_str(&format!("{}}}\n", indent));
        } else {
            let decoded = decode_ber_value(tag.number, tag.class, value_data);
            out.push_str(&format!("{}  Value: {}\n", indent, decoded));
        }

        pos = val_offset + length;
    }
}

fn action_ber(data: &[u8]) -> Result<String, String> {
    let mut out = format!("ASN.1 BER/DER — {} bytes\n", data.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');
    decode_ber_recursive(data, 0, &mut out, 0);
    Ok(out)
}

// ── DHCP Options ─────────────────────────────────────────────────────────────

fn dhcp_option_name(code: u8) -> &'static str {
    match code {
        0 => "Pad",
        1 => "Subnet Mask",
        2 => "Time Offset",
        3 => "Router",
        4 => "Time Server",
        5 => "Name Server",
        6 => "Domain Name Server",
        7 => "Log Server",
        12 => "Host Name",
        15 => "Domain Name",
        23 => "Default IP TTL",
        26 => "Interface MTU",
        28 => "Broadcast Address",
        33 => "Static Route",
        42 => "NTP Servers",
        43 => "Vendor-Specific Info",
        44 => "NetBIOS over TCP/IP Name Server",
        46 => "NetBIOS over TCP/IP Node Type",
        50 => "Requested IP Address",
        51 => "IP Address Lease Time",
        52 => "Option Overload",
        53 => "DHCP Message Type",
        54 => "Server Identifier",
        55 => "Parameter Request List",
        56 => "Message",
        57 => "Maximum DHCP Message Size",
        58 => "Renewal (T1) Time Value",
        59 => "Rebinding (T2) Time Value",
        60 => "Vendor Class Identifier",
        61 => "Client-identifier",
        66 => "TFTP Server Name",
        67 => "Bootfile Name",
        81 => "Client FQDN",
        82 => "Relay Agent Information",
        119 => "Domain Search",
        121 => "Classless Static Route",
        150 => "TFTP Server Address",
        255 => "End",
        _ => "Unknown",
    }
}

fn dhcp_msg_type(code: u8) -> &'static str {
    match code {
        1 => "DHCPDISCOVER",
        2 => "DHCPOFFER",
        3 => "DHCPREQUEST",
        4 => "DHCPDECLINE",
        5 => "DHCPACK",
        6 => "DHCPNAK",
        7 => "DHCPRELEASE",
        8 => "DHCPINFORM",
        _ => "Unknown",
    }
}

fn format_dhcp_value(code: u8, value: &[u8]) -> String {
    match code {
        1 | 28 => {
            // 4-byte IP address
            if value.len() == 4 {
                return format!("{}.{}.{}.{}", value[0], value[1], value[2], value[3]);
            }
        }
        3 | 4 | 5 | 6 | 7 | 42 | 44 => {
            // List of 4-byte IP addresses
            if value.len() % 4 == 0 && !value.is_empty() {
                let ips: Vec<String> = value
                    .chunks(4)
                    .map(|c| format!("{}.{}.{}.{}", c[0], c[1], c[2], c[3]))
                    .collect();
                return ips.join(", ");
            }
        }
        12 | 15 | 56 | 60 | 66 | 67 => {
            // ASCII string
            return String::from_utf8_lossy(value).to_string();
        }
        51 | 58 | 59 => {
            // 4-byte seconds
            if value.len() == 4 {
                let secs = u32::from_be_bytes([value[0], value[1], value[2], value[3]]);
                let h = secs / 3600;
                let m = (secs % 3600) / 60;
                let s = secs % 60;
                return format!("{} seconds ({}h {}m {}s)", secs, h, m, s);
            }
        }
        53 => {
            if value.len() == 1 {
                return format!("{} ({})", value[0], dhcp_msg_type(value[0]));
            }
        }
        54 | 50 => {
            if value.len() == 4 {
                return format!("{}.{}.{}.{}", value[0], value[1], value[2], value[3]);
            }
        }
        57 => {
            if value.len() == 2 {
                let n = u16::from_be_bytes([value[0], value[1]]);
                return format!("{} bytes", n);
            }
        }
        _ => {}
    }
    // Default hex
    if value.is_empty() {
        "(empty)".into()
    } else {
        let hex = hex_str(&value[..value.len().min(20)]);
        if value.len() > 20 {
            format!("{} +{} more bytes", hex, value.len() - 20)
        } else {
            hex
        }
    }
}

fn action_dhcp(data: &[u8]) -> Result<String, String> {
    let mut out = format!("DHCP Options — {} bytes\n", data.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut pos = 0;
    let mut count = 0usize;

    while pos < data.len() {
        let code = data[pos];
        pos += 1;

        if code == 0 {
            // Pad option
            out.push_str("  Option 0: Pad\n");
            count += 1;
            continue;
        }
        if code == 255 {
            out.push_str("  Option 255: End\n");
            count += 1;
            break;
        }

        if pos >= data.len() {
            out.push_str(&format!(
                "  [!] Option {} truncated: missing length byte\n",
                code
            ));
            break;
        }
        let length = data[pos] as usize;
        pos += 1;

        if pos + length > data.len() {
            out.push_str(&format!(
                "  [!] Option {} truncated: need {} value bytes, {} remain\n",
                code,
                length,
                data.len() - pos
            ));
            break;
        }

        let value = &data[pos..pos + length];
        let name = dhcp_option_name(code);
        let formatted = format_dhcp_value(code, value);

        out.push_str(&format!(
            "  Option {:3} — {} (len={})\n",
            code, name, length
        ));
        out.push_str(&format!("    Value: {}\n", formatted));
        pos += length;
        count += 1;
    }

    out.push('\n');
    out.push_str(&"─".repeat(60));
    out.push_str(&format!("\nTotal options: {}\n", count));
    Ok(out)
}

// ── 802.11 Information Elements ──────────────────────────────────────────────

fn wifi_ie_name(id: u8) -> &'static str {
    match id {
        0 => "SSID",
        1 => "Supported Rates",
        2 => "FH Parameter Set",
        3 => "DS Parameter Set (channel)",
        4 => "CF Parameter Set",
        5 => "TIM",
        6 => "IBSS Parameter Set",
        7 => "Country",
        10 => "Request",
        11 => "BSS Load",
        13 => "Challenge Text",
        32 => "Power Constraint",
        33 => "Power Capability",
        35 => "TPC Report",
        36 => "Supported Channels",
        37 => "Channel Switch Announcement",
        39 => "Quiet",
        40 => "IBSS DFS",
        41 => "ERP Information",
        42 => "HT Capabilities",
        45 => "HT Operation",
        46 => "Additional HT Information",
        48 => "RSN (WPA2/WPA3)",
        50 => "Extended Supported Rates",
        52 => "EDCA Parameter Set",
        54 => "Measurement Pilot Transmission",
        59 => "HCCA TXOP Update Count",
        61 => "HT Operation",
        62 => "Secondary Channel Offset",
        70 => "Neighbor Report",
        74 => "OBSS Scan Parameters",
        107 => "Interworking",
        108 => "Advertisement Protocol",
        111 => "QoS Map Set",
        113 => "Mesh Configuration",
        114 => "Mesh ID",
        127 => "Extended Capabilities",
        128 => "AGERE Proprietary",
        133 => "CISCO CCX1 CKIP",
        150 => "CISCO Unknown",
        191 => "VHT Capabilities",
        192 => "VHT Operation",
        195 => "VHT Transmit Power Envelope",
        221 => "Vendor Specific",
        255 => "Extension Element",
        _ => "Unknown",
    }
}

fn format_wifi_ie_value(id: u8, value: &[u8]) -> String {
    match id {
        0 => {
            // SSID
            let ssid = String::from_utf8_lossy(value);
            if ssid.is_empty() {
                return "(hidden SSID)".into();
            }
            format!("\"{}\"", ssid)
        }
        1 | 50 => {
            // Supported / Extended Supported Rates (each byte * 0.5 Mbps, LSB=basic rate)
            let rates: Vec<String> = value
                .iter()
                .map(|&b| {
                    let rate = (b & 0x7f) as f32 * 0.5;
                    let basic = if (b & 0x80) != 0 { "*" } else { "" };
                    format!("{}{}", rate, basic)
                })
                .collect();
            format!("{} Mbps", rates.join(", "))
        }
        3 => {
            if !value.is_empty() {
                return format!("Channel {}", value[0]);
            }
            hex_str(value)
        }
        41 => {
            if !value.is_empty() {
                let erp = value[0];
                let mut flags = Vec::new();
                if erp & 0x01 != 0 {
                    flags.push("NonERP Present");
                }
                if erp & 0x02 != 0 {
                    flags.push("Use Protection");
                }
                if erp & 0x04 != 0 {
                    flags.push("Barker Preamble Mode");
                }
                return if flags.is_empty() {
                    "No ERP flags set".into()
                } else {
                    flags.join(", ")
                };
            }
            hex_str(value)
        }
        48 => {
            // RSN — briefly decode version and cipher suites
            if value.len() >= 2 {
                let version = u16::from_le_bytes([value[0], value[1]]);
                return format!("RSN version={}", version);
            }
            hex_str(value)
        }
        221 => {
            // Vendor Specific — show OUI
            if value.len() >= 3 {
                let oui_label = match &value[..3] {
                    [0x00, 0x50, 0xf2] => "Microsoft (WPA/WMM)",
                    [0x00, 0x0f, 0xac] => "IEEE 802.11i",
                    [0x00, 0x17, 0xf2] => "Apple",
                    [0x8c, 0xfd, 0xf0] => "Qualcomm",
                    _ => "Unknown OUI",
                };
                return format!(
                    "OUI {:02x}:{:02x}:{:02x} ({}), type=0x{:02x}",
                    value[0],
                    value[1],
                    value[2],
                    oui_label,
                    value.get(3).copied().unwrap_or(0)
                );
            }
            hex_str(value)
        }
        _ => {
            if value.is_empty() {
                return "(empty)".into();
            }
            let hex = hex_str(&value[..value.len().min(16)]);
            if value.len() > 16 {
                format!("{} +{} more bytes", hex, value.len() - 16)
            } else {
                hex
            }
        }
    }
}

fn action_wifi(data: &[u8]) -> Result<String, String> {
    let mut out = format!("802.11 Information Elements — {} bytes\n", data.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut pos = 0;
    let mut count = 0usize;

    while pos + 2 <= data.len() {
        let id = data[pos];
        let length = data[pos + 1] as usize;
        pos += 2;

        if pos + length > data.len() {
            out.push_str(&format!(
                "  [!] IE {} truncated: need {} bytes, {} remain\n",
                id,
                length,
                data.len() - pos
            ));
            break;
        }

        let value = &data[pos..pos + length];
        let name = wifi_ie_name(id);
        let formatted = format_wifi_ie_value(id, value);

        out.push_str(&format!("  IE {:3} — {} (len={})\n", id, name, length));
        out.push_str(&format!("    Value: {}\n", formatted));
        pos += length;
        count += 1;
    }

    if pos < data.len() && count > 0 {
        out.push_str(&format!(
            "  ({} trailing bytes not parsed)\n",
            data.len() - pos
        ));
    }

    out.push('\n');
    out.push_str(&"─".repeat(60));
    out.push_str(&format!("\nTotal IEs: {}\n", count));
    Ok(out)
}

// ── Build ────────────────────────────────────────────────────────────────────

fn write_uint(value: u64, size: usize, big_endian: bool, out: &mut Vec<u8>) {
    match size {
        1 => out.push(value as u8),
        2 => {
            let b = (value as u16).to_be_bytes();
            if big_endian {
                out.extend_from_slice(&b);
            } else {
                out.extend_from_slice(&b.iter().rev().cloned().collect::<Vec<_>>());
            }
        }
        4 => {
            let b = (value as u32).to_be_bytes();
            if big_endian {
                out.extend_from_slice(&b);
            } else {
                out.extend_from_slice(&b.iter().rev().cloned().collect::<Vec<_>>());
            }
        }
        _ => out.push(value as u8),
    }
}

fn action_build(
    items: &[Value],
    type_size: usize,
    length_size: usize,
    big_endian: bool,
) -> Result<String, String> {
    let mut assembled: Vec<u8> = Vec::new();
    let mut summary = String::new();

    for (i, item) in items.iter().enumerate() {
        let tag_val = item
            .get("type")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| format!("Item #{}: missing 'type' (integer)", i + 1))?;

        let value_bytes: Vec<u8> = if let Some(hex) = item.get("value_hex").and_then(|v| v.as_str())
        {
            parse_hex(hex)?
        } else if let Some(s) = item.get("value_string").and_then(|v| v.as_str()) {
            s.as_bytes().to_vec()
        } else if let Some(n) = item.get("value_u8").and_then(|v| v.as_u64()) {
            vec![n as u8]
        } else {
            Vec::new()
        };

        let tag_hex = match type_size {
            1 => format!("0x{:02x}", tag_val),
            2 => format!("0x{:04x}", tag_val),
            _ => format!("0x{:08x}", tag_val),
        };
        summary.push_str(&format!(
            "  Item #{}: tag={}, len={} bytes\n",
            i + 1,
            tag_hex,
            value_bytes.len()
        ));

        write_uint(tag_val, type_size, big_endian, &mut assembled);
        write_uint(
            value_bytes.len() as u64,
            length_size,
            big_endian,
            &mut assembled,
        );
        assembled.extend_from_slice(&value_bytes);
    }

    let hex_out = assembled
        .chunks(16)
        .enumerate()
        .map(|(i, chunk)| format!("{:04x}  {}", i * 16, hex_str(chunk)))
        .collect::<Vec<_>>()
        .join("\n");

    let mut out = format!(
        "TLV Build — {} items → {} bytes\n",
        items.len(),
        assembled.len()
    );
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&summary);
    out.push('\n');
    out.push_str("Hex output:\n");
    out.push_str(&hex_out);
    out.push('\n');
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str("Compact hex (no spaces):\n");
    let compact: String = assembled.iter().map(|b| format!("{:02x}", b)).collect();
    out.push_str(&compact);
    out.push('\n');

    Ok(out)
}
