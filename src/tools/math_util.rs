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

// ── GCD / LCM ─────────────────────────────────────────────────────────────────

fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 { a } else { gcd(b, a % b) }
}

pub fn gcd_lcm(a: u128, b: u128) -> String {
    let g = gcd(a, b);
    let l = if g == 0 { 0 } else { a / g * b };
    format!("GCD({a}, {b}) = {g}\nLCM({a}, {b}) = {l}")
}

// ── Roman numerals ────────────────────────────────────────────────────────────

pub fn to_roman(mut n: u64) -> String {
    if n == 0 { return "Roman numerals start at 1.".into(); }
    if n > 3_999_999 { return format!("{n} is too large for standard Roman numerals (max 3,999,999)."); }
    const TABLE: &[(u64, &str)] = &[
        (1_000_000,"M̄"),(900_000,"C̄M̄"),(500_000,"D̄"),(400_000,"C̄D̄"),
        (100_000,"C̄"),(90_000,"X̄C̄"),(50_000,"L̄"),(40_000,"X̄L̄"),
        (10_000,"X̄"),(9_000,"MX̄"),(5_000,"V̄"),(4_000,"MV̄"),
        (1000,"M"),(900,"CM"),(500,"D"),(400,"CD"),(100,"C"),(90,"XC"),
        (50,"L"),(40,"XL"),(10,"X"),(9,"IX"),(5,"V"),(4,"IV"),(1,"I"),
    ];
    let mut out = String::new();
    for &(val, sym) in TABLE {
        while n >= val { out.push_str(sym); n -= val; }
    }
    out
}

pub fn from_roman(s: &str) -> String {
    let s = s.trim().to_uppercase();
    let map = [("M̄",1_000_000u64),("C̄M̄",900_000),("D̄",500_000),("C̄D̄",400_000),
               ("C̄",100_000),("X̄C̄",90_000),("L̄",50_000),("X̄L̄",40_000),
               ("X̄",10_000),("MX̄",9_000),("V̄",5_000),("MV̄",4_000),
               ("M",1000),("CM",900),("D",500),("CD",400),("C",100),("XC",90),
               ("L",50),("XL",40),("X",10),("IX",9),("V",5),("IV",4),("I",1)];
    let mut pos = 0usize; let chars: Vec<char> = s.chars().collect();
    let mut total = 0u64;
    'outer: while pos < chars.len() {
        for &(sym, val) in &map {
            let sc: Vec<char> = sym.chars().collect();
            if chars[pos..].starts_with(&sc) {
                total += val;
                pos += sc.len();
                continue 'outer;
            }
        }
        return format!("Unrecognized Roman numeral character at position {}: '{}'", pos, chars[pos]);
    }
    format!("{} = {}", s, total)
}

pub fn roman_info(input: &str) -> String {
    let t = input.trim();
    if let Ok(n) = t.parse::<u64>() {
        let r = to_roman(n);
        format!("{n} = {r}")
    } else {
        from_roman(t)
    }
}

// ── Number base conversion ────────────────────────────────────────────────────

pub fn base_convert(input: &str, from_base: u32, to_base: u32) -> String {
    if from_base < 2 || from_base > 36 || to_base < 2 || to_base > 36 {
        return "Base must be between 2 and 36.".into();
    }
    let s = input.trim().to_ascii_uppercase();
    let value = u128::from_str_radix(&s, from_base)
        .unwrap_or_else(|_| return 0);
    // check parse success separately
    if u128::from_str_radix(&s, from_base).is_err() {
        return format!("'{}' is not a valid base-{} number.", s, from_base);
    }
    let to_str = |mut v: u128, base: u32| -> String {
        if v == 0 { return "0".into(); }
        let digits: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut result = Vec::new();
        while v > 0 { result.push(digits[(v % base as u128) as usize] as char); v /= base as u128; }
        result.into_iter().rev().collect()
    };
    let _ = value; // used via closure
    let value2 = u128::from_str_radix(&s, from_base).unwrap();
    let mut out = String::new();
    let _ = writeln!(out, "Input (base {}): {}", from_base, s);
    let _ = writeln!(out, "Decimal: {}", value2);
    let _ = writeln!(out, "Output (base {}): {}", to_base, to_str(value2, to_base));
    if to_base != 2 && from_base != 2 { let _ = writeln!(out, "Binary:  {}", to_str(value2, 2)); }
    if to_base != 16 && from_base != 16 { let _ = writeln!(out, "Hex:     {}", to_str(value2, 16)); }
    if to_base != 8 && from_base != 8 { let _ = writeln!(out, "Octal:   {}", to_str(value2, 8)); }
    out
}

// ── Date arithmetic ───────────────────────────────────────────────────────────

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1|3|5|7|8|10|12 => 31,
        4|6|9|11 => 30,
        2 => if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) { 29 } else { 28 },
        _ => 30,
    }
}

fn to_jdn(y: i32, m: u32, d: u32) -> i64 {
    let a = (14 - m as i32) / 12;
    let yr = y + 4800 - a;
    let mo = m as i32 + 12 * a - 3;
    d as i64 + (153 * mo + 2) as i64 / 5 + 365 * yr as i64 + yr as i64 / 4 - yr as i64 / 100 + yr as i64 / 400 - 32045
}

fn from_jdn(jdn: i64) -> (i32, u32, u32) {
    let l = jdn + 68569;
    let n = 4 * l / 146097;
    let l = l - (146097 * n + 3) / 4;
    let i = 4000 * (l + 1) / 1461001;
    let l = l - 1461 * i / 4 + 31;
    let j = 80 * l / 2447;
    let d = l - 2447 * j / 80;
    let l = j / 11;
    let m = j + 2 - 12 * l;
    let y = 100 * (n - 49) + i + l;
    (y as i32, m as u32, d as u32)
}

fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let s = s.trim();
    // Try YYYY-MM-DD
    let parts: Vec<&str> = s.splitn(3, |c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).collect();
    if parts.len() == 3 {
        let y = parts[0].parse::<i32>().ok()?;
        let m = parts[1].parse::<u32>().ok()?;
        let d = parts[2].parse::<u32>().ok()?;
        if m >= 1 && m <= 12 && d >= 1 && d <= days_in_month(y, m) {
            return Some((y, m, d));
        }
    }
    None
}

const WEEKDAYS: &[&str] = &["Monday","Tuesday","Wednesday","Thursday","Friday","Saturday","Sunday"];

pub fn date_calc(input: &str) -> String {
    let s = input.trim();
    let mut out = String::new();

    // "DATE1 to DATE2" or "DATE1, DATE2" → days between
    // "DATE +N" or "DATE -N" → add/subtract days
    // "DATE" alone → info about that date
    // "unix TIMESTAMP" → convert Unix timestamp
    // "timestamp DATE" → date to Unix timestamp

    if s.to_lowercase().starts_with("unix ") {
        let ts: i64 = match s[5..].trim().parse() {
            Ok(v) => v,
            Err(_) => return "Usage: --date 'unix 1700000000'".into(),
        };
        let jdn = ts / 86400 + 2440588;
        let (y, m, d) = from_jdn(jdn);
        let dow = ((jdn + 1) % 7) as usize;
        let _ = writeln!(out, "Unix: {}", ts);
        let _ = writeln!(out, "Date: {}-{:02}-{:02} ({})", y, m, d, WEEKDAYS[dow]);
        return out;
    }

    if s.to_lowercase().starts_with("timestamp ") {
        let date_str = &s["timestamp ".len()..];
        if let Some((y, m, d)) = parse_date(date_str) {
            let jdn = to_jdn(y, m, d);
            let ts = (jdn - 2440588) * 86400;
            let _ = writeln!(out, "Date: {}-{:02}-{:02}", y, m, d);
            let _ = writeln!(out, "Unix timestamp (midnight UTC): {}", ts);
        } else {
            out.push_str("Could not parse date. Use YYYY-MM-DD format.");
        }
        return out;
    }

    // Split on "to" or ","
    let (a_str, b_str) = if let Some(pos) = s.to_lowercase().find(" to ") {
        (s[..pos].trim(), Some(s[pos+4..].trim()))
    } else if s.contains(',') {
        let mut it = s.splitn(2, ',');
        (it.next().unwrap_or("").trim(), Some(it.next().unwrap_or("").trim()))
    } else {
        (s, None)
    };

    // Check for "+N" or "-N" at the end (date arithmetic)
    let plus_re = {
        let a = a_str.trim_end();
        if let Some(idx) = a.rfind(|c: char| c == '+' || c == '-') {
            let (date_part, offset_part) = a.split_at(idx);
            if let Ok(n) = offset_part.trim().parse::<i64>() {
                Some((date_part.trim(), n))
            } else { None }
        } else { None }
    };

    if let Some((date_part, offset)) = plus_re {
        if let Some((y, m, d)) = parse_date(date_part) {
            let jdn = to_jdn(y, m, d) + offset;
            let (y2, m2, d2) = from_jdn(jdn);
            let dow = ((jdn + 1) % 7) as usize;
            let _ = writeln!(out, "{}-{:02}-{:02}  {} {} days  →  {}-{:02}-{:02} ({})",
                y, m, d, if offset >= 0 { "+" } else { "" }, offset,
                y2, m2, d2, WEEKDAYS[dow]);
        } else {
            out.push_str("Could not parse date. Use: --date '2024-03-15 +90' or '2024-01-01 to 2024-12-31'");
        }
        return out;
    }

    if let (Some((y1, m1, d1)), Some(b)) = (parse_date(a_str), b_str) {
        if let Some((y2, m2, d2)) = parse_date(b) {
            let jdn1 = to_jdn(y1, m1, d1);
            let jdn2 = to_jdn(y2, m2, d2);
            let diff = jdn2 - jdn1;
            let dow1 = ((jdn1 + 1) % 7) as usize;
            let dow2 = ((jdn2 + 1) % 7) as usize;
            let _ = writeln!(out, "From: {}-{:02}-{:02} ({})", y1, m1, d1, WEEKDAYS[dow1]);
            let _ = writeln!(out, "To:   {}-{:02}-{:02} ({})", y2, m2, d2, WEEKDAYS[dow2]);
            let _ = writeln!(out, "Difference: {} days  ({} weeks {} days)",
                diff.abs(), diff.abs() / 7, diff.abs() % 7);
            let _ = writeln!(out, "           ≈ {:.2} months  ≈ {:.3} years",
                diff.abs() as f64 / 30.4375, diff.abs() as f64 / 365.25);
            if diff < 0 { let _ = writeln!(out, "(B is before A — {} days ago)", diff.abs()); }
        } else {
            out.push_str("Could not parse second date.");
        }
        return out;
    }

    // Single date info
    if let Some((y, m, d)) = parse_date(a_str) {
        let jdn = to_jdn(y, m, d);
        let dow = ((jdn + 1) % 7) as usize;
        let ts  = (jdn - 2440588) * 86400;
        let day_of_year: u32 = (1..m).map(|mo| days_in_month(y, mo)).sum::<u32>() + d;
        let is_leap = y % 400 == 0 || (y % 4 == 0 && y % 100 != 0);
        let days_left = (if is_leap { 366 } else { 365 }) - day_of_year;
        let _ = writeln!(out, "Date:       {}-{:02}-{:02}", y, m, d);
        let _ = writeln!(out, "Day:        {} (day {} of {}, {} remaining)",
            WEEKDAYS[dow], day_of_year, if is_leap { 366 } else { 365 }, days_left);
        let _ = writeln!(out, "Leap year:  {}", if is_leap { "Yes" } else { "No" });
        let _ = writeln!(out, "Unix stamp: {} (midnight UTC)", ts);
        let _ = writeln!(out, "Julian day: {}", jdn);
    } else {
        out.push_str("Could not parse date. Examples:\n  --date '2024-06-15'\n  --date '2024-01-01 to 2024-12-31'\n  --date '2024-03-15 +90'\n  --date 'unix 1700000000'");
    }
    out
}

// ── IPv4 subnet calculator ────────────────────────────────────────────────────

pub fn subnet_calc(cidr: &str) -> String {
    let cidr = cidr.trim();
    let mut out = String::new();

    // Parse "A.B.C.D/prefix" or "A.B.C.D mask M.M.M.M"
    let (ip_str, prefix) = if let Some(idx) = cidr.find('/') {
        let prefix: u8 = match cidr[idx+1..].trim().parse() {
            Ok(v) => v,
            Err(_) => return "Invalid prefix length. Use CIDR format: 192.168.1.0/24".into(),
        };
        (&cidr[..idx], prefix)
    } else {
        return "Use CIDR notation: 192.168.1.0/24".into();
    };

    let parse_ip = |s: &str| -> Option<u32> {
        let parts: Vec<u8> = s.trim().split('.').filter_map(|x| x.parse().ok()).collect();
        if parts.len() == 4 { Some(((parts[0] as u32) << 24) | ((parts[1] as u32) << 16) | ((parts[2] as u32) << 8) | parts[3] as u32) }
        else { None }
    };

    let ip = match parse_ip(ip_str) {
        Some(v) => v,
        None => return format!("Invalid IP address: '{}'", ip_str),
    };

    if prefix > 32 { return "Prefix must be 0–32.".into(); }

    let mask: u32 = if prefix == 0 { 0 } else { !0u32 << (32 - prefix) };
    let network  = ip & mask;
    let broadcast = network | !mask;
    let first_host = if prefix >= 31 { network } else { network + 1 };
    let last_host  = if prefix >= 31 { broadcast } else { broadcast - 1 };
    let host_count: u64 = if prefix >= 32 { 1 } else if prefix == 31 { 2 } else { (1u64 << (32 - prefix)) - 2 };

    let fmt_ip = |v: u32| format!("{}.{}.{}.{}", v >> 24, (v >> 16) & 0xff, (v >> 8) & 0xff, v & 0xff);

    let class = match ip >> 24 {
        0..=127   => "A",
        128..=191 => "B",
        192..=223 => "C",
        224..=239 => "D (Multicast)",
        _         => "E (Reserved)",
    };
    let private = (ip >> 24) == 10
        || ((ip >> 24) == 172 && ((ip >> 20) & 0xf) == 1)
        || ((ip >> 24) == 192 && ((ip >> 16) & 0xff) == 168);

    let _ = writeln!(out, "CIDR:       {}/{}", fmt_ip(ip), prefix);
    let _ = writeln!(out, "Network:    {}/{}", fmt_ip(network), prefix);
    let _ = writeln!(out, "Broadcast:  {}", fmt_ip(broadcast));
    let _ = writeln!(out, "Subnet mask:{}", fmt_ip(mask));
    let _ = writeln!(out, "First host: {}", fmt_ip(first_host));
    let _ = writeln!(out, "Last host:  {}", fmt_ip(last_host));
    let _ = writeln!(out, "Hosts:      {}", host_count);
    let _ = writeln!(out, "Class:      {}  |  Private: {}", class, if private { "Yes" } else { "No" });
    out
}

// ── Color space conversion ────────────────────────────────────────────────────

pub fn color_convert(input: &str) -> String {
    let s = input.trim().to_ascii_lowercase();
    let mut out = String::new();

    // Parse hex: #RRGGBB or RRGGBB or #RGB
    let hex_input = s.trim_start_matches('#');
    let (r8, g8, b8) = if hex_input.len() == 6 {
        if let (Ok(r),Ok(g),Ok(b)) = (
            u8::from_str_radix(&hex_input[0..2],16),
            u8::from_str_radix(&hex_input[2..4],16),
            u8::from_str_radix(&hex_input[4..6],16)) {
            (r, g, b)
        } else { return format!("Invalid hex: '{}'", input); }
    } else if hex_input.len() == 3 {
        if let (Ok(r),Ok(g),Ok(b)) = (
            u8::from_str_radix(&hex_input[0..1].repeat(2),16),
            u8::from_str_radix(&hex_input[1..2].repeat(2),16),
            u8::from_str_radix(&hex_input[2..3].repeat(2),16)) {
            (r, g, b)
        } else { return format!("Invalid hex: '{}'", input); }
    } else if s.starts_with("rgb(") || s.starts_with("rgb ") {
        let nums: Vec<u8> = s.chars().filter(|c| c.is_ascii_digit() || *c == ' ' || *c == ',')
            .collect::<String>().split(|c: char| !c.is_ascii_digit())
            .filter_map(|x| x.parse().ok()).collect();
        if nums.len() >= 3 { (nums[0], nums[1], nums[2]) }
        else { return "Usage: --color '#ff8800' or --color 'rgb(255,136,0)'".into(); }
    } else {
        return "Usage: --color '#ff8800' or --color 'rgb(255,136,0)' or --color '3f8'".into();
    };

    let rf = r8 as f64 / 255.0;
    let gf = g8 as f64 / 255.0;
    let bf = b8 as f64 / 255.0;

    // RGB → HSL
    let cmax = rf.max(gf).max(bf);
    let cmin = rf.min(gf).min(bf);
    let delta = cmax - cmin;
    let l = (cmax + cmin) / 2.0;
    let s_hsl = if delta == 0.0 { 0.0 } else { delta / (1.0 - (2.0 * l - 1.0).abs()) };
    let h_hsl = if delta == 0.0 { 0.0 } else if cmax == rf {
        60.0 * (((gf - bf) / delta) % 6.0)
    } else if cmax == gf {
        60.0 * ((bf - rf) / delta + 2.0)
    } else {
        60.0 * ((rf - gf) / delta + 4.0)
    };
    let h_hsl = if h_hsl < 0.0 { h_hsl + 360.0 } else { h_hsl };

    // RGB → HSV
    let v_hsv = cmax;
    let s_hsv = if cmax == 0.0 { 0.0 } else { delta / cmax };

    // RGB → CMYK
    let k_cmyk = 1.0 - cmax;
    let (c_cmyk, m_cmyk, y_cmyk) = if k_cmyk == 1.0 { (0.0,0.0,0.0) } else {
        ((1.0-rf-k_cmyk)/(1.0-k_cmyk), (1.0-gf-k_cmyk)/(1.0-k_cmyk), (1.0-bf-k_cmyk)/(1.0-k_cmyk))
    };

    let _ = writeln!(out, "Hex:   #{:02X}{:02X}{:02X}", r8, g8, b8);
    let _ = writeln!(out, "RGB:   rgb({}, {}, {})", r8, g8, b8);
    let _ = writeln!(out, "HSL:   hsl({:.1}°, {:.1}%, {:.1}%)", h_hsl, s_hsl*100.0, l*100.0);
    let _ = writeln!(out, "HSV:   hsv({:.1}°, {:.1}%, {:.1}%)", h_hsl, s_hsv*100.0, v_hsv*100.0);
    let _ = writeln!(out, "CMYK:  cmyk({:.0}%, {:.0}%, {:.0}%, {:.0}%)",
        c_cmyk*100.0, m_cmyk*100.0, y_cmyk*100.0, k_cmyk*100.0);
    // Luminance (WCAG)
    let lum = 0.2126*rf + 0.7152*gf + 0.0722*bf;
    let _ = writeln!(out, "Luminance: {:.4}  (WCAG relative, 0=black 1=white)", lum);
    let contrast_white = (1.0 + 0.05) / (lum + 0.05);
    let _ = writeln!(out, "Contrast vs white: {:.2}:1  (WCAG AA needs 4.5:1)", contrast_white);
    out
}

// ── Molecular weight calculator ───────────────────────────────────────────────
// Parses chemical formulas like H2O, C6H12O6, Ca(NO3)2, (NH4)2SO4

// Symbol → atomic mass (standard atomic weights, IUPAC 2021)
fn atomic_masses() -> &'static [(&'static str, f64)] {
    &[
        ("H",1.008),("He",4.0026),("Li",6.94),("Be",9.0122),("B",10.81),
        ("C",12.011),("N",14.007),("O",15.999),("F",18.998),("Ne",20.180),
        ("Na",22.990),("Mg",24.305),("Al",26.982),("Si",28.085),("P",30.974),
        ("S",32.06),("Cl",35.45),("Ar",39.948),("K",39.098),("Ca",40.078),
        ("Sc",44.956),("Ti",47.867),("V",50.942),("Cr",51.996),("Mn",54.938),
        ("Fe",55.845),("Co",58.933),("Ni",58.693),("Cu",63.546),("Zn",65.38),
        ("Ga",69.723),("Ge",72.630),("As",74.922),("Se",78.971),("Br",79.904),
        ("Kr",83.798),("Rb",85.468),("Sr",87.62),("Y",88.906),("Zr",91.224),
        ("Nb",92.906),("Mo",95.95),("Tc",98.0),("Ru",101.07),("Rh",102.906),
        ("Pd",106.42),("Ag",107.868),("Cd",112.414),("In",114.818),("Sn",118.710),
        ("Sb",121.760),("Te",127.60),("I",126.904),("Xe",131.293),("Cs",132.905),
        ("Ba",137.327),("La",138.905),("Ce",140.116),("Pr",140.908),("Nd",144.242),
        ("Pm",145.0),("Sm",150.36),("Eu",151.964),("Gd",157.25),("Tb",158.925),
        ("Dy",162.500),("Ho",164.930),("Er",167.259),("Tm",168.934),("Yb",173.045),
        ("Lu",174.967),("Hf",178.49),("Ta",180.948),("W",183.84),("Re",186.207),
        ("Os",190.23),("Ir",192.217),("Pt",195.084),("Au",196.967),("Hg",200.592),
        ("Tl",204.38),("Pb",207.2),("Bi",208.980),("Po",209.0),("At",210.0),
        ("Rn",222.0),("Fr",223.0),("Ra",226.0),("Ac",227.0),("Th",232.038),
        ("Pa",231.036),("U",238.029),("Np",237.0),("Pu",244.0),("Am",243.0),
        ("Cm",247.0),("Bk",247.0),("Cf",251.0),("Es",252.0),("Fm",257.0),
        ("Md",258.0),("No",259.0),("Lr",266.0),("Rf",267.0),("Db",268.0),
        ("Sg",271.0),("Bh",270.0),("Hs",277.0),("Mt",278.0),("Ds",281.0),
        ("Rg",282.0),("Cn",285.0),("Nh",286.0),("Fl",289.0),("Mc",290.0),
        ("Lv",293.0),("Ts",294.0),("Og",294.0),
    ]
}

fn lookup_mass(sym: &str) -> Option<f64> {
    atomic_masses().iter().find(|(s, _)| *s == sym).map(|(_, m)| *m)
}

fn parse_formula(chars: &[char], pos: &mut usize) -> Result<Vec<(String, u32)>, String> {
    let mut items: Vec<(String, u32)> = Vec::new();
    while *pos < chars.len() {
        match chars[*pos] {
            '(' => {
                *pos += 1;
                let inner = parse_formula(chars, pos)?;
                if *pos >= chars.len() || chars[*pos] != ')' {
                    return Err("Missing closing ')'".into());
                }
                *pos += 1;
                let count = read_number(chars, pos).unwrap_or(1);
                for (sym, n) in inner { items.push((sym, n * count)); }
            }
            '[' => {
                *pos += 1;
                let inner = parse_formula(chars, pos)?;
                if *pos >= chars.len() || chars[*pos] != ']' {
                    return Err("Missing closing ']'".into());
                }
                *pos += 1;
                let count = read_number(chars, pos).unwrap_or(1);
                for (sym, n) in inner { items.push((sym, n * count)); }
            }
            ')' | ']' => break,
            c if c.is_ascii_uppercase() => {
                let mut sym = c.to_string();
                *pos += 1;
                while *pos < chars.len() && chars[*pos].is_ascii_lowercase() {
                    sym.push(chars[*pos]);
                    *pos += 1;
                }
                let count = read_number(chars, pos).unwrap_or(1);
                items.push((sym, count));
            }
            '·' | '•' | '.' => { *pos += 1; } // hydrate dot
            ' ' | '\t' => { *pos += 1; }
            other => return Err(format!("Unexpected character: '{}'", other)),
        }
    }
    Ok(items)
}

fn read_number(chars: &[char], pos: &mut usize) -> Option<u32> {
    let start = *pos;
    while *pos < chars.len() && chars[*pos].is_ascii_digit() { *pos += 1; }
    if *pos == start { None } else {
        chars[start..*pos].iter().collect::<String>().parse().ok()
    }
}

pub fn molecular_weight(formula: &str) -> String {
    let chars: Vec<char> = formula.chars().collect();
    let mut pos = 0usize;
    let items = match parse_formula(&chars, &mut pos) {
        Ok(v) => v,
        Err(e) => return format!("Parse error: {}. Example: H2O, C6H12O6, Ca(NO3)2", e),
    };

    // Aggregate counts by element
    let mut counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for (sym, n) in &items {
        *counts.entry(sym.clone()).or_insert(0) += n;
    }

    let mut mw = 0.0f64;
    let mut breakdown: Vec<(String, u32, f64)> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    let mut syms: Vec<String> = counts.keys().cloned().collect();
    syms.sort();
    for sym in &syms {
        let n = counts[sym];
        if let Some(mass) = lookup_mass(sym) {
            let contrib = mass * n as f64;
            mw += contrib;
            breakdown.push((sym.clone(), n, contrib));
        } else {
            unknown.push(sym.clone());
        }
    }

    if !unknown.is_empty() {
        return format!("Unknown element(s): {}. Check your formula.", unknown.join(", "));
    }

    let mut out = String::new();
    let _ = writeln!(out, "Formula: {}", formula.trim());
    let _ = writeln!(out, "Molecular weight: {:.4} g/mol", mw);
    let _ = writeln!(out, "");
    let _ = writeln!(out, "Composition:");
    for (sym, n, contrib) in &breakdown {
        let pct = 100.0 * contrib / mw;
        let _ = writeln!(out, "  {:2} × {:2}  {:8.4} g/mol  ({:.2}%)", n, sym, contrib, pct);
    }
    // Common derived values
    let _ = writeln!(out, "");
    let _ = writeln!(out, "1 mole = {:.4} g", mw);
    let _ = writeln!(out, "1 g    = {:.6} mol", 1.0 / mw);
    out
}

// ── Physical constants ────────────────────────────────────────────────────────

const CONSTANTS: &[(&str, &str, &str, &str)] = &[
    // (name, value, unit, aliases)
    ("Speed of light",          "299792458",                     "m/s",            "c,light,speed of light,c0"),
    ("Planck constant",         "6.62607015e-34",                "J·s",            "h,planck,planck constant"),
    ("Reduced Planck (ℏ)",      "1.054571817e-34",               "J·s",            "hbar,h-bar,reduced planck,hbar"),
    ("Gravitational constant",  "6.67430e-11",                   "N·m²/kg²",       "G,gravity,gravitational,newton gravity"),
    ("Elementary charge",       "1.602176634e-19",               "C",              "e,electron charge,elementary charge,charge"),
    ("Electron mass",           "9.1093837015e-31",              "kg",             "me,electron mass,m_e"),
    ("Proton mass",             "1.67262192369e-27",             "kg",             "mp,proton mass,m_p"),
    ("Neutron mass",            "1.67492749804e-27",             "kg",             "mn,neutron mass,m_n"),
    ("Avogadro constant",       "6.02214076e23",                 "mol⁻¹",          "NA,avogadro,avogadro constant,N_A"),
    ("Boltzmann constant",      "1.380649e-23",                  "J/K",            "k,kb,boltzmann,boltzmann constant,k_B"),
    ("Gas constant",            "8.314462618",                   "J/(mol·K)",      "R,gas constant,universal gas constant,molar gas"),
    ("Stefan-Boltzmann",        "5.670374419e-8",                "W/(m²·K⁴)",      "sigma,stefan,stefan-boltzmann,σ"),
    ("Vacuum permittivity",     "8.8541878128e-12",              "F/m",            "eps0,epsilon0,vacuum permittivity,ε₀"),
    ("Vacuum permeability",     "1.25663706212e-6",              "N/A²",           "mu0,mu_0,vacuum permeability,μ₀"),
    ("Bohr radius",             "5.29177210903e-11",             "m",              "a0,bohr,bohr radius,a_0"),
    ("Fine structure constant", "7.2973525693e-3",               "dimensionless",  "alpha,fine structure,α"),
    ("Rydberg constant",        "10973731.568160",               "m⁻¹",            "Ry,rydberg,rydberg constant"),
    ("Faraday constant",        "96485.33212",                   "C/mol",          "F,faraday,faraday constant"),
    ("Standard gravity",        "9.80665",                       "m/s²",           "g,grav,standard gravity,g0,g_n"),
    ("Atomic mass unit",        "1.66053906660e-27",             "kg",             "amu,u,dalton,atomic mass unit"),
    ("Standard atmosphere",     "101325",                        "Pa",             "atm,atmosphere,standard atmosphere"),
    ("Electron volt",           "1.602176634e-19",               "J",              "eV,electronvolt,electron volt"),
    ("Speed of sound (20°C)",   "343",                           "m/s",            "sound,speed of sound,vsound"),
    ("Molar volume (STP)",      "22.414",                        "L/mol",          "molar volume,vm,STP volume"),
    ("Wien displacement",       "2.897771955e-3",                "m·K",            "wien,wien displacement,b"),
];

pub fn physical_const(query: &str) -> String {
    let q = query.trim().to_lowercase();
    let mut out = String::new();

    if q.is_empty() || q == "list" || q == "all" {
        let _ = writeln!(out, "Physical Constants  (use --const NAME to look up)");
        let _ = writeln!(out, "{}", "─".repeat(60));
        for (name, val, unit, _) in CONSTANTS {
            let _ = writeln!(out, "  {:<30} {} {}", name, val, unit);
        }
        return out;
    }

    let matches: Vec<_> = CONSTANTS.iter().filter(|(name, _, _, aliases)| {
        name.to_lowercase().contains(&q) || aliases.to_lowercase().contains(&q)
    }).collect();

    if matches.is_empty() {
        let _ = writeln!(out, "No constant found for '{}'. Use --const list to see all.", query.trim());
        return out;
    }

    for (name, val, unit, aliases) in matches {
        let _ = writeln!(out, "{}", name);
        let _ = writeln!(out, "  Value:   {}", val);
        let _ = writeln!(out, "  Unit:    {}", unit);
        let _ = writeln!(out, "  Aliases: {}", aliases);
        // Parse as f64 for display
        if let Ok(v) = val.parse::<f64>() {
            if v.abs() > 1e6 || v.abs() < 1e-4 {
                let _ = writeln!(out, "  ≈        {:.6e}", v);
            }
        }
        let _ = writeln!(out, "");
    }
    out
}

// ── Normal distribution (statistics) ─────────────────────────────────────────
// CDF, PDF, and inverse CDF (quantile) using rational approximations.

fn erf_approx(x: f64) -> f64 {
    // Abramowitz & Stegun 7.1.26 — max error 1.5e-7
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let y = 1.0 - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t + 0.254829592) * t * (-x * x).exp();
    sign * y
}

fn normal_cdf(x: f64, mu: f64, sigma: f64) -> f64 {
    0.5 * (1.0 + erf_approx((x - mu) / (sigma * 2.0f64.sqrt())))
}

fn normal_pdf(x: f64, mu: f64, sigma: f64) -> f64 {
    let z = (x - mu) / sigma;
    (-0.5 * z * z).exp() / (sigma * (2.0 * std::f64::consts::PI).sqrt())
}

fn normal_inv_cdf(p: f64) -> f64 {
    // Rational approximation (Peter Acklam)
    let p = p.clamp(1e-10, 1.0 - 1e-10);
    let (a, b) = if p < 0.5 {
        let t = (-2.0 * p.ln()).sqrt();
        let c = [2.515517, 0.802853, 0.010328];
        let d = [1.432788, 0.189269, 0.001308];
        let num = c[0] + c[1]*t + c[2]*t*t;
        let den = 1.0 + d[0]*t + d[1]*t*t + d[2]*t*t*t;
        (-(t - num/den), p)
    } else {
        let t = (-2.0 * (1.0 - p).ln()).sqrt();
        let c = [2.515517, 0.802853, 0.010328];
        let d = [1.432788, 0.189269, 0.001308];
        let num = c[0] + c[1]*t + c[2]*t*t;
        let den = 1.0 + d[0]*t + d[1]*t*t + d[2]*t*t*t;
        (t - num/den, p)
    };
    let _ = b;
    a
}

pub fn stat_normal(query: &str) -> String {
    let q = query.trim().to_lowercase();
    let mut out = String::new();

    // Parse: "cdf X", "cdf X mu sigma", "pdf X", "inv P", "z-score X mu sigma", "p-value Z"
    let parts: Vec<&str> = q.split_whitespace().collect();
    if parts.is_empty() {
        out.push_str("Usage:\n  --normal 'cdf 1.96'             P(Z ≤ 1.96) for standard normal\n  --normal 'cdf 70 60 10'        P(X ≤ 70) for N(60, 10)\n  --normal 'pdf 1.96'            Standard normal PDF at x=1.96\n  --normal 'inv 0.975'           z-score for 97.5th percentile\n  --normal 'between -1.96 1.96'  P(-1.96 ≤ Z ≤ 1.96)");
        return out;
    }

    let parse_f = |s: &str| s.parse::<f64>().ok();

    match parts[0] {
        "cdf" if parts.len() >= 2 => {
            let x  = parse_f(parts[1]).unwrap_or(0.0);
            let mu = if parts.len() > 2 { parse_f(parts[2]).unwrap_or(0.0) } else { 0.0 };
            let sg = if parts.len() > 3 { parse_f(parts[3]).unwrap_or(1.0) } else { 1.0 };
            let p  = normal_cdf(x, mu, sg);
            let z  = (x - mu) / sg;
            let _ = writeln!(out, "Distribution: N({}, {})", mu, sg);
            let _ = writeln!(out, "x = {}", x);
            let _ = writeln!(out, "z-score = {:.6}", z);
            let _ = writeln!(out, "P(X ≤ {}) = {:.8}  ({:.4}%)", x, p, p * 100.0);
            let _ = writeln!(out, "P(X > {}) = {:.8}  ({:.4}%)", x, 1.0 - p, (1.0-p)*100.0);
        }
        "pdf" if parts.len() >= 2 => {
            let x  = parse_f(parts[1]).unwrap_or(0.0);
            let mu = if parts.len() > 2 { parse_f(parts[2]).unwrap_or(0.0) } else { 0.0 };
            let sg = if parts.len() > 3 { parse_f(parts[3]).unwrap_or(1.0) } else { 1.0 };
            let p  = normal_pdf(x, mu, sg);
            let _ = writeln!(out, "PDF at x={}: {:.8}", x, p);
        }
        "inv" | "quantile" | "ppf" if parts.len() >= 2 => {
            let p  = parse_f(parts[1]).unwrap_or(0.5);
            let mu = if parts.len() > 2 { parse_f(parts[2]).unwrap_or(0.0) } else { 0.0 };
            let sg = if parts.len() > 3 { parse_f(parts[3]).unwrap_or(1.0) } else { 1.0 };
            let z  = normal_inv_cdf(p);
            let x  = mu + sg * z;
            let _ = writeln!(out, "P = {} → z-score = {:.6} → x = {:.6}", p, z, x);
            let _ = writeln!(out, "Interpretation: P(X ≤ {:.6}) = {:.4}%", x, p*100.0);
        }
        "between" | "interval" if parts.len() >= 3 => {
            let a  = parse_f(parts[1]).unwrap_or(-1.96);
            let b  = parse_f(parts[2]).unwrap_or( 1.96);
            let mu = if parts.len() > 3 { parse_f(parts[3]).unwrap_or(0.0) } else { 0.0 };
            let sg = if parts.len() > 4 { parse_f(parts[4]).unwrap_or(1.0) } else { 1.0 };
            let p  = normal_cdf(b, mu, sg) - normal_cdf(a, mu, sg);
            let _ = writeln!(out, "P({} ≤ X ≤ {}) = {:.8}  ({:.4}%)", a, b, p, p*100.0);
        }
        "table" | "z-table" => {
            let _ = writeln!(out, "Standard Normal CDF  P(Z ≤ z)");
            let _ = writeln!(out, "  z     P(Z ≤ z)   P(Z > z)");
            let _ = writeln!(out, "  ─────────────────────────────");
            for &z in &[-3.0f64, -2.576, -2.326, -1.960, -1.645, -1.282, -0.842, 0.0,
                         0.842, 1.282, 1.645, 1.960, 2.326, 2.576, 3.0] {
                let p = normal_cdf(z, 0.0, 1.0);
                let _ = writeln!(out, "  {:6.3}   {:.6}   {:.6}", z, p, 1.0 - p);
            }
        }
        _ => {
            // Try to parse as a plain z-score
            if let Some(z) = parse_f(parts[0]) {
                let p = normal_cdf(z, 0.0, 1.0);
                let _ = writeln!(out, "Standard normal CDF at z={}: {:.8}", z, p);
            } else {
                out.push_str("Usage: --normal 'cdf 1.96'  --normal 'inv 0.975'  --normal 'table'\n       --normal 'between -1.96 1.96'  --normal 'pdf 0'");
            }
        }
    }
    out
}
