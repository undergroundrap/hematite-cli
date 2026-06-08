use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("flowchart");
    match action {
        "flowchart" | "flow" => action_flowchart(args),
        "sequence" | "seq" => action_sequence(args),
        "class" => action_class(args),
        "gantt" => action_gantt(args),
        "pie" => action_pie(args),
        "er" => action_er(args),
        other => Err(format!(
            "mermaid_tools: unknown action '{other}'. Valid: flowchart, sequence, class, gantt, pie, er"
        )),
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn safe_id(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn mermaid_node_str(id: &str, label: &str, shape: &str) -> String {
    let esc = label.replace('"', "'");
    match shape {
        "diamond" | "decision" => format!("    {}{{\"{}\"}}\n", id, esc),
        "circle" | "start" | "end" => {
            // ((label))
            format!("    {}((\"{}\")\n", id, esc)
        }
        "stadium" | "pill" => format!("    {}([\"{}\"]\n", id, esc),
        "cylinder" | "db" => format!("    {}[(\"{}\"]\n", id, esc),
        _ => format!("    {}[\"{}\"]\n", id, esc),
    }
}

// ── flowchart ─────────────────────────────────────────────────────────────────

fn action_flowchart(args: &Value) -> Result<String, String> {
    let direction = args
        .get("direction")
        .or_else(|| args.get("dir"))
        .and_then(|v| v.as_str())
        .unwrap_or("TD");

    let direction = match direction.to_uppercase().as_str() {
        "LR" | "LEFT" => "LR",
        "RL" | "RIGHT" => "RL",
        "BT" | "BOTTOM" => "BT",
        _ => "TD",
    };

    let mut out = format!("flowchart {}\n", direction);

    // nodes: [{id, label, shape?}] or just strings
    if let Some(nodes) = args.get("nodes").and_then(|v| v.as_array()) {
        for n in nodes {
            if let Some(s) = n.as_str() {
                let id = safe_id(s);
                out.push_str(&format!("    {}[\"{}\"]\n", id, s));
            } else if let Some(obj) = n.as_object() {
                let id = obj
                    .get("id")
                    .or_else(|| obj.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("node");
                let label = obj.get("label").and_then(|v| v.as_str()).unwrap_or(id);
                let shape = obj.get("shape").and_then(|v| v.as_str()).unwrap_or("box");
                let safe = safe_id(id);
                let node_str = mermaid_node_str(&safe, label, shape);
                out.push_str(&node_str);
            }
        }
    }

    // edges: [{from, to, label?}] or [[from, to, label?]]
    if let Some(edges) = args.get("edges").and_then(|v| v.as_array()) {
        for e in edges {
            if let Some(obj) = e.as_object() {
                let from = match obj
                    .get("from")
                    .or_else(|| obj.get("source"))
                    .or_else(|| obj.get("a"))
                    .and_then(|v| v.as_str())
                {
                    Some(v) => v,
                    None => continue,
                };
                let to = match obj
                    .get("to")
                    .or_else(|| obj.get("target"))
                    .or_else(|| obj.get("b"))
                    .and_then(|v| v.as_str())
                {
                    Some(v) => v,
                    None => continue,
                };
                let label = obj.get("label").and_then(|v| v.as_str());
                let arrow = obj.get("style").and_then(|v| v.as_str()).unwrap_or("-->");
                if let Some(lbl) = label {
                    out.push_str(&format!(
                        "    {} {}|\"{}\"| {}\n",
                        safe_id(from),
                        arrow,
                        lbl,
                        safe_id(to)
                    ));
                } else {
                    out.push_str(&format!(
                        "    {} {} {}\n",
                        safe_id(from),
                        arrow,
                        safe_id(to)
                    ));
                }
            } else if let Some(arr) = e.as_array() {
                if arr.len() >= 2 {
                    let from = arr[0].as_str().unwrap_or("a");
                    let to = arr[1].as_str().unwrap_or("b");
                    let label = arr.get(2).and_then(|v| v.as_str());
                    if let Some(lbl) = label {
                        out.push_str(&format!(
                            "    {} -->|\"{}\"| {}\n",
                            safe_id(from),
                            lbl,
                            safe_id(to)
                        ));
                    } else {
                        out.push_str(&format!("    {} --> {}\n", safe_id(from), safe_id(to)));
                    }
                }
            }
        }
    }

    // steps shorthand: sequential chain
    if let Some(steps) = args.get("steps").and_then(|v| v.as_array()) {
        let step_strs: Vec<&str> = steps.iter().filter_map(|v| v.as_str()).collect();
        for (i, step) in step_strs.iter().enumerate() {
            out.push_str(&format!("    step{}[\"{}\"]\n", i, step));
        }
        for i in 0..step_strs.len().saturating_sub(1) {
            out.push_str(&format!("    step{} --> step{}\n", i, i + 1));
        }
    }

    if out.trim() == format!("flowchart {}", direction).trim() {
        return Err(
            "mermaid_tools flowchart: pass 'nodes'+'edges' or 'steps' to generate a flowchart"
                .into(),
        );
    }

    let mut result = format!("Mermaid Flowchart\n{}\n\n```mermaid\n", "─".repeat(40));
    result.push_str(&out);
    result.push_str("```\n\nPaste the code block into any Mermaid renderer (GitHub, GitLab, Notion, Obsidian, mermaid.live).");
    Ok(result)
}

// ── sequence ──────────────────────────────────────────────────────────────────

fn action_sequence(args: &Value) -> Result<String, String> {
    // messages: [{from, to, text, type?}] where type: ->> sync arrow, -->> dashed, -x lost
    let messages = args
        .get("messages")
        .or_else(|| args.get("steps"))
        .and_then(|v| v.as_array())
        .ok_or("mermaid_tools sequence: pass 'messages' as [{from, to, text}] array")?;

    let mut out = String::from("sequenceDiagram\n");

    // Declare participants in order
    let mut participants: Vec<String> = Vec::new();
    for msg in messages {
        if let Some(obj) = msg.as_object() {
            let from = obj
                .get("from")
                .or_else(|| obj.get("actor"))
                .and_then(|v| v.as_str())
                .unwrap_or("A");
            let to = obj.get("to").and_then(|v| v.as_str()).unwrap_or("B");
            if !participants.contains(&from.to_string()) {
                participants.push(from.to_string());
            }
            if !participants.contains(&to.to_string()) {
                participants.push(to.to_string());
            }
        }
    }
    // Optional explicit participant list
    if let Some(ps) = args.get("participants").and_then(|v| v.as_array()) {
        for p in ps {
            if let Some(s) = p.as_str() {
                if !participants.contains(&s.to_string()) {
                    participants.push(s.to_string());
                }
            }
        }
    }
    for p in &participants {
        out.push_str(&format!("    participant {}\n", p));
    }
    out.push('\n');

    for msg in messages {
        if let Some(obj) = msg.as_object() {
            let from = obj
                .get("from")
                .or_else(|| obj.get("actor"))
                .and_then(|v| v.as_str())
                .unwrap_or("A");
            let to = obj.get("to").and_then(|v| v.as_str()).unwrap_or("B");
            let text = obj
                .get("text")
                .or_else(|| obj.get("label"))
                .and_then(|v| v.as_str())
                .unwrap_or("message");
            let arrow = match obj.get("type").and_then(|v| v.as_str()).unwrap_or("sync") {
                "async" | "dashed" | "--" => "-->>",
                "lost" | "-x" => "-x",
                "dashed-x" | "--x" => "--x",
                _ => "->>",
            };
            out.push_str(&format!("    {} {} {}: {}\n", from, arrow, to, text));

            if let Some(note) = obj.get("note").and_then(|v| v.as_str()) {
                let over = obj
                    .get("note_over")
                    .and_then(|v| v.as_str())
                    .unwrap_or(from);
                out.push_str(&format!("    Note over {}: {}\n", over, note));
            }
        }
    }

    let mut result = format!(
        "Mermaid Sequence Diagram\n{}\n\n```mermaid\n",
        "─".repeat(40)
    );
    result.push_str(&out);
    result.push_str("```\n\nPaste into GitHub, GitLab, Notion, Obsidian, or mermaid.live.");
    Ok(result)
}

// ── class diagram ─────────────────────────────────────────────────────────────

fn action_class(args: &Value) -> Result<String, String> {
    let classes = args.get("classes").and_then(|v| v.as_array()).ok_or(
        "mermaid_tools class: pass 'classes' as [{name, fields?, methods?, relationships?}] array",
    )?;

    let mut out = String::from("classDiagram\n");

    for cls in classes {
        if let Some(obj) = cls.as_object() {
            let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("Class");
            out.push_str(&format!("    class {} {{\n", name));

            if let Some(fields) = obj.get("fields").and_then(|v| v.as_array()) {
                for f in fields {
                    if let Some(s) = f.as_str() {
                        out.push_str(&format!("        +{}\n", s));
                    } else if let Some(fobj) = f.as_object() {
                        let vis = fobj
                            .get("visibility")
                            .and_then(|v| v.as_str())
                            .unwrap_or("+");
                        let typ = fobj.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        let fname = fobj.get("name").and_then(|v| v.as_str()).unwrap_or("field");
                        out.push_str(&format!("        {} {} {}\n", vis, typ, fname));
                    }
                }
            }
            if let Some(methods) = obj.get("methods").and_then(|v| v.as_array()) {
                for m in methods {
                    if let Some(s) = m.as_str() {
                        out.push_str(&format!("        +{}()\n", s));
                    } else if let Some(mobj) = m.as_object() {
                        let vis = mobj
                            .get("visibility")
                            .and_then(|v| v.as_str())
                            .unwrap_or("+");
                        let mname = mobj
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("method");
                        let ret = mobj
                            .get("return")
                            .and_then(|v| v.as_str())
                            .unwrap_or("void");
                        let params = mobj.get("params").and_then(|v| v.as_str()).unwrap_or("");
                        out.push_str(&format!("        {} {}({}) {}\n", vis, mname, params, ret));
                    }
                }
            }
            out.push_str("    }\n");

            if let Some(rels) = obj.get("relationships").and_then(|v| v.as_array()) {
                for r in rels {
                    if let Some(robj) = r.as_object() {
                        let target = robj
                            .get("target")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Other");
                        let rel = match robj
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("association")
                        {
                            "inheritance" | "extends" => "<|--",
                            "composition" => "*--",
                            "aggregation" => "o--",
                            "dependency" => "<..>",
                            "realization" | "implements" => "<|..",
                            _ => "--",
                        };
                        let label = robj.get("label").and_then(|v| v.as_str());
                        if let Some(lbl) = label {
                            out.push_str(&format!("    {} {} {} : {}\n", name, rel, target, lbl));
                        } else {
                            out.push_str(&format!("    {} {} {}\n", name, rel, target));
                        }
                    } else if let Some(s) = r.as_str() {
                        out.push_str(&format!("    {} -- {}\n", name, s));
                    }
                }
            }
        }
    }

    let mut result = format!("Mermaid Class Diagram\n{}\n\n```mermaid\n", "─".repeat(40));
    result.push_str(&out);
    result.push_str("```\n\nPaste into GitHub, GitLab, Notion, Obsidian, or mermaid.live.");
    Ok(result)
}

// ── gantt ─────────────────────────────────────────────────────────────────────

fn action_gantt(args: &Value) -> Result<String, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Project Timeline");
    let date_format = args
        .get("date_format")
        .and_then(|v| v.as_str())
        .unwrap_or("YYYY-MM-DD");
    let sections = args
        .get("sections")
        .and_then(|v| v.as_array())
        .ok_or("mermaid_tools gantt: pass 'sections' as [{name, tasks: [{name, start, duration, status?}]}]")?;

    let mut out = format!(
        "gantt\n    title {}\n    dateFormat {}\n\n",
        title, date_format
    );

    for section in sections {
        if let Some(sobj) = section.as_object() {
            let sname = sobj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Section");
            out.push_str(&format!("    section {}\n", sname));
            if let Some(tasks) = sobj.get("tasks").and_then(|v| v.as_array()) {
                for task in tasks {
                    if let Some(tobj) = task.as_object() {
                        let tname = tobj.get("name").and_then(|v| v.as_str()).unwrap_or("Task");
                        let start = tobj
                            .get("start")
                            .and_then(|v| v.as_str())
                            .unwrap_or("2025-01-01");
                        let dur = tobj
                            .get("duration")
                            .or_else(|| tobj.get("dur"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("1d");
                        let status = tobj
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("active");
                        out.push_str(&format!("    {} : {}, {}, {}\n", tname, status, start, dur));
                    } else if let Some(s) = task.as_str() {
                        out.push_str(&format!("    {} : active, 2025-01-01, 1d\n", s));
                    }
                }
            }
        }
    }

    let mut result = format!("Mermaid Gantt Chart\n{}\n\n```mermaid\n", "─".repeat(40));
    result.push_str(&out);
    result.push_str("```\n\nPaste into GitHub, GitLab, Notion, Obsidian, or mermaid.live.");
    Ok(result)
}

// ── pie chart ─────────────────────────────────────────────────────────────────

fn action_pie(args: &Value) -> Result<String, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Distribution");
    let data = args
        .get("data")
        .or_else(|| args.get("slices"))
        .and_then(|v| v.as_object())
        .ok_or("mermaid_tools pie: pass 'data' as {\"label\": value, ...}")?;

    let mut out = format!("pie title {}\n", title);
    for (label, val) in data {
        let v = val.as_f64().unwrap_or(0.0);
        out.push_str(&format!("    \"{}\" : {}\n", label, v));
    }

    let mut result = format!("Mermaid Pie Chart\n{}\n\n```mermaid\n", "─".repeat(40));
    result.push_str(&out);
    result.push_str("```\n\nPaste into GitHub, GitLab, Notion, Obsidian, or mermaid.live.");
    Ok(result)
}

// ── entity-relationship ───────────────────────────────────────────────────────

fn action_er(args: &Value) -> Result<String, String> {
    let entities = args
        .get("entities")
        .and_then(|v| v.as_array())
        .ok_or("mermaid_tools er: pass 'entities' as [{name, attributes?}] and optional 'relationships' [{left, right, cardinality, label}]")?;

    let mut out = String::from("erDiagram\n");

    for ent in entities {
        if let Some(eobj) = ent.as_object() {
            let ename = eobj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Entity");
            out.push_str(&format!("    {} {{\n", ename));
            if let Some(attrs) = eobj.get("attributes").and_then(|v| v.as_array()) {
                for attr in attrs {
                    if let Some(s) = attr.as_str() {
                        out.push_str(&format!("        string {}\n", s));
                    } else if let Some(aobj) = attr.as_object() {
                        let atype = aobj
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("string");
                        let aname = aobj.get("name").and_then(|v| v.as_str()).unwrap_or("field");
                        let pk = aobj.get("pk").and_then(|v| v.as_bool()).unwrap_or(false);
                        if pk {
                            out.push_str(&format!("        {} {} PK\n", atype, aname));
                        } else {
                            out.push_str(&format!("        {} {}\n", atype, aname));
                        }
                    }
                }
            }
            out.push_str("    }\n");
        }
    }

    if let Some(rels) = args.get("relationships").and_then(|v| v.as_array()) {
        out.push('\n');
        for r in rels {
            if let Some(robj) = r.as_object() {
                let left = robj.get("left").and_then(|v| v.as_str()).unwrap_or("A");
                let right = robj.get("right").and_then(|v| v.as_str()).unwrap_or("B");
                let card = match robj
                    .get("cardinality")
                    .and_then(|v| v.as_str())
                    .unwrap_or("many-to-one")
                {
                    "one-to-one" | "1:1" => "||--||",
                    "one-to-many" | "1:N" => "||--o{",
                    "many-to-one" | "N:1" => "}o--||",
                    "many-to-many" | "N:N" => "}o--o{",
                    "zero-or-one" => "|o--||",
                    _ => "||--||",
                };
                let label = robj.get("label").and_then(|v| v.as_str()).unwrap_or("has");
                out.push_str(&format!(
                    "    {} {} {} : \"{}\"\n",
                    left, card, right, label
                ));
            }
        }
    }

    let mut result = format!("Mermaid ER Diagram\n{}\n\n```mermaid\n", "─".repeat(40));
    result.push_str(&out);
    result.push_str("```\n\nPaste into GitHub, GitLab, Notion, Obsidian, or mermaid.live.");
    Ok(result)
}
