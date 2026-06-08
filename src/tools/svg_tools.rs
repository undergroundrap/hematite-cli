use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "name": "svg_tools",
        "description": "Parse and analyze SVG (Scalable Vector Graphics) files without external utilities. \
    Actions: info (default — dimensions, viewBox, element counts, title/desc), \
    elements (all distinct element types with counts), \
    ids (all id attributes with element type), \
    links (href/xlink:href references and linked resources), \
    styles (inline style rules and class usage), \
    validate (common SVG issues: missing viewBox, deprecated attributes, accessibility). \
    Pass file (path to .svg) or svg (inline SVG text). \
    Example: svg_tools(file: 'icon.svg') or svg_tools(action: 'ids', file: 'diagram.svg') or svg_tools(action: 'elements', svg: '<svg>...</svg>').",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "info|elements|ids|links|styles|validate" },
                "file": { "type": "string", "description": "Path to SVG file" },
                "svg": { "type": "string", "description": "Inline SVG text" }
            },
            "required": []
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let text = get_text(args);
    let text = match text {
        Some(t) => t,
        None => {
            return Ok(
                "Error: provide 'file' (path to SVG file) or 'svg' (inline SVG text).".to_string(),
            )
        }
    };

    if !text.contains("<svg") && !text.contains("<SVG") {
        return Ok(
            "Error: input does not appear to be an SVG document (no <svg> element found)."
                .to_string(),
        );
    }

    Ok(match action {
        "elements" => format_elements(&text),
        "ids" => format_ids(&text),
        "links" => format_links(&text),
        "styles" => format_styles(&text),
        "validate" => format_validate(&text),
        _ => format_info(&text),
    })
}

fn get_text(args: &Value) -> Option<String> {
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path).ok();
    }
    if let Some(t) = args.get("svg").and_then(|v| v.as_str()) {
        return Some(t.to_string());
    }
    None
}

// ── Minimal XML token iterator ──────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Token<'a> {
    StartTag {
        name: &'a str,
        attrs: &'a str,
        self_closing: bool,
    },
    EndTag {
        name: &'a str,
    },
    Text(&'a str),
}

fn next_token<'a>(src: &'a str, pos: usize) -> Option<(Token<'a>, usize)> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    if pos >= len {
        return None;
    }

    if bytes[pos] == b'<' {
        // Find closing >
        let mut end = pos + 1;
        let mut in_str: Option<u8> = None;
        while end < len {
            let b = bytes[end];
            if let Some(q) = in_str {
                if b == q {
                    in_str = None;
                }
            } else if b == b'"' || b == b'\'' {
                in_str = Some(b);
            } else if b == b'>' {
                break;
            }
            end += 1;
        }
        let tag_src = &src[pos..end + 1];

        // Comment or PI or CDATA — skip
        if tag_src.starts_with("<!--") || tag_src.starts_with("<?") || tag_src.starts_with("<![") {
            return Some((Token::Text(""), end + 1));
        }

        let inner = &tag_src[1..tag_src.len() - 1]; // strip < >
        let self_closing = inner.ends_with('/');
        let inner = if self_closing {
            &inner[..inner.len() - 1]
        } else {
            inner
        };
        let inner = inner.trim();

        if let Some(after_slash) = inner.strip_prefix('/') {
            let name = after_slash
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('/');
            return Some((Token::EndTag { name }, end + 1));
        }

        let (name, attrs) = split_tag_name(inner);
        return Some((
            Token::StartTag {
                name,
                attrs,
                self_closing,
            },
            end + 1,
        ));
    }

    // Text node
    let end = src[pos..].find('<').map(|i| pos + i).unwrap_or(len);
    Some((Token::Text(&src[pos..end]), end))
}

fn split_tag_name(inner: &str) -> (&str, &str) {
    let name_end = inner
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(inner.len());
    (&inner[..name_end], inner[name_end..].trim_start())
}

fn strip_ns(name: &str) -> &str {
    if let Some(p) = name.find(':') {
        &name[p + 1..]
    } else {
        name
    }
}

// ── Attribute extraction helpers ────────────────────────────────────────────

fn get_attr<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    // Find key= and return the quoted value
    let mut rest = attrs;
    while let Some(pos) = rest.find(key) {
        let after = &rest[pos + key.len()..].trim_start();
        if let Some(after_eq) = after.strip_prefix('=') {
            let val = after_eq.trim_start();
            if let Some(val_inner) = val.strip_prefix('"') {
                let end = val_inner.find('"').unwrap_or(val_inner.len());
                return Some(&val_inner[..end]);
            } else if let Some(val_inner) = val.strip_prefix('\'') {
                let end = val_inner.find('\'').unwrap_or(val_inner.len());
                return Some(&val_inner[..end]);
            } else {
                let end = val
                    .find(|c: char| c.is_ascii_whitespace())
                    .unwrap_or(val.len());
                return Some(&val[..end]);
            }
        }
        rest = &rest[pos + key.len()..];
    }
    None
}

fn get_attr_owned(attrs: &str, key: &str) -> Option<String> {
    get_attr(attrs, key).map(|s| s.to_string())
}

// ── SVG-level attributes from the root <svg> element ───────────────────────

struct SvgRoot {
    width: Option<String>,
    height: Option<String>,
    viewbox: Option<String>,
    xmlns: Option<String>,
    version: Option<String>,
    title: Option<String>,
    desc: Option<String>,
}

fn parse_root(text: &str) -> SvgRoot {
    let mut pos = 0;
    let mut root_attrs: Option<&str> = None;
    let mut title: Option<String> = None;
    let mut desc: Option<String> = None;
    let mut in_title = false;
    let mut in_desc = false;
    let mut text_buf = String::new();

    while let Some((tok, next)) = next_token(text, pos) {
        pos = next;
        match tok {
            Token::StartTag { name, attrs, .. } => {
                let bare = strip_ns(name).to_ascii_lowercase();
                if bare == "svg" && root_attrs.is_none() {
                    root_attrs = Some(attrs);
                }
                if bare == "title" {
                    in_title = true;
                    text_buf.clear();
                }
                if bare == "desc" {
                    in_desc = true;
                    text_buf.clear();
                }
            }
            Token::EndTag { name } => {
                let bare = strip_ns(name).to_ascii_lowercase();
                if bare == "title" {
                    title = Some(text_buf.trim().to_string());
                    in_title = false;
                }
                if bare == "desc" {
                    desc = Some(text_buf.trim().to_string());
                    in_desc = false;
                }
            }
            Token::Text(t) => {
                if in_title || in_desc {
                    text_buf.push_str(t);
                }
            }
        }
    }

    let attrs = root_attrs.unwrap_or("");
    SvgRoot {
        width: get_attr_owned(attrs, "width"),
        height: get_attr_owned(attrs, "height"),
        viewbox: get_attr_owned(attrs, "viewBox"),
        xmlns: get_attr_owned(attrs, "xmlns"),
        version: get_attr_owned(attrs, "version"),
        title,
        desc,
    }
}

// ── Element counter ──────────────────────────────────────────────────────────

fn count_elements(text: &str) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    let mut pos = 0;
    while let Some((tok, next)) = next_token(text, pos) {
        pos = next;
        if let Token::StartTag { name, .. } = tok {
            let bare = strip_ns(name).to_ascii_lowercase();
            if !bare.is_empty() {
                *counts.entry(bare).or_insert(0) += 1;
            }
        }
    }
    counts
}

// ── Format functions ─────────────────────────────────────────────────────────

fn format_info(text: &str) -> String {
    let root = parse_root(text);
    let counts = count_elements(text);

    let total_elements: usize = counts.values().sum();
    let file_size = text.len();

    let mut out = String::from("SVG Document Info\n\n");

    if let Some(ref t) = root.title {
        if !t.is_empty() {
            out.push_str(&format!("  {:24} {}\n", "Title:", t));
        }
    }
    if let Some(ref d) = root.desc {
        if !d.is_empty() {
            let short = if d.len() > 80 {
                format!("{}...", &d[..80])
            } else {
                d.clone()
            };
            out.push_str(&format!("  {:24} {}\n", "Description:", short));
        }
    }

    out.push_str("\nDimensions\n");
    match (&root.width, &root.height) {
        (Some(w), Some(h)) => {
            out.push_str(&format!("  {:24} {} × {}\n", "Size:", w, h));
        }
        (Some(w), None) => {
            out.push_str(&format!("  {:24} {} (no height)\n", "Width:", w));
        }
        (None, Some(h)) => {
            out.push_str(&format!("  {:24} {} (no width)\n", "Height:", h));
        }
        (None, None) => {
            out.push_str("  No explicit width/height attributes.\n");
        }
    }
    if let Some(ref vb) = root.viewbox {
        out.push_str(&format!("  {:24} {}\n", "viewBox:", vb));
    } else {
        out.push_str("  No viewBox attribute.\n");
    }
    if let Some(ref v) = root.version {
        out.push_str(&format!("  {:24} {}\n", "SVG Version:", v));
    }
    if let Some(ref ns) = root.xmlns {
        out.push_str(&format!("  {:24} {}\n", "Namespace:", ns));
    }

    out.push_str("\nDocument Stats\n");
    out.push_str(&format!("  {:24} {} bytes\n", "File Size:", file_size));
    out.push_str(&format!("  {:24} {}\n", "Total Elements:", total_elements));

    // Top element types
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    out.push_str("\nTop Element Types\n");
    for (tag, count) in sorted.iter().take(10) {
        out.push_str(&format!("  {:24} {}\n", format!("<{}>:", tag), count));
    }

    // Useful flags
    let has_defs = counts.contains_key("defs");
    let has_use = counts.contains_key("use");
    let has_script = counts.contains_key("script");
    let has_anim = counts.contains_key("animate")
        || counts.contains_key("animatetransform")
        || counts.contains_key("animatemotion");
    let has_filter = counts.contains_key("filter");
    let has_mask = counts.contains_key("mask");
    let has_gradient =
        counts.contains_key("lineargradient") || counts.contains_key("radialgradient");
    let has_text = counts.contains_key("text")
        || counts.contains_key("tspan")
        || counts.contains_key("textpath");

    let mut flags = Vec::new();
    if has_defs {
        flags.push("uses <defs>");
    }
    if has_use {
        flags.push("uses <use> (symbol reuse)");
    }
    if has_script {
        flags.push("contains <script>");
    }
    if has_anim {
        flags.push("has animations");
    }
    if has_filter {
        flags.push("has filters/effects");
    }
    if has_mask {
        flags.push("has masks");
    }
    if has_gradient {
        flags.push("has gradients");
    }
    if has_text {
        flags.push("contains text elements");
    }

    if !flags.is_empty() {
        out.push_str("\nFeatures\n");
        for f in flags {
            out.push_str(&format!("  ✓ {}\n", f));
        }
    }

    out
}

fn format_elements(text: &str) -> String {
    let counts = count_elements(text);
    if counts.is_empty() {
        return "No elements found.".to_string();
    }
    let total: usize = counts.values().sum();
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

    let mut out = format!(
        "SVG Elements  ({} total, {} types)\n\n",
        total,
        sorted.len()
    );
    out.push_str(&format!("  {:24} {:>6}  {}\n", "Element", "Count", "Role"));
    out.push_str(&format!("  {:24} {:>6}  {}\n", "───────", "─────", "────"));
    for (tag, count) in &sorted {
        let role = element_role(tag);
        out.push_str(&format!(
            "  {:24} {:>6}  {}\n",
            format!("<{}>", tag),
            count,
            role
        ));
    }
    out
}

fn element_role(tag: &str) -> &'static str {
    match tag {
        "svg" => "Root document element",
        "g" => "Group",
        "path" => "Path (curves and lines)",
        "rect" => "Rectangle",
        "circle" => "Circle",
        "ellipse" => "Ellipse",
        "line" => "Line segment",
        "polyline" => "Polyline",
        "polygon" => "Polygon",
        "text" | "tspan" | "textpath" => "Text",
        "image" => "Embedded image",
        "use" => "Reuse of defined element",
        "symbol" => "Reusable symbol definition",
        "defs" => "Definitions (not rendered directly)",
        "marker" => "Marker (arrows etc.)",
        "clippath" => "Clipping path",
        "mask" => "Mask",
        "pattern" => "Pattern fill",
        "filter" => "Filter effect",
        "fegaussianblur" | "fecolormatrix" | "fecomposite" | "feblend" => "Filter primitive",
        "lineargradient" | "radialgradient" | "stop" => "Gradient",
        "animate" | "animatetransform" | "animatemotion" | "set" => "Animation",
        "script" => "Script",
        "style" => "Inline stylesheet",
        "title" => "Title (accessibility)",
        "desc" => "Description (accessibility)",
        "metadata" => "Metadata",
        "a" => "Hyperlink",
        "foreignobject" => "Foreign XML content",
        _ => "SVG element",
    }
}

fn format_ids(text: &str) -> String {
    let mut items: Vec<(String, String)> = Vec::new();
    let mut pos = 0;
    while let Some((tok, next)) = next_token(text, pos) {
        pos = next;
        if let Token::StartTag { name, attrs, .. } = tok {
            if let Some(id) = get_attr(attrs, "id") {
                items.push((id.to_string(), strip_ns(name).to_ascii_lowercase()));
            }
        }
    }
    if items.is_empty() {
        return "No id attributes found in this SVG.".to_string();
    }
    let mut out = format!("SVG ID Attributes  ({} found)\n\n", items.len());
    out.push_str(&format!("  {:40} {}\n", "ID", "Element"));
    out.push_str(&format!("  {:40} {}\n", "──", "───────"));
    for (id, elem) in &items {
        out.push_str(&format!("  {:40} <{}>\n", id, elem));
    }
    out
}

fn format_links(text: &str) -> String {
    let mut hrefs: Vec<(String, String)> = Vec::new();
    let mut pos = 0;
    while let Some((tok, next)) = next_token(text, pos) {
        pos = next;
        if let Token::StartTag { name, attrs, .. } = tok {
            let bare = strip_ns(name).to_ascii_lowercase();
            for attr_key in &["href", "xlink:href"] {
                if let Some(href) = get_attr(attrs, attr_key) {
                    if !href.is_empty() {
                        hrefs.push((href.to_string(), bare.clone()));
                    }
                }
            }
        }
    }
    if hrefs.is_empty() {
        return "No href or xlink:href references found.".to_string();
    }

    let mut out = format!("SVG Links & References  ({} found)\n\n", hrefs.len());

    let mut internal: Vec<_> = hrefs.iter().filter(|(h, _)| h.starts_with('#')).collect();
    let mut external: Vec<_> = hrefs.iter().filter(|(h, _)| !h.starts_with('#')).collect();

    if !internal.is_empty() {
        out.push_str(&format!("Internal References ({})\n", internal.len()));
        internal.sort();
        for (href, elem) in &internal {
            out.push_str(&format!("  {:40} from <{}>\n", href, elem));
        }
        out.push('\n');
    }
    if !external.is_empty() {
        out.push_str(&format!("External References ({})\n", external.len()));
        external.sort();
        for (href, elem) in &external {
            out.push_str(&format!("  {:40} from <{}>\n", href, elem));
        }
    }
    out
}

fn format_styles(text: &str) -> String {
    let mut inline_styles: usize = 0;
    let mut class_usage: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut style_block = String::new();
    let mut in_style = false;
    let mut pos = 0;

    while let Some((tok, next)) = next_token(text, pos) {
        pos = next;
        match tok {
            Token::StartTag {
                name,
                attrs,
                self_closing,
            } => {
                let bare = strip_ns(name).to_ascii_lowercase();
                if bare == "style" && !self_closing {
                    in_style = true;
                }
                if get_attr(attrs, "style").is_some() {
                    inline_styles += 1;
                }
                if let Some(classes) = get_attr(attrs, "class") {
                    for cls in classes.split_ascii_whitespace() {
                        *class_usage.entry(cls.to_string()).or_insert(0) += 1;
                    }
                }
            }
            Token::EndTag { name } => {
                if strip_ns(name).eq_ignore_ascii_case("style") {
                    in_style = false;
                }
            }
            Token::Text(t) => {
                if in_style {
                    style_block.push_str(t);
                }
            }
        }
    }

    let mut out = String::from("SVG Styles\n\n");
    out.push_str(&format!(
        "  {:32} {}\n",
        "Elements with inline style:", inline_styles
    ));
    out.push_str(&format!(
        "  {:32} {}\n",
        "Unique CSS classes used:",
        class_usage.len()
    ));

    if !style_block.trim().is_empty() {
        let rule_count = style_block.matches('{').count();
        out.push_str(&format!(
            "  {:32} {}\n",
            "CSS rules in <style> block:", rule_count
        ));
        out.push_str("\nStyle Block Preview (first 500 chars)\n");
        let preview: String = style_block.trim().chars().take(500).collect();
        out.push_str(&format!("  {}\n", preview.replace('\n', "\n  ")));
        if style_block.len() > 500 {
            out.push_str("  ...\n");
        }
    }

    if !class_usage.is_empty() {
        let mut sorted: Vec<_> = class_usage.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        out.push_str("\nCSS Class Usage\n");
        for (cls, count) in sorted.iter().take(20) {
            out.push_str(&format!("  .{:35} {} element(s)\n", cls, count));
        }
        if sorted.len() > 20 {
            out.push_str(&format!("  ... and {} more\n", sorted.len() - 20));
        }
    }
    out
}

fn format_validate(text: &str) -> String {
    let mut issues: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut ok: Vec<&str> = Vec::new();

    let root = parse_root(text);
    let counts = count_elements(text);

    // viewBox check
    if root.viewbox.is_none() {
        warnings
            .push("Missing viewBox — SVG will not scale correctly in all contexts.".to_string());
    } else {
        ok.push("viewBox present");
    }

    // width/height check
    if root.width.is_none() && root.height.is_none() && root.viewbox.is_some() {
        ok.push("Responsive sizing (no explicit width/height + viewBox)");
    } else if root.width.is_none() || root.height.is_none() {
        warnings.push("Only one of width/height set — may cause layout issues.".to_string());
    }

    // xmlns check
    if root.xmlns.is_none() {
        warnings.push(
            "Missing xmlns attribute on <svg> — may cause rendering issues in some contexts."
                .to_string(),
        );
    } else {
        ok.push("xmlns declared");
    }

    // Accessibility checks
    if root.title.as_deref().map(|t| t.is_empty()).unwrap_or(true) {
        warnings.push(
            "No <title> element — reduces accessibility (screen readers need a title).".to_string(),
        );
    } else {
        ok.push("<title> present");
    }
    if root.desc.as_deref().map(|t| t.is_empty()).unwrap_or(true) {
        warnings.push(
            "No <desc> element — consider adding a description for accessibility.".to_string(),
        );
    }

    // Script check
    if counts.contains_key("script") {
        issues.push("<script> element found — SVG scripts are disabled in many embedding contexts and pose XSS risk.".to_string());
    }

    // Deprecated attributes check
    let deprecated_count = count_deprecated(text);
    if deprecated_count > 0 {
        warnings.push(format!("{} deprecated attribute(s) found (e.g. xlink:href — use href instead, xml:space — use CSS white-space).", deprecated_count));
    }

    // foreignObject
    if counts.contains_key("foreignobject") {
        warnings.push("<foreignObject> present — may not render in all SVG viewers.".to_string());
    }

    // Empty defs
    // (Hard to detect without full parsing — skip)

    // Duplicate IDs
    let dup_ids = find_duplicate_ids(text);
    if !dup_ids.is_empty() {
        issues.push(format!(
            "Duplicate id attributes found: {} — IDs must be unique.",
            dup_ids.join(", ")
        ));
    } else {
        ok.push("No duplicate IDs");
    }

    let mut out = String::from("SVG Validation\n\n");

    if issues.is_empty() && warnings.is_empty() {
        out.push_str("✓ No issues found.\n");
    }

    if !issues.is_empty() {
        out.push_str(&format!("Issues ({}) — fix these\n", issues.len()));
        for issue in &issues {
            out.push_str(&format!("  ✗ {}\n", issue));
        }
        out.push('\n');
    }
    if !warnings.is_empty() {
        out.push_str(&format!(
            "Warnings ({}) — consider fixing\n",
            warnings.len()
        ));
        for w in &warnings {
            out.push_str(&format!("  ⚠ {}\n", w));
        }
        out.push('\n');
    }
    if !ok.is_empty() {
        out.push_str(&format!("OK ({})\n", ok.len()));
        for o in &ok {
            out.push_str(&format!("  ✓ {}\n", o));
        }
    }

    out
}

fn count_deprecated(text: &str) -> usize {
    let mut count = 0;
    let deprecated = &[
        "xlink:href",
        "xlink:title",
        "xlink:show",
        "xlink:actuate",
        "xml:space",
        "xml:lang",
        "enable-background",
        "clip",
        "color-rendering",
    ];
    for d in deprecated {
        count += text.matches(d).count();
    }
    count
}

fn find_duplicate_ids(text: &str) -> Vec<String> {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut pos = 0;
    while let Some((tok, next)) = next_token(text, pos) {
        pos = next;
        if let Token::StartTag { attrs, .. } = tok {
            if let Some(id) = get_attr(attrs, "id") {
                *seen.entry(id.to_string()).or_insert(0) += 1;
            }
        }
    }
    let mut dups: Vec<String> = seen
        .into_iter()
        .filter(|(_, c)| *c > 1)
        .map(|(id, _)| id)
        .collect();
    dups.sort();
    dups
}
