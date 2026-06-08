use serde_json::{json, Value};

pub fn leb128_tools_schema() -> Value {
    json!({
        "name": "leb128_tools",
        "description": "Encode and decode LEB128 (Little-Endian Base-128) variable-length integers without external utilities. LEB128 is used in WebAssembly, DWARF debug info, Android DEX, protobuf (ULEB128 for field tags), and many binary formats. Unsigned LEB128 (ULEB128): encode non-negative integers; each byte's high bit signals continuation. Signed LEB128 (SLEB128): encode signed integers using two's complement. Actions: encode (integer → LEB128 hex bytes), decode (hex bytes → integer value), analyze (inspect each byte of a LEB128 stream), multi (encode/decode an array of values), explain (show bit-level breakdown of encoding).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["encode", "decode", "analyze", "multi", "explain"],
                    "description": "encode (integer → bytes), decode (bytes → integer), analyze (byte-by-byte inspection), multi (batch encode/decode), explain (bit-level breakdown)"
                },
                "value": {
                    "type": "integer",
                    "description": "For 'encode' and 'explain': the integer to encode"
                },
                "hex": {
                    "type": "string",
                    "description": "For 'decode' and 'analyze': hex-encoded LEB128 bytes to decode (spaces stripped)"
                },
                "signed": {
                    "type": "boolean",
                    "description": "Use signed LEB128 (SLEB128) interpretation. Default: false (unsigned ULEB128)"
                },
                "values": {
                    "type": "array",
                    "description": "For 'multi': array of integers to encode, or use with 'hex' for a stream of consecutive LEB128 values"
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
        .unwrap_or("encode");

    let signed = args
        .get("signed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match action {
        "decode" | "analyze" => {
            let hex = args
                .get("hex")
                .and_then(|v| v.as_str())
                .ok_or("Pass 'hex' with LEB128 encoded bytes to decode.")?;
            let data = parse_hex(hex)?;
            if action == "analyze" {
                action_analyze(&data, signed)
            } else {
                action_decode(&data, signed)
            }
        }
        "multi" => action_multi(args, signed),
        "explain" => {
            let val = get_value(args)?;
            action_explain(val, signed)
        }
        _ => {
            // encode
            let val = get_value(args)?;
            action_encode(val, signed)
        }
    }
}

fn get_value(args: &Value) -> Result<i64, String> {
    if let Some(n) = args.get("value").and_then(|v| v.as_i64()) {
        Ok(n)
    } else if let Some(n) = args.get("value").and_then(|v| v.as_u64()) {
        Ok(n as i64)
    } else if let Some(s) = args.get("value").and_then(|v| v.as_str()) {
        s.trim()
            .parse::<i64>()
            .map_err(|_| format!("Cannot parse '{}' as integer", s))
    } else {
        Err("Pass 'value' with the integer to encode.".into())
    }
}

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() % 2 != 0 {
        return Err(format!("Odd hex digit count ({})", clean.len()));
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&clean[i..i + 2], 16).map_err(|e| format!("Bad hex at {}: {}", i, e))
        })
        .collect()
}

// ── ULEB128 encode ────────────────────────────────────────────────────────────

fn encode_uleb128(mut val: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (val & 0x7f) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if val == 0 {
            break;
        }
    }
    out
}

fn decode_uleb128(data: &[u8], start: usize) -> Result<(u64, usize), String> {
    let mut result = 0u64;
    let mut shift = 0u32;
    let mut pos = start;

    loop {
        if pos >= data.len() {
            return Err(format!(
                "Unexpected end of data at byte {} — LEB128 continuation bit set but no more bytes",
                pos
            ));
        }
        let byte = data[pos];
        pos += 1;

        if shift >= 64 {
            return Err("LEB128 value too large for u64 (exceeds 64 bits)".into());
        }

        result |= ((byte & 0x7f) as u64) << shift;
        shift += 7;

        if byte & 0x80 == 0 {
            break;
        }
    }

    Ok((result, pos - start))
}

// ── SLEB128 encode ────────────────────────────────────────────────────────────

fn encode_sleb128(mut val: i64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (val & 0x7f) as u8;
        val >>= 7; // arithmetic right shift
        let done = (val == 0 && byte & 0x40 == 0) || (val == -1 && byte & 0x40 != 0);
        if !done {
            byte |= 0x80;
        }
        out.push(byte);
        if done {
            break;
        }
    }
    out
}

fn decode_sleb128(data: &[u8], start: usize) -> Result<(i64, usize), String> {
    let mut result = 0i64;
    let mut shift = 0u32;
    let mut pos = start;
    let mut last_byte;

    loop {
        if pos >= data.len() {
            return Err(format!(
                "Unexpected end of data at byte {} — LEB128 continuation bit set",
                pos
            ));
        }
        let byte = data[pos];
        last_byte = byte;
        pos += 1;

        if shift >= 64 {
            return Err("SLEB128 value too large for i64 (exceeds 64 bits)".into());
        }

        result |= ((byte & 0x7f) as i64) << shift;
        shift += 7;

        if byte & 0x80 == 0 {
            break;
        }
    }

    // Sign-extend if the sign bit of the last group is set
    if shift < 64 && (last_byte & 0x40) != 0 {
        result |= -(1i64 << shift);
    }

    Ok((result, pos - start))
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_encode(val: i64, signed: bool) -> Result<String, String> {
    let kind = if signed { "SLEB128" } else { "ULEB128" };

    let bytes = if signed {
        encode_sleb128(val)
    } else {
        if val < 0 {
            return Err(format!(
                "Value {} is negative. Use 'signed: true' for signed LEB128 (SLEB128).",
                val
            ));
        }
        encode_uleb128(val as u64)
    };

    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let hex_spaced: String = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ");

    let bits_per_group: Vec<String> = bytes
        .iter()
        .map(|b| {
            format!(
                "[{}|{:07b}]",
                if b & 0x80 != 0 { "1" } else { "0" },
                b & 0x7f
            )
        })
        .collect();

    let mut out = format!("{} Encode\n", kind);
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!("  Input:      {} ({:#x})\n", val, val));
    out.push_str(&format!("  Encoding:   {}\n", kind));
    out.push_str(&format!(
        "  Bytes:      {} ({} byte{})\n",
        hex_spaced,
        bytes.len(),
        if bytes.len() == 1 { "" } else { "s" }
    ));
    out.push_str(&format!("  Compact:    {}\n", hex));
    out.push('\n');
    out.push_str("  Bit layout: [more|7 data bits] per byte\n");
    out.push_str(&format!("  {}\n", bits_per_group.join(" ")));
    if bytes.len() > 1 {
        out.push_str(&format!(
            "               first byte                {}last byte\n",
            " ".repeat((bits_per_group[0].len() + 1) * (bytes.len() - 2).max(0))
        ));
    }

    Ok(out)
}

fn action_decode(data: &[u8], signed: bool) -> Result<String, String> {
    let kind = if signed { "SLEB128" } else { "ULEB128" };

    let (value, consumed) = if signed {
        let (v, n) = decode_sleb128(data, 0)?;
        (v, n)
    } else {
        let (v, n) = decode_uleb128(data, 0)?;
        (v as i64, n)
    };

    let hex: String = data[..consumed]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ");

    let mut out = format!("{} Decode\n", kind);
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!("  Input bytes: {}\n", hex));
    out.push_str(&format!(
        "  Consumed:    {} byte{}\n",
        consumed,
        if consumed == 1 { "" } else { "s" }
    ));
    out.push_str(&format!("  Value:       {} ({:#x})\n", value, value));
    if !signed {
        out.push_str(&format!("  As unsigned: {}\n", value as u64));
    }

    if consumed < data.len() {
        let remaining: String = data[consumed..]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        out.push('\n');
        out.push_str(&format!(
            "  Remaining {} byte{} not consumed: {}\n",
            data.len() - consumed,
            if data.len() - consumed == 1 { "" } else { "s" },
            remaining
        ));

        // Try to decode subsequent values
        out.push('\n');
        out.push_str("  Additional values in stream:\n");
        let mut pos = consumed;
        let mut idx = 2;
        while pos < data.len() && idx <= 10 {
            let result = if signed {
                decode_sleb128(data, pos)
            } else {
                decode_uleb128(data, pos).map(|(v, n)| (v as i64, n))
            };
            match result {
                Ok((v, n)) => {
                    let seg: String = data[pos..pos + n]
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(" ");
                    out.push_str(&format!(
                        "    Value {}: {} ({:#x}) — {} byte{} [{}]\n",
                        idx,
                        v,
                        v,
                        n,
                        if n == 1 { "" } else { "s" },
                        seg
                    ));
                    pos += n;
                    idx += 1;
                }
                Err(e) => {
                    out.push_str(&format!("    (decode error at offset {}: {})\n", pos, e));
                    break;
                }
            }
        }
    }

    Ok(out)
}

fn action_analyze(data: &[u8], signed: bool) -> Result<String, String> {
    let kind = if signed { "SLEB128" } else { "ULEB128" };

    let mut out = format!("{} Byte Analysis\n", kind);
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "  {:3}  {:6}  {:8}  {:8}  {}\n",
        "#", "Offset", "Hex", "Bin", "Role"
    ));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut pos = 0usize;
    let mut value_idx = 1usize;

    while pos < data.len() {
        // Figure out where this LEB128 value ends
        let start = pos;
        let result = if signed {
            decode_sleb128(data, start)
        } else {
            decode_uleb128(data, start).map(|(v, n)| (v as i64, n))
        };

        match result {
            Ok((val, len)) => {
                for (i, &b) in data[start..start + len].iter().enumerate() {
                    let cont = b & 0x80 != 0;
                    let data_bits = b & 0x7f;
                    let role = if len == 1 {
                        format!("Value {} = {} ({:#x})", value_idx, val, val)
                    } else if i == 0 {
                        format!(
                            "Value {} start, {} bytes total = {} ({:#x})",
                            value_idx, len, val, val
                        )
                    } else if i == len - 1 {
                        format!("Value {} end", value_idx)
                    } else {
                        format!("Value {} byte {}/{}", value_idx, i + 1, len)
                    };
                    out.push_str(&format!(
                        "  {:3}  {:6}  {:08b}  {:08b}  {} [more={} data={:07b}]\n",
                        pos + i + 1,
                        pos + i,
                        b,
                        b,
                        role,
                        if cont { "1" } else { "0" },
                        data_bits
                    ));
                }
                pos += len;
                value_idx += 1;
            }
            Err(e) => {
                out.push_str(&format!("  Error at offset {}: {}\n", pos, e));
                break;
            }
        }
    }

    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "  {} byte{}, {} value{}\n",
        data.len(),
        if data.len() == 1 { "" } else { "s" },
        value_idx - 1,
        if value_idx - 1 == 1 { "" } else { "s" }
    ));

    Ok(out)
}

fn action_multi(args: &Value, signed: bool) -> Result<String, String> {
    let kind = if signed { "SLEB128" } else { "ULEB128" };

    // If 'hex' is provided, decode a stream of consecutive values
    if let Some(hex) = args.get("hex").and_then(|v| v.as_str()) {
        let data = parse_hex(hex)?;
        let mut out = format!("{} Multi-Decode — {} bytes\n", kind, data.len());
        out.push_str(&"─".repeat(50));
        out.push('\n');
        out.push_str(&format!(
            "  {:4}  {:12}  {:10}  {}\n",
            "Idx", "Value", "Hex", "Bytes"
        ));
        out.push_str(&"─".repeat(50));
        out.push('\n');

        let mut pos = 0usize;
        let mut idx = 1usize;
        while pos < data.len() {
            let result = if signed {
                decode_sleb128(&data, pos)
            } else {
                decode_uleb128(&data, pos).map(|(v, n)| (v as i64, n))
            };
            match result {
                Ok((val, n)) => {
                    let seg: String = data[pos..pos + n]
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(" ");
                    out.push_str(&format!(
                        "  {:4}  {:12}  {:#010x}  {}\n",
                        idx, val, val, seg
                    ));
                    pos += n;
                    idx += 1;
                }
                Err(e) => {
                    out.push_str(&format!("  Error at offset {}: {}\n", pos, e));
                    break;
                }
            }
        }
        return Ok(out);
    }

    // Otherwise encode an array of values
    let values = args
        .get("values")
        .and_then(|v| v.as_array())
        .ok_or("Pass 'values' as an array of integers to encode, or 'hex' to decode a stream.")?;

    let mut out = format!("{} Multi-Encode — {} values\n", kind, values.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "  {:4}  {:16}  {:6}  {}\n",
        "Idx", "Value", "Bytes", "Hex"
    ));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut all_bytes: Vec<u8> = Vec::new();

    for (i, v) in values.iter().enumerate() {
        let n = if let Some(n) = v.as_i64() {
            n
        } else if let Some(n) = v.as_u64() {
            n as i64
        } else {
            return Err(format!("Value at index {} is not an integer", i));
        };

        let bytes = if signed {
            encode_sleb128(n)
        } else {
            if n < 0 {
                return Err(format!(
                    "Value {} at index {} is negative. Use 'signed: true'.",
                    n, i
                ));
            }
            encode_uleb128(n as u64)
        };

        let hex: String = bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "  {:4}  {:16}  {:6}  {}\n",
            i + 1,
            n,
            bytes.len(),
            hex
        ));
        all_bytes.extend_from_slice(&bytes);
    }

    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "  Total: {} byte{}\n",
        all_bytes.len(),
        if all_bytes.len() == 1 { "" } else { "s" }
    ));
    let stream: String = all_bytes.iter().map(|b| format!("{:02x}", b)).collect();
    out.push_str(&format!("  Stream: {}\n", stream));

    Ok(out)
}

fn action_explain(val: i64, signed: bool) -> Result<String, String> {
    let kind = if signed { "SLEB128" } else { "ULEB128" };

    let bytes = if signed {
        encode_sleb128(val)
    } else {
        if val < 0 {
            return Err(format!("Value {} is negative. Use 'signed: true'.", val));
        }
        encode_uleb128(val as u64)
    };

    let mut out = format!("{} Bit-Level Explanation\n", kind);
    out.push_str(&"─".repeat(70));
    out.push('\n');
    out.push_str(&format!("  Value:   {} ({:#x})\n", val, val));
    out.push_str(&format!("  Decimal binary: {:b}\n", val as u64));
    out.push('\n');
    out.push_str(&format!(
        "  Encoded as {} byte{}:\n",
        bytes.len(),
        if bytes.len() == 1 { "" } else { "s" }
    ));
    out.push('\n');

    // Reconstruct the 7-bit groups (LSB first for ULEB128)
    let mut groups: Vec<u8> = Vec::new();
    if signed {
        let mut v = val;
        loop {
            groups.push((v & 0x7f) as u8);
            v >>= 7;
            if (v == 0 && groups.last().unwrap() & 0x40 == 0)
                || (v == -1 && groups.last().unwrap() & 0x40 != 0)
            {
                break;
            }
        }
    } else {
        let mut v = val as u64;
        loop {
            groups.push((v & 0x7f) as u8);
            v >>= 7;
            if v == 0 {
                break;
            }
        }
    }

    for (i, (&byte, &group)) in bytes.iter().zip(groups.iter()).enumerate() {
        let is_last = i == bytes.len() - 1;
        let more_bit = if is_last { 0u8 } else { 1u8 };
        out.push_str(&format!(
            "  Byte {:2}: {:08b}  →  more={} | data={:07b}  (bits {}-{} of value)\n",
            i + 1,
            byte,
            more_bit,
            group,
            i * 7,
            i * 7 + 6
        ));
    }

    out.push('\n');
    out.push_str("  How to read:\n");
    out.push_str("    - Bit 7 (MSB) = 'more' flag: 1 = more bytes follow, 0 = last byte\n");
    out.push_str("    - Bits 6-0 = 7 data bits (LSB first across bytes)\n");
    if signed {
        out.push_str("    - SLEB128: sign bit is bit 6 of the last byte — if set and no more bytes, sign-extend\n");
    }

    Ok(out)
}
