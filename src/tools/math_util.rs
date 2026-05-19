// ─── Pure-Rust math utilities ─────────────────────────────────────────────────
// Number theory, sequences, combinatorics — no Python sandbox, instant results.

use std::fmt::Write;

// ── Primality and factorization ───────────────────────────────────────────────

fn is_prime(n: u64) -> bool {
    if n < 2 { return false; }
    if n < 4 { return true; }
    if n % 2 == 0 || n % 3 == 0 { return false; }
    let mut i = 5u64;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 { return false; }
        i += 6;
    }
    true
}

fn factorize(mut n: u64) -> Vec<(u64, u32)> {
    let mut factors: Vec<(u64, u32)> = Vec::new();
    if n < 2 { return factors; }
    for p in [2u64, 3] {
        if n % p == 0 {
            let mut exp = 0u32;
            while n % p == 0 { n /= p; exp += 1; }
            factors.push((p, exp));
        }
    }
    let mut i = 5u64;
    while i * i <= n {
        if n % i == 0 {
            let mut exp = 0u32;
            while n % i == 0 { n /= i; exp += 1; }
            factors.push((i, exp));
        }
        if n % (i + 2) == 0 {
            let mut exp = 0u32;
            while n % (i + 2) == 0 { n /= i + 2; exp += 1; }
            factors.push((i + 2, exp));
        }
        i += 6;
    }
    if n > 1 { factors.push((n, 1)); }
    factors
}

fn next_prime(n: u64) -> u64 {
    let mut c = n + 1;
    while !is_prime(c) { c += 1; }
    c
}

fn prev_prime(n: u64) -> Option<u64> {
    if n <= 2 { return None; }
    let mut c = n - 1;
    while c >= 2 {
        if is_prime(c) { return Some(c); }
        if c == 2 { break; }
        c -= 1;
    }
    None
}

pub fn prime_info(n: u64) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Number: {}", n);
    let _ = writeln!(out, "Prime:  {}", if is_prime(n) { "Yes" } else { "No" });

    let factors = factorize(n);
    if factors.is_empty() {
        let _ = writeln!(out, "Factors: 1 (or 0)");
    } else {
        let expr: Vec<String> = factors.iter().map(|(p, e)| {
            if *e == 1 { format!("{}", p) } else { format!("{}^{}", p, e) }
        }).collect();
        let _ = writeln!(out, "Factors: {}", expr.join(" × "));

        // divisors from factors
        let mut divisors = vec![1u64];
        for (p, e) in &factors {
            let len = divisors.len();
            let mut pw = 1u64;
            for _ in 0..*e {
                pw *= p;
                for i in 0..len { divisors.push(divisors[i] * pw); }
            }
        }
        divisors.sort_unstable();
        let _ = writeln!(out, "Divisors ({} total): {}", divisors.len(),
            if divisors.len() <= 24 {
                divisors.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", ")
            } else {
                format!("{} ... {} [first 12 + last 12]",
                    divisors[..12].iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", "),
                    divisors[divisors.len()-12..].iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", "))
            }
        );
        // Euler's totient φ(n)
        let phi = factors.iter().fold(n, |acc, (p, _)| acc / p * (p - 1));
        let _ = writeln!(out, "φ(n):   {}", phi);
        // sum of divisors σ(n)
        let sigma: u64 = factors.iter().map(|(p, e)| {
            (p.pow(e + 1) - 1) / (p - 1)
        }).product();
        let _ = writeln!(out, "σ(n):   {}", sigma);
        // perfect number check
        if sigma == 2 * n { let _ = writeln!(out, "✓ Perfect number"); }
    }

    if let Some(pp) = prev_prime(n) { let _ = writeln!(out, "Prev prime: {}", pp); }
    else { let _ = writeln!(out, "Prev prime: (none)"); }
    let np = next_prime(n);
    let _ = writeln!(out, "Next prime: {}", np);
    out
}

// ── Sequences ─────────────────────────────────────────────────────────────────

pub fn generate_sequence(kind: &str, count: usize, start: f64, step: f64) -> String {
    let count = count.max(1).min(10_000);
    let kind = kind.trim().to_lowercase();
    let mut out = String::new();

    let nums: Vec<f64> = match kind.as_str() {
        "arithmetic" | "arith" | "linear" => {
            (0..count).map(|i| start + i as f64 * step).collect()
        }
        "geometric" | "geo" | "geom" => {
            let ratio = if step == 0.0 { 2.0 } else { step };
            let mut v = start;
            (0..count).map(|_| { let x = v; v *= ratio; x }).collect()
        }
        "fibonacci" | "fib" => {
            let (mut a, mut b) = (start as u64, (start + step) as u64);
            let mut seq = vec![a as f64, b as f64];
            for _ in 2..count {
                let c = a.saturating_add(b);
                seq.push(c as f64);
                a = b; b = c;
            }
            seq.truncate(count);
            seq
        }
        "prime" | "primes" => {
            let mut seq = Vec::with_capacity(count);
            let mut n: u64 = start.max(2.0) as u64;
            if !is_prime(n) { n = next_prime(n - 1); }
            while seq.len() < count {
                seq.push(n as f64);
                n = next_prime(n);
            }
            seq
        }
        "square" | "squares" => {
            let s = start.max(0.0) as u64;
            (s..s + count as u64).map(|i| (i * i) as f64).collect()
        }
        "triangular" | "triangle" => {
            let s = start.max(0.0) as u64;
            (s..s + count as u64).map(|i| (i * (i + 1) / 2) as f64).collect()
        }
        "cube" | "cubes" => {
            let s = start.max(0.0) as u64;
            (s..s + count as u64).map(|i| (i * i * i) as f64).collect()
        }
        "power2" | "powers-of-2" | "powers_of_2" => {
            (0..count).map(|i| (1u64 << i.min(62)) as f64).collect()
        }
        _ => {
            return format!(
                "Unknown sequence type: '{}'\n\
                 Available: arithmetic  geometric  fibonacci  prime  square  triangular  cube  power2\n\
                 Defaults: --seq-start 1  --seq-step 1  --seq-count 10",
                kind
            );
        }
    };

    let label = match kind.as_str() {
        "arithmetic" | "arith" | "linear" =>
            format!("Arithmetic (start={}, step={})", start, step),
        "geometric" | "geo" | "geom" =>
            format!("Geometric (start={}, ratio={})", start, if step == 0.0 { 2.0 } else { step }),
        _ => kind[..1].to_uppercase() + &kind[1..],
    };
    let _ = writeln!(out, "{}: {} terms", label, nums.len());
    let strs: Vec<String> = nums.iter().map(|x| {
        if x.fract() == 0.0 && x.abs() < 1e15 { format!("{}", *x as i64) }
        else {
            let s = format!("{:.6e}", x);
            // trim trailing zeros in mantissa
            s
        }
    }).collect();
    // Wrap at 72 chars
    let mut line = String::new();
    for (i, s) in strs.iter().enumerate() {
        let piece = if i == 0 { s.clone() } else { format!(", {}", s) };
        if line.len() + piece.len() > 72 { let _ = writeln!(out, "{}", line); line = s.clone(); }
        else { line.push_str(&piece); }
    }
    if !line.is_empty() { let _ = writeln!(out, "{}", line); }
    out
}

// ── Combinatorics ─────────────────────────────────────────────────────────────

pub fn combinatorics(n: u64, k: u64) -> String {
    let mut out = String::new();

    // C(n,k) using multiplicative formula (avoid overflow for reasonable n)
    let binom = if k > n {
        0u128
    } else {
        let k = k.min(n - k);
        (0..k).fold(1u128, |acc, i| acc * (n - i) as u128 / (i + 1) as u128)
    };

    // P(n,k) = n! / (n-k)!
    let perm: u128 = if k > n { 0 } else {
        (n - k + 1..=n).fold(1u128, |acc, i| acc.saturating_mul(i as u128))
    };

    let _ = writeln!(out, "n = {}  k = {}", n, k);
    let _ = writeln!(out, "C(n,k) = n! / (k!(n-k)!) = {}  (combinations — order does not matter)", binom);
    let _ = writeln!(out, "P(n,k) = n! / (n-k)!     = {}  (permutations — order matters)", perm);

    // Pascal's triangle row
    if n <= 20 {
        let row: Vec<u128> = (0..=n).map(|j| {
            let j = j.min(n - j);
            (0..j).fold(1u128, |acc, i| acc * (n - i) as u128 / (i + 1) as u128)
        }).collect();
        let _ = writeln!(out, "Pascal row {}: {}", n, row.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "));
    }
    out
}

// ── Boolean truth table ───────────────────────────────────────────────────────
// Pure-Rust: parses and evaluates a Boolean expression over single-letter variables.

pub fn truth_table(expr: &str) -> String {
    // Collect variables (single uppercase or lowercase letters A-Z)
    let mut vars: Vec<char> = expr.chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect::<std::collections::HashSet<char>>()
        .into_iter()
        .collect();
    vars.sort_unstable();

    if vars.is_empty() {
        return format!("No variables found in: {}\nUse single letters (A, B, C, ...) as variables.", expr);
    }
    if vars.len() > 6 {
        return format!("Too many variables ({}). Limit: 6 (for 2^6 = 64 rows).", vars.len());
    }

    let n_vars = vars.len();
    let n_rows = 1usize << n_vars;
    let mut out = String::new();
    let expr_display = expr.trim()
        .replace("AND", "∧").replace("OR", "∨").replace("NOT", "¬")
        .replace("XOR", "⊕").replace("NAND", "⊼").replace("NOR", "⊽");

    // Header
    for v in &vars { let _ = write!(out, " {}  ", v); }
    let _ = writeln!(out, "| {}", expr_display);
    let sep: String = vars.iter().map(|_| "----").collect::<String>() + "+--" + &"-".repeat(expr_display.len());
    let _ = writeln!(out, "{}", sep);

    let mut true_rows = 0usize;
    for row in 0..n_rows {
        let vals: Vec<bool> = (0..n_vars).map(|i| (row >> (n_vars - 1 - i)) & 1 == 1).collect();
        for &v in &vals { let _ = write!(out, " {}  ", if v { 'T' } else { 'F' }); }
        let result = eval_bool(expr, &vars, &vals);
        match result {
            Ok(r) => {
                if r { true_rows += 1; }
                let _ = writeln!(out, "| {}", if r { 'T' } else { 'F' });
            }
            Err(e) => { let _ = writeln!(out, "| Error: {}", e); }
        }
    }
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "True rows: {} / {}  ({}%)",
        true_rows, n_rows, 100 * true_rows / n_rows);
    if true_rows == 0       { let _ = writeln!(out, "Classification: Contradiction (always false)"); }
    else if true_rows == n_rows { let _ = writeln!(out, "Classification: Tautology (always true)"); }
    else                    { let _ = writeln!(out, "Classification: Contingency"); }
    out
}

fn eval_bool(expr: &str, vars: &[char], vals: &[bool]) -> Result<bool, String> {
    let tokens = tokenize_bool(expr)?;
    let (result, _) = parse_bool_or(&tokens, 0, vars, vals)?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq)]
enum BoolToken { Var(char), True, False, Not, And, Or, Xor, Nand, Nor, LParen, RParen }

fn tokenize_bool(s: &str) -> Result<Vec<BoolToken>, String> {
    let mut tokens = Vec::new();
    let s = s.replace("NAND", "⊼").replace("NOR", "⊽").replace("XOR", "⊕")
             .replace("AND", "∧").replace("OR", "∨").replace("NOT", "¬")
             .replace("&&", "∧").replace("||", "∨").replace("!", "¬")
             .replace('&', "∧").replace('|', "∨").replace('^', "⊕");
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\n' => {}
            '(' => tokens.push(BoolToken::LParen),
            ')' => tokens.push(BoolToken::RParen),
            '¬' => tokens.push(BoolToken::Not),
            '∧' => tokens.push(BoolToken::And),
            '∨' => tokens.push(BoolToken::Or),
            '⊕' => tokens.push(BoolToken::Xor),
            '⊼' => tokens.push(BoolToken::Nand),
            '⊽' => tokens.push(BoolToken::Nor),
            '1' | 'T' => tokens.push(BoolToken::True),
            '0' | 'F' => tokens.push(BoolToken::False),
            c if c.is_ascii_alphabetic() => tokens.push(BoolToken::Var(c)),
            other => return Err(format!("Unknown token: '{}'", other)),
        }
    }
    Ok(tokens)
}

fn parse_bool_or(tokens: &[BoolToken], pos: usize, vars: &[char], vals: &[bool]) -> Result<(bool, usize), String> {
    let (mut lhs, mut pos) = parse_bool_and(tokens, pos, vars, vals)?;
    while pos < tokens.len() {
        match &tokens[pos] {
            BoolToken::Or  => { let (rhs, np) = parse_bool_and(tokens, pos+1, vars, vals)?; lhs = lhs || rhs; pos = np; }
            BoolToken::Nor => { let (rhs, np) = parse_bool_and(tokens, pos+1, vars, vals)?; lhs = !(lhs || rhs); pos = np; }
            _ => break,
        }
    }
    Ok((lhs, pos))
}

fn parse_bool_and(tokens: &[BoolToken], pos: usize, vars: &[char], vals: &[bool]) -> Result<(bool, usize), String> {
    let (mut lhs, mut pos) = parse_bool_xor(tokens, pos, vars, vals)?;
    while pos < tokens.len() {
        match &tokens[pos] {
            BoolToken::And  => { let (rhs, np) = parse_bool_xor(tokens, pos+1, vars, vals)?; lhs = lhs && rhs; pos = np; }
            BoolToken::Nand => { let (rhs, np) = parse_bool_xor(tokens, pos+1, vars, vals)?; lhs = !(lhs && rhs); pos = np; }
            _ => break,
        }
    }
    Ok((lhs, pos))
}

fn parse_bool_xor(tokens: &[BoolToken], pos: usize, vars: &[char], vals: &[bool]) -> Result<(bool, usize), String> {
    let (mut lhs, mut pos) = parse_bool_not(tokens, pos, vars, vals)?;
    while pos < tokens.len() && tokens[pos] == BoolToken::Xor {
        let (rhs, np) = parse_bool_not(tokens, pos+1, vars, vals)?;
        lhs = lhs ^ rhs;
        pos = np;
    }
    Ok((lhs, pos))
}

fn parse_bool_not(tokens: &[BoolToken], pos: usize, vars: &[char], vals: &[bool]) -> Result<(bool, usize), String> {
    if pos < tokens.len() && tokens[pos] == BoolToken::Not {
        let (inner, np) = parse_bool_not(tokens, pos+1, vars, vals)?;
        return Ok((!inner, np));
    }
    parse_bool_atom(tokens, pos, vars, vals)
}

fn parse_bool_atom(tokens: &[BoolToken], pos: usize, vars: &[char], vals: &[bool]) -> Result<(bool, usize), String> {
    if pos >= tokens.len() { return Err("Unexpected end of expression".into()); }
    match &tokens[pos] {
        BoolToken::LParen => {
            let (inner, np) = parse_bool_or(tokens, pos+1, vars, vals)?;
            if np >= tokens.len() || tokens[np] != BoolToken::RParen {
                return Err("Missing closing ')'".into());
            }
            Ok((inner, np + 1))
        }
        BoolToken::Var(c) => {
            let idx = vars.iter().position(|v| v == c)
                .ok_or_else(|| format!("Unknown variable '{}'", c))?;
            Ok((vals[idx], pos + 1))
        }
        BoolToken::True  => Ok((true,  pos + 1)),
        BoolToken::False => Ok((false, pos + 1)),
        other => Err(format!("Unexpected token: {:?}", other)),
    }
}
