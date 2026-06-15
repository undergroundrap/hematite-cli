use serde_json::{json, Value};

pub fn msgpack_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["decode", "info", "annotate"],
                "description": "decode (default — human-readable decoded value), info (type/structure summary), annotate (hex dump with MessagePack format annotations). Default: decode."
            },
            "hex": {
                "type": "string",
                "description": "Hex-encoded MessagePack bytes to decode."
            },
            "file": {
                "type": "string",
                "description": "Path to a binary file containing MessagePack data."
            },
            "base64": {
                "type": "string",
                "description": "Base64-encoded MessagePack bytes to decode."
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

// ── Input loading ─────────────────────────────────────────────────────────────

fn load_bytes(args: &Value) -> Result<Vec<u8>, String> {
    if let Some(h) = args.get("hex").and_then(|v| v.as_str()) {
        return decode_hex(h);
    }
    if let Some(b) = args.get("base64").and_then(|v| v.as_str()) {
        return decode_base64(b);
    }
    if let Some(path) = args
        .get("file")
        .or_else(|| args.get("path"))
        .and_then(|v| v.as_str())
    {
        return std::fs::read(path)
            .map_err(|e| format!("msgpack_tools: cannot read '{}': {}", path, e));
    }
    Err("msgpack_tools: provide 'hex', 'base64', or 'file'".to_string())
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !s.len().is_multiple_of(2) {
        return Err("msgpack_tools: hex string must have even length".into());
    }
    (0..s.len() / 2)
        .map(|i| {
            u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|_| format!("msgpack_tools: invalid hex at byte {}", i))
        })
        .collect()
}

fn decode_base64(s: &str) -> Result<Vec<u8>, String> {
    let s: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            c => c,
        })
        .collect();
    const TABLE: &[u8; 128] = b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\x3e\xff\xff\xff\x3f\
                                \x34\x35\x36\x37\x38\x39\x3a\x3b\x3c\x3d\xff\xff\xff\xfe\xff\xff\
                                \xff\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\
                                \x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\xff\xff\xff\xff\xff\
                                \xff\x1a\x1b\x1c\x1d\x1e\x1f\x20\x21\x22\x23\x24\x25\x26\x27\x28\
                                \x29\x2a\x2b\x2c\x2d\x2e\x2f\x30\x31\x32\x33\xff\xff\xff\xff\xff";
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let chars: Vec<u8> = s.bytes().collect();
    let mut i = 0;
    while i < chars.len() {
        let mut buf = [0u32; 4];
        let mut group_len = 0;
        for (slot, slot_val) in buf.iter_mut().enumerate() {
            if i >= chars.len() {
                break;
            }
            let c = chars[i] as usize;
            i += 1;
            if c >= 128 {
                return Err("msgpack_tools: invalid base64 character".into());
            }
            let v = TABLE[c];
            if v == 0xfe {
                break;
            }
            if v == 0xff {
                return Err("msgpack_tools: invalid base64 character".into());
            }
            *slot_val = v as u32;
            group_len = slot + 1;
        }
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

// ── MessagePack value types ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum MpValue {
    Nil,
    Bool(bool),
    Int(i64),
    Uint(u64),
    Float32(f32),
    Float64(f64),
    Str(String),
    Bin(Vec<u8>),
    Array(Vec<MpValue>),
    Map(Vec<(MpValue, MpValue)>),
    Ext(i8, Vec<u8>),
}

struct MpDecoder<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> MpDecoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        MpDecoder { data, pos: 0 }
    }

    fn read_byte(&mut self) -> Result<u8, String> {
        let b = self
            .data
            .get(self.pos)
            .copied()
            .ok_or_else(|| "msgpack_tools: unexpected end of data".to_string())?;
        self.pos += 1;
        Ok(b)
    }

    fn read_n(&mut self, n: usize) -> Result<&[u8], String> {
        if self.pos + n > self.data.len() {
            return Err("msgpack_tools: unexpected end of data".into());
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let b = self.read_n(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let b = self.read_n(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let b = self.read_n(8)?;
        Ok(u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_str(&mut self, len: usize) -> Result<String, String> {
        let bytes = self.read_n(len)?.to_vec();
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    fn read_bin(&mut self, len: usize) -> Result<Vec<u8>, String> {
        Ok(self.read_n(len)?.to_vec())
    }

    fn decode(&mut self, depth: usize) -> Result<MpValue, String> {
        if depth > 64 {
            return Err("msgpack_tools: maximum nesting depth (64) exceeded".into());
        }
        let b = self.read_byte()?;

        match b {
            // Positive fixint: 0xxxxxxx
            0x00..=0x7f => Ok(MpValue::Uint(b as u64)),
            // Fixmap: 1000xxxx
            0x80..=0x8f => {
                let len = (b & 0x0f) as usize;
                self.read_map(len, depth)
            }
            // Fixarray: 1001xxxx
            0x90..=0x9f => {
                let len = (b & 0x0f) as usize;
                self.read_array(len, depth)
            }
            // Fixstr: 101xxxxx
            0xa0..=0xbf => {
                let len = (b & 0x1f) as usize;
                Ok(MpValue::Str(self.read_str(len)?))
            }
            0xc0 => Ok(MpValue::Nil),
            0xc2 => Ok(MpValue::Bool(false)),
            0xc3 => Ok(MpValue::Bool(true)),
            // bin8
            0xc4 => {
                let len = self.read_byte()? as usize;
                Ok(MpValue::Bin(self.read_bin(len)?))
            }
            // bin16
            0xc5 => {
                let len = self.read_u16()? as usize;
                Ok(MpValue::Bin(self.read_bin(len)?))
            }
            // bin32
            0xc6 => {
                let len = self.read_u32()? as usize;
                Ok(MpValue::Bin(self.read_bin(len)?))
            }
            // ext8
            0xc7 => {
                let len = self.read_byte()? as usize;
                let ext_type = self.read_byte()? as i8;
                Ok(MpValue::Ext(ext_type, self.read_bin(len)?))
            }
            // ext16
            0xc8 => {
                let len = self.read_u16()? as usize;
                let ext_type = self.read_byte()? as i8;
                Ok(MpValue::Ext(ext_type, self.read_bin(len)?))
            }
            // ext32
            0xc9 => {
                let len = self.read_u32()? as usize;
                let ext_type = self.read_byte()? as i8;
                Ok(MpValue::Ext(ext_type, self.read_bin(len)?))
            }
            // float32
            0xca => {
                let b = self.read_n(4)?;
                let bits = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
                Ok(MpValue::Float32(f32::from_bits(bits)))
            }
            // float64
            0xcb => {
                let b = self.read_n(8)?;
                let bits = u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
                Ok(MpValue::Float64(f64::from_bits(bits)))
            }
            // uint8
            0xcc => Ok(MpValue::Uint(self.read_byte()? as u64)),
            // uint16
            0xcd => Ok(MpValue::Uint(self.read_u16()? as u64)),
            // uint32
            0xce => Ok(MpValue::Uint(self.read_u32()? as u64)),
            // uint64
            0xcf => Ok(MpValue::Uint(self.read_u64()?)),
            // int8
            0xd0 => Ok(MpValue::Int(self.read_byte()? as i8 as i64)),
            // int16
            0xd1 => {
                let b = self.read_n(2)?;
                Ok(MpValue::Int(i16::from_be_bytes([b[0], b[1]]) as i64))
            }
            // int32
            0xd2 => {
                let b = self.read_n(4)?;
                Ok(MpValue::Int(
                    i32::from_be_bytes([b[0], b[1], b[2], b[3]]) as i64
                ))
            }
            // int64
            0xd3 => {
                let b = self.read_n(8)?;
                Ok(MpValue::Int(i64::from_be_bytes([
                    b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
                ])))
            }
            // fixext1
            0xd4 => {
                let ext_type = self.read_byte()? as i8;
                Ok(MpValue::Ext(ext_type, self.read_bin(1)?))
            }
            // fixext2
            0xd5 => {
                let ext_type = self.read_byte()? as i8;
                Ok(MpValue::Ext(ext_type, self.read_bin(2)?))
            }
            // fixext4
            0xd6 => {
                let ext_type = self.read_byte()? as i8;
                Ok(MpValue::Ext(ext_type, self.read_bin(4)?))
            }
            // fixext8
            0xd7 => {
                let ext_type = self.read_byte()? as i8;
                Ok(MpValue::Ext(ext_type, self.read_bin(8)?))
            }
            // fixext16
            0xd8 => {
                let ext_type = self.read_byte()? as i8;
                Ok(MpValue::Ext(ext_type, self.read_bin(16)?))
            }
            // str8
            0xd9 => {
                let len = self.read_byte()? as usize;
                Ok(MpValue::Str(self.read_str(len)?))
            }
            // str16
            0xda => {
                let len = self.read_u16()? as usize;
                Ok(MpValue::Str(self.read_str(len)?))
            }
            // str32
            0xdb => {
                let len = self.read_u32()? as usize;
                Ok(MpValue::Str(self.read_str(len)?))
            }
            // array16
            0xdc => {
                let len = self.read_u16()? as usize;
                self.read_array(len, depth)
            }
            // array32
            0xdd => {
                let len = self.read_u32()? as usize;
                self.read_array(len, depth)
            }
            // map16
            0xde => {
                let len = self.read_u16()? as usize;
                self.read_map(len, depth)
            }
            // map32
            0xdf => {
                let len = self.read_u32()? as usize;
                self.read_map(len, depth)
            }
            // Negative fixint: 111xxxxx
            0xe0..=0xff => Ok(MpValue::Int(b as i8 as i64)),
            _ => Err(format!("msgpack_tools: unknown format byte 0x{:02x}", b)),
        }
    }

    fn read_array(&mut self, len: usize, depth: usize) -> Result<MpValue, String> {
        let mut arr = Vec::with_capacity(len.min(1024));
        for _ in 0..len {
            arr.push(self.decode(depth + 1)?);
        }
        Ok(MpValue::Array(arr))
    }

    fn read_map(&mut self, len: usize, depth: usize) -> Result<MpValue, String> {
        let mut map = Vec::with_capacity(len.min(256));
        for _ in 0..len {
            let k = self.decode(depth + 1)?;
            let v = self.decode(depth + 1)?;
            map.push((k, v));
        }
        Ok(MpValue::Map(map))
    }
}

// ── Ext type names ────────────────────────────────────────────────────────────

fn ext_type_name(t: i8) -> &'static str {
    match t {
        -1 => "Timestamp",
        1 => "msgpack-ext-1",
        _ => "(custom)",
    }
}

// ── Pretty printer ────────────────────────────────────────────────────────────

fn pretty(val: &MpValue, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    match val {
        MpValue::Nil => out.push_str("nil"),
        MpValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        MpValue::Int(n) => out.push_str(&format!("{}", n)),
        MpValue::Uint(n) => out.push_str(&format!("{}", n)),
        MpValue::Float32(f) => out.push_str(&format!("{}", f)),
        MpValue::Float64(f) => out.push_str(&format!("{}", f)),
        MpValue::Str(s) => out.push_str(&format!("{:?}", s)),
        MpValue::Bin(b) => {
            if b.len() <= 32 {
                out.push_str(&format!(
                    "bin({})[{}]",
                    b.len(),
                    b.iter().map(|x| format!("{:02x}", x)).collect::<String>()
                ));
            } else {
                out.push_str(&format!(
                    "bin({}) {}...",
                    b.len(),
                    b[..16]
                        .iter()
                        .map(|x| format!("{:02x}", x))
                        .collect::<String>()
                ));
            }
        }
        MpValue::Ext(t, data) => {
            out.push_str(&format!(
                "ext({}, {}, {} bytes)",
                t,
                ext_type_name(*t),
                data.len()
            ));
            // Decode Timestamp ext (-1) when 4 or 8 bytes
            if *t == -1 {
                if data.len() == 4 {
                    let secs = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as u64;
                    out.push_str(&format!(" = {} (unix epoch secs)", secs));
                } else if data.len() == 8 {
                    let val = u64::from_be_bytes([
                        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                    ]);
                    let nsec = val >> 34;
                    let sec = val & 0x3ffffffff;
                    out.push_str(&format!(" = {} sec {} nsec (unix)", sec, nsec));
                }
            }
        }
        MpValue::Array(arr) => {
            if arr.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for item in arr {
                out.push_str(&format!("{}  ", pad));
                pretty(item, indent + 1, out);
                out.push_str(",\n");
            }
            out.push_str(&format!("{}]", pad));
        }
        MpValue::Map(pairs) => {
            if pairs.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (k, v) in pairs {
                out.push_str(&format!("{}  ", pad));
                pretty(k, indent + 1, out);
                out.push_str(": ");
                pretty(v, indent + 1, out);
                out.push_str(",\n");
            }
            out.push_str(&format!("{}}}", pad));
        }
    }
}

fn mp_type_name(val: &MpValue) -> &'static str {
    match val {
        MpValue::Nil => "nil",
        MpValue::Bool(_) => "bool",
        MpValue::Int(_) => "int",
        MpValue::Uint(_) => "uint",
        MpValue::Float32(_) => "float32",
        MpValue::Float64(_) => "float64",
        MpValue::Str(_) => "str",
        MpValue::Bin(_) => "bin",
        MpValue::Array(_) => "array",
        MpValue::Map(_) => "map",
        MpValue::Ext(_, _) => "ext",
    }
}

// ── action: decode ────────────────────────────────────────────────────────────

fn action_decode(data: &[u8]) -> Result<String, String> {
    let mut dec = MpDecoder::new(data);
    let val = dec.decode(0)?;
    let remaining = data.len() - dec.pos;

    let mut out = String::new();
    out.push_str("MessagePack Decoded\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    pretty(&val, 0, &mut out);
    out.push('\n');
    if remaining > 0 {
        out.push_str(&format!("\n({} trailing byte(s) not decoded)\n", remaining));
    }
    Ok(out)
}

// ── action: info ──────────────────────────────────────────────────────────────

fn count_types(val: &MpValue, stats: &mut std::collections::HashMap<String, usize>) {
    *stats.entry(mp_type_name(val).to_string()).or_insert(0) += 1;
    match val {
        MpValue::Array(arr) => {
            for item in arr {
                count_types(item, stats);
            }
        }
        MpValue::Map(pairs) => {
            for (k, v) in pairs {
                count_types(k, stats);
                count_types(v, stats);
            }
        }
        _ => {}
    }
}

fn action_info(data: &[u8]) -> Result<String, String> {
    let mut dec = MpDecoder::new(data);
    let val = dec.decode(0)?;
    let remaining = data.len() - dec.pos;

    let mut stats = std::collections::HashMap::new();
    count_types(&val, &mut stats);

    let mut out = String::new();
    out.push_str("MessagePack Info\n");
    out.push_str(&"─".repeat(40));
    out.push('\n');
    out.push_str(&format!("Total bytes:  {}\n", data.len()));
    out.push_str(&format!("Root type:    {}\n", mp_type_name(&val)));

    match &val {
        MpValue::Array(arr) => {
            out.push_str(&format!("Array length: {}\n", arr.len()));
        }
        MpValue::Map(pairs) => {
            out.push_str(&format!("Map entries:  {}\n", pairs.len()));
            let str_keys: Vec<&str> = pairs
                .iter()
                .filter_map(|(k, _)| {
                    if let MpValue::Str(s) = k {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .collect();
            if !str_keys.is_empty() {
                out.push_str(&format!("Keys:         {}\n", str_keys.join(", ")));
            }
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

fn format_byte_label(b: u8) -> (&'static str, String) {
    match b {
        0x00..=0x7f => ("uint(fixint)", format!("value = {}", b)),
        0x80..=0x8f => ("map(fixmap)", format!("length = {}", b & 0x0f)),
        0x90..=0x9f => ("array(fixarray)", format!("length = {}", b & 0x0f)),
        0xa0..=0xbf => ("str(fixstr)", format!("length = {}", b & 0x1f)),
        0xc0 => ("nil", String::new()),
        0xc2 => ("false", String::new()),
        0xc3 => ("true", String::new()),
        0xc4 => ("bin8", "length follows (1 byte)".into()),
        0xc5 => ("bin16", "length follows (2 bytes)".into()),
        0xc6 => ("bin32", "length follows (4 bytes)".into()),
        0xc7 => ("ext8", "length+type follow".into()),
        0xc8 => ("ext16", "length+type follow".into()),
        0xc9 => ("ext32", "length+type follow".into()),
        0xca => ("float32", "4 bytes follow".into()),
        0xcb => ("float64", "8 bytes follow".into()),
        0xcc => ("uint8", "1 byte follows".into()),
        0xcd => ("uint16", "2 bytes follow".into()),
        0xce => ("uint32", "4 bytes follow".into()),
        0xcf => ("uint64", "8 bytes follow".into()),
        0xd0 => ("int8", "1 byte follows".into()),
        0xd1 => ("int16", "2 bytes follow".into()),
        0xd2 => ("int32", "4 bytes follow".into()),
        0xd3 => ("int64", "8 bytes follow".into()),
        0xd4 => ("fixext1", "type+1 byte follow".into()),
        0xd5 => ("fixext2", "type+2 bytes follow".into()),
        0xd6 => ("fixext4", "type+4 bytes follow".into()),
        0xd7 => ("fixext8", "type+8 bytes follow".into()),
        0xd8 => ("fixext16", "type+16 bytes follow".into()),
        0xd9 => ("str8", "length follows (1 byte)".into()),
        0xda => ("str16", "length follows (2 bytes)".into()),
        0xdb => ("str32", "length follows (4 bytes)".into()),
        0xdc => ("array16", "length follows (2 bytes)".into()),
        0xdd => ("array32", "length follows (4 bytes)".into()),
        0xde => ("map16", "length follows (2 bytes)".into()),
        0xdf => ("map32", "length follows (4 bytes)".into()),
        0xe0..=0xff => ("int(negfixint)", format!("value = {}", b as i8)),
        _ => ("?", String::new()),
    }
}

fn action_annotate(data: &[u8]) -> Result<String, String> {
    let limit = data.len().min(256);
    let mut out = String::new();
    out.push_str("MessagePack Hex Annotated\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "{:<6} {:<4} {:<18} {}\n",
        "Offset", "Hex", "Format", "Info"
    ));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for (i, &byte) in data[..limit].iter().enumerate() {
        let (label, info) = format_byte_label(byte);
        out.push_str(&format!("{:<6} {:02x}   {:<18} {}\n", i, byte, label, info));
    }

    if data.len() > 256 {
        out.push_str(&format!(
            "\n... ({} more bytes not shown)\n",
            data.len() - 256
        ));
    }
    Ok(out)
}
