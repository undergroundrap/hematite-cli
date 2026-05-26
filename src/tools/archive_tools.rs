use std::io::Read;
use zip::ZipArchive;

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("list");

    match action {
        "list" => list(args),
        "info" => info(args),
        "inspect" => inspect(args),
        "extract" => extract(args),
        other => Err(format!(
            "archive_tools: unknown action '{other}'. Valid: list, info, inspect, extract"
        )),
    }
}

// ── Path resolution ────────────────────────────────────────────────────────────

fn resolve_path(args: &serde_json::Value) -> Result<std::path::PathBuf, String> {
    let file = args
        .get("file")
        .or_else(|| args.get("path"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("archive_tools: 'file' is required (path to the .zip archive)")?;

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
        return Err(format!("archive_tools: file not found: {}", path.display()));
    }
    Ok(path)
}

fn open_archive(path: &std::path::Path) -> Result<ZipArchive<std::fs::File>, String> {
    let f = std::fs::File::open(path)
        .map_err(|e| format!("archive_tools: cannot open '{}': {e}", path.display()))?;
    ZipArchive::new(f).map_err(|e| format!("archive_tools: not a valid zip archive: {e}"))
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn compression_name(method: zip::CompressionMethod) -> &'static str {
    match method {
        zip::CompressionMethod::Stored => "Stored",
        zip::CompressionMethod::Deflated => "Deflate",
        _ => "Other",
    }
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn list(args: &serde_json::Value) -> Result<String, String> {
    let path = resolve_path(args)?;
    let mut archive = open_archive(&path)?;
    let max = args.get("max").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
    let filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    let total = archive.len();
    let mut out = format!(
        "ARCHIVE LIST: {}\n{}\n",
        path.file_name().unwrap_or_default().to_string_lossy(),
        "─".repeat(50)
    );
    out.push_str(&format!(
        "{:<10} {:<12} {:<12} {:<8} {}\n",
        "Compressed", "Uncompressed", "Method", "Type", "Name"
    ));
    out.push_str(&format!("{}\n", "─".repeat(70)));

    let mut shown = 0;
    let mut skipped = 0;
    for i in 0..total {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("archive_tools: error reading entry {i}: {e}"))?;
        let name = entry.name().to_string();
        if !filter.is_empty() && !name.to_lowercase().contains(&filter) {
            skipped += 1;
            continue;
        }
        if shown >= max {
            skipped += 1;
            continue;
        }
        let kind = if entry.is_dir() { "DIR " } else { "FILE" };
        out.push_str(&format!(
            "{:<10} {:<12} {:<12} {:<8} {}\n",
            human_size(entry.compressed_size()),
            human_size(entry.size()),
            compression_name(entry.compression()),
            kind,
            name,
        ));
        shown += 1;
    }

    out.push_str(&format!("{}\n", "─".repeat(70)));
    out.push_str(&format!("Showing {shown} of {total} entries"));
    if skipped > 0 {
        out.push_str(&format!(" ({skipped} filtered/truncated)"));
    }
    out.push('\n');
    Ok(out)
}

fn info(args: &serde_json::Value) -> Result<String, String> {
    let path = resolve_path(args)?;
    let mut archive = open_archive(&path)?;

    let metadata =
        std::fs::metadata(&path).map_err(|e| format!("archive_tools: metadata error: {e}"))?;
    let archive_size = metadata.len();

    let mut file_count: u64 = 0;
    let mut dir_count: u64 = 0;
    let mut total_uncompressed: u64 = 0;
    let mut total_compressed: u64 = 0;
    let mut methods: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();

    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("archive_tools: read error: {e}"))?;
        if entry.is_dir() {
            dir_count += 1;
        } else {
            file_count += 1;
            total_uncompressed += entry.size();
            total_compressed += entry.compressed_size();
            *methods
                .entry(compression_name(entry.compression()).to_string())
                .or_insert(0) += 1;
        }
    }

    let ratio = if total_uncompressed > 0 {
        format!(
            "{:.1}%",
            (1.0 - total_compressed as f64 / total_uncompressed as f64) * 100.0
        )
    } else {
        "N/A".to_string()
    };

    let mut out = format!(
        "ARCHIVE INFO: {}\n{}\n",
        path.file_name().unwrap_or_default().to_string_lossy(),
        "─".repeat(50)
    );
    out.push_str(&format!(
        "Archive size     : {}\n",
        human_size(archive_size)
    ));
    out.push_str(&format!("Files            : {file_count}\n"));
    out.push_str(&format!("Directories      : {dir_count}\n"));
    out.push_str(&format!(
        "Total uncompressed: {}\n",
        human_size(total_uncompressed)
    ));
    out.push_str(&format!(
        "Total compressed : {}\n",
        human_size(total_compressed)
    ));
    out.push_str(&format!("Space saved      : {ratio}\n"));

    if !methods.is_empty() {
        out.push_str("Compression methods:\n");
        for (m, count) in &methods {
            out.push_str(&format!("  {m}: {count} file(s)\n"));
        }
    }

    let comment = archive.comment();
    if !comment.is_empty() {
        out.push_str(&format!(
            "Comment          : {}\n",
            String::from_utf8_lossy(comment)
        ));
    }

    Ok(out)
}

fn inspect(args: &serde_json::Value) -> Result<String, String> {
    let path = resolve_path(args)?;
    let entry_name = args
        .get("entry")
        .or_else(|| args.get("name"))
        .and_then(|v| v.as_str())
        .ok_or(
            "archive_tools inspect: 'entry' is required (name of the file inside the archive)",
        )?;

    let mut archive = open_archive(&path)?;
    let entry = archive
        .by_name(entry_name)
        .map_err(|_| format!("archive_tools: entry '{entry_name}' not found in archive"))?;

    let last_mod = entry.last_modified().map(|dt| {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second()
        )
    });

    let mut out = format!("ARCHIVE INSPECT\n{}\n", "─".repeat(50));
    out.push_str(&format!("Name        : {}\n", entry.name()));
    out.push_str(&format!(
        "Type        : {}\n",
        if entry.is_dir() { "Directory" } else { "File" }
    ));
    out.push_str(&format!(
        "Compressed  : {}\n",
        human_size(entry.compressed_size())
    ));
    out.push_str(&format!("Uncompressed: {}\n", human_size(entry.size())));
    out.push_str(&format!(
        "Method      : {}\n",
        compression_name(entry.compression())
    ));
    if let Some(dt) = last_mod {
        out.push_str(&format!("Modified    : {dt}\n"));
    }
    if let Some(crc) = Some(entry.crc32()) {
        out.push_str(&format!("CRC-32      : {crc:#010x}\n"));
    }
    if entry.is_file() {
        let ratio = if entry.size() > 0 {
            format!(
                "{:.1}%",
                (1.0 - entry.compressed_size() as f64 / entry.size() as f64) * 100.0
            )
        } else {
            "N/A".to_string()
        };
        out.push_str(&format!("Ratio       : {ratio} saved\n"));
    }

    Ok(out)
}

fn extract(args: &serde_json::Value) -> Result<String, String> {
    let path = resolve_path(args)?;
    let entry_name = args
        .get("entry")
        .or_else(|| args.get("name"))
        .and_then(|v| v.as_str())
        .ok_or("archive_tools extract: 'entry' is required")?;

    const MAX_BYTES: u64 = 1024 * 1024; // 1 MB text limit

    let mut archive = open_archive(&path)?;
    let mut entry = archive
        .by_name(entry_name)
        .map_err(|_| format!("archive_tools: entry '{entry_name}' not found"))?;

    if entry.is_dir() {
        return Err(format!(
            "archive_tools: '{entry_name}' is a directory, not a file"
        ));
    }
    if entry.size() > MAX_BYTES {
        return Err(format!(
            "archive_tools: '{entry_name}' is {} — too large to extract as text (limit: 1 MB). \
             Use a dedicated tool to extract binary files.",
            human_size(entry.size())
        ));
    }

    let mut buf = Vec::new();
    entry
        .read_to_end(&mut buf)
        .map_err(|e| format!("archive_tools: read error: {e}"))?;

    let text = String::from_utf8(buf).map_err(|_| {
        format!(
            "archive_tools: '{entry_name}' is not valid UTF-8 (binary file). \
             Use a dedicated tool for binary extraction."
        )
    })?;

    let mut out = format!("ARCHIVE EXTRACT: {entry_name}\n{}\n", "─".repeat(50));
    out.push_str(&text);
    if !text.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}
