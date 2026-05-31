use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("summary");
    let har = get_har(args)?;
    match action {
        "summary" | "info" => action_summary(&har),
        "entries" | "list" => action_entries(&har, args),
        "slowest" => action_slowest(&har, args),
        "errors" => action_errors(&har),
        "domains" => action_domains(&har),
        "search" => {
            let q = args
                .get("query")
                .or_else(|| args.get("q"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_lowercase();
            action_search(&har, &q)
        }
        other => Err(format!(
            "har_tools: unknown action '{other}'. Valid: summary, entries, slowest, errors, domains, search"
        )),
    }
}

// ── input ─────────────────────────────────────────────────────────────────────

fn get_har(args: &Value) -> Result<Value, String> {
    for key in &["har", "json", "text", "content", "input"] {
        if let Some(v) = args.get(key) {
            if v.is_object() {
                return Ok(v.clone());
            }
            if let Some(s) = v.as_str() {
                return serde_json::from_str(s)
                    .map_err(|e| format!("failed to parse HAR JSON: {}", e));
            }
        }
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("cannot read '{}': {}", path, e))?;
        return serde_json::from_str(&content)
            .map_err(|e| format!("failed to parse HAR JSON in '{}': {}", path, e));
    }
    Err("har_tools: pass 'har' with a parsed HAR object, 'json'/'text' with a HAR JSON string, or 'file' with a path to a .har file".into())
}

fn entries(har: &Value) -> Vec<&Value> {
    har.pointer("/log/entries")
        .and_then(|e| e.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}

fn entry_url(e: &Value) -> &str {
    e.pointer("/request/url")
        .and_then(|v| v.as_str())
        .unwrap_or("-")
}

fn entry_method(e: &Value) -> &str {
    e.pointer("/request/method")
        .and_then(|v| v.as_str())
        .unwrap_or("-")
}

fn entry_status(e: &Value) -> u64 {
    e.pointer("/response/status")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

fn entry_time(e: &Value) -> f64 {
    e.get("time").and_then(|v| v.as_f64()).unwrap_or(0.0)
}

fn entry_size(e: &Value) -> i64 {
    e.pointer("/response/content/size")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

fn domain_of(url: &str) -> &str {
    let s = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let end = s.find('/').unwrap_or(s.len());
    &s[..end]
}

fn fmt_ms(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.2}s", ms / 1000.0)
    } else {
        format!("{:.0}ms", ms)
    }
}

fn fmt_bytes(b: i64) -> String {
    if b < 0 {
        return "-".into();
    }
    if b >= 1_048_576 {
        format!("{:.1} MB", b as f64 / 1_048_576.0)
    } else if b >= 1024 {
        format!("{:.1} KB", b as f64 / 1024.0)
    } else {
        format!("{} B", b)
    }
}

fn short_url(url: &str, max: usize) -> String {
    if url.len() <= max {
        url.to_string()
    } else {
        format!("{}…", &url[..max - 1])
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_summary(har: &Value) -> Result<String, String> {
    let log = har
        .get("log")
        .ok_or("HAR missing 'log' key — is this a valid .har file?")?;
    let version = log.get("version").and_then(|v| v.as_str()).unwrap_or("?");
    let creator = log
        .pointer("/creator/name")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let entries = entries(har);
    let n = entries.len();

    let total_time: f64 = entries.iter().map(|e| entry_time(e)).sum();
    let total_size: i64 = entries.iter().map(|e| entry_size(e)).sum();

    let mut status_counts = std::collections::HashMap::new();
    let mut mime_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut max_time = 0.0f64;
    let mut slowest_url = "";

    for e in &entries {
        let s = entry_status(e);
        let bucket = if s == 0 {
            "0xx".to_string()
        } else {
            format!("{}xx", s / 100)
        };
        *status_counts.entry(bucket).or_insert(0usize) += 1;

        if let Some(mt) = e
            .pointer("/response/content/mimeType")
            .and_then(|v| v.as_str())
        {
            let key = mt.split(';').next().unwrap_or(mt).trim().to_string();
            *mime_counts.entry(key).or_insert(0) += 1;
        }

        let t = entry_time(e);
        if t > max_time {
            max_time = t;
            slowest_url = entry_url(e);
        }
    }

    let errors = entries
        .iter()
        .filter(|e| {
            let s = entry_status(e);
            s >= 400 || s == 0
        })
        .count();

    let mut domains: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for e in &entries {
        domains.insert(domain_of(entry_url(e)));
    }

    let mut out = format!("HAR Summary\n{}\n", "─".repeat(50));
    out.push_str(&format!("HAR Version:   {}\n", version));
    out.push_str(&format!("Creator:       {}\n", creator));
    out.push('\n');
    out.push_str(&format!("Total entries: {}\n", n));
    out.push_str(&format!("Unique domains:{}\n", domains.len()));
    out.push_str(&format!(
        "Error entries: {} ({})\n",
        errors,
        if n > 0 {
            format!("{:.0}%", errors as f64 / n as f64 * 100.0)
        } else {
            "0%".to_string()
        }
    ));
    out.push_str(&format!("Total time:    {}\n", fmt_ms(total_time)));
    out.push_str(&format!("Total size:    {}\n", fmt_bytes(total_size)));
    out.push_str(&format!(
        "Slowest req:   {} ({})\n",
        fmt_ms(max_time),
        short_url(slowest_url, 60)
    ));

    if !status_counts.is_empty() {
        out.push_str("\nStatus distribution:\n");
        let mut counts: Vec<_> = status_counts.into_iter().collect();
        counts.sort_by_key(|(k, _)| k.clone());
        for (bucket, count) in counts {
            out.push_str(&format!("  {:4}  {}\n", bucket, count));
        }
    }

    if !mime_counts.is_empty() {
        out.push_str("\nContent types:\n");
        let mut mt_vec: Vec<_> = mime_counts.into_iter().collect();
        mt_vec.sort_by(|a, b| b.1.cmp(&a.1));
        for (mt, count) in mt_vec.iter().take(8) {
            out.push_str(&format!("  {:4}  {}\n", count, mt));
        }
    }

    Ok(out)
}

fn action_entries(har: &Value, args: &Value) -> Result<String, String> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(25) as usize;
    let entries = entries(har);
    if entries.is_empty() {
        return Ok("No entries found.".into());
    }

    let mut out = format!(
        "{:<6} {:<7} {:<8} {:<8} {}\n",
        "Status", "Method", "Time", "Size", "URL"
    );
    out.push_str(&"─".repeat(80));
    out.push('\n');

    for e in entries.iter().take(limit) {
        let status = entry_status(e);
        let method = entry_method(e);
        let time = entry_time(e);
        let size = entry_size(e);
        let url = short_url(entry_url(e), 50);
        out.push_str(&format!(
            "{:<6} {:<7} {:<8} {:<8} {}\n",
            if status == 0 {
                "ERR".to_string()
            } else {
                status.to_string()
            },
            method,
            fmt_ms(time),
            fmt_bytes(size),
            url
        ));
    }

    if entries.len() > limit {
        out.push_str(&format!(
            "… {} more entries (pass 'limit' to show more)\n",
            entries.len() - limit
        ));
    }
    Ok(out)
}

fn action_slowest(har: &Value, args: &Value) -> Result<String, String> {
    let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let mut entries: Vec<&Value> = entries(har);
    entries.sort_by(|a, b| {
        entry_time(b)
            .partial_cmp(&entry_time(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut out = format!("Top {} Slowest Requests\n{}\n", n, "─".repeat(60));
    for (i, e) in entries.iter().take(n).enumerate() {
        out.push_str(&format!(
            "\n[{}] {} ({} {} {})\n",
            i + 1,
            fmt_ms(entry_time(e)),
            entry_status(e),
            entry_method(e),
            short_url(entry_url(e), 60)
        ));
        if let Some(timings) = e.get("timings") {
            let dns = timings.get("dns").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let conn = timings
                .get("connect")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let ssl = timings.get("ssl").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let send = timings.get("send").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let wait = timings.get("wait").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let recv = timings
                .get("receive")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if dns > 0.0 {
                out.push_str(&format!("    DNS:     {}\n", fmt_ms(dns)));
            }
            if conn > 0.0 {
                out.push_str(&format!("    Connect: {}\n", fmt_ms(conn)));
            }
            if ssl > 0.0 {
                out.push_str(&format!("    SSL:     {}\n", fmt_ms(ssl)));
            }
            if send > 0.0 {
                out.push_str(&format!("    Send:    {}\n", fmt_ms(send)));
            }
            if wait > 0.0 {
                out.push_str(&format!("    Wait:    {}\n", fmt_ms(wait)));
            }
            if recv > 0.0 {
                out.push_str(&format!("    Receive: {}\n", fmt_ms(recv)));
            }
        }
    }
    Ok(out)
}

fn action_errors(har: &Value) -> Result<String, String> {
    let entries: Vec<&Value> = entries(har)
        .into_iter()
        .filter(|e| {
            let s = entry_status(e);
            s >= 400 || s == 0
        })
        .collect();

    if entries.is_empty() {
        return Ok("No error responses found (4xx/5xx/network errors).".into());
    }

    let mut out = format!("{} error(s)\n{}\n", entries.len(), "─".repeat(60));
    for (i, e) in entries.iter().enumerate() {
        let status = entry_status(e);
        let label = match status {
            0 => "NET ERROR",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            429 => "Rate Limited",
            500 => "Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "",
        };
        out.push_str(&format!(
            "\n[{}] {} {}\n    {} {}\n",
            i + 1,
            if status == 0 {
                "ERR".to_string()
            } else {
                status.to_string()
            },
            label,
            entry_method(e),
            short_url(entry_url(e), 70)
        ));
        out.push_str(&format!(
            "    Time: {}  Size: {}\n",
            fmt_ms(entry_time(e)),
            fmt_bytes(entry_size(e))
        ));
    }
    Ok(out)
}

fn action_domains(har: &Value) -> Result<String, String> {
    let entries = entries(har);
    let mut domain_stats: std::collections::HashMap<String, (usize, f64, i64)> =
        std::collections::HashMap::new();

    for e in &entries {
        let d = domain_of(entry_url(e)).to_string();
        let entry = domain_stats.entry(d).or_insert((0, 0.0, 0));
        entry.0 += 1;
        entry.1 += entry_time(e);
        let sz = entry_size(e);
        if sz >= 0 {
            entry.2 += sz;
        }
    }

    let mut sorted: Vec<_> = domain_stats.into_iter().collect();
    sorted.sort_by(|a, b| {
        b.1 .1
            .partial_cmp(&a.1 .1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut out = format!("{} domain(s)\n{}\n", sorted.len(), "─".repeat(70));
    out.push_str(&format!(
        "{:<5} {:<10} {:<12} {}\n",
        "Reqs", "Total ms", "Size", "Domain"
    ));
    out.push_str(&"─".repeat(70));
    out.push('\n');
    for (domain, (count, total_time, total_size)) in &sorted {
        out.push_str(&format!(
            "{:<5} {:<10} {:<12} {}\n",
            count,
            fmt_ms(*total_time),
            fmt_bytes(*total_size),
            domain
        ));
    }
    Ok(out)
}

fn action_search(har: &Value, query: &str) -> Result<String, String> {
    if query.is_empty() {
        return Err("har_tools search: pass 'query' or 'q' with a search term".into());
    }
    let entries: Vec<&Value> = entries(har)
        .into_iter()
        .filter(|e| entry_url(e).to_lowercase().contains(query))
        .collect();

    if entries.is_empty() {
        return Ok(format!("No entries matching '{}' found.", query));
    }

    let mut out = format!(
        "{} match(es) for '{}'\n{}\n",
        entries.len(),
        query,
        "─".repeat(60)
    );
    for (i, e) in entries.iter().enumerate() {
        out.push_str(&format!(
            "\n[{}] {} {} {}\n    Time: {}  Size: {}\n",
            i + 1,
            entry_status(e),
            entry_method(e),
            entry_url(e),
            fmt_ms(entry_time(e)),
            fmt_bytes(entry_size(e))
        ));
    }
    Ok(out)
}
