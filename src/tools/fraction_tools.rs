use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("simplify");
    match action {
        "simplify" => action_simplify(args),
        "add" => action_binop(args, 'a'),
        "sub" => action_binop(args, 's'),
        "mul" => action_binop(args, 'm'),
        "div" => action_binop(args, 'd'),
        "convert" => action_convert(args),
        "compare" => action_compare(args),
        "series" => action_series(args),
        other => Err(format!(
            "Unknown action '{other}'. Use: simplify, add, sub, mul, div, convert, compare, series"
        )),
    }
}

fn gcd64(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

#[derive(Clone, Copy, Debug)]
struct Frac {
    num: i64,
    den: i64,
}

impl Frac {
    fn new(num: i64, den: i64) -> Result<Self, String> {
        if den == 0 {
            return Err("Denominator cannot be zero".to_string());
        }
        let g = gcd64(num, den);
        let sign = if den < 0 { -1i64 } else { 1i64 };
        Ok(Frac {
            num: sign * num / g,
            den: sign * den / g,
        })
    }

    fn from_str(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if let Some((n, d)) = s.split_once('/') {
            let num = n
                .trim()
                .parse::<i64>()
                .map_err(|_| format!("Bad numerator: '{}'", n.trim()))?;
            let den = d
                .trim()
                .parse::<i64>()
                .map_err(|_| format!("Bad denominator: '{}'", d.trim()))?;
            Frac::new(num, den)
        } else {
            let n = s
                .parse::<i64>()
                .map_err(|_| format!("Cannot parse '{}' as fraction", s))?;
            Ok(Frac { num: n, den: 1 })
        }
    }

    fn parse_field(args: &Value, key: &str) -> Result<Self, String> {
        if let Some(s) = args.get(key).and_then(|v| v.as_str()) {
            Self::from_str(s)
        } else if let Some(n) = args.get(key).and_then(|v| v.as_i64()) {
            Ok(Frac { num: n, den: 1 })
        } else {
            Err(format!(
                "Missing or invalid field '{key}'. Expected a fraction string like \"3/4\" or an integer."
            ))
        }
    }

    fn display(self) -> String {
        if self.den == 1 {
            self.num.to_string()
        } else {
            format!("{}/{}", self.num, self.den)
        }
    }

    fn mixed(self) -> Option<String> {
        if self.den == 1 || self.num.abs() < self.den.abs() {
            return None;
        }
        let whole = self.num / self.den;
        let rem = self.num % self.den;
        if rem == 0 {
            None
        } else {
            Some(format!("{} {}/{}", whole, rem.abs(), self.den))
        }
    }

    fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    fn add(self, o: Self) -> Result<Self, String> {
        let num = self
            .num
            .checked_mul(o.den)
            .and_then(|a| a.checked_add(o.num.checked_mul(self.den)?))
            .ok_or("Overflow in addition")?;
        let den = self.den.checked_mul(o.den).ok_or("Overflow")?;
        Frac::new(num, den)
    }

    fn sub(self, o: Self) -> Result<Self, String> {
        let num = self
            .num
            .checked_mul(o.den)
            .and_then(|a| a.checked_sub(o.num.checked_mul(self.den)?))
            .ok_or("Overflow in subtraction")?;
        let den = self.den.checked_mul(o.den).ok_or("Overflow")?;
        Frac::new(num, den)
    }

    fn mul(self, o: Self) -> Result<Self, String> {
        let num = self
            .num
            .checked_mul(o.num)
            .ok_or("Overflow in multiplication")?;
        let den = self.den.checked_mul(o.den).ok_or("Overflow")?;
        Frac::new(num, den)
    }

    fn div(self, o: Self) -> Result<Self, String> {
        if o.num == 0 {
            return Err("Division by zero".to_string());
        }
        self.mul(Frac {
            num: o.den,
            den: o.num,
        })
    }
}

fn action_simplify(args: &Value) -> Result<String, String> {
    let (orig_num, orig_den, f) = if let Some(s) = args.get("fraction").and_then(|v| v.as_str()) {
        let s = s.trim();
        if let Some((n, d)) = s.split_once('/') {
            let on = n
                .trim()
                .parse::<i64>()
                .map_err(|_| format!("Bad numerator: '{}'", n.trim()))?;
            let od = d
                .trim()
                .parse::<i64>()
                .map_err(|_| format!("Bad denominator: '{}'", d.trim()))?;
            (Some(on), Some(od), Frac::new(on, od)?)
        } else {
            let n = s
                .parse::<i64>()
                .map_err(|_| format!("Cannot parse: '{s}'"))?;
            (Some(n), Some(1), Frac { num: n, den: 1 })
        }
    } else {
        let num = args
            .get("numerator")
            .and_then(|v| v.as_i64())
            .ok_or("Provide 'fraction' (\"3/4\") or 'numerator'/'denominator'")?;
        let den = args
            .get("denominator")
            .and_then(|v| v.as_i64())
            .ok_or("Missing 'denominator'")?;
        (Some(num), Some(den), Frac::new(num, den)?)
    };

    let mut out = String::from("fraction_tools — simplify\n\n");
    if let (Some(on), Some(od)) = (orig_num, orig_den) {
        if od != 0 {
            out.push_str(&format!("Input:      {}/{}\n", on, od));
            out.push_str(&format!("GCD:        {}\n", gcd64(on, od)));
        }
    }
    out.push_str(&format!("Simplified: {}\n", f.display()));
    if let Some(m) = f.mixed() {
        out.push_str(&format!("Mixed:      {}\n", m));
    }
    out.push_str(&format!("Decimal:    {:.10}\n", f.to_f64()));
    out.push_str(&format!("Percent:    {:.4}%\n", f.to_f64() * 100.0));
    Ok(out)
}

fn action_binop(args: &Value, op: char) -> Result<String, String> {
    let a = Frac::parse_field(args, "a")?;
    let b = Frac::parse_field(args, "b")?;
    let (result, symbol, name) = match op {
        'a' => (a.add(b)?, "+", "add"),
        's' => (a.sub(b)?, "\u{2212}", "sub"),
        'm' => (a.mul(b)?, "\u{00d7}", "mul"),
        'd' => (a.div(b)?, "\u{00f7}", "div"),
        _ => unreachable!(),
    };
    let mut out = format!("fraction_tools — {name}\n\n");
    out.push_str(&format!(
        "  {} {} {} = {}\n",
        a.display(),
        symbol,
        b.display(),
        result.display()
    ));
    if let Some(m) = result.mixed() {
        out.push_str(&format!("  Mixed:   {}\n", m));
    }
    out.push_str(&format!("  Decimal: {:.10}\n", result.to_f64()));
    out.push_str(&format!("  Percent: {:.4}%\n", result.to_f64() * 100.0));
    Ok(out)
}

fn decimal_to_frac(x: f64, tol: f64) -> Result<Frac, String> {
    if x.is_nan() || x.is_infinite() {
        return Err("Cannot convert NaN or infinite value".to_string());
    }
    let neg = x < 0.0;
    let x = x.abs();
    // Continued fraction algorithm
    let (mut h0, mut h1) = (0i64, 1i64);
    let (mut k0, mut k1) = (1i64, 0i64);
    let mut rem = x;
    for _ in 0..64 {
        let a = rem.floor() as i64;
        let h2 = a * h1 + h0;
        let k2 = a * k1 + k0;
        if k2 == 0 {
            break;
        }
        if (h2 as f64 / k2 as f64 - x).abs() < tol {
            let sign = if neg { -1 } else { 1 };
            return Frac::new(sign * h2, k2);
        }
        h0 = h1;
        h1 = h2;
        k0 = k1;
        k1 = k2;
        let frac_part = rem - rem.floor();
        if frac_part < 1e-12 {
            break;
        }
        rem = 1.0 / frac_part;
    }
    let sign = if neg { -1 } else { 1 };
    Frac::new(sign * h1, k1)
}

fn action_convert(args: &Value) -> Result<String, String> {
    let mut out = String::from("fraction_tools — convert\n\n");
    if let Some(s) = args.get("fraction").and_then(|v| v.as_str()) {
        let f = Frac::from_str(s)?;
        out.push_str(&format!("Fraction:  {}\n", f.display()));
        out.push_str(&format!("Decimal:   {:.10}\n", f.to_f64()));
        out.push_str(&format!("Percent:   {:.4}%\n", f.to_f64() * 100.0));
        if let Some(m) = f.mixed() {
            out.push_str(&format!("Mixed:     {}\n", m));
        }
    } else if let Some(d) = args.get("decimal").and_then(|v| v.as_f64()) {
        let tol = args
            .get("tolerance")
            .and_then(|v| v.as_f64())
            .unwrap_or(1e-6);
        let f = decimal_to_frac(d, tol)?;
        out.push_str(&format!("Decimal:   {}\n", d));
        out.push_str(&format!("Fraction:  {}\n", f.display()));
        out.push_str(&format!("Approx:    {:.10}\n", f.to_f64()));
        out.push_str(&format!("Error:     {:.2e}\n", (d - f.to_f64()).abs()));
    } else {
        return Err("Provide 'fraction' (string like \"3/4\") or 'decimal' (number)".to_string());
    }
    Ok(out)
}

fn action_compare(args: &Value) -> Result<String, String> {
    let fracs: Vec<Frac> = if let Some(arr) = args.get("fractions").and_then(|v| v.as_array()) {
        arr.iter()
            .enumerate()
            .map(|(i, v)| {
                if let Some(s) = v.as_str() {
                    Frac::from_str(s)
                } else if let Some(n) = v.as_i64() {
                    Ok(Frac { num: n, den: 1 })
                } else {
                    Err(format!("Element {i}: expected fraction string"))
                }
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![Frac::parse_field(args, "a")?, Frac::parse_field(args, "b")?]
    };

    if fracs.len() < 2 {
        return Err("Provide at least 2 fractions via 'a'/'b' or 'fractions' array".to_string());
    }

    let mut out = String::from("fraction_tools — compare\n\n");

    if fracs.len() == 2 {
        let a = fracs[0];
        let b = fracs[1];
        let diff = a.sub(b)?;
        let rel = if diff.num == 0 {
            "="
        } else if diff.num > 0 {
            ">"
        } else {
            "<"
        };
        out.push_str(&format!("  {} {} {}\n", a.display(), rel, b.display()));
        out.push_str(&format!(
            "  {} ({:.6}) vs {} ({:.6})\n",
            a.display(),
            a.to_f64(),
            b.display(),
            b.to_f64()
        ));
        if diff.num != 0 {
            out.push_str(&format!("  Difference: {}\n", diff.display()));
        }
    } else {
        let mut indexed: Vec<(usize, Frac)> = fracs.iter().cloned().enumerate().collect();
        indexed.sort_by(|x, y| {
            x.1.to_f64()
                .partial_cmp(&y.1.to_f64())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out.push_str("Sorted ascending:\n");
        for (rank, (orig, f)) in indexed.iter().enumerate() {
            out.push_str(&format!(
                "  {}. {} (input #{}) = {:.8}\n",
                rank + 1,
                f.display(),
                orig + 1,
                f.to_f64()
            ));
        }
    }
    Ok(out)
}

fn action_series(args: &Value) -> Result<String, String> {
    let kind = args
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("harmonic");
    let mut out = format!("fraction_tools — series ({kind})\n\n");

    match kind {
        "harmonic" => {
            let terms = args
                .get("terms")
                .and_then(|v| v.as_u64())
                .unwrap_or(10)
                .min(50) as usize;
            let mut sum = Frac { num: 0, den: 1 };
            out.push_str(&format!(
                "{:<5} {:>10} {:>24} {:>12}\n",
                "n", "Term", "Partial Sum", "Decimal"
            ));
            out.push_str(&format!("{}\n", "-".repeat(55)));
            for n in 1..=terms {
                let term = Frac::new(1, n as i64)?;
                sum = sum.add(term)?;
                out.push_str(&format!(
                    "{:<5} {:>10} {:>24} {:>12.6}\n",
                    n,
                    term.display(),
                    sum.display(),
                    sum.to_f64()
                ));
            }
        }
        "egyptian" => {
            let f = if let Some(s) = args.get("fraction").and_then(|v| v.as_str()) {
                Frac::from_str(s)?
            } else {
                return Err("Egyptian series requires 'fraction' arg e.g. \"3/7\"".to_string());
            };
            if f.num <= 0 || f.num >= f.den {
                return Err(
                    "Egyptian decomposition requires a proper positive fraction (0 < p/q < 1)"
                        .to_string(),
                );
            }
            out.push_str(&format!(
                "Egyptian fraction decomposition of {}:\n\n",
                f.display()
            ));
            let mut rem = f;
            let mut units: Vec<Frac> = Vec::new();
            for _ in 0..30 {
                if rem.num <= 0 {
                    break;
                }
                let ceil_den = ((rem.den as f64) / (rem.num as f64)).ceil() as i64;
                let unit = Frac::new(1, ceil_den)?;
                units.push(unit);
                rem = rem.sub(unit)?;
                if rem.num == 0 {
                    break;
                }
            }
            let expr = units
                .iter()
                .map(|u| u.display())
                .collect::<Vec<_>>()
                .join(" + ");
            out.push_str(&format!("{} = {}\n", f.display(), expr));
        }
        "farey" => {
            let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(7).min(20) as i64;
            out.push_str(&format!("Farey sequence F{}:\n\n", n));
            let mut seq: Vec<(i64, i64)> = vec![(0, 1)];
            for q in 1..=n {
                for p in 1..=q {
                    if gcd64(p, q) == 1 {
                        seq.push((p, q));
                    }
                }
            }
            seq.sort_by(|a, b| (a.0 * b.1).cmp(&(b.0 * a.1)));
            let parts: Vec<String> = seq
                .iter()
                .map(|(p, q)| {
                    if *q == 1 {
                        p.to_string()
                    } else {
                        format!("{}/{}", p, q)
                    }
                })
                .collect();
            out.push_str(&parts.join(", "));
            out.push_str(&format!("\n\nTerms: {}\n", seq.len()));
        }
        other => {
            return Err(format!(
                "Unknown series type '{other}'. Use: harmonic, egyptian, farey"
            ))
        }
    }
    Ok(out)
}
