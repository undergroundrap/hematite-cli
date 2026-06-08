use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("levenshtein");
    match action {
        "levenshtein" | "edit_distance" | "edit-distance" => action_levenshtein(args),
        "damerau" | "damerau_levenshtein" => action_damerau(args),
        "jaro" => action_jaro(args),
        "jaro_winkler" | "jaro-winkler" => action_jaro_winkler(args),
        "hamming" => action_hamming(args),
        "lcs" => action_lcs(args),
        "similarity" => action_similarity(args),
        "fuzzy" | "fuzzy_match" => action_fuzzy_match(args),
        other => Err(format!(
            "string_metric_tools: unknown action '{other}'. \
             Valid: levenshtein, damerau, jaro, jaro_winkler, hamming, lcs, similarity, fuzzy"
        )),
    }
}

// ── Core algorithms ───────────────────────────────────────────────────────────

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[m][n]
}

fn damerau_levenshtein_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
            // Transposition
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                dp[i][j] = dp[i][j].min(dp[i - 2][j - 2] + cost);
            }
        }
    }
    dp[m][n]
}

fn jaro_similarity(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let len_a = a.len();
    let len_b = b.len();
    if len_a == 0 && len_b == 0 {
        return 1.0;
    }
    if len_a == 0 || len_b == 0 {
        return 0.0;
    }
    let match_dist = (len_a.max(len_b) / 2).saturating_sub(1);
    let mut a_matched = vec![false; len_a];
    let mut b_matched = vec![false; len_b];
    let mut matches = 0usize;
    for i in 0..len_a {
        let start = i.saturating_sub(match_dist);
        let end = (i + match_dist + 1).min(len_b);
        for j in start..end {
            if !b_matched[j] && a[i] == b[j] {
                a_matched[i] = true;
                b_matched[j] = true;
                matches += 1;
                break;
            }
        }
    }
    if matches == 0 {
        return 0.0;
    }
    let mut transpositions = 0usize;
    let mut k = 0;
    for i in 0..len_a {
        if a_matched[i] {
            while !b_matched[k] {
                k += 1;
            }
            if a[i] != b[k] {
                transpositions += 1;
            }
            k += 1;
        }
    }
    let m = matches as f64;
    (m / len_a as f64 + m / len_b as f64 + (m - transpositions as f64 / 2.0) / m) / 3.0
}

fn jaro_winkler_similarity(a: &str, b: &str) -> f64 {
    let jaro = jaro_similarity(a, b);
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let prefix_len = a_chars
        .iter()
        .zip(b_chars.iter())
        .take(4)
        .take_while(|(x, y)| x == y)
        .count();
    jaro + prefix_len as f64 * 0.1 * (1.0 - jaro)
}

fn hamming_distance(a: &str, b: &str) -> Result<usize, String> {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len() != b.len() {
        return Err(format!(
            "Hamming distance requires equal-length strings: {} vs {} characters",
            a.len(),
            b.len()
        ));
    }
    Ok(a.iter().zip(b.iter()).filter(|(x, y)| x != y).count())
}

fn lcs_length(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }
    dp[m][n]
}

fn lcs_string(a: &str, b: &str) -> String {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }
    // Backtrack
    let mut result = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push(a[i - 1]);
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.iter().rev().collect()
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn get_ab(args: &Value) -> Result<(String, String), String> {
    let a = args
        .get("a")
        .or_else(|| args.get("s1"))
        .or_else(|| args.get("source"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "string_metric_tools: 'a' field is required".to_string())?
        .to_string();
    let b = args
        .get("b")
        .or_else(|| args.get("s2"))
        .or_else(|| args.get("target"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "string_metric_tools: 'b' field is required".to_string())?
        .to_string();
    Ok((a, b))
}

fn action_levenshtein(args: &Value) -> Result<String, String> {
    let (a, b) = get_ab(args)?;
    let case_sensitive = args
        .get("case_sensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let (sa, sb) = if case_sensitive {
        (a.clone(), b.clone())
    } else {
        (a.to_lowercase(), b.to_lowercase())
    };
    let dist = levenshtein_distance(&sa, &sb);
    let max_len = sa.len().max(sb.len());
    let similarity = if max_len == 0 {
        1.0
    } else {
        1.0 - dist as f64 / max_len as f64
    };

    let mut out = "string_metric_tools — levenshtein\n\n".to_string();
    out.push_str(&format!("  A: \"{}\"\n", a));
    out.push_str(&format!("  B: \"{}\"\n\n", b));
    out.push_str(&format!("  Edit distance : {}\n", dist));
    out.push_str(&format!("  Similarity    : {:.1}%\n", similarity * 100.0));
    out.push_str(&format!(
        "  Case-sensitive: {}\n\n",
        if case_sensitive { "yes" } else { "no" }
    ));

    // Show interpretation
    let label = if dist == 0 {
        "Identical"
    } else if dist == 1 {
        "1 operation apart"
    } else if similarity >= 0.9 {
        "Very similar"
    } else if similarity >= 0.7 {
        "Moderately similar"
    } else if similarity >= 0.5 {
        "Somewhat similar"
    } else {
        "Very different"
    };
    out.push_str(&format!("  Assessment    : {}\n", label));

    // Min edits breakdown note
    if dist > 0 && dist <= 5 {
        out.push_str(&format!(
            "  Min operations: {} (insert/delete/substitute)\n",
            dist
        ));
    }
    Ok(out)
}

fn action_damerau(args: &Value) -> Result<String, String> {
    let (a, b) = get_ab(args)?;
    let dist = damerau_levenshtein_distance(&a, &b);
    let lev = levenshtein_distance(&a, &b);
    let max_len = a.len().max(b.len());
    let similarity = if max_len == 0 {
        1.0
    } else {
        1.0 - dist as f64 / max_len as f64
    };

    let mut out = "string_metric_tools — damerau-levenshtein\n\n".to_string();
    out.push_str(&format!("  A: \"{}\"\n", a));
    out.push_str(&format!("  B: \"{}\"\n\n", b));
    out.push_str(&format!("  Damerau distance: {}\n", dist));
    out.push_str(&format!(
        "  Levenshtein dist: {} (no transpositions)\n",
        lev
    ));
    out.push_str(&format!("  Similarity      : {:.1}%\n", similarity * 100.0));
    if lev > dist {
        out.push_str(&format!(
            "\n  Note: {} transposition(s) account for the difference\n",
            lev - dist
        ));
    }
    Ok(out)
}

fn action_jaro(args: &Value) -> Result<String, String> {
    let (a, b) = get_ab(args)?;
    let score = jaro_similarity(&a, &b);

    let label = if score >= 0.95 {
        "Near-identical"
    } else if score >= 0.85 {
        "Very similar"
    } else if score >= 0.70 {
        "Moderately similar"
    } else {
        "Different"
    };

    let mut out = "string_metric_tools — jaro\n\n".to_string();
    out.push_str(&format!("  A: \"{}\"\n", a));
    out.push_str(&format!("  B: \"{}\"\n\n", b));
    out.push_str(&format!(
        "  Jaro similarity: {:.4}  ({:.1}%)\n",
        score,
        score * 100.0
    ));
    out.push_str(&format!("  Assessment     : {}\n", label));
    out.push_str("\n  Range: 0.0 (completely different) to 1.0 (identical)\n");
    Ok(out)
}

fn action_jaro_winkler(args: &Value) -> Result<String, String> {
    let (a, b) = get_ab(args)?;
    let jaro = jaro_similarity(&a, &b);
    let jw = jaro_winkler_similarity(&a, &b);

    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let prefix: String = a_chars
        .iter()
        .zip(b_chars.iter())
        .take(4)
        .take_while(|(x, y)| x == y)
        .map(|(c, _)| *c)
        .collect();

    let mut out = "string_metric_tools — jaro-winkler\n\n".to_string();
    out.push_str(&format!("  A: \"{}\"\n", a));
    out.push_str(&format!("  B: \"{}\"\n\n", b));
    out.push_str(&format!(
        "  Jaro            : {:.4}  ({:.1}%)\n",
        jaro,
        jaro * 100.0
    ));
    out.push_str(&format!(
        "  Jaro-Winkler    : {:.4}  ({:.1}%)\n",
        jw,
        jw * 100.0
    ));
    if !prefix.is_empty() {
        out.push_str(&format!(
            "  Common prefix   : \"{}\" (boosted score by {:.4})\n",
            prefix,
            jw - jaro
        ));
    }
    out.push_str(
        "\n  Jaro-Winkler gives extra weight to strings with matching prefixes.\n\
         Useful for name matching and record linkage.\n",
    );
    Ok(out)
}

fn action_hamming(args: &Value) -> Result<String, String> {
    let (a, b) = get_ab(args)?;
    let dist = hamming_distance(&a, &b)?;
    let len = a.chars().count();
    let similarity = if len == 0 {
        1.0
    } else {
        1.0 - dist as f64 / len as f64
    };

    let mut out = "string_metric_tools — hamming\n\n".to_string();
    out.push_str(&format!("  A: \"{}\"\n", a));
    out.push_str(&format!("  B: \"{}\"\n\n", b));
    out.push_str(&format!("  Hamming distance: {}\n", dist));
    out.push_str(&format!(
        "  Similarity      : {:.1}%\n\n",
        similarity * 100.0
    ));

    // Show differing positions
    let positions: Vec<usize> = a
        .chars()
        .zip(b.chars())
        .enumerate()
        .filter(|(_, (x, y))| x != y)
        .map(|(i, _)| i)
        .collect();
    if !positions.is_empty() && positions.len() <= 10 {
        out.push_str("  Differing positions:\n");
        for pos in &positions {
            let ca: char = a.chars().nth(*pos).unwrap_or('?');
            let cb: char = b.chars().nth(*pos).unwrap_or('?');
            out.push_str(&format!("    [{:>3}] '{}' → '{}'\n", pos, ca, cb));
        }
    }
    Ok(out)
}

fn action_lcs(args: &Value) -> Result<String, String> {
    let (a, b) = get_ab(args)?;
    let lcs = lcs_string(&a, &b);
    let lcs_len = lcs_length(&a, &b);
    let coverage_a = if a.is_empty() {
        1.0
    } else {
        lcs_len as f64 / a.chars().count() as f64
    };
    let coverage_b = if b.is_empty() {
        1.0
    } else {
        lcs_len as f64 / b.chars().count() as f64
    };

    let mut out = "string_metric_tools — lcs\n\n".to_string();
    out.push_str(&format!("  A: \"{}\"\n", a));
    out.push_str(&format!("  B: \"{}\"\n\n", b));
    out.push_str(&format!("  LCS           : \"{}\"\n", lcs));
    out.push_str(&format!("  Length        : {} chars\n", lcs_len));
    out.push_str(&format!("  Coverage of A : {:.1}%\n", coverage_a * 100.0));
    out.push_str(&format!("  Coverage of B : {:.1}%\n", coverage_b * 100.0));
    out.push_str(
        "\n  Longest Common Subsequence — characters in order but not necessarily contiguous.\n",
    );
    Ok(out)
}

fn action_similarity(args: &Value) -> Result<String, String> {
    let (a, b) = get_ab(args)?;

    let lev = levenshtein_distance(&a, &b);
    let max_len = a.len().max(b.len());
    let lev_sim = if max_len == 0 {
        1.0
    } else {
        1.0 - lev as f64 / max_len as f64
    };

    let jaro = jaro_similarity(&a, &b);
    let jw = jaro_winkler_similarity(&a, &b);

    let lcs_len = lcs_length(&a, &b);
    let dice = if a.len() + b.len() == 0 {
        1.0
    } else {
        2.0 * lcs_len as f64 / (a.len() + b.len()) as f64
    };

    let avg = (lev_sim + jaro + jw + dice) / 4.0;

    let mut out = "string_metric_tools — similarity\n\n".to_string();
    out.push_str(&format!("  A: \"{}\"\n", a));
    out.push_str(&format!("  B: \"{}\"\n\n", b));
    out.push_str(&format!(
        "  {:<22}  {:>8}%\n",
        "Levenshtein",
        format!("{:.1}", lev_sim * 100.0)
    ));
    out.push_str(&format!(
        "  {:<22}  {:>8}%\n",
        "Jaro",
        format!("{:.1}", jaro * 100.0)
    ));
    out.push_str(&format!(
        "  {:<22}  {:>8}%\n",
        "Jaro-Winkler",
        format!("{:.1}", jw * 100.0)
    ));
    out.push_str(&format!(
        "  {:<22}  {:>8}%\n",
        "Dice / LCS",
        format!("{:.1}", dice * 100.0)
    ));
    out.push_str(&format!("  {}\n", "─".repeat(34)));
    out.push_str(&format!(
        "  {:<22}  {:>8}%\n\n",
        "Average",
        format!("{:.1}", avg * 100.0)
    ));

    let verdict = if avg >= 0.95 {
        "Near-identical"
    } else if avg >= 0.80 {
        "Very similar — likely same entity"
    } else if avg >= 0.60 {
        "Moderately similar — possible match"
    } else if avg >= 0.40 {
        "Weakly similar"
    } else {
        "Dissimilar"
    };
    out.push_str(&format!("  Verdict: {}\n", verdict));
    Ok(out)
}

fn action_fuzzy_match(args: &Value) -> Result<String, String> {
    let query = args
        .get("query")
        .or_else(|| args.get("a"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "string_metric_tools: 'query' field is required for fuzzy".to_string())?;

    let candidates: Vec<String> = if let Some(arr) =
        args.get("candidates").and_then(|v| v.as_array())
    {
        arr.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect()
    } else if let Some(s) = args.get("candidates").and_then(|v| v.as_str()) {
        s.split('\n')
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    } else {
        return Err("string_metric_tools: 'candidates' array is required for fuzzy".to_string());
    };

    let threshold = args
        .get("threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let top_n = args.get("top").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let ql = query.to_lowercase();
    let mut scored: Vec<(f64, &str)> = candidates
        .iter()
        .map(|c| {
            let cl = c.to_lowercase();
            // Composite score: Jaro-Winkler + LCS coverage boost
            let jw = jaro_winkler_similarity(&ql, &cl);
            let lcs_len = lcs_length(&ql, &cl);
            let lcs_boost = lcs_len as f64 / ql.len().max(1) as f64 * 0.2;
            let score = (jw + lcs_boost).min(1.0);
            (score, c.as_str())
        })
        .filter(|(s, _)| *s >= threshold)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut out = "string_metric_tools — fuzzy match\n\n".to_string();
    out.push_str(&format!("  Query: \"{}\"\n", query));
    out.push_str(&format!(
        "  {} candidates  top {} shown\n\n",
        candidates.len(),
        top_n
    ));
    out.push_str(&format!("  {:<6}  {}\n", "Score", "Candidate"));
    out.push_str(&format!("  {}\n", "─".repeat(40)));

    if scored.is_empty() {
        out.push_str("  No matches above threshold\n");
    } else {
        for (score, candidate) in scored.iter().take(top_n) {
            out.push_str(&format!("  {:.1}%   {}\n", score * 100.0, candidate));
        }
    }
    Ok(out)
}
