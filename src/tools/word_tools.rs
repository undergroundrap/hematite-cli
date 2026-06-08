use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("frequency");
    match action {
        "frequency" | "freq" => action_frequency(args),
        "anagram" => action_anagram(args),
        "soundex" => action_soundex(args),
        "palindrome" => action_palindrome(args),
        "syllables" => action_syllables(args),
        other => Err(format!(
            "word_tools: unknown action '{other}'. Valid: frequency, anagram, soundex, palindrome, syllables"
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "word_tools: 'text' field is required".to_string())
}

fn tokenize_words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect()
}

// ── Soundex ───────────────────────────────────────────────────────────────────

fn soundex_code(word: &str) -> String {
    let word = word.to_uppercase();
    let chars: Vec<char> = word.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if chars.is_empty() {
        return String::new();
    }
    let first = chars[0];
    let encode = |c: char| match c {
        'B' | 'F' | 'P' | 'V' => '1',
        'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => '2',
        'D' | 'T' => '3',
        'L' => '4',
        'M' | 'N' => '5',
        'R' => '6',
        _ => '0', // A E I O U H W Y
    };
    let mut code = String::new();
    code.push(first);
    let mut prev = encode(first);
    for &c in &chars[1..] {
        let digit = encode(c);
        if digit != '0' && digit != prev {
            code.push(digit);
            if code.len() == 4 {
                break;
            }
        }
        if digit != '0' {
            prev = digit;
        }
    }
    while code.len() < 4 {
        code.push('0');
    }
    code
}

// ── Syllable estimation ───────────────────────────────────────────────────────

fn count_syllables(word: &str) -> usize {
    let word = word.to_lowercase();
    if word.is_empty() {
        return 0;
    }
    let vowels = "aeiouy";
    let chars: Vec<char> = word.chars().collect();
    let mut count: i32 = 0;
    let mut prev_vowel = false;

    for &c in &chars {
        let is_vowel = vowels.contains(c);
        if is_vowel && !prev_vowel {
            count += 1;
        }
        prev_vowel = is_vowel;
    }

    // Silent trailing 'e'
    if word.ends_with('e') && word.len() > 2 {
        count -= 1;
    }
    // "le" ending after consonant counts as syllable
    if word.len() >= 3 && word.ends_with("le") {
        let pre = chars[chars.len() - 3];
        if !"aeiouy".contains(pre) {
            count += 1;
        }
    }

    count.max(1) as usize
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_frequency(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let top_n = args.get("top").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let stop = args
        .get("stop_words")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let stop_words: std::collections::HashSet<&str> = if stop {
        [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "from", "as", "is", "was", "are", "were", "be", "been", "has", "have", "had",
            "do", "does", "did", "will", "would", "could", "should", "may", "might", "shall",
            "can", "it", "its", "this", "that", "these", "those", "i", "you", "he", "she", "we",
            "they", "me", "him", "her", "us", "them", "my", "your", "his", "not", "no", "so", "if",
            "then", "than", "there", "their", "they",
        ]
        .iter()
        .copied()
        .collect()
    } else {
        std::collections::HashSet::new()
    };

    let words = tokenize_words(&text);
    let total = words.len();
    if total == 0 {
        return Err("word_tools: no words found in input".to_string());
    }

    let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for w in &words {
        if !stop || !stop_words.contains(w.as_str()) {
            *freq.entry(w.clone()).or_insert(0) += 1;
        }
    }

    let mut pairs: Vec<(String, usize)> = freq.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let unique = pairs.len();
    let shown = pairs.len().min(top_n);

    let mut out = "word_tools — frequency\n\n".to_string();
    out.push_str(&format!(
        "Total words: {}  Unique (after filter): {}\n\n",
        total, unique
    ));
    out.push_str(&format!("{:<20} {:>6}  {:>6}\n", "Word", "Count", "Freq%"));
    out.push_str(&format!("{}\n", "─".repeat(38)));
    for (word, count) in pairs.iter().take(shown) {
        let pct = (*count as f64 / total as f64) * 100.0;
        out.push_str(&format!("{:<20} {:>6}  {:>5.1}%\n", word, count, pct));
    }
    if unique > shown {
        out.push_str(&format!(
            "\n… {} more unique words not shown\n",
            unique - shown
        ));
    }
    Ok(out)
}

fn action_anagram(args: &Value) -> Result<String, String> {
    let word_a = args
        .get("a")
        .or_else(|| args.get("word_a"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "word_tools: 'a' field required for anagram check".to_string())?;
    let word_b = args
        .get("b")
        .or_else(|| args.get("word_b"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "word_tools: 'b' field required for anagram check".to_string())?;

    let normalize = |s: &str| -> Vec<char> {
        let mut v: Vec<char> = s
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect();
        v.sort_unstable();
        v
    };

    let na = normalize(word_a);
    let nb = normalize(word_b);
    let is_anagram = na == nb;

    let mut freq_a: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    let mut freq_b: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    for c in word_a.to_lowercase().chars().filter(|c| c.is_alphabetic()) {
        *freq_a.entry(c).or_insert(0) += 1;
    }
    for c in word_b.to_lowercase().chars().filter(|c| c.is_alphabetic()) {
        *freq_b.entry(c).or_insert(0) += 1;
    }

    let mut all_chars: std::collections::BTreeSet<char> = std::collections::BTreeSet::new();
    all_chars.extend(freq_a.keys());
    all_chars.extend(freq_b.keys());

    let mut out = "word_tools — anagram\n\n".to_string();
    out.push_str(&format!("  A: \"{}\"  ({} letters)\n", word_a, na.len()));
    out.push_str(&format!("  B: \"{}\"  ({} letters)\n\n", word_b, nb.len()));
    out.push_str(&format!(
        "  Result: {}\n\n",
        if is_anagram {
            "✓ ANAGRAM"
        } else {
            "✗ NOT an anagram"
        }
    ));

    if !is_anagram {
        let mut missing_in_b: Vec<(char, usize)> = Vec::new();
        let mut missing_in_a: Vec<(char, usize)> = Vec::new();
        for c in &all_chars {
            let ca = *freq_a.get(c).unwrap_or(&0);
            let cb = *freq_b.get(c).unwrap_or(&0);
            if ca > cb {
                missing_in_b.push((*c, ca - cb));
            } else if cb > ca {
                missing_in_a.push((*c, cb - ca));
            }
        }
        if !missing_in_b.is_empty() {
            out.push_str("  B missing letters: ");
            for (c, n) in &missing_in_b {
                out.push_str(&format!("'{}' ×{}  ", c, n));
            }
            out.push('\n');
        }
        if !missing_in_a.is_empty() {
            out.push_str("  A missing letters: ");
            for (c, n) in &missing_in_a {
                out.push_str(&format!("'{}' ×{}  ", c, n));
            }
            out.push('\n');
        }
    }

    // Letter frequency table
    out.push_str("\n  Letter breakdown:\n");
    out.push_str(&format!("  {:<8}  {:>4}  {:>4}\n", "Char", "A", "B"));
    out.push_str(&format!("  {}\n", "─".repeat(20)));
    for c in &all_chars {
        let ca = *freq_a.get(c).unwrap_or(&0);
        let cb = *freq_b.get(c).unwrap_or(&0);
        let flag = if ca == cb { "" } else { " ←" };
        out.push_str(&format!("  {:<8}  {:>4}  {:>4}{}\n", c, ca, cb, flag));
    }
    Ok(out)
}

fn action_soundex(args: &Value) -> Result<String, String> {
    // Accept single word or list of words
    let words: Vec<String> = if let Some(arr) = args.get("words").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect()
    } else if let Some(w) = args.get("word").and_then(|v| v.as_str()) {
        vec![w.to_string()]
    } else if let Some(t) = args.get("text").and_then(|v| v.as_str()) {
        tokenize_words(t)
    } else {
        return Err("word_tools: provide 'word', 'words', or 'text' for soundex".to_string());
    };

    if words.is_empty() {
        return Err("word_tools: no words provided for soundex".to_string());
    }

    // Group by soundex code
    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for w in &words {
        let code = soundex_code(w);
        groups.entry(code).or_default().push(w.clone());
    }

    let mut out = "word_tools — soundex\n\n".to_string();
    if words.len() == 1 {
        let code = soundex_code(&words[0]);
        out.push_str(&format!("  Word:    \"{}\"\n", words[0]));
        out.push_str(&format!("  Soundex: {}\n", code));
    } else {
        out.push_str(&format!("  {} words analyzed\n\n", words.len()));
        out.push_str(&format!("{:<10}  Words\n", "Code"));
        out.push_str(&format!("{}\n", "─".repeat(50)));
        for (code, grp) in &groups {
            let phonetically_similar = grp.len() > 1;
            let flag = if phonetically_similar {
                " ← sound-alikes"
            } else {
                ""
            };
            out.push_str(&format!("{:<10}  {}{}\n", code, grp.join(", "), flag));
        }
        let similar_count = groups.values().filter(|g| g.len() > 1).count();
        if similar_count > 0 {
            out.push_str(&format!(
                "\n  {} group(s) with phonetically similar words\n",
                similar_count
            ));
        }
    }
    Ok(out)
}

fn action_palindrome(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let strict = args
        .get("strict")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Check whole-text palindrome
    let cleaned: String = if strict {
        text.chars().collect()
    } else {
        text.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect()
    };

    let reversed: String = cleaned.chars().rev().collect();
    let is_palindrome = cleaned == reversed;

    let mut out = "word_tools — palindrome\n\n".to_string();
    out.push_str(&format!("  Input:   \"{}\"\n", text.trim()));
    if !strict {
        out.push_str(&format!("  Cleaned: \"{}\"\n", cleaned));
    }
    out.push_str(&format!(
        "  Result:  {}\n",
        if is_palindrome {
            "✓ PALINDROME"
        } else {
            "✗ Not a palindrome"
        }
    ));

    // Also check each word
    let words = tokenize_words(&text);
    if words.len() > 1 {
        let word_palindromes: Vec<&String> = words
            .iter()
            .filter(|w| {
                let c: String = w.chars().collect();
                let r: String = c.chars().rev().collect();
                c == r && c.len() > 1
            })
            .collect();
        if !word_palindromes.is_empty() {
            out.push_str(&format!(
                "\n  Palindrome words found: {}\n",
                word_palindromes
                    .iter()
                    .map(|w| format!("\"{}\"", w))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    // Longest palindromic substring
    if !strict && text.len() <= 200 {
        let chars: Vec<char> = cleaned.chars().collect();
        let n = chars.len();
        let mut best_start = 0;
        let mut best_len = 1;
        for center in 0..n {
            // Odd length
            let mut l = center as i64;
            let mut r = center as i64;
            while l >= 0 && r < n as i64 && chars[l as usize] == chars[r as usize] {
                if (r - l + 1) as usize > best_len {
                    best_len = (r - l + 1) as usize;
                    best_start = l as usize;
                }
                l -= 1;
                r += 1;
            }
            // Even length
            let mut l = center as i64;
            let mut r = center as i64 + 1;
            while l >= 0 && r < n as i64 && chars[l as usize] == chars[r as usize] {
                if (r - l + 1) as usize > best_len {
                    best_len = (r - l + 1) as usize;
                    best_start = l as usize;
                }
                l -= 1;
                r += 1;
            }
        }
        if best_len > 1 {
            let sub: String = chars[best_start..best_start + best_len].iter().collect();
            out.push_str(&format!(
                "\n  Longest palindromic substring: \"{}\" ({} chars)\n",
                sub, best_len
            ));
        }
    }
    Ok(out)
}

fn action_syllables(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let words = tokenize_words(&text);

    if words.is_empty() {
        return Err("word_tools: no words found in input".to_string());
    }

    let counts: Vec<(String, usize)> = words
        .iter()
        .map(|w| (w.clone(), count_syllables(w)))
        .collect();

    let total_syllables: usize = counts.iter().map(|(_, c)| c).sum();
    let total_words = counts.len();
    let avg = total_syllables as f64 / total_words as f64;

    // Flesch-Kincaid grade level proxy (no sentence count available — approximate)
    let sentences = text
        .chars()
        .filter(|c| matches!(c, '.' | '!' | '?'))
        .count()
        .max(1);
    let fk_grade = 0.39 * (total_words as f64 / sentences as f64) + 11.8 * avg - 15.59;

    let mut out = "word_tools — syllables\n\n".to_string();
    out.push_str(&format!(
        "  Total words:     {}\n\
         Total syllables: {}\n\
         Avg per word:    {:.2}\n\
         FK grade (est):  {:.1}\n\n",
        total_words, total_syllables, avg, fk_grade
    ));

    // Breakdown by syllable count
    let mut by_count: std::collections::BTreeMap<usize, Vec<String>> =
        std::collections::BTreeMap::new();
    for (w, c) in &counts {
        by_count.entry(*c).or_default().push(w.clone());
    }
    out.push_str(&format!("{:<6}  {:>6}  Examples\n", "Sylls", "Words"));
    out.push_str(&format!("{}\n", "─".repeat(50)));
    for (sylls, words_in_group) in &by_count {
        let examples: Vec<&String> = words_in_group.iter().take(5).collect();
        let more = if words_in_group.len() > 5 {
            format!(" +{}", words_in_group.len() - 5)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{:<6}  {:>6}  {}{}\n",
            sylls,
            words_in_group.len(),
            examples
                .iter()
                .map(|w| w.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            more
        ));
    }

    // Hardest words (most syllables)
    let mut sorted = counts.clone();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    sorted.dedup_by_key(|e| e.0.clone());
    let top: Vec<_> = sorted.iter().take(5).collect();
    if top[0].1 > 1 {
        out.push_str("\n  Most complex words:\n");
        for (w, c) in &top {
            out.push_str(&format!(
                "    {} — {} syllable{}\n",
                w,
                c,
                if *c == 1 { "" } else { "s" }
            ));
        }
    }
    Ok(out)
}
