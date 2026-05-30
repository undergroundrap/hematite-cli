use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("rot13");
    match action {
        "rot13" => action_rot13(args),
        "caesar" => action_caesar(args),
        "vigenere" => action_vigenere(args),
        "atbash" => action_atbash(args),
        "rail_fence" => action_rail_fence(args),
        "analyze" => action_analyze(args),
        other => Err(format!(
            "Unknown action '{other}'. Use: rot13, caesar, vigenere, atbash, rail_fence, analyze"
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

fn rot13_char(c: char) -> char {
    match c {
        'a'..='z' => (((c as u8 - b'a' + 13) % 26) + b'a') as char,
        'A'..='Z' => (((c as u8 - b'A' + 13) % 26) + b'A') as char,
        _ => c,
    }
}

fn caesar_shift(c: char, shift: i32, encode: bool) -> char {
    let s = if encode { shift } else { 26 - (shift % 26) };
    match c {
        'a'..='z' => (((c as i32 - 'a' as i32 + s).rem_euclid(26)) as u8 + b'a') as char,
        'A'..='Z' => (((c as i32 - 'A' as i32 + s).rem_euclid(26)) as u8 + b'A') as char,
        _ => c,
    }
}

fn action_rot13(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let result: String = text.chars().map(rot13_char).collect();
    let mut out = String::from("cipher_tools — rot13\n\n");
    out.push_str(&format!("Input:  {}\n", text));
    out.push_str(&format!("Output: {}\n", result));
    out.push_str("\n(ROT13 is its own inverse — apply twice to recover original)\n");
    Ok(out)
}

fn action_caesar(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let shift = args.get("shift").and_then(|v| v.as_i64()).unwrap_or(3) as i32;
    let decode = args
        .get("decode")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let result: String = text
        .chars()
        .map(|c| caesar_shift(c, shift.rem_euclid(26), !decode))
        .collect();

    let mode = if decode { "decode" } else { "encode" };
    let mut out = format!("cipher_tools — caesar ({mode}, shift {})\n\n", shift);
    out.push_str(&format!("Input:  {}\n", text));
    out.push_str(&format!("Output: {}\n", result));

    if !decode {
        // Show brute-force table for short messages
        if text.len() <= 40 {
            out.push_str("\nAll shifts:\n");
            for s in 0i32..26 {
                let candidate: String = text.chars().map(|c| caesar_shift(c, s, true)).collect();
                out.push_str(&format!("  {:>2}: {}\n", s, candidate));
            }
        }
    }
    Ok(out)
}

fn action_vigenere(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'key' field for Vigenère cipher")?;
    let decode = args
        .get("decode")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err("'key' must be a non-empty ASCII alphabetic string".to_string());
    }

    let key_bytes: Vec<u8> = key.to_uppercase().bytes().map(|b| b - b'A').collect();
    let mut ki = 0usize;
    let result: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                let k = key_bytes[ki % key_bytes.len()] as i32;
                ki += 1;
                let shifted = if decode {
                    let base = if c.is_uppercase() { b'A' } else { b'a' };
                    (((c as i32 - base as i32 - k).rem_euclid(26)) as u8 + base) as char
                } else {
                    let base = if c.is_uppercase() { b'A' } else { b'a' };
                    (((c as i32 - base as i32 + k).rem_euclid(26)) as u8 + base) as char
                };
                shifted
            } else {
                c
            }
        })
        .collect();

    let mode = if decode { "decode" } else { "encode" };
    let mut out = format!("cipher_tools — vigenere ({mode})\n\n");
    out.push_str(&format!("Key:    {}\n", key.to_uppercase()));
    out.push_str(&format!("Input:  {}\n", text));
    out.push_str(&format!("Output: {}\n", result));
    Ok(out)
}

fn action_atbash(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let result: String = text
        .chars()
        .map(|c| match c {
            'a'..='z' => (b'z' - (c as u8 - b'a')) as char,
            'A'..='Z' => (b'Z' - (c as u8 - b'A')) as char,
            _ => c,
        })
        .collect();

    let mut out = String::from("cipher_tools — atbash\n\n");
    out.push_str(&format!("Input:  {}\n", text));
    out.push_str(&format!("Output: {}\n", result));
    out.push_str("\n(Atbash maps A↔Z, B↔Y, ... and is its own inverse)\n");
    Ok(out)
}

fn action_rail_fence(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let rails = args.get("rails").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
    let decode = args
        .get("decode")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if rails < 2 {
        return Err("'rails' must be >= 2".to_string());
    }
    if text.is_empty() {
        return Err("'text' must not be empty".to_string());
    }

    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();

    // Build rail assignment for each position
    let mut rail_idx: Vec<usize> = vec![0; n];
    let cycle = 2 * (rails - 1);
    for i in 0..n {
        let pos = i % cycle;
        rail_idx[i] = if pos < rails { pos } else { cycle - pos };
    }

    let result: String = if !decode {
        // Encode: read row by row
        let mut rows: Vec<Vec<char>> = vec![Vec::new(); rails];
        for (i, &c) in chars.iter().enumerate() {
            rows[rail_idx[i]].push(c);
        }
        rows.concat().iter().collect()
    } else {
        // Decode: figure out row lengths, then interleave
        let mut row_lens: Vec<usize> = vec![0; rails];
        for &r in &rail_idx {
            row_lens[r] += 1;
        }

        let mut rows: Vec<Vec<char>> = Vec::new();
        let mut pos = 0;
        for len in &row_lens {
            rows.push(chars[pos..pos + len].to_vec());
            pos += len;
        }

        let mut row_cursors: Vec<usize> = vec![0; rails];
        let mut out_chars: Vec<char> = vec![' '; n];
        for (i, &r) in rail_idx.iter().enumerate() {
            out_chars[i] = rows[r][row_cursors[r]];
            row_cursors[r] += 1;
        }
        out_chars.iter().collect()
    };

    let mode = if decode { "decode" } else { "encode" };
    let mut out = format!("cipher_tools — rail_fence ({mode}, {} rails)\n\n", rails);
    out.push_str(&format!("Input:  {}\n", text));
    out.push_str(&format!("Output: {}\n", result));

    if !decode && n <= 40 {
        out.push_str("\nRail diagram:\n");
        for r in 0..rails {
            out.push_str(&format!("  Rail {}: ", r));
            let row: String = (0..n)
                .map(|i| if rail_idx[i] == r { chars[i] } else { '.' })
                .collect();
            out.push_str(&format!("{}\n", row));
        }
    }
    Ok(out)
}

fn action_analyze(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let letters: Vec<char> = text.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    let total = letters.len();

    if total == 0 {
        return Err("No alphabetic characters to analyze".to_string());
    }

    let mut freq = [0u32; 26];
    for c in &letters {
        freq[(c.to_ascii_lowercase() as u8 - b'a') as usize] += 1;
    }

    // Index of Coincidence
    let ic: f64 = freq
        .iter()
        .map(|&f| f as f64 * (f as f64 - 1.0))
        .sum::<f64>()
        / (total as f64 * (total as f64 - 1.0));

    let mut out = String::from("cipher_tools — analyze\n\n");
    out.push_str(&format!(
        "Length:    {} characters ({} letters)\n",
        text.len(),
        total
    ));
    out.push_str(&format!(
        "IC:        {:.4}  (English ≈ 0.065, random ≈ 0.038)\n",
        ic
    ));

    if ic > 0.060 {
        out.push_str(
            "IC hint:   Likely monoalphabetic (Caesar, Atbash, Vigenère with short key)\n",
        );
    } else if ic < 0.045 {
        out.push_str("IC hint:   Likely polyalphabetic or transposition cipher\n");
    } else {
        out.push_str("IC hint:   Ambiguous — could be short Vigenère key or transposition\n");
    }

    out.push_str("\nLetter frequency (vs English):\n");
    // English approximate frequencies
    let eng = [
        8.2, 1.5, 2.8, 4.3, 12.7, 2.2, 2.0, 6.1, 7.0, 0.2, 0.8, 4.0, 2.4, 6.7, 7.5, 1.9, 0.1, 6.0,
        6.3, 9.1, 2.8, 1.0, 2.4, 0.2, 2.0, 0.1f64,
    ];
    let mut pairs: Vec<(char, u32, f64)> = freq
        .iter()
        .enumerate()
        .map(|(i, &f)| ((b'a' + i as u8) as char, f, f as f64 / total as f64 * 100.0))
        .filter(|(_, f, _)| *f > 0)
        .collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));

    for (c, count, pct) in &pairs {
        let bar_len = (*pct / 0.5).round() as usize;
        let bar: String = "█".repeat(bar_len.min(40));
        out.push_str(&format!(
            "  {} {:>4} {:>5.1}% {} (eng {:.1}%)\n",
            c.to_uppercase(),
            count,
            pct,
            bar,
            eng[((*c as u8) - b'a') as usize]
        ));
    }

    // Caesar break attempt
    let most_common = pairs[0].0 as u8 - b'a';
    let shift_if_e = (most_common as i32 - 4).rem_euclid(26) as u8;
    out.push_str(&format!(
        "\nCaesar break guess (assuming '{}' = 'E'): shift {} → \"{}\"\n",
        pairs[0].0.to_uppercase(),
        shift_if_e,
        text.chars()
            .map(|c| caesar_shift(c, shift_if_e as i32, false))
            .collect::<String>()
    ));

    Ok(out)
}
