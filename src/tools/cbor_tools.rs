use serde_json::{json, Value};

pub fn cbor_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["decode", "info", "annotate"],
                "description": "decode (default — human-readable decoded value), info (type/structure summary), annotate (hex dump with CBOR type annotations). Default: decode."
            },
            "hex": {
                "type": "string",
                "description": "Hex-encoded CBOR bytes to decode."
            },
            "file": {
                "type": "string",
                "description": "Path to a binary file containing CBOR data."
            },
            "base64": {
                "type": "string",
                "description": "Base64-encoded CBOR bytes to decode."
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("decode");

    let data = load_bytes(args)?;

    match action {
        "decode" | "" => action_decode(&data),
        "info" => action_info(&data),
        "annotate" => action_annotate(&data),
        _ => Err(format!(
            "Unknown action '{}'. Use: decode, info, annotate",
            action
        )),
    }
}

// ── Input loading ────────────────────────────────────────────────────────────

fn load_bytes(args: &Value) -> Result<Vec<u8>, String> {
    if let Some(h) = args.get("hex").and_then(|v| v.as_str()) {
        return decode_hex(h);
    }
    if let Some(b) = args.get("base64").and_then(|v| v.as_str()) {
        return decode_base64_str(b);
    }
    if let Some(path) = args
        .get("file")
        .or_else(|| args.get("path"))
        .and_then(|v| v.as_str())
    {
        return std::fs::read(path)
            .map_err(|e| format!("cbor_tools: cannot read '{}': {}", path, e));
    }
    Err("cbor_tools: provide 'hex', 'base64', or 'file'".to_string())
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.len() % 2 != 0 {
        return Err("cbor_tools: hex string must have even length".into());
    }
    (0..s.len() / 2)
        .map(|i| {
            u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|_| format!("cbor_tools: invalid hex at byte {}", i))
        })
        .collect()
}

fn decode_base64_str(s: &str) -> Result<Vec<u8>, String> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    const TABLE: &[u8; 128] = b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\x3e\xff\xff\xff\x3f\
                                \x34\x35\x36\x37\x38\x39\x3a\x3b\x3c\x3d\xff\xff\xff\xfe\xff\xff\
                                \xff\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\
                                \x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\xff\xff\xff\xff\xff\
                                \xff\x1a\x1b\x1c\x1d\x1e\x1f\x20\x21\x22\x23\x24\x25\x26\x27\x28\
                                \x29\x2a\x2b\x2c\x2d\x2e\x2f\x30\x31\x32\x33\xff\xff\xff\xff\xff";
    // support base64url by mapping - and _ to + and /
    let s: String = s
        .chars()
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            c => c,
        })
        .collect();
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf = [0u32; 4];
    let mut i = 0;
    let chars: Vec<u8> = s.bytes().collect();
    while i < chars.len() {
        let mut group_len = 0;
        let mut acc = 0u32;
        for slot in 0..4 {
            if i >= chars.len() {
                break;
            }
            let c = chars[i] as usize;
            i += 1;
            if c >= 128 {
                return Err("cbor_tools: invalid base64 character".into());
            }
            let v = TABLE[c];
            if v == 0xfe {
                break; // padding
            }
            if v == 0xff {
                return Err(format!(
                    "cbor_tools: invalid base64 character '{}'",
                    chars[i - 1] as char
                ));
            }
            buf[slot] = v as u32;
            group_len = slot + 1;
            acc = (acc << 6) | buf[slot];
        }
        let _ = acc;
        // decode via direct byte ops
        if group_len >= 2 {
            out.push(((buf[0] << 2) | (buf[1] >> 4)) as u8);
        }
        if group_len >= 3 {
            out.push(((buf[1] << 4) | (buf[2] >> 2)) as u8);
        }
        if group_len >= 4 {
            out.push(((buf[2] << 6) | buf[3]) as u8);
        }
    }
    Ok(out)
}

// ── CBOR types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum CborValue {
    Uint(u64),
    Int(i64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<CborValue>),
    Map(Vec<(CborValue, CborValue)>),
    Tag(u64, Box<CborValue>),
    Simple(u8),
    Float(f64),
    Bool(bool),
    Null,
    Undefined,
}

struct Decoder<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        Decoder { data, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn read_byte(&mut self) -> Result<u8, String> {
        let b = self
            .data
            .get(self.pos)
            .copied()
            .ok_or_else(|| "cbor_tools: unexpected end of data".to_string())?;
        self.pos += 1;
        Ok(b)
    }

    fn read_n(&mut self, n: usize) -> Result<&[u8], String> {
        if self.pos + n > self.data.len() {
            return Err("cbor_tools: unexpected end of data".into());
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_uint(&mut self, additional: u8) -> Result<u64, String> {
        match additional {
            0..=23 => Ok(additional as u64),
            24 => Ok(self.read_byte()? as u64),
            25 => {
                let b = self.read_n(2)?;
                Ok(u16::from_be_bytes([b[0], b[1]]) as u64)
            }
            26 => {
                let b = self.read_n(4)?;
                Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as u64)
            }
            27 => {
                let b = self.read_n(8)?;
                Ok(u64::from_be_bytes([
                    b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
                ]))
            }
            _ => Err(format!(
                "cbor_tools: reserved additional info {}",
                additional
            )),
        }
    }

    fn decode_value(&mut self, depth: usize) -> Result<CborValue, String> {
        if depth > 64 {
            return Err("cbor_tools: maximum nesting depth (64) exceeded".into());
        }
        let initial = self.read_byte()?;
        let major = (initial >> 5) & 0x07;
        let additional = initial & 0x1f;

        match major {
            0 => {
                let n = self.read_uint(additional)?;
                Ok(CborValue::Uint(n))
            }
            1 => {
                let n = self.read_uint(additional)?;
                Ok(CborValue::Int(-1i64 - n as i64))
            }
            2 => {
                if additional == 31 {
                    // indefinite length byte string
                    let mut bytes = Vec::new();
                    loop {
                        if self.peek() == Some(0xff) {
                            self.pos += 1;
                            break;
                        }
                        let chunk = self.decode_value(depth + 1)?;
                        if let CborValue::Bytes(b) = chunk {
                            bytes.extend_from_slice(&b);
                        } else {
                            return Err(
                                "cbor_tools: expected byte chunk in indefinite byte string".into(),
                            );
                        }
                    }
                    Ok(CborValue::Bytes(bytes))
                } else {
                    let len = self.read_uint(additional)? as usize;
                    let bytes = self.read_n(len)?.to_vec();
                    Ok(CborValue::Bytes(bytes))
                }
            }
            3 => {
                if additional == 31 {
                    let mut text = String::new();
                    loop {
                        if self.peek() == Some(0xff) {
                            self.pos += 1;
                            break;
                        }
                        let chunk = self.decode_value(depth + 1)?;
                        if let CborValue::Text(s) = chunk {
                            text.push_str(&s);
                        } else {
                            return Err(
                                "cbor_tools: expected text chunk in indefinite text string".into(),
                            );
                        }
                    }
                    Ok(CborValue::Text(text))
                } else {
                    let len = self.read_uint(additional)? as usize;
                    let bytes = self.read_n(len)?.to_vec();
                    let s = String::from_utf8_lossy(&bytes).to_string();
                    Ok(CborValue::Text(s))
                }
            }
            4 => {
                if additional == 31 {
                    let mut arr = Vec::new();
                    loop {
                        if self.peek() == Some(0xff) {
                            self.pos += 1;
                            break;
                        }
                        arr.push(self.decode_value(depth + 1)?);
                    }
                    Ok(CborValue::Array(arr))
                } else {
                    let len = self.read_uint(additional)? as usize;
                    let mut arr = Vec::with_capacity(len.min(1024));
                    for _ in 0..len {
                        arr.push(self.decode_value(depth + 1)?);
                    }
                    Ok(CborValue::Array(arr))
                }
            }
            5 => {
                if additional == 31 {
                    let mut map = Vec::new();
                    loop {
                        if self.peek() == Some(0xff) {
                            self.pos += 1;
                            break;
                        }
                        let k = self.decode_value(depth + 1)?;
                        let v = self.decode_value(depth + 1)?;
                        map.push((k, v));
                    }
                    Ok(CborValue::Map(map))
                } else {
                    let len = self.read_uint(additional)? as usize;
                    let mut map = Vec::with_capacity(len.min(256));
                    for _ in 0..len {
                        let k = self.decode_value(depth + 1)?;
                        let v = self.decode_value(depth + 1)?;
                        map.push((k, v));
                    }
                    Ok(CborValue::Map(map))
                }
            }
            6 => {
                let tag_num = self.read_uint(additional)?;
                let tagged = self.decode_value(depth + 1)?;
                Ok(CborValue::Tag(tag_num, Box::new(tagged)))
            }
            7 => match additional {
                20 => Ok(CborValue::Bool(false)),
                21 => Ok(CborValue::Bool(true)),
                22 => Ok(CborValue::Null),
                23 => Ok(CborValue::Undefined),
                24 => {
                    let b = self.read_byte()?;
                    Ok(CborValue::Simple(b))
                }
                25 => {
                    // 16-bit IEEE 754 half float
                    let b = self.read_n(2)?;
                    let bits = u16::from_be_bytes([b[0], b[1]]);
                    Ok(CborValue::Float(half_to_f64(bits)))
                }
                26 => {
                    let b = self.read_n(4)?;
                    let bits = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
                    Ok(CborValue::Float(f32::from_bits(bits) as f64))
                }
                27 => {
                    let b = self.read_n(8)?;
                    let bits = u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
                    Ok(CborValue::Float(f64::from_bits(bits)))
                }
                0..=19 => Ok(CborValue::Simple(additional)),
                _ => Err(format!(
                    "cbor_tools: reserved simple/float value {}",
                    additional
                )),
            },
            _ => unreachable!(),
        }
    }
}

fn half_to_f64(bits: u16) -> f64 {
    let exp = ((bits >> 10) & 0x1f) as i32;
    let mant = (bits & 0x3ff) as u64;
    let sign = if bits >> 15 != 0 { -1.0f64 } else { 1.0f64 };
    match exp {
        0 => sign * (mant as f64) * 2.0f64.powi(-24),
        31 => {
            if mant == 0 {
                sign * f64::INFINITY
            } else {
                f64::NAN
            }
        }
        e => sign * ((mant as f64) / 1024.0 + 1.0) * 2.0f64.powi(e - 15),
    }
}

// ── Tag annotations ───────────────────────────────────────────────────────────

fn tag_name(tag: u64) -> &'static str {
    match tag {
        0 => "datetime(text)",
        1 => "epoch-datetime",
        2 => "positive-bignum",
        3 => "negative-bignum",
        4 => "decimal-fraction",
        5 => "bigfloat",
        16 => "cose-encrypt0",
        17 => "cose-mac0",
        18 => "cose-sign1",
        24 => "cbor-data-item",
        32 => "uri",
        33 => "base64url",
        34 => "base64",
        35 => "regexp",
        36 => "mime-message",
        37 => "uuid",
        55799 => "self-described-cbor",
        _ => "(custom)",
    }
}

// ── Pretty printer ────────────────────────────────────────────────────────────

fn pretty_value(val: &CborValue, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    match val {
        CborValue::Uint(n) => out.push_str(&format!("{}", n)),
        CborValue::Int(n) => out.push_str(&format!("{}", n)),
        CborValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        CborValue::Null => out.push_str("null"),
        CborValue::Undefined => out.push_str("undefined"),
        CborValue::Simple(s) => out.push_str(&format!("simple({})", s)),
        CborValue::Float(f) => {
            if f.is_nan() {
                out.push_str("NaN");
            } else if f.is_infinite() {
                out.push_str(if *f > 0.0 { "Infinity" } else { "-Infinity" });
            } else {
                out.push_str(&format!("{}", f));
            }
        }
        CborValue::Text(s) => out.push_str(&format!("{:?}", s)),
        CborValue::Bytes(b) => {
            if b.len() <= 32 {
                out.push_str(&format!(
                    "h'{}'",
                    b.iter().map(|x| format!("{:02x}", x)).collect::<String>()
                ));
            } else {
                out.push_str(&format!(
                    "h'{}...({} bytes)'",
                    b[..16]
                        .iter()
                        .map(|x| format!("{:02x}", x))
                        .collect::<String>(),
                    b.len()
                ));
            }
        }
        CborValue::Tag(tag, inner) => {
            out.push_str(&format!("{}(", tag_name(*tag)));
            pretty_value(inner, indent, out);
            out.push(')');
        }
        CborValue::Array(arr) => {
            if arr.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for item in arr {
                out.push_str(&format!("{}  ", pad));
                pretty_value(item, indent + 1, out);
                out.push_str(",\n");
            }
            out.push_str(&format!("{}]", pad));
        }
        CborValue::Map(pairs) => {
            if pairs.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (k, v) in pairs {
                out.push_str(&format!("{}  ", pad));
                pretty_value(k, indent + 1, out);
                out.push_str(": ");
                pretty_value(v, indent + 1, out);
                out.push_str(",\n");
            }
            out.push_str(&format!("{}}}", pad));
        }
    }
}

// ── action: decode ────────────────────────────────────────────────────────────

fn action_decode(data: &[u8]) -> Result<String, String> {
    let mut decoder = Decoder::new(data);
    let val = decoder.decode_value(0)?;
    let remaining = data.len() - decoder.pos;

    let mut out = String::new();
    out.push_str("CBOR Decoded\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    pretty_value(&val, 0, &mut out);
    out.push('\n');

    if remaining > 0 {
        out.push_str(&format!("\n({} trailing byte(s) not decoded)\n", remaining));
    }

    // Known-format hints
    let hint = format_hint(&val);
    if !hint.is_empty() {
        out.push('\n');
        out.push_str(&hint);
    }

    Ok(out)
}

fn format_hint(val: &CborValue) -> String {
    match val {
        CborValue::Map(pairs) => {
            // WebAuthn AttestationObject keys: "fmt", "authData", "attStmt"
            let keys: Vec<&str> = pairs
                .iter()
                .filter_map(|(k, _)| {
                    if let CborValue::Text(s) = k {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .collect();
            if keys.contains(&"fmt") && keys.contains(&"authData") {
                return "Likely WebAuthn AttestationObject (fmt, authData, attStmt detected)\n"
                    .to_string();
            }
            if keys.contains(&"1") || keys.iter().any(|k| matches!(*k, "alg" | "kty")) {
                return "May be COSE Key or JWT CBOR structure\n".to_string();
            }
        }
        CborValue::Tag(55799, _) => {
            return "Self-Described CBOR (tag 55799)\n".to_string();
        }
        _ => {}
    }
    String::new()
}

// ── action: info ──────────────────────────────────────────────────────────────

fn cbor_type_name(val: &CborValue) -> &'static str {
    match val {
        CborValue::Uint(_) => "uint",
        CborValue::Int(_) => "nint",
        CborValue::Bytes(_) => "bytes",
        CborValue::Text(_) => "tstr",
        CborValue::Array(_) => "array",
        CborValue::Map(_) => "map",
        CborValue::Tag(_, _) => "tagged",
        CborValue::Bool(_) => "bool",
        CborValue::Null => "null",
        CborValue::Undefined => "undefined",
        CborValue::Simple(_) => "simple",
        CborValue::Float(_) => "float",
    }
}

fn count_items(val: &CborValue, stats: &mut std::collections::HashMap<String, usize>) {
    let key = cbor_type_name(val).to_string();
    *stats.entry(key).or_insert(0) += 1;
    match val {
        CborValue::Array(arr) => {
            for item in arr {
                count_items(item, stats);
            }
        }
        CborValue::Map(pairs) => {
            for (k, v) in pairs {
                count_items(k, stats);
                count_items(v, stats);
            }
        }
        CborValue::Tag(_, inner) => count_items(inner, stats),
        _ => {}
    }
}

fn action_info(data: &[u8]) -> Result<String, String> {
    let mut decoder = Decoder::new(data);
    let val = decoder.decode_value(0)?;
    let remaining = data.len() - decoder.pos;

    let mut stats = std::collections::HashMap::new();
    count_items(&val, &mut stats);

    let mut out = String::new();
    out.push_str("CBOR Info\n");
    out.push_str(&"─".repeat(40));
    out.push('\n');
    out.push_str(&format!("Total bytes:  {}\n", data.len()));
    out.push_str(&format!("Root type:    {}\n", cbor_type_name(&val)));

    match &val {
        CborValue::Array(arr) => {
            out.push_str(&format!("Array length: {}\n", arr.len()));
        }
        CborValue::Map(pairs) => {
            out.push_str(&format!("Map entries:  {}\n", pairs.len()));
            let text_keys: Vec<&str> = pairs
                .iter()
                .filter_map(|(k, _)| {
                    if let CborValue::Text(s) = k {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .collect();
            if !text_keys.is_empty() {
                out.push_str(&format!("Keys:         {}\n", text_keys.join(", ")));
            }
        }
        CborValue::Tag(tag, _) => {
            out.push_str(&format!("Tag:          {} ({})\n", tag, tag_name(*tag)));
        }
        _ => {}
    }

    if remaining > 0 {
        out.push_str(&format!("Trailing:     {} byte(s)\n", remaining));
    }

    out.push_str("\nType distribution:\n");
    let mut type_counts: Vec<(String, usize)> = stats.into_iter().collect();
    type_counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (t, n) in &type_counts {
        out.push_str(&format!("  {:<12} {}\n", t, n));
    }

    Ok(out)
}

// ── action: annotate ──────────────────────────────────────────────────────────

fn major_type_label(initial: u8) -> (&'static str, u8) {
    let major = (initial >> 5) & 0x07;
    let additional = initial & 0x1f;
    let name = match major {
        0 => "uint",
        1 => "nint",
        2 => "bytes",
        3 => "text",
        4 => "array",
        5 => "map",
        6 => "tag",
        7 => match additional {
            20 => "false",
            21 => "true",
            22 => "null",
            23 => "undefined",
            24 => "simple",
            25 => "float16",
            26 => "float32",
            27 => "float64",
            31 => "break",
            _ => "simple",
        },
        _ => "?",
    };
    (name, additional)
}

fn action_annotate(data: &[u8]) -> Result<String, String> {
    let limit = data.len().min(256);
    let mut out = String::new();
    out.push_str("CBOR Hex Annotated\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "{:<6} {:<4} {:<10} {}\n",
        "Offset", "Hex", "Type", "Info"
    ));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut pos = 0;
    while pos < limit {
        let byte = data[pos];
        let (type_label, additional) = major_type_label(byte);
        let info = match (byte >> 5) & 0x07 {
            0 => {
                if additional <= 23 {
                    format!("value = {}", additional)
                } else {
                    format!("additional = {}", additional)
                }
            }
            4 | 5 => {
                if additional == 31 {
                    "indefinite length".to_string()
                } else if additional <= 23 {
                    format!("length = {}", additional)
                } else {
                    format!("length follows ({} bytes)", 1 << (additional - 24))
                }
            }
            6 => {
                if additional <= 23 {
                    format!("tag {} ({})", additional, tag_name(additional as u64))
                } else {
                    "tag (multibyte)".to_string()
                }
            }
            _ => String::new(),
        };
        out.push_str(&format!(
            "{:<6} {:02x}   {:<10} {}\n",
            pos, byte, type_label, info
        ));
        pos += 1;
    }

    if data.len() > 256 {
        out.push_str(&format!(
            "\n... ({} more bytes not shown)\n",
            data.len() - 256
        ));
    }

    Ok(out)
}
