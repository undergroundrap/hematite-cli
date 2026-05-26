pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("convert");

    match action {
        "convert" => convert_base(args),
        "format" => format_number(args),
        "roman" => to_roman(args),
        "from-roman" => from_roman(args),
        "si" => si_prefix(args),
        "factors" => factorize(args),
        "gcd" => gcd_lcm(args),
        "clamp" => clamp(args),
        other => Err(format!(
            "number_tools: unknown action '{other}'. Valid: convert, format, roman, from-roman, si, factors, gcd, clamp"
        )),
    }
}

fn get_integer(args: &serde_json::Value) -> Result<i128, String> {
    // Accept number or string
    if let Some(n) = args.get("input").and_then(|v| v.as_i64()) {
        return Ok(n as i128);
    }
    if let Some(s) = args.get("input").and_then(|v| v.as_str()) {
        let s = s.trim().replace('_', "").replace(',', "");
        // Detect prefix
        if s.starts_with("0x") || s.starts_with("0X") {
            return i128::from_str_radix(&s[2..], 16)
                .map_err(|e| format!("number_tools: invalid hex '{s}': {e}"));
        }
        if s.starts_with("0b") || s.starts_with("0B") {
            return i128::from_str_radix(&s[2..], 2)
                .map_err(|e| format!("number_tools: invalid binary '{s}': {e}"));
        }
        if s.starts_with("0o") || s.starts_with("0O") {
            return i128::from_str_radix(&s[2..], 8)
                .map_err(|e| format!("number_tools: invalid octal '{s}': {e}"));
        }
        return s
            .parse::<i128>()
            .map_err(|e| format!("number_tools: invalid number '{s}': {e}"));
    }
    Err("number_tools: 'input' is required (number or numeric string)".into())
}

fn get_float(args: &serde_json::Value) -> Result<f64, String> {
    if let Some(n) = args.get("input").and_then(|v| v.as_f64()) {
        return Ok(n);
    }
    if let Some(s) = args.get("input").and_then(|v| v.as_str()) {
        return s
            .trim()
            .replace(',', "")
            .parse::<f64>()
            .map_err(|e| format!("number_tools: invalid number '{s}': {e}"));
    }
    Err("number_tools: 'input' is required".into())
}

fn convert_base(args: &serde_json::Value) -> Result<String, String> {
    let n = get_integer(args)?;
    let to_base = args.get("to").and_then(|v| v.as_u64()).unwrap_or(0);

    // If no explicit target, show all common bases
    if to_base == 0 {
        let unsigned = n as u128;
        let mut out = format!("NUMBER CONVERT\n{}\n", "─".repeat(50));
        out.push_str(&format!("Input   : {n}\n\n"));
        out.push_str(&format!("Decimal : {n}\n"));
        out.push_str(&format!(
            "Hex     : 0x{:X}  (lowercase: 0x{:x})\n",
            unsigned, unsigned
        ));
        out.push_str(&format!("Binary  : 0b{:b}\n", unsigned));
        out.push_str(&format!("Octal   : 0o{:o}\n", unsigned));
        out.push_str(&format!("Base36  : {}\n", to_base36(unsigned)));
        if n >= 0 && n <= 3999 {
            if let Some(r) = int_to_roman(n as u32) {
                out.push_str(&format!("Roman   : {r}\n"));
            }
        }
        return Ok(out);
    }

    let result = match to_base {
        2 => format!("0b{:b}", n as u128),
        8 => format!("0o{:o}", n as u128),
        10 => format!("{n}"),
        16 => format!("0x{:X}", n as u128),
        36 => to_base36(n as u128),
        b if b >= 2 && b <= 36 => to_base_n(n as u128, b as u32),
        _ => {
            return Err(format!(
                "number_tools: unsupported base {to_base} (use 2–36)"
            ))
        }
    };

    Ok(format!(
        "NUMBER CONVERT\n{}\nInput  : {n}\nBase   : {to_base}\nResult : {result}",
        "─".repeat(50)
    ))
}

fn to_base_n(mut n: u128, base: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let digits: Vec<char> = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect();
    let mut result = Vec::new();
    while n > 0 {
        result.push(digits[(n % base as u128) as usize]);
        n /= base as u128;
    }
    result.iter().rev().collect()
}

fn to_base36(n: u128) -> String {
    to_base_n(n, 36)
}

fn format_number(args: &serde_json::Value) -> Result<String, String> {
    let n = get_float(args)?;
    let decimals = args.get("decimals").and_then(|v| v.as_u64()).unwrap_or(2) as usize;

    let abs = n.abs();
    let sign = if n < 0.0 { "-" } else { "" };

    // Thousands-separated decimal
    let formatted = format_with_commas(n, decimals);

    // Scientific notation
    let scientific = format!("{sign}{:.prec$e}", abs, prec = decimals)
        .replace("e", " × 10^")
        .replace("× 10^-", "× 10⁻")
        .replace("× 10^", "× 10^");

    // Engineering (exponent multiple of 3)
    let engineering = engineering_notation(n, decimals);

    // SI prefix
    let si = si_format(n);

    let mut out = format!("NUMBER FORMAT\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input       : {n}\n"));
    out.push_str(&format!("Decimal     : {formatted}\n"));
    out.push_str(&format!("Scientific  : {scientific}\n"));
    out.push_str(&format!("Engineering : {engineering}\n"));
    out.push_str(&format!("SI prefix   : {si}\n"));
    out.push_str(&format!(
        "Binary repr : {} bits (i64 range)\n",
        if abs <= i64::MAX as f64 { "64" } else { ">64" }
    ));
    Ok(out)
}

fn format_with_commas(n: f64, decimals: usize) -> String {
    let formatted = format!("{:.prec$}", n.abs(), prec = decimals);
    let parts: Vec<&str> = formatted.splitn(2, '.').collect();
    let int_part = parts[0];
    let dec_part = parts.get(1).copied().unwrap_or("");

    let mut result = String::new();
    for (i, ch) in int_part.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    let mut int_str: String = result.chars().rev().collect();
    if n < 0.0 {
        int_str = format!("-{int_str}");
    }
    if decimals > 0 {
        format!("{int_str}.{dec_part}")
    } else {
        int_str
    }
}

fn engineering_notation(n: f64, decimals: usize) -> String {
    if n == 0.0 {
        return format!("0.{:0<prec$}e+0", "", prec = decimals);
    }
    let exp = n.abs().log10().floor() as i32;
    let eng_exp = (exp / 3) * 3;
    let mantissa = n / 10f64.powi(eng_exp);
    format!("{:.prec$}e{:+}", mantissa, eng_exp, prec = decimals)
}

fn si_format(n: f64) -> String {
    let abs = n.abs();
    let (divisor, prefix) = if abs >= 1e24 {
        (1e24, "Y")
    } else if abs >= 1e21 {
        (1e21, "Z")
    } else if abs >= 1e18 {
        (1e18, "E")
    } else if abs >= 1e15 {
        (1e15, "P")
    } else if abs >= 1e12 {
        (1e12, "T")
    } else if abs >= 1e9 {
        (1e9, "G")
    } else if abs >= 1e6 {
        (1e6, "M")
    } else if abs >= 1e3 {
        (1e3, "k")
    } else if abs >= 1.0 || abs == 0.0 {
        (1.0, "")
    } else if abs >= 1e-3 {
        (1e-3, "m")
    } else if abs >= 1e-6 {
        (1e-6, "μ")
    } else if abs >= 1e-9 {
        (1e-9, "n")
    } else {
        (1e-12, "p")
    };
    format!("{:.3}{prefix}", n / divisor)
}

fn to_roman(args: &serde_json::Value) -> Result<String, String> {
    let n = get_integer(args)?;
    if n < 1 || n > 3999 {
        return Err(format!("number_tools roman: {n} is out of range (1–3999)"));
    }
    let roman = int_to_roman(n as u32).unwrap();
    Ok(format!("ROMAN NUMERAL\n{}\n{n} → {roman}", "─".repeat(50)))
}

fn int_to_roman(mut n: u32) -> Option<String> {
    const VALS: &[(u32, &str)] = &[
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
    if n == 0 || n > 3999 {
        return None;
    }
    let mut result = String::new();
    for &(val, sym) in VALS {
        while n >= val {
            result.push_str(sym);
            n -= val;
        }
    }
    Some(result)
}

fn from_roman(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("number_tools from-roman: 'input' is required")?;

    let upper = input.trim().to_uppercase();
    let val_map = |c| match c {
        'I' => Some(1u32),
        'V' => Some(5),
        'X' => Some(10),
        'L' => Some(50),
        'C' => Some(100),
        'D' => Some(500),
        'M' => Some(1000),
        _ => None,
    };

    let chars: Vec<u32> = upper
        .chars()
        .map(|c| val_map(c).ok_or_else(|| format!("number_tools: invalid Roman character '{c}'")))
        .collect::<Result<Vec<_>, _>>()?;

    let mut total = 0u32;
    for i in 0..chars.len() {
        if i + 1 < chars.len() && chars[i] < chars[i + 1] {
            total = total.saturating_sub(chars[i]);
        } else {
            total = total.saturating_add(chars[i]);
        }
    }

    Ok(format!("FROM ROMAN\n{}\n{input} → {total}", "─".repeat(50)))
}

fn si_prefix(args: &serde_json::Value) -> Result<String, String> {
    let n = get_float(args)?;

    let mut out = format!("SI PREFIX\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input : {n}\n\n"));

    const PREFIXES: &[(f64, &str, &str)] = &[
        (1e24, "Y", "yotta"),
        (1e21, "Z", "zetta"),
        (1e18, "E", "exa"),
        (1e15, "P", "peta"),
        (1e12, "T", "tera"),
        (1e9, "G", "giga"),
        (1e6, "M", "mega"),
        (1e3, "k", "kilo"),
        (1.0, "", ""),
        (1e-3, "m", "milli"),
        (1e-6, "μ", "micro"),
        (1e-9, "n", "nano"),
        (1e-12, "p", "pico"),
        (1e-15, "f", "femto"),
    ];

    for &(scale, sym, name) in PREFIXES {
        let val = n / scale;
        if val.abs() >= 0.1 && val.abs() < 1000.0 {
            let label = if name.is_empty() {
                "(base)".to_string()
            } else {
                format!("{name} ({sym})")
            };
            out.push_str(&format!("  {:.6} {label}\n", val));
        }
    }
    Ok(out)
}

fn factorize(args: &serde_json::Value) -> Result<String, String> {
    let n = get_integer(args)?;
    if n < 2 {
        return Err(format!("number_tools factors: {n} must be ≥ 2"));
    }
    if n > 1_000_000_000 {
        return Err("number_tools factors: input too large (max 1,000,000,000)".into());
    }

    let mut factors = Vec::new();
    let mut remaining = n as u64;
    let mut d = 2u64;
    while d * d <= remaining {
        while remaining % d == 0 {
            factors.push(d);
            remaining /= d;
        }
        d += 1;
    }
    if remaining > 1 {
        factors.push(remaining);
    }

    let is_prime = factors.len() == 1;
    let expr = factors
        .iter()
        .fold(std::collections::HashMap::new(), |mut map, &f| {
            *map.entry(f).or_insert(0u32) += 1;
            map
        });
    let mut sorted_factors: Vec<_> = expr.iter().collect();
    sorted_factors.sort_by_key(|(&k, _)| k);
    let factored: Vec<String> = sorted_factors
        .iter()
        .map(|(&base, &exp)| {
            if exp == 1 {
                base.to_string()
            } else {
                format!("{base}^{exp}")
            }
        })
        .collect();

    let mut out = format!("PRIME FACTORS\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input      : {n}\n"));
    out.push_str(&format!("Is prime   : {is_prime}\n"));
    out.push_str(&format!("Factors    : {} = {}\n", n, factored.join(" × ")));
    out.push_str(&format!("Factor list: {:?}\n", factors));
    Ok(out)
}

fn gcd_lcm(args: &serde_json::Value) -> Result<String, String> {
    let a_val = args
        .get("a")
        .and_then(|v| v.as_i64())
        .ok_or("number_tools gcd: 'a' is required")?
        .unsigned_abs();
    let b_val = args
        .get("b")
        .and_then(|v| v.as_i64())
        .ok_or("number_tools gcd: 'b' is required")?
        .unsigned_abs();

    let gcd = euclid_gcd(a_val, b_val);
    let lcm = if gcd == 0 { 0 } else { a_val / gcd * b_val };

    Ok(format!(
        "GCD / LCM\n{}\na   : {a_val}\nb   : {b_val}\nGCD : {gcd}\nLCM : {lcm}",
        "─".repeat(50)
    ))
}

fn euclid_gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        euclid_gcd(b, a % b)
    }
}

fn clamp(args: &serde_json::Value) -> Result<String, String> {
    let n = get_float(args)?;
    let min = args
        .get("min")
        .and_then(|v| v.as_f64())
        .ok_or("number_tools clamp: 'min' is required")?;
    let max = args
        .get("max")
        .and_then(|v| v.as_f64())
        .ok_or("number_tools clamp: 'max' is required")?;

    let clamped = n.clamp(min, max);
    let status = if n < min {
        "below minimum → clamped to min"
    } else if n > max {
        "above maximum → clamped to max"
    } else {
        "within range → unchanged"
    };

    Ok(format!(
        "CLAMP\n{}\nInput  : {n}\nMin    : {min}\nMax    : {max}\nResult : {clamped}  ({status})",
        "─".repeat(50)
    ))
}
