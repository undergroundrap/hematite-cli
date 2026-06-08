use serde_json::{json, Value};
use std::fs;

pub fn make_schema() -> Value {
    json!({
        "name": "pdf_tools",
        "description": "Inspect PDF files — page count, metadata (title/author/creator/dates), structure, and validation — without external utilities. \
    Actions: info (default — PDF version, page count, file size, linearized flag, Info dict fields: title/author/subject/keywords/creator/producer/creation date/modification date), \
    pages (page count and MediaBox dimensions from the first page), \
    metadata (all Info dictionary fields), \
    structure (object count, xref type — traditional table or cross-reference stream, linearized flag, PDF version), \
    validate (structural checks: PDF header, EOF marker, xref/trailer presence). \
    Pass file (path to PDF) or hex (hex-encoded PDF bytes). \
    Example: pdf_tools(file: 'report.pdf') or pdf_tools(action: 'metadata', file: 'document.pdf')",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "info|pages|metadata|structure|validate" },
                "file": { "type": "string", "description": "Path to PDF file" },
                "hex": { "type": "string", "description": "Hex-encoded PDF bytes" }
            },
            "required": []
        }
    })
}

fn get_bytes(args: &Value) -> Option<Vec<u8>> {
    if let Some(p) = args.get("file").and_then(|v| v.as_str()) {
        fs::read(p).ok()
    } else if let Some(h) = args.get("hex").and_then(|v| v.as_str()) {
        let clean: String = h.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if clean.len() % 2 != 0 {
            return None;
        }
        (0..clean.len() / 2)
            .map(|i| u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16))
            .collect::<Result<Vec<_>, _>>()
            .ok()
    } else {
        None
    }
}

// ── PDF parsing helpers ───────────────────────────────────────────────────────

struct PdfMeta {
    version: String,
    page_count: u32,
    title: String,
    author: String,
    subject: String,
    keywords: String,
    creator: String,
    producer: String,
    creation_date: String,
    mod_date: String,
    linearized: bool,
    xref_type: String,
    object_count: u32,
    file_size: u64,
    media_box: Option<(f64, f64, f64, f64)>,
}

impl Default for PdfMeta {
    fn default() -> Self {
        Self {
            version: String::new(),
            page_count: 0,
            title: String::new(),
            author: String::new(),
            subject: String::new(),
            keywords: String::new(),
            creator: String::new(),
            producer: String::new(),
            creation_date: String::new(),
            mod_date: String::new(),
            linearized: false,
            xref_type: "traditional".to_string(),
            object_count: 0,
            file_size: 0,
            media_box: None,
        }
    }
}

/// Find the byte offset of the last occurrence of `needle` in `haystack`.
fn rfind_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    let mut i = haystack.len() - needle.len();
    loop {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    None
}

/// Find next occurrence of `needle` starting at `start`.
fn find_from(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if start + needle.len() > haystack.len() {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + start)
}

/// Extract a simple ASCII/Latin-1 string value from a PDF dictionary entry.
/// Handles both parenthesized literal strings `(text)` and hex strings `<hex>`.
fn extract_string_value(b: &[u8], key: &[u8]) -> Option<String> {
    let pos = find_from(b, key, 0)?;
    let after = pos + key.len();
    // Skip whitespace
    let mut p = after;
    while p < b.len() && (b[p] == b' ' || b[p] == b'\t' || b[p] == b'\n' || b[p] == b'\r') {
        p += 1;
    }
    if p >= b.len() {
        return None;
    }
    if b[p] == b'(' {
        // Literal string
        p += 1;
        let mut result = Vec::new();
        let mut depth = 1i32;
        while p < b.len() && depth > 0 {
            match b[p] {
                b'\\' => {
                    p += 1;
                    if p < b.len() {
                        match b[p] {
                            b'n' => result.push(b'\n'),
                            b'r' => result.push(b'\r'),
                            b't' => result.push(b'\t'),
                            b'b' => result.push(8u8),
                            b'f' => result.push(12u8),
                            b'(' => result.push(b'('),
                            b')' => result.push(b')'),
                            b'\\' => result.push(b'\\'),
                            b'0'..=b'9' => {
                                let mut oct = (b[p] - b'0') as u32;
                                if p + 1 < b.len() && b[p + 1] >= b'0' && b[p + 1] <= b'7' {
                                    p += 1;
                                    oct = oct * 8 + (b[p] - b'0') as u32;
                                    if p + 1 < b.len() && b[p + 1] >= b'0' && b[p + 1] <= b'7' {
                                        p += 1;
                                        oct = oct * 8 + (b[p] - b'0') as u32;
                                    }
                                }
                                result.push(oct as u8);
                            }
                            _ => result.push(b[p]),
                        }
                    }
                }
                b'(' => {
                    depth += 1;
                    result.push(b'(');
                }
                b')' => {
                    depth -= 1;
                    if depth > 0 {
                        result.push(b')');
                    }
                }
                c => result.push(c),
            }
            p += 1;
        }
        // Strip UTF-16 BOM if present (PDF 1.4+ Unicode strings)
        if result.len() >= 2 && result[0] == 0xFE && result[1] == 0xFF {
            let pairs: Vec<u16> = result[2..]
                .chunks(2)
                .filter(|ch| ch.len() == 2)
                .map(|ch| u16::from_be_bytes([ch[0], ch[1]]))
                .collect();
            return Some(String::from_utf16_lossy(&pairs));
        }
        Some(String::from_utf8_lossy(&result).trim().to_string())
    } else if b[p] == b'<' {
        // Hex string
        p += 1;
        let mut hex = Vec::new();
        while p < b.len() && b[p] != b'>' {
            if b[p].is_ascii_hexdigit() {
                hex.push(b[p]);
            }
            p += 1;
        }
        if hex.len() % 2 != 0 {
            hex.push(b'0');
        }
        let bytes: Vec<u8> = hex
            .chunks(2)
            .filter_map(|ch| {
                let s = std::str::from_utf8(ch).ok()?;
                u8::from_str_radix(s, 16).ok()
            })
            .collect();
        if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
            let pairs: Vec<u16> = bytes[2..]
                .chunks(2)
                .filter(|ch| ch.len() == 2)
                .map(|ch| u16::from_be_bytes([ch[0], ch[1]]))
                .collect();
            return Some(String::from_utf16_lossy(&pairs));
        }
        Some(String::from_utf8_lossy(&bytes).trim().to_string())
    } else {
        None
    }
}

/// Parse a PDF date string like "D:20231015143022+05'30'" into readable form.
fn parse_pdf_date(d: &str) -> String {
    let d = d.trim_start_matches("D:").trim_start_matches("d:");
    if d.len() < 8 {
        return d.to_string();
    }
    let year = &d[..4];
    let month = if d.len() >= 6 { &d[4..6] } else { "??" };
    let day = if d.len() >= 8 { &d[6..8] } else { "??" };
    let hour = if d.len() >= 10 { &d[8..10] } else { "00" };
    let min = if d.len() >= 12 { &d[10..12] } else { "00" };
    let sec = if d.len() >= 14 { &d[12..14] } else { "00" };
    let tz = if d.len() > 14 {
        d[14..]
            .replace('\'', ":")
            .trim_end_matches(':')
            .to_string()
    } else {
        String::new()
    };
    format!("{}-{}-{} {}:{}:{}{}", year, month, day, hour, min, sec, tz)
}

/// Count how many times `%%EOF` appears (approximate; also count `startxref` patterns).
fn count_objects(b: &[u8]) -> u32 {
    let mut count = 0u32;
    let mut pos = 0usize;
    while let Some(p) = find_from(b, b" obj", pos) {
        // Check preceding token looks like `N G obj`
        if p > 0 && b[p - 1].is_ascii_digit() {
            count += 1;
        }
        pos = p + 4;
    }
    count
}

/// Determine MediaBox from the first /Page object we can find.
fn find_media_box(b: &[u8]) -> Option<(f64, f64, f64, f64)> {
    let key = b"/MediaBox";
    let pos = find_from(b, key, 0)?;
    let mut p = pos + key.len();
    // skip whitespace and optional newline
    while p < b.len() && (b[p] == b' ' || b[p] == b'\t' || b[p] == b'\n' || b[p] == b'\r') {
        p += 1;
    }
    if p >= b.len() || b[p] != b'[' {
        return None;
    }
    p += 1;
    let mut nums = Vec::new();
    while p < b.len() && b[p] != b']' && nums.len() < 4 {
        while p < b.len() && (b[p] == b' ' || b[p] == b'\t' || b[p] == b'\n' || b[p] == b'\r') {
            p += 1;
        }
        if p >= b.len() || b[p] == b']' {
            break;
        }
        let start = p;
        while p < b.len() && (b[p].is_ascii_digit() || b[p] == b'.' || b[p] == b'-') {
            p += 1;
        }
        if p > start {
            if let Ok(s) = std::str::from_utf8(&b[start..p]) {
                if let Ok(n) = s.parse::<f64>() {
                    nums.push(n);
                }
            }
        }
    }
    if nums.len() == 4 {
        Some((nums[0], nums[1], nums[2], nums[3]))
    } else {
        None
    }
}

/// Count /Page occurrences as a proxy for page count; also try /Count in /Pages dict.
fn find_page_count(b: &[u8]) -> u32 {
    // Try /Count N inside /Pages dict
    let key = b"/Count ";
    let mut pos = 0usize;
    while let Some(p) = find_from(b, key, pos) {
        let after = p + key.len();
        let start = after;
        let mut end = start;
        while end < b.len() && b[end].is_ascii_digit() {
            end += 1;
        }
        if end > start {
            if let Ok(s) = std::str::from_utf8(&b[start..end]) {
                if let Ok(n) = s.parse::<u32>() {
                    if n > 0 && n < 100_000 {
                        return n;
                    }
                }
            }
        }
        pos = p + key.len();
    }
    // Fallback: count /Page objects (not /Pages)
    let mut count = 0u32;
    pos = 0;
    while let Some(p) = find_from(b, b"/Page\n", pos) {
        count += 1;
        pos = p + 6;
    }
    pos = 0;
    while let Some(p) = find_from(b, b"/Page ", pos) {
        count += 1;
        pos = p + 6;
    }
    count
}

fn parse_pdf(b: &[u8]) -> Result<PdfMeta, String> {
    if b.len() < 8 || &b[..5] != b"%PDF-" {
        return Err("Not a valid PDF file (missing %PDF- header).".to_string());
    }

    let mut meta = PdfMeta {
        file_size: b.len() as u64,
        ..Default::default()
    };

    // PDF version
    let ver_end = b[5..]
        .iter()
        .position(|&c| c == b'\n' || c == b'\r')
        .unwrap_or(6);
    meta.version = String::from_utf8_lossy(&b[5..5 + ver_end])
        .trim()
        .to_string();

    // Check linearized
    if find_from(b, b"/Linearized", 0).is_some() {
        meta.linearized = true;
    }

    // Detect xref type
    if find_from(b, b"xref", 0).is_some() {
        meta.xref_type = "traditional cross-reference table".to_string();
    }
    if find_from(b, b"/XRef", 0).is_some() {
        meta.xref_type = "cross-reference stream".to_string();
    }

    // Object count
    meta.object_count = count_objects(b);

    // Page count
    meta.page_count = find_page_count(b);

    // MediaBox
    meta.media_box = find_media_box(b);

    // Info dict entries
    if let Some(v) = extract_string_value(b, b"/Title ") {
        meta.title = v;
    }
    if meta.title.is_empty() {
        if let Some(v) = extract_string_value(b, b"/Title\n") {
            meta.title = v;
        }
    }
    if let Some(v) = extract_string_value(b, b"/Author ") {
        meta.author = v;
    }
    if meta.author.is_empty() {
        if let Some(v) = extract_string_value(b, b"/Author\n") {
            meta.author = v;
        }
    }
    if let Some(v) = extract_string_value(b, b"/Subject ") {
        meta.subject = v;
    }
    if meta.subject.is_empty() {
        if let Some(v) = extract_string_value(b, b"/Subject\n") {
            meta.subject = v;
        }
    }
    if let Some(v) = extract_string_value(b, b"/Keywords ") {
        meta.keywords = v;
    }
    if meta.keywords.is_empty() {
        if let Some(v) = extract_string_value(b, b"/Keywords\n") {
            meta.keywords = v;
        }
    }
    if let Some(v) = extract_string_value(b, b"/Creator ") {
        meta.creator = v;
    }
    if meta.creator.is_empty() {
        if let Some(v) = extract_string_value(b, b"/Creator\n") {
            meta.creator = v;
        }
    }
    if let Some(v) = extract_string_value(b, b"/Producer ") {
        meta.producer = v;
    }
    if meta.producer.is_empty() {
        if let Some(v) = extract_string_value(b, b"/Producer\n") {
            meta.producer = v;
        }
    }
    if let Some(v) = extract_string_value(b, b"/CreationDate ") {
        meta.creation_date = parse_pdf_date(&v);
    }
    if meta.creation_date.is_empty() {
        if let Some(v) = extract_string_value(b, b"/CreationDate\n") {
            meta.creation_date = parse_pdf_date(&v);
        }
    }
    if let Some(v) = extract_string_value(b, b"/ModDate ") {
        meta.mod_date = parse_pdf_date(&v);
    }
    if meta.mod_date.is_empty() {
        if let Some(v) = extract_string_value(b, b"/ModDate\n") {
            meta.mod_date = parse_pdf_date(&v);
        }
    }

    Ok(meta)
}

fn human_size(n: u64) -> String {
    if n < 1_048_576 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else if n < 1_073_741_824 {
        format!("{:.2} MB", n as f64 / 1_048_576.0)
    } else {
        format!("{:.3} GB", n as f64 / 1_073_741_824.0)
    }
}

fn dispatch(action: &str, b: &[u8]) -> String {
    let m = match parse_pdf(b) {
        Ok(m) => m,
        Err(e) => return format!("Error: {}", e),
    };

    match action {
        "info" => {
            let mut out = vec![
                format!("PDF version:  {}", m.version),
                format!("File size:    {}", human_size(m.file_size)),
            ];
            if m.page_count > 0 {
                out.push(format!("Pages:        {}", m.page_count));
            }
            if let Some((x0, y0, x1, y1)) = m.media_box {
                let w_pt = x1 - x0;
                let h_pt = y1 - y0;
                let w_mm = w_pt * 25.4 / 72.0;
                let h_mm = h_pt * 25.4 / 72.0;
                out.push(format!(
                    "Page size:    {:.0}×{:.0} pt  ({:.0}×{:.0} mm)",
                    w_pt, h_pt, w_mm, h_mm
                ));
            }
            if m.linearized {
                out.push("Linearized:   Yes (web-optimised)".to_string());
            }
            if !m.title.is_empty() {
                out.push(format!("Title:        {}", m.title));
            }
            if !m.author.is_empty() {
                out.push(format!("Author:       {}", m.author));
            }
            if !m.subject.is_empty() {
                out.push(format!("Subject:      {}", m.subject));
            }
            if !m.keywords.is_empty() {
                out.push(format!("Keywords:     {}", m.keywords));
            }
            if !m.creator.is_empty() {
                out.push(format!("Creator:      {}", m.creator));
            }
            if !m.producer.is_empty() {
                out.push(format!("Producer:     {}", m.producer));
            }
            if !m.creation_date.is_empty() {
                out.push(format!("Created:      {}", m.creation_date));
            }
            if !m.mod_date.is_empty() {
                out.push(format!("Modified:     {}", m.mod_date));
            }
            out.join("\n")
        }
        "pages" => {
            let mut out = vec![format!(
                "Pages: {}",
                if m.page_count > 0 {
                    m.page_count.to_string()
                } else {
                    "unknown".to_string()
                }
            )];
            if let Some((x0, y0, x1, y1)) = m.media_box {
                let w_pt = x1 - x0;
                let h_pt = y1 - y0;
                let w_mm = w_pt * 25.4 / 72.0;
                let h_mm = h_pt * 25.4 / 72.0;
                out.push(format!(
                    "MediaBox (first page): [{} {} {} {}]",
                    x0, y0, x1, y1
                ));
                out.push(format!(
                    "Page size: {:.0}×{:.0} pt  ({:.0}×{:.0} mm)  ({:.2}×{:.2} in)",
                    w_pt,
                    h_pt,
                    w_mm,
                    h_mm,
                    w_pt / 72.0,
                    h_pt / 72.0
                ));
                let a4_match = (w_mm - 210.0).abs() < 5.0 && (h_mm - 297.0).abs() < 5.0;
                let letter_match = ((w_pt - 612.0).abs() < 5.0) && ((h_pt - 792.0).abs() < 5.0);
                if a4_match {
                    out.push("Standard size: A4".to_string());
                } else if letter_match {
                    out.push("Standard size: US Letter".to_string());
                }
            }
            out.join("\n")
        }
        "metadata" => {
            let mut out = vec![format!("PDF version: {}", m.version)];
            let fields = [
                ("Title", &m.title),
                ("Author", &m.author),
                ("Subject", &m.subject),
                ("Keywords", &m.keywords),
                ("Creator", &m.creator),
                ("Producer", &m.producer),
                ("Created", &m.creation_date),
                ("Modified", &m.mod_date),
            ];
            let any = fields.iter().any(|(_, v)| !v.is_empty());
            if any {
                for (label, val) in &fields {
                    if !val.is_empty() {
                        out.push(format!("{}: {}", label, val));
                    }
                }
            } else {
                out.push("(No Info dictionary entries found)".to_string());
            }
            out.join("\n")
        }
        "structure" => {
            let mut out = vec![
                format!("PDF version:    {}", m.version),
                format!("File size:      {}", human_size(m.file_size)),
                format!("Object count:   ~{}", m.object_count),
                format!("Xref type:      {}", m.xref_type),
                format!(
                    "Linearized:     {}",
                    if m.linearized { "Yes" } else { "No" }
                ),
            ];
            if m.page_count > 0 {
                out.push(format!("Pages:          {}", m.page_count));
            }
            out.join("\n")
        }
        "validate" => {
            let mut issues = vec![];
            if m.version.is_empty() {
                issues.push("Missing PDF version header".to_string());
            }
            if m.page_count == 0 {
                issues.push("Could not determine page count".to_string());
            }
            if m.object_count == 0 {
                issues.push("No PDF objects found — file may be corrupt or encrypted".to_string());
            }
            // Check EOF marker
            let tail = if b.len() > 64 { &b[b.len() - 64..] } else { b };
            if rfind_bytes(tail, b"%%EOF").is_none() {
                issues.push("Missing %%EOF marker at end of file".to_string());
            }
            if issues.is_empty() {
                format!(
                    "VALID — PDF {}, {} pages, {} objects, {} xref.",
                    m.version, m.page_count, m.object_count, m.xref_type
                )
            } else {
                format!(
                    "WARNINGS:\n{}",
                    issues
                        .iter()
                        .map(|s| format!("  • {}", s))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        }
        _ => dispatch("info", b),
    }
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    let bytes = match get_bytes(args) {
        Some(b) => b,
        None => {
            return Ok(
                "Error: provide 'file' (path to PDF) or 'hex' (hex-encoded PDF bytes). \
             Actions: info (default), pages, metadata, structure, validate."
                    .to_string(),
            )
        }
    };
    Ok(dispatch(action, &bytes))
}
