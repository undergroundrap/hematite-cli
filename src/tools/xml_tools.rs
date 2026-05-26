use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::BTreeMap;

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("validate");

    match action {
        "validate" => validate(args),
        "format" => format_xml(args),
        "get" => get_path(args),
        "keys" => list_keys(args),
        "to-json" => to_json(args),
        "query" => query(args),
        other => Err(format!(
            "xml_tools: unknown action '{other}'. Valid: validate, format, get, keys, to-json, query"
        )),
    }
}

// ── Internal tree ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct XmlNode {
    tag: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<XmlNode>,
}

// ── Input resolution ───────────────────────────────────────────────────────────

fn resolve_input(args: &serde_json::Value) -> Result<String, String> {
    if let Some(inline) = args.get("xml").and_then(|v| v.as_str()) {
        return Ok(inline.to_string());
    }
    if let Some(file) = args
        .get("file")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
        let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
            std::path::PathBuf::from(r)
        } else {
            crate::tools::file_ops::workspace_root()
        };
        let path = if std::path::Path::new(file).is_absolute() {
            std::path::PathBuf::from(file)
        } else {
            root.join(file)
        };
        return std::fs::read_to_string(&path)
            .map_err(|e| format!("xml_tools: cannot read '{file}': {e}"));
    }
    Err("xml_tools: provide 'xml' (inline string) or 'file' (path)".into())
}

// ── Parser ─────────────────────────────────────────────────────────────────────

fn parse(src: &str) -> Result<XmlNode, String> {
    let mut reader = Reader::from_str(src);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root: Option<XmlNode> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                let attrs = attrs_from_event(e.attributes());
                stack.push(XmlNode {
                    tag,
                    attrs,
                    text: String::new(),
                    children: Vec::new(),
                });
            }
            Ok(Event::End(_)) => {
                if let Some(node) = stack.pop() {
                    if stack.is_empty() {
                        root = Some(node);
                    } else {
                        stack.last_mut().unwrap().children.push(node);
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                let attrs = attrs_from_event(e.attributes());
                let node = XmlNode {
                    tag,
                    attrs,
                    text: String::new(),
                    children: Vec::new(),
                };
                if stack.is_empty() {
                    root = Some(node);
                } else {
                    stack.last_mut().unwrap().children.push(node);
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(cur) = stack.last_mut() {
                    if let Ok(s) = e.unescape() {
                        let trimmed = s.trim().to_string();
                        if !trimmed.is_empty() {
                            if cur.text.is_empty() {
                                cur.text = trimmed;
                            } else {
                                cur.text.push(' ');
                                cur.text.push_str(&trimmed);
                            }
                        }
                    }
                }
            }
            Ok(Event::CData(e)) => {
                if let Some(cur) = stack.last_mut() {
                    let s = String::from_utf8_lossy(e.as_ref()).to_string();
                    if !s.trim().is_empty() {
                        if cur.text.is_empty() {
                            cur.text = s.trim().to_string();
                        } else {
                            cur.text.push(' ');
                            cur.text.push_str(s.trim());
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!(
                    "xml_tools: parse error at byte {}: {e}",
                    reader.buffer_position()
                ))
            }
            _ => {}
        }
    }

    root.ok_or_else(|| "xml_tools: no root element found".to_string())
}

fn attrs_from_event(attrs: quick_xml::events::attributes::Attributes) -> Vec<(String, String)> {
    attrs
        .filter_map(|a| a.ok())
        .map(|a| {
            let key = String::from_utf8_lossy(a.key.local_name().as_ref()).to_string();
            let val = a
                .unescape_value()
                .map(|v| v.to_string())
                .unwrap_or_else(|_| String::from_utf8_lossy(&a.value).to_string());
            (key, val)
        })
        .collect()
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn validate(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let root = parse(&src)?;

    let elem_count = count_elements(&root);
    let depth = tree_depth(&root);
    let attr_count = count_attrs(&root);

    let mut out = format!("XML VALID\n{}\n", "─".repeat(50));
    out.push_str(&format!("Root element : <{}>\n", root.tag));
    out.push_str(&format!("Elements     : {elem_count}\n"));
    out.push_str(&format!("Max depth    : {depth}\n"));
    out.push_str(&format!("Attributes   : {attr_count}\n"));

    if !root.attrs.is_empty() {
        out.push_str("Root attrs   :");
        for (k, v) in &root.attrs {
            out.push_str(&format!(" {k}=\"{v}\""));
        }
        out.push('\n');
    }

    let child_tags: Vec<&str> = root.children.iter().map(|c| c.tag.as_str()).collect();
    if !child_tags.is_empty() {
        out.push_str(&format!("Direct children ({}):\n", root.children.len()));
        let mut seen = BTreeMap::new();
        for tag in &child_tags {
            *seen.entry(*tag).or_insert(0u32) += 1;
        }
        for (tag, count) in seen.iter().take(10) {
            out.push_str(&format!(
                "  <{tag}>{}",
                if *count > 1 {
                    format!(" ×{count}")
                } else {
                    String::new()
                }
            ));
            out.push('\n');
        }
        if seen.len() > 10 {
            out.push_str(&format!("  ... and {} more unique tags\n", seen.len() - 10));
        }
    }

    Ok(out)
}

fn format_xml(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let root = parse(&src)?;

    let had_decl = src.trim_start().starts_with("<?xml");
    let mut out = format!("XML FORMAT\n{}\n", "─".repeat(50));
    if had_decl {
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    }
    out.push_str(&render_node(&root, 0));
    Ok(out)
}

fn get_path(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("xml_tools get: 'path' is required (e.g. 'project.build' or 'deps.dep[2]')")?;
    let root = parse(&src)?;

    // Support starting path with the root tag name or skipping it
    let effective_path = if let Some(stripped) = path.strip_prefix(&format!("{}.", root.tag)) {
        stripped
    } else if path == root.tag {
        let node_json = node_to_json(&root);
        let pretty = serde_json::to_string_pretty(&serde_json::json!({ &root.tag: node_json }))
            .unwrap_or_default();
        return Ok(format!("XML GET: {path}\n{}\n{pretty}", "─".repeat(50)));
    } else {
        path
    };

    let node = navigate(&root, effective_path)
        .ok_or_else(|| format!("xml_tools: path '{path}' not found"))?;

    let mut out = format!("XML GET: {path}\n{}\n", "─".repeat(50));
    out.push_str(&format!("<{}>", node.tag));
    if !node.attrs.is_empty() {
        for (k, v) in &node.attrs {
            out.push_str(&format!(" {k}=\"{v}\""));
        }
    }
    out.push('\n');
    if !node.text.is_empty() {
        out.push_str(&format!("  text: {}\n", node.text));
    }
    out.push_str(&format!("  children: {}\n", node.children.len()));
    for child in node.children.iter().take(10) {
        let snippet = if !child.text.is_empty() {
            format!(": \"{}\"", truncate(&child.text, 60))
        } else if !child.children.is_empty() {
            format!(" ({} children)", child.children.len())
        } else {
            String::new()
        };
        out.push_str(&format!("    <{}>{snippet}\n", child.tag));
    }
    if node.children.len() > 10 {
        out.push_str(&format!("  ... and {} more\n", node.children.len() - 10));
    }
    Ok(out)
}

fn list_keys(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let path = args.get("path").and_then(|v| v.as_str());
    let root = parse(&src)?;

    let target = if let Some(p) = path {
        let effective = if let Some(stripped) = p.strip_prefix(&format!("{}.", root.tag)) {
            stripped
        } else if p == root.tag {
            ""
        } else {
            p
        };
        if effective.is_empty() {
            &root
        } else {
            navigate(&root, effective).ok_or_else(|| format!("xml_tools: path '{p}' not found"))?
        }
    } else {
        &root
    };

    let mut out = format!(
        "XML KEYS{}\n{}\n",
        path.map(|p| format!(": {p}")).unwrap_or_default(),
        "─".repeat(50)
    );
    out.push_str(&format!("Element: <{}>\n", target.tag));
    if !target.attrs.is_empty() {
        out.push_str("Attributes:\n");
        for (k, v) in &target.attrs {
            out.push_str(&format!("  @{k} = \"{v}\"\n"));
        }
    }

    if target.children.is_empty() {
        if !target.text.is_empty() {
            out.push_str(&format!("Text: \"{}\"\n", truncate(&target.text, 80)));
        } else {
            out.push_str("No children (empty element)\n");
        }
    } else {
        let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
        for child in &target.children {
            *counts.entry(child.tag.as_str()).or_insert(0) += 1;
        }
        out.push_str(&format!("Children ({}):\n", target.children.len()));
        for (tag, count) in &counts {
            out.push_str(&format!(
                "  <{tag}>{}",
                if *count > 1 {
                    format!(" ×{count}")
                } else {
                    String::new()
                }
            ));
            out.push('\n');
        }
        out.push_str(&format!(
            "\nTotal: {} child element(s)\n",
            target.children.len()
        ));
    }
    Ok(out)
}

fn to_json(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let root = parse(&src)?;

    let json_val = serde_json::json!({ &root.tag: node_to_json(&root) });
    let pretty =
        serde_json::to_string_pretty(&json_val).map_err(|e| format!("xml_tools to-json: {e}"))?;
    Ok(format!("XML → JSON\n{}\n{pretty}", "─".repeat(50)))
}

fn query(args: &serde_json::Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let tag = args
        .get("tag")
        .or_else(|| args.get("name"))
        .and_then(|v| v.as_str())
        .ok_or("xml_tools query: 'tag' is required (element name to search for)")?;
    let root = parse(&src)?;

    let mut results: Vec<&XmlNode> = Vec::new();
    collect_by_tag(&root, tag, &mut results);

    let mut out = format!("XML QUERY: <{tag}>\n{}\n", "─".repeat(50));
    if results.is_empty() {
        out.push_str(&format!("No <{tag}> elements found.\n"));
    } else {
        out.push_str(&format!("Found {} match(es):\n\n", results.len()));
        for (i, node) in results.iter().enumerate().take(20) {
            out.push_str(&format!("[{}] <{}>", i + 1, node.tag));
            for (k, v) in &node.attrs {
                out.push_str(&format!(" {k}=\"{}\"", truncate(v, 40)));
            }
            if !node.text.is_empty() {
                out.push_str(&format!("\n    text: \"{}\"", truncate(&node.text, 80)));
            }
            if !node.children.is_empty() {
                out.push_str(&format!("\n    children: {}", node.children.len()));
            }
            out.push_str("\n\n");
        }
        if results.len() > 20 {
            out.push_str(&format!(
                "... and {} more (showing first 20)\n",
                results.len() - 20
            ));
        }
    }
    Ok(out)
}

// ── Tree helpers ───────────────────────────────────────────────────────────────

fn navigate<'a>(node: &'a XmlNode, path: &str) -> Option<&'a XmlNode> {
    if path.is_empty() {
        return Some(node);
    }
    let mut cur = node;
    for part in path.split('.') {
        if let Some(bracket) = part.find('[') {
            let tag = &part[..bracket];
            let idx: usize = part[bracket + 1..].trim_end_matches(']').parse().ok()?;
            let matches: Vec<&XmlNode> = cur.children.iter().filter(|c| c.tag == tag).collect();
            cur = matches.get(idx)?;
        } else {
            cur = cur.children.iter().find(|c| c.tag == part)?;
        }
    }
    Some(cur)
}

fn collect_by_tag<'a>(node: &'a XmlNode, tag: &str, out: &mut Vec<&'a XmlNode>) {
    if node.tag == tag {
        out.push(node);
    }
    for child in &node.children {
        collect_by_tag(child, tag, out);
    }
}

fn render_node(node: &XmlNode, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let mut s = String::new();
    s.push_str(&pad);
    s.push('<');
    s.push_str(&node.tag);
    for (k, v) in &node.attrs {
        s.push_str(&format!(" {k}=\"{v}\""));
    }
    if node.children.is_empty() && node.text.is_empty() {
        s.push_str("/>\n");
    } else if node.children.is_empty() {
        s.push('>');
        s.push_str(&xml_escape(&node.text));
        s.push_str(&format!("</{}>\n", node.tag));
    } else {
        s.push_str(">\n");
        if !node.text.is_empty() {
            s.push_str(&format!("{}  {}\n", pad, xml_escape(&node.text)));
        }
        for child in &node.children {
            s.push_str(&render_node(child, indent + 1));
        }
        s.push_str(&format!("{}</{}>\n", pad, node.tag));
    }
    s
}

fn node_to_json(node: &XmlNode) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    if !node.attrs.is_empty() {
        let mut attr_map = serde_json::Map::new();
        for (k, v) in &node.attrs {
            attr_map.insert(format!("@{k}"), serde_json::Value::String(v.clone()));
        }
        map.insert(
            "@attributes".to_string(),
            serde_json::Value::Object(attr_map),
        );
    }

    if !node.text.is_empty() {
        map.insert(
            "#text".to_string(),
            serde_json::Value::String(node.text.clone()),
        );
    }

    // Group same-name children into arrays
    let mut child_groups: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for child in &node.children {
        child_groups
            .entry(child.tag.clone())
            .or_default()
            .push(node_to_json(child));
    }
    for (tag, mut values) in child_groups {
        if values.len() == 1 {
            map.insert(tag, values.pop().unwrap());
        } else {
            map.insert(tag, serde_json::Value::Array(values));
        }
    }

    // Simple text-only node: just return the string
    if map.len() == 1 && map.contains_key("#text") {
        return serde_json::Value::String(node.text.clone());
    }

    if map.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::Object(map)
    }
}

fn count_elements(node: &XmlNode) -> usize {
    1 + node.children.iter().map(count_elements).sum::<usize>()
}

fn tree_depth(node: &XmlNode) -> usize {
    if node.children.is_empty() {
        1
    } else {
        1 + node.children.iter().map(tree_depth).max().unwrap_or(0)
    }
}

fn count_attrs(node: &XmlNode) -> usize {
    node.attrs.len() + node.children.iter().map(count_attrs).sum::<usize>()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
