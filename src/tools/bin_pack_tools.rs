use serde_json::{json, Value};

pub fn bin_pack_tools_schema() -> Value {
    json!({
        "name": "bin_pack_tools",
        "description": "Pack and unpack binary data using struct-style format strings, similar to Python's struct module. Encode named fields into binary bytes or decode raw hex bytes back to named values. Format characters: b/B (int8/uint8), h/H (int16/uint16), i/I (int32/uint32), q/Q (int64/uint64), f (float32), d (float64), s (length-prefixed UTF-8 string), x (pad byte). Prefix with < (little-endian) or > (big-endian, default). Actions: pack (fields → hex bytes), unpack (hex bytes → field values), describe (explain a format string), size (total byte size of format).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["pack", "unpack", "describe", "size"],
                    "description": "pack (values → hex bytes), unpack (hex bytes → values), describe (explain format), size (byte size of format)"
                },
                "format": {
                    "type": "string",
                    "description": "Format string. Optional endian prefix < (little) or > (big, default). Then field specifiers: b/B (int8/uint8), h/H (int16/uint16), i/I (int32/uint32), q/Q (int64/uint64), f (float32), d (float64), s (length-prefixed UTF-8 string), x (pad byte). Repeat count prefix allowed: '4B' = four uint8 fields."
                },
                "values": {
                    "type": "array",
                    "description": "For 'pack': ordered array of values matching the format fields (integers, floats, or strings for 's')"
                },
                "hex": {
                    "type": "string",
                    "description": "For 'unpack': hex-encoded raw bytes to decode (spaces and colons ignored)"
                },
                "names": {
                    "type": "array",
                    "description": "Optional field names — if provided, output maps names to values instead of using positional indices"
                }
            },
            "required": ["format"]
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .ok_or("Pass 'format' with a struct format string.")?;

    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("describe");

    let fields = parse_format(format)?;

    match action {
        "pack" => {
            let values = args
                .get("values")
                .and_then(|v| v.as_array())
                .ok_or("Pass 'values' as an array of values to pack.")?;
            let names = get_names(args);
            action_pack(&fields, values, &names)
        }
        "unpack" => {
            let hex_raw = args
                .get("hex")
                .and_then(|v| v.as_str())
                .ok_or("Pass 'hex' with raw bytes to unpack.")?;
            let data = parse_hex(hex_raw)?;
            let names = get_names(args);
            action_unpack(&fields, &data, &names)
        }
        "size" => action_size(&fields),
        _ => action_describe(&fields, format),
    }
}

fn get_names(args: &Value) -> Vec<String> {
    args.get("names")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if !clean.len().is_multiple_of(2) {
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

#[derive(Debug, Clone, PartialEq)]
enum FieldType {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Str, // length-prefixed UTF-8: u32 length then bytes
    Pad, // skip byte, no value
}

#[derive(Debug, Clone)]
struct Field {
    ftype: FieldType,
    big_endian: bool, // inherited from format-level endian
}

impl Field {
    fn byte_size(&self) -> Option<usize> {
        match self.ftype {
            FieldType::I8 | FieldType::U8 | FieldType::Pad => Some(1),
            FieldType::I16 | FieldType::U16 => Some(2),
            FieldType::I32 | FieldType::U32 | FieldType::F32 => Some(4),
            FieldType::I64 | FieldType::U64 | FieldType::F64 => Some(8),
            FieldType::Str => None, // variable
        }
    }

    fn type_name(&self) -> &'static str {
        match self.ftype {
            FieldType::I8 => "int8",
            FieldType::U8 => "uint8",
            FieldType::I16 => "int16",
            FieldType::U16 => "uint16",
            FieldType::I32 => "int32",
            FieldType::U32 => "uint32",
            FieldType::I64 => "int64",
            FieldType::U64 => "uint64",
            FieldType::F32 => "float32",
            FieldType::F64 => "float64",
            FieldType::Str => "string (u32 length + utf-8 bytes)",
            FieldType::Pad => "pad (skip byte)",
        }
    }
}

fn parse_format(fmt: &str) -> Result<Vec<Field>, String> {
    let mut chars = fmt.chars().peekable();
    let mut big_endian = true;

    if let Some(&c) = chars.peek() {
        if c == '<' {
            big_endian = false;
            chars.next();
        } else if c == '>' {
            big_endian = true;
            chars.next();
        }
    }

    let mut fields: Vec<Field> = Vec::new();

    while let Some(c) = chars.next() {
        // Check for a repeat count prefix
        if c.is_ascii_digit() {
            let mut count_str = c.to_string();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    count_str.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            let count: usize = count_str
                .parse()
                .map_err(|_| format!("Invalid repeat count: {}", count_str))?;
            if count == 0 {
                return Err("Repeat count cannot be 0.".into());
            }
            let specifier = chars.next().ok_or("Format ended after repeat count.")?;
            let ftype = char_to_field_type(specifier)?;
            for _ in 0..count {
                fields.push(Field {
                    ftype: ftype.clone(),
                    big_endian,
                });
            }
        } else {
            let ftype = char_to_field_type(c)?;
            fields.push(Field { ftype, big_endian });
        }
    }

    Ok(fields)
}

fn char_to_field_type(c: char) -> Result<FieldType, String> {
    match c {
        'b' => Ok(FieldType::I8),
        'B' => Ok(FieldType::U8),
        'h' => Ok(FieldType::I16),
        'H' => Ok(FieldType::U16),
        'i' => Ok(FieldType::I32),
        'I' => Ok(FieldType::U32),
        'q' => Ok(FieldType::I64),
        'Q' => Ok(FieldType::U64),
        'f' => Ok(FieldType::F32),
        'd' => Ok(FieldType::F64),
        's' => Ok(FieldType::Str),
        'x' => Ok(FieldType::Pad),
        _ => Err(format!(
            "Unknown format character '{}'. Valid: b B h H i I q Q f d s x",
            c
        )),
    }
}

// ── pack ─────────────────────────────────────────────────────────────────────

fn action_pack(fields: &[Field], values: &[Value], names: &[String]) -> Result<String, String> {
    // Count non-pad fields — those need a value
    let value_fields: Vec<&Field> = fields
        .iter()
        .filter(|f| f.ftype != FieldType::Pad)
        .collect();

    if values.len() != value_fields.len() {
        return Err(format!(
            "Format has {} value fields but {} values provided.",
            value_fields.len(),
            values.len()
        ));
    }

    let mut bytes: Vec<u8> = Vec::new();
    let mut val_idx = 0usize;
    let mut summary_lines: Vec<String> = Vec::new();

    for (i, field) in fields.iter().enumerate() {
        let offset = bytes.len();

        if field.ftype == FieldType::Pad {
            bytes.push(0x00);
            summary_lines.push(format!(
                "  Field {:2}: [pad]          offset={:4}  0x00",
                i + 1,
                offset
            ));
            continue;
        }

        let name = names
            .get(val_idx)
            .cloned()
            .unwrap_or_else(|| format!("field_{}", val_idx + 1));
        let val = &values[val_idx];
        val_idx += 1;

        match &field.ftype {
            FieldType::I8 => {
                let n = val_to_i64(val, "int8")?;
                if !(-128..=127).contains(&n) {
                    return Err(format!("{}: value {} out of int8 range", name, n));
                }
                bytes.push(n as i8 as u8);
                summary_lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {:02x}  ({})",
                    i + 1,
                    name,
                    offset,
                    n as i8 as u8,
                    n
                ));
            }
            FieldType::U8 => {
                let n = val_to_u64(val, "uint8")?;
                if n > 255 {
                    return Err(format!("{}: value {} out of uint8 range", name, n));
                }
                bytes.push(n as u8);
                summary_lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {:02x}  ({})",
                    i + 1,
                    name,
                    offset,
                    n as u8,
                    n
                ));
            }
            FieldType::I16 => {
                let n = val_to_i64(val, "int16")?;
                if !(-32768..=32767).contains(&n) {
                    return Err(format!("{}: value {} out of int16 range", name, n));
                }
                let b = if field.big_endian {
                    (n as i16).to_be_bytes()
                } else {
                    (n as i16).to_le_bytes()
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                summary_lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  ({})",
                    i + 1,
                    name,
                    offset,
                    hex,
                    n
                ));
                bytes.extend_from_slice(&b);
            }
            FieldType::U16 => {
                let n = val_to_u64(val, "uint16")?;
                if n > 65535 {
                    return Err(format!("{}: value {} out of uint16 range", name, n));
                }
                let b = if field.big_endian {
                    (n as u16).to_be_bytes()
                } else {
                    (n as u16).to_le_bytes()
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                summary_lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  ({})",
                    i + 1,
                    name,
                    offset,
                    hex,
                    n
                ));
                bytes.extend_from_slice(&b);
            }
            FieldType::I32 => {
                let n = val_to_i64(val, "int32")?;
                if n < i32::MIN as i64 || n > i32::MAX as i64 {
                    return Err(format!("{}: value {} out of int32 range", name, n));
                }
                let b = if field.big_endian {
                    (n as i32).to_be_bytes()
                } else {
                    (n as i32).to_le_bytes()
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                summary_lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  ({})",
                    i + 1,
                    name,
                    offset,
                    hex,
                    n
                ));
                bytes.extend_from_slice(&b);
            }
            FieldType::U32 => {
                let n = val_to_u64(val, "uint32")?;
                if n > u32::MAX as u64 {
                    return Err(format!("{}: value {} out of uint32 range", name, n));
                }
                let b = if field.big_endian {
                    (n as u32).to_be_bytes()
                } else {
                    (n as u32).to_le_bytes()
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                summary_lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  ({})",
                    i + 1,
                    name,
                    offset,
                    hex,
                    n
                ));
                bytes.extend_from_slice(&b);
            }
            FieldType::I64 => {
                let n = val_to_i64(val, "int64")?;
                let b = if field.big_endian {
                    n.to_be_bytes()
                } else {
                    n.to_le_bytes()
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                summary_lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  ({})",
                    i + 1,
                    name,
                    offset,
                    hex,
                    n
                ));
                bytes.extend_from_slice(&b);
            }
            FieldType::U64 => {
                let n = val_to_u64(val, "uint64")?;
                let b = if field.big_endian {
                    n.to_be_bytes()
                } else {
                    n.to_le_bytes()
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                summary_lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  ({})",
                    i + 1,
                    name,
                    offset,
                    hex,
                    n
                ));
                bytes.extend_from_slice(&b);
            }
            FieldType::F32 => {
                let f = val_to_f64(val, "float32")? as f32;
                let b = if field.big_endian {
                    f.to_be_bytes()
                } else {
                    f.to_le_bytes()
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                summary_lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  ({})",
                    i + 1,
                    name,
                    offset,
                    hex,
                    f
                ));
                bytes.extend_from_slice(&b);
            }
            FieldType::F64 => {
                let f = val_to_f64(val, "float64")?;
                let b = if field.big_endian {
                    f.to_be_bytes()
                } else {
                    f.to_le_bytes()
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                summary_lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  ({})",
                    i + 1,
                    name,
                    offset,
                    hex,
                    f
                ));
                bytes.extend_from_slice(&b);
            }
            FieldType::Str => {
                let s = val
                    .as_str()
                    .ok_or_else(|| format!("{}: expected string value for 's' field", name))?;
                let str_bytes = s.as_bytes();
                let len = str_bytes.len() as u32;
                let len_b = if field.big_endian {
                    len.to_be_bytes()
                } else {
                    len.to_le_bytes()
                };
                bytes.extend_from_slice(&len_b);
                bytes.extend_from_slice(str_bytes);
                summary_lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  [u32 len={}] + {} bytes  (\"{}\")",
                    i + 1,
                    name,
                    offset,
                    len,
                    str_bytes.len(),
                    s
                ));
            }
            FieldType::Pad => unreachable!(),
        }
    }

    let hex_out: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let hex_spaced: String = bytes
        .chunks(8)
        .enumerate()
        .map(|(i, chunk)| {
            format!(
                "{:04x}  {}",
                i * 8,
                chunk
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut out = format!(
        "Pack — format='{}', {} bytes total\n",
        trim_fmt(fields),
        bytes.len()
    );
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&summary_lines.join("\n"));
    out.push('\n');
    out.push('\n');
    out.push_str(&hex_spaced);
    out.push('\n');
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str("Compact: ");
    out.push_str(&hex_out);
    out.push('\n');

    Ok(out)
}

// ── unpack ───────────────────────────────────────────────────────────────────

fn action_unpack(fields: &[Field], data: &[u8], names: &[String]) -> Result<String, String> {
    let mut pos = 0usize;
    let mut val_idx = 0usize;
    let mut lines: Vec<String> = Vec::new();
    let endian_label = if fields.first().map(|f| f.big_endian).unwrap_or(true) {
        "big-endian"
    } else {
        "little-endian"
    };

    let mut out = format!(
        "Unpack — {} bytes, format='{}', {}\n",
        data.len(),
        trim_fmt(fields),
        endian_label
    );
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for (i, field) in fields.iter().enumerate() {
        if field.ftype == FieldType::Pad {
            if pos >= data.len() {
                return Err(format!(
                    "Truncated at field {} (pad): need 1 byte, {} remain",
                    i + 1,
                    data.len() - pos
                ));
            }
            lines.push(format!(
                "  Field {:2}: [pad]          offset={:4}  0x{:02x}",
                i + 1,
                pos,
                data[pos]
            ));
            pos += 1;
            continue;
        }

        let name = names
            .get(val_idx)
            .cloned()
            .unwrap_or_else(|| format!("field_{}", val_idx + 1));
        val_idx += 1;

        match &field.ftype {
            FieldType::I8 => {
                check_remaining(data, pos, 1, i + 1, &name)?;
                let v = data[pos] as i8;
                lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {:02x}  = {} (0x{:02x})",
                    i + 1,
                    name,
                    pos,
                    data[pos],
                    v,
                    data[pos]
                ));
                pos += 1;
            }
            FieldType::U8 => {
                check_remaining(data, pos, 1, i + 1, &name)?;
                let v = data[pos];
                lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {:02x}  = {} (0x{:02x})",
                    i + 1,
                    name,
                    pos,
                    v,
                    v,
                    v
                ));
                pos += 1;
            }
            FieldType::I16 => {
                check_remaining(data, pos, 2, i + 1, &name)?;
                let b = [data[pos], data[pos + 1]];
                let v = if field.big_endian {
                    i16::from_be_bytes(b)
                } else {
                    i16::from_le_bytes(b)
                };
                lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {:02x} {:02x}  = {} (0x{:04x})",
                    i + 1,
                    name,
                    pos,
                    b[0],
                    b[1],
                    v,
                    v as u16
                ));
                pos += 2;
            }
            FieldType::U16 => {
                check_remaining(data, pos, 2, i + 1, &name)?;
                let b = [data[pos], data[pos + 1]];
                let v = if field.big_endian {
                    u16::from_be_bytes(b)
                } else {
                    u16::from_le_bytes(b)
                };
                lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {:02x} {:02x}  = {} (0x{:04x})",
                    i + 1,
                    name,
                    pos,
                    b[0],
                    b[1],
                    v,
                    v
                ));
                pos += 2;
            }
            FieldType::I32 => {
                check_remaining(data, pos, 4, i + 1, &name)?;
                let b = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
                let v = if field.big_endian {
                    i32::from_be_bytes(b)
                } else {
                    i32::from_le_bytes(b)
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  = {} (0x{:08x})",
                    i + 1,
                    name,
                    pos,
                    hex,
                    v,
                    v as u32
                ));
                pos += 4;
            }
            FieldType::U32 => {
                check_remaining(data, pos, 4, i + 1, &name)?;
                let b = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
                let v = if field.big_endian {
                    u32::from_be_bytes(b)
                } else {
                    u32::from_le_bytes(b)
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  = {} (0x{:08x})",
                    i + 1,
                    name,
                    pos,
                    hex,
                    v,
                    v
                ));
                pos += 4;
            }
            FieldType::I64 => {
                check_remaining(data, pos, 8, i + 1, &name)?;
                let mut b = [0u8; 8];
                b.copy_from_slice(&data[pos..pos + 8]);
                let v = if field.big_endian {
                    i64::from_be_bytes(b)
                } else {
                    i64::from_le_bytes(b)
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  = {}",
                    i + 1,
                    name,
                    pos,
                    hex,
                    v
                ));
                pos += 8;
            }
            FieldType::U64 => {
                check_remaining(data, pos, 8, i + 1, &name)?;
                let mut b = [0u8; 8];
                b.copy_from_slice(&data[pos..pos + 8]);
                let v = if field.big_endian {
                    u64::from_be_bytes(b)
                } else {
                    u64::from_le_bytes(b)
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  = {}",
                    i + 1,
                    name,
                    pos,
                    hex,
                    v
                ));
                pos += 8;
            }
            FieldType::F32 => {
                check_remaining(data, pos, 4, i + 1, &name)?;
                let b = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
                let v = if field.big_endian {
                    f32::from_be_bytes(b)
                } else {
                    f32::from_le_bytes(b)
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  = {}",
                    i + 1,
                    name,
                    pos,
                    hex,
                    v
                ));
                pos += 4;
            }
            FieldType::F64 => {
                check_remaining(data, pos, 8, i + 1, &name)?;
                let mut b = [0u8; 8];
                b.copy_from_slice(&data[pos..pos + 8]);
                let v = if field.big_endian {
                    f64::from_be_bytes(b)
                } else {
                    f64::from_le_bytes(b)
                };
                let hex: String = b
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(" ");
                lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  {}  = {}",
                    i + 1,
                    name,
                    pos,
                    hex,
                    v
                ));
                pos += 8;
            }
            FieldType::Str => {
                check_remaining(data, pos, 4, i + 1, &name)?;
                let lb = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
                let str_len = if field.big_endian {
                    u32::from_be_bytes(lb)
                } else {
                    u32::from_le_bytes(lb)
                } as usize;
                pos += 4;
                check_remaining(data, pos, str_len, i + 1, &name)?;
                let s = std::str::from_utf8(&data[pos..pos + str_len])
                    .map_err(|e| format!("Field {}: string is not valid UTF-8: {}", name, e))?;
                lines.push(format!(
                    "  Field {:2}: {:<16} offset={:4}  [len={}] = \"{}\"",
                    i + 1,
                    name,
                    pos - 4,
                    str_len,
                    s
                ));
                pos += str_len;
            }
            FieldType::Pad => unreachable!(),
        }
    }

    if pos < data.len() {
        lines.push(format!(
            "  ({} trailing bytes not consumed)\n",
            data.len() - pos
        ));
    }

    out.push_str(&lines.join("\n"));
    out.push('\n');
    Ok(out)
}

fn check_remaining(
    data: &[u8],
    pos: usize,
    need: usize,
    field_num: usize,
    name: &str,
) -> Result<(), String> {
    if pos + need > data.len() {
        Err(format!(
            "Truncated at field {} ({}): need {} bytes, {} remain at offset {}",
            field_num,
            name,
            need,
            data.len().saturating_sub(pos),
            pos
        ))
    } else {
        Ok(())
    }
}

// ── describe ─────────────────────────────────────────────────────────────────

fn action_describe(fields: &[Field], original_fmt: &str) -> Result<String, String> {
    let endian = if fields.first().map(|f| f.big_endian).unwrap_or(true) {
        "big-endian (default)"
    } else {
        "little-endian (<)"
    };

    let mut out = format!("Format: '{}'\nByte order: {}\n", original_fmt, endian);
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut byte_offset = 0usize;
    let mut has_variable = false;

    for (i, field) in fields.iter().enumerate() {
        let size_str = match field.byte_size() {
            Some(n) => {
                let s = format!("{} byte{}", n, if n == 1 { "" } else { "s" });
                byte_offset += n;
                s
            }
            None => {
                has_variable = true;
                "variable (4-byte length + string bytes)".to_string()
            }
        };
        out.push_str(&format!(
            "  Field {:2}: {} — {}\n",
            i + 1,
            field.type_name(),
            size_str
        ));
    }

    out.push_str(&"─".repeat(60));
    out.push('\n');
    let value_count = fields.iter().filter(|f| f.ftype != FieldType::Pad).count();
    out.push_str(&format!("Value fields: {}\n", value_count));
    if !has_variable {
        out.push_str(&format!("Fixed total size: {} bytes\n", byte_offset));
    } else {
        out.push_str(&format!(
            "Minimum fixed size: {} bytes (+ variable string bytes)\n",
            byte_offset
        ));
    }

    Ok(out)
}

// ── size ─────────────────────────────────────────────────────────────────────

fn action_size(fields: &[Field]) -> Result<String, String> {
    let mut fixed = 0usize;
    let mut has_variable = false;

    for field in fields {
        match field.byte_size() {
            Some(n) => fixed += n,
            None => {
                has_variable = true;
                fixed += 4; // minimum: the length prefix
            }
        }
    }

    if has_variable {
        Ok(format!(
            "Minimum size: {} bytes (fixed) + variable string bytes",
            fixed
        ))
    } else {
        Ok(format!("Fixed size: {} bytes", fixed))
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn trim_fmt(fields: &[Field]) -> String {
    let endian_char = if fields.first().map(|f| f.big_endian).unwrap_or(true) {
        ">"
    } else {
        "<"
    };
    let mut s = endian_char.to_string();
    for f in fields {
        let c = match f.ftype {
            FieldType::I8 => 'b',
            FieldType::U8 => 'B',
            FieldType::I16 => 'h',
            FieldType::U16 => 'H',
            FieldType::I32 => 'i',
            FieldType::U32 => 'I',
            FieldType::I64 => 'q',
            FieldType::U64 => 'Q',
            FieldType::F32 => 'f',
            FieldType::F64 => 'd',
            FieldType::Str => 's',
            FieldType::Pad => 'x',
        };
        s.push(c);
    }
    s
}

fn val_to_i64(v: &Value, ctx: &str) -> Result<i64, String> {
    if let Some(n) = v.as_i64() {
        Ok(n)
    } else if let Some(n) = v.as_u64() {
        Ok(n as i64)
    } else if let Some(n) = v.as_f64() {
        Ok(n as i64)
    } else if let Some(s) = v.as_str() {
        s.trim()
            .parse::<i64>()
            .map_err(|_| format!("{}: cannot parse '{}' as integer", ctx, s))
    } else {
        Err(format!("{}: expected integer, got {:?}", ctx, v))
    }
}

fn val_to_u64(v: &Value, ctx: &str) -> Result<u64, String> {
    if let Some(n) = v.as_u64() {
        Ok(n)
    } else if let Some(n) = v.as_i64() {
        if n < 0 {
            return Err(format!(
                "{}: cannot use negative value {} as unsigned",
                ctx, n
            ));
        }
        Ok(n as u64)
    } else if let Some(n) = v.as_f64() {
        Ok(n as u64)
    } else if let Some(s) = v.as_str() {
        s.trim()
            .parse::<u64>()
            .map_err(|_| format!("{}: cannot parse '{}' as unsigned integer", ctx, s))
    } else {
        Err(format!("{}: expected integer, got {:?}", ctx, v))
    }
}

fn val_to_f64(v: &Value, ctx: &str) -> Result<f64, String> {
    if let Some(n) = v.as_f64() {
        Ok(n)
    } else if let Some(n) = v.as_i64() {
        Ok(n as f64)
    } else if let Some(s) = v.as_str() {
        s.trim()
            .parse::<f64>()
            .map_err(|_| format!("{}: cannot parse '{}' as float", ctx, s))
    } else {
        Err(format!("{}: expected number, got {:?}", ctx, v))
    }
}
