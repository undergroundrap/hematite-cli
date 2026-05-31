use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("generate");
    match action {
        "generate" | "dot" => action_generate(args),
        "parse" => action_parse(args),
        "flowchart" => action_flowchart(args),
        "tree" => action_tree(args),
        other => Err(format!(
            "graphviz_tools: unknown action '{other}'. Valid: generate, parse, flowchart, tree"
        )),
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn escape_label(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn node_id(s: &str) -> String {
    // Sanitize for DOT node identifiers
    let safe: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if safe
        .chars()
        .next()
        .map(|c| c.is_alphabetic())
        .unwrap_or(false)
    {
        safe
    } else {
        format!("n_{}", safe)
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_generate(args: &Value) -> Result<String, String> {
    let directed = args
        .get("directed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let name = args
        .get("name")
        .or_else(|| args.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("G");
    let rankdir = args.get("rankdir").and_then(|v| v.as_str()).unwrap_or("TB");
    let graph_kw = if directed { "digraph" } else { "graph" };
    let edge_op = if directed { " -> " } else { " -- " };

    // Parse nodes
    let nodes: Vec<(String, String)> =
        if let Some(arr) = args.get("nodes").and_then(|v| v.as_array()) {
            arr.iter()
                .map(|n| {
                    if let Some(s) = n.as_str() {
                        (node_id(s), s.to_string())
                    } else if let Some(obj) = n.as_object() {
                        let id = obj
                            .get("id")
                            .or_else(|| obj.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("node");
                        let label = obj.get("label").and_then(|v| v.as_str()).unwrap_or(id);
                        (node_id(id), label.to_string())
                    } else {
                        ("node".to_string(), "node".to_string())
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

    // Parse edges
    let edges: Vec<(String, String, Option<String>)> =
        if let Some(arr) = args.get("edges").and_then(|v| v.as_array()) {
            arr.iter()
                .filter_map(|e| {
                    if let Some(obj) = e.as_object() {
                        let from = obj
                            .get("from")
                            .or_else(|| obj.get("source"))
                            .or_else(|| obj.get("a"))
                            .and_then(|v| v.as_str())?;
                        let to = obj
                            .get("to")
                            .or_else(|| obj.get("target"))
                            .or_else(|| obj.get("b"))
                            .and_then(|v| v.as_str())?;
                        let label = obj.get("label").or_else(|| obj.get("weight")).map(|v| {
                            v.as_str()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| v.to_string())
                        });
                        Some((from.to_string(), to.to_string(), label))
                    } else if let Some(arr) = e.as_array() {
                        let from = arr.first()?.as_str()?.to_string();
                        let to = arr.get(1)?.as_str()?.to_string();
                        let label = arr.get(2).map(|v| {
                            v.as_str()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| v.to_string())
                        });
                        Some((from, to, label))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

    if nodes.is_empty() && edges.is_empty() {
        return Err(
            "graphviz_tools: pass 'nodes' array and/or 'edges' array to generate DOT output".into(),
        );
    }

    let mut dot = format!("{} {} {{\n", graph_kw, name);
    dot.push_str(&format!("    rankdir={};\n", rankdir));
    dot.push_str("    node [shape=box, style=rounded];\n");
    dot.push('\n');

    for (id, label) in &nodes {
        if label == id {
            dot.push_str(&format!("    {};\n", id));
        } else {
            dot.push_str(&format!(
                "    {} [label=\"{}\"];\n",
                id,
                escape_label(label)
            ));
        }
    }

    if !nodes.is_empty() && !edges.is_empty() {
        dot.push('\n');
    }

    for (from, to, label) in &edges {
        let from_id = node_id(from);
        let to_id = node_id(to);
        if let Some(lbl) = label {
            dot.push_str(&format!(
                "    {}{}{} [label=\"{}\"];\n",
                from_id,
                edge_op,
                to_id,
                escape_label(lbl)
            ));
        } else {
            dot.push_str(&format!("    {}{}{};\n", from_id, edge_op, to_id));
        }
    }

    dot.push('}');

    let node_count = if nodes.is_empty() {
        // Count unique nodes from edges
        let mut seen = std::collections::HashSet::new();
        for (f, t, _) in &edges {
            seen.insert(f);
            seen.insert(t);
        }
        seen.len()
    } else {
        nodes.len()
    };

    let mut out = format!(
        "DOT Output ({} nodes, {} edges)\n{}\n\n",
        node_count,
        edges.len(),
        "─".repeat(50)
    );
    out.push_str(&dot);
    out.push_str("\n\nRender with: dot -Tpng output.dot -o graph.png");
    out.push_str("\n             dot -Tsvg output.dot -o graph.svg");
    out.push_str("\n             dot -Tpdf output.dot -o graph.pdf");
    Ok(out)
}

fn action_parse(args: &Value) -> Result<String, String> {
    let dot_src = args
        .get("dot")
        .or_else(|| args.get("text"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("graphviz_tools parse: pass 'dot' or 'text' with DOT language source")?;

    let is_directed = dot_src.contains("digraph");
    let mut node_ids: Vec<String> = Vec::new();
    let mut edges: Vec<(String, String, Option<String>)> = Vec::new();

    for line in dot_src.lines() {
        let line = line.trim().trim_end_matches(';');
        if line.starts_with("//") || line.starts_with('#') || line.starts_with("/*") {
            continue;
        }
        if line.starts_with("digraph") || line.starts_with("graph") || line == "{" || line == "}" {
            continue;
        }
        if line.contains("->") || line.contains("--") {
            let (sep, parts): (&str, Vec<&str>) = if line.contains("->") {
                ("->", line.splitn(2, "->").collect())
            } else {
                ("--", line.splitn(2, "--").collect())
            };
            let _ = sep;
            if parts.len() == 2 {
                let from = parts[0].trim().trim_matches('"').to_string();
                let rest = parts[1];
                // strip attributes
                let to_part = rest
                    .split('[')
                    .next()
                    .unwrap_or(rest)
                    .trim()
                    .trim_matches('"');
                let label = if rest.contains("label") {
                    rest.find("label").and_then(|i| {
                        let after = &rest[i + 5..];
                        let after =
                            after.trim_start_matches(|c: char| c == '=' || c.is_whitespace());
                        after
                            .strip_prefix('"')
                            .map(|s| s.split('"').next().unwrap_or("").to_string())
                    })
                } else {
                    None
                };
                if !from.is_empty() && !to_part.is_empty() {
                    edges.push((from, to_part.to_string(), label));
                }
            }
        } else if !line.is_empty() && !line.contains('=') && !line.starts_with('}') {
            // Standalone node declaration
            let id = line
                .split('[')
                .next()
                .unwrap_or(line)
                .trim()
                .trim_matches('"');
            if !id.is_empty() && id != "{" {
                node_ids.push(id.to_string());
            }
        }
    }

    // Collect unique nodes from edges
    let mut all_nodes: Vec<String> = node_ids.clone();
    for (f, t, _) in &edges {
        if !all_nodes.contains(f) {
            all_nodes.push(f.clone());
        }
        if !all_nodes.contains(t) {
            all_nodes.push(t.clone());
        }
    }

    let mut out = format!("DOT Parse Result\n{}\n", "─".repeat(40));
    out.push_str(&format!(
        "Type:   {}\n",
        if is_directed {
            "digraph (directed)"
        } else {
            "graph (undirected)"
        }
    ));
    out.push_str(&format!("Nodes:  {}\n", all_nodes.len()));
    out.push_str(&format!("Edges:  {}\n", edges.len()));

    if !all_nodes.is_empty() {
        out.push_str("\nNode list:\n");
        for n in &all_nodes {
            out.push_str(&format!("  {}\n", n));
        }
    }
    if !edges.is_empty() {
        out.push_str("\nEdge list:\n");
        let sep = if is_directed { "->" } else { "--" };
        for (f, t, l) in &edges {
            if let Some(lbl) = l {
                out.push_str(&format!("  {} {} {} [\"{}\"]\n", f, sep, t, lbl));
            } else {
                out.push_str(&format!("  {} {} {}\n", f, sep, t));
            }
        }
    }
    Ok(out)
}

fn action_flowchart(args: &Value) -> Result<String, String> {
    // Generate a simple top-down flowchart from steps
    let steps: Vec<&str> = if let Some(arr) = args.get("steps").and_then(|v| v.as_array()) {
        arr.iter().filter_map(|v| v.as_str()).collect()
    } else {
        return Err(
            "graphviz_tools flowchart: pass 'steps' as an array of step name strings".into(),
        );
    };

    if steps.is_empty() {
        return Err("graphviz_tools flowchart: 'steps' array must not be empty".into());
    }

    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Flowchart");
    let start = args
        .get("start")
        .and_then(|v| v.as_str())
        .unwrap_or("Start");
    let end = args.get("end").and_then(|v| v.as_str()).unwrap_or("End");

    let mut dot = format!("digraph {} {{\n", node_id(name));
    dot.push_str("    rankdir=TB;\n");
    dot.push_str("    node [shape=box, style=rounded];\n");
    dot.push_str(&format!(
        "    start [label=\"{}\", shape=oval];\n",
        escape_label(start)
    ));

    for (i, step) in steps.iter().enumerate() {
        dot.push_str(&format!(
            "    step{} [label=\"{}\"];\n",
            i,
            escape_label(step)
        ));
    }
    dot.push_str(&format!(
        "    end_ [label=\"{}\", shape=oval];\n",
        escape_label(end)
    ));
    dot.push('\n');
    dot.push_str("    start -> step0;\n");
    for i in 0..steps.len().saturating_sub(1) {
        dot.push_str(&format!("    step{} -> step{};\n", i, i + 1));
    }
    if !steps.is_empty() {
        dot.push_str(&format!("    step{} -> end_;\n", steps.len() - 1));
    }
    dot.push('}');

    let mut out = format!(
        "Flowchart DOT ({} steps)\n{}\n\n",
        steps.len(),
        "─".repeat(50)
    );
    out.push_str(&dot);
    Ok(out)
}

fn action_tree(args: &Value) -> Result<String, String> {
    let root = args.get("root").and_then(|v| v.as_str()).unwrap_or("root");
    let children_val = args.get("children").or_else(|| args.get("nodes"));

    let mut dot = format!("digraph Tree {{\n");
    dot.push_str("    rankdir=TB;\n");
    dot.push_str("    node [shape=box];\n\n");

    fn add_node(
        dot: &mut String,
        parent_id: &str,
        parent_label: &str,
        val: &Value,
        counter: &mut usize,
    ) {
        let id = format!("n{}", counter);
        *counter += 1;
        let label = val
            .get("label")
            .or_else(|| val.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or(parent_label);
        if !parent_id.is_empty() {
            dot.push_str(&format!(
                "    {} [label=\"{}\"];\n",
                id,
                escape_label(label)
            ));
            dot.push_str(&format!("    {} -> {};\n", parent_id, id));
        } else {
            dot.push_str(&format!(
                "    {} [label=\"{}\"];\n",
                id,
                escape_label(label)
            ));
        }
        if let Some(children) = val.get("children").and_then(|v| v.as_array()) {
            for child in children {
                add_node(dot, &id, label, child, counter);
            }
        }
    }

    let root_id = "n0";
    dot.push_str(&format!(
        "    {} [label=\"{}\"];\n",
        root_id,
        escape_label(root)
    ));
    let mut counter = 1usize;

    if let Some(children) = children_val.and_then(|v| v.as_array()) {
        for child in children {
            let label = if child.is_string() {
                child.as_str().unwrap_or("node").to_string()
            } else {
                child
                    .get("label")
                    .or_else(|| child.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("node")
                    .to_string()
            };
            let id = format!("n{}", counter);
            counter += 1;
            dot.push_str(&format!(
                "    {} [label=\"{}\"];\n",
                id,
                escape_label(&label)
            ));
            dot.push_str(&format!("    {} -> {};\n", root_id, id));

            // Recurse into nested children
            if let Some(grandchildren) = child.get("children").and_then(|v| v.as_array()) {
                for gc in grandchildren {
                    let gc_label = if gc.is_string() {
                        gc.as_str().unwrap_or("node").to_string()
                    } else {
                        gc.get("label")
                            .or_else(|| gc.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("node")
                            .to_string()
                    };
                    let gc_id = format!("n{}", counter);
                    counter += 1;
                    dot.push_str(&format!(
                        "    {} [label=\"{}\"];\n",
                        gc_id,
                        escape_label(&gc_label)
                    ));
                    dot.push_str(&format!("    {} -> {};\n", id, gc_id));
                }
            }
        }
    }

    dot.push('}');

    let mut out = format!("Tree DOT\n{}\n\n", "─".repeat(50));
    out.push_str(&dot);
    Ok(out)
}
