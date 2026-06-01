use serde_json::{json, Value};

pub fn text_align_tools_schema() -> Value {
    json!({
        "name": "text_align_tools",
        "description": "Text alignment and column formatting without external utilities. Actions: align (left/right/center/justify a block of text to a target width), columns (format multiple text columns side-by-side with configurable separator and per-column alignment), indent (add/remove indentation from each line), normalize (normalize inconsistent whitespace — collapse multiple spaces, strip trailing whitespace, standardize line endings), center_block (center an entire block of lines as a unit), ruler (generate a character ruler for alignment reference at a given width).",
        "parameters": {
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Input text to align or format"
                },
                "action": {
                    "type": "string",
                    "enum": ["align", "columns", "indent", "normalize", "center_block", "ruler"],
                    "description": "Action to perform (default: align)"
                },
                "alignment": {
                    "type": "string",
                    "enum": ["left", "right", "center", "justify"],
                    "description": "Alignment direction for 'align' action (default: left)"
                },
                "width": {
                    "type": "integer",
                    "description": "Target width in characters (default: 80)"
                },
                "fill": {
                    "type": "string",
                    "description": "Fill character for padding (default: space)"
                },
                "columns": {
                    "type": "array",
                    "description": "Array of column data objects for 'columns' action — each: {text, align?, width?}",
                    "items": {"type": "object"}
                },
                "separator": {
                    "type": "string",
                    "description": "Column separator string for 'columns' action (default: '  ')"
                },
                "indent_width": {
                    "type": "integer",
                    "description": "Number of spaces to add (positive) or remove (negative) for 'indent' action (default: 4)"
                },
                "indent_char": {
                    "type": "string",
                    "description": "Indent character for 'indent' action — space or tab (default: space)"
                }
            },
            "required": []
        }
    })
}

// ── alignment helpers ─────────────────────────────────────────────────────────

fn visible_len(s: &str) -> usize {
    s.chars().count()
}

fn align_line(line: &str, alignment: &str, width: usize, fill: char) -> String {
    let len = visible_len(line);
    if len >= width {
        return line.to_string();
    }
    let pad = width - len;
    match alignment {
        "right" => {
            format!("{}{}", fill.to_string().repeat(pad), line)
        }
        "center" => {
            let left_pad = pad / 2;
            let right_pad = pad - left_pad;
            format!(
                "{}{}{}",
                fill.to_string().repeat(left_pad),
                line,
                fill.to_string().repeat(right_pad)
            )
        }
        "justify" => justify_line(line, width),
        _ => {
            // left
            format!("{}{}", line, fill.to_string().repeat(pad))
        }
    }
}

fn justify_line(line: &str, width: usize) -> String {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() <= 1 {
        // Can't justify a single word — return left-aligned
        return format!("{:<width$}", line);
    }
    let word_total: usize = words.iter().map(|w| visible_len(w)).sum();
    let total_spaces = width.saturating_sub(word_total);
    let gaps = words.len() - 1;
    let base_space = total_spaces / gaps;
    let extra = total_spaces % gaps;

    let mut out = words[0].to_string();
    for (i, word) in words[1..].iter().enumerate() {
        let spaces = base_space + if i < extra { 1 } else { 0 };
        out.push_str(&" ".repeat(spaces));
        out.push_str(word);
    }
    out
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_align(args: &Value) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("'text' field is required")?;

    let alignment = args
        .get("alignment")
        .and_then(|v| v.as_str())
        .unwrap_or("left");

    let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(80) as usize;

    let fill_str = args.get("fill").and_then(|v| v.as_str()).unwrap_or(" ");
    let fill = fill_str.chars().next().unwrap_or(' ');

    let lines: Vec<String> = text
        .lines()
        .map(|l| align_line(l, alignment, width, fill))
        .collect();

    Ok(lines.join("\n") + "\n")
}

fn action_columns(args: &Value) -> Result<String, String> {
    let cols_val = args
        .get("columns")
        .and_then(|v| v.as_array())
        .ok_or("'columns' array is required for columns action")?;

    let separator = args
        .get("separator")
        .and_then(|v| v.as_str())
        .unwrap_or("  ");

    // Parse each column: text, align, width
    struct Col {
        lines: Vec<String>,
        align: String,
        width: usize,
    }

    let mut cols: Vec<Col> = Vec::new();
    for col_val in cols_val {
        let text = col_val.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let align = col_val
            .get("align")
            .and_then(|v| v.as_str())
            .unwrap_or("left")
            .to_string();
        let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
        let natural_width = lines.iter().map(|l| visible_len(l)).max().unwrap_or(0);
        let width = col_val
            .get("width")
            .and_then(|v| v.as_u64())
            .map(|w| w as usize)
            .unwrap_or(natural_width);
        cols.push(Col {
            lines,
            align,
            width,
        });
    }

    if cols.is_empty() {
        return Ok(String::new());
    }

    let row_count = cols.iter().map(|c| c.lines.len()).max().unwrap_or(0);
    let mut out_lines: Vec<String> = Vec::new();

    for row in 0..row_count {
        let mut parts: Vec<String> = Vec::new();
        for col in &cols {
            let line = col.lines.get(row).map(|s| s.as_str()).unwrap_or("");
            parts.push(align_line(line, &col.align, col.width, ' '));
        }
        out_lines.push(parts.join(separator));
    }

    Ok(out_lines.join("\n") + "\n")
}

fn action_indent(args: &Value) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("'text' field is required")?;

    let indent_width = args
        .get("indent_width")
        .and_then(|v| v.as_i64())
        .unwrap_or(4);

    let indent_char = args
        .get("indent_char")
        .and_then(|v| v.as_str())
        .unwrap_or(" ");
    let ic = indent_char.chars().next().unwrap_or(' ');

    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            if indent_width >= 0 {
                let prefix = ic.to_string().repeat(indent_width as usize);
                format!("{}{}", prefix, line)
            } else {
                // Remove up to |indent_width| leading characters
                let remove = (-indent_width) as usize;
                let leading: usize = line.chars().take(remove).take_while(|&c| c == ic).count();
                line[leading..].to_string()
            }
        })
        .collect();

    Ok(lines.join("\n") + "\n")
}

fn action_normalize(args: &Value) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("'text' field is required")?;

    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            // Collapse runs of spaces (but not leading/trailing — just internal)
            let trimmed = line.trim_end();
            let mut result = String::new();
            let mut prev_space = false;
            let mut at_start = true;
            for c in trimmed.chars() {
                if c == ' ' || c == '\t' {
                    if at_start {
                        // Preserve leading whitespace as-is
                        result.push(c);
                    } else if !prev_space {
                        result.push(' ');
                        prev_space = true;
                    }
                } else {
                    at_start = false;
                    prev_space = false;
                    result.push(c);
                }
            }
            result
        })
        .collect();

    Ok(lines.join("\n") + "\n")
}

fn action_center_block(args: &Value) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("'text' field is required")?;

    let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(80) as usize;

    let lines: Vec<&str> = text.lines().collect();
    // Find the width of the widest line
    let block_width = lines.iter().map(|l| visible_len(l)).max().unwrap_or(0);
    let pad = width.saturating_sub(block_width) / 2;
    let prefix = " ".repeat(pad);

    let result: Vec<String> = lines
        .iter()
        .map(|line| format!("{}{}", prefix, line))
        .collect();

    Ok(result.join("\n") + "\n")
}

fn action_ruler(args: &Value) -> Result<String, String> {
    let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(80) as usize;

    let mut top = String::new();
    let mut bot = String::new();
    let mut nums = String::new();

    for i in 1..=width {
        if i % 10 == 0 {
            let s = i.to_string();
            // Replace last digits of nums with the number
            let pad = 10 - s.len();
            nums.push_str(&" ".repeat(pad));
            nums.push_str(&s);
            top.push('┤');
            bot.push('┤');
        } else if i % 5 == 0 {
            top.push('┼');
            bot.push('┼');
            nums.push(' ');
        } else {
            top.push('─');
            bot.push('─');
            nums.push(' ');
        }
    }

    // Simple tick ruler
    let mut ticks = String::new();
    for i in 1..=width {
        if i % 10 == 0 {
            ticks.push('|');
        } else if i % 5 == 0 {
            ticks.push('+');
        } else {
            ticks.push('·');
        }
    }

    // Number row (tens only)
    let mut number_row = String::new();
    let mut col = 0;
    while col < width {
        let next_ten = ((col / 10) + 1) * 10;
        if next_ten <= width {
            let gap = next_ten - col - 1;
            number_row.push_str(&" ".repeat(gap));
            let s = next_ten.to_string();
            number_row.push_str(&s);
            col = next_ten;
        } else {
            break;
        }
    }

    let mut out = format!("Ruler ({} chars):\n\n", width);
    out.push_str(&format!("{}\n", ticks));
    out.push_str(&format!("{}\n", number_row));
    Ok(out)
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("align");

    match action {
        "align" => action_align(args),
        "columns" => action_columns(args),
        "indent" => action_indent(args),
        "normalize" => action_normalize(args),
        "center_block" => action_center_block(args),
        "ruler" => action_ruler(args),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: align, columns, indent, normalize, center_block, ruler"
        )),
    }
}
