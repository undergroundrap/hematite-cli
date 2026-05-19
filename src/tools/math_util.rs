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

// ── Extended number theory ────────────────────────────────────────────────────
// Query forms:
//   "extgcd 35 15"         — extended Euclidean algorithm
//   "crt 2 3 3 5"          — Chinese Remainder Theorem: x ≡ 2 (mod 3), x ≡ 3 (mod 5)
//   "mobius 12"            — Möbius function μ(n)
//   "modinv 7 13"          — modular inverse of 7 mod 13
//   "modpow 3 10 1000"     — 3^10 mod 1000
//   "cf 355/113"           — continued fraction expansion
//   "goldbach 28"          — Goldbach conjecture: express as sum of two primes
//   "totient 36"           — Euler's totient φ(n) (already in prime_info, but here as standalone)
//   "jacobi 5 15"          — Jacobi symbol (a/n)
//   "fermat 17"            — Fermat primality witness check

pub fn number_theory(query: &str) -> String {
    let q = query.trim();
    let tokens: Vec<&str> = q.split_whitespace().collect();
    if tokens.is_empty() {
        return nt_usage();
    }
    match tokens[0].to_lowercase().as_str() {
        "extgcd" | "xgcd" => {
            if tokens.len() < 3 { return "Usage: extgcd A B".into(); }
            let a: i128 = match tokens[1].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[1]) };
            let b: i128 = match tokens[2].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[2]) };
            let (g, x, y) = ext_gcd(a, b);
            format!(
                "Extended GCD({}, {}):\n  GCD = {}\n  Bézout: {}×{} + {}×{} = {}\n  (verify: {}×{} + {}×{} = {})",
                a, b, g, x, a, y, b, g, x, a, y, b, x*a + y*b
            )
        }
        "crt" => {
            // Interleaved: crt r1 m1 r2 m2 ...
            if tokens.len() < 5 || tokens.len() % 2 == 0 {
                return "Usage: crt r1 m1 r2 m2 [r3 m3 ...]\n  Example: crt 2 3 3 5  (x ≡ 2 mod 3 and x ≡ 3 mod 5)".into();
            }
            let pairs: Vec<(i128,i128)> = tokens[1..].chunks(2)
                .filter_map(|c| {
                    let r = c[0].parse::<i128>().ok()?;
                    let m = c[1].parse::<i128>().ok()?;
                    Some((r, m))
                })
                .collect();
            if pairs.len() < 2 { return "Need at least 2 remainder-modulus pairs.".into(); }
            match crt(&pairs) {
                Some((x, m)) => {
                    let mut out = format!("Chinese Remainder Theorem:\n");
                    for (r, mo) in &pairs { out.push_str(&format!("  x ≡ {} (mod {})\n", r, mo)); }
                    out.push_str(&format!("Solution: x ≡ {} (mod {})  [smallest positive: {}]", x, m, ((x % m) + m) % m));
                    out
                }
                None => "No solution — moduli are not pairwise coprime.".into(),
            }
        }
        "mobius" | "möbius" | "mu" => {
            if tokens.len() < 2 { return "Usage: mobius N".into(); }
            let n: u64 = match tokens[1].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[1]) };
            let mu = mobius(n);
            let explanation = match mu {
                0  => "n has a squared prime factor → μ(n) = 0",
                1  => "n is squarefree with even number of prime factors → μ(n) = 1",
                -1 => "n is squarefree with odd number of prime factors → μ(n) = -1",
                _  => "",
            };
            format!("Möbius function μ({}) = {}\n  {}", n, mu, explanation)
        }
        "modinv" => {
            if tokens.len() < 3 { return "Usage: modinv A MOD".into(); }
            let a: i128 = match tokens[1].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[1]) };
            let m: i128 = match tokens[2].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[2]) };
            match mod_inv(a, m) {
                Some(inv) => format!("Modular inverse: {}⁻¹ ≡ {} (mod {})\nVerify: {} × {} = {} ≡ 1 (mod {})", a, inv, m, a, inv, a*inv, m),
                None => format!("No modular inverse: gcd({}, {}) ≠ 1 (not coprime)", a, m),
            }
        }
        "modpow" | "powmod" => {
            if tokens.len() < 4 { return "Usage: modpow BASE EXP MOD".into(); }
            let base: u128 = match tokens[1].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[1]) };
            let exp:  u128 = match tokens[2].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[2]) };
            let modu: u128 = match tokens[3].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[3]) };
            if modu == 0 { return "Modulus cannot be zero.".into(); }
            let result = mod_pow(base, exp, modu);
            format!("{}^{} mod {} = {}", base, exp, modu, result)
        }
        "cf" | "cfrac" | "continued_fraction" => {
            if tokens.len() < 2 { return "Usage: cf N/D  or  cf DECIMAL".into(); }
            let input = tokens[1];
            let (num, den) = if input.contains('/') {
                let parts: Vec<&str> = input.splitn(2, '/').collect();
                let n: i64 = parts[0].parse().unwrap_or(0);
                let d: i64 = parts[1].parse().unwrap_or(1);
                (n, d)
            } else if let Ok(f) = input.parse::<f64>() {
                // Approximate as fraction with denominator up to 1e6
                let scale = 1_000_000i64;
                ((f * scale as f64).round() as i64, scale)
            } else {
                return format!("Cannot parse: {}", input);
            };
            if den == 0 { return "Denominator cannot be zero.".into(); }
            let coeffs = cf_expansion(num, den, 20);
            let convergents = cf_convergents(&coeffs);
            let mut out = format!("Continued fraction of {}/{} = {}:\n", num, den, num as f64 / den as f64);
            out.push_str(&format!("  CF = [{}]\n", coeffs.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("; ")));
            out.push_str("  Convergents:\n");
            for (p, q) in &convergents {
                out.push_str(&format!("    {}/{} = {:.8}\n", p, q, *p as f64 / *q as f64));
            }
            out
        }
        "goldbach" => {
            if tokens.len() < 2 { return "Usage: goldbach N (must be even, > 2)".into(); }
            let n: u64 = match tokens[1].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[1]) };
            if n <= 2 || n % 2 != 0 { return format!("{} must be even and > 2 for Goldbach's conjecture.", n); }
            let pairs: Vec<(u64,u64)> = (2..=n/2)
                .filter(|&p| is_prime(p) && is_prime(n - p))
                .map(|p| (p, n - p))
                .collect();
            let mut out = format!("Goldbach decompositions of {}:\n", n);
            if pairs.is_empty() {
                out.push_str("  No decompositions found (unexpected for n > 2).\n");
            } else {
                for (p, q) in pairs.iter().take(10) {
                    out.push_str(&format!("  {} = {} + {}\n", n, p, q));
                }
                if pairs.len() > 10 { out.push_str(&format!("  ... ({} total decompositions)\n", pairs.len())); }
            }
            out
        }
        "totient" | "phi" | "euler" => {
            if tokens.len() < 2 { return "Usage: totient N".into(); }
            let n: u64 = match tokens[1].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[1]) };
            let phi = euler_totient(n);
            format!("Euler's totient φ({}) = {}\n  (count of integers 1..{} coprime to {})", n, phi, n, n)
        }
        "jacobi" => {
            if tokens.len() < 3 { return "Usage: jacobi A N (N must be odd)".into(); }
            let a: i64 = match tokens[1].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[1]) };
            let n: i64 = match tokens[2].parse() { Ok(v) => v, Err(_) => return format!("Not a number: {}", tokens[2]) };
            if n <= 0 || n % 2 == 0 { return "N must be a positive odd integer.".into(); }
            let j = jacobi_symbol(a, n);
            let meaning = match j {
                0  => "a is not coprime to n",
                1  => "a is a quadratic residue mod n (or n is composite)",
                -1 => "a is a quadratic non-residue mod n",
                _  => "",
            };
            format!("Jacobi symbol ({}/{}) = {}\n  {}", a, n, j, meaning)
        }
        _ => {
            // Try to interpret as a single number for a complete number theory report
            if let Ok(n) = tokens[0].parse::<u64>() {
                nt_report(n)
            } else {
                nt_usage()
            }
        }
    }
}

fn nt_usage() -> String {
    "Number theory operations:\n\
     hematite --number-theory 'extgcd 35 15'\n\
     hematite --number-theory 'crt 2 3 3 5'\n\
     hematite --number-theory 'mobius 30'\n\
     hematite --number-theory 'modinv 7 13'\n\
     hematite --number-theory 'modpow 3 10 1000'\n\
     hematite --number-theory 'cf 355/113'\n\
     hematite --number-theory 'goldbach 28'\n\
     hematite --number-theory 'totient 36'\n\
     hematite --number-theory 'jacobi 5 15'\n\
     hematite --number-theory '42'    (full report for a number)".into()
}

fn nt_report(n: u64) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Number theory report for {}", n);
    let _ = writeln!(out, "  Euler's totient φ(n) = {}", euler_totient(n));
    let _ = writeln!(out, "  Möbius μ(n) = {}", mobius(n));
    if n < 1_000_000 {
        let sigma: u64 = (1..=n).filter(|d| n % d == 0).sum();
        let _ = writeln!(out, "  Sum of divisors σ(n) = {}", sigma);
        if sigma == 2 * n { let _ = writeln!(out, "  → Perfect number!"); }
        else if sigma > 2 * n { let _ = writeln!(out, "  → Abundant number"); }
        else { let _ = writeln!(out, "  → Deficient number"); }
    }
    if n >= 4 && n % 2 == 0 {
        if let Some((p, q)) = (2..=n/2).filter(|&p| is_prime(p) && is_prime(n - p)).map(|p| (p, n-p)).next() {
            let _ = writeln!(out, "  Goldbach: {} = {} + {}", n, p, q);
        }
    }
    out
}

fn ext_gcd(a: i128, b: i128) -> (i128, i128, i128) {
    if b == 0 { return (a, 1, 0); }
    let (g, x1, y1) = ext_gcd(b, a % b);
    (g, y1, x1 - (a / b) * y1)
}

fn mod_inv(a: i128, m: i128) -> Option<i128> {
    let (g, x, _) = ext_gcd(a.rem_euclid(m), m);
    if g != 1 { return None; }
    Some(x.rem_euclid(m))
}

fn mod_pow(mut base: u128, mut exp: u128, modu: u128) -> u128 {
    if modu == 1 { return 0; }
    let mut result = 1u128;
    base %= modu;
    while exp > 0 {
        if exp % 2 == 1 { result = result.wrapping_mul(base) % modu; }
        exp /= 2;
        base = base.wrapping_mul(base) % modu;
    }
    result
}

fn crt(pairs: &[(i128, i128)]) -> Option<(i128, i128)> {
    let mut x = pairs[0].0;
    let mut m = pairs[0].1;
    for &(r, mi) in &pairs[1..] {
        let g = gcd(m as u128, mi.unsigned_abs()) as i128;
        if (r - x) % g != 0 { return None; }
        let lcm = m / g * mi;
        let inv = mod_inv(m / g, mi / g)?;
        x = x + m * ((r - x) / g % (mi / g) * inv % (mi / g));
        m = lcm;
        x = x.rem_euclid(m);
    }
    Some((x, m))
}

fn mobius(n: u64) -> i32 {
    if n == 1 { return 1; }
    let factors = factorize(n);
    for (_, exp) in &factors { if *exp > 1 { return 0; } }
    if factors.len() % 2 == 0 { 1 } else { -1 }
}

fn euler_totient(n: u64) -> u64 {
    if n == 0 { return 0; }
    let factors = factorize(n);
    let mut phi = n;
    for (p, _) in factors { phi = phi / p * (p - 1); }
    phi
}

fn cf_expansion(mut num: i64, mut den: i64, max_terms: usize) -> Vec<i64> {
    let mut coeffs = Vec::new();
    for _ in 0..max_terms {
        coeffs.push(num / den);
        let rem = num % den;
        if rem == 0 { break; }
        num = den; den = rem;
    }
    coeffs
}

fn cf_convergents(coeffs: &[i64]) -> Vec<(i64, i64)> {
    let mut result = Vec::new();
    let (mut p_prev, mut q_prev) = (1i64, 0i64);
    let (mut p_curr, mut q_curr) = (coeffs[0], 1i64);
    result.push((p_curr, q_curr));
    for &a in &coeffs[1..] {
        let p_next = a * p_curr + p_prev;
        let q_next = a * q_curr + q_prev;
        result.push((p_next, q_next));
        p_prev = p_curr; q_prev = q_curr;
        p_curr = p_next; q_curr = q_next;
    }
    result
}

fn jacobi_symbol(mut a: i64, mut n: i64) -> i32 {
    if n <= 0 || n % 2 == 0 { return 0; }
    let mut result = 1i32;
    a = a.rem_euclid(n);
    while a != 0 {
        while a % 2 == 0 {
            a /= 2;
            if n % 8 == 3 || n % 8 == 5 { result = -result; }
        }
        std::mem::swap(&mut a, &mut n);
        if a % 4 == 3 && n % 4 == 3 { result = -result; }
        a %= n;
    }
    if n == 1 { result } else { 0 }
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

// ── Unit conversion ───────────────────────────────────────────────────────────
// Query forms:
//   "5 km to miles"   "32 F to C"   "1 atm to Pa"   "100 mph to km/h"
//   "list" or "units" — show all supported categories and units

pub fn unit_convert(query: &str) -> String {
    let q = query.trim();
    if q.eq_ignore_ascii_case("list") || q.eq_ignore_ascii_case("units") || q.eq_ignore_ascii_case("help") {
        return unit_convert_list();
    }

    // Parse: "<value> <from_unit> to <to_unit>"
    // Also accept: "<value> <from_unit> in <to_unit>"
    let lower = q.to_lowercase();
    let sep = if lower.contains(" to ") { " to " } else if lower.contains(" in ") { " in " } else { "" };

    if sep.is_empty() {
        return format!(
            "Usage: hematite --convert '5 km to miles'\n\
             Common examples:\n\
             hematite --convert '100 f to c'\n\
             hematite --convert '1 atm to Pa'\n\
             hematite --convert '60 mph to km/h'\n\
             hematite --convert '1 GiB to MB'\n\
             hematite --convert '1 cal to J'\n\
             hematite --convert 'list'    (show all units)"
        );
    }

    let parts: Vec<&str> = q.splitn(2, &sep.to_uppercase().as_str().to_string()).collect();
    let parts: Vec<&str> = if parts.len() < 2 {
        q.splitn(2, sep).collect()
    } else { parts };

    if parts.len() < 2 {
        return format!("Could not parse: '{}'. Try: '5 km to miles'", q);
    }

    let lhs = parts[0].trim();
    let to_unit = parts[1].trim();

    // Split lhs into numeric value and unit
    let (value_str, from_unit) = split_value_unit(lhs);
    let value: f64 = match value_str.parse() {
        Ok(v) => v,
        Err(_) => return format!("Cannot parse value: '{}'", value_str),
    };

    match convert(value, from_unit.trim(), to_unit.trim()) {
        Ok((result, category)) => {
            let result_str = if result.abs() >= 1e-3 && result.abs() < 1e7 {
                format!("{:.8}", result).trim_end_matches('0').trim_end_matches('.').to_string()
            } else {
                format!("{:.6e}", result)
            };
            format!(
                "{} {} = {} {}\n({})",
                value_str.trim(), from_unit.trim(), result_str, to_unit.trim(), category
            )
        }
        Err(e) => e,
    }
}

fn split_value_unit(s: &str) -> (&str, &str) {
    // Find the boundary between the numeric part and the unit part
    let s = s.trim();
    let mut end = 0;
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
            end = i + c.len_utf8();
        } else if i == 0 && (c == '-' || c == '+') {
            end = 1;
        } else if i > 0 {
            break;
        }
    }
    if end == 0 { end = s.len(); }
    (&s[..end], s[end..].trim())
}

// Returns (converted_value, category_name) or Err(message)
fn convert(value: f64, from: &str, to: &str) -> Result<(f64, &'static str), String> {
    // Normalize unit names
    let from_n = norm_unit(from);
    let to_n   = norm_unit(to);

    // Walk categories
    for (cat_name, units) in UNIT_CATEGORIES {
        // Find from_unit base factor
        let from_entry = units.iter().find(|(names, _)| names.iter().any(|n| *n == from_n.as_str()));
        let to_entry   = units.iter().find(|(names, _)| names.iter().any(|n| *n == to_n.as_str()));

        if let (Some(fe), Some(te)) = (from_entry, to_entry) {
            // Temperature handled specially
            if *cat_name == "Temperature" {
                return Ok((convert_temperature(value, from_n.as_str(), to_n.as_str()), "Temperature"));
            }
            // For all other categories: value * from_factor = SI base; SI base / to_factor = result
            let si = value * fe.1;
            let result = si / te.1;
            return Ok((result, cat_name));
        }
    }

    // Try to give a helpful error
    let known: Vec<&str> = UNIT_CATEGORIES
        .iter()
        .flat_map(|(_, units)| units.iter().flat_map(|(names, _)| names.iter().copied()))
        .collect();
    let mut close: Vec<&str> = known.iter().filter(|n| levenshtein(n, from_n.as_str()) <= 2).copied().collect();
    close.extend(known.iter().filter(|n| levenshtein(n, to_n.as_str()) <= 2).copied());
    close.dedup();

    if close.is_empty() {
        Err(format!("Unknown units: '{}' or '{}'. Run 'hematite --convert list' to see all.", from, to))
    } else {
        Err(format!("Unknown unit(s). Did you mean: {}?\nRun 'hematite --convert list' for all supported units.", close.join(", ")))
    }
}

fn norm_unit(s: &str) -> String {
    s.trim().to_lowercase()
        .replace("°", "")
        .replace("²", "2")
        .replace("³", "3")
        .replace("/s", "_per_s")
        .replace("per second", "_per_s")
}

fn convert_temperature(value: f64, from: &str, to: &str) -> f64 {
    // Convert to Kelvin first
    let kelvin = match from {
        "c" | "celsius"     => value + 273.15,
        "f" | "fahrenheit"  => (value - 32.0) * 5.0 / 9.0 + 273.15,
        "k" | "kelvin"      => value,
        "r" | "rankine"     => value * 5.0 / 9.0,
        _                   => value,
    };
    match to {
        "c" | "celsius"     => kelvin - 273.15,
        "f" | "fahrenheit"  => (kelvin - 273.15) * 9.0 / 5.0 + 32.0,
        "k" | "kelvin"      => kelvin,
        "r" | "rankine"     => kelvin * 9.0 / 5.0,
        _                   => kelvin,
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i-1] == b[j-1] {
                dp[i-1][j-1]
            } else {
                1 + dp[i-1][j].min(dp[i][j-1]).min(dp[i-1][j-1])
            };
        }
    }
    dp[m][n]
}

// Each entry: (&[unit_aliases], factor_to_SI_base)
// Temperature is handled separately (non-linear)
type UnitEntry = (&'static [&'static str], f64);
type UnitCategory = (&'static str, &'static [UnitEntry]);

static UNIT_CATEGORIES: &[UnitCategory] = &[
    ("Length", &[
        (&["m", "meter", "meters", "metre", "metres"],                   1.0),
        (&["km", "kilometer", "kilometers", "kilometre", "kilometres"],   1000.0),
        (&["cm", "centimeter", "centimeters", "centimetre", "centimetres"], 0.01),
        (&["mm", "millimeter", "millimeters", "millimetre", "millimetres"], 0.001),
        (&["um", "micrometer", "micrometers", "micron", "microns"],       1e-6),
        (&["nm", "nanometer", "nanometers", "nanometre", "nanometres"],   1e-9),
        (&["mi", "mile", "miles"],                                        1609.344),
        (&["yd", "yard", "yards"],                                        0.9144),
        (&["ft", "foot", "feet"],                                         0.3048),
        (&["in", "inch", "inches"],                                       0.0254),
        (&["nmi", "nautical_mile", "nautical_miles"],                     1852.0),
        (&["ly", "light_year", "light_years", "lightyear", "lightyears"], 9.460730472580800e15),
        (&["au", "astronomical_unit", "astronomical_units"],              1.495978707e11),
        (&["pc", "parsec", "parsecs"],                                    3.085677581e16),
        (&["ang", "angstrom", "angstroms"],                               1e-10),
    ]),
    ("Mass", &[
        (&["kg", "kilogram", "kilograms", "kilogramme"],                  1.0),
        (&["g", "gram", "grams", "gramme"],                               0.001),
        (&["mg", "milligram", "milligrams", "milligramme"],               1e-6),
        (&["ug", "microgram", "micrograms"],                              1e-9),
        (&["t", "tonne", "tonnes", "metric_ton", "metric_tons"],          1000.0),
        (&["lb", "lbs", "pound", "pounds"],                               0.45359237),
        (&["oz", "ounce", "ounces"],                                      0.028349523125),
        (&["st", "stone", "stones"],                                      6.35029318),
        (&["ton", "short_ton", "short_tons"],                             907.18474),
        (&["long_ton", "long_tons"],                                      1016.0469088),
        (&["gr", "grain", "grains"],                                      6.479891e-5),
        (&["u", "amu", "dalton", "daltons", "da"],                        1.66053906660e-27),
    ]),
    ("Temperature", &[
        (&["c", "celsius"],                                               1.0),  // factors unused
        (&["f", "fahrenheit"],                                            1.0),
        (&["k", "kelvin"],                                                1.0),
        (&["r", "rankine"],                                               1.0),
    ]),
    ("Time", &[
        (&["s", "sec", "second", "seconds"],                              1.0),
        (&["ms", "millisecond", "milliseconds"],                          0.001),
        (&["us", "microsecond", "microseconds"],                          1e-6),
        (&["ns", "nanosecond", "nanoseconds"],                            1e-9),
        (&["min", "minute", "minutes"],                                   60.0),
        (&["h", "hr", "hour", "hours"],                                   3600.0),
        (&["d", "day", "days"],                                           86400.0),
        (&["wk", "week", "weeks"],                                        604800.0),
        (&["mo", "month", "months"],                                      2629800.0),
        (&["yr", "year", "years"],                                        31557600.0),
    ]),
    ("Area", &[
        (&["m2", "sqm", "square_meter", "square_meters", "square_metre"],  1.0),
        (&["km2", "sqkm", "square_kilometer", "square_km"],                1e6),
        (&["cm2", "sqcm", "square_centimeter", "square_centimeters"],      1e-4),
        (&["mm2", "sqmm", "square_millimeter", "square_millimeters"],      1e-6),
        (&["ha", "hectare", "hectares"],                                    1e4),
        (&["ac", "acre", "acres"],                                          4046.8564224),
        (&["sqft", "sq_ft", "square_foot", "square_feet"],                 0.09290304),
        (&["sqin", "sq_in", "square_inch", "square_inches"],               6.4516e-4),
        (&["sqmi", "sq_mi", "square_mile", "square_miles"],                2589988.110336),
        (&["sqyd", "sq_yd", "square_yard", "square_yards"],                0.83612736),
    ]),
    ("Volume", &[
        (&["m3", "cubic_meter", "cubic_meters", "cubic_metre"],            1.0),
        (&["l", "liter", "liters", "litre", "litres"],                     0.001),
        (&["ml", "milliliter", "milliliters", "millilitre"],               1e-6),
        (&["cl", "centiliter", "centiliters"],                             1e-5),
        (&["dl", "deciliter", "deciliters"],                               1e-4),
        (&["ul", "microliter", "microliters"],                             1e-9),
        (&["cm3", "cc", "cubic_centimeter", "cubic_centimeters"],          1e-6),
        (&["mm3", "cubic_millimeter", "cubic_millimeters"],                1e-9),
        (&["km3", "cubic_kilometer", "cubic_kilometers"],                  1e9),
        (&["ft3", "cubic_foot", "cubic_feet"],                             0.0283168466),
        (&["in3", "cubic_inch", "cubic_inches"],                           1.6387064e-5),
        (&["yd3", "cubic_yard", "cubic_yards"],                            0.764554858),
        (&["gal", "gallon", "gallons"],                                    0.003785411784),
        (&["qt", "quart", "quarts"],                                       9.46352946e-4),
        (&["pt", "pint", "pints"],                                         4.73176473e-4),
        (&["cup", "cups"],                                                 2.36588237e-4),
        (&["floz", "fl_oz", "fluid_ounce", "fluid_ounces"],                2.95735296e-5),
        (&["tbsp", "tablespoon", "tablespoons"],                           1.47867648e-5),
        (&["tsp", "teaspoon", "teaspoons"],                                4.92892159e-6),
        (&["bbl", "barrel", "barrels"],                                    0.158987295),
        (&["gal_uk", "uk_gallon", "imperial_gallon", "imperial_gallons"],  0.00454609),
    ]),
    ("Speed", &[
        (&["m_per_s", "m/s", "mps"],                                       1.0),
        (&["km_per_s", "km/s", "kmps"],                                    1000.0),
        (&["km/h", "kmh", "kph", "km_per_h", "km_per_hour"],              1.0/3.6),
        (&["mph", "mi/h", "mi_per_h", "miles_per_hour"],                   0.44704),
        (&["knot", "knots", "kn"],                                          0.514444),
        (&["ft_per_s", "ft/s", "fps"],                                      0.3048),
        (&["c_speed", "speed_of_light"],                                    299792458.0),
        (&["mach"],                                                          340.29),
    ]),
    ("Force", &[
        (&["n", "newton", "newtons"],                                       1.0),
        (&["kn", "kilonewton", "kilonewtons"],                              1000.0),
        (&["mn", "meganewton", "meganewtons"],                              1e6),
        (&["lbf", "pound_force", "pound-force"],                            4.44822162),
        (&["kgf", "kilogram_force", "kilogram-force"],                      9.80665),
        (&["dyn", "dyne", "dynes"],                                         1e-5),
        (&["ozf", "ounce_force"],                                           0.278013851),
    ]),
    ("Pressure", &[
        (&["pa", "pascal", "pascals"],                                      1.0),
        (&["kpa", "kilopascal", "kilopascals"],                             1000.0),
        (&["mpa", "megapascal", "megapascals"],                             1e6),
        (&["gpa", "gigapascal", "gigapascals"],                             1e9),
        (&["hpa", "hectopascal", "hectopascals", "mbar", "millibar", "millibars"], 100.0),
        (&["bar", "bars"],                                                  1e5),
        (&["atm", "atmosphere", "atmospheres"],                             101325.0),
        (&["torr"],                                                         133.322368),
        (&["mmhg", "mm_hg", "millimeter_of_mercury"],                       133.322368),
        (&["psi", "pound_per_square_inch"],                                 6894.75729),
        (&["inhg", "in_hg", "inch_of_mercury"],                             3386.389),
    ]),
    ("Energy", &[
        (&["j", "joule", "joules"],                                         1.0),
        (&["kj", "kilojoule", "kilojoules"],                                1000.0),
        (&["mj", "megajoule", "megajoules"],                                1e6),
        (&["gj", "gigajoule", "gigajoules"],                                1e9),
        (&["cal", "calorie", "calories", "thermochemical_calorie"],         4.184),
        (&["kcal", "kilocalorie", "kilocalories", "food_calorie"],          4184.0),
        (&["wh", "watt_hour", "watt_hours"],                                3600.0),
        (&["kwh", "kilowatt_hour", "kilowatt_hours"],                       3.6e6),
        (&["mwh", "megawatt_hour", "megawatt_hours"],                       3.6e9),
        (&["ev", "electronvolt", "electronvolts"],                          1.602176634e-19),
        (&["kev", "kiloelectronvolt", "kiloelectronvolts"],                 1.602176634e-16),
        (&["mev", "megaelectronvolt", "megaelectronvolts"],                 1.602176634e-13),
        (&["gev", "gigaelectronvolt", "gigaelectronvolts"],                 1.602176634e-10),
        (&["tev", "teraelectronvolt", "teraelectronvolts"],                 1.602176634e-7),
        (&["btu", "british_thermal_unit"],                                  1055.05585),
        (&["erg", "ergs"],                                                  1e-7),
        (&["ft_lb", "foot_pound", "foot_pounds"],                           1.35581795),
        (&["therm", "therms"],                                              1.05480400e8),
    ]),
    ("Power", &[
        (&["w", "watt", "watts"],                                           1.0),
        (&["kw", "kilowatt", "kilowatts"],                                  1000.0),
        (&["mw", "megawatt", "megawatts"],                                  1e6),
        (&["gw", "gigawatt", "gigawatts"],                                  1e9),
        (&["tw", "terawatt", "terawatts"],                                  1e12),
        (&["mw_milli", "milliwatt", "milliwatts"],                          0.001),
        (&["hp", "horsepower"],                                             745.69987),
        (&["ps", "metric_horsepower"],                                      735.49875),
        (&["btu_h", "btu/h", "btu_per_hour"],                              0.29307107),
        (&["erg_s", "erg/s", "erg_per_second"],                            1e-7),
        (&["ft_lb_s", "ft_lb/s"],                                          1.35581795),
    ]),
    ("Data", &[
        (&["bit", "bits"],                                                   1.0),
        (&["byte", "bytes", "b"],                                            8.0),
        (&["kb", "kilobit", "kilobits"],                                    1e3),
        (&["kib", "kibibit", "kibibits"],                                   1024.0),
        (&["mb", "megabit", "megabits"],                                    1e6),
        (&["mib", "mebibit", "mebibits"],                                   1024.0*1024.0),
        (&["gb", "gigabit", "gigabits"],                                    1e9),
        (&["gib", "gibibit", "gibibits"],                                   1024.0*1024.0*1024.0),
        (&["tb", "terabit", "terabits"],                                    1e12),
        (&["tib", "tebibit", "tebibits"],                                   1024.0*1024.0*1024.0*1024.0),
        (&["pb", "petabit", "petabits"],                                    1e15),
        (&["kb_byte", "kilobyte", "kilobytes"],                             8e3),
        (&["kib_byte", "kibibyte", "kibibytes"],                            8.0*1024.0),
        (&["mb_byte", "megabyte", "megabytes"],                             8e6),
        (&["mib_byte", "mebibyte", "mebibytes"],                            8.0*1024.0*1024.0),
        (&["gb_byte", "gigabyte", "gigabytes"],                             8e9),
        (&["gib_byte", "gibibyte", "gibibytes"],                            8.0*1024.0*1024.0*1024.0),
        (&["tb_byte", "terabyte", "terabytes"],                             8e12),
        (&["tib_byte", "tebibyte", "tebibytes"],                            8.0*1024.0*1024.0*1024.0*1024.0),
    ]),
    ("Angle", &[
        (&["rad", "radian", "radians"],                                      1.0),
        (&["deg", "degree", "degrees"],                                      std::f64::consts::PI / 180.0),
        (&["grad", "gradian", "gradians", "gon", "gons"],                   std::f64::consts::PI / 200.0),
        (&["arcmin", "arc_minute", "arc_minutes", "minute_of_arc"],         std::f64::consts::PI / 10800.0),
        (&["arcsec", "arc_second", "arc_seconds", "second_of_arc"],         std::f64::consts::PI / 648000.0),
        (&["rev", "revolution", "revolutions", "turn", "turns"],            2.0 * std::f64::consts::PI),
    ]),
    ("Frequency", &[
        (&["hz", "hertz"],                                                   1.0),
        (&["khz", "kilohertz"],                                              1e3),
        (&["mhz", "megahertz"],                                              1e6),
        (&["ghz", "gigahertz"],                                              1e9),
        (&["thz", "terahertz"],                                              1e12),
        (&["rpm", "revolutions_per_minute"],                                 1.0/60.0),
        (&["rad_per_s", "rad/s"],                                            1.0/(2.0*std::f64::consts::PI)),
    ]),
    ("Illuminance", &[
        (&["lx", "lux"],                                                     1.0),
        (&["fc", "footcandle", "footcandles"],                               10.7639),
        (&["phot", "phots"],                                                 1e4),
    ]),
    ("Fuel Economy", &[
        (&["mpg", "miles_per_gallon"],                                       1.0),
        (&["mpg_uk", "miles_per_gallon_uk", "imperial_mpg"],                 1.20095),
        (&["l_per_100km", "l/100km", "liters_per_100km"],                    235.214583),
        (&["km_per_l", "km/l", "km_per_liter"],                              2.35214583),
    ]),
];

fn unit_convert_list() -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Supported unit categories and aliases:");
    let _ = writeln!(out, "Usage: hematite --convert '<value> <from> to <to>'");
    let _ = writeln!(out);
    for (cat, units) in UNIT_CATEGORIES {
        let _ = writeln!(out, "  {}:", cat);
        for (names, _) in *units {
            let _ = writeln!(out, "    {}", names.join(", "));
        }
        let _ = writeln!(out);
    }
    out
}

// ── Vector / linear-algebra calculator ───────────────────────────────────────
// Pure-Rust — no sandbox, instant results.
// Supports 2D and 3D vectors, arbitrary-dimension for dot/mag/normalize.
//
// Query forms:
//   "[1,2,3] dot [4,5,6]"
//   "[1,2,3] cross [4,5,6]"
//   "[1,2,3] + [4,5,6]"    (add)
//   "[1,2,3] - [4,5,6]"    (subtract)
//   "3 * [1,2,3]"          (scalar multiply)
//   "mag [3,4]"            (magnitude)
//   "norm [1,2,3]"         (normalize)
//   "angle [1,0] [0,1]"    (angle in degrees)
//   "proj [1,2] onto [3,4]" (vector projection)
//   "[1,2,3]"              (info about a single vector)

pub fn vector_calc(query: &str) -> String {
    let q = query.trim();

    // ── unary ops ────────────────────────────────────────────────────────────
    if let Some(rest) = strip_prefix_ci(q, "mag") {
        if let Some(v) = parse_vec(rest.trim()) {
            return format_vec_result("Magnitude", &[], vec_mag(&v));
        }
    }
    if let Some(rest) = strip_prefix_ci(q, "magnitude") {
        if let Some(v) = parse_vec(rest.trim()) {
            return format_vec_result("Magnitude", &[], vec_mag(&v));
        }
    }
    if let Some(rest) = strip_prefix_ci(q, "norm") {
        if let Some(v) = parse_vec(rest.trim()) {
            let mag = vec_mag(&v);
            if mag == 0.0 { return "Zero vector has no unit direction.".into(); }
            let n: Vec<f64> = v.iter().map(|x| x / mag).collect();
            return format_vec_display("Unit vector (normalized)", &n);
        }
    }
    if let Some(rest) = strip_prefix_ci(q, "normalize") {
        if let Some(v) = parse_vec(rest.trim()) {
            let mag = vec_mag(&v);
            if mag == 0.0 { return "Zero vector has no unit direction.".into(); }
            let n: Vec<f64> = v.iter().map(|x| x / mag).collect();
            return format_vec_display("Unit vector (normalized)", &n);
        }
    }

    // ── angle between two vectors ─────────────────────────────────────────────
    if let Some(rest) = strip_prefix_ci(q, "angle") {
        let vecs = find_all_vecs(rest.trim());
        if vecs.len() >= 2 {
            let a = &vecs[0]; let b = &vecs[1];
            if a.len() != b.len() { return "Vectors must have the same dimension for angle.".into(); }
            let dot = vec_dot(a, b);
            let ma = vec_mag(a); let mb = vec_mag(b);
            if ma == 0.0 || mb == 0.0 { return "Cannot compute angle involving a zero vector.".into(); }
            let cos_theta = (dot / (ma * mb)).clamp(-1.0, 1.0);
            let deg = cos_theta.acos().to_degrees();
            let rad = cos_theta.acos();
            return format!(
                "Angle between {} and {}:\n  {:.6}°  ({:.6} radians)\n  cos θ = {:.6}",
                fmt_vec(a), fmt_vec(b), deg, rad, cos_theta
            );
        }
    }

    // ── projection ────────────────────────────────────────────────────────────
    if q.to_lowercase().contains("proj") && q.to_lowercase().contains("onto") {
        let vecs = find_all_vecs(q);
        if vecs.len() >= 2 {
            let a = &vecs[0]; let b = &vecs[1];
            if a.len() != b.len() { return "Vectors must have the same dimension for projection.".into(); }
            let b_mag2: f64 = b.iter().map(|x| x * x).sum();
            if b_mag2 == 0.0 { return "Cannot project onto a zero vector.".into(); }
            let scalar = vec_dot(a, b) / b_mag2;
            let proj: Vec<f64> = b.iter().map(|x| x * scalar).collect();
            let mut out = String::new();
            let _ = writeln!(out, "Projection of {} onto {}:", fmt_vec(a), fmt_vec(b));
            let _ = writeln!(out, "  proj = {}", fmt_vec(&proj));
            let _ = writeln!(out, "  scalar factor = {:.6}", scalar);
            return out;
        }
    }

    // ── binary ops: look for keyword between two vectors ──────────────────────
    let lower = q.to_lowercase();

    // dot product
    if lower.contains(" dot ") {
        if let Some(idx) = lower.find(" dot ") {
            let left = &q[..idx]; let right = &q[idx+5..];
            if let (Some(a), Some(b)) = (parse_vec(left.trim()), parse_vec(right.trim())) {
                if a.len() != b.len() { return format!("Dimension mismatch: {} vs {}", a.len(), b.len()); }
                let d = vec_dot(&a, &b);
                return format!("{} · {} = {}", fmt_vec(&a), fmt_vec(&b), fmt_scalar(d));
            }
        }
    }

    // cross product
    if lower.contains(" cross ") {
        if let Some(idx) = lower.find(" cross ") {
            let left = &q[..idx]; let right = &q[idx+7..];
            if let (Some(a), Some(b)) = (parse_vec(left.trim()), parse_vec(right.trim())) {
                if a.len() != 3 || b.len() != 3 {
                    return "Cross product requires two 3D vectors.".into();
                }
                let c = vec_cross(&a, &b);
                return format!(
                    "{} × {} = {}\n  |result| = {}",
                    fmt_vec(&a), fmt_vec(&b), fmt_vec(&c), fmt_scalar(vec_mag(&c))
                );
            }
        }
    }

    // scalar × vector: "3 * [1,2,3]" or "[1,2,3] * 3"
    if lower.contains(" * ") {
        if let Some(idx) = q.find(" * ") {
            let left = q[..idx].trim(); let right = q[idx+3..].trim();
            // scalar * vec
            if let (Ok(s), Some(v)) = (left.parse::<f64>(), parse_vec(right)) {
                let result: Vec<f64> = v.iter().map(|x| x * s).collect();
                return format!("{} × {} = {}", s, fmt_vec(&v), fmt_vec(&result));
            }
            // vec * scalar
            if let (Some(v), Ok(s)) = (parse_vec(left), right.parse::<f64>()) {
                let result: Vec<f64> = v.iter().map(|x| x * s).collect();
                return format!("{} × {} = {}", fmt_vec(&v), s, fmt_vec(&result));
            }
        }
    }

    // vector + vector
    if let Some(idx) = q.find(" + ") {
        let left = q[..idx].trim(); let right = q[idx+3..].trim();
        if let (Some(a), Some(b)) = (parse_vec(left), parse_vec(right)) {
            if a.len() != b.len() { return format!("Dimension mismatch: {} vs {}", a.len(), b.len()); }
            let c: Vec<f64> = a.iter().zip(b.iter()).map(|(x,y)| x + y).collect();
            return format!("{} + {} = {}", fmt_vec(&a), fmt_vec(&b), fmt_vec(&c));
        }
    }

    // vector - vector
    if let Some(idx) = q.rfind(" - ") {
        let left = q[..idx].trim(); let right = q[idx+3..].trim();
        if let (Some(a), Some(b)) = (parse_vec(left), parse_vec(right)) {
            if a.len() != b.len() { return format!("Dimension mismatch: {} vs {}", a.len(), b.len()); }
            let c: Vec<f64> = a.iter().zip(b.iter()).map(|(x,y)| x - y).collect();
            return format!("{} - {} = {}", fmt_vec(&a), fmt_vec(&b), fmt_vec(&c));
        }
    }

    // single vector — info card
    if let Some(v) = parse_vec(q) {
        let mut out = String::new();
        let mag = vec_mag(&v);
        let _ = writeln!(out, "Vector:    {}", fmt_vec(&v));
        let _ = writeln!(out, "Dimension: {}", v.len());
        let _ = writeln!(out, "Magnitude: {}", fmt_scalar(mag));
        if mag > 0.0 {
            let unit: Vec<f64> = v.iter().map(|x| x / mag).collect();
            let _ = writeln!(out, "Unit vec:  {}", fmt_vec(&unit));
        }
        if v.len() == 2 {
            let angle = v[1].atan2(v[0]).to_degrees();
            let _ = writeln!(out, "Angle (from +x): {:.4}°", angle);
        }
        return out;
    }

    format!(
        "Could not parse: '{}'\n\
         Examples:\n\
           hematite --vectors '[1,2,3] dot [4,5,6]'\n\
           hematite --vectors '[1,2,3] cross [4,5,6]'\n\
           hematite --vectors '[1,2,3] + [4,5,6]'\n\
           hematite --vectors 'mag [3,4]'\n\
           hematite --vectors 'norm [1,2,3]'\n\
           hematite --vectors 'angle [1,0] [0,1]'\n\
           hematite --vectors 'proj [1,2] onto [3,4]'\n\
           hematite --vectors '3 * [1,2,3]'",
        q
    )
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

fn parse_vec(s: &str) -> Option<Vec<f64>> {
    // Accepts: [1,2,3]  (1,2,3)  1,2,3  1 2 3
    let s = s.trim().trim_start_matches(['[', '(']).trim_end_matches([']', ')']);
    let parts: Vec<&str> = if s.contains(',') {
        s.split(',').collect()
    } else {
        s.split_whitespace().collect()
    };
    if parts.is_empty() { return None; }
    let nums: Vec<f64> = parts.iter()
        .filter_map(|p| p.trim().parse::<f64>().ok())
        .collect();
    if nums.len() == parts.len() && !nums.is_empty() { Some(nums) } else { None }
}

fn find_all_vecs(s: &str) -> Vec<Vec<f64>> {
    // Find all bracket-delimited vectors in a string
    let mut result = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = s.chars().collect();
    while i < chars.len() {
        if chars[i] == '[' || chars[i] == '(' {
            let close = if chars[i] == '[' { ']' } else { ')' };
            if let Some(j) = chars[i+1..].iter().position(|&c| c == close) {
                let inner: String = chars[i+1..i+1+j].iter().collect();
                if let Some(v) = parse_vec(&inner) {
                    result.push(v);
                }
                i += j + 2;
                continue;
            }
        }
        i += 1;
    }
    result
}

fn vec_dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn vec_mag(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

fn vec_cross(a: &[f64], b: &[f64]) -> Vec<f64> {
    vec![
        a[1]*b[2] - a[2]*b[1],
        a[2]*b[0] - a[0]*b[2],
        a[0]*b[1] - a[1]*b[0],
    ]
}

fn fmt_vec(v: &[f64]) -> String {
    let inner: Vec<String> = v.iter().map(|x| fmt_scalar(*x)).collect();
    format!("[{}]", inner.join(", "))
}

fn fmt_scalar(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e12 {
        format!("{}", x as i64)
    } else if x.abs() >= 1e-3 && x.abs() < 1e7 {
        format!("{:.6}", x).trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        format!("{:.6e}", x)
    }
}

fn format_vec_result(label: &str, _v: &[f64], val: f64) -> String {
    format!("{}: {}", label, fmt_scalar(val))
}

fn format_vec_display(label: &str, v: &[f64]) -> String {
    format!("{}: {}", label, fmt_vec(v))
}

// ── Monte Carlo simulation ────────────────────────────────────────────────────
// Pure Rust — no Python sandbox, instant results.
// Query forms:
//   "pi N"             — estimate π by random darts (N trials, default 1e6)
//   "dice NdM [+K] R"  — roll N d-M dice R times, show distribution
//   "birthday N"       — birthday problem: probability ≥2 share a birthday in room of N
//   "ruin P A B N"     — gambler's ruin: win prob P, start $A, goal $B, N simulations
//   "ci N MEAN STD"    — 95%/99% confidence interval via bootstrap-style simulation
//   "walk N STEPS"     — N random walks of STEPS steps, report stats

pub fn simulate(query: &str) -> String {
    let q = query.trim();
    let tokens: Vec<&str> = q.split_whitespace().collect();
    if tokens.is_empty() {
        return simulate_usage();
    }

    match tokens[0].to_lowercase().as_str() {
        "pi" => {
            let n: u64 = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(1_000_000);
            let n = n.min(100_000_000);
            let mut inside = 0u64;
            let mut rng = Lcg64::new(0xdeadbeef_12345678);
            for _ in 0..n {
                let x = rng.next_f64() * 2.0 - 1.0;
                let y = rng.next_f64() * 2.0 - 1.0;
                if x*x + y*y <= 1.0 { inside += 1; }
            }
            let pi_est = 4.0 * inside as f64 / n as f64;
            let error = (pi_est - std::f64::consts::PI).abs();
            format!(
                "Monte Carlo π estimate ({} trials):\n  π ≈ {:.8}\n  True π = {:.8}\n  Error: {:.6e}\n  Inside circle: {} / {}",
                n, pi_est, std::f64::consts::PI, error, inside, n
            )
        }
        "birthday" => {
            let n: u32 = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(23);
            // Exact probability via inclusion-exclusion
            let p_no_match = (0..n as u64).fold(1.0f64, |acc, i| acc * (365 - i) as f64 / 365.0);
            let p_match = 1.0 - p_no_match;
            let mut out = format!("Birthday problem — room of {} people:\n", n);
            out.push_str(&format!("  P(at least 2 share a birthday) = {:.6} ({:.2}%)\n", p_match, p_match*100.0));
            out.push_str(&format!("  P(all different birthdays)      = {:.6} ({:.2}%)\n", p_no_match, p_no_match*100.0));
            // Find 50% threshold
            let n50 = (1..366u32).find(|&k| {
                let p = 1.0 - (0..k as u64).fold(1.0f64, |a,i| a*(365-i) as f64/365.0);
                p >= 0.5
            }).unwrap_or(23);
            out.push_str(&format!("  Minimum group for ≥50% chance: {} people\n", n50));
            out
        }
        "dice" => {
            // dice 2d6 1000   or   dice 1d20+3 500
            let spec = tokens.get(1).copied().unwrap_or("1d6");
            let rolls: u64 = tokens.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
            let rolls = rolls.min(1_000_000);
            // Parse NdM+K
            let (n_dice, sides, bonus) = parse_dice_spec(spec);
            let mut counts: std::collections::HashMap<i64, u64> = std::collections::HashMap::new();
            let mut rng = Lcg64::new(0xcafe_babe_dead_beef);
            for _ in 0..rolls {
                let total: i64 = (0..n_dice).map(|_| (rng.next_u64() % sides as u64) as i64 + 1).sum::<i64>() + bonus;
                *counts.entry(total).or_insert(0) += 1;
            }
            let mut sorted_keys: Vec<i64> = counts.keys().copied().collect();
            sorted_keys.sort();
            let mean: f64 = sorted_keys.iter().map(|&k| k as f64 * counts[&k] as f64).sum::<f64>() / rolls as f64;
            let mut out = format!("Dice simulation: {} × {} rolls\n", rolls, spec);
            let _ = write!(out, "  Mean: {:.3}   Range: {}–{}\n", mean, sorted_keys.first().unwrap_or(&0), sorted_keys.last().unwrap_or(&0));
            out.push_str("  Distribution:\n");
            let max_count = counts.values().copied().max().unwrap_or(1);
            for k in &sorted_keys {
                let c = counts[k];
                let pct = 100.0 * c as f64 / rolls as f64;
                let bar_len = (c as f64 / max_count as f64 * 30.0) as usize;
                let _ = write!(out, "    {:4}  {:6.2}%  {}\n", k, pct, "█".repeat(bar_len));
            }
            out
        }
        "ruin" | "gambler" => {
            let p: f64   = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.5);
            let a: i64   = tokens.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            let b: i64   = tokens.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);
            let n: u64   = tokens.get(4).and_then(|s| s.parse().ok()).unwrap_or(10_000);
            let n = n.min(100_000);
            if a <= 0 || b <= a { return "Usage: ruin PROB START GOAL N_SIM (GOAL > START > 0)".into(); }

            let mut wins = 0u64;
            let mut steps_total = 0u64;
            let mut rng = Lcg64::new(0x1234_5678_9abc_def0);
            for _ in 0..n {
                let mut money = a;
                let mut steps = 0u64;
                while money > 0 && money < b {
                    let r = rng.next_f64();
                    money += if r < p { 1 } else { -1 };
                    steps += 1;
                    if steps > 100_000 { break; }
                }
                if money >= b { wins += 1; }
                steps_total += steps;
            }
            let win_rate = wins as f64 / n as f64;
            let avg_steps = steps_total as f64 / n as f64;
            // Exact formula for fair/unfair game
            let exact = if (p - 0.5).abs() < 1e-10 {
                a as f64 / b as f64
            } else {
                let q = 1.0 - p;
                let r = q / p;
                (1.0 - r.powi(a as i32)) / (1.0 - r.powi(b as i32))
            };
            format!(
                "Gambler's Ruin ({} simulations):\n  Win prob p={:.4}  Start=${} → Goal=${}\n\
                 \n  Simulated win rate:  {:.4} ({:.2}%)\n  Exact formula:       {:.4} ({:.2}%)\n\
                 \n  Average steps to finish: {:.1}",
                n, p, a, b, win_rate, win_rate*100.0, exact, exact*100.0, avg_steps
            )
        }
        "walk" | "random_walk" => {
            let n_walks: u64 = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000);
            let steps: u64   = tokens.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);
            let n_walks = n_walks.min(100_000);
            let steps = steps.min(100_000);
            let mut final_positions: Vec<f64> = Vec::with_capacity(n_walks as usize);
            let mut max_deviation: f64 = 0.0;
            let mut rng = Lcg64::new(0xabcdef01_23456789);
            for _ in 0..n_walks {
                let mut pos = 0.0f64;
                for _ in 0..steps {
                    pos += if rng.next_f64() < 0.5 { 1.0 } else { -1.0 };
                }
                final_positions.push(pos);
                if pos.abs() > max_deviation { max_deviation = pos.abs(); }
            }
            let mean = final_positions.iter().sum::<f64>() / n_walks as f64;
            let variance: f64 = final_positions.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n_walks as f64;
            let std_dev = variance.sqrt();
            let theoretical_std = (steps as f64).sqrt();
            format!(
                "Random Walk simulation ({} walks × {} steps):\n  Mean final position: {:.4}\n  Std deviation:       {:.4}  (theoretical √N = {:.4})\n  Max |deviation|:     {:.0}\n  Expected: walk ends within ±{:.1} of origin with 95% probability",
                n_walks, steps, mean, std_dev, theoretical_std, max_deviation, 1.96 * theoretical_std
            )
        }
        _ => {
            // Try to parse as "pi N" with the number as first token
            if let Ok(n) = tokens[0].parse::<u64>() {
                // Assume it's an N for pi estimation
                return simulate(&format!("pi {}", n));
            }
            simulate_usage()
        }
    }
}

fn simulate_usage() -> String {
    "Monte Carlo simulation:\n\
     hematite --simulate 'pi 1000000'           estimate π with N darts\n\
     hematite --simulate 'birthday 23'          birthday problem\n\
     hematite --simulate 'dice 2d6 10000'       roll 2d6 × 10000\n\
     hematite --simulate 'ruin 0.48 10 20 5000' gambler's ruin\n\
     hematite --simulate 'walk 1000 200'        random walk simulation".into()
}

fn parse_dice_spec(spec: &str) -> (i64, i64, i64) {
    // NdM+K or NdM-K
    let lower = spec.to_lowercase();
    let (dice_part, bonus) = if let Some(idx) = lower.rfind('+') {
        let b: i64 = spec[idx+1..].parse().unwrap_or(0);
        (&spec[..idx], b)
    } else if let Some(idx) = lower[1..].rfind('-').map(|i| i+1) {
        let b: i64 = spec[idx+1..].parse().unwrap_or(0);
        (&spec[..idx], -b)
    } else {
        (spec, 0i64)
    };
    if let Some(d_pos) = dice_part.to_lowercase().find('d') {
        let n: i64 = dice_part[..d_pos].parse().unwrap_or(1).max(1);
        let s: i64 = dice_part[d_pos+1..].parse().unwrap_or(6).max(2);
        (n, s, bonus)
    } else {
        (1, 6, 0)
    }
}

// Minimal 64-bit LCG PRNG — no stdlib rng needed
struct Lcg64 { state: u64 }
impl Lcg64 {
    fn new(seed: u64) -> Self { Self { state: seed.wrapping_add(1) } }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.state
    }
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ── Propositional logic / Boolean algebra ────────────────────────────────────
// Parse Boolean expression → truth table, CNF/DNF, SAT/TAUT check, simplify.
// Operators: AND (&& / & / and / *)  OR (|| / | / or / +)
//            NOT (! / ~ / not)  XOR (^ / xor)  NAND XNOR NOR
//            IMPLIES (-> / => / implies)  IFF (<-> / <=> / iff)
//
// Modes (first token):
//   table EXPR         truth table
//   sat   EXPR         satisfiability check
//   taut  EXPR         tautology check
//   cnf   EXPR         conjunctive normal form
//   dnf   EXPR         disjunctive normal form
//   equiv EXPR1 ; EXPR2  check logical equivalence
//   simplify EXPR      rule-based simplification
//   (default)          table + sat + taut

// ── Boolean expression AST ────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum BExpr {
    Var(String),
    Not(Box<BExpr>),
    And(Box<BExpr>, Box<BExpr>),
    Or(Box<BExpr>, Box<BExpr>),
    Xor(Box<BExpr>, Box<BExpr>),
    Implies(Box<BExpr>, Box<BExpr>),
    Iff(Box<BExpr>, Box<BExpr>),
    Nand(Box<BExpr>, Box<BExpr>),
    Nor(Box<BExpr>, Box<BExpr>),
    Xnor(Box<BExpr>, Box<BExpr>),
    Const(bool),
}

struct BParser<'a> {
    chars: &'a [char],
    pos: usize,
}

impl<'a> BParser<'a> {
    fn new(chars: &'a [char]) -> Self { Self { chars, pos: 0 } }
    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn consume(&mut self) -> Option<char> { let c = self.peek(); self.pos += 1; c }
    fn skip_ws(&mut self) { while matches!(self.peek(), Some(' ') | Some('\t')) { self.pos += 1; } }

    fn parse_iff(&mut self) -> Result<BExpr, String> {
        let mut left = self.parse_implies()?;
        loop {
            self.skip_ws();
            if self.try_keyword("iff") || self.try_str("<->") || self.try_str("<=>") {
                let right = self.parse_implies()?;
                left = BExpr::Iff(Box::new(left), Box::new(right));
            } else { break; }
        }
        Ok(left)
    }

    fn parse_implies(&mut self) -> Result<BExpr, String> {
        let left = self.parse_or()?;
        self.skip_ws();
        if self.try_str("->") || self.try_str("=>") || self.try_keyword("implies") {
            let right = self.parse_implies()?;
            return Ok(BExpr::Implies(Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<BExpr, String> {
        let mut left = self.parse_xor()?;
        loop {
            self.skip_ws();
            if self.try_str("||") || self.try_str("|") || self.try_keyword("or") || self.try_keyword("nor") {
                let is_nor = {
                    let prev_is_nor = self.chars.get(self.pos.saturating_sub(3))
                        .map(|c| *c == 'r').unwrap_or(false);
                    // Check if we consumed "nor"
                    false // simplification: treat nor separately
                };
                let right = self.parse_xor()?;
                left = BExpr::Or(Box::new(left), Box::new(right));
            } else { break; }
        }
        Ok(left)
    }

    fn parse_xor(&mut self) -> Result<BExpr, String> {
        let mut left = self.parse_and()?;
        loop {
            self.skip_ws();
            if self.try_keyword("xor") || self.try_keyword("xnor") {
                let right = self.parse_and()?;
                left = BExpr::Xor(Box::new(left), Box::new(right));
            } else if self.try_str("^") {
                let right = self.parse_and()?;
                left = BExpr::Xor(Box::new(left), Box::new(right));
            } else { break; }
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<BExpr, String> {
        let mut left = self.parse_not()?;
        loop {
            self.skip_ws();
            if self.try_str("&&") || self.try_str("&") || self.try_keyword("and") || self.try_keyword("nand") {
                let right = self.parse_not()?;
                left = BExpr::And(Box::new(left), Box::new(right));
            } else if self.try_str("*") {
                let right = self.parse_not()?;
                left = BExpr::And(Box::new(left), Box::new(right));
            } else { break; }
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<BExpr, String> {
        self.skip_ws();
        if self.peek() == Some('!') || self.peek() == Some('~') {
            self.consume();
            let inner = self.parse_not()?;
            return Ok(BExpr::Not(Box::new(inner)));
        }
        if self.try_keyword("not") {
            let inner = self.parse_not()?;
            return Ok(BExpr::Not(Box::new(inner)));
        }
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Result<BExpr, String> {
        self.skip_ws();
        if self.peek() == Some('(') {
            self.consume();
            let inner = self.parse_iff()?;
            self.skip_ws();
            if self.peek() == Some(')') { self.consume(); }
            return Ok(inner);
        }
        // Keyword literals
        if self.try_keyword("true") || self.try_keyword("1") { return Ok(BExpr::Const(true)); }
        if self.try_keyword("false") || self.try_keyword("0") { return Ok(BExpr::Const(false)); }
        // Variable name
        if matches!(self.peek(), Some(c) if c.is_alphabetic() || c == '_') {
            let start = self.pos;
            while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') { self.pos += 1; }
            let name: String = self.chars[start..self.pos].iter().collect();
            return Ok(BExpr::Var(name));
        }
        Err(format!("unexpected char '{}'", self.peek().map(|c| c.to_string()).unwrap_or("EOF".into())))
    }

    fn try_str(&mut self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        let remaining = &self.chars[self.pos..];
        if remaining.len() >= chars.len() && remaining[..chars.len()] == chars[..] {
            self.pos += chars.len();
            return true;
        }
        false
    }

    fn try_keyword(&mut self, kw: &str) -> bool {
        let saved = self.pos;
        self.skip_ws();
        let chars: Vec<char> = kw.chars().collect();
        let remaining = &self.chars[self.pos..];
        if remaining.len() >= chars.len()
            && remaining[..chars.len()].iter().map(|c| c.to_lowercase().next().unwrap()).collect::<Vec<_>>() == chars
            && !matches!(remaining.get(chars.len()), Some(c) if c.is_alphanumeric() || *c == '_')
        {
            self.pos += chars.len();
            return true;
        }
        self.pos = saved;
        false
    }
}

fn parse_bexpr(s: &str) -> Result<BExpr, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut p = BParser::new(&chars);
    let e = p.parse_iff()?;
    p.skip_ws();
    if p.pos < p.chars.len() {
        let rest: String = p.chars[p.pos..].iter().collect();
        if !rest.trim().is_empty() {
            return Err(format!("unexpected trailing: '{}'", rest.trim()));
        }
    }
    Ok(e)
}

// Collect variables in order of first appearance
fn collect_vars(e: &BExpr, vars: &mut Vec<String>) {
    match e {
        BExpr::Var(v) => { if !vars.contains(v) { vars.push(v.clone()); } }
        BExpr::Not(a) => collect_vars(a, vars),
        BExpr::And(a,b)|BExpr::Or(a,b)|BExpr::Xor(a,b)|BExpr::Implies(a,b)|BExpr::Iff(a,b)
        |BExpr::Nand(a,b)|BExpr::Nor(a,b)|BExpr::Xnor(a,b) => {
            collect_vars(a, vars); collect_vars(b, vars);
        }
        BExpr::Const(_) => {}
    }
}

fn eval_bexpr(e: &BExpr, assignment: &[(&str, bool)]) -> bool {
    match e {
        BExpr::Const(b) => *b,
        BExpr::Var(v) => assignment.iter().find(|(n,_)| n == v).map(|(_,b)| *b).unwrap_or(false),
        BExpr::Not(a) => !eval_bexpr(a, assignment),
        BExpr::And(a,b) => eval_bexpr(a, assignment) && eval_bexpr(b, assignment),
        BExpr::Or(a,b)  => eval_bexpr(a, assignment) || eval_bexpr(b, assignment),
        BExpr::Xor(a,b) => eval_bexpr(a, assignment) ^ eval_bexpr(b, assignment),
        BExpr::Xnor(a,b) => !(eval_bexpr(a, assignment) ^ eval_bexpr(b, assignment)),
        BExpr::Nand(a,b) => !(eval_bexpr(a, assignment) && eval_bexpr(b, assignment)),
        BExpr::Nor(a,b)  => !(eval_bexpr(a, assignment) || eval_bexpr(b, assignment)),
        BExpr::Implies(a,b) => !eval_bexpr(a, assignment) || eval_bexpr(b, assignment),
        BExpr::Iff(a,b)     => eval_bexpr(a, assignment) == eval_bexpr(b, assignment),
    }
}

fn bexpr_to_str(e: &BExpr) -> String {
    match e {
        BExpr::Const(true) => "true".into(),
        BExpr::Const(false) => "false".into(),
        BExpr::Var(v) => v.clone(),
        BExpr::Not(a) => format!("¬{}", bexpr_atom_str(a)),
        BExpr::And(a,b) => format!("({} ∧ {})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Or(a,b)  => format!("({} ∨ {})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Xor(a,b) => format!("({} ⊕ {})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Implies(a,b) => format!("({} → {})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Iff(a,b) => format!("({} ↔ {})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Nand(a,b) => format!("({}↑{})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Nor(a,b)  => format!("({}↓{})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Xnor(a,b) => format!("({}⊙{})", bexpr_to_str(a), bexpr_to_str(b)),
    }
}

fn bexpr_atom_str(e: &BExpr) -> String {
    match e {
        BExpr::Var(v) => v.clone(),
        BExpr::Const(b) => b.to_string(),
        _ => format!("({})", bexpr_to_str(e)),
    }
}

fn simplify_bexpr(e: BExpr) -> BExpr {
    match e {
        BExpr::Not(a) => {
            let a = simplify_bexpr(*a);
            match a {
                BExpr::Const(b) => BExpr::Const(!b),
                BExpr::Not(inner) => *inner,
                _ => BExpr::Not(Box::new(a)),
            }
        }
        BExpr::And(a, b) => {
            let a = simplify_bexpr(*a); let b = simplify_bexpr(*b);
            match (&a, &b) {
                (BExpr::Const(false), _) | (_, BExpr::Const(false)) => BExpr::Const(false),
                (BExpr::Const(true), _) => b,
                (_, BExpr::Const(true)) => a,
                _ if a == b => a,
                _ => BExpr::And(Box::new(a), Box::new(b)),
            }
        }
        BExpr::Or(a, b) => {
            let a = simplify_bexpr(*a); let b = simplify_bexpr(*b);
            match (&a, &b) {
                (BExpr::Const(true), _) | (_, BExpr::Const(true)) => BExpr::Const(true),
                (BExpr::Const(false), _) => b,
                (_, BExpr::Const(false)) => a,
                _ if a == b => a,
                _ => BExpr::Or(Box::new(a), Box::new(b)),
            }
        }
        BExpr::Xor(a, b) => {
            let a = simplify_bexpr(*a); let b = simplify_bexpr(*b);
            match (&a, &b) {
                (BExpr::Const(false), _) => b,
                (_, BExpr::Const(false)) => a,
                (BExpr::Const(true), _) => BExpr::Not(Box::new(b)),
                (_, BExpr::Const(true)) => BExpr::Not(Box::new(a)),
                _ if a == b => BExpr::Const(false),
                _ => BExpr::Xor(Box::new(a), Box::new(b)),
            }
        }
        BExpr::Implies(a, b) => {
            let a = simplify_bexpr(*a); let b = simplify_bexpr(*b);
            match (&a, &b) {
                (BExpr::Const(false), _) => BExpr::Const(true),
                (BExpr::Const(true), _) => b,
                (_, BExpr::Const(true)) => BExpr::Const(true),
                _ if a == b => BExpr::Const(true),
                _ => BExpr::Implies(Box::new(a), Box::new(b)),
            }
        }
        BExpr::Iff(a, b) => {
            let a = simplify_bexpr(*a); let b = simplify_bexpr(*b);
            match (&a, &b) {
                _ if a == b => BExpr::Const(true),
                _ => BExpr::Iff(Box::new(a), Box::new(b)),
            }
        }
        other => other,
    }
}

pub fn logic_calc(query: &str) -> String {
    let q = query.trim();
    let q_lower = q.to_lowercase();

    // Detect mode
    let (mode, expr_str, expr2_str) = if q_lower.starts_with("table ") || q_lower.starts_with("truth ") {
        ("table", q.splitn(2, ' ').nth(1).unwrap_or("").trim(), "")
    } else if q_lower.starts_with("sat ") {
        ("sat", q.splitn(2, ' ').nth(1).unwrap_or("").trim(), "")
    } else if q_lower.starts_with("taut ") {
        ("taut", q.splitn(2, ' ').nth(1).unwrap_or("").trim(), "")
    } else if q_lower.starts_with("cnf ") {
        ("cnf", q.splitn(2, ' ').nth(1).unwrap_or("").trim(), "")
    } else if q_lower.starts_with("dnf ") {
        ("dnf", q.splitn(2, ' ').nth(1).unwrap_or("").trim(), "")
    } else if q_lower.starts_with("simplify ") {
        ("simplify", q.splitn(2, ' ').nth(1).unwrap_or("").trim(), "")
    } else if q_lower.starts_with("equiv ") {
        let rest = q.splitn(2, ' ').nth(1).unwrap_or("").trim();
        if let Some(semi) = rest.find(';') {
            ("equiv", rest[..semi].trim(), rest[semi+1..].trim())
        } else {
            ("equiv", rest, "")
        }
    } else {
        ("info", q, "")
    };

    let mut out = String::new();
    let w = 64usize;
    let _ = writeln!(out, "{}", "=".repeat(w));

    let expr = match parse_bexpr(expr_str) {
        Ok(e) => e,
        Err(e) => {
            let _ = writeln!(out, "  Logic — parse error: {}", e);
            let _ = writeln!(out, "  Input: {}", expr_str.chars().take(60).collect::<String>());
            let _ = writeln!(out, "  Usage: hematite --logic 'A and (B or C)'");
            let _ = writeln!(out, "{}", "=".repeat(w));
            return out;
        }
    };

    let mut vars: Vec<String> = Vec::new();
    collect_vars(&expr, &mut vars);

    if vars.is_empty() {
        let result = eval_bexpr(&expr, &[]);
        let _ = writeln!(out, "  Logic  |  Constant expression: {}", result);
        let _ = writeln!(out, "{}", "=".repeat(w));
        return out;
    }

    if vars.len() > 20 {
        let _ = writeln!(out, "  Logic — too many variables ({}), max 20", vars.len());
        let _ = writeln!(out, "{}", "=".repeat(w));
        return out;
    }

    let n = vars.len();
    let rows = 1usize << n;

    // Evaluate all rows
    let results: Vec<bool> = (0..rows).map(|mask| {
        let assignment: Vec<(&str, bool)> = vars.iter().enumerate()
            .map(|(i, v)| (v.as_str(), (mask >> (n-1-i)) & 1 == 1))
            .collect();
        eval_bexpr(&expr, &assignment)
    }).collect();

    let sat_count = results.iter().filter(|&&b| b).count();
    let is_taut = sat_count == rows;
    let is_sat  = sat_count > 0;
    let is_contra = sat_count == 0;

    let _ = writeln!(out, "  Boolean Logic Analysis");
    let _ = writeln!(out, "  Expression: {}", bexpr_to_str(&expr));
    let _ = writeln!(out, "  Variables : {}", vars.join(", "));
    let _ = writeln!(out, "  {} satisfying assignments of {} ({}%)",
        sat_count, rows, sat_count * 100 / rows);
    let _ = writeln!(out, "  Status: {}",
        if is_taut { "TAUTOLOGY (always true)" }
        else if is_contra { "CONTRADICTION (always false)" }
        else { "CONTINGENT (sometimes true)" });

    match mode {
        "sat" => {
            if is_sat {
                let first_sat = (0..rows).find(|&mask| results[mask]).unwrap();
                let assignment: Vec<String> = vars.iter().enumerate()
                    .map(|(i, v)| format!("{}={}", v, (first_sat >> (n-1-i)) & 1 == 1))
                    .collect();
                let _ = writeln!(out, "  SAT: YES — satisfying assignment: {}", assignment.join(", "));
            } else {
                let _ = writeln!(out, "  SAT: NO — contradiction");
            }
        }
        "taut" => {
            let _ = writeln!(out, "  TAUTOLOGY: {}", if is_taut { "YES" } else { "NO" });
            if !is_taut {
                let first_false = (0..rows).find(|&mask| !results[mask]).unwrap();
                let assignment: Vec<String> = vars.iter().enumerate()
                    .map(|(i, v)| format!("{}={}", v, (first_false >> (n-1-i)) & 1 == 1))
                    .collect();
                let _ = writeln!(out, "  Counterexample: {}", assignment.join(", "));
            }
        }
        "cnf" => {
            // Build CNF from false rows
            let false_rows: Vec<usize> = (0..rows).filter(|&m| !results[m]).collect();
            if false_rows.is_empty() {
                let _ = writeln!(out, "  CNF: true (tautology)");
            } else {
                let _ = writeln!(out, "  CNF (maxterms):");
                for mask in &false_rows[..false_rows.len().min(8)] {
                    let clause: Vec<String> = vars.iter().enumerate()
                        .map(|(i, v)| if (mask >> (n-1-i)) & 1 == 0 { v.clone() } else { format!("¬{}", v) })
                        .collect();
                    let _ = writeln!(out, "  ({})", clause.join(" ∨ "));
                }
                if false_rows.len() > 8 { let _ = writeln!(out, "  ... ({} more clauses)", false_rows.len()-8); }
            }
        }
        "dnf" => {
            // Build DNF from true rows
            let true_rows: Vec<usize> = (0..rows).filter(|&m| results[m]).collect();
            if true_rows.is_empty() {
                let _ = writeln!(out, "  DNF: false (contradiction)");
            } else {
                let _ = writeln!(out, "  DNF (minterms):");
                for mask in &true_rows[..true_rows.len().min(8)] {
                    let term: Vec<String> = vars.iter().enumerate()
                        .map(|(i, v)| if (mask >> (n-1-i)) & 1 == 1 { v.clone() } else { format!("¬{}", v) })
                        .collect();
                    let _ = writeln!(out, "  ({})", term.join(" ∧ "));
                }
                if true_rows.len() > 8 { let _ = writeln!(out, "  ... ({} more terms)", true_rows.len()-8); }
            }
        }
        "simplify" => {
            let simp = simplify_bexpr(expr.clone());
            let _ = writeln!(out, "  Simplified: {}", bexpr_to_str(&simp));
        }
        "equiv" => {
            let expr2 = match parse_bexpr(expr2_str) {
                Ok(e) => e,
                Err(e) => { let _ = writeln!(out, "  Parse error (expr2): {}", e); let _ = writeln!(out, "{}", "=".repeat(w)); return out; }
            };
            let mut vars2 = vars.clone();
            collect_vars(&expr2, &mut vars2);
            vars2.sort(); vars2.dedup();
            let n2 = vars2.len();
            let rows2 = 1usize << n2;
            let equiv = (0..rows2).all(|mask| {
                let assignment: Vec<(&str, bool)> = vars2.iter().enumerate()
                    .map(|(i, v)| (v.as_str(), (mask >> (n2-1-i)) & 1 == 1))
                    .collect();
                eval_bexpr(&expr, &assignment) == eval_bexpr(&expr2, &assignment)
            });
            let _ = writeln!(out, "  Expr1: {}", bexpr_to_str(&expr));
            let _ = writeln!(out, "  Expr2: {}", bexpr_to_str(&expr2));
            let _ = writeln!(out, "  Logically equivalent: {}", if equiv { "YES" } else { "NO" });
        }
        _ => {
            // "info" or "table" — show full truth table
            let max_table_rows = if n <= 4 { rows } else { rows.min(32) };
            let _ = writeln!(out, "\n  Truth Table:");
            // Header
            let var_header: String = vars.iter().map(|v| format!("  {:>3}", v)).collect::<Vec<_>>().join("");
            let _ = writeln!(out, "{}  │  Result", var_header);
            let _ = writeln!(out, "  {}", "-".repeat(vars.len()*5 + 10));
            for mask in 0..max_table_rows {
                let row_vals: String = (0..n).map(|i| format!("  {:>3}", if (mask >> (n-1-i)) & 1 == 1 { "T" } else { "F" })).collect::<Vec<_>>().join("");
                let _ = writeln!(out, "{}  │  {}", row_vals, if results[mask] { "T" } else { "F" });
            }
            if max_table_rows < rows { let _ = writeln!(out, "  ... ({} rows omitted — use --logic 'table EXPR' for full table with ≤4 vars)", rows - max_table_rows); }

            // SAT summary
            if is_sat {
                let first_sat = (0..rows).find(|&m| results[m]).unwrap();
                let sat_ex: Vec<String> = vars.iter().enumerate()
                    .map(|(i, v)| format!("{}={}", v, if (first_sat >> (n-1-i)) & 1 == 1 { "T" } else { "F" }))
                    .collect();
                let _ = writeln!(out, "\n  SAT witness: {}", sat_ex.join(", "));
            }
        }
    }

    let _ = writeln!(out, "{}", "=".repeat(w));
    out
}

// ── Linear algebra / matrix operations ───────────────────────────────────────
// det / inv / solve / mul / transpose / eigenvalues / rank — pure-Rust
//
// Matrix input formats (any mix):
//   [[1,2,3],[4,5,6],[7,8,9]]   JSON-style
//   1 2 3; 4 5 6; 7 8 9         semicolon rows
//   1 2 3\n4 5 6\n7 8 9         newline rows
//
// Modes (first token of query):
//   det A         determinant
//   inv A         inverse
//   solve A b     Ax = b  (b is an extra row/column vector)
//   mul A B       A × B
//   transpose A   transpose
//   eigen A       eigenvalues & eigenvectors (up to 8×8)
//   rank A        matrix rank
//   lu A          LU decomposition
//   info A        all basic info (default)

type Matrix = Vec<Vec<f64>>;

fn mat_rows(m: &Matrix) -> usize { m.len() }
fn mat_cols(m: &Matrix) -> usize { m.first().map(|r| r.len()).unwrap_or(0) }

fn parse_matrix(s: &str) -> Result<Matrix, String> {
    let s = s.trim();
    // Try [[...],[...]] JSON-like
    if s.starts_with('[') {
        return parse_matrix_json(s);
    }
    // Semicolon or newline rows
    let row_strs: Vec<&str> = s.split(|c: char| c == ';' || c == '\n')
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .collect();
    if row_strs.is_empty() { return Err("empty matrix".into()); }
    let mut mat: Matrix = Vec::new();
    for row_str in &row_strs {
        let row: Vec<f64> = row_str.split(|c: char| c == ',' || c == ' ' || c == '\t')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|tok| tok.parse::<f64>().map_err(|_| format!("bad number: {}", tok)))
            .collect::<Result<Vec<_>, _>>()?;
        mat.push(row);
    }
    let ncols = mat[0].len();
    for (i, row) in mat.iter().enumerate() {
        if row.len() != ncols {
            return Err(format!("row {} has {} columns, expected {}", i, row.len(), ncols));
        }
    }
    Ok(mat)
}

fn parse_matrix_json(s: &str) -> Result<Matrix, String> {
    // Simple recursive bracket parser — no serde needed
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0;
    fn skip(chars: &[char], pos: &mut usize) {
        while *pos < chars.len() && chars[*pos].is_whitespace() { *pos += 1; }
    }
    fn parse_num(chars: &[char], pos: &mut usize) -> Result<f64, String> {
        skip(chars, pos);
        let start = *pos;
        while *pos < chars.len() && (chars[*pos].is_ascii_digit() || matches!(chars[*pos], '.' | '-' | '+' | 'e' | 'E')) {
            *pos += 1;
        }
        let s: String = chars[start..*pos].iter().collect();
        s.trim().parse::<f64>().map_err(|_| format!("bad number: '{}'", s))
    }
    fn parse_row(chars: &[char], pos: &mut usize) -> Result<Vec<f64>, String> {
        skip(chars, pos);
        if chars.get(*pos) != Some(&'[') { return Err("expected '[' for row".into()); }
        *pos += 1;
        let mut row = Vec::new();
        loop {
            skip(chars, pos);
            if chars.get(*pos) == Some(&']') { *pos += 1; break; }
            if !row.is_empty() {
                if chars.get(*pos) == Some(&',') { *pos += 1; } else { return Err("expected ','".into()); }
            }
            row.push(parse_num(chars, pos)?);
        }
        Ok(row)
    }
    skip(&chars, &mut pos);
    if chars.get(pos) != Some(&'[') { return Err("expected outer '['".into()); }
    pos += 1;
    let mut mat: Matrix = Vec::new();
    loop {
        skip(&chars, &mut pos);
        if chars.get(pos) == Some(&']') { pos += 1; break; }
        if !mat.is_empty() {
            if chars.get(pos) == Some(&',') { pos += 1; } else { return Err("expected ','".into()); }
        }
        skip(&chars, &mut pos);
        // Check if this is a number (1-D vector case) or another row
        if chars.get(pos) == Some(&'[') {
            mat.push(parse_row(&chars, &mut pos)?);
        } else {
            // flat 1-D vector — wrap as single row
            let n = parse_num(&chars, &mut pos)?;
            mat.push(vec![n]);
        }
    }
    if mat.is_empty() { return Err("empty matrix".into()); }
    let ncols = mat[0].len();
    for (i, row) in mat.iter().enumerate() {
        if row.len() != ncols {
            return Err(format!("row {} has {} cols, expected {}", i, row.len(), ncols));
        }
    }
    Ok(mat)
}

fn mat_clone(m: &Matrix) -> Matrix { m.clone() }

fn mat_identity(n: usize) -> Matrix {
    (0..n).map(|i| (0..n).map(|j| if i==j { 1.0 } else { 0.0 }).collect()).collect()
}

fn mat_fmt(m: &Matrix) -> String {
    let rows = mat_rows(m);
    let cols = mat_cols(m);
    let cells: Vec<String> = m.iter().flat_map(|row| row.iter().map(|v| {
        if v.abs() < 1e-12 { "0".to_string() }
        else if v.fract() == 0.0 && v.abs() < 1e9 { format!("{}", *v as i64) }
        else { format!("{:.6}", v).trim_end_matches('0').trim_end_matches('.').to_string() }
    })).collect();
    // Align columns
    let mut col_widths: Vec<usize> = vec![0; cols];
    for r in 0..rows {
        for c in 0..cols {
            col_widths[c] = col_widths[c].max(cells[r*cols+c].len());
        }
    }
    let mut out = String::new();
    for r in 0..rows {
        out.push_str("  [ ");
        for c in 0..cols {
            let s = &cells[r*cols+c];
            out.push_str(&format!("{:>w$}", s, w = col_widths[c]));
            if c < cols-1 { out.push_str("  "); }
        }
        out.push_str(" ]\n");
    }
    out
}

// Returns (L, U, P, sign) where P*A = L*U and sign is the permutation sign
fn lu_decompose(a: &Matrix) -> Result<(Matrix, Matrix, Vec<usize>, i32), String> {
    let n = mat_rows(a);
    if mat_cols(a) != n { return Err("LU requires square matrix".into()); }
    let mut u = mat_clone(a);
    let mut l = mat_identity(n);
    let mut perm: Vec<usize> = (0..n).collect();
    let mut sign = 1i32;

    for col in 0..n {
        // Partial pivoting
        let mut max_row = col;
        let mut max_val = u[col][col].abs();
        for row in (col+1)..n {
            if u[row][col].abs() > max_val { max_val = u[row][col].abs(); max_row = row; }
        }
        if max_val < 1e-14 { return Err("matrix is singular (or near-singular)".into()); }
        if max_row != col {
            u.swap(col, max_row);
            perm.swap(col, max_row);
            sign = -sign;
            // Also swap l columns already filled
            for j in 0..col {
                let tmp = l[col][j];
                l[col][j] = l[max_row][j];
                l[max_row][j] = tmp;
            }
        }
        for row in (col+1)..n {
            let factor = u[row][col] / u[col][col];
            l[row][col] = factor;
            for k in col..n { u[row][k] -= factor * u[col][k]; }
        }
    }
    Ok((l, u, perm, sign))
}

fn mat_det(a: &Matrix) -> Result<f64, String> {
    match lu_decompose(a) {
        Ok((_, u, _, sign)) => {
            let d: f64 = (0..mat_rows(a)).map(|i| u[i][i]).product();
            Ok(d * sign as f64)
        }
        Err(_) => Ok(0.0), // singular
    }
}

fn mat_solve_lu(l: &Matrix, u: &Matrix, perm: &[usize], b: &[f64]) -> Vec<f64> {
    let n = l.len();
    // Apply permutation to b
    let pb: Vec<f64> = (0..n).map(|i| b[perm[i]]).collect();
    // Forward substitution Ly = Pb
    let mut y = vec![0.0f64; n];
    for i in 0..n {
        y[i] = pb[i] - (0..i).map(|j| l[i][j] * y[j]).sum::<f64>();
    }
    // Back substitution Ux = y
    let mut x = vec![0.0f64; n];
    for i in (0..n).rev() {
        x[i] = (y[i] - (i+1..n).map(|j| u[i][j] * x[j]).sum::<f64>()) / u[i][i];
    }
    x
}

fn mat_inv(a: &Matrix) -> Result<Matrix, String> {
    let n = mat_rows(a);
    let (l, u, perm, _) = lu_decompose(a)?;
    let mut inv = mat_identity(n);
    for col in 0..n {
        let b: Vec<f64> = (0..n).map(|i| if i==col { 1.0 } else { 0.0 }).collect();
        let x = mat_solve_lu(&l, &u, &perm, &b);
        for row in 0..n { inv[row][col] = x[row]; }
    }
    Ok(inv)
}

fn mat_mul(a: &Matrix, b: &Matrix) -> Result<Matrix, String> {
    let (ar, ac) = (mat_rows(a), mat_cols(a));
    let (br, bc) = (mat_rows(b), mat_cols(b));
    if ac != br { return Err(format!("incompatible dimensions {}×{} × {}×{}", ar, ac, br, bc)); }
    let mut c = vec![vec![0.0f64; bc]; ar];
    for i in 0..ar { for j in 0..bc { for k in 0..ac { c[i][j] += a[i][k] * b[k][j]; } } }
    Ok(c)
}

fn mat_transpose(a: &Matrix) -> Matrix {
    let (r, c) = (mat_rows(a), mat_cols(a));
    (0..c).map(|j| (0..r).map(|i| a[i][j]).collect()).collect()
}

fn mat_rank(a: &Matrix) -> usize {
    let mut m = mat_clone(a);
    let rows = mat_rows(&m);
    let cols = mat_cols(&m);
    let mut rank = 0usize;
    let mut row_cursor = 0usize;
    for col in 0..cols {
        let pivot = (row_cursor..rows).find(|&r| m[r][col].abs() > 1e-10);
        if let Some(pr) = pivot {
            m.swap(row_cursor, pr);
            let pivot_val = m[row_cursor][col];
            for j in col..cols { m[row_cursor][j] /= pivot_val; }
            for r in 0..rows {
                if r != row_cursor && m[r][col].abs() > 1e-10 {
                    let factor = m[r][col];
                    for j in col..cols { m[r][j] -= factor * m[row_cursor][j]; }
                }
            }
            rank += 1;
            row_cursor += 1;
        }
    }
    rank
}

// Power iteration eigenvalue (largest eigenvalue only)
fn mat_eigen_power(a: &Matrix, max_iter: usize) -> Option<(f64, Vec<f64>)> {
    let n = mat_rows(a);
    if n == 0 { return None; }
    let mut v: Vec<f64> = (0..n).map(|i| if i==0 { 1.0 } else { 0.1 }).collect();
    let mut lam = 0.0f64;
    for _ in 0..max_iter {
        // Av
        let av: Vec<f64> = (0..n).map(|i| (0..n).map(|j| a[i][j] * v[j]).sum::<f64>()).collect();
        let norm = av.iter().map(|x| x*x).sum::<f64>().sqrt();
        if norm < 1e-14 { break; }
        lam = av.iter().zip(&v).map(|(a,b)| a*b).sum::<f64>();
        v = av.iter().map(|x| x/norm).collect();
    }
    Some((lam, v))
}

pub fn matrix_calc(query: &str) -> String {
    let q = query.trim();

    // Detect mode
    let (mode, rest) = {
        let words: Vec<&str> = q.splitn(2, char::is_whitespace).collect();
        let m = words[0].to_lowercase();
        let rest = words.get(1).copied().unwrap_or("").trim();
        match m.as_str() {
            "det"|"determinant" => ("det", rest.to_string()),
            "inv"|"inverse"     => ("inv", rest.to_string()),
            "solve"             => ("solve", rest.to_string()),
            "mul"|"multiply"    => ("mul", rest.to_string()),
            "transpose"|"trans" => ("transpose", rest.to_string()),
            "eigen"|"eigenvalues"|"eig" => ("eigen", rest.to_string()),
            "rank"              => ("rank", rest.to_string()),
            "lu"                => ("lu", rest.to_string()),
            _                   => ("info", q.to_string()),
        }
    };

    let mut out = String::new();
    let w = 64usize;
    let _ = writeln!(out, "{}", "=".repeat(w));

    // For "solve" we need two matrices: A and b
    if mode == "solve" {
        // rest should be "A_matrix b_vector" — split on " / " or last matrix
        // Try to split at ']  [' or ']; [' or just split the two bracket groups
        let parts = split_two_matrices(&rest);
        if parts.len() < 2 {
            let _ = writeln!(out, "  Matrix — Solve Ax = b");
            let _ = writeln!(out, "  Error: provide matrix A and vector b, e.g.:");
            let _ = writeln!(out, "  --matrix 'solve [[1,2],[3,4]] [[5],[6]]'");
            let _ = writeln!(out, "{}", "=".repeat(w));
            return out;
        }
        let a_str = &parts[0];
        let b_str = &parts[1];
        let a = match parse_matrix(a_str) { Ok(m) => m, Err(e) => { let _ = writeln!(out, "  Parse error (A): {}", e); let _ = writeln!(out, "{}", "=".repeat(w)); return out; } };
        let b_mat = match parse_matrix(b_str) { Ok(m) => m, Err(e) => { let _ = writeln!(out, "  Parse error (b): {}", e); let _ = writeln!(out, "{}", "=".repeat(w)); return out; } };
        let n = mat_rows(&a);
        let b_vec: Vec<f64> = if mat_cols(&b_mat) == 1 {
            b_mat.iter().map(|r| r[0]).collect()
        } else if mat_rows(&b_mat) == 1 {
            b_mat[0].clone()
        } else {
            let _ = writeln!(out, "  Error: b must be a column or row vector"); let _ = writeln!(out, "{}", "=".repeat(w)); return out;
        };
        if b_vec.len() != n { let _ = writeln!(out, "  Error: A is {}×{} but b has {} elements", n, mat_cols(&a), b_vec.len()); let _ = writeln!(out, "{}", "=".repeat(w)); return out; }
        let _ = writeln!(out, "  Matrix — Solve Ax = b");
        let _ = writeln!(out, "  A ({}×{}):", n, mat_cols(&a));
        out.push_str(&mat_fmt(&a));
        let _ = writeln!(out, "  b:");
        let b_col: Matrix = b_vec.iter().map(|&v| vec![v]).collect();
        out.push_str(&mat_fmt(&b_col));
        match lu_decompose(&a) {
            Ok((l, u, perm, _)) => {
                let x = mat_solve_lu(&l, &u, &perm, &b_vec);
                let _ = writeln!(out, "  Solution x:");
                for (i, &xi) in x.iter().enumerate() {
                    let _ = writeln!(out, "    x[{}] = {:.8}", i, xi);
                }
                // Verify: Ax - b residual
                let residual: f64 = (0..n).map(|i| {
                    let ax_i: f64 = (0..n).map(|j| a[i][j] * x[j]).sum();
                    (ax_i - b_vec[i]).powi(2)
                }).sum::<f64>().sqrt();
                let _ = writeln!(out, "  Residual |Ax - b| = {:.2e}", residual);
            }
            Err(e) => { let _ = writeln!(out, "  Error: {}", e); }
        }
        let _ = writeln!(out, "{}", "=".repeat(w));
        return out;
    }

    // For mul we need two matrices
    if mode == "mul" {
        let parts = split_two_matrices(&rest);
        if parts.len() < 2 {
            let _ = writeln!(out, "  Matrix multiply: provide two matrices, e.g.:");
            let _ = writeln!(out, "  --matrix 'mul [[1,2],[3,4]] [[5,6],[7,8]]'");
            let _ = writeln!(out, "{}", "=".repeat(w));
            return out;
        }
        let a = match parse_matrix(&parts[0]) { Ok(m) => m, Err(e) => { let _ = writeln!(out, "  Parse error (A): {}", e); let _ = writeln!(out, "{}", "=".repeat(w)); return out; } };
        let b = match parse_matrix(&parts[1]) { Ok(m) => m, Err(e) => { let _ = writeln!(out, "  Parse error (B): {}", e); let _ = writeln!(out, "{}", "=".repeat(w)); return out; } };
        let _ = writeln!(out, "  Matrix Multiply  A × B");
        let _ = writeln!(out, "  A ({}×{}):", mat_rows(&a), mat_cols(&a));
        out.push_str(&mat_fmt(&a));
        let _ = writeln!(out, "  B ({}×{}):", mat_rows(&b), mat_cols(&b));
        out.push_str(&mat_fmt(&b));
        match mat_mul(&a, &b) {
            Ok(c) => { let _ = writeln!(out, "  A × B ({}×{}):", mat_rows(&c), mat_cols(&c)); out.push_str(&mat_fmt(&c)); }
            Err(e) => { let _ = writeln!(out, "  Error: {}", e); }
        }
        let _ = writeln!(out, "{}", "=".repeat(w));
        return out;
    }

    // Single-matrix operations
    let mat_str = &rest;
    let a = match parse_matrix(mat_str) {
        Ok(m) => m,
        Err(e) => {
            let _ = writeln!(out, "  Parse error: {}", e);
            let _ = writeln!(out, "  Input: {}", mat_str.chars().take(80).collect::<String>());
            let _ = writeln!(out, "  Formats: [[1,2],[3,4]]  or  1 2; 3 4");
            let _ = writeln!(out, "{}", "=".repeat(w));
            return out;
        }
    };

    let (rows, cols) = (mat_rows(&a), mat_cols(&a));
    let _ = writeln!(out, "  Matrix Operations  ({}×{})", rows, cols);
    out.push_str(&mat_fmt(&a));

    match mode {
        "det" => {
            if rows != cols { let _ = writeln!(out, "  Error: det requires square matrix"); }
            else {
                match mat_det(&a) {
                    Ok(d) => { let _ = writeln!(out, "  det(A) = {:.8}", d); }
                    Err(e) => { let _ = writeln!(out, "  Error: {}", e); }
                }
            }
        }
        "inv" => {
            if rows != cols { let _ = writeln!(out, "  Error: inv requires square matrix"); }
            else {
                match mat_inv(&a) {
                    Ok(inv) => { let _ = writeln!(out, "  A⁻¹:"); out.push_str(&mat_fmt(&inv)); }
                    Err(e)  => { let _ = writeln!(out, "  Error: {}", e); }
                }
            }
        }
        "transpose" => {
            let t = mat_transpose(&a);
            let _ = writeln!(out, "  Aᵀ ({}×{}):", mat_cols(&a), rows);
            out.push_str(&mat_fmt(&t));
        }
        "rank" => {
            let r = mat_rank(&a);
            let _ = writeln!(out, "  rank(A) = {}", r);
            if rows == cols {
                let _ = writeln!(out, "  {} ({}×{} square, rank {})", if r == rows { "Full rank" } else { "Rank-deficient" }, rows, cols, r);
            }
        }
        "lu" => {
            if rows != cols { let _ = writeln!(out, "  Error: LU requires square matrix"); }
            else {
                match lu_decompose(&a) {
                    Ok((l, u, perm, _)) => {
                        let _ = writeln!(out, "  L (lower triangular):");
                        out.push_str(&mat_fmt(&l));
                        let _ = writeln!(out, "  U (upper triangular):");
                        out.push_str(&mat_fmt(&u));
                        let perm_str = perm.iter().map(|&p| p.to_string()).collect::<Vec<_>>().join(", ");
                        let _ = writeln!(out, "  Pivot permutation: [{}]", perm_str);
                    }
                    Err(e) => { let _ = writeln!(out, "  Error: {}", e); }
                }
            }
        }
        "eigen" => {
            if rows != cols { let _ = writeln!(out, "  Error: eigenvalues require square matrix"); }
            else if rows > 8 { let _ = writeln!(out, "  Error: power iteration limited to 8×8 (matrix is {}×{})", rows, cols); }
            else {
                let _ = writeln!(out, "  Eigenvalues (power iteration + deflation):");
                let mut a_copy = mat_clone(&a);
                for k in 0..rows {
                    match mat_eigen_power(&a_copy, 500) {
                        Some((lam, v)) => {
                            let v_str = v.iter().map(|x| format!("{:.4}", x)).collect::<Vec<_>>().join(", ");
                            let _ = writeln!(out, "  λ{} = {:.6}  eigenvector ≈ [{}]", k+1, lam, v_str);
                            // Deflate
                            for i in 0..rows { for j in 0..rows { a_copy[i][j] -= lam * v[i] * v[j]; } }
                        }
                        None => break,
                    }
                }
                let trace: f64 = (0..rows).map(|i| a[i][i]).sum();
                let _ = writeln!(out, "  Trace = {:.6}", trace);
                if let Ok(d) = mat_det(&a) { let _ = writeln!(out, "  Det   = {:.6}", d); }
            }
        }
        _ => {
            // "info" — show all applicable results
            let _ = writeln!(out, "  Rank: {}", mat_rank(&a));
            if rows == cols {
                if let Ok(d) = mat_det(&a) { let _ = writeln!(out, "  Det:  {:.6}", d); }
                match mat_inv(&a) {
                    Ok(inv) => { let _ = writeln!(out, "  Inverse:"); out.push_str(&mat_fmt(&inv)); }
                    Err(_)  => { let _ = writeln!(out, "  Inverse: N/A (singular)"); }
                }
                let trace: f64 = (0..rows).map(|i| a[i][i]).sum();
                let _ = writeln!(out, "  Trace: {:.6}", trace);
                let frobenius: f64 = a.iter().flat_map(|r| r.iter()).map(|v| v*v).sum::<f64>().sqrt();
                let _ = writeln!(out, "  Frobenius norm: {:.6}", frobenius);
            }
            let t = mat_transpose(&a);
            let _ = writeln!(out, "  Transpose:"); out.push_str(&mat_fmt(&t));
        }
    }

    let _ = writeln!(out, "{}", "=".repeat(w));
    out
}

fn split_two_matrices(s: &str) -> Vec<String> {
    // Split "A B" where A and B are either [[...]] or "row; row" groups
    let s = s.trim();
    if s.starts_with('[') {
        // Find the end of the first [[...]] group
        let mut depth = 0;
        let mut end = 0;
        for (i, c) in s.chars().enumerate() {
            if c == '[' { depth += 1; }
            else if c == ']' { depth -= 1; if depth == 0 { end = i+1; break; } }
        }
        if end == 0 || end >= s.len() { return vec![s.to_string()]; }
        let a_str = s[..end].trim().to_string();
        let b_str = s[end..].trim().trim_start_matches(|c: char| c == ',' || c == ' ').to_string();
        if b_str.is_empty() { return vec![a_str]; }
        return vec![a_str, b_str];
    }
    // For semicolon format: split on " / " or "| " or just try natural language split
    if let Some(pos) = s.find(" / ") {
        return vec![s[..pos].trim().to_string(), s[pos+3..].trim().to_string()];
    }
    vec![s.to_string()]
}

// ── Financial math ───────────────────────────────────────────────────────────
// NPV / IRR / loan amortization / compound interest / bond pricing / Black-Scholes
// Pure-Rust, instant, no sandbox.
//
// Query syntax:
//   npv RATE CF0 CF1 CF2 ...          net present value
//   irr CF0 CF1 CF2 ...               internal rate of return (bisection)
//   loan PRINCIPAL RATE_PCT YEARS     loan amortization schedule summary
//   compound PRINCIPAL RATE_PCT YEARS [PERIODS_PER_YEAR]
//   bond FACE COUPON_PCT YIELD_PCT YEARS [PERIODS_PER_YEAR]
//   bs SPOT STRIKE RATE_PCT SIGMA_PCT YEARS [call|put]   Black-Scholes

pub fn finance_calc(query: &str) -> String {
    let q = query.trim();
    let tokens: Vec<&str> = q.split_whitespace().collect();
    if tokens.is_empty() { return finance_usage(); }

    let mut out = String::new();
    let w = 64usize;
    let _ = writeln!(out, "{}", "=".repeat(w));

    match tokens[0].to_lowercase().as_str() {
        "npv" => {
            if tokens.len() < 3 { let _ = writeln!(out, "  Usage: npv RATE CF0 CF1 CF2 ..."); let _ = writeln!(out, "{}", "=".repeat(w)); return out; }
            let rate: f64 = match tokens[1].trim_end_matches('%').parse() {
                Ok(v) => if tokens[1].contains('%') { v / 100.0 } else { v },
                Err(_) => { let _ = writeln!(out, "  Error: bad rate '{}'", tokens[1]); let _ = writeln!(out, "{}", "=".repeat(w)); return out; }
            };
            let cfs: Vec<f64> = tokens[2..].iter().filter_map(|s| s.replace(',', "").parse::<f64>().ok()).collect();
            if cfs.is_empty() { let _ = writeln!(out, "  Error: no cash flows found"); let _ = writeln!(out, "{}", "=".repeat(w)); return out; }
            let npv: f64 = cfs.iter().enumerate().map(|(t, &cf)| cf / (1.0 + rate).powi(t as i32)).sum();
            let _ = writeln!(out, "  NPV Analysis");
            let _ = writeln!(out, "  Discount rate : {:.4}%", rate * 100.0);
            let _ = writeln!(out, "  Cash flows    : {}", cfs.iter().map(|cf| format!("{:.2}", cf)).collect::<Vec<_>>().join("  "));
            let _ = writeln!(out, "  NPV           : {:.4}", npv);
            let _ = writeln!(out, "  Decision      : {}", if npv > 0.0 { "Accept (NPV > 0)" } else if npv < 0.0 { "Reject (NPV < 0)" } else { "Indifferent" });
        }
        "irr" => {
            if tokens.len() < 3 { let _ = writeln!(out, "  Usage: irr CF0 CF1 CF2 ..."); let _ = writeln!(out, "{}", "=".repeat(w)); return out; }
            let cfs: Vec<f64> = tokens[1..].iter().filter_map(|s| s.replace(',', "").parse::<f64>().ok()).collect();
            if cfs.is_empty() { let _ = writeln!(out, "  Error: no cash flows"); let _ = writeln!(out, "{}", "=".repeat(w)); return out; }
            fn npv_at(rate: f64, cfs: &[f64]) -> f64 {
                cfs.iter().enumerate().map(|(t, &cf)| cf / (1.0 + rate).powi(t as i32)).sum()
            }
            // Bisection search for IRR in (-0.9999, 10.0)
            let mut lo = -0.9999f64;
            let mut hi = 10.0f64;
            let npv_lo = npv_at(lo, &cfs);
            let npv_hi = npv_at(hi, &cfs);
            let _ = writeln!(out, "  IRR Analysis");
            if npv_lo * npv_hi > 0.0 {
                let _ = writeln!(out, "  IRR: no unique root found in (-99.99%, 1000%) — check sign changes in cash flows");
            } else {
                for _ in 0..200 {
                    let mid = (lo + hi) / 2.0;
                    if npv_at(mid, &cfs) * npv_at(lo, &cfs) < 0.0 { hi = mid; } else { lo = mid; }
                    if (hi - lo).abs() < 1e-10 { break; }
                }
                let irr = (lo + hi) / 2.0;
                let _ = writeln!(out, "  Cash flows : {}", cfs.iter().map(|cf| format!("{:.2}", cf)).collect::<Vec<_>>().join("  "));
                let _ = writeln!(out, "  IRR        : {:.6}%", irr * 100.0);
                let npv_check = npv_at(irr, &cfs);
                let _ = writeln!(out, "  NPV @ IRR  : {:.8} (should be ~0)", npv_check);
            }
        }
        "loan" => {
            if tokens.len() < 4 { let _ = writeln!(out, "  Usage: loan PRINCIPAL RATE_PCT YEARS"); let _ = writeln!(out, "{}", "=".repeat(w)); return out; }
            let principal: f64 = tokens[1].replace(',', "").parse().unwrap_or(0.0);
            let annual_rate: f64 = tokens[2].trim_end_matches('%').parse::<f64>().unwrap_or(0.0) / 100.0;
            let years: f64 = tokens[3].parse().unwrap_or(0.0);
            let n = (years * 12.0).round() as u32;
            let r = annual_rate / 12.0;
            let payment = if r.abs() < 1e-12 {
                principal / n as f64
            } else {
                principal * r * (1.0 + r).powi(n as i32) / ((1.0 + r).powi(n as i32) - 1.0)
            };
            let total_paid = payment * n as f64;
            let total_interest = total_paid - principal;
            let _ = writeln!(out, "  Loan Amortization");
            let _ = writeln!(out, "  Principal      : {:>12.2}", principal);
            let _ = writeln!(out, "  Annual rate    : {:>12.4}%", annual_rate * 100.0);
            let _ = writeln!(out, "  Term           : {:>12} months ({} years)", n, years);
            let _ = writeln!(out, "  Monthly payment: {:>12.2}", payment);
            let _ = writeln!(out, "  Total paid     : {:>12.2}", total_paid);
            let _ = writeln!(out, "  Total interest : {:>12.2}", total_interest);
            let _ = writeln!(out, "  Interest ratio : {:>12.2}%", total_interest / total_paid * 100.0);
            // Show amortization table for first/last few months if reasonable
            if n <= 60 || n <= 360 {
                let show_rows = 6usize.min(n as usize);
                let _ = writeln!(out, "\n  {:<6}  {:>12}  {:>12}  {:>12}  {:>12}", "Month", "Payment", "Principal", "Interest", "Balance");
                let _ = writeln!(out, "  {}", "-".repeat(58));
                let mut balance = principal;
                for mo in 1..=n {
                    let interest_part = balance * r;
                    let principal_part = payment - interest_part;
                    balance -= principal_part;
                    if balance < 0.0 { balance = 0.0; }
                    if mo as usize <= show_rows || mo as usize > n as usize - show_rows {
                        let _ = writeln!(out, "  {:<6}  {:>12.2}  {:>12.2}  {:>12.2}  {:>12.2}", mo, payment, principal_part, interest_part, balance);
                    } else if mo as usize == show_rows + 1 {
                        let _ = writeln!(out, "  {:^58}", "...");
                    }
                }
            }
        }
        "compound" => {
            if tokens.len() < 4 { let _ = writeln!(out, "  Usage: compound PRINCIPAL RATE_PCT YEARS [PERIODS]"); let _ = writeln!(out, "{}", "=".repeat(w)); return out; }
            let p: f64 = tokens[1].replace(',', "").parse().unwrap_or(0.0);
            let r: f64 = tokens[2].trim_end_matches('%').parse::<f64>().unwrap_or(0.0) / 100.0;
            let t: f64 = tokens[3].parse().unwrap_or(1.0);
            let n: f64 = tokens.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
            let fv = p * (1.0 + r / n).powf(n * t);
            let fv_cont = p * (r * t).exp();
            let _ = writeln!(out, "  Compound Interest");
            let _ = writeln!(out, "  Principal     : {:>12.2}", p);
            let _ = writeln!(out, "  Annual rate   : {:>12.4}%", r * 100.0);
            let _ = writeln!(out, "  Years         : {:>12}", t);
            let _ = writeln!(out, "  Periods/year  : {:>12}", n);
            let _ = writeln!(out, "  Future value  : {:>12.4}", fv);
            let _ = writeln!(out, "  Interest earned: {:>12.4}", fv - p);
            let _ = writeln!(out, "  Continuous FV : {:>12.4}", fv_cont);
            let eff_rate = (1.0 + r / n).powf(n) - 1.0;
            let _ = writeln!(out, "  Effective rate: {:>12.4}%", eff_rate * 100.0);
        }
        "bond" => {
            if tokens.len() < 6 { let _ = writeln!(out, "  Usage: bond FACE COUPON_PCT YIELD_PCT YEARS [PERIODS]"); let _ = writeln!(out, "{}", "=".repeat(w)); return out; }
            let face: f64 = tokens[1].replace(',', "").parse().unwrap_or(1000.0);
            let coupon_rate: f64 = tokens[2].trim_end_matches('%').parse::<f64>().unwrap_or(0.0) / 100.0;
            let yield_rate: f64 = tokens[3].trim_end_matches('%').parse::<f64>().unwrap_or(0.0) / 100.0;
            let years: f64 = tokens[4].parse().unwrap_or(1.0);
            let m: f64 = tokens.get(5).and_then(|s| s.parse().ok()).unwrap_or(2.0); // semi-annual default
            let n = (years * m).round() as i32;
            let r = yield_rate / m;
            let c = face * coupon_rate / m;
            // Price = PV of coupons + PV of face
            let pv_coupons = if r.abs() < 1e-12 { c * n as f64 } else { c * (1.0 - (1.0 + r).powi(-n)) / r };
            let pv_face = face / (1.0 + r).powi(n);
            let price = pv_coupons + pv_face;
            let duration_num: f64 = (1..=n).map(|t| t as f64 / m * c / (1.0 + r).powi(t)).sum::<f64>()
                + years * pv_face;
            let duration = duration_num / price;
            let _ = writeln!(out, "  Bond Pricing");
            let _ = writeln!(out, "  Face value     : {:>12.2}", face);
            let _ = writeln!(out, "  Coupon rate    : {:>12.4}%  ({:.2} per period)", coupon_rate * 100.0, c);
            let _ = writeln!(out, "  Yield to mat.  : {:>12.4}%", yield_rate * 100.0);
            let _ = writeln!(out, "  Years to mat.  : {:>12}", years);
            let _ = writeln!(out, "  Periods/year   : {:>12}", m);
            let _ = writeln!(out, "  Total periods  : {:>12}", n);
            let _ = writeln!(out, "  Bond price     : {:>12.4}", price);
            let _ = writeln!(out, "  PV of coupons  : {:>12.4}", pv_coupons);
            let _ = writeln!(out, "  PV of face     : {:>12.4}", pv_face);
            let status = if price > face { "Premium" } else if price < face { "Discount" } else { "Par" };
            let _ = writeln!(out, "  Bond trades at : {} ({:.2}% of face)", status, price / face * 100.0);
            let _ = writeln!(out, "  Macaulay dur.  : {:>12.4} years", duration);
        }
        "bs" | "black-scholes" | "blackscholes" | "option" => {
            if tokens.len() < 6 { let _ = writeln!(out, "  Usage: bs SPOT STRIKE RATE_PCT SIGMA_PCT YEARS [call|put]"); let _ = writeln!(out, "{}", "=".repeat(w)); return out; }
            let s: f64 = tokens[1].replace(',', "").parse().unwrap_or(0.0);
            let k: f64 = tokens[2].replace(',', "").parse().unwrap_or(0.0);
            let r: f64 = tokens[3].trim_end_matches('%').parse::<f64>().unwrap_or(0.0) / 100.0;
            let sigma: f64 = tokens[4].trim_end_matches('%').parse::<f64>().unwrap_or(0.0) / 100.0;
            let t: f64 = tokens[5].parse().unwrap_or(1.0);
            let opt_type = tokens.get(6).copied().unwrap_or("call");
            let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt());
            let d2 = d1 - sigma * t.sqrt();
            let nd1 = bs_ncdf(d1);
            let nd2 = bs_ncdf(d2);
            let (price, delta) = if opt_type.to_lowercase().starts_with('p') {
                let p = k * (-r * t).exp() * bs_ncdf(-d2) - s * bs_ncdf(-d1);
                (p, nd1 - 1.0)
            } else {
                let c = s * nd1 - k * (-r * t).exp() * nd2;
                (c, nd1)
            };
            let gamma = bs_npdf(d1) / (s * sigma * t.sqrt());
            let vega  = s * bs_npdf(d1) * t.sqrt() / 100.0;
            let theta_call = (-s * bs_npdf(d1) * sigma / (2.0 * t.sqrt()) - r * k * (-r * t).exp() * nd2) / 365.0;
            let _ = writeln!(out, "  Black-Scholes Option Pricing");
            let _ = writeln!(out, "  Spot price     : {:>12.4}", s);
            let _ = writeln!(out, "  Strike price   : {:>12.4}", k);
            let _ = writeln!(out, "  Risk-free rate : {:>12.4}%", r * 100.0);
            let _ = writeln!(out, "  Volatility (σ) : {:>12.4}%", sigma * 100.0);
            let _ = writeln!(out, "  Time (years)   : {:>12.4}", t);
            let _ = writeln!(out, "  Option type    : {:>12}", opt_type.to_uppercase());
            let _ = writeln!(out, "  ─────────────────────────────────────────────");
            let _ = writeln!(out, "  d1             : {:>12.6}", d1);
            let _ = writeln!(out, "  d2             : {:>12.6}", d2);
            let _ = writeln!(out, "  N(d1) / N(d2)  : {:>12.6} / {:>12.6}", nd1, nd2);
            let _ = writeln!(out, "  ─────────────────────────────────────────────");
            let _ = writeln!(out, "  Option price   : {:>12.6}", price);
            let _ = writeln!(out, "  Delta          : {:>12.6}", delta);
            let _ = writeln!(out, "  Gamma          : {:>12.6}", gamma);
            let _ = writeln!(out, "  Vega (per 1%σ) : {:>12.6}", vega);
            let _ = writeln!(out, "  Theta (per day): {:>12.6}", theta_call);
        }
        _ => {
            let _ = writeln!(out, "{}", finance_usage());
            let _ = writeln!(out, "{}", "=".repeat(w));
            return out;
        }
    }

    let _ = writeln!(out, "{}", "=".repeat(w));
    out
}

fn bs_ncdf(x: f64) -> f64 {
    // Abramowitz & Stegun approximation (max error 7.5e-8)
    if x < -8.0 { return 0.0; }
    if x >  8.0 { return 1.0; }
    if x >= 0.0 {
        0.5 * (1.0 + erf_approx(x / std::f64::consts::SQRT_2))
    } else {
        0.5 * (1.0 - erf_approx(-x / std::f64::consts::SQRT_2))
    }
}

fn bs_npdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

fn finance_usage() -> String {
    "Financial math:\n\
     hematite --finance 'npv 10% -1000 300 400 500 200'      NPV\n\
     hematite --finance 'irr -1000 300 400 500 200'           IRR\n\
     hematite --finance 'loan 200000 6.5% 30'                 30yr mortgage\n\
     hematite --finance 'compound 10000 7% 10 12'             compound interest\n\
     hematite --finance 'bond 1000 5% 4% 10 2'               bond pricing\n\
     hematite --finance 'bs 100 100 5% 20% 1 call'           Black-Scholes call\n\
     hematite --finance 'bs 100 105 5% 20% 0.5 put'          Black-Scholes put".into()
}

// ── Graph theory ──────────────────────────────────────────────────────────────
// Parses an edge list, then runs BFS/DFS/Dijkstra/components/topo-sort.
//
// Input format — one edge per line or semicolon-separated:
//   A B          (unweighted, undirected)
//   A B 5        (weighted)
//   A->B or A->B:5   (directed)
//   A-B or A-B:5     (undirected)
//
// Modes (first word of query before the edge list):
//   bfs FROM       breadth-first search from a node
//   dfs FROM       depth-first search from a node
//   shortest FROM TO   Dijkstra shortest path
//   components     connected components
//   topo           topological sort (directed)
//   info           degree table + basic stats (default)

pub fn graph_theory(query: &str) -> String {
    let q = query.trim();

    // Split mode/args from edge list
    // Edge list starts when a line/token contains a separator or is all non-alpha… heuristic:
    // Look for the first token containing '-', '>' or a digit after a space — that's the edge list.
    // But first try to strip a known mode keyword from the front.

    let (mode, rest) = {
        let tokens: Vec<&str> = q.splitn(2, |c: char| c == '\n' || c == ';').collect();
        let first_line = tokens[0].trim();
        let _fl_lower = first_line.to_lowercase();
        // Check if the entire first line looks like a mode+args header (no edge separators)
        let looks_like_mode = !first_line.contains("->") && !first_line.contains(" - ")
            && first_line.split_whitespace().count() <= 3;
        if looks_like_mode {
            let words: Vec<&str> = first_line.splitn(2, char::is_whitespace).collect();
            let m = words[0].to_lowercase();
            let after_mode = words.get(1).copied().unwrap_or("").trim();
            let rest_str = if tokens.len() > 1 {
                format!("{}\n{}", after_mode, tokens[1])
            } else {
                after_mode.to_string()
            };
            match m.as_str() {
                "bfs"|"dfs"|"shortest"|"path"|"components"|"topo"|"topological"|"info"|"degree" => {
                    (m, rest_str)
                }
                _ => {
                    // The first line might be part of an edge list; treat the whole thing as "info"
                    ("info".to_string(), q.to_string())
                }
            }
        } else {
            ("info".to_string(), q.to_string())
        }
    };

    // Parse edge list
    // Edges separated by newline or semicolon
    let edge_strs: Vec<&str> = rest.split(|c: char| c == '\n' || c == ';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let mut directed = false;
    let mut nodes: Vec<String> = Vec::new();
    let mut edges: Vec<(String, String, f64)> = Vec::new();

    let node_id = |name: &str, nodes: &mut Vec<String>| -> usize {
        if let Some(p) = nodes.iter().position(|n| n == name) {
            p
        } else {
            nodes.push(name.to_string());
            nodes.len() - 1
        }
    };

    for line in &edge_strs {
        let line = line.trim();
        if line.is_empty() { continue; }
        // Detect directed
        let (a, b, w, dir) = if let Some(pos) = line.find("->") {
            directed = true;
            let a = line[..pos].trim().trim_matches(':');
            let rest2 = line[pos+2..].trim();
            let (b, w) = parse_node_weight(rest2);
            (a, b, w, true)
        } else if let Some(pos) = line.find(" - ").or_else(|| {
            // "A-B" but avoid matching negative numbers
            let parts: Vec<&str> = line.splitn(3, char::is_whitespace).collect();
            if parts.len() >= 2 {
                // space-separated "A B [w]"
                None
            } else {
                // Check for single hyphen between non-numeric tokens
                let hp = line.find('-');
                if let Some(h) = hp {
                    if h > 0 && !line[..h].trim().parse::<f64>().is_ok() {
                        Some(h)
                    } else { None }
                } else { None }
            }
        }) {
            let sep_len = if line[pos..].starts_with(" - ") { 3 } else { 1 };
            let a = line[..pos].trim();
            let rest2 = line[pos+sep_len..].trim();
            let (b, w) = parse_node_weight(rest2);
            (a, b, w, false)
        } else {
            // Space-separated: "A B [w]"
            let parts: Vec<&str> = line.splitn(3, char::is_whitespace).collect();
            if parts.len() < 2 {
                node_id(line, &mut nodes);
                continue;
            }
            let a = parts[0].trim();
            let b_raw = parts[1].trim();
            // b_raw may be "NodeName:weight" or just "NodeName"; weight may be parts[2]
            let (b, w) = if let Some(cp) = b_raw.find(':') {
                let wt = b_raw[cp+1..].parse::<f64>().unwrap_or(1.0);
                (&b_raw[..cp], wt)
            } else {
                let wt = parts.get(2).and_then(|s| s.trim().parse::<f64>().ok()).unwrap_or(1.0);
                (b_raw, wt)
            };
            (a, b, w, false)
        };

        if a.is_empty() || b.is_empty() { continue; }
        let ai = node_id(a, &mut nodes);
        let bi = node_id(b, &mut nodes);
        edges.push((nodes[ai].clone(), nodes[bi].clone(), w));
        if !dir { /* undirected edge added both ways below */ }
    }

    if nodes.is_empty() {
        return graph_usage();
    }

    let n = nodes.len();

    // Build adjacency list: adj[i] = Vec<(j, weight)>
    let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for (a_name, b_name, w) in &edges {
        let ai = nodes.iter().position(|x| x == a_name).unwrap();
        let bi = nodes.iter().position(|x| x == b_name).unwrap();
        adj[ai].push((bi, *w));
        if !directed {
            adj[bi].push((ai, *w));
        }
    }

    let mut out = String::new();
    let w = 64usize;
    let _ = writeln!(out, "{}", "=".repeat(w));
    let _ = writeln!(out, "  Graph Analysis  |  {} nodes  |  {} edges  |  {}",
        n, edges.len(),
        if directed { "directed" } else { "undirected" });
    let _ = writeln!(out, "{}", "=".repeat(w));

    match mode.as_str() {
        "bfs" => {
            let start_name = rest.split_whitespace().next().unwrap_or(&nodes[0]);
            let start = nodes.iter().position(|x| x == start_name).unwrap_or(0);
            let order = bfs_order(&adj, start, n);
            let _ = writeln!(out, "  BFS from \"{}\":", nodes[start]);
            let _ = writeln!(out, "  Visit order: {}", order.iter().map(|&i| nodes[i].as_str()).collect::<Vec<_>>().join(" → "));
        }
        "dfs" => {
            let start_name = rest.split_whitespace().next().unwrap_or(&nodes[0]);
            let start = nodes.iter().position(|x| x == start_name).unwrap_or(0);
            let order = dfs_order(&adj, start, n);
            let _ = writeln!(out, "  DFS from \"{}\":", nodes[start]);
            let _ = writeln!(out, "  Visit order: {}", order.iter().map(|&i| nodes[i].as_str()).collect::<Vec<_>>().join(" → "));
        }
        "shortest" | "path" => {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            let from_name = parts.first().copied().unwrap_or(&nodes[0]);
            let to_name   = parts.get(1).copied().unwrap_or(&nodes[n-1]);
            let from = nodes.iter().position(|x| x == from_name).unwrap_or(0);
            let to   = nodes.iter().position(|x| x == to_name).unwrap_or(n.saturating_sub(1));
            match dijkstra(&adj, from, to, n) {
                Some((dist, path)) => {
                    let path_str = path.iter().map(|&i| nodes[i].as_str()).collect::<Vec<_>>().join(" → ");
                    let _ = writeln!(out, "  Shortest path: {} → {}", nodes[from], nodes[to]);
                    let _ = writeln!(out, "  Distance: {:.4}", dist);
                    let _ = writeln!(out, "  Path: {}", path_str);
                }
                None => {
                    let _ = writeln!(out, "  No path from \"{}\" to \"{}\"", nodes[from], nodes[to]);
                }
            }
            // Also show all-pairs distances from source
            let dists = dijkstra_all(&adj, from, n);
            let _ = writeln!(out, "\n  All distances from \"{}\":", nodes[from]);
            for (i, d) in dists.iter().enumerate() {
                if i == from { continue; }
                if *d == f64::INFINITY {
                    let _ = writeln!(out, "    → {:<20}  unreachable", &nodes[i]);
                } else {
                    let _ = writeln!(out, "    → {:<20}  {:.4}", &nodes[i], d);
                }
            }
        }
        "components" => {
            let comps = connected_components(&adj, n, directed);
            let _ = writeln!(out, "  Connected components: {}", comps.len());
            for (ci, comp) in comps.iter().enumerate() {
                let names: Vec<&str> = comp.iter().map(|&i| nodes[i].as_str()).collect();
                let _ = writeln!(out, "  [{}] {}", ci+1, names.join(", "));
            }
        }
        "topo" | "topological" => {
            match topo_sort(&adj, n) {
                Ok(order) => {
                    let _ = writeln!(out, "  Topological sort:");
                    let _ = writeln!(out, "  {}", order.iter().map(|&i| nodes[i].as_str()).collect::<Vec<_>>().join(" → "));
                }
                Err(_) => {
                    let _ = writeln!(out, "  Cycle detected — topological sort not possible.");
                }
            }
        }
        _ => {
            // Default: degree table + basic stats
            let mut in_deg  = vec![0usize; n];
            let mut out_deg = vec![0usize; n];
            for (ai, nbrs) in adj.iter().enumerate() {
                out_deg[ai] = nbrs.len();
                for &(bi, _) in nbrs {
                    in_deg[bi] += 1;
                }
            }
            let _ = writeln!(out, "  {:<20}  {:>8}  {:>8}", "Node", if directed {"Out-deg"} else {"Degree"}, if directed {"In-deg"} else {""});
            let _ = writeln!(out, "  {}", "-".repeat(40));
            let mut sorted_nodes: Vec<usize> = (0..n).collect();
            sorted_nodes.sort_by(|&a, &b| out_deg[b].cmp(&out_deg[a]));
            for &i in &sorted_nodes {
                if directed {
                    let _ = writeln!(out, "  {:<20}  {:>8}  {:>8}", &nodes[i], out_deg[i], in_deg[i]);
                } else {
                    let _ = writeln!(out, "  {:<20}  {:>8}", &nodes[i], out_deg[i]);
                }
            }
            // Connectivity
            let comps = connected_components(&adj, n, directed);
            let _ = writeln!(out, "\n  Components: {}  |  {}",
                comps.len(),
                if comps.len() == 1 { "connected".to_string() } else { "disconnected".to_string() });
            // Check for cycles via DFS
            let has_cycle = detect_cycle(&adj, n, directed);
            let _ = writeln!(out, "  Cycles: {}", if has_cycle { "yes" } else { "none detected" });
            if directed {
                match topo_sort(&adj, n) {
                    Ok(order) => {
                        let _ = writeln!(out, "  Topo order: {}", order.iter().map(|&i| nodes[i].as_str()).collect::<Vec<_>>().join(" → "));
                    }
                    Err(_) => {}
                }
            }
        }
    }

    let _ = writeln!(out, "{}", "=".repeat(w));
    out
}

fn parse_node_weight(s: &str) -> (&str, f64) {
    // "NodeName:weight" or "NodeName weight"
    if let Some(pos) = s.find(':') {
        let name = &s[..pos];
        let w = s[pos+1..].trim().parse::<f64>().unwrap_or(1.0);
        (name.trim(), w)
    } else {
        let parts: Vec<&str> = s.splitn(2, char::is_whitespace).collect();
        let name = parts[0].trim();
        let w = parts.get(1).and_then(|x| x.trim().parse::<f64>().ok()).unwrap_or(1.0);
        (name, w)
    }
}

fn bfs_order(adj: &[Vec<(usize, f64)>], start: usize, n: usize) -> Vec<usize> {
    let mut visited = vec![false; n];
    let mut queue = std::collections::VecDeque::new();
    let mut order = Vec::new();
    visited[start] = true;
    queue.push_back(start);
    while let Some(u) = queue.pop_front() {
        order.push(u);
        let mut nbrs: Vec<usize> = adj[u].iter().map(|&(v,_)| v).collect();
        nbrs.sort();
        for v in nbrs {
            if !visited[v] { visited[v] = true; queue.push_back(v); }
        }
    }
    order
}

fn dfs_order(adj: &[Vec<(usize, f64)>], start: usize, n: usize) -> Vec<usize> {
    let mut visited = vec![false; n];
    let mut stack = vec![start];
    let mut order = Vec::new();
    while let Some(u) = stack.pop() {
        if visited[u] { continue; }
        visited[u] = true;
        order.push(u);
        let mut nbrs: Vec<usize> = adj[u].iter().map(|&(v,_)| v).collect();
        nbrs.sort_by(|a, b| b.cmp(a));
        for v in nbrs { if !visited[v] { stack.push(v); } }
    }
    order
}

fn dijkstra(adj: &[Vec<(usize, f64)>], from: usize, to: usize, n: usize) -> Option<(f64, Vec<usize>)> {
    use std::collections::BinaryHeap;
    use std::cmp::Ordering;
    #[derive(PartialEq)]
    struct State { cost: f64, node: usize }
    impl Eq for State {}
    impl PartialOrd for State {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
    }
    impl Ord for State {
        fn cmp(&self, other: &Self) -> Ordering {
            other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
        }
    }

    let mut dist = vec![f64::INFINITY; n];
    let mut prev = vec![usize::MAX; n];
    dist[from] = 0.0;
    let mut heap = BinaryHeap::new();
    heap.push(State { cost: 0.0, node: from });

    while let Some(State { cost, node }) = heap.pop() {
        if node == to { break; }
        if cost > dist[node] { continue; }
        for &(v, w) in &adj[node] {
            let next_cost = dist[node] + w;
            if next_cost < dist[v] {
                dist[v] = next_cost;
                prev[v] = node;
                heap.push(State { cost: next_cost, node: v });
            }
        }
    }

    if dist[to] == f64::INFINITY { return None; }
    let mut path = Vec::new();
    let mut cur = to;
    while cur != usize::MAX {
        path.push(cur);
        cur = prev[cur];
    }
    path.reverse();
    Some((dist[to], path))
}

fn dijkstra_all(adj: &[Vec<(usize, f64)>], from: usize, n: usize) -> Vec<f64> {
    use std::collections::BinaryHeap;
    use std::cmp::Ordering;
    #[derive(PartialEq)]
    struct State { cost: f64, node: usize }
    impl Eq for State {}
    impl PartialOrd for State {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
    }
    impl Ord for State {
        fn cmp(&self, other: &Self) -> Ordering {
            other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
        }
    }
    let mut dist = vec![f64::INFINITY; n];
    dist[from] = 0.0;
    let mut heap = BinaryHeap::new();
    heap.push(State { cost: 0.0, node: from });
    while let Some(State { cost, node }) = heap.pop() {
        if cost > dist[node] { continue; }
        for &(v, w) in &adj[node] {
            let nc = dist[node] + w;
            if nc < dist[v] { dist[v] = nc; heap.push(State { cost: nc, node: v }); }
        }
    }
    dist
}

fn connected_components(adj: &[Vec<(usize, f64)>], n: usize, directed: bool) -> Vec<Vec<usize>> {
    // For directed graphs, use weakly connected components (ignore direction)
    let mut visited = vec![false; n];
    let mut comps = Vec::new();
    for start in 0..n {
        if visited[start] { continue; }
        let mut comp = Vec::new();
        let mut stack = vec![start];
        while let Some(u) = stack.pop() {
            if visited[u] { continue; }
            visited[u] = true;
            comp.push(u);
            for &(v, _) in &adj[u] {
                if !visited[v] { stack.push(v); }
            }
            if directed {
                // Also traverse reverse edges for weak connectivity
                for other in 0..n {
                    if !visited[other] && adj[other].iter().any(|&(t,_)| t == u) {
                        stack.push(other);
                    }
                }
            }
        }
        comp.sort();
        comps.push(comp);
    }
    comps
}

fn topo_sort(adj: &[Vec<(usize, f64)>], n: usize) -> Result<Vec<usize>, ()> {
    let mut in_deg = vec![0usize; n];
    for u in 0..n {
        for &(v,_) in &adj[u] { in_deg[v] += 1; }
    }
    let mut queue: std::collections::VecDeque<usize> = (0..n).filter(|&i| in_deg[i]==0).collect();
    let mut order = Vec::new();
    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &(v,_) in &adj[u] {
            in_deg[v] -= 1;
            if in_deg[v] == 0 { queue.push_back(v); }
        }
    }
    if order.len() == n { Ok(order) } else { Err(()) }
}

fn detect_cycle(adj: &[Vec<(usize, f64)>], n: usize, directed: bool) -> bool {
    // DFS-based cycle detection
    let mut color = vec![0u8; n]; // 0=white 1=gray 2=black
    fn dfs_cycle(u: usize, adj: &[Vec<(usize, f64)>], color: &mut Vec<u8>, directed: bool, parent: usize) -> bool {
        color[u] = 1;
        for &(v, _) in &adj[u] {
            if color[v] == 0 {
                if dfs_cycle(v, adj, color, directed, u) { return true; }
            } else if directed && color[v] == 1 {
                return true;
            } else if !directed && v != parent {
                return true;
            }
        }
        color[u] = 2;
        false
    }
    for start in 0..n {
        if color[start] == 0 {
            if dfs_cycle(start, adj, &mut color, directed, usize::MAX) { return true; }
        }
    }
    false
}

fn graph_usage() -> String {
    "Graph theory — edge list input:\n\
     hematite --graph 'A B\\nB C\\nC D'                  info (degree table, components)\n\
     hematite --graph 'bfs A\\nA B\\nB C\\nA C'           BFS from node A\n\
     hematite --graph 'dfs A\\nA B\\nB C\\nA C'           DFS from node A\n\
     hematite --graph 'shortest A D\\nA B 2\\nB D 3\\nA D 10'  Dijkstra shortest path\n\
     hematite --graph 'components\\nA B\\nC D'            connected components\n\
     hematite --graph 'topo\\nA->B\\nA->C\\nB->D'          topological sort\n\
     \n\
     Edge formats: 'A B' 'A B 5' 'A->B' 'A->B:5' 'A-B:3'\n\
     Weighted edges: add weight as third token or after colon".into()
}

// ── Symbolic calculus ─────────────────────────────────────────────────────────
// Recursive-descent parser → AST → symbolic diff/integrate → pretty-printer.
// Supported: +  -  *  /  ^  unary-  sin  cos  tan  ln  log  exp  sqrt  abs
// Variable: default x, overridable with "wrt y" suffix.
//
// Modes:
//   diff EXPR [wrt VAR]      symbolic derivative
//   integrate EXPR [wrt VAR] symbolic integral (table lookup + linearity)
//   simplify EXPR            simplify/reduce
//   eval EXPR at VAR=VALUE   numeric evaluation

#[derive(Clone, Debug)]
enum Expr {
    Num(f64),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Sin(Box<Expr>),
    Cos(Box<Expr>),
    Tan(Box<Expr>),
    Ln(Box<Expr>),
    Exp(Box<Expr>),
    Sqrt(Box<Expr>),
    Abs(Box<Expr>),
}

// ── Parser ─────────────────────────────────────────────────────────────────

struct Parser<'a> {
    chars: &'a [char],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(chars: &'a [char]) -> Self { Self { chars, pos: 0 } }

    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }

    fn consume(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) { self.pos += 1; }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> { self.parse_add() }

    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_mul()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('+') => { self.consume(); let r = self.parse_mul()?; left = Expr::Add(Box::new(left), Box::new(r)); }
                Some('-') => { self.consume(); let r = self.parse_mul()?; left = Expr::Sub(Box::new(left), Box::new(r)); }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_pow()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('*') => { self.consume(); let r = self.parse_pow()?; left = Expr::Mul(Box::new(left), Box::new(r)); }
                Some('/') => { self.consume(); let r = self.parse_pow()?; left = Expr::Div(Box::new(left), Box::new(r)); }
                // Implicit multiplication: if next token is a function or '(' or var
                Some(c) if c.is_alphabetic() || c == '(' => {
                    let r = self.parse_pow()?;
                    left = Expr::Mul(Box::new(left), Box::new(r));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_pow(&mut self) -> Result<Expr, String> {
        let base = self.parse_unary()?;
        self.skip_ws();
        if self.peek() == Some('^') {
            self.consume();
            let exp = self.parse_unary()?;
            return Ok(Expr::Pow(Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        self.skip_ws();
        if self.peek() == Some('-') {
            self.consume();
            let inner = self.parse_atom()?;
            return Ok(Expr::Neg(Box::new(inner)));
        }
        if self.peek() == Some('+') { self.consume(); }
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Result<Expr, String> {
        self.skip_ws();
        match self.peek() {
            Some('(') => {
                self.consume();
                let inner = self.parse_expr()?;
                self.skip_ws();
                if self.peek() == Some(')') { self.consume(); }
                Ok(inner)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.parse_number(),
            Some(c) if c.is_alphabetic() || c == '_' => self.parse_name(),
            Some(c) => Err(format!("unexpected char '{}'", c)),
            None => Err("unexpected end".into()),
        }
    }

    fn parse_number(&mut self) -> Result<Expr, String> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E') {
            self.pos += 1;
            // handle e+/e-
            if matches!(self.chars.get(self.pos-1), Some('e') | Some('E')) {
                if matches!(self.peek(), Some('+') | Some('-')) { self.pos += 1; }
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        s.parse::<f64>().map(Expr::Num).map_err(|_| format!("bad number: {}", s))
    }

    fn parse_name(&mut self) -> Result<Expr, String> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') { self.pos += 1; }
        let name: String = self.chars[start..self.pos].iter().collect();
        self.skip_ws();
        // Check if it's a function call
        if self.peek() == Some('(') {
            self.consume();
            let arg = self.parse_expr()?;
            self.skip_ws();
            if self.peek() == Some(')') { self.consume(); }
            let e = Box::new(arg);
            return match name.to_lowercase().as_str() {
                "sin"  => Ok(Expr::Sin(e)),
                "cos"  => Ok(Expr::Cos(e)),
                "tan"  => Ok(Expr::Tan(e)),
                "ln"   => Ok(Expr::Ln(e)),
                "log"  => Ok(Expr::Ln(e)),   // treat log as natural log
                "exp"  => Ok(Expr::Exp(e)),
                "sqrt" => Ok(Expr::Sqrt(e)),
                "abs"  => Ok(Expr::Abs(e)),
                _ => Err(format!("unknown function: {}", name)),
            };
        }
        // Constants
        match name.as_str() {
            "pi" | "PI" => return Ok(Expr::Num(std::f64::consts::PI)),
            "e"  | "E"  => return Ok(Expr::Num(std::f64::consts::E)),
            _ => {}
        }
        Ok(Expr::Var(name))
    }
}

fn parse_sym(s: &str) -> Result<Expr, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut p = Parser::new(&chars);
    let e = p.parse_expr()?;
    p.skip_ws();
    if p.pos < p.chars.len() {
        // Tolerate trailing whitespace/comments
        let rest: String = p.chars[p.pos..].iter().collect();
        if !rest.trim().is_empty() {
            return Err(format!("unexpected trailing: '{}'", rest.trim()));
        }
    }
    Ok(e)
}

// ── Pretty-printer ──────────────────────────────────────────────────────────

fn fmt_expr(e: &Expr) -> String {
    match e {
        Expr::Num(n) => {
            if n.fract() == 0.0 && n.abs() < 1e12 { format!("{}", *n as i64) }
            else { format!("{}", n) }
        }
        Expr::Var(v) => v.clone(),
        Expr::Add(a, b) => format!("({} + {})", fmt_expr(a), fmt_expr(b)),
        Expr::Sub(a, b) => format!("({} - {})", fmt_expr(a), fmt_expr(b)),
        Expr::Mul(a, b) => format!("({} * {})", fmt_expr(a), fmt_expr(b)),
        Expr::Div(a, b) => format!("({} / {})", fmt_expr(a), fmt_expr(b)),
        Expr::Pow(a, b) => format!("({}^{})", fmt_expr(a), fmt_expr(b)),
        Expr::Neg(a)    => format!("(-{})", fmt_expr(a)),
        Expr::Sin(a)    => format!("sin({})", fmt_expr(a)),
        Expr::Cos(a)    => format!("cos({})", fmt_expr(a)),
        Expr::Tan(a)    => format!("tan({})", fmt_expr(a)),
        Expr::Ln(a)     => format!("ln({})", fmt_expr(a)),
        Expr::Exp(a)    => format!("exp({})", fmt_expr(a)),
        Expr::Sqrt(a)   => format!("sqrt({})", fmt_expr(a)),
        Expr::Abs(a)    => format!("abs({})", fmt_expr(a)),
    }
}

// ── Simplification ─────────────────────────────────────────────────────────
// Single-pass algebraic simplification: fold constants, remove identity ops.

fn simplify(e: Expr) -> Expr {
    match e {
        Expr::Add(a, b) => {
            let a = simplify(*a); let b = simplify(*b);
            match (&a, &b) {
                (Expr::Num(x), Expr::Num(y)) => Expr::Num(x + y),
                (Expr::Num(0.0), _) => b,
                (_, Expr::Num(0.0)) => a,
                _ => Expr::Add(Box::new(a), Box::new(b)),
            }
        }
        Expr::Sub(a, b) => {
            let a = simplify(*a); let b = simplify(*b);
            match (&a, &b) {
                (Expr::Num(x), Expr::Num(y)) => Expr::Num(x - y),
                (_, Expr::Num(0.0)) => a,
                _ if fmt_expr(&a) == fmt_expr(&b) => Expr::Num(0.0),
                _ => Expr::Sub(Box::new(a), Box::new(b)),
            }
        }
        Expr::Mul(a, b) => {
            let a = simplify(*a); let b = simplify(*b);
            match (&a, &b) {
                (Expr::Num(x), Expr::Num(y)) => Expr::Num(x * y),
                (Expr::Num(0.0), _) | (_, Expr::Num(0.0)) => Expr::Num(0.0),
                (Expr::Num(1.0), _) => b,
                (_, Expr::Num(1.0)) => a,
                (Expr::Num(-1.0), _) => Expr::Neg(Box::new(b)),
                _ => Expr::Mul(Box::new(a), Box::new(b)),
            }
        }
        Expr::Div(a, b) => {
            let a = simplify(*a); let b = simplify(*b);
            match (&a, &b) {
                (_, Expr::Num(1.0)) => a,
                (Expr::Num(x), Expr::Num(y)) if *y != 0.0 => Expr::Num(x / y),
                _ => Expr::Div(Box::new(a), Box::new(b)),
            }
        }
        Expr::Pow(a, b) => {
            let a = simplify(*a); let b = simplify(*b);
            match (&a, &b) {
                (_, Expr::Num(0.0)) => Expr::Num(1.0),
                (_, Expr::Num(1.0)) => a,
                (Expr::Num(1.0), _) => Expr::Num(1.0),
                (Expr::Num(x), Expr::Num(y)) => Expr::Num(x.powf(*y)),
                _ => Expr::Pow(Box::new(a), Box::new(b)),
            }
        }
        Expr::Neg(a) => {
            let a = simplify(*a);
            match a {
                Expr::Num(n) => Expr::Num(-n),
                Expr::Neg(inner) => *inner,
                _ => Expr::Neg(Box::new(a)),
            }
        }
        Expr::Sin(a) => { let a = simplify(*a); Expr::Sin(Box::new(a)) }
        Expr::Cos(a) => { let a = simplify(*a); Expr::Cos(Box::new(a)) }
        Expr::Tan(a) => { let a = simplify(*a); Expr::Tan(Box::new(a)) }
        Expr::Ln(a)  => {
            let a = simplify(*a);
            if let Expr::Num(n) = &a { if (*n - std::f64::consts::E).abs() < 1e-12 { return Expr::Num(1.0); } }
            Expr::Ln(Box::new(a))
        }
        Expr::Exp(a) => {
            let a = simplify(*a);
            if let Expr::Num(n) = &a { return Expr::Num(n.exp()); }
            if let Expr::Num(0.0) = &a { return Expr::Num(1.0); }
            Expr::Exp(Box::new(a))
        }
        Expr::Sqrt(a) => {
            let a = simplify(*a);
            if let Expr::Num(n) = &a { if *n >= 0.0 { return Expr::Num(n.sqrt()); } }
            Expr::Sqrt(Box::new(a))
        }
        other => other,
    }
}

// ── Differentiation ────────────────────────────────────────────────────────

fn diff(e: &Expr, var: &str) -> Expr {
    match e {
        Expr::Num(_) => Expr::Num(0.0),
        Expr::Var(v) => if v == var { Expr::Num(1.0) } else { Expr::Num(0.0) },
        Expr::Add(a, b) => simplify(Expr::Add(Box::new(diff(a, var)), Box::new(diff(b, var)))),
        Expr::Sub(a, b) => simplify(Expr::Sub(Box::new(diff(a, var)), Box::new(diff(b, var)))),
        Expr::Mul(a, b) => {
            // product rule: f'g + fg'
            let fp_g = Expr::Mul(Box::new(diff(a, var)), Box::new(*b.clone()));
            let f_gp = Expr::Mul(Box::new(*a.clone()), Box::new(diff(b, var)));
            simplify(Expr::Add(Box::new(fp_g), Box::new(f_gp)))
        }
        Expr::Div(a, b) => {
            // quotient rule: (f'g - fg') / g^2
            let fp_g = Expr::Mul(Box::new(diff(a, var)), Box::new(*b.clone()));
            let f_gp = Expr::Mul(Box::new(*a.clone()), Box::new(diff(b, var)));
            let num  = Expr::Sub(Box::new(fp_g), Box::new(f_gp));
            let den  = Expr::Pow(Box::new(*b.clone()), Box::new(Expr::Num(2.0)));
            simplify(Expr::Div(Box::new(num), Box::new(den)))
        }
        Expr::Pow(base, exp) => {
            // If exp is a constant: power rule n*x^(n-1) * base'
            if let Expr::Num(n) = exp.as_ref() {
                let new_exp = Expr::Num(n - 1.0);
                let power   = Expr::Pow(Box::new(*base.clone()), Box::new(new_exp));
                let coeff   = Expr::Mul(Box::new(Expr::Num(*n)), Box::new(power));
                let chain   = diff(base, var);
                return simplify(Expr::Mul(Box::new(coeff), Box::new(chain)));
            }
            // General: e^(exp * ln(base)) rule: f^g * (g' ln f + g f'/f)
            let ln_base = Expr::Ln(Box::new(*base.clone()));
            let g_ln_f  = Expr::Mul(Box::new(*exp.clone()), Box::new(ln_base));
            let g_ln_f_d = diff(&g_ln_f, var);
            let result  = Expr::Mul(Box::new(e.clone()), Box::new(g_ln_f_d));
            simplify(result)
        }
        Expr::Neg(a) => simplify(Expr::Neg(Box::new(diff(a, var)))),
        Expr::Sin(a)  => {
            let cos_a = Expr::Cos(Box::new(*a.clone()));
            simplify(Expr::Mul(Box::new(cos_a), Box::new(diff(a, var))))
        }
        Expr::Cos(a)  => {
            let neg_sin = Expr::Neg(Box::new(Expr::Sin(Box::new(*a.clone()))));
            simplify(Expr::Mul(Box::new(neg_sin), Box::new(diff(a, var))))
        }
        Expr::Tan(a)  => {
            // sec^2(a) * a' = 1/cos^2(a) * a'
            let cos_a = Expr::Cos(Box::new(*a.clone()));
            let cos2  = Expr::Pow(Box::new(cos_a), Box::new(Expr::Num(2.0)));
            let sec2  = Expr::Div(Box::new(Expr::Num(1.0)), Box::new(cos2));
            simplify(Expr::Mul(Box::new(sec2), Box::new(diff(a, var))))
        }
        Expr::Ln(a) => {
            // 1/a * a'
            let inv_a = Expr::Div(Box::new(Expr::Num(1.0)), Box::new(*a.clone()));
            simplify(Expr::Mul(Box::new(inv_a), Box::new(diff(a, var))))
        }
        Expr::Exp(a) => {
            // exp(a) * a'
            simplify(Expr::Mul(Box::new(e.clone()), Box::new(diff(a, var))))
        }
        Expr::Sqrt(a) => {
            // 1/(2*sqrt(a)) * a'
            let two_sqrt = Expr::Mul(Box::new(Expr::Num(2.0)), Box::new(Expr::Sqrt(Box::new(*a.clone()))));
            let inv      = Expr::Div(Box::new(Expr::Num(1.0)), Box::new(two_sqrt));
            simplify(Expr::Mul(Box::new(inv), Box::new(diff(a, var))))
        }
        Expr::Abs(a)  => {
            // d/dx |a| = a/|a| * a'  (sign(a) * a')
            let sign = Expr::Div(Box::new(*a.clone()), Box::new(Expr::Abs(Box::new(*a.clone()))));
            simplify(Expr::Mul(Box::new(sign), Box::new(diff(a, var))))
        }
    }
}

// ── Integration (table lookup + linearity) ──────────────────────────────────
// Returns Some(integral) for forms in the table, None for unknowns.
// Adds no "+ C" — the caller adds it.

fn integrate(e: &Expr, var: &str) -> Option<Expr> {
    match e {
        Expr::Num(n) => {
            // ∫ n dx = n*x
            Some(Expr::Mul(Box::new(Expr::Num(*n)), Box::new(Expr::Var(var.to_string()))))
        }
        Expr::Var(v) => {
            if v == var {
                // ∫ x dx = x^2/2
                Some(Expr::Div(
                    Box::new(Expr::Pow(Box::new(Expr::Var(v.clone())), Box::new(Expr::Num(2.0)))),
                    Box::new(Expr::Num(2.0)),
                ))
            } else {
                // ∫ c dx where c is another variable treated as constant
                Some(Expr::Mul(Box::new(Expr::Var(v.clone())), Box::new(Expr::Var(var.to_string()))))
            }
        }
        Expr::Add(a, b) => {
            let ia = integrate(a, var)?;
            let ib = integrate(b, var)?;
            Some(simplify(Expr::Add(Box::new(ia), Box::new(ib))))
        }
        Expr::Sub(a, b) => {
            let ia = integrate(a, var)?;
            let ib = integrate(b, var)?;
            Some(simplify(Expr::Sub(Box::new(ia), Box::new(ib))))
        }
        Expr::Neg(a) => {
            let ia = integrate(a, var)?;
            Some(simplify(Expr::Neg(Box::new(ia))))
        }
        Expr::Mul(a, b) => {
            // c * f(x) where c is constant
            if !contains_var(a, var) {
                let ib = integrate(b, var)?;
                return Some(simplify(Expr::Mul(Box::new(*a.clone()), Box::new(ib))));
            }
            if !contains_var(b, var) {
                let ia = integrate(a, var)?;
                return Some(simplify(Expr::Mul(Box::new(*b.clone()), Box::new(ia))));
            }
            None // general product — no IBP
        }
        Expr::Pow(base, exp) => {
            if let Expr::Var(v) = base.as_ref() {
                if v == var {
                    if let Expr::Num(n) = exp.as_ref() {
                        if (*n + 1.0).abs() < 1e-12 {
                            // ∫ x^-1 = ln|x|
                            return Some(Expr::Ln(Box::new(Expr::Abs(Box::new(Expr::Var(v.clone()))))));
                        }
                        // ∫ x^n = x^(n+1)/(n+1)
                        let new_exp = Expr::Num(n + 1.0);
                        let pow     = Expr::Pow(Box::new(Expr::Var(v.clone())), Box::new(new_exp.clone()));
                        return Some(simplify(Expr::Div(Box::new(pow), Box::new(new_exp))));
                    }
                }
            }
            None
        }
        Expr::Sin(a) => {
            if let Expr::Var(v) = a.as_ref() {
                if v == var {
                    return Some(Expr::Neg(Box::new(Expr::Cos(Box::new(*a.clone())))));
                }
            }
            // ∫ sin(n*x) = -cos(n*x)/n
            if let Some((coeff, _inner_var)) = linear_coeff(a, var) {
                let cos_part = Expr::Cos(Box::new(*a.clone()));
                let neg_cos  = Expr::Neg(Box::new(cos_part));
                return Some(simplify(Expr::Div(Box::new(neg_cos), Box::new(Expr::Num(coeff)))));
            }
            None
        }
        Expr::Cos(a) => {
            if let Expr::Var(v) = a.as_ref() {
                if v == var {
                    return Some(Expr::Sin(Box::new(*a.clone())));
                }
            }
            if let Some((coeff, _)) = linear_coeff(a, var) {
                let sin_part = Expr::Sin(Box::new(*a.clone()));
                return Some(simplify(Expr::Div(Box::new(sin_part), Box::new(Expr::Num(coeff)))));
            }
            None
        }
        Expr::Exp(a) => {
            if let Expr::Var(v) = a.as_ref() {
                if v == var {
                    return Some(e.clone()); // ∫ e^x = e^x
                }
            }
            if let Some((coeff, _)) = linear_coeff(a, var) {
                return Some(simplify(Expr::Div(Box::new(e.clone()), Box::new(Expr::Num(coeff)))));
            }
            None
        }
        Expr::Ln(a) => {
            if let Expr::Var(v) = a.as_ref() {
                if v == var {
                    // ∫ ln(x) = x*ln(x) - x
                    let x_ln_x = Expr::Mul(Box::new(Expr::Var(v.clone())), Box::new(e.clone()));
                    return Some(simplify(Expr::Sub(Box::new(x_ln_x), Box::new(Expr::Var(v.clone())))));
                }
            }
            None
        }
        Expr::Div(a, b) => {
            // ∫ 1/x = ln|x|
            if let (Expr::Num(1.0), Expr::Var(v)) = (a.as_ref(), b.as_ref()) {
                if v == var {
                    return Some(Expr::Ln(Box::new(Expr::Abs(Box::new(Expr::Var(v.clone()))))));
                }
            }
            // ∫ c/x = c*ln|x|
            if let Expr::Var(v) = b.as_ref() {
                if v == var && !contains_var(a, var) {
                    let ln_abs = Expr::Ln(Box::new(Expr::Abs(Box::new(Expr::Var(v.clone())))));
                    return Some(simplify(Expr::Mul(Box::new(*a.clone()), Box::new(ln_abs))));
                }
            }
            None
        }
        _ => None,
    }
}

// Returns Some((coeff, var)) if expr = coeff * var + constant (linear in var)
fn linear_coeff<'a>(e: &'a Expr, var: &'a str) -> Option<(f64, &'a str)> {
    match e {
        Expr::Mul(a, b) => {
            if let (Expr::Num(c), Expr::Var(v)) = (a.as_ref(), b.as_ref()) {
                if v == var { return Some((*c, var)); }
            }
            if let (Expr::Var(v), Expr::Num(c)) = (a.as_ref(), b.as_ref()) {
                if v == var { return Some((*c, var)); }
            }
            None
        }
        _ => None,
    }
}

fn contains_var(e: &Expr, var: &str) -> bool {
    match e {
        Expr::Var(v) => v == var,
        Expr::Num(_) => false,
        Expr::Add(a,b)|Expr::Sub(a,b)|Expr::Mul(a,b)|Expr::Div(a,b)|Expr::Pow(a,b) =>
            contains_var(a,var) || contains_var(b,var),
        Expr::Neg(a)|Expr::Sin(a)|Expr::Cos(a)|Expr::Tan(a)|Expr::Ln(a)|Expr::Exp(a)|Expr::Sqrt(a)|Expr::Abs(a) =>
            contains_var(a, var),
    }
}

// ── Numeric eval ──────────────────────────────────────────────────────────

fn eval_expr(e: &Expr, var: &str, val: f64) -> Result<f64, String> {
    match e {
        Expr::Num(n) => Ok(*n),
        Expr::Var(v) => if v == var { Ok(val) } else { Err(format!("unbound variable: {}", v)) },
        Expr::Add(a,b) => Ok(eval_expr(a,var,val)? + eval_expr(b,var,val)?),
        Expr::Sub(a,b) => Ok(eval_expr(a,var,val)? - eval_expr(b,var,val)?),
        Expr::Mul(a,b) => Ok(eval_expr(a,var,val)? * eval_expr(b,var,val)?),
        Expr::Div(a,b) => {
            let d = eval_expr(b,var,val)?;
            if d.abs() < 1e-300 { return Err("division by zero".into()); }
            Ok(eval_expr(a,var,val)? / d)
        }
        Expr::Pow(a,b) => Ok(eval_expr(a,var,val)?.powf(eval_expr(b,var,val)?)),
        Expr::Neg(a)   => Ok(-eval_expr(a,var,val)?),
        Expr::Sin(a)   => Ok(eval_expr(a,var,val)?.sin()),
        Expr::Cos(a)   => Ok(eval_expr(a,var,val)?.cos()),
        Expr::Tan(a)   => Ok(eval_expr(a,var,val)?.tan()),
        Expr::Ln(a)    => Ok(eval_expr(a,var,val)?.ln()),
        Expr::Exp(a)   => Ok(eval_expr(a,var,val)?.exp()),
        Expr::Sqrt(a)  => Ok(eval_expr(a,var,val)?.sqrt()),
        Expr::Abs(a)   => Ok(eval_expr(a,var,val)?.abs()),
    }
}

// ── Public entry point ────────────────────────────────────────────────────

pub fn symbolic_calc(query: &str) -> String {
    let q = query.trim();

    // Parse optional "wrt VAR" suffix
    let (q_body, var) = if let Some(pos) = q.to_lowercase().rfind(" wrt ") {
        let v = q[pos+5..].trim().to_string();
        (q[..pos].trim(), v)
    } else {
        (q, "x".to_string())
    };

    // Parse mode
    let (mode, expr_str) = {
        let low = q_body.to_lowercase();
        if low.starts_with("diff ") || low.starts_with("differentiate ") || low.starts_with("d/dx ") || low.starts_with("d/d") {
            // d/dy expr — extract var from d/dy if present
            let (m, rest, var_from_mode) = if low.starts_with("d/d") {
                let after = &q_body[3..];
                let sp = after.find(char::is_whitespace).unwrap_or(after.len());
                let v = after[..sp].to_string();
                let rest = after[sp..].trim();
                ("diff", rest, Some(v))
            } else {
                let rest = q_body.splitn(2, char::is_whitespace).nth(1).unwrap_or("").trim();
                ("diff", rest, None)
            };
            let var2 = var_from_mode.unwrap_or_else(|| var.clone());
            (m, (rest.to_string(), var2))
        } else if low.starts_with("int ") || low.starts_with("integrate ") || low.starts_with("∫") {
            let rest = q_body.splitn(2, char::is_whitespace).nth(1).unwrap_or("").trim();
            ("integrate", (rest.to_string(), var.clone()))
        } else if low.starts_with("simplify ") || low.starts_with("simplify") {
            let rest = q_body.splitn(2, char::is_whitespace).nth(1).unwrap_or("").trim();
            ("simplify", (rest.to_string(), var.clone()))
        } else if low.contains(" at ") {
            ("eval", (q_body.to_string(), var.clone()))
        } else {
            // Default: try to detect if expr contains d/dx or integral sign
            ("diff", (q_body.to_string(), var.clone()))
        }
    };

    let (expr_text, var_name) = expr_str;
    let var_name = var_name.trim().to_string();
    let var_name = if var_name.is_empty() { "x".to_string() } else { var_name };

    let mut out = String::new();
    let w = 64usize;
    let _ = writeln!(out, "{}", "=".repeat(w));
    let _ = writeln!(out, "  Symbolic Calculus");

    if mode == "eval" {
        // "expr at var=value"
        let parts: Vec<&str> = expr_text.splitn(2, " at ").collect();
        if parts.len() != 2 {
            let _ = writeln!(out, "  Error: use 'EXPR at VAR=VALUE'");
            return out;
        }
        let e_str = parts[0].trim();
        let at_str = parts[1].trim();
        let (av, val_str) = if let Some(eq) = at_str.find('=') {
            (&at_str[..eq], &at_str[eq+1..])
        } else {
            (&var_name[..], at_str)
        };
        let val: f64 = match val_str.trim().parse() {
            Ok(v) => v, Err(_) => { let _ = writeln!(out, "  Error: bad value '{}'", val_str); return out; }
        };
        match parse_sym(e_str) {
            Ok(expr) => {
                let _ = writeln!(out, "  f({}) = {}", av, fmt_expr(&expr));
                match eval_expr(&expr, av.trim(), val) {
                    Ok(result) => { let _ = writeln!(out, "  f({} = {}) = {}", av.trim(), val, result); }
                    Err(e) => { let _ = writeln!(out, "  Eval error: {}", e); }
                }
            }
            Err(e) => { let _ = writeln!(out, "  Parse error: {}", e); }
        }
        let _ = writeln!(out, "{}", "=".repeat(w));
        return out;
    }

    let expr_text = expr_text.trim();
    match parse_sym(expr_text) {
        Err(e) => {
            let _ = writeln!(out, "  Parse error: {}", e);
            let _ = writeln!(out, "  Input: {}", expr_text);
            let _ = writeln!(out, "{}", "=".repeat(w));
            return out;
        }
        Ok(expr) => {
            let simplified = simplify(expr.clone());
            let _ = writeln!(out, "  f({}) = {}", var_name, fmt_expr(&simplified));
            match mode {
                "diff" => {
                    let d = diff(&simplified, &var_name);
                    let d_simp = simplify(d);
                    let _ = writeln!(out, "  d/d{} = {}", var_name, fmt_expr(&d_simp));
                    // Spot-check with numeric diff at x=1.5
                    let h = 1e-6f64;
                    let x0 = 1.5f64;
                    if let (Ok(fp), Ok(fm)) = (eval_expr(&simplified, &var_name, x0+h), eval_expr(&simplified, &var_name, x0-h)) {
                        let numeric = (fp - fm) / (2.0 * h);
                        if let Ok(symbolic_val) = eval_expr(&d_simp, &var_name, x0) {
                            let err = (symbolic_val - numeric).abs();
                            if err < 1e-4 {
                                let _ = writeln!(out, "  ✓ Verified: numeric check at {}={} → diff={:.6}, numeric={:.6}", var_name, x0, symbolic_val, numeric);
                            } else {
                                let _ = writeln!(out, "  ⚠ Numeric check mismatch at {}={}: symbolic={:.6}, numeric={:.6}", var_name, x0, symbolic_val, numeric);
                            }
                        }
                    }
                }
                "integrate" => {
                    match integrate(&simplified, &var_name) {
                        Some(integral) => {
                            let i_simp = simplify(integral);
                            let _ = writeln!(out, "  ∫f d{} = {} + C", var_name, fmt_expr(&i_simp));
                            // Verify by differentiating the result
                            let check = simplify(diff(&i_simp, &var_name));
                            let orig  = fmt_expr(&simplified);
                            let back  = fmt_expr(&check);
                            if orig == back {
                                let _ = writeln!(out, "  ✓ Verified: d/d{} of integral = f", var_name);
                            } else {
                                // Numeric check at a sample point
                                let x0 = 1.5f64;
                                if let (Ok(v1), Ok(v2)) = (eval_expr(&simplified, &var_name, x0), eval_expr(&check, &var_name, x0)) {
                                    if (v1 - v2).abs() < 1e-6 {
                                        let _ = writeln!(out, "  ✓ Numerically verified: d/d{}(integral) = f at {}={}", var_name, var_name, x0);
                                    } else {
                                        let _ = writeln!(out, "  ⚠ Verification: d/d{}(integral) = {} (expected {})", var_name, back, orig);
                                    }
                                }
                            }
                        }
                        None => {
                            let _ = writeln!(out, "  ∫f d{} = (not in table — try --simulate or a CAS)", var_name);
                        }
                    }
                }
                "simplify" => {
                    let _ = writeln!(out, "  simplified: {}", fmt_expr(&simplified));
                }
                _ => {}
            }
        }
    }

    let _ = writeln!(out, "{}", "=".repeat(w));
    out
}

#[allow(dead_code)]
fn symbolic_usage() -> String {
    "Symbolic calculus:\n\
     hematite --symbolic 'diff x^3 + 2*x'          differentiate (wrt x)\n\
     hematite --symbolic 'diff sin(x)*cos(x)'        product rule\n\
     hematite --symbolic 'diff x^2 + y wrt y'        differentiate wrt y\n\
     hematite --symbolic 'integrate x^3'             antiderivative\n\
     hematite --symbolic 'integrate sin(x) + 3*x^2' linearity\n\
     hematite --symbolic 'simplify (x+1)*(x+1)'      simplify\n\
     hematite --symbolic 'x^2 + 2*x at x=3'          numeric eval\n\
     Supported: + - * / ^ sin cos tan ln exp sqrt abs".into()
}

// ── Signal processing (DSP) ───────────────────────────────────────────────────
// DFT/IDFT, convolution, cross-correlation, moving average,
// FIR window filter design, waveform generation, RMS/SNR stats.
// All pure-Rust — no Python subprocess, no external crates.

use std::f64::consts::PI;

fn dft(signal: &[f64]) -> Vec<(f64, f64)> {
    let n = signal.len();
    (0..n).map(|k| {
        let (mut re, mut im) = (0.0_f64, 0.0_f64);
        for (t, &x) in signal.iter().enumerate() {
            let angle = -2.0 * PI * (k * t) as f64 / n as f64;
            re += x * angle.cos();
            im += x * angle.sin();
        }
        (re, im)
    }).collect()
}

fn idft(spectrum: &[(f64, f64)]) -> Vec<f64> {
    let n = spectrum.len();
    (0..n).map(|t| {
        let mut val = 0.0_f64;
        for (k, &(re, im)) in spectrum.iter().enumerate() {
            let angle = 2.0 * PI * (k * t) as f64 / n as f64;
            val += re * angle.cos() - im * angle.sin();
        }
        val / n as f64
    }).collect()
}

fn convolve(x: &[f64], h: &[f64]) -> Vec<f64> {
    let n = x.len() + h.len() - 1;
    (0..n).map(|i| {
        let mut s = 0.0_f64;
        for (j, &hv) in h.iter().enumerate() {
            if i >= j && i - j < x.len() { s += x[i - j] * hv; }
        }
        s
    }).collect()
}

fn xcorr(x: &[f64], y: &[f64]) -> Vec<f64> {
    let n = x.len(); let m = y.len();
    let out_len = n + m - 1;
    (0..out_len).map(|lag| {
        let lag_i = lag as isize - (m as isize - 1);
        let mut s = 0.0_f64;
        for (i, &xv) in x.iter().enumerate() {
            let j = i as isize - lag_i;
            if j >= 0 && j < m as isize { s += xv * y[j as usize]; }
        }
        s
    }).collect()
}

fn moving_avg(signal: &[f64], window: usize) -> Vec<f64> {
    let w = window.max(1);
    signal.windows(w).map(|s| s.iter().sum::<f64>() / w as f64).collect()
}

fn hann_window(n: usize) -> Vec<f64> {
    (0..n).map(|i| 0.5 * (1.0 - (2.0 * PI * i as f64 / (n - 1) as f64).cos())).collect()
}

fn hamming_window(n: usize) -> Vec<f64> {
    (0..n).map(|i| 0.54 - 0.46 * (2.0 * PI * i as f64 / (n - 1) as f64).cos()).collect()
}

fn blackman_window(n: usize) -> Vec<f64> {
    (0..n).map(|i| {
        let a = 2.0 * PI * i as f64 / (n - 1) as f64;
        0.42 - 0.5 * a.cos() + 0.08 * (2.0 * a).cos()
    }).collect()
}

fn sinc(x: f64) -> f64 { if x == 0.0 { 1.0 } else { (PI * x).sin() / (PI * x) } }

fn fir_lowpass(n_taps: usize, cutoff_norm: f64, window: &str) -> Vec<f64> {
    let m = n_taps - 1;
    let wins: Vec<f64> = match window {
        "hann" | "hanning" => hann_window(n_taps),
        "hamming"          => hamming_window(n_taps),
        "blackman"         => blackman_window(n_taps),
        _                  => vec![1.0; n_taps],
    };
    let mut h: Vec<f64> = (0..n_taps).map(|i| {
        let n = i as f64 - m as f64 / 2.0;
        2.0 * cutoff_norm * sinc(2.0 * cutoff_norm * n) * wins[i]
    }).collect();
    let sum: f64 = h.iter().sum();
    if sum.abs() > 1e-12 { for v in &mut h { *v /= sum; } }
    h
}

fn fir_highpass(n_taps: usize, cutoff_norm: f64, window: &str) -> Vec<f64> {
    let mut h = fir_lowpass(n_taps, cutoff_norm, window);
    for (i, v) in h.iter_mut().enumerate() {
        *v = if i == n_taps / 2 { 1.0 - *v } else { -*v };
    }
    h
}

fn parse_signal(s: &str) -> Option<Vec<f64>> {
    let v: Vec<f64> = s.split([',', ' ', '\t', ';'].as_ref())
        .filter_map(|t| t.trim().parse::<f64>().ok())
        .collect();
    if v.is_empty() { None } else { Some(v) }
}

fn signal_stats(sig: &[f64]) -> (f64, f64, f64, f64) {
    let n = sig.len() as f64;
    let mean = sig.iter().sum::<f64>() / n;
    let rms = (sig.iter().map(|x| x * x).sum::<f64>() / n).sqrt();
    let min = sig.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = sig.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (mean, rms, min, max)
}

fn ascii_waveform(sig: &[f64], width: usize, height: usize) -> String {
    if sig.is_empty() { return String::new(); }
    let mn = sig.iter().cloned().fold(f64::INFINITY, f64::min);
    let mx = sig.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (mx - mn).max(1e-12);
    let step = sig.len().max(1) as f64 / width as f64;
    let samples: Vec<f64> = (0..width).map(|col| {
        let idx = ((col as f64 * step) as usize).min(sig.len() - 1);
        sig[idx]
    }).collect();
    let mut rows = vec![vec![' '; width]; height];
    for (col, &val) in samples.iter().enumerate() {
        let row = height - 1 - ((val - mn) / range * (height - 1) as f64).round() as usize;
        let row = row.min(height - 1);
        rows[row][col] = '█';
    }
    rows.iter().map(|r| r.iter().collect::<String>()).collect::<Vec<_>>().join("\n")
}

pub fn signal_calc(query: &str) -> String {
    let mut out = String::new();
    let w = 60;
    let sep = "═".repeat(w);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  SIGNAL PROCESSING");
    let _ = writeln!(out, "{}", sep);

    let q = query.trim();
    let lower = q.to_lowercase();

    // --- DFT ----------------------------------------------------------------
    if lower.starts_with("dft ") || lower.starts_with("fft ") {
        let rest = q[4..].trim();
        match parse_signal(rest) {
            None => { let _ = writeln!(out, "  ERROR: no numeric values found."); }
            Some(sig) => {
                let n = sig.len();
                let spectrum = dft(&sig);
                let (mean, rms, mn, mx) = signal_stats(&sig);
                let _ = writeln!(out, "  DFT of {}-point signal", n);
                let _ = writeln!(out, "  mean={:.4}  RMS={:.4}  min={:.4}  max={:.4}", mean, rms, mn, mx);
                let _ = writeln!(out, "");
                let _ = writeln!(out, "  {:>5}  {:>10}  {:>10}  {:>10}  {:>10}", "Bin", "Re", "Im", "Magnitude", "Phase°");
                let _ = writeln!(out, "  {}", "-".repeat(52));
                let show = (n / 2 + 1).min(20);
                for k in 0..show {
                    let (re, im) = spectrum[k];
                    let mag = (re * re + im * im).sqrt();
                    let phase = im.atan2(re).to_degrees();
                    let _ = writeln!(out, "  {:>5}  {:>10.4}  {:>10.4}  {:>10.4}  {:>10.2}", k, re, im, mag, phase);
                }
                if show < n / 2 + 1 { let _ = writeln!(out, "  … ({} bins total)", n / 2 + 1); }
                let dc = spectrum[0].0 / n as f64;
                let _ = writeln!(out, "");
                let _ = writeln!(out, "  DC component: {:.6}", dc);
                let dominant = spectrum[1..n/2+1].iter().enumerate()
                    .map(|(i, &(r, im))| (i + 1, (r*r+im*im).sqrt()))
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .map(|(k, mag)| (k, mag));
                if let Some((k, mag)) = dominant {
                    let _ = writeln!(out, "  Dominant frequency bin: {} (mag={:.4})", k, mag);
                }
            }
        }
    }
    // --- IDFT ---------------------------------------------------------------
    else if lower.starts_with("idft ") {
        let rest = q[5..].trim();
        match parse_signal(rest) {
            None => { let _ = writeln!(out, "  ERROR: no numeric values found."); }
            Some(vals) => {
                if vals.len() % 2 != 0 {
                    let _ = writeln!(out, "  ERROR: IDFT needs even number of values (re,im pairs).");
                } else {
                    let spectrum: Vec<(f64, f64)> = vals.chunks(2).map(|c| (c[0], c[1])).collect();
                    let sig = idft(&spectrum);
                    let _ = writeln!(out, "  IDFT result ({} samples):", sig.len());
                    let _ = writeln!(out, "  {:?}", sig.iter().map(|v| format!("{:.6}", v)).collect::<Vec<_>>().join(", "));
                }
            }
        }
    }
    // --- CONVOLVE -----------------------------------------------------------
    else if lower.starts_with("conv ") || lower.starts_with("convolve ") {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        if let Some(mid) = rest.find(" ; ").or_else(|| rest.find(" with ")).or_else(|| rest.find(" | ")) {
            let (a_str, b_str) = rest.split_at(mid);
            let b_str = b_str.trim_start_matches([' ', ';', '|'].as_ref()).trim_start_matches("with").trim();
            match (parse_signal(a_str.trim()), parse_signal(b_str)) {
                (Some(x), Some(h)) => {
                    let y = convolve(&x, &h);
                    let _ = writeln!(out, "  Convolution  x[{}] * h[{}] = y[{}]", x.len(), h.len(), y.len());
                    let _ = writeln!(out, "  x: {}", x.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>().join(", "));
                    let _ = writeln!(out, "  h: {}", h.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>().join(", "));
                    let _ = writeln!(out, "  y: {}", y.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>().join(", "));
                    let (mean, rms, mn, mx) = signal_stats(&y);
                    let _ = writeln!(out, "  mean={:.4}  RMS={:.4}  min={:.4}  max={:.4}", mean, rms, mn, mx);
                }
                _ => { let _ = writeln!(out, "  ERROR: use  conv  A,B,C ; D,E,F  (separate signals with ;)"); }
            }
        } else {
            let _ = writeln!(out, "  ERROR: use  conv  A,B,C ; D,E,F  (separate signals with ;)");
        }
    }
    // --- XCORR --------------------------------------------------------------
    else if lower.starts_with("xcorr ") || lower.starts_with("correlate ") {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        if let Some(mid) = rest.find(" ; ").or_else(|| rest.find(" | ")) {
            let (a_str, b_str) = rest.split_at(mid);
            let b_str = b_str.trim_start_matches([' ', ';', '|'].as_ref()).trim();
            match (parse_signal(a_str.trim()), parse_signal(b_str)) {
                (Some(x), Some(y)) => {
                    let r = xcorr(&x, &y);
                    let peak_lag = r.iter().enumerate()
                        .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
                        .map(|(i, _)| i as isize - (y.len() as isize - 1));
                    let _ = writeln!(out, "  Cross-correlation  x[{}] ⋆ y[{}] = r[{}]", x.len(), y.len(), r.len());
                    let _ = writeln!(out, "  r: {}", r.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>().join(", "));
                    if let Some(lag) = peak_lag {
                        let _ = writeln!(out, "  Peak lag: {} samples", lag);
                    }
                }
                _ => { let _ = writeln!(out, "  ERROR: use  xcorr  A,B,C ; D,E,F"); }
            }
        } else {
            let _ = writeln!(out, "  ERROR: use  xcorr  A,B,C ; D,E,F");
        }
    }
    // --- MOVING AVERAGE -----------------------------------------------------
    else if lower.starts_with("movavg ") || lower.starts_with("moving-avg ") || lower.starts_with("sma ") {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        let window = parts[0].parse::<usize>().unwrap_or(3);
        let data_str = if parts.len() > 1 { parts[1] } else { "" };
        match parse_signal(data_str) {
            None => { let _ = writeln!(out, "  ERROR: use  movavg WINDOW v1,v2,..."); }
            Some(sig) => {
                let smoothed = moving_avg(&sig, window);
                let _ = writeln!(out, "  Simple Moving Average  window={}", window);
                let _ = writeln!(out, "  Input ({} pts): {}", sig.len(), sig.iter().take(8).map(|v| format!("{:.3}", v)).collect::<Vec<_>>().join(", "));
                let _ = writeln!(out, "  Output ({} pts): {}", smoothed.len(), smoothed.iter().take(8).map(|v| format!("{:.4}", v)).collect::<Vec<_>>().join(", "));
                if sig.len() > 8 { let _ = writeln!(out, "  (showing first 8 of {} values)", sig.len()); }
            }
        }
    }
    // --- FIR LOW-PASS FILTER ------------------------------------------------
    else if lower.starts_with("fir-lp ") || lower.starts_with("lowpass ") || lower.starts_with("lp ") {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        let parts: Vec<&str> = rest.splitn(3, ' ').collect();
        let cutoff = parts.get(0).and_then(|s| s.trim_end_matches('%').parse::<f64>().ok()).unwrap_or(0.25) / if rest.contains('%') { 100.0 } else { 1.0 };
        let n_taps = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(21);
        let window = parts.get(2).map(|s| s.trim()).unwrap_or("hamming");
        let h = fir_lowpass(n_taps, cutoff.min(0.5), window);
        let _ = writeln!(out, "  FIR Low-Pass Filter");
        let _ = writeln!(out, "  Cutoff: {:.4} (normalized, 0.5 = Nyquist)  Taps: {}  Window: {}", cutoff, n_taps, window);
        let _ = writeln!(out, "  Coefficients:");
        for (i, c) in h.iter().enumerate() {
            let _ = write!(out, "  h[{:2}]={:>10.6}", i, c);
            if (i + 1) % 4 == 0 { let _ = writeln!(out, ""); }
        }
        let _ = writeln!(out, "");
        let sum: f64 = h.iter().sum();
        let _ = writeln!(out, "  Sum of taps: {:.6}  (DC gain = {:.4} dB)", sum, 20.0 * sum.abs().log10());
    }
    // --- FIR HIGH-PASS FILTER -----------------------------------------------
    else if lower.starts_with("fir-hp ") || lower.starts_with("highpass ") || lower.starts_with("hp ") {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        let parts: Vec<&str> = rest.splitn(3, ' ').collect();
        let cutoff = parts.get(0).and_then(|s| s.trim_end_matches('%').parse::<f64>().ok()).unwrap_or(0.25) / if rest.contains('%') { 100.0 } else { 1.0 };
        let n_taps = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(21);
        let window = parts.get(2).map(|s| s.trim()).unwrap_or("hamming");
        let h = fir_highpass(n_taps, cutoff.min(0.49), window);
        let _ = writeln!(out, "  FIR High-Pass Filter");
        let _ = writeln!(out, "  Cutoff: {:.4}  Taps: {}  Window: {}", cutoff, n_taps, window);
        let _ = writeln!(out, "  Coefficients:");
        for (i, c) in h.iter().enumerate() {
            let _ = write!(out, "  h[{:2}]={:>10.6}", i, c);
            if (i + 1) % 4 == 0 { let _ = writeln!(out, ""); }
        }
        let _ = writeln!(out, "");
    }
    // --- APPLY FILTER -------------------------------------------------------
    else if lower.starts_with("filter ") {
        let rest = q[7..].trim();
        if let Some(mid) = rest.find(" ; ") {
            let (a_str, b_str) = rest.split_at(mid);
            let b_str = b_str[3..].trim();
            match (parse_signal(a_str.trim()), parse_signal(b_str)) {
                (Some(h), Some(x)) => {
                    let y = convolve(&x, &h);
                    let _ = writeln!(out, "  Filter applied  h[{}] * x[{}] = y[{}]", h.len(), x.len(), y.len());
                    let (_, rms_x, _, _) = signal_stats(&x);
                    let (_, rms_y, _, _) = signal_stats(&y);
                    let _ = writeln!(out, "  Input  RMS: {:.4}", rms_x);
                    let _ = writeln!(out, "  Output RMS: {:.4}", rms_y);
                    let _ = writeln!(out, "  y: {}", y.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>().join(", "));
                }
                _ => { let _ = writeln!(out, "  ERROR: use  filter h1,h2,... ; x1,x2,..."); }
            }
        } else {
            let _ = writeln!(out, "  ERROR: use  filter h1,h2,... ; x1,x2,...");
        }
    }
    // --- STATS / INFO -------------------------------------------------------
    else if lower.starts_with("stats ") || lower.starts_with("info ") || lower.starts_with("rms ") {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        match parse_signal(rest) {
            None => { let _ = writeln!(out, "  ERROR: no numeric values."); }
            Some(sig) => {
                let n = sig.len();
                let (mean, rms, mn, mx) = signal_stats(&sig);
                let variance = sig.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / n as f64;
                let std_dev = variance.sqrt();
                let energy: f64 = sig.iter().map(|x| x * x).sum();
                let _ = writeln!(out, "  Signal Statistics  ({} samples)", n);
                let _ = writeln!(out, "  Mean:     {:>12.6}", mean);
                let _ = writeln!(out, "  RMS:      {:>12.6}", rms);
                let _ = writeln!(out, "  Std dev:  {:>12.6}", std_dev);
                let _ = writeln!(out, "  Min:      {:>12.6}", mn);
                let _ = writeln!(out, "  Max:      {:>12.6}", mx);
                let _ = writeln!(out, "  Range:    {:>12.6}", mx - mn);
                let _ = writeln!(out, "  Energy:   {:>12.6}", energy);
                let _ = writeln!(out, "  Power:    {:>12.6}", energy / n as f64);
                if rms > 1e-12 {
                    let crest = mx.abs().max(mn.abs()) / rms;
                    let _ = writeln!(out, "  Crest:    {:>12.6}  ({:.2} dB)", crest, 20.0 * crest.log10());
                }
                let _ = writeln!(out, "");
                let _ = writeln!(out, "  Waveform ({}×8):", sig.len().min(60));
                let wave = ascii_waveform(&sig, sig.len().min(60), 8);
                for line in wave.lines() { let _ = writeln!(out, "  |{}|", line); }
            }
        }
    }
    // --- WAVEFORM GENERATE --------------------------------------------------
    else if lower.starts_with("gen ") || lower.starts_with("wave ") || lower.starts_with("generate ") {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        let parts: Vec<&str> = rest.splitn(4, ' ').collect();
        let shape = parts.get(0).map(|s| s.to_lowercase()).unwrap_or_else(|| "sine".into());
        let freq: f64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let n: usize = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(64);
        let amp: f64 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let sig: Vec<f64> = (0..n).map(|i| {
            let t = i as f64 / n as f64;
            let phase = 2.0 * PI * freq * t;
            match shape.as_str() {
                "cos" | "cosine"   => amp * phase.cos(),
                "square"           => amp * if phase.sin() >= 0.0 { 1.0 } else { -1.0 },
                "sawtooth" | "saw" => amp * (2.0 * (freq * t - (freq * t + 0.5).floor())),
                "triangle" | "tri" => amp * 2.0 * (2.0 * (freq * t - (freq * t + 0.5).floor())).abs() - 1.0,
                "noise" | "rand"   => amp * (((i * 6364136223846793005 + 1442695040888963407) >> 33) as f64 / u32::MAX as f64 * 2.0 - 1.0),
                _                  => amp * phase.sin(),
            }
        }).collect();
        let (mean, rms, mn, mx) = signal_stats(&sig);
        let _ = writeln!(out, "  Waveform: {}  freq={} cycles  n={}  amp={}", shape, freq, n, amp);
        let _ = writeln!(out, "  mean={:.4}  RMS={:.4}  min={:.4}  max={:.4}", mean, rms, mn, mx);
        let _ = writeln!(out, "");
        let wave = ascii_waveform(&sig, sig.len().min(60), 8);
        for line in wave.lines() { let _ = writeln!(out, "  |{}|", line); }
        let _ = writeln!(out, "");
        let first = sig.iter().take(16).map(|v| format!("{:.4}", v)).collect::<Vec<_>>().join(", ");
        let _ = writeln!(out, "  First 16 samples: {}{}", first, if n > 16 { " …" } else { "" });
    }
    // --- WINDOW PREVIEW -----------------------------------------------------
    else if lower.starts_with("window ") {
        let parts: Vec<&str> = q.splitn(3, ' ').collect();
        let win_type = parts.get(1).map(|s| s.to_lowercase()).unwrap_or_else(|| "hann".into());
        let n: usize = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(32);
        let w = match win_type.as_str() {
            "hamming"  => hamming_window(n),
            "blackman" => blackman_window(n),
            _          => hann_window(n),
        };
        let (mean, rms, mn, mx) = signal_stats(&w);
        let _ = writeln!(out, "  {} window  n={}", win_type, n);
        let _ = writeln!(out, "  mean={:.4}  RMS={:.4}  min={:.4}  max={:.4}", mean, rms, mn, mx);
        let _ = writeln!(out, "");
        let wave = ascii_waveform(&w, n.min(60), 6);
        for line in wave.lines() { let _ = writeln!(out, "  |{}|", line); }
        let _ = writeln!(out, "");
        let first8 = w.iter().take(8).map(|v| format!("{:.4}", v)).collect::<Vec<_>>().join(", ");
        let _ = writeln!(out, "  First 8 coeffs: {}", first8);
    }
    // --- HELP ---------------------------------------------------------------
    else {
        let _ = writeln!(out, "{}", signal_usage());
    }

    let _ = writeln!(out, "{}", sep);
    out
}

fn signal_usage() -> String {
    "Signal processing (DSP) — no model, no cloud:\n\
     \n\
     hematite --signal 'dft 1,0,-1,0,1,0,-1,0'         Discrete Fourier Transform\n\
     hematite --signal 'idft 4,0,0,0 ; 0,0,0,0'        Inverse DFT (re,im pairs)\n\
     hematite --signal 'conv 1,2,3 ; 1,-1'              Convolution (separate with ;)\n\
     hematite --signal 'xcorr 1,0,1 ; 0,1,0'            Cross-correlation\n\
     hematite --signal 'movavg 3 1,3,5,7,5,3,1'         3-point moving average\n\
     hematite --signal 'lowpass 0.1 31 hamming'         FIR low-pass (cutoff taps window)\n\
     hematite --signal 'highpass 0.3 21 hann'           FIR high-pass\n\
     hematite --signal 'filter 0.25,0.5,0.25 ; 1,2,3,4' Apply FIR filter to signal\n\
     hematite --signal 'stats 1,2,3,4,5'                RMS, energy, crest, waveform plot\n\
     hematite --signal 'gen sine 2 64'                  Generate 64-pt sine, 2 cycles\n\
     hematite --signal 'gen square 1 32 2.5'            Square wave, amp=2.5\n\
     hematite --signal 'gen sawtooth 3 128'             Sawtooth wave\n\
     hematite --signal 'window hann 64'                 Preview Hann window coefficients\n\
     \n\
     Window types: rectangular hann hamming blackman\n\
     Wave shapes:  sine cosine square sawtooth triangle noise".into()
}

// ── Interpolation & curve fitting ─────────────────────────────────────────────
// Linear, Lagrange polynomial, natural cubic spline, nearest-neighbor,
// and linear extrapolation — all pure-Rust, instant, no subprocess.

fn interp_parse_points(s: &str) -> Option<Vec<(f64, f64)>> {
    // Accept:  "x1,y1 x2,y2 ..."  or  "x1,y1; x2,y2; ..."  or  "(x,y),(x,y)"
    let clean = s.replace('(', "").replace(')', "").replace(';', " ");
    let tokens: Vec<f64> = clean.split([',', ' ', '\t'].as_ref())
        .filter_map(|t| t.trim().parse::<f64>().ok())
        .collect();
    if tokens.len() < 4 || tokens.len() % 2 != 0 { return None; }
    Some(tokens.chunks(2).map(|c| (c[0], c[1])).collect())
}

fn interp_linear(points: &[(f64, f64)], x: f64) -> f64 {
    let n = points.len();
    if n == 0 { return f64::NAN; }
    if n == 1 { return points[0].1; }
    // extrapolate with end segments
    if x <= points[0].0 {
        let (x0, y0) = points[0]; let (x1, y1) = points[1];
        return y0 + (x - x0) * (y1 - y0) / (x1 - x0);
    }
    if x >= points[n - 1].0 {
        let (x0, y0) = points[n - 2]; let (x1, y1) = points[n - 1];
        return y0 + (x - x0) * (y1 - y0) / (x1 - x0);
    }
    for i in 0..n - 1 {
        let (x0, y0) = points[i]; let (x1, y1) = points[i + 1];
        if x >= x0 && x <= x1 { return y0 + (x - x0) * (y1 - y0) / (x1 - x0); }
    }
    f64::NAN
}

fn interp_nearest(points: &[(f64, f64)], x: f64) -> f64 {
    points.iter()
        .min_by(|a, b| (a.0 - x).abs().partial_cmp(&(b.0 - x).abs()).unwrap())
        .map(|p| p.1)
        .unwrap_or(f64::NAN)
}

fn interp_lagrange(points: &[(f64, f64)], x: f64) -> f64 {
    let n = points.len();
    (0..n).map(|i| {
        let (xi, yi) = points[i];
        let li = (0..n).filter(|&j| j != i)
            .fold(1.0_f64, |acc, j| acc * (x - points[j].0) / (xi - points[j].0));
        yi * li
    }).sum()
}

// Natural cubic spline via tridiagonal solve
fn interp_spline_build(points: &[(f64, f64)]) -> Vec<(f64, f64, f64, f64)> {
    let n = points.len();
    if n < 2 { return vec![]; }
    let h: Vec<f64> = (0..n - 1).map(|i| points[i + 1].0 - points[i].0).collect();
    let mut alpha = vec![0.0_f64; n];
    for i in 1..n - 1 {
        alpha[i] = (3.0 / h[i]) * (points[i + 1].1 - points[i].1)
                 - (3.0 / h[i - 1]) * (points[i].1 - points[i - 1].1);
    }
    let mut l = vec![1.0_f64; n];
    let mut mu = vec![0.0_f64; n];
    let mut z = vec![0.0_f64; n];
    for i in 1..n - 1 {
        l[i] = 2.0 * (points[i + 1].0 - points[i - 1].0) - h[i - 1] * mu[i - 1];
        mu[i] = h[i] / l[i];
        z[i] = (alpha[i] - h[i - 1] * z[i - 1]) / l[i];
    }
    let mut c = vec![0.0_f64; n];
    let mut b = vec![0.0_f64; n];
    let mut d = vec![0.0_f64; n];
    for j in (0..n - 1).rev() {
        c[j] = z[j] - mu[j] * c[j + 1];
        b[j] = (points[j + 1].1 - points[j].1) / h[j] - h[j] * (c[j + 1] + 2.0 * c[j]) / 3.0;
        d[j] = (c[j + 1] - c[j]) / (3.0 * h[j]);
    }
    (0..n - 1).map(|i| (b[i], c[i], d[i], points[i].1)).collect()
}

fn interp_spline_eval(points: &[(f64, f64)], coeffs: &[(f64, f64, f64, f64)], x: f64) -> f64 {
    let n = points.len();
    if n == 0 { return f64::NAN; }
    let i = if x <= points[0].0 { 0 }
            else if x >= points[n - 1].0 { n - 2 }
            else {
                points[..n - 1].iter().enumerate()
                    .find(|(i, p)| x >= p.0 && x <= points[i + 1].0)
                    .map(|(i, _)| i)
                    .unwrap_or(n - 2)
            };
    let i = i.min(coeffs.len().saturating_sub(1));
    let dx = x - points[i].0;
    let (b, c, d, a) = coeffs[i];
    a + b * dx + c * dx * dx + d * dx * dx * dx
}

fn interp_ascii_curve(points: &[(f64, f64)], eval_fn: &dyn Fn(f64) -> f64, width: usize, height: usize) -> String {
    if points.is_empty() { return String::new(); }
    let xs: Vec<f64> = points.iter().map(|p| p.0).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.1).collect();
    let xmin = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let xmax = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let step = (xmax - xmin) / (width - 1) as f64;
    let curve_y: Vec<f64> = (0..width).map(|i| eval_fn(xmin + i as f64 * step)).collect();
    let ymin = curve_y.iter().cloned().chain(ys.iter().cloned()).fold(f64::INFINITY, f64::min);
    let ymax = curve_y.iter().cloned().chain(ys.iter().cloned()).fold(f64::NEG_INFINITY, f64::max);
    let yrange = (ymax - ymin).max(1e-12);
    let mut grid = vec![vec![' '; width]; height];
    for (col, &y) in curve_y.iter().enumerate() {
        let row = height - 1 - ((y - ymin) / yrange * (height - 1) as f64).round() as usize;
        let row = row.min(height - 1);
        grid[row][col] = '·';
    }
    for &(xp, yp) in points {
        let col = ((xp - xmin) / (xmax - xmin) * (width - 1) as f64).round() as usize;
        let row = height - 1 - ((yp - ymin) / yrange * (height - 1) as f64).round() as usize;
        let col = col.min(width - 1); let row = row.min(height - 1);
        grid[row][col] = '●';
    }
    let result = grid.iter().map(|r| r.iter().collect::<String>()).collect::<Vec<_>>().join("\n");
    format!("{}\n  y: [{:.4} .. {:.4}]\n  x: [{:.4} .. {:.4}]\n  ● = data point  · = curve", result, ymin, ymax, xmin, xmax)
}

pub fn interpolate_calc(query: &str) -> String {
    let mut out = String::new();
    let sep = "═".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  INTERPOLATION & CURVE FITTING");
    let _ = writeln!(out, "{}", sep);

    let q = query.trim();
    let lower = q.to_lowercase();

    // parse method prefix: "linear POINTS at X", "spline POINTS at X", etc.
    let (method, rest) = if lower.starts_with("linear ") { ("linear", &q[7..]) }
        else if lower.starts_with("spline ") || lower.starts_with("cubic ") { ("spline", &q[7..]) }
        else if lower.starts_with("lagrange ") || lower.starts_with("poly ") { ("lagrange", &q[lower.find(' ').unwrap_or(0)+1..]) }
        else if lower.starts_with("nearest ") { ("nearest", &q[8..]) }
        else { ("linear", q) };

    // split on "at" keyword for evaluation point(s)
    let (pts_str, query_str) = if let Some(pos) = rest.to_lowercase().rfind(" at ") {
        (&rest[..pos], rest[pos + 4..].trim())
    } else {
        (rest, "")
    };

    let mut points = match interp_parse_points(pts_str.trim()) {
        Some(p) => p,
        None => {
            let _ = writeln!(out, "  ERROR: could not parse data points.");
            let _ = writeln!(out, "  Format: x1,y1 x2,y2 x3,y3 ...");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
    };
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let _ = writeln!(out, "  Method: {}  |  {} data points", method, points.len());
    let _ = writeln!(out, "  Points: {}", points.iter().map(|(x, y)| format!("({:.4},{:.4})", x, y)).collect::<Vec<_>>().join("  "));
    let _ = writeln!(out, "");

    // Build spline coefficients once if needed
    let spline_coeffs = if method == "spline" { interp_spline_build(&points) } else { vec![] };

    let eval_fn: Box<dyn Fn(f64) -> f64> = match method {
        "spline"   => { let pts = points.clone(); let sc = spline_coeffs.clone();
                        Box::new(move |x| interp_spline_eval(&pts, &sc, x)) }
        "lagrange" => { let pts = points.clone(); Box::new(move |x| interp_lagrange(&pts, x)) }
        "nearest"  => { let pts = points.clone(); Box::new(move |x| interp_nearest(&pts, x)) }
        _          => { let pts = points.clone(); Box::new(move |x| interp_linear(&pts, x)) }
    };

    // Evaluate at requested x values
    if !query_str.is_empty() {
        let xs: Vec<f64> = query_str.split([',', ' '].as_ref())
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();
        if !xs.is_empty() {
            let _ = writeln!(out, "  {:>12}  {:>14}", "x", "y (interpolated)");
            let _ = writeln!(out, "  {}", "-".repeat(28));
            for &x in &xs {
                let y = eval_fn(x);
                let xmin = points[0].0; let xmax = points[points.len()-1].0;
                let tag = if x < xmin || x > xmax { "  [extrapolated]" } else { "" };
                let _ = writeln!(out, "  {:>12.6}  {:>14.8}{}", x, y, tag);
            }
            let _ = writeln!(out, "");
        }
    }

    // Always show a dense evaluation table and ASCII curve
    let xmin = points[0].0; let xmax = points[points.len()-1].0;
    let steps = 9_usize;
    let _ = writeln!(out, "  Sampled curve ({} pts across range):", steps + 1);
    let _ = writeln!(out, "  {:>10}  {:>14}", "x", "y");
    let _ = writeln!(out, "  {}", "-".repeat(26));
    for i in 0..=steps {
        let x = xmin + i as f64 * (xmax - xmin) / steps as f64;
        let y = eval_fn(x);
        let _ = writeln!(out, "  {:>10.4}  {:>14.8}", x, y);
    }
    let _ = writeln!(out, "");

    // ASCII curve
    let curve_str = interp_ascii_curve(&points, &eval_fn, 56, 10);
    for line in curve_str.lines() { let _ = writeln!(out, "  {}", line); }

    let _ = writeln!(out, "{}", sep);
    out
}
