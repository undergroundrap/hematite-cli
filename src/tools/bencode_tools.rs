use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("decode");
    match action {
        "decode" | "parse" => action_decode(args),
        "info" => action_info(args),
        "files" => action_files(args),
        "trackers" => action_trackers(args),
        other => Err(format!(
            "bencode_tools: unknown action '{other}'. Valid: decode, info, files, trackers"
        )),
    }
}

// ── Input resolution ───────────────────────────────────────────────────────────

fn resolve_bytes(args: &Value) -> Result<Vec<u8>, String> {
    if let Some(hex) = args.get("hex").and_then(|v| v.as_str()) {
        return decode_hex(hex.trim());
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read(path)
            .map_err(|e| format!("bencode_tools: cannot read '{path}': {e}"));
    }
    // Fall back to raw text (ASCII bencode)
    if let Some(text) = args
        .get("text")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
        return Ok(text.as_bytes().to_vec());
    }
    Err("bencode_tools: provide 'hex' (hex-encoded bytes), 'file' (path to .torrent/.bencode), or 'text' (raw bencode string)".into())
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.len() % 2 != 0 {
        return Err("bencode_tools: hex string has odd length".into());
    }
    (0..s.len() / 2)
        .map(|i| {
            u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|_| format!("bencode_tools: invalid hex byte at position {}", i * 2))
        })
        .collect()
}

// ── Bencode value tree ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum BencodeValue {
    Integer(i64),
    Bytes(Vec<u8>),
    List(Vec<BencodeValue>),
    Dict(Vec<(Vec<u8>, BencodeValue)>), // preserve insertion order
}

fn bytes_to_str(b: &[u8]) -> String {
    std::str::from_utf8(b)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| format!("[binary: {} bytes]", b.len()))
}

fn bytes_to_str_truncated(b: &[u8], max: usize) -> String {
    let s = bytes_to_str(b);
    if s.len() > max {
        format!("{}…", &s[..max])
    } else {
        s
    }
}

// ── Recursive descent parser ───────────────────────────────────────────────────

fn parse(data: &[u8], pos: &mut usize) -> Result<BencodeValue, String> {
    if *pos >= data.len() {
        return Err(format!(
            "bencode_tools: unexpected end of data at position {pos}"
        ));
    }
    match data[*pos] {
        b'i' => parse_integer(data, pos),
        b'l' => parse_list(data, pos),
        b'd' => parse_dict(data, pos),
        b'0'..=b'9' => parse_bytes(data, pos),
        other => Err(format!(
            "bencode_tools: unexpected byte 0x{:02x} ('{}') at position {pos}",
            other,
            if other.is_ascii_graphic() {
                other as char
            } else {
                '?'
            }
        )),
    }
}

fn parse_integer(data: &[u8], pos: &mut usize) -> Result<BencodeValue, String> {
    // Format: i<number>e
    *pos += 1; // skip 'i'
    let start = *pos;
    while *pos < data.len() && data[*pos] != b'e' {
        *pos += 1;
    }
    if *pos >= data.len() {
        return Err("bencode_tools: unterminated integer (missing 'e')".into());
    }
    let num_str = std::str::from_utf8(&data[start..*pos])
        .map_err(|_| "bencode_tools: non-UTF-8 integer value".to_string())?;
    let n: i64 = num_str
        .parse()
        .map_err(|_| format!("bencode_tools: invalid integer '{num_str}'"))?;
    *pos += 1; // skip 'e'
    Ok(BencodeValue::Integer(n))
}

fn parse_bytes(data: &[u8], pos: &mut usize) -> Result<BencodeValue, String> {
    // Format: <length>:<data>
    let start = *pos;
    while *pos < data.len() && data[*pos] != b':' {
        *pos += 1;
    }
    if *pos >= data.len() {
        return Err("bencode_tools: missing ':' in byte string".into());
    }
    let len_str = std::str::from_utf8(&data[start..*pos])
        .map_err(|_| "bencode_tools: non-UTF-8 byte string length".to_string())?;
    let len: usize = len_str
        .parse()
        .map_err(|_| format!("bencode_tools: invalid byte string length '{len_str}'"))?;
    *pos += 1; // skip ':'
    if *pos + len > data.len() {
        return Err(format!(
            "bencode_tools: byte string of length {len} exceeds data (pos {pos}, data len {})",
            data.len()
        ));
    }
    let bytes = data[*pos..*pos + len].to_vec();
    *pos += len;
    Ok(BencodeValue::Bytes(bytes))
}

fn parse_list(data: &[u8], pos: &mut usize) -> Result<BencodeValue, String> {
    *pos += 1; // skip 'l'
    let mut items = Vec::new();
    while *pos < data.len() && data[*pos] != b'e' {
        items.push(parse(data, pos)?);
    }
    if *pos >= data.len() {
        return Err("bencode_tools: unterminated list (missing 'e')".into());
    }
    *pos += 1; // skip 'e'
    Ok(BencodeValue::List(items))
}

fn parse_dict(data: &[u8], pos: &mut usize) -> Result<BencodeValue, String> {
    *pos += 1; // skip 'd'
    let mut entries: Vec<(Vec<u8>, BencodeValue)> = Vec::new();
    while *pos < data.len() && data[*pos] != b'e' {
        // Keys must be byte strings
        let key = match parse(data, pos)? {
            BencodeValue::Bytes(b) => b,
            other => {
                return Err(format!(
                    "bencode_tools: dict key must be a byte string, got {:?}",
                    other
                ))
            }
        };
        if *pos >= data.len() {
            return Err("bencode_tools: dict key without value".into());
        }
        let val = parse(data, pos)?;
        entries.push((key, val));
    }
    if *pos >= data.len() {
        return Err("bencode_tools: unterminated dict (missing 'e')".into());
    }
    *pos += 1; // skip 'e'
    Ok(BencodeValue::Dict(entries))
}

// ── Dict helpers ───────────────────────────────────────────────────────────────

fn dict_get<'a>(entries: &'a [(Vec<u8>, BencodeValue)], key: &str) -> Option<&'a BencodeValue> {
    entries
        .iter()
        .find(|(k, _)| k.as_slice() == key.as_bytes())
        .map(|(_, v)| v)
}

fn dict_str(entries: &[(Vec<u8>, BencodeValue)], key: &str) -> Option<String> {
    match dict_get(entries, key)? {
        BencodeValue::Bytes(b) => Some(bytes_to_str(b)),
        _ => None,
    }
}

fn dict_int(entries: &[(Vec<u8>, BencodeValue)], key: &str) -> Option<i64> {
    match dict_get(entries, key)? {
        BencodeValue::Integer(n) => Some(*n),
        _ => None,
    }
}

// ── Human-readable size ────────────────────────────────────────────────────────

fn human_size(bytes: u64) -> String {
    const UNITS: &[(&str, u64)] = &[
        ("PB", 1_000_000_000_000_000),
        ("TB", 1_000_000_000_000),
        ("GB", 1_000_000_000),
        ("MB", 1_000_000),
        ("KB", 1_000),
    ];
    for (label, div) in UNITS {
        if bytes >= *div {
            let f = bytes as f64 / *div as f64;
            return format!("{:.1} {label}", f);
        }
    }
    format!("{bytes} B")
}

// ── Unix timestamp to human date ───────────────────────────────────────────────

fn unix_to_date(ts: i64) -> String {
    // Days since Unix epoch
    if ts < 0 {
        return format!("{ts} (negative timestamp)");
    }
    let secs = ts as u64;
    let days_total = secs / 86400;
    let time_of_day = secs % 86400;
    let hh = time_of_day / 3600;
    let mm = (time_of_day % 3600) / 60;
    let ss = time_of_day % 60;

    // Gregorian calendar calculation
    let (year, month, day) = days_to_date(days_total);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        hh, mm, ss
    )
}

fn days_to_date(days: u64) -> (u64, u64, u64) {
    // Reference: Unix epoch = 1970-01-01
    // Use the proleptic Gregorian calendar algorithm
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// ── Display renderer ───────────────────────────────────────────────────────────

fn render_bencode(val: &BencodeValue, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    match val {
        BencodeValue::Integer(n) => {
            out.push_str(&format!("{n} [integer]\n"));
        }
        BencodeValue::Bytes(b) => {
            let s = bytes_to_str_truncated(b, 200);
            if std::str::from_utf8(b).is_ok() {
                out.push_str(&format!("\"{s}\"\n"));
            } else {
                out.push_str(&format!("[binary: {} bytes]\n", b.len()));
            }
        }
        BencodeValue::List(items) => {
            out.push_str(&format!(
                "[list, {} item{}]\n",
                items.len(),
                if items.len() == 1 { "" } else { "s" }
            ));
            for (i, item) in items.iter().enumerate() {
                out.push_str(&format!("{pad}  [{i}] "));
                render_bencode(item, indent + 2, out);
            }
        }
        BencodeValue::Dict(entries) => {
            out.push_str(&format!(
                "{{dict, {} key{}}}\n",
                entries.len(),
                if entries.len() == 1 { "" } else { "s" }
            ));
            for (k, v) in entries {
                let key = bytes_to_str(k);
                // Skip printing raw 'pieces' blob inline
                if key == "pieces" {
                    if let BencodeValue::Bytes(b) = v {
                        out.push_str(&format!(
                            "{pad}  \"{key}\" = [SHA1 hashes: {} pieces]\n",
                            b.len() / 20
                        ));
                        continue;
                    }
                }
                out.push_str(&format!("{pad}  \"{key}\" = "));
                render_bencode(v, indent + 2, out);
            }
        }
    }
}

// ── action_decode ──────────────────────────────────────────────────────────────

fn action_decode(args: &Value) -> Result<String, String> {
    let data = resolve_bytes(args)?;
    let mut pos = 0usize;
    let root = parse(&data, &mut pos).map_err(|e| e.to_string())?;

    let mut out = format!("Bencode Decode\n{}\n\n", "─".repeat(34));
    render_bencode(&root, 0, &mut out);

    if pos < data.len() {
        out.push_str(&format!(
            "\n(Note: {} trailing bytes after root value)\n",
            data.len() - pos
        ));
    }
    Ok(out)
}

// ── Torrent parsing helpers ────────────────────────────────────────────────────

struct TorrentInfo {
    name: String,
    piece_length: i64,
    piece_count: usize,
    total_length: u64,
    is_multi: bool,
    file_count: usize,
    files: Vec<TorrentFile>,
    announce: String,
    announce_list: Vec<Vec<String>>,
    comment: String,
    created_by: String,
    creation_date: Option<i64>,
    encoding: String,
}

struct TorrentFile {
    path: String,
    length: u64,
}

fn parse_torrent(data: &[u8]) -> Result<TorrentInfo, String> {
    let mut pos = 0usize;
    let root = parse(data, &mut pos)?;
    let root_entries = match &root {
        BencodeValue::Dict(e) => e,
        _ => return Err("bencode_tools: torrent is not a dict at root level".into()),
    };

    let announce = dict_str(root_entries, "announce").unwrap_or_default();
    let comment = dict_str(root_entries, "comment").unwrap_or_else(|| "—".to_string());
    let created_by = dict_str(root_entries, "created by").unwrap_or_default();
    let creation_date = dict_int(root_entries, "creation date");
    let encoding = dict_str(root_entries, "encoding").unwrap_or_default();

    // announce-list: list of list of strings
    let announce_list = match dict_get(root_entries, "announce-list") {
        Some(BencodeValue::List(tiers)) => tiers
            .iter()
            .filter_map(|tier| {
                if let BencodeValue::List(urls) = tier {
                    Some(
                        urls.iter()
                            .filter_map(|u| {
                                if let BencodeValue::Bytes(b) = u {
                                    Some(bytes_to_str(b))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    let info_entries = match dict_get(root_entries, "info") {
        Some(BencodeValue::Dict(e)) => e,
        _ => return Err("bencode_tools: missing or invalid 'info' dict in torrent".into()),
    };

    let name = dict_str(info_entries, "name").unwrap_or_else(|| "(unnamed)".into());
    let piece_length = dict_int(info_entries, "piece length").unwrap_or(0);

    let piece_count = match dict_get(info_entries, "pieces") {
        Some(BencodeValue::Bytes(b)) => b.len() / 20,
        _ => 0,
    };

    // Detect single vs multi file
    let (is_multi, total_length, file_count, files) =
        if let Some(BencodeValue::List(file_list)) = dict_get(info_entries, "files") {
            let mut total: u64 = 0;
            let mut tfiles: Vec<TorrentFile> = Vec::new();
            for f in file_list {
                if let BencodeValue::Dict(fe) = f {
                    let len = dict_int(fe, "length").unwrap_or(0) as u64;
                    total += len;
                    let path_str = match dict_get(fe, "path") {
                        Some(BencodeValue::List(parts)) => parts
                            .iter()
                            .filter_map(|p| {
                                if let BencodeValue::Bytes(b) = p {
                                    Some(bytes_to_str(b))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("/"),
                        _ => "(unknown)".into(),
                    };
                    tfiles.push(TorrentFile {
                        path: path_str,
                        length: len,
                    });
                }
            }
            let fc = tfiles.len();
            (true, total, fc, tfiles)
        } else {
            let len = dict_int(info_entries, "length").unwrap_or(0) as u64;
            let tf = TorrentFile {
                path: name.clone(),
                length: len,
            };
            (false, len, 1, vec![tf])
        };

    Ok(TorrentInfo {
        name,
        piece_length,
        piece_count,
        total_length,
        is_multi,
        file_count,
        files,
        announce,
        announce_list,
        comment,
        created_by,
        creation_date,
        encoding,
    })
}

// ── action_info ────────────────────────────────────────────────────────────────

fn action_info(args: &Value) -> Result<String, String> {
    let data = resolve_bytes(args)?;
    let t = parse_torrent(&data)?;

    let file_desc = if t.is_multi {
        format!("{} files (multi-file torrent)", t.file_count)
    } else {
        "1 file (single-file torrent)".into()
    };

    let size_desc = format!(
        "{} ({} bytes)",
        human_size(t.total_length),
        format_comma(t.total_length)
    );

    let piece_size_desc = if t.piece_length >= 1_000_000 {
        format!("{} MB", t.piece_length / 1_000_000)
    } else if t.piece_length >= 1_024 {
        format!("{} KB", t.piece_length / 1_024)
    } else {
        format!("{} B", t.piece_length)
    };

    let alt_tracker_count = t
        .announce_list
        .iter()
        .flat_map(|tier| tier.iter())
        .filter(|u| !t.announce.is_empty() && u.as_str() != t.announce.as_str())
        .count();

    let alt_desc = if alt_tracker_count > 0 {
        format!("{alt_tracker_count} additional")
    } else {
        "—".into()
    };

    let created_desc = t
        .creation_date
        .map(unix_to_date)
        .unwrap_or_else(|| "—".into());

    let comment_desc = if t.comment.is_empty() || t.comment == "—" {
        "—".into()
    } else {
        t.comment.clone()
    };

    let created_by_desc = if t.created_by.is_empty() {
        "—".into()
    } else {
        t.created_by.clone()
    };

    let mut out = format!("Torrent Info\n{}\n", "─".repeat(34));
    out.push_str(&format!("Name:          {}\n", t.name));
    out.push_str(&format!("Files:         {file_desc}\n"));
    out.push_str(&format!("Total size:    {size_desc}\n"));
    out.push_str(&format!("Piece size:    {piece_size_desc}\n"));
    out.push_str(&format!(
        "Piece count:   {}\n",
        format_comma(t.piece_count as u64)
    ));
    if !t.announce.is_empty() {
        out.push_str(&format!("Tracker:       {}\n", t.announce));
    } else {
        out.push_str("Tracker:       — (DHT/trackerless)\n");
    }
    out.push_str(&format!("Alt trackers:  {alt_desc}\n"));
    out.push_str(&format!("Created by:    {created_by_desc}\n"));
    out.push_str(&format!("Created:       {created_desc}\n"));
    out.push_str(&format!("Comment:       {comment_desc}\n"));
    if !t.encoding.is_empty() {
        out.push_str(&format!("Encoding:      {}\n", t.encoding));
    }

    Ok(out)
}

// ── action_files ───────────────────────────────────────────────────────────────

fn action_files(args: &Value) -> Result<String, String> {
    let data = resolve_bytes(args)?;
    let t = parse_torrent(&data)?;

    let mut out = format!("Files — {}\n{}\n\n", t.name, "─".repeat(34));

    if !t.is_multi {
        out.push_str(&format!(
            "Single-file torrent\n\nFile: {}\nSize: {}\n",
            t.files[0].path,
            human_size(t.files[0].length)
        ));
        return Ok(out);
    }

    // Multi-file: tabular view with cumulative offset
    let max_path = t
        .files
        .iter()
        .map(|f| f.path.len())
        .max()
        .unwrap_or(4)
        .max(4);
    out.push_str(&format!(
        "{:<width$}  {:>12}  {:>14}\n",
        "Path",
        "Size",
        "Cumulative",
        width = max_path
    ));
    out.push_str(&format!("{}\n", "─".repeat(max_path + 2 + 12 + 2 + 14)));

    let mut cumulative: u64 = 0;
    for f in &t.files {
        cumulative += f.length;
        out.push_str(&format!(
            "{:<width$}  {:>12}  {:>14}\n",
            f.path,
            human_size(f.length),
            human_size(cumulative),
            width = max_path
        ));
    }

    out.push_str(&format!(
        "\n{} files  —  Total: {}\n",
        t.file_count,
        human_size(t.total_length)
    ));
    Ok(out)
}

// ── action_trackers ────────────────────────────────────────────────────────────

fn action_trackers(args: &Value) -> Result<String, String> {
    let data = resolve_bytes(args)?;
    let t = parse_torrent(&data)?;

    let mut out = format!("Trackers — {}\n{}\n\n", t.name, "─".repeat(34));

    if t.announce.is_empty() && t.announce_list.is_empty() {
        out.push_str("No trackers found — this is a DHT/trackerless torrent\n");
        return Ok(out);
    }

    // Collect all unique tracker URLs across tiers
    let mut all_urls: Vec<(usize, String, bool)> = Vec::new(); // (tier, url, is_primary)

    // Primary tracker (tier 0 if no announce-list)
    if !t.announce.is_empty() {
        all_urls.push((0, t.announce.clone(), true));
    }

    // announce-list tiers
    if !t.announce_list.is_empty() {
        for (tier_idx, tier) in t.announce_list.iter().enumerate() {
            for url in tier {
                let is_primary = url == &t.announce;
                // Avoid duplicating the primary already added above
                if is_primary && !t.announce.is_empty() && tier_idx == 0 {
                    // already added above; update or skip
                    if let Some(existing) = all_urls.iter_mut().find(|(_, u, _)| u == url) {
                        existing.0 = tier_idx;
                        continue;
                    }
                }
                if !all_urls.iter().any(|(_, u, _)| u == url) {
                    all_urls.push((tier_idx, url.clone(), is_primary));
                }
            }
        }
    }

    // Unique domains
    let unique_domains: std::collections::HashSet<String> = all_urls
        .iter()
        .filter_map(|(_, url, _)| {
            // Very simple domain extraction
            let after_scheme = url
                .find("://")
                .map(|i| &url[i + 3..])
                .unwrap_or(url.as_str());
            let domain = after_scheme.split('/').next().unwrap_or(after_scheme);
            // Strip port
            let domain = domain.split(':').next().unwrap_or(domain);
            if domain.is_empty() {
                None
            } else {
                Some(domain.to_lowercase())
            }
        })
        .collect();

    out.push_str(&format!(
        "{} tracker{}, {} unique domain{}\n\n",
        all_urls.len(),
        if all_urls.len() == 1 { "" } else { "s" },
        unique_domains.len(),
        if unique_domains.len() == 1 { "" } else { "s" }
    ));

    for (tier, url, is_primary) in &all_urls {
        let scheme = if url.starts_with("udp://") {
            "UDP"
        } else if url.starts_with("https://") {
            "HTTPS"
        } else if url.starts_with("http://") {
            "HTTP"
        } else {
            "?"
        };
        let primary_marker = if *is_primary { " [primary]" } else { "" };
        out.push_str(&format!(
            "  Tier {tier}  {scheme:5}  {url}{primary_marker}\n"
        ));
    }

    Ok(out)
}

// ── Formatting helpers ─────────────────────────────────────────────────────────

fn format_comma(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let start = s.len() % 3;
    if start > 0 {
        result.push_str(&s[..start]);
    }
    let mut i = start;
    while i < s.len() {
        if !result.is_empty() {
            result.push(',');
        }
        result.push_str(&s[i..i + 3]);
        i += 3;
    }
    result
}
