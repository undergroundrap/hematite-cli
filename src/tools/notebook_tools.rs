use serde_json::{json, Value};
use std::fs;

pub fn notebook_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["info", "cells", "source", "outputs", "stats"],
                "description": "info: notebook overview | cells: list all cells | source: extract source code | outputs: list cell outputs | stats: summary statistics"
            },
            "file": {"type": "string", "description": "Path to .ipynb file"},
            "json": {"type": "string", "description": "Inline notebook JSON string"},
            "type": {"type": "string", "description": "Filter by cell type: code/markdown/raw"},
            "limit": {"type": "integer", "description": "Max cells to show"}
        },
        "required": []
    })
}

fn load_notebook(args: &Value) -> Result<Value, String> {
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Cannot read '{}': {}", path, e))?;
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON in '{}': {}", path, e))
    } else if let Some(raw) = args.get("json").and_then(|v| v.as_str()) {
        serde_json::from_str(raw).map_err(|e| format!("Invalid JSON: {}", e))
    } else {
        Err("Provide 'file' (path to .ipynb) or 'json' (inline notebook JSON).".to_string())
    }
}

fn get_cells(nb: &Value) -> Vec<&Value> {
    // nbformat 4: top-level cells
    if let Some(arr) = nb.get("cells").and_then(|c| c.as_array()) {
        return arr.iter().collect();
    }
    // nbformat 3: worksheets[0].cells
    if let Some(ws) = nb.get("worksheets").and_then(|w| w.as_array()) {
        if let Some(first) = ws.first() {
            if let Some(arr) = first.get("cells").and_then(|c| c.as_array()) {
                return arr.iter().collect();
            }
        }
    }
    vec![]
}

fn cell_source(cell: &Value) -> String {
    match cell.get("source") {
        Some(Value::Array(lines)) => lines
            .iter()
            .filter_map(|l| l.as_str())
            .collect::<Vec<_>>()
            .join(""),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn cell_type(cell: &Value) -> &str {
    cell.get("cell_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
}

fn output_count(cell: &Value) -> usize {
    cell.get("outputs")
        .and_then(|o| o.as_array())
        .map(|a| a.len())
        .unwrap_or(0)
}

fn action_info(nb: &Value) -> String {
    let fmt = nb.get("nbformat").and_then(|v| v.as_u64()).unwrap_or(0);
    let fmt_minor = nb
        .get("nbformat_minor")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let meta = nb.get("metadata").unwrap_or(&Value::Null);
    let kernelspec = meta.get("kernelspec").unwrap_or(&Value::Null);
    let kernel_name = kernelspec
        .get("display_name")
        .and_then(|v| v.as_str())
        .or_else(|| kernelspec.get("name").and_then(|v| v.as_str()))
        .unwrap_or("unknown");
    let language = kernelspec
        .get("language")
        .and_then(|v| v.as_str())
        .or_else(|| {
            meta.get("language_info")
                .and_then(|li| li.get("name"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("unknown");

    let cells = get_cells(nb);
    let mut code_cells = 0usize;
    let mut md_cells = 0usize;
    let mut raw_cells = 0usize;
    let mut total_source_lines = 0usize;
    let mut total_outputs = 0usize;

    for c in &cells {
        match cell_type(c) {
            "code" => {
                code_cells += 1;
                total_outputs += output_count(c);
            }
            "markdown" => md_cells += 1,
            _ => raw_cells += 1,
        }
        let src = cell_source(c);
        total_source_lines += src.lines().count();
    }

    let mut out = format!("Jupyter Notebook\n{}\n\n", "=".repeat(40));
    out += &format!("Format:         nbformat {}.{}\n", fmt, fmt_minor);
    out += &format!("Kernel:         {}\n", kernel_name);
    out += &format!("Language:       {}\n", language);
    out += "\nCells\n-----\n";
    out += &format!("  Code:         {}\n", code_cells);
    out += &format!("  Markdown:     {}\n", md_cells);
    out += &format!("  Raw:          {}\n", raw_cells);
    out += &format!("  Total:        {}\n", cells.len());
    out += "\nContent\n-------\n";
    out += &format!("  Source lines: {}\n", total_source_lines);
    out += &format!("  Total outputs:{}\n", total_outputs);

    // Optional metadata fields
    if let Some(title) = meta.get("title").and_then(|v| v.as_str()) {
        out += &format!("\nTitle:          {}\n", title);
    }
    if let Some(authors) = meta.get("authors").and_then(|v| v.as_array()) {
        let names: Vec<&str> = authors
            .iter()
            .filter_map(|a| a.get("name").and_then(|v| v.as_str()))
            .collect();
        if !names.is_empty() {
            out += &format!("Authors:        {}\n", names.join(", "));
        }
    }

    out
}

fn action_cells(args: &Value, nb: &Value) -> String {
    let type_filter = args.get("type").and_then(|v| v.as_str());
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(u64::MAX) as usize;

    let cells = get_cells(nb);
    let mut out = format!(
        "{:<5} {:<10} {:<8} {:<8} {}\n",
        "Index", "Type", "Lines", "Outputs", "Exec#"
    );
    out += &format!("{}\n", "-".repeat(45));

    let mut shown = 0;
    for (i, cell) in cells.iter().enumerate() {
        let ct = cell_type(cell);
        if let Some(f) = type_filter {
            if ct != f {
                continue;
            }
        }
        if shown >= limit {
            break;
        }
        let src = cell_source(cell);
        let lines = src.lines().count();
        let outputs = output_count(cell);
        let exec = cell
            .get("execution_count")
            .and_then(|v| v.as_u64())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        out += &format!("{:<5} {:<10} {:<8} {:<8} {}\n", i, ct, lines, outputs, exec);
        shown += 1;
    }

    if shown == 0 {
        out += "(no matching cells)\n";
    } else {
        out += &format!("\n{} cell(s) shown", shown);
        if let Some(f) = type_filter {
            out += &format!(" (type={})", f);
        }
        out += "\n";
    }

    out
}

fn action_source(args: &Value, nb: &Value) -> String {
    let type_filter = args.get("type").and_then(|v| v.as_str()).unwrap_or("code");
    let cells = get_cells(nb);
    let mut out = String::new();
    let mut code_idx = 0;

    for (i, cell) in cells.iter().enumerate() {
        let ct = cell_type(cell);
        if ct != type_filter {
            continue;
        }
        let src = cell_source(cell);
        if src.trim().is_empty() {
            continue;
        }
        if !out.is_empty() {
            out += "\n";
        }
        out += &format!("# --- Cell {} (index {}) ---\n", code_idx, i);
        out += &src;
        if !src.ends_with('\n') {
            out += "\n";
        }
        code_idx += 1;
    }

    if out.is_empty() {
        format!("No {} cells with source found.", type_filter)
    } else {
        out
    }
}

fn action_outputs(nb: &Value) -> String {
    let cells = get_cells(nb);
    let mut out = String::new();
    let mut found = 0;

    for (i, cell) in cells.iter().enumerate() {
        if cell_type(cell) != "code" {
            continue;
        }
        let outputs = match cell.get("outputs").and_then(|o| o.as_array()) {
            Some(a) if !a.is_empty() => a,
            _ => continue,
        };
        for (j, output) in outputs.iter().enumerate() {
            let otype = output
                .get("output_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            out += &format!("Cell {} Output {}  [{}]\n", i, j, otype);

            match otype {
                "error" => {
                    let ename = output.get("ename").and_then(|v| v.as_str()).unwrap_or("?");
                    let evalue = output.get("evalue").and_then(|v| v.as_str()).unwrap_or("");
                    out += &format!("  {}: {}\n", ename, &evalue[..evalue.len().min(120)]);
                }
                "stream" => {
                    let text = match output.get("text") {
                        Some(Value::Array(lines)) => lines
                            .iter()
                            .filter_map(|l| l.as_str())
                            .collect::<Vec<_>>()
                            .join(""),
                        Some(Value::String(s)) => s.clone(),
                        _ => String::new(),
                    };
                    let preview = &text[..text.len().min(200)];
                    let name = output.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    out += &format!("  [{}] {}\n", name, preview.replace('\n', " "));
                }
                "display_data" | "execute_result" => {
                    // Try text/plain first
                    let text = output
                        .get("data")
                        .and_then(|d| d.get("text/plain"))
                        .and_then(|v| match v {
                            Value::Array(lines) => Some(
                                lines
                                    .iter()
                                    .filter_map(|l| l.as_str())
                                    .collect::<Vec<_>>()
                                    .join(""),
                            ),
                            Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let preview = &text[..text.len().min(200)];
                    out += &format!("  {}\n", preview.replace('\n', " "));
                }
                _ => {}
            }
            found += 1;
        }
    }

    if found == 0 {
        "No outputs found in any code cell.".to_string()
    } else {
        out
    }
}

fn action_stats(nb: &Value) -> String {
    let cells = get_cells(nb);
    let mut code = 0usize;
    let mut markdown = 0usize;
    let mut raw = 0usize;
    let mut source_lines = 0usize;
    let mut total_outputs = 0usize;
    let mut cells_with_errors = 0usize;
    let mut code_no_output = 0usize;

    for cell in &cells {
        let ct = cell_type(cell);
        let src = cell_source(cell);
        source_lines += src.lines().count();
        match ct {
            "code" => {
                code += 1;
                let outs = cell
                    .get("outputs")
                    .and_then(|o| o.as_array())
                    .map(|a| a.as_slice())
                    .unwrap_or(&[]);
                let n = outs.len();
                total_outputs += n;
                if n == 0 {
                    code_no_output += 1;
                }
                let has_error = outs.iter().any(|o| {
                    o.get("output_type").and_then(|v| v.as_str()).unwrap_or("") == "error"
                });
                if has_error {
                    cells_with_errors += 1;
                }
            }
            "markdown" => markdown += 1,
            _ => raw += 1,
        }
    }

    let meta = nb.get("metadata").unwrap_or(&Value::Null);
    let kernel = meta
        .get("kernelspec")
        .and_then(|k| k.get("display_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let lang = meta
        .get("kernelspec")
        .and_then(|k| k.get("language"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            meta.get("language_info")
                .and_then(|li| li.get("name"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("unknown");

    let mut out = format!("Notebook Statistics\n{}\n\n", "=".repeat(40));
    out += &format!("Kernel:              {}\n", kernel);
    out += &format!("Language:            {}\n", lang);
    out += &format!("Total cells:         {}\n", cells.len());
    out += &format!("  Code cells:        {}\n", code);
    out += &format!("  Markdown cells:    {}\n", markdown);
    out += &format!("  Raw cells:         {}\n", raw);
    out += &format!("Total source lines:  {}\n", source_lines);
    out += &format!("Total outputs:       {}\n", total_outputs);
    out += &format!("Cells with errors:   {}\n", cells_with_errors);
    out += &format!("Code cells w/o output: {}\n", code_no_output);
    out
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let nb = load_notebook(args)?;

    Ok(match action {
        "info" => action_info(&nb),
        "cells" => action_cells(args, &nb),
        "source" => action_source(args, &nb),
        "outputs" => action_outputs(&nb),
        "stats" => action_stats(&nb),
        other => format!(
            "Unknown action '{}'. Use: info, cells, source, outputs, stats",
            other
        ),
    })
}
