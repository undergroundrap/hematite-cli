use serde_json::{json, Value};
use std::fmt::Write as FmtWrite;
use std::io::Read;

pub fn make_schema() -> Value {
    json!({
        "name": "office_tools",
        "description": "Inspect DOCX, XLSX, and PPTX Office Open XML documents without Microsoft Office. \
Actions: info (default — format, metadata, stats), content (extract text/sheet names/slide previews), \
structure (list ZIP parts), validate (check required Open XML parts). \
Pass file (path to .docx, .xlsx, or .pptx). \
Example: office_tools(file: 'report.docx') or office_tools(action: 'content', file: 'data.xlsx') or office_tools(action: 'structure', file: 'deck.pptx').",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "info|content|structure|validate" },
                "file": { "type": "string", "description": "Path to .docx, .xlsx, or .pptx file" }
            },
            "required": ["file"]
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("info");

    let path = match args.get("file").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return Ok(
                "Error: provide 'file' path to a DOCX, XLSX, or PPTX file.".to_string(),
            )
        }
    };

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return Ok(format!("Error opening file: {}", e)),
    };

    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return Ok("Error: file is not a valid ZIP/Office document.".to_string()),
    };

    Ok(match action {
        "content" => extract_content(&mut archive),
        "structure" => list_structure(&mut archive),
        "validate" => validate_doc(&mut archive),
        _ => extract_info(&mut archive),
    })
}

fn read_zip_entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let mut f = archive.by_name(name).ok()?;
    let mut s = String::new();
    f.read_to_string(&mut s).ok()?;
    Some(s)
}

fn detect_format(archive: &mut zip::ZipArchive<std::fs::File>) -> &'static str {
    if let Some(ct) = read_zip_entry(archive, "[Content_Types].xml") {
        if ct.contains("wordprocessingml.document.main") {
            return "docx";
        }
        if ct.contains("spreadsheetml.sheet.main") {
            return "xlsx";
        }
        if ct.contains("presentationml.presentation.main") {
            return "pptx";
        }
    }
    // Fallback: check file presence
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    if names.iter().any(|n| n == "word/document.xml") {
        return "docx";
    }
    if names.iter().any(|n| n == "xl/workbook.xml") {
        return "xlsx";
    }
    if names.iter().any(|n| n == "ppt/presentation.xml") {
        return "pptx";
    }
    "unknown"
}

// Naive extraction of text between XML tags — no full parser needed for simple metadata
fn xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}", tag);
    let start_tag = xml.find(&open)?;
    let content_start = xml[start_tag..].find('>')? + start_tag + 1;
    let close_pos = xml[content_start..].find(&close)? + content_start;
    let s = xml[content_start..close_pos].trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

// Strip all XML tags from a string
fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    // Collapse whitespace
    out.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn count_tag(xml: &str, tag: &str) -> usize {
    let open = format!("<{}", tag);
    let mut count = 0;
    let mut pos = 0;
    while let Some(found) = xml[pos..].find(&open) {
        count += 1;
        pos += found + open.len();
    }
    count
}

struct CoreMeta {
    title: Option<String>,
    creator: Option<String>,
    last_modified_by: Option<String>,
    created: Option<String>,
    modified: Option<String>,
    description: Option<String>,
    keywords: Option<String>,
}

fn read_core_meta(archive: &mut zip::ZipArchive<std::fs::File>) -> CoreMeta {
    let xml = read_zip_entry(archive, "docProps/core.xml").unwrap_or_default();
    // Clean namespace prefixes for simple matching
    let xml = xml
        .replace("dc:title", "dc_title")
        .replace("dc:creator", "dc_creator")
        .replace("dc:description", "dc_description")
        .replace("dc:subject", "dc_subject")
        .replace("cp:lastModifiedBy", "cp_lastModifiedBy")
        .replace("dcterms:created", "dcterms_created")
        .replace("dcterms:modified", "dcterms_modified")
        .replace("cp:keywords", "cp_keywords");

    CoreMeta {
        title: xml_text(&xml, "dc_title"),
        creator: xml_text(&xml, "dc_creator"),
        last_modified_by: xml_text(&xml, "cp_lastModifiedBy"),
        created: xml_text(&xml, "dcterms_created"),
        modified: xml_text(&xml, "dcterms_modified"),
        description: xml_text(&xml, "dc_description"),
        keywords: xml_text(&xml, "cp_keywords"),
    }
}

fn extract_info(archive: &mut zip::ZipArchive<std::fs::File>) -> String {
    let format = detect_format(archive);
    let meta = read_core_meta(archive);

    let mut out = String::new();
    let ext = format.to_uppercase();
    let _ = writeln!(out, "Office Document  ({})\n", ext);

    if let Some(v) = &meta.title {
        let _ = writeln!(out, "  Title:            {}", v);
    }
    if let Some(v) = &meta.creator {
        let _ = writeln!(out, "  Author:           {}", v);
    }
    if let Some(v) = &meta.last_modified_by {
        let _ = writeln!(out, "  Last Modified By: {}", v);
    }
    if let Some(v) = &meta.created {
        let _ = writeln!(out, "  Created:          {}", v);
    }
    if let Some(v) = &meta.modified {
        let _ = writeln!(out, "  Modified:         {}", v);
    }
    if let Some(v) = &meta.description {
        let _ = writeln!(out, "  Description:      {}", v);
    }
    if let Some(v) = &meta.keywords {
        let _ = writeln!(out, "  Keywords:         {}", v);
    }

    // Format-specific stats
    out.push('\n');
    match format {
        "docx" => {
            if let Some(doc) = read_zip_entry(archive, "word/document.xml") {
                let paragraphs = count_tag(&doc, "w:p ");
                let paragraphs = paragraphs + count_tag(&doc, "w:p>");
                let words: usize = doc
                    .split_whitespace()
                    .filter(|s| !s.starts_with('<'))
                    .count();
                // Rough word count from text nodes
                let text_content = strip_tags(&doc);
                let word_count = text_content.split_whitespace().count();
                let _ = writeln!(out, "  Paragraphs (approx): {}", paragraphs);
                let _ = writeln!(out, "  Words (approx):      {}", word_count);
                let _ = writeln!(out, "  Raw XML nodes:       ~{}", words);

                // Count tables, images
                let tables = count_tag(&doc, "w:tbl");
                let images = count_tag(&doc, "a:blip");
                if tables > 0 {
                    let _ = writeln!(out, "  Tables:              {}", tables);
                }
                if images > 0 {
                    let _ = writeln!(out, "  Embedded images:     {}", images);
                }
            }
        }
        "xlsx" => {
            if let Some(wb) = read_zip_entry(archive, "xl/workbook.xml") {
                let sheet_names = extract_sheet_names(&wb);
                let _ = writeln!(out, "  Sheets ({}):", sheet_names.len());
                for (i, name) in sheet_names.iter().enumerate() {
                    let _ = writeln!(out, "    {}. {}", i + 1, name);
                }
                // Count defined names
                let defined = count_tag(&wb, "definedName");
                if defined > 0 {
                    let _ = writeln!(out, "  Named ranges: {}", defined);
                }
            }
        }
        "pptx" => {
            let slide_count = count_slide_files(archive);
            let _ = writeln!(out, "  Slides: {}", slide_count);

            // Read first slide title
            if slide_count > 0 {
                if let Some(slide1) = read_zip_entry(archive, "ppt/slides/slide1.xml") {
                    let title = extract_slide_title(&slide1);
                    if let Some(t) = title {
                        let _ = writeln!(out, "  First slide: {}", t);
                    }
                }
            }
        }
        _ => {
            let _ = writeln!(out, "  Unknown or unsupported Office format.");
        }
    }

    let total_files = archive.len();
    let _ = writeln!(out, "\n  ZIP entries: {}", total_files);

    out
}

fn extract_sheet_names(workbook_xml: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut pos = 0;
    while let Some(found) = workbook_xml[pos..].find("<sheet ") {
        let start = pos + found;
        let entry_end = workbook_xml[start..]
            .find('>')
            .map(|e| start + e + 1)
            .unwrap_or(workbook_xml.len());
        let entry = &workbook_xml[start..entry_end];
        if let Some(name) = extract_attr(entry, "name") {
            names.push(name);
        }
        pos = entry_end;
    }
    names
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let search = format!("{}=\"", attr);
    let start = tag.find(&search)? + search.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

fn count_slide_files(archive: &mut zip::ZipArchive<std::fs::File>) -> usize {
    (0..archive.len())
        .filter(|&i| {
            archive
                .by_index(i)
                .ok()
                .map(|f| {
                    let n = f.name().to_string();
                    n.starts_with("ppt/slides/slide") && n.ends_with(".xml")
                })
                .unwrap_or(false)
        })
        .count()
}

fn extract_slide_title(slide_xml: &str) -> Option<String> {
    // Look for title placeholder: <p:sp>...<p:ph type="title"/>
    // Then extract the text from nearby <a:t> elements
    // Simple approach: find first non-empty text in the slide
    let text = strip_tags(slide_xml);
    let first = text.split_whitespace().take(12).collect::<Vec<_>>().join(" ");
    if first.is_empty() {
        None
    } else {
        Some(first)
    }
}

fn extract_content(archive: &mut zip::ZipArchive<std::fs::File>) -> String {
    let format = detect_format(archive);
    let mut out = String::new();

    match format {
        "docx" => {
            let _ = writeln!(out, "Document Content  (DOCX)\n");
            if let Some(doc) = read_zip_entry(archive, "word/document.xml") {
                let text = strip_tags(&doc);
                // Show first 2000 chars
                let preview: String = text.chars().take(2000).collect();
                let _ = writeln!(out, "{}", preview);
                if text.len() > 2000 {
                    let _ = writeln!(out, "\n[... {} chars total ...]", text.len());
                }
            } else {
                out.push_str("Could not read word/document.xml\n");
            }
        }
        "xlsx" => {
            let _ = writeln!(out, "Workbook Content  (XLSX)\n");
            if let Some(wb) = read_zip_entry(archive, "xl/workbook.xml") {
                let names = extract_sheet_names(&wb);
                let _ = writeln!(out, "Sheets ({}):", names.len());
                for (i, name) in names.iter().enumerate() {
                    let _ = writeln!(out, "  {}. {}", i + 1, name);
                }
            }
            // Try to read shared strings for data preview
            out.push('\n');
            if let Some(ss) = read_zip_entry(archive, "xl/sharedStrings.xml") {
                let strings: Vec<String> = {
                    let mut v = Vec::new();
                    let mut pos = 0;
                    while let Some(found) = ss[pos..].find("<t") {
                        let start = pos + found;
                        let after = ss[start..].find('>').map(|e| start + e + 1).unwrap_or(ss.len());
                        let close = ss[after..].find("</t>").map(|e| after + e).unwrap_or(ss.len());
                        let text = ss[after..close].trim().to_string();
                        if !text.is_empty() {
                            v.push(text);
                        }
                        pos = close + 4;
                        if v.len() >= 50 {
                            break;
                        }
                    }
                    v
                };
                if !strings.is_empty() {
                    let _ = writeln!(out, "Shared strings (first {}):", strings.len());
                    for s in &strings {
                        let _ = writeln!(out, "  {}", s);
                    }
                }
            }
        }
        "pptx" => {
            let _ = writeln!(out, "Presentation Content  (PPTX)\n");
            let count = count_slide_files(archive);
            let _ = writeln!(out, "Slides: {}\n", count);
            for i in 1..=count.min(10) {
                let path = format!("ppt/slides/slide{}.xml", i);
                if let Some(xml) = read_zip_entry(archive, &path) {
                    let text = strip_tags(&xml);
                    let preview: String = text.split_whitespace().take(20).collect::<Vec<_>>().join(" ");
                    let _ = writeln!(out, "  Slide {}: {}", i, preview);
                }
            }
            if count > 10 {
                let _ = writeln!(out, "  ... ({} more slides)", count - 10);
            }
        }
        _ => {
            out.push_str("Unknown or unsupported Office format.\n");
        }
    }
    out
}

fn list_structure(archive: &mut zip::ZipArchive<std::fs::File>) -> String {
    let mut out = String::from("Document Structure  (ZIP parts)\n\n");
    let mut entries: Vec<(String, u64)> = (0..archive.len())
        .filter_map(|i| {
            archive
                .by_index(i)
                .ok()
                .map(|f| (f.name().to_string(), f.size()))
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // Group by folder
    let mut last_folder = String::new();
    for (name, size) in &entries {
        let folder = name
            .rfind('/')
            .map(|i| name[..i].to_string())
            .unwrap_or_default();
        if folder != last_folder {
            if !last_folder.is_empty() {
                out.push('\n');
            }
            if !folder.is_empty() {
                let _ = writeln!(out, "  {}/", folder);
            }
            last_folder = folder;
        }
        let file_name = name.rfind('/').map(|i| &name[i + 1..]).unwrap_or(name);
        let _ = writeln!(out, "    {:40} {:>8}", file_name, human_size(*size));
    }
    let _ = writeln!(out, "\n  Total entries: {}", entries.len());
    out
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    }
}

fn validate_doc(archive: &mut zip::ZipArchive<std::fs::File>) -> String {
    let format = detect_format(archive);
    let mut issues = Vec::new();
    let mut out = String::new();

    let _ = writeln!(out, "Validation  ({})\n", format.to_uppercase());

    // Check required parts
    let has_content_types = read_zip_entry(archive, "[Content_Types].xml").is_some();
    if !has_content_types {
        issues.push("[Content_Types].xml missing (required)".to_string());
    }

    let has_rels = read_zip_entry(archive, "_rels/.rels").is_some();
    if !has_rels {
        issues.push("_rels/.rels missing (required)".to_string());
    }

    match format {
        "docx" => {
            if read_zip_entry(archive, "word/document.xml").is_none() {
                issues.push("word/document.xml missing".to_string());
            }
            if read_zip_entry(archive, "word/_rels/document.xml.rels").is_none() {
                issues.push("word/_rels/document.xml.rels missing (may cause opening errors)".to_string());
            }
        }
        "xlsx" => {
            if read_zip_entry(archive, "xl/workbook.xml").is_none() {
                issues.push("xl/workbook.xml missing".to_string());
            }
        }
        "pptx" => {
            if read_zip_entry(archive, "ppt/presentation.xml").is_none() {
                issues.push("ppt/presentation.xml missing".to_string());
            }
        }
        _ => {
            issues.push("Unknown format — cannot validate specific requirements".to_string());
        }
    }

    // Check core metadata
    let meta = read_core_meta(archive);
    if meta.title.is_none() {
        issues.push("No document title in metadata".to_string());
    }
    if meta.creator.is_none() {
        issues.push("No author in metadata".to_string());
    }

    if issues.is_empty() {
        let _ = writeln!(out, "  ✓  VALID — no structural issues found.");
    } else {
        let _ = writeln!(out, "  Found {} issue(s):\n", issues.len());
        for issue in &issues {
            let _ = writeln!(out, "  ⚠  {}", issue);
        }
    }

    out
}
