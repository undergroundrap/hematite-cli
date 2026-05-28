use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("query").is_some() || args.get("q").is_some() {
        "search".to_string()
    } else {
        "parse".to_string()
    };
    match action.as_str() {
        "parse" => parse_action(args),
        "search" => search_action(args),
        "stats" => stats_action(args),
        "list" => list_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: parse, search, stats, list",
            action
        )),
    }
}

fn get_xml(args: &Value) -> Result<String, String> {
    args.get("xml")
        .or_else(|| args.get("text"))
        .or_else(|| args.get("sitemap"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'xml' — pass the sitemap XML content as a string".to_string())
}

#[derive(Debug, Clone)]
struct SitemapUrl {
    loc: String,
    lastmod: Option<String>,
    changefreq: Option<String>,
    priority: Option<String>,
}

#[derive(Debug, Clone)]
struct SitemapIndex {
    loc: String,
    lastmod: Option<String>,
}

#[derive(Debug)]
enum SitemapContent {
    UrlSet(Vec<SitemapUrl>),
    Index(Vec<SitemapIndex>),
    Unknown,
}

fn parse_sitemap(xml: &str) -> Result<SitemapContent, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf: Vec<u8> = Vec::new();

    let mut urls: Vec<SitemapUrl> = Vec::new();
    let mut indexes: Vec<SitemapIndex> = Vec::new();
    let mut is_index = false;
    let mut detected = false;

    let mut current_loc = String::new();
    let mut current_lastmod: Option<String> = None;
    let mut current_changefreq: Option<String> = None;
    let mut current_priority: Option<String> = None;
    let mut in_url = false;
    let mut in_sitemap = false;
    let mut current_tag = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                current_tag = tag.clone();
                match tag.as_str() {
                    "urlset" => {
                        is_index = false;
                        detected = true;
                    }
                    "sitemapindex" => {
                        is_index = true;
                        detected = true;
                    }
                    "url" => {
                        in_url = true;
                        current_loc.clear();
                        current_lastmod = None;
                        current_changefreq = None;
                        current_priority = None;
                    }
                    "sitemap" if is_index => {
                        in_sitemap = true;
                        current_loc.clear();
                        current_lastmod = None;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().trim().to_string();
                if text.is_empty() {
                    continue;
                }
                match current_tag.as_str() {
                    "loc" => current_loc = text,
                    "lastmod" => current_lastmod = Some(text),
                    "changefreq" => current_changefreq = Some(text),
                    "priority" => current_priority = Some(text),
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                match tag.as_str() {
                    "url" if in_url => {
                        if !current_loc.is_empty() {
                            urls.push(SitemapUrl {
                                loc: current_loc.clone(),
                                lastmod: current_lastmod.clone(),
                                changefreq: current_changefreq.clone(),
                                priority: current_priority.clone(),
                            });
                        }
                        in_url = false;
                    }
                    "sitemap" if in_sitemap => {
                        if !current_loc.is_empty() {
                            indexes.push(SitemapIndex {
                                loc: current_loc.clone(),
                                lastmod: current_lastmod.clone(),
                            });
                        }
                        in_sitemap = false;
                    }
                    _ => {}
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    if !detected {
        return Ok(SitemapContent::Unknown);
    }
    if is_index {
        Ok(SitemapContent::Index(indexes))
    } else {
        Ok(SitemapContent::UrlSet(urls))
    }
}

fn parse_action(args: &Value) -> Result<String, String> {
    let xml = get_xml(args)?;
    let max = args.get("max").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let content = parse_sitemap(&xml)?;

    match content {
        SitemapContent::UrlSet(urls) => {
            let mut out = format!(
                "Sitemap (urlset)  [{} URLs]\n{}\n\n",
                urls.len(),
                "=".repeat(44)
            );
            for (i, url) in urls.iter().enumerate().take(max) {
                out += &format!("{}. {}\n", i + 1, url.loc);
                if let Some(lm) = &url.lastmod {
                    out += &format!("   lastmod:    {}\n", lm);
                }
                if let Some(cf) = &url.changefreq {
                    out += &format!("   changefreq: {}\n", cf);
                }
                if let Some(p) = &url.priority {
                    out += &format!("   priority:   {}\n", p);
                }
            }
            if urls.len() > max {
                out += &format!("\n... {} more URLs (pass 'max' to increase limit)\n", urls.len() - max);
            }
            Ok(out)
        }
        SitemapContent::Index(indexes) => {
            let mut out = format!(
                "Sitemap Index  [{} sitemaps]\n{}\n\n",
                indexes.len(),
                "=".repeat(44)
            );
            for (i, idx) in indexes.iter().enumerate().take(max) {
                out += &format!("{}. {}\n", i + 1, idx.loc);
                if let Some(lm) = &idx.lastmod {
                    out += &format!("   lastmod: {}\n", lm);
                }
            }
            if indexes.len() > max {
                out += &format!("\n... {} more sitemaps\n", indexes.len() - max);
            }
            Ok(out)
        }
        SitemapContent::Unknown => Ok(
            "Could not detect sitemap format. Ensure the XML has a <urlset> or <sitemapindex> root element.\n"
                .to_string(),
        ),
    }
}

fn search_action(args: &Value) -> Result<String, String> {
    let xml = get_xml(args)?;
    let query = args
        .get("query")
        .or_else(|| args.get("q"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'query' — pass a URL fragment to search for")?;
    let q = query.to_lowercase();

    let content = parse_sitemap(&xml)?;
    let urls = match content {
        SitemapContent::UrlSet(u) => u,
        SitemapContent::Index(_) => {
            return Ok(format!(
                "Search on sitemap index not supported — this is an index of {} child sitemaps.\n",
                query
            ))
        }
        SitemapContent::Unknown => return Err("Could not parse sitemap XML".to_string()),
    };

    let matches: Vec<&SitemapUrl> = urls
        .iter()
        .filter(|u| u.loc.to_lowercase().contains(&q))
        .collect();
    let mut out = format!(
        "Sitemap Search: '{}' ({} of {} URLs matched)\n{}\n\n",
        query,
        matches.len(),
        urls.len(),
        "=".repeat(44)
    );
    for url in &matches {
        out += &format!("{}\n", url.loc);
        if let Some(lm) = &url.lastmod {
            out += &format!("  lastmod: {}\n", lm);
        }
    }
    if matches.is_empty() {
        out += "No matching URLs found.\n";
    }
    Ok(out)
}

fn stats_action(args: &Value) -> Result<String, String> {
    let xml = get_xml(args)?;
    let content = parse_sitemap(&xml)?;

    let urls = match content {
        SitemapContent::UrlSet(u) => u,
        SitemapContent::Index(idx) => {
            let mut out = format!("Sitemap Index Stats\n{}\n\n", "=".repeat(44));
            out += &format!("Total child sitemaps: {}\n", idx.len());
            let with_lastmod = idx.iter().filter(|i| i.lastmod.is_some()).count();
            out += &format!("With lastmod:         {}\n", with_lastmod);
            return Ok(out);
        }
        SitemapContent::Unknown => return Err("Could not parse sitemap XML".to_string()),
    };

    let with_lastmod = urls.iter().filter(|u| u.lastmod.is_some()).count();
    let with_changefreq = urls.iter().filter(|u| u.changefreq.is_some()).count();
    let with_priority = urls.iter().filter(|u| u.priority.is_some()).count();

    let mut changefreq_counts: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for url in &urls {
        if let Some(cf) = &url.changefreq {
            *changefreq_counts.entry(cf.as_str()).or_insert(0) += 1;
        }
    }
    let mut priority_counts: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for url in &urls {
        if let Some(p) = &url.priority {
            *priority_counts.entry(p.as_str()).or_insert(0) += 1;
        }
    }

    let mut out = format!("Sitemap Stats\n{}\n\n", "=".repeat(44));
    out += &format!("Total URLs:        {}\n", urls.len());
    out += &format!(
        "With lastmod:      {} ({:.0}%)\n",
        with_lastmod,
        with_lastmod as f64 / urls.len().max(1) as f64 * 100.0
    );
    out += &format!(
        "With changefreq:   {} ({:.0}%)\n",
        with_changefreq,
        with_changefreq as f64 / urls.len().max(1) as f64 * 100.0
    );
    out += &format!(
        "With priority:     {} ({:.0}%)\n",
        with_priority,
        with_priority as f64 / urls.len().max(1) as f64 * 100.0
    );

    if !changefreq_counts.is_empty() {
        let mut cf_sorted: Vec<_> = changefreq_counts.iter().collect();
        cf_sorted.sort_by(|a, b| b.1.cmp(a.1));
        out += "\nChange frequency distribution:\n";
        for (cf, count) in cf_sorted {
            out += &format!("  {:<12} {}\n", cf, count);
        }
    }
    if !priority_counts.is_empty() {
        let mut p_sorted: Vec<_> = priority_counts.iter().collect();
        p_sorted.sort_by(|a, b| a.0.cmp(b.0));
        out += "\nPriority distribution:\n";
        for (p, count) in p_sorted {
            out += &format!("  {:<8} {}\n", p, count);
        }
    }
    Ok(out)
}

fn list_action(args: &Value) -> Result<String, String> {
    let xml = get_xml(args)?;
    let content = parse_sitemap(&xml)?;
    let filter = args
        .get("filter")
        .or_else(|| args.get("prefix"))
        .and_then(|v| v.as_str());

    let urls = match content {
        SitemapContent::UrlSet(u) => u,
        SitemapContent::Index(idx) => {
            let mut out = format!(
                "Sitemap Index — {} child sitemaps\n{}\n\n",
                idx.len(),
                "=".repeat(44)
            );
            for i in &idx {
                out += &format!("{}\n", i.loc);
            }
            return Ok(out);
        }
        SitemapContent::Unknown => return Err("Could not parse sitemap XML".to_string()),
    };

    let filtered: Vec<&SitemapUrl> = if let Some(f) = filter {
        urls.iter().filter(|u| u.loc.contains(f)).collect()
    } else {
        urls.iter().collect()
    };

    let mut out = if let Some(f) = filter {
        format!(
            "Sitemap URLs matching '{}' ({} of {})\n{}\n\n",
            f,
            filtered.len(),
            urls.len(),
            "=".repeat(44)
        )
    } else {
        format!(
            "Sitemap URLs ({} total)\n{}\n\n",
            filtered.len(),
            "=".repeat(44)
        )
    };
    for url in &filtered {
        out += &format!("{}\n", url.loc);
    }
    Ok(out)
}
