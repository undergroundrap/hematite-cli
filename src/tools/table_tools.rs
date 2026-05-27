pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("format");

    match action {
        "format" => format_table(args),
        "from-csv" => from_csv(args),
        "from-json" => from_json(args),
        "to-markdown" => to_markdown(args),
        "transpose" => transpose(args),
        other => Err(format!(
            "table_tools: unknown action '{other}'. Valid: format, from-csv, from-json, to-markdown, transpose"
        )),
    }
}

// ── Core table renderer ────────────────────────────────────────────────────────

fn col_widths(headers: &[String], rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i >= widths.len() {
                widths.push(cell.len());
            } else {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }
    widths.iter().map(|&w| w.max(1)).collect()
}

fn render_simple(headers: &[String], rows: &[Vec<String>], widths: &[usize]) -> String {
    let mut out = String::new();

    if !headers.is_empty() {
        let header_line: String = headers
            .iter()
            .enumerate()
            .map(|(i, h)| format!("{:<width$}", h, width = widths.get(i).copied().unwrap_or(1)))
            .collect::<Vec<_>>()
            .join("  ");
        out.push_str(&header_line);
        out.push('\n');

        let sep_line: String = widths
            .iter()
            .map(|&w| "-".repeat(w))
            .collect::<Vec<_>>()
            .join("  ");
        out.push_str(&sep_line);
        out.push('\n');
    }

    for row in rows {
        let row_line: String = (0..widths.len())
            .map(|i| {
                let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                format!("{:<width$}", cell, width = widths[i])
            })
            .collect::<Vec<_>>()
            .join("  ");
        out.push_str(row_line.trim_end());
        out.push('\n');
    }

    out
}

fn render_bordered(headers: &[String], rows: &[Vec<String>], widths: &[usize]) -> String {
    let bar = |ch: &str| {
        widths
            .iter()
            .map(|&w| ch.repeat(w + 2))
            .collect::<Vec<_>>()
            .join("+")
    };

    let top = format!("+{}+", bar("-"));
    let mid = format!("+{}+", bar("-"));
    let bot = format!("+{}+", bar("-"));

    let fmt_row = |cells: &[String]| -> String {
        let inner: String = (0..widths.len())
            .map(|i| {
                let cell = cells.get(i).map(|s| s.as_str()).unwrap_or("");
                format!(" {:<width$} ", cell, width = widths[i])
            })
            .collect::<Vec<_>>()
            .join("|");
        format!("|{inner}|")
    };

    let mut out = String::new();
    out.push_str(&top);
    out.push('\n');

    if !headers.is_empty() {
        out.push_str(&fmt_row(headers));
        out.push('\n');
        out.push_str(&mid);
        out.push('\n');
    }

    for row in rows {
        out.push_str(&fmt_row(row));
        out.push('\n');
    }

    out.push_str(&bot);
    out.push('\n');
    out
}

fn render_markdown(headers: &[String], rows: &[Vec<String>], widths: &[usize]) -> String {
    let fmt_row = |cells: &[String]| -> String {
        let inner: String = (0..widths.len())
            .map(|i| {
                let cell = cells.get(i).map(|s| s.as_str()).unwrap_or("");
                format!(" {:<width$} ", cell, width = widths[i])
            })
            .collect::<Vec<_>>()
            .join("|");
        format!("|{inner}|")
    };

    let sep_row: String = widths
        .iter()
        .map(|&w| format!(" {} ", "-".repeat(w)))
        .collect::<Vec<_>>()
        .join("|");
    let sep = format!("|{sep_row}|");

    let mut out = String::new();

    if !headers.is_empty() {
        out.push_str(&fmt_row(headers));
        out.push('\n');
        out.push_str(&sep);
        out.push('\n');
    } else {
        // No headers — still need a separator row to be valid markdown
        let blank_headers: Vec<String> = widths.iter().map(|_| String::new()).collect();
        out.push_str(&fmt_row(&blank_headers));
        out.push('\n');
        out.push_str(&sep);
        out.push('\n');
    }

    for row in rows {
        out.push_str(&fmt_row(row));
        out.push('\n');
    }

    out
}

fn choose_renderer<'a>(
    style: &str,
    headers: &'a [String],
    rows: &'a [Vec<String>],
    widths: &'a [usize],
) -> String {
    match style {
        "bordered" | "box" => render_bordered(headers, rows, widths),
        "markdown" | "md" => render_markdown(headers, rows, widths),
        _ => render_simple(headers, rows, widths),
    }
}

// ── CSV parser (minimal, RFC-4180 compliant) ───────────────────────────────────

fn parse_csv_text(text: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut in_quotes = false;
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '"' if in_quotes => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        cur.push('"');
                    } else {
                        in_quotes = false;
                    }
                }
                '"' => in_quotes = true,
                ',' if !in_quotes => {
                    fields.push(cur.trim().to_string());
                    cur.clear();
                }
                other => cur.push(other),
            }
        }
        fields.push(cur.trim().to_string());
        rows.push(fields);
    }
    rows
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn format_table(args: &serde_json::Value) -> Result<String, String> {
    let style = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("simple");

    let headers: Vec<String> = args
        .get("headers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default();

    let rows: Vec<Vec<String>> = args
        .get("rows")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|row| {
                    row.as_array().map(|cells| {
                        cells
                            .iter()
                            .map(|c| {
                                if c.is_string() {
                                    c.as_str().unwrap_or("").to_string()
                                } else {
                                    c.to_string()
                                }
                            })
                            .collect()
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    if rows.is_empty() && headers.is_empty() {
        return Err("table_tools format: 'rows' (2D array) is required".to_string());
    }

    let widths = col_widths(&headers, &rows);
    let table = choose_renderer(style, &headers, &rows, &widths);

    let mut out = format!("TABLE FORMAT\n{}\n", "─".repeat(50));
    out.push_str(&table);
    out.push_str(&format!(
        "\n{} row(s), {} column(s)\n",
        rows.len(),
        widths.len()
    ));
    Ok(out)
}

fn from_csv(args: &serde_json::Value) -> Result<String, String> {
    let text = args
        .get("text")
        .or_else(|| args.get("csv"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("table_tools from-csv: 'text' or 'csv' is required")?;

    let style = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("simple");
    let has_header = args.get("header").and_then(|v| v.as_bool()).unwrap_or(true);

    let mut all_rows = parse_csv_text(text);
    if all_rows.is_empty() {
        return Err("table_tools from-csv: no data found in CSV input".to_string());
    }

    let headers = if has_header {
        all_rows.remove(0)
    } else {
        Vec::new()
    };
    let rows = all_rows;

    let widths = col_widths(&headers, &rows);
    let table = choose_renderer(style, &headers, &rows, &widths);

    let mut out = format!("TABLE FROM CSV\n{}\n", "─".repeat(50));
    out.push_str(&table);
    out.push_str(&format!(
        "\n{} row(s), {} column(s)\n",
        rows.len(),
        widths.len()
    ));
    Ok(out)
}

fn from_json(args: &serde_json::Value) -> Result<String, String> {
    let raw = args
        .get("json")
        .or_else(|| args.get("text"))
        .or_else(|| args.get("input"));

    let arr = if let Some(a) = raw.and_then(|v| v.as_array()) {
        a.to_vec()
    } else if let Some(s) = raw.and_then(|v| v.as_str()) {
        serde_json::from_str::<serde_json::Value>(s)
            .map_err(|e| format!("table_tools from-json: invalid JSON: {e}"))?
            .as_array()
            .ok_or("table_tools from-json: JSON must be an array")?
            .to_vec()
    } else {
        return Err(
            "table_tools from-json: 'json' (array of objects or array of arrays) is required"
                .to_string(),
        );
    };

    let style = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("simple");

    // Detect array-of-arrays vs array-of-objects
    let (headers, rows): (Vec<String>, Vec<Vec<String>>) =
        if arr.first().map(|v| v.is_object()).unwrap_or(false) {
            // Collect all unique keys in insertion order from first object
            let keys: Vec<String> = arr
                .first()
                .and_then(|v| v.as_object())
                .map(|obj| obj.keys().cloned().collect())
                .unwrap_or_default();

            let rows: Vec<Vec<String>> = arr
                .iter()
                .filter_map(|item| {
                    item.as_object().map(|obj| {
                        keys.iter()
                            .map(|k| {
                                obj.get(k)
                                    .map(|v| {
                                        if v.is_string() {
                                            v.as_str().unwrap_or("").to_string()
                                        } else {
                                            v.to_string()
                                        }
                                    })
                                    .unwrap_or_default()
                            })
                            .collect()
                    })
                })
                .collect();

            (keys, rows)
        } else {
            // Array of arrays
            let rows: Vec<Vec<String>> = arr
                .iter()
                .filter_map(|item| {
                    item.as_array().map(|cells| {
                        cells
                            .iter()
                            .map(|c| {
                                if c.is_string() {
                                    c.as_str().unwrap_or("").to_string()
                                } else {
                                    c.to_string()
                                }
                            })
                            .collect()
                    })
                })
                .collect();
            (Vec::new(), rows)
        };

    let widths = col_widths(&headers, &rows);
    let table = choose_renderer(style, &headers, &rows, &widths);

    let mut out = format!("TABLE FROM JSON\n{}\n", "─".repeat(50));
    out.push_str(&table);
    out.push_str(&format!(
        "\n{} row(s), {} column(s)\n",
        rows.len(),
        widths.len()
    ));
    Ok(out)
}

fn to_markdown(args: &serde_json::Value) -> Result<String, String> {
    // Delegate to format_table or from-csv/from-json with style=markdown
    let mut new_args = args.clone();
    if let Some(obj) = new_args.as_object_mut() {
        obj.insert(
            "style".to_string(),
            serde_json::Value::String("markdown".to_string()),
        );
    }

    if args.get("rows").is_some() {
        let result = format_table(&new_args)?;
        return Ok(result.replacen("TABLE FORMAT", "TABLE TO MARKDOWN", 1));
    }
    if args.get("text").is_some() || args.get("csv").is_some() {
        let result = from_csv(&new_args)?;
        return Ok(result.replacen("TABLE FROM CSV", "TABLE TO MARKDOWN", 1));
    }
    if args.get("json").is_some() {
        let result = from_json(&new_args)?;
        return Ok(result.replacen("TABLE FROM JSON", "TABLE TO MARKDOWN", 1));
    }

    Err("table_tools to-markdown: provide 'rows', 'text' (CSV), or 'json' as input".to_string())
}

fn transpose(args: &serde_json::Value) -> Result<String, String> {
    let style = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("simple");

    let headers: Vec<String> = args
        .get("headers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default();

    let rows: Vec<Vec<String>> = args
        .get("rows")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|row| {
                    row.as_array().map(|cells| {
                        cells
                            .iter()
                            .map(|c| {
                                if c.is_string() {
                                    c.as_str().unwrap_or("").to_string()
                                } else {
                                    c.to_string()
                                }
                            })
                            .collect()
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    if rows.is_empty() {
        return Err("table_tools transpose: 'rows' (2D array) is required".to_string());
    }

    // Build combined data (headers prepended as first row if present)
    let mut all: Vec<Vec<String>> = Vec::new();
    if !headers.is_empty() {
        all.push(headers);
    }
    all.extend(rows);

    // Transpose: new_rows[col][row]
    let max_cols = all.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut transposed: Vec<Vec<String>> = vec![Vec::new(); max_cols];
    for row in &all {
        for (col, cell) in row.iter().enumerate() {
            transposed[col].push(cell.clone());
        }
    }

    let widths = col_widths(&[], &transposed);
    let table = choose_renderer(style, &[], &transposed, &widths);

    let mut out = format!("TABLE TRANSPOSE\n{}\n", "─".repeat(50));
    out.push_str(&table);
    out.push_str(&format!(
        "\n{} row(s), {} column(s) (transposed from {} x {})\n",
        transposed.len(),
        widths.len(),
        all.len(),
        max_cols
    ));
    Ok(out)
}
