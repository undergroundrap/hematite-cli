use serde_json::Value;
use std::fs;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    let data = get_bytes(args)?;

    validate_magic(&data)?;

    match action {
        "info" | "" => action_info(&data),
        "sections" => action_sections(&data),
        "imports" => action_imports(&data),
        "exports" => action_exports(&data),
        other => Err(format!(
            "wasm_tools: unknown action '{other}'. Valid: info, sections, imports, exports"
        )),
    }
}

// ── Input resolution ─────────────────────────────────────────────────────────

fn get_bytes(args: &Value) -> Result<Vec<u8>, String> {
    if let Some(path) = args
        .get("file")
        .or_else(|| args.get("path"))
        .and_then(|v| v.as_str())
    {
        return fs::read(path).map_err(|e| format!("wasm_tools: cannot read '{path}': {e}"));
    }
    if let Some(hex) = args.get("hex").and_then(|v| v.as_str()) {
        return decode_hex(hex);
    }
    Err("wasm_tools: provide 'file' (path to a .wasm file) or 'hex' (hex-encoded bytes)".into())
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.len() % 2 != 0 {
        return Err("wasm_tools: hex string must have an even number of characters".into());
    }
    (0..s.len() / 2)
        .map(|i| {
            u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|_| format!("wasm_tools: invalid hex byte at position {}", i * 2))
        })
        .collect()
}

// ── Magic / version check ────────────────────────────────────────────────────

fn validate_magic(data: &[u8]) -> Result<(), String> {
    if data.len() < 8 {
        return Err("wasm_tools: file too small to be a valid WebAssembly module".into());
    }
    if &data[0..4] != b"\x00asm" {
        return Err(format!(
            "wasm_tools: not a WebAssembly module (magic bytes: {:02x} {:02x} {:02x} {:02x}; expected 00 61 73 6d)",
            data[0], data[1], data[2], data[3]
        ));
    }
    Ok(())
}

fn wasm_version(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[4], data[5], data[6], data[7]])
}

// ── LEB128 decoder ───────────────────────────────────────────────────────────

fn read_uleb128(data: &[u8], pos: &mut usize) -> Option<u32> {
    let mut result: u32 = 0;
    let mut shift = 0u32;
    loop {
        if *pos >= data.len() || shift > 28 {
            return None;
        }
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7f) as u32) << shift;
        if byte & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
    }
}

fn read_bytes_vec(data: &[u8], pos: &mut usize) -> Option<Vec<u8>> {
    let len = read_uleb128(data, pos)? as usize;
    if *pos + len > data.len() {
        return None;
    }
    let v = data[*pos..*pos + len].to_vec();
    *pos += len;
    Some(v)
}

fn read_name(data: &[u8], pos: &mut usize) -> Option<String> {
    let bytes = read_bytes_vec(data, pos)?;
    Some(String::from_utf8_lossy(&bytes).to_string())
}

// ── Section parsing ──────────────────────────────────────────────────────────

#[derive(Debug)]
struct Section {
    id: u8,
    name: &'static str,
    size: u32,
    offset: usize,
}

fn section_name(id: u8) -> &'static str {
    match id {
        0 => "Custom",
        1 => "Type",
        2 => "Import",
        3 => "Function",
        4 => "Table",
        5 => "Memory",
        6 => "Global",
        7 => "Export",
        8 => "Start",
        9 => "Element",
        10 => "Code",
        11 => "Data",
        12 => "DataCount",
        _ => "Unknown",
    }
}

fn parse_sections(data: &[u8]) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut pos = 8; // skip magic + version

    while pos < data.len() {
        if pos >= data.len() {
            break;
        }
        let id = data[pos];
        pos += 1;
        let mut size_pos = pos;
        let size = match read_uleb128(data, &mut size_pos) {
            Some(s) => s,
            None => break,
        };
        let offset = size_pos;
        sections.push(Section {
            id,
            name: section_name(id),
            size,
            offset,
        });
        pos = size_pos + size as usize;
    }

    sections
}

fn find_section(sections: &[Section], id: u8) -> Option<&Section> {
    sections.iter().find(|s| s.id == id)
}

// ── Type section: function signatures ───────────────────────────────────────

fn valtype_name(t: u8) -> &'static str {
    match t {
        0x7f => "i32",
        0x7e => "i64",
        0x7d => "f32",
        0x7c => "f64",
        0x7b => "v128",
        0x70 => "funcref",
        0x6f => "externref",
        _ => "?",
    }
}

#[derive(Debug, Clone)]
struct FuncType {
    params: Vec<u8>,
    results: Vec<u8>,
}

fn parse_type_section(data: &[u8], sec: &Section) -> Vec<FuncType> {
    let mut types = Vec::new();
    let mut pos = sec.offset;
    let count = match read_uleb128(data, &mut pos) {
        Some(c) => c,
        None => return types,
    };

    for _ in 0..count {
        if pos >= data.len() || data[pos] != 0x60 {
            break;
        }
        pos += 1;

        let param_count = match read_uleb128(data, &mut pos) {
            Some(c) => c,
            None => break,
        };
        let mut params = Vec::new();
        for _ in 0..param_count {
            if pos >= data.len() {
                break;
            }
            params.push(data[pos]);
            pos += 1;
        }

        let result_count = match read_uleb128(data, &mut pos) {
            Some(c) => c,
            None => break,
        };
        let mut results = Vec::new();
        for _ in 0..result_count {
            if pos >= data.len() {
                break;
            }
            results.push(data[pos]);
            pos += 1;
        }

        types.push(FuncType { params, results });
    }

    types
}

fn format_func_sig(ft: &FuncType) -> String {
    let params: Vec<_> = ft.params.iter().map(|&t| valtype_name(t)).collect();
    let results: Vec<_> = ft.results.iter().map(|&t| valtype_name(t)).collect();
    let p = if params.is_empty() {
        "()".to_string()
    } else {
        format!("({})", params.join(", "))
    };
    let r = if results.is_empty() {
        "void".to_string()
    } else {
        results.join(", ")
    };
    format!("{p} -> {r}")
}

// ── Import section ───────────────────────────────────────────────────────────

#[derive(Debug)]
struct Import {
    module: String,
    name: String,
    kind: &'static str,
    type_idx: Option<u32>,
}

fn import_kind(tag: u8) -> &'static str {
    match tag {
        0 => "func",
        1 => "table",
        2 => "memory",
        3 => "global",
        _ => "?",
    }
}

fn parse_import_section(data: &[u8], sec: &Section) -> Vec<Import> {
    let mut imports = Vec::new();
    let mut pos = sec.offset;
    let count = match read_uleb128(data, &mut pos) {
        Some(c) => c,
        None => return imports,
    };

    for _ in 0..count {
        let module = match read_name(data, &mut pos) {
            Some(n) => n,
            None => break,
        };
        let name = match read_name(data, &mut pos) {
            Some(n) => n,
            None => break,
        };
        if pos >= data.len() {
            break;
        }
        let tag = data[pos];
        pos += 1;
        let kind = import_kind(tag);

        let type_idx = if tag == 0 {
            read_uleb128(data, &mut pos)
        } else {
            // Skip the descriptor (simplified — skip bytes for table/memory/global)
            skip_import_desc(data, &mut pos, tag);
            None
        };

        imports.push(Import {
            module,
            name,
            kind,
            type_idx,
        });
    }

    imports
}

fn skip_import_desc(data: &[u8], pos: &mut usize, tag: u8) {
    match tag {
        1 => {
            // table: reftype + limits
            if *pos < data.len() {
                *pos += 1;
            } // reftype
            skip_limits(data, pos);
        }
        2 => skip_limits(data, pos), // memory limits
        3 => {
            // global: valtype + mutability
            if *pos < data.len() {
                *pos += 1;
            }
            if *pos < data.len() {
                *pos += 1;
            }
        }
        _ => {}
    }
}

fn skip_limits(data: &[u8], pos: &mut usize) {
    if *pos >= data.len() {
        return;
    }
    let flag = data[*pos];
    *pos += 1;
    read_uleb128(data, pos); // min
    if flag & 1 != 0 {
        read_uleb128(data, pos); // max
    }
}

// ── Export section ───────────────────────────────────────────────────────────

#[derive(Debug)]
struct Export {
    name: String,
    kind: &'static str,
    index: u32,
}

fn export_kind(tag: u8) -> &'static str {
    match tag {
        0 => "func",
        1 => "table",
        2 => "memory",
        3 => "global",
        _ => "?",
    }
}

fn parse_export_section(data: &[u8], sec: &Section) -> Vec<Export> {
    let mut exports = Vec::new();
    let mut pos = sec.offset;
    let count = match read_uleb128(data, &mut pos) {
        Some(c) => c,
        None => return exports,
    };

    for _ in 0..count {
        let name = match read_name(data, &mut pos) {
            Some(n) => n,
            None => break,
        };
        if pos >= data.len() {
            break;
        }
        let tag = data[pos];
        pos += 1;
        let index = match read_uleb128(data, &mut pos) {
            Some(i) => i,
            None => break,
        };
        exports.push(Export {
            name,
            kind: export_kind(tag),
            index,
        });
    }

    exports
}

// ── Custom section: name ─────────────────────────────────────────────────────

fn parse_name_section(data: &[u8], sec: &Section) -> Vec<(u32, String)> {
    let mut names = Vec::new();
    let mut pos = sec.offset;

    // Skip the section name ("name")
    if read_name(data, &mut pos).is_none() {
        return names;
    }

    // Subsections: 0=module, 1=function names, 2=local names
    while pos < sec.offset + sec.size as usize {
        if pos >= data.len() {
            break;
        }
        let sub_id = data[pos];
        pos += 1;
        let sub_size = match read_uleb128(data, &mut pos) {
            Some(s) => s as usize,
            None => break,
        };
        let sub_end = pos + sub_size;

        if sub_id == 1 {
            // function name map
            let count = match read_uleb128(data, &mut pos) {
                Some(c) => c,
                None => break,
            };
            for _ in 0..count {
                let idx = match read_uleb128(data, &mut pos) {
                    Some(i) => i,
                    None => break,
                };
                let name = match read_name(data, &mut pos) {
                    Some(n) => n,
                    None => break,
                };
                names.push((idx, name));
            }
        }

        pos = sub_end;
    }

    names
}

// ── Actions ──────────────────────────────────────────────────────────────────

fn action_info(data: &[u8]) -> Result<String, String> {
    let version = wasm_version(data);
    let sections = parse_sections(data);
    let file_size = data.len();

    let func_count = find_section(&sections, 3)
        .and_then(|sec| {
            let mut pos = sec.offset;
            read_uleb128(data, &mut pos)
        })
        .unwrap_or(0);

    let import_count = find_section(&sections, 2)
        .and_then(|sec| {
            let mut pos = sec.offset;
            read_uleb128(data, &mut pos)
        })
        .unwrap_or(0);

    let export_count = find_section(&sections, 7)
        .and_then(|sec| {
            let mut pos = sec.offset;
            read_uleb128(data, &mut pos)
        })
        .unwrap_or(0);

    let type_count = find_section(&sections, 1)
        .and_then(|sec| {
            let mut pos = sec.offset;
            read_uleb128(data, &mut pos)
        })
        .unwrap_or(0);

    let memory_count = find_section(&sections, 5)
        .and_then(|sec| {
            let mut pos = sec.offset;
            read_uleb128(data, &mut pos)
        })
        .unwrap_or(0);

    let global_count = find_section(&sections, 6)
        .and_then(|sec| {
            let mut pos = sec.offset;
            read_uleb128(data, &mut pos)
        })
        .unwrap_or(0);

    let has_start = find_section(&sections, 8).is_some();

    // Try to get module name from custom "name" section
    let module_name = sections
        .iter()
        .filter(|s| s.id == 0)
        .find_map(|sec| {
            let mut pos = sec.offset;
            let name = read_name(data, &mut pos)?;
            if name == "name" {
                // peek for module-name subsection (sub_id 0)
                if pos < data.len() && data[pos] == 0 {
                    pos += 1;
                    read_uleb128(data, &mut pos)?; // sub_size
                                                   // count
                    read_uleb128(data, &mut pos)?;
                    let mname = read_name(data, &mut pos)?;
                    if !mname.is_empty() {
                        return Some(mname);
                    }
                }
            }
            None
        })
        .unwrap_or_default();

    let mut out = String::new();
    out.push_str("── WebAssembly Module Info ──────────────────────────────────────\n");
    if !module_name.is_empty() {
        out.push_str(&format!("Module name:  {module_name}\n"));
    }
    out.push_str(&format!("WASM version: {version}\n"));
    out.push_str(&format!("File size:    {} bytes\n", file_size));
    out.push_str(&format!("Sections:     {}\n", sections.len()));
    out.push('\n');
    out.push_str("── Counts ───────────────────────────────────────────────────────\n");
    out.push_str(&format!("Function types:  {type_count}\n"));
    out.push_str(&format!(
        "Functions:       {} (+ {} imported)\n",
        func_count, import_count
    ));
    out.push_str(&format!("Imports:         {import_count}\n"));
    out.push_str(&format!("Exports:         {export_count}\n"));
    out.push_str(&format!("Memories:        {memory_count}\n"));
    out.push_str(&format!("Globals:         {global_count}\n"));
    if has_start {
        out.push_str("Start entry:     yes\n");
    }
    out.push('\n');
    out.push_str("── Section Overview ─────────────────────────────────────────────\n");
    for sec in &sections {
        out.push_str(&format!(
            "  [{:2}] {:12} {:6} bytes\n",
            sec.id, sec.name, sec.size
        ));
    }
    Ok(out)
}

fn action_sections(data: &[u8]) -> Result<String, String> {
    let sections = parse_sections(data);
    let types = find_section(&sections, 1)
        .map(|s| parse_type_section(data, s))
        .unwrap_or_default();

    let mut out = String::new();
    out.push_str(&format!(
        "{:<4} {:<14} {:<8} {}\n",
        "ID", "Name", "Size", "Details"
    ));
    out.push_str(&"-".repeat(70));
    out.push('\n');

    for sec in &sections {
        let details = match sec.id {
            1 => format!("{} function type(s)", types.len()),
            2 => {
                let n = read_uleb128(data, &mut sec.offset.clone()).unwrap_or(0);
                format!("{n} import(s)")
            }
            3 => {
                let n = read_uleb128(data, &mut sec.offset.clone()).unwrap_or(0);
                format!("{n} function(s)")
            }
            4 => {
                let n = read_uleb128(data, &mut sec.offset.clone()).unwrap_or(0);
                format!("{n} table(s)")
            }
            5 => {
                let n = read_uleb128(data, &mut sec.offset.clone()).unwrap_or(0);
                format!("{n} memory(ies)")
            }
            6 => {
                let n = read_uleb128(data, &mut sec.offset.clone()).unwrap_or(0);
                format!("{n} global(s)")
            }
            7 => {
                let n = read_uleb128(data, &mut sec.offset.clone()).unwrap_or(0);
                format!("{n} export(s)")
            }
            10 => {
                let n = read_uleb128(data, &mut sec.offset.clone()).unwrap_or(0);
                format!("{n} code body(ies)")
            }
            11 => {
                let n = read_uleb128(data, &mut sec.offset.clone()).unwrap_or(0);
                format!("{n} data segment(s)")
            }
            0 => {
                let mut pos = sec.offset;
                let name = read_name(data, &mut pos).unwrap_or_default();
                format!("name: \"{name}\"")
            }
            _ => String::new(),
        };
        out.push_str(&format!(
            "{:<4} {:<14} {:<8} {}\n",
            sec.id, sec.name, sec.size, details
        ));
    }

    // Show function type signatures
    if !types.is_empty() {
        out.push_str("\n── Function Types ───────────────────────────────────────────────\n");
        for (i, ft) in types.iter().enumerate() {
            out.push_str(&format!("  type[{i}]  {}\n", format_func_sig(ft)));
        }
    }

    Ok(out)
}

fn action_imports(data: &[u8]) -> Result<String, String> {
    let sections = parse_sections(data);
    let types = find_section(&sections, 1)
        .map(|s| parse_type_section(data, s))
        .unwrap_or_default();

    let imports = match find_section(&sections, 2) {
        Some(sec) => parse_import_section(data, sec),
        None => return Ok("No import section found.\n".to_string()),
    };

    if imports.is_empty() {
        return Ok("Module has no imports.\n".to_string());
    }

    let mut out = String::new();
    out.push_str(&format!("{} import(s):\n\n", imports.len()));
    out.push_str(&format!(
        "{:<8} {:<20} {:<30} {}\n",
        "Kind", "Module", "Name", "Signature"
    ));
    out.push_str(&"-".repeat(80));
    out.push('\n');

    for imp in &imports {
        let sig = if imp.kind == "func" {
            imp.type_idx
                .and_then(|i| types.get(i as usize))
                .map(format_func_sig)
                .unwrap_or_default()
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{:<8} {:<20} {:<30} {}\n",
            imp.kind, imp.module, imp.name, sig
        ));
    }

    Ok(out)
}

fn action_exports(data: &[u8]) -> Result<String, String> {
    let sections = parse_sections(data);
    let types = find_section(&sections, 1)
        .map(|s| parse_type_section(data, s))
        .unwrap_or_default();

    // Function type indices from the function section
    let func_types: Vec<u32> = find_section(&sections, 3)
        .map(|sec| {
            let mut pos = sec.offset;
            let count = read_uleb128(data, &mut pos).unwrap_or(0);
            let mut indices = Vec::new();
            for _ in 0..count {
                if let Some(idx) = read_uleb128(data, &mut pos) {
                    indices.push(idx);
                }
            }
            indices
        })
        .unwrap_or_default();

    // How many functions are imported (they get the first indices)
    let import_func_count = find_section(&sections, 2)
        .map(|sec| {
            let imports = parse_import_section(data, sec);
            imports.iter().filter(|i| i.kind == "func").count() as u32
        })
        .unwrap_or(0);

    // Function names from custom "name" section
    let func_names: std::collections::HashMap<u32, String> = sections
        .iter()
        .filter(|s| s.id == 0)
        .find_map(|sec| {
            let mut pos = sec.offset;
            let name = read_name(data, &mut pos)?;
            if name == "name" {
                Some(parse_name_section(data, sec))
            } else {
                None
            }
        })
        .unwrap_or_default()
        .into_iter()
        .collect();

    let exports = match find_section(&sections, 7) {
        Some(sec) => parse_export_section(data, sec),
        None => return Ok("No export section found.\n".to_string()),
    };

    if exports.is_empty() {
        return Ok("Module has no exports.\n".to_string());
    }

    let mut out = String::new();
    out.push_str(&format!("{} export(s):\n\n", exports.len()));
    out.push_str(&format!(
        "{:<8} {:<5} {:<35} {}\n",
        "Kind", "Index", "Name", "Signature"
    ));
    out.push_str(&"-".repeat(80));
    out.push('\n');

    for exp in &exports {
        let sig = if exp.kind == "func" {
            // Look up the named function name
            let debug_name = func_names.get(&exp.index).cloned().unwrap_or_default();
            // Get the signature: index - import_func_count = index into func_types
            let local_idx = exp.index.saturating_sub(import_func_count) as usize;
            let type_idx = func_types.get(local_idx).copied();
            let sig_str = type_idx
                .and_then(|ti| types.get(ti as usize))
                .map(format_func_sig)
                .unwrap_or_default();
            if debug_name.is_empty() {
                sig_str
            } else {
                format!("{sig_str}  // {debug_name}")
            }
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{:<8} {:<5} {:<35} {}\n",
            exp.kind, exp.index, exp.name, sig
        ));
    }

    Ok(out)
}

// ── Schema ───────────────────────────────────────────────────────────────────

pub fn wasm_tools_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["info", "sections", "imports", "exports"],
                "description": "Operation: info (default — module overview: version, section counts, function/import/export totals), sections (all sections with size and content summary + function type signatures), imports (all imported functions/tables/memories/globals with signatures), exports (all exported symbols with signatures and index)"
            },
            "file": {
                "type": "string",
                "description": "Path to a .wasm file"
            },
            "hex": {
                "type": "string",
                "description": "Raw WASM bytes as a hex string (alternative to file)"
            }
        }
    })
}
