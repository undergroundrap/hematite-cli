use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "name": "font_tools",
        "description": "Inspect TTF, OTF, and WOFF/WOFF2 font files without external tools. \
    Actions: info (default — family, style, version, copyright, license, glyph count, tables), \
    names (all name records — IDs 0-22 with human labels), \
    tables (OpenType table directory with tag, offset, length), \
    chars (Unicode character coverage summary from cmap). \
    Pass file (path to .ttf/.otf/.woff/.woff2) or hex (hex-encoded bytes). \
    Example: font_tools(file: 'font.ttf') or font_tools(action: 'names', file: 'font.otf') or font_tools(action: 'tables', hex: '0001...').",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "info|names|tables|chars" },
                "file": { "type": "string", "description": "Path to TTF, OTF, WOFF, or WOFF2 file" },
                "hex": { "type": "string", "description": "Hex-encoded font bytes" }
            },
            "required": []
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let bytes = match get_bytes(args) {
        Some(b) => b,
        None => {
            return Ok("Error: provide 'file' (path to TTF/OTF/WOFF) or 'hex' bytes.".to_string())
        }
    };

    if bytes.len() < 12 {
        return Ok("Error: file too short to be a valid font.".to_string());
    }

    let font_data = match unwrap_woff(&bytes) {
        Ok(d) => d,
        Err(e) => return Ok(format!("Error: {}", e)),
    };

    if font_data.len() < 12 {
        return Ok("Error: font data too short after unwrapping.".to_string());
    }

    let sfnt_version = read_u32(&font_data, 0);
    match sfnt_version {
        0x00010000 | 0x4F54544F | 0x74727565 | 0x74797031 => {} // TTF, OTF CFF, true, typ1
        _ => {
            return Ok(format!(
                "Error: unrecognised SFNT version 0x{:08X}. Expected 0x00010000 (TTF) or 0x4F54544F (OTF/CFF).",
                sfnt_version
            ))
        }
    }

    let num_tables = read_u16(&font_data, 4) as usize;
    if 12 + num_tables * 16 > font_data.len() {
        return Ok("Error: table directory extends beyond file.".to_string());
    }

    let tables = parse_table_dir(&font_data, num_tables);

    Ok(match action {
        "names" => format_names(&font_data, &tables),
        "tables" => format_tables(&font_data, &tables, sfnt_version),
        "chars" => format_chars(&font_data, &tables),
        _ => format_info(&font_data, &tables, sfnt_version),
    })
}

fn get_bytes(args: &Value) -> Option<Vec<u8>> {
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read(path).ok();
    }
    if let Some(hex) = args.get("hex").and_then(|v| v.as_str()) {
        let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        let b = clean
            .as_bytes()
            .chunks(2)
            .filter_map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap_or(""), 16).ok())
            .collect();
        return Some(b);
    }
    None
}

fn unwrap_woff(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < 4 {
        return Err("file too short".to_string());
    }
    let sig = read_u32(bytes, 0);
    match sig {
        0x774F4646 => {
            // WOFF: header says SFNT flavor at offset 4, but the table data is raw SFNT
            // WOFF wraps the tables; we re-assemble a minimal SFNT from the compressed tables.
            // Simple approach: extract the SFNT data from WOFF by rebuilding the table dir.
            // WOFF header: sig(4) flavor(4) length(4) numTables(2) reserved(2) totalSfntSize(4) ...
            if bytes.len() < 48 {
                return Err("WOFF header truncated".to_string());
            }
            let flavor = read_u32(bytes, 4);
            let num_tables = read_u16(bytes, 12) as usize;
            // Rebuild a fake SFNT header + table dir + raw table data.
            // Each WOFF table record: tag(4) offset(4) compLength(4) origLength(4) origChecksum(4) = 20 bytes
            let woff_dir_off = 44usize; // WOFF header is 44 bytes
            let mut out: Vec<u8> = Vec::new();
            // SFNT header
            out.extend_from_slice(&flavor.to_be_bytes()); // sfntVersion
            let search_range = largest_power_of_2_leq(num_tables as u32) * 16;
            out.extend_from_slice(&(num_tables as u16).to_be_bytes());
            out.extend_from_slice(&(search_range as u16).to_be_bytes());
            let entry_selector = (largest_power_of_2_leq(num_tables as u32) as f64).log2() as u16;
            out.extend_from_slice(&entry_selector.to_be_bytes());
            out.extend_from_slice(&((num_tables as u16) * 16 - search_range as u16).to_be_bytes());
            // Table directory placeholder (filled after we know offsets)
            let table_dir_off = out.len();
            out.resize(out.len() + num_tables * 16, 0);
            // Append each table's data
            for i in 0..num_tables {
                let rec = woff_dir_off + i * 20;
                if rec + 20 > bytes.len() {
                    break;
                }
                let tag = read_u32(bytes, rec);
                let comp_off = read_u32(bytes, rec + 4) as usize;
                let comp_len = read_u32(bytes, rec + 8) as usize;
                let orig_len = read_u32(bytes, rec + 12) as usize;
                let checksum = read_u32(bytes, rec + 16);
                // Align to 4 bytes
                while out.len() % 4 != 0 {
                    out.push(0);
                }
                let table_off = out.len() as u32;
                if comp_len == orig_len {
                    // Uncompressed
                    if comp_off + comp_len <= bytes.len() {
                        out.extend_from_slice(&bytes[comp_off..comp_off + comp_len]);
                    }
                } else {
                    // zlib-compressed — we can't decompress without a dep, so just note size
                    out.extend(vec![0u8; orig_len]);
                }
                // Fill in table dir entry
                let entry = table_dir_off + i * 16;
                if entry + 16 <= out.len() {
                    out[entry..entry + 4].copy_from_slice(&tag.to_be_bytes());
                    out[entry + 4..entry + 8].copy_from_slice(&checksum.to_be_bytes());
                    out[entry + 8..entry + 12].copy_from_slice(&table_off.to_be_bytes());
                    out[entry + 12..entry + 16].copy_from_slice(&(orig_len as u32).to_be_bytes());
                }
            }
            Ok(out)
        }
        0x774F4632 => {
            // WOFF2 uses Brotli compression — cannot decompress without a dep.
            // Return the raw bytes; we'll detect the magic and report gracefully.
            Ok(bytes.to_vec())
        }
        _ => Ok(bytes.to_vec()),
    }
}

fn largest_power_of_2_leq(n: u32) -> u32 {
    if n == 0 {
        return 1;
    }
    let mut p = 1u32;
    while p * 2 <= n {
        p *= 2;
    }
    p
}

#[derive(Clone, Debug)]
struct TableEntry {
    tag: [u8; 4],
    checksum: u32,
    offset: u32,
    length: u32,
}

fn parse_table_dir(data: &[u8], num: usize) -> Vec<TableEntry> {
    let mut tables = Vec::with_capacity(num);
    for i in 0..num {
        let base = 12 + i * 16;
        if base + 16 > data.len() {
            break;
        }
        let mut tag = [0u8; 4];
        tag.copy_from_slice(&data[base..base + 4]);
        let checksum = read_u32(data, base + 4);
        let offset = read_u32(data, base + 8);
        let length = read_u32(data, base + 12);
        tables.push(TableEntry {
            tag,
            checksum,
            offset,
            length,
        });
    }
    tables
}

fn table_data<'a>(data: &'a [u8], tables: &[TableEntry], tag: &[u8; 4]) -> Option<&'a [u8]> {
    for t in tables {
        if &t.tag == tag {
            let off = t.offset as usize;
            let len = t.length as usize;
            if off + len <= data.len() {
                return Some(&data[off..off + len]);
            }
        }
    }
    None
}

fn read_u8(data: &[u8], off: usize) -> u8 {
    data.get(off).copied().unwrap_or(0)
}

fn read_u16(data: &[u8], off: usize) -> u16 {
    if off + 2 > data.len() {
        return 0;
    }
    u16::from_be_bytes([data[off], data[off + 1]])
}

fn read_u32(data: &[u8], off: usize) -> u32 {
    if off + 4 > data.len() {
        return 0;
    }
    u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

fn read_i16(data: &[u8], off: usize) -> i16 {
    i16::from_be_bytes([
        data.get(off).copied().unwrap_or(0),
        data.get(off + 1).copied().unwrap_or(0),
    ])
}

// ── Name table parsing ──────────────────────────────────────────────────────

fn name_id_label(id: u16) -> &'static str {
    match id {
        0 => "Copyright",
        1 => "Font Family",
        2 => "Subfamily (Weight/Style)",
        3 => "Unique Font ID",
        4 => "Full Name",
        5 => "Version",
        6 => "PostScript Name",
        7 => "Trademark",
        8 => "Manufacturer",
        9 => "Designer",
        10 => "Description",
        11 => "Vendor URL",
        12 => "Designer URL",
        13 => "License Description",
        14 => "License URL",
        16 => "Typographic Family",
        17 => "Typographic Subfamily",
        18 => "Compatible Full Name (Mac)",
        19 => "Sample Text",
        21 => "WWS Family",
        22 => "WWS Subfamily",
        25 => "Variations PostScript Prefix",
        _ => "Name",
    }
}

struct NameRecord {
    platform: u16,
    encoding: u16,
    language: u16,
    name_id: u16,
    string: String,
}

fn parse_name_table(name_data: &[u8]) -> Vec<NameRecord> {
    if name_data.len() < 6 {
        return vec![];
    }
    let format = read_u16(name_data, 0);
    let count = read_u16(name_data, 2) as usize;
    let storage_off = read_u16(name_data, 4) as usize;
    if format > 1 || count == 0 {
        return vec![];
    }
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let base = 6 + i * 12;
        if base + 12 > name_data.len() {
            break;
        }
        let platform = read_u16(name_data, base);
        let encoding = read_u16(name_data, base + 2);
        let language = read_u16(name_data, base + 4);
        let name_id = read_u16(name_data, base + 6);
        let length = read_u16(name_data, base + 8) as usize;
        let offset = read_u16(name_data, base + 10) as usize;
        let abs = storage_off + offset;
        if abs + length > name_data.len() {
            continue;
        }
        let raw = &name_data[abs..abs + length];
        let s = decode_name_string(platform, encoding, raw);
        records.push(NameRecord {
            platform,
            encoding,
            language,
            name_id,
            string: s,
        });
    }
    records
}

fn decode_name_string(platform: u16, encoding: u16, raw: &[u8]) -> String {
    // Platform 3 (Windows) encoding 1 = UTF-16BE; Platform 1 (Mac) = Mac Roman
    if platform == 3 && (encoding == 0 || encoding == 1) {
        let chars: Vec<u16> = raw
            .chunks(2)
            .map(|c| {
                u16::from_be_bytes([
                    c.get(0).copied().unwrap_or(0),
                    c.get(1).copied().unwrap_or(0),
                ])
            })
            .collect();
        return String::from_utf16_lossy(&chars).trim().to_string();
    }
    // Mac Roman or Platform 0 (Unicode)
    if platform == 0 || (platform == 1 && encoding == 0) {
        // Best-effort ASCII-compatible
        return raw
            .iter()
            .map(|&b| {
                if b >= 0x20 && b < 0x80 {
                    b as char
                } else {
                    '?'
                }
            })
            .collect::<String>()
            .trim()
            .to_string();
    }
    String::from_utf8_lossy(raw).trim().to_string()
}

fn best_name(records: &[NameRecord], name_id: u16) -> Option<String> {
    // Prefer Windows platform (3) UTF-16, then anything with a non-empty string
    for r in records {
        if r.name_id == name_id && r.platform == 3 && !r.string.is_empty() {
            return Some(r.string.clone());
        }
    }
    for r in records {
        if r.name_id == name_id && !r.string.is_empty() {
            return Some(r.string.clone());
        }
    }
    None
}

// ── maxp / head tables ──────────────────────────────────────────────────────

fn glyph_count(data: &[u8], tables: &[TableEntry]) -> u32 {
    if let Some(maxp) = table_data(data, tables, b"maxp") {
        if maxp.len() >= 6 {
            return read_u16(maxp, 4) as u32;
        }
    }
    0
}

fn head_units_per_em(data: &[u8], tables: &[TableEntry]) -> u16 {
    if let Some(head) = table_data(data, tables, b"head") {
        if head.len() >= 20 {
            return read_u16(head, 18);
        }
    }
    0
}

fn head_mac_style(data: &[u8], tables: &[TableEntry]) -> u16 {
    if let Some(head) = table_data(data, tables, b"head") {
        if head.len() >= 46 {
            return read_u16(head, 44);
        }
    }
    0
}

fn os2_fs_type(data: &[u8], tables: &[TableEntry]) -> Option<u16> {
    if let Some(os2) = table_data(data, tables, b"OS/2") {
        if os2.len() >= 10 {
            return Some(read_u16(os2, 8));
        }
    }
    None
}

fn decode_fs_type(fs: u16) -> &'static str {
    let bits = fs & 0x000F;
    match bits {
        0 => "Installable embedding",
        2 => "Restricted (no embedding)",
        4 => "Preview & Print only",
        8 => "Editable embedding",
        _ => "Unknown embedding rights",
    }
}

fn os2_weight_class(data: &[u8], tables: &[TableEntry]) -> u16 {
    if let Some(os2) = table_data(data, tables, b"OS/2") {
        if os2.len() >= 6 {
            return read_u16(os2, 4);
        }
    }
    0
}

fn weight_label(w: u16) -> &'static str {
    match w {
        100 => "Thin",
        200 => "Extra Light",
        300 => "Light",
        400 => "Regular",
        500 => "Medium",
        600 => "SemiBold",
        700 => "Bold",
        800 => "ExtraBold",
        900 => "Black",
        _ => "Custom",
    }
}

fn mac_style_str(style: u16) -> String {
    let mut parts = Vec::new();
    if style & 0x01 != 0 {
        parts.push("Bold");
    }
    if style & 0x02 != 0 {
        parts.push("Italic");
    }
    if style & 0x04 != 0 {
        parts.push("Underline");
    }
    if style & 0x08 != 0 {
        parts.push("Outline");
    }
    if style & 0x10 != 0 {
        parts.push("Shadow");
    }
    if parts.is_empty() {
        "Regular".to_string()
    } else {
        parts.join(", ")
    }
}

// ── cmap character coverage ─────────────────────────────────────────────────

fn cmap_coverage(data: &[u8], tables: &[TableEntry]) -> (usize, bool, bool, bool) {
    let cmap_data = match table_data(data, tables, b"cmap") {
        Some(d) => d,
        None => return (0, false, false, false),
    };
    if cmap_data.len() < 4 {
        return (0, false, false, false);
    }
    let num_subtables = read_u16(cmap_data, 2) as usize;
    let mut best_format4: Option<usize> = None;
    let mut best_format12: Option<usize> = None;

    for i in 0..num_subtables {
        let base = 4 + i * 8;
        if base + 8 > cmap_data.len() {
            break;
        }
        let _platform = read_u16(cmap_data, base);
        let _encoding = read_u16(cmap_data, base + 2);
        let offset = read_u32(cmap_data, base + 4) as usize;
        if offset + 2 > cmap_data.len() {
            continue;
        }
        let fmt = read_u16(cmap_data, offset);
        match fmt {
            4 => {
                if best_format4.is_none() {
                    best_format4 = Some(offset);
                }
            }
            12 => {
                if best_format12.is_none() {
                    best_format12 = Some(offset);
                }
            }
            _ => {}
        }
    }

    let mut count = 0usize;
    let mut has_latin = false;
    let mut has_greek = false;
    let mut has_cjk = false;

    if let Some(off) = best_format12.or(best_format4) {
        let fmt = read_u16(cmap_data, off);
        if fmt == 12 && off + 16 <= cmap_data.len() {
            let ngroups = read_u32(cmap_data, off + 12) as usize;
            for g in 0..ngroups {
                let base = off + 16 + g * 12;
                if base + 12 > cmap_data.len() {
                    break;
                }
                let start = read_u32(cmap_data, base);
                let end = read_u32(cmap_data, base + 4);
                count += (end - start + 1) as usize;
                if start <= 0x007F {
                    has_latin = true;
                }
                if start >= 0x0370 && end <= 0x03FF {
                    has_greek = true;
                }
                if start <= 0x9FFF && end >= 0x4E00 {
                    has_cjk = true;
                }
            }
        } else if fmt == 4 && off + 14 <= cmap_data.len() {
            let seg_count = (read_u16(cmap_data, off + 6) as usize) / 2;
            let end_off = off + 14;
            for s in 0..seg_count {
                let end_code = read_u16(cmap_data, end_off + s * 2) as u32;
                let start_code = read_u16(cmap_data, end_off + seg_count * 2 + 2 + s * 2) as u32;
                if start_code == 0xFFFF {
                    break;
                }
                count += (end_code - start_code + 1) as usize;
                if start_code <= 0x007F {
                    has_latin = true;
                }
                if start_code >= 0x0370 && end_code <= 0x03FF {
                    has_greek = true;
                }
                if start_code <= 0x9FFF && end_code >= 0x4E00 {
                    has_cjk = true;
                }
            }
        }
    }

    (count, has_latin, has_greek, has_cjk)
}

// ── Formatters ──────────────────────────────────────────────────────────────

fn format_info(data: &[u8], tables: &[TableEntry], sfnt_version: u32) -> String {
    // WOFF2 check
    if data.len() >= 4 && read_u32(data, 0) == 0x774F4632 {
        return "WOFF2 detected. WOFF2 uses Brotli compression which requires an external library.\nConvert to TTF/OTF first: use a tool like woff2_decompress or fonttools.\n\nFile contains WOFF2 data — metadata extraction requires decompression.".to_string();
    }

    let name_data = table_data(data, tables, b"name");
    let records: Vec<NameRecord> = name_data.map(parse_name_table).unwrap_or_default();

    let flavor = match sfnt_version {
        0x00010000 => "TrueType (TTF)",
        0x4F54544F => "OpenType CFF (OTF)",
        0x74727565 => "TrueType (Apple true)",
        _ => "TrueType (unknown flavor)",
    };

    let family = best_name(&records, 1).unwrap_or_else(|| "Unknown".to_string());
    let subfamily = best_name(&records, 2).unwrap_or_else(|| "Regular".to_string());
    let full_name = best_name(&records, 4);
    let version = best_name(&records, 5).unwrap_or_else(|| "Unknown".to_string());
    let ps_name = best_name(&records, 6);
    let copyright = best_name(&records, 0);
    let license = best_name(&records, 13);
    let manufacturer = best_name(&records, 8);

    let glyphs = glyph_count(data, tables);
    let upm = head_units_per_em(data, tables);
    let mac_style = head_mac_style(data, tables);
    let weight = os2_weight_class(data, tables);
    let fs_type = os2_fs_type(data, tables);
    let (cmap_count, has_latin, has_greek, has_cjk) = cmap_coverage(data, tables);

    let mut out = String::from("Font Information\n\n");

    out.push_str(&format!("  {:24} {}\n", "Format:", flavor));
    out.push_str(&format!("  {:24} {}\n", "Family:", family));
    out.push_str(&format!("  {:24} {}\n", "Subfamily:", subfamily));
    if let Some(ref n) = full_name {
        out.push_str(&format!("  {:24} {}\n", "Full Name:", n));
    }
    if let Some(ref n) = ps_name {
        out.push_str(&format!("  {:24} {}\n", "PostScript Name:", n));
    }
    out.push_str(&format!("  {:24} {}\n", "Version:", version));
    if let Some(ref m) = manufacturer {
        out.push_str(&format!("  {:24} {}\n", "Manufacturer:", m));
    }
    if glyphs > 0 {
        out.push_str(&format!("  {:24} {}\n", "Glyphs:", glyphs));
    }
    if upm > 0 {
        out.push_str(&format!("  {:24} {}\n", "Units Per Em:", upm));
    }
    if weight > 0 {
        out.push_str(&format!(
            "  {:24} {} ({})\n",
            "Weight Class:",
            weight,
            weight_label(weight)
        ));
    }
    if mac_style > 0 || weight == 400 {
        out.push_str(&format!(
            "  {:24} {}\n",
            "Style Flags:",
            mac_style_str(mac_style)
        ));
    }
    if let Some(fs) = fs_type {
        out.push_str(&format!(
            "  {:24} {}\n",
            "Embedding Rights:",
            decode_fs_type(fs)
        ));
    }
    out.push_str(&format!("  {:24} {}\n", "Tables:", tables.len()));

    out.push_str("\nCharacter Coverage\n");
    if cmap_count > 0 {
        out.push_str(&format!("  {:24} {}\n", "Mapped Codepoints:", cmap_count));
        let mut scripts = Vec::new();
        if has_latin {
            scripts.push("Latin");
        }
        if has_greek {
            scripts.push("Greek");
        }
        if has_cjk {
            scripts.push("CJK");
        }
        if !scripts.is_empty() {
            out.push_str(&format!(
                "  {:24} {}\n",
                "Scripts Detected:",
                scripts.join(", ")
            ));
        }
    } else {
        out.push_str("  No cmap table or unrecognised format.\n");
    }

    if let Some(ref c) = copyright {
        out.push_str("\nLegal\n");
        let short_copy = if c.len() > 120 {
            format!("{}...", &c[..120])
        } else {
            c.clone()
        };
        out.push_str(&format!("  {:24} {}\n", "Copyright:", short_copy));
    }
    if let Some(ref l) = license {
        let short_lic = if l.len() > 120 {
            format!("{}...", &l[..120])
        } else {
            l.clone()
        };
        out.push_str(&format!("  {:24} {}\n", "License:", short_lic));
    }

    out
}

fn format_names(data: &[u8], tables: &[TableEntry]) -> String {
    let name_data = match table_data(data, tables, b"name") {
        Some(d) => d,
        None => return "No 'name' table found in this font.".to_string(),
    };
    let records = parse_name_table(name_data);
    if records.is_empty() {
        return "Name table is empty or unreadable.".to_string();
    }

    let mut seen: std::collections::HashMap<u16, String> = std::collections::HashMap::new();
    for r in &records {
        if !r.string.is_empty() {
            seen.entry(r.name_id).or_insert_with(|| r.string.clone());
        }
    }

    let mut out = format!("Font Name Records  ({} unique IDs)\n\n", seen.len());
    let mut ids: Vec<u16> = seen.keys().cloned().collect();
    ids.sort_unstable();
    for id in ids {
        let label = name_id_label(id);
        let val = &seen[&id];
        let short = if val.len() > 100 {
            format!("{}...", &val[..100])
        } else {
            val.clone()
        };
        out.push_str(&format!("  {:3}  {:32} {}\n", id, label, short));
    }
    out
}

fn format_tables(data: &[u8], tables: &[TableEntry], sfnt_version: u32) -> String {
    let flavor = match sfnt_version {
        0x00010000 => "TrueType",
        0x4F54544F => "OpenType/CFF",
        _ => "TrueType",
    };
    let mut out = format!(
        "OpenType Table Directory — {} ({} tables)\n\n",
        flavor,
        tables.len()
    );
    out.push_str(&format!(
        "  {:8}  {:10}  {:10}  {}\n",
        "Tag", "Offset", "Length", "Description"
    ));
    out.push_str(&format!(
        "  {:8}  {:10}  {:10}  {}\n",
        "───", "──────", "──────", "───────────"
    ));
    for t in tables {
        let tag_str = String::from_utf8_lossy(&t.tag).to_string();
        let desc = table_desc(&t.tag);
        out.push_str(&format!(
            "  {:8}  {:10}  {:10}  {}\n",
            tag_str, t.offset, t.length, desc
        ));
    }
    out
}

fn table_desc(tag: &[u8; 4]) -> &'static str {
    match tag {
        b"cmap" => "Character to glyph mapping",
        b"glyf" => "Glyph data (TrueType outlines)",
        b"head" => "Font header",
        b"hhea" => "Horizontal header",
        b"hmtx" => "Horizontal metrics",
        b"loca" => "Index to glyph locations",
        b"maxp" => "Maximum profile",
        b"name" => "Naming table",
        b"post" => "PostScript name mapping",
        b"OS/2" => "OS/2 and Windows metrics",
        b"cvt " => "Control value table",
        b"fpgm" => "Font program",
        b"prep" => "Control value program",
        b"CFF " => "Compact Font Format (Type 2)",
        b"GDEF" => "Glyph definition data",
        b"GPOS" => "Glyph positioning data",
        b"GSUB" => "Glyph substitution data",
        b"kern" => "Kerning",
        b"BASE" => "Baseline data",
        b"JSTF" => "Justification data",
        b"MATH" => "Mathematical typesetting",
        b"COLR" => "Color table (layered glyphs)",
        b"CPAL" => "Color palette table",
        b"SVG " => "Scalable Vector Graphics table",
        b"sbix" => "Standard bitmap graphics",
        b"CBDT" => "Color bitmap data",
        b"CBLC" => "Color bitmap location data",
        b"vhea" => "Vertical header",
        b"vmtx" => "Vertical metrics",
        b"stat" => "Style attributes",
        b"fvar" => "Font variations (variable font)",
        b"gvar" => "Glyph variations",
        b"HVAR" => "Horizontal metrics variations",
        b"MVAR" => "Metrics variations",
        _ => "Font table",
    }
}

fn format_chars(data: &[u8], tables: &[TableEntry]) -> String {
    let cmap_data = match table_data(data, tables, b"cmap") {
        Some(d) => d,
        None => return "No 'cmap' table found — cannot determine character coverage.".to_string(),
    };
    if cmap_data.len() < 4 {
        return "cmap table too short.".to_string();
    }
    let version = read_u16(cmap_data, 0);
    let num = read_u16(cmap_data, 2) as usize;

    let mut out = format!(
        "Character Map  (cmap version {}, {} subtable(s))\n\n",
        version, num
    );

    for i in 0..num {
        let base = 4 + i * 8;
        if base + 8 > cmap_data.len() {
            break;
        }
        let platform = read_u16(cmap_data, base);
        let encoding = read_u16(cmap_data, base + 2);
        let offset = read_u32(cmap_data, base + 4) as usize;
        let platform_name = match platform {
            0 => "Unicode",
            1 => "Macintosh",
            3 => "Windows",
            _ => "Other",
        };
        let fmt = if offset + 2 <= cmap_data.len() {
            read_u16(cmap_data, offset)
        } else {
            99
        };
        out.push_str(&format!(
            "  Subtable {}: Platform={} ({}) Encoding={} Format={}\n",
            i, platform, platform_name, encoding, fmt
        ));
    }

    let (count, has_latin, has_greek, has_cjk) = cmap_coverage(data, tables);
    out.push('\n');
    if count > 0 {
        out.push_str(&format!("  Total mapped codepoints: {}\n", count));
        let mut scripts = Vec::new();
        if has_latin {
            scripts.push("Basic Latin (U+0000–U+007F)");
        }
        if has_greek {
            scripts.push("Greek (U+0370–U+03FF)");
        }
        if has_cjk {
            scripts.push("CJK Unified Ideographs");
        }
        for s in &scripts {
            out.push_str(&format!("  ✓ {}\n", s));
        }
    } else {
        out.push_str("  Could not enumerate codepoints (no format 4 or 12 subtable found).\n");
    }
    out
}
