use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("estimate");
    match action {
        "estimate" => estimate_action(args),
        "budget" => budget_action(args),
        "compare" => compare_action(args),
        "truncate" => truncate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: estimate, budget, compare, truncate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("content"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' argument".to_string())
}

fn token_estimate(text: &str) -> usize {
    let by_chars = text.len().div_ceil(4);
    let by_words = (text.split_whitespace().count() * 13).div_ceil(10);
    (by_chars + by_words) / 2
}

fn fill_bar(filled: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return ".".repeat(width);
    }
    let f = ((filled as f64 / total as f64) * width as f64) as usize;
    let f = f.min(width);
    "#".repeat(f) + &".".repeat(width - f)
}

fn estimate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let chars = text.len();
    let words = text.split_whitespace().count();
    let lines = text.lines().count();
    let by_chars = chars.div_ceil(4);
    let by_words = (words * 13).div_ceil(10);
    let best = (by_chars + by_words) / 2;

    let mut out = format!("Token Estimate\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Input: {} chars  {} words  {} lines\n\n",
        chars, words, lines
    );
    out += &format!("Chars/4 estimate:    ~{} tokens\n", by_chars);
    out += &format!("Words*1.3 estimate:  ~{} tokens\n", by_words);
    out += &format!("Best estimate:       ~{} tokens\n\n", best);
    out += "Context window fill:\n";
    for (label, size) in [
        ("4K", 4096usize),
        ("8K", 8192),
        ("32K", 32768),
        ("128K", 131072),
    ] {
        let pct = (best as f64 / size as f64 * 100.0).min(100.0);
        out += &format!(
            "  {:5}  [{}]  {:5.1}%\n",
            label,
            fill_bar(best, size, 20),
            pct
        );
    }
    Ok(out)
}

fn budget_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let ctx = args
        .get("context_size")
        .or_else(|| args.get("context"))
        .or_else(|| args.get("size"))
        .and_then(|v| v.as_u64())
        .unwrap_or(8192) as usize;
    let used = token_estimate(&text);
    let remaining = ctx.saturating_sub(used);
    let pct = used as f64 / ctx as f64 * 100.0;
    let status = if pct >= 90.0 {
        "CRITICAL"
    } else if pct >= 70.0 {
        "WARNING"
    } else {
        "OK"
    };
    let mut out = format!(
        "Context Budget  [{} token window]\n{}\n\n",
        ctx,
        "=".repeat(44)
    );
    out += &format!("Used:       ~{} tokens ({:.1}%)\n", used, pct);
    out += &format!(
        "Remaining:  ~{} tokens ({:.1}%)\n",
        remaining,
        100.0 - pct.min(100.0)
    );
    out += &format!("Status:     {}\n\n", status);
    out += &format!("[{}]  {:.1}%\n", fill_bar(used, ctx, 24), pct);
    Ok(out)
}

fn compare_action(args: &Value) -> Result<String, String> {
    let a = args
        .get("a")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'a' (first text)")?;
    let b = args
        .get("b")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'b' (second text)")?;
    let ta = token_estimate(a);
    let tb = token_estimate(b);
    let diff = ta.abs_diff(tb);
    let ratio = if tb > 0 { ta as f64 / tb as f64 } else { 0.0 };
    let mut out = format!("Token Comparison\n{}\n\n", "=".repeat(44));
    out += &format!("A: ~{} tokens  ({} chars)\n", ta, a.len());
    out += &format!("B: ~{} tokens  ({} chars)\n", tb, b.len());
    out += &format!("\nDifference: {} tokens\n", diff);
    out += &format!("Ratio A/B:  {:.2}x\n", ratio);
    if ta < tb {
        out += &format!("A is {:.1}% fewer tokens than B\n", (1.0 - ratio) * 100.0);
    } else if tb < ta {
        out += &format!(
            "B is {:.1}% fewer tokens than A\n",
            (1.0 - 1.0 / ratio) * 100.0
        );
    } else {
        out += "A and B have equal token cost\n";
    }
    Ok(out)
}

fn truncate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let max = args
        .get("max_tokens")
        .or_else(|| args.get("tokens"))
        .or_else(|| args.get("max"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1000) as usize;
    let max_chars = max * 4;
    if text.len() <= max_chars {
        let est = token_estimate(&text);
        return Ok(format!(
            "Text fits within {} token budget (est. ~{} tokens). No truncation needed.\n",
            max, est
        ));
    }
    let raw: String = text.chars().take(max_chars).collect();
    let cut = raw.rfind(|c: char| c.is_whitespace()).unwrap_or(raw.len());
    let truncated = &raw[..cut];
    let actual = token_estimate(truncated);
    let original = token_estimate(&text);
    let mut out = format!(
        "Truncated to ~{} tokens (from ~{}, limit {})\n{}\n\n",
        actual,
        original,
        max,
        "=".repeat(44)
    );
    out += truncated;
    out += "\n[... truncated]";
    Ok(out)
}
