use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("banner");
    match action {
        "banner" => action_banner(args),
        "box" => action_box(args),
        "bar" => action_bar(args),
        "table" => action_table(args),
        "tree" => action_tree(args),
        other => Err(format!(
            "ascii_tools: unknown action '{other}'. Valid: banner, box, bar, table, tree"
        )),
    }
}

fn get_text(args: &Value) -> Result<&str, String> {
    args.get("text")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("label"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "ascii_tools: 'text' is required".to_string())
}

// ── Banner font ───────────────────────────────────────────────────────────────

// Simple block-letter font (5 rows × variable cols per glyph)
fn big_char(c: char) -> [&'static str; 5] {
    match c.to_ascii_uppercase() {
        'A' => [" ██╗ ", "███║ ", "╚═██╗", "  ██║", "  ╚═╝"],
        'B' => ["██╗  ", "██╠╗ ", "████╗", "██╔██╗", "██╝╝"],
        'C' => [" ██╗", "██╔╝", "██║  ", "██╚╗", " ██╝"],
        'D' => ["██╗ ", "███╗ ", "██╔██╗", "██╔██╗", "████╝"],
        'E' => ["████╗", "██╔══╝", "████╗ ", "██╔══╝", "████╗"],
        'F' => ["████╗", "██╔══╝", "████╗ ", "██╔══╝", "██╔══╝"],
        'G' => [" ████╗", "██╔═══╝", "██║ ████", "██║   ██║", " ████╔╝"],
        'H' => ["██╗ ██╗", "██║ ██║", "███████║", "██╔══██║", "██║  ██║"],
        'I' => ["██╗", "██║", "██║", "██║", "██╝"],
        'J' => ["  ██╗", "  ██║", "  ██║", "██╔██║", " ██╔╝"],
        'K' => ["██╗ ██╗", "██╔██╔╝", "█████╔╝", "██╔═██╗", "██║  ██╗"],
        'L' => ["██╗   ", "██║   ", "██║   ", "██║   ", "████╗"],
        'M' => [
            "██╗ ██╗",
            "████╗███╗",
            "██╔████╔██╗",
            "██║╚██╔╝██║",
            "██║ ╚═╝ ██║",
        ],
        'N' => [
            "███╗ ██╗",
            "████╗ ██║",
            "██╔██╗ ██║",
            "██║╚██╗██║",
            "██║ ╚████║",
        ],
        'O' => [
            " ██████╗ ",
            "██╔════██╗",
            "██║    ██║",
            "██╚════██╝",
            " ██████╔╝",
        ],
        'P' => ["████╗ ", "██╔═██╗", "████╔╝", "██╔═══╝", "██║    "],
        'Q' => [
            " ██████╗ ",
            "██╔════██╗",
            "██║ ██╗██║",
            "██╚═██╔██╝",
            " ██████╗╝",
        ],
        'R' => ["████╗ ", "██╔═██╗", "████╔╝", "██╔══██╗", "██║   ██╗"],
        'S' => [" ████╗", "██╔══╝", " ████╗", "  ╚═██╗", "████╔╝"],
        'T' => ["██████╗", "╚═██╔═╝", "  ██║  ", "  ██║  ", "  ██║  "],
        'U' => ["██╗  ██╗", "██║  ██║", "██║  ██║", "██╚══██╝", " ██████╔╝"],
        'V' => ["██╗  ██╗", "██║  ██║", "██╚═╔██╝", "╚████╔╝ ", " ╚══╝  "],
        'W' => ["██╗  ██╗", "██║  ██║", "██║ ██╔╝", "████╔╝  ", "██╔╝╝   "],
        'X' => ["██╗ ██╗", "╚████╔╝", " ╚██╔╝ ", " ████╗ ", "██╔╝██╗"],
        'Y' => ["██╗  ██╗", "╚████╔╝ ", " ╚██╔╝  ", "  ██║   ", "  ██║   "],
        'Z' => ["██████╗", "  ╔██╔╝", " ██╔╝  ", "██╔╝   ", "███████╗"],
        '0' => [" ██╗ ", "██╔██╗", "██║██║", "██╚██╝", " ██╔╝"],
        '1' => ["██╗", "██║", "██║", "██║", "██║"],
        '2' => ["████╗", "╔══██║", " ███╔╝", "██╔══╝", "█████╗"],
        '3' => ["████╗", "  ██║", " ███╣", "  ██║", "████╣"],
        '4' => ["██╗ ██╗", "██╚══██╗", "████████╗", "   ╔██╔╝", "   ╚══╝"],
        '5' => ["█████╗", "██╔══╝", "█████╗", "   ██╗", "████╔╝"],
        '6' => [" ████╗", "██╔═══╝", "██████╗", "██╔══██╗", " █████╔╝"],
        '7' => ["███████╗", "   ╔██╔╝", "  ██╔╝ ", " ██╔╝  ", "██╔╝   "],
        '8' => [" ████╗", "██╔═██╗", " ████╔╝", "██╔═██╗", " ████╔╝"],
        '9' => [" ████╗", "██╔═██╗", " ████╣", "   ██╣", " ████╝"],
        '!' => ["██╗", "██║", "██║", "╚═╝", "██╗"],
        '?' => ["████╗", "  ╔█╣", " ██╔╝", " ╚═╝ ", " ██╗ "],
        '.' => ["   ", "   ", "   ", "   ", "██╗"],
        '-' => ["   ", "   ", "███╗", "   ", "   "],
        '_' => ["   ", "   ", "   ", "   ", "███╗"],
        ':' => ["██╗", "╚═╝", "   ", "██╗", "╚═╝"],
        '/' => ["  ██╗", " ██╔╝", "██╔╝ ", "██╝  ", "╝    "],
        ' ' => ["   ", "   ", "   ", "   ", "   "],
        _ => ["   ", " ░ ", " ░ ", " ░ ", "   "],
    }
}

fn action_banner(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    if text.len() > 30 {
        return Err("ascii_tools banner: text too long (max 30 characters)".to_string());
    }

    let rows = 5;
    let mut lines = vec![String::new(); rows];
    let mut first = true;
    for ch in text.chars() {
        let g = big_char(ch);
        for (i, row) in g.iter().enumerate() {
            if !first {
                lines[i].push(' ');
            }
            lines[i].push_str(row);
        }
        first = false;
    }
    let mut out = String::from("ascii_tools — banner\n\n");
    for line in &lines {
        out.push_str(line);
        out.push('\n');
    }
    Ok(out)
}

// ── Box drawing ───────────────────────────────────────────────────────────────

struct BoxStyle {
    tl: char,
    tr: char,
    bl: char,
    br: char,
    h: char,
    v: char,
}

impl BoxStyle {
    fn single() -> Self {
        Self {
            tl: '┌',
            tr: '┐',
            bl: '└',
            br: '┘',
            h: '─',
            v: '│',
        }
    }
    fn double() -> Self {
        Self {
            tl: '╔',
            tr: '╗',
            bl: '╚',
            br: '╝',
            h: '═',
            v: '║',
        }
    }
    fn rounded() -> Self {
        Self {
            tl: '╭',
            tr: '╮',
            bl: '╰',
            br: '╯',
            h: '─',
            v: '│',
        }
    }
    fn heavy() -> Self {
        Self {
            tl: '┏',
            tr: '┓',
            bl: '┗',
            br: '┛',
            h: '━',
            v: '┃',
        }
    }
    fn ascii() -> Self {
        Self {
            tl: '+',
            tr: '+',
            bl: '+',
            br: '+',
            h: '-',
            v: '|',
        }
    }
}

fn draw_box(lines: &[&str], style: BoxStyle, padding: usize) -> String {
    let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let inner_w = max_len + padding * 2;
    let h_str: String = std::iter::repeat_n(style.h, inner_w).collect();
    let pad_str: String = std::iter::repeat_n(' ', padding).collect();

    let mut out = String::new();
    out.push(style.tl);
    out.push_str(&h_str);
    out.push(style.tr);
    out.push('\n');

    let empty_pad: String = std::iter::repeat_n(' ', inner_w).collect();
    for _ in 0..padding.min(1) {
        out.push(style.v);
        out.push_str(&empty_pad);
        out.push(style.v);
        out.push('\n');
    }

    for line in lines {
        let len = line.chars().count();
        let right_pad: String = std::iter::repeat_n(' ', max_len - len + padding).collect();
        out.push(style.v);
        out.push_str(&pad_str);
        out.push_str(line);
        out.push_str(&right_pad);
        out.push(style.v);
        out.push('\n');
    }

    for _ in 0..padding.min(1) {
        out.push(style.v);
        out.push_str(&empty_pad);
        out.push(style.v);
        out.push('\n');
    }

    out.push(style.bl);
    out.push_str(&h_str);
    out.push(style.br);
    out.push('\n');
    out
}

fn action_box(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let style_name = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("single");
    let padding = args.get("padding").and_then(|v| v.as_u64()).unwrap_or(1) as usize;

    let style = match style_name {
        "double" => BoxStyle::double(),
        "rounded" | "round" => BoxStyle::rounded(),
        "heavy" | "thick" | "bold" => BoxStyle::heavy(),
        "ascii" => BoxStyle::ascii(),
        _ => BoxStyle::single(),
    };

    let lines: Vec<&str> = text.lines().collect();
    let mut out = format!("ascii_tools — box ({})\n\n", style_name);
    out.push_str(&draw_box(&lines, style, padding));
    Ok(out)
}

// ── Bar / progress ────────────────────────────────────────────────────────────

fn action_bar(args: &Value) -> Result<String, String> {
    let value = args
        .get("value")
        .or_else(|| args.get("v"))
        .and_then(|v| v.as_f64())
        .ok_or("ascii_tools bar: 'value' (number) is required")?;
    let max = args.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0);
    let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(40) as usize;
    let style = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("block");
    let label = args.get("label").and_then(|v| v.as_str()).unwrap_or("");

    if max <= 0.0 {
        return Err("ascii_tools bar: 'max' must be positive".to_string());
    }
    let pct = (value / max).clamp(0.0, 1.0);
    let filled = (pct * width as f64).round() as usize;
    let empty = width - filled;

    let (fill_char, empty_char, open, close) = match style {
        "hash" => ('#', '-', '[', ']'),
        "equals" | "eq" => ('=', ' ', '[', ']'),
        "arrow" => ('=', '-', '[', '>'),
        "dot" => ('.', ' ', '|', '|'),
        "circle" => ('●', '○', ' ', ' '),
        "shade" => ('█', '░', ' ', ' '),
        _ => ('█', ' ', '│', '│'),
    };

    let bar: String = std::iter::repeat_n(fill_char, filled)
        .chain(std::iter::repeat_n(empty_char, empty))
        .collect();

    let mut out = "ascii_tools — bar\n\n".to_string();
    if label.is_empty() {
        out.push_str(&format!("  {}{}{} {:.1}%\n", open, bar, close, pct * 100.0));
    } else {
        out.push_str(&format!(
            "  {:<20}  {}{}{} {:.1}%\n",
            label,
            open,
            bar,
            close,
            pct * 100.0
        ));
    }
    out.push_str(&format!("  Value: {} / {} (max)\n", value, max));
    Ok(out)
}

// ── ASCII table ───────────────────────────────────────────────────────────────

fn action_table(args: &Value) -> Result<String, String> {
    let headers = args
        .get("headers")
        .and_then(|v| v.as_array())
        .ok_or("ascii_tools table: 'headers' array is required")?;
    let rows = args
        .get("rows")
        .and_then(|v| v.as_array())
        .ok_or("ascii_tools table: 'rows' array (of arrays) is required")?;

    let style = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("single");

    let h: Vec<String> = headers
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();

    let data: Vec<Vec<String>> = rows
        .iter()
        .filter_map(|r| r.as_array())
        .map(|r| {
            r.iter()
                .map(|v| {
                    if v.is_null() {
                        String::new()
                    } else if let Some(s) = v.as_str() {
                        s.to_string()
                    } else {
                        v.to_string()
                    }
                })
                .collect()
        })
        .collect();

    let ncols = h.len().max(data.iter().map(|r| r.len()).max().unwrap_or(0));
    let mut col_w: Vec<usize> = h.iter().map(|s| s.chars().count()).collect();
    col_w.resize(ncols, 0);
    for row in &data {
        for (i, cell) in row.iter().enumerate() {
            if i < ncols {
                col_w[i] = col_w[i].max(cell.chars().count());
            }
        }
    }

    let (tl, tr, bl, br, h_ch, v_ch, mt, mb, ml, mr, mc) = match style {
        "double" => ('╔', '╗', '╚', '╝', '═', '║', '╦', '╩', '╠', '╣', '╬'),
        "heavy" => ('┏', '┓', '┗', '┛', '━', '┃', '┳', '┻', '┣', '┫', '╋'),
        "rounded" => ('╭', '╮', '╰', '╯', '─', '│', '┬', '┴', '├', '┤', '┼'),
        _ => ('┌', '┐', '└', '┘', '─', '│', '┬', '┴', '├', '┤', '┼'),
    };

    let render_row = |cells: &[String], separator: char| -> String {
        let mut s = String::new();
        s.push(separator);
        for (i, w) in col_w.iter().enumerate() {
            let cell = cells.get(i).map(|c| c.as_str()).unwrap_or("");
            let pad = w - cell.chars().count();
            s.push(' ');
            s.push_str(cell);
            s.push_str(&" ".repeat(pad));
            s.push(' ');
            s.push(v_ch);
        }
        s
    };

    let top_border: String = {
        let mut s = String::from(tl);
        for (i, w) in col_w.iter().enumerate() {
            s.push_str(&h_ch.to_string().repeat(w + 2));
            if i + 1 < ncols {
                s.push(mt);
            } else {
                s.push(tr);
            }
        }
        s
    };
    let mid_border: String = {
        let mut s = String::from(ml);
        for (i, w) in col_w.iter().enumerate() {
            s.push_str(&h_ch.to_string().repeat(w + 2));
            if i + 1 < ncols {
                s.push(mc);
            } else {
                s.push(mr);
            }
        }
        s
    };
    let bot_border: String = {
        let mut s = String::from(bl);
        for (i, w) in col_w.iter().enumerate() {
            s.push_str(&h_ch.to_string().repeat(w + 2));
            if i + 1 < ncols {
                s.push(mb);
            } else {
                s.push(br);
            }
        }
        s
    };

    let mut out = format!("ascii_tools — table ({})\n\n", style);
    out.push_str(&top_border);
    out.push('\n');
    out.push_str(&render_row(&h, v_ch));
    out.push('\n');
    out.push_str(&mid_border);
    out.push('\n');
    for row in &data {
        out.push_str(&render_row(row, v_ch));
        out.push('\n');
    }
    out.push_str(&bot_border);
    out.push('\n');
    Ok(out)
}

// ── ASCII tree ────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct TreeNode {
    label: String,
    children: Vec<TreeNode>,
}

fn parse_tree(value: &Value, label_key: &str, children_key: &str) -> TreeNode {
    let label = value
        .get(label_key)
        .or_else(|| value.get("name"))
        .or_else(|| value.get("label"))
        .and_then(|v| v.as_str())
        .unwrap_or("(node)")
        .to_string();
    let children = value
        .get(children_key)
        .or_else(|| value.get("children"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|c| parse_tree(c, label_key, children_key))
                .collect()
        })
        .unwrap_or_default();
    TreeNode { label, children }
}

fn render_tree(node: &TreeNode, prefix: &str, is_last: bool, out: &mut String) {
    let connector = if is_last { "└── " } else { "├── " };
    out.push_str(prefix);
    out.push_str(connector);
    out.push_str(&node.label);
    out.push('\n');
    let child_prefix = if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    };
    let count = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        render_tree(child, &child_prefix, i + 1 == count, out);
    }
}

fn action_tree(args: &Value) -> Result<String, String> {
    let label_key = args
        .get("label_key")
        .and_then(|v| v.as_str())
        .unwrap_or("label");
    let children_key = args
        .get("children_key")
        .and_then(|v| v.as_str())
        .unwrap_or("children");

    // Accept a single root or an array of roots
    let (root_label, roots) = if let Some(nodes) = args.get("nodes").and_then(|v| v.as_array()) {
        // array form: roots are the elements
        let label = args
            .get("root")
            .and_then(|v| v.as_str())
            .unwrap_or(".")
            .to_string();
        let children: Vec<TreeNode> = nodes
            .iter()
            .map(|n| parse_tree(n, label_key, children_key))
            .collect();
        (label, children)
    } else if let Some(root_val) = args.get("root").filter(|v| v.is_object()) {
        let node = parse_tree(root_val, label_key, children_key);
        (node.label.clone(), node.children)
    } else {
        // try lines-based format: "  child\n    grandchild"
        if let Some(text) = args.get("text").and_then(|v| v.as_str()) {
            let root_label = args
                .get("root")
                .and_then(|v| v.as_str())
                .unwrap_or(".")
                .to_string();
            let mut out = "ascii_tools — tree\n\n".to_string();
            out.push_str(&root_label);
            out.push('\n');

            struct LineNode {
                label: String,
                depth: usize,
            }
            let line_nodes: Vec<LineNode> = text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| {
                    let depth = l.chars().take_while(|c| *c == ' ').count() / 2;
                    LineNode {
                        label: l.trim().to_string(),
                        depth,
                    }
                })
                .collect();

            // Build prefix stack
            let mut depth_stack: Vec<bool> = Vec::new();
            let n = line_nodes.len();
            for (idx, node) in line_nodes.iter().enumerate() {
                let depth = node.depth;
                // check if next sibling at same depth exists
                let is_last = !line_nodes[idx + 1..].iter().any(|next| next.depth == depth);

                depth_stack.truncate(depth);
                let prefix: String = depth_stack
                    .iter()
                    .map(|&last| if last { "    " } else { "│   " })
                    .collect();
                let connector = if is_last { "└── " } else { "├── " };
                out.push_str(&prefix);
                out.push_str(connector);
                out.push_str(&node.label);
                out.push('\n');
                if depth < depth_stack.len() {
                    depth_stack[depth] = is_last;
                } else {
                    depth_stack.push(is_last);
                }
                let _ = n;
            }
            return Ok(out);
        }
        return Err(
            "ascii_tools tree: provide 'root' object, 'nodes' array, or 'text' outline".to_string(),
        );
    };

    let mut out = "ascii_tools — tree\n\n".to_string();
    out.push_str(&root_label);
    out.push('\n');
    let count = roots.len();
    for (i, child) in roots.iter().enumerate() {
        render_tree(child, "", i + 1 == count, &mut out);
    }
    Ok(out)
}
