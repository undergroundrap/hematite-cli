use serde_json::Value;

pub fn schema() -> Value {
    serde_json::json!({
        "name": "macho_tools",
        "description": "Inspect macOS Mach-O binaries (executables, dylibs, frameworks, bundles, fat/universal binaries) without external tools — no otool or nm required. 5 actions: info (default — magic, file type EXE/DYLIB/BUNDLE/OBJECT/CORE, architecture x86-64/ARM64/ARM/x86/PPC, CPU subtype, flags PIE/DYLDLINK/TWOLEVEL, UUID, install name for dylibs, entry point, source version, build platform, min OS/SDK, code signature presence, load command count), segments (all LC_SEGMENT_64/LC_SEGMENT load commands with virtual address, file offset, size, flags, and embedded section names), sections (all sections across all segments with type/flags/address/size), imports (all imported dylibs from LC_LOAD_DYLIB/LC_LOAD_WEAK_DYLIB with install name, compatibility and current version), fat (architectures in a fat/universal binary with cputype/cpusubtype, offset, size). Pass 'file' for a path to a .macho/.dylib/.o/binary, or 'hex' for raw bytes as a hex string.",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "segments", "sections", "imports", "fat"],
                    "description": "Operation to perform (default: info)"
                },
                "file": { "type": "string", "description": "Path to a Mach-O binary" },
                "hex":  { "type": "string", "description": "Raw Mach-O bytes as a hex string (spaces/colons stripped)" }
            }
        }
    })
}

// ── constants ─────────────────────────────────────────────────────────────────

const MH_MAGIC: u32 = 0xFEED_FACE; // 32-bit LE
const MH_CIGAM: u32 = 0xCEFA_EDFE; // 32-bit BE
const MH_MAGIC_64: u32 = 0xFEED_FACF; // 64-bit LE
const MH_CIGAM_64: u32 = 0xCFFE_EDFE; // 64-bit BE
const FAT_MAGIC: u32 = 0xCAFE_BABE; // fat LE
const FAT_CIGAM: u32 = 0xBEBA_FECA; // fat BE

// ── byte readers ──────────────────────────────────────────────────────────────

#[allow(dead_code)]
fn r16le(d: &[u8], o: usize) -> Option<u16> {
    d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}
fn r32le(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn r64le(d: &[u8], o: usize) -> Option<u64> {
    d.get(o..o + 8)
        .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
}
fn r32be(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}
fn r64be(d: &[u8], o: usize) -> Option<u64> {
    d.get(o..o + 8)
        .map(|b| u64::from_be_bytes(b.try_into().unwrap()))
}

fn read_cstr(d: &[u8], off: usize) -> String {
    if off >= d.len() {
        return String::new();
    }
    let end = d[off..]
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(d.len() - off);
    String::from_utf8_lossy(&d[off..off + end]).to_string()
}

fn read_tag(d: &[u8], o: usize) -> String {
    d.get(o..o + 4)
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default()
}

// ── name tables ───────────────────────────────────────────────────────────────

fn cpu_name(cputype: u32) -> &'static str {
    let base = cputype & 0x00FF_FFFF;
    match base {
        7 if cputype & 0x0100_0000 != 0 => "x86-64",
        7 => "x86 (i386)",
        12 if cputype & 0x0100_0000 != 0 => "ARM64",
        12 => "ARM",
        18 if cputype & 0x0100_0000 != 0 => "PPC64",
        18 => "PPC",
        _ => "Unknown",
    }
}

fn filetype_name(ft: u32) -> &'static str {
    match ft {
        1 => "MH_OBJECT (relocatable object file)",
        2 => "MH_EXECUTE (demand-paged executable)",
        3 => "MH_FVMLIB (fixed-VM shared library)",
        4 => "MH_CORE (core dump)",
        5 => "MH_PRELOAD (preloaded executable)",
        6 => "MH_DYLIB (dynamic shared library)",
        7 => "MH_DYLINKER (dynamic linker)",
        8 => "MH_BUNDLE (bundle/plug-in)",
        9 => "MH_DYLIB_STUB (stub for static linking)",
        10 => "MH_DSYM (debug symbols companion)",
        11 => "MH_KEXT_BUNDLE (kernel extension)",
        _ => "Unknown",
    }
}

fn mh_flags(flags: u32) -> Vec<&'static str> {
    let mut out = Vec::new();
    if flags & 0x0001 != 0 {
        out.push("NOUNDEFS");
    }
    if flags & 0x0002 != 0 {
        out.push("INCRLINK");
    }
    if flags & 0x0004 != 0 {
        out.push("DYLDLINK");
    }
    if flags & 0x0008 != 0 {
        out.push("BINDATLOAD");
    }
    if flags & 0x0010 != 0 {
        out.push("PREBOUND");
    }
    if flags & 0x0040 != 0 {
        out.push("TWOLEVEL");
    }
    if flags & 0x0080 != 0 {
        out.push("FORCE_FLAT");
    }
    if flags & 0x0200 != 0 {
        out.push("NOMULTIDEFS");
    }
    if flags & 0x1000 != 0 {
        out.push("CANONICAL");
    }
    if flags & 0x4000 != 0 {
        out.push("WEAK_DEFINES");
    }
    if flags & 0x8000 != 0 {
        out.push("BINDS_TO_WEAK");
    }
    if flags & 0x0010_0000 != 0 {
        out.push("PIE");
    }
    if flags & 0x0040_0000 != 0 {
        out.push("HAS_TLV_DESCRIPTORS");
    }
    if flags & 0x0080_0000 != 0 {
        out.push("NO_HEAP_EXECUTION");
    }
    if flags & 0x0100_0000 != 0 {
        out.push("APP_EXTENSION_SAFE");
    }
    out
}

fn platform_name(p: u32) -> &'static str {
    match p {
        1 => "macOS",
        2 => "iOS",
        3 => "tvOS",
        4 => "watchOS",
        5 => "bridgeOS",
        6 => "Mac Catalyst",
        7 => "iOS Simulator",
        8 => "tvOS Simulator",
        9 => "watchOS Simulator",
        10 => "DriverKit",
        _ => "Unknown",
    }
}

fn fmt_ver(v: u32) -> String {
    format!("{}.{}.{}", v >> 16, (v >> 8) & 0xFF, v & 0xFF)
}

fn fmt_src_ver(v: u64) -> String {
    let e = (v >> 40) & 0xFF_FFFF;
    let d = (v >> 30) & 0x3FF;
    let c = (v >> 20) & 0x3FF;
    let b = (v >> 10) & 0x3FF;
    let a = v & 0x3FF;
    format!("{}.{}.{}.{}.{}", e, d, c, b, a)
}

// ── Mach-O header ─────────────────────────────────────────────────────────────

#[allow(dead_code)]
struct MachHdr {
    is_be: bool,
    is_64: bool,
    cputype: u32,
    cpusubtype: u32,
    filetype: u32,
    ncmds: u32,
    sizeofcmds: u32,
    flags: u32,
    hdr_size: usize, // 28 for 32-bit, 32 for 64-bit
}

fn parse_hdr(d: &[u8]) -> Result<MachHdr, String> {
    if d.len() < 4 {
        return Err("File too small".into());
    }
    let magic = u32::from_le_bytes(d[0..4].try_into().unwrap());
    let (is_be, is_64) = match magic {
        MH_MAGIC => (false, false),
        MH_CIGAM => (true, false),
        MH_MAGIC_64 => (false, true),
        MH_CIGAM_64 => (true, true),
        _ => {
            return Err(format!(
                "Not a Mach-O file — unknown magic 0x{:08X} (expected 0xFEEDFACE/F or 0xCAFEBABE)",
                magic
            ))
        }
    };
    let r32 = if is_be { r32be } else { r32le };
    let hdr_size = if is_64 { 32 } else { 28 };
    if d.len() < hdr_size {
        return Err("File truncated before Mach-O header".into());
    }
    Ok(MachHdr {
        is_be,
        is_64,
        cputype: r32(d, 4).unwrap(),
        cpusubtype: r32(d, 8).unwrap(),
        filetype: r32(d, 12).unwrap(),
        ncmds: r32(d, 16).unwrap(),
        sizeofcmds: r32(d, 20).unwrap(),
        flags: r32(d, 24).unwrap(),
        hdr_size,
    })
}

// ── load command walker ────────────────────────────────────────────────────────

struct LoadCmd {
    cmd: u32,
    cmdsize: u32,
    offset: usize,
}

fn load_commands(d: &[u8], hdr: &MachHdr) -> Vec<LoadCmd> {
    let r32 = if hdr.is_be { r32be } else { r32le };
    let mut cmds = Vec::new();
    let mut off = hdr.hdr_size;
    for _ in 0..hdr.ncmds {
        if off + 8 > d.len() {
            break;
        }
        let cmd = r32(d, off).unwrap_or(0);
        let cmdsize = r32(d, off + 4).unwrap_or(0);
        if cmdsize < 8 {
            break;
        }
        cmds.push(LoadCmd {
            cmd,
            cmdsize,
            offset: off,
        });
        off += cmdsize as usize;
    }
    cmds
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
            .map(|i| {
                u8::from_str_radix(&clean[i..i + 2], 16).map_err(|_| format!("Bad hex at {}", i))
            })
            .collect()
    } else {
        Err("Provide 'file' (Mach-O path) or 'hex' (raw bytes as hex string)".into())
    }
}

// ── fat binary ────────────────────────────────────────────────────────────────

fn handle_fat(d: &[u8], action: &str) -> Result<String, String> {
    // fat header: magic(4) + nfat_arch(4), big-endian
    let nfat = r32be(d, 4).ok_or("Fat header truncated")?;
    let mut arches = Vec::new();
    for i in 0..nfat.min(64) as usize {
        let base = 8 + i * 20;
        if d.len() < base + 20 {
            break;
        }
        let cputype = r32be(d, base).unwrap_or(0);
        let cpusubtype = r32be(d, base + 4).unwrap_or(0);
        let offset = r32be(d, base + 8).unwrap_or(0);
        let size = r32be(d, base + 12).unwrap_or(0);
        let align = r32be(d, base + 16).unwrap_or(0);
        arches.push((cputype, cpusubtype, offset, size, align));
    }

    if action != "fat" && action != "info" {
        // Dispatch to the first matching arch if a specific action is requested
        if let Some((_, _, offset, size, _)) = arches.first() {
            let start = *offset as usize;
            let end = (start + *size as usize).min(d.len());
            if start < d.len() {
                let slice = &d[start..end].to_vec();
                let hdr = parse_hdr(slice)?;
                return dispatch(slice, &hdr, action);
            }
        }
    }

    let mut out = format!(
        "═══ FAT/UNIVERSAL BINARY — {} architectures ═══\n\n",
        arches.len()
    );
    out.push_str(&format!(
        "  {:<16}  {:<12}  {:>12}  {:>12}  ALIGN\n",
        "Architecture", "CPU Subtype", "File Offset", "Size"
    ));
    out.push_str(&format!("  {}\n", "─".repeat(66)));
    for (cputype, cpusubtype, offset, size, align) in &arches {
        out.push_str(&format!(
            "  {:<16}  {:<12}  {:>12}  {:>12}  2^{}\n",
            cpu_name(*cputype),
            cpusubtype,
            offset,
            size,
            align
        ));
    }
    out.push_str("\n  Use action='info/segments/sections/imports' to inspect the first slice.\n");
    Ok(out)
}

// ── per-action dispatch ───────────────────────────────────────────────────────

fn dispatch(d: &[u8], hdr: &MachHdr, action: &str) -> Result<String, String> {
    let r32 = if hdr.is_be { r32be } else { r32le };
    let r64 = if hdr.is_be { r64be } else { r64le };
    let cmds = load_commands(d, hdr);

    match action {
        "info" => {
            let flags_vec = mh_flags(hdr.flags);
            let flags_str = if flags_vec.is_empty() {
                "none".to_string()
            } else {
                flags_vec.join(" | ")
            };

            let mut uuid_str = String::new();
            let mut entry_point: Option<u64> = None;
            let mut install_name = String::new();
            let mut src_version: Option<u64> = None;
            let mut platform_str = String::new();
            let mut minos_str = String::new();
            let mut sdk_str = String::new();
            let mut has_code_sig = false;
            let mut has_encrypt = false;

            for lc in &cmds {
                match lc.cmd {
                    0x1B => {
                        // LC_UUID
                        if let Some(ub) = d.get(lc.offset + 8..lc.offset + 24) {
                            uuid_str = format!(
                                "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                                ub[0], ub[1], ub[2], ub[3], ub[4], ub[5], ub[6], ub[7],
                                ub[8], ub[9], ub[10], ub[11], ub[12], ub[13], ub[14], ub[15]
                            );
                        }
                    }
                    0x2800_0028 => {
                        // LC_MAIN
                        entry_point = r64(d, lc.offset + 8);
                    }
                    0x80000028 => {
                        // LC_MAIN (alternate encoding)
                        entry_point = r64(d, lc.offset + 8);
                    }
                    0xD => {
                        // LC_ID_DYLIB
                        let name_off = r32(d, lc.offset + 8).unwrap_or(0) as usize;
                        if name_off > 0 {
                            install_name = read_cstr(d, lc.offset + name_off);
                        }
                    }
                    0x2A => {
                        // LC_SOURCE_VERSION
                        src_version = r64(d, lc.offset + 8);
                    }
                    0x32 => {
                        // LC_BUILD_VERSION
                        let platform = r32(d, lc.offset + 8).unwrap_or(0);
                        let minos = r32(d, lc.offset + 12).unwrap_or(0);
                        let sdk = r32(d, lc.offset + 16).unwrap_or(0);
                        platform_str = platform_name(platform).to_string();
                        minos_str = fmt_ver(minos);
                        sdk_str = fmt_ver(sdk);
                    }
                    0x25 => {
                        // LC_MIN_VERSION_MACOSX
                        let minos = r32(d, lc.offset + 8).unwrap_or(0);
                        let sdk = r32(d, lc.offset + 12).unwrap_or(0);
                        if platform_str.is_empty() {
                            platform_str = "macOS".to_string();
                            minos_str = fmt_ver(minos);
                            sdk_str = fmt_ver(sdk);
                        }
                    }
                    0x26 => {
                        // LC_MIN_VERSION_IPHONEOS
                        let minos = r32(d, lc.offset + 8).unwrap_or(0);
                        if platform_str.is_empty() {
                            platform_str = "iOS".to_string();
                            minos_str = fmt_ver(minos);
                        }
                    }
                    0x1D => {
                        // LC_CODE_SIGNATURE
                        has_code_sig = true;
                    }
                    0x21 | 0x2C => {
                        // LC_ENCRYPTION_INFO / LC_ENCRYPTION_INFO_64
                        has_encrypt = true;
                    }
                    _ => {}
                }
            }

            let mut out = String::from("═══ MACH-O BINARY INFO ═══\n\n");
            out.push_str(&format!(
                "  Architecture  : {} (cputype=0x{:X})\n",
                cpu_name(hdr.cputype),
                hdr.cputype
            ));
            out.push_str(&format!("  CPU Subtype   : {}\n", hdr.cpusubtype));
            out.push_str(&format!(
                "  Bit Width     : {}\n",
                if hdr.is_64 {
                    "64-bit (Mach-O 64)"
                } else {
                    "32-bit (Mach-O)"
                }
            ));
            out.push_str(&format!(
                "  Byte Order    : {}\n",
                if hdr.is_be {
                    "Big-endian"
                } else {
                    "Little-endian"
                }
            ));
            out.push_str(&format!(
                "  File Type     : {}\n",
                filetype_name(hdr.filetype)
            ));
            out.push_str(&format!(
                "  Flags         : 0x{:08X}  ({})\n",
                hdr.flags, flags_str
            ));
            out.push_str(&format!("  Load Commands : {}\n", hdr.ncmds));
            if !uuid_str.is_empty() {
                out.push_str(&format!("  UUID          : {}\n", uuid_str));
            }
            if let Some(ep) = entry_point {
                out.push_str(&format!("  Entry Point   : 0x{:X} (file offset)\n", ep));
            }
            if !install_name.is_empty() {
                out.push_str(&format!("  Install Name  : {}\n", install_name));
            }
            if let Some(sv) = src_version {
                out.push_str(&format!("  Source Version: {}\n", fmt_src_ver(sv)));
            }
            if !platform_str.is_empty() {
                out.push_str(&format!("  Platform      : {}\n", platform_str));
                out.push_str(&format!("  Min OS        : {}\n", minos_str));
                if !sdk_str.is_empty() {
                    out.push_str(&format!("  SDK           : {}\n", sdk_str));
                }
            }
            out.push('\n');
            out.push_str("  ── Security Features ──\n");
            out.push_str(&format!(
                "  PIE           : {}\n",
                if hdr.flags & 0x0010_0000 != 0 {
                    "YES"
                } else {
                    "NO"
                }
            ));
            out.push_str(&format!(
                "  Code Signed   : {}\n",
                if has_code_sig {
                    "YES (LC_CODE_SIGNATURE present)"
                } else {
                    "NO"
                }
            ));
            out.push_str(&format!(
                "  Encrypted     : {}\n",
                if has_encrypt {
                    "YES (LC_ENCRYPTION_INFO present)"
                } else {
                    "NO"
                }
            ));

            // Count segments, sections, imports
            let seg_count = cmds
                .iter()
                .filter(|c| c.cmd == 0x1 || c.cmd == 0x19)
                .count();
            let import_count = cmds
                .iter()
                .filter(|c| c.cmd == 0xC || c.cmd == 0x18 || c.cmd == 0x80000018)
                .count();
            out.push_str(&format!("\n  Segments      : {}\n", seg_count));
            out.push_str(&format!("  Import DYLIBs : {}\n", import_count));
            out.push_str(
                "\n  Use action='segments', 'sections', 'imports', or 'fat' for details.\n",
            );
            Ok(out)
        }

        "segments" => {
            let seg_cmd = if hdr.is_64 { 0x19u32 } else { 0x1u32 };
            let segs: Vec<_> = cmds.iter().filter(|c| c.cmd == seg_cmd).collect();
            let mut out = format!("═══ MACH-O SEGMENTS ({}) ═══\n\n", segs.len());
            out.push_str(&format!(
                "  {:<20}  {:>18}  {:>14}  {:>14}  NSECTS  FLAGS\n",
                "Name", "VmAddr", "VmSize", "FileSize"
            ));
            out.push_str(&format!("  {}\n", "─".repeat(90)));

            for lc in segs {
                let off = lc.offset;
                let segname = read_tag(d, off + 8);
                let segname2 = if d.get(off + 12).copied().unwrap_or(0) != 0 {
                    read_tag(d, off + 12)
                } else {
                    String::new()
                };
                let full_name = format!("{}{}", segname, segname2);
                let (vmaddr, vmsize, filesize, nsects) = if hdr.is_64 {
                    let va = r64(d, off + 24).unwrap_or(0);
                    let vs = r64(d, off + 32).unwrap_or(0);
                    let fs = r64(d, off + 48).unwrap_or(0);
                    let ns = r32(d, off + 64).unwrap_or(0);
                    (va, vs, fs, ns)
                } else {
                    let va = r32(d, off + 24).unwrap_or(0) as u64;
                    let vs = r32(d, off + 28).unwrap_or(0) as u64;
                    let fs = r32(d, off + 36).unwrap_or(0) as u64;
                    let ns = r32(d, off + 48).unwrap_or(0);
                    (va, vs, fs, ns)
                };
                let flags = if hdr.is_64 {
                    r32(d, off + 68)
                } else {
                    r32(d, off + 52)
                }
                .unwrap_or(0);
                let flag_str = match flags {
                    0 => "none".to_string(),
                    1 => "HIGHVM".to_string(),
                    2 => "FVMLIB".to_string(),
                    4 => "NORELOC".to_string(),
                    8 => "PROTECTED_VERSION_1".to_string(),
                    _ => format!("0x{:X}", flags),
                };
                out.push_str(&format!(
                    "  {:<20}  0x{:016X}  {:>14}  {:>14}  {:>6}  {}\n",
                    full_name, vmaddr, vmsize, filesize, nsects, flag_str
                ));

                // List section names inside segment
                let sec_size = if hdr.is_64 { 80 } else { 68 };
                let secs_base = off + if hdr.is_64 { 72 } else { 56 };
                for si in 0..nsects.min(20) as usize {
                    let so = secs_base + si * sec_size;
                    if so + 16 > d.len() {
                        break;
                    }
                    let secname = read_tag(d, so);
                    let secname2 = if d.get(so + 4).copied().unwrap_or(0) != 0 {
                        read_tag(d, so + 4)
                    } else {
                        String::new()
                    };
                    out.push_str(&format!("    ├─ {}{}\n", secname, secname2));
                }
                if nsects > 20 {
                    out.push_str(&format!("    └─ ... and {} more\n", nsects - 20));
                }
            }
            Ok(out)
        }

        "sections" => {
            let seg_cmd = if hdr.is_64 { 0x19u32 } else { 0x1u32 };
            let sec_size = if hdr.is_64 { 80usize } else { 68 };
            let secs_base_off = if hdr.is_64 { 72usize } else { 56 };

            let mut all_secs = Vec::new();
            for lc in cmds.iter().filter(|c| c.cmd == seg_cmd) {
                let nsects = if hdr.is_64 {
                    r32(d, lc.offset + 64)
                } else {
                    r32(d, lc.offset + 48)
                }
                .unwrap_or(0);
                let sbase = lc.offset + secs_base_off;
                for si in 0..nsects.min(512) as usize {
                    let so = sbase + si * sec_size;
                    if so + 16 > d.len() {
                        break;
                    }
                    let secname = read_tag(d, so);
                    let secname2 = if d.get(so + 4).copied().unwrap_or(0) != 0 {
                        read_tag(d, so + 4)
                    } else {
                        String::new()
                    };
                    let segname = read_tag(d, so + 8);
                    let segname2 = if d.get(so + 12).copied().unwrap_or(0) != 0 {
                        read_tag(d, so + 12)
                    } else {
                        String::new()
                    };
                    let (addr, size) = if hdr.is_64 {
                        (r64(d, so + 16).unwrap_or(0), r64(d, so + 24).unwrap_or(0))
                    } else {
                        (
                            r32(d, so + 16).unwrap_or(0) as u64,
                            r32(d, so + 20).unwrap_or(0) as u64,
                        )
                    };
                    let sec_type_off = if hdr.is_64 { so + 64 } else { so + 52 };
                    let sec_type = r32(d, sec_type_off).unwrap_or(0);
                    let type_name = match sec_type & 0xFF {
                        0 => "REGULAR",
                        1 => "ZEROFILL",
                        2 => "CSTRING_LITERALS",
                        3 => "4BYTE_LITERALS",
                        4 => "8BYTE_LITERALS",
                        5 => "LITERAL_POINTERS",
                        6 => "NON_LAZY_SYMBOL_POINTERS",
                        7 => "LAZY_SYMBOL_POINTERS",
                        8 => "SYMBOL_STUBS",
                        9 => "MOD_INIT_FUNC_POINTERS",
                        10 => "MOD_TERM_FUNC_POINTERS",
                        11 => "COALESCED",
                        15 => "DTRACE_DOF",
                        16 => "LAZY_DYLIB_SYMBOL_POINTERS",
                        18 => "THREAD_LOCAL_VARIABLES",
                        _ => "OTHER",
                    };
                    all_secs.push((segname, segname2, secname, secname2, addr, size, type_name));
                }
            }

            let mut out = format!("═══ MACH-O SECTIONS ({}) ═══\n\n", all_secs.len());
            out.push_str(&format!(
                "  {:<12}  {:<20}  {:>18}  {:>14}  TYPE\n",
                "Segment", "Section", "Address", "Size"
            ));
            out.push_str(&format!("  {}\n", "─".repeat(78)));
            for (seg, seg2, sec, sec2, addr, size, tname) in &all_secs {
                out.push_str(&format!(
                    "  {:<12}  {:<20}  0x{:016X}  {:>14}  {}\n",
                    format!("{}{}", seg, seg2),
                    format!("{}{}", sec, sec2),
                    addr,
                    size,
                    tname
                ));
            }
            Ok(out)
        }

        "imports" => {
            // LC_LOAD_DYLIB=0xC, LC_LOAD_WEAK_DYLIB=0x18/0x80000018, LC_REEXPORT_DYLIB=0x1F
            let import_cmds: Vec<_> = cmds
                .iter()
                .filter(|c| c.cmd == 0xC || c.cmd == 0x18 || c.cmd == 0x8000_0018 || c.cmd == 0x1F)
                .collect();

            if import_cmds.is_empty() {
                return Ok("No imported dylibs found.\n".to_string());
            }

            let mut out = format!("═══ MACH-O IMPORTS — {} dylibs ═══\n\n", import_cmds.len());
            out.push_str(&format!(
                "  {:<8}  {:<20}  {:<20}  INSTALL PATH\n",
                "TYPE", "CompatVer", "CurrentVer"
            ));
            out.push_str(&format!("  {}\n", "─".repeat(80)));

            for lc in import_cmds {
                let name_off = r32(d, lc.offset + 8).unwrap_or(0) as usize;
                let compat = r32(d, lc.offset + 12).unwrap_or(0);
                let current = r32(d, lc.offset + 16).unwrap_or(0);
                let name = if name_off > 0 && name_off < lc.cmdsize as usize {
                    read_cstr(d, lc.offset + name_off)
                } else {
                    "<unknown>".to_string()
                };
                let kind = match lc.cmd {
                    0xC => "LOAD",
                    0x18 | 0x8000_0018 => "WEAK",
                    0x1F => "REEXPORT",
                    _ => "?",
                };
                out.push_str(&format!(
                    "  {:<8}  {:<20}  {:<20}  {}\n",
                    kind,
                    fmt_ver(compat),
                    fmt_ver(current),
                    name
                ));
            }
            Ok(out)
        }

        other => Err(format!(
            "Unknown action '{}'. Valid: info, segments, sections, imports, fat",
            other
        )),
    }
}

// ── public entry point ────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let d = read_bytes(args)?;
    let action = args["action"].as_str().unwrap_or("info");

    if d.len() < 4 {
        return Err("File too small to be a Mach-O binary".into());
    }

    let magic = u32::from_le_bytes(d[0..4].try_into().unwrap());
    if magic == FAT_MAGIC || magic == FAT_CIGAM {
        return handle_fat(&d, action);
    }

    let hdr = parse_hdr(&d)?;
    dispatch(&d, &hdr, action)
}
