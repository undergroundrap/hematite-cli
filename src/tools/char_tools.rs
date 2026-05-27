use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => info_action(args),
        "codepoint" => codepoint_action(args),
        "escape" => escape_action(args),
        "unescape" => unescape_action(args),
        "check" => check_action(args),
        other => Err(format!(
            "char_tools: unknown action '{other}'. Valid: info, codepoint, escape, unescape, check"
        )),
    }
}

fn get_input(args: &Value) -> Option<String> {
    args.get("input")
        .or_else(|| args.get("text"))
        .or_else(|| args.get("char"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn unicode_block(cp: u32) -> &'static str {
    match cp {
        0x0000..=0x007F => "Basic Latin",
        0x0080..=0x00FF => "Latin-1 Supplement",
        0x0100..=0x017F => "Latin Extended-A",
        0x0180..=0x024F => "Latin Extended-B",
        0x0250..=0x02AF => "IPA Extensions",
        0x02B0..=0x02FF => "Spacing Modifier Letters",
        0x0300..=0x036F => "Combining Diacritical Marks",
        0x0370..=0x03FF => "Greek and Coptic",
        0x0400..=0x04FF => "Cyrillic",
        0x0500..=0x052F => "Cyrillic Supplement",
        0x0530..=0x058F => "Armenian",
        0x0590..=0x05FF => "Hebrew",
        0x0600..=0x06FF => "Arabic",
        0x0900..=0x097F => "Devanagari",
        0x1100..=0x11FF => "Hangul Jamo",
        0x1E00..=0x1EFF => "Latin Extended Additional",
        0x1F00..=0x1FFF => "Greek Extended",
        0x2000..=0x206F => "General Punctuation",
        0x2070..=0x209F => "Superscripts and Subscripts",
        0x20A0..=0x20CF => "Currency Symbols",
        0x2100..=0x214F => "Letterlike Symbols",
        0x2150..=0x218F => "Number Forms",
        0x2190..=0x21FF => "Arrows",
        0x2200..=0x22FF => "Mathematical Operators",
        0x2300..=0x23FF => "Miscellaneous Technical",
        0x2400..=0x243F => "Control Pictures",
        0x2500..=0x257F => "Box Drawing",
        0x2580..=0x259F => "Block Elements",
        0x25A0..=0x25FF => "Geometric Shapes",
        0x2600..=0x26FF => "Miscellaneous Symbols",
        0x2700..=0x27BF => "Dingbats",
        0x2C60..=0x2C7F => "Latin Extended-C",
        0x3000..=0x303F => "CJK Symbols and Punctuation",
        0x3040..=0x309F => "Hiragana",
        0x30A0..=0x30FF => "Katakana",
        0x3100..=0x312F => "Bopomofo",
        0x3130..=0x318F => "Hangul Compatibility Jamo",
        0x4E00..=0x9FFF => "CJK Unified Ideographs",
        0xAC00..=0xD7AF => "Hangul Syllables",
        0xD800..=0xDFFF => "Surrogates",
        0xE000..=0xF8FF => "Private Use Area",
        0xF900..=0xFAFF => "CJK Compatibility Ideographs",
        0xFB00..=0xFB4F => "Alphabetic Presentation Forms",
        0xFE30..=0xFE4F => "CJK Compatibility Forms",
        0xFE50..=0xFE6F => "Small Form Variants",
        0xFF00..=0xFFEF => "Halfwidth and Fullwidth Forms",
        0x1D400..=0x1D7FF => "Mathematical Alphanumeric Symbols",
        0x1F300..=0x1F5FF => "Miscellaneous Symbols and Pictographs",
        0x1F600..=0x1F64F => "Emoticons",
        0x1F650..=0x1F67F => "Ornamental Dingbats",
        0x1F680..=0x1F6FF => "Transport and Map Symbols",
        0x1F700..=0x1F77F => "Alchemical Symbols",
        0x1F900..=0x1F9FF => "Supplemental Symbols and Pictographs",
        _ => "Unknown Block",
    }
}

fn char_category(c: char) -> &'static str {
    if c.is_uppercase() {
        "Uppercase Letter"
    } else if c.is_lowercase() {
        "Lowercase Letter"
    } else if c.is_numeric() {
        "Numeric"
    } else if c.is_whitespace() {
        "Whitespace"
    } else if c.is_control() {
        "Control"
    } else if c.is_alphanumeric() {
        "Alphanumeric"
    } else if c.is_ascii_punctuation() {
        "ASCII Punctuation"
    } else {
        "Symbol / Other"
    }
}

fn char_info_line(c: char) -> String {
    let cp = c as u32;
    let block = unicode_block(cp);
    let category = char_category(c);
    let display = if c.is_control() {
        format!("<control:{cp:#06X}>")
    } else {
        c.to_string()
    };
    let ascii_note = if c.is_ascii() { " [ASCII]" } else { "" };
    format!("  '{display}'  U+{cp:04X}  {category}  Block: {block}{ascii_note}")
}

fn info_action(args: &Value) -> Result<String, String> {
    let input = get_input(args).ok_or("char_tools info: 'input' or 'text' required")?;
    let chars: Vec<char> = input.chars().collect();
    if chars.is_empty() {
        return Ok("char_tools: empty input string".to_string());
    }

    let mut out = String::new();
    out.push_str(&format!(
        "String: {:?}  ({} chars, {} bytes)\n\n",
        input,
        chars.len(),
        input.len()
    ));

    let all_ascii = chars.iter().all(|c| c.is_ascii());
    let has_control = chars.iter().any(|c| c.is_control());
    out.push_str(&format!(
        "All ASCII: {}  Has control chars: {}\n\n",
        if all_ascii { "yes" } else { "no" },
        if has_control { "yes" } else { "no" }
    ));

    if chars.len() == 1 {
        out.push_str("Character detail:\n");
        out.push_str(&char_info_line(chars[0]));
        out.push('\n');
        let cp = chars[0] as u32;
        out.push_str(&format!(
            "  Decimal: {cp}  Hex: {cp:#010X}  Octal: {cp:o}  Binary: {cp:b}\n"
        ));
        let up: String = chars[0].to_uppercase().collect();
        let lo: String = chars[0].to_lowercase().collect();
        if up != chars[0].to_string() {
            out.push_str(&format!("  Uppercase: {up}\n"));
        }
        if lo != chars[0].to_string() {
            out.push_str(&format!("  Lowercase: {lo}\n"));
        }
    } else {
        let limit = chars.len().min(50);
        out.push_str(&format!("Characters ({}/{}):\n", limit, chars.len()));
        for c in &chars[..limit] {
            out.push_str(&char_info_line(*c));
            out.push('\n');
        }
        if chars.len() > 50 {
            out.push_str(&format!("  ... ({} more)\n", chars.len() - 50));
        }
    }
    Ok(out)
}

fn codepoint_action(args: &Value) -> Result<String, String> {
    // If 'char' or 'input' is a single character, show its codepoint.
    // If 'codepoint' is a number, convert to character.
    if let Some(cp_val) = args.get("codepoint") {
        let cp: u32 = if let Some(n) = cp_val.as_u64() {
            n as u32
        } else if let Some(s) = cp_val.as_str() {
            let s = s
                .trim_start_matches("U+")
                .trim_start_matches("u+")
                .trim_start_matches("0x");
            u32::from_str_radix(s, 16)
                .map_err(|_| format!("char_tools codepoint: cannot parse '{s}' as hex codepoint"))?
        } else {
            return Err(
                "char_tools codepoint: 'codepoint' must be a number or 'U+XXXX' string".to_string(),
            );
        };
        match char::from_u32(cp) {
            Some(c) => {
                let mut out = format!(
                    "U+{cp:04X} → '{c}'\n",
                    c = if c.is_control() { '?' } else { c }
                );
                out.push_str(&format!(
                    "  Block: {}\n  Category: {}\n",
                    unicode_block(cp),
                    char_category(c)
                ));
                Ok(out)
            }
            None => Err(format!("U+{cp:04X} is not a valid Unicode codepoint")),
        }
    } else {
        let input = get_input(args).ok_or("char_tools codepoint: provide 'input' (string to convert to codepoints) or 'codepoint' (number to convert to char)")?;
        let chars: Vec<char> = input.chars().collect();
        let mut out = format!("Codepoints for {:?}:\n", input);
        for c in &chars {
            let cp = *c as u32;
            let display = if c.is_control() {
                "?".to_string()
            } else {
                c.to_string()
            };
            out.push_str(&format!("  '{display}'  U+{cp:04X}  (decimal {cp})\n"));
        }
        Ok(out)
    }
}

fn escape_action(args: &Value) -> Result<String, String> {
    let input = get_input(args).ok_or("char_tools escape: 'input' or 'text' required")?;
    let style = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("unicode");

    let mut out = String::new();
    match style {
        "unicode" | "rust" => {
            // Rust-style \u{XXXXXX}
            for c in input.chars() {
                let cp = c as u32;
                if c.is_ascii_graphic() || c == ' ' {
                    out.push(c);
                } else {
                    out.push_str(&format!("\\u{{{cp:X}}}"));
                }
            }
        }
        "json" => {
            // JSON-style \uXXXX (BMP only; surrogates for SMP)
            for c in input.chars() {
                let cp = c as u32;
                if c.is_ascii_graphic() || c == ' ' {
                    out.push(c);
                } else if cp <= 0xFFFF {
                    out.push_str(&format!("\\u{cp:04X}"));
                } else {
                    // Surrogate pair
                    let cp = cp - 0x10000;
                    let high = 0xD800 + (cp >> 10);
                    let low = 0xDC00 + (cp & 0x3FF);
                    out.push_str(&format!("\\u{high:04X}\\u{low:04X}"));
                }
            }
        }
        "hex" => {
            for byte in input.bytes() {
                out.push_str(&format!("\\x{byte:02X}"));
            }
        }
        other => {
            return Err(format!(
                "char_tools escape: unknown style '{other}'. Valid: unicode (default), json, hex"
            ))
        }
    }

    Ok(format!(
        "Input:   {}\nEscaped: {}\nStyle:   {style}\n",
        input, out
    ))
}

fn parse_unicode_escape(s: &str, pos: usize) -> Option<(char, usize)> {
    let bytes = s.as_bytes();
    if pos + 1 >= bytes.len() || bytes[pos] != b'\\' {
        return None;
    }
    match bytes[pos + 1] {
        b'u' => {
            if pos + 2 < bytes.len() && bytes[pos + 2] == b'{' {
                // \u{XXXXXX}
                let start = pos + 3;
                let end = s[start..].find('}').map(|i| start + i)?;
                let hex = &s[start..end];
                let cp = u32::from_str_radix(hex, 16).ok()?;
                let c = char::from_u32(cp)?;
                Some((c, end + 1))
            } else if pos + 6 <= s.len() {
                // \uXXXX
                let hex = &s[pos + 2..pos + 6];
                let cp = u32::from_str_radix(hex, 16).ok()?;
                let c = char::from_u32(cp)?;
                Some((c, pos + 6))
            } else {
                None
            }
        }
        b'x' if pos + 4 <= s.len() => {
            let hex = &s[pos + 2..pos + 4];
            let cp = u32::from_str_radix(hex, 16).ok()?;
            let c = char::from_u32(cp)?;
            Some((c, pos + 4))
        }
        b'n' => Some(('\n', pos + 2)),
        b't' => Some(('\t', pos + 2)),
        b'r' => Some(('\r', pos + 2)),
        b'\\' => Some(('\\', pos + 2)),
        _ => None,
    }
}

fn unescape_action(args: &Value) -> Result<String, String> {
    let input = get_input(args).ok_or("char_tools unescape: 'input' or 'text' required")?;
    let mut out = String::new();
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();

    while i < input.len() {
        if let Some((c, next)) = parse_unicode_escape(&input, i) {
            out.push(c);
            i = next;
        } else {
            // advance by one char
            let c = chars.iter().find(|_| true);
            let ch = input[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
            let _ = c;
        }
    }

    Ok(format!("Input:     {}\nUnescaped: {}\n", input, out))
}

fn check_action(args: &Value) -> Result<String, String> {
    let input = get_input(args).ok_or("char_tools check: 'input' or 'text' required")?;
    let chars: Vec<char> = input.chars().collect();
    if chars.is_empty() {
        return Ok("char_tools check: empty input".to_string());
    }

    let check = |pred: fn(char) -> bool| -> (&'static str, usize) {
        let count = chars.iter().filter(|&&c| pred(c)).count();
        if count == chars.len() {
            ("all", count)
        } else {
            ("", count)
        }
    };

    let tick = |n: usize, total: usize| -> &'static str {
        if n == 0 {
            "✗"
        } else if n == total {
            "✓"
        } else {
            "~"
        }
    };
    let t = chars.len();

    let ascii_n = chars.iter().filter(|&&c| c.is_ascii()).count();
    let alpha_n = chars.iter().filter(|&&c| c.is_alphabetic()).count();
    let num_n = chars.iter().filter(|&&c| c.is_numeric()).count();
    let alnum_n = chars.iter().filter(|&&c| c.is_alphanumeric()).count();
    let upper_n = chars.iter().filter(|&&c| c.is_uppercase()).count();
    let lower_n = chars.iter().filter(|&&c| c.is_lowercase()).count();
    let ws_n = chars.iter().filter(|&&c| c.is_whitespace()).count();
    let ctrl_n = chars.iter().filter(|&&c| c.is_control()).count();
    let punct_n = chars.iter().filter(|&&c| c.is_ascii_punctuation()).count();

    let _ = check; // suppress unused warning

    let mut out = format!("Input: {:?}  ({t} chars)\n\n", input);
    out.push_str(&format!(
        "{} is_ascii        {}/{t}\n",
        tick(ascii_n, t),
        ascii_n
    ));
    out.push_str(&format!(
        "{} is_alphabetic   {}/{t}\n",
        tick(alpha_n, t),
        alpha_n
    ));
    out.push_str(&format!(
        "{} is_numeric      {}/{t}\n",
        tick(num_n, t),
        num_n
    ));
    out.push_str(&format!(
        "{} is_alphanumeric {}/{t}\n",
        tick(alnum_n, t),
        alnum_n
    ));
    out.push_str(&format!(
        "{} is_uppercase    {}/{t}\n",
        tick(upper_n, t),
        upper_n
    ));
    out.push_str(&format!(
        "{} is_lowercase    {}/{t}\n",
        tick(lower_n, t),
        lower_n
    ));
    out.push_str(&format!("{} is_whitespace   {}/{t}\n", tick(ws_n, t), ws_n));
    out.push_str(&format!(
        "{} is_control      {}/{t}\n",
        tick(ctrl_n, t),
        ctrl_n
    ));
    out.push_str(&format!(
        "{} is_ascii_punct  {}/{t}\n",
        tick(punct_n, t),
        punct_n
    ));
    out.push_str("\n✓ = all chars match  ~ = partial  ✗ = none match\n");
    Ok(out)
}
