use serde_json::{json, Value};

pub fn number_sequence_tools_schema() -> Value {
    json!({
        "name": "number_sequence_tools",
        "description": "Analyze and extend numeric sequences without external utilities. Actions: detect (identify pattern type — arithmetic, geometric, Fibonacci-like, polynomial, prime, square, cube, triangular, power-of-2, constant, or custom recurrence), continue (extend the sequence by N more terms), diff (show difference table to reveal polynomial patterns), stats (min/max/mean/sum/growth rate of the sequence). Input is a list of numbers.",
        "parameters": {
            "type": "object",
            "properties": {
                "numbers": {
                    "type": "array",
                    "description": "The numeric sequence as an array of numbers",
                    "items": { "type": "number" }
                },
                "data": {
                    "type": "string",
                    "description": "Alternative: comma or space-separated numbers as a string"
                },
                "action": {
                    "type": "string",
                    "enum": ["detect", "continue", "diff", "stats"],
                    "description": "Action to perform (default: detect)"
                },
                "n": {
                    "type": "integer",
                    "description": "Number of additional terms to generate for 'continue' action (default: 5, max: 50)"
                }
            },
            "required": []
        }
    })
}

// ── number parsing ─────────────────────────────────────────────────────────────

fn parse_numbers(args: &Value) -> Result<Vec<f64>, String> {
    if let Some(arr) = args.get("numbers").and_then(|v| v.as_array()) {
        let nums: Result<Vec<f64>, _> = arr
            .iter()
            .map(|v| {
                v.as_f64()
                    .ok_or_else(|| "Non-numeric value in array".to_string())
            })
            .collect();
        return nums;
    }
    if let Some(s) = args.get("data").and_then(|v| v.as_str()) {
        let nums: Result<Vec<f64>, _> = s
            .split([',', ' ', '\n'])
            .filter(|t| !t.trim().is_empty())
            .map(|t| {
                t.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("Cannot parse '{}' as number", t.trim()))
            })
            .collect();
        return nums;
    }
    Err("Provide 'numbers' array or 'data' string".to_string())
}

// ── pattern detection ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Pattern {
    Constant,
    Arithmetic { d: f64 },
    Geometric { r: f64 },
    FibonacciLike { a: f64, b: f64 },
    Squares,
    Cubes,
    Triangular,
    PowerOf2,
    Primes,
    Polynomial { degree: usize, diffs: Vec<f64> },
    Unknown,
}

fn approx_eq(a: f64, b: f64) -> bool {
    if a == 0.0 && b == 0.0 {
        return true;
    }
    let mag = a.abs().max(b.abs());
    (a - b).abs() / mag.max(1.0) < 1e-9
}

fn differences(seq: &[f64]) -> Vec<f64> {
    seq.windows(2).map(|w| w[1] - w[0]).collect()
}

fn is_constant(v: &[f64]) -> bool {
    v.windows(2).all(|w| approx_eq(w[0], w[1]))
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    let mut i = 3u64;
    while i * i <= n {
        if n % i == 0 {
            return false;
        }
        i += 2;
    }
    true
}

fn nth_prime(n: usize) -> u64 {
    let mut count = 0usize;
    let mut num = 1u64;
    while count < n {
        num += 1;
        if is_prime(num) {
            count += 1;
        }
    }
    num
}

fn detect_pattern(seq: &[f64]) -> Pattern {
    if seq.len() < 2 {
        return Pattern::Unknown;
    }

    // Constant
    if is_constant(seq) {
        return Pattern::Constant;
    }

    // Arithmetic
    let d1 = differences(seq);
    if is_constant(&d1) {
        return Pattern::Arithmetic { d: d1[0] };
    }

    // Geometric (all nonzero)
    if seq.iter().all(|&x| x != 0.0) {
        let ratios: Vec<f64> = seq.windows(2).map(|w| w[1] / w[0]).collect();
        if is_constant(&ratios) {
            return Pattern::Geometric { r: ratios[0] };
        }
    }

    // Fibonacci-like: each term = sum of previous two
    if seq.len() >= 3 {
        let fib_ok = seq.windows(3).all(|w| approx_eq(w[2], w[0] + w[1]));
        if fib_ok {
            return Pattern::FibonacciLike {
                a: seq[0],
                b: seq[1],
            };
        }
    }

    // Squares: n^2
    let as_squares = seq
        .iter()
        .enumerate()
        .all(|(i, &x)| approx_eq(x, ((i + 1) as f64).powi(2)));
    if as_squares {
        return Pattern::Squares;
    }

    // Cubes: n^3
    let as_cubes = seq
        .iter()
        .enumerate()
        .all(|(i, &x)| approx_eq(x, ((i + 1) as f64).powi(3)));
    if as_cubes {
        return Pattern::Cubes;
    }

    // Triangular: n*(n+1)/2
    let as_tri = seq.iter().enumerate().all(|(i, &x)| {
        let n = (i + 1) as f64;
        approx_eq(x, n * (n + 1.0) / 2.0)
    });
    if as_tri {
        return Pattern::Triangular;
    }

    // Powers of 2
    let as_pow2 = seq
        .iter()
        .enumerate()
        .all(|(i, &x)| approx_eq(x, 2f64.powi(i as i32)));
    if as_pow2 {
        return Pattern::PowerOf2;
    }

    // Prime numbers
    if seq.len() >= 3 && seq.iter().all(|&x| x > 0.0 && x == x.floor()) {
        let primes_ok = seq.iter().enumerate().all(|(i, &x)| {
            let p = nth_prime(i + 1) as f64;
            approx_eq(x, p)
        });
        if primes_ok {
            return Pattern::Primes;
        }
    }

    // Polynomial via finite differences (up to degree 4)
    let mut diffs_stack = vec![d1.clone()];
    for _ in 0..3 {
        let last = diffs_stack.last().unwrap();
        if last.len() < 2 {
            break;
        }
        let next = differences(last);
        if is_constant(&next) {
            let degree = diffs_stack.len() + 1;
            let mut all_diffs: Vec<f64> = vec![seq[0]];
            for ds in &diffs_stack {
                all_diffs.push(ds[0]);
            }
            all_diffs.push(next[0]);
            return Pattern::Polynomial {
                degree,
                diffs: all_diffs,
            };
        }
        diffs_stack.push(next);
    }

    Pattern::Unknown
}

// ── next term generation ──────────────────────────────────────────────────────

fn next_terms(seq: &[f64], pattern: &Pattern, n: usize) -> Vec<f64> {
    let mut result = Vec::new();
    let mut s = seq.to_vec();

    for _ in 0..n {
        let next = match pattern {
            Pattern::Constant => s[0],
            Pattern::Arithmetic { d } => s.last().unwrap() + d,
            Pattern::Geometric { r } => s.last().unwrap() * r,
            Pattern::FibonacciLike { .. } => {
                let len = s.len();
                s[len - 1] + s[len - 2]
            }
            Pattern::Squares => {
                let i = (s.len() + 1) as f64;
                i * i
            }
            Pattern::Cubes => {
                let i = (s.len() + 1) as f64;
                i * i * i
            }
            Pattern::Triangular => {
                let n = (s.len() + 1) as f64;
                n * (n + 1.0) / 2.0
            }
            Pattern::PowerOf2 => {
                let exp = s.len() as i32;
                2f64.powi(exp)
            }
            Pattern::Primes => nth_prime(s.len() + 1) as f64,
            Pattern::Polynomial { diffs, degree: _ } => {
                // Extend using Newton's forward difference formula
                // Apply forward differences: add the constant difference at each level
                let _len = s.len();
                let ext = s.clone();
                // We need the difference table for current ext
                let mut levels: Vec<Vec<f64>> = vec![ext.clone()];
                let mut cur = ext.clone();
                for _ in 0..diffs.len().saturating_sub(1) {
                    cur = differences(&cur);
                    levels.push(cur.clone());
                }
                // The last level should be constant — use it to propagate back up
                let last_val = *levels.last().unwrap().last().unwrap();
                let mut carry = vec![last_val];
                for level in levels.iter().rev().skip(1) {
                    let last_at_level = level.last().unwrap();
                    carry.push(last_at_level + carry.last().unwrap());
                }
                *carry.last().unwrap()
            }
            Pattern::Unknown => {
                // Last-difference extrapolation
                if s.len() >= 2 {
                    let d = s[s.len() - 1] - s[s.len() - 2];
                    s.last().unwrap() + d
                } else {
                    *s.last().unwrap()
                }
            }
        };
        s.push(next);
        result.push(next);
    }
    result
}

fn fmt_num(x: f64) -> String {
    if x == x.floor() && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        // Try to show a reasonable number of decimals
        let s = format!("{:.6}", x);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_detect(seq: &[f64]) -> String {
    if seq.len() < 2 {
        return "Need at least 2 numbers to detect a pattern.\n".to_string();
    }

    let pat = detect_pattern(seq);
    let seq_str: Vec<String> = seq.iter().map(|x| fmt_num(*x)).collect();
    let mut out = format!("Sequence:  {}\n", seq_str.join(", "));
    out.push_str(&"─".repeat(50));
    out.push('\n');

    match &pat {
        Pattern::Constant => {
            out.push_str("Pattern:   Constant\n");
            out.push_str(&format!("Value:     {}\n", fmt_num(seq[0])));
        }
        Pattern::Arithmetic { d } => {
            out.push_str("Pattern:   Arithmetic (linear)\n");
            out.push_str(&format!("Common difference (d):  {}\n", fmt_num(*d)));
            out.push_str(&format!(
                "Formula:   a(n) = {} + (n-1) × {}\n",
                fmt_num(seq[0]),
                fmt_num(*d)
            ));
        }
        Pattern::Geometric { r } => {
            out.push_str("Pattern:   Geometric\n");
            out.push_str(&format!("Common ratio (r):  {}\n", fmt_num(*r)));
            out.push_str(&format!(
                "Formula:   a(n) = {} × {}^(n-1)\n",
                fmt_num(seq[0]),
                fmt_num(*r)
            ));
        }
        Pattern::FibonacciLike { a, b } => {
            out.push_str("Pattern:   Fibonacci-like (each term = sum of previous two)\n");
            out.push_str(&format!("a(1) = {}  a(2) = {}\n", fmt_num(*a), fmt_num(*b)));
            out.push_str("Rule:      a(n) = a(n-1) + a(n-2)\n");
        }
        Pattern::Squares => {
            out.push_str("Pattern:   Perfect squares\n");
            out.push_str("Formula:   a(n) = n²\n");
        }
        Pattern::Cubes => {
            out.push_str("Pattern:   Perfect cubes\n");
            out.push_str("Formula:   a(n) = n³\n");
        }
        Pattern::Triangular => {
            out.push_str("Pattern:   Triangular numbers\n");
            out.push_str("Formula:   a(n) = n(n+1)/2\n");
        }
        Pattern::PowerOf2 => {
            out.push_str("Pattern:   Powers of 2\n");
            out.push_str("Formula:   a(n) = 2^(n-1)  (starting index 0: 2^n)\n");
        }
        Pattern::Primes => {
            out.push_str("Pattern:   Prime numbers\n");
            out.push_str("Rule:      a(n) = the n-th prime\n");
        }
        Pattern::Polynomial { degree, diffs } => {
            out.push_str(&format!("Pattern:   Polynomial (degree {})\n", degree));
            out.push_str(
                "Method:    Detected via finite differences (Newton's forward difference table)\n",
            );
            let diff_strs: Vec<String> = diffs.iter().map(|x| fmt_num(*x)).collect();
            out.push_str(&format!("Leading differences:  {}\n", diff_strs.join(", ")));
        }
        Pattern::Unknown => {
            out.push_str("Pattern:   Unknown / not recognized\n");
            out.push_str(
                "Tip:       Use action:'diff' to inspect the difference table manually.\n",
            );
        }
    }

    // Show next 5 terms
    let next = next_terms(seq, &pat, 5);
    let next_strs: Vec<String> = next.iter().map(|x| fmt_num(*x)).collect();
    out.push_str(&format!("\nNext 5 terms:  {}\n", next_strs.join(", ")));
    out
}

fn action_continue(seq: &[f64], n: usize) -> String {
    if seq.len() < 2 {
        return "Need at least 2 numbers to continue a sequence.\n".to_string();
    }

    let pat = detect_pattern(seq);
    let next = next_terms(seq, &pat, n);

    let orig_strs: Vec<String> = seq.iter().map(|x| fmt_num(*x)).collect();
    let next_strs: Vec<String> = next.iter().map(|x| fmt_num(*x)).collect();

    let mut out = format!("Original:  {}\n", orig_strs.join(", "));
    out.push_str(&format!("Next {}:    {}\n", n, next_strs.join(", ")));
    out.push_str(&format!(
        "Full:      {}, {}\n",
        orig_strs.join(", "),
        next_strs.join(", ")
    ));
    out
}

fn action_diff(seq: &[f64]) -> String {
    if seq.len() < 2 {
        return "Need at least 2 numbers for difference table.\n".to_string();
    }

    let orig_strs: Vec<String> = seq.iter().map(|x| fmt_num(*x)).collect();
    let mut out = format!("Sequence:  {}\n", orig_strs.join(", "));
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str("Difference table (each row = differences of the row above):\n\n");

    let mut current = seq.to_vec();
    let mut level = 0usize;
    loop {
        let prefix = "  ".repeat(level);
        let strs: Vec<String> = current.iter().map(|x| fmt_num(*x)).collect();
        out.push_str(&format!("{}Δ{}: {}\n", prefix, level, strs.join(", ")));

        if current.len() < 2 {
            break;
        }
        let next = differences(&current);
        if is_constant(&next) || next.len() < 2 {
            let prefix2 = "  ".repeat(level + 1);
            let strs2: Vec<String> = next.iter().map(|x| fmt_num(*x)).collect();
            out.push_str(&format!(
                "{}Δ{}: {}  ← constant (polynomial degree {} detected)\n",
                prefix2,
                level + 1,
                strs2.join(", "),
                level + 1
            ));
            break;
        }
        if level >= 8 {
            out.push_str("  (stopping at depth 8 — pattern may not be polynomial)\n");
            break;
        }
        current = next;
        level += 1;
    }
    out
}

fn action_stats(seq: &[f64]) -> String {
    if seq.is_empty() {
        return "No numbers provided.\n".to_string();
    }

    let n = seq.len();
    let min = seq.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = seq.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = seq.iter().sum();
    let mean = sum / n as f64;

    let variance = seq.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = variance.sqrt();

    let orig_strs: Vec<String> = seq.iter().map(|x| fmt_num(*x)).collect();
    let mut out = format!("Sequence:  {}\n", orig_strs.join(", "));
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!("Count:     {}\n", n));
    out.push_str(&format!("Min:       {}\n", fmt_num(min)));
    out.push_str(&format!("Max:       {}\n", fmt_num(max)));
    out.push_str(&format!("Sum:       {}\n", fmt_num(sum)));
    out.push_str(&format!("Mean:      {}\n", fmt_num(mean)));
    out.push_str(&format!("Std dev:   {}\n", fmt_num(std_dev)));

    if n >= 2 {
        let total_change = seq.last().unwrap() - seq.first().unwrap();
        out.push_str(&format!("Net change: {}\n", fmt_num(total_change)));

        if *seq.first().unwrap() != 0.0 {
            let pct = (total_change / seq.first().unwrap()) * 100.0;
            out.push_str(&format!("Total growth: {:.2}%\n", pct));
        }

        // Average step
        let avg_step = total_change / (n - 1) as f64;
        out.push_str(&format!("Avg step:  {}\n", fmt_num(avg_step)));
    }

    // Pattern hint
    let pat = detect_pattern(seq);
    let pat_name = match &pat {
        Pattern::Constant => "Constant",
        Pattern::Arithmetic { .. } => "Arithmetic (linear)",
        Pattern::Geometric { .. } => "Geometric",
        Pattern::FibonacciLike { .. } => "Fibonacci-like",
        Pattern::Squares => "Perfect squares",
        Pattern::Cubes => "Perfect cubes",
        Pattern::Triangular => "Triangular",
        Pattern::PowerOf2 => "Powers of 2",
        Pattern::Primes => "Prime numbers",
        Pattern::Polynomial { degree, .. } => {
            return format!("{out}Pattern hint: Polynomial degree {degree}\n");
        }
        Pattern::Unknown => "Unknown",
    };
    out.push_str(&format!("Pattern hint: {}\n", pat_name));
    out
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let seq = parse_numbers(args)?;

    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("detect");
    let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(5).min(50) as usize;

    match action {
        "detect" => Ok(action_detect(&seq)),
        "continue" => Ok(action_continue(&seq, n)),
        "diff" => Ok(action_diff(&seq)),
        "stats" => Ok(action_stats(&seq)),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: detect, continue, diff, stats"
        )),
    }
}
