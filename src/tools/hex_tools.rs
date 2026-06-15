pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("dump");

    match action {
        "dump" => dump(args),
        "strings" => strings(args),
        "bytes" => bytes_info(args),
        "analyze" => analyze(args),
        "to-hex" => to_hex(args),
        "from-hex" => from_hex(args),
        other => Err(format!(
            "hex_tools: unknown action '{other}'. Valid: dump, strings, bytes, analyze, to-hex, from-hex"
        )),
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn load_bytes(args: &serde_json::Value) -> Result<Vec<u8>, String> {
    if let Some(file) = args.get("file").and_then(|v| v.as_str()) {
        let path = if std::path::Path::new(file).is_absolute() {
            std::path::PathBuf::from(file)
        } else {
            let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
                std::path::PathBuf::from(r)
            } else {
                crate::tools::file_ops::workspace_root()
            };
            root.join(file)
        };
        if !path.exists() {
            return Err(format!("hex_tools: file not found: {}", path.display()));
        }
        return std::fs::read(&path)
            .map_err(|e| format!("hex_tools: cannot read '{}': {e}", path.display()));
    }
    if let Some(hex) = args.get("hex").and_then(|v| v.as_str()) {
        return decode_hex_str(hex);
    }
    if let Some(text) = args
        .get("text")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
        return Ok(text.as_bytes().to_vec());
    }
    Err(
        "hex_tools: 'file' (path), 'hex' (hex string), or 'text' (UTF-8 input) is required"
            .to_string(),
    )
}

fn decode_hex_str(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if !clean.len().is_multiple_of(2) {
        return Err("hex_tools: odd number of hex digits in input".to_string());
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&clean[i..i + 2], 16)
                .map_err(|e| format!("hex_tools: invalid hex byte at {i}: {e}"))
        })
        .collect()
}

fn is_printable_ascii(b: u8) -> bool {
    (0x20..=0x7E).contains(&b)
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn dump(args: &serde_json::Value) -> Result<String, String> {
    let bytes = load_bytes(args)?;
    let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(16) as usize;
    let max_bytes = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(4096) as usize;
    let offset_base = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    let total = bytes.len();
    let shown = bytes.len().min(max_bytes);
    let truncated = total > max_bytes;

    let file_label = args
        .get("file")
        .and_then(|v| v.as_str())
        .unwrap_or("(inline)");

    let mut out = format!(
        "HEX DUMP: {file_label}  ({total} bytes)\n{}\n",
        "─".repeat(60)
    );

    for (chunk_idx, chunk) in bytes[..shown].chunks(width).enumerate() {
        let offset = offset_base + chunk_idx * width;
        // Offset
        out.push_str(&format!("{offset:08x}  "));
        // Hex bytes (first half)
        let half = width / 2;
        for (i, b) in chunk.iter().enumerate() {
            if i == half {
                out.push(' ');
            }
            out.push_str(&format!("{b:02x} "));
        }
        // Pad if chunk is shorter than width
        let missing = width - chunk.len();
        let pad = missing * 3
            + if chunk.len() <= half && missing > 0 {
                1
            } else {
                0
            };
        out.push_str(&" ".repeat(pad));
        // ASCII sidebar
        out.push_str(" |");
        for b in chunk {
            out.push(if is_printable_ascii(*b) {
                *b as char
            } else {
                '.'
            });
        }
        out.push_str("|\n");
    }

    if truncated {
        out.push_str(&format!(
            "\n(showing first {max_bytes} of {total} bytes — use 'limit' to increase)\n"
        ));
    }
    Ok(out)
}

fn strings(args: &serde_json::Value) -> Result<String, String> {
    let bytes = load_bytes(args)?;
    let min_len = args.get("min").and_then(|v| v.as_u64()).unwrap_or(4) as usize;
    let max_results = args.get("max").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
    let show_offsets = args
        .get("offsets")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut results: Vec<(usize, String)> = Vec::new();
    let mut current = String::new();
    let mut start = 0usize;

    for (i, &b) in bytes.iter().enumerate() {
        if is_printable_ascii(b) || b == b'\t' {
            if current.is_empty() {
                start = i;
            }
            current.push(b as char);
        } else {
            if current.len() >= min_len {
                results.push((start, current.clone()));
            }
            current.clear();
        }
    }
    if current.len() >= min_len {
        results.push((start, current));
    }

    let truncated = results.len() > max_results;
    let shown = results.len().min(max_results);

    let file_label = args
        .get("file")
        .and_then(|v| v.as_str())
        .unwrap_or("(inline)");
    let mut out = format!(
        "HEX STRINGS: {file_label}  (min {min_len} chars)\n{}\n",
        "─".repeat(60)
    );

    for (offset, s) in results.iter().take(shown) {
        if show_offsets {
            out.push_str(&format!("{offset:08x}  {s}\n"));
        } else {
            out.push_str(&format!("{s}\n"));
        }
    }

    out.push('\n');
    out.push_str(&format!("{shown} string(s) found"));
    if truncated {
        out.push_str(&format!(" (showing first {max_results})"));
    }
    out.push('\n');
    Ok(out)
}

fn bytes_info(args: &serde_json::Value) -> Result<String, String> {
    let bytes = load_bytes(args)?;

    let total = bytes.len();
    let printable = bytes.iter().filter(|&&b| is_printable_ascii(b)).count();
    let null_count = bytes.iter().filter(|&&b| b == 0).count();
    let high_count = bytes.iter().filter(|&&b| b > 0x7F).count();

    // Simple byte frequency — top 8 most common
    let mut freq = [0u32; 256];
    for &b in &bytes {
        freq[b as usize] += 1;
    }
    let mut freq_pairs: Vec<(u8, u32)> = freq
        .iter()
        .enumerate()
        .filter(|(_, &c)| c > 0)
        .map(|(b, &c)| (b as u8, c))
        .collect();
    freq_pairs.sort_by_key(|b| std::cmp::Reverse(b.1));

    // Shannon entropy estimate
    let entropy = freq_pairs.iter().fold(0.0f64, |acc, (_, count)| {
        let p = *count as f64 / total as f64;
        acc - p * p.log2()
    });

    let mut out = format!("HEX BYTES INFO\n{}\n", "─".repeat(50));
    out.push_str(&format!("Total bytes   : {total}\n"));
    out.push_str(&format!(
        "Printable ASCII: {} ({:.1}%)\n",
        printable,
        100.0 * printable as f64 / total.max(1) as f64
    ));
    out.push_str(&format!("Null bytes    : {null_count}\n"));
    out.push_str(&format!("High bytes    : {high_count} (>0x7F)\n"));
    out.push_str(&format!("Entropy       : {entropy:.2} bits/byte\n"));
    out.push_str("  (0=flat, 8=random; >7.5 suggests compression/encryption)\n");

    out.push_str("\nTop bytes by frequency:\n");
    for (b, count) in freq_pairs.iter().take(8) {
        let display = if is_printable_ascii(*b) {
            format!("  {:02x}  '{}'  {count}\n", b, *b as char)
        } else {
            format!("  {:02x}  ---  {count}\n", b)
        };
        out.push_str(&display);
    }
    Ok(out)
}

fn analyze(args: &serde_json::Value) -> Result<String, String> {
    let bytes = load_bytes(args)?;
    let file_label = args
        .get("file")
        .and_then(|v| v.as_str())
        .unwrap_or("(inline)");

    let total = bytes.len();
    let magic = if bytes.len() >= 16 {
        &bytes[..16]
    } else {
        &bytes
    };

    let file_type = detect_type(magic);

    let mut out = format!("HEX ANALYZE: {file_label}\n{}\n", "─".repeat(50));
    out.push_str(&format!("Size      : {total} bytes\n"));
    out.push_str(&format!("Detected  : {file_type}\n"));

    // First 16 bytes as hex
    out.push_str("Magic     : ");
    for b in magic {
        out.push_str(&format!("{b:02x} "));
    }
    out.push('\n');

    // Entropy of first 256 bytes (fast estimate)
    let sample = if bytes.len() > 256 {
        &bytes[..256]
    } else {
        &bytes
    };
    let mut freq = [0u32; 256];
    for &b in sample {
        freq[b as usize] += 1;
    }
    let entropy = freq.iter().fold(0.0f64, |acc, &count| {
        if count == 0 {
            return acc;
        }
        let p = count as f64 / sample.len() as f64;
        acc - p * p.log2()
    });
    out.push_str(&format!("Entropy   : {entropy:.2} bits/byte"));
    if entropy > 7.5 {
        out.push_str("  ⚠ high — likely compressed or encrypted");
    } else if entropy < 2.0 {
        out.push_str("  low — sparse or text-like");
    }
    out.push('\n');

    Ok(out)
}

fn detect_type(magic: &[u8]) -> &'static str {
    if magic.starts_with(b"\x89PNG\r\n\x1a\n") {
        return "PNG image";
    }
    if magic.starts_with(b"\xFF\xD8\xFF") {
        return "JPEG image";
    }
    if magic.starts_with(b"GIF87a") || magic.starts_with(b"GIF89a") {
        return "GIF image";
    }
    if magic.starts_with(b"RIFF") && magic.len() >= 12 && &magic[8..12] == b"WEBP" {
        return "WebP image";
    }
    if magic.starts_with(b"BM") {
        return "BMP image";
    }
    if magic.starts_with(b"\x49\x49\x2A\x00") || magic.starts_with(b"\x4D\x4D\x00\x2A") {
        return "TIFF image";
    }
    if magic.starts_with(b"\x00\x00\x01\x00") {
        return "ICO file";
    }
    if magic.starts_with(b"PK\x03\x04") {
        return "ZIP archive (or .jar/.docx/.xlsx/.apk)";
    }
    if magic.starts_with(b"PK\x05\x06") || magic.starts_with(b"PK\x07\x08") {
        return "ZIP archive (empty/spanned)";
    }
    if magic.starts_with(b"\x1F\x8B") {
        return "gzip compressed data";
    }
    if magic.starts_with(b"BZh") {
        return "bzip2 compressed data";
    }
    if magic.starts_with(b"\xFD7zXZ\x00") {
        return "XZ compressed data";
    }
    if magic.starts_with(b"7z\xBC\xAF\x27\x1C") {
        return "7-Zip archive";
    }
    if magic.starts_with(b"Rar!\x1A\x07") {
        return "RAR archive";
    }
    if magic.starts_with(b"%PDF-") {
        return "PDF document";
    }
    if magic.starts_with(b"\x7FELF") {
        return "ELF executable (Linux/Unix binary)";
    }
    if magic.starts_with(b"MZ") {
        return "PE executable (Windows .exe/.dll)";
    }
    if magic.starts_with(b"\xCE\xFA\xED\xFE") || magic.starts_with(b"\xCF\xFA\xED\xFE") {
        return "Mach-O binary (macOS)";
    }
    if magic.starts_with(b"\xCA\xFE\xBA\xBE") {
        return "Java class file or Mach-O fat binary";
    }
    if magic.starts_with(b"CAFEBABE") {
        return "Java class file";
    }
    if magic.starts_with(b"SQLite format 3") {
        return "SQLite database";
    }
    if magic.starts_with(b"ID3")
        || (magic.len() >= 2 && magic[0] == 0xFF && (magic[1] & 0xE0) == 0xE0)
    {
        return "MP3 audio";
    }
    if magic.starts_with(b"fLaC") {
        return "FLAC audio";
    }
    if magic.starts_with(b"OggS") {
        return "Ogg container (audio/video)";
    }
    if magic.starts_with(b"RIFF") {
        return "RIFF file (WAV/AVI)";
    }
    if magic.starts_with(b"\x00\x00\x00") && magic.len() >= 8 && &magic[4..8] == b"ftyp" {
        return "MP4/M4V/MOV video (ISO base media)";
    }
    if magic.starts_with(b"\x1A\x45\xDF\xA3") {
        return "Matroska/WebM video";
    }
    if magic.starts_with(b"FLV\x01") {
        return "Flash Video (FLV)";
    }
    if magic.starts_with(b"<!DOCTYPE html") || magic.starts_with(b"<!doctype html") {
        return "HTML document";
    }
    if magic.starts_with(b"<?xml") {
        return "XML document";
    }
    if magic.starts_with(b"{\n") || magic.starts_with(b"{ ") || magic.starts_with(b"{\"") {
        return "JSON data";
    }
    if magic.starts_with(b"[package]") || magic.starts_with(b"[dependencies") {
        return "TOML (likely Cargo.toml)";
    }
    if magic.starts_with(b"#!") {
        return "Script (shebang)";
    }
    if magic.starts_with(b"\xEF\xBB\xBF") {
        return "UTF-8 text with BOM";
    }
    if magic.starts_with(b"\xFF\xFE") || magic.starts_with(b"\xFE\xFF") {
        return "UTF-16 text with BOM";
    }
    // Check if all printable ASCII — likely plain text
    if !magic.is_empty() && magic.iter().all(|&b| (0x09..0x80).contains(&b)) {
        return "Plain text (UTF-8 or ASCII)";
    }
    "Unknown / binary data"
}

fn to_hex(args: &serde_json::Value) -> Result<String, String> {
    let bytes = load_bytes(args)?;
    let separator = args.get("sep").and_then(|v| v.as_str()).unwrap_or(" ");
    let upper = args.get("upper").and_then(|v| v.as_bool()).unwrap_or(false);

    let hex: String = bytes
        .iter()
        .map(|b| {
            if upper {
                format!("{b:02X}")
            } else {
                format!("{b:02x}")
            }
        })
        .collect::<Vec<_>>()
        .join(separator);

    let mut out = format!("HEX ENCODE\n{}\n", "─".repeat(50));
    out.push_str(&hex);
    out.push('\n');
    Ok(out)
}

fn from_hex(args: &serde_json::Value) -> Result<String, String> {
    let hex = args
        .get("hex")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("text"))
        .and_then(|v| v.as_str())
        .ok_or("hex_tools from-hex: 'hex' is required")?;

    let bytes = decode_hex_str(hex)?;

    let mut out = format!("HEX DECODE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input bytes : {}\n", bytes.len()));

    // Try UTF-8 interpretation
    match std::str::from_utf8(&bytes) {
        Ok(s) => {
            out.push_str("Encoding    : UTF-8 text\n");
            out.push_str(&format!("Text        : {s}\n"));
        }
        Err(_) => {
            out.push_str("Encoding    : binary (not valid UTF-8)\n");
            out.push_str("Bytes       : ");
            for b in &bytes {
                out.push_str(&format!("{b:02x} "));
            }
            out.push('\n');
        }
    }
    Ok(out)
}
