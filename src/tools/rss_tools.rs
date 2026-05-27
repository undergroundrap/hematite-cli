use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("list");
    let content = get_content(args)?;
    match action {
        "list" => list_action(&content, args),
        "info" => info_action(&content),
        "links" => links_action(&content),
        "search" => search_action(&content, args),
        other => Err(format!(
            "rss_tools: unknown action '{other}'. Valid: list, info, links, search"
        )),
    }
}

fn get_content(args: &Value) -> Result<String, String> {
    if let Some(s) = args
        .get("text")
        .or_else(|| args.get("xml"))
        .or_else(|| args.get("rss"))
    {
        if let Some(text) = s.as_str() {
            return Ok(text.to_string());
        }
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path)
            .map_err(|e| format!("rss_tools: cannot read '{path}': {e}"));
    }
    Err("rss_tools: provide 'text'/'xml'/'rss' (inline feed content) or 'file' (path to .xml/.rss file)".to_string())
}

#[derive(Default, Debug)]
struct FeedEntry {
    title: String,
    link: String,
    description: String,
    pub_date: String,
    author: String,
    guid: String,
}

#[derive(Default, Debug)]
struct FeedMeta {
    title: String,
    link: String,
    description: String,
    language: String,
    pub_date: String,
    generator: String,
    feed_type: String,
}

fn parse_feed(xml: &str) -> Result<(FeedMeta, Vec<FeedEntry>), String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut meta = FeedMeta::default();
    let mut entries: Vec<FeedEntry> = Vec::new();
    let mut current: Option<FeedEntry> = None;
    let mut stack: Vec<String> = Vec::new();
    let mut text_buf = String::new();
    let mut in_channel = false;
    let mut channel_depth = 0usize;
    let mut is_atom = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_lowercase();
                // Strip namespace prefix
                let name = name.rsplit(':').next().unwrap_or(&name).to_string();

                if name == "feed" {
                    is_atom = true;
                    meta.feed_type = "Atom".to_string();
                    in_channel = true;
                    channel_depth = stack.len() + 1;
                } else if name == "channel" && !is_atom {
                    meta.feed_type = "RSS 2.0".to_string();
                    in_channel = true;
                    channel_depth = stack.len() + 1;
                } else if name == "item" || name == "entry" {
                    current = Some(FeedEntry::default());
                }

                // Capture link href attribute for Atom <link>
                if name == "link" && is_atom {
                    if let Some(href) = e.attributes().filter_map(|a| a.ok()).find(|a| {
                        std::str::from_utf8(a.key.as_ref())
                            .unwrap_or("")
                            .to_lowercase()
                            == "href"
                    }) {
                        let href_val = href
                            .decode_and_unescape_value(reader.decoder())
                            .unwrap_or_default()
                            .to_string();
                        if let Some(ref mut entry) = current {
                            if entry.link.is_empty() {
                                entry.link = href_val;
                            }
                        } else if meta.link.is_empty() {
                            meta.link = href_val;
                        }
                    }
                }

                stack.push(name);
                text_buf.clear();
            }
            Ok(Event::End(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_lowercase();
                let name = name.rsplit(':').next().unwrap_or(&name).to_string();
                let text = text_buf.trim().to_string();

                if let Some(ref mut entry) = current {
                    match name.as_str() {
                        "title" => entry.title = text.clone(),
                        "link" if !is_atom => entry.link = text.clone(),
                        "description" | "summary" | "content" => {
                            if entry.description.is_empty() {
                                entry.description = strip_html(&text);
                            }
                        }
                        "pubdate" | "published" | "updated" | "dc:date" | "date" => {
                            if entry.pub_date.is_empty() {
                                entry.pub_date = text.clone();
                            }
                        }
                        "author" | "name" => {
                            if entry.author.is_empty() {
                                entry.author = text.clone();
                            }
                        }
                        "guid" | "id" => entry.guid = text.clone(),
                        _ => {}
                    }
                } else if in_channel && stack.len() <= channel_depth + 1 {
                    match name.as_str() {
                        "title" => meta.title = text.clone(),
                        "link" if !is_atom => meta.link = text.clone(),
                        "description" | "subtitle" => {
                            if meta.description.is_empty() {
                                meta.description = strip_html(&text);
                            }
                        }
                        "language" => meta.language = text.clone(),
                        "pubdate" | "lastbuilddate" | "updated" => {
                            if meta.pub_date.is_empty() {
                                meta.pub_date = text.clone();
                            }
                        }
                        "generator" => meta.generator = text.clone(),
                        _ => {}
                    }
                }

                if name == "item" || name == "entry" {
                    if let Some(entry) = current.take() {
                        entries.push(entry);
                    }
                }
                stack.pop();
                text_buf.clear();
            }
            Ok(Event::Text(e)) => {
                if let Ok(s) = e.unescape() {
                    text_buf.push_str(&s);
                }
            }
            Ok(Event::CData(e)) => {
                if let Ok(s) = std::str::from_utf8(&e) {
                    text_buf.push_str(s);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("rss_tools: XML parse error: {e}")),
            _ => {}
        }
    }

    if meta.feed_type.is_empty() {
        meta.feed_type = "Unknown".to_string();
    }
    Ok((meta, entries))
}

fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
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
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn list_action(xml: &str, args: &Value) -> Result<String, String> {
    let (meta, entries) = parse_feed(xml)?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

    let mut out = format!(
        "Feed: {}\nType: {}  Entries: {}\n",
        meta.title,
        meta.feed_type,
        entries.len()
    );
    if !meta.link.is_empty() {
        out.push_str(&format!("URL:  {}\n", meta.link));
    }
    if !meta.description.is_empty() {
        let desc: String = meta.description.chars().take(120).collect();
        out.push_str(&format!("Desc: {desc}\n"));
    }
    out.push('\n');

    let show = entries.len().min(limit);
    out.push_str(&format!("Entries ({show}/{}):\n\n", entries.len()));
    for (i, e) in entries[..show].iter().enumerate() {
        out.push_str(&format!(
            "  [{:>2}] {}\n",
            i + 1,
            if e.title.is_empty() {
                "(no title)"
            } else {
                &e.title
            }
        ));
        if !e.pub_date.is_empty() {
            out.push_str(&format!("       Date: {}\n", e.pub_date));
        }
        if !e.author.is_empty() {
            out.push_str(&format!("       By:   {}\n", e.author));
        }
        if !e.link.is_empty() {
            out.push_str(&format!("       Link: {}\n", e.link));
        }
        if !e.description.is_empty() {
            let snippet: String = e.description.chars().take(160).collect();
            let snippet = if e.description.len() > 160 {
                format!("{snippet}…")
            } else {
                snippet
            };
            out.push_str(&format!("       Desc: {snippet}\n"));
        }
        out.push('\n');
    }
    if entries.len() > limit {
        out.push_str(&format!(
            "  ... ({} more entries, use 'limit' to increase)\n",
            entries.len() - limit
        ));
    }
    Ok(out)
}

fn info_action(xml: &str) -> Result<String, String> {
    let (meta, entries) = parse_feed(xml)?;
    let mut out = format!("Feed Information\n\n");
    out.push_str(&format!("  Type:        {}\n", meta.feed_type));
    out.push_str(&format!("  Title:       {}\n", meta.title));
    out.push_str(&format!("  Entries:     {}\n", entries.len()));
    if !meta.link.is_empty() {
        out.push_str(&format!("  Link:        {}\n", meta.link));
    }
    if !meta.description.is_empty() {
        out.push_str(&format!("  Description: {}\n", meta.description));
    }
    if !meta.language.is_empty() {
        out.push_str(&format!("  Language:    {}\n", meta.language));
    }
    if !meta.pub_date.is_empty() {
        out.push_str(&format!("  Last Updated:{}\n", meta.pub_date));
    }
    if !meta.generator.is_empty() {
        out.push_str(&format!("  Generator:   {}\n", meta.generator));
    }
    let authors: std::collections::HashSet<&str> = entries
        .iter()
        .filter(|e| !e.author.is_empty())
        .map(|e| e.author.as_str())
        .collect();
    if !authors.is_empty() {
        let mut v: Vec<&str> = authors.into_iter().collect();
        v.sort();
        out.push_str(&format!("  Authors:     {}\n", v.join(", ")));
    }
    Ok(out)
}

fn links_action(xml: &str) -> Result<String, String> {
    let (meta, entries) = parse_feed(xml)?;
    let mut out = format!("Links from: {}\n\n", meta.title);
    if !meta.link.is_empty() {
        out.push_str(&format!("  [feed]  {}\n\n", meta.link));
    }
    for (i, e) in entries.iter().enumerate() {
        if !e.link.is_empty() {
            let title = if e.title.is_empty() {
                format!("entry {}", i + 1)
            } else {
                let t: String = e.title.chars().take(60).collect();
                if e.title.len() > 60 {
                    format!("{t}…")
                } else {
                    t
                }
            };
            out.push_str(&format!("  {title}\n    {}\n\n", e.link));
        }
    }
    Ok(out)
}

fn search_action(xml: &str, args: &Value) -> Result<String, String> {
    let query = args
        .get("query")
        .or_else(|| args.get("q"))
        .and_then(|v| v.as_str())
        .ok_or("rss_tools search: 'query' or 'q' required")?
        .to_lowercase();

    let (meta, entries) = parse_feed(xml)?;
    let matches: Vec<&FeedEntry> = entries
        .iter()
        .filter(|e| {
            e.title.to_lowercase().contains(&query)
                || e.description.to_lowercase().contains(&query)
                || e.author.to_lowercase().contains(&query)
        })
        .collect();

    let mut out = format!(
        "Search results for '{query}' in: {}\n{} of {} entries match\n\n",
        meta.title,
        matches.len(),
        entries.len()
    );
    for (i, e) in matches.iter().enumerate() {
        out.push_str(&format!("  [{}] {}\n", i + 1, e.title));
        if !e.pub_date.is_empty() {
            out.push_str(&format!("      Date: {}\n", e.pub_date));
        }
        if !e.link.is_empty() {
            out.push_str(&format!("      Link: {}\n", e.link));
        }
        out.push('\n');
    }
    if matches.is_empty() {
        out.push_str("  No matching entries found.\n");
    }
    Ok(out)
}
