use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("toc");

    match action {
        "toc" => toc(args),
        "stats" => stats(args),
        "extract" => extract(args),
        "links" => links(args),
        "to-html" => to_html(args),
        "strip" => strip(args),
        other => Err(format!(
            "markdown_tools: unknown action '{other}'. Valid: toc, stats, extract, links, to-html, strip"
        )),
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn load_markdown(args: &serde_json::Value) -> Result<String, String> {
    if let Some(text) = args.get("text").and_then(|v| v.as_str()) {
        return Ok(text.to_string());
    }
    if let Some(file) = args
        .get("file")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
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
        return std::fs::read_to_string(&path)
            .map_err(|e| format!("markdown_tools: cannot read '{}': {e}", path.display()));
    }
    Err("markdown_tools: 'text' (inline markdown) or 'file' (path) is required".to_string())
}

fn heading_level_num(level: &HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn collect_text_in_heading(parser: &mut std::iter::Peekable<Parser>) -> String {
    let mut text = String::new();
    for event in parser.by_ref() {
        match event {
            Event::Text(s) | Event::Code(s) => text.push_str(&s),
            Event::End(TagEnd::Heading(_)) => break,
            _ => {}
        }
    }
    text
}

fn slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn toc(args: &serde_json::Value) -> Result<String, String> {
    let src = load_markdown(args)?;
    let max_depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(3) as u8;

    let opts = Options::all();
    let mut parser = Parser::new_ext(&src, opts).peekable();

    struct Heading {
        level: u8,
        text: String,
    }

    let mut headings: Vec<Heading> = Vec::new();

    while let Some(event) = parser.next() {
        if let Event::Start(Tag::Heading { level, .. }) = event {
            let num = heading_level_num(&level);
            if num <= max_depth {
                let text = collect_text_in_heading(&mut parser);
                headings.push(Heading { level: num, text });
            }
        }
    }

    if headings.is_empty() {
        return Ok("markdown_tools toc: no headings found.\n".to_string());
    }

    let min_level = headings.iter().map(|h| h.level).min().unwrap_or(1);

    let mut out = String::from("TABLE OF CONTENTS\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');

    for h in &headings {
        let indent = "  ".repeat((h.level - min_level) as usize);
        let anchor = slug(&h.text);
        out.push_str(&format!("{indent}- [{}](#{anchor})\n", h.text));
    }

    out.push_str(&format!("\n{} heading(s)\n", headings.len()));
    Ok(out)
}

fn stats(args: &serde_json::Value) -> Result<String, String> {
    let src = load_markdown(args)?;

    let opts = Options::all();
    let parser = Parser::new_ext(&src, opts);

    let mut word_count = 0usize;
    let mut heading_count = 0usize;
    let mut code_block_count = 0usize;
    let mut link_count = 0usize;
    let mut image_count = 0usize;
    let mut list_item_count = 0usize;
    let mut table_count = 0usize;
    let mut blockquote_count = 0usize;
    let mut in_code_block = false;
    let mut code_lines = 0usize;
    let mut h_levels = [0usize; 6];

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let n = heading_level_num(&level) as usize;
                if (1..=6).contains(&n) {
                    h_levels[n - 1] += 1;
                }
                heading_count += 1;
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                code_block_count += 1;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            Event::Start(Tag::Link { .. }) => link_count += 1,
            Event::Start(Tag::Image { .. }) => image_count += 1,
            Event::Start(Tag::Item) => list_item_count += 1,
            Event::Start(Tag::Table(_)) => table_count += 1,
            Event::Start(Tag::BlockQuote(_)) => blockquote_count += 1,
            Event::Text(s) => {
                if in_code_block {
                    code_lines += s.lines().count();
                } else {
                    word_count += s.split_whitespace().count();
                }
            }
            Event::Code(s) => {
                word_count += s.split_whitespace().count();
            }
            _ => {}
        }
    }

    let line_count = src.lines().count();
    let char_count = src.chars().count();
    let reading_minutes = (word_count as f64 / 200.0).max(0.5);

    let mut out = String::from("MARKDOWN STATS\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!("Lines          : {line_count}\n"));
    out.push_str(&format!("Characters     : {char_count}\n"));
    out.push_str(&format!("Words          : {word_count}\n"));
    out.push_str(&format!("Reading time   : ~{reading_minutes:.0} min\n"));
    out.push_str(&format!("Headings       : {heading_count}\n"));

    // Per-level counts
    let level_summary: String = h_levels
        .iter()
        .enumerate()
        .filter(|(_, &c)| c > 0)
        .map(|(i, c)| format!("H{}:{}", i + 1, c))
        .collect::<Vec<_>>()
        .join("  ");
    if !level_summary.is_empty() {
        out.push_str(&format!("  by level     : {level_summary}\n"));
    }

    out.push_str(&format!("Code blocks    : {code_block_count}"));
    if code_lines > 0 {
        out.push_str(&format!(" ({code_lines} lines)"));
    }
    out.push('\n');
    out.push_str(&format!("Links          : {link_count}\n"));
    out.push_str(&format!("Images         : {image_count}\n"));
    out.push_str(&format!("List items     : {list_item_count}\n"));
    out.push_str(&format!("Tables         : {table_count}\n"));
    out.push_str(&format!("Blockquotes    : {blockquote_count}\n"));
    Ok(out)
}

fn extract(args: &serde_json::Value) -> Result<String, String> {
    let src = load_markdown(args)?;
    let what = args
        .get("what")
        .or_else(|| args.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("headings");

    let opts = Options::all();
    let mut parser = Parser::new_ext(&src, opts).peekable();

    match what {
        "headings" | "headers" => {
            let max_depth = args
                .get("depth")
                .and_then(|v| v.as_u64())
                .unwrap_or(6) as u8;

            let mut results: Vec<(u8, String)> = Vec::new();
            while let Some(event) = parser.next() {
                if let Event::Start(Tag::Heading { level, .. }) = event {
                    let num = heading_level_num(&level);
                    if num <= max_depth {
                        let text = collect_text_in_heading(&mut parser);
                        results.push((num, text));
                    }
                }
            }

            let mut out = format!("MARKDOWN EXTRACT: headings\n{}\n", "─".repeat(50));
            for (level, text) in &results {
                out.push_str(&format!("{} {text}\n", "#".repeat(*level as usize)));
            }
            out.push_str(&format!("\n{} heading(s)\n", results.len()));
            Ok(out)
        }
        "code" | "code_blocks" | "codeblocks" => {
            let lang_filter = args.get("lang").and_then(|v| v.as_str());

            let mut results: Vec<(String, String)> = Vec::new();
            let mut current_lang = String::new();
            let mut in_block = false;

            for event in Parser::new_ext(&src, opts) {
                match event {
                    Event::Start(Tag::CodeBlock(kind)) => {
                        in_block = true;
                        current_lang = match kind {
                            pulldown_cmark::CodeBlockKind::Fenced(s) => s.to_string(),
                            pulldown_cmark::CodeBlockKind::Indented => String::new(),
                        };
                    }
                    Event::Text(s) if in_block => {
                        if lang_filter.is_none_or(|f| current_lang.starts_with(f)) {
                            results.push((current_lang.clone(), s.to_string()));
                        }
                    }
                    Event::End(TagEnd::CodeBlock) => {
                        in_block = false;
                    }
                    _ => {}
                }
            }

            let mut out = format!("MARKDOWN EXTRACT: code blocks\n{}\n", "─".repeat(50));
            if results.is_empty() {
                out.push_str("No code blocks found.\n");
            }
            for (i, (lang, code)) in results.iter().enumerate() {
                let label = if lang.is_empty() {
                    "(no language)".to_string()
                } else {
                    lang.clone()
                };
                out.push_str(&format!("── Block {} [{}] ──\n{}\n", i + 1, label, code));
            }
            Ok(out)
        }
        "links" | "urls" => links(args),
        "images" => {
            let mut results: Vec<(String, String)> = Vec::new();
            for event in Parser::new_ext(&src, opts) {
                if let Event::Start(Tag::Image {
                    dest_url, title, ..
                }) = event
                {
                    results.push((dest_url.to_string(), title.to_string()));
                }
            }
            let mut out = format!("MARKDOWN EXTRACT: images\n{}\n", "─".repeat(50));
            for (url, title) in &results {
                if title.is_empty() {
                    out.push_str(&format!("  {url}\n"));
                } else {
                    out.push_str(&format!("  {url}  ({title})\n"));
                }
            }
            out.push_str(&format!("\n{} image(s)\n", results.len()));
            Ok(out)
        }
        other => Err(format!(
            "markdown_tools extract: unknown 'what' value '{other}'. Valid: headings, code, links, images"
        )),
    }
}

fn links(args: &serde_json::Value) -> Result<String, String> {
    let src = load_markdown(args)?;

    let opts = Options::all();
    let parser = Parser::new_ext(&src, opts);

    struct LinkEntry {
        text: String,
        url: String,
        title: String,
        is_image: bool,
    }

    let mut results: Vec<LinkEntry> = Vec::new();
    let mut link_text_buf = String::new();
    let mut in_link = false;
    let mut current_url = String::new();
    let mut current_title = String::new();
    let mut current_is_image = false;

    for event in parser {
        match event {
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) => {
                in_link = true;
                link_text_buf.clear();
                current_url = dest_url.to_string();
                current_title = title.to_string();
                current_is_image = false;
            }
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                results.push(LinkEntry {
                    text: String::new(),
                    url: dest_url.to_string(),
                    title: title.to_string(),
                    is_image: true,
                });
            }
            Event::Text(s) if in_link => link_text_buf.push_str(&s),
            Event::End(TagEnd::Link) => {
                results.push(LinkEntry {
                    text: link_text_buf.clone(),
                    url: current_url.clone(),
                    title: current_title.clone(),
                    is_image: current_is_image,
                });
                in_link = false;
            }
            _ => {}
        }
    }

    let mut out = format!("MARKDOWN LINKS\n{}\n", "─".repeat(50));
    if results.is_empty() {
        out.push_str("No links found.\n");
        return Ok(out);
    }

    let links_only: Vec<_> = results.iter().filter(|e| !e.is_image).collect();
    let images_only: Vec<_> = results.iter().filter(|e| e.is_image).collect();

    if !links_only.is_empty() {
        out.push_str(&format!("Links ({}):\n", links_only.len()));
        for e in &links_only {
            if e.title.is_empty() {
                out.push_str(&format!("  [{}] {}\n", e.text, e.url));
            } else {
                out.push_str(&format!("  [{}] {}  \"{}\"\n", e.text, e.url, e.title));
            }
        }
    }
    if !images_only.is_empty() {
        out.push_str(&format!("\nImages ({}):\n", images_only.len()));
        for e in &images_only {
            if e.title.is_empty() {
                out.push_str(&format!("  {}\n", e.url));
            } else {
                out.push_str(&format!("  {}  \"{}\"\n", e.url, e.title));
            }
        }
    }
    Ok(out)
}

fn to_html(args: &serde_json::Value) -> Result<String, String> {
    let src = load_markdown(args)?;

    let opts = Options::all();
    let parser = Parser::new_ext(&src, opts);

    let mut html_output = String::with_capacity(src.len() * 2);
    pulldown_cmark::html::push_html(&mut html_output, parser);

    let wrap = args.get("wrap").and_then(|v| v.as_bool()).unwrap_or(false);

    if wrap {
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Document");
        Ok(format!(
            "<!DOCTYPE html>\n<html>\n<head><meta charset=\"utf-8\"><title>{title}</title></head>\n<body>\n{html_output}\n</body>\n</html>\n"
        ))
    } else {
        Ok(html_output)
    }
}

fn strip(args: &serde_json::Value) -> Result<String, String> {
    let src = load_markdown(args)?;

    let opts = Options::all();
    let parser = Parser::new_ext(&src, opts);

    let mut plain = String::with_capacity(src.len());
    let mut in_code = false;
    let mut after_block = false;

    for event in parser {
        match event {
            Event::Text(s) => {
                if in_code {
                    plain.push_str(&s);
                } else {
                    if after_block {
                        plain.push('\n');
                        after_block = false;
                    }
                    plain.push_str(&s);
                }
            }
            Event::Code(s) => {
                plain.push('`');
                plain.push_str(&s);
                plain.push('`');
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code = true;
                plain.push('\n');
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                plain.push('\n');
            }
            Event::Start(Tag::Paragraph)
            | Event::Start(Tag::Heading { .. })
            | Event::Start(Tag::BlockQuote(_))
            | Event::Start(Tag::Item) => {
                if !plain.is_empty() && !plain.ends_with('\n') {
                    plain.push('\n');
                }
            }
            Event::End(TagEnd::Paragraph)
            | Event::End(TagEnd::Heading(_))
            | Event::End(TagEnd::BlockQuote(_)) => {
                after_block = true;
            }
            Event::SoftBreak => plain.push(' '),
            Event::HardBreak => plain.push('\n'),
            Event::Rule => {
                plain.push_str("\n---\n");
            }
            _ => {}
        }
    }

    let result = plain.trim().to_string();
    let mut out = format!("MARKDOWN STRIPPED\n{}\n", "─".repeat(50));
    out.push_str(&result);
    out.push('\n');
    Ok(out)
}
