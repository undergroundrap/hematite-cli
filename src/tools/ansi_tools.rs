pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("strip");
    match action {
        "strip" => strip_action(args),
        "colorize" => colorize_action(args),
        "length" => length_action(args),
        "parse" => parse_action(args),
        other => Err(format!(
            "ansi_tools: unknown action '{other}'. Valid: strip, colorize, length, parse"
        )),
    }
}

// ── Input helper ──────────────────────────────────────────────────────────────

fn get_text(args: &serde_json::Value) -> Result<&str, String> {
    args.get("text")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "ansi_tools: 'text' is required".to_string())
}

// ── ANSI sequence handling ────────────────────────────────────────────────────

/// Remove all ANSI/VT100 escape sequences from `text`.
fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            match chars.peek().copied() {
                Some('[') => {
                    chars.next(); // consume '['
                                  // CSI: parameter bytes (0x30–0x3F), intermediate bytes (0x20–0x2F), final byte (0x40–0x7E)
                    loop {
                        match chars.next() {
                            Some(c) if c.is_ascii_alphabetic() => break,
                            Some(_) => {}
                            None => break,
                        }
                    }
                }
                Some(']') => {
                    chars.next(); // consume ']'
                                  // OSC: consume until ST (ESC \) or BEL
                    loop {
                        match chars.next() {
                            Some('\x07') => break,
                            Some('\x1b') => {
                                if chars.peek() == Some(&'\\') {
                                    chars.next();
                                }
                                break;
                            }
                            Some(_) => {}
                            None => break,
                        }
                    }
                }
                Some(_) => {
                    chars.next(); // consume one more char (2-char ESC sequence)
                }
                None => {}
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Visible (non-ANSI) character count.
fn visible_len(text: &str) -> usize {
    strip_ansi(text).chars().count()
}

// ── SGR code tables ───────────────────────────────────────────────────────────

fn fg_code(color: &str) -> Option<String> {
    match color.to_lowercase().replace('-', "_").as_str() {
        "black" => Some("30".into()),
        "red" => Some("31".into()),
        "green" => Some("32".into()),
        "yellow" => Some("33".into()),
        "blue" => Some("34".into()),
        "magenta" | "purple" => Some("35".into()),
        "cyan" => Some("36".into()),
        "white" => Some("37".into()),
        "default" => Some("39".into()),
        "bright_black" | "gray" | "grey" => Some("90".into()),
        "bright_red" => Some("91".into()),
        "bright_green" => Some("92".into()),
        "bright_yellow" => Some("93".into()),
        "bright_blue" => Some("94".into()),
        "bright_magenta" | "bright_purple" => Some("95".into()),
        "bright_cyan" => Some("96".into()),
        "bright_white" => Some("97".into()),
        _ => None,
    }
}

fn bg_code(color: &str) -> Option<String> {
    // bg codes are fg + 10
    fg_code(color).map(|c| {
        let n: u32 = c.parse().unwrap_or(0);
        (n + 10).to_string()
    })
}

fn style_code(style: &str) -> Option<&'static str> {
    match style.to_lowercase().as_str() {
        "bold" => Some("1"),
        "dim" | "faint" => Some("2"),
        "italic" => Some("3"),
        "underline" => Some("4"),
        "blink" => Some("5"),
        "reverse" | "inverse" => Some("7"),
        "strikethrough" | "strike" => Some("9"),
        _ => None,
    }
}

fn sgr_name(code: &str) -> &'static str {
    match code {
        "0" => "Reset",
        "1" => "Bold",
        "2" => "Dim",
        "3" => "Italic",
        "4" => "Underline",
        "5" => "Blink",
        "7" => "Reverse",
        "9" => "Strikethrough",
        "22" => "Normal intensity",
        "23" => "Not italic",
        "24" => "Not underline",
        "25" => "Not blink",
        "27" => "Not reverse",
        "29" => "Not strikethrough",
        "30" => "FG Black",
        "31" => "FG Red",
        "32" => "FG Green",
        "33" => "FG Yellow",
        "34" => "FG Blue",
        "35" => "FG Magenta",
        "36" => "FG Cyan",
        "37" => "FG White",
        "39" => "FG Default",
        "40" => "BG Black",
        "41" => "BG Red",
        "42" => "BG Green",
        "43" => "BG Yellow",
        "44" => "BG Blue",
        "45" => "BG Magenta",
        "46" => "BG Cyan",
        "47" => "BG White",
        "49" => "BG Default",
        "90" => "FG Bright Black (Gray)",
        "91" => "FG Bright Red",
        "92" => "FG Bright Green",
        "93" => "FG Bright Yellow",
        "94" => "FG Bright Blue",
        "95" => "FG Bright Magenta",
        "96" => "FG Bright Cyan",
        "97" => "FG Bright White",
        _ => "Unknown",
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn strip_action(args: &serde_json::Value) -> Result<String, String> {
    let text = get_text(args)?;
    let stripped = strip_ansi(text);

    let mut out = format!("ANSI STRIP\n{}\n", "─".repeat(50));
    let orig_len = text.chars().count();
    let stripped_len = stripped.chars().count();
    out.push_str(&format!(
        "Original : {} chars\nStripped : {} chars ({} escape chars removed)\n\n",
        orig_len,
        stripped_len,
        orig_len - stripped_len
    ));
    out.push_str("Result:\n");
    out.push_str(&stripped);
    out.push('\n');
    Ok(out)
}

fn colorize_action(args: &serde_json::Value) -> Result<String, String> {
    let text = get_text(args)?;

    let mut codes: Vec<String> = Vec::new();

    // Foreground color
    if let Some(fg) = args.get("fg").and_then(|v| v.as_str()) {
        match fg_code(fg) {
            Some(c) => codes.push(c),
            None => {
                return Err(format!(
                    "ansi_tools colorize: unknown foreground color '{fg}'. \
                     Valid: black, red, green, yellow, blue, magenta, cyan, white, \
                     bright_red, bright_green, bright_yellow, bright_blue, \
                     bright_magenta, bright_cyan, bright_white, gray"
                ))
            }
        }
    }

    // Background color
    if let Some(bg) = args.get("bg").and_then(|v| v.as_str()) {
        match bg_code(bg) {
            Some(c) => codes.push(c),
            None => {
                return Err(format!(
                    "ansi_tools colorize: unknown background color '{bg}'"
                ))
            }
        }
    }

    // Style(s) — accept a single string or an array
    let styles: Vec<String> = if let Some(arr) = args.get("style").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else if let Some(s) = args.get("style").and_then(|v| v.as_str()) {
        vec![s.to_string()]
    } else {
        Vec::new()
    };

    for style in &styles {
        match style_code(style) {
            Some(c) => codes.push(c.to_string()),
            None => {
                return Err(format!(
                    "ansi_tools colorize: unknown style '{style}'. \
                     Valid: bold, dim, italic, underline, blink, reverse, strikethrough"
                ))
            }
        }
    }

    if codes.is_empty() {
        return Err(
            "ansi_tools colorize: specify at least one of 'fg', 'bg', or 'style'".to_string(),
        );
    }

    let sgr = codes.join(";");
    let colored = format!("\x1b[{sgr}m{text}\x1b[0m");

    let mut out = format!("ANSI COLORIZE\n{}\n", "─".repeat(50));
    out.push_str(&format!("SGR codes  : {sgr}\n"));
    out.push_str(&format!("Escape seq : \\e[{sgr}m ... \\e[0m\n\n"));
    out.push_str(&colored);
    out.push('\n');
    Ok(out)
}

fn length_action(args: &serde_json::Value) -> Result<String, String> {
    let text = get_text(args)?;
    let raw_len = text.chars().count();
    let vis_len = visible_len(text);

    let mut out = format!("ANSI LENGTH\n{}\n", "─".repeat(50));
    out.push_str(&format!("Raw chars    : {raw_len}\n"));
    out.push_str(&format!("Visible chars: {vis_len}\n"));
    out.push_str(&format!(
        "Escape chars : {}\n",
        raw_len.saturating_sub(vis_len)
    ));
    Ok(out)
}

fn parse_action(args: &serde_json::Value) -> Result<String, String> {
    let text = get_text(args)?;

    let mut out = format!("ANSI PARSE\n{}\n", "─".repeat(50));
    let mut sequences: Vec<String> = Vec::new();
    let mut chars = text.chars().peekable();
    let mut pos = 0usize;

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            let start_pos = pos;
            pos += ch.len_utf8();
            let mut seq = String::from('\x1b');

            match chars.peek().copied() {
                Some('[') => {
                    seq.push('[');
                    chars.next();
                    pos += 1;
                    let mut params = String::new();
                    for c in chars.by_ref() {
                        pos += c.len_utf8();
                        seq.push(c);
                        if c.is_ascii_alphabetic() {
                            // Decode CSI
                            let desc = if c == 'm' {
                                // SGR
                                let codes: Vec<&str> = if params.is_empty() {
                                    vec!["0"]
                                } else {
                                    params.split(';').collect()
                                };
                                let names: Vec<&str> =
                                    codes.iter().map(|p| sgr_name(p.trim())).collect();
                                format!("SGR: {}", names.join(", "))
                            } else {
                                format!(
                                    "CSI {c} (params: {})",
                                    if params.is_empty() { "none" } else { &params }
                                )
                            };
                            sequences.push(format!(
                                "  pos {:5} │ {:30} │ {}",
                                start_pos,
                                seq.replace('\x1b', "ESC"),
                                desc
                            ));
                            break;
                        }
                        params.push(c);
                    }
                }
                Some(c) => {
                    seq.push(c);
                    chars.next();
                    pos += c.len_utf8();
                    sequences.push(format!(
                        "  pos {:5} │ {:30} │ ESC {}",
                        start_pos,
                        seq.replace('\x1b', "ESC"),
                        c
                    ));
                }
                None => {
                    pos += 1;
                }
            }
        } else {
            pos += ch.len_utf8();
        }
    }

    if sequences.is_empty() {
        out.push_str("No ANSI escape sequences found.\n");
    } else {
        out.push_str(&format!(
            "{} escape sequence(s) found:\n\n",
            sequences.len()
        ));
        out.push_str(&format!(
            "  {:5}   {:30}   {}\n",
            "POS", "SEQUENCE", "MEANING"
        ));
        out.push_str(&format!("  {}\n", "-".repeat(70)));
        for seq in &sequences {
            out.push_str(seq);
            out.push('\n');
        }
    }
    out.push_str(&format!("\nVisible chars: {}\n", visible_len(text)));
    Ok(out)
}
