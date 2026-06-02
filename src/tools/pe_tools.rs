use serde_json::Value;

pub fn schema() -> Value {
    serde_json::json!({
        "name": "pe_tools",
        "description": "Inspect Windows PE (EXE/DLL/SYS/OCX) binaries without external tools — no dumpbin or readpe required. 5 actions: info (default — machine type, file type EXE/DLL, architecture PE32/PE32+, subsystem, timestamp, image base, entry point, ASLR/DEP/CFG flags, import DLL count, export count), sections (section table with names/virtual addresses/sizes/raw offsets/flags EXEC/READ/WRITE/CODE), imports (all imported DLLs with their function names or ordinals; up to 200 functions per DLL), exports (exported function names with ordinals; for DLLs), headers (DOS/COFF/Optional header field dump with data directory RVAs). Pass 'file' for a path to a .exe/.dll/.sys file, or 'hex' for raw PE bytes as a hex string.",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "sections", "imports", "exports", "headers"],
                    "description": "Operation to perform (default: info)"
                },
                "file": { "type": "string", "description": "Path to a PE file (.exe/.dll/.sys/.ocx)" },
                "hex":  { "type": "string", "description": "Raw PE bytes as a hex string (spaces/colons stripped)" }
            }
        }
    })
}

// ── byte readers (little-endian) ─────────────────────────────────────────────

fn r16(d: &[u8], off: usize) -> Option<u16> {
    d.get(off..off + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}
fn r32(d: &[u8], off: usize) -> Option<u32> {
    d.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn r64(d: &[u8], off: usize) -> Option<u64> {
    d.get(off..off + 8).map(|b| {
        u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    })
}

fn read_cstr(d: &[u8], off: usize) -> String {
    let end = d[off..].iter().position(|&b| b == 0).unwrap_or(256.min(d.len().saturating_sub(off)));
    String::from_utf8_lossy(&d[off..off + end]).to_string()
}

// ── name tables ──────────────────────────────────────────────────────────────

fn machine_name(m: u16) -> &'static str {
    match m {
        0x014C => "x86 (i386)",
        0x8664 => "x86-64 (AMD64)",
        0xAA64 => "ARM64 (AArch64)",
        0x01C0 => "ARM Thumb",
        0x01C4 => "ARM Thumb-2",
        0x0200 => "IA64 (Itanium)",
        0x0EBC => "EFI Bytecode",
        0x5032 => "RISC-V 32-bit",
        0x5064 => "RISC-V 64-bit",
        0x0000 => "Any",
        _ => "Unknown",
    }
}

fn subsystem_name(s: u16) -> &'static str {
    match s {
        0 => "Unknown",
        1 => "Native",
        2 => "Windows GUI",
        3 => "Windows Console (CUI)",
        5 => "OS/2 Console",
        7 => "POSIX Console",
        9 => "Windows CE GUI",
        10 => "EFI Application",
        11 => "EFI Boot Service Driver",
        12 => "EFI Runtime Driver",
        13 => "EFI ROM",
        14 => "Xbox",
        16 => "Windows Boot Application",
        _ => "Unknown",
    }
}

fn sec_flags(c: u32) -> String {
    let mut f = Vec::new();
    if c & 0x0000_0020 != 0 { f.push("CODE"); }
    if c & 0x0000_0040 != 0 { f.push("INIT_DATA"); }
    if c & 0x0000_0080 != 0 { f.push("UNINIT_DATA"); }
    if c & 0x0200_0000 != 0 { f.push("DISCARDABLE"); }
    if c & 0x2000_0000 != 0 { f.push("EXEC"); }
    if c & 0x4000_0000 != 0 { f.push("READ"); }
    if c & 0x8000_0000 != 0 { f.push("WRITE"); }
    if f.is_empty() { "─".to_string() } else { f.join("|") }
}

fn char_flags(c: u16) -> String {
    let mut f = Vec::new();
    if c & 0x0002 != 0 { f.push("EXECUTABLE"); }
    if c & 0x2000 != 0 { f.push("DLL"); }
    if c & 0x0100 != 0 { f.push("32BIT_MACHINE"); }
    if c & 0x0020 != 0 { f.push("LARGE_ADDR"); }
    if c & 0x0001 != 0 { f.push("RELOCS_STRIPPED"); }
    if c & 0x1000 != 0 { f.push("SYSTEM"); }
    if c & 0x0200 != 0 { f.push("DEBUG_STRIPPED"); }
    if f.is_empty() { "0".to_string() } else { f.join(" | ") }
}

fn dll_flags(c: u16) -> String {
    let mut f = Vec::new();
    if c & 0x0020 != 0 { f.push("HIGH_ENTROPY_VA"); }
    if c & 0x0040 != 0 { f.push("DYNAMIC_BASE(ASLR)"); }
    if c & 0x0080 != 0 { f.push("FORCE_INTEGRITY"); }
    if c & 0x0100 != 0 { f.push("NX_COMPAT(DEP)"); }
    if c & 0x0200 != 0 { f.push("NO_ISOLATION"); }
    if c & 0x0400 != 0 { f.push("NO_SEH"); }
    if c & 0x0800 != 0 { f.push("NO_BIND"); }
    if c & 0x1000 != 0 { f.push("APPCONTAINER"); }
    if c & 0x2000 != 0 { f.push("WDM_DRIVER"); }
    if c & 0x4000 != 0 { f.push("GUARD_CF"); }
    if c & 0x8000 != 0 { f.push("TERMINAL_SERVER_AWARE"); }
    if f.is_empty() { "none".to_string() } else { f.join(" | ") }
}

fn fmt_ts(ts: u32) -> String {
    if ts == 0 {
        return "0 (not set)".to_string();
    }
    // Approx UTC date from Unix timestamp
    let secs = ts as u64;
    let year = 1970 + secs / 31_557_600;
    let rem = secs % 31_557_600;
    let month = (rem / 2_629_800) + 1;
    let day = (rem % 2_629_800) / 86400 + 1;
    let h = (secs % 86400) / 3600;
    let min = (secs % 3600) / 60;
    format!("~{}-{:02}-{:02} {:02}:{:02} UTC", year, month, day, h, min)
}

// ── PE structure ─────────────────────────────────────────────────────────────

struct PeHdr {
    e_lfanew: u32,
    machine: u16,
    num_sections: u16,
    timestamp: u32,
    opt_hdr_size: u16,
    characteristics: u16,
    is_plus: bool,        // PE32+ (64-bit)
    entry_point: u32,
    image_base: u64,
    size_of_image: u32,
    subsystem: u16,
    dll_char: u16,
    export_rva: u32,
    export_size: u32,
    import_rva: u32,
}

fn parse_hdr(d: &[u8]) -> Result<PeHdr, String> {
    if d.len() < 64 {
        return Err("File too small to be a PE binary".into());
    }
    if &d[0..2] != b"MZ" {
        return Err("Not a PE file — missing MZ (DOS) signature".into());
    }
    let e_lfanew = r32(d, 0x3C).ok_or("Cannot read e_lfanew")?;
    let pe = e_lfanew as usize;
    if d.len() < pe + 24 {
        return Err("File truncated before PE header".into());
    }
    if &d[pe..pe + 4] != b"PE\0\0" {
        return Err("Invalid PE signature — expected 'PE\\0\\0'".into());
    }

    // COFF header at pe+4
    let coff = pe + 4;
    let machine = r16(d, coff).ok_or("Cannot read Machine")?;
    let num_sections = r16(d, coff + 2).ok_or("Cannot read NumberOfSections")?;
    let timestamp = r32(d, coff + 4).ok_or("Cannot read TimeDateStamp")?;
    let opt_hdr_size = r16(d, coff + 16).ok_or("Cannot read SizeOfOptionalHeader")?;
    let characteristics = r16(d, coff + 18).ok_or("Cannot read Characteristics")?;

    // Optional header at coff+20
    let opt = coff + 20;
    if d.len() < opt + 4 {
        return Err("File truncated at Optional Header".into());
    }
    let magic = r16(d, opt).ok_or("Cannot read OptionalHeader magic")?;
    let is_plus = magic == 0x20B;

    let entry_point = r32(d, opt + 16).unwrap_or(0);

    // ImageBase: PE32 at opt+28 (u32), PE32+ at opt+24 (u64)
    let image_base = if is_plus {
        r64(d, opt + 24).unwrap_or(0)
    } else {
        r32(d, opt + 28).unwrap_or(0) as u64
    };

    // SizeOfImage at opt+56 (same for both)
    let size_of_image = r32(d, opt + 56).unwrap_or(0);

    // Subsystem at opt+68 (same for both)
    let subsystem = r16(d, opt + 68).unwrap_or(0);
    let dll_char = r16(d, opt + 70).unwrap_or(0);

    // Data directories: PE32 at opt+96, PE32+ at opt+112
    let dd = if is_plus { opt + 112 } else { opt + 96 };
    let export_rva = r32(d, dd).unwrap_or(0);
    let export_size = r32(d, dd + 4).unwrap_or(0);
    let import_rva = r32(d, dd + 8).unwrap_or(0);

    Ok(PeHdr {
        e_lfanew,
        machine,
        num_sections,
        timestamp,
        opt_hdr_size,
        characteristics,
        is_plus,
        entry_point,
        image_base,
        size_of_image,
        subsystem,
        dll_char,
        export_rva,
        export_size,
        import_rva,
    })
}

struct Sec {
    name: String,
    vsize: u32,
    vaddr: u32,
    raw_size: u32,
    raw_off: u32,
    chars: u32,
}

fn parse_sections(d: &[u8], hdr: &PeHdr) -> Vec<Sec> {
    let coff = hdr.e_lfanew as usize + 4;
    let sec_start = coff + 20 + hdr.opt_hdr_size as usize;
    (0..hdr.num_sections as usize)
        .filter_map(|i| {
            let off = sec_start + i * 40;
            d.get(off..off + 40)?;
            let nb = &d[off..off + 8];
            let end = nb.iter().position(|&b| b == 0).unwrap_or(8);
            Some(Sec {
                name: String::from_utf8_lossy(&nb[..end]).to_string(),
                vsize: r32(d, off + 8).unwrap_or(0),
                vaddr: r32(d, off + 12).unwrap_or(0),
                raw_size: r32(d, off + 16).unwrap_or(0),
                raw_off: r32(d, off + 20).unwrap_or(0),
                chars: r32(d, off + 36).unwrap_or(0),
            })
        })
        .collect()
}

fn rva2off(rva: u32, secs: &[Sec]) -> Option<usize> {
    secs.iter().find_map(|s| {
        let va_end = s.vaddr.saturating_add(s.vsize.max(s.raw_size));
        if rva >= s.vaddr && rva < va_end {
            Some((rva - s.vaddr + s.raw_off) as usize)
        } else {
            None
        }
    })
}

fn parse_imports(d: &[u8], hdr: &PeHdr, secs: &[Sec]) -> Vec<(String, Vec<String>)> {
    if hdr.import_rva == 0 {
        return vec![];
    }
    let mut result = Vec::new();
    let mut idt = match rva2off(hdr.import_rva, secs) {
        Some(o) => o,
        None => return vec![],
    };
    loop {
        if d.len() < idt + 20 {
            break;
        }
        let ilt_rva = r32(d, idt).unwrap_or(0);
        let name_rva = r32(d, idt + 12).unwrap_or(0);
        if name_rva == 0 && ilt_rva == 0 {
            break;
        }
        let dll = rva2off(name_rva, secs)
            .map(|o| read_cstr(d, o))
            .unwrap_or_else(|| "<unknown>".to_string());

        let mut funcs = Vec::new();
        if ilt_rva != 0 {
            if let Some(mut ilt) = rva2off(ilt_rva, secs) {
                let entry_size = if hdr.is_plus { 8 } else { 4 };
                loop {
                    let entry = if hdr.is_plus {
                        r64(d, ilt).unwrap_or(0)
                    } else {
                        r32(d, ilt).unwrap_or(0) as u64
                    };
                    if entry == 0 {
                        break;
                    }
                    ilt += entry_size;
                    let ord_flag = if hdr.is_plus { entry >> 63 } else { entry >> 31 };
                    if ord_flag != 0 {
                        funcs.push(format!("Ordinal #{}", entry & 0xFFFF));
                    } else {
                        let hint_rva = (entry & 0x7FFF_FFFF) as u32;
                        if let Some(ho) = rva2off(hint_rva, secs) {
                            if ho + 2 < d.len() {
                                funcs.push(read_cstr(d, ho + 2));
                            }
                        }
                    }
                    if funcs.len() >= 201 {
                        funcs.push("...".to_string());
                        break;
                    }
                }
            }
        }
        result.push((dll, funcs));
        idt += 20;
        if result.len() > 200 {
            break;
        }
    }
    result
}

fn parse_exports(d: &[u8], hdr: &PeHdr, secs: &[Sec]) -> (String, Vec<(u32, String)>) {
    if hdr.export_rva == 0 {
        return (String::new(), vec![]);
    }
    let edt = match rva2off(hdr.export_rva, secs) {
        Some(o) => o,
        None => return (String::new(), vec![]),
    };
    if d.len() < edt + 40 {
        return (String::new(), vec![]);
    }
    let name_rva = r32(d, edt + 12).unwrap_or(0);
    let ordinal_base = r32(d, edt + 16).unwrap_or(0);
    let num_names = r32(d, edt + 24).unwrap_or(0);
    let names_rva = r32(d, edt + 32).unwrap_or(0);
    let ords_rva = r32(d, edt + 36).unwrap_or(0);

    let dll_name = rva2off(name_rva, secs)
        .map(|o| read_cstr(d, o))
        .unwrap_or_default();

    let mut exports = Vec::new();
    if let (Some(n_off), Some(o_off)) = (rva2off(names_rva, secs), rva2off(ords_rva, secs)) {
        for i in 0..num_names.min(2000) as usize {
            let nr = r32(d, n_off + i * 4).unwrap_or(0);
            let oi = r16(d, o_off + i * 2).unwrap_or(0);
            let ord = ordinal_base + oi as u32;
            let fname = rva2off(nr, secs)
                .map(|o| read_cstr(d, o))
                .unwrap_or_else(|| format!("<Ordinal {}>", ord));
            exports.push((ord, fname));
        }
    }
    (dll_name, exports)
}

fn read_bytes(args: &Value) -> Result<Vec<u8>, String> {
    if let Some(fp) = args["file"].as_str() {
        std::fs::read(fp).map_err(|e| format!("Cannot read '{}': {}", fp, e))
    } else if let Some(hex) = args["hex"].as_str() {
        let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if clean.len() % 2 != 0 {
            return Err("Hex string has odd length".into());
        }
        (0..clean.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).map_err(|_| format!("Bad hex at {}", i)))
            .collect()
    } else {
        Err("Provide 'file' (path to PE) or 'hex' (raw bytes as hex string)".into())
    }
}

// ── execute ───────────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let d = read_bytes(args)?;
    let action = args["action"].as_str().unwrap_or("info");
    let hdr = parse_hdr(&d)?;
    let secs = parse_sections(&d, &hdr);

    match action {
        "info" => {
            let file_type = if hdr.characteristics & 0x2000 != 0 {
                "DLL"
            } else if hdr.characteristics & 0x0002 != 0 {
                "EXE"
            } else {
                "Object/Driver"
            };
            let arch = if hdr.is_plus { "PE32+ (64-bit)" } else { "PE32 (32-bit)" };
            let imports = parse_imports(&d, &hdr, &secs);
            let (exp_name, exports) = parse_exports(&d, &hdr, &secs);

            let mut out = String::from("═══ PE BINARY INFO ═══\n\n");
            out.push_str(&format!("  File Type     : {}\n", file_type));
            out.push_str(&format!("  Architecture  : {}\n", arch));
            out.push_str(&format!("  Machine       : {} (0x{:04X})\n", machine_name(hdr.machine), hdr.machine));
            out.push_str(&format!("  Subsystem     : {} ({})\n", subsystem_name(hdr.subsystem), hdr.subsystem));
            out.push_str(&format!("  Timestamp     : 0x{:08X} ({})\n", hdr.timestamp, fmt_ts(hdr.timestamp)));
            out.push_str(&format!("  Image Base    : 0x{:016X}\n", hdr.image_base));
            out.push_str(&format!("  Entry Point   : 0x{:08X} (RVA)\n", hdr.entry_point));
            out.push_str(&format!(
                "  Image Size    : {} bytes ({:.1} MB)\n",
                hdr.size_of_image,
                hdr.size_of_image as f64 / 1_048_576.0
            ));
            out.push_str(&format!("  Sections      : {}\n", hdr.num_sections));
            out.push_str(&format!("  Characteristics: {}\n", char_flags(hdr.characteristics)));
            out.push('\n');
            out.push_str("  ── Security Features ──\n");
            out.push_str(&format!(
                "  ASLR          : {}\n",
                if hdr.dll_char & 0x0040 != 0 { "YES (DYNAMIC_BASE)" } else { "NO" }
            ));
            out.push_str(&format!(
                "  DEP (NX)      : {}\n",
                if hdr.dll_char & 0x0100 != 0 { "YES (NX_COMPAT)" } else { "NO" }
            ));
            out.push_str(&format!(
                "  CFG Guard     : {}\n",
                if hdr.dll_char & 0x4000 != 0 { "YES (GUARD_CF)" } else { "NO" }
            ));
            out.push_str(&format!(
                "  High Entropy  : {}\n",
                if hdr.dll_char & 0x0020 != 0 { "YES" } else { "NO" }
            ));
            out.push_str(&format!(
                "  Force Integrity: {}\n",
                if hdr.dll_char & 0x0080 != 0 { "YES" } else { "NO" }
            ));
            out.push('\n');
            out.push_str(&format!("  Import DLLs   : {}\n", imports.len()));
            if !exp_name.is_empty() {
                out.push_str(&format!("  Exports from  : {} ({} functions)\n", exp_name, exports.len()));
            } else {
                out.push_str("  Exports       : none\n");
            }
            out.push_str("\n  Use action='sections', 'imports', 'exports', or 'headers' for details.\n");
            Ok(out)
        }

        "sections" => {
            let mut out = format!("═══ PE SECTIONS ({}) ═══\n\n", secs.len());
            out.push_str(&format!(
                "  {:<12}  {:>10}  {:>10}  {:>10}  {:>10}  FLAGS\n",
                "Name", "VirtAddr", "VirtSize", "RawSize", "RawOffset"
            ));
            out.push_str(&format!("  {}\n", "─".repeat(75)));
            for s in &secs {
                out.push_str(&format!(
                    "  {:<12}  0x{:08X}  {:>10}  {:>10}  {:>10}  {}\n",
                    s.name, s.vaddr, s.vsize, s.raw_size, s.raw_off, sec_flags(s.chars)
                ));
            }
            Ok(out)
        }

        "imports" => {
            let imports = parse_imports(&d, &hdr, &secs);
            if imports.is_empty() {
                return Ok("No imports found (or import table not readable at this offset).\n".to_string());
            }
            let total: usize = imports.iter().map(|(_, f)| f.len()).sum();
            let mut out = format!("═══ PE IMPORTS — {} DLLs, {} functions ═══\n\n", imports.len(), total);
            for (dll, funcs) in &imports {
                out.push_str(&format!("  {} ({} imports)\n", dll, funcs.len()));
                let show = funcs.len().min(50);
                for f in &funcs[..show] {
                    out.push_str(&format!("    ├─ {}\n", f));
                }
                if funcs.len() > 50 {
                    out.push_str(&format!("    └─ ... and {} more\n", funcs.len() - 50));
                }
                out.push('\n');
            }
            Ok(out)
        }

        "exports" => {
            let (exp_name, exports) = parse_exports(&d, &hdr, &secs);
            if exports.is_empty() {
                return Ok("No exports found (not a DLL or export table empty).\n".to_string());
            }
            let mut out = format!(
                "═══ PE EXPORTS from '{}' — {} functions ═══\n\n",
                exp_name,
                exports.len()
            );
            out.push_str(&format!("  {:<8}  FUNCTION\n", "ORDINAL"));
            out.push_str(&format!("  {}\n", "─".repeat(60)));
            for (ord, name) in exports.iter().take(300) {
                out.push_str(&format!("  {:<8}  {}\n", ord, name));
            }
            if exports.len() > 300 {
                out.push_str(&format!("\n  ... and {} more\n", exports.len() - 300));
            }
            Ok(out)
        }

        "headers" => {
            let coff = hdr.e_lfanew as usize + 4;
            let opt = coff + 20;
            let magic = r16(&d, opt).unwrap_or(0);
            let dd = if hdr.is_plus { opt + 112 } else { opt + 96 };

            let mut out = String::from("═══ PE HEADER DETAIL ═══\n\n");
            out.push_str("── DOS Header ──────────────────\n");
            out.push_str("  e_magic   : 0x5A4D (MZ)\n");
            out.push_str(&format!("  e_lfanew  : 0x{:08X}\n\n", hdr.e_lfanew));

            out.push_str("── COFF Header ─────────────────\n");
            out.push_str(&format!("  Machine            : 0x{:04X}  ({})\n", hdr.machine, machine_name(hdr.machine)));
            out.push_str(&format!("  NumberOfSections   : {}\n", hdr.num_sections));
            out.push_str(&format!("  TimeDateStamp      : 0x{:08X}  ({})\n", hdr.timestamp, fmt_ts(hdr.timestamp)));
            out.push_str(&format!("  SizeOfOptionalHdr  : {} bytes\n", hdr.opt_hdr_size));
            out.push_str(&format!(
                "  Characteristics    : 0x{:04X}  ({})\n\n",
                hdr.characteristics,
                char_flags(hdr.characteristics)
            ));

            out.push_str("── Optional Header ─────────────\n");
            out.push_str(&format!("  Magic              : 0x{:04X}  ({})\n", magic, if hdr.is_plus { "PE32+" } else { "PE32" }));
            out.push_str(&format!("  AddressOfEntryPoint: 0x{:08X}\n", hdr.entry_point));
            out.push_str(&format!("  ImageBase          : 0x{:016X}\n", hdr.image_base));
            out.push_str(&format!("  SizeOfImage        : 0x{:08X}  ({} bytes)\n", hdr.size_of_image, hdr.size_of_image));
            out.push_str(&format!("  Subsystem          : {}  ({})\n", hdr.subsystem, subsystem_name(hdr.subsystem)));
            out.push_str(&format!(
                "  DllCharacteristics : 0x{:04X}  ({})\n\n",
                hdr.dll_char,
                dll_flags(hdr.dll_char)
            ));

            out.push_str("── Data Directories ────────────\n");
            out.push_str(&format!(
                "  [0] Export Table   : RVA=0x{:08X}  Size={}\n",
                hdr.export_rva, hdr.export_size
            ));
            out.push_str(&format!(
                "  [1] Import Table   : RVA=0x{:08X}  Size={}\n",
                hdr.import_rva,
                r32(&d, dd + 12).unwrap_or(0)
            ));
            let res_rva = r32(&d, dd + 16).unwrap_or(0);
            let tls_rva = r32(&d, dd + 40).unwrap_or(0);
            let cfg_rva = r32(&d, dd + 80).unwrap_or(0);
            out.push_str(&format!("  [2] Resource Table : RVA=0x{:08X}\n", res_rva));
            out.push_str(&format!("  [9] TLS Table      : RVA=0x{:08X}\n", tls_rva));
            out.push_str(&format!("  [10] Load Config   : RVA=0x{:08X}  (CFG)\n", cfg_rva));
            Ok(out)
        }

        other => Err(format!(
            "Unknown action '{}'. Valid actions: info, sections, imports, exports, headers",
            other
        )),
    }
}
