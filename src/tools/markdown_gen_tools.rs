use serde_json::{json, Value};

pub fn markdown_gen_tools_schema() -> Value {
    json!({
        "name": "markdown_gen_tools",
        "description": "Generate Markdown constructs programmatically without external utilities — the complement to markdown_tools which reads/parses Markdown. Actions: table (generate a GitHub-flavored Markdown table from headers and rows), badge (generate a shields.io-style Markdown badge), toc (generate a table of contents from a heading list), admonition (GitHub-style > [!NOTE] / > [!WARNING] / > [!TIP] / > [!IMPORTANT] / > [!CAUTION] callout blocks), link (format Markdown links, image links, and reference-style links), doc (generate a complete Markdown document from structured sections).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["table", "badge", "toc", "admonition", "link", "doc"],
                    "description": "Generation action to perform (default: table)"
                },
                "headers": {
                    "type": "array",
                    "description": "Column headers for 'table' action (string array)",
                    "items": { "type": "string" }
                },
                "rows": {
                    "type": "array",
                    "description": "Rows for 'table' action — array of string arrays",
                    "items": { "type": "array" }
                },
                "align": {
                    "type": "array",
                    "description": "Column alignments for 'table': 'left', 'right', 'center' per column (default: all left)",
                    "items": { "type": "string" }
                },
                "label": {
                    "type": "string",
                    "description": "Badge label (left side) for 'badge' action, or admonition body text for 'admonition'"
                },
                "message": {
                    "type": "string",
                    "description": "Badge message (right side) for 'badge' action"
                },
                "color": {
                    "type": "string",
                    "description": "Badge color for 'badge': brightgreen, green, yellow, orange, red, blue, lightgrey, or hex"
                },
                "url": {
                    "type": "string",
                    "description": "URL to link to for 'badge' or 'link' action"
                },
                "headings": {
                    "type": "array",
                    "description": "Heading strings for 'toc' action — plain text or '# Heading' format",
                    "items": { "type": "string" }
                },
                "kind": {
                    "type": "string",
                    "description": "'admonition' type: NOTE, TIP, IMPORTANT, WARNING, CAUTION"
                },
                "text": {
                    "type": "string",
                    "description": "Link display text for 'link' action, or title for 'admonition'"
                },
                "image": {
                    "type": "string",
                    "description": "Image URL for 'link' action (produces image link)"
                },
                "alt": {
                    "type": "string",
                    "description": "Alt text for image links in 'link' action"
                },
                "style": {
                    "type": "string",
                    "description": "'link' style: inline (default), reference, image, image_link"
                },
                "ref_id": {
                    "type": "string",
                    "description": "Reference ID for 'reference' style links"
                },
                "title": {
                    "type": "string",
                    "description": "Hover title for links (optional), or document title for 'doc'"
                },
                "sections": {
                    "type": "array",
                    "description": "Sections for 'doc' action — array of {heading, level?, body, code?, lang?} objects"
                }
            },
            "required": []
        }
    })
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn heading_to_anchor(h: &str) -> String {
    // Strip leading #s and spaces, then generate GitHub-style anchor
    let text = h.trim_start_matches('#').trim();
    let anchor = text
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() {
                Some(c.to_lowercase().next().unwrap_or(c))
            } else if c == ' ' || c == '-' {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>();
    // Remove consecutive dashes
    let mut result = String::new();
    let mut last_dash = false;
    for ch in anchor.chars() {
        if ch == '-' {
            if !last_dash {
                result.push(ch);
            }
            last_dash = true;
        } else {
            last_dash = false;
            result.push(ch);
        }
    }
    result.trim_matches('-').to_string()
}

fn heading_level(h: &str) -> usize {
    h.chars().take_while(|&c| c == '#').count().max(1)
}

fn heading_text(h: &str) -> &str {
    h.trim_start_matches('#').trim()
}

fn col_align_sep(align: &str) -> &'static str {
    match align.to_lowercase().as_str() {
        "right" | "r" => "---:",
        "center" | "c" => ":---:",
        _ => "---",
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_table(args: &Value) -> Result<String, String> {
    let headers: Vec<String> = args
        .get("headers")
        .and_then(|v| v.as_array())
        .ok_or("'headers' array is required")?
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();

    if headers.is_empty() {
        return Err("'headers' must not be empty".to_string());
    }

    let rows: Vec<Vec<String>> = args
        .get("rows")
        .and_then(|v| v.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|cell| match cell {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect()
        })
        .collect();

    let aligns: Vec<String> = args
        .get("align")
        .and_then(|v| v.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .map(|v| v.as_str().unwrap_or("left").to_string())
        .collect();

    // Compute column widths
    let n_cols = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in &rows {
        for (i, cell) in row.iter().enumerate().take(n_cols) {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }
    // Minimum width for separator
    for w in widths.iter_mut() {
        *w = (*w).max(3);
    }

    let pad = |s: &str, width: usize, align: &str| -> String {
        let s_len = s.len();
        if s_len >= width {
            return s.to_string();
        }
        let pad = width - s_len;
        match align.to_lowercase().as_str() {
            "right" | "r" => format!("{}{}", " ".repeat(pad), s),
            "center" | "c" => {
                let left = pad / 2;
                let right = pad - left;
                format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
            }
            _ => format!("{}{}", s, " ".repeat(pad)),
        }
    };

    let header_align = |i: usize| aligns.get(i).map(|s| s.as_str()).unwrap_or("left");

    // Header row
    let header_row = headers
        .iter()
        .enumerate()
        .map(|(i, h)| pad(h, widths[i], header_align(i)))
        .collect::<Vec<_>>()
        .join(" | ");
    let mut out = format!("| {header_row} |\n");

    // Separator row
    let sep_row = (0..n_cols)
        .map(|i| {
            let align = header_align(i);
            let base = col_align_sep(align);
            let needed = widths[i];
            match align.to_lowercase().as_str() {
                "center" | "c" => {
                    let dashes = needed.saturating_sub(2);
                    format!(":{}{}", "-".repeat(dashes.max(1)), ":")
                }
                "right" | "r" => {
                    let dashes = needed.saturating_sub(1);
                    format!("{}{}", "-".repeat(dashes.max(3)), ":")
                }
                _ => {
                    let _ = base;
                    "-".repeat(needed.max(3))
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    out.push_str(&format!("| {sep_row} |\n"));

    // Data rows
    for row in &rows {
        let cells: Vec<String> = (0..n_cols)
            .map(|i| {
                let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                pad(cell, widths[i], header_align(i))
            })
            .collect();
        out.push_str(&format!("| {} |\n", cells.join(" | ")));
    }

    Ok(out)
}

fn action_badge(args: &Value) -> Result<String, String> {
    let label = args
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("build");
    let message = args
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("passing");
    let color = args
        .get("color")
        .and_then(|v| v.as_str())
        .unwrap_or("brightgreen");
    let url = args.get("url").and_then(|v| v.as_str());
    let alt = args.get("alt").and_then(|v| v.as_str()).unwrap_or(label);

    // URL-encode label and message for shields.io
    let encode = |s: &str| s.replace('-', "--").replace('_', "__").replace(' ', "_");
    let badge_url = format!(
        "https://img.shields.io/badge/{}-{}-{}",
        encode(label),
        encode(message),
        encode(color)
    );
    let img_md = format!("![{alt}]({badge_url})");

    let mut out = if let Some(link_url) = url {
        format!("[{img_md}]({link_url})\n")
    } else {
        format!("{img_md}\n")
    };

    out.push_str(&format!("\nBadge URL: {badge_url}\n"));
    if let Some(link_url) = url {
        out.push_str(&format!("Links to:  {link_url}\n"));
    }
    Ok(out)
}

fn action_toc(args: &Value) -> Result<String, String> {
    let headings: Vec<String> = args
        .get("headings")
        .and_then(|v| v.as_array())
        .ok_or("'headings' array is required")?
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();

    if headings.is_empty() {
        return Err("'headings' must not be empty".to_string());
    }

    let min_level = headings.iter().map(|h| heading_level(h)).min().unwrap_or(1);

    let mut out = "## Table of Contents\n\n".to_string();
    let mut counters: Vec<(usize, usize)> = Vec::new(); // (level, count)

    for h in &headings {
        let level = heading_level(h);
        let text = heading_text(h);
        let anchor = heading_to_anchor(h);

        // Track for numbering (optional, just indent for now)
        let indent_level = level.saturating_sub(min_level);
        let indent = "  ".repeat(indent_level);

        // Number tracking
        while counters.last().map(|(l, _)| *l).unwrap_or(0) > level {
            counters.pop();
        }
        if counters.last().map(|(l, _)| *l).unwrap_or(0) == level {
            counters.last_mut().map(|(_, c)| *c += 1);
        } else {
            counters.push((level, 1));
        }

        out.push_str(&format!("{indent}- [{text}](#{anchor})\n"));
    }
    Ok(out)
}

fn action_admonition(args: &Value) -> Result<String, String> {
    let kind = args
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("NOTE")
        .to_uppercase();
    let body = args
        .get("label")
        .or_else(|| args.get("text"))
        .or_else(|| args.get("body"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let valid = ["NOTE", "TIP", "IMPORTANT", "WARNING", "CAUTION"];
    if !valid.contains(&kind.as_str()) {
        return Err(format!(
            "Unknown admonition type '{kind}'. Valid: NOTE, TIP, IMPORTANT, WARNING, CAUTION"
        ));
    }

    let mut out = format!("> [!{kind}]\n");
    if !body.is_empty() {
        for line in body.lines() {
            out.push_str(&format!("> {line}\n"));
        }
    }
    out.push('\n');
    out.push_str(&format!(
        "<!-- GitHub renders >[!{kind}] callout blocks in supported contexts -->\n"
    ));
    Ok(out)
}

fn action_link(args: &Value) -> Result<String, String> {
    let style = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("inline");
    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("link");
    let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let title = args.get("title").and_then(|v| v.as_str());
    let image = args.get("image").and_then(|v| v.as_str());
    let alt = args.get("alt").and_then(|v| v.as_str()).unwrap_or(text);
    let ref_id = args.get("ref_id").and_then(|v| v.as_str()).unwrap_or("ref");

    let title_part = title.map(|t| format!(" \"{t}\"")).unwrap_or_default();

    let md = match style {
        "reference" => {
            format!("[{text}][{ref_id}]\n\n[{ref_id}]: {url}{title_part}")
        }
        "image" => {
            format!("![{alt}]({url}{title_part})")
        }
        "image_link" => {
            let img = if let Some(img_url) = image {
                format!("![{alt}]({img_url})")
            } else {
                format!("![{alt}]({url})")
            };
            if url.is_empty() {
                img
            } else {
                format!("[{img}]({url}{title_part})")
            }
        }
        _ => {
            // inline
            format!("[{text}]({url}{title_part})")
        }
    };

    let mut out = format!("{md}\n\nStyle: {style}\n");
    if style == "reference" {
        out.push_str("(Place the reference definition at the bottom of your document)\n");
    }
    Ok(out)
}

fn action_doc(args: &Value) -> Result<String, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Document");

    let sections = args
        .get("sections")
        .and_then(|v| v.as_array())
        .ok_or("'sections' array of {heading, body, level?, code?, lang?} is required")?;

    let mut out = format!("# {title}\n\n");

    for (i, section) in sections.iter().enumerate() {
        let default_heading = format!("Section {}", i + 1);
        let heading = section
            .get("heading")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_heading);
        let level = section
            .get("level")
            .and_then(|v| v.as_u64())
            .unwrap_or(2)
            .clamp(1, 6) as usize;
        let body = section.get("body").and_then(|v| v.as_str()).unwrap_or("");
        let code = section.get("code").and_then(|v| v.as_str());
        let lang = section.get("lang").and_then(|v| v.as_str()).unwrap_or("");

        out.push_str(&format!("{} {heading}\n\n", "#".repeat(level)));

        if !body.is_empty() {
            out.push_str(body);
            out.push_str("\n\n");
        }

        if let Some(code_str) = code {
            out.push_str(&format!("```{lang}\n{code_str}\n```\n\n"));
        }
    }

    Ok(out)
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("table");

    match action {
        "table" => action_table(args),
        "badge" => action_badge(args),
        "toc" => action_toc(args),
        "admonition" => action_admonition(args),
        "link" => action_link(args),
        "doc" => action_doc(args),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: table, badge, toc, admonition, link, doc"
        )),
    }
}
