use serde_json::Value;

// (extension, mime_type, category)
static MIME_TABLE: &[(&str, &str, &str)] = &[
    // Text / Code
    ("html", "text/html", "text"),
    ("htm", "text/html", "text"),
    ("css", "text/css", "text"),
    ("csv", "text/csv", "text"),
    ("txt", "text/plain", "text"),
    ("md", "text/markdown", "text"),
    ("markdown", "text/markdown", "text"),
    ("xml", "text/xml", "text"),
    ("js", "application/javascript", "text"),
    ("mjs", "application/javascript", "text"),
    ("cjs", "application/javascript", "text"),
    ("ts", "application/typescript", "text"),
    ("tsx", "application/typescript", "text"),
    ("jsx", "application/javascript", "text"),
    ("json", "application/json", "text"),
    ("jsonl", "application/x-ndjson", "text"),
    ("ndjson", "application/x-ndjson", "text"),
    ("yaml", "application/x-yaml", "text"),
    ("yml", "application/x-yaml", "text"),
    ("toml", "application/toml", "text"),
    ("rs", "text/x-rust", "text"),
    ("py", "text/x-python", "text"),
    ("rb", "text/x-ruby", "text"),
    ("php", "text/x-php", "text"),
    ("go", "text/x-go", "text"),
    ("java", "text/x-java-source", "text"),
    ("kt", "text/x-kotlin", "text"),
    ("swift", "text/x-swift", "text"),
    ("c", "text/x-c", "text"),
    ("h", "text/x-c", "text"),
    ("cpp", "text/x-c++", "text"),
    ("cc", "text/x-c++", "text"),
    ("cs", "text/x-csharp", "text"),
    ("sh", "application/x-sh", "text"),
    ("bash", "application/x-sh", "text"),
    ("zsh", "application/x-sh", "text"),
    ("bat", "application/x-bat", "text"),
    ("ps1", "text/plain", "text"),
    ("sql", "application/sql", "text"),
    ("graphql", "application/graphql", "text"),
    ("gql", "application/graphql", "text"),
    ("tex", "application/x-tex", "text"),
    ("log", "text/plain", "text"),
    ("ini", "text/plain", "text"),
    ("cfg", "text/plain", "text"),
    ("conf", "text/plain", "text"),
    ("env", "text/plain", "text"),
    ("rtf", "application/rtf", "text"),
    ("diff", "text/x-diff", "text"),
    ("patch", "text/x-diff", "text"),
    ("proto", "text/plain", "text"),
    // Images
    ("png", "image/png", "image"),
    ("jpg", "image/jpeg", "image"),
    ("jpeg", "image/jpeg", "image"),
    ("gif", "image/gif", "image"),
    ("webp", "image/webp", "image"),
    ("bmp", "image/bmp", "image"),
    ("ico", "image/x-icon", "image"),
    ("svg", "image/svg+xml", "image"),
    ("tiff", "image/tiff", "image"),
    ("tif", "image/tiff", "image"),
    ("avif", "image/avif", "image"),
    ("heic", "image/heic", "image"),
    ("heif", "image/heif", "image"),
    ("raw", "image/x-raw", "image"),
    ("psd", "image/vnd.adobe.photoshop", "image"),
    // Audio
    ("mp3", "audio/mpeg", "audio"),
    ("wav", "audio/wav", "audio"),
    ("ogg", "audio/ogg", "audio"),
    ("flac", "audio/flac", "audio"),
    ("aac", "audio/aac", "audio"),
    ("m4a", "audio/mp4", "audio"),
    ("opus", "audio/opus", "audio"),
    ("weba", "audio/webm", "audio"),
    ("mid", "audio/midi", "audio"),
    ("midi", "audio/midi", "audio"),
    // Video
    ("mp4", "video/mp4", "video"),
    ("webm", "video/webm", "video"),
    ("avi", "video/x-msvideo", "video"),
    ("mkv", "video/x-matroska", "video"),
    ("mov", "video/quicktime", "video"),
    ("wmv", "video/x-ms-wmv", "video"),
    ("flv", "video/x-flv", "video"),
    ("m4v", "video/mp4", "video"),
    ("ogv", "video/ogg", "video"),
    ("m2ts", "video/mp2t", "video"),
    ("mts", "video/mp2t", "video"),
    // Application / Binary / Archive
    ("pdf", "application/pdf", "application"),
    ("zip", "application/zip", "application"),
    ("gz", "application/gzip", "application"),
    ("tar", "application/x-tar", "application"),
    ("bz2", "application/x-bzip2", "application"),
    ("xz", "application/x-xz", "application"),
    ("7z", "application/x-7z-compressed", "application"),
    ("rar", "application/x-rar-compressed", "application"),
    ("jar", "application/java-archive", "application"),
    ("war", "application/java-archive", "application"),
    ("ear", "application/java-archive", "application"),
    ("wasm", "application/wasm", "application"),
    ("exe", "application/octet-stream", "application"),
    ("dll", "application/octet-stream", "application"),
    ("so", "application/octet-stream", "application"),
    ("bin", "application/octet-stream", "application"),
    ("dmg", "application/x-apple-diskimage", "application"),
    ("iso", "application/x-iso9660-image", "application"),
    ("doc", "application/msword", "application"),
    (
        "docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application",
    ),
    ("xls", "application/vnd.ms-excel", "application"),
    (
        "xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "application",
    ),
    ("ppt", "application/vnd.ms-powerpoint", "application"),
    (
        "pptx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "application",
    ),
    (
        "odt",
        "application/vnd.oasis.opendocument.text",
        "application",
    ),
    (
        "ods",
        "application/vnd.oasis.opendocument.spreadsheet",
        "application",
    ),
    (
        "odp",
        "application/vnd.oasis.opendocument.presentation",
        "application",
    ),
    ("epub", "application/epub+zip", "application"),
    ("whl", "application/zip", "application"),
    ("vsix", "application/zip", "application"),
    (
        "apk",
        "application/vnd.android.package-archive",
        "application",
    ),
    ("ipa", "application/octet-stream", "application"),
    ("deb", "application/x-deb", "application"),
    ("rpm", "application/x-rpm", "application"),
    ("crx", "application/x-chrome-extension", "application"),
    // Fonts
    ("ttf", "font/ttf", "font"),
    ("otf", "font/otf", "font"),
    ("woff", "font/woff", "font"),
    ("woff2", "font/woff2", "font"),
    ("eot", "application/vnd.ms-fontobject", "font"),
];

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("ext").is_some() || args.get("extension").is_some() {
        "from_ext".to_string()
    } else if args.get("mime").is_some() || args.get("type").is_some() {
        "from_mime".to_string()
    } else if args.get("query").is_some() || args.get("q").is_some() {
        "search".to_string()
    } else if args.get("category").is_some() || args.get("cat").is_some() {
        "category".to_string()
    } else {
        "from_ext".to_string()
    };
    match action.as_str() {
        "from_ext" | "ext" => from_ext_action(args),
        "from_mime" | "mime" => from_mime_action(args),
        "search" => search_action(args),
        "category" => category_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: from_ext, from_mime, search, category",
            action
        )),
    }
}

fn from_ext_action(args: &Value) -> Result<String, String> {
    let raw = args
        .get("ext")
        .or_else(|| args.get("extension"))
        .or_else(|| args.get("file"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'ext' — pass extension as 'js', '.ts', or 'image.png'")?;

    // Accept "file.js", ".js", or "js"
    let stripped = raw.trim_start_matches('.');
    let ext_lower = if stripped.contains('.') {
        stripped.rsplit('.').next().unwrap_or(stripped)
    } else {
        stripped
    }
    .to_lowercase();

    let matches: Vec<_> = MIME_TABLE
        .iter()
        .filter(|(e, _, _)| *e == ext_lower.as_str())
        .collect();

    if matches.is_empty() {
        return Ok(format!(
            "No MIME type found for '.{}'\n\nFallback: application/octet-stream (generic binary)\n",
            ext_lower
        ));
    }

    let mut out = format!("MIME Type for .{}\n{}\n\n", ext_lower, "=".repeat(44));
    for (e, mime, cat) in &matches {
        out += &format!("MIME type: {}\n", mime);
        out += &format!("Category:  {}\n", cat);
        out += &format!("Extension: .{}\n", e);
    }
    Ok(out)
}

fn from_mime_action(args: &Value) -> Result<String, String> {
    let mime = args
        .get("mime")
        .or_else(|| args.get("type"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'mime' argument — pass MIME type like 'image/png'")?;
    let lower = mime.to_lowercase();

    let exact: Vec<_> = MIME_TABLE
        .iter()
        .filter(|(_, m, _)| m.to_lowercase() == lower)
        .collect();

    if !exact.is_empty() {
        let exts: Vec<String> = exact.iter().map(|(e, _, _)| format!(".{}", e)).collect();
        let cat = exact[0].2;
        let mut out = format!("Extensions for {}\n{}\n\n", lower, "=".repeat(44));
        out += &format!("Extensions: {}\n", exts.join(", "));
        out += &format!("Category:   {}\n", cat);
        return Ok(out);
    }

    // Partial match
    let partial: Vec<_> = MIME_TABLE
        .iter()
        .filter(|(_, m, _)| m.to_lowercase().contains(lower.as_str()))
        .collect();

    if partial.is_empty() {
        return Ok(format!(
            "No extensions found for MIME type '{}'\n\nUse 'search' action for fuzzy matching.\n",
            mime
        ));
    }

    let mut out = format!(
        "Partial matches for '{}' ({} results)\n{}\n\n",
        mime,
        partial.len(),
        "=".repeat(44)
    );
    out += &format!("{:<12} {:<45} {}\n", "Extension", "MIME Type", "Category");
    out += &format!("{}\n", "-".repeat(65));
    for (e, m, cat) in &partial {
        out += &format!(".{:<11} {:<45} {}\n", e, m, cat);
    }
    Ok(out)
}

fn search_action(args: &Value) -> Result<String, String> {
    let query = args
        .get("query")
        .or_else(|| args.get("q"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'query' argument")?;
    let q = query.to_lowercase();

    let matches: Vec<_> = MIME_TABLE
        .iter()
        .filter(|(e, m, _)| e.contains(q.as_str()) || m.contains(q.as_str()))
        .collect();

    if matches.is_empty() {
        return Ok(format!("No MIME types matching '{}'\n", query));
    }

    let mut out = format!(
        "MIME search: '{}' ({} results)\n{}\n\n",
        query,
        matches.len(),
        "=".repeat(44)
    );
    out += &format!("{:<12} {:<45} {}\n", "Extension", "MIME Type", "Category");
    out += &format!("{}\n", "-".repeat(65));
    for (e, m, cat) in &matches {
        out += &format!(".{:<11} {:<45} {}\n", e, m, cat);
    }
    Ok(out)
}

fn category_action(args: &Value) -> Result<String, String> {
    let cat_arg = args
        .get("category")
        .or_else(|| args.get("cat"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str());

    if cat_arg.is_none() {
        let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for (_, _, c) in MIME_TABLE {
            *counts.entry(c).or_insert(0) += 1;
        }
        let mut out = format!(
            "MIME Type Categories  ({} entries total)\n{}\n\n",
            MIME_TABLE.len(),
            "=".repeat(44)
        );
        for (c, n) in &counts {
            out += &format!("  {:<12} {} types\n", c, n);
        }
        out += "\nPass category: \"image\" to list entries in a specific category.\n";
        return Ok(out);
    }

    let cat = cat_arg.unwrap().to_lowercase();
    let matches: Vec<_> = MIME_TABLE
        .iter()
        .filter(|(_, _, c)| c.to_lowercase() == cat || c.to_lowercase().contains(cat.as_str()))
        .collect();

    if matches.is_empty() {
        return Ok(format!(
            "No entries in category '{}'.\nValid categories: application, audio, font, image, text, video\n",
            cat
        ));
    }

    let mut out = format!(
        "Category: {} ({} types)\n{}\n\n",
        cat,
        matches.len(),
        "=".repeat(44)
    );
    out += &format!("{:<12} {}\n", "Extension", "MIME Type");
    out += &format!("{}\n", "-".repeat(55));
    for (e, m, _) in &matches {
        out += &format!(".{:<11} {}\n", e, m);
    }
    Ok(out)
}
