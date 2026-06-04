use serde_json::Value;
use std::collections::HashMap;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "epub_tools",
        "description": "Parse and inspect EPUB 2/3 ebook files without external utilities. Reads OPF package metadata, spine order, and NCX/nav table of contents.",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "metadata", "toc", "spine", "validate"],
                    "description": "info (default): summary with title/author/language/chapter count. metadata: full OPF metadata fields. toc: table of contents from NCX or nav.xhtml. spine: reading order of content documents. validate: check required EPUB structure."
                },
                "file": {
                    "type": "string",
                    "description": "Path to an .epub file."
                },
                "hex": {
                    "type": "string",
                    "description": "Hex-encoded EPUB bytes (ZIP format)."
                }
            }
        }
    })
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn ru16le(b: &[u8], o: usize) -> u16 {
    if o + 1 >= b.len() {
        return 0;
    }
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn ru32le(b: &[u8], o: usize) -> u32 {
    if o + 3 >= b.len() {
        return 0;
    }
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

struct ZipEntry {
    name: String,
    offset: usize,
    compressed_size: usize,
    uncompressed_size: usize,
    compression: u16,
}

fn parse_zip_central_dir(data: &[u8]) -> Vec<ZipEntry> {
    let mut entries = Vec::new();
    // find end of central directory record
    let n = data.len();
    if n < 22 {
        return entries;
    }
    let mut eocd = None;
    let search_start = if n > 65557 { n - 65557 } else { 0 };
    for i in (search_start..=n.saturating_sub(22)).rev() {
        if data[i..].starts_with(b"PK\x05\x06") {
            eocd = Some(i);
            break;
        }
    }
    let eocd = match eocd {
        Some(e) => e,
        None => return entries,
    };
    let cd_size = ru32le(data, eocd + 12) as usize;
    let cd_offset = ru32le(data, eocd + 16) as usize;
    if cd_offset + cd_size > n {
        return entries;
    }
    let mut pos = cd_offset;
    while pos + 46 <= cd_offset + cd_size {
        if !data[pos..].starts_with(b"PK\x01\x02") {
            break;
        }
        let compression = ru16le(data, pos + 10);
        let compressed_size = ru32le(data, pos + 20) as usize;
        let uncompressed_size = ru32le(data, pos + 24) as usize;
        let name_len = ru16le(data, pos + 28) as usize;
        let extra_len = ru16le(data, pos + 30) as usize;
        let comment_len = ru16le(data, pos + 32) as usize;
        let local_offset = ru32le(data, pos + 42) as usize;
        if pos + 46 + name_len > n {
            break;
        }
        let name = String::from_utf8_lossy(&data[pos + 46..pos + 46 + name_len]).to_string();
        entries.push(ZipEntry {
            name,
            offset: local_offset,
            compressed_size,
            uncompressed_size,
            compression,
        });
        pos += 46 + name_len + extra_len + comment_len;
    }
    entries
}

fn extract_entry<'a>(data: &'a [u8], entry: &ZipEntry) -> Option<&'a [u8]> {
    let off = entry.offset;
    if off + 30 > data.len() {
        return None;
    }
    if !data[off..].starts_with(b"PK\x03\x04") {
        return None;
    }
    let name_len = ru16le(data, off + 26) as usize;
    let extra_len = ru16le(data, off + 28) as usize;
    let data_start = off + 30 + name_len + extra_len;
    if entry.compression != 0 {
        // compressed — we can't inflate without a library; return None
        return None;
    }
    if data_start + entry.uncompressed_size > data.len() {
        return None;
    }
    Some(&data[data_start..data_start + entry.uncompressed_size])
}

fn extract_entry_text(data: &[u8], entry: &ZipEntry) -> Option<String> {
    let bytes = extract_entry(data, entry)?;
    Some(String::from_utf8_lossy(bytes).to_string())
}

fn xml_attr<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
    let key = format!("{}=\"", attr);
    let start = tag.find(key.as_str())? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

fn tag_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start_tag = xml.find(open.as_str())?;
    let body_start = xml[start_tag..].find('>')? + start_tag + 1;
    let end = xml[body_start..].find(close.as_str())? + body_start;
    let content = xml[body_start..end].trim().to_string();
    if content.is_empty() {
        None
    } else {
        Some(content)
    }
}

fn all_tag_texts(xml: &str, tag: &str) -> Vec<String> {
    let mut results = Vec::new();
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut pos = 0;
    while let Some(rel) = xml[pos..].find(open.as_str()) {
        let start_tag = pos + rel;
        match xml[start_tag..].find('>') {
            Some(rel2) => {
                let body_start = start_tag + rel2 + 1;
                match xml[body_start..].find(close.as_str()) {
                    Some(rel3) => {
                        let text = xml[body_start..body_start + rel3].trim().to_string();
                        if !text.is_empty() {
                            results.push(text);
                        }
                        pos = body_start + rel3 + close.len();
                    }
                    None => break,
                }
            }
            None => break,
        }
    }
    results
}

fn all_tag_occurrences<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let mut results = Vec::new();
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut pos = 0;
    while let Some(rel) = xml[pos..].find(open.as_str()) {
        let start = pos + rel;
        match xml[start..].find(close.as_str()) {
            Some(rel_end) => {
                results.push(&xml[start..start + rel_end + close.len()]);
                pos = start + rel_end + close.len();
            }
            None => match xml[start..].find("/>") {
                Some(rel_self) => {
                    results.push(&xml[start..start + rel_self + 2]);
                    pos = start + rel_self + 2;
                }
                None => break,
            },
        }
    }
    results
}

struct EpubMeta {
    title: Option<String>,
    authors: Vec<String>,
    publisher: Option<String>,
    language: Option<String>,
    identifier: Option<String>,
    subject: Option<String>,
    description: Option<String>,
    date: Option<String>,
    rights: Option<String>,
    cover_present: bool,
    epub_version: String,
    spine_items: Vec<String>,
    toc_items: Vec<String>,
    manifest_items: usize,
}

fn parse_opf(opf: &str) -> EpubMeta {
    let mut meta = EpubMeta {
        title: tag_text(opf, "dc:title"),
        authors: all_tag_texts(opf, "dc:creator"),
        publisher: tag_text(opf, "dc:publisher"),
        language: tag_text(opf, "dc:language"),
        identifier: tag_text(opf, "dc:identifier"),
        subject: tag_text(opf, "dc:subject"),
        description: tag_text(opf, "dc:description"),
        date: tag_text(opf, "dc:date"),
        rights: tag_text(opf, "dc:rights"),
        cover_present: opf.contains("cover-image") || opf.contains("cover.jpg") || opf.contains("cover.png"),
        epub_version: String::from("2.0"),
        spine_items: Vec::new(),
        toc_items: Vec::new(),
        manifest_items: 0,
    };

    // detect EPUB version from <package> tag
    if let Some(pkg_start) = opf.find("<package") {
        if let Some(end) = opf[pkg_start..].find('>') {
            let pkg_tag = &opf[pkg_start..pkg_start + end + 1];
            if let Some(ver) = xml_attr(pkg_tag, "version") {
                meta.epub_version = ver.to_string();
            }
        }
    }

    // parse spine idref list
    if let Some(spine_start) = opf.find("<spine") {
        let after_spine = &opf[spine_start..];
        for itemref in all_tag_occurrences(after_spine, "itemref") {
            if let Some(idref) = xml_attr(itemref, "idref") {
                meta.spine_items.push(idref.to_string());
            }
        }
    }

    // count manifest items
    if let Some(mf_start) = opf.find("<manifest") {
        let after_mf = &opf[mf_start..];
        if let Some(mf_end) = after_mf.find("</manifest>") {
            meta.manifest_items = after_mf[..mf_end].matches("<item ").count()
                + after_mf[..mf_end].matches("<item\t").count();
        }
    }

    meta
}

fn parse_ncx(ncx: &str) -> Vec<String> {
    let mut items = Vec::new();
    for nav_point in all_tag_occurrences(ncx, "navPoint") {
        if let Some(label) = tag_text(nav_point, "text") {
            items.push(label);
        }
    }
    items
}

fn parse_nav_xhtml(nav: &str) -> Vec<String> {
    let mut items = Vec::new();
    // look for <nav epub:type="toc"> or just all <li> children of <nav>
    let text = nav;
    for a_tag in all_tag_occurrences(text, "a") {
        // strip tags from anchor text
        let inner = if let Some(gt) = a_tag.find('>') {
            &a_tag[gt + 1..]
        } else {
            continue;
        };
        let stripped: String = inner
            .chars()
            .scan(0i32, |depth, c| {
                if c == '<' {
                    *depth += 1;
                    Some(None)
                } else if c == '>' {
                    *depth -= 1;
                    Some(None)
                } else if *depth == 0 {
                    Some(Some(c))
                } else {
                    Some(None)
                }
            })
            .flatten()
            .collect();
        let label = stripped.trim().to_string();
        if !label.is_empty() {
            items.push(label);
        }
    }
    items
}

fn resolve_opf_path(container_xml: &str, zip_entries: &[ZipEntry]) -> Option<String> {
    // container.xml: <rootfile full-path="..." media-type="application/oebps-package+xml"/>
    for rootfile in all_tag_occurrences(container_xml, "rootfile") {
        if let Some(path) = xml_attr(rootfile, "full-path") {
            return Some(path.to_string());
        }
    }
    // fallback: find any .opf file in the zip
    for e in zip_entries {
        if e.name.ends_with(".opf") {
            return Some(e.name.clone());
        }
    }
    None
}

fn find_entry<'a>(entries: &'a [ZipEntry], name: &str) -> Option<&'a ZipEntry> {
    entries.iter().find(|e| e.name == name)
}

fn opf_dir(opf_path: &str) -> &str {
    if let Some(pos) = opf_path.rfind('/') {
        &opf_path[..pos + 1]
    } else {
        ""
    }
}

fn resolve_toc(
    opf: &str,
    opf_path: &str,
    entries: &[ZipEntry],
    data: &[u8],
) -> Vec<String> {
    let prefix = opf_dir(opf_path);

    // EPUB 3: look for <item properties="nav">
    if let Some(mf_start) = opf.find("<manifest") {
        let after_mf = &opf[mf_start..];
        if let Some(mf_end) = after_mf.find("</manifest>") {
            let manifest_block = &after_mf[..mf_end];
            for item in all_tag_occurrences(manifest_block, "item") {
                let props = xml_attr(item, "properties").unwrap_or("");
                if props.contains("nav") {
                    if let Some(href) = xml_attr(item, "href") {
                        let full = format!("{}{}", prefix, href);
                        if let Some(e) = find_entry(entries, &full) {
                            if let Some(nav_text) = extract_entry_text(data, e) {
                                return parse_nav_xhtml(&nav_text);
                            }
                        }
                    }
                }
            }
        }
    }

    // EPUB 2: look for toc= attribute on <spine>
    if let Some(spine_start) = opf.find("<spine") {
        if let Some(end) = opf[spine_start..].find('>') {
            let spine_tag = &opf[spine_start..spine_start + end + 1];
            if let Some(toc_id) = xml_attr(spine_tag, "toc") {
                // find item with this id in manifest
                if let Some(mf_start) = opf.find("<manifest") {
                    let after = &opf[mf_start..];
                    if let Some(mf_end) = after.find("</manifest>") {
                        let block = &after[..mf_end];
                        for item in all_tag_occurrences(block, "item") {
                            let id = xml_attr(item, "id").unwrap_or("");
                            if id == toc_id {
                                if let Some(href) = xml_attr(item, "href") {
                                    let full = format!("{}{}", prefix, href);
                                    if let Some(e) = find_entry(entries, &full) {
                                        if let Some(ncx) = extract_entry_text(data, e) {
                                            return parse_ncx(&ncx);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // fallback: look for toc.ncx anywhere
    for e in entries {
        if e.name.ends_with("toc.ncx") {
            if let Some(ncx) = extract_entry_text(data, e) {
                return parse_ncx(&ncx);
            }
        }
    }

    Vec::new()
}

fn manifest_href_map(opf: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(mf_start) = opf.find("<manifest") {
        let after = &opf[mf_start..];
        if let Some(mf_end) = after.find("</manifest>") {
            for item in all_tag_occurrences(&after[..mf_end], "item") {
                if let (Some(id), Some(href)) = (xml_attr(item, "id"), xml_attr(item, "href")) {
                    map.insert(id.to_string(), href.to_string());
                }
            }
        }
    }
    map
}

fn dispatch(action: &str, data: &[u8]) -> String {
    if !data.starts_with(b"PK\x03\x04") && !data.starts_with(b"PK\x05\x06") {
        return "Error: not a ZIP/EPUB file (missing PK signature)".to_string();
    }

    let entries = parse_zip_central_dir(data);
    if entries.is_empty() {
        return "Error: could not parse ZIP central directory".to_string();
    }

    // verify mimetype entry
    let has_mimetype = entries.iter().any(|e| e.name == "mimetype");
    let has_container = entries
        .iter()
        .any(|e| e.name == "META-INF/container.xml");

    // read container.xml
    let container_text = entries
        .iter()
        .find(|e| e.name == "META-INF/container.xml")
        .and_then(|e| extract_entry_text(data, e))
        .unwrap_or_default();

    let opf_path = match resolve_opf_path(&container_text, &entries) {
        Some(p) => p,
        None => return "Error: cannot find OPF package file in EPUB".to_string(),
    };

    let opf_text = match find_entry(&entries, &opf_path)
        .and_then(|e| extract_entry_text(data, e))
    {
        Some(t) => t,
        None => return format!("Error: OPF file '{}' is compressed or missing", opf_path),
    };

    let mut meta = parse_opf(&opf_text);
    let prefix = opf_dir(&opf_path);
    meta.toc_items = resolve_toc(&opf_text, &opf_path, &entries, data);

    match action {
        "info" => {
            let mut out = String::new();
            out.push_str("EPUB INFO\n");
            out.push_str(&format!("  EPUB Version  : {}\n", meta.epub_version));
            if let Some(t) = &meta.title {
                out.push_str(&format!("  Title         : {}\n", t));
            }
            if !meta.authors.is_empty() {
                out.push_str(&format!("  Author(s)     : {}\n", meta.authors.join("; ")));
            }
            if let Some(p) = &meta.publisher {
                out.push_str(&format!("  Publisher     : {}\n", p));
            }
            if let Some(l) = &meta.language {
                out.push_str(&format!("  Language      : {}\n", l));
            }
            if let Some(id) = &meta.identifier {
                out.push_str(&format!("  Identifier    : {}\n", id));
            }
            if let Some(d) = &meta.date {
                out.push_str(&format!("  Date          : {}\n", d));
            }
            out.push_str(&format!(
                "  Cover Present : {}\n",
                if meta.cover_present { "Yes" } else { "No" }
            ));
            out.push_str(&format!(
                "  Spine Items   : {} chapters/documents\n",
                meta.spine_items.len()
            ));
            out.push_str(&format!(
                "  TOC Entries   : {}\n",
                if meta.toc_items.is_empty() {
                    "none detected".to_string()
                } else {
                    meta.toc_items.len().to_string()
                }
            ));
            out.push_str(&format!(
                "  Manifest Files: {}\n",
                meta.manifest_items
            ));
            out.push_str(&format!("  Total ZIP Entries: {}\n", entries.len()));
            out
        }
        "metadata" => {
            let mut out = String::from("EPUB METADATA (OPF Dublin Core)\n");
            let fields: &[(&str, Option<&String>)] = &[
                ("Title", meta.title.as_ref()),
                ("Language", meta.language.as_ref()),
                ("Identifier", meta.identifier.as_ref()),
                ("Date", meta.date.as_ref()),
                ("Publisher", meta.publisher.as_ref()),
                ("Subject", meta.subject.as_ref()),
                ("Description", meta.description.as_ref()),
                ("Rights", meta.rights.as_ref()),
            ];
            for (label, val) in fields {
                if let Some(v) = val {
                    out.push_str(&format!("  {:12}: {}\n", label, v));
                }
            }
            if !meta.authors.is_empty() {
                for (i, a) in meta.authors.iter().enumerate() {
                    out.push_str(&format!("  Author[{}]    : {}\n", i + 1, a));
                }
            }
            out.push_str(&format!("\nOPF Path: {}\n", opf_path));
            out.push_str(&format!("EPUB Version: {}\n", meta.epub_version));
            out
        }
        "toc" => {
            if meta.toc_items.is_empty() {
                return "No table of contents found (may be compressed or missing)".to_string();
            }
            let mut out = String::from("TABLE OF CONTENTS\n");
            for (i, item) in meta.toc_items.iter().enumerate() {
                out.push_str(&format!("  {:3}. {}\n", i + 1, item));
            }
            out
        }
        "spine" => {
            if meta.spine_items.is_empty() {
                return "No spine items found in OPF".to_string();
            }
            let href_map = manifest_href_map(&opf_text);
            let mut out = String::from("READING ORDER (spine)\n");
            for (i, idref) in meta.spine_items.iter().enumerate() {
                let href = href_map
                    .get(idref)
                    .map(|h| format!("{}{}", prefix, h))
                    .unwrap_or_else(|| format!("({})", idref));
                out.push_str(&format!("  {:3}. {}\n", i + 1, href));
            }
            out
        }
        "validate" => {
            let mut issues: Vec<String> = Vec::new();
            if !has_mimetype {
                issues.push("WARN: missing 'mimetype' entry (required by EPUB spec)".to_string());
            } else {
                // check mimetype content
                if let Some(mt) = find_entry(&entries, "mimetype").and_then(|e| extract_entry_text(data, e)) {
                    if mt.trim() != "application/epub+zip" {
                        issues.push(format!("WARN: mimetype content is '{}' (should be 'application/epub+zip')", mt.trim()));
                    }
                }
            }
            if !has_container {
                issues.push("ERROR: missing META-INF/container.xml".to_string());
            }
            if meta.title.is_none() {
                issues.push("WARN: no dc:title in OPF metadata".to_string());
            }
            if meta.language.is_none() {
                issues.push("WARN: no dc:language in OPF metadata".to_string());
            }
            if meta.identifier.is_none() {
                issues.push("WARN: no dc:identifier (ISBN/UUID) in OPF metadata".to_string());
            }
            if meta.spine_items.is_empty() {
                issues.push("ERROR: empty spine — no reading order defined".to_string());
            }
            if meta.authors.is_empty() {
                issues.push("WARN: no dc:creator (author) in OPF metadata".to_string());
            }

            let verdict = if issues.iter().any(|i| i.starts_with("ERROR")) {
                "INVALID"
            } else if issues.is_empty() {
                "VALID"
            } else {
                "WARNINGS"
            };

            let mut out = format!("EPUB VALIDATION: {}\n\n", verdict);
            out.push_str(&format!("  EPUB Version  : {}\n", meta.epub_version));
            out.push_str(&format!("  mimetype entry: {}\n", if has_mimetype { "present" } else { "MISSING" }));
            out.push_str(&format!("  container.xml : {}\n", if has_container { "present" } else { "MISSING" }));
            out.push_str(&format!("  OPF path      : {}\n", opf_path));
            out.push_str(&format!("  Spine items   : {}\n", meta.spine_items.len()));
            if !issues.is_empty() {
                out.push_str("\nIssues:\n");
                for issue in &issues {
                    out.push_str(&format!("  {}\n", issue));
                }
            }
            out
        }
        _ => format!("Error: unknown action '{}'", action),
    }
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args["action"].as_str().unwrap_or("info");

    let data: Vec<u8> = if let Some(path) = args["file"].as_str() {
        match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => return Ok(format!("Error reading file '{}': {}", path, e)),
        }
    } else if let Some(hex_str) = args["hex"].as_str() {
        let clean: String = hex_str.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        match (0..clean.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&clean[i..i + 2], 16))
            .collect::<Result<Vec<u8>, _>>()
        {
            Ok(b) => b,
            Err(_) => return Ok("Error: invalid hex input".to_string()),
        }
    } else {
        return Ok("Error: provide 'file' (path to .epub) or 'hex' (hex-encoded EPUB bytes)".to_string());
    };

    Ok(dispatch(action, &data))
}
