use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("nato");
    match action {
        "nato" => action_nato(args),
        "from_nato" => action_from_nato(args),
        "morse" => action_morse(args),
        "spell" => action_spell(args),
        other => Err(format!(
            "Unknown action '{other}'. Use: nato, from_nato, morse, spell"
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("message"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' field".to_string())
}

fn nato_word(c: char) -> Option<&'static str> {
    match c.to_ascii_uppercase() {
        'A' => Some("Alpha"),
        'B' => Some("Bravo"),
        'C' => Some("Charlie"),
        'D' => Some("Delta"),
        'E' => Some("Echo"),
        'F' => Some("Foxtrot"),
        'G' => Some("Golf"),
        'H' => Some("Hotel"),
        'I' => Some("India"),
        'J' => Some("Juliet"),
        'K' => Some("Kilo"),
        'L' => Some("Lima"),
        'M' => Some("Mike"),
        'N' => Some("November"),
        'O' => Some("Oscar"),
        'P' => Some("Papa"),
        'Q' => Some("Quebec"),
        'R' => Some("Romeo"),
        'S' => Some("Sierra"),
        'T' => Some("Tango"),
        'U' => Some("Uniform"),
        'V' => Some("Victor"),
        'W' => Some("Whiskey"),
        'X' => Some("X-ray"),
        'Y' => Some("Yankee"),
        'Z' => Some("Zulu"),
        '0' => Some("Zero"),
        '1' => Some("One"),
        '2' => Some("Two"),
        '3' => Some("Three"),
        '4' => Some("Four"),
        '5' => Some("Five"),
        '6' => Some("Six"),
        '7' => Some("Seven"),
        '8' => Some("Eight"),
        '9' => Some("Nine"),
        _ => None,
    }
}

fn nato_to_char(word: &str) -> Option<char> {
    match word.to_lowercase().as_str() {
        "alpha" | "alfa" => Some('A'),
        "bravo" => Some('B'),
        "charlie" => Some('C'),
        "delta" => Some('D'),
        "echo" => Some('E'),
        "foxtrot" => Some('F'),
        "golf" => Some('G'),
        "hotel" => Some('H'),
        "india" => Some('I'),
        "juliet" | "juliett" => Some('J'),
        "kilo" => Some('K'),
        "lima" => Some('L'),
        "mike" => Some('M'),
        "november" => Some('N'),
        "oscar" => Some('O'),
        "papa" => Some('P'),
        "quebec" => Some('Q'),
        "romeo" => Some('R'),
        "sierra" => Some('S'),
        "tango" => Some('T'),
        "uniform" => Some('U'),
        "victor" => Some('V'),
        "whiskey" | "whisky" => Some('W'),
        "x-ray" | "xray" => Some('X'),
        "yankee" => Some('Y'),
        "zulu" => Some('Z'),
        "zero" => Some('0'),
        "one" | "wun" => Some('1'),
        "two" | "too" => Some('2'),
        "three" | "tree" => Some('3'),
        "four" | "fower" => Some('4'),
        "five" | "fife" => Some('5'),
        "six" => Some('6'),
        "seven" => Some('7'),
        "eight" | "ait" => Some('8'),
        "nine" | "niner" => Some('9'),
        _ => None,
    }
}

fn morse_encode(c: char) -> Option<&'static str> {
    match c.to_ascii_uppercase() {
        'A' => Some(".-"),
        'B' => Some("-..."),
        'C' => Some("-.-."),
        'D' => Some("-.."),
        'E' => Some("."),
        'F' => Some("..-."),
        'G' => Some("--."),
        'H' => Some("...."),
        'I' => Some(".."),
        'J' => Some(".---"),
        'K' => Some("-.-"),
        'L' => Some(".-.."),
        'M' => Some("--"),
        'N' => Some("-."),
        'O' => Some("---"),
        'P' => Some(".--."),
        'Q' => Some("--.-"),
        'R' => Some(".-."),
        'S' => Some("..."),
        'T' => Some("-"),
        'U' => Some("..-"),
        'V' => Some("...-"),
        'W' => Some(".--"),
        'X' => Some("-..-"),
        'Y' => Some("-.--"),
        'Z' => Some("--.."),
        '0' => Some("-----"),
        '1' => Some(".----"),
        '2' => Some("..---"),
        '3' => Some("...--"),
        '4' => Some("....-"),
        '5' => Some("....."),
        '6' => Some("-...."),
        '7' => Some("--..."),
        '8' => Some("---.."),
        '9' => Some("----."),
        '.' => Some(".-.-.-"),
        ',' => Some("--..--"),
        '?' => Some("..--.."),
        '!' => Some("-.-.--"),
        '\'' => Some(".----."),
        '/' => Some("-..-."),
        '(' => Some("-.--."),
        ')' => Some("-.--.-"),
        '&' => Some(".-..."),
        ':' => Some("---..."),
        ';' => Some("-.-.-."),
        '=' => Some("-...-"),
        '+' => Some(".-.-."),
        '-' => Some("-....-"),
        '_' => Some("..--.-"),
        '"' => Some(".-..-."),
        '$' => Some("...-..-"),
        '@' => Some(".--.-."),
        _ => None,
    }
}

fn morse_decode_char(code: &str) -> Option<char> {
    let table = [
        (".-", 'A'),
        ("-...", 'B'),
        ("-.-.", 'C'),
        ("-..", 'D'),
        (".", 'E'),
        ("..-.", 'F'),
        ("--.", 'G'),
        ("....", 'H'),
        ("..", 'I'),
        (".---", 'J'),
        ("-.-", 'K'),
        (".-..", 'L'),
        ("--", 'M'),
        ("-.", 'N'),
        ("---", 'O'),
        (".--.", 'P'),
        ("--.-", 'Q'),
        (".-.", 'R'),
        ("...", 'S'),
        ("-", 'T'),
        ("..-", 'U'),
        ("...-", 'V'),
        (".--", 'W'),
        ("-..-", 'X'),
        ("-.--", 'Y'),
        ("--..", 'Z'),
        ("-----", '0'),
        (".----", '1'),
        ("..---", '2'),
        ("...--", '3'),
        ("....-", '4'),
        (".....", '5'),
        ("-....", '6'),
        ("--...", '7'),
        ("---..", '8'),
        ("----.", '9'),
        (".-.-.-", '.'),
        ("--..--", ','),
        ("..--..", '?'),
        ("-.-.--", '!'),
        (".----.", '\''),
        ("-..-.", '/'),
        ("-.--.", '('),
        ("-.--.-", ')'),
        (".-...", '&'),
        ("---...", ':'),
        ("-.-.-.", ';'),
        ("-...-", '='),
        (".-.-.", '+'),
        ("-....-", '-'),
        ("..--.-", '_'),
        (".-..-.", '"'),
        ("...-..-", '$'),
        (".--.-.", '@'),
    ];
    table
        .iter()
        .find(|(code_pat, _)| *code_pat == code)
        .map(|(_, c)| *c)
}

fn action_nato(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let mut out = String::from("nato_tools â€” nato\n\n");
    out.push_str(&format!("Input: {}\n\n", text));

    let words: Vec<String> = text
        .chars()
        .map(|c| {
            if c == ' ' {
                "[SPACE]".to_string()
            } else if let Some(w) = nato_word(c) {
                w.to_string()
            } else {
                format!("[{}]", c)
            }
        })
        .collect();

    out.push_str("NATO:\n  ");
    out.push_str(&words.join(" "));
    out.push('\n');

    if text.len() <= 20 {
        out.push_str("\nLetter breakdown:\n");
        for (c, w) in text.chars().zip(words.iter()) {
            out.push_str(&format!("  {:>2} â†’ {}\n", c, w));
        }
    }
    Ok(out)
}

fn action_from_nato(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let mut out = String::from("nato_tools â€” from_nato\n\n");

    let mut result = String::new();
    let mut unrecognized: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
        if clean.eq_ignore_ascii_case("space") || clean.eq_ignore_ascii_case("[space]") {
            result.push(' ');
        } else if let Some(c) = nato_to_char(clean) {
            result.push(c);
        } else {
            unrecognized.push(word.to_string());
        }
    }

    out.push_str(&format!("Input:  {}\n", text));
    out.push_str(&format!("Output: {}\n", result));
    if !unrecognized.is_empty() {
        out.push_str(&format!(
            "Unrecognized words: {}\n",
            unrecognized.join(", ")
        ));
    }
    Ok(out)
}

fn action_morse(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let decode = args
        .get("decode")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| text.chars().all(|c| matches!(c, '.' | '-' | ' ' | '/')));

    let mut out = String::from("nato_tools â€” morse\n\n");

    if !decode {
        // Encode text to Morse
        let mut words_morse: Vec<String> = Vec::new();
        for word in text.split_whitespace() {
            let morse_word: Vec<String> = word
                .chars()
                .filter_map(|c| morse_encode(c).map(|s| s.to_string()))
                .collect();
            if !morse_word.is_empty() {
                words_morse.push(morse_word.join(" "));
            }
        }
        let result = words_morse.join(" / ");
        out.push_str(&format!("Input:  {}\n", text));
        out.push_str(&format!("Output: {}\n", result));
        out.push_str("\n(letter separator: space, word separator: /)\n");
    } else {
        // Decode Morse to text
        let mut result = String::new();
        for word in text.split('/') {
            for code in word.split_whitespace() {
                let code = code.trim();
                if code.is_empty() {
                    continue;
                }
                if let Some(c) = morse_decode_char(code) {
                    result.push(c);
                } else {
                    result.push('?');
                }
            }
            result.push(' ');
        }
        let result = result.trim().to_string();
        out.push_str(&format!("Input:  {}\n", text));
        out.push_str(&format!("Output: {}\n", result));
    }
    Ok(out)
}

fn action_spell(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let separator = args
        .get("separator")
        .and_then(|v| v.as_str())
        .unwrap_or(", ");

    let mut out = String::from("nato_tools â€” spell\n\n");
    out.push_str(&format!("Input: {}\n\n", text));

    let spelled: Vec<String> = text
        .chars()
        .map(|c| {
            if c == ' ' {
                "[space]".to_string()
            } else if let Some(w) = nato_word(c) {
                format!("{} ({})", c.to_uppercase(), w)
            } else {
                format!("[{}]", c)
            }
        })
        .collect();

    out.push_str(&spelled.join(separator));
    out.push('\n');
    Ok(out)
}
