use serde_json::{json, Value};

pub fn unicode_tools_schema() -> Value {
    json!({
        "name": "unicode_tools",
        "description": "Analyze, inspect, and work with Unicode text without external utilities. Covers character properties, script/block detection, bidirectional text analysis, normalization form checking (NFC/NFD indicator), confusable/homoglyph detection, and encoding size analysis. Actions: analyze (default — per-character breakdown with codepoint, name category, script, block, and UTF-8 bytes), scripts (distribution of Unicode scripts in the text), blocks (Unicode block distribution), bidi (bidirectional text analysis — detects RTL, mixed-direction, and potential spoofing), confusables (flag characters that could visually impersonate ASCII), encoding (show UTF-8/UTF-16/UTF-32 byte sequences for the text), normalize (check and explain NFC/NFD/NFKC/NFKD normalization status).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["analyze", "scripts", "blocks", "bidi", "confusables", "encoding", "normalize"],
                    "description": "analyze (default — per-char detail), scripts (script distribution), blocks (block distribution), bidi (bidirectional analysis), confusables (homoglyph detection), encoding (UTF-8/16/32 bytes), normalize (NFC/NFD status)"
                },
                "text": {
                    "type": "string",
                    "description": "The Unicode text to analyze"
                }
            },
            "required": ["text"]
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("Pass 'text' with the Unicode string to analyze.")?;

    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("analyze");

    match action {
        "scripts" => action_scripts(text),
        "blocks" => action_blocks(text),
        "bidi" => action_bidi(text),
        "confusables" => action_confusables(text),
        "encoding" => action_encoding(text),
        "normalize" => action_normalize(text),
        _ => action_analyze(text),
    }
}

// ── Unicode property tables ───────────────────────────────────────────────────

fn char_category(c: char) -> &'static str {
    let cp = c as u32;
    match cp {
        0x0000..=0x001f | 0x007f..=0x009f => "Control",
        0x0020 => "Space",
        0x0021..=0x002f | 0x003a..=0x0040 | 0x005b..=0x0060 | 0x007b..=0x007e => {
            "Punctuation/Symbol"
        }
        0x0030..=0x0039 => "Decimal Digit",
        0x0041..=0x005a | 0x0061..=0x007a => "Latin Letter (ASCII)",
        0x00c0..=0x024f => "Latin Extended",
        0x0250..=0x02af => "IPA Extensions",
        0x0300..=0x036f => "Combining Diacritical Marks",
        0x0370..=0x03ff => "Greek/Coptic",
        0x0400..=0x04ff => "Cyrillic",
        0x0500..=0x052f => "Cyrillic Supplement",
        0x0530..=0x058f => "Armenian",
        0x0590..=0x05ff => "Hebrew",
        0x0600..=0x06ff => "Arabic",
        0x0700..=0x074f => "Syriac",
        0x0900..=0x097f => "Devanagari",
        0x0980..=0x09ff => "Bengali",
        0x0a00..=0x0a7f => "Gurmukhi",
        0x0a80..=0x0aff => "Gujarati",
        0x0b00..=0x0b7f => "Oriya",
        0x0b80..=0x0bff => "Tamil",
        0x0c00..=0x0c7f => "Telugu",
        0x0c80..=0x0cff => "Kannada",
        0x0d00..=0x0d7f => "Malayalam",
        0x0e00..=0x0e7f => "Thai",
        0x0e80..=0x0eff => "Lao",
        0x0f00..=0x0fff => "Tibetan",
        0x1000..=0x109f => "Myanmar",
        0x10a0..=0x10ff => "Georgian",
        0x1100..=0x11ff => "Hangul Jamo",
        0x1200..=0x137f => "Ethiopic",
        0x13a0..=0x13ff => "Cherokee",
        0x1400..=0x167f => "Unified Canadian Aboriginal Syllabics",
        0x1680..=0x169f => "Ogham",
        0x16a0..=0x16ff => "Runic",
        0x1700..=0x177f => "Tagalog/Hanunoo/Buhid/Tagbanwa",
        0x1800..=0x18af => "Mongolian",
        0x1e00..=0x1eff => "Latin Extended Additional",
        0x1f00..=0x1fff => "Greek Extended",
        0x2000..=0x206f => "General Punctuation",
        0x2070..=0x209f => "Superscripts and Subscripts",
        0x20a0..=0x20cf => "Currency Symbols",
        0x20d0..=0x20ff => "Combining Diacritical Marks for Symbols",
        0x2100..=0x214f => "Letterlike Symbols",
        0x2150..=0x218f => "Number Forms",
        0x2190..=0x21ff => "Arrows",
        0x2200..=0x22ff => "Mathematical Operators",
        0x2300..=0x23ff => "Miscellaneous Technical",
        0x2400..=0x243f => "Control Pictures",
        0x2440..=0x245f => "Optical Character Recognition",
        0x2460..=0x24ff => "Enclosed Alphanumerics",
        0x2500..=0x257f => "Box Drawing",
        0x2580..=0x259f => "Block Elements",
        0x25a0..=0x25ff => "Geometric Shapes",
        0x2600..=0x26ff => "Miscellaneous Symbols",
        0x2700..=0x27bf => "Dingbats",
        0x2c00..=0x2c5f => "Glagolitic",
        0x2c60..=0x2c7f => "Latin Extended-C",
        0x2c80..=0x2cff => "Coptic",
        0x2d00..=0x2d2f => "Georgian Supplement",
        0x3000..=0x303f => "CJK Symbols and Punctuation",
        0x3040..=0x309f => "Hiragana",
        0x30a0..=0x30ff => "Katakana",
        0x3100..=0x312f | 0x31a0..=0x31bf => "Bopomofo",
        0x3130..=0x318f => "Hangul Compatibility Jamo",
        0x3200..=0x32ff => "Enclosed CJK Letters and Months",
        0x3300..=0x33ff => "CJK Compatibility",
        0x3400..=0x4dbf => "CJK Unified Ideographs Extension A",
        0x4e00..=0x9fff => "CJK Unified Ideographs",
        0xa000..=0xa48f => "Yi Syllables",
        0xa490..=0xa4cf => "Yi Radicals",
        0xa720..=0xa7ff => "Latin Extended-D",
        0xac00..=0xd7af => "Hangul Syllables",
        0xd800..=0xdfff => "Surrogates",
        0xe000..=0xf8ff => "Private Use Area",
        0xf900..=0xfaff => "CJK Compatibility Ideographs",
        0xfb00..=0xfb4f => "Alphabetic Presentation Forms",
        0xfb50..=0xfdff => "Arabic Presentation Forms-A",
        0xfe30..=0xfe4f => "CJK Compatibility Forms",
        0xfe50..=0xfe6f => "Small Form Variants",
        0xfe70..=0xfeff => "Arabic Presentation Forms-B",
        0xff00..=0xffef => "Halfwidth and Fullwidth Forms",
        0x1f300..=0x1f5ff => "Misc Symbols and Pictographs",
        0x1f600..=0x1f64f => "Emoticons",
        0x1f650..=0x1f67f => "Ornamental Dingbats",
        0x1f680..=0x1f6ff => "Transport and Map Symbols",
        0x1f700..=0x1f77f => "Alchemical Symbols",
        0x1f900..=0x1f9ff => "Supplemental Symbols and Pictographs",
        0x1fa00..=0x1fa6f => "Chess Symbols",
        0x1fa70..=0x1faff => "Symbols and Pictographs Extended-A",
        0x20000..=0x2a6df => "CJK Unified Ideographs Extension B",
        _ => "Other/Unknown",
    }
}

fn char_script(c: char) -> &'static str {
    let cp = c as u32;
    match cp {
        0x0000..=0x007f => "Latin (ASCII)",
        0x00c0..=0x024f | 0x1e00..=0x1eff | 0xa720..=0xa7ff => "Latin",
        0x0300..=0x036f | 0x20d0..=0x20ff => "Common (Combining)",
        0x0370..=0x03ff | 0x1f00..=0x1fff => "Greek",
        0x0400..=0x052f => "Cyrillic",
        0x0530..=0x058f => "Armenian",
        0x0590..=0x05ff | 0xfb1d..=0xfb4f => "Hebrew",
        0x0600..=0x06ff | 0xfb50..=0xfdff | 0xfe70..=0xfeff => "Arabic",
        0x0700..=0x074f => "Syriac",
        0x0900..=0x097f => "Devanagari",
        0x0980..=0x09ff => "Bengali",
        0x0a00..=0x0a7f => "Gurmukhi",
        0x0a80..=0x0aff => "Gujarati",
        0x0b00..=0x0b7f => "Oriya",
        0x0b80..=0x0bff => "Tamil",
        0x0c00..=0x0c7f => "Telugu",
        0x0c80..=0x0cff => "Kannada",
        0x0d00..=0x0d7f => "Malayalam",
        0x0e00..=0x0e7f => "Thai",
        0x0e80..=0x0eff => "Lao",
        0x0f00..=0x0fff => "Tibetan",
        0x1000..=0x109f => "Myanmar",
        0x10a0..=0x10ff => "Georgian",
        0x1100..=0x11ff | 0x3130..=0x318f | 0xac00..=0xd7af => "Hangul",
        0x1200..=0x137f => "Ethiopic",
        0x13a0..=0x13ff => "Cherokee",
        0x1800..=0x18af => "Mongolian",
        0x2000..=0x2bff | 0x3000..=0x303f => "Common",
        0x3040..=0x309f => "Hiragana",
        0x30a0..=0x30ff | 0x31f0..=0x31ff => "Katakana",
        0x3400..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x2a6df => "Han (CJK)",
        0xa000..=0xa4cf => "Yi",
        0x1f300..=0x1faff => "Emoji",
        _ => "Unknown/Common",
    }
}

fn is_rtl_char(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        0x0590..=0x05ff |  // Hebrew
        0x0600..=0x06ff |  // Arabic
        0x0700..=0x074f |  // Syriac
        0x07c0..=0x07ff |  // Nko
        0xfb1d..=0xfb4f |  // Hebrew presentation
        0xfb50..=0xfdff |  // Arabic presentation A
        0xfe70..=0xfeff    // Arabic presentation B
    )
}

fn is_control_char(c: char) -> bool {
    let cp = c as u32;
    // Bidirectional control characters that can be used for spoofing
    matches!(
        cp,
        0x200b | // ZERO WIDTH SPACE
        0x200c | // ZERO WIDTH NON-JOINER
        0x200d | // ZERO WIDTH JOINER
        0x200e | // LEFT-TO-RIGHT MARK
        0x200f | // RIGHT-TO-LEFT MARK
        0x202a | // LEFT-TO-RIGHT EMBEDDING
        0x202b | // RIGHT-TO-LEFT EMBEDDING
        0x202c | // POP DIRECTIONAL FORMATTING
        0x202d | // LEFT-TO-RIGHT OVERRIDE
        0x202e | // RIGHT-TO-LEFT OVERRIDE
        0x2066 | // LEFT-TO-RIGHT ISOLATE
        0x2067 | // RIGHT-TO-LEFT ISOLATE
        0x2068 | // FIRST STRONG ISOLATE
        0x2069 // POP DIRECTIONAL ISOLATE
    )
}

// Confusables: characters that look like ASCII but aren't
// Source: selected subset of Unicode confusables data
fn ascii_confusable_of(c: char) -> Option<char> {
    match c {
        // Latin lookalikes for digits
        'O' | 'o' => None, // already ASCII
        'А' => Some('A'),  // Cyrillic А
        'а' => Some('a'),  // Cyrillic а
        'В' => Some('B'),  // Cyrillic В
        'С' => Some('C'),  // Cyrillic С
        'с' => Some('c'),  // Cyrillic с
        'Е' => Some('E'),  // Cyrillic Е
        'е' => Some('e'),  // Cyrillic е
        'Н' => Some('H'),  // Cyrillic Н
        'І' => Some('I'),  // Cyrillic І
        'і' => Some('i'),  // Cyrillic і
        'ј' => Some('j'),  // Cyrillic ј
        'К' => Some('K'),  // Cyrillic К
        'М' => Some('M'),  // Cyrillic М
        'м' => Some('m'),  // Cyrillic м
        'О' => Some('O'),  // Cyrillic О
        'о' => Some('o'),  // Cyrillic о
        'Р' => Some('P'),  // Cyrillic Р
        'р' => Some('p'),  // Cyrillic р
        'Ѕ' => Some('S'),  // Cyrillic Ѕ
        'Т' => Some('T'),  // Cyrillic Т
        'Х' => Some('X'),  // Cyrillic Х
        'х' => Some('x'),  // Cyrillic х
        'У' => Some('Y'),  // Cyrillic У
        'у' => Some('y'),  // Cyrillic у
        'ν' => Some('v'),  // Greek nu
        'ω' => Some('w'),  // Greek omega (approximate)
        'α' => Some('a'),  // Greek alpha
        'ο' => Some('o'),  // Greek omicron
        'ρ' => Some('p'),  // Greek rho
        'Α' => Some('A'),  // Greek Alpha
        'Β' => Some('B'),  // Greek Beta
        'Ε' => Some('E'),  // Greek Epsilon
        'Ζ' => Some('Z'),  // Greek Zeta
        'Η' => Some('H'),  // Greek Eta
        'Ι' => Some('I'),  // Greek Iota
        'Κ' => Some('K'),  // Greek Kappa
        'Μ' => Some('M'),  // Greek Mu
        'Ν' => Some('N'),  // Greek Nu
        'Ο' => Some('O'),  // Greek Omicron
        'Ρ' => Some('P'),  // Greek Rho
        'Τ' => Some('T'),  // Greek Tau
        'Υ' => Some('Y'),  // Greek Upsilon
        'Χ' => Some('X'),  // Greek Chi
        // Mathematical alphanumeric symbols (bold, italic, etc.)
        '\u{1d400}'..='\u{1d7ff}' => {
            // Try to map to ASCII (simplified)
            Some('?')
        }
        // Fullwidth ASCII
        '\u{ff01}'..='\u{ff5e}' => {
            let ascii = char::from_u32(c as u32 - 0xff00 + 0x20);
            ascii.filter(|&ch| ch.is_ascii_graphic())
        }
        _ => None,
    }
}

fn char_utf8_bytes(c: char) -> Vec<u8> {
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    s.as_bytes().to_vec()
}

fn char_utf16_bytes(c: char, be: bool) -> Vec<u8> {
    let mut units = [0u16; 2];
    let filled = c.encode_utf16(&mut units);
    let mut out = Vec::new();
    for unit in filled {
        let b = if be {
            unit.to_be_bytes()
        } else {
            unit.to_le_bytes()
        };
        out.extend_from_slice(&b);
    }
    out
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_analyze(text: &str) -> Result<String, String> {
    let chars: Vec<char> = text.chars().collect();
    let limit = 100;
    let shown = chars.len().min(limit);

    let mut out = format!(
        "UNICODE ANALYSIS — {} char{}, {} UTF-8 byte{}\n",
        chars.len(),
        if chars.len() == 1 { "" } else { "s" },
        text.len(),
        if text.len() == 1 { "" } else { "s" }
    );
    out.push_str(&"─".repeat(80));
    out.push('\n');
    out.push_str(&format!(
        "  {:4}  {:10}  {:2}  {:<30}  {:<8}  {}\n",
        "Idx", "Codepoint", "Ch", "Category", "Script", "UTF-8 bytes"
    ));
    out.push_str(&"─".repeat(80));
    out.push('\n');

    for (i, &c) in chars[..shown].iter().enumerate() {
        let cp = c as u32;
        let display = if c.is_control() || cp == 0x20 {
            format!("<U+{:04X}>", cp)
        } else {
            c.to_string()
        };
        let cat = char_category(c);
        let script = char_script(c);
        let utf8: String = char_utf8_bytes(c)
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");

        out.push_str(&format!(
            "  {:4}  U+{:06X}   {:<2}  {:<30}  {:<8}  {}\n",
            i + 1,
            cp,
            &display,
            truncate(cat, 30),
            truncate(script, 8),
            utf8
        ));
    }

    if chars.len() > limit {
        out.push_str(&format!(
            "  ... ({} more characters)\n",
            chars.len() - limit
        ));
    }

    // Summary stats
    out.push_str(&"─".repeat(80));
    out.push('\n');
    let ascii_count = chars.iter().filter(|&&c| c.is_ascii()).count();
    let non_ascii = chars.len() - ascii_count;
    let multi_byte = chars.iter().filter(|&&c| c.len_utf8() > 1).count();
    out.push_str(&format!("  ASCII chars:     {}\n", ascii_count));
    out.push_str(&format!("  Non-ASCII chars: {}\n", non_ascii));
    out.push_str(&format!("  Multi-byte UTF-8: {}\n", multi_byte));

    Ok(out)
}

fn action_scripts(text: &str) -> Result<String, String> {
    let mut counts: std::collections::HashMap<&'static str, usize> =
        std::collections::HashMap::new();
    for c in text.chars() {
        if !c.is_whitespace() {
            *counts.entry(char_script(c)).or_insert(0) += 1;
        }
    }

    let total: usize = counts.values().sum();
    let mut sorted: Vec<(&'static str, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let mut out = format!("SCRIPT DISTRIBUTION — {} non-whitespace chars\n", total);
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!(
        "  {:<30}  {:>6}  {:>6}  {}\n",
        "Script", "Count", "Pct", "Bar"
    ));
    out.push_str(&"─".repeat(50));
    out.push('\n');

    for (script, count) in &sorted {
        let pct = if total > 0 { *count * 100 / total } else { 0 };
        let bar_len = (pct / 2).min(25);
        let bar: String = "█".repeat(bar_len);
        out.push_str(&format!(
            "  {:<30}  {:>6}  {:>5}%  {}\n",
            truncate(script, 30),
            count,
            pct,
            bar
        ));
    }

    // Multi-script warning
    let script_count = sorted.len();
    let has_latin = sorted.iter().any(|(s, _)| s.starts_with("Latin"));
    let has_other = sorted
        .iter()
        .any(|(s, _)| !s.starts_with("Latin") && !s.starts_with("Common") && *s != "Latin (ASCII)");
    if script_count > 1 && has_latin && has_other {
        out.push('\n');
        out.push_str("  ⚠ Mixed scripts detected — potential homoglyph/IDN spoofing risk\n");
        out.push_str("  Run action='confusables' for detail\n");
    }

    Ok(out)
}

fn action_blocks(text: &str) -> Result<String, String> {
    let mut counts: std::collections::HashMap<&'static str, usize> =
        std::collections::HashMap::new();
    for c in text.chars() {
        *counts.entry(char_category(c)).or_insert(0) += 1;
    }

    let total: usize = counts.values().sum();
    let mut sorted: Vec<(&'static str, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let mut out = format!("UNICODE BLOCK DISTRIBUTION — {} chars\n", total);
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for (block, count) in &sorted {
        let pct = if total > 0 { *count * 100 / total } else { 0 };
        out.push_str(&format!(
            "  {:<40}  {:>6}  ({:>3}%)\n",
            truncate(block, 40),
            count,
            pct
        ));
    }

    Ok(out)
}

fn action_bidi(text: &str) -> Result<String, String> {
    let chars: Vec<char> = text.chars().collect();
    let rtl_chars: Vec<(usize, char)> = chars
        .iter()
        .enumerate()
        .filter(|(_, &c)| is_rtl_char(c))
        .map(|(i, &c)| (i, c))
        .collect();

    let control_chars: Vec<(usize, char)> = chars
        .iter()
        .enumerate()
        .filter(|(_, &c)| is_control_char(c))
        .map(|(i, &c)| (i, c))
        .collect();

    let ltr_count = chars
        .iter()
        .filter(|&&c| c.is_alphabetic() && !is_rtl_char(c))
        .count();
    let rtl_count = rtl_chars.len();

    let mut out = String::from("BIDIRECTIONAL TEXT ANALYSIS\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!("  Total chars:      {}\n", chars.len()));
    out.push_str(&format!("  LTR alpha chars:  {}\n", ltr_count));
    out.push_str(&format!("  RTL chars:        {}\n", rtl_count));
    out.push_str(&format!("  Bidi controls:    {}\n", control_chars.len()));
    out.push('\n');

    if rtl_count == 0 && control_chars.is_empty() {
        out.push_str("  Direction: LEFT-TO-RIGHT (no RTL or bidi controls detected)\n");
    } else if ltr_count == 0 && control_chars.is_empty() {
        out.push_str("  Direction: RIGHT-TO-LEFT\n");
    } else if rtl_count > 0 && ltr_count > 0 {
        out.push_str("  Direction: MIXED — both LTR and RTL characters present\n");
    }

    if !control_chars.is_empty() {
        out.push('\n');
        out.push_str("  ⚠ BIDI CONTROL CHARACTERS DETECTED — potential Unicode spoofing (CVE-2021-42574 / Trojan Source):\n");
        for (i, c) in &control_chars {
            let name = bidi_control_name(*c);
            out.push_str(&format!(
                "    Position {:4}: U+{:04X} {}\n",
                i + 1,
                *c as u32,
                name
            ));
        }
        out.push_str("  These characters can change how code is displayed in editors without changing its meaning.\n");
        out.push_str("  Reference: https://trojansource.codes/\n");
    }

    if !rtl_chars.is_empty() && !control_chars.is_empty() {
        out.push('\n');
        out.push_str(
            "  ⚠ RTL characters combined with bidi controls is a strong spoofing indicator\n",
        );
    }

    Ok(out)
}

fn bidi_control_name(c: char) -> &'static str {
    match c as u32 {
        0x200b => "ZERO WIDTH SPACE",
        0x200c => "ZERO WIDTH NON-JOINER",
        0x200d => "ZERO WIDTH JOINER",
        0x200e => "LEFT-TO-RIGHT MARK",
        0x200f => "RIGHT-TO-LEFT MARK",
        0x202a => "LEFT-TO-RIGHT EMBEDDING",
        0x202b => "RIGHT-TO-LEFT EMBEDDING",
        0x202c => "POP DIRECTIONAL FORMATTING",
        0x202d => "LEFT-TO-RIGHT OVERRIDE",
        0x202e => "RIGHT-TO-LEFT OVERRIDE (⚠ HIGH RISK)",
        0x2066 => "LEFT-TO-RIGHT ISOLATE",
        0x2067 => "RIGHT-TO-LEFT ISOLATE",
        0x2068 => "FIRST STRONG ISOLATE",
        0x2069 => "POP DIRECTIONAL ISOLATE",
        _ => "UNKNOWN BIDI CONTROL",
    }
}

fn action_confusables(text: &str) -> Result<String, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut confusable_hits: Vec<(usize, char, char)> = Vec::new();

    for (i, &c) in chars.iter().enumerate() {
        if let Some(ascii_eq) = ascii_confusable_of(c) {
            confusable_hits.push((i, c, ascii_eq));
        }
    }

    let mut out = String::from("CONFUSABLE / HOMOGLYPH DETECTION\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!("  Input: {} chars\n", chars.len()));
    out.push('\n');

    if confusable_hits.is_empty() {
        out.push_str("  No confusable characters detected.\n");
        out.push_str(
            "  (Note: detection covers Cyrillic, Greek, and fullwidth ASCII lookalikes)\n",
        );
    } else {
        out.push_str(&format!(
            "  ⚠ {} confusable character{} detected:\n\n",
            confusable_hits.len(),
            if confusable_hits.len() == 1 { "" } else { "s" }
        ));
        out.push_str(&format!(
            "  {:4}  {:10}  {:<4}  {:<4}  {:<30}  {}\n",
            "Pos", "Codepoint", "Char", "Looks", "Script", "Risk"
        ));
        out.push_str(&"─".repeat(60));
        out.push('\n');

        for (pos, c, ascii_eq) in &confusable_hits {
            let script = char_script(*c);
            let risk = if ['a', 'e', 'i', 'o', 'u', 'n', 'm', 'c', 'p', 'x', 'y'].contains(ascii_eq)
            {
                "HIGH"
            } else {
                "MEDIUM"
            };
            out.push_str(&format!(
                "  {:4}  U+{:06X}   {:<4}  {:<4}  {:<30}  {}\n",
                pos + 1,
                *c as u32,
                c.to_string(),
                ascii_eq.to_string(),
                truncate(script, 30),
                risk
            ));
        }

        out.push('\n');
        out.push_str("  These characters can be used for:\n");
        out.push_str("    • IDN homograph attacks (fake domain names)\n");
        out.push_str("    • Impersonating variable/function names in source code\n");
        out.push_str("    • Bypass string comparison security checks\n");
    }

    Ok(out)
}

fn action_encoding(text: &str) -> Result<String, String> {
    let chars: Vec<char> = text.chars().collect();
    let limit = 50;
    let shown = chars.len().min(limit);

    let utf8_bytes = text.len();
    let utf16_units: usize = chars.iter().map(|c| c.len_utf16()).sum();
    let utf32_bytes = chars.len() * 4;

    let mut out = "ENCODING SIZES\n".to_string();
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!("  Characters:      {}\n", chars.len()));
    out.push_str(&format!(
        "  UTF-8 bytes:     {} ({:.1}x)\n",
        utf8_bytes,
        utf8_bytes as f64 / chars.len().max(1) as f64
    ));
    out.push_str(&format!(
        "  UTF-16 code units: {} ({} bytes)\n",
        utf16_units,
        utf16_units * 2
    ));
    out.push_str(&format!(
        "  UTF-32 bytes:    {} (4 per char)\n",
        utf32_bytes
    ));
    out.push('\n');
    out.push_str("  BOM:\n");
    out.push_str("    UTF-8:    ef bb bf\n");
    out.push_str("    UTF-16 BE: fe ff\n");
    out.push_str("    UTF-16 LE: ff fe\n");
    out.push_str("    UTF-32 BE: 00 00 fe ff\n");
    out.push_str("    UTF-32 LE: ff fe 00 00\n");
    out.push('\n');

    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "  {:4}  {:10}  {:<4}  {:<24}  {:<20}  {}\n",
        "Idx", "Codepoint", "Chr", "UTF-8", "UTF-16 LE", "UTF-32 LE"
    ));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    for (i, &c) in chars[..shown].iter().enumerate() {
        let cp = c as u32;
        let display = if c.is_control() || cp == 0x20 {
            format!("<{:04X}>", cp)
        } else {
            c.to_string()
        };

        let utf8: String = char_utf8_bytes(c)
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");

        let utf16: String = char_utf16_bytes(c, false)
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");

        let utf32: String = cp
            .to_le_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");

        out.push_str(&format!(
            "  {:4}  U+{:06X}   {:<4}  {:<24}  {:<20}  {}\n",
            i + 1,
            cp,
            &display,
            utf8,
            utf16,
            utf32
        ));
    }

    if chars.len() > limit {
        out.push_str(&format!(
            "  ... ({} more characters)\n",
            chars.len() - limit
        ));
    }

    Ok(out)
}

fn action_normalize(text: &str) -> Result<String, String> {
    // We can't do full NFC/NFD normalization without unicode tables,
    // but we can detect common issues and classify the text.
    let chars: Vec<char> = text.chars().collect();
    let combining_count = chars
        .iter()
        .filter(|&&c| (c as u32) >= 0x0300 && (c as u32) <= 0x036f)
        .count();
    let precomposed_count = chars
        .iter()
        .filter(|&&c| {
            let cp = c as u32;
            // Common precomposed Latin characters (NFC)
            matches!(cp,
                0x00c0..=0x00d6 | 0x00d8..=0x00f6 | 0x00f8..=0x00ff |  // Latin-1 Supplement
                0x0100..=0x024f |  // Latin Extended A/B
                0x1e00..=0x1eff    // Latin Extended Additional
            )
        })
        .count();

    let ascii_only = chars.iter().all(|&c| c.is_ascii());
    let has_combining = combining_count > 0;
    let has_precomposed = precomposed_count > 0;

    let mut out = String::from("UNICODE NORMALIZATION STATUS\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!("  Total chars:         {}\n", chars.len()));
    out.push_str(&format!("  UTF-8 bytes:         {}\n", text.len()));
    out.push_str(&format!("  Combining marks:     {}\n", combining_count));
    out.push_str(&format!("  Precomposed chars:   {}\n", precomposed_count));
    out.push('\n');

    if ascii_only {
        out.push_str("  Form status: ASCII-only — normalization irrelevant\n");
        out.push_str("  NFC ≡ NFD ≡ NFKC ≡ NFKD for pure ASCII text\n");
    } else if !has_combining && !has_precomposed {
        out.push_str("  Form status: Likely already in NFC form\n");
        out.push_str("  No combining marks or obviously precomposed characters detected\n");
    } else if has_combining && has_precomposed {
        out.push_str(
            "  Form status: MIXED — contains both combining marks and precomposed chars\n",
        );
        out.push_str("  ⚠ Inconsistent normalization — may cause string comparison failures\n");
        out.push_str("  Recommendation: normalize to NFC (canonical composed form)\n");
    } else if has_combining {
        out.push_str("  Form status: Likely NFD (decomposed — combining marks present)\n");
        out.push_str("  Converting to NFC would combine marks with base characters\n");
    } else {
        out.push_str("  Form status: Likely NFC (precomposed characters present)\n");
    }

    out.push('\n');
    out.push_str("  Normalization forms:\n");
    out.push_str("    NFC  (Canonical Decomposition then Canonical Composition)    — most common for storage/interchange\n");
    out.push_str("    NFD  (Canonical Decomposition)                               — base + combining marks separated\n");
    out.push_str("    NFKC (Compatibility Decomposition + Canonical Composition)   — collapses font/style variants (ﬁ→fi)\n");
    out.push_str(
        "    NFKD (Compatibility Decomposition)                           — most decomposed form\n",
    );
    out.push('\n');
    out.push_str("  Why it matters:\n");
    out.push_str("    • \"é\" can be U+00E9 (NFC, 1 char) or U+0065 U+0301 (NFD, 2 chars)\n");
    out.push_str("    • Byte equality ≠ visual equality without normalization\n");
    out.push_str("    • Databases, file systems, and languages may use different default forms\n");

    if has_combining {
        out.push('\n');
        out.push_str("  Combining marks in this text:\n");
        for (i, &c) in chars.iter().enumerate() {
            let cp = c as u32;
            if (0x0300..=0x036f).contains(&cp) {
                out.push_str(&format!(
                    "    Position {:4}: U+{:04X} (combining diacritical mark)\n",
                    i + 1,
                    cp
                ));
            }
        }
    }

    Ok(out)
}

fn truncate(s: &str, max: usize) -> &str {
    let mut end = s.len();
    while end > max {
        end -= 1;
        if s.is_char_boundary(end) {
            break;
        }
    }
    &s[..end]
}
