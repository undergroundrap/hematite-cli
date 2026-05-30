use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("factor");
    match action {
        "factor" => action_factor(args),
        "primes" => action_primes(args),
        "gcd" | "lcm" => action_gcd_lcm(args, action),
        "totient" => action_totient(args),
        "modpow" => action_modpow(args),
        "modinv" => action_modinv(args),
        "collatz" => action_collatz(args),
        "fibonacci" => action_fibonacci(args),
        "perfect" => action_perfect(args),
        other => Err(format!(
            "Unknown action '{other}'. Use: factor, primes, gcd, lcm, totient, modpow, modinv, collatz, fibonacci, perfect"
        )),
    }
}

fn trial_factor(mut n: u64) -> Vec<(u64, u32)> {
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
    let mut d = 5u64;
    let mut step = 2u64;
    while d * d <= n {
        if n % d == 0 {
            let mut exp = 0u32;
            while n % d == 0 {
                n /= d;
                exp += 1;
            }
            factors.push((d, exp));
        }
        d += step;
        step = 6 - step;
    }
    if n > 1 {
        factors.push((n, 1));
    }
    factors
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }
    let mut d = 5u64;
    let mut step = 2u64;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += step;
        step = 6 - step;
    }
    true
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

// Extended GCD: returns (gcd, x, y) such that a*x + b*y = gcd
fn gcd_ext(a: i64, b: i64) -> (i64, i64, i64) {
    if b == 0 {
        return (a, 1, 0);
    }
    let (g, x1, y1) = gcd_ext(b, a % b);
    (g, y1, x1 - (a / b) * y1)
}

fn modpow_u64(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result
                .checked_mul(base)
                .map(|v| v % modulus)
                .unwrap_or_else(|| (result as u128 * base as u128 % modulus as u128) as u64);
        }
        exp >>= 1;
        base = base
            .checked_mul(base)
            .map(|v| v % modulus)
            .unwrap_or_else(|| (base as u128 * base as u128 % modulus as u128) as u64);
    }
    result
}

fn parse_n(args: &Value) -> Result<u64, String> {
    args.get("n")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Missing 'n' field (positive integer)".to_string())
}

fn action_factor(args: &Value) -> Result<String, String> {
    let n = parse_n(args)?;
    if n == 0 {
        return Err("n must be a positive integer".to_string());
    }
    let factors = trial_factor(n);

    let mut out = format!("number_theory_tools — factor\n\nn = {}\n\n", n);

    if factors.is_empty() {
        out.push_str("1 (unit, no prime factors)\n");
        return Ok(out);
    }

    out.push_str("Prime factorization:\n");
    let expr: Vec<String> = factors
        .iter()
        .map(|(p, e)| {
            if *e == 1 {
                p.to_string()
            } else {
                format!("{}^{}", p, e)
            }
        })
        .collect();
    out.push_str(&format!("  {} = {}\n", n, expr.join(" × ")));

    out.push_str("\nFactors:\n");
    for (p, e) in &factors {
        out.push_str(&format!("  {:>12}  exp {}\n", p, e));
    }

    // All divisors
    let mut divs: Vec<u64> = vec![1];
    for (p, e) in &factors {
        let len = divs.len();
        let mut pk = 1u64;
        for _ in 0..*e {
            pk *= p;
            for i in 0..len {
                divs.push(divs[i] * pk);
            }
        }
    }
    // Wait, the above generates incorrect divisors. Let me fix:
    // Actually rebuild properly
    let mut divs2: Vec<u64> = vec![1];
    for (p, e) in &factors {
        let old = divs2.clone();
        divs2.clear();
        let mut pk = 1u64;
        for _ in 0..=*e {
            for &d in &old {
                divs2.push(d * pk);
            }
            pk *= p;
        }
    }
    divs2.sort_unstable();

    out.push_str(&format!("\nDivisors ({} total): ", divs2.len()));
    if divs2.len() <= 24 {
        let s: Vec<String> = divs2.iter().map(|d| d.to_string()).collect();
        out.push_str(&s.join(", "));
    } else {
        let head: Vec<String> = divs2[..12].iter().map(|d| d.to_string()).collect();
        let tail: Vec<String> = divs2[divs2.len() - 4..]
            .iter()
            .map(|d| d.to_string())
            .collect();
        out.push_str(&format!("{} ... {}", head.join(", "), tail.join(", ")));
    }
    out.push('\n');

    let sigma: u64 = divs2.iter().sum();
    out.push_str(&format!("\nDivisor sum σ(n): {}\n", sigma));
    if sigma == 2 * n {
        out.push_str("  → PERFECT number\n");
    } else if sigma > 2 * n {
        out.push_str(&format!("  → ABUNDANT (excess {})\n", sigma - 2 * n));
    } else {
        out.push_str(&format!("  → DEFICIENT (deficit {})\n", 2 * n - sigma));
    }

    Ok(out)
}

fn action_primes(args: &Value) -> Result<String, String> {
    let mut out = String::from("number_theory_tools — primes\n\n");

    // Test mode: primality test for specific n
    if let Some(t) = args.get("test").and_then(|v| v.as_u64()) {
        let verdict = if is_prime(t) { "PRIME" } else { "COMPOSITE" };
        out.push_str(&format!("{} is {}\n", t, verdict));
        if !is_prime(t) && t > 1 {
            let factors = trial_factor(t);
            let expr: Vec<String> = factors
                .iter()
                .map(|(p, e)| {
                    if *e == 1 {
                        p.to_string()
                    } else {
                        format!("{}^{}", p, e)
                    }
                })
                .collect();
            out.push_str(&format!("Factorization: {}\n", expr.join(" × ")));
        }
        return Ok(out);
    }

    // Nth prime
    if let Some(nth) = args.get("nth").and_then(|v| v.as_u64()) {
        if nth == 0 {
            return Err("nth must be >= 1".to_string());
        }
        let nth = nth.min(10_000) as usize;
        let mut count = 0usize;
        let mut candidate = 2u64;
        loop {
            if is_prime(candidate) {
                count += 1;
                if count == nth {
                    out.push_str(&format!("The {}th prime is {}\n", nth, candidate));
                    return Ok(out);
                }
            }
            candidate += if candidate == 2 { 1 } else { 2 };
        }
    }

    // List primes up to limit (Sieve of Eratosthenes)
    let limit = args
        .get("limit")
        .or_else(|| args.get("n"))
        .and_then(|v| v.as_u64())
        .unwrap_or(100);
    let limit = limit.min(1_000_000) as usize;

    let mut sieve = vec![true; limit + 1];
    sieve[0] = false;
    if limit > 0 {
        sieve[1] = false;
    }
    let mut i = 2;
    while i * i <= limit {
        if sieve[i] {
            let mut j = i * i;
            while j <= limit {
                sieve[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    let primes: Vec<u64> = sieve
        .iter()
        .enumerate()
        .filter_map(|(i, &p)| if p { Some(i as u64) } else { None })
        .collect();

    out.push_str(&format!(
        "Primes up to {}: {} found\n\n",
        limit,
        primes.len()
    ));
    if primes.len() <= 100 {
        let s: Vec<String> = primes.iter().map(|p| p.to_string()).collect();
        out.push_str(&s.join(", "));
        out.push('\n');
    } else {
        let head: Vec<String> = primes[..20].iter().map(|p| p.to_string()).collect();
        let tail: Vec<String> = primes[primes.len() - 5..]
            .iter()
            .map(|p| p.to_string())
            .collect();
        out.push_str(&format!(
            "{} ... {} (last)\n",
            head.join(", "),
            tail.join(", ")
        ));
    }
    Ok(out)
}

fn action_gcd_lcm(args: &Value, action: &str) -> Result<String, String> {
    let mut out = format!("number_theory_tools — {action}\n\n");

    let get_pair = |args: &Value| -> Result<(u64, u64), String> {
        let a = args
            .get("a")
            .and_then(|v| v.as_u64())
            .ok_or("Missing 'a'")?;
        let b = args
            .get("b")
            .and_then(|v| v.as_u64())
            .ok_or("Missing 'b'")?;
        Ok((a, b))
    };

    // Support array of numbers for GCD/LCM over a list
    if let Some(arr) = args.get("numbers").and_then(|v| v.as_array()) {
        let nums: Vec<u64> = arr
            .iter()
            .enumerate()
            .map(|(i, v)| {
                v.as_u64()
                    .ok_or_else(|| format!("Element {i} is not a non-negative integer"))
            })
            .collect::<Result<_, _>>()?;
        if nums.is_empty() {
            return Err("'numbers' array is empty".to_string());
        }
        let result = if action == "gcd" {
            nums.iter().cloned().reduce(gcd_u64).unwrap()
        } else {
            nums.iter()
                .cloned()
                .reduce(|a, b| {
                    let g = gcd_u64(a, b);
                    a / g * b
                })
                .unwrap()
        };
        out.push_str(&format!(
            "{}({}) = {}\n",
            action.to_uppercase(),
            nums.iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            result
        ));
        return Ok(out);
    }

    let (a, b) = get_pair(args)?;
    let g = gcd_u64(a, b);

    if action == "gcd" {
        let (_, x, y) = gcd_ext(a as i64, b as i64);
        out.push_str(&format!("GCD({}, {}) = {}\n", a, b, g));
        out.push_str(&format!("Bézout:     {}×{} + {}×{} = {}\n", a, x, b, y, g));
        if g == 1 {
            out.push_str("  → {} and {} are COPRIME\n");
        }
    } else {
        let lcm = if g == 0 { 0 } else { a / g * b };
        out.push_str(&format!("LCM({}, {}) = {}\n", a, b, lcm));
        out.push_str(&format!("GCD({}, {}) = {} (used internally)\n", a, b, g));
        out.push_str(&format!(
            "Check:      {} × {} = {} × {}\n",
            a,
            lcm / a,
            b,
            lcm / b
        ));
    }
    Ok(out)
}

fn action_totient(args: &Value) -> Result<String, String> {
    let n = parse_n(args)?;
    if n == 0 {
        return Err("n must be positive".to_string());
    }

    let factors = trial_factor(n);
    // φ(n) = n × ∏(1 - 1/p) for distinct prime factors p
    let phi = factors.iter().fold(n, |acc, (p, _)| acc - acc / p);

    let mut out = format!("number_theory_tools — totient\n\nn = {}\n", n);
    out.push_str(&format!("φ({}) = {}\n", n, phi));
    out.push_str(&format!("\nIntegers 1..{} coprime to {}:\n", n, n));
    if n <= 60 {
        let coprimes: Vec<u64> = (1..n).filter(|&k| gcd_u64(k, n) == 1).collect();
        let s: Vec<String> = coprimes.iter().map(|k| k.to_string()).collect();
        out.push_str(&format!("  {}\n", s.join(", ")));
    } else {
        out.push_str(&format!("  ({} values)\n", phi));
    }
    if is_prime(n) {
        out.push_str(&format!(
            "\n{} is prime → φ({}) = {} - 1 = {}\n",
            n,
            n,
            n,
            n - 1
        ));
    }
    Ok(out)
}

fn action_modpow(args: &Value) -> Result<String, String> {
    let base = args
        .get("base")
        .and_then(|v| v.as_u64())
        .ok_or("Missing 'base'")?;
    let exp = args
        .get("exp")
        .or_else(|| args.get("exponent"))
        .and_then(|v| v.as_u64())
        .ok_or("Missing 'exp'")?;
    let modulus = args
        .get("modulus")
        .or_else(|| args.get("mod"))
        .and_then(|v| v.as_u64())
        .ok_or("Missing 'modulus'")?;
    if modulus == 0 {
        return Err("Modulus cannot be zero".to_string());
    }

    let result = modpow_u64(base, exp, modulus);
    let mut out = format!("number_theory_tools — modpow\n\n");
    out.push_str(&format!("{}^{} mod {} = {}\n", base, exp, modulus, result));
    Ok(out)
}

fn action_modinv(args: &Value) -> Result<String, String> {
    let a = args
        .get("a")
        .and_then(|v| v.as_i64())
        .ok_or("Missing 'a'")?;
    let m = args
        .get("modulus")
        .or_else(|| args.get("m"))
        .and_then(|v| v.as_i64())
        .ok_or("Missing 'modulus'")?;
    if m <= 0 {
        return Err("Modulus must be positive".to_string());
    }

    let (g, x, _) = gcd_ext(a, m);
    let mut out = format!("number_theory_tools — modinv\n\n");
    if g != 1 {
        out.push_str(&format!(
            "No modular inverse: gcd({}, {}) = {} ≠ 1\n",
            a, m, g
        ));
        out.push_str("Modular inverse exists only when a and m are coprime.\n");
    } else {
        let inv = ((x % m) + m) % m;
        out.push_str(&format!("{}⁻¹ mod {} = {}\n", a, m, inv));
        out.push_str(&format!(
            "Verify: {} × {} mod {} = {}\n",
            a,
            inv,
            m,
            ((a * inv).rem_euclid(m))
        ));
    }
    Ok(out)
}

fn action_collatz(args: &Value) -> Result<String, String> {
    let n = parse_n(args)?;
    if n == 0 {
        return Err("n must be a positive integer".to_string());
    }

    let mut seq = vec![n];
    let mut cur = n;
    let max_steps = 10_000usize;
    while cur != 1 && seq.len() < max_steps {
        cur = if cur % 2 == 0 { cur / 2 } else { 3 * cur + 1 };
        seq.push(cur);
    }

    let max_val = *seq.iter().max().unwrap();
    let stopping_time = seq.len() - 1;

    let mut out = format!("number_theory_tools — collatz\n\nStarting value: {}\n", n);
    out.push_str(&format!("Steps to 1:     {}\n", stopping_time));
    out.push_str(&format!("Maximum value:  {}\n", max_val));
    out.push_str(&format!("Sequence length:{}\n\n", seq.len()));

    if seq.len() <= 50 {
        let s: Vec<String> = seq.iter().map(|v| v.to_string()).collect();
        out.push_str(&s.join(" → "));
        out.push('\n');
    } else {
        let head: Vec<String> = seq[..15].iter().map(|v| v.to_string()).collect();
        let tail: Vec<String> = seq[seq.len() - 5..].iter().map(|v| v.to_string()).collect();
        out.push_str(&format!("{} ... {}\n", head.join(" → "), tail.join(" → ")));
    }
    Ok(out)
}

fn action_fibonacci(args: &Value) -> Result<String, String> {
    let mut out = String::from("number_theory_tools — fibonacci\n\n");

    // Check if a number is Fibonacci
    if let Some(t) = args.get("test").and_then(|v| v.as_u64()) {
        // A number is Fibonacci iff 5n²+4 or 5n²-4 is a perfect square
        let is_fib = |n: u64| -> bool {
            let a = 5u128 * n as u128 * n as u128 + 4;
            let b = 5u128 * n as u128 * n as u128 + 4 - 8; // 5n²-4
            let sqrt_a = (a as f64).sqrt() as u128;
            let sqrt_b = (b as f64).sqrt() as u128;
            sqrt_a * sqrt_a == a || (n >= 1 && sqrt_b * sqrt_b == b)
        };
        let verdict = if is_fib(t) { "IS" } else { "IS NOT" };
        out.push_str(&format!("{} {} a Fibonacci number\n", t, verdict));
        return Ok(out);
    }

    // List first N Fibonacci numbers
    let n = args
        .get("n")
        .and_then(|v| v.as_u64())
        .unwrap_or(20)
        .min(100) as usize;
    let (mut a, mut b) = (0u128, 1u128);
    let mut fibs: Vec<u128> = Vec::with_capacity(n);
    for _ in 0..n {
        fibs.push(a);
        let next = a + b;
        a = b;
        b = next;
    }

    if args.get("nth").and_then(|v| v.as_u64()).is_some() {
        let idx = args.get("nth").and_then(|v| v.as_u64()).unwrap() as usize;
        if idx == 0 || idx > n {
            return Err(format!("nth out of range (1–{})", n));
        }
        out.push_str(&format!("F({}) = {}\n", idx, fibs[idx - 1]));
    } else {
        out.push_str(&format!("First {} Fibonacci numbers:\n\n", n));
        for (i, f) in fibs.iter().enumerate() {
            out.push_str(&format!("  F({:>3}) = {}\n", i + 1, f));
        }
        // Golden ratio approximation
        if n >= 2 {
            let ratio = fibs[n - 1] as f64 / fibs[n - 2] as f64;
            out.push_str(&format!(
                "\nF({n})/F({}) ≈ {:.10} (φ ≈ 1.6180339887)\n",
                n - 1,
                ratio
            ));
        }
    }
    Ok(out)
}

fn action_perfect(args: &Value) -> Result<String, String> {
    let mut out = String::from("number_theory_tools — perfect\n\n");

    // List perfect numbers up to limit
    if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
        let limit = limit.min(10_000_000);
        out.push_str(&format!("Perfect numbers up to {}:\n", limit));
        let mut found = 0;
        for n in 2..=limit {
            let sigma: u64 = (1..n).filter(|&d| n % d == 0).sum();
            if sigma == n {
                out.push_str(&format!("  {}\n", n));
                found += 1;
            }
        }
        if found == 0 {
            out.push_str("  None found\n");
        }
        return Ok(out);
    }

    let n = parse_n(args)?;
    let sigma: u64 = (1..n).filter(|&d| n % d == 0).sum();
    out.push_str(&format!("n = {}\n", n));
    out.push_str(&format!("Sum of proper divisors: {}\n", sigma));

    if sigma == n {
        out.push_str("Classification: PERFECT\n");
        out.push_str("  All divisors sum to exactly 2n.\n");
    } else if sigma > n {
        out.push_str(&format!(
            "Classification: ABUNDANT (excess {})\n",
            sigma - n
        ));
    } else if sigma == 1 {
        out.push_str("Classification: PRIME (only divisor is 1)\n");
    } else {
        out.push_str(&format!(
            "Classification: DEFICIENT (deficit {})\n",
            n - sigma
        ));
    }

    if n <= 1000 {
        let divs: Vec<u64> = (1..n).filter(|&d| n % d == 0).collect();
        let s: Vec<String> = divs.iter().map(|d| d.to_string()).collect();
        out.push_str(&format!("Proper divisors: {}\n", s.join(" + ")));
        out.push_str(&format!("  = {}\n", sigma));
    }
    Ok(out)
}
