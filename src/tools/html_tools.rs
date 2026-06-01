use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;

pub fn html_tools_schema() -> Value {
    json!({
        "name": "html_tools",
        "description": "Parse and analyze HTML documents: extract links, images, forms, scripts, and tables; check for common accessibility and SEO issues; strip to plain text. Pass 'html' for inline content or 'file' for a path.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "links", "images", "forms", "tables", "scripts", "validate", "text", "stats"],
                    "description": "parse (default — overview of all major elements), links (all hyperlinks with href and text), images (all img tags with src/alt/dimensions), forms (form elements with method, action, and input fields), tables (table structure and cell preview), scripts (script tags and inline/external), validate (accessibility and SEO checks), text (strip all HTML to plain text), stats (element counts, depth, file size)"
                },
                "html": {
                    "type": "string",
                    "description": "Inline HTML content to analyze"
                },
                "file": {
                    "type": "string",
                    "description": "Path to an HTML file"
                }
            },
            "required": []
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");

    let raw = if let Some(p) = args.get("file").and_then(|v| v.as_str()) {
        fs::read_to_string(p).map_err(|e| format!("Cannot read file: {e}"))?
    } else if let Some(h) = args
        .get("html")
        .or_else(|| args.get("text"))
        .and_then(|v| v.as_str())
    {
        h.to_string()
    } else {
        return Err("Pass 'html' or 'file'.".into());
    };

    match action {
        "links" => action_links(&raw),
        "images" => action_images(&raw),
        "forms" => action_forms(&raw),
        "tables" => action_tables(&raw),
        "scripts" => action_scripts(&raw),
        "validate" => action_validate(&raw),
        "text" => Ok(strip_html(&raw)),
        "stats" => action_stats(&raw),
        _ => action_parse(&raw),
    }
}

// ── Tokenizer ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum HtmlToken {
    Open(String, HashMap<String, String>, bool), // tag, attrs, self-closing
    Close(String),
    Text(String),
    Comment(String),
    Doctype(String),
}

fn tokenize(html: &str) -> Vec<HtmlToken> {
    let mut tokens = Vec::new();
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' {
            // find closing >
            if i + 1 < len && bytes[i + 1] == b'!' {
                // comment or doctype
                if i + 3 < len && &bytes[i + 1..i + 4] == b"!--" {
                    let end = html[i + 4..]
                        .find("-->")
                        .map(|x| i + 4 + x + 3)
                        .unwrap_or(len);
                    tokens.push(HtmlToken::Comment(
                        html[i + 4..end.saturating_sub(3)].to_string(),
                    ));
                    i = end;
                } else {
                    let end = html[i..].find('>').map(|x| i + x + 1).unwrap_or(len);
                    tokens.push(HtmlToken::Doctype(html[i..end].to_string()));
                    i = end;
                }
                continue;
            }
            let end = find_tag_end(html, i);
            let inner = &html[i + 1..end.saturating_sub(1)];
            let inner = inner.trim();
            if inner.starts_with('/') {
                let tag = inner[1..].trim().to_lowercase();
                let tag = tag.split_whitespace().next().unwrap_or("").to_string();
                tokens.push(HtmlToken::Close(tag));
            } else {
                let self_closing = inner.ends_with('/');
                let inner = if self_closing {
                    inner[..inner.len() - 1].trim()
                } else {
                    inner
                };
                let (tag, attrs) = parse_tag(inner);
                tokens.push(HtmlToken::Open(tag, attrs, self_closing));
            }
            i = end;
        } else {
            let end = html[i..].find('<').map(|x| i + x).unwrap_or(len);
            let text = &html[i..end];
            if !text.trim().is_empty() {
                tokens.push(HtmlToken::Text(text.to_string()));
            }
            i = end;
        }
    }
    tokens
}

fn find_tag_end(html: &str, start: usize) -> usize {
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = start + 1;
    let mut in_quote: Option<u8> = None;
    while i < len {
        let b = bytes[i];
        match in_quote {
            Some(q) if b == q => in_quote = None,
            Some(_) => {}
            None => {
                if b == b'"' || b == b'\'' {
                    in_quote = Some(b);
                } else if b == b'>' {
                    return i + 1;
                }
            }
        }
        i += 1;
    }
    len
}

fn parse_tag(s: &str) -> (String, HashMap<String, String>) {
    let mut iter = s.splitn(2, |c: char| c.is_whitespace());
    let tag = iter.next().unwrap_or("").to_lowercase();
    let rest = iter.next().unwrap_or("").trim();
    let attrs = parse_attrs(rest);
    (tag, attrs)
}

fn parse_attrs(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // skip whitespace
        while i < len && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n') {
            i += 1;
        }
        if i >= len {
            break;
        }
        // read key
        let key_start = i;
        while i < len && bytes[i] != b'=' && bytes[i] != b' ' && bytes[i] != b'\t' {
            i += 1;
        }
        let key = s[key_start..i].to_lowercase();
        if key.is_empty() {
            break;
        }
        if i < len && bytes[i] == b'=' {
            i += 1; // skip =
            let val = if i < len && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let q = bytes[i];
                i += 1;
                let vs = i;
                while i < len && bytes[i] != q {
                    i += 1;
                }
                let v = s[vs..i].to_string();
                if i < len {
                    i += 1;
                }
                v
            } else {
                let vs = i;
                while i < len && bytes[i] != b' ' && bytes[i] != b'\t' {
                    i += 1;
                }
                s[vs..i].to_string()
            };
            map.insert(key, val);
        } else {
            map.insert(key, String::new());
        }
    }
    map
}

// ── Strip HTML ─────────────────────────────────────────────────────────────────

fn strip_html(html: &str) -> String {
    let tokens = tokenize(html);
    let skip_tags = ["script", "style", "head"];
    let mut buf = String::new();
    let mut skip_depth = 0usize;
    for t in tokens {
        match t {
            HtmlToken::Open(ref tag, _, _) if skip_tags.contains(&tag.as_str()) => {
                skip_depth += 1;
            }
            HtmlToken::Close(ref tag) if skip_tags.contains(&tag.as_str()) => {
                if skip_depth > 0 {
                    skip_depth -= 1;
                }
            }
            HtmlToken::Text(ref text) if skip_depth == 0 => {
                buf.push_str(text);
            }
            HtmlToken::Open(ref tag, _, _)
                if skip_depth == 0
                    && matches!(
                        tag.as_str(),
                        "p" | "br"
                            | "div"
                            | "h1"
                            | "h2"
                            | "h3"
                            | "h4"
                            | "h5"
                            | "h6"
                            | "li"
                            | "tr"
                            | "article"
                            | "section"
                    ) =>
            {
                buf.push('\n');
            }
            _ => {}
        }
    }
    // decode common HTML entities
    let buf = buf
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");
    // normalize whitespace
    let lines: Vec<&str> = buf
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    lines.join("\n")
}

// ── parse (overview) ──────────────────────────────────────────────────────────

fn action_parse(html: &str) -> Result<String, String> {
    let tokens = tokenize(html);
    let mut out = String::from("HTML DOCUMENT OVERVIEW\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');

    // doctype
    let doctype = tokens.iter().find_map(|t| {
        if let HtmlToken::Doctype(d) = t {
            Some(d.clone())
        } else {
            None
        }
    });
    if let Some(dt) = doctype {
        let snip = dt.chars().take(60).collect::<String>();
        out.push_str(&format!("DOCTYPE        {snip}\n"));
    }

    // title
    let mut in_title = false;
    let mut title = String::new();
    for t in &tokens {
        match t {
            HtmlToken::Open(tag, _, _) if tag == "title" => in_title = true,
            HtmlToken::Close(tag) if tag == "title" => in_title = false,
            HtmlToken::Text(text) if in_title => title.push_str(text.trim()),
            _ => {}
        }
    }
    if !title.is_empty() {
        out.push_str(&format!("Title          {title}\n"));
    }

    // meta description
    let meta_desc = tokens.iter().find_map(|t| {
        if let HtmlToken::Open(tag, attrs, _) = t {
            if tag == "meta"
                && attrs.get("name").map(|s| s.to_lowercase()) == Some("description".into())
            {
                attrs.get("content").cloned()
            } else {
                None
            }
        } else {
            None
        }
    });
    if let Some(md) = meta_desc {
        let snip = md.chars().take(80).collect::<String>();
        out.push_str(&format!("Meta Desc      {snip}\n"));
    }

    // counts
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for t in &tokens {
        if let HtmlToken::Open(tag, _, _) = t {
            *tag_counts.entry(tag.clone()).or_insert(0) += 1;
        }
    }

    let link_count = tag_counts.get("a").copied().unwrap_or(0);
    let img_count = tag_counts.get("img").copied().unwrap_or(0);
    let form_count = tag_counts.get("form").copied().unwrap_or(0);
    let table_count = tag_counts.get("table").copied().unwrap_or(0);
    let script_count = tag_counts.get("script").copied().unwrap_or(0);
    let style_count = tag_counts.get("style").copied().unwrap_or(0);

    out.push_str(&format!(
        "\nElement Counts:\n  Links (a)    {link_count}\n  Images       {img_count}\n  Forms        {form_count}\n  Tables       {table_count}\n  Scripts      {script_count}\n  Style blocks {style_count}\n"
    ));

    // heading structure
    let headings: Vec<(String, String)> = {
        let mut list = Vec::new();
        let mut in_h: Option<String> = None;
        let mut h_text = String::new();
        for t in &tokens {
            match t {
                HtmlToken::Open(tag, _, _)
                    if matches!(tag.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") =>
                {
                    in_h = Some(tag.clone());
                    h_text.clear();
                }
                HtmlToken::Close(tag)
                    if matches!(tag.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") =>
                {
                    if let Some(h) = in_h.take() {
                        list.push((h, h_text.trim().to_string()));
                    }
                    h_text.clear();
                }
                HtmlToken::Text(text) if in_h.is_some() => h_text.push_str(text.trim()),
                _ => {}
            }
        }
        list
    };
    if !headings.is_empty() {
        out.push_str("\nHeading Structure:\n");
        for (level, text) in &headings {
            let indent = match level.as_str() {
                "h1" => 0,
                "h2" => 2,
                "h3" => 4,
                "h4" => 6,
                _ => 8,
            };
            let snip = text.chars().take(60).collect::<String>();
            out.push_str(&format!(
                "  {:<indent$}{} {snip}\n",
                "",
                level.to_uppercase()
            ));
        }
    }

    out.push_str(&format!("\nDocument size  {} bytes\n", html.len()));
    Ok(out)
}

// ── links ─────────────────────────────────────────────────────────────────────

fn action_links(html: &str) -> Result<String, String> {
    let tokens = tokenize(html);
    let mut links: Vec<(String, String, bool)> = Vec::new(); // href, text, nofollow
    let mut in_a = false;
    let mut a_text = String::new();
    let mut current_href = String::new();
    let mut current_nofollow = false;

    for t in &tokens {
        match t {
            HtmlToken::Open(tag, attrs, _) if tag == "a" => {
                in_a = true;
                a_text.clear();
                current_href = attrs.get("href").cloned().unwrap_or_default();
                current_nofollow = attrs
                    .get("rel")
                    .map(|r| r.contains("nofollow"))
                    .unwrap_or(false);
            }
            HtmlToken::Close(tag) if tag == "a" => {
                if in_a {
                    links.push((
                        current_href.clone(),
                        a_text.trim().to_string(),
                        current_nofollow,
                    ));
                    in_a = false;
                }
            }
            HtmlToken::Text(text) if in_a => a_text.push_str(text),
            _ => {}
        }
    }

    if links.is_empty() {
        return Ok("No links found.".into());
    }

    let mut out = format!("LINKS ({} total)\n", links.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut external = 0;
    let mut internal = 0;
    let mut anchors = 0;
    let mut mailto = 0;

    for (href, text, nofollow) in &links {
        let kind = if href.starts_with("http://") || href.starts_with("https://") {
            external += 1;
            "EXT"
        } else if href.starts_with("mailto:") {
            mailto += 1;
            "MAIL"
        } else if href.starts_with('#') {
            anchors += 1;
            "ANCH"
        } else {
            internal += 1;
            "INT"
        };
        let nf = if *nofollow { " [nofollow]" } else { "" };
        let t = if text.is_empty() {
            "(no text)"
        } else {
            text.as_str()
        };
        let t_snip = t.chars().take(30).collect::<String>();
        let h_snip = href.chars().take(50).collect::<String>();
        out.push_str(&format!("[{kind}]{nf} {t_snip:<30} → {h_snip}\n"));
    }

    out.push_str(&format!(
        "\nSummary: {external} external, {internal} internal, {anchors} anchors, {mailto} mailto\n"
    ));
    Ok(out)
}

// ── images ────────────────────────────────────────────────────────────────────

fn action_images(html: &str) -> Result<String, String> {
    let tokens = tokenize(html);
    let imgs: Vec<&HashMap<String, String>> = tokens
        .iter()
        .filter_map(|t| {
            if let HtmlToken::Open(tag, attrs, _) = t {
                if tag == "img" {
                    Some(attrs)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    if imgs.is_empty() {
        return Ok("No images found.".into());
    }

    let mut out = format!("IMAGES ({} total)\n", imgs.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut missing_alt = 0;
    for attrs in &imgs {
        let src = attrs.get("src").map(|s| s.as_str()).unwrap_or("(no src)");
        let alt = attrs.get("alt");
        let w = attrs.get("width").map(|s| s.as_str()).unwrap_or("?");
        let h = attrs.get("height").map(|s| s.as_str()).unwrap_or("?");
        let loading = attrs.get("loading").map(|s| s.as_str()).unwrap_or("");

        let alt_label = match alt {
            None => {
                missing_alt += 1;
                "[NO ALT]".to_string()
            }
            Some(a) if a.is_empty() => "(decorative)".to_string(),
            Some(a) => format!("\"{}\"", a.chars().take(30).collect::<String>()),
        };

        let dims = if w != "?" || h != "?" {
            format!(" {w}×{h}")
        } else {
            String::new()
        };
        let lazy = if loading == "lazy" { " [lazy]" } else { "" };
        let src_snip = src.chars().take(50).collect::<String>();
        out.push_str(&format!("{src_snip}{dims}{lazy}\n  alt: {alt_label}\n"));
    }

    if missing_alt > 0 {
        out.push_str(&format!(
            "\n⚠ {missing_alt} image(s) missing alt attribute (accessibility issue)\n"
        ));
    }
    Ok(out)
}

// ── forms ─────────────────────────────────────────────────────────────────────

fn action_forms(html: &str) -> Result<String, String> {
    let tokens = tokenize(html);
    let mut forms: Vec<(
        HashMap<String, String>,
        Vec<(String, HashMap<String, String>)>,
    )> = Vec::new();
    let mut in_form = false;
    let mut cur_form_attrs = HashMap::new();
    let mut cur_inputs: Vec<(String, HashMap<String, String>)> = Vec::new();

    for t in &tokens {
        match t {
            HtmlToken::Open(tag, attrs, _) if tag == "form" => {
                in_form = true;
                cur_form_attrs = attrs.clone();
                cur_inputs.clear();
            }
            HtmlToken::Close(tag) if tag == "form" => {
                forms.push((cur_form_attrs.clone(), cur_inputs.clone()));
                in_form = false;
                cur_inputs.clear();
            }
            HtmlToken::Open(tag, attrs, _)
                if in_form
                    && matches!(tag.as_str(), "input" | "select" | "textarea" | "button") =>
            {
                cur_inputs.push((tag.clone(), attrs.clone()));
            }
            _ => {}
        }
    }

    if forms.is_empty() {
        return Ok("No forms found.".into());
    }

    let mut out = format!("FORMS ({} total)\n", forms.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for (i, (attrs, inputs)) in forms.iter().enumerate() {
        let action = attrs.get("action").map(|s| s.as_str()).unwrap_or("(none)");
        let method = attrs
            .get("method")
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| "GET".into());
        let enc = attrs.get("enctype").map(|s| s.as_str()).unwrap_or("");

        out.push_str(&format!(
            "\nForm #{} — {method} → {action}{}\n",
            i + 1,
            if enc.is_empty() {
                String::new()
            } else {
                format!(" ({enc})")
            }
        ));
        out.push_str(&format!("  {} field(s):\n", inputs.len()));
        for (tag, ia) in inputs {
            let name = ia.get("name").map(|s| s.as_str()).unwrap_or("(unnamed)");
            let typ = ia.get("type").map(|s| s.as_str()).unwrap_or(tag.as_str());
            let required = if ia.contains_key("required") {
                " [required]"
            } else {
                ""
            };
            let placeholder = ia
                .get("placeholder")
                .map(|s| {
                    format!(
                        " placeholder=\"{}\"",
                        s.chars().take(20).collect::<String>()
                    )
                })
                .unwrap_or_default();
            out.push_str(&format!("  • {name} ({typ}){required}{placeholder}\n"));
        }
    }
    Ok(out)
}

// ── tables ────────────────────────────────────────────────────────────────────

fn action_tables(html: &str) -> Result<String, String> {
    let tokens = tokenize(html);
    let mut tables: Vec<Vec<Vec<String>>> = Vec::new();
    let mut in_table = false;
    let mut cur_rows: Vec<Vec<String>> = Vec::new();
    let mut cur_cells: Vec<String> = Vec::new();
    let mut in_cell = false;
    let mut cell_text = String::new();

    for t in &tokens {
        match t {
            HtmlToken::Open(tag, _, _) if tag == "table" => {
                in_table = true;
                cur_rows.clear();
            }
            HtmlToken::Close(tag) if tag == "table" => {
                tables.push(cur_rows.clone());
                in_table = false;
            }
            HtmlToken::Open(tag, _, _) if in_table && tag == "tr" => {
                cur_cells.clear();
            }
            HtmlToken::Close(tag) if in_table && tag == "tr" => {
                cur_rows.push(cur_cells.clone());
            }
            HtmlToken::Open(tag, _, _) if in_table && (tag == "td" || tag == "th") => {
                in_cell = true;
                cell_text.clear();
            }
            HtmlToken::Close(tag) if in_table && (tag == "td" || tag == "th") => {
                cur_cells.push(cell_text.trim().to_string());
                in_cell = false;
            }
            HtmlToken::Text(text) if in_cell => cell_text.push_str(text),
            _ => {}
        }
    }

    if tables.is_empty() {
        return Ok("No tables found.".into());
    }

    let mut out = format!("TABLES ({} total)\n", tables.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for (i, rows) in tables.iter().enumerate() {
        let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        out.push_str(&format!(
            "\nTable #{} — {} row(s) × {} column(s)\n",
            i + 1,
            rows.len(),
            col_count
        ));
        // show first 5 rows
        for (ri, row) in rows.iter().take(5).enumerate() {
            let cells = row
                .iter()
                .map(|c| {
                    let s = c.chars().take(15).collect::<String>();
                    format!("{s:<15}")
                })
                .collect::<Vec<_>>()
                .join(" | ");
            let label = if ri == 0 { "  header: " } else { "  row:    " };
            out.push_str(&format!("{label}{cells}\n"));
        }
        if rows.len() > 5 {
            out.push_str(&format!("  ... {} more rows\n", rows.len() - 5));
        }
    }
    Ok(out)
}

// ── scripts ───────────────────────────────────────────────────────────────────

fn action_scripts(html: &str) -> Result<String, String> {
    let tokens = tokenize(html);
    let mut scripts: Vec<(Option<String>, Option<String>, usize)> = Vec::new(); // src, type, inline_len
    let mut in_script = false;
    let mut script_src: Option<String> = None;
    let mut script_type: Option<String> = None;
    let mut inline_text = String::new();

    for t in &tokens {
        match t {
            HtmlToken::Open(tag, attrs, _) if tag == "script" => {
                in_script = true;
                inline_text.clear();
                script_src = attrs.get("src").cloned();
                script_type = attrs.get("type").cloned();
            }
            HtmlToken::Close(tag) if tag == "script" => {
                scripts.push((
                    script_src.clone(),
                    script_type.clone(),
                    inline_text.trim().len(),
                ));
                in_script = false;
            }
            HtmlToken::Text(text) if in_script => inline_text.push_str(text),
            _ => {}
        }
    }

    if scripts.is_empty() {
        return Ok("No script tags found.".into());
    }

    let mut out = format!("SCRIPTS ({} total)\n", scripts.len());
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut external = 0;
    let mut inline_count = 0;
    for (src, typ, inline_len) in &scripts {
        let t = typ.as_deref().unwrap_or("text/javascript");
        match src {
            Some(s) => {
                external += 1;
                let s_snip = s.chars().take(60).collect::<String>();
                out.push_str(&format!("[EXT] {s_snip}  type={t}\n"));
            }
            None if *inline_len > 0 => {
                inline_count += 1;
                out.push_str(&format!("[INLINE] {inline_len} chars  type={t}\n"));
            }
            None => {
                out.push_str(&format!("[EMPTY]  type={t}\n"));
            }
        }
    }
    out.push_str(&format!(
        "\nSummary: {external} external, {inline_count} inline\n"
    ));
    Ok(out)
}

// ── validate ──────────────────────────────────────────────────────────────────

fn action_validate(html: &str) -> Result<String, String> {
    let tokens = tokenize(html);
    let mut issues: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let has_doctype = tokens.iter().any(|t| matches!(t, HtmlToken::Doctype(_)));
    if !has_doctype {
        issues.push("Missing <!DOCTYPE html> declaration".into());
    }

    let has_html = tokens
        .iter()
        .any(|t| matches!(t, HtmlToken::Open(tag, _, _) if tag == "html"));
    if !has_html {
        issues.push("Missing <html> root element".into());
    }

    // lang attribute on html
    let html_lang = tokens.iter().find_map(|t| {
        if let HtmlToken::Open(tag, attrs, _) = t {
            if tag == "html" {
                Some(attrs.contains_key("lang"))
            } else {
                None
            }
        } else {
            None
        }
    });
    if html_lang == Some(false) || html_lang.is_none() {
        issues.push("Missing lang attribute on <html> element (accessibility)".into());
    }

    let has_head = tokens
        .iter()
        .any(|t| matches!(t, HtmlToken::Open(tag, _, _) if tag == "head"));
    if !has_head {
        warnings.push("No <head> element found".into());
    }

    // charset meta
    let has_charset = tokens.iter().any(|t| {
        if let HtmlToken::Open(tag, attrs, _) = t {
            tag == "meta"
                && (attrs.contains_key("charset")
                    || attrs
                        .get("http-equiv")
                        .map(|v| v.to_lowercase().contains("content-type"))
                        .unwrap_or(false))
        } else {
            false
        }
    });
    if !has_charset {
        issues.push("Missing charset meta tag (<meta charset=\"utf-8\">)".into());
    }

    // title
    let has_title = tokens
        .iter()
        .any(|t| matches!(t, HtmlToken::Open(tag, _, _) if tag == "title"));
    if !has_title {
        issues.push("Missing <title> element (SEO + accessibility)".into());
    }

    // viewport
    let has_viewport = tokens.iter().any(|t| {
        if let HtmlToken::Open(tag, attrs, _) = t {
            tag == "meta" && attrs.get("name").map(|v| v.to_lowercase()) == Some("viewport".into())
        } else {
            false
        }
    });
    if !has_viewport {
        warnings.push("Missing viewport meta tag (mobile responsiveness)".into());
    }

    // images without alt
    let imgs_no_alt: usize = tokens
        .iter()
        .filter(|t| {
            if let HtmlToken::Open(tag, attrs, _) = t {
                tag == "img" && !attrs.contains_key("alt")
            } else {
                false
            }
        })
        .count();
    if imgs_no_alt > 0 {
        issues.push(format!(
            "{imgs_no_alt} image(s) missing alt attribute (accessibility)"
        ));
    }

    // forms without labels
    let inputs_without_id: usize = tokens
        .iter()
        .filter(|t| {
            if let HtmlToken::Open(tag, attrs, _) = t {
                tag == "input"
                    && attrs
                        .get("type")
                        .map(|t| t != "hidden" && t != "submit" && t != "button")
                        .unwrap_or(true)
                    && !attrs.contains_key("id")
            } else {
                false
            }
        })
        .count();
    if inputs_without_id > 0 {
        warnings.push(format!(
            "{inputs_without_id} input(s) without id (may prevent label association)"
        ));
    }

    // multiple h1
    let h1_count = tokens
        .iter()
        .filter(|t| matches!(t, HtmlToken::Open(tag, _, _) if tag == "h1"))
        .count();
    if h1_count > 1 {
        warnings.push(format!(
            "Multiple <h1> tags ({h1_count}) — typically only one h1 per page (SEO)"
        ));
    }
    if h1_count == 0 {
        warnings.push("No <h1> heading found (SEO)".into());
    }

    let mut out = String::from("HTML VALIDATION\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');

    if issues.is_empty() && warnings.is_empty() {
        out.push_str("✓ No issues detected.\n");
    } else {
        if !issues.is_empty() {
            out.push_str(&format!("\nISSUES ({}):\n", issues.len()));
            for issue in &issues {
                out.push_str(&format!("  ✗ {issue}\n"));
            }
        }
        if !warnings.is_empty() {
            out.push_str(&format!("\nWARNINGS ({}):\n", warnings.len()));
            for w in &warnings {
                out.push_str(&format!("  ⚠ {w}\n"));
            }
        }
    }

    let verdict = if !issues.is_empty() {
        "NEEDS ATTENTION"
    } else if !warnings.is_empty() {
        "MINOR WARNINGS"
    } else {
        "VALID"
    };
    out.push_str(&format!("\nVerdict: {verdict}\n"));
    Ok(out)
}

// ── stats ─────────────────────────────────────────────────────────────────────

fn action_stats(html: &str) -> Result<String, String> {
    let tokens = tokenize(html);
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let mut open_stack: Vec<String> = Vec::new();
    let mut max_depth = 0usize;
    let mut comment_count = 0usize;
    let mut text_bytes = 0usize;

    for t in &tokens {
        match t {
            HtmlToken::Open(tag, _, self_close) => {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
                if !self_close {
                    open_stack.push(tag.clone());
                    max_depth = max_depth.max(open_stack.len());
                }
            }
            HtmlToken::Close(_) => {
                open_stack.pop();
            }
            HtmlToken::Comment(_) => comment_count += 1,
            HtmlToken::Text(t) => text_bytes += t.len(),
            _ => {}
        }
    }

    let unique_tags = tag_counts.len();
    let total_elements: usize = tag_counts.values().sum();

    let mut out = String::from("HTML DOCUMENT STATS\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!("Document size  {} bytes\n", html.len()));
    out.push_str(&format!("Text content   {} bytes\n", text_bytes));
    out.push_str(&format!("Total elements {total_elements}\n"));
    out.push_str(&format!("Unique tags    {unique_tags}\n"));
    out.push_str(&format!("Max nest depth {max_depth}\n"));
    out.push_str(&format!("Comments       {comment_count}\n"));

    // top 15 tags
    let mut sorted: Vec<(&String, &usize)> = tag_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    out.push_str("\nTop elements:\n");
    for (tag, count) in sorted.iter().take(15) {
        out.push_str(&format!("  {:<15} {count}\n", tag));
    }
    Ok(out)
}
