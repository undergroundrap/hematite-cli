use serde_json::{json, Value};

pub fn number_words_tools_schema() -> Value {
    json!({
        "name": "number_words_tools",
        "description": "Convert numbers to English words and back without external utilities. Actions: to_words (number → English text like 'one hundred twenty-three'), to_ordinal (1 → 'first', 42 → 'forty-second'), from_words (English text → number), currency (123.45 → 'one hundred twenty-three dollars and forty-five cents'), digits (spell each digit individually: 123 → 'one two three'), roman (integer → Roman numeral or Roman → integer). Handles integers up to 999 quadrillion and negative numbers.",
        "parameters": {
            "type": "object",
            "properties": {
                "number": {
                    "type": "number",
                    "description": "The number to convert (for to_words, to_ordinal, currency, digits, roman)"
                },
                "text": {
                    "type": "string",
                    "description": "English word text to parse back to a number (for from_words), or Roman numeral string (for roman with decode direction)"
                },
                "action": {
                    "type": "string",
                    "enum": ["to_words", "to_ordinal", "from_words", "currency", "digits", "roman"],
                    "description": "Action to perform (default: to_words)"
                },
                "currency_name": {
                    "type": "string",
                    "description": "Currency name for 'currency' action (default: 'dollar'/'dollars'/'cent'/'cents')"
                },
                "uppercase": {
                    "type": "boolean",
                    "description": "Return result in ALL CAPS (default: false)"
                }
            },
            "required": []
        }
    })
}

// ── word tables ───────────────────────────────────────────────────────────────

const ONES: &[&str] = &[
    "",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];

const TENS: &[&str] = &[
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

const ORDINAL_ONES: &[&str] = &[
    "",
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "sixth",
    "seventh",
    "eighth",
    "ninth",
    "tenth",
    "eleventh",
    "twelfth",
    "thirteenth",
    "fourteenth",
    "fifteenth",
    "sixteenth",
    "seventeenth",
    "eighteenth",
    "nineteenth",
];

const ORDINAL_TENS: &[&str] = &[
    "",
    "",
    "twentieth",
    "thirtieth",
    "fortieth",
    "fiftieth",
    "sixtieth",
    "seventieth",
    "eightieth",
    "ninetieth",
];

// (divisor, name)
const SCALE: &[(u64, &str)] = &[
    (1_000_000_000_000_000, "quadrillion"),
    (1_000_000_000_000, "trillion"),
    (1_000_000_000, "billion"),
    (1_000_000, "million"),
    (1_000, "thousand"),
    (100, "hundred"),
];

// ── core converter ────────────────────────────────────────────────────────────

fn hundreds_words(n: u64) -> String {
    debug_assert!(n < 1000);
    if n == 0 {
        return String::new();
    }
    let mut parts = Vec::new();
    let h = n / 100;
    let rem = n % 100;
    if h > 0 {
        parts.push(format!("{} hundred", ONES[h as usize]));
    }
    if rem > 0 {
        if rem < 20 {
            parts.push(ONES[rem as usize].to_string());
        } else {
            let t = rem / 10;
            let o = rem % 10;
            if o == 0 {
                parts.push(TENS[t as usize].to_string());
            } else {
                parts.push(format!("{}-{}", TENS[t as usize], ONES[o as usize]));
            }
        }
    }
    parts.join(" ")
}

fn int_to_words(n: u64) -> String {
    if n == 0 {
        return "zero".to_string();
    }
    let mut parts: Vec<String> = Vec::new();
    let mut rem = n;

    for &(div, name) in SCALE {
        if div == 100 {
            // hundreds handled inside hundreds_words, skip standalone
            continue;
        }
        if rem >= div {
            let chunk = rem / div;
            rem %= div;
            parts.push(format!("{} {}", int_to_words(chunk), name));
        }
    }

    // remaining < 1000
    if rem > 0 {
        parts.push(hundreds_words(rem));
    }

    parts.join(" ")
}

fn n_to_words(n: i64) -> String {
    if n < 0 {
        format!("negative {}", int_to_words((-n) as u64))
    } else {
        int_to_words(n as u64)
    }
}

// ── ordinal ───────────────────────────────────────────────────────────────────

fn ones_ordinal(n: u64) -> String {
    debug_assert!(n < 1000);
    if n == 0 {
        return String::new();
    }
    let rem = n % 100;
    let h = n / 100;
    let mut base = if h > 0 {
        format!("{} hundred", ONES[h as usize])
    } else {
        String::new()
    };

    if rem == 0 && h > 0 {
        // e.g. "three hundredth"
        base.push_str("th");
        // "one hundred" → "one hundredth"
        return base.replacen("hundred", "hundredth", 1);
    }
    if rem > 0 {
        let suffix = if rem < 20 {
            ORDINAL_ONES[rem as usize].to_string()
        } else {
            let t = rem / 10;
            let o = rem % 10;
            if o == 0 {
                ORDINAL_TENS[t as usize].to_string()
            } else {
                format!("{}-{}", TENS[t as usize], ORDINAL_ONES[o as usize])
            }
        };
        if base.is_empty() {
            suffix
        } else {
            format!("{} {}", base, suffix)
        }
    } else {
        base
    }
}

fn int_to_ordinal(n: u64) -> String {
    if n == 0 {
        return "zeroth".to_string();
    }
    let mut parts: Vec<String> = Vec::new();
    let mut rem = n;

    for &(div, name) in SCALE {
        if div == 100 {
            continue;
        }
        if rem >= div {
            let chunk = rem / div;
            rem %= div;
            if rem == 0 {
                // The scale word becomes ordinal
                let ord_name = match name {
                    "quadrillion" => "quadrillionth",
                    "trillion" => "trillionth",
                    "billion" => "billionth",
                    "million" => "millionth",
                    "thousand" => "thousandth",
                    _ => name,
                };
                parts.push(format!("{} {}", int_to_words(chunk), ord_name));
                return parts.join(" ");
            } else {
                parts.push(format!("{} {}", int_to_words(chunk), name));
            }
        }
    }

    // remaining < 1000
    parts.push(ones_ordinal(rem));
    parts.join(" ")
}

fn n_to_ordinal(n: i64) -> String {
    if n < 0 {
        format!("negative {}", int_to_ordinal((-n) as u64))
    } else {
        int_to_ordinal(n as u64)
    }
}

// ── from_words parser ─────────────────────────────────────────────────────────

fn parse_word_token(tok: &str) -> Option<i64> {
    match tok {
        "zero" => Some(0),
        "one" | "a" | "an" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        "thirteen" => Some(13),
        "fourteen" => Some(14),
        "fifteen" => Some(15),
        "sixteen" => Some(16),
        "seventeen" => Some(17),
        "eighteen" => Some(18),
        "nineteen" => Some(19),
        "twenty" => Some(20),
        "thirty" => Some(30),
        "forty" => Some(40),
        "fifty" => Some(50),
        "sixty" => Some(60),
        "seventy" => Some(70),
        "eighty" => Some(80),
        "ninety" => Some(90),
        "hundred" => Some(100),
        "thousand" => Some(1_000),
        "million" => Some(1_000_000),
        "billion" => Some(1_000_000_000),
        "trillion" => Some(1_000_000_000_000),
        "quadrillion" => Some(1_000_000_000_000_000),
        _ => None,
    }
}

fn words_to_int(text: &str) -> Result<i64, String> {
    let lower = text.to_lowercase();
    let lower = lower.replace(['-', ','], " ").replace(" and ", " ");
    let tokens: Vec<&str> = lower.split_whitespace().filter(|t| !t.is_empty()).collect();

    if tokens.is_empty() {
        return Err("No words provided".to_string());
    }

    let negative = tokens[0] == "negative" || tokens[0] == "minus";
    let tokens = if negative { &tokens[1..] } else { &tokens[..] };

    if tokens.is_empty() {
        return Err("No number words after negative/minus".to_string());
    }

    // Convert each token
    let mut values: Vec<i64> = Vec::new();
    for tok in tokens {
        // Handle hyphenated like "twenty-three" already split
        match parse_word_token(tok) {
            Some(v) => values.push(v),
            None => return Err(format!("Unrecognised word: '{}'", tok)),
        }
    }

    // Reduce: apply hundred multiplier, then scale (thousand/million/etc.)
    // Pass 1: apply "hundred"
    let mut p1: Vec<i64> = Vec::new();
    let mut i = 0;
    while i < values.len() {
        if values[i] == 100 {
            if p1.is_empty() {
                p1.push(100);
            } else {
                let top = p1.pop().unwrap();
                p1.push(top * 100);
            }
        } else {
            p1.push(values[i]);
        }
        i += 1;
    }

    // Pass 2: accumulate sub-groups and multiply by scale words
    let mut total: i64 = 0;
    let mut current: i64 = 0;
    for v in p1 {
        if v >= 1_000 {
            current += if current == 0 { 1 } else { 0 }; // edge: "thousand" alone
            total += current * v;
            current = 0;
        } else {
            current += v;
        }
    }
    total += current;

    Ok(if negative { -total } else { total })
}

// ── currency ──────────────────────────────────────────────────────────────────

fn currency_words(amount: f64, currency_name: &str) -> String {
    // Determine singular/plural of the unit names
    let (unit_s, unit_p, cent_s, cent_p) = match currency_name.to_lowercase().as_str() {
        "euro" | "eur" => ("euro", "euros", "cent", "cents"),
        "pound" | "gbp" => ("pound", "pounds", "penny", "pence"),
        "yen" | "jpy" => ("yen", "yen", "sen", "sen"),
        _ => ("dollar", "dollars", "cent", "cents"),
    };

    let abs_amount = amount.abs();
    let whole = abs_amount.floor() as i64;
    let cents = ((abs_amount * 100.0).round() as i64) % 100;

    let mut out = if amount < 0.0 {
        "negative ".to_string()
    } else {
        String::new()
    };

    let unit = if whole == 1 { unit_s } else { unit_p };
    out.push_str(&format!("{} {}", int_to_words(whole as u64), unit));

    if cents > 0 {
        let cent_unit = if cents == 1 { cent_s } else { cent_p };
        out.push_str(&format!(
            " and {} {}",
            int_to_words(cents as u64),
            cent_unit
        ));
    }
    out
}

// ── digits ────────────────────────────────────────────────────────────────────

fn spell_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0' => "zero",
            '1' => "one",
            '2' => "two",
            '3' => "three",
            '4' => "four",
            '5' => "five",
            '6' => "six",
            '7' => "seven",
            '8' => "eight",
            '9' => "nine",
            '-' => "negative",
            '.' => "point",
            _ => "",
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

// ── roman numerals ────────────────────────────────────────────────────────────

const ROMAN_VALS: &[(u32, &str)] = &[
    (1000, "M"),
    (900, "CM"),
    (500, "D"),
    (400, "CD"),
    (100, "C"),
    (90, "XC"),
    (50, "L"),
    (40, "XL"),
    (10, "X"),
    (9, "IX"),
    (5, "V"),
    (4, "IV"),
    (1, "I"),
];

fn int_to_roman(n: u32) -> Result<String, String> {
    if n == 0 || n > 3999 {
        return Err(format!("Roman numerals only support 1–3999 (got {})", n));
    }
    let mut rem = n;
    let mut out = String::new();
    for &(val, sym) in ROMAN_VALS {
        while rem >= val {
            out.push_str(sym);
            rem -= val;
        }
    }
    Ok(out)
}

fn roman_to_int(s: &str) -> Result<u32, String> {
    let upper = s.trim().to_uppercase();
    let bytes = upper.as_bytes();
    let roman_val = |c: u8| -> Option<u32> {
        match c {
            b'I' => Some(1),
            b'V' => Some(5),
            b'X' => Some(10),
            b'L' => Some(50),
            b'C' => Some(100),
            b'D' => Some(500),
            b'M' => Some(1000),
            _ => None,
        }
    };
    if bytes.is_empty() {
        return Err("Empty Roman numeral".to_string());
    }
    let mut total = 0u32;
    let mut i = 0;
    while i < bytes.len() {
        let cur = roman_val(bytes[i])
            .ok_or_else(|| format!("Invalid Roman numeral character: '{}'", bytes[i] as char))?;
        let next = if i + 1 < bytes.len() {
            roman_val(bytes[i + 1])
        } else {
            None
        };
        if let Some(nv) = next {
            if nv > cur {
                total += nv - cur;
                i += 2;
                continue;
            }
        }
        total += cur;
        i += 1;
    }
    if total == 0 || total > 3999 {
        return Err(format!("Roman numeral out of range: {}", total));
    }
    Ok(total)
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_to_words(args: &Value) -> Result<String, String> {
    let n = args
        .get("number")
        .and_then(|v| v.as_f64())
        .ok_or("'number' field is required")?;

    let i = n as i64;
    if (i as f64 - n).abs() > 0.5 {
        return Err(format!("'to_words' requires an integer, got {}", n));
    }
    let words = n_to_words(i);

    let mut out = format!("Number:  {}\n", i);
    out.push_str(&"─".repeat(40));
    out.push('\n');
    out.push_str(&format!("Words:   {}\n", words));
    Ok(out)
}

fn action_to_ordinal(args: &Value) -> Result<String, String> {
    let n = args
        .get("number")
        .and_then(|v| v.as_f64())
        .ok_or("'number' field is required")?;

    let i = n as i64;
    if (i as f64 - n).abs() > 0.5 {
        return Err(format!("'to_ordinal' requires an integer, got {}", n));
    }
    let ord = n_to_ordinal(i);

    let mut out = format!("Number:  {}\n", i);
    out.push_str(&"─".repeat(40));
    out.push('\n');
    out.push_str(&format!("Ordinal: {}\n", ord));
    Ok(out)
}

fn action_from_words(args: &Value) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("'text' field is required for from_words")?;

    let n = words_to_int(text)?;

    let mut out = format!("Input:   {}\n", text);
    out.push_str(&"─".repeat(40));
    out.push('\n');
    out.push_str(&format!("Number:  {}\n", n));
    Ok(out)
}

fn action_currency(args: &Value) -> Result<String, String> {
    let n = args
        .get("number")
        .and_then(|v| v.as_f64())
        .ok_or("'number' field is required")?;

    let currency_name = args
        .get("currency_name")
        .and_then(|v| v.as_str())
        .unwrap_or("dollar");

    let words = currency_words(n, currency_name);

    let mut out = format!("Amount:  {:.2}\n", n);
    out.push_str(&"─".repeat(40));
    out.push('\n');
    out.push_str(&format!("Words:   {}\n", words));
    Ok(out)
}

fn action_digits(args: &Value) -> Result<String, String> {
    let n = args
        .get("number")
        .and_then(|v| v.as_f64())
        .ok_or("'number' field is required")?;

    let s = format!("{}", n);
    let spelled = spell_digits(&s);

    let mut out = format!("Number:  {}\n", s);
    out.push_str(&"─".repeat(40));
    out.push('\n');
    out.push_str(&format!("Digits:  {}\n", spelled));
    Ok(out)
}

fn action_roman(args: &Value) -> Result<String, String> {
    // If 'text' is provided → decode Roman → integer
    // If 'number' is provided → encode integer → Roman
    if let Some(text) = args.get("text").and_then(|v| v.as_str()) {
        let n = roman_to_int(text)?;
        let words = int_to_words(n as u64);
        let mut out = format!("Roman:   {}\n", text.trim().to_uppercase());
        out.push_str(&"─".repeat(40));
        out.push('\n');
        out.push_str(&format!("Integer: {}\n", n));
        out.push_str(&format!("Words:   {}\n", words));
        return Ok(out);
    }
    let n = args
        .get("number")
        .and_then(|v| v.as_f64())
        .ok_or("'number' or 'text' field is required for roman")?;

    let i = n as u32;
    if (i as f64 - n).abs() > 0.5 {
        return Err(format!(
            "Roman numerals require a positive integer, got {}",
            n
        ));
    }
    let roman = int_to_roman(i)?;
    let words = int_to_words(i as u64);

    let mut out = format!("Number:  {}\n", i);
    out.push_str(&"─".repeat(40));
    out.push('\n');
    out.push_str(&format!("Roman:   {}\n", roman));
    out.push_str(&format!("Words:   {}\n", words));
    Ok(out)
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("to_words");

    let uppercase = args
        .get("uppercase")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let result = match action {
        "to_words" => action_to_words(args),
        "to_ordinal" => action_to_ordinal(args),
        "from_words" => action_from_words(args),
        "currency" => action_currency(args),
        "digits" => action_digits(args),
        "roman" => action_roman(args),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: to_words, to_ordinal, from_words, currency, digits, roman"
        )),
    };

    if uppercase {
        result.map(|s| s.to_uppercase())
    } else {
        result
    }
}
