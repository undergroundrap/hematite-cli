use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "name": "protobuf_wire_tools",
        "description": "Decode raw protobuf wire format bytes without a .proto schema — shows field numbers, wire types, varint/fixed/length-delimited values, and string candidates. Complements proto_tools (which parses .proto schema files). Useful for debugging gRPC calls, inspecting binary API payloads, and reverse-engineering protobuf messages.",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["decode", "fields", "strings", "explain"],
                    "description": "decode (default) = recursive field-by-field decoding, fields = just field numbers and wire types, strings = extract UTF-8 string candidates from length-delimited fields, explain = verbose per-field interpretation with all possible type readings"
                },
                "hex": {
                    "type": "string",
                    "description": "Hex-encoded protobuf wire bytes (spaces and colons stripped automatically)"
                },
                "file": {
                    "type": "string",
                    "description": "Path to a binary file containing raw protobuf bytes"
                },
                "depth": {
                    "type": "integer",
                    "description": "Maximum recursion depth for nested message detection (default: 3)"
                }
            }
        }
    })
}

// ── Varint decoder ────────────────────────────────────────────────────────────

fn read_varint(data: &[u8], pos: &mut usize) -> Option<u64> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        if *pos >= data.len() { return None; }
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 { return Some(result); }
        shift += 7;
        if shift >= 64 { return None; }
    }
}

fn varint_len(data: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < data.len() {
        let byte = data[i]; i += 1;
        if byte & 0x80 == 0 { return i - start; }
    }
    0
}

fn decode_zigzag32(v: u64) -> i32 {
    let v = v as u32;
    ((v >> 1) as i32) ^ -((v & 1) as i32)
}

fn decode_zigzag64(v: u64) -> i64 {
    ((v >> 1) as i64) ^ -((v & 1) as i64)
}

// ── Wire type names ───────────────────────────────────────────────────────────

fn wire_type_name(wt: u64) -> &'static str {
    match wt {
        0 => "Varint",
        1 => "64-bit",
        2 => "Length-delimited",
        5 => "32-bit",
        _ => "Unknown",
    }
}

// ── Is this byte slice plausibly a nested protobuf message? ──────────────────

fn looks_like_proto(data: &[u8]) -> bool {
    if data.is_empty() { return false; }
    let mut pos = 0;
    let mut field_count = 0;
    while pos < data.len() {
        let tag = match read_varint(data, &mut pos) { Some(v) => v, None => return false };
        let wt = tag & 0x07;
        let _field_num = tag >> 3;
        if _field_num == 0 { return false; }
        match wt {
            0 => { if read_varint(data, &mut pos).is_none() { return false; } }
            1 => { if pos + 8 > data.len() { return false; } pos += 8; }
            2 => {
                let len = match read_varint(data, &mut pos) { Some(v) => v as usize, None => return false };
                if pos + len > data.len() { return false; }
                pos += len;
            }
            5 => { if pos + 4 > data.len() { return false; } pos += 4; }
            _ => return false,
        }
        field_count += 1;
        if field_count >= 3 { return true; }
    }
    pos == data.len() && field_count > 0
}

fn is_valid_utf8(data: &[u8]) -> bool {
    std::str::from_utf8(data).is_ok()
}

fn bytes_preview(data: &[u8]) -> String {
    if data.len() <= 16 {
        data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")
    } else {
        let head = data[..8].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
        format!("{} ... ({} bytes total)", head, data.len())
    }
}

// ── Core decode ───────────────────────────────────────────────────────────────

struct Field {
    field_num: u64,
    wire_type: u64,
    value: FieldValue,
}

enum FieldValue {
    Varint(u64),
    Fixed64([u8; 8]),
    LenDelim(Vec<u8>),
    Fixed32([u8; 4]),
}

fn decode_fields(data: &[u8]) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let tag = match read_varint(data, &mut pos) { Some(v) => v, None => break };
        let wire_type = tag & 0x07;
        let field_num = tag >> 3;
        if field_num == 0 { break; }
        match wire_type {
            0 => {
                match read_varint(data, &mut pos) {
                    Some(v) => fields.push(Field { field_num, wire_type, value: FieldValue::Varint(v) }),
                    None => break,
                }
            }
            1 => {
                if pos + 8 > data.len() { break; }
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&data[pos..pos + 8]);
                pos += 8;
                fields.push(Field { field_num, wire_type, value: FieldValue::Fixed64(buf) });
            }
            2 => {
                let len = match read_varint(data, &mut pos) { Some(v) => v as usize, None => break };
                if pos + len > data.len() { break; }
                let bytes = data[pos..pos + len].to_vec();
                pos += len;
                fields.push(Field { field_num, wire_type, value: FieldValue::LenDelim(bytes) });
            }
            5 => {
                if pos + 4 > data.len() { break; }
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&data[pos..pos + 4]);
                pos += 4;
                fields.push(Field { field_num, wire_type, value: FieldValue::Fixed32(buf) });
            }
            _ => break,
        }
    }
    fields
}

// ── Formatting ────────────────────────────────────────────────────────────────

fn format_fields_recursive(data: &[u8], indent: usize, max_depth: usize, out: &mut String) {
    let pad = " ".repeat(indent * 2);
    let fields = decode_fields(data);

    if fields.is_empty() {
        out.push_str(&format!("{}(no fields decoded)\n", pad));
        return;
    }

    for f in &fields {
        match &f.value {
            FieldValue::Varint(v) => {
                out.push_str(&format!("{}field {:3}  [Varint]  {}", pad, f.field_num, v));
                // Also show signed and zigzag interpretations for large values
                if *v > 0 {
                    let signed = *v as i64;
                    let zz32 = decode_zigzag32(*v);
                    let zz64 = decode_zigzag64(*v);
                    if signed < 0 || zz32 != signed as i32 || zz64 != signed {
                        out.push_str(&format!("  (signed: {}  zz32: {}  zz64: {})", signed, zz32, zz64));
                    }
                }
                out.push('\n');
            }
            FieldValue::Fixed64(buf) => {
                let u = u64::from_le_bytes(*buf);
                let f64v = f64::from_le_bytes(*buf);
                out.push_str(&format!("{}field {:3}  [Fixed64] 0x{:016x}  u64={}", pad, f.field_num,
                    u, u));
                if f64v.is_finite() {
                    out.push_str(&format!("  f64={:.6}", f64v));
                }
                out.push('\n');
            }
            FieldValue::Fixed32(buf) => {
                let u = u32::from_le_bytes(*buf);
                let f32v = f32::from_le_bytes(*buf);
                out.push_str(&format!("{}field {:3}  [Fixed32] 0x{:08x}  u32={}", pad, f.field_num,
                    u, u));
                if f32v.is_finite() {
                    out.push_str(&format!("  f32={:.6}", f32v));
                }
                out.push('\n');
            }
            FieldValue::LenDelim(bytes) => {
                if bytes.is_empty() {
                    out.push_str(&format!("{}field {:3}  [LenDelim] (empty)\n", pad, f.field_num));
                } else if let Ok(s) = std::str::from_utf8(bytes) {
                    let preview = if s.len() > 80 { format!("{}...", &s[..77]) } else { s.to_string() };
                    let display: String = preview.chars()
                        .map(|c| if c.is_control() && c != '\n' { '·' } else { c }).collect();
                    out.push_str(&format!("{}field {:3}  [LenDelim] {:3} bytes  string: \"{}\"\n",
                        pad, f.field_num, bytes.len(), display));
                } else if indent < max_depth && looks_like_proto(bytes) {
                    out.push_str(&format!("{}field {:3}  [LenDelim] {:3} bytes  nested message:\n",
                        pad, f.field_num, bytes.len()));
                    format_fields_recursive(bytes, indent + 1, max_depth, out);
                } else {
                    out.push_str(&format!("{}field {:3}  [LenDelim] {:3} bytes  {}\n",
                        pad, f.field_num, bytes.len(), bytes_preview(bytes)));
                }
            }
        }
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_decode(data: &[u8], max_depth: usize) -> Result<String, String> {
    if data.is_empty() { return Err("empty input".into()); }
    let mut out = String::new();
    out.push_str(&format!("── Protobuf Wire Decode ({} bytes) ─────────────────────\n\n", data.len()));
    format_fields_recursive(data, 0, max_depth, &mut out);
    Ok(out)
}

fn action_fields(data: &[u8]) -> Result<String, String> {
    if data.is_empty() { return Err("empty input".into()); }
    let fields = decode_fields(data);
    if fields.is_empty() { return Err("No protobuf fields decoded — check that input is valid protobuf wire format".into()); }
    let mut out = String::new();
    out.push_str(&format!("── Field Summary ({} fields) ────────────────────────────\n", fields.len()));
    out.push_str("  Field#  Wire Type           Size/Value\n");
    out.push_str("  ──────  ──────────────────  ──────────────────────────\n");
    for f in &fields {
        let detail = match &f.value {
            FieldValue::Varint(v) => format!("{}", v),
            FieldValue::Fixed64(_) => "8 bytes".into(),
            FieldValue::Fixed32(_) => "4 bytes".into(),
            FieldValue::LenDelim(b) => format!("{} bytes", b.len()),
        };
        out.push_str(&format!("  {:6}  {:<18}  {}\n", f.field_num, wire_type_name(f.wire_type), detail));
    }
    Ok(out)
}

fn action_strings(data: &[u8]) -> Result<String, String> {
    if data.is_empty() { return Err("empty input".into()); }
    let fields = decode_fields(data);
    let mut strings: Vec<(u64, String)> = Vec::new();
    collect_strings(&fields, &mut strings);
    if strings.is_empty() {
        return Ok("No UTF-8 string candidates found in length-delimited fields.".into());
    }
    let mut out = String::new();
    out.push_str(&format!("── String Candidates ({}) ───────────────────────────────\n", strings.len()));
    for (field_num, s) in &strings {
        let preview = if s.len() > 120 { format!("{}...", &s[..117]) } else { s.clone() };
        out.push_str(&format!("  field {:3}: \"{}\"\n", field_num, preview));
    }
    Ok(out)
}

fn collect_strings(fields: &[Field], out: &mut Vec<(u64, String)>) {
    for f in fields {
        if let FieldValue::LenDelim(bytes) = &f.value {
            if let Ok(s) = std::str::from_utf8(bytes) {
                if !s.is_empty() && s.chars().any(|c| !c.is_control()) {
                    out.push((f.field_num, s.to_string()));
                }
            } else if looks_like_proto(bytes) {
                let nested = decode_fields(bytes);
                collect_strings(&nested, out);
            }
        }
    }
}

fn action_explain(data: &[u8]) -> Result<String, String> {
    if data.is_empty() { return Err("empty input".into()); }
    let fields = decode_fields(data);
    if fields.is_empty() { return Err("No protobuf fields decoded".into()); }

    let mut out = String::new();
    out.push_str(&format!("── Protobuf Wire Explain ({} bytes, {} fields) ──────────\n\n", data.len(), fields.len()));

    for f in &fields {
        out.push_str(&format!("  field {} (wire_type={}  {})\n", f.field_num, f.wire_type, wire_type_name(f.wire_type)));
        match &f.value {
            FieldValue::Varint(v) => {
                out.push_str(&format!("    raw uint64     : {}\n", v));
                out.push_str(&format!("    int64 signed   : {}\n", *v as i64));
                out.push_str(&format!("    sint32 zigzag  : {}\n", decode_zigzag32(*v)));
                out.push_str(&format!("    sint64 zigzag  : {}\n", decode_zigzag64(*v)));
                out.push_str(&format!("    bool           : {}\n", *v != 0));
                out.push_str(&format!("    hex            : 0x{:x}\n", v));
            }
            FieldValue::Fixed64(buf) => {
                let u = u64::from_le_bytes(*buf);
                let i = i64::from_le_bytes(*buf);
                let f64v = f64::from_le_bytes(*buf);
                out.push_str(&format!("    fixed64 uint64 : {}\n", u));
                out.push_str(&format!("    sfixed64 int64 : {}\n", i));
                out.push_str(&format!("    double float64 : {}\n", f64v));
                out.push_str(&format!("    hex            : 0x{:016x}\n", u));
            }
            FieldValue::Fixed32(buf) => {
                let u = u32::from_le_bytes(*buf);
                let i = i32::from_le_bytes(*buf);
                let f32v = f32::from_le_bytes(*buf);
                out.push_str(&format!("    fixed32 uint32 : {}\n", u));
                out.push_str(&format!("    sfixed32 int32 : {}\n", i));
                out.push_str(&format!("    float float32  : {}\n", f32v));
                out.push_str(&format!("    hex            : 0x{:08x}\n", u));
            }
            FieldValue::LenDelim(bytes) => {
                out.push_str(&format!("    length         : {} bytes\n", bytes.len()));
                out.push_str(&format!("    bytes (hex)    : {}\n", bytes_preview(bytes)));
                if is_valid_utf8(bytes) {
                    let s = std::str::from_utf8(bytes).unwrap();
                    let preview = if s.len() > 80 { format!("{}...", &s[..77]) } else { s.to_string() };
                    let display: String = preview.chars()
                        .map(|c| if c.is_control() { '·' } else { c }).collect();
                    out.push_str(&format!("    string         : \"{}\"\n", display));
                }
                if looks_like_proto(bytes) {
                    out.push_str("    nested message : likely (parses as valid proto fields)\n");
                }
            }
        }
        out.push('\n');
    }
    Ok(out)
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn load_input(args: &Value) -> Result<Vec<u8>, String> {
    if let Some(hex) = args.get("hex").and_then(Value::as_str) {
        let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if clean.len() % 2 != 0 { return Err("odd-length hex string".into()); }
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
    let action = args.get("action").and_then(Value::as_str).unwrap_or("decode");
    let max_depth = args.get("depth").and_then(Value::as_u64).unwrap_or(3) as usize;
    let data = load_input(args)?;
    match action {
        "decode" => action_decode(&data, max_depth),
        "fields" => action_fields(&data),
        "strings" => action_strings(&data),
        "explain" => action_explain(&data),
        other => Err(format!("Unknown action '{}'. Use: decode, fields, strings, explain", other)),
    }
}
