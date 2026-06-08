pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("count");

    match action {
        "to-snake" => convert_case(args, Case::Snake),
        "to-camel" => convert_case(args, Case::Camel),
        "to-pascal" => convert_case(args, Case::Pascal),
        "to-kebab" => convert_case(args, Case::Kebab),
        "to-screaming" => convert_case(args, Case::Screaming),
        "to-title" => convert_case(args, Case::Title),
        "to-lower" => convert_case(args, Case::Lower),
        "to-upper" => convert_case(args, Case::Upper),
        "slugify" => slugify(args),
        "count" => count(args),
        "truncate" => truncate(args),
        "pad" => pad(args),
        "wrap" => wrap(args),
        "repeat" => repeat(args),
        "reverse" => reverse(args),
        "lines" => lines(args),
        other => Err(format!(
            "text_tools: unknown action '{other}'. Valid: to-snake, to-camel, to-pascal, to-kebab, \
             to-screaming, to-title, to-lower, to-upper, slugify, count, truncate, pad, wrap, repeat, reverse, lines"
        )),
    }
}

fn get_input(args: &serde_json::Value) -> Result<String, String> {
    args.get("input")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "text_tools: 'input' field is required".to_string())
}

// ── Case detection and splitting ──────────────────────────────────────────────

enum Case {
    Snake,
    Camel,
    Pascal,
    Kebab,
    Screaming,
    Title,
    Lower,
    Upper,
}

/// Split any casing style into lowercase words.
fn split_words(s: &str) -> Vec<String> {
    if s.is_empty() {
        return vec![];
    }

    // Replace common delimiters with spaces, then split on whitespace
    let normalized: String = s
        .chars()
        .map(|c| {
            if c == '-' || c == '_' || c == '.' || c == ' ' {
                ' '
            } else {
                c
            }
        })
        .collect();

    // Now split on uppercase boundaries (camelCase → camel Case) after delimiter split
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();

    for (i, ch) in normalized.char_indices() {
        if ch == ' ' {
            if !current.is_empty() {
                words.push(current.to_lowercase());
                current = String::new();
            }
        } else if ch.is_uppercase() && i > 0 && !current.is_empty() {
            // Split on uppercase transition, but not when previous was also upper (acronyms)
            let prev_lower = current
                .chars()
                .last()
                .map(|c| c.is_lowercase())
                .unwrap_or(false);
            if prev_lower {
                words.push(current.to_lowercase());
                current = String::new();
            }
            current.push(ch);
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        words.push(current.to_lowercase());
    }

    words.into_iter().filter(|w| !w.is_empty()).collect()
}

fn convert_case(args: &serde_json::Value, case: Case) -> Result<String, String> {
    let input = get_input(args)?;

    // Handle multi-line: convert each line independently
    let converted: String = input
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                return line.to_string();
            }
            let words = split_words(line);
            if words.is_empty() {
                return line.to_string();
            }
            match case {
                Case::Snake => words.join("_"),
                Case::Camel => {
                    let mut out = words[0].clone();
                    for w in &words[1..] {
                        let mut c = w.chars();
                        if let Some(first) = c.next() {
                            out.push_str(&first.to_uppercase().to_string());
                            out.push_str(c.as_str());
                        }
                    }
                    out
                }
                Case::Pascal => words
                    .iter()
                    .map(|w| {
                        let mut c = w.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().to_string() + c.as_str(),
                        }
                    })
                    .collect(),
                Case::Kebab => words.join("-"),
                Case::Screaming => words.join("_").to_uppercase(),
                Case::Title => words
                    .iter()
                    .map(|w| {
                        let mut c = w.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().to_string() + c.as_str(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
                Case::Lower => line.to_lowercase(),
                Case::Upper => line.to_uppercase(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let label = match case {
        Case::Snake => "snake_case",
        Case::Camel => "camelCase",
        Case::Pascal => "PascalCase",
        Case::Kebab => "kebab-case",
        Case::Screaming => "SCREAMING_SNAKE_CASE",
        Case::Title => "Title Case",
        Case::Lower => "lowercase",
        Case::Upper => "UPPERCASE",
    };

    Ok(format!(
        "TEXT CASE → {label}\n{}\n{converted}",
        "─".repeat(50)
    ))
}

fn slugify(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let slug: String = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else if c == ' ' || c == '-' || c == '_' {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|c| *c != '\0')
        .collect::<String>()
        // collapse consecutive hyphens
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .trim_matches('-')
        .to_string();

    Ok(format!("SLUGIFY\n{}\n{slug}", "─".repeat(50)))
}

fn count(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;

    let chars = input.chars().count();
    let bytes = input.len();
    let lines = input.lines().count();
    let words = input.split_whitespace().count();
    let sentences = input
        .chars()
        .filter(|c| *c == '.' || *c == '!' || *c == '?')
        .count();

    let mut out = format!("TEXT COUNT\n{}\n", "─".repeat(50));
    out.push_str(&format!("Characters : {chars}\n"));
    out.push_str(&format!("Bytes      : {bytes}\n"));
    out.push_str(&format!("Words      : {words}\n"));
    out.push_str(&format!("Lines      : {lines}\n"));
    out.push_str(&format!("Sentences  : {sentences}\n"));
    Ok(out)
}

fn truncate(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let max = args.get("max").and_then(|v| v.as_u64()).unwrap_or(80) as usize;
    let ellipsis = args
        .get("ellipsis")
        .and_then(|v| v.as_str())
        .unwrap_or("...");

    let chars: Vec<char> = input.chars().collect();
    let result = if chars.len() <= max {
        input.clone()
    } else {
        let keep = max.saturating_sub(ellipsis.chars().count());
        let truncated: String = chars[..keep].iter().collect();
        format!("{truncated}{ellipsis}")
    };

    Ok(format!(
        "TRUNCATE (max {max})\n{}\n{result}",
        "─".repeat(50)
    ))
}

fn pad(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let align = args
        .get("align")
        .and_then(|v| v.as_str())
        .unwrap_or("right");
    let fill = args
        .get("fill")
        .and_then(|v| v.as_str())
        .and_then(|s| s.chars().next())
        .unwrap_or(' ');

    let char_count = input.chars().count();
    let result = if char_count >= width {
        input.clone()
    } else {
        let pad_len = width - char_count;
        match align {
            "left" => {
                let padding: String = std::iter::repeat_n(fill, pad_len).collect();
                format!("{input}{padding}")
            }
            "center" => {
                let left_pad = pad_len / 2;
                let right_pad = pad_len - left_pad;
                let lp: String = std::iter::repeat_n(fill, left_pad).collect();
                let rp: String = std::iter::repeat_n(fill, right_pad).collect();
                format!("{lp}{input}{rp}")
            }
            _ => {
                // right (default)
                let padding: String = std::iter::repeat_n(fill, pad_len).collect();
                format!("{padding}{input}")
            }
        }
    };

    Ok(format!(
        "PAD (width={width}, align={align})\n{}\n{result}",
        "─".repeat(50)
    ))
}

fn wrap(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(80) as usize;

    let mut out_lines = Vec::new();
    for line in input.lines() {
        if line.is_empty() {
            out_lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        let mut current_len = 0usize;
        for word in line.split_whitespace() {
            let word_len = word.chars().count();
            if current.is_empty() {
                current.push_str(word);
                current_len = word_len;
            } else if current_len + 1 + word_len <= width {
                current.push(' ');
                current.push_str(word);
                current_len += 1 + word_len;
            } else {
                out_lines.push(current.clone());
                current = word.to_string();
                current_len = word_len;
            }
        }
        if !current.is_empty() {
            out_lines.push(current);
        }
    }

    Ok(format!(
        "WRAP (width={width})\n{}\n{}",
        "─".repeat(50),
        out_lines.join("\n")
    ))
}

fn repeat(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
    let sep = args.get("sep").and_then(|v| v.as_str()).unwrap_or("");

    if n == 0 {
        return Ok(format!("REPEAT\n{}\n", "─".repeat(50)));
    }

    let result = std::iter::repeat_n(input.as_str(), n)
        .collect::<Vec<_>>()
        .join(sep);

    Ok(format!("REPEAT ×{n}\n{}\n{result}", "─".repeat(50)))
}

fn reverse(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let reversed: String = input.chars().rev().collect();
    Ok(format!("REVERSE\n{}\n{reversed}", "─".repeat(50)))
}

fn lines(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let sort = args.get("sort").and_then(|v| v.as_bool()).unwrap_or(false);
    let dedupe = args
        .get("dedupe")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let filter_empty = args
        .get("filter_empty")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut result: Vec<&str> = input.lines().collect();

    if filter_empty {
        result.retain(|l| !l.trim().is_empty());
    }
    if dedupe {
        let mut seen = std::collections::HashSet::new();
        result.retain(|l| seen.insert(*l));
    }
    if sort {
        result.sort_unstable();
    }

    let count = result.len();
    let out = result.join("\n");
    Ok(format!("LINES ({count} lines)\n{}\n{out}", "─".repeat(50)))
}
