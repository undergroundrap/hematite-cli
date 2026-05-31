use serde_json::Value;
use std::fs;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("list");
    let path = args
        .get("file")
        .or_else(|| args.get("path"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "tar_tools: 'file' is required (path to a .tar archive)".to_string())?;

    let data = fs::read(path).map_err(|e| format!("tar_tools: cannot read '{path}': {e}"))?;

    check_compression(&data)?;

    match action {
        "list" | "" => action_list(&data, args),
        "info" => action_info(&data, path),
        "find" => action_find(&data, args),
        "extract" => action_extract(&data, args),
        other => Err(format!(
            "tar_tools: unknown action '{other}'. Valid: list, info, find, extract"
        )),
    }
}

// ── Compression detection ───────────────────────────────────────────────────

fn check_compression(data: &[u8]) -> Result<(), String> {
    if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
        return Err("tar_tools: file is gzip-compressed (.tar.gz / .tgz). \
             Decompress first: tar -tzf file.tar.gz (to list) or \
             tar -xzf file.tar.gz (to extract)"
            .to_string());
    }
    if data.len() >= 3 && &data[0..3] == b"BZh" {
        return Err("tar_tools: file is bzip2-compressed (.tar.bz2). \
             Decompress first: tar -tjf file.tar.bz2"
            .to_string());
    }
    if data.len() >= 6 && data[0] == 0xfd && &data[1..6] == b"7zXZ\x00" {
        return Err("tar_tools: file is xz-compressed (.tar.xz). \
             Decompress first: tar -tJf file.tar.xz"
            .to_string());
    }
    if data.len() >= 4 && data[0] == 0x28 && data[1] == 0xb5 && data[2] == 0x2f && data[3] == 0xfd {
        return Err("tar_tools: file is zstd-compressed (.tar.zst). \
             Decompress first: tar --zstd -tf file.tar.zst"
            .to_string());
    }
    Ok(())
}

// ── TAR parsing ─────────────────────────────────────────────────────────────

struct TarEntry {
    name: String,
    size: u64,
    mtime: u64,
    mode: u32,
    typeflag: u8,
    uname: String,
    linkname: String,
    data_start: usize,
}

fn parse_tar(data: &[u8]) -> Result<Vec<TarEntry>, String> {
    let mut entries = Vec::new();
    let mut pos = 0;
    let mut pending_long_name: Option<String> = None;

    while pos + 512 <= data.len() {
        // End-of-archive: two zero blocks
        if data[pos..pos + 512].iter().all(|&b| b == 0) {
            break;
        }

        let hdr = &data[pos..pos + 512];

        let typeflag = hdr[156];

        // GNU TAR long name extension: next entry has the real name
        if typeflag == b'L' {
            let sz = parse_octal(&hdr[124..136]) as usize;
            pos += 512;
            if pos + sz <= data.len() {
                let raw = &data[pos..pos + sz];
                let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
                pending_long_name = Some(String::from_utf8_lossy(&raw[..end]).to_string());
            }
            let blocks = (sz as u64 + 511) / 512;
            pos += blocks as usize * 512;
            continue;
        }

        let name_raw = read_cstr(&hdr[0..100]);
        let prefix = read_cstr(&hdr[345..500]);
        let name = if let Some(long) = pending_long_name.take() {
            long
        } else if !prefix.is_empty() {
            format!("{prefix}/{name_raw}")
        } else {
            name_raw
        };

        let size = parse_octal(&hdr[124..136]);
        let mtime = parse_octal(&hdr[136..148]);
        let mode = parse_octal(&hdr[100..108]) as u32;
        let uname = read_cstr(&hdr[265..297]);
        let linkname = read_cstr(&hdr[157..257]);

        pos += 512;
        let data_start = pos;
        let blocks = (size + 511) / 512;
        pos += blocks as usize * 512;

        if !name.is_empty() {
            entries.push(TarEntry {
                name,
                size,
                mtime,
                mode,
                typeflag,
                uname,
                linkname,
                data_start,
            });
        }
    }

    if entries.is_empty() && data.len() > 0 {
        return Err("tar_tools: no entries found — is this a valid uncompressed .tar file?".into());
    }

    Ok(entries)
}

fn read_cstr(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(bytes[..end].trim_ascii_end()).to_string()
}

fn parse_octal(bytes: &[u8]) -> u64 {
    // GNU TAR uses base-256 encoding for large values (first byte 0x80 or 0xFF)
    if !bytes.is_empty() && (bytes[0] == 0x80 || bytes[0] == 0xff) {
        let mut val: u64 = 0;
        for &b in &bytes[1..] {
            val = val.wrapping_shl(8) | b as u64;
        }
        return val;
    }
    let s = read_cstr(bytes);
    u64::from_str_radix(s.trim(), 8).unwrap_or(0)
}

fn type_label(t: u8) -> &'static str {
    match t {
        b'0' | 0 => "file",
        b'1' => "hardlink",
        b'2' => "symlink",
        b'3' => "chardev",
        b'4' => "blkdev",
        b'5' => "dir",
        b'6' => "fifo",
        b'7' => "contiguous",
        _ => "?",
    }
}

fn format_perms(mode: u32, typeflag: u8) -> String {
    let type_ch = if typeflag == b'5' { 'd' } else { '-' };
    let bits = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    let perms: String = bits
        .iter()
        .map(|&(mask, ch)| if mode & mask != 0 { ch } else { '-' })
        .collect();
    format!("{type_ch}{perms}")
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes}B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_mtime(ts: u64) -> String {
    // Rough ISO-ish UTC from Unix timestamp (no chrono dependency)
    if ts == 0 {
        return "1970-01-01".to_string();
    }
    let secs = ts;
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;

    // Compute year/month/day from days_since_epoch (Gregorian proleptic)
    let z = days_since_epoch as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let yr = if mo <= 2 { y + 1 } else { y };

    format!("{yr:04}-{mo:02}-{d:02} {h:02}:{m:02}")
}

// ── Actions ─────────────────────────────────────────────────────────────────

fn action_list(data: &[u8], args: &Value) -> Result<String, String> {
    let entries = parse_tar(data)?;
    let max = args.get("max").and_then(|v| v.as_u64()).unwrap_or(200) as usize;

    let mut out = String::new();
    out.push_str(&format!(
        "{:<12} {:<10} {:<17} {:<8} {}\n",
        "Permissions", "Size", "Modified", "Type", "Name"
    ));
    out.push_str(&"-".repeat(80));
    out.push('\n');

    let shown = entries.iter().take(max);
    for e in shown {
        let perms = format_perms(e.mode, e.typeflag);
        let sz = if e.typeflag == b'5' {
            "-".to_string()
        } else {
            format_size(e.size)
        };
        let date = format_mtime(e.mtime);
        let kind = type_label(e.typeflag);
        let name_display = if e.typeflag == b'2' && !e.linkname.is_empty() {
            format!("{} -> {}", e.name, e.linkname)
        } else {
            e.name.clone()
        };
        out.push_str(&format!(
            "{perms:<12} {sz:<10} {date:<17} {kind:<8} {name_display}\n"
        ));
    }

    if entries.len() > max {
        out.push_str(&format!(
            "\n... {} more entries (use max: {} to see all)\n",
            entries.len() - max,
            entries.len()
        ));
    }

    out.push('\n');
    out.push_str(&format!("Total: {} entries\n", entries.len()));
    Ok(out)
}

fn action_info(data: &[u8], path: &str) -> Result<String, String> {
    let entries = parse_tar(data)?;

    let mut files = 0u64;
    let mut dirs = 0u64;
    let mut symlinks = 0u64;
    let mut other = 0u64;
    let mut total_size: u64 = 0;
    let mut oldest = u64::MAX;
    let mut newest: u64 = 0;
    let mut users: std::collections::HashSet<String> = std::collections::HashSet::new();

    for e in &entries {
        match e.typeflag {
            b'0' | 0 => {
                files += 1;
                total_size += e.size;
            }
            b'5' => dirs += 1,
            b'2' => symlinks += 1,
            _ => other += 1,
        }
        if e.mtime > 0 {
            if e.mtime < oldest {
                oldest = e.mtime;
            }
            if e.mtime > newest {
                newest = e.mtime;
            }
        }
        if !e.uname.is_empty() {
            users.insert(e.uname.clone());
        }
    }

    let archive_size = data.len() as u64;
    let mut out = String::new();
    out.push_str("── TAR Archive Info ───────────────────────────────────────────\n");
    out.push_str(&format!("Archive:      {path}\n"));
    out.push_str(&format!(
        "Archive size: {} ({} bytes)\n",
        format_size(archive_size),
        archive_size
    ));
    out.push_str(&format!("Total entries: {}\n", entries.len()));
    out.push_str(&format!("  Files:     {files}\n"));
    out.push_str(&format!("  Dirs:      {dirs}\n"));
    out.push_str(&format!("  Symlinks:  {symlinks}\n"));
    if other > 0 {
        out.push_str(&format!("  Other:     {other}\n"));
    }
    out.push_str(&format!(
        "Content size: {} ({} bytes)\n",
        format_size(total_size),
        total_size
    ));
    if !users.is_empty() {
        let mut u: Vec<_> = users.into_iter().collect();
        u.sort();
        out.push_str(&format!("Owners:      {}\n", u.join(", ")));
    }
    if oldest != u64::MAX {
        out.push_str(&format!("Oldest:      {}\n", format_mtime(oldest)));
        out.push_str(&format!("Newest:      {}\n", format_mtime(newest)));
    }
    Ok(out)
}

fn action_find(data: &[u8], args: &Value) -> Result<String, String> {
    let query = args
        .get("query")
        .or_else(|| args.get("q"))
        .or_else(|| args.get("pattern"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "tar_tools: 'query' is required for find action".to_string())?
        .to_lowercase();

    let entries = parse_tar(data)?;
    let matches: Vec<_> = entries
        .iter()
        .filter(|e| e.name.to_lowercase().contains(&query))
        .collect();

    if matches.is_empty() {
        return Ok(format!("No entries matching '{query}'.\n"));
    }

    let mut out = String::new();
    out.push_str(&format!(
        "Found {} entries matching '{}':\n\n",
        matches.len(),
        query
    ));
    for e in &matches {
        let kind = type_label(e.typeflag);
        let sz = format_size(e.size);
        out.push_str(&format!("  [{kind}] {} ({})\n", e.name, sz));
    }
    Ok(out)
}

fn action_extract(data: &[u8], args: &Value) -> Result<String, String> {
    let target = args
        .get("entry")
        .or_else(|| args.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "tar_tools: 'entry' is required — name of the file to extract".to_string()
        })?;

    let entries = parse_tar(data)?;
    let entry = entries
        .iter()
        .find(|e| e.name == target || e.name.ends_with(&format!("/{target}")))
        .ok_or_else(|| format!("tar_tools: entry '{}' not found in archive", target))?;

    if entry.typeflag == b'5' {
        return Err(format!("tar_tools: '{}' is a directory", entry.name));
    }
    if entry.typeflag == b'2' {
        return Ok(format!(
            "tar_tools: '{}' is a symlink → {}",
            entry.name, entry.linkname
        ));
    }

    const MAX_BYTES: usize = 512 * 1024;
    let end = (entry.data_start + entry.size as usize).min(entry.data_start + MAX_BYTES);
    if end > data.len() {
        return Err(format!(
            "tar_tools: archive truncated — cannot read '{}' fully",
            entry.name
        ));
    }

    let raw = &data[entry.data_start..end];
    let text = String::from_utf8_lossy(raw);

    let mut out = String::new();
    out.push_str(&format!(
        "── {} ({}) ────\n",
        entry.name,
        format_size(entry.size)
    ));
    if entry.size as usize > MAX_BYTES {
        out.push_str(&format!(
            "[Showing first {} of {} bytes]\n\n",
            format_size(MAX_BYTES as u64),
            format_size(entry.size)
        ));
    }
    out.push_str(&text);
    Ok(out)
}

// ── Schema ───────────────────────────────────────────────────────────────────

pub fn tar_tools_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["list", "info", "find", "extract"],
                "description": "Operation: list (default — table of all entries), info (archive statistics), find (search by name), extract (read a text entry)"
            },
            "file": {
                "type": "string",
                "description": "Path to the .tar archive file (required)"
            },
            "max": {
                "type": "integer",
                "description": "list: max entries to display (default 200)"
            },
            "query": {
                "type": "string",
                "description": "find: substring to search in entry names"
            },
            "entry": {
                "type": "string",
                "description": "extract: exact name of the entry to read"
            }
        },
        "required": ["file"]
    })
}
