use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "name": "dex_tools",
        "description": "Inspect Android DEX (Dalvik Executable) files without dexdump or an Android SDK. 5 actions: info (default — magic/version/byte-order/file-size/string+type+field+method+class counts/API level note), classes (class definitions with name/superclass/access/fields/methods; 'limit' to cap), methods (all method references with class/method name/return type; 'limit' to cap), strings (full string pool — useful for hardcoded URLs/keys/permissions; 'limit' to cap), imports (all type descriptors categorized as Android Framework / Java stdlib / App). Pass 'file' (path to .dex file) or 'hex' (hex-encoded DEX bytes).",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "classes", "methods", "strings", "imports"],
                    "description": "Operation to perform (default: info)"
                },
                "file": { "type": "string", "description": "Path to a .dex file" },
                "hex": { "type": "string", "description": "Hex-encoded DEX bytes" },
                "limit": { "type": "integer", "description": "Maximum entries to return for classes/methods/strings actions (default 50)" }
            }
        }
    })
}

const DEX_MAGIC_PREFIX: &[u8] = b"dex\n";
const ENDIAN_CONSTANT: u32 = 0x1234_5678;
const REVERSE_ENDIAN_CONSTANT: u32 = 0x7856_3412;

struct DexHeader {
    version: String,
    checksum: u32,
    file_size: u32,
    header_size: u32,
    string_ids_size: u32,
    string_ids_off: u32,
    type_ids_size: u32,
    type_ids_off: u32,
    proto_ids_size: u32,
    proto_ids_off: u32,
    field_ids_size: u32,
    field_ids_off: u32,
    method_ids_size: u32,
    method_ids_off: u32,
    class_defs_size: u32,
    class_defs_off: u32,
    data_size: u32,
    data_off: u32,
}

fn parse_header(data: &[u8]) -> Result<(DexHeader, bool), String> {
    if data.len() < 112 {
        return Err("file too small to be a DEX file (< 112 bytes)".to_string());
    }
    if &data[0..4] != DEX_MAGIC_PREFIX {
        return Err(format!(
            "not a DEX file (expected b\"dex\\n\", got {:?})",
            &data[0..4]
        ));
    }
    let version = String::from_utf8_lossy(&data[4..7]).to_string();

    // Endian tag at offset 40
    let endian_le =
        u32::from_le_bytes([data[40], data[41], data[42], data[43]]);
    let le = endian_le == ENDIAN_CONSTANT;
    if endian_le != ENDIAN_CONSTANT && endian_le != REVERSE_ENDIAN_CONSTANT {
        return Err(format!(
            "unknown endian tag: 0x{:08x} (expected 0x12345678 or 0x78563412)",
            endian_le
        ));
    }

    let u32_at = |off: usize| -> u32 {
        if le {
            u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
        } else {
            u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
        }
    };

    Ok((
        DexHeader {
            version,
            checksum: u32_at(8),
            file_size: u32_at(32),
            header_size: u32_at(36),
            string_ids_size: u32_at(56),
            string_ids_off: u32_at(60),
            type_ids_size: u32_at(64),
            type_ids_off: u32_at(68),
            proto_ids_size: u32_at(72),
            proto_ids_off: u32_at(76),
            field_ids_size: u32_at(80),
            field_ids_off: u32_at(84),
            method_ids_size: u32_at(88),
            method_ids_off: u32_at(92),
            class_defs_size: u32_at(96),
            class_defs_off: u32_at(100),
            data_size: u32_at(104),
            data_off: u32_at(108),
        },
        le,
    ))
}

fn read_string(
    data: &[u8],
    string_ids_off: u32,
    string_ids_size: u32,
    idx: u32,
    le: bool,
) -> String {
    if idx >= string_ids_size {
        return format!("<string#{}>", idx);
    }
    let id_off = string_ids_off as usize + idx as usize * 4;
    if id_off + 4 > data.len() {
        return format!("<string#{}-oob>", idx);
    }
    let data_off = (if le {
        u32::from_le_bytes([data[id_off], data[id_off + 1], data[id_off + 2], data[id_off + 3]])
    } else {
        u32::from_be_bytes([data[id_off], data[id_off + 1], data[id_off + 2], data[id_off + 3]])
    }) as usize;

    if data_off >= data.len() {
        return format!("<string#{}-oob2>", idx);
    }

    // Skip ULEB128 length prefix
    let mut pos = data_off;
    loop {
        if pos >= data.len() {
            break;
        }
        let byte = data[pos];
        pos += 1;
        if byte & 0x80 == 0 {
            break;
        }
    }

    // Read null-terminated modified UTF-8
    let start = pos;
    while pos < data.len() && data[pos] != 0 {
        pos += 1;
    }
    String::from_utf8_lossy(&data[start..pos]).to_string()
}

fn read_type_descriptor(data: &[u8], hdr: &DexHeader, type_idx: u32, le: bool) -> String {
    if type_idx >= hdr.type_ids_size {
        return format!("<type#{}>", type_idx);
    }
    let id_off = hdr.type_ids_off as usize + type_idx as usize * 4;
    if id_off + 4 > data.len() {
        return format!("<type#{}-oob>", type_idx);
    }
    let string_idx = if le {
        u32::from_le_bytes([data[id_off], data[id_off + 1], data[id_off + 2], data[id_off + 3]])
    } else {
        u32::from_be_bytes([data[id_off], data[id_off + 1], data[id_off + 2], data[id_off + 3]])
    };
    let raw = read_string(data, hdr.string_ids_off, hdr.string_ids_size, string_idx, le);
    decode_type_desc(&raw)
}

fn decode_type_desc(desc: &str) -> String {
    match desc.chars().next() {
        Some('L') => desc[1..].trim_end_matches(';').replace('/', "."),
        Some('[') => format!("{}[]", decode_type_desc(&desc[1..])),
        Some('B') => "byte".to_string(),
        Some('C') => "char".to_string(),
        Some('D') => "double".to_string(),
        Some('F') => "float".to_string(),
        Some('I') => "int".to_string(),
        Some('J') => "long".to_string(),
        Some('S') => "short".to_string(),
        Some('Z') => "boolean".to_string(),
        Some('V') => "void".to_string(),
        _ => desc.to_string(),
    }
}

fn dex_access_label(flags: u32) -> String {
    let mut parts = Vec::new();
    if flags & 0x0001 != 0 {
        parts.push("public");
    }
    if flags & 0x0002 != 0 {
        parts.push("private");
    }
    if flags & 0x0004 != 0 {
        parts.push("protected");
    }
    if flags & 0x0008 != 0 {
        parts.push("static");
    }
    if flags & 0x0010 != 0 {
        parts.push("final");
    }
    if flags & 0x0200 != 0 {
        parts.push("interface");
    }
    if flags & 0x0400 != 0 {
        parts.push("abstract");
    }
    if flags & 0x1000 != 0 {
        parts.push("synthetic");
    }
    if flags & 0x2000 != 0 {
        parts.push("annotation");
    }
    if flags & 0x4000 != 0 {
        parts.push("enum");
    }
    if parts.is_empty() {
        "package-private".to_string()
    } else {
        parts.join(" ")
    }
}

fn read_uleb128(data: &[u8], pos: &mut usize) -> u32 {
    let mut result = 0u32;
    let mut shift = 0u32;
    loop {
        if *pos >= data.len() {
            break;
        }
        let byte = data[*pos];
        result |= ((byte & 0x7f) as u32) << shift;
        *pos += 1;
        shift += 7;
        if byte & 0x80 == 0 || shift >= 35 {
            break;
        }
    }
    result
}

struct ClassDef {
    class_idx: u32,
    access_flags: u32,
    superclass_idx: u32,
    interfaces_off: u32,
    class_data_off: u32,
}

fn read_class_def(data: &[u8], hdr: &DexHeader, i: u32, le: bool) -> Option<ClassDef> {
    let off = hdr.class_defs_off as usize + i as usize * 32;
    if off + 32 > data.len() {
        return None;
    }
    let u32_at = |o: usize| -> u32 {
        if le {
            u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]])
        } else {
            u32::from_be_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]])
        }
    };
    Some(ClassDef {
        class_idx: u32_at(off),
        access_flags: u32_at(off + 4),
        superclass_idx: u32_at(off + 8),
        interfaces_off: u32_at(off + 12),
        class_data_off: u32_at(off + 24),
    })
}

fn count_interfaces(data: &[u8], interfaces_off: u32, le: bool) -> u32 {
    if interfaces_off == 0 || interfaces_off as usize + 4 > data.len() {
        return 0;
    }
    let off = interfaces_off as usize;
    if le {
        u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
    } else {
        u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
    }
}

fn class_field_method_counts(data: &[u8], class_data_off: u32) -> (u32, u32) {
    if class_data_off == 0 || class_data_off as usize >= data.len() {
        return (0, 0);
    }
    let mut pos = class_data_off as usize;
    let static_fields = read_uleb128(data, &mut pos);
    let instance_fields = read_uleb128(data, &mut pos);
    let direct_methods = read_uleb128(data, &mut pos);
    let virtual_methods = read_uleb128(data, &mut pos);
    (
        static_fields + instance_fields,
        direct_methods + virtual_methods,
    )
}

fn dex_version_note(ver: &str) -> &'static str {
    match ver {
        "035" => "API 1-23 (pre-ART support)",
        "036" => "API 24 (default-method support)",
        "037" => "API 24+",
        "038" => "API 26 (call-sites, method handles)",
        "039" => "API 28 (const-method-handle/type)",
        "040" | "041" => "API 31+",
        _ => "unknown version",
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("...{}", &s[s.len().saturating_sub(max - 3)..])
    } else {
        s.to_string()
    }
}

fn action_info(data: &[u8], hdr: &DexHeader, le: bool) -> String {
    let mut out = String::new();
    out.push_str("=== Android DEX Info ===\n\n");

    out.push_str(&format!("Magic:         dex\\n{}\n", hdr.version));
    out.push_str(&format!("Byte order:    {}\n", if le { "little-endian" } else { "big-endian" }));
    out.push_str(&format!(
        "File size:     {} bytes ({:.1} KB)\n",
        hdr.file_size,
        hdr.file_size as f64 / 1024.0
    ));
    out.push_str(&format!("Checksum:      0x{:08x}\n", hdr.checksum));
    out.push_str(&format!("Header size:   0x{:x}\n", hdr.header_size));
    out.push_str(&format!(
        "DEX version:   0{} — {}\n",
        hdr.version,
        dex_version_note(&hdr.version)
    ));

    out.push_str("\nContents:\n");
    out.push_str(&format!("  Strings:     {}\n", hdr.string_ids_size));
    out.push_str(&format!("  Types:       {}\n", hdr.type_ids_size));
    out.push_str(&format!("  Prototypes:  {}\n", hdr.proto_ids_size));
    out.push_str(&format!("  Fields:      {}\n", hdr.field_ids_size));
    out.push_str(&format!("  Methods:     {}\n", hdr.method_ids_size));
    out.push_str(&format!("  Classes:     {}\n", hdr.class_defs_size));
    out.push_str(&format!(
        "  Data:        {} bytes @ 0x{:x}\n",
        hdr.data_size, hdr.data_off
    ));

    // Quick class breakdown
    let _ = data;
    let _ = le;

    out
}

fn action_classes(data: &[u8], hdr: &DexHeader, le: bool, limit: usize) -> String {
    let mut out = String::new();
    out.push_str("=== Class Definitions ===\n\n");

    if hdr.class_defs_size == 0 {
        out.push_str("(no classes defined)\n");
        return out;
    }

    let count = (hdr.class_defs_size as usize).min(limit);
    out.push_str(&format!(
        "{:<50} {:<28} {:>7} {:>6} {:>7}\n",
        "Class", "Superclass", "Access", "Fields", "Methods"
    ));
    out.push_str(&format!("{}\n", "-".repeat(105)));

    for i in 0..count as u32 {
        let Some(def) = read_class_def(data, hdr, i, le) else {
            continue;
        };
        let class_name = read_type_descriptor(data, hdr, def.class_idx, le);
        let super_name = if def.superclass_idx == 0xffff_ffff {
            "(none)".to_string()
        } else {
            read_type_descriptor(data, hdr, def.superclass_idx, le)
        };
        let ifaces = count_interfaces(data, def.interfaces_off, le);
        let (fields, methods) = class_field_method_counts(data, def.class_data_off);
        let flags = dex_access_label(def.access_flags);

        let class_disp = truncate(&class_name, 48);
        let super_disp = truncate(&super_name, 26);

        let iface_suffix = if ifaces > 0 {
            format!("{} (+{}i)", flags, ifaces)
        } else {
            flags
        };

        out.push_str(&format!(
            "{:<50} {:<28} {:>7} {:>6} {:>7}\n",
            class_disp, super_disp, iface_suffix, fields, methods
        ));
    }

    if hdr.class_defs_size as usize > limit {
        out.push_str(&format!(
            "\n... {} more classes (pass limit parameter to see more)\n",
            hdr.class_defs_size as usize - limit
        ));
    }

    out
}

fn action_methods(data: &[u8], hdr: &DexHeader, le: bool, limit: usize) -> String {
    let mut out = String::new();
    out.push_str("=== Method References ===\n\n");

    if hdr.method_ids_size == 0 {
        out.push_str("(no methods)\n");
        return out;
    }

    let count = (hdr.method_ids_size as usize).min(limit);
    out.push_str(&format!("{:<48} {:<25} {}\n", "Class", "Method", "Return type"));
    out.push_str(&format!("{}\n", "-".repeat(100)));

    for i in 0..count {
        let off = hdr.method_ids_off as usize + i * 8;
        if off + 8 > data.len() {
            break;
        }
        let class_idx = (if le {
            u16::from_le_bytes([data[off], data[off + 1]])
        } else {
            u16::from_be_bytes([data[off], data[off + 1]])
        }) as u32;

        let proto_idx = (if le {
            u16::from_le_bytes([data[off + 2], data[off + 3]])
        } else {
            u16::from_be_bytes([data[off + 2], data[off + 3]])
        }) as u32;

        let name_idx = if le {
            u32::from_le_bytes([data[off + 4], data[off + 5], data[off + 6], data[off + 7]])
        } else {
            u32::from_be_bytes([data[off + 4], data[off + 5], data[off + 6], data[off + 7]])
        };

        let class_name = read_type_descriptor(data, hdr, class_idx, le);
        let method_name =
            read_string(data, hdr.string_ids_off, hdr.string_ids_size, name_idx, le);

        let return_type = {
            let proto_off = hdr.proto_ids_off as usize + proto_idx as usize * 12;
            if proto_off + 8 <= data.len() {
                let ret_idx = if le {
                    u32::from_le_bytes([
                        data[proto_off + 4],
                        data[proto_off + 5],
                        data[proto_off + 6],
                        data[proto_off + 7],
                    ])
                } else {
                    u32::from_be_bytes([
                        data[proto_off + 4],
                        data[proto_off + 5],
                        data[proto_off + 6],
                        data[proto_off + 7],
                    ])
                };
                read_type_descriptor(data, hdr, ret_idx, le)
            } else {
                "(?)".to_string()
            }
        };

        out.push_str(&format!(
            "{:<48} {:<25} {}\n",
            truncate(&class_name, 46),
            truncate(&method_name, 23),
            return_type
        ));
    }

    if hdr.method_ids_size as usize > limit {
        out.push_str(&format!(
            "\n... {} more methods\n",
            hdr.method_ids_size as usize - limit
        ));
    }

    out
}

fn action_strings(data: &[u8], hdr: &DexHeader, le: bool, limit: usize) -> String {
    let mut out = String::new();
    out.push_str("=== String Pool ===\n\n");

    let count = (hdr.string_ids_size as usize).min(limit);
    out.push_str(&format!("Total strings: {}", hdr.string_ids_size));
    if hdr.string_ids_size as usize > limit {
        out.push_str(&format!(" (showing first {})", limit));
    }
    out.push_str("\n\n");

    for i in 0..count {
        let s = read_string(data, hdr.string_ids_off, hdr.string_ids_size, i as u32, le);
        if !s.is_empty() {
            let display = s
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            let display = if display.len() > 120 {
                format!("{}...", &display[..117])
            } else {
                display
            };
            out.push_str(&format!("{:>7}: {}\n", i, display));
        }
    }

    out
}

fn action_imports(data: &[u8], hdr: &DexHeader, le: bool) -> String {
    let mut out = String::new();
    out.push_str("=== Referenced Types ===\n\n");

    let mut android: Vec<String> = Vec::new();
    let mut java_std: Vec<String> = Vec::new();
    let mut third_party: Vec<String> = Vec::new();

    for i in 0..hdr.type_ids_size {
        let desc = read_type_descriptor(data, hdr, i, le);
        // Skip primitives and array-of-primitive
        if desc.len() <= 1 {
            continue;
        }
        // Strip array suffixes for categorization
        let base = desc.trim_end_matches("[]");
        if base.len() <= 1 {
            continue;
        }

        if base.starts_with("android.")
            || base.starts_with("com.android.")
            || base.starts_with("dalvik.")
            || base.starts_with("libcore.")
        {
            android.push(desc);
        } else if base.starts_with("java.")
            || base.starts_with("javax.")
            || base.starts_with("sun.")
        {
            java_std.push(desc);
        } else {
            third_party.push(desc);
        }
    }

    android.sort();
    java_std.sort();
    third_party.sort();

    let print_section = |out: &mut String, label: &str, items: &[String]| {
        if items.is_empty() {
            return;
        }
        out.push_str(&format!("{} ({}):\n", label, items.len()));
        for c in items.iter().take(60) {
            out.push_str(&format!("  {}\n", c));
        }
        if items.len() > 60 {
            out.push_str(&format!("  ... {} more\n", items.len() - 60));
        }
        out.push('\n');
    };

    print_section(&mut out, "Android Framework", &android);
    print_section(&mut out, "Java Standard Library", &java_std);
    print_section(&mut out, "App / Third-party", &third_party);

    if android.is_empty() && java_std.is_empty() && third_party.is_empty() {
        out.push_str("(no type references found)\n");
    }

    out
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let bytes = if let Some(file_path) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read(file_path)
            .map_err(|e| format!("cannot read file '{}': {}", file_path, e))?
    } else if let Some(hex_str) = args.get("hex").and_then(|v| v.as_str()) {
        let clean: String = hex_str.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if clean.len() % 2 != 0 {
            return Err("hex string has odd length".to_string());
        }
        (0..clean.len() / 2)
            .map(|i| {
                u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        return Err(
            "provide 'file' (path to .dex file) or 'hex' (hex-encoded DEX bytes)".to_string(),
        );
    };

    let (hdr, le) = parse_header(&bytes)?;

    match action {
        "info" => Ok(action_info(&bytes, &hdr, le)),
        "classes" => Ok(action_classes(&bytes, &hdr, le, limit)),
        "methods" => Ok(action_methods(&bytes, &hdr, le, limit)),
        "strings" => Ok(action_strings(&bytes, &hdr, le, limit)),
        "imports" | "types" => Ok(action_imports(&bytes, &hdr, le)),
        _ => Err(format!(
            "unknown action '{}'; use: info, classes, methods, strings, imports",
            action
        )),
    }
}
