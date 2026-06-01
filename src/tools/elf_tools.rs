use serde_json::{json, Value};

pub fn elf_tools_schema() -> Value {
    json!({
        "name": "elf_tools",
        "description": "Inspect ELF (Executable and Linkable Format) binary files — Linux executables, shared libraries (.so), object files (.o), and kernel modules (.ko). Parses the ELF header, program headers (segments), and section headers without external tools like readelf or objdump. Actions: info (default — ELF class/endian/type/machine/entry point/header counts), segments (program headers — type, flags, virtual address, file size, memory size), sections (section headers — name, type, flags, address, size), symbols (symbol table entries if present), dynamic (dynamic linking info: needed libraries, RPATH, interpreter).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "segments", "sections", "symbols", "dynamic"],
                    "description": "info (default — ELF overview), segments (program headers), sections (section headers), symbols (symbol table), dynamic (shared library dependencies)"
                },
                "file": {
                    "type": "string",
                    "description": "Path to an ELF binary file (.so, .o, executable, .ko, .elf)"
                },
                "hex": {
                    "type": "string",
                    "description": "Raw ELF bytes as a hex string (alternative to 'file')"
                }
            },
            "required": []
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let data = load_data(args)?;

    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let elf = parse_elf_header(&data)?;

    match action {
        "segments" => action_segments(&elf, &data),
        "sections" => action_sections(&elf, &data),
        "symbols" => action_symbols(&elf, &data),
        "dynamic" => action_dynamic(&elf, &data),
        _ => action_info(&elf, &data),
    }
}

fn load_data(args: &Value) -> Result<Vec<u8>, String> {
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read(path).map_err(|e| format!("Cannot read '{}': {}", path, e))
    } else if let Some(hex) = args.get("hex").and_then(|v| v.as_str()) {
        parse_hex(hex)
    } else {
        Err("Pass 'file' with a path to an ELF binary, or 'hex' with raw bytes as hex.".into())
    }
}

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() % 2 != 0 {
        return Err("Odd hex digit count.".into());
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&clean[i..i + 2], 16).map_err(|e| format!("Bad hex at {}: {}", i, e))
        })
        .collect()
}

// ── ELF constants ─────────────────────────────────────────────────────────────

const EI_MAG0: usize = 0;
const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const EI_VERSION: usize = 6;
const EI_OSABI: usize = 7;
const EI_NIDENT: usize = 16;

// ── parsed header ─────────────────────────────────────────────────────────────

#[derive(Debug)]
struct ElfHeader {
    class: u8, // 1=32-bit, 2=64-bit
    data: u8,  // 1=LE, 2=BE
    version: u8,
    osabi: u8,
    elf_type: u16, // ET_*
    machine: u16,  // EM_*
    entry: u64,
    phoff: u64, // program header table offset
    shoff: u64, // section header table offset
    flags: u32,
    ehsize: u16,
    phentsize: u16,
    phnum: u16,
    shentsize: u16,
    shnum: u16,
    shstrndx: u16, // section index of section name string table
}

fn parse_elf_header(data: &[u8]) -> Result<ElfHeader, String> {
    if data.len() < EI_NIDENT + 2 {
        return Err("File too short to be an ELF binary.".into());
    }

    // Magic check
    if &data[EI_MAG0..EI_MAG0 + 4] != b"\x7fELF" {
        return Err(
            "Not an ELF file (magic bytes mismatch — expected 0x7f 0x45 0x4c 0x46).".into(),
        );
    }

    let class = data[EI_CLASS];
    let ei_data = data[EI_DATA];
    let version = data[EI_VERSION];
    let osabi = data[EI_OSABI];

    if class != 1 && class != 2 {
        return Err(format!(
            "Unknown ELF class byte {:#x} (expected 1=32-bit or 2=64-bit).",
            class
        ));
    }
    let le = ei_data == 1;

    let min_len = if class == 1 { 52 } else { 64 };
    if data.len() < min_len {
        return Err(format!(
            "File truncated: ELF {} header needs {} bytes.",
            if class == 1 { "32-bit" } else { "64-bit" },
            min_len
        ));
    }

    let (
        elf_type,
        machine,
        flags,
        ehsize,
        phentsize,
        phnum,
        shentsize,
        shnum,
        shstrndx,
        entry,
        phoff,
        shoff,
    );

    if class == 1 {
        // 32-bit ELF
        elf_type = r16(data, 16, le);
        machine = r16(data, 18, le);
        // version u32 at 20
        entry = r32(data, 24, le) as u64;
        phoff = r32(data, 28, le) as u64;
        shoff = r32(data, 32, le) as u64;
        flags = r32(data, 36, le);
        ehsize = r16(data, 40, le);
        phentsize = r16(data, 42, le);
        phnum = r16(data, 44, le);
        shentsize = r16(data, 46, le);
        shnum = r16(data, 48, le);
        shstrndx = r16(data, 50, le);
    } else {
        // 64-bit ELF
        elf_type = r16(data, 16, le);
        machine = r16(data, 18, le);
        // version u32 at 20
        entry = r64(data, 24, le);
        phoff = r64(data, 32, le);
        shoff = r64(data, 40, le);
        flags = r32(data, 48, le);
        ehsize = r16(data, 52, le);
        phentsize = r16(data, 54, le);
        phnum = r16(data, 56, le);
        shentsize = r16(data, 58, le);
        shnum = r16(data, 60, le);
        shstrndx = r16(data, 62, le);
    }

    Ok(ElfHeader {
        class,
        data: ei_data,
        version,
        osabi,
        elf_type,
        machine,
        entry,
        phoff,
        shoff,
        flags,
        ehsize,
        phentsize,
        phnum,
        shentsize,
        shnum,
        shstrndx,
    })
}

// ── integer readers ───────────────────────────────────────────────────────────

fn r16(data: &[u8], off: usize, le: bool) -> u16 {
    if off + 2 > data.len() {
        return 0;
    }
    let b = [data[off], data[off + 1]];
    if le {
        u16::from_le_bytes(b)
    } else {
        u16::from_be_bytes(b)
    }
}

fn r32(data: &[u8], off: usize, le: bool) -> u32 {
    if off + 4 > data.len() {
        return 0;
    }
    let mut b = [0u8; 4];
    b.copy_from_slice(&data[off..off + 4]);
    if le {
        u32::from_le_bytes(b)
    } else {
        u32::from_be_bytes(b)
    }
}

fn r64(data: &[u8], off: usize, le: bool) -> u64 {
    if off + 8 > data.len() {
        return 0;
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(&data[off..off + 8]);
    if le {
        u64::from_le_bytes(b)
    } else {
        u64::from_be_bytes(b)
    }
}

// ── name lookups ──────────────────────────────────────────────────────────────

fn elf_type_name(t: u16) -> &'static str {
    match t {
        0 => "ET_NONE (no file type)",
        1 => "ET_REL (relocatable object)",
        2 => "ET_EXEC (executable)",
        3 => "ET_DYN (shared library / PIE executable)",
        4 => "ET_CORE (core dump)",
        _ => "unknown",
    }
}

fn machine_name(m: u16) -> &'static str {
    match m {
        0 => "None",
        2 => "SPARC",
        3 => "i386",
        8 => "MIPS",
        20 => "PowerPC",
        21 => "PowerPC64",
        22 => "S390",
        40 => "ARM (32-bit)",
        50 => "Intel IA-64",
        62 => "x86-64 (AMD64)",
        83 => "AVR",
        164 => "TriCore",
        183 => "AArch64 (ARM 64-bit)",
        188 => "RISC-V",
        247 => "eBPF",
        _ => "other",
    }
}

fn osabi_name(o: u8) -> &'static str {
    match o {
        0 => "System V / Linux",
        1 => "HP-UX",
        2 => "NetBSD",
        3 => "GNU/Linux",
        6 => "Solaris",
        7 => "AIX",
        8 => "IRIX",
        9 => "FreeBSD",
        12 => "OpenBSD",
        64 => "ARM EABI",
        97 => "ARM",
        255 => "Standalone",
        _ => "other",
    }
}

fn pt_type_name(t: u32) -> &'static str {
    match t {
        0 => "PT_NULL",
        1 => "PT_LOAD",
        2 => "PT_DYNAMIC",
        3 => "PT_INTERP",
        4 => "PT_NOTE",
        5 => "PT_SHLIB",
        6 => "PT_PHDR",
        7 => "PT_TLS",
        0x6474e550 => "PT_GNU_EH_FRAME",
        0x6474e551 => "PT_GNU_STACK",
        0x6474e552 => "PT_GNU_RELRO",
        0x6474e553 => "PT_GNU_PROPERTY",
        _ => "PT_?",
    }
}

fn sh_type_name(t: u32) -> &'static str {
    match t {
        0 => "SHT_NULL",
        1 => "SHT_PROGBITS",
        2 => "SHT_SYMTAB",
        3 => "SHT_STRTAB",
        4 => "SHT_RELA",
        5 => "SHT_HASH",
        6 => "SHT_DYNAMIC",
        7 => "SHT_NOTE",
        8 => "SHT_NOBITS",
        9 => "SHT_REL",
        11 => "SHT_DYNSYM",
        14 => "SHT_INIT_ARRAY",
        15 => "SHT_FINI_ARRAY",
        _ => "SHT_?",
    }
}

fn pf_flags(f: u32) -> String {
    let mut s = String::new();
    if f & 4 != 0 {
        s.push('R');
    } else {
        s.push('-');
    }
    if f & 2 != 0 {
        s.push('W');
    } else {
        s.push('-');
    }
    if f & 1 != 0 {
        s.push('X');
    } else {
        s.push('-');
    }
    s
}

fn sh_flags(f: u64) -> String {
    let mut s = String::new();
    if f & 0x2 != 0 {
        s.push('A');
    } // SHF_ALLOC
    if f & 0x4 != 0 {
        s.push('X');
    } // SHF_EXECINSTR
    if f & 0x1 != 0 {
        s.push('W');
    } // SHF_WRITE
    if f & 0x10 != 0 {
        s.push('M');
    } // SHF_MERGE
    if f & 0x20 != 0 {
        s.push('S');
    } // SHF_STRINGS
    if s.is_empty() {
        s.push('-');
    }
    s
}

fn dt_tag_name(tag: i64) -> &'static str {
    match tag {
        0 => "DT_NULL",
        1 => "DT_NEEDED",
        5 => "DT_STRTAB",
        6 => "DT_SYMTAB",
        7 => "DT_RELA",
        10 => "DT_STRSZ",
        12 => "DT_INIT",
        13 => "DT_FINI",
        14 => "DT_SONAME",
        15 => "DT_RPATH",
        29 => "DT_RUNPATH",
        _ => "DT_?",
    }
}

// ── string table helpers ──────────────────────────────────────────────────────

fn read_strtab_entry(data: &[u8], strtab_off: u64, strtab_sz: u64, idx: u32) -> String {
    let start = strtab_off as usize + idx as usize;
    let end_limit = (strtab_off + strtab_sz) as usize;
    if start >= data.len() || start >= end_limit {
        return format!("<str@{:#x}>", idx);
    }
    let end = data[start..end_limit.min(data.len())]
        .iter()
        .position(|&b| b == 0)
        .map(|p| start + p)
        .unwrap_or(end_limit.min(data.len()));
    String::from_utf8_lossy(&data[start..end]).into_owned()
}

// Find a section by name (only usable after we can read shstrtab)
fn find_section_by_type(elf: &ElfHeader, data: &[u8], sh_type: u32) -> Option<(u64, u64)> {
    let le = elf.data == 1;
    let addr_size: usize = if elf.class == 1 { 4 } else { 8 };
    let shentsize = elf.shentsize as usize;
    if shentsize == 0 || elf.shnum == 0 {
        return None;
    }

    for i in 0..elf.shnum as usize {
        let sh_off = elf.shoff as usize + i * shentsize;
        if sh_off + shentsize > data.len() {
            break;
        }

        let stype = r32(data, sh_off + 4, le);
        if stype != sh_type {
            continue;
        }

        let (offset, size) = if elf.class == 1 {
            (
                r32(data, sh_off + 16, le) as u64,
                r32(data, sh_off + 20, le) as u64,
            )
        } else {
            let _ = addr_size;
            (r64(data, sh_off + 24, le), r64(data, sh_off + 32, le))
        };
        return Some((offset, size));
    }
    None
}

// ── section name lookup ───────────────────────────────────────────────────────

fn section_name(elf: &ElfHeader, data: &[u8], name_idx: u32) -> String {
    if elf.shstrndx == 0 || elf.shstrndx == 0xffff {
        return format!("{:#x}", name_idx);
    }
    let le = elf.data == 1;
    let shentsize = elf.shentsize as usize;
    if shentsize == 0 {
        return format!("{:#x}", name_idx);
    }

    let str_sh_off = elf.shoff as usize + elf.shstrndx as usize * shentsize;
    if str_sh_off + shentsize > data.len() {
        return format!("{:#x}", name_idx);
    }

    let (strtab_off, strtab_sz) = if elf.class == 1 {
        (
            r32(data, str_sh_off + 16, le) as u64,
            r32(data, str_sh_off + 20, le) as u64,
        )
    } else {
        (
            r64(data, str_sh_off + 24, le),
            r64(data, str_sh_off + 32, le),
        )
    };

    read_strtab_entry(data, strtab_off, strtab_sz, name_idx)
}

// ── action: info ──────────────────────────────────────────────────────────────

fn action_info(elf: &ElfHeader, data: &[u8]) -> Result<String, String> {
    let endian = if elf.data == 1 {
        "Little-endian (LE)"
    } else {
        "Big-endian (BE)"
    };
    let bits = if elf.class == 1 { "32-bit" } else { "64-bit" };

    let mut out = String::from("ELF BINARY INFO\n");
    out.push_str(&"═".repeat(60));
    out.push('\n');
    out.push_str(&format!("  Class:        {} ELF\n", bits));
    out.push_str(&format!("  Endian:       {}\n", endian));
    out.push_str(&format!("  ELF version:  {}\n", elf.version));
    out.push_str(&format!(
        "  OS/ABI:       {} ({:#x})\n",
        osabi_name(elf.osabi),
        elf.osabi
    ));
    out.push_str(&format!(
        "  Type:         {} ({:#x})\n",
        elf_type_name(elf.elf_type),
        elf.elf_type
    ));
    out.push_str(&format!(
        "  Machine:      {} (EM={:#x})\n",
        machine_name(elf.machine),
        elf.machine
    ));
    out.push_str(&format!("  Entry point:  {:#x}\n", elf.entry));
    out.push_str(&format!("  Flags:        {:#x}\n", elf.flags));
    out.push_str(&format!("  ELF header:   {} bytes\n", elf.ehsize));
    out.push('\n');
    out.push_str(&format!(
        "  Program headers (segments): {} × {} bytes  @ {:#x}\n",
        elf.phnum, elf.phentsize, elf.phoff
    ));
    out.push_str(&format!(
        "  Section headers:            {} × {} bytes  @ {:#x}\n",
        elf.shnum, elf.shentsize, elf.shoff
    ));
    out.push_str(&format!("  Shstrtab index: {}\n", elf.shstrndx));
    out.push_str(&format!("  File size:    {} bytes\n", data.len()));

    // PT_INTERP — interpreter (dynamic linker)
    if let Some(interp) = read_interp(elf, data) {
        out.push_str(&format!("  Interpreter:  {}\n", interp));
    }

    // Summary of segment types
    if elf.phnum > 0 {
        let segtypes = segment_type_summary(elf, data);
        if !segtypes.is_empty() {
            out.push('\n');
            out.push_str("  Segments: ");
            out.push_str(&segtypes.join(", "));
            out.push('\n');
        }
    }

    Ok(out)
}

fn read_interp(elf: &ElfHeader, data: &[u8]) -> Option<String> {
    let le = elf.data == 1;
    let phentsize = elf.phentsize as usize;
    if phentsize == 0 {
        return None;
    }
    for i in 0..elf.phnum as usize {
        let ph_off = elf.phoff as usize + i * phentsize;
        if ph_off + phentsize > data.len() {
            break;
        }
        let ptype = r32(data, ph_off, le);
        if ptype != 3 {
            continue;
        } // PT_INTERP
        let (offset, filesz) = if elf.class == 1 {
            (
                r32(data, ph_off + 4, le) as u64,
                r32(data, ph_off + 16, le) as u64,
            )
        } else {
            (r64(data, ph_off + 8, le), r64(data, ph_off + 32, le))
        };
        let start = offset as usize;
        let end = (start + filesz as usize).min(data.len());
        if start >= data.len() {
            return None;
        }
        let s = &data[start..end];
        let nul = s.iter().position(|&b| b == 0).unwrap_or(s.len());
        return Some(String::from_utf8_lossy(&s[..nul]).into_owned());
    }
    None
}

fn segment_type_summary(elf: &ElfHeader, data: &[u8]) -> Vec<String> {
    let le = elf.data == 1;
    let phentsize = elf.phentsize as usize;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for i in 0..elf.phnum as usize {
        let ph_off = elf.phoff as usize + i * phentsize;
        if ph_off + 4 > data.len() {
            break;
        }
        let t = r32(data, ph_off, le);
        if seen.insert(t) {
            out.push(pt_type_name(t).to_string());
        }
    }
    out
}

// ── action: segments ─────────────────────────────────────────────────────────

fn action_segments(elf: &ElfHeader, data: &[u8]) -> Result<String, String> {
    let le = elf.data == 1;
    let phentsize = elf.phentsize as usize;

    if elf.phnum == 0 {
        return Ok("No program headers (segments) in this ELF file.".into());
    }

    let mut out = format!("PROGRAM HEADERS (SEGMENTS) — {} entries\n", elf.phnum);
    out.push_str(&"─".repeat(80));
    out.push('\n');
    out.push_str(&format!(
        "  {:3}  {:<22}  {:>10}  {:>10}  {:>10}  {:>10}  {:<5}\n",
        "#", "Type", "VirtAddr", "FileOff", "FileSz", "MemSz", "Flags"
    ));
    out.push_str(&"─".repeat(80));
    out.push('\n');

    for i in 0..elf.phnum as usize {
        let ph_off = elf.phoff as usize + i * phentsize;
        if ph_off + phentsize > data.len() {
            out.push_str("  <truncated>\n");
            break;
        }

        let ptype = r32(data, ph_off, le);
        let (poffset, vaddr, filesz, memsz, flags) = if elf.class == 1 {
            (
                r32(data, ph_off + 4, le) as u64,
                r32(data, ph_off + 8, le) as u64,
                r32(data, ph_off + 16, le) as u64,
                r32(data, ph_off + 20, le) as u64,
                r32(data, ph_off + 24, le),
            )
        } else {
            (
                r64(data, ph_off + 8, le),
                r64(data, ph_off + 16, le),
                r64(data, ph_off + 32, le),
                r64(data, ph_off + 40, le),
                r32(data, ph_off + 4, le),
            )
        };

        out.push_str(&format!(
            "  {:3}  {:<22}  {:#010x}  {:#010x}  {:>10}  {:>10}  {}\n",
            i,
            pt_type_name(ptype),
            vaddr,
            poffset,
            filesz,
            memsz,
            pf_flags(flags),
        ));

        // For PT_INTERP show the interpreter path
        if ptype == 3 {
            let start = poffset as usize;
            let end = (start + filesz as usize).min(data.len());
            if start < data.len() {
                let s = &data[start..end];
                let nul = s.iter().position(|&b| b == 0).unwrap_or(s.len());
                out.push_str(&format!(
                    "       → interpreter: {}\n",
                    String::from_utf8_lossy(&s[..nul])
                ));
            }
        }
    }

    Ok(out)
}

// ── action: sections ─────────────────────────────────────────────────────────

fn action_sections(elf: &ElfHeader, data: &[u8]) -> Result<String, String> {
    let le = elf.data == 1;
    let shentsize = elf.shentsize as usize;

    if elf.shnum == 0 {
        return Ok("No section headers in this ELF file (stripped binary).".into());
    }

    let mut out = format!("SECTION HEADERS — {} entries\n", elf.shnum);
    out.push_str(&"─".repeat(80));
    out.push('\n');
    out.push_str(&format!(
        "  {:3}  {:<20}  {:<16}  {:>10}  {:>10}  {:>10}  {:<8}\n",
        "#", "Name", "Type", "Address", "Offset", "Size", "Flags"
    ));
    out.push_str(&"─".repeat(80));
    out.push('\n');

    for i in 0..elf.shnum as usize {
        let sh_off = elf.shoff as usize + i * shentsize;
        if sh_off + shentsize > data.len() {
            out.push_str("  <truncated>\n");
            break;
        }

        let name_idx = r32(data, sh_off, le);
        let stype = r32(data, sh_off + 4, le);
        let (sflags, addr, offset, size) = if elf.class == 1 {
            (
                r32(data, sh_off + 8, le) as u64,
                r32(data, sh_off + 12, le) as u64,
                r32(data, sh_off + 16, le) as u64,
                r32(data, sh_off + 20, le) as u64,
            )
        } else {
            (
                r64(data, sh_off + 8, le),
                r64(data, sh_off + 16, le),
                r64(data, sh_off + 24, le),
                r64(data, sh_off + 32, le),
            )
        };

        let name = section_name(elf, data, name_idx);

        out.push_str(&format!(
            "  {:3}  {:<20}  {:<16}  {:#010x}  {:#010x}  {:>10}  {}\n",
            i,
            truncate(&name, 20),
            sh_type_name(stype),
            addr,
            offset,
            size,
            sh_flags(sflags),
        ));
    }

    Ok(out)
}

// ── action: symbols ───────────────────────────────────────────────────────────

fn action_symbols(elf: &ElfHeader, data: &[u8]) -> Result<String, String> {
    let le = elf.data == 1;

    // Try .symtab (SHT_SYMTAB=2), fall back to .dynsym (SHT_DYNSYM=11)
    let (sym_off, sym_sz, linked_strtab) =
        if let Some((off, sz)) = find_section_by_type(elf, data, 2) {
            (off, sz, find_strtab_for_sym(elf, data, 2))
        } else if let Some((off, sz)) = find_section_by_type(elf, data, 11) {
            (off, sz, find_strtab_for_sym(elf, data, 11))
        } else {
            return Ok("No symbol table found in this ELF file (stripped binary).".into());
        };

    let (strtab_off, strtab_sz) = linked_strtab.unwrap_or((0, 0));
    let sym_size: usize = if elf.class == 1 { 16 } else { 24 };
    let count = sym_sz as usize / sym_size;

    let mut out = format!("SYMBOL TABLE — {} symbols\n", count);
    out.push_str(&"─".repeat(80));
    out.push('\n');
    out.push_str(&format!(
        "  {:4}  {:>10}  {:>10}  {:>4}  {:<6}  {:<8}  {}\n",
        "Idx", "Value", "Size", "Bind", "Type", "Section", "Name"
    ));
    out.push_str(&"─".repeat(80));
    out.push('\n');

    let limit = count.min(200);
    for i in 0..limit {
        let s_off = sym_off as usize + i * sym_size;
        if s_off + sym_size > data.len() {
            break;
        }

        let (name_idx, value, size, info, shndx) = if elf.class == 1 {
            (
                r32(data, s_off, le),
                r32(data, s_off + 4, le) as u64,
                r32(data, s_off + 8, le) as u64,
                data[s_off + 12],
                r16(data, s_off + 14, le),
            )
        } else {
            (
                r32(data, s_off, le),
                r64(data, s_off + 8, le),
                r64(data, s_off + 16, le),
                data[s_off + 4],
                r16(data, s_off + 6, le),
            )
        };

        let bind = match info >> 4 {
            0 => "LOCAL",
            1 => "GLOBAL",
            2 => "WEAK",
            _ => "?",
        };
        let stype = match info & 0xf {
            0 => "NOTYPE",
            1 => "OBJECT",
            2 => "FUNC",
            3 => "SECTION",
            4 => "FILE",
            _ => "?",
        };
        let section = match shndx {
            0 => "UND".to_string(),
            0xfff1 => "ABS".to_string(),
            0xfff2 => "COM".to_string(),
            n => format!("{}", n),
        };

        let name = if strtab_off > 0 {
            read_strtab_entry(data, strtab_off, strtab_sz, name_idx)
        } else {
            format!("{:#x}", name_idx)
        };

        out.push_str(&format!(
            "  {:4}  {:#010x}  {:>10}  {:>4}  {:<6}  {:<8}  {}\n",
            i, value, size, bind, stype, section, name
        ));
    }

    if count > limit {
        out.push_str(&format!(
            "  ... ({} more symbols not shown)\n",
            count - limit
        ));
    }

    Ok(out)
}

fn find_strtab_for_sym(elf: &ElfHeader, data: &[u8], sh_type: u32) -> Option<(u64, u64)> {
    let le = elf.data == 1;
    let shentsize = elf.shentsize as usize;
    if shentsize == 0 || elf.shnum == 0 {
        return None;
    }

    for i in 0..elf.shnum as usize {
        let sh_off = elf.shoff as usize + i * shentsize;
        if sh_off + shentsize > data.len() {
            break;
        }
        let stype = r32(data, sh_off + 4, le);
        if stype != sh_type {
            continue;
        }
        // sh_link is the section index of the associated string table
        let link = r32(data, sh_off + 28, le) as usize;
        // now get that section's offset and size
        let link_off = elf.shoff as usize + link * shentsize;
        if link_off + shentsize > data.len() {
            return None;
        }
        let (off, sz) = if elf.class == 1 {
            (
                r32(data, link_off + 16, le) as u64,
                r32(data, link_off + 20, le) as u64,
            )
        } else {
            (r64(data, link_off + 24, le), r64(data, link_off + 32, le))
        };
        return Some((off, sz));
    }
    None
}

// ── action: dynamic ───────────────────────────────────────────────────────────

fn action_dynamic(elf: &ElfHeader, data: &[u8]) -> Result<String, String> {
    let le = elf.data == 1;

    // Find PT_DYNAMIC segment
    let (dyn_off, dyn_sz) = match find_pt_dynamic(elf, data) {
        Some(v) => v,
        None => return Ok("No PT_DYNAMIC segment — this is a statically linked binary.".into()),
    };

    // Find the associated string table offset via DT_STRTAB and DT_STRSZ
    let entry_size: usize = if elf.class == 1 { 8 } else { 16 };
    let count = dyn_sz as usize / entry_size;

    let mut strtab_vaddr = 0u64;
    let mut strtab_sz = 0u64;
    let mut needed: Vec<u32> = Vec::new();
    let mut soname_idx: Option<u32> = None;
    let mut rpath_idx: Option<u32> = None;
    let mut runpath_idx: Option<u32> = None;

    for i in 0..count {
        let e_off = dyn_off as usize + i * entry_size;
        if e_off + entry_size > data.len() {
            break;
        }
        let (tag, val) = if elf.class == 1 {
            (r32(data, e_off, le) as i64, r32(data, e_off + 4, le) as u64)
        } else {
            let tag = r64(data, e_off, le) as i64;
            let val = r64(data, e_off + 8, le);
            (tag, val)
        };
        match tag {
            0 => break, // DT_NULL
            5 => strtab_vaddr = val,
            10 => strtab_sz = val,
            1 => needed.push(val as u32),
            14 => soname_idx = Some(val as u32),
            15 => rpath_idx = Some(val as u32),
            29 => runpath_idx = Some(val as u32),
            _ => {}
        }
    }

    // Resolve strtab virtual address to file offset
    // We do this by scanning PT_LOAD segments for the one covering strtab_vaddr
    let strtab_off = vaddr_to_offset(elf, data, strtab_vaddr).unwrap_or(strtab_vaddr);

    let stab_lookup = |idx: u32| -> String { read_strtab_entry(data, strtab_off, strtab_sz, idx) };

    let mut out = String::from("DYNAMIC LINKING INFO\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');

    if let Some(idx) = soname_idx {
        out.push_str(&format!("  SONAME:       {}\n", stab_lookup(idx)));
    }
    if let Some(idx) = rpath_idx {
        out.push_str(&format!("  RPATH:        {}\n", stab_lookup(idx)));
    }
    if let Some(idx) = runpath_idx {
        out.push_str(&format!("  RUNPATH:      {}\n", stab_lookup(idx)));
    }

    if needed.is_empty() {
        out.push_str("  Needed libs:  (none — statically linked or no DT_NEEDED entries)\n");
    } else {
        out.push_str(&format!("  Needed libs ({}):\n", needed.len()));
        for idx in &needed {
            out.push_str(&format!("    → {}\n", stab_lookup(*idx)));
        }
    }

    // Show all dynamic entries
    out.push('\n');
    out.push_str("  Dynamic entries:\n");
    out.push_str(&format!("  {:3}  {:<20}  {}\n", "#", "Tag", "Value"));
    out.push_str(&"─".repeat(60));
    out.push('\n');
    for i in 0..count {
        let e_off = dyn_off as usize + i * entry_size;
        if e_off + entry_size > data.len() {
            break;
        }
        let (tag, val) = if elf.class == 1 {
            (r32(data, e_off, le) as i64, r32(data, e_off + 4, le) as u64)
        } else {
            (r64(data, e_off, le) as i64, r64(data, e_off + 8, le))
        };
        if tag == 0 {
            break;
        }
        out.push_str(&format!(
            "  {:3}  {:<20}  {:#x}\n",
            i,
            dt_tag_name(tag),
            val
        ));
    }

    Ok(out)
}

fn find_pt_dynamic(elf: &ElfHeader, data: &[u8]) -> Option<(u64, u64)> {
    let le = elf.data == 1;
    let phentsize = elf.phentsize as usize;
    if phentsize == 0 {
        return None;
    }
    for i in 0..elf.phnum as usize {
        let ph_off = elf.phoff as usize + i * phentsize;
        if ph_off + phentsize > data.len() {
            break;
        }
        let ptype = r32(data, ph_off, le);
        if ptype != 2 {
            continue;
        } // PT_DYNAMIC
        let (offset, filesz) = if elf.class == 1 {
            (
                r32(data, ph_off + 4, le) as u64,
                r32(data, ph_off + 16, le) as u64,
            )
        } else {
            (r64(data, ph_off + 8, le), r64(data, ph_off + 32, le))
        };
        return Some((offset, filesz));
    }
    None
}

fn vaddr_to_offset(elf: &ElfHeader, data: &[u8], vaddr: u64) -> Option<u64> {
    let le = elf.data == 1;
    let phentsize = elf.phentsize as usize;
    if phentsize == 0 {
        return None;
    }
    for i in 0..elf.phnum as usize {
        let ph_off = elf.phoff as usize + i * phentsize;
        if ph_off + phentsize > data.len() {
            break;
        }
        let ptype = r32(data, ph_off, le);
        if ptype != 1 {
            continue;
        } // PT_LOAD only
        let (poffset, pvaddr, filesz) = if elf.class == 1 {
            (
                r32(data, ph_off + 4, le) as u64,
                r32(data, ph_off + 8, le) as u64,
                r32(data, ph_off + 16, le) as u64,
            )
        } else {
            (
                r64(data, ph_off + 8, le),
                r64(data, ph_off + 16, le),
                r64(data, ph_off + 32, le),
            )
        };
        if vaddr >= pvaddr && vaddr < pvaddr + filesz {
            return Some(poffset + (vaddr - pvaddr));
        }
    }
    None
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
