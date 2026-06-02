use serde_json::{json, Value};

pub fn compression_tools_schema() -> Value {
    json!({
        "name": "compression_tools",
        "description": "Lossless text compression and analysis without external utilities. Actions: rle (Run-Length Encoding encode/decode), lz (LZ77-inspired sliding-window compression), analyze (entropy and compressibility estimate for any text), huffman (Huffman coding — symbol frequency table, optimal code lengths, and theoretical compressed size).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["rle", "lz", "analyze", "huffman"],
                    "description": "Action to perform (default: analyze)"
                },
                "text": {
                    "type": "string",
                    "description": "Input text to compress or analyze"
                },
                "encoded": {
                    "type": "string",
                    "description": "Encoded string to decode (for rle decode)"
                },
                "op": {
                    "type": "string",
                    "enum": ["encode", "decode"],
                    "description": "encode or decode (for rle and lz; default: encode)"
                },
                "window": {
                    "type": "integer",
                    "description": "LZ77 sliding window size in characters (default: 32, max: 256)"
                },
                "lookahead": {
                    "type": "integer",
                    "description": "LZ77 lookahead buffer size (default: 16, max: 64)"
                }
            }
        }
    })
}

// ── RLE ────────────────────────────────────────────────────────────────────────

fn rle_encode(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::new();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        let mut count = 1usize;
        while i + count < chars.len() && chars[i + count] == ch {
            count += 1;
        }
        if count > 1 {
            result.push_str(&count.to_string());
        }
        result.push(ch);
        i += count;
    }
    result
}

fn rle_decode(encoded: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut num_buf = String::new();
    for ch in encoded.chars() {
        if ch.is_ascii_digit() {
            num_buf.push(ch);
        } else {
            let count = if num_buf.is_empty() {
                1
            } else {
                num_buf
                    .parse::<usize>()
                    .map_err(|_| format!("invalid count '{}'", num_buf))?
            };
            for _ in 0..count {
                result.push(ch);
            }
            num_buf.clear();
        }
    }
    if !num_buf.is_empty() {
        return Err(format!("trailing number '{}' without a character", num_buf));
    }
    Ok(result)
}

fn action_rle(args: &Value) -> Result<String, String> {
    let op = args.get("op").and_then(|v| v.as_str()).unwrap_or("encode");

    match op {
        "decode" => {
            let encoded = args
                .get("encoded")
                .or_else(|| args.get("text"))
                .and_then(|v| v.as_str())
                .ok_or("pass 'encoded' string to decode")?;
            let decoded = rle_decode(encoded)?;
            let mut lines = Vec::new();
            lines.push(format!("RLE Decode"));
            lines.push(format!(
                "  Input:   {} chars — {:?}",
                encoded.len(),
                encoded
            ));
            lines.push(format!(
                "  Output:  {} chars — {:?}",
                decoded.len(),
                if decoded.len() > 80 {
                    format!("{}...", &decoded[..80])
                } else {
                    decoded.clone()
                }
            ));
            Ok(lines.join("\n"))
        }
        _ => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("pass 'text' to encode")?;
            if text.is_empty() {
                return Err("'text' is empty".to_string());
            }
            let encoded = rle_encode(text);
            let ratio = encoded.len() as f64 / text.len() as f64;
            let saving = if encoded.len() < text.len() {
                format!("{:.1}% smaller", (1.0 - ratio) * 100.0)
            } else {
                format!(
                    "{:.1}% larger (RLE not effective here)",
                    (ratio - 1.0) * 100.0
                )
            };
            let mut lines = Vec::new();
            lines.push(format!("RLE Encode"));
            lines.push(format!(
                "  Input:   {} chars — {:?}",
                text.len(),
                if text.len() > 60 {
                    format!("{}...", &text[..60])
                } else {
                    text.to_string()
                }
            ));
            lines.push(format!(
                "  Encoded: {} chars — {:?}",
                encoded.len(),
                if encoded.len() > 60 {
                    format!("{}...", &encoded[..60])
                } else {
                    encoded.clone()
                }
            ));
            lines.push(format!("  Result:  {}", saving));
            lines.push(format!("  Ratio:   {:.3}", ratio));
            lines.push(String::new());
            lines.push(format!(
                "  Best for: highly repetitive sequences (e.g. 'AAAAAABBB')"
            ));
            lines.push(format!(
                "  Worst for: diverse text — every unique char costs 1 extra byte"
            ));
            Ok(lines.join("\n"))
        }
    }
}

// ── LZ77-inspired ─────────────────────────────────────────────────────────────

#[derive(Debug)]
enum LzToken {
    Literal(char),
    BackRef { offset: usize, length: usize },
}

fn lz_encode(text: &str, window: usize, lookahead: usize) -> Vec<LzToken> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut pos = 0;
    let mut tokens = Vec::new();
    while pos < n {
        let win_start = pos.saturating_sub(window);
        let la_end = (pos + lookahead).min(n);
        let mut best_len = 0usize;
        let mut best_offset = 0usize;
        for start in win_start..pos {
            let mut len = 0;
            while pos + len < la_end && chars[start + len] == chars[pos + len] {
                len += 1;
                if start + len >= pos {
                    break;
                } // don't overlap source
            }
            if len > best_len {
                best_len = len;
                best_offset = pos - start;
            }
        }
        if best_len >= 3 {
            tokens.push(LzToken::BackRef {
                offset: best_offset,
                length: best_len,
            });
            pos += best_len;
        } else {
            tokens.push(LzToken::Literal(chars[pos]));
            pos += 1;
        }
    }
    tokens
}

fn lz_decode(tokens: &[LzToken]) -> String {
    let mut out: Vec<char> = Vec::new();
    for tok in tokens {
        match tok {
            LzToken::Literal(c) => out.push(*c),
            LzToken::BackRef { offset, length } => {
                let start = out.len().saturating_sub(*offset);
                for i in 0..*length {
                    let ch = out[start + i];
                    out.push(ch);
                }
            }
        }
    }
    out.into_iter().collect()
}

fn action_lz(args: &Value) -> Result<String, String> {
    let op = args.get("op").and_then(|v| v.as_str()).unwrap_or("encode");
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("pass 'text' to compress")?;
    if text.is_empty() {
        return Err("'text' is empty".to_string());
    }
    let window = args
        .get("window")
        .and_then(|v| v.as_u64())
        .unwrap_or(32)
        .min(256) as usize;
    let lookahead = args
        .get("lookahead")
        .and_then(|v| v.as_u64())
        .unwrap_or(16)
        .min(64) as usize;

    let tokens = lz_encode(text, window, lookahead);
    let literal_count = tokens
        .iter()
        .filter(|t| matches!(t, LzToken::Literal(_)))
        .count();
    let backref_count = tokens
        .iter()
        .filter(|t| matches!(t, LzToken::BackRef { .. }))
        .count();
    let total_output_chars: usize = tokens
        .iter()
        .map(|t| match t {
            LzToken::Literal(_) => 1,
            LzToken::BackRef { length, .. } => *length,
        })
        .sum();

    // token stream encoded size estimate: literals = 1 byte, backrefs ≈ 3 bytes (offset+len)
    let encoded_bytes = literal_count + backref_count * 3;
    let ratio = encoded_bytes as f64 / text.len() as f64;
    let saving = if encoded_bytes < text.len() {
        format!("{:.1}% smaller", (1.0 - ratio) * 100.0)
    } else {
        format!(
            "{:.1}% larger (LZ not effective here)",
            (ratio - 1.0) * 100.0
        )
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "LZ77 Compress  (window={} chars, lookahead={})",
        window, lookahead
    ));
    lines.push(format!("  Input:      {} chars", text.len()));
    lines.push(format!(
        "  Tokens:     {} literals + {} back-references = {} total",
        literal_count,
        backref_count,
        tokens.len()
    ));
    lines.push(format!("  Est. bytes: {} → {}", text.len(), encoded_bytes));
    lines.push(format!("  Result:     {}", saving));
    lines.push(format!("  Ratio:      {:.3}", ratio));
    lines.push(String::new());

    // show up to 15 tokens
    lines.push(format!("Token stream (first {}):", "20".to_string()));
    for (i, tok) in tokens.iter().take(20).enumerate() {
        match tok {
            LzToken::Literal(c) => lines.push(format!("  {:>3}. LIT {:?}", i + 1, c)),
            LzToken::BackRef { offset, length } => {
                lines.push(format!("  {:>3}. REF (-{}, len={})", i + 1, offset, length));
            }
        }
    }
    if tokens.len() > 20 {
        lines.push(format!("  ... ({} more tokens)", tokens.len() - 20));
    }

    if op == "decode" || op == "verify" {
        let decoded = lz_decode(&tokens);
        lines.push(String::new());
        lines.push(format!("Verification: decoded {} chars", decoded.len()));
        if decoded == text {
            lines.push("  ✓ round-trip match".to_string());
        } else {
            lines.push("  ✗ round-trip mismatch (bug)".to_string());
        }
    }

    Ok(lines.join("\n"))
}

// ── Entropy / analyze ─────────────────────────────────────────────────────────

fn shannon_entropy(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let mut freq: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    for ch in text.chars() {
        *freq.entry(ch).or_insert(0) += 1;
    }
    let n = text.chars().count() as f64;
    freq.values()
        .map(|&c| {
            let p = c as f64 / n;
            -p * p.log2()
        })
        .sum()
}

fn action_analyze(args: &Value) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("pass 'text' to analyze")?;
    if text.is_empty() {
        return Err("'text' is empty".to_string());
    }
    let n = text.chars().count();
    let bytes = text.len();
    let entropy = shannon_entropy(text);
    let max_entropy = 8.0f64;
    let theoretical_min_bits = entropy * n as f64;
    let theoretical_min_bytes = (theoretical_min_bits / 8.0).ceil() as usize;
    let compressibility = 1.0 - entropy / max_entropy;

    // character frequency top 8
    let mut freq: Vec<(char, usize)> = {
        let mut map: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
        for ch in text.chars() {
            *map.entry(ch).or_insert(0) += 1;
        }
        map.into_iter().collect()
    };
    freq.sort_by(|a, b| b.1.cmp(&a.1));

    // run-length
    let rle = rle_encode(text);
    let rle_ratio = rle.len() as f64 / n as f64;

    let verdict = if entropy < 1.0 {
        "Highly compressible — very low entropy, many repeated characters"
    } else if entropy < 3.0 {
        "Good compressibility — low entropy, RLE or LZ should work well"
    } else if entropy < 5.0 {
        "Moderate compressibility — typical natural language text"
    } else if entropy < 7.0 {
        "Low compressibility — dense or pre-compressed data"
    } else {
        "Not compressible — near-random data (already compressed or encrypted)"
    };

    let mut lines = Vec::new();
    lines.push(format!("Compressibility Analysis"));
    lines.push(format!("  Characters:       {}", n));
    lines.push(format!("  Bytes (UTF-8):    {}", bytes));
    lines.push(format!("  Unique chars:     {}", freq.len()));
    lines.push(format!(
        "  Shannon entropy:  {:.4} bits/char  (max 8.0 for pure random)",
        entropy
    ));
    lines.push(format!(
        "  Compressibility:  {:.1}%  (0%=random, 100%=all same char)",
        compressibility * 100.0
    ));
    lines.push(format!(
        "  Theoretical min:  {} bits → {} bytes  (Huffman lower bound)",
        theoretical_min_bits as usize, theoretical_min_bytes
    ));
    lines.push(format!(
        "  RLE estimate:     {} chars ({:.1}x)",
        rle.len(),
        rle_ratio
    ));
    lines.push(String::new());
    lines.push(format!("Verdict: {}", verdict));
    lines.push(String::new());
    lines.push(format!("Top characters (by frequency):"));
    for (ch, count) in freq.iter().take(8) {
        let pct = *count as f64 / n as f64 * 100.0;
        let bar = "#".repeat((*count * 30 / freq[0].1).max(1));
        lines.push(format!("  {:?}  {:>5} ({:>5.1}%)  {}", ch, count, pct, bar));
    }
    if freq.len() > 8 {
        lines.push(format!("  ... ({} more unique chars)", freq.len() - 8));
    }

    Ok(lines.join("\n"))
}

// ── Huffman ───────────────────────────────────────────────────────────────────

fn action_huffman(args: &Value) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("pass 'text' for Huffman analysis")?;
    if text.is_empty() {
        return Err("'text' is empty".to_string());
    }
    let n = text.chars().count();

    // frequency table
    let mut freq: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    for ch in text.chars() {
        *freq.entry(ch).or_insert(0) += 1;
    }
    let mut symbols: Vec<(char, usize)> = freq.into_iter().collect();
    symbols.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    // compute Huffman code lengths via priority-queue simulation
    // Using a simple O(n²) approach for small alphabets
    let k = symbols.len();
    if k == 1 {
        let (ch, cnt) = symbols[0];
        let mut lines = Vec::new();
        lines.push("Huffman Coding".to_string());
        lines.push(format!(
            "  Only one unique character {:?} × {} — code length = 1 bit",
            ch, cnt
        ));
        lines.push(format!(
            "  Compressed size: {} bits = {} bytes (with 1 bit/char)",
            cnt,
            (cnt + 7) / 8
        ));
        return Ok(lines.join("\n"));
    }

    // build Huffman tree via symbol weights
    // represent each node as (weight, depth_sum, nodes)
    // simplified: derive code lengths from optimal Huffman tree using iterative merging
    let mut weights: Vec<f64> = symbols.iter().map(|(_, c)| *c as f64).collect();
    let mut lengths = vec![0u32; k];
    let mut node_weights = weights.clone();

    // iterative Huffman: merge two smallest, track depths
    let mut active: Vec<(f64, Vec<usize>)> = symbols
        .iter()
        .enumerate()
        .map(|(i, (_, c))| (*c as f64, vec![i]))
        .collect();

    while active.len() > 1 {
        // sort by weight
        active.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap()
                .then(a.1.len().cmp(&b.1.len()))
        });
        let (w1, leaves1) = active.remove(0);
        let (w2, leaves2) = active.remove(0);
        // increment depth of all leaves in both subtrees
        for idx in &leaves1 {
            lengths[*idx] += 1;
        }
        for idx in &leaves2 {
            lengths[*idx] += 1;
        }
        let combined_leaves: Vec<usize> = leaves1.into_iter().chain(leaves2).collect();
        active.push((w1 + w2, combined_leaves));
    }

    // compute compressed size
    let compressed_bits: usize = symbols
        .iter()
        .zip(lengths.iter())
        .map(|((_, cnt), len)| cnt * *len as usize)
        .sum();
    let original_bits = n * 8;
    let ratio = compressed_bits as f64 / original_bits as f64;
    let entropy = shannon_entropy(text);
    let _ = node_weights;
    let _ = weights;

    let mut lines = Vec::new();
    lines.push("Huffman Coding".to_string());
    lines.push(format!(
        "  Input:            {} chars, {} unique symbols",
        n, k
    ));
    lines.push(format!("  Entropy:          {:.4} bits/char", entropy));
    lines.push(format!(
        "  Original size:    {} bits ({} bytes, 8 bits/char)",
        original_bits,
        (original_bits + 7) / 8
    ));
    lines.push(format!(
        "  Huffman size:     {} bits ({} bytes)",
        compressed_bits,
        (compressed_bits + 7) / 8
    ));
    lines.push(format!(
        "  Compression:      {:.1}%  ({:.3}x)",
        (1.0 - ratio) * 100.0,
        ratio
    ));
    lines.push(String::new());
    lines.push(format!(
        "{:<6}  {:>8}  {:>7}  {:>8}  {}",
        "Symbol", "Count", "Prob%", "CodeLen", "Contribution"
    ));
    lines.push("-".repeat(52));
    for ((ch, cnt), &len) in symbols.iter().zip(lengths.iter()) {
        let prob = *cnt as f64 / n as f64 * 100.0;
        let contrib = cnt * len as usize;
        lines.push(format!(
            "{:<6}  {:>8}  {:>6.2}%  {:>8}  {} bits",
            format!("{:?}", ch),
            cnt,
            prob,
            len,
            contrib
        ));
    }
    lines.push(String::new());
    lines.push("Note: actual Huffman tree also requires transmitting the code table.".to_string());

    Ok(lines.join("\n"))
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("analyze");
    match action {
        "rle" => action_rle(args),
        "lz" => action_lz(args),
        "analyze" => action_analyze(args),
        "huffman" => action_huffman(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: rle, lz, analyze, huffman",
            action
        )),
    }
}
