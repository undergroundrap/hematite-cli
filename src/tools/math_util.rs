// ─── Pure-Rust math utilities ─────────────────────────────────────────────────
// Number theory, sequences, combinatorics — no Python sandbox, instant results.

// Index-based loops are standard notation for DP tables, matrix ops, and
// numeric algorithms. Iterator rewrites hurt readability in math code.
#![allow(clippy::needless_range_loop)]

use md5::Digest as Md5DigestTrait;
use std::fmt::Write;

// ── Primality and factorization ───────────────────────────────────────────────

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n < 4 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }
    let mut i = 5u64;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return false;
        }
        i += 6;
    }
    true
}

fn factorize(mut n: u64) -> Vec<(u64, u32)> {
    let mut factors: Vec<(u64, u32)> = Vec::new();
    if n < 2 {
        return factors;
    }
    for p in [2u64, 3] {
        if n % p == 0 {
            let mut exp = 0u32;
            while n % p == 0 {
                n /= p;
                exp += 1;
            }
            factors.push((p, exp));
        }
    }
    let mut i = 5u64;
    while i * i <= n {
        if n % i == 0 {
            let mut exp = 0u32;
            while n % i == 0 {
                n /= i;
                exp += 1;
            }
            factors.push((i, exp));
        }
        if n % (i + 2) == 0 {
            let mut exp = 0u32;
            while n % (i + 2) == 0 {
                n /= i + 2;
                exp += 1;
            }
            factors.push((i + 2, exp));
        }
        i += 6;
    }
    if n > 1 {
        factors.push((n, 1));
    }
    factors
}

fn next_prime(n: u64) -> u64 {
    let mut c = n + 1;
    while !is_prime(c) {
        c += 1;
    }
    c
}

fn prev_prime(n: u64) -> Option<u64> {
    if n <= 2 {
        return None;
    }
    let mut c = n - 1;
    while c >= 2 {
        if is_prime(c) {
            return Some(c);
        }
        if c == 2 {
            break;
        }
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
        let expr: Vec<String> = factors
            .iter()
            .map(|(p, e)| {
                if *e == 1 {
                    format!("{}", p)
                } else {
                    format!("{}^{}", p, e)
                }
            })
            .collect();
        let _ = writeln!(out, "Factors: {}", expr.join(" × "));

        // divisors from factors
        let mut divisors = vec![1u64];
        for (p, e) in &factors {
            let len = divisors.len();
            let mut pw = 1u64;
            for _ in 0..*e {
                pw *= p;
                for i in 0..len {
                    divisors.push(divisors[i] * pw);
                }
            }
        }
        divisors.sort_unstable();
        let _ = writeln!(
            out,
            "Divisors ({} total): {}",
            divisors.len(),
            if divisors.len() <= 24 {
                divisors
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                format!(
                    "{} ... {} [first 12 + last 12]",
                    divisors[..12]
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    divisors[divisors.len() - 12..]
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        );
        // Euler's totient φ(n)
        let phi = factors.iter().fold(n, |acc, (p, _)| acc / p * (p - 1));
        let _ = writeln!(out, "φ(n):   {}", phi);
        // sum of divisors σ(n)
        let sigma: u64 = factors
            .iter()
            .map(|(p, e)| (p.pow(e + 1) - 1) / (p - 1))
            .product();
        let _ = writeln!(out, "σ(n):   {}", sigma);
        // perfect number check
        if sigma == 2 * n {
            let _ = writeln!(out, "✓ Perfect number");
        }
    }

    if let Some(pp) = prev_prime(n) {
        let _ = writeln!(out, "Prev prime: {}", pp);
    } else {
        let _ = writeln!(out, "Prev prime: (none)");
    }
    let np = next_prime(n);
    let _ = writeln!(out, "Next prime: {}", np);
    out
}

// ── Sequences ─────────────────────────────────────────────────────────────────

pub fn generate_sequence(kind: &str, count: usize, start: f64, step: f64) -> String {
    let count = count.clamp(1, 10_000);
    let kind = kind.trim().to_lowercase();
    let mut out = String::new();

    let nums: Vec<f64> = match kind.as_str() {
        "arithmetic" | "arith" | "linear" => (0..count).map(|i| start + i as f64 * step).collect(),
        "geometric" | "geo" | "geom" => {
            let ratio = if step == 0.0 { 2.0 } else { step };
            let mut v = start;
            (0..count)
                .map(|_| {
                    let x = v;
                    v *= ratio;
                    x
                })
                .collect()
        }
        "fibonacci" | "fib" => {
            let (mut a, mut b) = (start as u64, (start + step) as u64);
            let mut seq = vec![a as f64, b as f64];
            for _ in 2..count {
                let c = a.saturating_add(b);
                seq.push(c as f64);
                a = b;
                b = c;
            }
            seq.truncate(count);
            seq
        }
        "prime" | "primes" => {
            let mut seq = Vec::with_capacity(count);
            let mut n: u64 = start.max(2.0) as u64;
            if !is_prime(n) {
                n = next_prime(n - 1);
            }
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
            (s..s + count as u64)
                .map(|i| (i * (i + 1) / 2) as f64)
                .collect()
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
        "arithmetic" | "arith" | "linear" => format!("Arithmetic (start={}, step={})", start, step),
        "geometric" | "geo" | "geom" => format!(
            "Geometric (start={}, ratio={})",
            start,
            if step == 0.0 { 2.0 } else { step }
        ),
        _ => kind[..1].to_uppercase() + &kind[1..],
    };
    let _ = writeln!(out, "{}: {} terms", label, nums.len());
    let strs: Vec<String> = nums
        .iter()
        .map(|x| {
            if x.fract() == 0.0 && x.abs() < 1e15 {
                format!("{}", *x as i64)
            } else {
                let s = format!("{:.6e}", x);
                // trim trailing zeros in mantissa
                s
            }
        })
        .collect();
    // Wrap at 72 chars
    let mut line = String::new();
    for (i, s) in strs.iter().enumerate() {
        let piece = if i == 0 {
            s.clone()
        } else {
            format!(", {}", s)
        };
        if line.len() + piece.len() > 72 {
            let _ = writeln!(out, "{}", line);
            line = s.clone();
        } else {
            line.push_str(&piece);
        }
    }
    if !line.is_empty() {
        let _ = writeln!(out, "{}", line);
    }
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
    let perm: u128 = if k > n {
        0
    } else {
        (n - k + 1..=n).fold(1u128, |acc, i| acc.saturating_mul(i as u128))
    };

    let _ = writeln!(out, "n = {}  k = {}", n, k);
    let _ = writeln!(
        out,
        "C(n,k) = n! / (k!(n-k)!) = {}  (combinations — order does not matter)",
        binom
    );
    let _ = writeln!(
        out,
        "P(n,k) = n! / (n-k)!     = {}  (permutations — order matters)",
        perm
    );

    // Pascal's triangle row
    if n <= 20 {
        let row: Vec<u128> = (0..=n)
            .map(|j| {
                let j = j.min(n - j);
                (0..j).fold(1u128, |acc, i| acc * (n - i) as u128 / (i + 1) as u128)
            })
            .collect();
        let _ = writeln!(
            out,
            "Pascal row {}: {}",
            n,
            row.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    out
}

// ── Boolean truth table ───────────────────────────────────────────────────────
// Pure-Rust: parses and evaluates a Boolean expression over single-letter variables.

pub fn truth_table(expr: &str) -> String {
    // Collect variables (single uppercase or lowercase letters A-Z)
    let mut vars: Vec<char> = expr
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect::<std::collections::HashSet<char>>()
        .into_iter()
        .collect();
    vars.sort_unstable();

    if vars.is_empty() {
        return format!(
            "No variables found in: {}\nUse single letters (A, B, C, ...) as variables.",
            expr
        );
    }
    if vars.len() > 6 {
        return format!(
            "Too many variables ({}). Limit: 6 (for 2^6 = 64 rows).",
            vars.len()
        );
    }

    let n_vars = vars.len();
    let n_rows = 1usize << n_vars;
    let mut out = String::new();
    let expr_display = expr
        .trim()
        .replace("AND", "∧")
        .replace("OR", "∨")
        .replace("NOT", "¬")
        .replace("XOR", "⊕")
        .replace("NAND", "⊼")
        .replace("NOR", "⊽");

    // Header
    for v in &vars {
        let _ = write!(out, " {}  ", v);
    }
    let _ = writeln!(out, "| {}", expr_display);
    let sep: String =
        vars.iter().map(|_| "----").collect::<String>() + "+--" + &"-".repeat(expr_display.len());
    let _ = writeln!(out, "{}", sep);

    let mut true_rows = 0usize;
    for row in 0..n_rows {
        let vals: Vec<bool> = (0..n_vars)
            .map(|i| (row >> (n_vars - 1 - i)) & 1 == 1)
            .collect();
        for &v in &vals {
            let _ = write!(out, " {}  ", if v { 'T' } else { 'F' });
        }
        let result = eval_bool(expr, &vars, &vals);
        match result {
            Ok(r) => {
                if r {
                    true_rows += 1;
                }
                let _ = writeln!(out, "| {}", if r { 'T' } else { 'F' });
            }
            Err(e) => {
                let _ = writeln!(out, "| Error: {}", e);
            }
        }
    }
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(
        out,
        "True rows: {} / {}  ({}%)",
        true_rows,
        n_rows,
        100 * true_rows / n_rows
    );
    if true_rows == 0 {
        let _ = writeln!(out, "Classification: Contradiction (always false)");
    } else if true_rows == n_rows {
        let _ = writeln!(out, "Classification: Tautology (always true)");
    } else {
        let _ = writeln!(out, "Classification: Contingency");
    }
    out
}

fn eval_bool(expr: &str, vars: &[char], vals: &[bool]) -> Result<bool, String> {
    let tokens = tokenize_bool(expr)?;
    let (result, _) = parse_bool_or(&tokens, 0, vars, vals)?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq)]
enum BoolToken {
    Var(char),
    True,
    False,
    Not,
    And,
    Or,
    Xor,
    Nand,
    Nor,
    LParen,
    RParen,
}

fn tokenize_bool(s: &str) -> Result<Vec<BoolToken>, String> {
    let mut tokens = Vec::new();
    let s = s
        .replace("NAND", "⊼")
        .replace("NOR", "⊽")
        .replace("XOR", "⊕")
        .replace("AND", "∧")
        .replace("OR", "∨")
        .replace("NOT", "¬")
        .replace("&&", "∧")
        .replace("||", "∨")
        .replace("!", "¬")
        .replace('&', "∧")
        .replace('|', "∨")
        .replace('^', "⊕");
    let chars = s.chars().peekable();
    for c in chars {
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

fn parse_bool_or(
    tokens: &[BoolToken],
    pos: usize,
    vars: &[char],
    vals: &[bool],
) -> Result<(bool, usize), String> {
    let (mut lhs, mut pos) = parse_bool_and(tokens, pos, vars, vals)?;
    while pos < tokens.len() {
        match &tokens[pos] {
            BoolToken::Or => {
                let (rhs, np) = parse_bool_and(tokens, pos + 1, vars, vals)?;
                lhs = lhs || rhs;
                pos = np;
            }
            BoolToken::Nor => {
                let (rhs, np) = parse_bool_and(tokens, pos + 1, vars, vals)?;
                lhs = !(lhs || rhs);
                pos = np;
            }
            _ => break,
        }
    }
    Ok((lhs, pos))
}

fn parse_bool_and(
    tokens: &[BoolToken],
    pos: usize,
    vars: &[char],
    vals: &[bool],
) -> Result<(bool, usize), String> {
    let (mut lhs, mut pos) = parse_bool_xor(tokens, pos, vars, vals)?;
    while pos < tokens.len() {
        match &tokens[pos] {
            BoolToken::And => {
                let (rhs, np) = parse_bool_xor(tokens, pos + 1, vars, vals)?;
                lhs = lhs && rhs;
                pos = np;
            }
            BoolToken::Nand => {
                let (rhs, np) = parse_bool_xor(tokens, pos + 1, vars, vals)?;
                lhs = !(lhs && rhs);
                pos = np;
            }
            _ => break,
        }
    }
    Ok((lhs, pos))
}

fn parse_bool_xor(
    tokens: &[BoolToken],
    pos: usize,
    vars: &[char],
    vals: &[bool],
) -> Result<(bool, usize), String> {
    let (mut lhs, mut pos) = parse_bool_not(tokens, pos, vars, vals)?;
    while pos < tokens.len() && tokens[pos] == BoolToken::Xor {
        let (rhs, np) = parse_bool_not(tokens, pos + 1, vars, vals)?;
        lhs ^= rhs;
        pos = np;
    }
    Ok((lhs, pos))
}

fn parse_bool_not(
    tokens: &[BoolToken],
    pos: usize,
    vars: &[char],
    vals: &[bool],
) -> Result<(bool, usize), String> {
    if pos < tokens.len() && tokens[pos] == BoolToken::Not {
        let (inner, np) = parse_bool_not(tokens, pos + 1, vars, vals)?;
        return Ok((!inner, np));
    }
    parse_bool_atom(tokens, pos, vars, vals)
}

fn parse_bool_atom(
    tokens: &[BoolToken],
    pos: usize,
    vars: &[char],
    vals: &[bool],
) -> Result<(bool, usize), String> {
    if pos >= tokens.len() {
        return Err("Unexpected end of expression".into());
    }
    match &tokens[pos] {
        BoolToken::LParen => {
            let (inner, np) = parse_bool_or(tokens, pos + 1, vars, vals)?;
            if np >= tokens.len() || tokens[np] != BoolToken::RParen {
                return Err("Missing closing ')'".into());
            }
            Ok((inner, np + 1))
        }
        BoolToken::Var(c) => {
            let idx = vars
                .iter()
                .position(|v| v == c)
                .ok_or_else(|| format!("Unknown variable '{}'", c))?;
            Ok((vals[idx], pos + 1))
        }
        BoolToken::True => Ok((true, pos + 1)),
        BoolToken::False => Ok((false, pos + 1)),
        other => Err(format!("Unexpected token: {:?}", other)),
    }
}

// ── GCD / LCM ─────────────────────────────────────────────────────────────────

fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
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
            if tokens.len() < 3 {
                return "Usage: extgcd A B".into();
            }
            let a: i128 = match tokens[1].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[1]),
            };
            let b: i128 = match tokens[2].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[2]),
            };
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
            let pairs: Vec<(i128, i128)> = tokens[1..]
                .chunks(2)
                .filter_map(|c| {
                    let r = c[0].parse::<i128>().ok()?;
                    let m = c[1].parse::<i128>().ok()?;
                    Some((r, m))
                })
                .collect();
            if pairs.len() < 2 {
                return "Need at least 2 remainder-modulus pairs.".into();
            }
            match crt(&pairs) {
                Some((x, m)) => {
                    let mut out = "Chinese Remainder Theorem:\n".to_string();
                    for (r, mo) in &pairs {
                        out.push_str(&format!("  x ≡ {} (mod {})\n", r, mo));
                    }
                    out.push_str(&format!(
                        "Solution: x ≡ {} (mod {})  [smallest positive: {}]",
                        x,
                        m,
                        ((x % m) + m) % m
                    ));
                    out
                }
                None => "No solution — moduli are not pairwise coprime.".into(),
            }
        }
        "mobius" | "möbius" | "mu" => {
            if tokens.len() < 2 {
                return "Usage: mobius N".into();
            }
            let n: u64 = match tokens[1].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[1]),
            };
            let mu = mobius(n);
            let explanation = match mu {
                0 => "n has a squared prime factor → μ(n) = 0",
                1 => "n is squarefree with even number of prime factors → μ(n) = 1",
                -1 => "n is squarefree with odd number of prime factors → μ(n) = -1",
                _ => "",
            };
            format!("Möbius function μ({}) = {}\n  {}", n, mu, explanation)
        }
        "modinv" => {
            if tokens.len() < 3 {
                return "Usage: modinv A MOD".into();
            }
            let a: i128 = match tokens[1].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[1]),
            };
            let m: i128 = match tokens[2].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[2]),
            };
            match mod_inv(a, m) {
                Some(inv) => format!(
                    "Modular inverse: {}⁻¹ ≡ {} (mod {})\nVerify: {} × {} = {} ≡ 1 (mod {})",
                    a,
                    inv,
                    m,
                    a,
                    inv,
                    a * inv,
                    m
                ),
                None => format!("No modular inverse: gcd({}, {}) ≠ 1 (not coprime)", a, m),
            }
        }
        "modpow" | "powmod" => {
            if tokens.len() < 4 {
                return "Usage: modpow BASE EXP MOD".into();
            }
            let base: u128 = match tokens[1].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[1]),
            };
            let exp: u128 = match tokens[2].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[2]),
            };
            let modu: u128 = match tokens[3].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[3]),
            };
            if modu == 0 {
                return "Modulus cannot be zero.".into();
            }
            let result = mod_pow(base, exp, modu);
            format!("{}^{} mod {} = {}", base, exp, modu, result)
        }
        "cf" | "cfrac" | "continued_fraction" => {
            if tokens.len() < 2 {
                return "Usage: cf N/D  or  cf DECIMAL".into();
            }
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
            if den == 0 {
                return "Denominator cannot be zero.".into();
            }
            let coeffs = cf_expansion(num, den, 20);
            let convergents = cf_convergents(&coeffs);
            let mut out = format!(
                "Continued fraction of {}/{} = {}:\n",
                num,
                den,
                num as f64 / den as f64
            );
            out.push_str(&format!(
                "  CF = [{}]\n",
                coeffs
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
            out.push_str("  Convergents:\n");
            for (p, q) in &convergents {
                out.push_str(&format!("    {}/{} = {:.8}\n", p, q, *p as f64 / *q as f64));
            }
            out
        }
        "goldbach" => {
            if tokens.len() < 2 {
                return "Usage: goldbach N (must be even, > 2)".into();
            }
            let n: u64 = match tokens[1].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[1]),
            };
            if n <= 2 || n % 2 != 0 {
                return format!("{} must be even and > 2 for Goldbach's conjecture.", n);
            }
            let pairs: Vec<(u64, u64)> = (2..=n / 2)
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
                if pairs.len() > 10 {
                    out.push_str(&format!("  ... ({} total decompositions)\n", pairs.len()));
                }
            }
            out
        }
        "totient" | "phi" | "euler" => {
            if tokens.len() < 2 {
                return "Usage: totient N".into();
            }
            let n: u64 = match tokens[1].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[1]),
            };
            let phi = euler_totient(n);
            format!(
                "Euler's totient φ({}) = {}\n  (count of integers 1..{} coprime to {})",
                n, phi, n, n
            )
        }
        "jacobi" => {
            if tokens.len() < 3 {
                return "Usage: jacobi A N (N must be odd)".into();
            }
            let a: i64 = match tokens[1].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[1]),
            };
            let n: i64 = match tokens[2].parse() {
                Ok(v) => v,
                Err(_) => return format!("Not a number: {}", tokens[2]),
            };
            if n <= 0 || n % 2 == 0 {
                return "N must be a positive odd integer.".into();
            }
            let j = jacobi_symbol(a, n);
            let meaning = match j {
                0 => "a is not coprime to n",
                1 => "a is a quadratic residue mod n (or n is composite)",
                -1 => "a is a quadratic non-residue mod n",
                _ => "",
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
     hematite --number-theory '42'    (full report for a number)"
        .into()
}

fn nt_report(n: u64) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Number theory report for {}", n);
    let _ = writeln!(out, "  Euler's totient φ(n) = {}", euler_totient(n));
    let _ = writeln!(out, "  Möbius μ(n) = {}", mobius(n));
    if n < 1_000_000 {
        let sigma: u64 = (1..=n).filter(|d| n % d == 0).sum();
        let _ = writeln!(out, "  Sum of divisors σ(n) = {}", sigma);
        if sigma == 2 * n {
            let _ = writeln!(out, "  → Perfect number!");
        } else if sigma > 2 * n {
            let _ = writeln!(out, "  → Abundant number");
        } else {
            let _ = writeln!(out, "  → Deficient number");
        }
    }
    if n >= 4 && n % 2 == 0 {
        if let Some((p, q)) = (2..=n / 2)
            .filter(|&p| is_prime(p) && is_prime(n - p))
            .map(|p| (p, n - p))
            .next()
        {
            let _ = writeln!(out, "  Goldbach: {} = {} + {}", n, p, q);
        }
    }
    out
}

fn ext_gcd(a: i128, b: i128) -> (i128, i128, i128) {
    if b == 0 {
        return (a, 1, 0);
    }
    let (g, x1, y1) = ext_gcd(b, a % b);
    (g, y1, x1 - (a / b) * y1)
}

fn mod_inv(a: i128, m: i128) -> Option<i128> {
    let (g, x, _) = ext_gcd(a.rem_euclid(m), m);
    if g != 1 {
        return None;
    }
    Some(x.rem_euclid(m))
}

fn mod_pow(mut base: u128, mut exp: u128, modu: u128) -> u128 {
    if modu == 1 {
        return 0;
    }
    let mut result = 1u128;
    base %= modu;
    while exp > 0 {
        if exp % 2 == 1 {
            result = result.wrapping_mul(base) % modu;
        }
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
        if (r - x) % g != 0 {
            return None;
        }
        let lcm = m / g * mi;
        let inv = mod_inv(m / g, mi / g)?;
        x = x + m * ((r - x) / g % (mi / g) * inv % (mi / g));
        m = lcm;
        x = x.rem_euclid(m);
    }
    Some((x, m))
}

fn mobius(n: u64) -> i32 {
    if n == 1 {
        return 1;
    }
    let factors = factorize(n);
    for (_, exp) in &factors {
        if *exp > 1 {
            return 0;
        }
    }
    if factors.len() % 2 == 0 {
        1
    } else {
        -1
    }
}

fn euler_totient(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let factors = factorize(n);
    let mut phi = n;
    for (p, _) in factors {
        phi = phi / p * (p - 1);
    }
    phi
}

fn cf_expansion(mut num: i64, mut den: i64, max_terms: usize) -> Vec<i64> {
    let mut coeffs = Vec::new();
    for _ in 0..max_terms {
        coeffs.push(num / den);
        let rem = num % den;
        if rem == 0 {
            break;
        }
        num = den;
        den = rem;
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
        p_prev = p_curr;
        q_prev = q_curr;
        p_curr = p_next;
        q_curr = q_next;
    }
    result
}

fn jacobi_symbol(mut a: i64, mut n: i64) -> i32 {
    if n <= 0 || n % 2 == 0 {
        return 0;
    }
    let mut result = 1i32;
    a = a.rem_euclid(n);
    while a != 0 {
        while a % 2 == 0 {
            a /= 2;
            if n % 8 == 3 || n % 8 == 5 {
                result = -result;
            }
        }
        std::mem::swap(&mut a, &mut n);
        if a % 4 == 3 && n % 4 == 3 {
            result = -result;
        }
        a %= n;
    }
    if n == 1 {
        result
    } else {
        0
    }
}

// ── Roman numerals ────────────────────────────────────────────────────────────

pub fn to_roman(mut n: u64) -> String {
    if n == 0 {
        return "Roman numerals start at 1.".into();
    }
    if n > 3_999_999 {
        return format!("{n} is too large for standard Roman numerals (max 3,999,999).");
    }
    const TABLE: &[(u64, &str)] = &[
        (1_000_000, "M̄"),
        (900_000, "C̄M̄"),
        (500_000, "D̄"),
        (400_000, "C̄D̄"),
        (100_000, "C̄"),
        (90_000, "X̄C̄"),
        (50_000, "L̄"),
        (40_000, "X̄L̄"),
        (10_000, "X̄"),
        (9_000, "MX̄"),
        (5_000, "V̄"),
        (4_000, "MV̄"),
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for &(val, sym) in TABLE {
        while n >= val {
            out.push_str(sym);
            n -= val;
        }
    }
    out
}

pub fn from_roman(s: &str) -> String {
    let s = s.trim().to_uppercase();
    let map = [
        ("M̄", 1_000_000u64),
        ("C̄M̄", 900_000),
        ("D̄", 500_000),
        ("C̄D̄", 400_000),
        ("C̄", 100_000),
        ("X̄C̄", 90_000),
        ("L̄", 50_000),
        ("X̄L̄", 40_000),
        ("X̄", 10_000),
        ("MX̄", 9_000),
        ("V̄", 5_000),
        ("MV̄", 4_000),
        ("M", 1000),
        ("CM", 900),
        ("D", 500),
        ("CD", 400),
        ("C", 100),
        ("XC", 90),
        ("L", 50),
        ("XL", 40),
        ("X", 10),
        ("IX", 9),
        ("V", 5),
        ("IV", 4),
        ("I", 1),
    ];
    let mut pos = 0usize;
    let chars: Vec<char> = s.chars().collect();
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
        return format!(
            "Unrecognized Roman numeral character at position {}: '{}'",
            pos, chars[pos]
        );
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
    if !(2..=36).contains(&from_base) || !(2..=36).contains(&to_base) {
        return "Base must be between 2 and 36.".into();
    }
    let s = input.trim().to_ascii_uppercase();
    let value = u128::from_str_radix(&s, from_base).unwrap_or(0);
    // check parse success separately
    if u128::from_str_radix(&s, from_base).is_err() {
        return format!("'{}' is not a valid base-{} number.", s, from_base);
    }
    let to_str = |mut v: u128, base: u32| -> String {
        if v == 0 {
            return "0".into();
        }
        let digits: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut result = Vec::new();
        while v > 0 {
            result.push(digits[(v % base as u128) as usize] as char);
            v /= base as u128;
        }
        result.into_iter().rev().collect()
    };
    let _ = value; // used via closure
    let value2 = u128::from_str_radix(&s, from_base).unwrap();
    let mut out = String::new();
    let _ = writeln!(out, "Input (base {}): {}", from_base, s);
    let _ = writeln!(out, "Decimal: {}", value2);
    let _ = writeln!(
        out,
        "Output (base {}): {}",
        to_base,
        to_str(value2, to_base)
    );
    if to_base != 2 && from_base != 2 {
        let _ = writeln!(out, "Binary:  {}", to_str(value2, 2));
    }
    if to_base != 16 && from_base != 16 {
        let _ = writeln!(out, "Hex:     {}", to_str(value2, 16));
    }
    if to_base != 8 && from_base != 8 {
        let _ = writeln!(out, "Octal:   {}", to_str(value2, 8));
    }
    out
}

// ── Date arithmetic ───────────────────────────────────────────────────────────

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn to_jdn(y: i32, m: u32, d: u32) -> i64 {
    let a = (14 - m as i32) / 12;
    let yr = y + 4800 - a;
    let mo = m as i32 + 12 * a - 3;
    d as i64 + (153 * mo + 2) as i64 / 5 + 365 * yr as i64 + yr as i64 / 4 - yr as i64 / 100
        + yr as i64 / 400
        - 32045
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
    let parts: Vec<&str> = s
        .splitn(3, |c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() == 3 {
        let y = parts[0].parse::<i32>().ok()?;
        let m = parts[1].parse::<u32>().ok()?;
        let d = parts[2].parse::<u32>().ok()?;
        if (1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m) {
            return Some((y, m, d));
        }
    }
    None
}

const WEEKDAYS: &[&str] = &[
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

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
        (s[..pos].trim(), Some(s[pos + 4..].trim()))
    } else if s.contains(',') {
        let mut it = s.splitn(2, ',');
        (
            it.next().unwrap_or("").trim(),
            Some(it.next().unwrap_or("").trim()),
        )
    } else {
        (s, None)
    };

    // Check for "+N" or "-N" at the end (date arithmetic)
    let plus_re = {
        let a = a_str.trim_end();
        if let Some(idx) = a.rfind(['+', '-']) {
            let (date_part, offset_part) = a.split_at(idx);
            if let Ok(n) = offset_part.trim().parse::<i64>() {
                Some((date_part.trim(), n))
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some((date_part, offset)) = plus_re {
        if let Some((y, m, d)) = parse_date(date_part) {
            let jdn = to_jdn(y, m, d) + offset;
            let (y2, m2, d2) = from_jdn(jdn);
            let dow = ((jdn + 1) % 7) as usize;
            let _ = writeln!(
                out,
                "{}-{:02}-{:02}  {} {} days  →  {}-{:02}-{:02} ({})",
                y,
                m,
                d,
                if offset >= 0 { "+" } else { "" },
                offset,
                y2,
                m2,
                d2,
                WEEKDAYS[dow]
            );
        } else {
            out.push_str(
                "Could not parse date. Use: --date '2024-03-15 +90' or '2024-01-01 to 2024-12-31'",
            );
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
            let _ = writeln!(
                out,
                "Difference: {} days  ({} weeks {} days)",
                diff.abs(),
                diff.abs() / 7,
                diff.abs() % 7
            );
            let _ = writeln!(
                out,
                "           ≈ {:.2} months  ≈ {:.3} years",
                diff.abs() as f64 / 30.4375,
                diff.abs() as f64 / 365.25
            );
            if diff < 0 {
                let _ = writeln!(out, "(B is before A — {} days ago)", diff.abs());
            }
        } else {
            out.push_str("Could not parse second date.");
        }
        return out;
    }

    // Single date info
    if let Some((y, m, d)) = parse_date(a_str) {
        let jdn = to_jdn(y, m, d);
        let dow = ((jdn + 1) % 7) as usize;
        let ts = (jdn - 2440588) * 86400;
        let day_of_year: u32 = (1..m).map(|mo| days_in_month(y, mo)).sum::<u32>() + d;
        let is_leap = y % 400 == 0 || (y % 4 == 0 && y % 100 != 0);
        let days_left = (if is_leap { 366 } else { 365 }) - day_of_year;
        let _ = writeln!(out, "Date:       {}-{:02}-{:02}", y, m, d);
        let _ = writeln!(
            out,
            "Day:        {} (day {} of {}, {} remaining)",
            WEEKDAYS[dow],
            day_of_year,
            if is_leap { 366 } else { 365 },
            days_left
        );
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
        let prefix: u8 = match cidr[idx + 1..].trim().parse() {
            Ok(v) => v,
            Err(_) => return "Invalid prefix length. Use CIDR format: 192.168.1.0/24".into(),
        };
        (&cidr[..idx], prefix)
    } else {
        return "Use CIDR notation: 192.168.1.0/24".into();
    };

    let parse_ip = |s: &str| -> Option<u32> {
        let parts: Vec<u8> = s.trim().split('.').filter_map(|x| x.parse().ok()).collect();
        if parts.len() == 4 {
            Some(
                ((parts[0] as u32) << 24)
                    | ((parts[1] as u32) << 16)
                    | ((parts[2] as u32) << 8)
                    | parts[3] as u32,
            )
        } else {
            None
        }
    };

    let ip = match parse_ip(ip_str) {
        Some(v) => v,
        None => return format!("Invalid IP address: '{}'", ip_str),
    };

    if prefix > 32 {
        return "Prefix must be 0–32.".into();
    }

    let mask: u32 = if prefix == 0 {
        0
    } else {
        !0u32 << (32 - prefix)
    };
    let network = ip & mask;
    let broadcast = network | !mask;
    let first_host = if prefix >= 31 { network } else { network + 1 };
    let last_host = if prefix >= 31 {
        broadcast
    } else {
        broadcast - 1
    };
    let host_count: u64 = if prefix >= 32 {
        1
    } else if prefix == 31 {
        2
    } else {
        (1u64 << (32 - prefix)) - 2
    };

    let fmt_ip = |v: u32| {
        format!(
            "{}.{}.{}.{}",
            v >> 24,
            (v >> 16) & 0xff,
            (v >> 8) & 0xff,
            v & 0xff
        )
    };

    let class = match ip >> 24 {
        0..=127 => "A",
        128..=191 => "B",
        192..=223 => "C",
        224..=239 => "D (Multicast)",
        _ => "E (Reserved)",
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
    let _ = writeln!(
        out,
        "Class:      {}  |  Private: {}",
        class,
        if private { "Yes" } else { "No" }
    );
    out
}

// ── Color space conversion ────────────────────────────────────────────────────

pub fn color_convert(input: &str) -> String {
    let s = input.trim().to_ascii_lowercase();
    let mut out = String::new();

    // Parse hex: #RRGGBB or RRGGBB or #RGB
    let hex_input = s.trim_start_matches('#');
    let (r8, g8, b8) = if hex_input.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex_input[0..2], 16),
            u8::from_str_radix(&hex_input[2..4], 16),
            u8::from_str_radix(&hex_input[4..6], 16),
        ) {
            (r, g, b)
        } else {
            return format!("Invalid hex: '{}'", input);
        }
    } else if hex_input.len() == 3 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex_input[0..1].repeat(2), 16),
            u8::from_str_radix(&hex_input[1..2].repeat(2), 16),
            u8::from_str_radix(&hex_input[2..3].repeat(2), 16),
        ) {
            (r, g, b)
        } else {
            return format!("Invalid hex: '{}'", input);
        }
    } else if s.starts_with("rgb(") || s.starts_with("rgb ") {
        let nums: Vec<u8> = s
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == ' ' || *c == ',')
            .collect::<String>()
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|x| x.parse().ok())
            .collect();
        if nums.len() >= 3 {
            (nums[0], nums[1], nums[2])
        } else {
            return "Usage: --color '#ff8800' or --color 'rgb(255,136,0)'".into();
        }
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
    let s_hsl = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };
    let h_hsl = if delta == 0.0 {
        0.0
    } else if cmax == rf {
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
    let (c_cmyk, m_cmyk, y_cmyk) = if k_cmyk == 1.0 {
        (0.0, 0.0, 0.0)
    } else {
        (
            (1.0 - rf - k_cmyk) / (1.0 - k_cmyk),
            (1.0 - gf - k_cmyk) / (1.0 - k_cmyk),
            (1.0 - bf - k_cmyk) / (1.0 - k_cmyk),
        )
    };

    let _ = writeln!(out, "Hex:   #{:02X}{:02X}{:02X}", r8, g8, b8);
    let _ = writeln!(out, "RGB:   rgb({}, {}, {})", r8, g8, b8);
    let _ = writeln!(
        out,
        "HSL:   hsl({:.1}°, {:.1}%, {:.1}%)",
        h_hsl,
        s_hsl * 100.0,
        l * 100.0
    );
    let _ = writeln!(
        out,
        "HSV:   hsv({:.1}°, {:.1}%, {:.1}%)",
        h_hsl,
        s_hsv * 100.0,
        v_hsv * 100.0
    );
    let _ = writeln!(
        out,
        "CMYK:  cmyk({:.0}%, {:.0}%, {:.0}%, {:.0}%)",
        c_cmyk * 100.0,
        m_cmyk * 100.0,
        y_cmyk * 100.0,
        k_cmyk * 100.0
    );
    // Luminance (WCAG)
    let lum = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
    let _ = writeln!(
        out,
        "Luminance: {:.4}  (WCAG relative, 0=black 1=white)",
        lum
    );
    let contrast_white = (1.0 + 0.05) / (lum + 0.05);
    let _ = writeln!(
        out,
        "Contrast vs white: {:.2}:1  (WCAG AA needs 4.5:1)",
        contrast_white
    );
    out
}

// ── Molecular weight calculator ───────────────────────────────────────────────
// Parses chemical formulas like H2O, C6H12O6, Ca(NO3)2, (NH4)2SO4

// Symbol → atomic mass (standard atomic weights, IUPAC 2021)
fn atomic_masses() -> &'static [(&'static str, f64)] {
    &[
        ("H", 1.008),
        ("He", 4.0026),
        ("Li", 6.94),
        ("Be", 9.0122),
        ("B", 10.81),
        ("C", 12.011),
        ("N", 14.007),
        ("O", 15.999),
        ("F", 18.998),
        ("Ne", 20.180),
        ("Na", 22.990),
        ("Mg", 24.305),
        ("Al", 26.982),
        ("Si", 28.085),
        ("P", 30.974),
        ("S", 32.06),
        ("Cl", 35.45),
        ("Ar", 39.948),
        ("K", 39.098),
        ("Ca", 40.078),
        ("Sc", 44.956),
        ("Ti", 47.867),
        ("V", 50.942),
        ("Cr", 51.996),
        ("Mn", 54.938),
        ("Fe", 55.845),
        ("Co", 58.933),
        ("Ni", 58.693),
        ("Cu", 63.546),
        ("Zn", 65.38),
        ("Ga", 69.723),
        ("Ge", 72.630),
        ("As", 74.922),
        ("Se", 78.971),
        ("Br", 79.904),
        ("Kr", 83.798),
        ("Rb", 85.468),
        ("Sr", 87.62),
        ("Y", 88.906),
        ("Zr", 91.224),
        ("Nb", 92.906),
        ("Mo", 95.95),
        ("Tc", 98.0),
        ("Ru", 101.07),
        ("Rh", 102.906),
        ("Pd", 106.42),
        ("Ag", 107.868),
        ("Cd", 112.414),
        ("In", 114.818),
        ("Sn", 118.710),
        ("Sb", 121.760),
        ("Te", 127.60),
        ("I", 126.904),
        ("Xe", 131.293),
        ("Cs", 132.905),
        ("Ba", 137.327),
        ("La", 138.905),
        ("Ce", 140.116),
        ("Pr", 140.908),
        ("Nd", 144.242),
        ("Pm", 145.0),
        ("Sm", 150.36),
        ("Eu", 151.964),
        ("Gd", 157.25),
        ("Tb", 158.925),
        ("Dy", 162.500),
        ("Ho", 164.930),
        ("Er", 167.259),
        ("Tm", 168.934),
        ("Yb", 173.045),
        ("Lu", 174.967),
        ("Hf", 178.49),
        ("Ta", 180.948),
        ("W", 183.84),
        ("Re", 186.207),
        ("Os", 190.23),
        ("Ir", 192.217),
        ("Pt", 195.084),
        ("Au", 196.967),
        ("Hg", 200.592),
        ("Tl", 204.38),
        ("Pb", 207.2),
        ("Bi", 208.980),
        ("Po", 209.0),
        ("At", 210.0),
        ("Rn", 222.0),
        ("Fr", 223.0),
        ("Ra", 226.0),
        ("Ac", 227.0),
        ("Th", 232.038),
        ("Pa", 231.036),
        ("U", 238.029),
        ("Np", 237.0),
        ("Pu", 244.0),
        ("Am", 243.0),
        ("Cm", 247.0),
        ("Bk", 247.0),
        ("Cf", 251.0),
        ("Es", 252.0),
        ("Fm", 257.0),
        ("Md", 258.0),
        ("No", 259.0),
        ("Lr", 266.0),
        ("Rf", 267.0),
        ("Db", 268.0),
        ("Sg", 271.0),
        ("Bh", 270.0),
        ("Hs", 277.0),
        ("Mt", 278.0),
        ("Ds", 281.0),
        ("Rg", 282.0),
        ("Cn", 285.0),
        ("Nh", 286.0),
        ("Fl", 289.0),
        ("Mc", 290.0),
        ("Lv", 293.0),
        ("Ts", 294.0),
        ("Og", 294.0),
    ]
}

fn lookup_mass(sym: &str) -> Option<f64> {
    atomic_masses()
        .iter()
        .find(|(s, _)| *s == sym)
        .map(|(_, m)| *m)
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
                for (sym, n) in inner {
                    items.push((sym, n * count));
                }
            }
            '[' => {
                *pos += 1;
                let inner = parse_formula(chars, pos)?;
                if *pos >= chars.len() || chars[*pos] != ']' {
                    return Err("Missing closing ']'".into());
                }
                *pos += 1;
                let count = read_number(chars, pos).unwrap_or(1);
                for (sym, n) in inner {
                    items.push((sym, n * count));
                }
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
            '·' | '•' | '.' => {
                *pos += 1;
            } // hydrate dot
            ' ' | '\t' => {
                *pos += 1;
            }
            other => return Err(format!("Unexpected character: '{}'", other)),
        }
    }
    Ok(items)
}

fn read_number(chars: &[char], pos: &mut usize) -> Option<u32> {
    let start = *pos;
    while *pos < chars.len() && chars[*pos].is_ascii_digit() {
        *pos += 1;
    }
    if *pos == start {
        None
    } else {
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
        return format!(
            "Unknown element(s): {}. Check your formula.",
            unknown.join(", ")
        );
    }

    let mut out = String::new();
    let _ = writeln!(out, "Formula: {}", formula.trim());
    let _ = writeln!(out, "Molecular weight: {:.4} g/mol", mw);
    let _ = writeln!(out);
    let _ = writeln!(out, "Composition:");
    for (sym, n, contrib) in &breakdown {
        let pct = 100.0 * contrib / mw;
        let _ = writeln!(
            out,
            "  {:2} × {:2}  {:8.4} g/mol  ({:.2}%)",
            n, sym, contrib, pct
        );
    }
    // Common derived values
    let _ = writeln!(out);
    let _ = writeln!(out, "1 mole = {:.4} g", mw);
    let _ = writeln!(out, "1 g    = {:.6} mol", 1.0 / mw);
    out
}

// ── Physical constants ────────────────────────────────────────────────────────

const CONSTANTS: &[(&str, &str, &str, &str)] = &[
    // (name, value, unit, aliases)
    (
        "Speed of light",
        "299792458",
        "m/s",
        "c,light,speed of light,c0",
    ),
    (
        "Planck constant",
        "6.62607015e-34",
        "J·s",
        "h,planck,planck constant",
    ),
    (
        "Reduced Planck (ℏ)",
        "1.054571817e-34",
        "J·s",
        "hbar,h-bar,reduced planck,hbar",
    ),
    (
        "Gravitational constant",
        "6.67430e-11",
        "N·m²/kg²",
        "G,gravity,gravitational,newton gravity",
    ),
    (
        "Elementary charge",
        "1.602176634e-19",
        "C",
        "e,electron charge,elementary charge,charge",
    ),
    (
        "Electron mass",
        "9.1093837015e-31",
        "kg",
        "me,electron mass,m_e",
    ),
    (
        "Proton mass",
        "1.67262192369e-27",
        "kg",
        "mp,proton mass,m_p",
    ),
    (
        "Neutron mass",
        "1.67492749804e-27",
        "kg",
        "mn,neutron mass,m_n",
    ),
    (
        "Avogadro constant",
        "6.02214076e23",
        "mol⁻¹",
        "NA,avogadro,avogadro constant,N_A",
    ),
    (
        "Boltzmann constant",
        "1.380649e-23",
        "J/K",
        "k,kb,boltzmann,boltzmann constant,k_B",
    ),
    (
        "Gas constant",
        "8.314462618",
        "J/(mol·K)",
        "R,gas constant,universal gas constant,molar gas",
    ),
    (
        "Stefan-Boltzmann",
        "5.670374419e-8",
        "W/(m²·K⁴)",
        "sigma,stefan,stefan-boltzmann,σ",
    ),
    (
        "Vacuum permittivity",
        "8.8541878128e-12",
        "F/m",
        "eps0,epsilon0,vacuum permittivity,ε₀",
    ),
    (
        "Vacuum permeability",
        "1.25663706212e-6",
        "N/A²",
        "mu0,mu_0,vacuum permeability,μ₀",
    ),
    (
        "Bohr radius",
        "5.29177210903e-11",
        "m",
        "a0,bohr,bohr radius,a_0",
    ),
    (
        "Fine structure constant",
        "7.2973525693e-3",
        "dimensionless",
        "alpha,fine structure,α",
    ),
    (
        "Rydberg constant",
        "10973731.568160",
        "m⁻¹",
        "Ry,rydberg,rydberg constant",
    ),
    (
        "Faraday constant",
        "96485.33212",
        "C/mol",
        "F,faraday,faraday constant",
    ),
    (
        "Standard gravity",
        "9.80665",
        "m/s²",
        "g,grav,standard gravity,g0,g_n",
    ),
    (
        "Atomic mass unit",
        "1.66053906660e-27",
        "kg",
        "amu,u,dalton,atomic mass unit",
    ),
    (
        "Standard atmosphere",
        "101325",
        "Pa",
        "atm,atmosphere,standard atmosphere",
    ),
    (
        "Electron volt",
        "1.602176634e-19",
        "J",
        "eV,electronvolt,electron volt",
    ),
    (
        "Speed of sound (20°C)",
        "343",
        "m/s",
        "sound,speed of sound,vsound",
    ),
    (
        "Molar volume (STP)",
        "22.414",
        "L/mol",
        "molar volume,vm,STP volume",
    ),
    (
        "Wien displacement",
        "2.897771955e-3",
        "m·K",
        "wien,wien displacement,b",
    ),
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

    let matches: Vec<_> = CONSTANTS
        .iter()
        .filter(|(name, _, _, aliases)| {
            name.to_lowercase().contains(&q) || aliases.to_lowercase().contains(&q)
        })
        .collect();

    if matches.is_empty() {
        let _ = writeln!(
            out,
            "No constant found for '{}'. Use --const list to see all.",
            query.trim()
        );
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
        let _ = writeln!(out);
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
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-x * x).exp();
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
        let num = c[0] + c[1] * t + c[2] * t * t;
        let den = 1.0 + d[0] * t + d[1] * t * t + d[2] * t * t * t;
        (-(t - num / den), p)
    } else {
        let t = (-2.0 * (1.0 - p).ln()).sqrt();
        let c = [2.515517, 0.802853, 0.010328];
        let d = [1.432788, 0.189269, 0.001308];
        let num = c[0] + c[1] * t + c[2] * t * t;
        let den = 1.0 + d[0] * t + d[1] * t * t + d[2] * t * t * t;
        (t - num / den, p)
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
            let x = parse_f(parts[1]).unwrap_or(0.0);
            let mu = if parts.len() > 2 {
                parse_f(parts[2]).unwrap_or(0.0)
            } else {
                0.0
            };
            let sg = if parts.len() > 3 {
                parse_f(parts[3]).unwrap_or(1.0)
            } else {
                1.0
            };
            let p = normal_cdf(x, mu, sg);
            let z = (x - mu) / sg;
            let _ = writeln!(out, "Distribution: N({}, {})", mu, sg);
            let _ = writeln!(out, "x = {}", x);
            let _ = writeln!(out, "z-score = {:.6}", z);
            let _ = writeln!(out, "P(X ≤ {}) = {:.8}  ({:.4}%)", x, p, p * 100.0);
            let _ = writeln!(
                out,
                "P(X > {}) = {:.8}  ({:.4}%)",
                x,
                1.0 - p,
                (1.0 - p) * 100.0
            );
        }
        "pdf" if parts.len() >= 2 => {
            let x = parse_f(parts[1]).unwrap_or(0.0);
            let mu = if parts.len() > 2 {
                parse_f(parts[2]).unwrap_or(0.0)
            } else {
                0.0
            };
            let sg = if parts.len() > 3 {
                parse_f(parts[3]).unwrap_or(1.0)
            } else {
                1.0
            };
            let p = normal_pdf(x, mu, sg);
            let _ = writeln!(out, "PDF at x={}: {:.8}", x, p);
        }
        "inv" | "quantile" | "ppf" if parts.len() >= 2 => {
            let p = parse_f(parts[1]).unwrap_or(0.5);
            let mu = if parts.len() > 2 {
                parse_f(parts[2]).unwrap_or(0.0)
            } else {
                0.0
            };
            let sg = if parts.len() > 3 {
                parse_f(parts[3]).unwrap_or(1.0)
            } else {
                1.0
            };
            let z = normal_inv_cdf(p);
            let x = mu + sg * z;
            let _ = writeln!(out, "P = {} → z-score = {:.6} → x = {:.6}", p, z, x);
            let _ = writeln!(out, "Interpretation: P(X ≤ {:.6}) = {:.4}%", x, p * 100.0);
        }
        "between" | "interval" if parts.len() >= 3 => {
            let a = parse_f(parts[1]).unwrap_or(-1.96);
            let b = parse_f(parts[2]).unwrap_or(1.96);
            let mu = if parts.len() > 3 {
                parse_f(parts[3]).unwrap_or(0.0)
            } else {
                0.0
            };
            let sg = if parts.len() > 4 {
                parse_f(parts[4]).unwrap_or(1.0)
            } else {
                1.0
            };
            let p = normal_cdf(b, mu, sg) - normal_cdf(a, mu, sg);
            let _ = writeln!(out, "P({} ≤ X ≤ {}) = {:.8}  ({:.4}%)", a, b, p, p * 100.0);
        }
        "table" | "z-table" => {
            let _ = writeln!(out, "Standard Normal CDF  P(Z ≤ z)");
            let _ = writeln!(out, "  z     P(Z ≤ z)   P(Z > z)");
            let _ = writeln!(out, "  ─────────────────────────────");
            for &z in &[
                -3.0f64, -2.576, -2.326, -1.960, -1.645, -1.282, -0.842, 0.0, 0.842, 1.282, 1.645,
                1.960, 2.326, 2.576, 3.0,
            ] {
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

// ── Multi-distribution probability calculator ─────────────────────────────────
// Distributions: normal, binomial, poisson, t, chi2, exponential, uniform, geometric
// Operations: pdf/pmf, cdf, quantile/inv
//
// Syntax: DIST [op] PARAMS
//   normal cdf 1.96              P(Z ≤ 1.96)
//   normal pdf 0 mu=2 sd=1       f(0) for N(2,1)
//   binomial pmf 3 n=10 p=0.4    P(X=3) for Bin(10, 0.4)
//   binomial cdf 5 n=10 p=0.4    P(X ≤ 5)
//   poisson pmf 2 lam=3          P(X=2) for Poi(3)
//   t cdf 2.0 df=9               P(T ≤ 2.0) for t(9)
//   chi2 cdf 5.99 df=2           P(X ≤ 5.99) for chi²(2)
//   exponential cdf 1.0 lam=0.5  P(X ≤ 1.0) for Exp(0.5)
//   uniform cdf 0.7 a=0 b=1      P(X ≤ 0.7) for Uniform(0,1)
//   geometric pmf 3 p=0.5        P(X=3) for Geo(0.5)
pub fn prob_calc(query: &str) -> String {
    use std::f64::consts::PI;
    let q = query.trim().to_lowercase();
    if q.is_empty() || q == "help" || q == "?" {
        return prob_usage();
    }

    let parts: Vec<&str> = q.split_whitespace().collect();
    if parts.is_empty() {
        return prob_usage();
    }

    // Parse named params like mu=2.5, sd=1.0
    let get_param = |parts: &[&str], name: &str, default: f64| -> f64 {
        parts
            .iter()
            .find(|s| s.starts_with(name))
            .and_then(|s| s.split_once('=').map(|x| x.1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let get_positional = |parts: &[&str], idx: usize| -> Option<f64> {
        parts
            .iter()
            .filter(|s| !s.contains('='))
            .nth(idx)
            .and_then(|s| s.parse().ok())
    };

    let dist = parts[0];
    // Determine op: second token if it's a known op, else "all"
    let op_candidate = parts.get(1).copied().unwrap_or("all");
    let (op, data_start) = if matches!(
        op_candidate,
        "cdf" | "pdf" | "pmf" | "inv" | "quantile" | "between" | "all"
    ) {
        (op_candidate, 2usize)
    } else {
        ("all", 1usize)
    };
    let data: &[&str] = &parts[data_start..];

    let mut out = String::new();
    let sep = "=".repeat(64);

    match dist {
        "normal" | "norm" | "gaussian" => {
            let x = get_positional(data, 0).unwrap_or(0.0);
            let mu = get_param(data, "mu", get_param(data, "mean", 0.0));
            let sd = get_param(
                data,
                "sd",
                get_param(data, "sigma", get_param(data, "std", 1.0)),
            );
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(out, "  Normal Distribution  N(μ={}, σ={})", mu, sd);
            let _ = writeln!(out, "{}", sep);
            if op == "pdf" || op == "all" {
                let _ = writeln!(out, "  PDF f({}) = {:.8}", x, normal_pdf(x, mu, sd));
            }
            if op == "cdf" || op == "all" {
                let p = normal_cdf(x, mu, sd);
                let _ = writeln!(out, "  CDF P(X ≤ {}) = {:.8}", x, p);
                let _ = writeln!(out, "  P(X > {})     = {:.8}", x, 1.0 - p);
                let z = (x - mu) / sd;
                let _ = writeln!(out, "  z-score       = {:.4}", z);
            }
            if op == "inv" || op == "quantile" {
                let p = get_positional(data, 0).unwrap_or(0.975);
                let z = normal_inv_cdf(p);
                let _ = writeln!(out, "  Quantile p={:.4}  →  x = {:.6}", p, mu + sd * z);
            }
            if op == "between" {
                let a = get_positional(data, 0).unwrap_or(-1.0);
                let b = get_positional(data, 1).unwrap_or(1.0);
                let p = normal_cdf(b, mu, sd) - normal_cdf(a, mu, sd);
                let _ = writeln!(out, "  P({} ≤ X ≤ {}) = {:.8}", a, b, p);
            }
            if op == "all" {
                // Common quantiles
                let _ = writeln!(
                    out,
                    "  Quantiles: p=.025 → {:.4}  p=.975 → {:.4}  p=.5 → {:.4}",
                    mu + sd * normal_inv_cdf(0.025),
                    mu + sd * normal_inv_cdf(0.975),
                    mu
                );
            }
        }
        "binomial" | "binom" | "bin" => {
            let k = get_positional(data, 0).unwrap_or(0.0) as i64;
            let n = get_param(data, "n", 10.0) as u64;
            let p = get_param(data, "p", 0.5);
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(out, "  Binomial Distribution  Bin(n={}, p={})", n, p);
            let _ = writeln!(out, "{}", sep);
            let mean = n as f64 * p;
            let var = n as f64 * p * (1.0 - p);
            let _ = writeln!(
                out,
                "  Mean={:.4}  Var={:.4}  StdDev={:.4}",
                mean,
                var,
                var.sqrt()
            );
            let binom_pmf = |k: i64, n: u64, p: f64| -> f64 {
                if k < 0 || k > n as i64 {
                    return 0.0;
                }
                let k = k as u64;
                // Log-space computation to avoid overflow
                let log_binom: f64 = log_factorial(n) - log_factorial(k) - log_factorial(n - k);
                (log_binom + k as f64 * p.ln() + (n - k) as f64 * (1.0 - p).ln()).exp()
            };
            if op == "pmf" || op == "all" {
                let pmf = binom_pmf(k, n, p);
                let _ = writeln!(out, "  PMF P(X={}) = {:.8}", k, pmf);
            }
            if op == "cdf" || op == "all" {
                let cdf: f64 = (0..=k).map(|i| binom_pmf(i, n, p)).sum();
                let _ = writeln!(out, "  CDF P(X≤{}) = {:.8}", k, cdf);
                let _ = writeln!(out, "  P(X>{})     = {:.8}", k, 1.0 - cdf);
            }
            if op == "all" {
                // Show distribution for small n
                if n <= 20 {
                    let _ = writeln!(out, "  PMF table:");
                    for i in 0..=n {
                        let pmf = binom_pmf(i as i64, n, p);
                        let bar = (pmf * 50.0) as usize;
                        let _ = writeln!(out, "    k={:>3}  {:.6}  {}", i, pmf, "#".repeat(bar));
                    }
                }
            }
        }
        "poisson" | "pois" => {
            let k = get_positional(data, 0).unwrap_or(0.0) as i64;
            let lam = get_param(data, "lam", get_param(data, "lambda", 1.0));
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(out, "  Poisson Distribution  Poi(λ={})", lam);
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(
                out,
                "  Mean={:.4}  Var={:.4}  StdDev={:.4}",
                lam,
                lam,
                lam.sqrt()
            );
            let pois_pmf = |k: i64, lam: f64| -> f64 {
                if k < 0 {
                    return 0.0;
                }
                (-lam + k as f64 * lam.ln() - log_factorial(k as u64)).exp()
            };
            if op == "pmf" || op == "all" {
                let _ = writeln!(out, "  PMF P(X={}) = {:.8}", k, pois_pmf(k, lam));
            }
            if op == "cdf" || op == "all" {
                let cdf: f64 = (0..=k).map(|i| pois_pmf(i, lam)).sum();
                let _ = writeln!(out, "  CDF P(X≤{}) = {:.8}", k, cdf);
                let _ = writeln!(out, "  P(X>{})     = {:.8}", k, 1.0 - cdf);
            }
            if op == "all" && lam <= 30.0 {
                let hi = (lam + 4.0 * lam.sqrt()).ceil() as i64;
                let _ = writeln!(out, "  PMF table (up to k={}):", hi);
                for i in 0..=hi.min(40) {
                    let pmf = pois_pmf(i, lam);
                    let bar = (pmf * 60.0) as usize;
                    let _ = writeln!(out, "    k={:>3}  {:.6}  {}", i, pmf, "#".repeat(bar));
                }
            }
        }
        "t" | "student" | "t-dist" => {
            let x = get_positional(data, 0).unwrap_or(0.0);
            let df = get_param(data, "df", get_param(data, "dof", 1.0));
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(out, "  Student's t Distribution  t(df={})", df);
            let _ = writeln!(out, "{}", sep);
            let t_pdf = |x: f64, df: f64| -> f64 {
                let lg_n = lgamma((df + 1.0) / 2.0);
                let lg_d = lgamma(df / 2.0);
                let coef = (lg_n - lg_d - 0.5 * (df * PI).ln()).exp();
                coef * (1.0 + x * x / df).powf(-(df + 1.0) / 2.0)
            };
            // t CDF via regularized incomplete beta
            let t_cdf = |x: f64, df: f64| -> f64 {
                let t2 = x * x;
                let z = df / (df + t2);
                let ib = reg_inc_beta(df / 2.0, 0.5, z);
                if x >= 0.0 {
                    1.0 - 0.5 * ib
                } else {
                    0.5 * ib
                }
            };
            if op == "pdf" || op == "all" {
                let _ = writeln!(out, "  PDF f({:.4}) = {:.8}", x, t_pdf(x, df));
            }
            if op == "cdf" || op == "all" {
                let p = t_cdf(x, df);
                let _ = writeln!(out, "  CDF P(T≤{:.4}) = {:.8}", x, p);
                let _ = writeln!(
                    out,
                    "  Two-tailed P  = {:.8}  (p-value for |T|≥{:.4})",
                    2.0 * (1.0 - t_cdf(x.abs(), df)),
                    x.abs()
                );
            }
            if op == "all" {
                // Common critical values
                let _ = writeln!(out, "  Critical values (two-tailed):");
                for alpha in [0.10, 0.05, 0.01, 0.001] {
                    // Bisect to find t s.t. 2*(1-cdf(t)) = alpha
                    let target = 1.0 - alpha / 2.0;
                    let cv = bisect(|t| t_cdf(t, df) - target, 0.0, 100.0, 60);
                    let _ = writeln!(out, "    α={:.3}  t* = {:.4}", alpha, cv);
                }
            }
        }
        "chi2" | "chisquare" | "chi-square" | "chi_sq" => {
            let x = get_positional(data, 0).unwrap_or(1.0);
            let df = get_param(data, "df", get_param(data, "dof", 1.0));
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(out, "  Chi-Square Distribution  χ²(df={})", df);
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(
                out,
                "  Mean={:.4}  Var={:.4}  StdDev={:.4}",
                df,
                2.0 * df,
                (2.0 * df).sqrt()
            );
            let chi2_pdf = |x: f64, k: f64| -> f64 {
                if x <= 0.0 {
                    return 0.0;
                }
                let lg = lgamma(k / 2.0);
                ((k / 2.0 - 1.0) * x.ln() - x / 2.0 - (k / 2.0) * 2.0f64.ln() - lg).exp()
            };
            // Chi2 CDF via regularized incomplete gamma
            let chi2_cdf = |x: f64, k: f64| -> f64 {
                if x <= 0.0 {
                    return 0.0;
                }
                reg_inc_gamma(k / 2.0, x / 2.0)
            };
            if op == "pdf" || op == "all" {
                let _ = writeln!(out, "  PDF f({:.4}) = {:.8}", x, chi2_pdf(x, df));
            }
            if op == "cdf" || op == "all" {
                let p = chi2_cdf(x, df);
                let _ = writeln!(out, "  CDF P(X≤{:.4}) = {:.8}", x, p);
                let _ = writeln!(out, "  P-value (upper) = {:.8}", 1.0 - p);
            }
            if op == "all" {
                let _ = writeln!(out, "  Critical values (upper tail):");
                for alpha in [0.10, 0.05, 0.025, 0.01] {
                    let cv = bisect(|t| chi2_cdf(t, df) - (1.0 - alpha), 0.0, df + 100.0, 80);
                    let _ = writeln!(out, "    α={:.3}  χ² = {:.4}", alpha, cv);
                }
            }
        }
        "exponential" | "exp" | "expon" => {
            let x = get_positional(data, 0).unwrap_or(1.0);
            let lam = get_param(
                data,
                "lam",
                get_param(data, "lambda", get_param(data, "rate", 1.0)),
            );
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(out, "  Exponential Distribution  Exp(λ={})", lam);
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(
                out,
                "  Mean={:.4}  StdDev={:.4}  Median={:.4}",
                1.0 / lam,
                1.0 / lam,
                2.0f64.ln() / lam
            );
            if op == "pdf" || op == "all" {
                let pdf = if x >= 0.0 {
                    lam * (-lam * x).exp()
                } else {
                    0.0
                };
                let _ = writeln!(out, "  PDF f({:.4}) = {:.8}", x, pdf);
            }
            if op == "cdf" || op == "all" {
                let p = if x >= 0.0 {
                    1.0 - (-lam * x).exp()
                } else {
                    0.0
                };
                let _ = writeln!(out, "  CDF P(X≤{:.4}) = {:.8}", x, p);
                let _ = writeln!(out, "  P(X>{:.4})    = {:.8}", x, 1.0 - p);
                let _ = writeln!(out, "  Quantile p=.5  → {:.6}  (median)", 2.0f64.ln() / lam);
            }
        }
        "uniform" | "unif" => {
            let x = get_positional(data, 0).unwrap_or(0.5);
            let a = get_param(data, "a", get_param(data, "lo", 0.0));
            let b = get_param(data, "b", get_param(data, "hi", 1.0));
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(out, "  Uniform Distribution  U({}, {})", a, b);
            let _ = writeln!(out, "{}", sep);
            let rng = b - a;
            let _ = writeln!(
                out,
                "  Mean={:.4}  Var={:.6}  StdDev={:.4}",
                (a + b) / 2.0,
                rng * rng / 12.0,
                (rng * rng / 12.0).sqrt()
            );
            if op == "pdf" || op == "all" {
                let pdf = if x >= a && x <= b { 1.0 / rng } else { 0.0 };
                let _ = writeln!(out, "  PDF f({:.4}) = {:.8}", x, pdf);
            }
            if op == "cdf" || op == "all" {
                let p = if x < a {
                    0.0
                } else if x > b {
                    1.0
                } else {
                    (x - a) / rng
                };
                let _ = writeln!(out, "  CDF P(X≤{:.4}) = {:.8}", x, p);
            }
        }
        "geometric" | "geo" | "geom" => {
            let k = get_positional(data, 0).unwrap_or(1.0) as i64;
            let p = get_param(data, "p", 0.5);
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(
                out,
                "  Geometric Distribution  Geo(p={})  [trials until first success]",
                p
            );
            let _ = writeln!(out, "{}", sep);
            let _ = writeln!(
                out,
                "  Mean={:.4}  Var={:.4}  StdDev={:.4}",
                1.0 / p,
                (1.0 - p) / (p * p),
                ((1.0 - p) / (p * p)).sqrt()
            );
            let geo_pmf = |k: i64, p: f64| -> f64 {
                if k < 1 {
                    return 0.0;
                }
                (1.0 - p).powi((k - 1) as i32) * p
            };
            if op == "pmf" || op == "all" {
                let _ = writeln!(out, "  PMF P(X={}) = {:.8}", k, geo_pmf(k, p));
            }
            if op == "cdf" || op == "all" {
                let cdf = 1.0 - (1.0 - p).powi(k as i32);
                let _ = writeln!(out, "  CDF P(X≤{}) = {:.8}", k, cdf);
                let _ = writeln!(out, "  P(X>{})    = {:.8}", k, 1.0 - cdf);
            }
            if op == "all" {
                let _ = writeln!(out, "  PMF table:");
                for i in 1..=10i64.min((10.0 / p) as i64) {
                    let pmf = geo_pmf(i, p);
                    let bar = (pmf * 50.0) as usize;
                    let _ = writeln!(out, "    k={:>3}  {:.6}  {}", i, pmf, "#".repeat(bar));
                }
            }
        }
        _ => {
            out.push_str(&prob_usage());
        }
    }

    if out.trim().is_empty() || out.starts_with("Probability") {
        return out;
    }
    let _ = writeln!(out, "{}", sep);
    out
}

fn log_factorial(n: u64) -> f64 {
    (1..=n).map(|i| (i as f64).ln()).sum::<f64>()
}

fn lgamma(x: f64) -> f64 {
    // Lanczos approximation
    let g = 7.0;
    let c: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.5203681218851,
        -1259.1392167224028,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507343278686905,
        -0.13857109526572012,
        9.984_369_578_019_572e-6,
        1.5056327351493116e-7,
    ];
    if x < 0.5 {
        PI.ln() - (PI * x).sin().ln() - lgamma(1.0 - x)
    } else {
        let x = x - 1.0;
        let mut a = c[0];
        for i in 1..9 {
            a += c[i] / (x + i as f64);
        }
        let t = x + g + 0.5;
        0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
    }
}

fn reg_inc_beta(a: f64, b: f64, x: f64) -> f64 {
    // Regularized incomplete beta via continued fraction (Lentz's method)
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    // Use symmetry for stability
    if x > (a + 1.0) / (a + b + 2.0) {
        return 1.0 - reg_inc_beta(b, a, 1.0 - x);
    }
    let lbeta = lgamma(a) + lgamma(b) - lgamma(a + b);
    let front = (a * x.ln() + b * (1.0 - x).ln() - lbeta).exp() / a;
    // Lentz continued fraction
    let mut c_cf = 1.0;
    let mut d_cf = 1.0 - (a + b) * x / (a + 1.0);
    if d_cf.abs() < 1e-30 {
        d_cf = 1e-30;
    }
    d_cf = 1.0 / d_cf;
    let mut f = d_cf;
    for m in 1..200usize {
        let mf = m as f64;
        // Even step
        let nm = mf * (b - mf) * x / ((a + 2.0 * mf - 1.0) * (a + 2.0 * mf));
        d_cf = 1.0 + nm * d_cf;
        if d_cf.abs() < 1e-30 {
            d_cf = 1e-30;
        }
        c_cf = 1.0 + nm / c_cf;
        if c_cf.abs() < 1e-30 {
            c_cf = 1e-30;
        }
        d_cf = 1.0 / d_cf;
        f *= d_cf * c_cf;
        // Odd step
        let nm2 = -(a + mf) * (a + b + mf) * x / ((a + 2.0 * mf) * (a + 2.0 * mf + 1.0));
        d_cf = 1.0 + nm2 * d_cf;
        if d_cf.abs() < 1e-30 {
            d_cf = 1e-30;
        }
        c_cf = 1.0 + nm2 / c_cf;
        if c_cf.abs() < 1e-30 {
            c_cf = 1e-30;
        }
        d_cf = 1.0 / d_cf;
        let delta = d_cf * c_cf;
        f *= delta;
        if (delta - 1.0).abs() < 3e-7 {
            break;
        }
    }
    front * f
}

fn reg_inc_gamma(a: f64, x: f64) -> f64 {
    // Regularized lower incomplete gamma via series expansion
    if x <= 0.0 {
        return 0.0;
    }
    if x > a + 1.0 {
        // Use continued fraction for large x
        return 1.0 - reg_inc_gamma_upper(a, x);
    }
    let mut sum = 1.0 / a;
    let mut term = 1.0 / a;
    for n in 1..200u64 {
        term *= x / (a + n as f64);
        sum += term;
        if term.abs() < 1e-10 * sum.abs() {
            break;
        }
    }
    sum * (-x + a * x.ln() - lgamma(a)).exp()
}

fn reg_inc_gamma_upper(a: f64, x: f64) -> f64 {
    // Regularized upper incomplete gamma via Lentz CF
    let mut c_cf = 1.0;
    let b0 = x + 1.0 - a;
    let mut d_cf = if b0.abs() < 1e-30 { 1e30 } else { 1.0 / b0 };
    let mut f = d_cf;
    for i in 1..200u64 {
        let ai = -(i as f64) * (i as f64 - a);
        let bi = x + (2 * i + 1) as f64 - a;
        d_cf = bi + ai * d_cf;
        if d_cf.abs() < 1e-30 {
            d_cf = 1e-30;
        }
        c_cf = bi + ai / c_cf;
        if c_cf.abs() < 1e-30 {
            c_cf = 1e-30;
        }
        d_cf = 1.0 / d_cf;
        let delta = d_cf * c_cf;
        f *= delta;
        if (delta - 1.0).abs() < 3e-7 {
            break;
        }
    }
    f * (-x + a * x.ln() - lgamma(a)).exp()
}

fn bisect<F: Fn(f64) -> f64>(f: F, mut lo: f64, mut hi: f64, iters: usize) -> f64 {
    for _ in 0..iters {
        let mid = (lo + hi) / 2.0;
        if f(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

fn prob_usage() -> String {
    "Probability distributions — instant, no model, no cloud:\n\
     \n\
     hematite --probability 'normal cdf 1.96'              P(Z ≤ 1.96) std normal\n\
     hematite --probability 'normal cdf 70 mu=60 sd=10'    P(X ≤ 70) for N(60,10)\n\
     hematite --probability 'binomial pmf 3 n=10 p=0.4'    P(X=3) Bin(10,0.4)\n\
     hematite --probability 'binomial cdf 5 n=10 p=0.4'    P(X≤5) Bin(10,0.4)\n\
     hematite --probability 'poisson pmf 2 lam=3'          P(X=2) Poi(3)\n\
     hematite --probability 't cdf 2.0 df=9'               P(T≤2.0) t(9)\n\
     hematite --probability 'chi2 cdf 5.99 df=2'           P(X≤5.99) chi²(2)\n\
     hematite --probability 'exponential cdf 1.0 lam=0.5'  P(X≤1) Exp(0.5)\n\
     hematite --probability 'uniform cdf 0.7 a=0 b=1'      P(X≤0.7) U(0,1)\n\
     hematite --probability 'geometric pmf 3 p=0.5'        P(X=3) Geo(0.5)\n\
     \n\
     Operations: pdf/pmf  cdf  inv/quantile  between  (default: show all)\n\
     Distributions: normal  binomial  poisson  t  chi2  exponential  uniform  geometric"
        .into()
}

// ── Unit conversion ───────────────────────────────────────────────────────────
// Query forms:
//   "5 km to miles"   "32 F to C"   "1 atm to Pa"   "100 mph to km/h"
//   "list" or "units" — show all supported categories and units

pub fn unit_convert(query: &str) -> String {
    let q = query.trim();
    if q.eq_ignore_ascii_case("list")
        || q.eq_ignore_ascii_case("units")
        || q.eq_ignore_ascii_case("help")
    {
        return unit_convert_list();
    }

    // Parse: "<value> <from_unit> to <to_unit>"
    // Also accept: "<value> <from_unit> in <to_unit>"
    let lower = q.to_lowercase();
    let sep = if lower.contains(" to ") {
        " to "
    } else if lower.contains(" in ") {
        " in "
    } else {
        ""
    };

    if sep.is_empty() {
        return "Usage: hematite --convert '5 km to miles'\n\
             Common examples:\n\
             hematite --convert '100 f to c'\n\
             hematite --convert '1 atm to Pa'\n\
             hematite --convert '60 mph to km/h'\n\
             hematite --convert '1 GiB to MB'\n\
             hematite --convert '1 cal to J'\n\
             hematite --convert 'list'    (show all units)"
            .to_string();
    }

    let parts: Vec<&str> = q
        .splitn(2, &sep.to_uppercase().as_str().to_string())
        .collect();
    let parts: Vec<&str> = if parts.len() < 2 {
        q.splitn(2, sep).collect()
    } else {
        parts
    };

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
                format!("{:.8}", result)
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            } else {
                format!("{:.6e}", result)
            };
            format!(
                "{} {} = {} {}\n({})",
                value_str.trim(),
                from_unit.trim(),
                result_str,
                to_unit.trim(),
                category
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
    if end == 0 {
        end = s.len();
    }
    (&s[..end], s[end..].trim())
}

// Returns (converted_value, category_name) or Err(message)
fn convert(value: f64, from: &str, to: &str) -> Result<(f64, &'static str), String> {
    // Normalize unit names
    let from_n = norm_unit(from);
    let to_n = norm_unit(to);

    // Walk categories
    for (cat_name, units) in UNIT_CATEGORIES {
        // Find from_unit base factor
        let from_entry = units
            .iter()
            .find(|(names, _)| names.contains(&from_n.as_str()));
        let to_entry = units
            .iter()
            .find(|(names, _)| names.contains(&to_n.as_str()));

        if let (Some(fe), Some(te)) = (from_entry, to_entry) {
            // Temperature handled specially
            if *cat_name == "Temperature" {
                return Ok((
                    convert_temperature(value, from_n.as_str(), to_n.as_str()),
                    "Temperature",
                ));
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
    let mut close: Vec<&str> = known
        .iter()
        .filter(|n| levenshtein(n, from_n.as_str()) <= 2)
        .copied()
        .collect();
    close.extend(
        known
            .iter()
            .filter(|n| levenshtein(n, to_n.as_str()) <= 2)
            .copied(),
    );
    close.dedup();

    if close.is_empty() {
        Err(format!(
            "Unknown units: '{}' or '{}'. Run 'hematite --convert list' to see all.",
            from, to
        ))
    } else {
        Err(format!("Unknown unit(s). Did you mean: {}?\nRun 'hematite --convert list' for all supported units.", close.join(", ")))
    }
}

fn norm_unit(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .replace("°", "")
        .replace("²", "2")
        .replace("³", "3")
        .replace("/s", "_per_s")
        .replace("per second", "_per_s")
}

fn convert_temperature(value: f64, from: &str, to: &str) -> f64 {
    // Convert to Kelvin first
    let kelvin = match from {
        "c" | "celsius" => value + 273.15,
        "f" | "fahrenheit" => (value - 32.0) * 5.0 / 9.0 + 273.15,
        "k" | "kelvin" => value,
        "r" | "rankine" => value * 5.0 / 9.0,
        _ => value,
    };
    match to {
        "c" | "celsius" => kelvin - 273.15,
        "f" | "fahrenheit" => (kelvin - 273.15) * 9.0 / 5.0 + 32.0,
        "k" | "kelvin" => kelvin,
        "r" | "rankine" => kelvin * 9.0 / 5.0,
        _ => kelvin,
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i;
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

// Each entry: (&[unit_aliases], factor_to_SI_base)
// Temperature is handled separately (non-linear)
type UnitEntry = (&'static [&'static str], f64);
type UnitCategory = (&'static str, &'static [UnitEntry]);

static UNIT_CATEGORIES: &[UnitCategory] = &[
    (
        "Length",
        &[
            (&["m", "meter", "meters", "metre", "metres"], 1.0),
            (
                &["km", "kilometer", "kilometers", "kilometre", "kilometres"],
                1000.0,
            ),
            (
                &[
                    "cm",
                    "centimeter",
                    "centimeters",
                    "centimetre",
                    "centimetres",
                ],
                0.01,
            ),
            (
                &[
                    "mm",
                    "millimeter",
                    "millimeters",
                    "millimetre",
                    "millimetres",
                ],
                0.001,
            ),
            (
                &["um", "micrometer", "micrometers", "micron", "microns"],
                1e-6,
            ),
            (
                &["nm", "nanometer", "nanometers", "nanometre", "nanometres"],
                1e-9,
            ),
            (&["mi", "mile", "miles"], 1609.344),
            (&["yd", "yard", "yards"], 0.9144),
            (&["ft", "foot", "feet"], 0.3048),
            (&["in", "inch", "inches"], 0.0254),
            (&["nmi", "nautical_mile", "nautical_miles"], 1852.0),
            (
                &["ly", "light_year", "light_years", "lightyear", "lightyears"],
                9.460_730_472_580_8e15,
            ),
            (
                &["au", "astronomical_unit", "astronomical_units"],
                1.495978707e11,
            ),
            (&["pc", "parsec", "parsecs"], 3.085677581e16),
            (&["ang", "angstrom", "angstroms"], 1e-10),
        ],
    ),
    (
        "Mass",
        &[
            (&["kg", "kilogram", "kilograms", "kilogramme"], 1.0),
            (&["g", "gram", "grams", "gramme"], 0.001),
            (&["mg", "milligram", "milligrams", "milligramme"], 1e-6),
            (&["ug", "microgram", "micrograms"], 1e-9),
            (
                &["t", "tonne", "tonnes", "metric_ton", "metric_tons"],
                1000.0,
            ),
            (&["lb", "lbs", "pound", "pounds"], 0.45359237),
            (&["oz", "ounce", "ounces"], 0.028349523125),
            (&["st", "stone", "stones"], 6.35029318),
            (&["ton", "short_ton", "short_tons"], 907.18474),
            (&["long_ton", "long_tons"], 1016.0469088),
            (&["gr", "grain", "grains"], 6.479891e-5),
            (&["u", "amu", "dalton", "daltons", "da"], 1.66053906660e-27),
        ],
    ),
    (
        "Temperature",
        &[
            (&["c", "celsius"], 1.0), // factors unused
            (&["f", "fahrenheit"], 1.0),
            (&["k", "kelvin"], 1.0),
            (&["r", "rankine"], 1.0),
        ],
    ),
    (
        "Time",
        &[
            (&["s", "sec", "second", "seconds"], 1.0),
            (&["ms", "millisecond", "milliseconds"], 0.001),
            (&["us", "microsecond", "microseconds"], 1e-6),
            (&["ns", "nanosecond", "nanoseconds"], 1e-9),
            (&["min", "minute", "minutes"], 60.0),
            (&["h", "hr", "hour", "hours"], 3600.0),
            (&["d", "day", "days"], 86400.0),
            (&["wk", "week", "weeks"], 604800.0),
            (&["mo", "month", "months"], 2629800.0),
            (&["yr", "year", "years"], 31557600.0),
        ],
    ),
    (
        "Area",
        &[
            (
                &["m2", "sqm", "square_meter", "square_meters", "square_metre"],
                1.0,
            ),
            (&["km2", "sqkm", "square_kilometer", "square_km"], 1e6),
            (
                &["cm2", "sqcm", "square_centimeter", "square_centimeters"],
                1e-4,
            ),
            (
                &["mm2", "sqmm", "square_millimeter", "square_millimeters"],
                1e-6,
            ),
            (&["ha", "hectare", "hectares"], 1e4),
            (&["ac", "acre", "acres"], 4046.8564224),
            (&["sqft", "sq_ft", "square_foot", "square_feet"], 0.09290304),
            (
                &["sqin", "sq_in", "square_inch", "square_inches"],
                6.4516e-4,
            ),
            (
                &["sqmi", "sq_mi", "square_mile", "square_miles"],
                2589988.110336,
            ),
            (
                &["sqyd", "sq_yd", "square_yard", "square_yards"],
                0.83612736,
            ),
        ],
    ),
    (
        "Volume",
        &[
            (&["m3", "cubic_meter", "cubic_meters", "cubic_metre"], 1.0),
            (&["l", "liter", "liters", "litre", "litres"], 0.001),
            (&["ml", "milliliter", "milliliters", "millilitre"], 1e-6),
            (&["cl", "centiliter", "centiliters"], 1e-5),
            (&["dl", "deciliter", "deciliters"], 1e-4),
            (&["ul", "microliter", "microliters"], 1e-9),
            (
                &["cm3", "cc", "cubic_centimeter", "cubic_centimeters"],
                1e-6,
            ),
            (&["mm3", "cubic_millimeter", "cubic_millimeters"], 1e-9),
            (&["km3", "cubic_kilometer", "cubic_kilometers"], 1e9),
            (&["ft3", "cubic_foot", "cubic_feet"], 0.0283168466),
            (&["in3", "cubic_inch", "cubic_inches"], 1.6387064e-5),
            (&["yd3", "cubic_yard", "cubic_yards"], 0.764554858),
            (&["gal", "gallon", "gallons"], 0.003785411784),
            (&["qt", "quart", "quarts"], 9.46352946e-4),
            (&["pt", "pint", "pints"], 4.73176473e-4),
            (&["cup", "cups"], 2.36588237e-4),
            (
                &["floz", "fl_oz", "fluid_ounce", "fluid_ounces"],
                2.95735296e-5,
            ),
            (&["tbsp", "tablespoon", "tablespoons"], 1.47867648e-5),
            (&["tsp", "teaspoon", "teaspoons"], 4.92892159e-6),
            (&["bbl", "barrel", "barrels"], 0.158987295),
            (
                &["gal_uk", "uk_gallon", "imperial_gallon", "imperial_gallons"],
                0.00454609,
            ),
        ],
    ),
    (
        "Speed",
        &[
            (&["m_per_s", "m/s", "mps"], 1.0),
            (&["km_per_s", "km/s", "kmps"], 1000.0),
            (
                &["km/h", "kmh", "kph", "km_per_h", "km_per_hour"],
                1.0 / 3.6,
            ),
            (&["mph", "mi/h", "mi_per_h", "miles_per_hour"], 0.44704),
            (&["knot", "knots", "kn"], 0.514444),
            (&["ft_per_s", "ft/s", "fps"], 0.3048),
            (&["c_speed", "speed_of_light"], 299792458.0),
            (&["mach"], 340.29),
        ],
    ),
    (
        "Force",
        &[
            (&["n", "newton", "newtons"], 1.0),
            (&["kn", "kilonewton", "kilonewtons"], 1000.0),
            (&["mn", "meganewton", "meganewtons"], 1e6),
            (&["lbf", "pound_force", "pound-force"], 4.44822162),
            (&["kgf", "kilogram_force", "kilogram-force"], 9.80665),
            (&["dyn", "dyne", "dynes"], 1e-5),
            (&["ozf", "ounce_force"], 0.278013851),
        ],
    ),
    (
        "Pressure",
        &[
            (&["pa", "pascal", "pascals"], 1.0),
            (&["kpa", "kilopascal", "kilopascals"], 1000.0),
            (&["mpa", "megapascal", "megapascals"], 1e6),
            (&["gpa", "gigapascal", "gigapascals"], 1e9),
            (
                &[
                    "hpa",
                    "hectopascal",
                    "hectopascals",
                    "mbar",
                    "millibar",
                    "millibars",
                ],
                100.0,
            ),
            (&["bar", "bars"], 1e5),
            (&["atm", "atmosphere", "atmospheres"], 101325.0),
            (&["torr"], 133.322368),
            (&["mmhg", "mm_hg", "millimeter_of_mercury"], 133.322368),
            (&["psi", "pound_per_square_inch"], 6894.75729),
            (&["inhg", "in_hg", "inch_of_mercury"], 3386.389),
        ],
    ),
    (
        "Energy",
        &[
            (&["j", "joule", "joules"], 1.0),
            (&["kj", "kilojoule", "kilojoules"], 1000.0),
            (&["mj", "megajoule", "megajoules"], 1e6),
            (&["gj", "gigajoule", "gigajoules"], 1e9),
            (
                &["cal", "calorie", "calories", "thermochemical_calorie"],
                4.184,
            ),
            (
                &["kcal", "kilocalorie", "kilocalories", "food_calorie"],
                4184.0,
            ),
            (&["wh", "watt_hour", "watt_hours"], 3600.0),
            (&["kwh", "kilowatt_hour", "kilowatt_hours"], 3.6e6),
            (&["mwh", "megawatt_hour", "megawatt_hours"], 3.6e9),
            (&["ev", "electronvolt", "electronvolts"], 1.602176634e-19),
            (
                &["kev", "kiloelectronvolt", "kiloelectronvolts"],
                1.602176634e-16,
            ),
            (
                &["mev", "megaelectronvolt", "megaelectronvolts"],
                1.602176634e-13,
            ),
            (
                &["gev", "gigaelectronvolt", "gigaelectronvolts"],
                1.602176634e-10,
            ),
            (
                &["tev", "teraelectronvolt", "teraelectronvolts"],
                1.602176634e-7,
            ),
            (&["btu", "british_thermal_unit"], 1055.05585),
            (&["erg", "ergs"], 1e-7),
            (&["ft_lb", "foot_pound", "foot_pounds"], 1.35581795),
            (&["therm", "therms"], 1.05480400e8),
        ],
    ),
    (
        "Power",
        &[
            (&["w", "watt", "watts"], 1.0),
            (&["kw", "kilowatt", "kilowatts"], 1000.0),
            (&["mw", "megawatt", "megawatts"], 1e6),
            (&["gw", "gigawatt", "gigawatts"], 1e9),
            (&["tw", "terawatt", "terawatts"], 1e12),
            (&["mw_milli", "milliwatt", "milliwatts"], 0.001),
            (&["hp", "horsepower"], 745.69987),
            (&["ps", "metric_horsepower"], 735.49875),
            (&["btu_h", "btu/h", "btu_per_hour"], 0.29307107),
            (&["erg_s", "erg/s", "erg_per_second"], 1e-7),
            (&["ft_lb_s", "ft_lb/s"], 1.35581795),
        ],
    ),
    (
        "Data",
        &[
            (&["bit", "bits"], 1.0),
            (&["byte", "bytes", "b"], 8.0),
            (&["kb", "kilobit", "kilobits"], 1e3),
            (&["kib", "kibibit", "kibibits"], 1024.0),
            (&["mb", "megabit", "megabits"], 1e6),
            (&["mib", "mebibit", "mebibits"], 1024.0 * 1024.0),
            (&["gb", "gigabit", "gigabits"], 1e9),
            (&["gib", "gibibit", "gibibits"], 1024.0 * 1024.0 * 1024.0),
            (&["tb", "terabit", "terabits"], 1e12),
            (
                &["tib", "tebibit", "tebibits"],
                1024.0 * 1024.0 * 1024.0 * 1024.0,
            ),
            (&["pb", "petabit", "petabits"], 1e15),
            (&["kb_byte", "kilobyte", "kilobytes"], 8e3),
            (&["kib_byte", "kibibyte", "kibibytes"], 8.0 * 1024.0),
            (&["mb_byte", "megabyte", "megabytes"], 8e6),
            (
                &["mib_byte", "mebibyte", "mebibytes"],
                8.0 * 1024.0 * 1024.0,
            ),
            (&["gb_byte", "gigabyte", "gigabytes"], 8e9),
            (
                &["gib_byte", "gibibyte", "gibibytes"],
                8.0 * 1024.0 * 1024.0 * 1024.0,
            ),
            (&["tb_byte", "terabyte", "terabytes"], 8e12),
            (
                &["tib_byte", "tebibyte", "tebibytes"],
                8.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
            ),
        ],
    ),
    (
        "Angle",
        &[
            (&["rad", "radian", "radians"], 1.0),
            (&["deg", "degree", "degrees"], std::f64::consts::PI / 180.0),
            (
                &["grad", "gradian", "gradians", "gon", "gons"],
                std::f64::consts::PI / 200.0,
            ),
            (
                &["arcmin", "arc_minute", "arc_minutes", "minute_of_arc"],
                std::f64::consts::PI / 10800.0,
            ),
            (
                &["arcsec", "arc_second", "arc_seconds", "second_of_arc"],
                std::f64::consts::PI / 648000.0,
            ),
            (
                &["rev", "revolution", "revolutions", "turn", "turns"],
                2.0 * std::f64::consts::PI,
            ),
        ],
    ),
    (
        "Frequency",
        &[
            (&["hz", "hertz"], 1.0),
            (&["khz", "kilohertz"], 1e3),
            (&["mhz", "megahertz"], 1e6),
            (&["ghz", "gigahertz"], 1e9),
            (&["thz", "terahertz"], 1e12),
            (&["rpm", "revolutions_per_minute"], 1.0 / 60.0),
            (&["rad_per_s", "rad/s"], 1.0 / (2.0 * std::f64::consts::PI)),
        ],
    ),
    (
        "Illuminance",
        &[
            (&["lx", "lux"], 1.0),
            (&["fc", "footcandle", "footcandles"], 10.7639),
            (&["phot", "phots"], 1e4),
        ],
    ),
    (
        "Fuel Economy",
        &[
            (&["mpg", "miles_per_gallon"], 1.0),
            (&["mpg_uk", "miles_per_gallon_uk", "imperial_mpg"], 1.20095),
            (&["l_per_100km", "l/100km", "liters_per_100km"], 235.214583),
            (&["km_per_l", "km/l", "km_per_liter"], 2.35214583),
        ],
    ),
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
            if mag == 0.0 {
                return "Zero vector has no unit direction.".into();
            }
            let n: Vec<f64> = v.iter().map(|x| x / mag).collect();
            return format_vec_display("Unit vector (normalized)", &n);
        }
    }
    if let Some(rest) = strip_prefix_ci(q, "normalize") {
        if let Some(v) = parse_vec(rest.trim()) {
            let mag = vec_mag(&v);
            if mag == 0.0 {
                return "Zero vector has no unit direction.".into();
            }
            let n: Vec<f64> = v.iter().map(|x| x / mag).collect();
            return format_vec_display("Unit vector (normalized)", &n);
        }
    }

    // ── angle between two vectors ─────────────────────────────────────────────
    if let Some(rest) = strip_prefix_ci(q, "angle") {
        let vecs = find_all_vecs(rest.trim());
        if vecs.len() >= 2 {
            let a = &vecs[0];
            let b = &vecs[1];
            if a.len() != b.len() {
                return "Vectors must have the same dimension for angle.".into();
            }
            let dot = vec_dot(a, b);
            let ma = vec_mag(a);
            let mb = vec_mag(b);
            if ma == 0.0 || mb == 0.0 {
                return "Cannot compute angle involving a zero vector.".into();
            }
            let cos_theta = (dot / (ma * mb)).clamp(-1.0, 1.0);
            let deg = cos_theta.acos().to_degrees();
            let rad = cos_theta.acos();
            return format!(
                "Angle between {} and {}:\n  {:.6}°  ({:.6} radians)\n  cos θ = {:.6}",
                fmt_vec(a),
                fmt_vec(b),
                deg,
                rad,
                cos_theta
            );
        }
    }

    // ── projection ────────────────────────────────────────────────────────────
    if q.to_lowercase().contains("proj") && q.to_lowercase().contains("onto") {
        let vecs = find_all_vecs(q);
        if vecs.len() >= 2 {
            let a = &vecs[0];
            let b = &vecs[1];
            if a.len() != b.len() {
                return "Vectors must have the same dimension for projection.".into();
            }
            let b_mag2: f64 = b.iter().map(|x| x * x).sum();
            if b_mag2 == 0.0 {
                return "Cannot project onto a zero vector.".into();
            }
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
            let left = &q[..idx];
            let right = &q[idx + 5..];
            if let (Some(a), Some(b)) = (parse_vec(left.trim()), parse_vec(right.trim())) {
                if a.len() != b.len() {
                    return format!("Dimension mismatch: {} vs {}", a.len(), b.len());
                }
                let d = vec_dot(&a, &b);
                return format!("{} · {} = {}", fmt_vec(&a), fmt_vec(&b), fmt_scalar(d));
            }
        }
    }

    // cross product
    if lower.contains(" cross ") {
        if let Some(idx) = lower.find(" cross ") {
            let left = &q[..idx];
            let right = &q[idx + 7..];
            if let (Some(a), Some(b)) = (parse_vec(left.trim()), parse_vec(right.trim())) {
                if a.len() != 3 || b.len() != 3 {
                    return "Cross product requires two 3D vectors.".into();
                }
                let c = vec_cross(&a, &b);
                return format!(
                    "{} × {} = {}\n  |result| = {}",
                    fmt_vec(&a),
                    fmt_vec(&b),
                    fmt_vec(&c),
                    fmt_scalar(vec_mag(&c))
                );
            }
        }
    }

    // scalar × vector: "3 * [1,2,3]" or "[1,2,3] * 3"
    if lower.contains(" * ") {
        if let Some(idx) = q.find(" * ") {
            let left = q[..idx].trim();
            let right = q[idx + 3..].trim();
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
        let left = q[..idx].trim();
        let right = q[idx + 3..].trim();
        if let (Some(a), Some(b)) = (parse_vec(left), parse_vec(right)) {
            if a.len() != b.len() {
                return format!("Dimension mismatch: {} vs {}", a.len(), b.len());
            }
            let c: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
            return format!("{} + {} = {}", fmt_vec(&a), fmt_vec(&b), fmt_vec(&c));
        }
    }

    // vector - vector
    if let Some(idx) = q.rfind(" - ") {
        let left = q[..idx].trim();
        let right = q[idx + 3..].trim();
        if let (Some(a), Some(b)) = (parse_vec(left), parse_vec(right)) {
            if a.len() != b.len() {
                return format!("Dimension mismatch: {} vs {}", a.len(), b.len());
            }
            let c: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x - y).collect();
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
    let s = s
        .trim()
        .trim_start_matches(['[', '('])
        .trim_end_matches([']', ')']);
    let parts: Vec<&str> = if s.contains(',') {
        s.split(',').collect()
    } else {
        s.split_whitespace().collect()
    };
    if parts.is_empty() {
        return None;
    }
    let nums: Vec<f64> = parts
        .iter()
        .filter_map(|p| p.trim().parse::<f64>().ok())
        .collect();
    if nums.len() == parts.len() && !nums.is_empty() {
        Some(nums)
    } else {
        None
    }
}

fn find_all_vecs(s: &str) -> Vec<Vec<f64>> {
    // Find all bracket-delimited vectors in a string
    let mut result = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = s.chars().collect();
    while i < chars.len() {
        if chars[i] == '[' || chars[i] == '(' {
            let close = if chars[i] == '[' { ']' } else { ')' };
            if let Some(j) = chars[i + 1..].iter().position(|&c| c == close) {
                let inner: String = chars[i + 1..i + 1 + j].iter().collect();
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
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
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
        format!("{:.6}", x)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
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
            let n: u64 = tokens
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1_000_000);
            let n = n.min(100_000_000);
            let mut inside = 0u64;
            let mut rng = Lcg64::new(0xdeadbeef_12345678);
            for _ in 0..n {
                let x = rng.next_f64() * 2.0 - 1.0;
                let y = rng.next_f64() * 2.0 - 1.0;
                if x * x + y * y <= 1.0 {
                    inside += 1;
                }
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
            out.push_str(&format!(
                "  P(at least 2 share a birthday) = {:.6} ({:.2}%)\n",
                p_match,
                p_match * 100.0
            ));
            out.push_str(&format!(
                "  P(all different birthdays)      = {:.6} ({:.2}%)\n",
                p_no_match,
                p_no_match * 100.0
            ));
            // Find 50% threshold
            let n50 = (1..366u32)
                .find(|&k| {
                    let p = 1.0 - (0..k as u64).fold(1.0f64, |a, i| a * (365 - i) as f64 / 365.0);
                    p >= 0.5
                })
                .unwrap_or(23);
            out.push_str(&format!(
                "  Minimum group for ≥50% chance: {} people\n",
                n50
            ));
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
                let total: i64 = (0..n_dice)
                    .map(|_| (rng.next_u64() % sides as u64) as i64 + 1)
                    .sum::<i64>()
                    + bonus;
                *counts.entry(total).or_insert(0) += 1;
            }
            let mut sorted_keys: Vec<i64> = counts.keys().copied().collect();
            sorted_keys.sort();
            let mean: f64 = sorted_keys
                .iter()
                .map(|&k| k as f64 * counts[&k] as f64)
                .sum::<f64>()
                / rolls as f64;
            let mut out = format!("Dice simulation: {} × {} rolls\n", rolls, spec);
            let _ = writeln!(
                out,
                "  Mean: {:.3}   Range: {}–{}",
                mean,
                sorted_keys.first().unwrap_or(&0),
                sorted_keys.last().unwrap_or(&0)
            );
            out.push_str("  Distribution:\n");
            let max_count = counts.values().copied().max().unwrap_or(1);
            for k in &sorted_keys {
                let c = counts[k];
                let pct = 100.0 * c as f64 / rolls as f64;
                let bar_len = (c as f64 / max_count as f64 * 30.0) as usize;
                let _ = writeln!(out, "    {:4}  {:6.2}%  {}", k, pct, "█".repeat(bar_len));
            }
            out
        }
        "ruin" | "gambler" => {
            let p: f64 = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.5);
            let a: i64 = tokens.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            let b: i64 = tokens.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);
            let n: u64 = tokens.get(4).and_then(|s| s.parse().ok()).unwrap_or(10_000);
            let n = n.min(100_000);
            if a <= 0 || b <= a {
                return "Usage: ruin PROB START GOAL N_SIM (GOAL > START > 0)".into();
            }

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
                    if steps > 100_000 {
                        break;
                    }
                }
                if money >= b {
                    wins += 1;
                }
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
                n,
                p,
                a,
                b,
                win_rate,
                win_rate * 100.0,
                exact,
                exact * 100.0,
                avg_steps
            )
        }
        "walk" | "random_walk" => {
            let n_walks: u64 = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000);
            let steps: u64 = tokens.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);
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
                if pos.abs() > max_deviation {
                    max_deviation = pos.abs();
                }
            }
            let mean = final_positions.iter().sum::<f64>() / n_walks as f64;
            let variance: f64 = final_positions
                .iter()
                .map(|x| (x - mean).powi(2))
                .sum::<f64>()
                / n_walks as f64;
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
     hematite --simulate 'walk 1000 200'        random walk simulation"
        .into()
}

fn parse_dice_spec(spec: &str) -> (i64, i64, i64) {
    // NdM+K or NdM-K
    let lower = spec.to_lowercase();
    let (dice_part, bonus) = if let Some(idx) = lower.rfind('+') {
        let b: i64 = spec[idx + 1..].parse().unwrap_or(0);
        (&spec[..idx], b)
    } else if let Some(idx) = lower[1..].rfind('-').map(|i| i + 1) {
        let b: i64 = spec[idx + 1..].parse().unwrap_or(0);
        (&spec[..idx], -b)
    } else {
        (spec, 0i64)
    };
    if let Some(d_pos) = dice_part.to_lowercase().find('d') {
        let n: i64 = dice_part[..d_pos].parse().unwrap_or(1).max(1);
        let s: i64 = dice_part[d_pos + 1..].parse().unwrap_or(6).max(2);
        (n, s, bonus)
    } else {
        (1, 6, 0)
    }
}

// Minimal 64-bit LCG PRNG — no stdlib rng needed
struct Lcg64 {
    state: u64,
}
impl Lcg64 {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
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
#[allow(dead_code)]
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
    fn new(chars: &'a [char]) -> Self {
        Self { chars, pos: 0 }
    }
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn consume(&mut self) -> Option<char> {
        let c = self.peek();
        self.pos += 1;
        c
    }
    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) {
            self.pos += 1;
        }
    }

    fn parse_iff(&mut self) -> Result<BExpr, String> {
        let mut left = self.parse_implies()?;
        loop {
            self.skip_ws();
            if self.try_keyword("iff") || self.try_str("<->") || self.try_str("<=>") {
                let right = self.parse_implies()?;
                left = BExpr::Iff(Box::new(left), Box::new(right));
            } else {
                break;
            }
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
            if self.try_str("||")
                || self.try_str("|")
                || self.try_keyword("or")
                || self.try_keyword("nor")
            {
                let right = self.parse_xor()?;
                left = BExpr::Or(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_xor(&mut self) -> Result<BExpr, String> {
        let mut left = self.parse_and()?;
        loop {
            self.skip_ws();
            if self.try_keyword("xor") || self.try_keyword("xnor") || self.try_str("^") {
                let right = self.parse_and()?;
                left = BExpr::Xor(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<BExpr, String> {
        let mut left = self.parse_not()?;
        loop {
            self.skip_ws();
            if self.try_str("&&")
                || self.try_str("&")
                || self.try_keyword("and")
                || self.try_keyword("nand")
                || self.try_str("*")
            {
                let right = self.parse_not()?;
                left = BExpr::And(Box::new(left), Box::new(right));
            } else {
                break;
            }
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
            if self.peek() == Some(')') {
                self.consume();
            }
            return Ok(inner);
        }
        // Keyword literals
        if self.try_keyword("true") || self.try_keyword("1") {
            return Ok(BExpr::Const(true));
        }
        if self.try_keyword("false") || self.try_keyword("0") {
            return Ok(BExpr::Const(false));
        }
        // Variable name
        if matches!(self.peek(), Some(c) if c.is_alphabetic() || c == '_') {
            let start = self.pos;
            while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
                self.pos += 1;
            }
            let name: String = self.chars[start..self.pos].iter().collect();
            return Ok(BExpr::Var(name));
        }
        Err(format!(
            "unexpected char '{}'",
            self.peek().map(|c| c.to_string()).unwrap_or("EOF".into())
        ))
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
            && remaining[..chars.len()]
                .iter()
                .map(|c| c.to_lowercase().next().unwrap())
                .collect::<Vec<_>>()
                == chars
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
        BExpr::Var(v) => {
            if !vars.contains(v) {
                vars.push(v.clone());
            }
        }
        BExpr::Not(a) => collect_vars(a, vars),
        BExpr::And(a, b)
        | BExpr::Or(a, b)
        | BExpr::Xor(a, b)
        | BExpr::Implies(a, b)
        | BExpr::Iff(a, b)
        | BExpr::Nand(a, b)
        | BExpr::Nor(a, b)
        | BExpr::Xnor(a, b) => {
            collect_vars(a, vars);
            collect_vars(b, vars);
        }
        BExpr::Const(_) => {}
    }
}

fn eval_bexpr(e: &BExpr, assignment: &[(&str, bool)]) -> bool {
    match e {
        BExpr::Const(b) => *b,
        BExpr::Var(v) => assignment
            .iter()
            .find(|(n, _)| n == v)
            .map(|(_, b)| *b)
            .unwrap_or(false),
        BExpr::Not(a) => !eval_bexpr(a, assignment),
        BExpr::And(a, b) => eval_bexpr(a, assignment) && eval_bexpr(b, assignment),
        BExpr::Or(a, b) => eval_bexpr(a, assignment) || eval_bexpr(b, assignment),
        BExpr::Xor(a, b) => eval_bexpr(a, assignment) ^ eval_bexpr(b, assignment),
        BExpr::Xnor(a, b) => !(eval_bexpr(a, assignment) ^ eval_bexpr(b, assignment)),
        BExpr::Nand(a, b) => !(eval_bexpr(a, assignment) && eval_bexpr(b, assignment)),
        BExpr::Nor(a, b) => !(eval_bexpr(a, assignment) || eval_bexpr(b, assignment)),
        BExpr::Implies(a, b) => !eval_bexpr(a, assignment) || eval_bexpr(b, assignment),
        BExpr::Iff(a, b) => eval_bexpr(a, assignment) == eval_bexpr(b, assignment),
    }
}

fn bexpr_to_str(e: &BExpr) -> String {
    match e {
        BExpr::Const(true) => "true".into(),
        BExpr::Const(false) => "false".into(),
        BExpr::Var(v) => v.clone(),
        BExpr::Not(a) => format!("¬{}", bexpr_atom_str(a)),
        BExpr::And(a, b) => format!("({} ∧ {})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Or(a, b) => format!("({} ∨ {})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Xor(a, b) => format!("({} ⊕ {})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Implies(a, b) => format!("({} → {})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Iff(a, b) => format!("({} ↔ {})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Nand(a, b) => format!("({}↑{})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Nor(a, b) => format!("({}↓{})", bexpr_to_str(a), bexpr_to_str(b)),
        BExpr::Xnor(a, b) => format!("({}⊙{})", bexpr_to_str(a), bexpr_to_str(b)),
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
            let a = simplify_bexpr(*a);
            let b = simplify_bexpr(*b);
            match (&a, &b) {
                (BExpr::Const(false), _) | (_, BExpr::Const(false)) => BExpr::Const(false),
                (BExpr::Const(true), _) => b,
                (_, BExpr::Const(true)) => a,
                _ if a == b => a,
                _ => BExpr::And(Box::new(a), Box::new(b)),
            }
        }
        BExpr::Or(a, b) => {
            let a = simplify_bexpr(*a);
            let b = simplify_bexpr(*b);
            match (&a, &b) {
                (BExpr::Const(true), _) | (_, BExpr::Const(true)) => BExpr::Const(true),
                (BExpr::Const(false), _) => b,
                (_, BExpr::Const(false)) => a,
                _ if a == b => a,
                _ => BExpr::Or(Box::new(a), Box::new(b)),
            }
        }
        BExpr::Xor(a, b) => {
            let a = simplify_bexpr(*a);
            let b = simplify_bexpr(*b);
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
            let a = simplify_bexpr(*a);
            let b = simplify_bexpr(*b);
            match (&a, &b) {
                (BExpr::Const(false), _) => BExpr::Const(true),
                (BExpr::Const(true), _) => b,
                (_, BExpr::Const(true)) => BExpr::Const(true),
                _ if a == b => BExpr::Const(true),
                _ => BExpr::Implies(Box::new(a), Box::new(b)),
            }
        }
        BExpr::Iff(a, b) => {
            let a = simplify_bexpr(*a);
            let b = simplify_bexpr(*b);
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
    let (mode, expr_str, expr2_str) =
        if q_lower.starts_with("table ") || q_lower.starts_with("truth ") {
            (
                "table",
                q.split_once(' ').map(|x| x.1).unwrap_or("").trim(),
                "",
            )
        } else if q_lower.starts_with("sat ") {
            (
                "sat",
                q.split_once(' ').map(|x| x.1).unwrap_or("").trim(),
                "",
            )
        } else if q_lower.starts_with("taut ") {
            (
                "taut",
                q.split_once(' ').map(|x| x.1).unwrap_or("").trim(),
                "",
            )
        } else if q_lower.starts_with("cnf ") {
            (
                "cnf",
                q.split_once(' ').map(|x| x.1).unwrap_or("").trim(),
                "",
            )
        } else if q_lower.starts_with("dnf ") {
            (
                "dnf",
                q.split_once(' ').map(|x| x.1).unwrap_or("").trim(),
                "",
            )
        } else if q_lower.starts_with("simplify ") {
            (
                "simplify",
                q.split_once(' ').map(|x| x.1).unwrap_or("").trim(),
                "",
            )
        } else if q_lower.starts_with("equiv ") {
            let rest = q.split_once(' ').map(|x| x.1).unwrap_or("").trim();
            if let Some(semi) = rest.find(';') {
                ("equiv", rest[..semi].trim(), rest[semi + 1..].trim())
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
            let _ = writeln!(
                out,
                "  Input: {}",
                expr_str.chars().take(60).collect::<String>()
            );
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
    let results: Vec<bool> = (0..rows)
        .map(|mask| {
            let assignment: Vec<(&str, bool)> = vars
                .iter()
                .enumerate()
                .map(|(i, v)| (v.as_str(), (mask >> (n - 1 - i)) & 1 == 1))
                .collect();
            eval_bexpr(&expr, &assignment)
        })
        .collect();

    let sat_count = results.iter().filter(|&&b| b).count();
    let is_taut = sat_count == rows;
    let is_sat = sat_count > 0;
    let is_contra = sat_count == 0;

    let _ = writeln!(out, "  Boolean Logic Analysis");
    let _ = writeln!(out, "  Expression: {}", bexpr_to_str(&expr));
    let _ = writeln!(out, "  Variables : {}", vars.join(", "));
    let _ = writeln!(
        out,
        "  {} satisfying assignments of {} ({}%)",
        sat_count,
        rows,
        sat_count * 100 / rows
    );
    let _ = writeln!(
        out,
        "  Status: {}",
        if is_taut {
            "TAUTOLOGY (always true)"
        } else if is_contra {
            "CONTRADICTION (always false)"
        } else {
            "CONTINGENT (sometimes true)"
        }
    );

    match mode {
        "sat" => {
            if is_sat {
                let first_sat = (0..rows).find(|&mask| results[mask]).unwrap();
                let assignment: Vec<String> = vars
                    .iter()
                    .enumerate()
                    .map(|(i, v)| format!("{}={}", v, (first_sat >> (n - 1 - i)) & 1 == 1))
                    .collect();
                let _ = writeln!(
                    out,
                    "  SAT: YES — satisfying assignment: {}",
                    assignment.join(", ")
                );
            } else {
                let _ = writeln!(out, "  SAT: NO — contradiction");
            }
        }
        "taut" => {
            let _ = writeln!(out, "  TAUTOLOGY: {}", if is_taut { "YES" } else { "NO" });
            if !is_taut {
                let first_false = (0..rows).find(|&mask| !results[mask]).unwrap();
                let assignment: Vec<String> = vars
                    .iter()
                    .enumerate()
                    .map(|(i, v)| format!("{}={}", v, (first_false >> (n - 1 - i)) & 1 == 1))
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
                    let clause: Vec<String> = vars
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            if (mask >> (n - 1 - i)) & 1 == 0 {
                                v.clone()
                            } else {
                                format!("¬{}", v)
                            }
                        })
                        .collect();
                    let _ = writeln!(out, "  ({})", clause.join(" ∨ "));
                }
                if false_rows.len() > 8 {
                    let _ = writeln!(out, "  ... ({} more clauses)", false_rows.len() - 8);
                }
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
                    let term: Vec<String> = vars
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            if (mask >> (n - 1 - i)) & 1 == 1 {
                                v.clone()
                            } else {
                                format!("¬{}", v)
                            }
                        })
                        .collect();
                    let _ = writeln!(out, "  ({})", term.join(" ∧ "));
                }
                if true_rows.len() > 8 {
                    let _ = writeln!(out, "  ... ({} more terms)", true_rows.len() - 8);
                }
            }
        }
        "simplify" => {
            let simp = simplify_bexpr(expr.clone());
            let _ = writeln!(out, "  Simplified: {}", bexpr_to_str(&simp));
        }
        "equiv" => {
            let expr2 = match parse_bexpr(expr2_str) {
                Ok(e) => e,
                Err(e) => {
                    let _ = writeln!(out, "  Parse error (expr2): {}", e);
                    let _ = writeln!(out, "{}", "=".repeat(w));
                    return out;
                }
            };
            let mut vars2 = vars.clone();
            collect_vars(&expr2, &mut vars2);
            vars2.sort();
            vars2.dedup();
            let n2 = vars2.len();
            let rows2 = 1usize << n2;
            let equiv = (0..rows2).all(|mask| {
                let assignment: Vec<(&str, bool)> = vars2
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (v.as_str(), (mask >> (n2 - 1 - i)) & 1 == 1))
                    .collect();
                eval_bexpr(&expr, &assignment) == eval_bexpr(&expr2, &assignment)
            });
            let _ = writeln!(out, "  Expr1: {}", bexpr_to_str(&expr));
            let _ = writeln!(out, "  Expr2: {}", bexpr_to_str(&expr2));
            let _ = writeln!(
                out,
                "  Logically equivalent: {}",
                if equiv { "YES" } else { "NO" }
            );
        }
        _ => {
            // "info" or "table" — show full truth table
            let max_table_rows = if n <= 4 { rows } else { rows.min(32) };
            let _ = writeln!(out, "\n  Truth Table:");
            // Header
            let var_header: String = vars
                .iter()
                .map(|v| format!("  {:>3}", v))
                .collect::<Vec<_>>()
                .join("");
            let _ = writeln!(out, "{}  │  Result", var_header);
            let _ = writeln!(out, "  {}", "-".repeat(vars.len() * 5 + 10));
            for mask in 0..max_table_rows {
                let row_vals: String = (0..n)
                    .map(|i| {
                        format!(
                            "  {:>3}",
                            if (mask >> (n - 1 - i)) & 1 == 1 {
                                "T"
                            } else {
                                "F"
                            }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("");
                let _ = writeln!(
                    out,
                    "{}  │  {}",
                    row_vals,
                    if results[mask] { "T" } else { "F" }
                );
            }
            if max_table_rows < rows {
                let _ = writeln!(out, "  ... ({} rows omitted — use --logic 'table EXPR' for full table with ≤4 vars)", rows - max_table_rows);
            }

            // SAT summary
            if is_sat {
                let first_sat = (0..rows).find(|&m| results[m]).unwrap();
                let sat_ex: Vec<String> = vars
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        format!(
                            "{}={}",
                            v,
                            if (first_sat >> (n - 1 - i)) & 1 == 1 {
                                "T"
                            } else {
                                "F"
                            }
                        )
                    })
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

fn mat_rows(m: &Matrix) -> usize {
    m.len()
}
fn mat_cols(m: &Matrix) -> usize {
    m.first().map(|r| r.len()).unwrap_or(0)
}

fn parse_matrix(s: &str) -> Result<Matrix, String> {
    let s = s.trim();
    // Try [[...],[...]] JSON-like
    if s.starts_with('[') {
        return parse_matrix_json(s);
    }
    // Semicolon or newline rows
    let row_strs: Vec<&str> = s
        .split([';', '\n'])
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .collect();
    if row_strs.is_empty() {
        return Err("empty matrix".into());
    }
    let mut mat: Matrix = Vec::new();
    for row_str in &row_strs {
        let row: Vec<f64> = row_str
            .split([',', ' ', '\t'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|tok| {
                tok.parse::<f64>()
                    .map_err(|_| format!("bad number: {}", tok))
            })
            .collect::<Result<Vec<_>, _>>()?;
        mat.push(row);
    }
    let ncols = mat[0].len();
    for (i, row) in mat.iter().enumerate() {
        if row.len() != ncols {
            return Err(format!(
                "row {} has {} columns, expected {}",
                i,
                row.len(),
                ncols
            ));
        }
    }
    Ok(mat)
}

fn parse_matrix_json(s: &str) -> Result<Matrix, String> {
    // Simple recursive bracket parser — no serde needed
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0;
    fn skip(chars: &[char], pos: &mut usize) {
        while *pos < chars.len() && chars[*pos].is_whitespace() {
            *pos += 1;
        }
    }
    fn parse_num(chars: &[char], pos: &mut usize) -> Result<f64, String> {
        skip(chars, pos);
        let start = *pos;
        while *pos < chars.len()
            && (chars[*pos].is_ascii_digit() || matches!(chars[*pos], '.' | '-' | '+' | 'e' | 'E'))
        {
            *pos += 1;
        }
        let s: String = chars[start..*pos].iter().collect();
        s.trim()
            .parse::<f64>()
            .map_err(|_| format!("bad number: '{}'", s))
    }
    fn parse_row(chars: &[char], pos: &mut usize) -> Result<Vec<f64>, String> {
        skip(chars, pos);
        if chars.get(*pos) != Some(&'[') {
            return Err("expected '[' for row".into());
        }
        *pos += 1;
        let mut row = Vec::new();
        loop {
            skip(chars, pos);
            if chars.get(*pos) == Some(&']') {
                *pos += 1;
                break;
            }
            if !row.is_empty() {
                if chars.get(*pos) == Some(&',') {
                    *pos += 1;
                } else {
                    return Err("expected ','".into());
                }
            }
            row.push(parse_num(chars, pos)?);
        }
        Ok(row)
    }
    skip(&chars, &mut pos);
    if chars.get(pos) != Some(&'[') {
        return Err("expected outer '['".into());
    }
    pos += 1;
    let mut mat: Matrix = Vec::new();
    loop {
        skip(&chars, &mut pos);
        if chars.get(pos) == Some(&']') {
            break;
        }
        if !mat.is_empty() {
            if chars.get(pos) == Some(&',') {
                pos += 1;
            } else {
                return Err("expected ','".into());
            }
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
    if mat.is_empty() {
        return Err("empty matrix".into());
    }
    let ncols = mat[0].len();
    for (i, row) in mat.iter().enumerate() {
        if row.len() != ncols {
            return Err(format!(
                "row {} has {} cols, expected {}",
                i,
                row.len(),
                ncols
            ));
        }
    }
    Ok(mat)
}

fn mat_clone(m: &Matrix) -> Matrix {
    m.clone()
}

fn mat_identity(n: usize) -> Matrix {
    (0..n)
        .map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect()
}

fn mat_fmt(m: &Matrix) -> String {
    let rows = mat_rows(m);
    let cols = mat_cols(m);
    let cells: Vec<String> = m
        .iter()
        .flat_map(|row| {
            row.iter().map(|v| {
                if v.abs() < 1e-12 {
                    "0".to_string()
                } else if v.fract() == 0.0 && v.abs() < 1e9 {
                    format!("{}", *v as i64)
                } else {
                    format!("{:.6}", v)
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_string()
                }
            })
        })
        .collect();
    // Align columns
    let mut col_widths: Vec<usize> = vec![0; cols];
    for r in 0..rows {
        for c in 0..cols {
            col_widths[c] = col_widths[c].max(cells[r * cols + c].len());
        }
    }
    let mut out = String::new();
    for r in 0..rows {
        out.push_str("  [ ");
        for c in 0..cols {
            let s = &cells[r * cols + c];
            out.push_str(&format!("{:>w$}", s, w = col_widths[c]));
            if c < cols - 1 {
                out.push_str("  ");
            }
        }
        out.push_str(" ]\n");
    }
    out
}

// Returns (L, U, P, sign) where P*A = L*U and sign is the permutation sign
fn lu_decompose(a: &Matrix) -> Result<(Matrix, Matrix, Vec<usize>, i32), String> {
    let n = mat_rows(a);
    if mat_cols(a) != n {
        return Err("LU requires square matrix".into());
    }
    let mut u = mat_clone(a);
    let mut l = mat_identity(n);
    let mut perm: Vec<usize> = (0..n).collect();
    let mut sign = 1i32;

    for col in 0..n {
        // Partial pivoting
        let mut max_row = col;
        let mut max_val = u[col][col].abs();
        for row in (col + 1)..n {
            if u[row][col].abs() > max_val {
                max_val = u[row][col].abs();
                max_row = row;
            }
        }
        if max_val < 1e-14 {
            return Err("matrix is singular (or near-singular)".into());
        }
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
        for row in (col + 1)..n {
            let factor = u[row][col] / u[col][col];
            l[row][col] = factor;
            for k in col..n {
                u[row][k] -= factor * u[col][k];
            }
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
        x[i] = (y[i] - (i + 1..n).map(|j| u[i][j] * x[j]).sum::<f64>()) / u[i][i];
    }
    x
}

fn mat_inv(a: &Matrix) -> Result<Matrix, String> {
    let n = mat_rows(a);
    let (l, u, perm, _) = lu_decompose(a)?;
    let mut inv = mat_identity(n);
    for col in 0..n {
        let b: Vec<f64> = (0..n).map(|i| if i == col { 1.0 } else { 0.0 }).collect();
        let x = mat_solve_lu(&l, &u, &perm, &b);
        for row in 0..n {
            inv[row][col] = x[row];
        }
    }
    Ok(inv)
}

fn mat_mul(a: &Matrix, b: &Matrix) -> Result<Matrix, String> {
    let (ar, ac) = (mat_rows(a), mat_cols(a));
    let (br, bc) = (mat_rows(b), mat_cols(b));
    if ac != br {
        return Err(format!(
            "incompatible dimensions {}×{} × {}×{}",
            ar, ac, br, bc
        ));
    }
    let mut c = vec![vec![0.0f64; bc]; ar];
    for i in 0..ar {
        for j in 0..bc {
            for k in 0..ac {
                c[i][j] += a[i][k] * b[k][j];
            }
        }
    }
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
            for j in col..cols {
                m[row_cursor][j] /= pivot_val;
            }
            for r in 0..rows {
                if r != row_cursor && m[r][col].abs() > 1e-10 {
                    let factor = m[r][col];
                    for j in col..cols {
                        m[r][j] -= factor * m[row_cursor][j];
                    }
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
    if n == 0 {
        return None;
    }
    let mut v: Vec<f64> = (0..n).map(|i| if i == 0 { 1.0 } else { 0.1 }).collect();
    let mut lam = 0.0f64;
    for _ in 0..max_iter {
        // Av
        let av: Vec<f64> = (0..n)
            .map(|i| (0..n).map(|j| a[i][j] * v[j]).sum::<f64>())
            .collect();
        let norm = av.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm < 1e-14 {
            break;
        }
        lam = av.iter().zip(&v).map(|(a, b)| a * b).sum::<f64>();
        v = av.iter().map(|x| x / norm).collect();
    }
    Some((lam, v))
}

// QR decomposition via modified Gram-Schmidt
fn mat_qr(a: &Matrix) -> (Matrix, Matrix) {
    let m = mat_rows(a);
    let n = mat_cols(a);
    let cols_a: Vec<Vec<f64>> = (0..n).map(|j| (0..m).map(|i| a[i][j]).collect()).collect();
    let mut q_cols: Vec<Vec<f64>> = Vec::new();
    let mut r: Matrix = vec![vec![0.0; n]; n.min(m)];
    for j in 0..n {
        let mut v: Vec<f64> = cols_a[j].clone();
        for (k, qk) in q_cols.iter().enumerate() {
            let proj: f64 = v.iter().zip(qk).map(|(a, b)| a * b).sum();
            r[k][j] = proj;
            for i in 0..m {
                v[i] -= proj * qk[i];
            }
        }
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-12 {
            let qj: Vec<f64> = v.iter().map(|x| x / norm).collect();
            r[q_cols.len()][j] = norm;
            q_cols.push(qj);
        }
    }
    let q: Matrix = (0..m)
        .map(|i| {
            q_cols
                .iter()
                .map(|col| *col.get(i).unwrap_or(&0.0))
                .collect()
        })
        .collect();
    (q, r)
}

// Singular values via eigenvalues of A^T A (Jacobi-style for small matrices)
fn mat_svd_values(a: &Matrix) -> (Vec<f64>, Matrix) {
    let n = mat_cols(a);
    // Build A^T A
    let at = mat_transpose(a);
    let ata = mat_mul(&at, a).unwrap_or_else(|_| vec![vec![0.0; n]; n]);
    // Power iteration for eigenvalues/vectors of A^T A
    let mut a_copy = ata.clone();
    let mut sigma_vals: Vec<f64> = Vec::new();
    let mut v_vecs: Vec<Vec<f64>> = Vec::new();
    for _ in 0..n {
        match mat_eigen_power(&a_copy, 1000) {
            Some((lam, v)) => {
                let sv = lam.abs().sqrt();
                sigma_vals.push(sv);
                v_vecs.push(v.clone());
                // Deflate A^T A
                for i in 0..n {
                    for j in 0..n {
                        a_copy[i][j] -= lam * v[i] * v[j];
                    }
                }
            }
            None => break,
        }
    }
    // Sort descending
    let mut pairs: Vec<(f64, Vec<f64>)> = sigma_vals.into_iter().zip(v_vecs).collect();
    pairs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let svs: Vec<f64> = pairs.iter().map(|p| p.0).collect();
    let v_mat: Matrix = (0..n)
        .map(|i| pairs.iter().map(|p| *p.1.get(i).unwrap_or(&0.0)).collect())
        .collect();
    (svs, v_mat)
}

// Cholesky decomposition (lower-triangular L s.t. A = L Lᵀ)
fn mat_cholesky(a: &Matrix) -> Result<Matrix, String> {
    let n = mat_rows(a);
    let mut l: Matrix = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let sum: f64 = (0..j).map(|k| l[i][k] * l[j][k]).sum();
            if i == j {
                let d = a[i][i] - sum;
                if d < -1e-10 {
                    return Err(format!("not positive definite (d[{}] = {:.4e})", i, d));
                }
                l[i][j] = d.max(0.0).sqrt();
            } else {
                if l[j][j].abs() < 1e-14 {
                    return Err("zero pivot".into());
                }
                l[i][j] = (a[i][j] - sum) / l[j][j];
            }
        }
    }
    Ok(l)
}

// Moore-Penrose pseudoinverse via normal equations (A⁺ = (AᵀA)⁻¹Aᵀ for full column rank)
fn mat_pinv(a: &Matrix) -> Result<Matrix, String> {
    let at = mat_transpose(a);
    let ata = mat_mul(&at, a)?;
    let ata_inv = mat_inv(&ata)?;
    mat_mul(&ata_inv, &at)
}

pub fn matrix_calc(query: &str) -> String {
    let q = query.trim();

    // Detect mode
    let (mode, rest) = {
        let words: Vec<&str> = q.splitn(2, char::is_whitespace).collect();
        let m = words[0].to_lowercase();
        let rest = words.get(1).copied().unwrap_or("").trim();
        match m.as_str() {
            "det" | "determinant" => ("det", rest.to_string()),
            "inv" | "inverse" => ("inv", rest.to_string()),
            "solve" => ("solve", rest.to_string()),
            "mul" | "multiply" => ("mul", rest.to_string()),
            "transpose" | "trans" => ("transpose", rest.to_string()),
            "eigen" | "eigenvalues" | "eig" => ("eigen", rest.to_string()),
            "rank" => ("rank", rest.to_string()),
            "lu" => ("lu", rest.to_string()),
            "qr" => ("qr", rest.to_string()),
            "svd" => ("svd", rest.to_string()),
            "chol" | "cholesky" => ("chol", rest.to_string()),
            "pinv" | "pseudoinverse" | "pseudo" => ("pinv", rest.to_string()),
            _ => ("info", q.to_string()),
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
        let a = match parse_matrix(a_str) {
            Ok(m) => m,
            Err(e) => {
                let _ = writeln!(out, "  Parse error (A): {}", e);
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
        };
        let b_mat = match parse_matrix(b_str) {
            Ok(m) => m,
            Err(e) => {
                let _ = writeln!(out, "  Parse error (b): {}", e);
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
        };
        let n = mat_rows(&a);
        let b_vec: Vec<f64> = if mat_cols(&b_mat) == 1 {
            b_mat.iter().map(|r| r[0]).collect()
        } else if mat_rows(&b_mat) == 1 {
            b_mat[0].clone()
        } else {
            let _ = writeln!(out, "  Error: b must be a column or row vector");
            let _ = writeln!(out, "{}", "=".repeat(w));
            return out;
        };
        if b_vec.len() != n {
            let _ = writeln!(
                out,
                "  Error: A is {}×{} but b has {} elements",
                n,
                mat_cols(&a),
                b_vec.len()
            );
            let _ = writeln!(out, "{}", "=".repeat(w));
            return out;
        }
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
                let residual: f64 = (0..n)
                    .map(|i| {
                        let ax_i: f64 = (0..n).map(|j| a[i][j] * x[j]).sum();
                        (ax_i - b_vec[i]).powi(2)
                    })
                    .sum::<f64>()
                    .sqrt();
                let _ = writeln!(out, "  Residual |Ax - b| = {:.2e}", residual);
            }
            Err(e) => {
                let _ = writeln!(out, "  Error: {}", e);
            }
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
        let a = match parse_matrix(&parts[0]) {
            Ok(m) => m,
            Err(e) => {
                let _ = writeln!(out, "  Parse error (A): {}", e);
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
        };
        let b = match parse_matrix(&parts[1]) {
            Ok(m) => m,
            Err(e) => {
                let _ = writeln!(out, "  Parse error (B): {}", e);
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
        };
        let _ = writeln!(out, "  Matrix Multiply  A × B");
        let _ = writeln!(out, "  A ({}×{}):", mat_rows(&a), mat_cols(&a));
        out.push_str(&mat_fmt(&a));
        let _ = writeln!(out, "  B ({}×{}):", mat_rows(&b), mat_cols(&b));
        out.push_str(&mat_fmt(&b));
        match mat_mul(&a, &b) {
            Ok(c) => {
                let _ = writeln!(out, "  A × B ({}×{}):", mat_rows(&c), mat_cols(&c));
                out.push_str(&mat_fmt(&c));
            }
            Err(e) => {
                let _ = writeln!(out, "  Error: {}", e);
            }
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
            let _ = writeln!(
                out,
                "  Input: {}",
                mat_str.chars().take(80).collect::<String>()
            );
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
            if rows != cols {
                let _ = writeln!(out, "  Error: det requires square matrix");
            } else {
                match mat_det(&a) {
                    Ok(d) => {
                        let _ = writeln!(out, "  det(A) = {:.8}", d);
                    }
                    Err(e) => {
                        let _ = writeln!(out, "  Error: {}", e);
                    }
                }
            }
        }
        "inv" => {
            if rows != cols {
                let _ = writeln!(out, "  Error: inv requires square matrix");
            } else {
                match mat_inv(&a) {
                    Ok(inv) => {
                        let _ = writeln!(out, "  A⁻¹:");
                        out.push_str(&mat_fmt(&inv));
                    }
                    Err(e) => {
                        let _ = writeln!(out, "  Error: {}", e);
                    }
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
                let _ = writeln!(
                    out,
                    "  {} ({}×{} square, rank {})",
                    if r == rows {
                        "Full rank"
                    } else {
                        "Rank-deficient"
                    },
                    rows,
                    cols,
                    r
                );
            }
        }
        "lu" => {
            if rows != cols {
                let _ = writeln!(out, "  Error: LU requires square matrix");
            } else {
                match lu_decompose(&a) {
                    Ok((l, u, perm, _)) => {
                        let _ = writeln!(out, "  L (lower triangular):");
                        out.push_str(&mat_fmt(&l));
                        let _ = writeln!(out, "  U (upper triangular):");
                        out.push_str(&mat_fmt(&u));
                        let perm_str = perm
                            .iter()
                            .map(|&p| p.to_string())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let _ = writeln!(out, "  Pivot permutation: [{}]", perm_str);
                    }
                    Err(e) => {
                        let _ = writeln!(out, "  Error: {}", e);
                    }
                }
            }
        }
        "eigen" => {
            if rows != cols {
                let _ = writeln!(out, "  Error: eigenvalues require square matrix");
            } else if rows > 8 {
                let _ = writeln!(
                    out,
                    "  Error: power iteration limited to 8×8 (matrix is {}×{})",
                    rows, cols
                );
            } else {
                let _ = writeln!(out, "  Eigenvalues (power iteration + deflation):");
                let mut a_copy = mat_clone(&a);
                for k in 0..rows {
                    match mat_eigen_power(&a_copy, 500) {
                        Some((lam, v)) => {
                            let v_str = v
                                .iter()
                                .map(|x| format!("{:.4}", x))
                                .collect::<Vec<_>>()
                                .join(", ");
                            let _ = writeln!(
                                out,
                                "  λ{} = {:.6}  eigenvector ≈ [{}]",
                                k + 1,
                                lam,
                                v_str
                            );
                            // Deflate
                            for i in 0..rows {
                                for j in 0..rows {
                                    a_copy[i][j] -= lam * v[i] * v[j];
                                }
                            }
                        }
                        None => break,
                    }
                }
                let trace: f64 = (0..rows).map(|i| a[i][i]).sum();
                let _ = writeln!(out, "  Trace = {:.6}", trace);
                if let Ok(d) = mat_det(&a) {
                    let _ = writeln!(out, "  Det   = {:.6}", d);
                }
            }
        }
        "qr" => {
            // Gram-Schmidt QR decomposition
            let (q_mat, r_mat) = mat_qr(&a);
            let _ = writeln!(out, "  QR Decomposition  (A = Q · R)");
            let _ = writeln!(out, "  Q (orthonormal columns):");
            out.push_str(&mat_fmt(&q_mat));
            let _ = writeln!(out, "  R (upper-triangular):");
            out.push_str(&mat_fmt(&r_mat));
        }
        "svd" => {
            // SVD via Jacobi iterations (symmetric A^T A → eigendecomposition)
            // Returns singular values and V matrix; U approximated for small matrices
            if rows > 8 || cols > 8 {
                let _ = writeln!(
                    out,
                    "  Error: SVD limited to 8×8 matrices ({}×{})",
                    rows, cols
                );
            } else {
                let (s_vals, v_mat) = mat_svd_values(&a);
                let _ = writeln!(out, "  SVD Singular Values:");
                for (i, sv) in s_vals.iter().enumerate() {
                    let bar_len = if s_vals[0].abs() > 1e-12 {
                        (sv / s_vals[0] * 20.0) as usize
                    } else {
                        0
                    };
                    let _ = writeln!(out, "  σ{} = {:.6}  {}", i + 1, sv, "#".repeat(bar_len));
                }
                let rank: usize = s_vals.iter().filter(|&&v| v.abs() > 1e-9).count();
                let cond = if s_vals.last().map(|v| v.abs()).unwrap_or(0.0) > 1e-12 {
                    format!(
                        "{:.4}",
                        s_vals[0] / s_vals.iter().cloned().fold(f64::INFINITY, f64::min)
                    )
                } else {
                    "∞ (singular)".to_string()
                };
                let _ = writeln!(out, "  Rank: {}  |  Condition number: {}", rank, cond);
                let _ = writeln!(out, "  V (right singular vectors):");
                out.push_str(&mat_fmt(&v_mat));
            }
        }
        "chol" => {
            if rows != cols {
                let _ = writeln!(out, "  Error: Cholesky requires square matrix");
            } else {
                match mat_cholesky(&a) {
                    Ok(l) => {
                        let _ = writeln!(out, "  Cholesky Decomposition  (A = L · Lᵀ)");
                        let _ = writeln!(out, "  L (lower-triangular):");
                        out.push_str(&mat_fmt(&l));
                    }
                    Err(e) => {
                        let _ = writeln!(
                            out,
                            "  Error: {}  (matrix must be symmetric positive-definite)",
                            e
                        );
                    }
                }
            }
        }
        "pinv" => {
            // Moore-Penrose pseudoinverse via SVD-based approach (normal equations for full-rank)
            match mat_pinv(&a) {
                Ok(p) => {
                    let _ = writeln!(out, "  Moore-Penrose Pseudoinverse (A⁺):");
                    out.push_str(&mat_fmt(&p));
                    // Verify: A A⁺ A ≈ A
                    if let Ok(aa_p) = mat_mul(&a, &p) {
                        if let Ok(aapa) = mat_mul(&aa_p, &a) {
                            let mut err = 0.0f64;
                            for i in 0..aapa.len() {
                                for j in 0..aapa[i].len() {
                                    let d = (aapa[i][j] - a[i][j]).abs();
                                    if d > err {
                                        err = d;
                                    }
                                }
                            }
                            let _ = writeln!(out, "  Verify ||A*A+*A - A||_inf = {:.2e}", err);
                        }
                    }
                }
                Err(e) => {
                    let _ = writeln!(out, "  Error: {}", e);
                }
            }
        }
        _ => {
            // "info" — show all applicable results
            let _ = writeln!(out, "  Rank: {}", mat_rank(&a));
            if rows == cols {
                if let Ok(d) = mat_det(&a) {
                    let _ = writeln!(out, "  Det:  {:.6}", d);
                }
                match mat_inv(&a) {
                    Ok(inv) => {
                        let _ = writeln!(out, "  Inverse:");
                        out.push_str(&mat_fmt(&inv));
                    }
                    Err(_) => {
                        let _ = writeln!(out, "  Inverse: N/A (singular)");
                    }
                }
                let trace: f64 = (0..rows).map(|i| a[i][i]).sum();
                let _ = writeln!(out, "  Trace: {:.6}", trace);
                let frobenius: f64 = a
                    .iter()
                    .flat_map(|r| r.iter())
                    .map(|v| v * v)
                    .sum::<f64>()
                    .sqrt();
                let _ = writeln!(out, "  Frobenius norm: {:.6}", frobenius);
            }
            let t = mat_transpose(&a);
            let _ = writeln!(out, "  Transpose:");
            out.push_str(&mat_fmt(&t));
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
            if c == '[' {
                depth += 1;
            } else if c == ']' {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
        }
        if end == 0 || end >= s.len() {
            return vec![s.to_string()];
        }
        let a_str = s[..end].trim().to_string();
        let b_str = s[end..].trim().trim_start_matches([',', ' ']).to_string();
        if b_str.is_empty() {
            return vec![a_str];
        }
        return vec![a_str, b_str];
    }
    // For semicolon format: split on " / " or "| " or just try natural language split
    if let Some(pos) = s.find(" / ") {
        return vec![s[..pos].trim().to_string(), s[pos + 3..].trim().to_string()];
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
    if tokens.is_empty() {
        return finance_usage();
    }

    let mut out = String::new();
    let w = 64usize;
    let _ = writeln!(out, "{}", "=".repeat(w));

    match tokens[0].to_lowercase().as_str() {
        "npv" => {
            if tokens.len() < 3 {
                let _ = writeln!(out, "  Usage: npv RATE CF0 CF1 CF2 ...");
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
            let rate: f64 = match tokens[1].trim_end_matches('%').parse() {
                Ok(v) => {
                    if tokens[1].contains('%') {
                        v / 100.0
                    } else {
                        v
                    }
                }
                Err(_) => {
                    let _ = writeln!(out, "  Error: bad rate '{}'", tokens[1]);
                    let _ = writeln!(out, "{}", "=".repeat(w));
                    return out;
                }
            };
            let cfs: Vec<f64> = tokens[2..]
                .iter()
                .filter_map(|s| s.replace(',', "").parse::<f64>().ok())
                .collect();
            if cfs.is_empty() {
                let _ = writeln!(out, "  Error: no cash flows found");
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
            let npv: f64 = cfs
                .iter()
                .enumerate()
                .map(|(t, &cf)| cf / (1.0 + rate).powi(t as i32))
                .sum();
            let _ = writeln!(out, "  NPV Analysis");
            let _ = writeln!(out, "  Discount rate : {:.4}%", rate * 100.0);
            let _ = writeln!(
                out,
                "  Cash flows    : {}",
                cfs.iter()
                    .map(|cf| format!("{:.2}", cf))
                    .collect::<Vec<_>>()
                    .join("  ")
            );
            let _ = writeln!(out, "  NPV           : {:.4}", npv);
            let _ = writeln!(
                out,
                "  Decision      : {}",
                if npv > 0.0 {
                    "Accept (NPV > 0)"
                } else if npv < 0.0 {
                    "Reject (NPV < 0)"
                } else {
                    "Indifferent"
                }
            );
        }
        "irr" => {
            if tokens.len() < 3 {
                let _ = writeln!(out, "  Usage: irr CF0 CF1 CF2 ...");
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
            let cfs: Vec<f64> = tokens[1..]
                .iter()
                .filter_map(|s| s.replace(',', "").parse::<f64>().ok())
                .collect();
            if cfs.is_empty() {
                let _ = writeln!(out, "  Error: no cash flows");
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
            fn npv_at(rate: f64, cfs: &[f64]) -> f64 {
                cfs.iter()
                    .enumerate()
                    .map(|(t, &cf)| cf / (1.0 + rate).powi(t as i32))
                    .sum()
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
                    if npv_at(mid, &cfs) * npv_at(lo, &cfs) < 0.0 {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                    if (hi - lo).abs() < 1e-10 {
                        break;
                    }
                }
                let irr = (lo + hi) / 2.0;
                let _ = writeln!(
                    out,
                    "  Cash flows : {}",
                    cfs.iter()
                        .map(|cf| format!("{:.2}", cf))
                        .collect::<Vec<_>>()
                        .join("  ")
                );
                let _ = writeln!(out, "  IRR        : {:.6}%", irr * 100.0);
                let npv_check = npv_at(irr, &cfs);
                let _ = writeln!(out, "  NPV @ IRR  : {:.8} (should be ~0)", npv_check);
            }
        }
        "loan" => {
            if tokens.len() < 4 {
                let _ = writeln!(out, "  Usage: loan PRINCIPAL RATE_PCT YEARS");
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
            let principal: f64 = tokens[1].replace(',', "").parse().unwrap_or(0.0);
            let annual_rate: f64 = tokens[2]
                .trim_end_matches('%')
                .parse::<f64>()
                .unwrap_or(0.0)
                / 100.0;
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
            let _ = writeln!(
                out,
                "  Interest ratio : {:>12.2}%",
                total_interest / total_paid * 100.0
            );
            // Show amortization table for first/last few months if reasonable
            if n <= 60 || n <= 360 {
                let show_rows = 6usize.min(n as usize);
                let _ = writeln!(
                    out,
                    "\n  {:<6}  {:>12}  {:>12}  {:>12}  {:>12}",
                    "Month", "Payment", "Principal", "Interest", "Balance"
                );
                let _ = writeln!(out, "  {}", "-".repeat(58));
                let mut balance = principal;
                for mo in 1..=n {
                    let interest_part = balance * r;
                    let principal_part = payment - interest_part;
                    balance -= principal_part;
                    if balance < 0.0 {
                        balance = 0.0;
                    }
                    if mo as usize <= show_rows || mo as usize > n as usize - show_rows {
                        let _ = writeln!(
                            out,
                            "  {:<6}  {:>12.2}  {:>12.2}  {:>12.2}  {:>12.2}",
                            mo, payment, principal_part, interest_part, balance
                        );
                    } else if mo as usize == show_rows + 1 {
                        let _ = writeln!(out, "  {:^58}", "...");
                    }
                }
            }
        }
        "compound" => {
            if tokens.len() < 4 {
                let _ = writeln!(out, "  Usage: compound PRINCIPAL RATE_PCT YEARS [PERIODS]");
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
            let p: f64 = tokens[1].replace(',', "").parse().unwrap_or(0.0);
            let r: f64 = tokens[2]
                .trim_end_matches('%')
                .parse::<f64>()
                .unwrap_or(0.0)
                / 100.0;
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
            if tokens.len() < 6 {
                let _ = writeln!(
                    out,
                    "  Usage: bond FACE COUPON_PCT YIELD_PCT YEARS [PERIODS]"
                );
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
            let face: f64 = tokens[1].replace(',', "").parse().unwrap_or(1000.0);
            let coupon_rate: f64 = tokens[2]
                .trim_end_matches('%')
                .parse::<f64>()
                .unwrap_or(0.0)
                / 100.0;
            let yield_rate: f64 = tokens[3]
                .trim_end_matches('%')
                .parse::<f64>()
                .unwrap_or(0.0)
                / 100.0;
            let years: f64 = tokens[4].parse().unwrap_or(1.0);
            let m: f64 = tokens.get(5).and_then(|s| s.parse().ok()).unwrap_or(2.0); // semi-annual default
            let n = (years * m).round() as i32;
            let r = yield_rate / m;
            let c = face * coupon_rate / m;
            // Price = PV of coupons + PV of face
            let pv_coupons = if r.abs() < 1e-12 {
                c * n as f64
            } else {
                c * (1.0 - (1.0 + r).powi(-n)) / r
            };
            let pv_face = face / (1.0 + r).powi(n);
            let price = pv_coupons + pv_face;
            let duration_num: f64 = (1..=n)
                .map(|t| t as f64 / m * c / (1.0 + r).powi(t))
                .sum::<f64>()
                + years * pv_face;
            let duration = duration_num / price;
            let _ = writeln!(out, "  Bond Pricing");
            let _ = writeln!(out, "  Face value     : {:>12.2}", face);
            let _ = writeln!(
                out,
                "  Coupon rate    : {:>12.4}%  ({:.2} per period)",
                coupon_rate * 100.0,
                c
            );
            let _ = writeln!(out, "  Yield to mat.  : {:>12.4}%", yield_rate * 100.0);
            let _ = writeln!(out, "  Years to mat.  : {:>12}", years);
            let _ = writeln!(out, "  Periods/year   : {:>12}", m);
            let _ = writeln!(out, "  Total periods  : {:>12}", n);
            let _ = writeln!(out, "  Bond price     : {:>12.4}", price);
            let _ = writeln!(out, "  PV of coupons  : {:>12.4}", pv_coupons);
            let _ = writeln!(out, "  PV of face     : {:>12.4}", pv_face);
            let status = if price > face {
                "Premium"
            } else if price < face {
                "Discount"
            } else {
                "Par"
            };
            let _ = writeln!(
                out,
                "  Bond trades at : {} ({:.2}% of face)",
                status,
                price / face * 100.0
            );
            let _ = writeln!(out, "  Macaulay dur.  : {:>12.4} years", duration);
        }
        "bs" | "black-scholes" | "blackscholes" | "option" => {
            if tokens.len() < 6 {
                let _ = writeln!(
                    out,
                    "  Usage: bs SPOT STRIKE RATE_PCT SIGMA_PCT YEARS [call|put]"
                );
                let _ = writeln!(out, "{}", "=".repeat(w));
                return out;
            }
            let s: f64 = tokens[1].replace(',', "").parse().unwrap_or(0.0);
            let k: f64 = tokens[2].replace(',', "").parse().unwrap_or(0.0);
            let r: f64 = tokens[3]
                .trim_end_matches('%')
                .parse::<f64>()
                .unwrap_or(0.0)
                / 100.0;
            let sigma: f64 = tokens[4]
                .trim_end_matches('%')
                .parse::<f64>()
                .unwrap_or(0.0)
                / 100.0;
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
            let vega = s * bs_npdf(d1) * t.sqrt() / 100.0;
            let theta_call = (-s * bs_npdf(d1) * sigma / (2.0 * t.sqrt())
                - r * k * (-r * t).exp() * nd2)
                / 365.0;
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
    if x < -8.0 {
        return 0.0;
    }
    if x > 8.0 {
        return 1.0;
    }
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
     hematite --finance 'bs 100 105 5% 20% 0.5 put'          Black-Scholes put"
        .into()
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
        let tokens: Vec<&str> = q.splitn(2, ['\n', ';']).collect();
        let first_line = tokens[0].trim();
        let _fl_lower = first_line.to_lowercase();
        // Check if the entire first line looks like a mode+args header (no edge separators)
        let looks_like_mode = !first_line.contains("->")
            && !first_line.contains(" - ")
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
                "bfs" | "dfs" | "shortest" | "path" | "components" | "topo" | "topological"
                | "info" | "degree" => (m, rest_str),
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
    let edge_strs: Vec<&str> = rest
        .split(['\n', ';'])
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
        if line.is_empty() {
            continue;
        }
        // Detect directed
        let (a, b, w, dir) = if let Some(pos) = line.find("->") {
            directed = true;
            let a = line[..pos].trim().trim_matches(':');
            let rest2 = line[pos + 2..].trim();
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
                hp.filter(|&h| h > 0 && line[..h].trim().parse::<f64>().is_err())
            }
        }) {
            let sep_len = if line[pos..].starts_with(" - ") { 3 } else { 1 };
            let a = line[..pos].trim();
            let rest2 = line[pos + sep_len..].trim();
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
                let wt = b_raw[cp + 1..].parse::<f64>().unwrap_or(1.0);
                (&b_raw[..cp], wt)
            } else {
                let wt = parts
                    .get(2)
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(1.0);
                (b_raw, wt)
            };
            (a, b, w, false)
        };

        if a.is_empty() || b.is_empty() {
            continue;
        }
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
    let _ = writeln!(
        out,
        "  Graph Analysis  |  {} nodes  |  {} edges  |  {}",
        n,
        edges.len(),
        if directed { "directed" } else { "undirected" }
    );
    let _ = writeln!(out, "{}", "=".repeat(w));

    match mode.as_str() {
        "bfs" => {
            let start_name = rest.split_whitespace().next().unwrap_or(&nodes[0]);
            let start = nodes.iter().position(|x| x == start_name).unwrap_or(0);
            let order = bfs_order(&adj, start, n);
            let _ = writeln!(out, "  BFS from \"{}\":", nodes[start]);
            let _ = writeln!(
                out,
                "  Visit order: {}",
                order
                    .iter()
                    .map(|&i| nodes[i].as_str())
                    .collect::<Vec<_>>()
                    .join(" → ")
            );
        }
        "dfs" => {
            let start_name = rest.split_whitespace().next().unwrap_or(&nodes[0]);
            let start = nodes.iter().position(|x| x == start_name).unwrap_or(0);
            let order = dfs_order(&adj, start, n);
            let _ = writeln!(out, "  DFS from \"{}\":", nodes[start]);
            let _ = writeln!(
                out,
                "  Visit order: {}",
                order
                    .iter()
                    .map(|&i| nodes[i].as_str())
                    .collect::<Vec<_>>()
                    .join(" → ")
            );
        }
        "shortest" | "path" => {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            let from_name = parts.first().copied().unwrap_or(&nodes[0]);
            let to_name = parts.get(1).copied().unwrap_or(&nodes[n - 1]);
            let from = nodes.iter().position(|x| x == from_name).unwrap_or(0);
            let to = nodes
                .iter()
                .position(|x| x == to_name)
                .unwrap_or(n.saturating_sub(1));
            match dijkstra(&adj, from, to, n) {
                Some((dist, path)) => {
                    let path_str = path
                        .iter()
                        .map(|&i| nodes[i].as_str())
                        .collect::<Vec<_>>()
                        .join(" → ");
                    let _ = writeln!(out, "  Shortest path: {} → {}", nodes[from], nodes[to]);
                    let _ = writeln!(out, "  Distance: {:.4}", dist);
                    let _ = writeln!(out, "  Path: {}", path_str);
                }
                None => {
                    let _ = writeln!(
                        out,
                        "  No path from \"{}\" to \"{}\"",
                        nodes[from], nodes[to]
                    );
                }
            }
            // Also show all-pairs distances from source
            let dists = dijkstra_all(&adj, from, n);
            let _ = writeln!(out, "\n  All distances from \"{}\":", nodes[from]);
            for (i, d) in dists.iter().enumerate() {
                if i == from {
                    continue;
                }
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
                let _ = writeln!(out, "  [{}] {}", ci + 1, names.join(", "));
            }
        }
        "topo" | "topological" => match topo_sort(&adj, n) {
            Ok(order) => {
                let _ = writeln!(out, "  Topological sort:");
                let _ = writeln!(
                    out,
                    "  {}",
                    order
                        .iter()
                        .map(|&i| nodes[i].as_str())
                        .collect::<Vec<_>>()
                        .join(" → ")
                );
            }
            Err(_) => {
                let _ = writeln!(out, "  Cycle detected — topological sort not possible.");
            }
        },
        "centrality" | "betweenness" => {
            // Brandes algorithm for betweenness centrality (unweighted BFS)
            let bc = betweenness_centrality(&adj, n);
            let mut ranked: Vec<(usize, f64)> = bc.iter().copied().enumerate().collect();
            ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let _ = writeln!(
                out,
                "  Betweenness Centrality (fraction of shortest paths through node):"
            );
            let _ = writeln!(out, "  {:<22}  {:>12}  bar", "Node", "Centrality");
            let _ = writeln!(out, "  {}", "-".repeat(w - 2));
            let max_bc = ranked.first().map(|x| x.1).unwrap_or(1.0).max(1e-9);
            for &(i, val) in &ranked {
                let bar_len = (val / max_bc * 30.0) as usize;
                let _ = writeln!(
                    out,
                    "  {:<22}  {:>12.6}  {}",
                    &nodes[i],
                    val,
                    "#".repeat(bar_len)
                );
            }
        }
        "pagerank" | "pr" => {
            let d: f64 = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.85);
            let pr = pagerank(&adj, n, d, 100);
            let mut ranked: Vec<(usize, f64)> = pr.iter().copied().enumerate().collect();
            ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let _ = writeln!(out, "  PageRank  (damping={:.2}, 100 iterations):", d);
            let _ = writeln!(out, "  {:<22}  {:>10}  bar", "Node", "Score");
            let _ = writeln!(out, "  {}", "-".repeat(w - 2));
            let max_pr = ranked.first().map(|x| x.1).unwrap_or(1e-9).max(1e-9);
            for &(i, val) in &ranked {
                let bar_len = (val / max_pr * 30.0) as usize;
                let _ = writeln!(
                    out,
                    "  {:<22}  {:>10.6}  {}",
                    &nodes[i],
                    val,
                    "#".repeat(bar_len)
                );
            }
        }
        "clustering" | "cluster" => {
            let cc = clustering_coefficients(&adj, n, directed);
            let global_cc = if n > 0 {
                cc.iter().sum::<f64>() / n as f64
            } else {
                0.0
            };
            let mut ranked: Vec<(usize, f64)> = cc.iter().copied().enumerate().collect();
            ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let _ = writeln!(
                out,
                "  Clustering Coefficients  (global avg: {:.4}):",
                global_cc
            );
            let _ = writeln!(out, "  {:<22}  {:>12}  bar", "Node", "Coefficient");
            let _ = writeln!(out, "  {}", "-".repeat(w - 2));
            for &(i, val) in &ranked {
                let bar_len = (val * 30.0) as usize;
                let _ = writeln!(
                    out,
                    "  {:<22}  {:>12.6}  {}",
                    &nodes[i],
                    val,
                    "#".repeat(bar_len)
                );
            }
        }
        "diameter" | "stats" | "metrics" => {
            // All-pairs shortest paths via repeated Dijkstra
            let (diameter, avg_path, eccentricities) = graph_diameter(&adj, n);
            let _ = writeln!(out, "  Network Metrics:");
            if diameter == f64::INFINITY {
                let _ = writeln!(out, "  Diameter        : ∞ (disconnected graph)");
                let _ = writeln!(out, "  Avg path length : N/A");
            } else {
                let _ = writeln!(out, "  Diameter        : {:.4}", diameter);
                let _ = writeln!(out, "  Avg path length : {:.4}", avg_path);
            }
            // Density
            let max_edges = if directed {
                n * (n - 1)
            } else {
                n * (n - 1) / 2
            };
            let density = if max_edges > 0 {
                edges.len() as f64 / max_edges as f64
            } else {
                0.0
            };
            let _ = writeln!(
                out,
                "  Density         : {:.4}  ({} / {} possible edges)",
                density,
                edges.len(),
                max_edges
            );
            // Eccentricity table
            let _ = writeln!(
                out,
                "\n  Eccentricities (max distance from node to any other):"
            );
            let _ = writeln!(out, "  {:<22}  {:>12}", "Node", "Eccentricity");
            let _ = writeln!(out, "  {}", "-".repeat(36));
            let mut ecc_sorted: Vec<(usize, f64)> =
                eccentricities.into_iter().enumerate().collect();
            ecc_sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            for (i, ecc) in &ecc_sorted {
                if *ecc == f64::INFINITY {
                    let _ = writeln!(out, "  {:<22}  {:>12}", &nodes[*i], "∞");
                } else {
                    let _ = writeln!(out, "  {:<22}  {:>12.4}", &nodes[*i], ecc);
                }
            }
            // Center nodes (minimum eccentricity)
            let min_ecc = ecc_sorted
                .iter()
                .map(|x| x.1)
                .filter(|x| x.is_finite())
                .fold(f64::INFINITY, f64::min);
            if min_ecc.is_finite() {
                let centers: Vec<&str> = ecc_sorted
                    .iter()
                    .filter(|&&(_, e)| (e - min_ecc).abs() < 1e-9)
                    .map(|&(i, _)| nodes[i].as_str())
                    .collect();
                let _ = writeln!(out, "\n  Center node(s): {}", centers.join(", "));
            }
        }
        _ => {
            // Default: degree table + basic stats
            let mut in_deg = vec![0usize; n];
            let mut out_deg = vec![0usize; n];
            for (ai, nbrs) in adj.iter().enumerate() {
                out_deg[ai] = nbrs.len();
                for &(bi, _) in nbrs {
                    in_deg[bi] += 1;
                }
            }
            let _ = writeln!(
                out,
                "  {:<20}  {:>8}  {:>8}",
                "Node",
                if directed { "Out-deg" } else { "Degree" },
                if directed { "In-deg" } else { "" }
            );
            let _ = writeln!(out, "  {}", "-".repeat(40));
            let mut sorted_nodes: Vec<usize> = (0..n).collect();
            sorted_nodes.sort_by(|&a, &b| out_deg[b].cmp(&out_deg[a]));
            for &i in &sorted_nodes {
                if directed {
                    let _ = writeln!(
                        out,
                        "  {:<20}  {:>8}  {:>8}",
                        &nodes[i], out_deg[i], in_deg[i]
                    );
                } else {
                    let _ = writeln!(out, "  {:<20}  {:>8}", &nodes[i], out_deg[i]);
                }
            }
            // Connectivity
            let comps = connected_components(&adj, n, directed);
            let _ = writeln!(
                out,
                "\n  Components: {}  |  {}",
                comps.len(),
                if comps.len() == 1 {
                    "connected".to_string()
                } else {
                    "disconnected".to_string()
                }
            );
            // Check for cycles via DFS
            let has_cycle = detect_cycle(&adj, n, directed);
            let _ = writeln!(
                out,
                "  Cycles: {}",
                if has_cycle { "yes" } else { "none detected" }
            );
            if directed {
                if let Ok(order) = topo_sort(&adj, n) {
                    let _ = writeln!(
                        out,
                        "  Topo order: {}",
                        order
                            .iter()
                            .map(|&i| nodes[i].as_str())
                            .collect::<Vec<_>>()
                            .join(" → ")
                    );
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
        let w = s[pos + 1..].trim().parse::<f64>().unwrap_or(1.0);
        (name.trim(), w)
    } else {
        let parts: Vec<&str> = s.splitn(2, char::is_whitespace).collect();
        let name = parts[0].trim();
        let w = parts
            .get(1)
            .and_then(|x| x.trim().parse::<f64>().ok())
            .unwrap_or(1.0);
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
        let mut nbrs: Vec<usize> = adj[u].iter().map(|&(v, _)| v).collect();
        nbrs.sort();
        for v in nbrs {
            if !visited[v] {
                visited[v] = true;
                queue.push_back(v);
            }
        }
    }
    order
}

fn dfs_order(adj: &[Vec<(usize, f64)>], start: usize, n: usize) -> Vec<usize> {
    let mut visited = vec![false; n];
    let mut stack = vec![start];
    let mut order = Vec::new();
    while let Some(u) = stack.pop() {
        if visited[u] {
            continue;
        }
        visited[u] = true;
        order.push(u);
        let mut nbrs: Vec<usize> = adj[u].iter().map(|&(v, _)| v).collect();
        nbrs.sort_by(|a, b| b.cmp(a));
        for v in nbrs {
            if !visited[v] {
                stack.push(v);
            }
        }
    }
    order
}

fn dijkstra(
    adj: &[Vec<(usize, f64)>],
    from: usize,
    to: usize,
    n: usize,
) -> Option<(f64, Vec<usize>)> {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;
    #[derive(PartialEq)]
    struct State {
        cost: f64,
        node: usize,
    }
    impl Eq for State {}
    impl PartialOrd for State {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for State {
        fn cmp(&self, other: &Self) -> Ordering {
            other
                .cost
                .partial_cmp(&self.cost)
                .unwrap_or(Ordering::Equal)
        }
    }

    let mut dist = vec![f64::INFINITY; n];
    let mut prev = vec![usize::MAX; n];
    dist[from] = 0.0;
    let mut heap = BinaryHeap::new();
    heap.push(State {
        cost: 0.0,
        node: from,
    });

    while let Some(State { cost, node }) = heap.pop() {
        if node == to {
            break;
        }
        if cost > dist[node] {
            continue;
        }
        for &(v, w) in &adj[node] {
            let next_cost = dist[node] + w;
            if next_cost < dist[v] {
                dist[v] = next_cost;
                prev[v] = node;
                heap.push(State {
                    cost: next_cost,
                    node: v,
                });
            }
        }
    }

    if dist[to] == f64::INFINITY {
        return None;
    }
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
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;
    #[derive(PartialEq)]
    struct State {
        cost: f64,
        node: usize,
    }
    impl Eq for State {}
    impl PartialOrd for State {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for State {
        fn cmp(&self, other: &Self) -> Ordering {
            other
                .cost
                .partial_cmp(&self.cost)
                .unwrap_or(Ordering::Equal)
        }
    }
    let mut dist = vec![f64::INFINITY; n];
    dist[from] = 0.0;
    let mut heap = BinaryHeap::new();
    heap.push(State {
        cost: 0.0,
        node: from,
    });
    while let Some(State { cost, node }) = heap.pop() {
        if cost > dist[node] {
            continue;
        }
        for &(v, w) in &adj[node] {
            let nc = dist[node] + w;
            if nc < dist[v] {
                dist[v] = nc;
                heap.push(State { cost: nc, node: v });
            }
        }
    }
    dist
}

fn connected_components(adj: &[Vec<(usize, f64)>], n: usize, directed: bool) -> Vec<Vec<usize>> {
    // For directed graphs, use weakly connected components (ignore direction)
    let mut visited = vec![false; n];
    let mut comps = Vec::new();
    for start in 0..n {
        if visited[start] {
            continue;
        }
        let mut comp = Vec::new();
        let mut stack = vec![start];
        while let Some(u) = stack.pop() {
            if visited[u] {
                continue;
            }
            visited[u] = true;
            comp.push(u);
            for &(v, _) in &adj[u] {
                if !visited[v] {
                    stack.push(v);
                }
            }
            if directed {
                // Also traverse reverse edges for weak connectivity
                for other in 0..n {
                    if !visited[other] && adj[other].iter().any(|&(t, _)| t == u) {
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
        for &(v, _) in &adj[u] {
            in_deg[v] += 1;
        }
    }
    let mut queue: std::collections::VecDeque<usize> = (0..n).filter(|&i| in_deg[i] == 0).collect();
    let mut order = Vec::new();
    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &(v, _) in &adj[u] {
            in_deg[v] -= 1;
            if in_deg[v] == 0 {
                queue.push_back(v);
            }
        }
    }
    if order.len() == n {
        Ok(order)
    } else {
        Err(())
    }
}

fn detect_cycle(adj: &[Vec<(usize, f64)>], n: usize, directed: bool) -> bool {
    // DFS-based cycle detection
    let mut color = vec![0u8; n]; // 0=white 1=gray 2=black
    fn dfs_cycle(
        u: usize,
        adj: &[Vec<(usize, f64)>],
        color: &mut Vec<u8>,
        directed: bool,
        parent: usize,
    ) -> bool {
        color[u] = 1;
        for &(v, _) in &adj[u] {
            if color[v] == 0 {
                if dfs_cycle(v, adj, color, directed, u) {
                    return true;
                }
            } else if (directed && color[v] == 1) || (!directed && v != parent) {
                return true;
            }
        }
        color[u] = 2;
        false
    }
    for start in 0..n {
        if color[start] == 0 && dfs_cycle(start, adj, &mut color, directed, usize::MAX) {
            return true;
        }
    }
    false
}

// Brandes betweenness centrality (unweighted, normalized)
fn betweenness_centrality(adj: &[Vec<(usize, f64)>], n: usize) -> Vec<f64> {
    let mut bc = vec![0.0f64; n];
    for s in 0..n {
        let mut stack = Vec::new();
        let mut pred: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut sigma = vec![0.0f64; n];
        sigma[s] = 1.0;
        let mut dist = vec![-1i64; n];
        dist[s] = 0;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(s);
        while let Some(v) = queue.pop_front() {
            stack.push(v);
            for &(w, _) in &adj[v] {
                if dist[w] < 0 {
                    queue.push_back(w);
                    dist[w] = dist[v] + 1;
                }
                if dist[w] == dist[v] + 1 {
                    sigma[w] += sigma[v];
                    pred[w].push(v);
                }
            }
        }
        let mut delta = vec![0.0f64; n];
        while let Some(w) = stack.pop() {
            for &v in &pred[w] {
                delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
            }
            if w != s {
                bc[w] += delta[w];
            }
        }
    }
    // Normalize
    let norm = if n > 2 {
        ((n - 1) * (n - 2)) as f64
    } else {
        1.0
    };
    bc.iter_mut().for_each(|x| *x /= norm);
    bc
}

// PageRank (power iteration)
fn pagerank(adj: &[Vec<(usize, f64)>], n: usize, damping: f64, iters: usize) -> Vec<f64> {
    let mut pr = vec![1.0 / n as f64; n];
    let out_deg: Vec<usize> = adj.iter().map(|nbrs| nbrs.len()).collect();
    for _ in 0..iters {
        let mut new_pr = vec![(1.0 - damping) / n as f64; n];
        for v in 0..n {
            if out_deg[v] == 0 {
                // dangling node: distribute evenly
                let share = damping * pr[v] / n as f64;
                new_pr.iter_mut().for_each(|x| *x += share);
            } else {
                let share = damping * pr[v] / out_deg[v] as f64;
                for &(u, _) in &adj[v] {
                    new_pr[u] += share;
                }
            }
        }
        pr = new_pr;
    }
    pr
}

// Local clustering coefficient
fn clustering_coefficients(adj: &[Vec<(usize, f64)>], n: usize, directed: bool) -> Vec<f64> {
    let mut cc = vec![0.0f64; n];
    for u in 0..n {
        let nbrs: Vec<usize> = adj[u].iter().map(|&(v, _)| v).collect();
        let k = nbrs.len();
        if k < 2 {
            continue;
        }
        let mut triangles = 0usize;
        for i in 0..k {
            for j in (i + 1)..k {
                let vi = nbrs[i];
                let vj = nbrs[j];
                if adj[vi].iter().any(|&(x, _)| x == vj) || adj[vj].iter().any(|&(x, _)| x == vi) {
                    triangles += 1;
                }
            }
        }
        let denom = if directed {
            k * (k - 1)
        } else {
            k * (k - 1) / 2
        };
        if denom > 0 {
            cc[u] = triangles as f64 / denom as f64;
        }
    }
    cc
}

// Graph diameter, average path length, eccentricities via all-pairs Dijkstra
fn graph_diameter(adj: &[Vec<(usize, f64)>], n: usize) -> (f64, f64, Vec<f64>) {
    let mut diameter = 0.0f64;
    let mut path_sum = 0.0f64;
    let mut path_cnt = 0u64;
    let mut ecc = vec![0.0f64; n];
    for s in 0..n {
        let dists = dijkstra_all(adj, s, n);
        let finite: Vec<f64> = dists
            .iter()
            .copied()
            .filter(|d| d.is_finite() && *d > 0.0)
            .collect();
        let max_d = finite.iter().cloned().fold(0.0f64, f64::max);
        if finite.len() < n - 1 {
            ecc[s] = f64::INFINITY; // disconnected
        } else {
            ecc[s] = max_d;
        }
        if max_d.is_finite() && max_d > diameter {
            diameter = max_d;
        }
        for d in &finite {
            path_sum += d;
            path_cnt += 1;
        }
    }
    let avg = if path_cnt > 0 {
        path_sum / path_cnt as f64
    } else {
        f64::INFINITY
    };
    let diam = if ecc.iter().any(|x| x.is_infinite()) {
        f64::INFINITY
    } else {
        diameter
    };
    (diam, avg, ecc)
}

fn graph_usage() -> String {
    "Graph theory — edge list input:\n\
     hematite --graph 'A B\\nB C\\nC D'                  info (degree table, components)\n\
     hematite --graph 'bfs A\\nA B\\nB C\\nA C'           BFS from node A\n\
     hematite --graph 'dfs A\\nA B\\nB C\\nA C'           DFS from node A\n\
     hematite --graph 'shortest A D\\nA B 2\\nB D 3\\nA D 10'  Dijkstra shortest path\n\
     hematite --graph 'components\\nA B\\nC D'            connected components\n\
     hematite --graph 'topo\\nA->B\\nA->C\\nB->D'          topological sort\n\
     hematite --graph 'centrality\\nA B\\nB C\\nA C'       betweenness centrality\n\
     hematite --graph 'pagerank\\nA->B\\nB->C\\nC->A'      PageRank scores\n\
     hematite --graph 'clustering\\nA B\\nB C\\nA C'       local clustering coefficients\n\
     hematite --graph 'diameter\\nA B 1\\nB C 2\\nA C 4'   diameter, avg path, eccentricity\n\
     \n\
     Edge formats: 'A B' 'A B 5' 'A->B' 'A->B:5' 'A-B:3'\n\
     Weighted edges: add weight as third token or after colon"
        .into()
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
    fn new(chars: &'a [char]) -> Self {
        Self { chars, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn consume(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) {
            self.pos += 1;
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_add()
    }

    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_mul()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('+') => {
                    self.consume();
                    let r = self.parse_mul()?;
                    left = Expr::Add(Box::new(left), Box::new(r));
                }
                Some('-') => {
                    self.consume();
                    let r = self.parse_mul()?;
                    left = Expr::Sub(Box::new(left), Box::new(r));
                }
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
                Some('*') => {
                    self.consume();
                    let r = self.parse_pow()?;
                    left = Expr::Mul(Box::new(left), Box::new(r));
                }
                Some('/') => {
                    self.consume();
                    let r = self.parse_pow()?;
                    left = Expr::Div(Box::new(left), Box::new(r));
                }
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
        if self.peek() == Some('+') {
            self.consume();
        }
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Result<Expr, String> {
        self.skip_ws();
        match self.peek() {
            Some('(') => {
                self.consume();
                let inner = self.parse_expr()?;
                self.skip_ws();
                if self.peek() == Some(')') {
                    self.consume();
                }
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
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E')
        {
            self.pos += 1;
            // handle e+/e-
            if matches!(self.chars.get(self.pos - 1), Some('e') | Some('E'))
                && matches!(self.peek(), Some('+') | Some('-'))
            {
                self.pos += 1;
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        s.parse::<f64>()
            .map(Expr::Num)
            .map_err(|_| format!("bad number: {}", s))
    }

    fn parse_name(&mut self) -> Result<Expr, String> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            self.pos += 1;
        }
        let name: String = self.chars[start..self.pos].iter().collect();
        self.skip_ws();
        // Check if it's a function call
        if self.peek() == Some('(') {
            self.consume();
            let arg = self.parse_expr()?;
            self.skip_ws();
            if self.peek() == Some(')') {
                self.consume();
            }
            let e = Box::new(arg);
            return match name.to_lowercase().as_str() {
                "sin" => Ok(Expr::Sin(e)),
                "cos" => Ok(Expr::Cos(e)),
                "tan" => Ok(Expr::Tan(e)),
                "ln" => Ok(Expr::Ln(e)),
                "log" => Ok(Expr::Ln(e)), // treat log as natural log
                "exp" => Ok(Expr::Exp(e)),
                "sqrt" => Ok(Expr::Sqrt(e)),
                "abs" => Ok(Expr::Abs(e)),
                _ => Err(format!("unknown function: {}", name)),
            };
        }
        // Constants
        match name.as_str() {
            "pi" | "PI" => return Ok(Expr::Num(std::f64::consts::PI)),
            "e" | "E" => return Ok(Expr::Num(std::f64::consts::E)),
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
            if n.fract() == 0.0 && n.abs() < 1e12 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        Expr::Var(v) => v.clone(),
        Expr::Add(a, b) => format!("({} + {})", fmt_expr(a), fmt_expr(b)),
        Expr::Sub(a, b) => format!("({} - {})", fmt_expr(a), fmt_expr(b)),
        Expr::Mul(a, b) => format!("({} * {})", fmt_expr(a), fmt_expr(b)),
        Expr::Div(a, b) => format!("({} / {})", fmt_expr(a), fmt_expr(b)),
        Expr::Pow(a, b) => format!("({}^{})", fmt_expr(a), fmt_expr(b)),
        Expr::Neg(a) => format!("(-{})", fmt_expr(a)),
        Expr::Sin(a) => format!("sin({})", fmt_expr(a)),
        Expr::Cos(a) => format!("cos({})", fmt_expr(a)),
        Expr::Tan(a) => format!("tan({})", fmt_expr(a)),
        Expr::Ln(a) => format!("ln({})", fmt_expr(a)),
        Expr::Exp(a) => format!("exp({})", fmt_expr(a)),
        Expr::Sqrt(a) => format!("sqrt({})", fmt_expr(a)),
        Expr::Abs(a) => format!("abs({})", fmt_expr(a)),
    }
}

// ── Simplification ─────────────────────────────────────────────────────────
// Single-pass algebraic simplification: fold constants, remove identity ops.

fn simplify(e: Expr) -> Expr {
    match e {
        Expr::Add(a, b) => {
            let a = simplify(*a);
            let b = simplify(*b);
            match (&a, &b) {
                (Expr::Num(x), Expr::Num(y)) => Expr::Num(x + y),
                (Expr::Num(0.0), _) => b,
                (_, Expr::Num(0.0)) => a,
                _ => Expr::Add(Box::new(a), Box::new(b)),
            }
        }
        Expr::Sub(a, b) => {
            let a = simplify(*a);
            let b = simplify(*b);
            match (&a, &b) {
                (Expr::Num(x), Expr::Num(y)) => Expr::Num(x - y),
                (_, Expr::Num(0.0)) => a,
                _ if fmt_expr(&a) == fmt_expr(&b) => Expr::Num(0.0),
                _ => Expr::Sub(Box::new(a), Box::new(b)),
            }
        }
        Expr::Mul(a, b) => {
            let a = simplify(*a);
            let b = simplify(*b);
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
            let a = simplify(*a);
            let b = simplify(*b);
            match (&a, &b) {
                (_, Expr::Num(1.0)) => a,
                (Expr::Num(x), Expr::Num(y)) if *y != 0.0 => Expr::Num(x / y),
                _ => Expr::Div(Box::new(a), Box::new(b)),
            }
        }
        Expr::Pow(a, b) => {
            let a = simplify(*a);
            let b = simplify(*b);
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
        Expr::Sin(a) => {
            let a = simplify(*a);
            Expr::Sin(Box::new(a))
        }
        Expr::Cos(a) => {
            let a = simplify(*a);
            Expr::Cos(Box::new(a))
        }
        Expr::Tan(a) => {
            let a = simplify(*a);
            Expr::Tan(Box::new(a))
        }
        Expr::Ln(a) => {
            let a = simplify(*a);
            if let Expr::Num(n) = &a {
                if (*n - std::f64::consts::E).abs() < 1e-12 {
                    return Expr::Num(1.0);
                }
            }
            Expr::Ln(Box::new(a))
        }
        Expr::Exp(a) => {
            let a = simplify(*a);
            if let Expr::Num(n) = &a {
                return Expr::Num(n.exp());
            }
            if let Expr::Num(0.0) = &a {
                return Expr::Num(1.0);
            }
            Expr::Exp(Box::new(a))
        }
        Expr::Sqrt(a) => {
            let a = simplify(*a);
            if let Expr::Num(n) = &a {
                if *n >= 0.0 {
                    return Expr::Num(n.sqrt());
                }
            }
            Expr::Sqrt(Box::new(a))
        }
        other => other,
    }
}

// ── Differentiation ────────────────────────────────────────────────────────

fn diff(e: &Expr, var: &str) -> Expr {
    match e {
        Expr::Num(_) => Expr::Num(0.0),
        Expr::Var(v) => {
            if v == var {
                Expr::Num(1.0)
            } else {
                Expr::Num(0.0)
            }
        }
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
            let num = Expr::Sub(Box::new(fp_g), Box::new(f_gp));
            let den = Expr::Pow(Box::new(*b.clone()), Box::new(Expr::Num(2.0)));
            simplify(Expr::Div(Box::new(num), Box::new(den)))
        }
        Expr::Pow(base, exp) => {
            // If exp is a constant: power rule n*x^(n-1) * base'
            if let Expr::Num(n) = exp.as_ref() {
                let new_exp = Expr::Num(n - 1.0);
                let power = Expr::Pow(Box::new(*base.clone()), Box::new(new_exp));
                let coeff = Expr::Mul(Box::new(Expr::Num(*n)), Box::new(power));
                let chain = diff(base, var);
                return simplify(Expr::Mul(Box::new(coeff), Box::new(chain)));
            }
            // General: e^(exp * ln(base)) rule: f^g * (g' ln f + g f'/f)
            let ln_base = Expr::Ln(Box::new(*base.clone()));
            let g_ln_f = Expr::Mul(Box::new(*exp.clone()), Box::new(ln_base));
            let g_ln_f_d = diff(&g_ln_f, var);
            let result = Expr::Mul(Box::new(e.clone()), Box::new(g_ln_f_d));
            simplify(result)
        }
        Expr::Neg(a) => simplify(Expr::Neg(Box::new(diff(a, var)))),
        Expr::Sin(a) => {
            let cos_a = Expr::Cos(Box::new(*a.clone()));
            simplify(Expr::Mul(Box::new(cos_a), Box::new(diff(a, var))))
        }
        Expr::Cos(a) => {
            let neg_sin = Expr::Neg(Box::new(Expr::Sin(Box::new(*a.clone()))));
            simplify(Expr::Mul(Box::new(neg_sin), Box::new(diff(a, var))))
        }
        Expr::Tan(a) => {
            // sec^2(a) * a' = 1/cos^2(a) * a'
            let cos_a = Expr::Cos(Box::new(*a.clone()));
            let cos2 = Expr::Pow(Box::new(cos_a), Box::new(Expr::Num(2.0)));
            let sec2 = Expr::Div(Box::new(Expr::Num(1.0)), Box::new(cos2));
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
            let two_sqrt = Expr::Mul(
                Box::new(Expr::Num(2.0)),
                Box::new(Expr::Sqrt(Box::new(*a.clone()))),
            );
            let inv = Expr::Div(Box::new(Expr::Num(1.0)), Box::new(two_sqrt));
            simplify(Expr::Mul(Box::new(inv), Box::new(diff(a, var))))
        }
        Expr::Abs(a) => {
            // d/dx |a| = a/|a| * a'  (sign(a) * a')
            let sign = Expr::Div(
                Box::new(*a.clone()),
                Box::new(Expr::Abs(Box::new(*a.clone()))),
            );
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
            Some(Expr::Mul(
                Box::new(Expr::Num(*n)),
                Box::new(Expr::Var(var.to_string())),
            ))
        }
        Expr::Var(v) => {
            if v == var {
                // ∫ x dx = x^2/2
                Some(Expr::Div(
                    Box::new(Expr::Pow(
                        Box::new(Expr::Var(v.clone())),
                        Box::new(Expr::Num(2.0)),
                    )),
                    Box::new(Expr::Num(2.0)),
                ))
            } else {
                // ∫ c dx where c is another variable treated as constant
                Some(Expr::Mul(
                    Box::new(Expr::Var(v.clone())),
                    Box::new(Expr::Var(var.to_string())),
                ))
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
                            return Some(Expr::Ln(Box::new(Expr::Abs(Box::new(Expr::Var(
                                v.clone(),
                            ))))));
                        }
                        // ∫ x^n = x^(n+1)/(n+1)
                        let new_exp = Expr::Num(n + 1.0);
                        let pow =
                            Expr::Pow(Box::new(Expr::Var(v.clone())), Box::new(new_exp.clone()));
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
                let neg_cos = Expr::Neg(Box::new(cos_part));
                return Some(simplify(Expr::Div(
                    Box::new(neg_cos),
                    Box::new(Expr::Num(coeff)),
                )));
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
                return Some(simplify(Expr::Div(
                    Box::new(sin_part),
                    Box::new(Expr::Num(coeff)),
                )));
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
                return Some(simplify(Expr::Div(
                    Box::new(e.clone()),
                    Box::new(Expr::Num(coeff)),
                )));
            }
            None
        }
        Expr::Ln(a) => {
            if let Expr::Var(v) = a.as_ref() {
                if v == var {
                    // ∫ ln(x) = x*ln(x) - x
                    let x_ln_x = Expr::Mul(Box::new(Expr::Var(v.clone())), Box::new(e.clone()));
                    return Some(simplify(Expr::Sub(
                        Box::new(x_ln_x),
                        Box::new(Expr::Var(v.clone())),
                    )));
                }
            }
            None
        }
        Expr::Div(a, b) => {
            // ∫ 1/x = ln|x|
            if let (Expr::Num(1.0), Expr::Var(v)) = (a.as_ref(), b.as_ref()) {
                if v == var {
                    return Some(Expr::Ln(Box::new(Expr::Abs(Box::new(Expr::Var(
                        v.clone(),
                    ))))));
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
                if v == var {
                    return Some((*c, var));
                }
            }
            if let (Expr::Var(v), Expr::Num(c)) = (a.as_ref(), b.as_ref()) {
                if v == var {
                    return Some((*c, var));
                }
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
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            contains_var(a, var) || contains_var(b, var)
        }
        Expr::Neg(a)
        | Expr::Sin(a)
        | Expr::Cos(a)
        | Expr::Tan(a)
        | Expr::Ln(a)
        | Expr::Exp(a)
        | Expr::Sqrt(a)
        | Expr::Abs(a) => contains_var(a, var),
    }
}

// ── Numeric eval ──────────────────────────────────────────────────────────

fn eval_expr(e: &Expr, var: &str, val: f64) -> Result<f64, String> {
    match e {
        Expr::Num(n) => Ok(*n),
        Expr::Var(v) => {
            if v == var {
                Ok(val)
            } else {
                Err(format!("unbound variable: {}", v))
            }
        }
        Expr::Add(a, b) => Ok(eval_expr(a, var, val)? + eval_expr(b, var, val)?),
        Expr::Sub(a, b) => Ok(eval_expr(a, var, val)? - eval_expr(b, var, val)?),
        Expr::Mul(a, b) => Ok(eval_expr(a, var, val)? * eval_expr(b, var, val)?),
        Expr::Div(a, b) => {
            let d = eval_expr(b, var, val)?;
            if d.abs() < 1e-300 {
                return Err("division by zero".into());
            }
            Ok(eval_expr(a, var, val)? / d)
        }
        Expr::Pow(a, b) => Ok(eval_expr(a, var, val)?.powf(eval_expr(b, var, val)?)),
        Expr::Neg(a) => Ok(-eval_expr(a, var, val)?),
        Expr::Sin(a) => Ok(eval_expr(a, var, val)?.sin()),
        Expr::Cos(a) => Ok(eval_expr(a, var, val)?.cos()),
        Expr::Tan(a) => Ok(eval_expr(a, var, val)?.tan()),
        Expr::Ln(a) => Ok(eval_expr(a, var, val)?.ln()),
        Expr::Exp(a) => Ok(eval_expr(a, var, val)?.exp()),
        Expr::Sqrt(a) => Ok(eval_expr(a, var, val)?.sqrt()),
        Expr::Abs(a) => Ok(eval_expr(a, var, val)?.abs()),
    }
}

// ── Public entry point ────────────────────────────────────────────────────

pub fn symbolic_calc(query: &str) -> String {
    let q = query.trim();

    // Parse optional "wrt VAR" suffix
    let (q_body, var) = if let Some(pos) = q.to_lowercase().rfind(" wrt ") {
        let v = q[pos + 5..].trim().to_string();
        (q[..pos].trim(), v)
    } else {
        (q, "x".to_string())
    };

    // Parse mode
    let (mode, expr_str) = {
        let low = q_body.to_lowercase();
        if low.starts_with("diff ")
            || low.starts_with("differentiate ")
            || low.starts_with("d/dx ")
            || low.starts_with("d/d")
        {
            // d/dy expr — extract var from d/dy if present
            let (m, rest, var_from_mode) = if low.starts_with("d/d") {
                let after = &q_body[3..];
                let sp = after.find(char::is_whitespace).unwrap_or(after.len());
                let v = after[..sp].to_string();
                let rest = after[sp..].trim();
                ("diff", rest, Some(v))
            } else {
                let rest = q_body
                    .split_once(char::is_whitespace)
                    .map(|x| x.1)
                    .unwrap_or("")
                    .trim();
                ("diff", rest, None)
            };
            let var2 = var_from_mode.unwrap_or_else(|| var.clone());
            (m, (rest.to_string(), var2))
        } else if low.starts_with("int ") || low.starts_with("integrate ") || low.starts_with("∫")
        {
            let rest = q_body
                .split_once(char::is_whitespace)
                .map(|x| x.1)
                .unwrap_or("")
                .trim();
            ("integrate", (rest.to_string(), var.clone()))
        } else if low.starts_with("simplify ") || low.starts_with("simplify") {
            let rest = q_body
                .split_once(char::is_whitespace)
                .map(|x| x.1)
                .unwrap_or("")
                .trim();
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
    let var_name = if var_name.is_empty() {
        "x".to_string()
    } else {
        var_name
    };

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
            (&at_str[..eq], &at_str[eq + 1..])
        } else {
            (&var_name[..], at_str)
        };
        let val: f64 = match val_str.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                let _ = writeln!(out, "  Error: bad value '{}'", val_str);
                return out;
            }
        };
        match parse_sym(e_str) {
            Ok(expr) => {
                let _ = writeln!(out, "  f({}) = {}", av, fmt_expr(&expr));
                match eval_expr(&expr, av.trim(), val) {
                    Ok(result) => {
                        let _ = writeln!(out, "  f({} = {}) = {}", av.trim(), val, result);
                    }
                    Err(e) => {
                        let _ = writeln!(out, "  Eval error: {}", e);
                    }
                }
            }
            Err(e) => {
                let _ = writeln!(out, "  Parse error: {}", e);
            }
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
                    if let (Ok(fp), Ok(fm)) = (
                        eval_expr(&simplified, &var_name, x0 + h),
                        eval_expr(&simplified, &var_name, x0 - h),
                    ) {
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
                            let orig = fmt_expr(&simplified);
                            let back = fmt_expr(&check);
                            if orig == back {
                                let _ =
                                    writeln!(out, "  ✓ Verified: d/d{} of integral = f", var_name);
                            } else {
                                // Numeric check at a sample point
                                let x0 = 1.5f64;
                                if let (Ok(v1), Ok(v2)) = (
                                    eval_expr(&simplified, &var_name, x0),
                                    eval_expr(&check, &var_name, x0),
                                ) {
                                    if (v1 - v2).abs() < 1e-6 {
                                        let _ = writeln!(out, "  ✓ Numerically verified: d/d{}(integral) = f at {}={}", var_name, var_name, x0);
                                    } else {
                                        let _ = writeln!(
                                            out,
                                            "  ⚠ Verification: d/d{}(integral) = {} (expected {})",
                                            var_name, back, orig
                                        );
                                    }
                                }
                            }
                        }
                        None => {
                            let _ = writeln!(
                                out,
                                "  ∫f d{} = (not in table — try --simulate or a CAS)",
                                var_name
                            );
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
     Supported: + - * / ^ sin cos tan ln exp sqrt abs"
        .into()
}

// ── Signal processing (DSP) ───────────────────────────────────────────────────
// DFT/IDFT, convolution, cross-correlation, moving average,
// FIR window filter design, waveform generation, RMS/SNR stats.
// All pure-Rust — no Python subprocess, no external crates.

use std::f64::consts::PI;

fn dft(signal: &[f64]) -> Vec<(f64, f64)> {
    let n = signal.len();
    (0..n)
        .map(|k| {
            let (mut re, mut im) = (0.0_f64, 0.0_f64);
            for (t, &x) in signal.iter().enumerate() {
                let angle = -2.0 * PI * (k * t) as f64 / n as f64;
                re += x * angle.cos();
                im += x * angle.sin();
            }
            (re, im)
        })
        .collect()
}

fn idft(spectrum: &[(f64, f64)]) -> Vec<f64> {
    let n = spectrum.len();
    (0..n)
        .map(|t| {
            let mut val = 0.0_f64;
            for (k, &(re, im)) in spectrum.iter().enumerate() {
                let angle = 2.0 * PI * (k * t) as f64 / n as f64;
                val += re * angle.cos() - im * angle.sin();
            }
            val / n as f64
        })
        .collect()
}

fn convolve(x: &[f64], h: &[f64]) -> Vec<f64> {
    let n = x.len() + h.len() - 1;
    (0..n)
        .map(|i| {
            let mut s = 0.0_f64;
            for (j, &hv) in h.iter().enumerate() {
                if i >= j && i - j < x.len() {
                    s += x[i - j] * hv;
                }
            }
            s
        })
        .collect()
}

fn xcorr(x: &[f64], y: &[f64]) -> Vec<f64> {
    let n = x.len();
    let m = y.len();
    let out_len = n + m - 1;
    (0..out_len)
        .map(|lag| {
            let lag_i = lag as isize - (m as isize - 1);
            let mut s = 0.0_f64;
            for (i, &xv) in x.iter().enumerate() {
                let j = i as isize - lag_i;
                if j >= 0 && j < m as isize {
                    s += xv * y[j as usize];
                }
            }
            s
        })
        .collect()
}

fn moving_avg(signal: &[f64], window: usize) -> Vec<f64> {
    let w = window.max(1);
    signal
        .windows(w)
        .map(|s| s.iter().sum::<f64>() / w as f64)
        .collect()
}

fn hann_window(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f64 / (n - 1) as f64).cos()))
        .collect()
}

fn hamming_window(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| 0.54 - 0.46 * (2.0 * PI * i as f64 / (n - 1) as f64).cos())
        .collect()
}

fn blackman_window(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let a = 2.0 * PI * i as f64 / (n - 1) as f64;
            0.42 - 0.5 * a.cos() + 0.08 * (2.0 * a).cos()
        })
        .collect()
}

fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

fn fir_lowpass(n_taps: usize, cutoff_norm: f64, window: &str) -> Vec<f64> {
    let m = n_taps - 1;
    let wins: Vec<f64> = match window {
        "hann" | "hanning" => hann_window(n_taps),
        "hamming" => hamming_window(n_taps),
        "blackman" => blackman_window(n_taps),
        _ => vec![1.0; n_taps],
    };
    let mut h: Vec<f64> = (0..n_taps)
        .map(|i| {
            let n = i as f64 - m as f64 / 2.0;
            2.0 * cutoff_norm * sinc(2.0 * cutoff_norm * n) * wins[i]
        })
        .collect();
    let sum: f64 = h.iter().sum();
    if sum.abs() > 1e-12 {
        for v in &mut h {
            *v /= sum;
        }
    }
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
    let v: Vec<f64> = s
        .split([',', ' ', '\t', ';'].as_ref())
        .filter_map(|t| t.trim().parse::<f64>().ok())
        .collect();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
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
    if sig.is_empty() {
        return String::new();
    }
    let mn = sig.iter().cloned().fold(f64::INFINITY, f64::min);
    let mx = sig.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (mx - mn).max(1e-12);
    let step = sig.len().max(1) as f64 / width as f64;
    let samples: Vec<f64> = (0..width)
        .map(|col| {
            let idx = ((col as f64 * step) as usize).min(sig.len() - 1);
            sig[idx]
        })
        .collect();
    let mut rows = vec![vec![' '; width]; height];
    for (col, &val) in samples.iter().enumerate() {
        let row = height - 1 - ((val - mn) / range * (height - 1) as f64).round() as usize;
        let row = row.min(height - 1);
        rows[row][col] = '█';
    }
    rows.iter()
        .map(|r| r.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
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
            None => {
                let _ = writeln!(out, "  ERROR: no numeric values found.");
            }
            Some(sig) => {
                let n = sig.len();
                let spectrum = dft(&sig);
                let (mean, rms, mn, mx) = signal_stats(&sig);
                let _ = writeln!(out, "  DFT of {}-point signal", n);
                let _ = writeln!(
                    out,
                    "  mean={:.4}  RMS={:.4}  min={:.4}  max={:.4}",
                    mean, rms, mn, mx
                );
                let _ = writeln!(out);
                let _ = writeln!(
                    out,
                    "  {:>5}  {:>10}  {:>10}  {:>10}  {:>10}",
                    "Bin", "Re", "Im", "Magnitude", "Phase°"
                );
                let _ = writeln!(out, "  {}", "-".repeat(52));
                let show = (n / 2 + 1).min(20);
                for k in 0..show {
                    let (re, im) = spectrum[k];
                    let mag = (re * re + im * im).sqrt();
                    let phase = im.atan2(re).to_degrees();
                    let _ = writeln!(
                        out,
                        "  {:>5}  {:>10.4}  {:>10.4}  {:>10.4}  {:>10.2}",
                        k, re, im, mag, phase
                    );
                }
                if show < n / 2 + 1 {
                    let _ = writeln!(out, "  … ({} bins total)", n / 2 + 1);
                }
                let dc = spectrum[0].0 / n as f64;
                let _ = writeln!(out);
                let _ = writeln!(out, "  DC component: {:.6}", dc);
                let dominant = spectrum[1..n / 2 + 1]
                    .iter()
                    .enumerate()
                    .map(|(i, &(r, im))| (i + 1, (r * r + im * im).sqrt()))
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
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
            None => {
                let _ = writeln!(out, "  ERROR: no numeric values found.");
            }
            Some(vals) => {
                if vals.len() % 2 != 0 {
                    let _ = writeln!(
                        out,
                        "  ERROR: IDFT needs even number of values (re,im pairs)."
                    );
                } else {
                    let spectrum: Vec<(f64, f64)> = vals.chunks(2).map(|c| (c[0], c[1])).collect();
                    let sig = idft(&spectrum);
                    let _ = writeln!(out, "  IDFT result ({} samples):", sig.len());
                    let _ = writeln!(
                        out,
                        "  {:?}",
                        sig.iter()
                            .map(|v| format!("{:.6}", v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }
        }
    }
    // --- CONVOLVE -----------------------------------------------------------
    else if lower.starts_with("conv ") || lower.starts_with("convolve ") {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        if let Some(mid) = rest
            .find(" ; ")
            .or_else(|| rest.find(" with "))
            .or_else(|| rest.find(" | "))
        {
            let (a_str, b_str) = rest.split_at(mid);
            let b_str = b_str
                .trim_start_matches([' ', ';', '|'].as_ref())
                .trim_start_matches("with")
                .trim();
            match (parse_signal(a_str.trim()), parse_signal(b_str)) {
                (Some(x), Some(h)) => {
                    let y = convolve(&x, &h);
                    let _ = writeln!(
                        out,
                        "  Convolution  x[{}] * h[{}] = y[{}]",
                        x.len(),
                        h.len(),
                        y.len()
                    );
                    let _ = writeln!(
                        out,
                        "  x: {}",
                        x.iter()
                            .map(|v| format!("{:.4}", v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    let _ = writeln!(
                        out,
                        "  h: {}",
                        h.iter()
                            .map(|v| format!("{:.4}", v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    let _ = writeln!(
                        out,
                        "  y: {}",
                        y.iter()
                            .map(|v| format!("{:.4}", v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    let (mean, rms, mn, mx) = signal_stats(&y);
                    let _ = writeln!(
                        out,
                        "  mean={:.4}  RMS={:.4}  min={:.4}  max={:.4}",
                        mean, rms, mn, mx
                    );
                }
                _ => {
                    let _ = writeln!(
                        out,
                        "  ERROR: use  conv  A,B,C ; D,E,F  (separate signals with ;)"
                    );
                }
            }
        } else {
            let _ = writeln!(
                out,
                "  ERROR: use  conv  A,B,C ; D,E,F  (separate signals with ;)"
            );
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
                    let peak_lag = r
                        .iter()
                        .enumerate()
                        .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
                        .map(|(i, _)| i as isize - (y.len() as isize - 1));
                    let _ = writeln!(
                        out,
                        "  Cross-correlation  x[{}] ⋆ y[{}] = r[{}]",
                        x.len(),
                        y.len(),
                        r.len()
                    );
                    let _ = writeln!(
                        out,
                        "  r: {}",
                        r.iter()
                            .map(|v| format!("{:.4}", v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    if let Some(lag) = peak_lag {
                        let _ = writeln!(out, "  Peak lag: {} samples", lag);
                    }
                }
                _ => {
                    let _ = writeln!(out, "  ERROR: use  xcorr  A,B,C ; D,E,F");
                }
            }
        } else {
            let _ = writeln!(out, "  ERROR: use  xcorr  A,B,C ; D,E,F");
        }
    }
    // --- MOVING AVERAGE -----------------------------------------------------
    else if lower.starts_with("movavg ")
        || lower.starts_with("moving-avg ")
        || lower.starts_with("sma ")
    {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        let window = parts[0].parse::<usize>().unwrap_or(3);
        let data_str = if parts.len() > 1 { parts[1] } else { "" };
        match parse_signal(data_str) {
            None => {
                let _ = writeln!(out, "  ERROR: use  movavg WINDOW v1,v2,...");
            }
            Some(sig) => {
                let smoothed = moving_avg(&sig, window);
                let _ = writeln!(out, "  Simple Moving Average  window={}", window);
                let _ = writeln!(
                    out,
                    "  Input ({} pts): {}",
                    sig.len(),
                    sig.iter()
                        .take(8)
                        .map(|v| format!("{:.3}", v))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                let _ = writeln!(
                    out,
                    "  Output ({} pts): {}",
                    smoothed.len(),
                    smoothed
                        .iter()
                        .take(8)
                        .map(|v| format!("{:.4}", v))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                if sig.len() > 8 {
                    let _ = writeln!(out, "  (showing first 8 of {} values)", sig.len());
                }
            }
        }
    }
    // --- FIR LOW-PASS FILTER ------------------------------------------------
    else if lower.starts_with("fir-lp ")
        || lower.starts_with("lowpass ")
        || lower.starts_with("lp ")
    {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        let parts: Vec<&str> = rest.splitn(3, ' ').collect();
        let cutoff = parts
            .first()
            .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok())
            .unwrap_or(0.25)
            / if rest.contains('%') { 100.0 } else { 1.0 };
        let n_taps = parts
            .get(1)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(21);
        let window = parts.get(2).map(|s| s.trim()).unwrap_or("hamming");
        let h = fir_lowpass(n_taps, cutoff.min(0.5), window);
        let _ = writeln!(out, "  FIR Low-Pass Filter");
        let _ = writeln!(
            out,
            "  Cutoff: {:.4} (normalized, 0.5 = Nyquist)  Taps: {}  Window: {}",
            cutoff, n_taps, window
        );
        let _ = writeln!(out, "  Coefficients:");
        for (i, c) in h.iter().enumerate() {
            let _ = write!(out, "  h[{:2}]={:>10.6}", i, c);
            if (i + 1) % 4 == 0 {
                let _ = writeln!(out);
            }
        }
        let _ = writeln!(out);
        let sum: f64 = h.iter().sum();
        let _ = writeln!(
            out,
            "  Sum of taps: {:.6}  (DC gain = {:.4} dB)",
            sum,
            20.0 * sum.abs().log10()
        );
    }
    // --- FIR HIGH-PASS FILTER -----------------------------------------------
    else if lower.starts_with("fir-hp ")
        || lower.starts_with("highpass ")
        || lower.starts_with("hp ")
    {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        let parts: Vec<&str> = rest.splitn(3, ' ').collect();
        let cutoff = parts
            .first()
            .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok())
            .unwrap_or(0.25)
            / if rest.contains('%') { 100.0 } else { 1.0 };
        let n_taps = parts
            .get(1)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(21);
        let window = parts.get(2).map(|s| s.trim()).unwrap_or("hamming");
        let h = fir_highpass(n_taps, cutoff.min(0.49), window);
        let _ = writeln!(out, "  FIR High-Pass Filter");
        let _ = writeln!(
            out,
            "  Cutoff: {:.4}  Taps: {}  Window: {}",
            cutoff, n_taps, window
        );
        let _ = writeln!(out, "  Coefficients:");
        for (i, c) in h.iter().enumerate() {
            let _ = write!(out, "  h[{:2}]={:>10.6}", i, c);
            if (i + 1) % 4 == 0 {
                let _ = writeln!(out);
            }
        }
        let _ = writeln!(out);
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
                    let _ = writeln!(
                        out,
                        "  Filter applied  h[{}] * x[{}] = y[{}]",
                        h.len(),
                        x.len(),
                        y.len()
                    );
                    let (_, rms_x, _, _) = signal_stats(&x);
                    let (_, rms_y, _, _) = signal_stats(&y);
                    let _ = writeln!(out, "  Input  RMS: {:.4}", rms_x);
                    let _ = writeln!(out, "  Output RMS: {:.4}", rms_y);
                    let _ = writeln!(
                        out,
                        "  y: {}",
                        y.iter()
                            .map(|v| format!("{:.4}", v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                _ => {
                    let _ = writeln!(out, "  ERROR: use  filter h1,h2,... ; x1,x2,...");
                }
            }
        } else {
            let _ = writeln!(out, "  ERROR: use  filter h1,h2,... ; x1,x2,...");
        }
    }
    // --- STATS / INFO -------------------------------------------------------
    else if lower.starts_with("stats ") || lower.starts_with("info ") || lower.starts_with("rms ")
    {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        match parse_signal(rest) {
            None => {
                let _ = writeln!(out, "  ERROR: no numeric values.");
            }
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
                    let _ = writeln!(
                        out,
                        "  Crest:    {:>12.6}  ({:.2} dB)",
                        crest,
                        20.0 * crest.log10()
                    );
                }
                let _ = writeln!(out);
                let _ = writeln!(out, "  Waveform ({}×8):", sig.len().min(60));
                let wave = ascii_waveform(&sig, sig.len().min(60), 8);
                for line in wave.lines() {
                    let _ = writeln!(out, "  |{}|", line);
                }
            }
        }
    }
    // --- WAVEFORM GENERATE --------------------------------------------------
    else if lower.starts_with("gen ")
        || lower.starts_with("wave ")
        || lower.starts_with("generate ")
    {
        let rest = q[q.find(' ').unwrap_or(0)..].trim();
        let parts: Vec<&str> = rest.splitn(4, ' ').collect();
        let shape = parts
            .first()
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "sine".into());
        let freq: f64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let n: usize = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(64);
        let amp: f64 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let sig: Vec<f64> = (0..n)
            .map(|i| {
                let t = i as f64 / n as f64;
                let phase = 2.0 * PI * freq * t;
                match shape.as_str() {
                    "cos" | "cosine" => amp * phase.cos(),
                    "square" => amp * if phase.sin() >= 0.0 { 1.0 } else { -1.0 },
                    "sawtooth" | "saw" => amp * (2.0 * (freq * t - (freq * t + 0.5).floor())),
                    "triangle" | "tri" => {
                        amp * 2.0 * (2.0 * (freq * t - (freq * t + 0.5).floor())).abs() - 1.0
                    }
                    "noise" | "rand" => {
                        amp * (((i * 6364136223846793005 + 1442695040888963407) >> 33) as f64
                            / u32::MAX as f64
                            * 2.0
                            - 1.0)
                    }
                    _ => amp * phase.sin(),
                }
            })
            .collect();
        let (mean, rms, mn, mx) = signal_stats(&sig);
        let _ = writeln!(
            out,
            "  Waveform: {}  freq={} cycles  n={}  amp={}",
            shape, freq, n, amp
        );
        let _ = writeln!(
            out,
            "  mean={:.4}  RMS={:.4}  min={:.4}  max={:.4}",
            mean, rms, mn, mx
        );
        let _ = writeln!(out);
        let wave = ascii_waveform(&sig, sig.len().min(60), 8);
        for line in wave.lines() {
            let _ = writeln!(out, "  |{}|", line);
        }
        let _ = writeln!(out);
        let first = sig
            .iter()
            .take(16)
            .map(|v| format!("{:.4}", v))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "  First 16 samples: {}{}",
            first,
            if n > 16 { " …" } else { "" }
        );
    }
    // --- WINDOW PREVIEW -----------------------------------------------------
    else if lower.starts_with("window ") {
        let parts: Vec<&str> = q.splitn(3, ' ').collect();
        let win_type = parts
            .get(1)
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "hann".into());
        let n: usize = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(32);
        let w = match win_type.as_str() {
            "hamming" => hamming_window(n),
            "blackman" => blackman_window(n),
            _ => hann_window(n),
        };
        let (mean, rms, mn, mx) = signal_stats(&w);
        let _ = writeln!(out, "  {} window  n={}", win_type, n);
        let _ = writeln!(
            out,
            "  mean={:.4}  RMS={:.4}  min={:.4}  max={:.4}",
            mean, rms, mn, mx
        );
        let _ = writeln!(out);
        let wave = ascii_waveform(&w, n.min(60), 6);
        for line in wave.lines() {
            let _ = writeln!(out, "  |{}|", line);
        }
        let _ = writeln!(out);
        let first8 = w
            .iter()
            .take(8)
            .map(|v| format!("{:.4}", v))
            .collect::<Vec<_>>()
            .join(", ");
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
     Wave shapes:  sine cosine square sawtooth triangle noise"
        .into()
}

// ── Interpolation & curve fitting ─────────────────────────────────────────────
// Linear, Lagrange polynomial, natural cubic spline, nearest-neighbor,
// and linear extrapolation — all pure-Rust, instant, no subprocess.

fn interp_parse_points(s: &str) -> Option<Vec<(f64, f64)>> {
    // Accept:  "x1,y1 x2,y2 ..."  or  "x1,y1; x2,y2; ..."  or  "(x,y),(x,y)"
    let clean = s.replace(['(', ')'], "").replace(';', " ");
    let tokens: Vec<f64> = clean
        .split([',', ' ', '\t'].as_ref())
        .filter_map(|t| t.trim().parse::<f64>().ok())
        .collect();
    if tokens.len() < 4 || tokens.len() % 2 != 0 {
        return None;
    }
    Some(tokens.chunks(2).map(|c| (c[0], c[1])).collect())
}

fn interp_linear(points: &[(f64, f64)], x: f64) -> f64 {
    let n = points.len();
    if n == 0 {
        return f64::NAN;
    }
    if n == 1 {
        return points[0].1;
    }
    // extrapolate with end segments
    if x <= points[0].0 {
        let (x0, y0) = points[0];
        let (x1, y1) = points[1];
        return y0 + (x - x0) * (y1 - y0) / (x1 - x0);
    }
    if x >= points[n - 1].0 {
        let (x0, y0) = points[n - 2];
        let (x1, y1) = points[n - 1];
        return y0 + (x - x0) * (y1 - y0) / (x1 - x0);
    }
    for i in 0..n - 1 {
        let (x0, y0) = points[i];
        let (x1, y1) = points[i + 1];
        if x >= x0 && x <= x1 {
            return y0 + (x - x0) * (y1 - y0) / (x1 - x0);
        }
    }
    f64::NAN
}

fn interp_nearest(points: &[(f64, f64)], x: f64) -> f64 {
    points
        .iter()
        .min_by(|a, b| (a.0 - x).abs().partial_cmp(&(b.0 - x).abs()).unwrap())
        .map(|p| p.1)
        .unwrap_or(f64::NAN)
}

fn interp_lagrange(points: &[(f64, f64)], x: f64) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| {
            let (xi, yi) = points[i];
            let li = (0..n).filter(|&j| j != i).fold(1.0_f64, |acc, j| {
                acc * (x - points[j].0) / (xi - points[j].0)
            });
            yi * li
        })
        .sum()
}

// Natural cubic spline via tridiagonal solve
fn interp_spline_build(points: &[(f64, f64)]) -> Vec<(f64, f64, f64, f64)> {
    let n = points.len();
    if n < 2 {
        return vec![];
    }
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
    (0..n - 1)
        .map(|i| (b[i], c[i], d[i], points[i].1))
        .collect()
}

fn interp_spline_eval(points: &[(f64, f64)], coeffs: &[(f64, f64, f64, f64)], x: f64) -> f64 {
    let n = points.len();
    if n == 0 {
        return f64::NAN;
    }
    let i = if x <= points[0].0 {
        0
    } else if x >= points[n - 1].0 {
        n - 2
    } else {
        points[..n - 1]
            .iter()
            .enumerate()
            .find(|(i, p)| x >= p.0 && x <= points[i + 1].0)
            .map(|(i, _)| i)
            .unwrap_or(n - 2)
    };
    let i = i.min(coeffs.len().saturating_sub(1));
    let dx = x - points[i].0;
    let (b, c, d, a) = coeffs[i];
    a + b * dx + c * dx * dx + d * dx * dx * dx
}

fn interp_ascii_curve(
    points: &[(f64, f64)],
    eval_fn: &dyn Fn(f64) -> f64,
    width: usize,
    height: usize,
) -> String {
    if points.is_empty() {
        return String::new();
    }
    let xs: Vec<f64> = points.iter().map(|p| p.0).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.1).collect();
    let xmin = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let xmax = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let step = (xmax - xmin) / (width - 1) as f64;
    let curve_y: Vec<f64> = (0..width)
        .map(|i| eval_fn(xmin + i as f64 * step))
        .collect();
    let ymin = curve_y
        .iter()
        .cloned()
        .chain(ys.iter().cloned())
        .fold(f64::INFINITY, f64::min);
    let ymax = curve_y
        .iter()
        .cloned()
        .chain(ys.iter().cloned())
        .fold(f64::NEG_INFINITY, f64::max);
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
        let col = col.min(width - 1);
        let row = row.min(height - 1);
        grid[row][col] = '●';
    }
    let result = grid
        .iter()
        .map(|r| r.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n  y: [{:.4} .. {:.4}]\n  x: [{:.4} .. {:.4}]\n  ● = data point  · = curve",
        result, ymin, ymax, xmin, xmax
    )
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
    let (method, rest) = if lower.starts_with("linear ") {
        ("linear", &q[7..])
    } else if lower.starts_with("spline ") || lower.starts_with("cubic ") {
        ("spline", &q[7..])
    } else if lower.starts_with("lagrange ") || lower.starts_with("poly ") {
        ("lagrange", &q[lower.find(' ').unwrap_or(0) + 1..])
    } else if lower.starts_with("nearest ") {
        ("nearest", &q[8..])
    } else {
        ("linear", q)
    };

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
    let _ = writeln!(
        out,
        "  Points: {}",
        points
            .iter()
            .map(|(x, y)| format!("({:.4},{:.4})", x, y))
            .collect::<Vec<_>>()
            .join("  ")
    );
    let _ = writeln!(out);

    // Build spline coefficients once if needed
    let spline_coeffs = if method == "spline" {
        interp_spline_build(&points)
    } else {
        vec![]
    };

    let eval_fn: Box<dyn Fn(f64) -> f64> = match method {
        "spline" => {
            let pts = points.clone();
            let sc = spline_coeffs.clone();
            Box::new(move |x| interp_spline_eval(&pts, &sc, x))
        }
        "lagrange" => {
            let pts = points.clone();
            Box::new(move |x| interp_lagrange(&pts, x))
        }
        "nearest" => {
            let pts = points.clone();
            Box::new(move |x| interp_nearest(&pts, x))
        }
        _ => {
            let pts = points.clone();
            Box::new(move |x| interp_linear(&pts, x))
        }
    };

    // Evaluate at requested x values
    if !query_str.is_empty() {
        let xs: Vec<f64> = query_str
            .split([',', ' '].as_ref())
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();
        if !xs.is_empty() {
            let _ = writeln!(out, "  {:>12}  {:>14}", "x", "y (interpolated)");
            let _ = writeln!(out, "  {}", "-".repeat(28));
            for &x in &xs {
                let y = eval_fn(x);
                let xmin = points[0].0;
                let xmax = points[points.len() - 1].0;
                let tag = if x < xmin || x > xmax {
                    "  [extrapolated]"
                } else {
                    ""
                };
                let _ = writeln!(out, "  {:>12.6}  {:>14.8}{}", x, y, tag);
            }
            let _ = writeln!(out);
        }
    }

    // Always show a dense evaluation table and ASCII curve
    let xmin = points[0].0;
    let xmax = points[points.len() - 1].0;
    let steps = 9_usize;
    let _ = writeln!(out, "  Sampled curve ({} pts across range):", steps + 1);
    let _ = writeln!(out, "  {:>10}  {:>14}", "x", "y");
    let _ = writeln!(out, "  {}", "-".repeat(26));
    for i in 0..=steps {
        let x = xmin + i as f64 * (xmax - xmin) / steps as f64;
        let y = eval_fn(x);
        let _ = writeln!(out, "  {:>10.4}  {:>14.8}", x, y);
    }
    let _ = writeln!(out);

    // ASCII curve
    let curve_str = interp_ascii_curve(&points, &eval_fn, 56, 10);
    for line in curve_str.lines() {
        let _ = writeln!(out, "  {}", line);
    }

    let _ = writeln!(out, "{}", sep);
    out
}

// ── Unit conversion ───────────────────────────────────────────────────────────
// Pure-Rust lookup tables. Handles VALUE FROM_UNIT to TO_UNIT, or just
// VALUE UNIT for a multi-target broadcast. Temperature uses offset math.

struct UnitDef {
    names: &'static [&'static str],
    to_base: f64, // multiply by this to reach SI base; NaN for temperature
}

struct UnitCat {
    name: &'static str,
    base: &'static str,
    units: &'static [UnitDef],
}

static UNIT_TABLE: &[UnitCat] = &[
    UnitCat {
        name: "Length",
        base: "m",
        units: &[
            UnitDef {
                names: &["m", "meter", "metre", "meters", "metres"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["km", "kilometer", "kilometre", "kilometers", "kilometres"],
                to_base: 1e3,
            },
            UnitDef {
                names: &[
                    "cm",
                    "centimeter",
                    "centimetre",
                    "centimeters",
                    "centimetres",
                ],
                to_base: 0.01,
            },
            UnitDef {
                names: &[
                    "mm",
                    "millimeter",
                    "millimetre",
                    "millimeters",
                    "millimetres",
                ],
                to_base: 0.001,
            },
            UnitDef {
                names: &["um", "micrometer", "micrometre", "micron"],
                to_base: 1e-6,
            },
            UnitDef {
                names: &["nm", "nanometer", "nanometre"],
                to_base: 1e-9,
            },
            UnitDef {
                names: &["mi", "mile", "miles"],
                to_base: 1609.344,
            },
            UnitDef {
                names: &["yd", "yard", "yards"],
                to_base: 0.9144,
            },
            UnitDef {
                names: &["ft", "foot", "feet"],
                to_base: 0.3048,
            },
            UnitDef {
                names: &["in", "inch", "inches"],
                to_base: 0.0254,
            },
            UnitDef {
                names: &["nmi", "nautical_mile", "nm_sea"],
                to_base: 1852.0,
            },
            UnitDef {
                names: &["ly", "lightyear", "light_year"],
                to_base: 9.461e15,
            },
            UnitDef {
                names: &["au", "astronomical_unit"],
                to_base: 1.496e11,
            },
            UnitDef {
                names: &["pc", "parsec"],
                to_base: 3.086e16,
            },
            UnitDef {
                names: &["angstrom", "ang"],
                to_base: 1e-10,
            },
        ],
    },
    UnitCat {
        name: "Area",
        base: "m2",
        units: &[
            UnitDef {
                names: &["m2", "sqm", "square_meter", "square_metre"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["km2", "sqkm", "square_kilometer"],
                to_base: 1e6,
            },
            UnitDef {
                names: &["cm2", "sqcm", "square_centimeter"],
                to_base: 1e-4,
            },
            UnitDef {
                names: &["mm2", "sqmm", "square_millimeter"],
                to_base: 1e-6,
            },
            UnitDef {
                names: &["ha", "hectare", "hectares"],
                to_base: 1e4,
            },
            UnitDef {
                names: &["acre", "acres"],
                to_base: 4046.856,
            },
            UnitDef {
                names: &["ft2", "sqft", "square_foot", "square_feet"],
                to_base: 0.09290304,
            },
            UnitDef {
                names: &["in2", "sqin", "square_inch"],
                to_base: 6.4516e-4,
            },
            UnitDef {
                names: &["mi2", "sqmi", "square_mile"],
                to_base: 2.58999e6,
            },
            UnitDef {
                names: &["yd2", "sqyd", "square_yard"],
                to_base: 0.83612736,
            },
        ],
    },
    UnitCat {
        name: "Volume",
        base: "m3",
        units: &[
            UnitDef {
                names: &["m3", "cbm", "cubic_meter", "cubic_metre"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["l", "liter", "litre", "liters", "litres"],
                to_base: 0.001,
            },
            UnitDef {
                names: &[
                    "ml",
                    "milliliter",
                    "millilitre",
                    "milliliters",
                    "millilitres",
                ],
                to_base: 1e-6,
            },
            UnitDef {
                names: &["cl", "centiliter", "centilitre"],
                to_base: 1e-5,
            },
            UnitDef {
                names: &["dl", "deciliter", "decilitre"],
                to_base: 1e-4,
            },
            UnitDef {
                names: &["gal", "gallon", "gallons"],
                to_base: 0.00378541,
            },
            UnitDef {
                names: &["qt", "quart", "quarts"],
                to_base: 9.46353e-4,
            },
            UnitDef {
                names: &["pt", "pint", "pints"],
                to_base: 4.73176e-4,
            },
            UnitDef {
                names: &["cup", "cups"],
                to_base: 2.36588e-4,
            },
            UnitDef {
                names: &["floz", "fl_oz", "fluid_ounce", "fluid_ounces"],
                to_base: 2.95735e-5,
            },
            UnitDef {
                names: &["tbsp", "tablespoon", "tablespoons"],
                to_base: 1.47868e-5,
            },
            UnitDef {
                names: &["tsp", "teaspoon", "teaspoons"],
                to_base: 4.92892e-6,
            },
            UnitDef {
                names: &["ft3", "cbft", "cubic_foot", "cubic_feet"],
                to_base: 0.0283168,
            },
            UnitDef {
                names: &["in3", "cbin", "cubic_inch"],
                to_base: 1.63871e-5,
            },
            UnitDef {
                names: &["bbl", "barrel", "barrels"],
                to_base: 0.158987,
            },
        ],
    },
    UnitCat {
        name: "Mass",
        base: "kg",
        units: &[
            UnitDef {
                names: &["kg", "kilogram", "kilograms", "kilo"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["g", "gram", "grams"],
                to_base: 0.001,
            },
            UnitDef {
                names: &["mg", "milligram", "milligrams"],
                to_base: 1e-6,
            },
            UnitDef {
                names: &["ug", "microgram", "micrograms"],
                to_base: 1e-9,
            },
            UnitDef {
                names: &["t", "tonne", "metric_ton", "tonnes"],
                to_base: 1000.0,
            },
            UnitDef {
                names: &["lb", "pound", "pounds", "lbs"],
                to_base: 0.453592,
            },
            UnitDef {
                names: &["oz", "ounce", "ounces"],
                to_base: 0.0283495,
            },
            UnitDef {
                names: &["st", "stone", "stones"],
                to_base: 6.35029,
            },
            UnitDef {
                names: &["ton", "short_ton", "tons"],
                to_base: 907.185,
            },
            UnitDef {
                names: &["long_ton", "long_tons"],
                to_base: 1016.05,
            },
            UnitDef {
                names: &["ct", "carat", "carats"],
                to_base: 2e-4,
            },
            UnitDef {
                names: &["gr", "grain", "grains"],
                to_base: 6.47989e-5,
            },
            UnitDef {
                names: &["u", "amu", "dalton"],
                to_base: 1.66054e-27,
            },
        ],
    },
    UnitCat {
        name: "Time",
        base: "s",
        units: &[
            UnitDef {
                names: &["s", "sec", "second", "seconds"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["ms", "millisecond", "milliseconds"],
                to_base: 0.001,
            },
            UnitDef {
                names: &["us", "microsecond", "microseconds"],
                to_base: 1e-6,
            },
            UnitDef {
                names: &["ns", "nanosecond", "nanoseconds"],
                to_base: 1e-9,
            },
            UnitDef {
                names: &["min", "minute", "minutes"],
                to_base: 60.0,
            },
            UnitDef {
                names: &["h", "hr", "hour", "hours"],
                to_base: 3600.0,
            },
            UnitDef {
                names: &["d", "day", "days"],
                to_base: 86400.0,
            },
            UnitDef {
                names: &["wk", "week", "weeks"],
                to_base: 604800.0,
            },
            UnitDef {
                names: &["mo", "month", "months"],
                to_base: 2.628e6,
            },
            UnitDef {
                names: &["yr", "year", "years"],
                to_base: 3.156e7,
            },
            UnitDef {
                names: &["decade", "decades"],
                to_base: 3.156e8,
            },
            UnitDef {
                names: &["century", "centuries"],
                to_base: 3.156e9,
            },
        ],
    },
    UnitCat {
        name: "Speed",
        base: "m/s",
        units: &[
            UnitDef {
                names: &["mps", "m/s", "meter_per_second"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["kph", "km/h", "kmh", "kilometer_per_hour"],
                to_base: 1.0 / 3.6,
            },
            UnitDef {
                names: &["mph", "mi/h", "mile_per_hour"],
                to_base: 0.44704,
            },
            UnitDef {
                names: &["kn", "kt", "knot", "knots"],
                to_base: 0.514444,
            },
            UnitDef {
                names: &["fps", "ft/s", "foot_per_second"],
                to_base: 0.3048,
            },
            UnitDef {
                names: &["mach"],
                to_base: 343.0,
            },
            UnitDef {
                names: &["c", "speed_of_light"],
                to_base: 2.998e8,
            },
        ],
    },
    UnitCat {
        name: "Pressure",
        base: "Pa",
        units: &[
            UnitDef {
                names: &["pa", "pascal", "pascals"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["kpa", "kilopascal", "kilopascals"],
                to_base: 1000.0,
            },
            UnitDef {
                names: &["mpa", "megapascal", "megapascals"],
                to_base: 1e6,
            },
            UnitDef {
                names: &["bar", "bars"],
                to_base: 1e5,
            },
            UnitDef {
                names: &["mbar", "millibar", "millibars"],
                to_base: 100.0,
            },
            UnitDef {
                names: &["atm", "atmosphere", "atmospheres"],
                to_base: 101325.0,
            },
            UnitDef {
                names: &["torr", "mmhg"],
                to_base: 133.322,
            },
            UnitDef {
                names: &["psi", "pound_per_square_inch"],
                to_base: 6894.76,
            },
            UnitDef {
                names: &["inhg", "in_hg", "inches_of_mercury"],
                to_base: 3386.39,
            },
        ],
    },
    UnitCat {
        name: "Energy",
        base: "J",
        units: &[
            UnitDef {
                names: &["j", "joule", "joules"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["kj", "kilojoule", "kilojoules"],
                to_base: 1000.0,
            },
            UnitDef {
                names: &["mj", "megajoule", "megajoules"],
                to_base: 1e6,
            },
            UnitDef {
                names: &["gj", "gigajoule", "gigajoules"],
                to_base: 1e9,
            },
            UnitDef {
                names: &["cal", "calorie", "calories"],
                to_base: 4.184,
            },
            UnitDef {
                names: &["kcal", "kilocalorie", "kilocalories", "food_calorie"],
                to_base: 4184.0,
            },
            UnitDef {
                names: &["wh", "watt_hour", "watt_hours"],
                to_base: 3600.0,
            },
            UnitDef {
                names: &["kwh", "kilowatt_hour", "kilowatt_hours"],
                to_base: 3.6e6,
            },
            UnitDef {
                names: &["mwh", "megawatt_hour"],
                to_base: 3.6e9,
            },
            UnitDef {
                names: &["gwh", "gigawatt_hour"],
                to_base: 3.6e12,
            },
            UnitDef {
                names: &["btu", "british_thermal_unit"],
                to_base: 1055.06,
            },
            UnitDef {
                names: &["ev", "electronvolt", "electron_volt"],
                to_base: 1.602e-19,
            },
            UnitDef {
                names: &["ftlb", "ft_lb", "foot_pound"],
                to_base: 1.35582,
            },
            UnitDef {
                names: &["erg", "ergs"],
                to_base: 1e-7,
            },
        ],
    },
    UnitCat {
        name: "Power",
        base: "W",
        units: &[
            UnitDef {
                names: &["w", "watt", "watts"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["kw", "kilowatt", "kilowatts"],
                to_base: 1000.0,
            },
            UnitDef {
                names: &["mw", "megawatt", "megawatts"],
                to_base: 1e6,
            },
            UnitDef {
                names: &["gw", "gigawatt", "gigawatts"],
                to_base: 1e9,
            },
            UnitDef {
                names: &["hp", "horsepower"],
                to_base: 745.7,
            },
            UnitDef {
                names: &["btu_h", "btu_per_hour"],
                to_base: 0.29307,
            },
            UnitDef {
                names: &["mw_th", "milliwatt"],
                to_base: 1e-3,
            },
        ],
    },
    UnitCat {
        name: "Digital Storage",
        base: "bytes",
        units: &[
            UnitDef {
                names: &["b", "bit", "bits"],
                to_base: 0.125,
            },
            UnitDef {
                names: &["byte", "bytes"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["kb", "kilobyte", "kilobytes"],
                to_base: 1000.0,
            },
            UnitDef {
                names: &["mb", "megabyte", "megabytes"],
                to_base: 1e6,
            },
            UnitDef {
                names: &["gb", "gigabyte", "gigabytes"],
                to_base: 1e9,
            },
            UnitDef {
                names: &["tb", "terabyte", "terabytes"],
                to_base: 1e12,
            },
            UnitDef {
                names: &["pb", "petabyte", "petabytes"],
                to_base: 1e15,
            },
            UnitDef {
                names: &["kib", "kibibyte", "kibibytes"],
                to_base: 1024.0,
            },
            UnitDef {
                names: &["mib", "mebibyte", "mebibytes"],
                to_base: 1048576.0,
            },
            UnitDef {
                names: &["gib", "gibibyte", "gibibytes"],
                to_base: 1073741824.0,
            },
            UnitDef {
                names: &["tib", "tebibyte", "tebibytes"],
                to_base: 1.0995e12,
            },
            UnitDef {
                names: &["kbps", "kilobit_per_second"],
                to_base: 125.0,
            },
            UnitDef {
                names: &["mbps", "megabit_per_second"],
                to_base: 125000.0,
            },
            UnitDef {
                names: &["gbps", "gigabit_per_second"],
                to_base: 125000000.0,
            },
        ],
    },
    UnitCat {
        name: "Angle",
        base: "rad",
        units: &[
            UnitDef {
                names: &["rad", "radian", "radians"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["deg", "degree", "degrees"],
                to_base: std::f64::consts::PI / 180.0,
            },
            UnitDef {
                names: &["grad", "gradian", "gradians", "gon"],
                to_base: std::f64::consts::PI / 200.0,
            },
            UnitDef {
                names: &["rev", "revolution", "turn", "turns"],
                to_base: 2.0 * std::f64::consts::PI,
            },
            UnitDef {
                names: &["arcmin", "arcminute", "arcminutes"],
                to_base: std::f64::consts::PI / 10800.0,
            },
            UnitDef {
                names: &["arcsec", "arcsecond", "arcseconds"],
                to_base: std::f64::consts::PI / 648000.0,
            },
        ],
    },
    UnitCat {
        name: "Force",
        base: "N",
        units: &[
            UnitDef {
                names: &["n", "newton", "newtons"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["kn_f", "kilonewton", "kilonewtons"],
                to_base: 1000.0,
            },
            UnitDef {
                names: &["lbf", "pound_force", "pounds_force"],
                to_base: 4.44822,
            },
            UnitDef {
                names: &["kgf", "kilogram_force"],
                to_base: 9.80665,
            },
            UnitDef {
                names: &["dyn", "dyne", "dynes"],
                to_base: 1e-5,
            },
            UnitDef {
                names: &["gf", "gram_force"],
                to_base: 0.00980665,
            },
        ],
    },
    UnitCat {
        name: "Temperature",
        base: "K",
        units: &[
            UnitDef {
                names: &["k", "kelvin", "kelvins"],
                to_base: f64::NAN,
            },
            UnitDef {
                names: &["c", "celsius", "degc"],
                to_base: f64::NAN,
            },
            UnitDef {
                names: &["f", "fahrenheit", "degf"],
                to_base: f64::NAN,
            },
            UnitDef {
                names: &["r", "rankine", "degr"],
                to_base: f64::NAN,
            },
        ],
    },
    UnitCat {
        name: "Fuel Economy",
        base: "L/100km",
        units: &[
            UnitDef {
                names: &["l/100km", "lpkm", "liter_per_100km"],
                to_base: 1.0,
            },
            UnitDef {
                names: &["mpg", "mile_per_gallon", "miles_per_gallon"],
                to_base: f64::NAN,
            },
            UnitDef {
                names: &["km/l", "kpl", "kilometer_per_liter"],
                to_base: f64::NAN,
            },
        ],
    },
];

fn units_to_base(val: f64, from_lower: &str) -> Option<(f64, usize)> {
    for (cat_idx, cat) in UNIT_TABLE.iter().enumerate() {
        for entry in cat.units {
            if entry.names.contains(&from_lower) {
                if cat.name == "Temperature" {
                    let k = temp_to_kelvin(val, from_lower)?;
                    return Some((k, cat_idx));
                }
                if cat.name == "Fuel Economy" {
                    let l100 = fuel_to_l100(val, from_lower)?;
                    return Some((l100, cat_idx));
                }
                return Some((val * entry.to_base, cat_idx));
            }
        }
    }
    None
}

fn units_from_base(base_val: f64, to_lower: &str, cat_idx: usize) -> Option<f64> {
    let cat = &UNIT_TABLE[cat_idx];
    for entry in cat.units {
        if entry.names.contains(&to_lower) {
            if cat.name == "Temperature" {
                return kelvin_to_temp(base_val, to_lower);
            }
            if cat.name == "Fuel Economy" {
                return l100_to_fuel(base_val, to_lower);
            }
            return Some(base_val / entry.to_base);
        }
    }
    None
}

fn temp_to_kelvin(val: f64, unit: &str) -> Option<f64> {
    match unit {
        "k" | "kelvin" | "kelvins" => Some(val),
        "c" | "celsius" | "degc" => Some(val + 273.15),
        "f" | "fahrenheit" | "degf" => Some((val + 459.67) * 5.0 / 9.0),
        "r" | "rankine" | "degr" => Some(val * 5.0 / 9.0),
        _ => None,
    }
}

fn kelvin_to_temp(k: f64, unit: &str) -> Option<f64> {
    match unit {
        "k" | "kelvin" | "kelvins" => Some(k),
        "c" | "celsius" | "degc" => Some(k - 273.15),
        "f" | "fahrenheit" | "degf" => Some(k * 9.0 / 5.0 - 459.67),
        "r" | "rankine" | "degr" => Some(k * 9.0 / 5.0),
        _ => None,
    }
}

fn fuel_to_l100(val: f64, unit: &str) -> Option<f64> {
    match unit {
        "l/100km" | "lpkm" | "liter_per_100km" => Some(val),
        "mpg" | "mile_per_gallon" | "miles_per_gallon" => Some(235.215 / val),
        "km/l" | "kpl" | "kilometer_per_liter" => Some(100.0 / val),
        _ => None,
    }
}

fn l100_to_fuel(l100: f64, unit: &str) -> Option<f64> {
    match unit {
        "l/100km" | "lpkm" | "liter_per_100km" => Some(l100),
        "mpg" | "mile_per_gallon" | "miles_per_gallon" => Some(235.215 / l100),
        "km/l" | "kpl" | "kilometer_per_liter" => Some(100.0 / l100),
        _ => None,
    }
}

fn find_unit_category(unit_lower: &str) -> Option<usize> {
    for (i, cat) in UNIT_TABLE.iter().enumerate() {
        for entry in cat.units {
            if entry.names.contains(&unit_lower) {
                return Some(i);
            }
        }
    }
    None
}

fn fmt_unit_val(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let abs = v.abs();
    if !(1e-4..1e9).contains(&abs) {
        format!("{:.6e}", v)
    } else if abs >= 1000.0 {
        format!("{:.4}", v)
    } else {
        format!("{:.8}", v)
    }
}

pub fn units_calc(query: &str) -> String {
    let mut out = String::new();
    let sep = "═".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  UNIT CONVERSION");
    let _ = writeln!(out, "{}", sep);

    // Parse: "VALUE FROM [to|in|->] TO"  or  "VALUE FROM"  (broadcast)
    let q = query.trim();
    let tokens: Vec<&str> = q.split_whitespace().collect();

    if tokens.is_empty() {
        let _ = writeln!(out, "{}", units_usage());
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // First token must be a number
    let val = match tokens[0].replace(',', "").parse::<f64>() {
        Ok(v) => v,
        Err(_) => {
            // maybe "list CATEGORY"
            if tokens[0].to_lowercase() == "list" {
                let cat_query = tokens[1..].join(" ").to_lowercase();
                for cat in UNIT_TABLE {
                    if cat_query.is_empty() || cat.name.to_lowercase().contains(&cat_query) {
                        let _ = writeln!(out, "  {}  (base: {})", cat.name, cat.base);
                        for entry in cat.units {
                            let _ = writeln!(out, "    {}", entry.names[0]);
                        }
                        let _ = writeln!(out);
                    }
                }
                let _ = writeln!(out, "{}", sep);
                return out;
            }
            let _ = writeln!(
                out,
                "  ERROR: first token must be a number. Got '{}'",
                tokens[0]
            );
            let _ = writeln!(out, "{}", units_usage());
            let _ = writeln!(out, "{}", sep);
            return out;
        }
    };

    if tokens.len() < 2 {
        let _ = writeln!(out, "  ERROR: need VALUE UNIT  e.g.  5 km");
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    let from_raw = tokens[1].to_lowercase();

    // Skip connector tokens: "to", "in", "->", "as"
    let connector_set = ["to", "in", "->", "as", "into"];
    let to_raw: Option<String> = tokens[2..]
        .iter()
        .find(|t| !connector_set.contains(&t.to_lowercase().as_str()))
        .map(|t| t.to_lowercase());

    // Look up source unit
    let (base_val, cat_idx) = match units_to_base(val, &from_raw) {
        Some(v) => v,
        None => {
            let _ = writeln!(out, "  ERROR: unknown unit '{}'.", tokens[1]);
            let _ = writeln!(out, "  Tip: run  hematite --units list  to see all units.");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
    };

    let cat = &UNIT_TABLE[cat_idx];

    if let Some(to_unit) = to_raw {
        // Single target conversion
        match units_from_base(base_val, &to_unit, cat_idx) {
            Some(result) => {
                let _ = writeln!(
                    out,
                    "  {} {} = {} {}",
                    val,
                    tokens[1],
                    fmt_unit_val(result),
                    &to_unit
                );
            }
            None => {
                // Check if to_unit exists in a different category
                if let Some(other_idx) = find_unit_category(&to_unit) {
                    let _ = writeln!(
                        out,
                        "  ERROR: '{}' is a {} unit, but '{}' is {}.",
                        &to_unit, UNIT_TABLE[other_idx].name, tokens[1], cat.name
                    );
                } else {
                    let _ = writeln!(out, "  ERROR: unknown target unit '{}'.", &to_unit);
                }
            }
        }
    } else {
        // Broadcast: convert to all units in the category
        let _ = writeln!(out, "  {} {}  =", val, tokens[1]);
        let _ = writeln!(out, "  {:>30}  unit", "value");
        let _ = writeln!(out, "  {}", "-".repeat(42));
        for entry in cat.units {
            let to_name = entry.names[0];
            if to_name == from_raw {
                continue;
            }
            if let Some(result) = units_from_base(base_val, to_name, cat_idx) {
                let abs = result.abs();
                // skip values that are so tiny or huge they're not useful
                if abs > 0.0 && !(1e-30..=1e30).contains(&abs) {
                    continue;
                }
                let _ = writeln!(out, "  {:>30}  {}", fmt_unit_val(result), to_name);
            }
        }
    }

    let _ = writeln!(out, "{}", sep);
    out
}

fn units_usage() -> &'static str {
    "Usage:\n\
     hematite --units '100 km to miles'       convert single value\n\
     hematite --units '32 f to c'             temperature: 32°F → °C\n\
     hematite --units '5 kg'                  broadcast: kg to all mass units\n\
     hematite --units '1 kwh to j'            energy: kWh → Joules\n\
     hematite --units '1 gbps to mbps'        digital storage speed\n\
     hematite --units '2.5 atm to psi'        pressure\n\
     hematite --units 'list length'           list all length units\n\
     hematite --units 'list'                  list all categories\n\
     \n\
     Categories: Length Area Volume Mass Time Speed Pressure Energy\n\
                 Power Digital-Storage Angle Force Temperature Fuel-Economy"
}

// ── ODE solver ────────────────────────────────────────────────────────────────
// Solves first-order ODEs and systems using Euler, RK4, or adaptive RK45.
// Equations are parsed as simple expressions using the symbolic engine.
// Supports preset equations for well-known models.

// Evaluate a simple 1-variable function f(t, y) from a string.
// Handles: constants, t, y, arithmetic +−*/^, sin/cos/exp/ln/sqrt.
fn ode_eval_expr(expr_str: &str, t: f64, y: f64) -> f64 {
    let substituted = expr_str
        .replace("exp(", "\x00EXP\x00(")
        .replace('y', &format!("({:.17e})", y))
        .replace('t', &format!("({:.17e})", t))
        .replace("\x00EXP\x00(", "exp(");
    let expr = match parse_sym(&substituted) {
        Ok(e) => e,
        Err(_) => return f64::NAN,
    };
    eval_expr(&expr, "x", 0.0).unwrap_or(f64::NAN)
}

// RK4 step for scalar ODE dy/dt = f(t, y)
fn rk4_step(f: &dyn Fn(f64, f64) -> f64, t: f64, y: f64, h: f64) -> f64 {
    let k1 = f(t, y);
    let k2 = f(t + h / 2.0, y + h * k1 / 2.0);
    let k3 = f(t + h / 2.0, y + h * k2 / 2.0);
    let k4 = f(t + h, y + h * k3);
    y + h * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0
}

// Euler step
fn euler_step(f: &dyn Fn(f64, f64) -> f64, t: f64, y: f64, h: f64) -> f64 {
    y + h * f(t, y)
}

// Adaptive RK45 (Dormand-Prince) — returns (y_new, error_estimate, h_used)
fn rk45_step(f: &dyn Fn(f64, f64) -> f64, t: f64, y: f64, h: f64) -> (f64, f64) {
    // Dormand-Prince coefficients
    let k1 = f(t, y);
    let k2 = f(t + h / 5.0, y + h * (k1 / 5.0));
    let k3 = f(
        t + 3.0 * h / 10.0,
        y + h * (3.0 * k1 / 40.0 + 9.0 * k2 / 40.0),
    );
    let k4 = f(
        t + 4.0 * h / 5.0,
        y + h * (44.0 * k1 / 45.0 - 56.0 * k2 / 15.0 + 32.0 * k3 / 9.0),
    );
    let k5 = f(
        t + 8.0 * h / 9.0,
        y + h
            * (19372.0 * k1 / 6561.0 - 25360.0 * k2 / 2187.0 + 64448.0 * k3 / 6561.0
                - 212.0 * k4 / 729.0),
    );
    let k6 = f(
        t + h,
        y + h
            * (9017.0 * k1 / 3168.0 - 355.0 * k2 / 33.0
                + 46732.0 * k3 / 5247.0
                + 49.0 * k4 / 176.0
                - 5103.0 * k5 / 18656.0),
    );
    // 5th order solution
    let y5 = y + h
        * (35.0 * k1 / 384.0 + 500.0 * k3 / 1113.0 + 125.0 * k4 / 192.0 - 2187.0 * k5 / 6784.0
            + 11.0 * k6 / 84.0);
    // 4th order for error estimate
    let y4 = y + h
        * (5179.0 * k1 / 57600.0 + 7571.0 * k3 / 16695.0 + 393.0 * k4 / 640.0
            - 92097.0 * k5 / 339200.0
            + 187.0 * k6 / 2100.0
            + k6 / 40.0);
    let err = (y5 - y4).abs();
    (y5, err)
}

// 2D system RK4: dy1/dt = f1(t,y1,y2), dy2/dt = f2(t,y1,y2)
fn rk4_system_step(
    f1: &dyn Fn(f64, f64, f64) -> f64,
    f2: &dyn Fn(f64, f64, f64) -> f64,
    t: f64,
    y1: f64,
    y2: f64,
    h: f64,
) -> (f64, f64) {
    let k1a = f1(t, y1, y2);
    let k1b = f2(t, y1, y2);
    let k2a = f1(t + h / 2.0, y1 + h * k1a / 2.0, y2 + h * k1b / 2.0);
    let k2b = f2(t + h / 2.0, y1 + h * k1a / 2.0, y2 + h * k1b / 2.0);
    let k3a = f1(t + h / 2.0, y1 + h * k2a / 2.0, y2 + h * k2b / 2.0);
    let k3b = f2(t + h / 2.0, y1 + h * k2a / 2.0, y2 + h * k2b / 2.0);
    let k4a = f1(t + h, y1 + h * k3a, y2 + h * k3b);
    let k4b = f2(t + h, y1 + h * k3a, y2 + h * k3b);
    (
        y1 + h * (k1a + 2.0 * k2a + 2.0 * k3a + k4a) / 6.0,
        y2 + h * (k1b + 2.0 * k2b + 2.0 * k3b + k4b) / 6.0,
    )
}

fn ode_ascii_plot(ts: &[f64], ys: &[f64], width: usize, height: usize) -> String {
    if ts.is_empty() || ys.is_empty() {
        return String::new();
    }
    let mn = ys.iter().cloned().fold(f64::INFINITY, f64::min);
    let mx = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let yrange = (mx - mn).max(1e-12);
    let tmin = ts[0];
    let tmax = ts[ts.len() - 1];
    let trange = (tmax - tmin).max(1e-12);
    let mut grid = vec![vec![' '; width]; height];
    for (i, (&t, &y)) in ts.iter().zip(ys.iter()).enumerate() {
        let _ = i;
        let col = ((t - tmin) / trange * (width - 1) as f64).round() as usize;
        let row = height - 1 - ((y - mn) / yrange * (height - 1) as f64).round() as usize;
        let col = col.min(width - 1);
        let row = row.min(height - 1);
        grid[row][col] = '·';
    }
    let result = grid
        .iter()
        .map(|r| r.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n  y: [{:.4} .. {:.4}]\n  t: [{:.4} .. {:.4}]",
        result, mn, mx, tmin, tmax
    )
}

pub fn ode_solve(query: &str) -> String {
    let mut out = String::new();
    let sep = "═".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  ODE SOLVER");
    let _ = writeln!(out, "{}", sep);

    let q = query.trim().to_lowercase();

    // --- Preset models ---
    if q.starts_with("logistic") || q.contains("logistic growth") {
        // dy/dt = r*y*(1 - y/K)
        let tokens: Vec<&str> = query.split_whitespace().collect();
        let r: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "r")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(1.0);
        let k: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "k")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(100.0);
        let y0: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "y0")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(10.0);
        let t_end: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "t")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(10.0);
        let n = 200_usize;
        let h = t_end / n as f64;
        let _ = writeln!(out, "  Logistic Growth:  dy/dt = r·y·(1 - y/K)");
        let _ = writeln!(
            out,
            "  r={:.4}  K={:.4}  y0={:.4}  t=[0,{:.4}]",
            r, k, y0, t_end
        );
        let _ = writeln!(out);
        let f = |_t: f64, y: f64| r * y * (1.0 - y / k);
        let mut t = 0.0_f64;
        let mut y = y0;
        let mut ts = vec![t];
        let mut ys = vec![y];
        for _ in 0..n {
            y = rk4_step(&f, t, y, h);
            t += h;
            ts.push(t);
            ys.push(y);
        }
        let analytical_k = k / (1.0 + (k / y0 - 1.0) * (-r * t_end).exp());
        let _ = writeln!(
            out,
            "  y(t_end) = {:.6}  [analytical: {:.6}]",
            ys[ys.len() - 1],
            analytical_k
        );
        let plot = ode_ascii_plot(&ts, &ys, 56, 10);
        let _ = writeln!(out);
        for line in plot.lines() {
            let _ = writeln!(out, "  {}", line);
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    if q.starts_with("exponential") || q.starts_with("decay") || q.starts_with("growth") {
        let tokens: Vec<&str> = query.split_whitespace().collect();
        let r: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "r")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(1.0);
        let y0: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "y0")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(1.0);
        let t_end: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "t")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(5.0);
        let n = 200_usize;
        let h = t_end / n as f64;
        let _ = writeln!(out, "  Exponential:  dy/dt = r·y");
        let _ = writeln!(out, "  r={:.4}  y0={:.4}  t=[0,{:.4}]", r, y0, t_end);
        let f = |_t: f64, y: f64| r * y;
        let mut t = 0.0_f64;
        let mut y = y0;
        let mut ts = vec![t];
        let mut ys = vec![y];
        for _ in 0..n {
            y = rk4_step(&f, t, y, h);
            t += h;
            ts.push(t);
            ys.push(y);
        }
        let analytical = y0 * (r * t_end).exp();
        let _ = writeln!(
            out,
            "  y(t_end) = {:.6}  [analytical: {:.6}]",
            ys[ys.len() - 1],
            analytical
        );
        let plot = ode_ascii_plot(&ts, &ys, 56, 10);
        let _ = writeln!(out);
        for line in plot.lines() {
            let _ = writeln!(out, "  {}", line);
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    if q.starts_with("lotka") || q.contains("predator") || q.contains("prey") {
        // Lotka-Volterra: dx/dt = α*x - β*x*y,  dy/dt = δ*x*y - γ*y
        let tokens: Vec<&str> = query.split_whitespace().collect();
        let alpha: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "alpha")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(1.0);
        let beta: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "beta")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(0.1);
        let delta: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "delta")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(0.075);
        let gamma: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "gamma")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(1.5);
        let x0: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "x0")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(10.0);
        let y0: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "y0")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(5.0);
        let t_end: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "t")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(30.0);
        let n = 2000_usize;
        let h = t_end / n as f64;
        let _ = writeln!(out, "  Lotka-Volterra (predator-prey)");
        let _ = writeln!(out, "  dx/dt = {:.3}x - {:.3}xy  (prey)", alpha, beta);
        let _ = writeln!(out, "  dy/dt = {:.3}xy - {:.3}y  (predator)", delta, gamma);
        let _ = writeln!(out, "  x0={:.2}  y0={:.2}  t=[0,{:.1}]", x0, y0, t_end);
        let f1 = |_t: f64, x: f64, y: f64| alpha * x - beta * x * y;
        let f2 = |_t: f64, x: f64, y: f64| delta * x * y - gamma * y;
        let mut t = 0.0_f64;
        let mut x = x0;
        let mut y = y0;
        let mut ts = vec![t];
        let mut xs = vec![x];
        let mut ys = vec![y];
        for _ in 0..n {
            let (nx, ny) = rk4_system_step(&f1, &f2, t, x, y, h);
            x = nx;
            y = ny;
            t += h;
            ts.push(t);
            xs.push(x);
            ys.push(y);
        }
        let _ = writeln!(
            out,
            "  x(t_end)={:.4}  y(t_end)={:.4}",
            xs[xs.len() - 1],
            ys[ys.len() - 1]
        );
        let _ = writeln!(out, "  Prey trajectory:");
        let plot = ode_ascii_plot(&ts, &xs, 56, 8);
        for line in plot.lines() {
            let _ = writeln!(out, "  {}", line);
        }
        let _ = writeln!(out, "  Predator trajectory:");
        let plot2 = ode_ascii_plot(&ts, &ys, 56, 8);
        for line in plot2.lines() {
            let _ = writeln!(out, "  {}", line);
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    if q.starts_with("sir") || q.contains("susceptible") || q.contains("epidemic") {
        // SIR model: dS/dt=-β*S*I/N, dI/dt=β*S*I/N - γ*I, dR/dt=γ*I
        let tokens: Vec<&str> = query.split_whitespace().collect();
        let n_pop: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "n")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(1000.0);
        let beta: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "beta")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(0.3);
        let gamma: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "gamma")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(0.05);
        let i0: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "i0")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(1.0);
        let t_end: f64 = tokens
            .iter()
            .position(|&t| t.to_lowercase() == "t")
            .and_then(|i| tokens.get(i + 1)?.parse().ok())
            .unwrap_or(200.0);
        let n_steps = 2000_usize;
        let h = t_end / n_steps as f64;
        let r0 = beta / gamma;
        let _ = writeln!(
            out,
            "  SIR Epidemic Model  (N={}, β={:.4}, γ={:.4}, R₀={:.2})",
            n_pop, beta, gamma, r0
        );
        let _ = writeln!(
            out,
            "  dS/dt = -β·S·I/N   dI/dt = β·S·I/N - γ·I   dR/dt = γ·I"
        );
        let mut s = n_pop - i0;
        let mut inf = i0;
        let mut r = 0.0_f64;
        let mut t = 0.0_f64;
        let mut ts = vec![t];
        let mut is = vec![inf];
        let mut peak_i = i0;
        let mut peak_t = 0.0_f64;
        for _ in 0..n_steps {
            let ds = -beta * s * inf / n_pop;
            let di = beta * s * inf / n_pop - gamma * inf;
            let dr = gamma * inf;
            s += h * ds;
            inf += h * di;
            r += h * dr;
            t += h;
            if inf > peak_i {
                peak_i = inf;
                peak_t = t;
            }
            ts.push(t);
            is.push(inf);
        }
        let _ = writeln!(out, "  Peak infected: {:.1} at t={:.1}", peak_i, peak_t);
        let _ = writeln!(out, "  Final: S={:.1}  I={:.1}  R={:.1}", s, inf, r);
        let plot = ode_ascii_plot(&ts, &is, 56, 10);
        for line in plot.lines() {
            let _ = writeln!(out, "  {}", line);
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // --- Generic: dy/dt = EXPR  y0=VALUE  t=END  [n=STEPS] [method=euler|rk4|rk45] ---
    let tokens: Vec<&str> = query.split_whitespace().collect();

    // Parse: "dy/dt = EXPR" form or "EXPR y0=V t=END"
    let eq_str: String = if let Some(eq_pos) = query.find('=') {
        // Could be "dy/dt = ..." — find the first '=' and take what's after
        // But also y0= or t= have = so find the equation part
        let before = &query[..eq_pos].trim().to_lowercase();
        if before.ends_with("dy/dt") || before.ends_with("f") || before.ends_with("dydt") {
            query[eq_pos + 1..]
                .split_whitespace()
                .take_while(|t| {
                    !t.to_lowercase().starts_with("y0=")
                        && !t.to_lowercase().starts_with("t=")
                        && !t.to_lowercase().starts_with("n=")
                        && !t.to_lowercase().starts_with("method=")
                })
                .collect::<Vec<_>>()
                .join("")
        } else {
            tokens[0].to_string()
        }
    } else {
        tokens[0].to_string()
    };

    // Extract named params
    let get_param = |key: &str| -> Option<f64> {
        query
            .split_whitespace()
            .find(|t| t.to_lowercase().starts_with(&format!("{}=", key)))
            .and_then(|t| t.split_once('=')?.1.parse().ok())
    };
    let get_str_param = |key: &str| -> Option<String> {
        query
            .split_whitespace()
            .find(|t| t.to_lowercase().starts_with(&format!("{}=", key)))
            .and_then(|t| t.split_once('=').map(|x| x.1).map(|s| s.to_lowercase()))
    };

    let y0 = get_param("y0").unwrap_or(1.0);
    let t_end = get_param("t")
        .or_else(|| get_param("tend"))
        .or_else(|| get_param("t_end"))
        .unwrap_or(10.0);
    let n = get_param("n")
        .map(|v| v as usize)
        .unwrap_or(100)
        .clamp(4, 10000);
    let method = get_str_param("method").unwrap_or_else(|| "rk4".into());

    if eq_str.is_empty() || eq_str == "0" {
        let _ = writeln!(out, "{}", ode_usage());
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    let eq = eq_str.clone();
    let f = move |t: f64, y: f64| ode_eval_expr(&eq, t, y);
    let h = t_end / n as f64;

    let _ = writeln!(
        out,
        "  dy/dt = {}   y0={}   t=[0,{}]   n={}   method={}",
        eq_str, y0, t_end, n, method
    );
    let _ = writeln!(out);

    let (ts, ys): (Vec<f64>, Vec<f64>) = match method.as_str() {
        "euler" => {
            let mut t = 0.0_f64;
            let mut y = y0;
            let mut ts = vec![t];
            let mut ys = vec![y];
            for _ in 0..n {
                y = euler_step(&f, t, y, h);
                t += h;
                ts.push(t);
                ys.push(y);
            }
            (ts, ys)
        }
        "rk45" | "adaptive" => {
            let mut t = 0.0_f64;
            let mut y = y0;
            let mut ts = vec![t];
            let mut ys = vec![y];
            let mut h_cur = h;
            let tol = 1e-6_f64;
            let mut steps = 0_usize;
            while t < t_end && steps < 100_000 {
                let (y_new, err) = rk45_step(&f, t, y, h_cur);
                if err < tol || h_cur < 1e-10 {
                    t += h_cur;
                    y = y_new;
                    ts.push(t);
                    ys.push(y);
                    steps += 1;
                }
                let scale = if err > 0.0 {
                    0.9 * (tol / err).powf(0.2)
                } else {
                    2.0
                };
                h_cur = (h_cur * scale.clamp(0.1, 5.0)).min(t_end - t + 1e-15);
            }
            (ts, ys)
        }
        _ => {
            let mut t = 0.0_f64;
            let mut y = y0;
            let mut ts = vec![t];
            let mut ys = vec![y];
            for _ in 0..n {
                y = rk4_step(&f, t, y, h);
                t += h;
                ts.push(t);
                ys.push(y);
            }
            (ts, ys)
        }
    };

    // Print table (max 20 rows)
    let step = (ts.len() / 20).max(1);
    let _ = writeln!(out, "  {:>10}  {:>16}", "t", "y");
    let _ = writeln!(out, "  {}", "-".repeat(28));
    for (i, (&t, &y)) in ts.iter().zip(ys.iter()).enumerate() {
        if i % step == 0 || i == ts.len() - 1 {
            let _ = writeln!(out, "  {:>10.6}  {:>16.10}", t, y);
        }
    }
    let _ = writeln!(out);

    // Plot
    let plot = ode_ascii_plot(&ts, &ys, 56, 10);
    for line in plot.lines() {
        let _ = writeln!(out, "  {}", line);
    }

    let _ = writeln!(out, "{}", sep);
    out
}

fn ode_usage() -> &'static str {
    "ODE solver — no model, no cloud:\n\
     \n\
     Generic:\n\
     hematite --ode 'dy/dt = -y  y0=1  t=5'          exponential decay\n\
     hematite --ode 'dy/dt = t*y  y0=1  t=3  n=50'   custom ODE\n\
     hematite --ode 'dy/dt = sin(t)-y  y0=0  t=20 method=rk45'  adaptive\n\
     hematite --ode 'dy/dt = r*y  y0=0.5  t=4 method=euler'\n\
     \n\
     Preset models:\n\
     hematite --ode 'exponential r=0.5 y0=1 t=5'\n\
     hematite --ode 'logistic r=1 K=100 y0=5 t=10'\n\
     hematite --ode 'lotka alpha=1 beta=0.1 delta=0.075 gamma=1.5 x0=10 y0=5 t=30'\n\
     hematite --ode 'sir N=10000 beta=0.3 gamma=0.05 i0=1 t=200'\n\
     \n\
     Methods: euler  rk4 (default)  rk45 (adaptive)\n\
     Params:  y0=INITIAL  t=TEND  n=STEPS  method=NAME"
}

// ── Numerical optimization ─────────────────────────────────────────────────────
// Golden-section search (1D), gradient descent (nD), Nelder-Mead simplex (2D).
// Evaluates arbitrary expressions via the symbolic engine.

fn opt_eval(expr_str: &str, x: f64, y: f64) -> f64 {
    let s = expr_str
        .replace("exp(", "\x00E\x00(")
        .replace('y', &format!("({:.17e})", y))
        .replace('x', &format!("({:.17e})", x))
        .replace("\x00E\x00(", "exp(");
    match parse_sym(&s) {
        Ok(e) => eval_expr(&e, "z", 0.0).unwrap_or(f64::NAN),
        Err(_) => f64::NAN,
    }
}

// Golden-section search: minimize f over [a, b]
fn golden_section(
    f: &dyn Fn(f64) -> f64,
    mut a: f64,
    mut b: f64,
    tol: f64,
    max_iter: usize,
) -> (f64, f64, usize) {
    let phi = (5.0_f64.sqrt() - 1.0) / 2.0;
    let mut c = b - phi * (b - a);
    let mut d = a + phi * (b - a);
    let mut fc = f(c);
    let mut fd = f(d);
    let mut iters = 0;
    while (b - a).abs() > tol && iters < max_iter {
        if fc < fd {
            b = d;
            d = c;
            fd = fc;
            c = b - phi * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + phi * (b - a);
            fd = f(d);
        }
        iters += 1;
    }
    let x_min = (a + b) / 2.0;
    (x_min, f(x_min), iters)
}

// Finite-difference gradient
fn grad(f: &dyn Fn(&[f64]) -> f64, x: &[f64], h: f64) -> Vec<f64> {
    x.iter()
        .enumerate()
        .map(|(i, _)| {
            let mut xp = x.to_vec();
            xp[i] += h;
            let mut xm = x.to_vec();
            xm[i] -= h;
            (f(&xp) - f(&xm)) / (2.0 * h)
        })
        .collect()
}

// Gradient descent with backtracking line search
fn gradient_descent(
    f: &dyn Fn(&[f64]) -> f64,
    x0: &[f64],
    max_iter: usize,
    tol: f64,
) -> (Vec<f64>, f64, usize) {
    let mut x = x0.to_vec();
    let mut iters = 0;
    let mut alpha = 0.1_f64;
    for _ in 0..max_iter {
        let g = grad(f, &x, 1e-6);
        let gnorm = g.iter().map(|v| v * v).sum::<f64>().sqrt();
        if gnorm < tol {
            break;
        }
        // backtracking
        let fx = f(&x);
        let mut step = alpha;
        let mut x_new = x
            .iter()
            .zip(g.iter())
            .map(|(&xi, &gi)| xi - step * gi)
            .collect::<Vec<_>>();
        let mut tries = 0;
        while f(&x_new) > fx - 0.5 * step * gnorm * gnorm && tries < 30 {
            step *= 0.5;
            tries += 1;
            x_new = x
                .iter()
                .zip(g.iter())
                .map(|(&xi, &gi)| xi - step * gi)
                .collect();
        }
        alpha = step;
        x = x_new;
        iters += 1;
    }
    let fval = f(&x);
    (x, fval, iters)
}

// Nelder-Mead simplex for 2D
fn nelder_mead(
    f: &dyn Fn(f64, f64) -> f64,
    x0: f64,
    y0: f64,
    max_iter: usize,
    tol: f64,
) -> (f64, f64, f64, usize) {
    let g = |p: &[f64; 2]| f(p[0], p[1]);
    let mut s = [[x0, y0], [x0 + 1.0, y0], [x0, y0 + 1.0]];
    let mut iters = 0;
    loop {
        // sort by function value
        let mut fs: [(f64, usize); 3] = [(g(&s[0]), 0), (g(&s[1]), 1), (g(&s[2]), 2)];
        fs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let (_, i_best) = fs[0];
        let (_, i_worst) = fs[2];
        let (_, i_2nd) = fs[1];
        // convergence
        let spread = (fs[2].0 - fs[0].0).abs();
        if spread < tol || iters >= max_iter {
            break;
        }
        // centroid of best two
        let cx = (s[i_best][0] + s[i_2nd][0]) / 2.0;
        let cy = (s[i_best][1] + s[i_2nd][1]) / 2.0;
        // reflect
        let rx = 2.0 * cx - s[i_worst][0];
        let ry = 2.0 * cy - s[i_worst][1];
        let fr = g(&[rx, ry]);
        if fr < fs[0].0 {
            // expand
            let ex = 3.0 * cx - 2.0 * s[i_worst][0];
            let ey = 3.0 * cy - 2.0 * s[i_worst][1];
            if g(&[ex, ey]) < fr {
                s[i_worst] = [ex, ey];
            } else {
                s[i_worst] = [rx, ry];
            }
        } else if fr < fs[1].0 {
            s[i_worst] = [rx, ry];
        } else {
            // contract
            let kx = 0.5 * (cx + s[i_worst][0]);
            let ky = 0.5 * (cy + s[i_worst][1]);
            if g(&[kx, ky]) < fs[2].0 {
                s[i_worst] = [kx, ky];
            } else {
                // shrink
                for j in 1..3 {
                    let idx = fs[j].1;
                    s[idx] = [
                        (s[i_best][0] + s[idx][0]) / 2.0,
                        (s[i_best][1] + s[idx][1]) / 2.0,
                    ];
                }
            }
        }
        iters += 1;
    }
    let mut fs: [(f64, usize); 3] = [(g(&s[0]), 0), (g(&s[1]), 1), (g(&s[2]), 2)];
    fs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let best = s[fs[0].1];
    (best[0], best[1], fs[0].0, iters)
}

pub fn optimize_calc(query: &str) -> String {
    let mut out = String::new();
    let sep = "═".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  NUMERICAL OPTIMIZATION");
    let _ = writeln!(out, "{}", sep);

    let tokens: Vec<&str> = query.split_whitespace().collect();
    if tokens.is_empty() {
        let _ = writeln!(out, "{}", optimize_usage());
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    let get_param = |key: &str| -> Option<f64> {
        query
            .split_whitespace()
            .find(|t| t.to_lowercase().starts_with(&format!("{}=", key)))
            .and_then(|t| t.split_once('=')?.1.parse().ok())
    };
    let mode = tokens[0].to_lowercase();

    match mode.as_str() {
        // --- 1D minimize/maximize ---
        "min" | "minimize" | "max" | "maximize" | "find-min" | "find-max" => {
            let maximize = mode == "max" || mode == "maximize" || mode == "find-max";
            let f_str = tokens.get(1).copied().unwrap_or("x^2");
            let a = get_param("a")
                .or_else(|| get_param("from"))
                .unwrap_or(-10.0);
            let b = get_param("b").or_else(|| get_param("to")).unwrap_or(10.0);
            let tol = get_param("tol").unwrap_or(1e-8);
            let max_iter = get_param("iter").map(|v| v as usize).unwrap_or(500);

            let sign = if maximize { -1.0_f64 } else { 1.0_f64 };
            let f = |x: f64| sign * opt_eval(f_str, x, 0.0);
            let (x_opt, f_opt_signed, iters) = golden_section(&f, a, b, tol, max_iter);
            let f_opt = if maximize {
                -f_opt_signed
            } else {
                f_opt_signed
            };

            let _ = writeln!(
                out,
                "  {} f(x) = {}   x ∈ [{}, {}]",
                if maximize { "Maximize" } else { "Minimize" },
                f_str,
                a,
                b
            );
            let _ = writeln!(out, "  Converged in {} iterations (tol={:.2e})", iters, tol);
            let _ = writeln!(out);
            let _ = writeln!(out, "  x* = {:.10}  f(x*) = {:.10}", x_opt, f_opt);
            let _ = writeln!(out);

            // ASCII plot
            let n_plot = 56_usize;
            let ys: Vec<f64> = (0..n_plot)
                .map(|i| {
                    let x = a + i as f64 * (b - a) / (n_plot - 1) as f64;
                    opt_eval(f_str, x, 0.0)
                })
                .collect();
            let ymin = ys.iter().cloned().fold(f64::INFINITY, f64::min);
            let ymax = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let yrange = (ymax - ymin).max(1e-12);
            let h = 10_usize;
            let mut grid = vec![vec![' '; n_plot]; h];
            for (col, &y) in ys.iter().enumerate() {
                let row = h - 1 - ((y - ymin) / yrange * (h - 1) as f64).round() as usize;
                let row = row.min(h - 1);
                grid[row][col] = '·';
            }
            // mark x*
            let opt_col = ((x_opt - a) / (b - a) * (n_plot - 1) as f64).round() as usize;
            let opt_row = h - 1 - ((f_opt - ymin) / yrange * (h - 1) as f64).round() as usize;
            if opt_col < n_plot && opt_row < h {
                grid[opt_row][opt_col] = '★';
            }
            for row in &grid {
                let _ = writeln!(out, "  |{}|", row.iter().collect::<String>());
            }
            let _ = writeln!(
                out,
                "  f: [{:.4} .. {:.4}]   x: [{:.4} .. {:.4}]   ★=optimum",
                ymin, ymax, a, b
            );
        }

        // --- 2D Nelder-Mead ---
        "min2" | "minimize2" | "max2" | "maximize2" | "simplex" => {
            let maximize = mode.starts_with("max");
            let f_str = tokens.get(1).copied().unwrap_or("x^2+y^2");
            let x0 = get_param("x0").unwrap_or(0.0);
            let y0 = get_param("y0").unwrap_or(0.0);
            let max_iter = get_param("iter").map(|v| v as usize).unwrap_or(1000);
            let tol = get_param("tol").unwrap_or(1e-8);
            let sign = if maximize { -1.0_f64 } else { 1.0_f64 };
            let f = |x: f64, y: f64| sign * opt_eval(f_str, x, y);
            let (xopt, yopt, fval_s, iters) = nelder_mead(&f, x0, y0, max_iter, tol);
            let fval = if maximize { -fval_s } else { fval_s };
            let _ = writeln!(
                out,
                "  {} f(x,y) = {}   start: ({}, {})",
                if maximize { "Maximize" } else { "Minimize" },
                f_str,
                x0,
                y0
            );
            let _ = writeln!(out, "  Nelder-Mead simplex  iterations={}", iters);
            let _ = writeln!(out);
            let _ = writeln!(out, "  x* = {:.10}", xopt);
            let _ = writeln!(out, "  y* = {:.10}", yopt);
            let _ = writeln!(out, "  f(x*,y*) = {:.10}", fval);
        }

        // --- nD gradient descent ---
        "gradient" | "gd" | "grad-descent" => {
            let f_str = tokens.get(1).copied().unwrap_or("x^2");
            let x0_str = tokens.get(2).copied().unwrap_or("0");
            let x0: Vec<f64> = x0_str.split(',').filter_map(|s| s.parse().ok()).collect();
            let x0 = if x0.is_empty() { vec![0.0] } else { x0 };
            let max_iter = get_param("iter").map(|v| v as usize).unwrap_or(2000);
            let tol = get_param("tol").unwrap_or(1e-7);
            let f = |v: &[f64]| opt_eval(f_str, v[0], v.get(1).copied().unwrap_or(0.0));
            let (x_opt, f_opt, iters) = gradient_descent(&f, &x0, max_iter, tol);
            let _ = writeln!(out, "  Gradient Descent  f = {}   x0={:?}", f_str, x0);
            let _ = writeln!(out, "  Iterations: {}", iters);
            let _ = writeln!(out);
            let _ = writeln!(
                out,
                "  x* = {:?}",
                x_opt
                    .iter()
                    .map(|v| format!("{:.10}", v))
                    .collect::<Vec<_>>()
            );
            let _ = writeln!(out, "  f(x*) = {:.10}", f_opt);
        }

        // --- Root finding (bisection) ---
        "root" | "find-root" | "zero" => {
            let f_str = tokens.get(1).copied().unwrap_or("x^2-2");
            let a = get_param("a")
                .or_else(|| get_param("from"))
                .unwrap_or(-10.0);
            let b = get_param("b").or_else(|| get_param("to")).unwrap_or(10.0);
            let tol = get_param("tol").unwrap_or(1e-10);
            let max_iter = get_param("iter").map(|v| v as usize).unwrap_or(200);
            let f = |x: f64| opt_eval(f_str, x, 0.0);
            let fa = f(a);
            let fb = f(b);
            if fa * fb > 0.0 {
                let _ = writeln!(out, "  Root finding: f(x) = {}   [{}, {}]", f_str, a, b);
                let _ = writeln!(
                    out,
                    "  ERROR: f(a) and f(b) must have opposite signs for bisection."
                );
                let _ = writeln!(out, "  f({}) = {:.6}  f({}) = {:.6}", a, fa, b, fb);
            } else {
                let mut lo = a;
                let mut hi = b;
                let mut iters = 0;
                while (hi - lo).abs() > tol && iters < max_iter {
                    let mid = (lo + hi) / 2.0;
                    if f(lo) * f(mid) <= 0.0 {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                    iters += 1;
                }
                let root = (lo + hi) / 2.0;
                let _ = writeln!(out, "  Root finding: f(x) = {}   [{}, {}]", f_str, a, b);
                let _ = writeln!(out, "  Bisection: {} iterations (tol={:.2e})", iters, tol);
                let _ = writeln!(out);
                let _ = writeln!(out, "  x* = {:.12}  f(x*) = {:.3e}", root, f(root));
            }
        }

        _ => {
            let _ = writeln!(out, "{}", optimize_usage());
        }
    }

    let _ = writeln!(out, "{}", sep);
    out
}

fn optimize_usage() -> &'static str {
    "Numerical optimization — no model, no cloud:\n\
     \n\
     hematite --optimize 'min x^2-4*x+3 a=0 b=5'       minimize 1D\n\
     hematite --optimize 'max sin(x) a=0 b=6.28'        maximize 1D\n\
     hematite --optimize 'min2 x^2+y^2 x0=3 y0=2'       minimize 2D (Nelder-Mead)\n\
     hematite --optimize 'gradient x^4-4*x^2 0'         gradient descent\n\
     hematite --optimize 'root x^3-2 a=0 b=2'           find root (bisection)\n\
     hematite --optimize 'min (x-2)^2+(x-3)^2 a=-5 b=8' least-squares style\n\
     \n\
     Parameters:  a=  b=  range bounds  x0=  y0=  start point\n\
                  tol=  tolerance (default 1e-8)\n\
                  iter=  max iterations (default 500)\n\
     Expressions: x y t sin cos exp ln sqrt ^ + - * /"
}

// ─── Bitwise calculator ────────────────────────────────────────────────────────

pub fn bitwise_calc(query: &str) -> String {
    let mut out = String::new();
    let sep = "─".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  BITWISE CALCULATOR");
    let _ = writeln!(out, "{}", sep);

    let q = query.trim();

    // Parse a token that might be decimal, 0x hex, 0b binary, 0o octal, or a single char
    let parse_val = |s: &str| -> Option<u64> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            return u64::from_str_radix(h, 16).ok();
        }
        if let Some(b) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
            return u64::from_str_radix(b, 2).ok();
        }
        if let Some(o) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
            return u64::from_str_radix(o, 8).ok();
        }
        if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 {
            let inner = &s[1..s.len() - 1];
            let mut chars = inner.chars();
            if let Some(c) = chars.next() {
                if chars.next().is_none() {
                    return Some(c as u64);
                }
            }
        }
        s.parse::<u64>()
            .ok()
            .or_else(|| s.parse::<i64>().ok().map(|v| v as u64))
    };

    let fmt_row = |out: &mut String, label: &str, v: u64| {
        let _ = writeln!(out, "  {:<18} {:>20}  (0x{:016X})", label, v as i64, v);
    };

    let fmt_bin = |out: &mut String, label: &str, v: u64| {
        // Show as 4 groups of 16 bits
        let b = format!("{:064b}", v);
        let groups: Vec<&str> = [&b[0..16], &b[16..32], &b[32..48], &b[48..64]].to_vec();
        let _ = writeln!(
            out,
            "  {:<18} {}_{}_{}_{}",
            label, groups[0], groups[1], groups[2], groups[3]
        );
    };

    // ─── ieee754: float bit pattern analysis ───
    if q.starts_with("ieee754 ") || q.starts_with("float ") {
        let raw = q.split_once(' ').map(|x| x.1).unwrap_or("").trim();
        let fval: f64 = match raw.parse::<f64>() {
            Ok(v) => v,
            Err(_) => {
                let _ = writeln!(out, "  Cannot parse float: {}", raw);
                return out;
            }
        };
        let bits = fval.to_bits();
        let sign = (bits >> 63) & 1;
        let exp_raw = ((bits >> 52) & 0x7FF) as i64;
        let mantissa = bits & 0x000F_FFFF_FFFF_FFFF;
        let exp_actual = exp_raw - 1023;

        let _ = writeln!(out, "  Value:    {}", fval);
        let _ = writeln!(out, "  Bits:     0x{:016X}", bits);
        let b = format!("{:064b}", bits);
        let _ = writeln!(
            out,
            "  Binary:   {} {} {} {}",
            &b[0..1],
            &b[1..12],
            &b[12..32],
            &b[32..64]
        );
        let _ = writeln!(
            out,
            "  Sign:     {} ({})",
            sign,
            if sign == 0 { "positive" } else { "negative" }
        );
        let _ = writeln!(
            out,
            "  Exponent: {:011b} raw={} actual={}",
            exp_raw, exp_raw, exp_actual
        );
        let _ = writeln!(out, "  Mantissa: {:052b}", mantissa);
        let category = if exp_raw == 0x7FF {
            if mantissa == 0 {
                if sign == 0 {
                    "+Infinity"
                } else {
                    "-Infinity"
                }
            } else {
                "NaN"
            }
        } else if exp_raw == 0 {
            if mantissa == 0 {
                "Zero"
            } else {
                "Subnormal"
            }
        } else {
            "Normal"
        };
        let _ = writeln!(out, "  Category: {}", category);
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── single value inspection ───
    let words: Vec<&str> = q.split_whitespace().collect();

    if words.is_empty() {
        let _ = writeln!(out, "{}", bitwise_usage());
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // Two-operand ops: A op B
    if words.len() >= 3 {
        let a_str = words[0];
        let op = words[1];
        let b_str = words[2];
        let a = match parse_val(a_str) {
            Some(v) => v,
            None => {
                let _ = writeln!(out, "  Cannot parse operand: {}", a_str);
                return out;
            }
        };
        match op.to_lowercase().as_str() {
            "and" | "&" => {
                let result = a & parse_val(b_str).unwrap_or(0);
                let _ = writeln!(out, "  A (dec):  {}", a as i64);
                let _ = writeln!(out, "  B (dec):  {}", parse_val(b_str).unwrap_or(0) as i64);
                let b_val = parse_val(b_str).unwrap_or(0);
                fmt_bin(&mut out, "  A binary:", a);
                fmt_bin(&mut out, "  B binary:", b_val);
                fmt_bin(&mut out, "  AND:     ", result);
                fmt_row(&mut out, "  Result:", result);
                let _ = writeln!(out, "{}", sep);
                return out;
            }
            "or" | "|" => {
                let b_val = parse_val(b_str).unwrap_or(0);
                let result = a | b_val;
                fmt_bin(&mut out, "  A binary:", a);
                fmt_bin(&mut out, "  B binary:", b_val);
                fmt_bin(&mut out, "  OR:      ", result);
                fmt_row(&mut out, "  Result:", result);
                let _ = writeln!(out, "{}", sep);
                return out;
            }
            "xor" | "^" => {
                let b_val = parse_val(b_str).unwrap_or(0);
                let result = a ^ b_val;
                fmt_bin(&mut out, "  A binary:", a);
                fmt_bin(&mut out, "  B binary:", b_val);
                fmt_bin(&mut out, "  XOR:     ", result);
                fmt_row(&mut out, "  Result:", result);
                let _ = writeln!(out, "{}", sep);
                return out;
            }
            "shl" | "<<" | "lsh" => {
                let n = parse_val(b_str).unwrap_or(0) as u32;
                let result = a.checked_shl(n).unwrap_or(0);
                fmt_bin(&mut out, "  Before:  ", a);
                fmt_bin(&mut out, "  SHL by:", result);
                let _ = writeln!(out, "  Shift by: {}", n);
                fmt_row(&mut out, "  Result:", result);
                let _ = writeln!(out, "{}", sep);
                return out;
            }
            "shr" | ">>" | "rsh" => {
                let n = parse_val(b_str).unwrap_or(0) as u32;
                let result = a.checked_shr(n).unwrap_or(0);
                fmt_bin(&mut out, "  Before:  ", a);
                fmt_bin(&mut out, "  SHR by:", result);
                let _ = writeln!(out, "  Shift by: {}", n);
                fmt_row(&mut out, "  Result:", result);
                let _ = writeln!(out, "{}", sep);
                return out;
            }
            "rol" | "rotl" => {
                let n = (parse_val(b_str).unwrap_or(0) % 64) as u32;
                let result = a.rotate_left(n);
                fmt_bin(&mut out, "  Before:  ", a);
                fmt_bin(&mut out, "  ROL:     ", result);
                let _ = writeln!(out, "  Rotate by: {}", n);
                fmt_row(&mut out, "  Result:", result);
                let _ = writeln!(out, "{}", sep);
                return out;
            }
            "ror" | "rotr" => {
                let n = (parse_val(b_str).unwrap_or(0) % 64) as u32;
                let result = a.rotate_right(n);
                fmt_bin(&mut out, "  Before:  ", a);
                fmt_bin(&mut out, "  ROR:     ", result);
                let _ = writeln!(out, "  Rotate by: {}", n);
                fmt_row(&mut out, "  Result:", result);
                let _ = writeln!(out, "{}", sep);
                return out;
            }
            _ => {}
        }
    }

    // Single operand or unknown → full inspection
    let val_str = if words.len() == 2 && words[0].to_lowercase() == "not" {
        words[1]
    } else {
        words[0]
    };
    let is_not = words.len() == 2 && words[0].to_lowercase() == "not";

    let val = match parse_val(val_str) {
        Some(v) => v,
        None => {
            let _ = writeln!(out, "{}", bitwise_usage());
            let _ = writeln!(out, "{}", sep);
            return out;
        }
    };
    let display_val = if is_not { !val } else { val };
    let label = if is_not { "NOT result" } else { "Value" };

    let popcount = display_val.count_ones();
    let parity = popcount % 2;
    let leading = display_val.leading_zeros();
    let trailing = display_val.trailing_zeros();
    let msb_pos = if display_val == 0 { 0u32 } else { 63 - leading };

    let _ = writeln!(out, "  {} (input): {}", label, val as i64);
    if is_not {
        let _ = writeln!(out, "  NOT {:016X} = {:016X}", val, !val);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Representations ───");
    let _ = writeln!(out, "  Decimal (signed):    {}", display_val as i64);
    let _ = writeln!(out, "  Decimal (unsigned):  {}", display_val);
    let _ = writeln!(out, "  Hexadecimal:         0x{:016X}", display_val);
    let _ = writeln!(out, "  Octal:               0o{:o}", display_val);

    // Binary with spacing every 8 bits
    let b = format!("{:064b}", display_val);
    let _ = writeln!(out, "  Binary (64-bit):");
    let _ = writeln!(out, "    Bit 63-48:  {}_{}", &b[0..8], &b[8..16]);
    let _ = writeln!(out, "    Bit 47-32:  {}_{}", &b[16..24], &b[24..32]);
    let _ = writeln!(out, "    Bit 31-16:  {}_{}", &b[32..40], &b[40..48]);
    let _ = writeln!(out, "    Bit 15- 0:  {}_{}", &b[48..56], &b[56..64]);

    // Two's complement (flip bits + 1)
    let twos = (!display_val).wrapping_add(1);
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Bit Properties ───");
    let _ = writeln!(out, "  Popcount (set bits): {}", popcount);
    let _ = writeln!(
        out,
        "  Parity:              {} ({})",
        parity,
        if parity == 0 { "even" } else { "odd" }
    );
    let _ = writeln!(out, "  Leading zeros:       {}", leading);
    let _ = writeln!(out, "  Trailing zeros:      {}", trailing);
    let _ = writeln!(out, "  MSB position:        {}", msb_pos);
    let _ = writeln!(
        out,
        "  Two's complement:    {} (0x{:016X})",
        twos as i64, twos
    );
    let is_pow2 = display_val != 0 && (display_val & display_val.wrapping_sub(1)) == 0;
    let _ = writeln!(
        out,
        "  Power of 2:          {}",
        if is_pow2 { "yes" } else { "no" }
    );

    // Low byte / word / dword breakdown
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Byte Decomposition (little-endian) ───");
    for i in 0..8usize {
        let byte = ((display_val >> (i * 8)) & 0xFF) as u8;
        let _ = writeln!(
            out,
            "  Byte {}: 0x{:02X}  {:08b}  dec={}",
            i, byte, byte, byte
        );
    }

    let _ = writeln!(out, "{}", sep);
    out
}

fn bitwise_usage() -> &'static str {
    "Bitwise calculator — no model, no cloud:\n\
     \n\
     hematite --bitwise '255'                  inspect value (dec/hex/bin/octal/bytes)\n\
     hematite --bitwise '0xFF'                 hex input\n\
     hematite --bitwise '0b11001010'           binary input\n\
     hematite --bitwise 'NOT 0xFF'             bitwise NOT\n\
     hematite --bitwise '0xF0 AND 0x3C'        AND\n\
     hematite --bitwise '0xF0 OR 0x0F'         OR\n\
     hematite --bitwise '0xAB XOR 0xFF'        XOR\n\
     hematite --bitwise '1 SHL 7'              left shift\n\
     hematite --bitwise '256 SHR 4'            right shift\n\
     hematite --bitwise '0xDEAD ROL 4'         rotate left\n\
     hematite --bitwise '0xDEAD ROR 4'         rotate right\n\
     hematite --bitwise 'ieee754 3.14159'       IEEE 754 float breakdown\n\
     hematite --bitwise 'ieee754 NaN'           special float analysis\n\
     \n\
     Inputs: decimal, 0x hex, 0b binary, 0o octal, negative (-1)"
}

// ─── Set theory calculator ─────────────────────────────────────────────────────

pub fn set_calc(query: &str) -> String {
    let mut out = String::new();
    let sep = "─".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  SET THEORY CALCULATOR");
    let _ = writeln!(out, "{}", sep);

    let q = query.trim();

    // Parse a set literal: {1,2,3} or [1,2,3] or bare comma list
    // Items are strings (so sets of anything work)
    let parse_set = |s: &str| -> Vec<String> {
        let s = s
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim_start_matches('[')
            .trim_end_matches(']');
        let mut items: Vec<String> = s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect();
        items.sort();
        items.dedup();
        items
    };

    let fmt_set = |v: &[String]| -> String {
        if v.is_empty() {
            "{}".to_string()
        } else {
            format!("{{{}}}", v.join(", "))
        }
    };

    let q_lower = q.to_lowercase();

    // ─── power set ───
    if q_lower.starts_with("powerset ")
        || q_lower.starts_with("power_set ")
        || q_lower.starts_with("power set ")
    {
        let raw = q.split_once(' ').map(|x| x.1).unwrap_or("").trim();
        let set = parse_set(raw);
        if set.len() > 20 {
            let _ = writeln!(out, "  Set too large for power set (max 20 elements).");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
        let n = set.len();
        let count = 1usize << n;
        let _ = writeln!(out, "  Set A = {}", fmt_set(&set));
        let _ = writeln!(out, "  |A| = {}   |P(A)| = {}", n, count);
        let _ = writeln!(out);
        let _ = writeln!(out, "  Power set P(A):");
        let mut subsets: Vec<Vec<String>> = Vec::with_capacity(count);
        for mask in 0..count {
            let subset: Vec<String> = (0..n)
                .filter(|&i| mask & (1 << i) != 0)
                .map(|i| set[i].clone())
                .collect();
            subsets.push(subset);
        }
        // Sort by cardinality then lexicographic
        subsets.sort_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)));
        for (i, subset) in subsets.iter().enumerate() {
            let _ = writeln!(out, "  {:3}. {}", i + 1, fmt_set(subset));
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── cartesian product ───
    if q_lower.starts_with("cartesian ")
        || q_lower.starts_with("product ")
        || q_lower.contains(" x ")
    {
        // Split on " x " or keyword
        let parts: Vec<&str> = if q_lower.contains(" x ") {
            q.splitn(3, " x ").collect()
        } else {
            let kw = if q_lower.starts_with("cartesian ") {
                "cartesian "
            } else {
                "product "
            };
            let rest = &q[kw.len()..];
            rest.splitn(2, " x ").collect()
        };
        if parts.len() < 2 {
            let _ = writeln!(out, "  Usage: cartesian {{1,2}} x {{a,b,c}}");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
        let a = parse_set(parts[0]);
        let b = parse_set(parts[1]);
        let _ = writeln!(out, "  A = {}  (|A|={})", fmt_set(&a), a.len());
        let _ = writeln!(out, "  B = {}  (|B|={})", fmt_set(&b), b.len());
        let _ = writeln!(out, "  |A × B| = {}", a.len() * b.len());
        let _ = writeln!(out);
        let _ = writeln!(out, "  A × B:");
        for x in &a {
            for y in &b {
                let _ = writeln!(out, "    ({}, {})", x, y);
            }
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── two-set operations — split on keyword ───
    // Supported: union, intersection, difference, symmetric_difference, subset, superset, disjoint
    let ops = [
        "symmetric_difference",
        "sym_diff",
        "symdiff",
        "difference",
        "intersection",
        "intersect",
        "union",
        "subset",
        "superset",
        "disjoint",
        "equal",
    ];
    let mut op_found: Option<(&str, Vec<String>, Vec<String>)> = None;
    for &op in &ops {
        let needle = format!(" {} ", op);
        if let Some(pos) = q_lower.find(needle.as_str()) {
            let a_raw = q[..pos].trim();
            let b_raw = q[pos + needle.len()..].trim();
            op_found = Some((op, parse_set(a_raw), parse_set(b_raw)));
            break;
        }
    }

    if let Some((op, a, b)) = op_found {
        let _ = writeln!(out, "  A = {}  (|A|={})", fmt_set(&a), a.len());
        let _ = writeln!(out, "  B = {}  (|B|={})", fmt_set(&b), b.len());
        let _ = writeln!(out);
        match op {
            "union" => {
                let mut result = a.clone();
                for x in &b {
                    if !result.contains(x) {
                        result.push(x.clone());
                    }
                }
                result.sort();
                let _ = writeln!(out, "  A ∪ B = {}", fmt_set(&result));
                let _ = writeln!(out, "  |A ∪ B| = {}", result.len());
            }
            "intersection" | "intersect" => {
                let result: Vec<String> = a.iter().filter(|x| b.contains(x)).cloned().collect();
                let _ = writeln!(out, "  A ∩ B = {}", fmt_set(&result));
                let _ = writeln!(out, "  |A ∩ B| = {}", result.len());
            }
            "difference" => {
                let result: Vec<String> = a.iter().filter(|x| !b.contains(x)).cloned().collect();
                let _ = writeln!(out, "  A \\ B = {}", fmt_set(&result));
                let _ = writeln!(out, "  |A \\ B| = {}", result.len());
                let result2: Vec<String> = b.iter().filter(|x| !a.contains(x)).cloned().collect();
                let _ = writeln!(out, "  B \\ A = {}", fmt_set(&result2));
            }
            "symmetric_difference" | "sym_diff" | "symdiff" => {
                let in_a_not_b: Vec<String> =
                    a.iter().filter(|x| !b.contains(x)).cloned().collect();
                let in_b_not_a: Vec<String> =
                    b.iter().filter(|x| !a.contains(x)).cloned().collect();
                let mut result = in_a_not_b.clone();
                result.extend(in_b_not_a.clone());
                result.sort();
                let _ = writeln!(out, "  A Δ B = {}", fmt_set(&result));
                let _ = writeln!(out, "  |A Δ B| = {}", result.len());
                let _ = writeln!(out, "  (in A not B: {})", fmt_set(&in_a_not_b));
                let _ = writeln!(out, "  (in B not A: {})", fmt_set(&in_b_not_a));
            }
            "subset" => {
                let is_sub = a.iter().all(|x| b.contains(x));
                let is_proper = is_sub && a.len() < b.len();
                let _ = writeln!(out, "  A ⊆ B: {}", if is_sub { "YES" } else { "NO" });
                let _ = writeln!(
                    out,
                    "  A ⊂ B (proper): {}",
                    if is_proper { "YES" } else { "NO" }
                );
            }
            "superset" => {
                let is_super = b.iter().all(|x| a.contains(x));
                let is_proper = is_super && a.len() > b.len();
                let _ = writeln!(out, "  A ⊇ B: {}", if is_super { "YES" } else { "NO" });
                let _ = writeln!(
                    out,
                    "  A ⊃ B (proper): {}",
                    if is_proper { "YES" } else { "NO" }
                );
            }
            "disjoint" => {
                let is_disj = !a.iter().any(|x| b.contains(x));
                let _ = writeln!(
                    out,
                    "  A ∩ B = ∅ (disjoint): {}",
                    if is_disj { "YES" } else { "NO" }
                );
                if !is_disj {
                    let common: Vec<String> = a.iter().filter(|x| b.contains(x)).cloned().collect();
                    let _ = writeln!(out, "  Common elements: {}", fmt_set(&common));
                }
            }
            "equal" => {
                let equal = a.iter().all(|x| b.contains(x)) && b.iter().all(|x| a.contains(x));
                let _ = writeln!(out, "  A = B: {}", if equal { "YES" } else { "NO" });
            }
            _ => {}
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── single set inspection ───
    let set = parse_set(q);
    if !set.is_empty() {
        let _ = writeln!(out, "  Set = {}", fmt_set(&set));
        let _ = writeln!(out, "  Cardinality: {}", set.len());
        let _ = writeln!(
            out,
            "  Min: {}   Max: {}",
            set.first().unwrap_or(&String::new()),
            set.last().unwrap_or(&String::new())
        );
        // numeric stats if all parse as f64
        let nums: Vec<f64> = set.iter().filter_map(|x| x.parse::<f64>().ok()).collect();
        if nums.len() == set.len() && !nums.is_empty() {
            let sum: f64 = nums.iter().sum();
            let mean = sum / nums.len() as f64;
            let _ = writeln!(out, "  Sum: {}   Mean: {:.4}", sum, mean);
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    let _ = writeln!(out, "{}", set_usage());
    let _ = writeln!(out, "{}", sep);
    out
}

fn set_usage() -> &'static str {
    "Set theory calculator — no model, no cloud:\n\
     \n\
     hematite --set '{1,2,3} union {3,4,5}'           A ∪ B\n\
     hematite --set '{1,2,3} intersection {2,3,4}'    A ∩ B\n\
     hematite --set '{1,2,3,4} difference {2,4}'      A \\ B\n\
     hematite --set '{1,2,3} sym_diff {2,3,4}'        A Δ B\n\
     hematite --set '{1,2} subset {1,2,3}'            subset check\n\
     hematite --set '{1,2,3} superset {1,2}'          superset check\n\
     hematite --set '{1,2,3} disjoint {4,5,6}'        disjoint check\n\
     hematite --set 'powerset {a,b,c}'                P(A) — all subsets\n\
     hematite --set 'cartesian {1,2} x {a,b}'         A × B\n\
     hematite --set '{5,3,1,4,2}'                     inspect a set\n\
     \n\
     Elements can be numbers or strings. Sets are auto-deduplicated and sorted."
}

// ─── Classical cipher encoder / decoder ───────────────────────────────────────

pub fn cipher_calc(query: &str) -> String {
    let mut out = String::new();
    let sep = "─".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  CLASSICAL CIPHER");
    let _ = writeln!(out, "{}", sep);

    let q = query.trim();
    let words: Vec<&str> = q.splitn(3, ' ').collect();
    if words.is_empty() {
        let _ = writeln!(out, "{}", cipher_usage());
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    let cipher_name = words[0].to_lowercase();

    // ─── ROT13 ───
    if cipher_name == "rot13" {
        let text = words[1..].join(" ");
        let result: String = text
            .chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    let base = if c.is_ascii_uppercase() { b'A' } else { b'a' };
                    (((c as u8 - base + 13) % 26) + base) as char
                } else {
                    c
                }
            })
            .collect();
        let _ = writeln!(out, "  Cipher:  ROT13 (symmetric)");
        let _ = writeln!(out, "  Input:   {}", text);
        let _ = writeln!(out, "  Output:  {}", result);
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── Atbash ───
    if cipher_name == "atbash" {
        let text = words[1..].join(" ");
        let result: String = text
            .chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    let base = if c.is_ascii_uppercase() { b'A' } else { b'a' };
                    (base + 25 - (c as u8 - base)) as char
                } else {
                    c
                }
            })
            .collect();
        let _ = writeln!(out, "  Cipher:  Atbash (symmetric)");
        let _ = writeln!(out, "  Input:   {}", text);
        let _ = writeln!(out, "  Output:  {}", result);
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── Caesar / ROT-N ───
    if cipher_name == "caesar" || cipher_name == "rot" {
        if words.len() < 3 {
            let _ = writeln!(out, "  Usage: caesar <shift> <text>   or   rot <n> <text>");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
        let shift: i32 = words[1].parse().unwrap_or(13);
        let text = words[2];
        let shift_norm = ((shift % 26) + 26) as u8 % 26;
        let result: String = text
            .chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    let base = if c.is_ascii_uppercase() { b'A' } else { b'a' };
                    ((c as u8 - base + shift_norm) % 26 + base) as char
                } else {
                    c
                }
            })
            .collect();
        let decode: String = text
            .chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    let base = if c.is_ascii_uppercase() { b'A' } else { b'a' };
                    let s = (26 - shift_norm) % 26;
                    ((c as u8 - base + s) % 26 + base) as char
                } else {
                    c
                }
            })
            .collect();
        let _ = writeln!(out, "  Cipher:  Caesar / ROT-{}", shift);
        let _ = writeln!(out, "  Input:   {}", text);
        let _ = writeln!(out, "  Encoded: {}", result);
        let _ = writeln!(out, "  Decoded: {}", decode);
        // Show all 25 rotations in compact form
        let _ = writeln!(out);
        let _ = writeln!(out, "  ─── Brute-force all rotations ───");
        for n in 0u8..26 {
            let r: String = text
                .chars()
                .map(|c| {
                    if c.is_ascii_alphabetic() {
                        let base = if c.is_ascii_uppercase() { b'A' } else { b'a' };
                        ((c as u8 - base + n) % 26 + base) as char
                    } else {
                        c
                    }
                })
                .collect();
            let _ = writeln!(out, "  ROT-{:2}:  {}", n, r);
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── Vigenère encode/decode ───
    if cipher_name == "vigenere" || cipher_name == "vigenère" {
        // vigenere encode <key> <text>  OR  vigenere decode <key> <text>
        let parts: Vec<&str> = q.splitn(4, ' ').collect();
        if parts.len() < 4 {
            let _ = writeln!(out, "  Usage: vigenere encode <key> <text>");
            let _ = writeln!(out, "         vigenere decode <key> <text>");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
        let mode = parts[1].to_lowercase();
        let key = parts[2];
        let text = parts[3];
        let decode = mode == "decode" || mode == "dec" || mode == "d";

        let key_clean: Vec<u8> = key
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| c.to_ascii_uppercase() as u8 - b'A')
            .collect();
        if key_clean.is_empty() {
            let _ = writeln!(out, "  Key must contain at least one letter.");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
        let mut ki = 0usize;
        let result: String = text
            .chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    let base = if c.is_ascii_uppercase() { b'A' } else { b'a' };
                    let shift = key_clean[ki % key_clean.len()];
                    ki += 1;
                    if decode {
                        ((c as u8 - base + 26 - shift) % 26 + base) as char
                    } else {
                        ((c as u8 - base + shift) % 26 + base) as char
                    }
                } else {
                    c
                }
            })
            .collect();
        let _ = writeln!(out, "  Cipher:  Vigenère");
        let _ = writeln!(
            out,
            "  Mode:    {}",
            if decode { "decode" } else { "encode" }
        );
        let _ = writeln!(out, "  Key:     {}", key);
        let _ = writeln!(out, "  Input:   {}", text);
        let _ = writeln!(out, "  Output:  {}", result);
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── Rail Fence encode/decode ───
    if cipher_name == "railfence" || cipher_name == "rail_fence" || cipher_name == "rail" {
        let parts: Vec<&str> = q.splitn(4, ' ').collect();
        if parts.len() < 4 {
            let _ = writeln!(out, "  Usage: railfence encode <rails> <text>");
            let _ = writeln!(out, "         railfence decode <rails> <text>");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
        let mode = parts[1].to_lowercase();
        let rails: usize = parts[2].parse().unwrap_or(2).max(2);
        let text = parts[3];
        let decode = mode == "decode" || mode == "dec" || mode == "d";

        let chars: Vec<char> = text.chars().collect();
        let n = chars.len();

        if decode {
            // Reconstruct rail assignment pattern, fill, then read
            let mut pattern = vec![0usize; n];
            let mut rail = 0i32;
            let mut dir = 1i32;
            for i in 0..n {
                pattern[i] = rail as usize;
                if rail == 0 {
                    dir = 1;
                } else if rail == (rails as i32 - 1) {
                    dir = -1;
                }
                rail += dir;
            }
            let mut rail_len = vec![0usize; rails];
            for &r in &pattern {
                rail_len[r] += 1;
            }
            let mut rail_start = vec![0usize; rails];
            for r in 1..rails {
                rail_start[r] = rail_start[r - 1] + rail_len[r - 1];
            }
            let char_arr: Vec<char> = chars.to_vec();
            let mut result = vec![' '; n];
            let mut rail_idx: Vec<usize> = rail_start.clone();
            for i in 0..n {
                result[i] = char_arr[rail_idx[pattern[i]]];
                rail_idx[pattern[i]] += 1;
            }
            let decoded: String = result.into_iter().collect();
            let _ = writeln!(out, "  Cipher:  Rail Fence");
            let _ = writeln!(out, "  Rails:   {}", rails);
            let _ = writeln!(out, "  Input:   {}", text);
            let _ = writeln!(out, "  Decoded: {}", decoded);
        } else {
            let mut fence: Vec<Vec<char>> = vec![Vec::new(); rails];
            let mut rail = 0i32;
            let mut dir = 1i32;
            for &ch in &chars {
                fence[rail as usize].push(ch);
                if rail == 0 {
                    dir = 1;
                } else if rail == (rails as i32 - 1) {
                    dir = -1;
                }
                rail += dir;
            }
            let encoded: String = fence.iter().flat_map(|r| r.iter()).collect();
            let _ = writeln!(out, "  Cipher:  Rail Fence");
            let _ = writeln!(out, "  Rails:   {}", rails);
            let _ = writeln!(out, "  Input:   {}", text);
            let _ = writeln!(out, "  Encoded: {}", encoded);
            let _ = writeln!(out);
            let _ = writeln!(out, "  ─── Rail diagram ───");
            for (i, rail_row) in fence.iter().enumerate() {
                let s: String = rail_row.iter().collect();
                let _ = writeln!(out, "  Rail {}: {}", i, s);
            }
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── Columnar transposition ───
    if cipher_name == "columnar" || cipher_name == "transposition" {
        let parts: Vec<&str> = q.splitn(4, ' ').collect();
        if parts.len() < 4 {
            let _ = writeln!(out, "  Usage: columnar encode <key> <text>");
            let _ = writeln!(out, "         columnar decode <key> <text>");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
        let mode = parts[1].to_lowercase();
        let key = parts[2];
        let text = parts[3];
        let decode = mode == "decode" || mode == "dec" || mode == "d";

        let num_cols = key.len();
        let mut key_chars: Vec<(usize, char)> = key.chars().enumerate().collect();
        key_chars.sort_by_key(|&(_, c)| c);
        let col_order: Vec<usize> = key_chars.iter().map(|&(i, _)| i).collect();

        let pad: usize = if text.len() % num_cols == 0 {
            0
        } else {
            num_cols - text.len() % num_cols
        };
        let padded: Vec<char> = text.chars().chain(std::iter::repeat_n('_', pad)).collect();
        let num_rows = padded.len() / num_cols;

        if decode {
            // Reconstruct by filling columns in sorted key order
            let col_lens: Vec<usize> = (0..num_cols).map(|_| num_rows).collect();
            let mut cols: Vec<Vec<char>> = vec![Vec::new(); num_cols];
            let mut idx = 0;
            for &ci in &col_order {
                for _ in 0..col_lens[ci] {
                    if idx < text.len() {
                        cols[ci].push(text.chars().nth(idx).unwrap_or('_'));
                        idx += 1;
                    }
                }
            }
            let mut decoded = String::new();
            for r in 0..num_rows {
                for c in 0..num_cols {
                    if r < cols[c].len() {
                        decoded.push(cols[c][r]);
                    }
                }
            }
            let decoded = decoded.trim_end_matches('_');
            let _ = writeln!(out, "  Cipher:  Columnar Transposition");
            let _ = writeln!(out, "  Key:     {}", key);
            let _ = writeln!(out, "  Input:   {}", text);
            let _ = writeln!(out, "  Decoded: {}", decoded);
        } else {
            // Read row by row, output column by column in sorted key order
            let grid: Vec<Vec<char>> = padded.chunks(num_cols).map(|r| r.to_vec()).collect();
            let mut encoded = String::new();
            for &ci in &col_order {
                for row in &grid {
                    encoded.push(row[ci]);
                }
            }
            let _ = writeln!(out, "  Cipher:  Columnar Transposition");
            let _ = writeln!(out, "  Key:     {} (sorted order: {:?})", key, col_order);
            let _ = writeln!(out, "  Input:   {}", text);
            let _ = writeln!(out, "  Encoded: {}", encoded);
            let _ = writeln!(out);
            let _ = writeln!(out, "  ─── Grid ───");
            // Print key header
            let _ = writeln!(
                out,
                "  Key:  {}",
                key.chars().map(|c| format!("{} ", c)).collect::<String>()
            );
            for row in &grid {
                let row_str: String = row.iter().map(|c| format!("{} ", c)).collect();
                let _ = writeln!(out, "        {}", row_str);
            }
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── Morse code ───
    if cipher_name == "morse" {
        let parts: Vec<&str> = q.splitn(3, ' ').collect();
        if parts.len() < 3 {
            let _ = writeln!(out, "  Usage: morse encode <text>");
            let _ = writeln!(out, "         morse decode <morse>");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
        let mode = parts[1].to_lowercase();
        let text = parts[2];
        let decode = mode == "decode" || mode == "dec" || mode == "d";

        let morse_table: &[(&str, &str)] = &[
            ("A", ".-"),
            ("B", "-..."),
            ("C", "-.-."),
            ("D", "-.."),
            ("E", "."),
            ("F", "..-."),
            ("G", "--."),
            ("H", "...."),
            ("I", ".."),
            ("J", ".---"),
            ("K", "-.-"),
            ("L", ".-.."),
            ("M", "--"),
            ("N", "-."),
            ("O", "---"),
            ("P", ".--."),
            ("Q", "--.-"),
            ("R", ".-."),
            ("S", "..."),
            ("T", "-"),
            ("U", "..-"),
            ("V", "...-"),
            ("W", ".--"),
            ("X", "-..-"),
            ("Y", "-.--"),
            ("Z", "--.."),
            ("0", "-----"),
            ("1", ".----"),
            ("2", "..---"),
            ("3", "...--"),
            ("4", "....-"),
            ("5", "....."),
            ("6", "-...."),
            ("7", "--..."),
            ("8", "---.."),
            ("9", "----."),
            (",", "--..--"),
            (".", ".-.-.-"),
            ("?", "..--.."),
            ("!", "-.-.--"),
            ("/", "-..-."),
            ("@", ".--.-."),
            ("&", ".-..."),
        ];

        if decode {
            let code_to_char: std::collections::HashMap<&str, &str> =
                morse_table.iter().map(|&(c, m)| (m, c)).collect();
            let decoded: String = text
                .split("   ") // triple space = word boundary
                .map(|word| {
                    word.split(' ')
                        .map(|sym| code_to_char.get(sym).copied().unwrap_or("?"))
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join(" ");
            let _ = writeln!(out, "  Cipher:  Morse Code");
            let _ = writeln!(out, "  Input:   {}", text);
            let _ = writeln!(out, "  Decoded: {}", decoded);
        } else {
            let char_to_morse: std::collections::HashMap<&str, &str> =
                morse_table.iter().map(|&(c, m)| (c, m)).collect();
            let encoded: String = text
                .to_uppercase()
                .chars()
                .map(|c| {
                    if c == ' ' {
                        "   ".to_string()
                    } else {
                        let s = c.to_string();
                        char_to_morse
                            .get(s.as_str())
                            .copied()
                            .unwrap_or("?")
                            .to_string()
                            + " "
                    }
                })
                .collect();
            let _ = writeln!(out, "  Cipher:  Morse Code");
            let _ = writeln!(out, "  Input:   {}", text);
            let _ = writeln!(out, "  Encoded: {}", encoded.trim());
        }
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── Unknown ───
    let _ = writeln!(out, "{}", cipher_usage());
    let _ = writeln!(out, "{}", sep);
    out
}

fn cipher_usage() -> &'static str {
    "Classical cipher encoder/decoder — no model, no cloud:\n\
     \n\
     hematite --cipher 'rot13 Hello World'               ROT13 (symmetric)\n\
     hematite --cipher 'atbash Hello World'              Atbash (symmetric)\n\
     hematite --cipher 'caesar 13 Hello World'           Caesar shift (with brute-force)\n\
     hematite --cipher 'vigenere encode KEY plaintext'   Vigenère encode\n\
     hematite --cipher 'vigenere decode KEY ciphertext'  Vigenère decode\n\
     hematite --cipher 'railfence encode 3 WEAREDISCOVERED'  Rail Fence encode\n\
     hematite --cipher 'railfence decode 3 WECRLTEERDSOEEVEAAID'  Rail Fence decode\n\
     hematite --cipher 'columnar encode ZEBRA plaintext'  Columnar transposition encode\n\
     hematite --cipher 'morse encode Hello World'        Text to Morse\n\
     hematite --cipher 'morse decode .... . .-.. .-.. ---'  Morse to text\n\
     \n\
     Ciphers: rot13, atbash, caesar, vigenere, railfence, columnar, morse"
}

// ─── Validation toolkit (Luhn, ISBN, EAN, IBAN, UUID) ────────────────────────

pub fn validate_calc(input: &str) -> String {
    let mut out = String::new();
    let sep = "─".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  VALIDATION TOOLKIT");
    let _ = writeln!(out, "{}", sep);

    let q = input.trim();
    let _ = writeln!(out, "  Input: \"{}\"", q);
    let _ = writeln!(out);

    // Strip spaces/dashes for most checks
    let clean: String = q
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != ' ')
        .collect();

    // ─── Luhn algorithm (credit card) ───
    let luhn_result = {
        let digits: Vec<u32> = clean.chars().filter_map(|c| c.to_digit(10)).collect();
        if digits.len() >= 2 {
            let sum: u32 = digits
                .iter()
                .rev()
                .enumerate()
                .map(|(i, &d)| {
                    if i % 2 == 1 {
                        let doubled = d * 2;
                        if doubled > 9 {
                            doubled - 9
                        } else {
                            doubled
                        }
                    } else {
                        d
                    }
                })
                .sum();
            Some(sum % 10 == 0)
        } else {
            None
        }
    };

    // ─── Detect card network from prefix ───
    let card_network = {
        let digits_only: String = clean.chars().filter(|c| c.is_ascii_digit()).collect();
        let n = digits_only.len();
        if n >= 1 {
            let d1: u32 = digits_only
                .chars()
                .next()
                .unwrap()
                .to_digit(10)
                .unwrap_or(0);
            let d2: u32 = if n >= 2 {
                digits_only[..2].parse().unwrap_or(0)
            } else {
                0
            };
            let d4: u32 = if n >= 4 {
                digits_only[..4].parse().unwrap_or(0)
            } else {
                0
            };
            let d6: u32 = if n >= 6 {
                digits_only[..6].parse().unwrap_or(0)
            } else {
                0
            };
            if d1 == 4 {
                Some("Visa (starts with 4)")
            } else if d2 == 51 || d2 == 52 || d2 == 53 || d2 == 54 || d2 == 55 {
                Some("Mastercard (51-55)")
            } else if (622126..=622925).contains(&d6) || d4 == 6011 || d2 == 65 {
                Some("Discover")
            } else if d4 == 3782 || d4 == 3714 || d4 == 3787 || d4 == 3728 || d2 == 34 || d2 == 37 {
                Some("American Express")
            } else if d4 == 3528 || d4 == 3589 {
                Some("JCB")
            } else {
                Some("Unknown network")
            }
        } else {
            None
        }
    };

    if let Some(valid) = luhn_result {
        let _ = writeln!(out, "  ─── Luhn (Credit Card) ───");
        let _ = writeln!(
            out,
            "  Valid: {}  {}",
            if valid { "YES ✓" } else { "NO ✗" },
            if valid {
                "passes Luhn check"
            } else {
                "fails Luhn check"
            }
        );
        if let Some(network) = card_network {
            let digits_only: String = clean.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits_only.len() >= 12 {
                let _ = writeln!(out, "  Network: {}", network);
                let _ = writeln!(out, "  Length:  {} digits", digits_only.len());
            }
        }
        // Compute check digit needed to make it valid
        if !valid {
            let digits: Vec<u32> = clean.chars().filter_map(|c| c.to_digit(10)).collect();
            if digits.len() >= 2 {
                let without_last: Vec<u32> = digits[..digits.len() - 1].to_vec();
                for check in 0u32..10 {
                    let mut test = without_last.clone();
                    test.push(check);
                    let sum: u32 = test
                        .iter()
                        .rev()
                        .enumerate()
                        .map(|(i, &d)| {
                            if i % 2 == 1 {
                                let x = d * 2;
                                if x > 9 {
                                    x - 9
                                } else {
                                    x
                                }
                            } else {
                                d
                            }
                        })
                        .sum();
                    if sum % 10 == 0 {
                        let _ =
                            writeln!(out, "  Correct check digit: {} (replace last digit)", check);
                        break;
                    }
                }
            }
        }
        let _ = writeln!(out);
    }

    // ─── ISBN-10 ───
    let isbn10_digits: Vec<u32> = clean
        .chars()
        .filter_map(|c| {
            if c == 'X' || c == 'x' {
                Some(10)
            } else {
                c.to_digit(10)
            }
        })
        .collect();
    if isbn10_digits.len() == 10 {
        let sum: u32 = isbn10_digits
            .iter()
            .enumerate()
            .map(|(i, &d)| (10 - i as u32) * d)
            .sum();
        let valid = sum % 11 == 0;
        let _ = writeln!(out, "  ─── ISBN-10 ───");
        let _ = writeln!(
            out,
            "  Valid: {}  (sum mod 11 = {})",
            if valid { "YES ✓" } else { "NO ✗" },
            sum % 11
        );
        if !valid {
            // Compute correct check digit
            let sum9: u32 = isbn10_digits[..9]
                .iter()
                .enumerate()
                .map(|(i, &d)| (10 - i as u32) * d)
                .sum();
            let check = (11 - (sum9 % 11)) % 11;
            let check_str = if check == 10 {
                "X".to_string()
            } else {
                check.to_string()
            };
            let _ = writeln!(out, "  Correct check digit: {}", check_str);
        }
        let _ = writeln!(out);
    }

    // ─── ISBN-13 / EAN-13 ───
    let isbn13_digits: Vec<u32> = clean.chars().filter_map(|c| c.to_digit(10)).collect();
    if isbn13_digits.len() == 13 {
        let sum: u32 = isbn13_digits
            .iter()
            .enumerate()
            .map(|(i, &d)| if i % 2 == 0 { d } else { d * 3 })
            .sum();
        let valid = sum % 10 == 0;
        let prefix = &clean[..3];
        let kind = if prefix == "978" || prefix == "979" {
            "ISBN-13"
        } else {
            "EAN-13"
        };
        let _ = writeln!(out, "  ─── {} ───", kind);
        let _ = writeln!(
            out,
            "  Valid: {}  (weighted sum mod 10 = {})",
            if valid { "YES ✓" } else { "NO ✗" },
            sum % 10
        );
        if kind == "ISBN-13" {
            let _ = writeln!(out, "  Prefix: {} (Bookland)", prefix);
        }
        if !valid {
            let sum12: u32 = isbn13_digits[..12]
                .iter()
                .enumerate()
                .map(|(i, &d)| if i % 2 == 0 { d } else { d * 3 })
                .sum();
            let check = (10 - (sum12 % 10)) % 10;
            let _ = writeln!(out, "  Correct check digit: {}", check);
        }
        let _ = writeln!(out);
    }

    // ─── IBAN ───
    // IBAN: move first 4 chars to end, replace letters A=10..Z=35, check mod 97 == 1
    let iban_upper = clean.to_uppercase();
    if iban_upper.len() >= 15
        && iban_upper.len() <= 34
        && iban_upper.chars().take(2).all(|c| c.is_ascii_uppercase())
        && iban_upper
            .chars()
            .skip(2)
            .take(2)
            .all(|c| c.is_ascii_digit())
    {
        let rearranged = format!("{}{}", &iban_upper[4..], &iban_upper[..4]);
        let numeric: String = rearranged
            .chars()
            .map(|c| {
                if c.is_ascii_uppercase() {
                    format!("{}", c as u32 - 'A' as u32 + 10)
                } else {
                    c.to_string()
                }
            })
            .collect();
        // Modular arithmetic on long number string
        let remainder = numeric.chars().fold(0u64, |acc, c| {
            let d = c.to_digit(10).unwrap_or(0) as u64;
            (acc * 10 + d) % 97
        });
        let valid = remainder == 1;
        let country = &iban_upper[..2];
        let _ = writeln!(out, "  ─── IBAN ───");
        let _ = writeln!(out, "  Country: {}", country);
        let _ = writeln!(out, "  Length:  {} characters", iban_upper.len());
        let _ = writeln!(
            out,
            "  Valid:   {}  (mod 97 = {})",
            if valid { "YES ✓" } else { "NO ✗" },
            remainder
        );
        let _ = writeln!(out);
    }

    // ─── UUID ───
    let uuid_clean: String = clean.to_lowercase();
    let uuid_nodash: String = uuid_clean.chars().filter(|c| *c != '-').collect();
    if uuid_nodash.len() == 32 && uuid_nodash.chars().all(|c| c.is_ascii_hexdigit()) {
        let formatted = format!(
            "{}-{}-{}-{}-{}",
            &uuid_nodash[0..8],
            &uuid_nodash[8..12],
            &uuid_nodash[12..16],
            &uuid_nodash[16..20],
            &uuid_nodash[20..32]
        );
        let version = u8::from_str_radix(&uuid_nodash[12..13], 16).unwrap_or(0);
        let variant_bits = u8::from_str_radix(&uuid_nodash[16..17], 16).unwrap_or(0);
        let variant = if variant_bits & 0xC == 0xC {
            "Microsoft (variant 11)"
        } else if variant_bits & 0x8 != 0 {
            "RFC 4122 (variant 10)"
        } else {
            "NCS/backward compat (variant 0)"
        };
        let _ = writeln!(out, "  ─── UUID ───");
        let _ = writeln!(out, "  Format:  {}", formatted);
        let _ = writeln!(out, "  Version: {}", version);
        let _ = writeln!(out, "  Variant: {}", variant);
        let _ = writeln!(out);
    }

    // ─── If nothing matched, show usage ───
    let nothing = luhn_result.is_none()
        && isbn10_digits.len() != 10
        && isbn13_digits.len() != 13
        && !clean
            .to_uppercase()
            .starts_with(|c: char| c.is_ascii_uppercase());
    if nothing {
        let _ = writeln!(out, "  No recognizable format detected. Examples:");
        let _ = writeln!(
            out,
            "  hematite --validate '4532015112830366'    credit card (Luhn)"
        );
        let _ = writeln!(out, "  hematite --validate '0-306-40615-2'       ISBN-10");
        let _ = writeln!(
            out,
            "  hematite --validate '978-0-306-40615-7'   ISBN-13 / EAN-13"
        );
        let _ = writeln!(out, "  hematite --validate 'GB82WEST12345698765432'  IBAN");
        let _ = writeln!(
            out,
            "  hematite --validate '550e8400-e29b-41d4-a716-446655440000'  UUID"
        );
    }

    let _ = writeln!(out, "{}", sep);
    out
}

// ─── Checksum calculator ──────────────────────────────────────────────────────

pub fn checksum_calc(input: &str) -> String {
    let mut out = String::new();
    let sep = "─".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  CHECKSUM CALCULATOR");
    let _ = writeln!(out, "{}", sep);

    let bytes: Vec<u8> = input.as_bytes().to_vec();
    let len = bytes.len();

    let _ = writeln!(out, "  Input:   \"{}\"", input);
    let _ = writeln!(out, "  Length:  {} bytes", len);
    let _ = writeln!(out);

    // ─── CRC-32 (IEEE 802.3 polynomial 0xEDB88320) ───
    let crc32 = {
        let mut table = [0u32; 256];
        for i in 0u32..256 {
            let mut c = i;
            for _ in 0..8 {
                if c & 1 != 0 {
                    c = 0xEDB8_8320 ^ (c >> 1);
                } else {
                    c >>= 1;
                }
            }
            table[i as usize] = c;
        }
        let mut crc: u32 = 0xFFFF_FFFF;
        for &b in &bytes {
            crc = (crc >> 8) ^ table[((crc ^ b as u32) & 0xFF) as usize];
        }
        crc ^ 0xFFFF_FFFF
    };

    // ─── CRC-16 (CCITT / X.25) ───
    let crc16 = {
        let mut crc: u16 = 0xFFFF;
        for &b in &bytes {
            crc ^= (b as u16) << 8;
            for _ in 0..8 {
                if crc & 0x8000 != 0 {
                    crc = (crc << 1) ^ 0x1021;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    };

    // ─── Adler-32 ───
    let adler32 = {
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in &bytes {
            a = (a + byte as u32) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    };

    // ─── FNV-1a 32-bit ───
    let fnv1a_32 = {
        let mut h: u32 = 2166136261;
        for &b in &bytes {
            h ^= b as u32;
            h = h.wrapping_mul(16777619);
        }
        h
    };

    // ─── FNV-1a 64-bit ───
    let fnv1a_64 = {
        let mut h: u64 = 14695981039346656037;
        for &b in &bytes {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
        h
    };

    // ─── DJB2 ───
    let djb2 = {
        let mut h: u64 = 5381;
        for &b in &bytes {
            h = h.wrapping_mul(33).wrapping_add(b as u64);
        }
        h
    };

    // ─── SDBM ───
    let sdbm = {
        let mut h: u64 = 0;
        for &b in &bytes {
            h = (b as u64)
                .wrapping_add(h.wrapping_shl(6))
                .wrapping_add(h.wrapping_shl(16))
                .wrapping_sub(h);
        }
        h
    };

    // ─── XOR checksum ───
    let xor8: u8 = bytes.iter().fold(0u8, |acc, &b| acc ^ b);
    let xor16: u16 = bytes.chunks(2).fold(0u16, |acc, chunk| {
        let w = if chunk.len() == 2 {
            (chunk[0] as u16) << 8 | chunk[1] as u16
        } else {
            (chunk[0] as u16) << 8
        };
        acc ^ w
    });

    // ─── Sum checksums ───
    let sum8: u8 = bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    let sum16: u16 = bytes
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
    let sum32: u32 = bytes
        .iter()
        .fold(0u32, |acc, &b| acc.wrapping_add(b as u32));

    let _ = writeln!(out, "  ─── Cyclic Redundancy Checks ───");
    let _ = writeln!(out, "  CRC-32 (IEEE):     0x{:08X}  ({:10})", crc32, crc32);
    let _ = writeln!(
        out,
        "  CRC-16 (CCITT):    0x{:04X}      ({:6})",
        crc16, crc16
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Hash Functions ───");
    let _ = writeln!(
        out,
        "  Adler-32:          0x{:08X}  ({:10})",
        adler32, adler32
    );
    let _ = writeln!(
        out,
        "  FNV-1a 32:         0x{:08X}  ({:10})",
        fnv1a_32, fnv1a_32
    );
    let _ = writeln!(out, "  FNV-1a 64:         0x{:016X}", fnv1a_64);
    let _ = writeln!(out, "  DJB2:              0x{:016X}", djb2);
    let _ = writeln!(out, "  SDBM:              0x{:016X}", sdbm);
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Simple Checksums ───");
    let _ = writeln!(out, "  XOR-8:             0x{:02X}  ({})", xor8, xor8);
    let _ = writeln!(out, "  XOR-16:            0x{:04X}  ({})", xor16, xor16);
    let _ = writeln!(out, "  Sum-8 (mod 256):   0x{:02X}  ({})", sum8, sum8);
    let _ = writeln!(out, "  Sum-16:            0x{:04X}  ({})", sum16, sum16);
    let _ = writeln!(out, "  Sum-32:            0x{:08X}  ({})", sum32, sum32);
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Byte Analysis ───");
    let _ = writeln!(
        out,
        "  Min byte:          0x{:02X}  ({})",
        bytes.iter().min().copied().unwrap_or(0),
        bytes.iter().min().copied().unwrap_or(0)
    );
    let _ = writeln!(
        out,
        "  Max byte:          0x{:02X}  ({})",
        bytes.iter().max().copied().unwrap_or(0),
        bytes.iter().max().copied().unwrap_or(0)
    );
    let avg_byte = bytes.iter().map(|&b| b as f64).sum::<f64>() / len.max(1) as f64;
    let _ = writeln!(out, "  Avg byte value:    {:.2}", avg_byte);
    // Hex dump (first 32 bytes)
    if len <= 32 {
        let _ = writeln!(
            out,
            "  Hex: {}",
            bytes
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ")
        );
    } else {
        let preview: String = bytes[..32]
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");
        let _ = writeln!(out, "  Hex (first 32): {} ...", preview);
    }

    let _ = writeln!(out, "{}", sep);
    out
}

// ─── Sorting algorithm visualizer ─────────────────────────────────────────────

pub fn sort_viz(query: &str) -> String {
    let mut out = String::new();
    let sep = "─".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  SORTING ALGORITHM VISUALIZER");
    let _ = writeln!(out, "{}", sep);

    let q = query.trim();
    // Format: "<algorithm> <n1,n2,n3,...>"  or just "<n1,n2,...>" for all algorithms
    let words: Vec<&str> = q.splitn(2, ' ').collect();
    let algos_all = ["bubble", "insertion", "selection", "merge", "quick", "heap"];

    let (algos, data_str) = if words.len() == 2 {
        let first_lower = words[0].to_lowercase();
        if algos_all.contains(&first_lower.as_str()) {
            (vec![first_lower], words[1])
        } else {
            (algos_all.iter().map(|s| s.to_string()).collect(), q)
        }
    } else {
        (algos_all.iter().map(|s| s.to_string()).collect(), q)
    };

    // Parse data
    let data: Vec<i64> = data_str
        .split(|c: char| !c.is_numeric() && c != '-')
        .filter_map(|s| s.parse().ok())
        .collect();

    if data.is_empty() || data.len() < 2 {
        let _ = writeln!(
            out,
            "  Provide at least 2 numbers: hematite --sort-viz '5,3,8,1,9,2'"
        );
        let _ = writeln!(
            out,
            "  Or specify an algorithm:    hematite --sort-viz 'bubble 5,3,8,1'"
        );
        let _ = writeln!(out, "{}", sep);
        return out;
    }
    if data.len() > 20 {
        let _ = writeln!(
            out,
            "  Max 20 elements for visualization (got {})",
            data.len()
        );
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    let _ = writeln!(out, "  Input: {:?}", data);
    let _ = writeln!(out);

    let max_val = *data.iter().max().unwrap_or(&1);
    let min_val = *data.iter().min().unwrap_or(&0);
    let range = (max_val - min_val).max(1);

    // Bar chart renderer
    let bar_height = 8usize;
    let bar_render = |arr: &[i64]| -> String {
        let mut lines = vec![String::new(); bar_height + 1];
        for &v in arr {
            let h = ((v - min_val) as f64 / range as f64 * bar_height as f64).round() as usize;
            let h = h.min(bar_height);
            for row in 0..bar_height {
                let bar_row = bar_height - 1 - row;
                if bar_row < h {
                    lines[row].push_str("██ ");
                } else {
                    lines[row].push_str("   ");
                }
            }
            lines[bar_height].push_str(&format!("{:<3}", v));
        }
        lines
            .iter()
            .map(|l| format!("  {}", l))
            .collect::<Vec<_>>()
            .join("\n")
    };

    for algo in &algos {
        let _ = writeln!(out, "  ═══ {} Sort ═══", algo.to_uppercase());
        let _ = writeln!(out);

        let mut arr = data.clone();
        let mut steps: Vec<(Vec<i64>, String)> = Vec::new();
        let mut comparisons = 0usize;
        let mut swaps = 0usize;

        match algo.as_str() {
            "bubble" => {
                let n = arr.len();
                for i in 0..n {
                    let mut swapped = false;
                    for j in 0..n - 1 - i {
                        comparisons += 1;
                        if arr[j] > arr[j + 1] {
                            arr.swap(j, j + 1);
                            swaps += 1;
                            swapped = true;
                        }
                    }
                    steps.push((
                        arr.clone(),
                        format!("Pass {} (sorted tail: {})", i + 1, i + 1),
                    ));
                    if !swapped {
                        break;
                    }
                }
            }
            "insertion" => {
                for i in 1..arr.len() {
                    let key = arr[i];
                    let mut j = i as i64 - 1;
                    while j >= 0 {
                        comparisons += 1;
                        if arr[j as usize] > key {
                            arr[(j + 1) as usize] = arr[j as usize];
                            swaps += 1;
                            j -= 1;
                        } else {
                            break;
                        }
                    }
                    arr[(j + 1) as usize] = key;
                    steps.push((arr.clone(), format!("Insert {} at position {}", key, j + 1)));
                }
            }
            "selection" => {
                let n = arr.len();
                for i in 0..n - 1 {
                    let mut min_idx = i;
                    for j in i + 1..n {
                        comparisons += 1;
                        if arr[j] < arr[min_idx] {
                            min_idx = j;
                        }
                    }
                    if min_idx != i {
                        arr.swap(i, min_idx);
                        swaps += 1;
                    }
                    steps.push((
                        arr.clone(),
                        format!("Select min={} → position {}", arr[i], i),
                    ));
                }
            }
            "merge" => {
                fn merge_sort_steps(
                    arr: &mut Vec<i64>,
                    steps: &mut Vec<(Vec<i64>, String)>,
                    comparisons: &mut usize,
                    swaps: &mut usize,
                    lo: usize,
                    hi: usize,
                ) {
                    if hi - lo <= 1 {
                        return;
                    }
                    let mid = (lo + hi) / 2;
                    merge_sort_steps(arr, steps, comparisons, swaps, lo, mid);
                    merge_sort_steps(arr, steps, comparisons, swaps, mid, hi);
                    let left: Vec<i64> = arr[lo..mid].to_vec();
                    let right: Vec<i64> = arr[mid..hi].to_vec();
                    let (mut li, mut ri) = (0, 0);
                    let mut idx = lo;
                    while li < left.len() && ri < right.len() {
                        *comparisons += 1;
                        if left[li] <= right[ri] {
                            arr[idx] = left[li];
                            li += 1;
                        } else {
                            arr[idx] = right[ri];
                            ri += 1;
                            *swaps += 1;
                        }
                        idx += 1;
                    }
                    while li < left.len() {
                        arr[idx] = left[li];
                        li += 1;
                        idx += 1;
                    }
                    while ri < right.len() {
                        arr[idx] = right[ri];
                        ri += 1;
                        idx += 1;
                    }
                    steps.push((arr.clone(), format!("Merge [{}, {})", lo, hi)));
                }
                let n = arr.len();
                merge_sort_steps(&mut arr, &mut steps, &mut comparisons, &mut swaps, 0, n);
            }
            "quick" => {
                fn quick_sort_steps(
                    arr: &mut Vec<i64>,
                    steps: &mut Vec<(Vec<i64>, String)>,
                    comparisons: &mut usize,
                    swaps: &mut usize,
                    lo: usize,
                    hi: usize,
                ) {
                    if lo + 1 >= hi {
                        return;
                    }
                    let pivot = arr[hi - 1];
                    let mut i = lo;
                    for j in lo..hi - 1 {
                        *comparisons += 1;
                        if arr[j] <= pivot {
                            arr.swap(i, j);
                            i += 1;
                            *swaps += 1;
                        }
                    }
                    arr.swap(i, hi - 1);
                    *swaps += 1;
                    steps.push((arr.clone(), format!("Pivot={} partitioned at {}", pivot, i)));
                    if i > lo {
                        quick_sort_steps(arr, steps, comparisons, swaps, lo, i);
                    }
                    if i + 1 < hi {
                        quick_sort_steps(arr, steps, comparisons, swaps, i + 1, hi);
                    }
                }
                let n = arr.len();
                quick_sort_steps(&mut arr, &mut steps, &mut comparisons, &mut swaps, 0, n);
            }
            "heap" => {
                let n = arr.len();
                // Build max-heap
                for i in (0..n / 2).rev() {
                    let mut root = i;
                    loop {
                        let left = 2 * root + 1;
                        let right = 2 * root + 2;
                        let mut largest = root;
                        if left < n {
                            comparisons += 1;
                            if arr[left] > arr[largest] {
                                largest = left;
                            }
                        }
                        if right < n {
                            comparisons += 1;
                            if arr[right] > arr[largest] {
                                largest = right;
                            }
                        }
                        if largest != root {
                            arr.swap(root, largest);
                            swaps += 1;
                            root = largest;
                        } else {
                            break;
                        }
                    }
                }
                steps.push((arr.clone(), "Heap built".to_string()));
                for end in (1..n).rev() {
                    arr.swap(0, end);
                    swaps += 1;
                    // Sift down
                    let mut root = 0;
                    loop {
                        let left = 2 * root + 1;
                        let right = 2 * root + 2;
                        let mut largest = root;
                        if left < end {
                            comparisons += 1;
                            if arr[left] > arr[largest] {
                                largest = left;
                            }
                        }
                        if right < end {
                            comparisons += 1;
                            if arr[right] > arr[largest] {
                                largest = right;
                            }
                        }
                        if largest != root {
                            arr.swap(root, largest);
                            swaps += 1;
                            root = largest;
                        } else {
                            break;
                        }
                    }
                    steps.push((
                        arr.clone(),
                        format!("Extracted max, sorted: {} elements", n - end),
                    ));
                }
            }
            _ => {}
        }

        // Show steps (cap at 12 to avoid walls of text)
        let max_steps = 12usize;
        let step_count = steps.len();
        let show_steps = if step_count > max_steps {
            // Show first 4, last 4, mark ellipsis
            let mut s = steps[..4].to_vec();
            s.push((vec![], format!("... {} more steps ...", step_count - 8)));
            s.extend_from_slice(&steps[step_count - 4..]);
            s
        } else {
            steps.clone()
        };

        for (step_arr, label) in &show_steps {
            if step_arr.is_empty() {
                let _ = writeln!(out, "  {}", label);
                continue;
            }
            let _ = writeln!(out, "  → {}", label);
            let _ = write!(out, "{}", bar_render(step_arr));
            let _ = writeln!(out);
        }

        let _ = writeln!(out);
        let _ = writeln!(out, "  Final: {:?}", arr);
        let _ = writeln!(
            out,
            "  Comparisons: {}  |  Swaps/moves: {}  |  Steps: {}",
            comparisons, swaps, step_count
        );
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "{}", sep);
    out
}

// ─── Number formatter ─────────────────────────────────────────────────────────

pub fn number_format(query: &str) -> String {
    let mut out = String::new();
    let sep = "─".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  NUMBER FORMAT CONVERTER");
    let _ = writeln!(out, "{}", sep);

    let q = query.trim();

    // Try parsing as integer first, then float
    let val: f64 = match q.replace(['_', ','], "").parse::<f64>() {
        Ok(v) => v,
        Err(_) => {
            let _ = writeln!(out, "  Cannot parse: {}", q);
            let _ = writeln!(out, "  Usage: hematite --number-format '1234567890'");
            let _ = writeln!(out, "         hematite --number-format '6.022e23'");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
    };

    let _ = writeln!(out, "  Input: {}", q);
    let _ = writeln!(out);

    // ─── Thousands-separated form ───
    let fmt_thousands = |v: f64| -> String {
        if v == v.floor() && v.abs() < 1e15 {
            let i = v as i64;
            let s = format!("{}", i.abs());
            let with_sep: String = s
                .chars()
                .rev()
                .enumerate()
                .flat_map(|(i, c)| {
                    if i > 0 && i % 3 == 0 {
                        vec![',', c]
                    } else {
                        vec![c]
                    }
                })
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if i < 0 {
                format!("-{}", with_sep)
            } else {
                with_sep
            }
        } else {
            // Show decimal with commas on integer part
            let int_part = v.abs().floor() as i64;
            let frac = (v.abs() - int_part as f64).to_string();
            let frac_str = if frac.len() > 2 { &frac[1..] } else { "" };
            let s = format!("{}", int_part);
            let with_sep: String = s
                .chars()
                .rev()
                .enumerate()
                .flat_map(|(i, c)| {
                    if i > 0 && i % 3 == 0 {
                        vec![',', c]
                    } else {
                        vec![c]
                    }
                })
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            let signed = if v < 0.0 {
                format!("-{}{}", with_sep, frac_str)
            } else {
                format!("{}{}", with_sep, frac_str)
            };
            signed
        }
    };

    // ─── Scientific notation ───
    let sci = if val == 0.0 {
        "0.000000e0".to_string()
    } else {
        let exp = val.abs().log10().floor() as i32;
        let mantissa = val / 10f64.powi(exp);
        format!("{:.6}e{}", mantissa, exp)
    };

    // ─── Engineering notation (exponent multiple of 3) ───
    let eng = if val == 0.0 {
        "0.000e0".to_string()
    } else {
        let exp = val.abs().log10().floor() as i32;
        let eng_exp = (exp as f64 / 3.0).floor() as i32 * 3;
        let mantissa = val / 10f64.powi(eng_exp);
        format!("{:.3}e{}", mantissa, eng_exp)
    };

    // ─── SI prefix ───
    let si = {
        let si_prefixes: &[(f64, &str, &str)] = &[
            (1e24, "Y", "yotta"),
            (1e21, "Z", "zetta"),
            (1e18, "E", "exa"),
            (1e15, "P", "peta"),
            (1e12, "T", "tera"),
            (1e9, "G", "giga"),
            (1e6, "M", "mega"),
            (1e3, "k", "kilo"),
            (1e0, "", ""),
            (1e-3, "m", "milli"),
            (1e-6, "μ", "micro"),
            (1e-9, "n", "nano"),
            (1e-12, "p", "pico"),
            (1e-15, "f", "femto"),
        ];
        let abs_val = val.abs();
        let mut result = format!("{:.6}", val);
        for &(scale, sym, name) in si_prefixes {
            if abs_val >= scale * 0.999 || scale <= 1e-12 {
                let scaled = val / scale;
                if sym.is_empty() {
                    result = format!("{:.4}", scaled);
                } else {
                    result = format!("{:.4} {} ({})", scaled, sym, name);
                }
                break;
            }
        }
        result
    };

    // ─── Binary / Hex (integers only) ───
    let int_forms = if val == val.floor() && val.abs() < 2e63 {
        let i = val as i64;
        let u = i as u64;
        Some((
            format!("0x{:X}", u),
            format!("0b{:b}", u),
            format!("0o{:o}", u),
        ))
    } else {
        None
    };

    // ─── Word form (English) ───
    let word_form = number_to_words(val);

    let _ = writeln!(out, "  ─── Number Representations ───");
    let _ = writeln!(out, "  Decimal (formatted):  {}", fmt_thousands(val));
    let _ = writeln!(out, "  Scientific notation:  {}", sci);
    let _ = writeln!(out, "  Engineering notation: {}", eng);
    let _ = writeln!(out, "  SI prefix:            {}", si);
    if let Some((hex, bin, oct)) = int_forms {
        let _ = writeln!(out, "  Hexadecimal:          {}", hex);
        let _ = writeln!(out, "  Binary:               {}", bin);
        let _ = writeln!(out, "  Octal:                {}", oct);
    }
    let _ = writeln!(out, "  Word form:            {}", word_form);
    if val != 0.0 {
        let _ = writeln!(out, "  Reciprocal:           {:.6} (1/{})", 1.0 / val, q);
        let _ = writeln!(out, "  Percentage:           {:.4}%", val * 100.0);
        let log10 = val.abs().log10();
        let ln_v = val.abs().ln();
        let _ = writeln!(out, "  log₁₀:               {:.6}", log10);
        let _ = writeln!(out, "  ln:                   {:.6}", ln_v);
        let _ = writeln!(out, "  √:                    {:.6}", val.abs().sqrt());
        if val == val.floor() && val.abs() < 1e15 {
            let _ = writeln!(out, "  ²:                    {}", fmt_thousands(val * val));
        }
    }
    let _ = writeln!(out, "{}", sep);
    out
}

fn number_to_words(v: f64) -> String {
    if v == 0.0 {
        return "zero".to_string();
    }
    let is_neg = v < 0.0;
    let abs_v = v.abs();

    // Only handle integers up to ~999 trillion
    if abs_v != abs_v.floor() || abs_v >= 1e15 {
        return "(too large or fractional for word form)".to_string();
    }
    let n = abs_v as u64;

    let ones = [
        "",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    let tens = [
        "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];

    let under100 = |n: u64| -> String {
        if n < 20 {
            ones[n as usize].to_string()
        } else {
            let t = tens[(n / 10) as usize];
            let o = ones[(n % 10) as usize];
            if o.is_empty() {
                t.to_string()
            } else {
                format!("{}-{}", t, o)
            }
        }
    };

    let under1000 = |n: u64| -> String {
        if n < 100 {
            under100(n)
        } else {
            let h = ones[(n / 100) as usize];
            let rem = n % 100;
            if rem == 0 {
                format!("{} hundred", h)
            } else {
                format!("{} hundred {}", h, under100(rem))
            }
        }
    };

    let scales: &[(u64, &str)] = &[
        (1_000_000_000_000, "trillion"),
        (1_000_000_000, "billion"),
        (1_000_000, "million"),
        (1_000, "thousand"),
    ];

    let mut parts: Vec<String> = Vec::new();
    let mut remaining = n;
    for &(scale, name) in scales {
        if remaining >= scale {
            let chunk = remaining / scale;
            remaining %= scale;
            parts.push(format!("{} {}", under1000(chunk), name));
        }
    }
    if remaining > 0 {
        parts.push(under1000(remaining));
    }

    let result = parts.join(" ");
    if is_neg {
        format!("negative {}", result)
    } else {
        result
    }
}

// ─── String distance metrics ──────────────────────────────────────────────────

pub fn string_dist(query: &str) -> String {
    let mut out = String::new();
    let sep = "─".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  STRING DISTANCE METRICS");
    let _ = writeln!(out, "{}", sep);

    // Split on " vs " or " | " to get two strings
    let (a, b) = if let Some(pos) = query.find(" vs ") {
        (query[..pos].trim(), query[pos + 4..].trim())
    } else if let Some(pos) = query.find(" | ") {
        (query[..pos].trim(), query[pos + 3..].trim())
    } else {
        // Try splitting on comma if no quoted form
        let parts: Vec<&str> = query.splitn(2, ',').collect();
        if parts.len() == 2 {
            (parts[0].trim(), parts[1].trim())
        } else {
            let _ = writeln!(
                out,
                "  Usage: hematite --levenshtein '<string1> vs <string2>'"
            );
            let _ = writeln!(out, "         hematite --levenshtein 'kitten vs sitting'");
            let _ = writeln!(out, "         hematite --levenshtein 'hello, helo'");
            let _ = writeln!(out, "{}", sep);
            return out;
        }
    };

    let _ = writeln!(out, "  String A: \"{}\"  (len={})", a, a.chars().count());
    let _ = writeln!(out, "  String B: \"{}\"  (len={})", b, b.chars().count());
    let _ = writeln!(out);

    let ac: Vec<char> = a.chars().collect();
    let bc: Vec<char> = b.chars().collect();
    let m = ac.len();
    let n = bc.len();

    // ─── Levenshtein edit distance ───
    let lev_dist = {
        let mut dp = vec![vec![0usize; n + 1]; m + 1];
        for i in 0..=m {
            dp[i][0] = i;
        }
        for j in 0..=n {
            dp[0][j] = j;
        }
        for i in 1..=m {
            for j in 1..=n {
                if ac[i - 1] == bc[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1];
                } else {
                    dp[i][j] = 1 + dp[i - 1][j - 1].min(dp[i - 1][j]).min(dp[i][j - 1]);
                }
            }
        }
        dp[m][n]
    };
    let max_len = m.max(n).max(1);
    let lev_sim = 1.0 - lev_dist as f64 / max_len as f64;

    // ─── Damerau-Levenshtein (transpositions allowed) ───
    let dl_dist = {
        let mut dp = vec![vec![0usize; n + 1]; m + 1];
        for i in 0..=m {
            dp[i][0] = i;
        }
        for j in 0..=n {
            dp[0][j] = j;
        }
        for i in 1..=m {
            for j in 1..=n {
                let cost = if ac[i - 1] == bc[j - 1] { 0 } else { 1 };
                dp[i][j] = (dp[i - 1][j] + 1)
                    .min(dp[i][j - 1] + 1)
                    .min(dp[i - 1][j - 1] + cost);
                if i > 1 && j > 1 && ac[i - 1] == bc[j - 2] && ac[i - 2] == bc[j - 1] {
                    dp[i][j] = dp[i][j].min(dp[i - 2][j - 2] + cost);
                }
            }
        }
        dp[m][n]
    };

    // ─── Hamming distance (same-length strings only) ───
    let hamming = if m == n {
        let h = ac.iter().zip(bc.iter()).filter(|(a, b)| a != b).count();
        Some(h)
    } else {
        None
    };

    // ─── Jaro similarity ───
    let jaro = {
        if m == 0 && n == 0 {
            1.0f64
        } else if m == 0 || n == 0 {
            0.0f64
        } else {
            let match_dist = (m.max(n) / 2).saturating_sub(1);
            let mut s1_matches = vec![false; m];
            let mut s2_matches = vec![false; n];
            let mut matches = 0usize;
            for i in 0..m {
                let start = i.saturating_sub(match_dist);
                let end = (i + match_dist + 1).min(n);
                for j in start..end {
                    if !s2_matches[j] && ac[i] == bc[j] {
                        s1_matches[i] = true;
                        s2_matches[j] = true;
                        matches += 1;
                        break;
                    }
                }
            }
            if matches == 0 {
                0.0
            } else {
                let s1m: Vec<char> = ac
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| s1_matches[*i])
                    .map(|(_, c)| *c)
                    .collect();
                let s2m: Vec<char> = bc
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| s2_matches[*i])
                    .map(|(_, c)| *c)
                    .collect();
                let transpositions = s1m.iter().zip(s2m.iter()).filter(|(a, b)| a != b).count() / 2;
                let mf = matches as f64;
                (mf / m as f64 + mf / n as f64 + (mf - transpositions as f64) / mf) / 3.0
            }
        }
    };

    // ─── Jaro-Winkler ───
    let jaro_winkler = {
        let prefix_len = ac
            .iter()
            .zip(bc.iter())
            .take(4)
            .take_while(|(a, b)| a == b)
            .count();
        let p = 0.1f64;
        jaro + prefix_len as f64 * p * (1.0 - jaro)
    };
    let jaro_winkler = jaro_winkler.min(1.0);

    // ─── LCS length ───
    let lcs_len = {
        let mut dp = vec![vec![0usize; n + 1]; m + 1];
        for i in 1..=m {
            for j in 1..=n {
                if ac[i - 1] == bc[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }
        dp[m][n]
    };
    let lcs_sim = if max_len > 0 {
        lcs_len as f64 / max_len as f64
    } else {
        0.0
    };

    // ─── Longest common substring ───
    let (lcsub_len, lcsub) = {
        let mut best_len = 0usize;
        let mut best_end = 0usize;
        let mut dp = vec![vec![0usize; n + 1]; m + 1];
        for i in 1..=m {
            for j in 1..=n {
                if ac[i - 1] == bc[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                    if dp[i][j] > best_len {
                        best_len = dp[i][j];
                        best_end = i;
                    }
                }
            }
        }
        let substr: String = ac[best_end.saturating_sub(best_len)..best_end]
            .iter()
            .collect();
        (best_len, substr)
    };

    // ─── Output ───
    let _ = writeln!(out, "  ─── Edit Distance ───");
    let _ = writeln!(
        out,
        "  Levenshtein:            {:>6}  (similarity: {:.1}%)",
        lev_dist,
        lev_sim * 100.0
    );
    let _ = writeln!(
        out,
        "  Damerau-Levenshtein:    {:>6}  (allows transpositions)",
        dl_dist
    );
    if let Some(h) = hamming {
        let _ = writeln!(
            out,
            "  Hamming:                {:>6}  (substitutions only)",
            h
        );
    } else {
        let _ = writeln!(out, "  Hamming:          n/a (strings must be same length)");
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Similarity Scores (0=no match, 1=identical) ───");
    let _ = writeln!(
        out,
        "  Jaro:                 {:.6}  ({:.1}%)",
        jaro,
        jaro * 100.0
    );
    let _ = writeln!(
        out,
        "  Jaro-Winkler:         {:.6}  ({:.1}%)",
        jaro_winkler,
        jaro_winkler * 100.0
    );
    let _ = writeln!(
        out,
        "  LCS similarity:       {:.6}  ({:.1}%)",
        lcs_sim,
        lcs_sim * 100.0
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Common Subsequence / Substring ───");
    let _ = writeln!(
        out,
        "  LCS length:           {}  ({:.1}% of max length)",
        lcs_len,
        lcs_sim * 100.0
    );
    let _ = writeln!(
        out,
        "  Longest common sub:   \"{}\"  (length {})",
        lcsub, lcsub_len
    );

    let _ = writeln!(out, "{}", sep);
    out
}

// ─── Text statistics and readability analyzer ─────────────────────────────────

pub fn text_stats(input: &str) -> String {
    let mut out = String::new();
    let sep = "─".repeat(60);
    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  TEXT STATISTICS & READABILITY");
    let _ = writeln!(out, "{}", sep);

    let text = input.trim();
    if text.is_empty() {
        let _ = writeln!(out, "  No text provided. Pass text as the argument.");
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // ─── Basic counts ───
    let char_count = text.chars().count();
    let char_no_space = text.chars().filter(|c| !c.is_whitespace()).count();
    let byte_count = text.len();

    // Words: split on whitespace
    let words: Vec<&str> = text.split_whitespace().collect();
    let word_count = words.len();

    // Sentences: end with . ! ? (crude but accurate enough for readability)
    let sentence_count = text
        .chars()
        .filter(|&c| c == '.' || c == '!' || c == '?')
        .count()
        .max(1);

    // Paragraphs: blank lines
    let paragraph_count = text
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .count()
        .max(1);

    // ─── Syllable counting (heuristic: vowel groups) ───
    let count_syllables = |word: &str| -> usize {
        let w = word.to_lowercase();
        let w: String = w.chars().filter(|c| c.is_alphabetic()).collect();
        if w.is_empty() {
            return 0;
        }
        let vowels = "aeiouy";
        let mut count = 0usize;
        let mut prev_vowel = false;
        let chars: Vec<char> = w.chars().collect();
        for &ch in &chars {
            let is_v = vowels.contains(ch);
            if is_v && !prev_vowel {
                count += 1;
            }
            prev_vowel = is_v;
        }
        // Silent trailing 'e'
        if w.ends_with('e') && count > 1 {
            count = count.saturating_sub(1);
        }
        count.max(1)
    };

    let total_syllables: usize = words.iter().map(|w| count_syllables(w)).sum();
    let avg_syllables = if word_count > 0 {
        total_syllables as f64 / word_count as f64
    } else {
        0.0
    };

    // ─── Readability scores ───
    let words_per_sentence = if sentence_count > 0 {
        word_count as f64 / sentence_count as f64
    } else {
        0.0
    };
    let syllables_per_word = avg_syllables;

    // Flesch Reading Ease: 206.835 - 1.015*(words/sentences) - 84.6*(syllables/word)
    let flesch_ease = 206.835 - 1.015 * words_per_sentence - 84.6 * syllables_per_word;
    let flesch_ease = flesch_ease.clamp(0.0, 100.0);

    // Flesch-Kincaid Grade Level: 0.39*(words/sentences) + 11.8*(syllables/word) - 15.59
    let fk_grade = 0.39 * words_per_sentence + 11.8 * syllables_per_word - 15.59;
    let fk_grade = fk_grade.max(0.0);

    // Gunning Fog Index: 0.4 * ((words/sentences) + 100*(complex_words/words))
    // Complex = 3+ syllables, not proper nouns (crude: just count syllables >= 3)
    let complex_words = words.iter().filter(|w| count_syllables(w) >= 3).count();
    let fog_index =
        0.4 * (words_per_sentence + 100.0 * complex_words as f64 / word_count.max(1) as f64);

    // SMOG: sqrt(complex * (30 / sentences)) + 3
    let smog = (complex_words as f64 * 30.0 / sentence_count as f64).sqrt() + 3.0;

    // Coleman-Liau: 0.0588*L - 0.296*S - 15.8  (L=avg letters/100 words, S=avg sentences/100 words)
    let letters_per_100: f64 = char_no_space as f64 / word_count.max(1) as f64 * 100.0;
    let sent_per_100: f64 = sentence_count as f64 / word_count.max(1) as f64 * 100.0;
    let coleman_liau = 0.0588 * letters_per_100 - 0.296 * sent_per_100 - 15.8;

    // Flesch ease → grade label
    let ease_label = if flesch_ease >= 90.0 {
        "Very Easy (5th grade)"
    } else if flesch_ease >= 80.0 {
        "Easy (6th grade)"
    } else if flesch_ease >= 70.0 {
        "Fairly Easy (7th grade)"
    } else if flesch_ease >= 60.0 {
        "Standard (8th–9th grade)"
    } else if flesch_ease >= 50.0 {
        "Fairly Difficult (10th–12th grade)"
    } else if flesch_ease >= 30.0 {
        "Difficult (College)"
    } else {
        "Very Confusing (Professional)"
    };

    // ─── Word frequency (top 20 excluding common stop words) ───
    let stop_words: &[&str] = &[
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "is",
        "it", "its", "was", "are", "were", "be", "been", "being", "have", "has", "had", "do",
        "does", "did", "will", "would", "could", "should", "may", "might", "shall", "that", "this",
        "these", "those", "i", "you", "he", "she", "we", "they", "them", "his", "her", "our",
        "their", "my", "your", "as", "by", "from", "up", "about", "into", "through", "than",
        "more", "also", "if", "not", "so", "all", "can",
    ];
    let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for word in &words {
        let clean: String = word
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        if !clean.is_empty() && clean.len() > 1 && !stop_words.contains(&clean.as_str()) {
            *freq.entry(clean).or_insert(0) += 1;
        }
    }
    let mut freq_sorted: Vec<(String, usize)> = freq.into_iter().collect();
    freq_sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    // ─── Character frequency ───
    let mut char_freq: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    for ch in text.chars() {
        if ch.is_alphabetic() {
            *char_freq.entry(ch.to_ascii_lowercase()).or_insert(0) += 1;
        }
    }
    let mut char_sorted: Vec<(char, usize)> = char_freq.into_iter().collect();
    char_sorted.sort_by(|a, b| b.1.cmp(&a.1));

    // ─── Average word length ───
    let total_word_chars: usize = words
        .iter()
        .map(|w| w.chars().filter(|c| c.is_alphabetic()).count())
        .sum();
    let avg_word_len = if word_count > 0 {
        total_word_chars as f64 / word_count as f64
    } else {
        0.0
    };

    // ─── Longest words ───
    let mut word_lengths: Vec<(&&str, usize)> = words
        .iter()
        .map(|w| (w, w.chars().filter(|c| c.is_alphabetic()).count()))
        .collect();
    word_lengths.sort_by(|a, b| b.1.cmp(&a.1));
    word_lengths.dedup_by_key(|w| w.1);

    // ─── Output ───
    let _ = writeln!(out, "  ─── Basic Counts ───");
    let _ = writeln!(out, "  Characters (total):     {:>8}", char_count);
    let _ = writeln!(out, "  Characters (no spaces): {:>8}", char_no_space);
    let _ = writeln!(out, "  Bytes:                  {:>8}", byte_count);
    let _ = writeln!(out, "  Words:                  {:>8}", word_count);
    let _ = writeln!(out, "  Sentences:              {:>8}", sentence_count);
    let _ = writeln!(out, "  Paragraphs:             {:>8}", paragraph_count);
    let _ = writeln!(out, "  Syllables (est.):       {:>8}", total_syllables);
    let _ = writeln!(out, "  Complex words (3+ syl): {:>8}", complex_words);
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Averages ───");
    let _ = writeln!(out, "  Avg word length:        {:>8.2} chars", avg_word_len);
    let _ = writeln!(out, "  Avg syllables/word:     {:>8.2}", avg_syllables);
    let _ = writeln!(out, "  Avg words/sentence:     {:>8.2}", words_per_sentence);
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Readability Scores ───");
    let _ = writeln!(
        out,
        "  Flesch Reading Ease:    {:>8.1}  ({})",
        flesch_ease, ease_label
    );
    let _ = writeln!(
        out,
        "  Flesch-Kincaid Grade:   {:>8.1}  (Grade {:.0})",
        fk_grade, fk_grade
    );
    let _ = writeln!(
        out,
        "  Gunning Fog Index:      {:>8.1}  (Grade {:.0})",
        fog_index, fog_index
    );
    let _ = writeln!(
        out,
        "  SMOG Index:             {:>8.1}  (Grade {:.0})",
        smog, smog
    );
    let _ = writeln!(
        out,
        "  Coleman-Liau Index:     {:>8.1}  (Grade {:.0})",
        coleman_liau, coleman_liau
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Top Words (excluding stop words) ───");
    for (word, count) in freq_sorted.iter().take(20) {
        let bar: String = "█".repeat((count * 30 / freq_sorted[0].1.max(1)).min(30));
        let _ = writeln!(out, "  {:>4}x  {:<20}  {}", count, word, bar);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Letter Frequency ───");
    let total_letters: usize = char_sorted.iter().map(|&(_, c)| c).sum();
    for (ch, count) in char_sorted.iter().take(10) {
        let pct = *count as f64 / total_letters.max(1) as f64 * 100.0;
        let bar: String = "█".repeat((pct as usize * 2).min(40));
        let _ = writeln!(out, "  '{}': {:>5} ({:5.2}%)  {}", ch, count, pct, bar);
    }
    if !word_lengths.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "  ─── Longest Words ───");
        for (w, len) in word_lengths.iter().take(5) {
            let clean: String = w.chars().filter(|c| c.is_alphanumeric()).collect();
            let _ = writeln!(out, "  {} ({} chars)", clean, len);
        }
    }

    let _ = writeln!(out, "{}", sep);
    out
}

// ── Descriptive statistics ────────────────────────────────────────────────────

pub fn stats_calc(input: &str) -> String {
    let sep = "─".repeat(60);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n  ┌─ Descriptive Statistics ─────────────────────────┐"
    );

    // Parse numbers — accept space/comma/newline separated, ignore non-numeric tokens
    let numbers: Vec<f64> = input
        .split(|c: char| c == ',' || c == ' ' || c == '\n' || c == '\t')
        .filter_map(|tok| {
            let t = tok.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<f64>().ok()
            }
        })
        .collect();

    if numbers.is_empty() {
        let _ = writeln!(out, "  No numeric values found in input.");
        let _ = writeln!(out, "  Usage: hematite --stats '1,2,3,4,5'");
        let _ = writeln!(out, "         hematite --stats '10 20 30 40 50'");
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    let n = numbers.len();
    let mut sorted = numbers.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let sum: f64 = numbers.iter().sum();
    let mean = sum / n as f64;
    let min = sorted[0];
    let max = sorted[n - 1];
    let range = max - min;

    // Median
    let median = if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    };

    // Mode (most frequent value)
    let mut freq_map: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    for &v in &numbers {
        *freq_map.entry((v * 1e9) as i64).or_insert(0) += 1;
    }
    let max_freq = *freq_map.values().max().unwrap_or(&0);
    let mut modes: Vec<f64> = freq_map
        .iter()
        .filter(|(_, &c)| c == max_freq)
        .map(|(&k, _)| k as f64 * 1e-9)
        .collect();
    modes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mode_str = if modes.len() == n {
        "none (all unique)".to_string()
    } else {
        modes
            .iter()
            .map(|v| {
                let s = format!("{:.4}", v);
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    // Variance and std dev
    let pop_variance: f64 = numbers.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let samp_variance: f64 = if n > 1 {
        numbers.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64
    } else {
        0.0
    };
    let pop_std = pop_variance.sqrt();
    let samp_std = samp_variance.sqrt();

    // Percentile helper
    let percentile = |p: f64| -> f64 {
        let idx = p / 100.0 * (n - 1) as f64;
        let lo = idx.floor() as usize;
        let hi = (lo + 1).min(n - 1);
        let frac = idx - lo as f64;
        sorted[lo] + frac * (sorted[hi] - sorted[lo])
    };

    let q1 = percentile(25.0);
    let q3 = percentile(75.0);
    let iqr = q3 - q1;

    // Skewness (Fisher)
    let skewness = if pop_std > 0.0 {
        numbers
            .iter()
            .map(|v| ((v - mean) / pop_std).powi(3))
            .sum::<f64>()
            / n as f64
    } else {
        0.0
    };

    // Excess kurtosis
    let kurtosis = if pop_std > 0.0 {
        numbers
            .iter()
            .map(|v| ((v - mean) / pop_std).powi(4))
            .sum::<f64>()
            / n as f64
            - 3.0
    } else {
        0.0
    };

    // Tukey outliers (beyond 1.5 * IQR)
    let lower_fence = q1 - 1.5 * iqr;
    let upper_fence = q3 + 1.5 * iqr;
    let outliers: Vec<f64> = sorted
        .iter()
        .copied()
        .filter(|&v| v < lower_fence || v > upper_fence)
        .collect();

    let _ = writeln!(out, "{}", sep);
    let _ = writeln!(out, "  ─── Sample Overview ───");
    let _ = writeln!(out, "  Count:     {:>12}", n);
    let _ = writeln!(out, "  Sum:       {:>12.4}", sum);
    let _ = writeln!(out, "  Min:       {:>12.4}", min);
    let _ = writeln!(out, "  Max:       {:>12.4}", max);
    let _ = writeln!(out, "  Range:     {:>12.4}", range);
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Central Tendency ───");
    let _ = writeln!(out, "  Mean:      {:>12.6}", mean);
    let _ = writeln!(out, "  Median:    {:>12.6}", median);
    let _ = writeln!(out, "  Mode:      {}", mode_str);
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Dispersion ───");
    let _ = writeln!(out, "  Pop Var:   {:>12.6}", pop_variance);
    let _ = writeln!(out, "  Samp Var:  {:>12.6}", samp_variance);
    let _ = writeln!(out, "  Pop SD:    {:>12.6}", pop_std);
    let _ = writeln!(out, "  Samp SD:   {:>12.6}", samp_std);
    let _ = writeln!(
        out,
        "  CV (%):    {:>12.4}",
        if mean.abs() > 1e-12 {
            samp_std / mean.abs() * 100.0
        } else {
            0.0
        }
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Percentiles ───");
    let _ = writeln!(out, "  P5:        {:>12.4}", percentile(5.0));
    let _ = writeln!(out, "  P10:       {:>12.4}", percentile(10.0));
    let _ = writeln!(out, "  Q1 (P25):  {:>12.4}", q1);
    let _ = writeln!(out, "  Q2 (P50):  {:>12.4}", median);
    let _ = writeln!(out, "  Q3 (P75):  {:>12.4}", q3);
    let _ = writeln!(out, "  P90:       {:>12.4}", percentile(90.0));
    let _ = writeln!(out, "  P95:       {:>12.4}", percentile(95.0));
    let _ = writeln!(out, "  P99:       {:>12.4}", percentile(99.0));
    let _ = writeln!(out, "  IQR:       {:>12.4}", iqr);
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Shape ───");
    let _ = writeln!(
        out,
        "  Skewness:  {:>12.6}  ({})",
        skewness,
        if skewness.abs() < 0.5 {
            "symmetric"
        } else if skewness > 0.0 {
            "right-skewed"
        } else {
            "left-skewed"
        }
    );
    let _ = writeln!(
        out,
        "  Kurtosis:  {:>12.6}  (excess; {} tails)",
        kurtosis,
        if kurtosis.abs() < 0.5 {
            "normal"
        } else if kurtosis > 0.0 {
            "heavy"
        } else {
            "light"
        }
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Outlier Detection (Tukey 1.5×IQR) ───");
    let _ = writeln!(out, "  Fences:    [{:.4}, {:.4}]", lower_fence, upper_fence);
    if outliers.is_empty() {
        let _ = writeln!(out, "  Outliers:  none");
    } else {
        let outlier_strs: Vec<String> = outliers.iter().map(|v| format!("{:.4}", v)).collect();
        let _ = writeln!(out, "  Outliers:  {}", outlier_strs.join(", "));
    }

    // ASCII histogram (10 bins)
    let _ = writeln!(out);
    let _ = writeln!(out, "  ─── Histogram (10 bins) ───");
    let bins = 10usize;
    let bin_width = if range > 0.0 {
        range / bins as f64
    } else {
        1.0
    };
    let mut counts = vec![0usize; bins];
    for &v in &numbers {
        let idx = ((v - min) / bin_width).floor() as usize;
        let idx = idx.min(bins - 1);
        counts[idx] += 1;
    }
    let max_count = *counts.iter().max().unwrap_or(&1);
    for (i, &c) in counts.iter().enumerate() {
        let lo = min + i as f64 * bin_width;
        let hi = lo + bin_width;
        let bar_len = if max_count > 0 { c * 30 / max_count } else { 0 };
        let bar: String = "█".repeat(bar_len);
        let _ = writeln!(out, "  [{:>8.2}, {:>8.2})  {:>4}  {}", lo, hi, c, bar);
    }

    let _ = writeln!(out, "{}", sep);
    out
}

// ── Hash / checksum ───────────────────────────────────────────────────────────

fn sha1_digest(data: &[u8]) -> [u8; 20] {
    // Pure-Rust SHA-1 (RFC 3174) — avoids an extra crate dependency
    const K: [u32; 4] = [0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6];
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), K[0]),
                20..=39 => (b ^ c ^ d, K[1]),
                40..=59 => ((b & c) | (b & d) | (c & d), K[2]),
                _ => (b ^ c ^ d, K[3]),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for i in 0..5 {
        out[i * 4..(i + 1) * 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

pub fn hash_calc(input: &str) -> String {
    use md5::Md5;
    use sha2::{Sha256, Sha512};

    let sep = "─".repeat(60);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n  ┌─ Hash / Digest Calculator ───────────────────────┐"
    );
    let _ = writeln!(out, "{}", sep);

    // Check if input looks like a file path that exists
    let trimmed = input.trim();
    let (label, bytes) = {
        let path = std::path::Path::new(trimmed);
        if path.exists() && path.is_file() {
            match std::fs::read(path) {
                Ok(b) => (format!("file: {}", trimmed), b),
                Err(e) => {
                    let _ = writeln!(out, "  Error reading file: {}", e);
                    let _ = writeln!(out, "{}", sep);
                    return out;
                }
            }
        } else {
            (
                format!("string: {:?}", trimmed),
                trimmed.as_bytes().to_vec(),
            )
        }
    };

    let _ = writeln!(out, "  Input: {}", label);
    let _ = writeln!(out, "  Size:  {} bytes", bytes.len());
    let _ = writeln!(out);

    // MD5
    let mut md5h = Md5::new();
    md5h.update(&bytes);
    let md5_result = md5h.finalize();
    let md5_hex: String = md5_result.iter().map(|b| format!("{:02x}", b)).collect();

    // SHA-1 (inline)
    let sha1_bytes = sha1_digest(&bytes);
    let sha1_hex: String = sha1_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    // SHA-256
    let mut sha256h = Sha256::new();
    sha256h.update(&bytes);
    let sha256_result = sha256h.finalize();
    let sha256_hex: String = sha256_result.iter().map(|b| format!("{:02x}", b)).collect();

    // SHA-512
    let mut sha512h = Sha512::new();
    sha512h.update(&bytes);
    let sha512_result = sha512h.finalize();
    let sha512_hex: String = sha512_result.iter().map(|b| format!("{:02x}", b)).collect();

    let _ = writeln!(out, "  ─── Cryptographic Hashes ───");
    let _ = writeln!(out, "  MD5     ({:>3} bits):  {}", 128, md5_hex);
    let _ = writeln!(out, "  SHA-1   ({:>3} bits):  {}", 160, sha1_hex);
    let _ = writeln!(out, "  SHA-256 ({:>3} bits):  {}", 256, sha256_hex);
    let _ = writeln!(out, "  SHA-512 ({:>3} bits):  {}", 512, sha512_hex);
    let _ = writeln!(out);

    // Non-cryptographic checksums
    let crc32_val = {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &b in &bytes {
            crc ^= b as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB8_8320;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    };
    let adler32_val = {
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in &bytes {
            a = (a + byte as u32) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    };
    let fnv1a_val: u32 = bytes
        .iter()
        .fold(2166136261u32, |h, &b| h.wrapping_mul(16777619) ^ b as u32);

    let _ = writeln!(out, "  ─── Non-Cryptographic Checksums ───");
    let _ = writeln!(out, "  CRC-32:    {:08X}  ({})", crc32_val, crc32_val);
    let _ = writeln!(out, "  Adler-32:  {:08X}  ({})", adler32_val, adler32_val);
    let _ = writeln!(out, "  FNV-1a:    {:08X}  ({})", fnv1a_val, fnv1a_val);

    let _ = writeln!(out, "{}", sep);
    out
}

// ── Encoding / decoding ───────────────────────────────────────────────────────

fn encode_usage() -> String {
    let mut s = String::new();
    let _ = writeln!(s, "  Usage: hematite --encode '<text>'");
    let _ = writeln!(s, "         hematite --encode 'base64 encode Hello World'");
    let _ = writeln!(
        s,
        "         hematite --encode 'base64 decode SGVsbG8gV29ybGQ='"
    );
    let _ = writeln!(s, "         hematite --encode 'hex encode Hello'");
    let _ = writeln!(s, "         hematite --encode 'hex decode 48656c6c6f'");
    let _ = writeln!(
        s,
        "         hematite --encode 'url encode https://example.com/foo bar'"
    );
    let _ = writeln!(
        s,
        "         hematite --encode 'url decode https%3A%2F%2Fexample.com'"
    );
    let _ = writeln!(s, "         hematite --encode 'html encode <b>bold</b>'");
    let _ = writeln!(s, "         hematite --encode 'binary Hello'");
    let _ = writeln!(s, "         hematite --encode 'rot13 Hello World'");
    s
}

pub fn encode_calc(query: &str) -> String {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};

    let sep = "─".repeat(60);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n  ┌─ Encoding / Decoding ────────────────────────────┐"
    );
    let _ = writeln!(out, "{}", sep);

    let q = query.trim();
    let lower = q.to_lowercase();

    // Detect mode keyword
    let (mode, payload) = if lower.starts_with("base64 encode ") {
        ("base64-enc", &q["base64 encode ".len()..])
    } else if lower.starts_with("base64 decode ") {
        ("base64-dec", &q["base64 decode ".len()..])
    } else if lower.starts_with("hex encode ") {
        ("hex-enc", &q["hex encode ".len()..])
    } else if lower.starts_with("hex decode ") {
        ("hex-dec", &q["hex decode ".len()..])
    } else if lower.starts_with("url encode ") {
        ("url-enc", &q["url encode ".len()..])
    } else if lower.starts_with("url decode ") {
        ("url-dec", &q["url decode ".len()..])
    } else if lower.starts_with("html encode ") {
        ("html-enc", &q["html encode ".len()..])
    } else if lower.starts_with("html decode ") {
        ("html-dec", &q["html decode ".len()..])
    } else if lower.starts_with("binary ") {
        ("binary", &q["binary ".len()..])
    } else if lower.starts_with("rot13 ") {
        ("rot13", &q["rot13 ".len()..])
    } else {
        ("all", q)
    };

    match mode {
        "base64-enc" => {
            let encoded = B64.encode(payload.as_bytes());
            let _ = writeln!(out, "  Base64 Encode");
            let _ = writeln!(out, "  Input:   {}", payload);
            let _ = writeln!(out, "  Output:  {}", encoded);
        }
        "base64-dec" => {
            let _ = writeln!(out, "  Base64 Decode");
            let _ = writeln!(out, "  Input:   {}", payload);
            match B64.decode(payload.trim()) {
                Ok(b) => match std::str::from_utf8(&b) {
                    Ok(s) => {
                        let _ = writeln!(out, "  Output:  {}", s);
                    }
                    Err(_) => {
                        let _ = writeln!(out, "  Output:  {} bytes (binary)", b.len());
                    }
                },
                Err(e) => {
                    let _ = writeln!(out, "  Error:   {}", e);
                }
            }
        }
        "hex-enc" => {
            let hex: String = payload.bytes().map(|b| format!("{:02x}", b)).collect();
            let _ = writeln!(out, "  Hex Encode");
            let _ = writeln!(out, "  Input:   {}", payload);
            let _ = writeln!(out, "  Output:  {}", hex);
        }
        "hex-dec" => {
            let _ = writeln!(out, "  Hex Decode");
            let _ = writeln!(out, "  Input:   {}", payload);
            let cleaned: String = payload.chars().filter(|c| c.is_ascii_hexdigit()).collect();
            if cleaned.len() % 2 != 0 {
                let _ = writeln!(out, "  Error:   odd number of hex digits");
            } else {
                let bytes: Result<Vec<u8>, _> = (0..cleaned.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16))
                    .collect();
                match bytes {
                    Ok(b) => match std::str::from_utf8(&b) {
                        Ok(s) => {
                            let _ = writeln!(out, "  Output:  {}", s);
                        }
                        Err(_) => {
                            let _ = writeln!(out, "  Output:  {} bytes (binary)", b.len());
                        }
                    },
                    Err(e) => {
                        let _ = writeln!(out, "  Error:   {}", e);
                    }
                }
            }
        }
        "url-enc" => {
            let encoded = utf8_percent_encode(payload, NON_ALPHANUMERIC).to_string();
            let _ = writeln!(out, "  URL Encode");
            let _ = writeln!(out, "  Input:   {}", payload);
            let _ = writeln!(out, "  Output:  {}", encoded);
        }
        "url-dec" => {
            let _ = writeln!(out, "  URL Decode");
            let _ = writeln!(out, "  Input:   {}", payload);
            match percent_decode_str(payload).decode_utf8() {
                Ok(s) => {
                    let _ = writeln!(out, "  Output:  {}", s);
                }
                Err(e) => {
                    let _ = writeln!(out, "  Error:   {}", e);
                }
            }
        }
        "html-enc" => {
            let encoded = payload
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#39;");
            let _ = writeln!(out, "  HTML Encode");
            let _ = writeln!(out, "  Input:   {}", payload);
            let _ = writeln!(out, "  Output:  {}", encoded);
        }
        "html-dec" => {
            let decoded = payload
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&apos;", "'")
                .replace("&nbsp;", " ");
            let _ = writeln!(out, "  HTML Decode");
            let _ = writeln!(out, "  Input:   {}", payload);
            let _ = writeln!(out, "  Output:  {}", decoded);
        }
        "binary" => {
            let bin: String = payload
                .bytes()
                .map(|b| format!("{:08b}", b))
                .collect::<Vec<_>>()
                .join(" ");
            let _ = writeln!(out, "  Binary Representation");
            let _ = writeln!(out, "  Input:   {}", payload);
            let _ = writeln!(out, "  Output:  {}", bin);
        }
        "rot13" => {
            let rotated: String = payload
                .chars()
                .map(|c| match c {
                    'a'..='m' | 'A'..='M' => (c as u8 + 13) as char,
                    'n'..='z' | 'N'..='Z' => (c as u8 - 13) as char,
                    _ => c,
                })
                .collect();
            let _ = writeln!(out, "  ROT13");
            let _ = writeln!(out, "  Input:   {}", payload);
            let _ = writeln!(out, "  Output:  {}", rotated);
        }
        _ => {
            // All-formats mode
            if q.is_empty() {
                let _ = writeln!(out, "{}", encode_usage());
                let _ = writeln!(out, "{}", sep);
                return out;
            }
            let b64 = B64.encode(q.as_bytes());
            let hex: String = q.bytes().map(|b| format!("{:02x}", b)).collect();
            let url = utf8_percent_encode(q, NON_ALPHANUMERIC).to_string();
            let html = q
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");
            let bin: String = q
                .bytes()
                .map(|b| format!("{:08b}", b))
                .collect::<Vec<_>>()
                .join(" ");
            let rot: String = q
                .chars()
                .map(|c| match c {
                    'a'..='m' | 'A'..='M' => (c as u8 + 13) as char,
                    'n'..='z' | 'N'..='Z' => (c as u8 - 13) as char,
                    _ => c,
                })
                .collect();
            let _ = writeln!(out, "  Input:     {}", q);
            let _ = writeln!(
                out,
                "  Length:    {} chars / {} bytes",
                q.chars().count(),
                q.len()
            );
            let _ = writeln!(out);
            let _ = writeln!(out, "  Base64:    {}", b64);
            let _ = writeln!(out, "  Hex:       {}", hex);
            let _ = writeln!(out, "  URL:       {}", url);
            let _ = writeln!(out, "  HTML:      {}", html);
            let _ = writeln!(out, "  Binary:    {}", bin);
            let _ = writeln!(out, "  ROT13:     {}", rot);
        }
    }

    let _ = writeln!(out, "{}", sep);
    out
}

// ── Geometry ──────────────────────────────────────────────────────────────────

pub fn geometry_calc(query: &str) -> String {
    let sep = "─".repeat(60);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n  ┌─ Geometry Calculator ────────────────────────────┐"
    );
    let _ = writeln!(out, "{}", sep);

    let q = query.trim().to_lowercase();
    let words: Vec<&str> = q.split_whitespace().collect();

    if words.is_empty() {
        let _ = writeln!(out, "  Usage examples:");
        let _ = writeln!(out, "    hematite --geometry 'circle r=5'");
        let _ = writeln!(out, "    hematite --geometry 'triangle a=3 b=4 c=5'");
        let _ = writeln!(out, "    hematite --geometry 'rectangle w=6 h=4'");
        let _ = writeln!(out, "    hematite --geometry 'sphere r=3'");
        let _ = writeln!(out, "    hematite --geometry 'cylinder r=2 h=10'");
        let _ = writeln!(out, "    hematite --geometry 'cone r=3 h=4'");
        let _ = writeln!(out, "    hematite --geometry 'box l=3 w=4 h=5'");
        let _ = writeln!(out, "    hematite --geometry 'ellipse a=5 b=3'");
        let _ = writeln!(out, "    hematite --geometry 'polygon n=6 s=4'");
        let _ = writeln!(
            out,
            "    hematite --geometry 'distance x1=0 y1=0 x2=3 y2=4'"
        );
        let _ = writeln!(out, "    hematite --geometry 'pythagorean a=3 b=4'");
        let _ = writeln!(out, "    hematite --geometry 'degrees 1.5708'");
        let _ = writeln!(out, "    hematite --geometry 'radians 90'");
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // Parse key=value pairs
    let parse_kv = |key: &str| -> Option<f64> {
        for w in &words {
            if w.starts_with(&format!("{}=", key)) {
                return w[key.len() + 1..].parse().ok();
            }
        }
        None
    };

    let pi = std::f64::consts::PI;
    let keyword = words[0];

    match keyword {
        "circle" => {
            if let Some(r) = parse_kv("r") {
                let area = pi * r * r;
                let circumference = 2.0 * pi * r;
                let _ = writeln!(out, "  Circle  r = {}", r);
                let _ = writeln!(out, "  Area:         {:>14.6}", area);
                let _ = writeln!(out, "  Circumference:{:>14.6}", circumference);
                let _ = writeln!(out, "  Diameter:     {:>14.6}", 2.0 * r);
            } else if let Some(d) = parse_kv("d") {
                let r = d / 2.0;
                let area = pi * r * r;
                let _ = writeln!(out, "  Circle  d = {}", d);
                let _ = writeln!(out, "  Radius:       {:>14.6}", r);
                let _ = writeln!(out, "  Area:         {:>14.6}", area);
                let _ = writeln!(out, "  Circumference:{:>14.6}", 2.0 * pi * r);
            } else {
                let _ = writeln!(out, "  Provide r=<radius> or d=<diameter>");
            }
        }
        "triangle" => {
            let a = parse_kv("a");
            let b = parse_kv("b");
            let c = parse_kv("c");
            let base = parse_kv("base");
            let height = parse_kv("h").or(parse_kv("height"));

            if let (Some(b_val), Some(h_val)) = (base, height) {
                let area = 0.5 * b_val * h_val;
                let _ = writeln!(out, "  Triangle  base = {}, height = {}", b_val, h_val);
                let _ = writeln!(out, "  Area:         {:>14.6}", area);
            } else if let (Some(a), Some(b), Some(c)) = (a, b, c) {
                let s = (a + b + c) / 2.0;
                let area_sq = s * (s - a) * (s - b) * (s - c);
                let _ = writeln!(out, "  Triangle  a={}, b={}, c={}", a, b, c);
                if area_sq >= 0.0 {
                    let area = area_sq.sqrt();
                    let _ = writeln!(out, "  Area (Heron):  {:>13.6}", area);
                    let _ = writeln!(out, "  Perimeter:     {:>13.6}", a + b + c);
                    let cos_a = (b * b + c * c - a * a) / (2.0 * b * c);
                    let cos_b = (a * a + c * c - b * b) / (2.0 * a * c);
                    let cos_c = (a * a + b * b - c * c) / (2.0 * a * b);
                    let ang_a = cos_a.clamp(-1.0, 1.0).acos().to_degrees();
                    let ang_b = cos_b.clamp(-1.0, 1.0).acos().to_degrees();
                    let ang_c = cos_c.clamp(-1.0, 1.0).acos().to_degrees();
                    let _ = writeln!(out, "  Angle A:       {:>13.4}°", ang_a);
                    let _ = writeln!(out, "  Angle B:       {:>13.4}°", ang_b);
                    let _ = writeln!(out, "  Angle C:       {:>13.4}°", ang_c);
                    let tri_type = if (ang_a - 90.0).abs() < 0.001
                        || (ang_b - 90.0).abs() < 0.001
                        || (ang_c - 90.0).abs() < 0.001
                    {
                        "right"
                    } else if ang_a > 90.0 || ang_b > 90.0 || ang_c > 90.0 {
                        "obtuse"
                    } else {
                        "acute"
                    };
                    let eq_type = if (a - b).abs() < 1e-9 && (b - c).abs() < 1e-9 {
                        "equilateral"
                    } else if (a - b).abs() < 1e-9 || (b - c).abs() < 1e-9 || (a - c).abs() < 1e-9 {
                        "isosceles"
                    } else {
                        "scalene"
                    };
                    let _ = writeln!(out, "  Type:          {} {}", tri_type, eq_type);
                } else {
                    let _ = writeln!(
                        out,
                        "  Invalid triangle (sides violate triangle inequality)"
                    );
                }
            } else {
                let _ = writeln!(out, "  Provide a=, b=, c= (SSS) or base= h= (base-height)");
            }
        }
        "rectangle" | "rect" => {
            let w = parse_kv("w").or(parse_kv("width"));
            let h = parse_kv("h").or(parse_kv("height"));
            if let (Some(w), Some(h)) = (w, h) {
                let _ = writeln!(out, "  Rectangle  w={}, h={}", w, h);
                let _ = writeln!(out, "  Area:         {:>14.6}", w * h);
                let _ = writeln!(out, "  Perimeter:    {:>14.6}", 2.0 * (w + h));
                let _ = writeln!(out, "  Diagonal:     {:>14.6}", (w * w + h * h).sqrt());
            } else {
                let _ = writeln!(out, "  Provide w=<width> h=<height>");
            }
        }
        "square" => {
            if let Some(s) = parse_kv("s").or(parse_kv("side")) {
                let _ = writeln!(out, "  Square  s={}", s);
                let _ = writeln!(out, "  Area:         {:>14.6}", s * s);
                let _ = writeln!(out, "  Perimeter:    {:>14.6}", 4.0 * s);
                let _ = writeln!(out, "  Diagonal:     {:>14.6}", s * 2.0_f64.sqrt());
            } else {
                let _ = writeln!(out, "  Provide s=<side>");
            }
        }
        "ellipse" => {
            let a = parse_kv("a");
            let b = parse_kv("b");
            if let (Some(a), Some(b)) = (a, b) {
                let h = ((a - b) / (a + b)).powi(2);
                let circumference =
                    pi * (a + b) * (1.0 + 3.0 * h / (10.0 + (4.0 - 3.0 * h).sqrt()));
                let _ = writeln!(out, "  Ellipse  a={}, b={}", a, b);
                let _ = writeln!(out, "  Area:         {:>14.6}", pi * a * b);
                let _ = writeln!(
                    out,
                    "  Circumference:{:>14.6}  (Ramanujan approx)",
                    circumference
                );
                let _ = writeln!(
                    out,
                    "  Eccentricity: {:>14.6}",
                    (1.0 - (b / a).powi(2)).sqrt()
                );
            } else {
                let _ = writeln!(out, "  Provide a=<semi-major> b=<semi-minor>");
            }
        }
        "polygon" => {
            let n_val = parse_kv("n").map(|v| v as u32);
            let s = parse_kv("s").or(parse_kv("side"));
            if let (Some(n), Some(s)) = (n_val, s) {
                let interior_angle = (n - 2) as f64 * 180.0 / n as f64;
                let area = (n as f64 * s * s) / (4.0 * (pi / n as f64).tan());
                let _ = writeln!(out, "  Regular Polygon  n={}, side={}", n, s);
                let _ = writeln!(out, "  Perimeter:     {:>13.6}", n as f64 * s);
                let _ = writeln!(out, "  Area:          {:>13.6}", area);
                let _ = writeln!(out, "  Interior angle:{:>13.4}°", interior_angle);
                let _ = writeln!(out, "  Exterior angle:{:>13.4}°", 360.0 / n as f64);
                let _ = writeln!(
                    out,
                    "  Circumradius:  {:>13.6}",
                    s / (2.0 * (pi / n as f64).sin())
                );
                let _ = writeln!(
                    out,
                    "  Inradius:      {:>13.6}",
                    s / (2.0 * (pi / n as f64).tan())
                );
            } else {
                let _ = writeln!(out, "  Provide n=<sides> s=<side_length>");
            }
        }
        "sphere" => {
            if let Some(r) = parse_kv("r") {
                let _ = writeln!(out, "  Sphere  r={}", r);
                let _ = writeln!(
                    out,
                    "  Volume:       {:>14.6}",
                    (4.0 / 3.0) * pi * r.powi(3)
                );
                let _ = writeln!(out, "  Surface area: {:>14.6}", 4.0 * pi * r * r);
                let _ = writeln!(out, "  Diameter:     {:>14.6}", 2.0 * r);
            } else {
                let _ = writeln!(out, "  Provide r=<radius>");
            }
        }
        "cylinder" => {
            let r = parse_kv("r");
            let h = parse_kv("h");
            if let (Some(r), Some(h)) = (r, h) {
                let _ = writeln!(out, "  Cylinder  r={}, h={}", r, h);
                let _ = writeln!(out, "  Volume:       {:>14.6}", pi * r * r * h);
                let _ = writeln!(out, "  Lateral area: {:>14.6}", 2.0 * pi * r * h);
                let _ = writeln!(out, "  Total area:   {:>14.6}", 2.0 * pi * r * (r + h));
            } else {
                let _ = writeln!(out, "  Provide r=<radius> h=<height>");
            }
        }
        "cone" => {
            let r = parse_kv("r");
            let h = parse_kv("h");
            if let (Some(r), Some(h)) = (r, h) {
                let slant = (r * r + h * h).sqrt();
                let _ = writeln!(out, "  Cone  r={}, h={}", r, h);
                let _ = writeln!(
                    out,
                    "  Volume:       {:>14.6}",
                    (1.0 / 3.0) * pi * r * r * h
                );
                let _ = writeln!(out, "  Slant height: {:>14.6}", slant);
                let _ = writeln!(out, "  Lateral area: {:>14.6}", pi * r * slant);
                let _ = writeln!(out, "  Total area:   {:>14.6}", pi * r * (r + slant));
            } else {
                let _ = writeln!(out, "  Provide r=<radius> h=<height>");
            }
        }
        "box" | "cuboid" | "rectangular" => {
            let l = parse_kv("l").or(parse_kv("length"));
            let w = parse_kv("w").or(parse_kv("width"));
            let h = parse_kv("h").or(parse_kv("height"));
            if let (Some(l), Some(w), Some(h)) = (l, w, h) {
                let _ = writeln!(out, "  Box  l={}, w={}, h={}", l, w, h);
                let _ = writeln!(out, "  Volume:       {:>14.6}", l * w * h);
                let _ = writeln!(
                    out,
                    "  Surface area: {:>14.6}",
                    2.0 * (l * w + w * h + l * h)
                );
                let _ = writeln!(
                    out,
                    "  Space diag:   {:>14.6}",
                    (l * l + w * w + h * h).sqrt()
                );
            } else {
                let _ = writeln!(out, "  Provide l=<length> w=<width> h=<height>");
            }
        }
        "pythagorean" | "pythagoras" | "pyth" => {
            let a = parse_kv("a");
            let b = parse_kv("b");
            let c = parse_kv("c");
            match (a, b, c) {
                (Some(a), Some(b), None) => {
                    let c = (a * a + b * b).sqrt();
                    let _ = writeln!(out, "  Pythagorean  a={}, b={}", a, b);
                    let _ = writeln!(out, "  c = √(a²+b²) = {:>10.6}", c);
                }
                (Some(a), None, Some(c)) => {
                    if c > a {
                        let b = (c * c - a * a).sqrt();
                        let _ = writeln!(out, "  Pythagorean  a={}, c={}", a, c);
                        let _ = writeln!(out, "  b = √(c²-a²) = {:>10.6}", b);
                    } else {
                        let _ = writeln!(out, "  Error: hypotenuse c must be greater than leg a");
                    }
                }
                (None, Some(b), Some(c)) => {
                    if c > b {
                        let a = (c * c - b * b).sqrt();
                        let _ = writeln!(out, "  Pythagorean  b={}, c={}", b, c);
                        let _ = writeln!(out, "  a = √(c²-b²) = {:>10.6}", a);
                    } else {
                        let _ = writeln!(out, "  Error: hypotenuse c must be greater than leg b");
                    }
                }
                _ => {
                    let _ = writeln!(out, "  Provide two of: a=, b= (legs), c= (hypotenuse)");
                }
            }
        }
        "distance" | "dist" => {
            let x1 = parse_kv("x1");
            let y1 = parse_kv("y1");
            let x2 = parse_kv("x2");
            let y2 = parse_kv("y2");
            if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (x1, y1, x2, y2) {
                let d = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
                let mid_x = (x1 + x2) / 2.0;
                let mid_y = (y1 + y2) / 2.0;
                let slope = if (x2 - x1).abs() > 1e-12 {
                    (y2 - y1) / (x2 - x1)
                } else {
                    f64::INFINITY
                };
                let _ = writeln!(out, "  Point Distance  ({}, {}) → ({}, {})", x1, y1, x2, y2);
                let _ = writeln!(out, "  Distance:     {:>14.6}", d);
                let _ = writeln!(out, "  Midpoint:     ({:.4}, {:.4})", mid_x, mid_y);
                let _ = writeln!(out, "  Slope:        {:>14.6}", slope);
            } else {
                let _ = writeln!(out, "  Provide x1=, y1=, x2=, y2=");
            }
        }
        "degrees" | "deg" => {
            if let Ok(rad) = words.get(1).unwrap_or(&"").parse::<f64>() {
                let _ = writeln!(out, "  {} radians = {:.6}°", rad, rad.to_degrees());
            } else {
                let _ = writeln!(
                    out,
                    "  Provide angle in radians: hematite --geometry 'degrees 1.5708'"
                );
            }
        }
        "radians" | "rad" => {
            if let Ok(deg) = words.get(1).unwrap_or(&"").parse::<f64>() {
                let _ = writeln!(out, "  {}° = {:.6} radians", deg, deg.to_radians());
            } else {
                let _ = writeln!(
                    out,
                    "  Provide angle in degrees: hematite --geometry 'radians 90'"
                );
            }
        }
        _ => {
            let _ = writeln!(out, "  Unknown shape or operation: '{}'", keyword);
            let _ = writeln!(
                out,
                "  Supported: circle, triangle, rectangle, square, ellipse,"
            );
            let _ = writeln!(
                out,
                "             polygon, sphere, cylinder, cone, box, pythagorean,"
            );
            let _ = writeln!(out, "             distance, degrees, radians");
        }
    }

    let _ = writeln!(out, "{}", sep);
    out
}

// ── Electrical engineering ────────────────────────────────────────────────────

fn parse_si_value(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(v) = s.parse::<f64>() {
        return Some(v);
    }
    // Strip trailing unit letters then check SI prefix
    let (num_part, suffix) = {
        let last = s.chars().last()?;
        if last.is_alphabetic() || last == 'Ω' {
            let num_end = s
                .rfind(|c: char| c.is_ascii_digit() || c == '.' || c == '-')
                .unwrap_or(0)
                + 1;
            (&s[..num_end], &s[num_end..])
        } else {
            (s, "")
        }
    };
    let base: f64 = num_part.parse().ok()?;
    let multiplier = match suffix.chars().next().unwrap_or(' ') {
        'T' => 1e12,
        'G' => 1e9,
        'M' => 1e6,
        'k' | 'K' => 1e3,
        'm' => 1e-3,
        'u' | 'µ' => 1e-6,
        'n' => 1e-9,
        'p' => 1e-12,
        _ => 1.0,
    };
    Some(base * multiplier)
}

fn fmt_si(v: f64, unit: &str) -> String {
    let abs = v.abs();
    let (scaled, prefix) = if abs >= 1e12 {
        (v / 1e12, "T")
    } else if abs >= 1e9 {
        (v / 1e9, "G")
    } else if abs >= 1e6 {
        (v / 1e6, "M")
    } else if abs >= 1e3 {
        (v / 1e3, "k")
    } else if abs >= 1.0 {
        (v, "")
    } else if abs >= 1e-3 {
        (v * 1e3, "m")
    } else if abs >= 1e-6 {
        (v * 1e6, "µ")
    } else if abs >= 1e-9 {
        (v * 1e9, "n")
    } else {
        (v * 1e12, "p")
    };
    format!("{:.4} {}{}", scaled, prefix, unit)
}

pub fn electrical_calc(query: &str) -> String {
    let sep = "─".repeat(60);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n  ┌─ Electrical Engineering Calculator ──────────────┐"
    );
    let _ = writeln!(out, "{}", sep);

    let q = query.trim().to_lowercase();
    let words: Vec<&str> = q.split_whitespace().collect();

    if words.is_empty() {
        let _ = writeln!(out, "  Usage examples:");
        let _ = writeln!(out, "    hematite --electrical 'ohm V=12 R=100'");
        let _ = writeln!(out, "    hematite --electrical 'ohm V=5 I=0.02'");
        let _ = writeln!(out, "    hematite --electrical 'ohm I=2 R=50'");
        let _ = writeln!(out, "    hematite --electrical 'rc R=10k C=100u'");
        let _ = writeln!(out, "    hematite --electrical 'rl R=100 L=10m'");
        let _ = writeln!(out, "    hematite --electrical 'lc L=10m C=100u'");
        let _ = writeln!(out, "    hematite --electrical 'db 0.5'");
        let _ = writeln!(out, "    hematite --electrical 'db2linear -20'");
        let _ = writeln!(out, "    hematite --electrical 'freq f=2.4G'");
        let _ = writeln!(
            out,
            "    hematite --electrical 'divider Vin=12 R1=10k R2=4.7k'"
        );
        let _ = writeln!(out, "    hematite --electrical 'energy C=100u V=5'");
        let _ = writeln!(out, "    hematite --electrical 'energy L=10m I=2'");
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // Parse key=value pairs — preserve case in original query for SI prefix detection
    let parse_kv = |key: &str| -> Option<f64> {
        let key_lower = key.to_lowercase();
        for w in query.trim().split_whitespace() {
            let wl = w.to_lowercase();
            if wl.starts_with(&format!("{}=", key_lower)) {
                return parse_si_value(&w[key.len() + 1..]);
            }
        }
        None
    };

    let keyword = words[0].as_ref();

    match keyword {
        "ohm" | "ohms" => {
            let v_opt = parse_kv("V").or_else(|| parse_kv("v"));
            let i_opt = parse_kv("I").or_else(|| parse_kv("i"));
            let r_opt = parse_kv("R").or_else(|| parse_kv("r"));

            let _ = writeln!(out, "  ─── Ohm's Law  (V = I × R, P = V × I) ───");
            match (v_opt, i_opt, r_opt) {
                (None, Some(i), Some(r)) => {
                    let v = i * r;
                    let p = v * i;
                    let _ = writeln!(out, "  I = {}  R = {}", fmt_si(i, "A"), fmt_si(r, "Ω"));
                    let _ = writeln!(out, "  V = I × R = {}", fmt_si(v, "V"));
                    let _ = writeln!(out, "  P = V × I = {}", fmt_si(p, "W"));
                }
                (Some(v), None, Some(r)) => {
                    let i = v / r;
                    let p = v * i;
                    let _ = writeln!(out, "  V = {}  R = {}", fmt_si(v, "V"), fmt_si(r, "Ω"));
                    let _ = writeln!(out, "  I = V / R = {}", fmt_si(i, "A"));
                    let _ = writeln!(out, "  P = V² / R = {}", fmt_si(p, "W"));
                }
                (Some(v), Some(i), None) => {
                    let r = v / i;
                    let p = v * i;
                    let _ = writeln!(out, "  V = {}  I = {}", fmt_si(v, "V"), fmt_si(i, "A"));
                    let _ = writeln!(out, "  R = V / I = {}", fmt_si(r, "Ω"));
                    let _ = writeln!(out, "  P = V × I = {}", fmt_si(p, "W"));
                }
                (Some(v), Some(i), Some(r)) => {
                    let v_calc = i * r;
                    let p = v * i;
                    let _ = writeln!(
                        out,
                        "  V = {}  I = {}  R = {}",
                        fmt_si(v, "V"),
                        fmt_si(i, "A"),
                        fmt_si(r, "Ω")
                    );
                    let _ = writeln!(
                        out,
                        "  Verify V = I×R = {}  (given {})",
                        fmt_si(v_calc, "V"),
                        fmt_si(v, "V")
                    );
                    let _ = writeln!(out, "  P = V × I = {}", fmt_si(p, "W"));
                    let diff = (v - v_calc).abs() / v.abs().max(1e-12) * 100.0;
                    if diff > 0.1 {
                        let _ = writeln!(out, "  Inconsistent: {:.2}% deviation", diff);
                    } else {
                        let _ = writeln!(out, "  Consistent");
                    }
                }
                _ => {
                    let _ = writeln!(out, "  Provide at least 2 of: V=, I=, R=");
                }
            }
        }
        "rc" => {
            let r_opt = parse_kv("R").or_else(|| parse_kv("r"));
            let c_opt = parse_kv("C").or_else(|| parse_kv("c"));
            if let (Some(r), Some(c)) = (r_opt, c_opt) {
                let tau = r * c;
                let f_cutoff = 1.0 / (2.0 * std::f64::consts::PI * r * c);
                let _ = writeln!(
                    out,
                    "  ─── RC Circuit  R={}, C={} ───",
                    fmt_si(r, "Ω"),
                    fmt_si(c, "F")
                );
                let _ = writeln!(out, "  Time constant τ = RC = {}", fmt_si(tau, "s"));
                let _ = writeln!(
                    out,
                    "  Cutoff freq fc = 1/(2πRC) = {}",
                    fmt_si(f_cutoff, "Hz")
                );
                let _ = writeln!(out, "  Charge curve  V(t) = V₀(1 - e^(-t/τ)):");
                for (label, t_frac) in &[("1τ", 1.0f64), ("2τ", 2.0), ("3τ", 3.0), ("5τ", 5.0)]
                {
                    let pct = (1.0 - (-t_frac).exp()) * 100.0;
                    let _ = writeln!(out, "    t={}: {:>6.2}% charged", label, pct);
                }
            } else {
                let _ = writeln!(
                    out,
                    "  Provide R=<resistance> C=<capacitance>  (e.g. R=10k C=100u)"
                );
            }
        }
        "rl" => {
            let r_opt = parse_kv("R").or_else(|| parse_kv("r"));
            let l_opt = parse_kv("L").or_else(|| parse_kv("l"));
            if let (Some(r), Some(l)) = (r_opt, l_opt) {
                let tau = l / r;
                let f_cutoff = r / (2.0 * std::f64::consts::PI * l);
                let _ = writeln!(
                    out,
                    "  ─── RL Circuit  R={}, L={} ───",
                    fmt_si(r, "Ω"),
                    fmt_si(l, "H")
                );
                let _ = writeln!(out, "  Time constant τ = L/R = {}", fmt_si(tau, "s"));
                let _ = writeln!(
                    out,
                    "  Cutoff freq fc = R/(2πL) = {}",
                    fmt_si(f_cutoff, "Hz")
                );
            } else {
                let _ = writeln!(
                    out,
                    "  Provide R=<resistance> L=<inductance>  (e.g. R=100 L=10m)"
                );
            }
        }
        "lc" => {
            let l_opt = parse_kv("L").or_else(|| parse_kv("l"));
            let c_opt = parse_kv("C").or_else(|| parse_kv("c"));
            if let (Some(l), Some(c)) = (l_opt, c_opt) {
                let f_res = 1.0 / (2.0 * std::f64::consts::PI * (l * c).sqrt());
                let omega = 2.0 * std::f64::consts::PI * f_res;
                let _ = writeln!(
                    out,
                    "  ─── LC Resonance  L={}, C={} ───",
                    fmt_si(l, "H"),
                    fmt_si(c, "F")
                );
                let _ = writeln!(
                    out,
                    "  Resonant freq f₀ = 1/(2π√LC) = {}",
                    fmt_si(f_res, "Hz")
                );
                let _ = writeln!(
                    out,
                    "  Angular freq  ω₀ = 1/√LC      = {} rad/s",
                    fmt_si(omega, "")
                );
            } else {
                let _ = writeln!(
                    out,
                    "  Provide L=<inductance> C=<capacitance>  (e.g. L=10m C=100u)"
                );
            }
        }
        "db" | "decibel" | "decibels" => {
            if let Some(ratio_str) = words.get(1) {
                if let Ok(ratio) = ratio_str.parse::<f64>() {
                    let db_voltage = 20.0 * ratio.abs().log10();
                    let db_power = 10.0 * ratio.abs().log10();
                    let _ = writeln!(out, "  dB Conversion  linear ratio = {}", ratio);
                    let _ = writeln!(
                        out,
                        "  Voltage/Current dB:  {:>10.4} dB  (20·log₁₀)",
                        db_voltage
                    );
                    let _ = writeln!(
                        out,
                        "  Power dB:            {:>10.4} dB  (10·log₁₀)",
                        db_power
                    );
                } else {
                    let _ = writeln!(out, "  Provide ratio: hematite --electrical 'db 0.5'");
                }
            } else {
                let _ = writeln!(out, "  Provide ratio: hematite --electrical 'db 0.5'");
            }
        }
        "db2linear" | "db-linear" => {
            if let Some(db_str) = words.get(1) {
                if let Ok(db) = db_str.parse::<f64>() {
                    let linear_v = 10.0_f64.powf(db / 20.0);
                    let linear_p = 10.0_f64.powf(db / 10.0);
                    let _ = writeln!(out, "  dB to Linear  {} dB", db);
                    let _ = writeln!(out, "  Voltage ratio: {:>12.6}  (10^(dB/20))", linear_v);
                    let _ = writeln!(out, "  Power ratio:   {:>12.6}  (10^(dB/10))", linear_p);
                } else {
                    let _ = writeln!(
                        out,
                        "  Provide dB value: hematite --electrical 'db2linear -20'"
                    );
                }
            } else {
                let _ = writeln!(
                    out,
                    "  Provide dB value: hematite --electrical 'db2linear -20'"
                );
            }
        }
        "freq" | "frequency" | "wavelength" => {
            let f_opt = parse_kv("f").or_else(|| parse_kv("F"));
            let wl_opt = parse_kv("wl").or_else(|| parse_kv("lambda"));
            let c_light = 299_792_458.0f64;
            if let Some(f) = f_opt {
                let wl = c_light / f;
                let _ = writeln!(out, "  Frequency ↔ Wavelength  (c = 299,792,458 m/s)");
                let _ = writeln!(out, "  f = {}  →  λ = {}", fmt_si(f, "Hz"), fmt_si(wl, "m"));
            } else if let Some(wl) = wl_opt {
                let f = c_light / wl;
                let _ = writeln!(out, "  Frequency ↔ Wavelength  (c = 299,792,458 m/s)");
                let _ = writeln!(out, "  λ = {}  →  f = {}", fmt_si(wl, "m"), fmt_si(f, "Hz"));
            } else {
                let _ = writeln!(
                    out,
                    "  Provide f=<frequency_Hz> or wl=<wavelength_m>  (SI prefixes OK)"
                );
            }
        }
        "divider" | "voltage-divider" => {
            let vin = parse_kv("Vin")
                .or_else(|| parse_kv("vin"))
                .or_else(|| parse_kv("V"))
                .or_else(|| parse_kv("v"));
            let r1 = parse_kv("R1").or_else(|| parse_kv("r1"));
            let r2 = parse_kv("R2").or_else(|| parse_kv("r2"));
            if let (Some(vin), Some(r1), Some(r2)) = (vin, r1, r2) {
                let vout = vin * r2 / (r1 + r2);
                let ratio = r2 / (r1 + r2);
                let current = vin / (r1 + r2);
                let _ = writeln!(
                    out,
                    "  ─── Voltage Divider  Vin={}, R1={}, R2={} ───",
                    fmt_si(vin, "V"),
                    fmt_si(r1, "Ω"),
                    fmt_si(r2, "Ω")
                );
                let _ = writeln!(out, "  Vout = Vin × R2/(R1+R2) = {}", fmt_si(vout, "V"));
                let _ = writeln!(out, "  Ratio: {:.4}  ({:.2}%)", ratio, ratio * 100.0);
                let _ = writeln!(out, "  Current: {}", fmt_si(current, "A"));
                let _ = writeln!(out, "  Power dissipated: {}", fmt_si(vin * current, "W"));
            } else {
                let _ = writeln!(
                    out,
                    "  Provide Vin=, R1=, R2=  (e.g. Vin=12 R1=10k R2=4.7k)"
                );
            }
        }
        "energy" => {
            let c_opt = parse_kv("C").or_else(|| parse_kv("c"));
            let l_opt = parse_kv("L").or_else(|| parse_kv("l"));
            let v_opt = parse_kv("V").or_else(|| parse_kv("v"));
            let i_opt = parse_kv("I").or_else(|| parse_kv("i"));
            if let (Some(c), Some(v)) = (c_opt, v_opt) {
                let e = 0.5 * c * v * v;
                let _ = writeln!(
                    out,
                    "  Capacitor Energy  C={}, V={}",
                    fmt_si(c, "F"),
                    fmt_si(v, "V")
                );
                let _ = writeln!(out, "  E = ½CV² = {}", fmt_si(e, "J"));
            } else if let (Some(l), Some(i)) = (l_opt, i_opt) {
                let e = 0.5 * l * i * i;
                let _ = writeln!(
                    out,
                    "  Inductor Energy  L={}, I={}",
                    fmt_si(l, "H"),
                    fmt_si(i, "A")
                );
                let _ = writeln!(out, "  E = ½LI² = {}", fmt_si(e, "J"));
            } else {
                let _ = writeln!(out, "  Provide C= V= (capacitor) or L= I= (inductor)");
            }
        }
        _ => {
            let _ = writeln!(out, "  Unknown command: '{}'", keyword);
            let _ = writeln!(
                out,
                "  Supported: ohm, rc, rl, lc, db, db2linear, freq, divider, energy"
            );
        }
    }

    let _ = writeln!(out, "{}", sep);
    out
}

// ── Physics ───────────────────────────────────────────────────────────────────

pub fn physics_calc(query: &str) -> String {
    let sep = "─".repeat(60);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n  ┌─ Physics Calculator ─────────────────────────────┐"
    );
    let _ = writeln!(out, "{}", sep);

    let q = query.trim().to_lowercase();
    let words: Vec<&str> = q.split_whitespace().collect();

    if words.is_empty() {
        let _ = writeln!(out, "  Usage examples:");
        let _ = writeln!(out, "    hematite --physics 'kinematic v0=0 a=9.8 t=3'");
        let _ = writeln!(out, "    hematite --physics 'projectile v0=20 angle=45'");
        let _ = writeln!(out, "    hematite --physics 'force m=5 a=3'");
        let _ = writeln!(out, "    hematite --physics 'energy m=10 v=5'");
        let _ = writeln!(out, "    hematite --physics 'momentum m=2 v=10'");
        let _ = writeln!(out, "    hematite --physics 'wave f=440'");
        let _ = writeln!(out, "    hematite --physics 'snell n1=1 n2=1.5 angle=30'");
        let _ = writeln!(out, "    hematite --physics 'lens f=0.2 do=0.5'");
        let _ = writeln!(out, "    hematite --physics 'gas P=101325 V=0.001 T=300'");
        let _ = writeln!(
            out,
            "    hematite --physics 'gravity m1=5.97e24 m2=1 r=6.37e6'"
        );
        let _ = writeln!(out, "    hematite --physics 'pendulum L=1'");
        let _ = writeln!(out, "    hematite --physics 'circular r=2 v=5'");
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    let parse_kv = |key: &str| -> Option<f64> {
        for w in query.trim().split_whitespace() {
            let wl = w.to_lowercase();
            if wl.starts_with(&format!("{}=", key.to_lowercase())) {
                return w[key.len() + 1..].parse().ok();
            }
        }
        None
    };

    let pi = std::f64::consts::PI;
    let g = 9.80665_f64; // standard gravity m/s²
    let keyword = words[0];

    match keyword {
        "kinematic" | "kinematics" | "1d" => {
            // SUVAT: s=v0*t + 0.5*a*t², v=v0+a*t, v²=v0²+2*a*s
            let v0 = parse_kv("v0").unwrap_or(0.0);
            let v = parse_kv("v");
            let a = parse_kv("a");
            let t = parse_kv("t");
            let s = parse_kv("s");
            let _ = writeln!(out, "  ─── 1D Kinematics (SUVAT) ───");
            let _ = writeln!(out, "  Given: v0={}", v0);
            if let Some(a) = a {
                if let Some(t) = t {
                    let vf = v0 + a * t;
                    let dist = v0 * t + 0.5 * a * t * t;
                    let _ = writeln!(out, "  a={} m/s², t={} s", a, t);
                    let _ = writeln!(out, "  v  = v0 + a·t      = {:.6} m/s", vf);
                    let _ = writeln!(out, "  s  = v0·t + ½a·t²  = {:.6} m", dist);
                    let _ = writeln!(out, "  v² = v0² + 2·a·s   = {:.6} m²/s²", vf * vf);
                } else if let Some(s) = s {
                    let vf_sq = v0 * v0 + 2.0 * a * s;
                    let _ = writeln!(out, "  a={} m/s², s={} m", a, s);
                    if vf_sq >= 0.0 {
                        let vf = vf_sq.sqrt();
                        let _ = writeln!(out, "  v  = √(v0²+2as)    = {:.6} m/s", vf);
                        let t_val = (vf - v0) / a;
                        let _ = writeln!(out, "  t  = (v-v0)/a      = {:.6} s", t_val);
                    } else {
                        let _ = writeln!(out, "  Object does not reach that displacement (v² < 0)");
                    }
                }
            } else if let Some(v) = v {
                if let Some(t) = t {
                    let a_calc = (v - v0) / t;
                    let s_calc = (v0 + v) / 2.0 * t;
                    let _ = writeln!(out, "  v={} m/s, t={} s", v, t);
                    let _ = writeln!(out, "  a  = (v-v0)/t      = {:.6} m/s²", a_calc);
                    let _ = writeln!(out, "  s  = (v0+v)/2 · t  = {:.6} m", s_calc);
                }
            } else {
                let _ = writeln!(out, "  Provide at least 2 of: v0=, v=, a=, t=, s=");
            }
        }
        "projectile" => {
            let v0 = parse_kv("v0").unwrap_or(10.0);
            let angle_deg = parse_kv("angle").unwrap_or(45.0);
            let h0 = parse_kv("h").unwrap_or(0.0);
            let angle_rad = angle_deg.to_radians();
            let vx = v0 * angle_rad.cos();
            let vy0 = v0 * angle_rad.sin();
            // Time of flight (quadratic: h0 + vy0*t - 0.5*g*t² = 0)
            let disc = vy0 * vy0 + 2.0 * g * h0;
            let _ = writeln!(out, "  ─── Projectile Motion ───");
            let _ = writeln!(out, "  v0={} m/s, angle={}°, h0={} m", v0, angle_deg, h0);
            let _ = writeln!(out, "  vx = v0·cos(θ) = {:.6} m/s", vx);
            let _ = writeln!(out, "  vy0= v0·sin(θ) = {:.6} m/s", vy0);
            if disc >= 0.0 {
                let t_flight = (vy0 + disc.sqrt()) / g;
                let range = vx * t_flight;
                let t_peak = vy0 / g;
                let h_max = h0 + vy0 * t_peak - 0.5 * g * t_peak * t_peak;
                let _ = writeln!(out, "  Time of flight = {:.6} s", t_flight);
                let _ = writeln!(out, "  Range          = {:.6} m", range);
                let _ = writeln!(
                    out,
                    "  Max height     = {:.6} m  (at t={:.4} s)",
                    h_max, t_peak
                );
                let vx_land = vx;
                let vy_land = vy0 - g * t_flight;
                let v_land = (vx_land * vx_land + vy_land * vy_land).sqrt();
                let _ = writeln!(out, "  Landing speed  = {:.6} m/s", v_land);
            }
        }
        "force" | "newton" => {
            let m = parse_kv("m");
            let a = parse_kv("a");
            let f = parse_kv("F").or_else(|| parse_kv("f"));
            let _ = writeln!(out, "  ─── Newton's Second Law  F = m·a ───");
            match (f, m, a) {
                (None, Some(m), Some(a)) => {
                    let _ = writeln!(out, "  m={} kg, a={} m/s²", m, a);
                    let _ = writeln!(out, "  F = m·a = {:.6} N", m * a);
                    let _ = writeln!(out, "  Weight on Earth = {:.6} N", m * g);
                }
                (Some(f), None, Some(a)) => {
                    let _ = writeln!(out, "  F={} N, a={} m/s²", f, a);
                    let _ = writeln!(out, "  m = F/a = {:.6} kg", f / a);
                }
                (Some(f), Some(m), None) => {
                    let _ = writeln!(out, "  F={} N, m={} kg", f, m);
                    let _ = writeln!(out, "  a = F/m = {:.6} m/s²", f / m);
                }
                _ => {
                    let _ = writeln!(out, "  Provide 2 of: F=, m=, a=");
                }
            }
        }
        "energy" | "ke" | "pe" => {
            let m = parse_kv("m");
            let v = parse_kv("v");
            let h = parse_kv("h");
            let _ = writeln!(out, "  ─── Mechanical Energy ───");
            if let Some(m) = m {
                if let Some(v) = v {
                    let ke = 0.5 * m * v * v;
                    let _ = writeln!(out, "  KE = ½mv²  m={} kg, v={} m/s  →  {:.6} J", m, v, ke);
                }
                if let Some(h) = h {
                    let pe = m * g * h;
                    let _ = writeln!(out, "  PE = mgh   m={} kg, h={} m   →  {:.6} J", m, h, pe);
                }
                if let (Some(v), Some(h)) = (v, h) {
                    let ke = 0.5 * m * v * v;
                    let pe = m * g * h;
                    let _ = writeln!(out, "  Total ME = KE + PE = {:.6} J", ke + pe);
                }
            } else {
                let _ = writeln!(out, "  Provide m=<mass> with v= (KE) and/or h= (PE)");
            }
        }
        "momentum" | "impulse" => {
            let m = parse_kv("m");
            let v = parse_kv("v");
            let f = parse_kv("F").or_else(|| parse_kv("f"));
            let t = parse_kv("t");
            let _ = writeln!(out, "  ─── Momentum & Impulse ───");
            if let (Some(m), Some(v)) = (m, v) {
                let p = m * v;
                let _ = writeln!(out, "  p = mv  m={} kg, v={} m/s  →  {:.6} kg·m/s", m, v, p);
            }
            if let (Some(f), Some(t)) = (f, t) {
                let j = f * t;
                let _ = writeln!(out, "  J = Ft  F={} N, t={} s    →  {:.6} N·s", f, t, j);
            }
            if m.is_none() && f.is_none() {
                let _ = writeln!(out, "  Provide m= v= for momentum, or F= t= for impulse");
            }
        }
        "work" | "power" => {
            let f = parse_kv("F").or_else(|| parse_kv("f"));
            let d = parse_kv("d");
            let t = parse_kv("t");
            let theta_deg = parse_kv("angle").unwrap_or(0.0);
            let _ = writeln!(out, "  ─── Work & Power ───");
            if let (Some(f), Some(d)) = (f, d) {
                let w = f * d * theta_deg.to_radians().cos();
                let _ = writeln!(
                    out,
                    "  W = F·d·cos(θ)  F={} N, d={} m, θ={}°  →  {:.6} J",
                    f, d, theta_deg, w
                );
                if let Some(t) = t {
                    let _ = writeln!(
                        out,
                        "  P = W/t                              →  {:.6} W",
                        w / t
                    );
                }
            } else {
                let _ = writeln!(out, "  Provide F= d= and optionally angle= t=");
            }
        }
        "wave" | "waves" => {
            let f = parse_kv("f").or_else(|| parse_kv("freq"));
            let lam = parse_kv("lambda")
                .or_else(|| parse_kv("wl"))
                .or_else(|| parse_kv("l"));
            let v_wave = parse_kv("v").unwrap_or(343.0); // default speed of sound in air
            let _ = writeln!(out, "  ─── Wave Properties ───");
            match (f, lam) {
                (Some(f), None) => {
                    let lambda = v_wave / f;
                    let period = 1.0 / f;
                    let omega = 2.0 * pi * f;
                    let _ = writeln!(out, "  f={} Hz, v={} m/s", f, v_wave);
                    let _ = writeln!(out, "  λ = v/f       = {:.6} m", lambda);
                    let _ = writeln!(out, "  T = 1/f       = {:.6} s", period);
                    let _ = writeln!(out, "  ω = 2πf       = {:.6} rad/s", omega);
                }
                (None, Some(lam)) => {
                    let freq = v_wave / lam;
                    let period = 1.0 / freq;
                    let _ = writeln!(out, "  λ={} m, v={} m/s", lam, v_wave);
                    let _ = writeln!(out, "  f = v/λ       = {:.6} Hz", freq);
                    let _ = writeln!(out, "  T = λ/v       = {:.6} s", period);
                }
                (Some(f), Some(lam)) => {
                    let v_calc = f * lam;
                    let _ = writeln!(out, "  f={} Hz, λ={} m", f, lam);
                    let _ = writeln!(out, "  v = f·λ       = {:.6} m/s", v_calc);
                }
                _ => {
                    let _ = writeln!(
                        out,
                        "  Provide f= (frequency Hz) and/or lambda= (wavelength m)"
                    );
                }
            }
        }
        "snell" | "refraction" => {
            let n1 = parse_kv("n1").unwrap_or(1.0);
            let n2 = parse_kv("n2").unwrap_or(1.5);
            let angle1_deg = parse_kv("angle").or_else(|| parse_kv("a1")).unwrap_or(30.0);
            let angle1_rad = angle1_deg.to_radians();
            let sin2 = n1 * angle1_rad.sin() / n2;
            let _ = writeln!(out, "  ─── Snell's Law  n₁·sin(θ₁) = n₂·sin(θ₂) ───");
            let _ = writeln!(out, "  n1={}, n2={}, θ1={}°", n1, n2, angle1_deg);
            if sin2.abs() <= 1.0 {
                let angle2_deg = sin2.asin().to_degrees();
                let _ = writeln!(out, "  θ2 = arcsin(n1/n2 · sin(θ1)) = {:.6}°", angle2_deg);
            } else {
                let _ = writeln!(out, "  Total internal reflection (no refracted ray)");
            }
            // Critical angle
            if n1 > n2 {
                let _ = writeln!(out, "  Note: n1>n2, going from denser to less dense");
            } else if n2 > 0.0 {
                let crit = (n1 / n2).asin().to_degrees();
                if n1 < n2 {
                    let _ = writeln!(
                        out,
                        "  Critical angle (n1→n2) = {:.4}°  (TIR from other side)",
                        crit
                    );
                }
            }
        }
        "lens" | "mirror" => {
            let f_len = parse_kv("f");
            let d_o = parse_kv("do").or_else(|| parse_kv("u"));
            let d_i = parse_kv("di").or_else(|| parse_kv("v"));
            let _ = writeln!(
                out,
                "  ─── Thin Lens / Mirror Equation  1/f = 1/do + 1/di ───"
            );
            match (f_len, d_o, d_i) {
                (Some(f), Some(do_), None) => {
                    let di = 1.0 / (1.0 / f - 1.0 / do_);
                    let m = -di / do_;
                    let _ = writeln!(out, "  f={} m, do={} m", f, do_);
                    let _ = writeln!(
                        out,
                        "  di = 1/(1/f - 1/do) = {:.6} m  ({})",
                        di,
                        if di > 0.0 {
                            "real image"
                        } else {
                            "virtual image"
                        }
                    );
                    let _ = writeln!(
                        out,
                        "  m  = -di/do         = {:.6}  ({})",
                        m,
                        if m.abs() > 1.0 {
                            "magnified"
                        } else {
                            "diminished"
                        }
                    );
                }
                (None, Some(do_), Some(di)) => {
                    let f = 1.0 / (1.0 / do_ + 1.0 / di);
                    let _ = writeln!(out, "  do={} m, di={} m", do_, di);
                    let _ = writeln!(out, "  f  = 1/(1/do + 1/di) = {:.6} m", f);
                    let _ = writeln!(out, "  P  = 1/f             = {:.6} diopters", 1.0 / f);
                }
                _ => {
                    let _ = writeln!(out, "  Provide 2 of: f=, do=, di=  (all in meters)");
                }
            }
        }
        "gas" | "ideal-gas" | "idealgas" => {
            // PV = nRT, R = 8.314 J/(mol·K)
            let r_gas = 8.314_f64;
            let p = parse_kv("P");
            let v_vol = parse_kv("V");
            let n = parse_kv("n");
            let temp = parse_kv("T");
            let _ = writeln!(
                out,
                "  ─── Ideal Gas Law  PV = nRT  (R = 8.314 J/mol·K) ───"
            );
            match (p, v_vol, n, temp) {
                (None, Some(v), Some(n), Some(t)) => {
                    let p_calc = n * r_gas * t / v;
                    let _ = writeln!(out, "  V={} m³, n={} mol, T={} K", v, n, t);
                    let _ = writeln!(
                        out,
                        "  P = nRT/V = {:.4} Pa  ({:.4} atm)",
                        p_calc,
                        p_calc / 101325.0
                    );
                }
                (Some(p), None, Some(n), Some(t)) => {
                    let v_calc = n * r_gas * t / p;
                    let _ = writeln!(out, "  P={} Pa, n={} mol, T={} K", p, n, t);
                    let _ = writeln!(
                        out,
                        "  V = nRT/P = {:.6} m³  ({:.4} L)",
                        v_calc,
                        v_calc * 1000.0
                    );
                }
                (Some(p), Some(v), None, Some(t)) => {
                    let n_calc = p * v / (r_gas * t);
                    let _ = writeln!(out, "  P={} Pa, V={} m³, T={} K", p, v, t);
                    let _ = writeln!(out, "  n = PV/RT = {:.6} mol", n_calc);
                }
                (Some(p), Some(v), Some(n), None) => {
                    let t_calc = p * v / (n * r_gas);
                    let _ = writeln!(out, "  P={} Pa, V={} m³, n={} mol", p, v, n);
                    let _ = writeln!(
                        out,
                        "  T = PV/nR = {:.4} K  ({:.2} °C)",
                        t_calc,
                        t_calc - 273.15
                    );
                }
                (Some(p), Some(v), Some(n), Some(t)) => {
                    let pv = p * v;
                    let nrt = n * r_gas * t;
                    let _ = writeln!(out, "  P={} Pa, V={} m³, n={} mol, T={} K", p, v, n, t);
                    let _ = writeln!(out, "  PV   = {:.4} J", pv);
                    let _ = writeln!(out, "  nRT  = {:.4} J", nrt);
                    let _ = writeln!(out, "  Z    = PV/nRT = {:.6}  (1.0 = ideal)", pv / nrt);
                }
                _ => {
                    let _ = writeln!(
                        out,
                        "  Provide 3 of: P=, V=, n=, T=  (SI units: Pa, m³, mol, K)"
                    );
                }
            }
        }
        "gravity" | "gravitation" => {
            let big_g = 6.674e-11_f64;
            let m1 = parse_kv("m1").unwrap_or(5.972e24);
            let m2 = parse_kv("m2").unwrap_or(1.0);
            let r = parse_kv("r").unwrap_or(6.371e6);
            let f_grav = big_g * m1 * m2 / (r * r);
            let _ = writeln!(out, "  ─── Universal Gravitation  F = G·m1·m2/r² ───");
            let _ = writeln!(out, "  G  = 6.674×10⁻¹¹ N·m²/kg²");
            let _ = writeln!(out, "  m1 = {:.4e} kg", m1);
            let _ = writeln!(out, "  m2 = {:.4e} kg", m2);
            let _ = writeln!(out, "  r  = {:.4e} m", r);
            let _ = writeln!(out, "  F  = {:.6e} N", f_grav);
            let g_surface = big_g * m1 / (r * r);
            let _ = writeln!(
                out,
                "  g  = G·m1/r² = {:.6} m/s²  (surface gravity)",
                g_surface
            );
        }
        "pendulum" => {
            let l = parse_kv("L").or_else(|| parse_kv("l")).unwrap_or(1.0);
            let t_period = 2.0 * pi * (l / g).sqrt();
            let freq = 1.0 / t_period;
            let _ = writeln!(out, "  ─── Simple Pendulum ───");
            let _ = writeln!(out, "  L = {} m  (g = {:.5} m/s²)", l, g);
            let _ = writeln!(out, "  T = 2π√(L/g) = {:.6} s", t_period);
            let _ = writeln!(out, "  f = 1/T      = {:.6} Hz", freq);
        }
        "circular" | "centripetal" => {
            let r = parse_kv("r");
            let v = parse_kv("v");
            let m = parse_kv("m");
            if let (Some(r), Some(v)) = (r, v) {
                let a_c = v * v / r;
                let omega = v / r;
                let period = 2.0 * pi * r / v;
                let _ = writeln!(out, "  ─── Circular Motion ───");
                let _ = writeln!(out, "  r={} m, v={} m/s", r, v);
                let _ = writeln!(out, "  a_c = v²/r = {:.6} m/s²  (centripetal)", a_c);
                let _ = writeln!(out, "  ω   = v/r  = {:.6} rad/s", omega);
                let _ = writeln!(out, "  T   = 2πr/v= {:.6} s", period);
                if let Some(m) = m {
                    let _ = writeln!(out, "  F_c = ma_c = {:.6} N  (centripetal force)", m * a_c);
                }
            } else {
                let _ = writeln!(
                    out,
                    "  Provide r=<radius_m> v=<speed_m/s> and optionally m=<mass_kg>"
                );
            }
        }
        _ => {
            let _ = writeln!(out, "  Unknown command: '{}'", keyword);
            let _ = writeln!(
                out,
                "  Supported: kinematic, projectile, force, energy, momentum,"
            );
            let _ = writeln!(
                out,
                "             work, wave, snell, lens, gas, gravity, pendulum, circular"
            );
        }
    }

    let _ = writeln!(out, "{}", sep);
    out
}

// ── Chemistry ─────────────────────────────────────────────────────────────────

// Atomic masses in g/mol for the 118 elements (IUPAC 2021)
fn atomic_mass(symbol: &str) -> Option<f64> {
    // Lowercase symbol for matching
    let s = symbol;
    Some(match s {
        "H" => 1.008,
        "He" => 4.0026,
        "Li" => 6.941,
        "Be" => 9.0122,
        "B" => 10.811,
        "C" => 12.011,
        "N" => 14.007,
        "O" => 15.999,
        "F" => 18.998,
        "Ne" => 20.180,
        "Na" => 22.990,
        "Mg" => 24.305,
        "Al" => 26.982,
        "Si" => 28.086,
        "P" => 30.974,
        "S" => 32.065,
        "Cl" => 35.453,
        "Ar" => 39.948,
        "K" => 39.098,
        "Ca" => 40.078,
        "Sc" => 44.956,
        "Ti" => 47.867,
        "V" => 50.942,
        "Cr" => 51.996,
        "Mn" => 54.938,
        "Fe" => 55.845,
        "Co" => 58.933,
        "Ni" => 58.693,
        "Cu" => 63.546,
        "Zn" => 65.38,
        "Ga" => 69.723,
        "Ge" => 72.630,
        "As" => 74.922,
        "Se" => 78.971,
        "Br" => 79.904,
        "Kr" => 83.798,
        "Rb" => 85.468,
        "Sr" => 87.62,
        "Y" => 88.906,
        "Zr" => 91.224,
        "Nb" => 92.906,
        "Mo" => 95.96,
        "Tc" => 98.0,
        "Ru" => 101.07,
        "Rh" => 102.91,
        "Pd" => 106.42,
        "Ag" => 107.87,
        "Cd" => 112.41,
        "In" => 114.82,
        "Sn" => 118.71,
        "Sb" => 121.76,
        "Te" => 127.60,
        "I" => 126.90,
        "Xe" => 131.29,
        "Cs" => 132.91,
        "Ba" => 137.33,
        "La" => 138.91,
        "Ce" => 140.12,
        "Pr" => 140.91,
        "Nd" => 144.24,
        "Pm" => 145.0,
        "Sm" => 150.36,
        "Eu" => 151.96,
        "Gd" => 157.25,
        "Tb" => 158.93,
        "Dy" => 162.50,
        "Ho" => 164.93,
        "Er" => 167.26,
        "Tm" => 168.93,
        "Yb" => 173.05,
        "Lu" => 174.97,
        "Hf" => 178.49,
        "Ta" => 180.95,
        "W" => 183.84,
        "Re" => 186.21,
        "Os" => 190.23,
        "Ir" => 192.22,
        "Pt" => 195.08,
        "Au" => 196.97,
        "Hg" => 200.59,
        "Tl" => 204.38,
        "Pb" => 207.2,
        "Bi" => 208.98,
        "Po" => 209.0,
        "At" => 210.0,
        "Rn" => 222.0,
        "Fr" => 223.0,
        "Ra" => 226.0,
        "Ac" => 227.0,
        "Th" => 232.04,
        "Pa" => 231.04,
        "U" => 238.03,
        "Np" => 237.0,
        "Pu" => 244.0,
        "Am" => 243.0,
        "Cm" => 247.0,
        "Bk" => 247.0,
        "Cf" => 251.0,
        "Es" => 252.0,
        "Fm" => 257.0,
        "Md" => 258.0,
        "No" => 259.0,
        "Lr" => 262.0,
        "Rf" => 267.0,
        "Db" => 268.0,
        "Sg" => 271.0,
        "Bh" => 272.0,
        "Hs" => 270.0,
        "Mt" => 278.0,
        "Ds" => 281.0,
        "Rg" => 282.0,
        "Cn" => 285.0,
        "Nh" => 286.0,
        "Fl" => 289.0,
        "Mc" => 290.0,
        "Lv" => 293.0,
        "Ts" => 294.0,
        "Og" => 294.0,
        _ => return None,
    })
}

/// Parse a molecular formula like "H2O", "C6H12O6", "NaCl", "Ca(OH)2"
/// Returns a map of element symbol → count, or an error string.
fn parse_chem_formula(formula: &str) -> Result<std::collections::HashMap<String, u32>, String> {
    let mut stack: Vec<std::collections::HashMap<String, u32>> =
        vec![std::collections::HashMap::new()];
    let chars: Vec<char> = formula.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '(' {
            stack.push(std::collections::HashMap::new());
            i += 1;
        } else if chars[i] == ')' {
            i += 1;
            let mut num_str = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                num_str.push(chars[i]);
                i += 1;
            }
            let multiplier: u32 = if num_str.is_empty() {
                1
            } else {
                num_str.parse().unwrap_or(1)
            };
            let top = stack.pop().ok_or("Unmatched ')'")?;
            let parent = stack.last_mut().ok_or("Empty stack")?;
            for (el, count) in top {
                *parent.entry(el).or_insert(0) += count * multiplier;
            }
        } else if chars[i].is_uppercase() {
            let mut sym = String::new();
            sym.push(chars[i]);
            i += 1;
            while i < chars.len() && chars[i].is_lowercase() {
                sym.push(chars[i]);
                i += 1;
            }
            let mut num_str = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                num_str.push(chars[i]);
                i += 1;
            }
            let count: u32 = if num_str.is_empty() {
                1
            } else {
                num_str.parse().unwrap_or(1)
            };
            let current = stack.last_mut().ok_or("Empty stack")?;
            *current.entry(sym).or_insert(0) += count;
        } else if chars[i] == '·' || chars[i] == '.' {
            i += 1;
        } else {
            return Err(format!("Unexpected character '{}'", chars[i]));
        }
    }
    if stack.len() != 1 {
        return Err("Unmatched '('".to_string());
    }
    Ok(stack.pop().unwrap())
}

pub fn chemistry_calc(query: &str) -> String {
    let sep = "─".repeat(60);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n  ┌─ Chemistry Calculator ───────────────────────────┐"
    );
    let _ = writeln!(out, "{}", sep);

    let q = query.trim();
    let lower = q.to_lowercase();
    let words: Vec<&str> = q.split_whitespace().collect();

    if q.is_empty() {
        let _ = writeln!(out, "  Usage examples:");
        let _ = writeln!(
            out,
            "    hematite --chemistry 'H2O'               — molar mass"
        );
        let _ = writeln!(
            out,
            "    hematite --chemistry 'C6H12O6'           — molar mass"
        );
        let _ = writeln!(
            out,
            "    hematite --chemistry 'Ca(OH)2'           — molar mass"
        );
        let _ = writeln!(
            out,
            "    hematite --chemistry 'molarity m=5 V=2'  — molarity (mol/L)"
        );
        let _ = writeln!(out, "    hematite --chemistry 'dilution C1=1 V1=0.5 V2=2'");
        let _ = writeln!(
            out,
            "    hematite --chemistry 'gas P=101325 V=0.001 T=300 n=?'"
        );
        let _ = writeln!(
            out,
            "    hematite --chemistry 'ph 0.001'          — H⁺ conc → pH"
        );
        let _ = writeln!(
            out,
            "    hematite --chemistry 'ph pH=3.5'         — pH → [H⁺]"
        );
        let _ = writeln!(
            out,
            "    hematite --chemistry 'buffer pKa=4.75 A=0.1 HA=0.1'"
        );
        let _ = writeln!(
            out,
            "    hematite --chemistry 'percent C6H12O6 C' — mass percent"
        );
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    // Helper to parse key=value from the original query
    let parse_kv = |key: &str| -> Option<f64> {
        for w in words.iter() {
            let wl = w.to_lowercase();
            if wl.starts_with(&format!("{}=", key.to_lowercase())) {
                return w[key.len() + 1..].parse().ok();
            }
        }
        None
    };

    let keyword = lower.split_whitespace().next().unwrap_or("");

    // Detect if first token is a molecular formula (starts with uppercase letter)
    let looks_like_formula = q.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        && !matches!(
            keyword,
            "molarity" | "dilution" | "gas" | "ph" | "buffer" | "percent" | "convert"
        );

    if looks_like_formula {
        // Molar mass mode — the whole input is treated as formula
        let formula = words[0]; // first whitespace-separated token
        let _ = writeln!(out, "  ─── Molar Mass: {} ───", formula);
        match parse_chem_formula(formula) {
            Ok(elements) => {
                let mut total_mass = 0.0;
                let mut entries: Vec<(String, u32, f64)> = Vec::new();
                let mut unknown = Vec::new();
                for (el, count) in &elements {
                    match atomic_mass(el.as_str()) {
                        Some(mass) => {
                            let contrib = mass * *count as f64;
                            total_mass += contrib;
                            entries.push((el.clone(), *count, contrib));
                        }
                        None => unknown.push(el.clone()),
                    }
                }
                if !unknown.is_empty() {
                    let _ = writeln!(out, "  Unknown elements: {}", unknown.join(", "));
                } else {
                    entries.sort_by(|a, b| a.0.cmp(&b.0));
                    let _ = writeln!(
                        out,
                        "  {:>4}  {:>6}  {:>10}  {:>8}",
                        "El", "Count", "Mass (g/mol)", "Percent"
                    );
                    for (el, count, contrib) in &entries {
                        let pct = contrib / total_mass * 100.0;
                        let _ = writeln!(
                            out,
                            "  {:>4}  {:>6}  {:>10.4}  {:>7.2}%",
                            el, count, contrib, pct
                        );
                    }
                    let _ = writeln!(out, "  {}", "─".repeat(38));
                    let total_atoms: u32 = elements.values().sum();
                    let _ = writeln!(out, "  Molar mass:   {:.4} g/mol", total_mass);
                    let _ = writeln!(out, "  Total atoms:  {}", total_atoms);
                    let _ = writeln!(
                        out,
                        "  1 mole =      {:.4e} molecules (Avogadro)",
                        6.02214076e23
                    );
                    let _ = writeln!(
                        out,
                        "  Molecule mass:{:.4e} g / molecule",
                        total_mass / 6.02214076e23
                    );
                }
            }
            Err(e) => {
                let _ = writeln!(out, "  Error parsing formula: {}", e);
            }
        }
    } else {
        match keyword {
            "molarity" => {
                // C = n/V  where V is in litres
                let n = parse_kv("n").or_else(|| parse_kv("m"));
                let v = parse_kv("V").or_else(|| parse_kv("v"));
                let c = parse_kv("C").or_else(|| parse_kv("c"));
                let _ = writeln!(out, "  ─── Molarity  C = n/V ───");
                match (c, n, v) {
                    (None, Some(n), Some(v)) => {
                        let _ = writeln!(out, "  n={} mol, V={} L", n, v);
                        let _ = writeln!(out, "  C = n/V = {:.6} mol/L (M)", n / v);
                    }
                    (Some(c), None, Some(v)) => {
                        let _ = writeln!(out, "  C={} M, V={} L", c, v);
                        let _ = writeln!(out, "  n = C·V = {:.6} mol", c * v);
                    }
                    (Some(c), Some(n), None) => {
                        let _ = writeln!(out, "  C={} M, n={} mol", c, n);
                        let _ = writeln!(out, "  V = n/C = {:.6} L", n / c);
                    }
                    _ => {
                        let _ = writeln!(out, "  Provide 2 of: C= (mol/L), n= (mol), V= (L)");
                    }
                }
            }
            "dilution" => {
                // C1V1 = C2V2
                let c1 = parse_kv("C1").or_else(|| parse_kv("c1"));
                let v1 = parse_kv("V1").or_else(|| parse_kv("v1"));
                let c2 = parse_kv("C2").or_else(|| parse_kv("c2"));
                let v2 = parse_kv("V2").or_else(|| parse_kv("v2"));
                let _ = writeln!(out, "  ─── Dilution  C₁V₁ = C₂V₂ ───");
                match (c1, v1, c2, v2) {
                    (Some(c1), Some(v1), None, Some(v2)) => {
                        let c2 = c1 * v1 / v2;
                        let _ = writeln!(out, "  C1={} M, V1={} L, V2={} L", c1, v1, v2);
                        let _ = writeln!(out, "  C2 = C1·V1/V2 = {:.6} M", c2);
                        let _ = writeln!(out, "  Dilution factor = {:.2}×", v2 / v1);
                    }
                    (Some(c1), Some(v1), Some(c2), None) => {
                        let v2 = c1 * v1 / c2;
                        let _ = writeln!(out, "  C1={} M, V1={} L, C2={} M", c1, v1, c2);
                        let _ = writeln!(out, "  V2 = C1·V1/C2 = {:.6} L", v2);
                    }
                    _ => {
                        let _ = writeln!(out, "  Provide C1= V1= and one of C2= or V2=");
                    }
                }
            }
            "ph" => {
                let conc = parse_kv("H").or_else(|| parse_kv("h"));
                let ph_given = parse_kv("pH").or_else(|| parse_kv("ph"));
                let _ = writeln!(out, "  ─── pH / [H⁺] Interconversion ───");
                if let Some(ph) = ph_given {
                    let h_conc = 10.0_f64.powf(-ph);
                    let oh_conc = 10.0_f64.powf(-(14.0 - ph));
                    let _ = writeln!(out, "  pH = {}", ph);
                    let _ = writeln!(out, "  [H⁺]  = 10^(-pH)     = {:.4e} mol/L", h_conc);
                    let _ = writeln!(out, "  [OH⁻] = 10^(pH-14)   = {:.4e} mol/L", oh_conc);
                    let _ = writeln!(out, "  pOH   = 14 - pH       = {:.4}", 14.0 - ph);
                    let nature = if (ph - 7.0).abs() < 0.01 {
                        "neutral"
                    } else if ph < 7.0 {
                        "acidic"
                    } else {
                        "basic"
                    };
                    let _ = writeln!(out, "  Nature: {}", nature);
                } else if let Some(h) = conc {
                    let ph = -h.log10();
                    let _ = writeln!(out, "  [H⁺] = {:.4e} mol/L", h);
                    let _ = writeln!(out, "  pH    = -log[H⁺] = {:.4}", ph);
                    let _ = writeln!(out, "  pOH   = 14 - pH   = {:.4}", 14.0 - ph);
                } else {
                    // Try bare float as [H+]
                    if let Ok(h) = q.split_whitespace().nth(1).unwrap_or("").parse::<f64>() {
                        let ph = -h.log10();
                        let _ = writeln!(out, "  [H⁺] = {:.4e} mol/L", h);
                        let _ = writeln!(out, "  pH    = -log[H⁺] = {:.4}", ph);
                        let _ = writeln!(out, "  pOH   = 14 - pH   = {:.4}", 14.0 - ph);
                    } else {
                        let _ = writeln!(out, "  Provide [H⁺] as 'ph 0.001' or pH as 'ph pH=3.5'");
                    }
                }
            }
            "buffer" | "henderson" => {
                // Henderson-Hasselbalch: pH = pKa + log([A-]/[HA])
                let pka = parse_kv("pKa").or_else(|| parse_kv("pka"));
                let a = parse_kv("A").or_else(|| parse_kv("a"));
                let ha = parse_kv("HA").or_else(|| parse_kv("ha"));
                let _ = writeln!(
                    out,
                    "  ─── Henderson-Hasselbalch  pH = pKa + log([A⁻]/[HA]) ───"
                );
                if let (Some(pka), Some(a), Some(ha)) = (pka, a, ha) {
                    let ph = pka + (a / ha).log10();
                    let _ = writeln!(out, "  pKa={}, [A⁻]={} M, [HA]={} M", pka, a, ha);
                    let _ = writeln!(out, "  pH = pKa + log([A⁻]/[HA]) = {:.4}", ph);
                    let _ = writeln!(out, "  Buffer ratio [A⁻]/[HA] = {:.4}", a / ha);
                    let max_buffer_ph = pka;
                    let _ = writeln!(
                        out,
                        "  Max buffer capacity at pH ≈ pKa = {:.2}",
                        max_buffer_ph
                    );
                } else {
                    let _ = writeln!(out, "  Provide pKa=, A=<[conjugate base]>, HA=<[acid]>");
                }
            }
            "percent" | "mass-percent" => {
                // Mass percent of an element in a compound
                if words.len() >= 2 {
                    let formula = words[1];
                    let target_el = words.get(2).copied().unwrap_or("");
                    match parse_chem_formula(formula) {
                        Ok(elements) => {
                            let mut total_mass = 0.0;
                            for (el, count) in &elements {
                                if let Some(mass) = atomic_mass(el.as_str()) {
                                    total_mass += mass * *count as f64;
                                }
                            }
                            let _ = writeln!(out, "  ─── Mass Percent in {} ───", formula);
                            if target_el.is_empty() {
                                let mut entries: Vec<(String, f64)> = elements
                                    .iter()
                                    .filter_map(|(el, count)| {
                                        atomic_mass(el.as_str()).map(|m| {
                                            (el.clone(), m * *count as f64 / total_mass * 100.0)
                                        })
                                    })
                                    .collect();
                                entries.sort_by(|a, b| {
                                    b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                                });
                                for (el, pct) in entries {
                                    let _ = writeln!(out, "  {:>4}: {:>7.3}%", el, pct);
                                }
                            } else if let Some(count) = elements.get(target_el) {
                                if let Some(mass) = atomic_mass(target_el) {
                                    let pct = mass * *count as f64 / total_mass * 100.0;
                                    let _ = writeln!(
                                        out,
                                        "  {} in {}: {:.4}%",
                                        target_el, formula, pct
                                    );
                                }
                            } else {
                                let _ =
                                    writeln!(out, "  Element '{}' not found in formula", target_el);
                            }
                            let _ = writeln!(out, "  Molar mass: {:.4} g/mol", total_mass);
                        }
                        Err(e) => {
                            let _ = writeln!(out, "  Error: {}", e);
                        }
                    }
                } else {
                    let _ = writeln!(out, "  Usage: hematite --chemistry 'percent H2O H'");
                }
            }
            _ => {
                let _ = writeln!(out, "  Unknown command: '{}'", keyword);
                let _ = writeln!(out, "  Try a molecular formula (H2O, C6H12O6, Ca(OH)2) or:");
                let _ = writeln!(out, "  molarity, dilution, ph, buffer, percent");
            }
        }
    }

    let _ = writeln!(out, "{}", sep);
    out
}

// ── Combinatorics ─────────────────────────────────────────────────────────────

fn factorial_big(n: u64) -> u128 {
    (1..=n).map(|x| x as u128).product()
}

fn comb(n: u64, k: u64) -> u128 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result: u128 = 1;
    for i in 0..k {
        result = result * (n - i) as u128 / (i + 1) as u128;
    }
    result
}

fn perm(n: u64, k: u64) -> u128 {
    if k > n {
        return 0;
    }
    (n - k + 1..=n).map(|x| x as u128).product()
}

pub fn combinatorics_calc(query: &str) -> String {
    let sep = "─".repeat(60);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n  ┌─ Combinatorics Calculator ───────────────────────┐"
    );
    let _ = writeln!(out, "{}", sep);

    let q = query.trim().to_lowercase();
    let words: Vec<&str> = q.split_whitespace().collect();

    if words.is_empty() {
        let _ = writeln!(out, "  Usage examples:");
        let _ = writeln!(
            out,
            "    hematite --combinatorics 'C 10 3'        — combinations C(10,3)"
        );
        let _ = writeln!(
            out,
            "    hematite --combinatorics 'P 10 3'        — permutations P(10,3)"
        );
        let _ = writeln!(out, "    hematite --combinatorics 'factorial 12'  — 12!");
        let _ = writeln!(out, "    hematite --combinatorics 'derangement 5' — D(5)");
        let _ = writeln!(out, "    hematite --combinatorics 'catalan 7'     — C_7");
        let _ = writeln!(
            out,
            "    hematite --combinatorics 'pascal 8'      — row 8 of Pascal's triangle"
        );
        let _ = writeln!(out, "    hematite --combinatorics 'bell 6'        — B_6");
        let _ = writeln!(
            out,
            "    hematite --combinatorics 'stirling 5 2'  — Stirling S(5,2)"
        );
        let _ = writeln!(
            out,
            "    hematite --combinatorics 'multinomial 12 3,4,5' — 12!/(3!4!5!)"
        );
        let _ = writeln!(
            out,
            "    hematite --combinatorics 'partition 6'   — number of integer partitions"
        );
        let _ = writeln!(out, "{}", sep);
        return out;
    }

    let parse_n = |idx: usize| -> Option<u64> { words.get(idx)?.parse().ok() };

    let keyword = words[0];

    match keyword {
        "c" | "choose" | "combination" | "combinations" => {
            if let (Some(n), Some(k)) = (parse_n(1), parse_n(2)) {
                let c = comb(n, k);
                let _ = writeln!(out, "  C({}, {}) = n! / (k!(n-k)!)", n, k);
                let _ = writeln!(out, "  = {}", c);
                let _ = writeln!(out, "  (choosing {} items from {} without order)", k, n);
            } else {
                let _ = writeln!(out, "  Usage: hematite --combinatorics 'C n k'");
            }
        }
        "p" | "permutation" | "permutations" => {
            if let (Some(n), Some(k)) = (parse_n(1), parse_n(2)) {
                let p = perm(n, k);
                let _ = writeln!(out, "  P({}, {}) = n! / (n-k)!", n, k);
                let _ = writeln!(out, "  = {}", p);
                let _ = writeln!(out, "  (arranging {} items from {} with order)", k, n);
            } else {
                let _ = writeln!(out, "  Usage: hematite --combinatorics 'P n k'");
            }
        }
        "factorial" | "fact" => {
            if let Some(n) = parse_n(1) {
                if n > 34 {
                    let _ = writeln!(out, "  {}! = {}", n, factorial_big(n));
                    let _ = writeln!(
                        out,
                        "  ≈ {:.6e}",
                        (0..=n).map(|x| (x as f64).max(1.0).ln()).sum::<f64>().exp()
                    );
                } else {
                    let _ = writeln!(out, "  {}! = {}", n, factorial_big(n));
                }
                // Stirling's approximation for n >= 10
                if n >= 10 {
                    let n_f = n as f64;
                    let stirling = (2.0 * std::f64::consts::PI * n_f).sqrt()
                        * (n_f / std::f64::consts::E).powf(n_f);
                    let _ = writeln!(
                        out,
                        "  Stirling approx: {:.6e}  (error < 1/(12n))",
                        stirling
                    );
                }
            } else {
                let _ = writeln!(out, "  Usage: hematite --combinatorics 'factorial n'");
            }
        }
        "derangement" | "derangements" | "d" => {
            if let Some(n) = parse_n(1) {
                // D(n) = n! * sum_{k=0}^{n} (-1)^k / k!
                let mut d: i128 = 0;
                let n_fact = factorial_big(n) as i128;
                let mut kfact: i128 = 1;
                for k in 0..=n {
                    if k > 0 {
                        kfact *= k as i128;
                    }
                    if k % 2 == 0 {
                        d += n_fact / kfact;
                    } else {
                        d -= n_fact / kfact;
                    }
                }
                let prob = d as f64 / factorial_big(n) as f64;
                let _ = writeln!(out, "  D({}) = {}", n, d);
                let _ = writeln!(
                    out,
                    "  Probability of a random derangement ≈ {:.6}  (→ 1/e)",
                    prob
                );
                let _ = writeln!(out, "  1/e ≈ {:.6}", 1.0 / std::f64::consts::E);
            } else {
                let _ = writeln!(out, "  Usage: hematite --combinatorics 'derangement n'");
            }
        }
        "catalan" => {
            if let Some(n) = parse_n(1) {
                let c = comb(2 * n, n) / (n as u128 + 1);
                let _ = writeln!(out, "  Catalan number C({}) = C(2n,n)/(n+1)", n);
                let _ = writeln!(out, "  = {}", c);
                let _ = writeln!(
                    out,
                    "  (balanced parentheses, full binary trees, polygon triangulations)"
                );
            } else {
                let _ = writeln!(out, "  Usage: hematite --combinatorics 'catalan n'");
            }
        }
        "pascal" => {
            if let Some(n) = parse_n(1) {
                let n = n.min(20); // cap for display
                let _ = writeln!(out, "  Pascal's Triangle — row {} (0-indexed):", n);
                let row: Vec<u128> = (0..=n).map(|k| comb(n, k)).collect();
                let row_str: Vec<String> = row.iter().map(|x| x.to_string()).collect();
                let _ = writeln!(out, "  {}", row_str.join("  "));
                let _ = writeln!(out, "  Sum of row {} = 2^{} = {}", n, n, 1u128 << n);
            } else {
                let _ = writeln!(
                    out,
                    "  Usage: hematite --combinatorics 'pascal n'  (shows row n)"
                );
            }
        }
        "bell" => {
            if let Some(n) = parse_n(1) {
                // Bell triangle
                let n = n.min(15) as usize;
                let mut prev_row = vec![1u128];
                let _ = writeln!(out, "  Bell numbers B(0)..B({}):", n);
                let mut bells = vec![1u128]; // B(0)=1
                for _ in 1..=n {
                    let mut next_row = vec![*prev_row.last().unwrap()];
                    for j in 0..prev_row.len() {
                        next_row.push(next_row[j] + prev_row[j]);
                    }
                    bells.push(next_row[0]);
                    prev_row = next_row;
                }
                let b_strs: Vec<String> = bells
                    .iter()
                    .enumerate()
                    .map(|(i, b)| format!("B({})={}", i, b))
                    .collect();
                for chunk in b_strs.chunks(4) {
                    let _ = writeln!(out, "  {}", chunk.join("  "));
                }
                let _ = writeln!(out, "  B({}) = {}", n, bells[n]);
            } else {
                let _ = writeln!(out, "  Usage: hematite --combinatorics 'bell n'");
            }
        }
        "stirling" => {
            if let (Some(n), Some(k)) = (parse_n(1), parse_n(2)) {
                let n = n as usize;
                let k = k as usize;
                if k > n {
                    let _ = writeln!(out, "  S({},{}) = 0  (k > n)", n, k);
                } else {
                    // Stirling numbers of the second kind via DP
                    let mut s = vec![vec![0u128; k + 1]; n + 1];
                    s[0][0] = 1;
                    for i in 1..=n {
                        for j in 1..=i.min(k) {
                            s[i][j] = j as u128 * s[i - 1][j] + s[i - 1][j - 1];
                        }
                    }
                    let _ = writeln!(out, "  Stirling number of the 2nd kind S({}, {})", n, k);
                    let _ = writeln!(
                        out,
                        "  = {} ways to partition {} elements into {} non-empty subsets",
                        s[n][k], n, k
                    );
                }
            } else {
                let _ = writeln!(out, "  Usage: hematite --combinatorics 'stirling n k'");
            }
        }
        "multinomial" => {
            // multinomial n k1,k2,k3,...  — n!/(k1!k2!k3!...)
            if let Some(n) = parse_n(1) {
                if let Some(parts_str) = words.get(2) {
                    let parts: Vec<u64> = parts_str
                        .split(',')
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    let parts_sum: u64 = parts.iter().sum();
                    if parts_sum != n {
                        let _ = writeln!(
                            out,
                            "  Error: parts sum ({}) must equal n ({})",
                            parts_sum, n
                        );
                    } else {
                        let num = factorial_big(n);
                        let den: u128 = parts.iter().map(|&k| factorial_big(k)).product();
                        let result = num / den;
                        let parts_str: Vec<String> = parts.iter().map(|x| x.to_string()).collect();
                        let _ = writeln!(
                            out,
                            "  Multinomial({} ; {}) = {}!/({}!)",
                            n,
                            parts_str.join(","),
                            n,
                            parts_str.join("!")
                        );
                        let _ = writeln!(out, "  = {}", result);
                    }
                } else {
                    let _ = writeln!(
                        out,
                        "  Usage: hematite --combinatorics 'multinomial 12 3,4,5'"
                    );
                }
            } else {
                let _ = writeln!(
                    out,
                    "  Usage: hematite --combinatorics 'multinomial n k1,k2,...'"
                );
            }
        }
        "partition" | "partitions" => {
            if let Some(n) = parse_n(1) {
                let n = n.min(100) as usize;
                // Partition function p(n) via Euler's pentagonal recurrence
                let mut p = vec![0u128; n + 1];
                p[0] = 1;
                for i in 1..=n {
                    let mut k: i64 = 1;
                    loop {
                        let pent1 = (k * (3 * k - 1) / 2) as usize;
                        let pent2 = (k * (3 * k + 1) / 2) as usize;
                        if pent1 > i {
                            break;
                        }
                        let sign = if k % 2 == 1 { 1i128 } else { -1i128 };
                        p[i] = (p[i] as i128 + sign * p[i - pent1] as i128) as u128;
                        if pent2 <= i {
                            p[i] = (p[i] as i128 + sign * p[i - pent2] as i128) as u128;
                        }
                        k += 1;
                    }
                }
                let _ = writeln!(out, "  Integer partitions of {}:", n);
                let _ = writeln!(out, "  p({}) = {}", n, p[n]);
                let _ = writeln!(
                    out,
                    "  (number of ways to write {} as an ordered sum of positive integers)",
                    n
                );
                if n <= 10 {
                    let _ = writeln!(out, "  First {} values:", n + 1);
                    let vals: Vec<String> = (0..=n).map(|i| format!("p({})={}", i, p[i])).collect();
                    for chunk in vals.chunks(5) {
                        let _ = writeln!(out, "  {}", chunk.join("  "));
                    }
                }
            } else {
                let _ = writeln!(out, "  Usage: hematite --combinatorics 'partition n'");
            }
        }
        _ => {
            let _ = writeln!(out, "  Unknown command: '{}'", keyword);
            let _ = writeln!(
                out,
                "  Supported: C, P, factorial, derangement, catalan, pascal,"
            );
            let _ = writeln!(out, "             bell, stirling, multinomial, partition");
        }
    }

    let _ = writeln!(out, "{}", sep);
    out
}
