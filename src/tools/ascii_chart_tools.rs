use serde_json::{json, Value};

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
    ascii_chart_tools(action, args)
}

pub fn ascii_chart_tools(action: &str, input: &Value) -> Result<String, String> {
    match action {
        "bar" | "" => action_bar(input),
        "line" => action_line(input),
        "scatter" => action_scatter(input),
        "sparkline" => action_sparkline(input),
        "hbar" => action_hbar(input),
        _ => Err(format!(
            "Unknown action '{}'. Available: bar, line, scatter, sparkline, hbar",
            action
        )),
    }
}

fn get_numbers(input: &Value) -> Result<Vec<f64>, String> {
    let raw = input
        .get("data")
        .or_else(|| input.get("values"))
        .or_else(|| input.get("numbers"));
    match raw {
        Some(Value::Array(arr)) => {
            let mut nums = Vec::new();
            for v in arr {
                match v {
                    Value::Number(n) => nums.push(n.as_f64().unwrap_or(0.0)),
                    Value::String(s) => {
                        nums.push(s.parse::<f64>().map_err(|_| format!("Not a number: {s}"))?)
                    }
                    _ => return Err("Data must be an array of numbers".into()),
                }
            }
            Ok(nums)
        }
        Some(Value::String(s)) => s
            .split(',')
            .map(|p| {
                p.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("Not a number: {p}"))
            })
            .collect(),
        _ => Err("Missing 'data' field (array of numbers or comma-separated string)".into()),
    }
}

fn get_labels(input: &Value, count: usize) -> Vec<String> {
    if let Some(Value::Array(arr)) = input.get("labels") {
        arr.iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect()
    } else {
        (0..count).map(|i| i.to_string()).collect()
    }
}

fn action_bar(input: &Value) -> Result<String, String> {
    let data = get_numbers(input)?;
    if data.is_empty() {
        return Err("Data array is empty".into());
    }
    let labels = get_labels(input, data.len());
    let width = input.get("width").and_then(|v| v.as_u64()).unwrap_or(40) as usize;
    let title = input
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let style = input
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("block");

    let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let show_negative = min_val < 0.0;
    let abs_max = data.iter().map(|v| v.abs()).fold(0.0f64, f64::max);

    let fill_char = match style {
        "hash" => '#',
        "equals" => '=',
        "dot" => '·',
        "shade" => '▒',
        _ => '█',
    };
    let empty_char = ' ';

    let max_label_len = labels.iter().map(|l| l.len()).max().unwrap_or(0);

    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(&format!("  {title}\n"));
        out.push_str(&format!("  {}\n\n", "─".repeat(title.len())));
    }

    for (i, &val) in data.iter().enumerate() {
        let label = labels.get(i).map(|s| s.as_str()).unwrap_or("");
        let padded_label = format!("{:>width$}", label, width = max_label_len);

        let bar_len = if abs_max > 0.0 {
            ((val.abs() / abs_max) * width as f64).round() as usize
        } else {
            0
        };

        let bar: String = std::iter::repeat(fill_char)
            .take(bar_len)
            .chain(std::iter::repeat(empty_char).take(width - bar_len))
            .collect();

        if show_negative {
            let zero_pos = ((0.0f64.max(-min_val) / abs_max) * width as f64).round() as usize;
            let zero_pos = zero_pos.min(width);
            let mut row = vec![' '; width];
            if val >= 0.0 {
                let start = zero_pos;
                let end = (zero_pos + bar_len).min(width);
                for c in &mut row[start..end] {
                    *c = fill_char;
                }
            } else {
                let end = zero_pos;
                let start = end.saturating_sub(bar_len);
                for c in &mut row[start..end] {
                    *c = fill_char;
                }
            }
            row[zero_pos.min(width - 1)] = '│';
            let row_str: String = row.into_iter().collect();
            out.push_str(&format!("  {padded_label} │{row_str}│ {val:.2}\n"));
        } else {
            out.push_str(&format!("  {padded_label} │{bar}│ {val}\n"));
        }
    }

    // X-axis
    let axis = "─".repeat(width);
    out.push_str(&format!("  {}─┴{}─┘\n", " ".repeat(max_label_len), axis));
    out.push_str(&format!(
        "  {} 0{}{:.2}\n",
        " ".repeat(max_label_len),
        " ".repeat(width.saturating_sub(4)),
        max_val
    ));

    out.push_str(&format!(
        "\n  Min: {min_val}  Max: {max_val}  Count: {}",
        data.len()
    ));
    Ok(out)
}

fn action_hbar(input: &Value) -> Result<String, String> {
    // Horizontal bar chart is essentially the same as bar — alias
    action_bar(input)
}

fn action_line(input: &Value) -> Result<String, String> {
    let data = get_numbers(input)?;
    if data.len() < 2 {
        return Err("Line chart requires at least 2 data points".into());
    }
    let height = input.get("height").and_then(|v| v.as_u64()).unwrap_or(15) as usize;
    let width = input.get("width").and_then(|v| v.as_u64()).unwrap_or(60) as usize;
    let title = input
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let marker = input
        .get("marker")
        .and_then(|v| v.as_str())
        .and_then(|s| s.chars().next())
        .unwrap_or('•');

    let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = if (max_val - min_val).abs() < f64::EPSILON {
        1.0
    } else {
        max_val - min_val
    };

    // Build a 2D grid
    let mut grid = vec![vec![' '; width]; height];

    // Map data points to grid columns
    let n = data.len();
    for (i, &val) in data.iter().enumerate() {
        let col = (i * (width - 1) / (n - 1)).min(width - 1);
        let row = ((max_val - val) / range * (height - 1) as f64).round() as usize;
        let row = row.min(height - 1);
        grid[row][col] = marker;
    }

    // Connect dots with simple line interpolation
    let connect = input
        .get("connect")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if connect {
        for i in 0..n - 1 {
            let col1 = i * (width - 1) / (n - 1);
            let col2 = (i + 1) * (width - 1) / (n - 1);
            let row1 = ((max_val - data[i]) / range * (height - 1) as f64).round() as usize;
            let row1 = row1.min(height - 1);
            let row2 = ((max_val - data[i + 1]) / range * (height - 1) as f64).round() as usize;
            let row2 = row2.min(height - 1);

            if col2 > col1 {
                for col in col1..col2 {
                    let t = (col - col1) as f64 / (col2 - col1) as f64;
                    let row = (row1 as f64 + t * (row2 as f64 - row1 as f64)).round() as usize;
                    let row = row.min(height - 1);
                    if grid[row][col] == ' ' {
                        grid[row][col] = '·';
                    }
                }
            }
        }
    }

    let y_label_width = 8;
    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(&format!("  {title}\n\n"));
    }

    for r in 0..height {
        let y_val = max_val - (r as f64 / (height - 1) as f64) * range;
        let y_label = if r == 0 || r == height - 1 || r == height / 2 {
            format!("{:>width$.2}", y_val, width = y_label_width)
        } else {
            " ".repeat(y_label_width)
        };
        let row_str: String = grid[r].iter().collect();
        out.push_str(&format!("{y_label} │{row_str}\n"));
    }

    // X-axis
    out.push_str(&format!(
        "{} └{}\n",
        " ".repeat(y_label_width),
        "─".repeat(width)
    ));

    let x_left = format!("{:.2}", data.first().unwrap_or(&0.0));
    let x_right = format!("{:.2}", data.last().unwrap_or(&0.0));
    out.push_str(&format!(
        "{} {x_left}{}{x_right}\n",
        " ".repeat(y_label_width),
        " ".repeat(width.saturating_sub(x_left.len() + x_right.len()))
    ));

    out.push_str(&format!(
        "\n  n={}, min={min_val:.2}, max={max_val:.2}",
        data.len()
    ));
    Ok(out)
}

fn action_scatter(input: &Value) -> Result<String, String> {
    // Expects x and y arrays, or a data array of [x,y] pairs
    let (xs, ys): (Vec<f64>, Vec<f64>) =
        if let (Some(xv), Some(yv)) = (input.get("x"), input.get("y")) {
            let xs = parse_num_array(xv)?;
            let ys = parse_num_array(yv)?;
            (xs, ys)
        } else if let Some(Value::Array(pairs)) = input.get("data") {
            let mut xs = Vec::new();
            let mut ys = Vec::new();
            for p in pairs {
                match p {
                    Value::Array(pair) if pair.len() >= 2 => {
                        xs.push(pair[0].as_f64().unwrap_or(0.0));
                        ys.push(pair[1].as_f64().unwrap_or(0.0));
                    }
                    _ => return Err("data must be array of [x,y] pairs".into()),
                }
            }
            (xs, ys)
        } else {
            return Err("Provide 'x' and 'y' arrays, or 'data' as [[x,y],...] pairs".into());
        };

    if xs.len() != ys.len() || xs.is_empty() {
        return Err("x and y must be non-empty arrays of equal length".into());
    }

    let width = input.get("width").and_then(|v| v.as_u64()).unwrap_or(60) as usize;
    let height = input.get("height").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let title = input
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let marker = input
        .get("marker")
        .and_then(|v| v.as_str())
        .and_then(|s| s.chars().next())
        .unwrap_or('*');

    let x_min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let x_max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let y_min = ys.iter().cloned().fold(f64::INFINITY, f64::min);
    let y_max = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let x_range = if (x_max - x_min).abs() < f64::EPSILON {
        1.0
    } else {
        x_max - x_min
    };
    let y_range = if (y_max - y_min).abs() < f64::EPSILON {
        1.0
    } else {
        y_max - y_min
    };

    let mut grid = vec![vec![' '; width]; height];
    for (&x, &y) in xs.iter().zip(ys.iter()) {
        let col = ((x - x_min) / x_range * (width - 1) as f64).round() as usize;
        let row = ((y_max - y) / y_range * (height - 1) as f64).round() as usize;
        let col = col.min(width - 1);
        let row = row.min(height - 1);
        grid[row][col] = marker;
    }

    let y_label_width = 8;
    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(&format!("  {title}\n\n"));
    }

    for r in 0..height {
        let y_val = y_max - (r as f64 / (height - 1) as f64) * y_range;
        let y_label = if r == 0 || r == height - 1 || r == height / 2 {
            format!("{:>width$.2}", y_val, width = y_label_width)
        } else {
            " ".repeat(y_label_width)
        };
        let row_str: String = grid[r].iter().collect();
        out.push_str(&format!("{y_label} │{row_str}\n"));
    }
    out.push_str(&format!(
        "{} └{}\n",
        " ".repeat(y_label_width),
        "─".repeat(width)
    ));
    out.push_str(&format!(
        "{} {:.2}{}{:.2}\n",
        " ".repeat(y_label_width),
        x_min,
        " ".repeat(width.saturating_sub(12)),
        x_max
    ));

    out.push_str(&format!(
        "\n  n={}, x=[{x_min:.2},{x_max:.2}], y=[{y_min:.2},{y_max:.2}]",
        xs.len()
    ));
    Ok(out)
}

fn action_sparkline(input: &Value) -> Result<String, String> {
    let data = get_numbers(input)?;
    if data.is_empty() {
        return Err("Data array is empty".into());
    }

    let bars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = if (max_val - min_val).abs() < f64::EPSILON {
        1.0
    } else {
        max_val - min_val
    };

    let title = input.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let label = input.get("label").and_then(|v| v.as_str()).unwrap_or("");

    let spark: String = data
        .iter()
        .map(|&v| {
            let idx = ((v - min_val) / range * 7.0).round() as usize;
            bars[idx.min(7)]
        })
        .collect();

    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(&format!("{title}: "));
    }
    out.push_str(&spark);
    if !label.is_empty() {
        out.push_str(&format!(" {label}"));
    }
    out.push_str(&format!(
        "\n  min={min_val:.2}  max={max_val:.2}  n={}",
        data.len()
    ));
    Ok(out)
}

fn parse_num_array(v: &Value) -> Result<Vec<f64>, String> {
    match v {
        Value::Array(arr) => arr
            .iter()
            .map(|x| {
                x.as_f64()
                    .ok_or_else(|| format!("Expected number, got: {x}"))
            })
            .collect(),
        Value::String(s) => s
            .split(',')
            .map(|p| {
                p.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("Not a number: {p}"))
            })
            .collect(),
        _ => Err("Expected array of numbers".into()),
    }
}

pub fn ascii_chart_schema() -> Value {
    json!({
        "name": "ascii_chart_tools",
        "description": "Renders ASCII/Unicode charts (bar, line, scatter, sparkline) from numeric data arrays without external utilities.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["bar", "line", "scatter", "sparkline", "hbar"],
                    "description": "Chart type: bar (vertical bar chart), line (line/time-series chart), scatter (XY scatter plot), sparkline (one-row inline sparkline), hbar (alias for bar)"
                },
                "data": {
                    "type": ["array", "string"],
                    "description": "Array of numbers (or comma-separated string) for bar/line/sparkline. For scatter, use x+y arrays or [[x,y],...] pairs."
                },
                "x": { "type": "array", "description": "X values for scatter plot" },
                "y": { "type": "array", "description": "Y values for scatter plot" },
                "labels": { "type": "array", "description": "Bar labels (one per data value)" },
                "title": { "type": "string", "description": "Chart title" },
                "width": { "type": "integer", "description": "Chart width in characters (default: 40 for bar, 60 for line/scatter)" },
                "height": { "type": "integer", "description": "Chart height in rows (default: 15 for line/scatter)" },
                "style": { "type": "string", "description": "Bar fill style: block (default), hash, equals, dot, shade" },
                "marker": { "type": "string", "description": "Point marker character for line/scatter (default: • / *)" },
                "connect": { "type": "boolean", "description": "Connect line chart points (default: true)" }
            }
        }
    })
}
