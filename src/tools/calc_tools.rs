use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("eval");
    match action {
        "eval" | "calculate" | "compute" => action_eval(args),
        "rpn" => action_rpn(args),
        "variables" | "vars" => action_variables(args),
        "sequence" => action_sequence(args),
        other => Err(format!(
            "calc_tools: unknown action '{other}'. Valid: eval, rpn, variables, sequence"
        )),
    }
}

// ── Tokenizer ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    LParen,
    RParen,
    Ident(String),
    Comma,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '\n' => i += 1,
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '%' => {
                tokens.push(Token::Percent);
                i += 1;
            }
            '^' | '²' | '³' => {
                tokens.push(Token::Caret);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '0'..='9' | '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let num_str: String = chars[start..i].iter().collect();
                let n: f64 = num_str
                    .parse()
                    .map_err(|_| format!("Invalid number: '{}'", num_str))?;
                tokens.push(Token::Number(n));
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push(Token::Ident(word));
            }
            c => {
                return Err(format!("Unknown character: '{}'", c));
            }
        }
    }
    Ok(tokens)
}

// ── Recursive-descent parser ──────────────────────────────────────────────────

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    vars: std::collections::HashMap<String, f64>,
}

impl Parser {
    fn new(tokens: Vec<Token>, vars: std::collections::HashMap<String, f64>) -> Self {
        Parser {
            tokens,
            pos: 0,
            vars,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn consume(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn parse_expr(&mut self) -> Result<f64, String> {
        self.parse_add()
    }

    fn parse_add(&mut self) -> Result<f64, String> {
        let mut left = self.parse_mul()?;
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.consume();
                    left += self.parse_mul()?;
                }
                Some(Token::Minus) => {
                    self.consume();
                    left -= self.parse_mul()?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<f64, String> {
        let mut left = self.parse_pow()?;
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.consume();
                    left *= self.parse_pow()?;
                }
                Some(Token::Slash) => {
                    self.consume();
                    let rhs = self.parse_pow()?;
                    if rhs == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    left /= rhs;
                }
                Some(Token::Percent) => {
                    self.consume();
                    let rhs = self.parse_pow()?;
                    if rhs == 0.0 {
                        return Err("Modulo by zero".to_string());
                    }
                    left = left.rem_euclid(rhs);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_pow(&mut self) -> Result<f64, String> {
        let base = self.parse_unary()?;
        if matches!(self.peek(), Some(Token::Caret)) {
            self.consume();
            let exp = self.parse_pow()?; // right-associative
            Ok(base.powf(exp))
        } else {
            Ok(base)
        }
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        if matches!(self.peek(), Some(Token::Minus)) {
            self.consume();
            return Ok(-self.parse_call()?);
        }
        if matches!(self.peek(), Some(Token::Plus)) {
            self.consume();
        }
        self.parse_call()
    }

    fn parse_call(&mut self) -> Result<f64, String> {
        let atom = self.parse_atom()?;
        // Look for implicit multiplication: number followed by ( — e.g. 2(3+4)
        if matches!(self.peek(), Some(Token::LParen)) {
            // Only if atom was from a number, not a function call
            // We handle function calls in parse_atom, so this is implicit mul
            let inner = self.parse_atom()?;
            return Ok(atom * inner);
        }
        Ok(atom)
    }

    fn parse_atom(&mut self) -> Result<f64, String> {
        match self.consume() {
            Some(Token::Number(n)) => Ok(n),
            Some(Token::LParen) => {
                let val = self.parse_expr()?;
                match self.consume() {
                    Some(Token::RParen) => Ok(val),
                    _ => Err("Expected ')'".to_string()),
                }
            }
            Some(Token::Ident(name)) => {
                // Function call?
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.consume(); // eat (
                    let mut args_vals = Vec::new();
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        args_vals.push(self.parse_expr()?);
                        while matches!(self.peek(), Some(Token::Comma)) {
                            self.consume();
                            args_vals.push(self.parse_expr()?);
                        }
                    }
                    match self.consume() {
                        Some(Token::RParen) => {}
                        _ => return Err(format!("Expected ')' after function '{}'", name)),
                    }
                    self.call_fn(&name, &args_vals)
                } else {
                    // Variable or constant
                    match name.to_lowercase().as_str() {
                        "pi" | "π" => Ok(std::f64::consts::PI),
                        "e" => Ok(std::f64::consts::E),
                        "tau" | "τ" => Ok(std::f64::consts::TAU),
                        "phi" | "φ" => Ok(1.618_033_988_749_895),
                        "inf" | "infinity" => Ok(f64::INFINITY),
                        _ => self
                            .vars
                            .get(&name)
                            .copied()
                            .ok_or_else(|| format!("Unknown variable: '{}'", name)),
                    }
                }
            }
            Some(t) => Err(format!("Unexpected token: {:?}", t)),
            None => Err("Unexpected end of expression".to_string()),
        }
    }

    fn call_fn(&self, name: &str, args: &[f64]) -> Result<f64, String> {
        let get1 = || -> Result<f64, String> {
            args.first()
                .copied()
                .ok_or_else(|| format!("{}() requires 1 argument", name))
        };
        let get2 = || -> Result<(f64, f64), String> {
            if args.len() < 2 {
                Err(format!("{}() requires 2 arguments", name))
            } else {
                Ok((args[0], args[1]))
            }
        };
        match name.to_lowercase().as_str() {
            "sqrt" | "√" => Ok(get1()?.sqrt()),
            "cbrt" => Ok(get1()?.cbrt()),
            "abs" => Ok(get1()?.abs()),
            "ceil" => Ok(get1()?.ceil()),
            "floor" => Ok(get1()?.floor()),
            "round" => Ok(get1()?.round()),
            "trunc" => Ok(get1()?.trunc()),
            "fract" => Ok(get1()?.fract()),
            "ln" => {
                let x = get1()?;
                if x <= 0.0 {
                    return Err("ln requires positive argument".to_string());
                }
                Ok(x.ln())
            }
            "log" | "log10" => {
                let x = get1()?;
                if x <= 0.0 {
                    return Err("log requires positive argument".to_string());
                }
                Ok(x.log10())
            }
            "log2" => {
                let x = get1()?;
                if x <= 0.0 {
                    return Err("log2 requires positive argument".to_string());
                }
                Ok(x.log2())
            }
            "logb" => {
                let (x, b) = get2()?;
                if x <= 0.0 || b <= 0.0 || b == 1.0 {
                    return Err("logb(x, base): x>0 and base>0 and base≠1".to_string());
                }
                Ok(x.log(b))
            }
            "exp" => Ok(get1()?.exp()),
            "exp2" => Ok(get1()?.exp2()),
            "sin" => Ok(get1()?.sin()),
            "cos" => Ok(get1()?.cos()),
            "tan" => Ok(get1()?.tan()),
            "asin" => Ok(get1()?.asin()),
            "acos" => Ok(get1()?.acos()),
            "atan" => Ok(get1()?.atan()),
            "atan2" => {
                let (y, x) = get2()?;
                Ok(y.atan2(x))
            }
            "sinh" => Ok(get1()?.sinh()),
            "cosh" => Ok(get1()?.cosh()),
            "tanh" => Ok(get1()?.tanh()),
            "deg" | "degrees" => Ok(get1()?.to_degrees()),
            "rad" | "radians" => Ok(get1()?.to_radians()),
            "pow" | "power" => {
                let (b, e) = get2()?;
                Ok(b.powf(e))
            }
            "max" => args
                .iter()
                .copied()
                .reduce(f64::max)
                .ok_or_else(|| "max() requires arguments".to_string()),
            "min" => args
                .iter()
                .copied()
                .reduce(f64::min)
                .ok_or_else(|| "min() requires arguments".to_string()),
            "sum" => Ok(args.iter().sum()),
            "avg" | "mean" => {
                if args.is_empty() {
                    Err("avg() requires arguments".to_string())
                } else {
                    Ok(args.iter().sum::<f64>() / args.len() as f64)
                }
            }
            "hypot" => {
                let (a, b) = get2()?;
                Ok(a.hypot(b))
            }
            "sign" | "signum" => Ok(get1()?.signum()),
            "factorial" | "fact" => {
                let x = get1()?;
                if x < 0.0 || x.fract() != 0.0 || x > 20.0 {
                    return Err("factorial requires non-negative integer ≤ 20".to_string());
                }
                Ok((1..=(x as u64)).product::<u64>() as f64)
            }
            "gcd" => {
                let (a, b) = get2()?;
                let (mut ai, mut bi) = (a.abs() as u64, b.abs() as u64);
                while bi != 0 {
                    let t = bi;
                    bi = ai % bi;
                    ai = t;
                }
                Ok(ai as f64)
            }
            "lcm" => {
                let (a, b) = get2()?;
                let (ai, bi) = (a.abs() as u64, b.abs() as u64);
                let mut gcd_ai = ai;
                let mut gcd_bi = bi;
                while gcd_bi != 0 {
                    let t = gcd_bi;
                    gcd_bi = gcd_ai % gcd_bi;
                    gcd_ai = t;
                }
                Ok((ai / gcd_ai * bi) as f64)
            }
            "clamp" => {
                if args.len() < 3 {
                    return Err("clamp(x, min, max) requires 3 arguments".to_string());
                }
                Ok(args[0].clamp(args[1], args[2]))
            }
            "nck" | "choose" | "comb" => {
                let (n, k) = get2()?;
                let (n, k) = (n as u64, k as u64);
                if k > n {
                    return Ok(0.0);
                }
                let k = k.min(n - k);
                let mut result = 1u64;
                for i in 0..k {
                    result = result * (n - i) / (i + 1);
                }
                Ok(result as f64)
            }
            _ => Err(format!("Unknown function: '{}'", name)),
        }
    }
}

fn eval_expr(expr: &str, vars: &std::collections::HashMap<String, f64>) -> Result<f64, String> {
    let tokens = tokenize(expr)?;
    let mut parser = Parser::new(tokens, vars.clone());
    let val = parser.parse_expr()?;
    if parser.pos < parser.tokens.len() {
        return Err(format!(
            "Unexpected token at position {}: {:?}",
            parser.pos,
            parser.tokens.get(parser.pos)
        ));
    }
    Ok(val)
}

fn format_result(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n.is_sign_positive() {
            "∞".to_string()
        } else {
            "-∞".to_string()
        };
    }
    if n == n.trunc() && n.abs() < 1e15 {
        // Integer — show without decimal unless it needs it
        let i = n as i64;
        format!("{}", i)
    } else if n.abs() >= 1e9 || (n.abs() < 1e-3 && n != 0.0) {
        format!("{:.6e}", n)
    } else {
        // Strip trailing zeros
        let s = format!("{:.10}", n);
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_eval(args: &Value) -> Result<String, String> {
    let expr = args
        .get("expr")
        .or_else(|| args.get("expression"))
        .or_else(|| args.get("input"))
        .or_else(|| args.get("text"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "calc_tools: 'expr' field is required".to_string())?;

    // Parse variable assignments from 'vars' object
    let mut vars: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    if let Some(obj) = args.get("vars").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            if let Some(n) = v.as_f64() {
                vars.insert(k.clone(), n);
            }
        }
    }

    let result = eval_expr(expr, &vars)?;
    let formatted = format_result(result);

    let mut out = format!("calc_tools — eval\n\n");
    out.push_str(&format!("  Expression : {}\n", expr));
    if !vars.is_empty() {
        let var_str: Vec<String> = vars.iter().map(|(k, v)| format!("{} = {}", k, v)).collect();
        out.push_str(&format!("  Variables  : {}\n", var_str.join(", ")));
    }
    out.push_str(&format!("  Result     : {}\n", formatted));

    // Show alternate forms if numeric
    if result.is_finite() && result == result.trunc() && result.abs() < 1e15 {
        let i = result as i64;
        if i.abs() < 256 {
            out.push_str(&format!("  Hex        : 0x{:X}\n", i as u64 & 0xFFFFFFFF));
            out.push_str(&format!("  Binary     : 0b{:b}\n", i as u64 & 0xFFFF));
        }
    } else if result.is_finite() && result != 0.0 {
        out.push_str(&format!("  Scientific : {:.6e}\n", result));
    }
    Ok(out)
}

fn action_rpn(args: &Value) -> Result<String, String> {
    let expr = args
        .get("expr")
        .or_else(|| args.get("expression"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "calc_tools: 'expr' field is required for rpn".to_string())?;

    let mut stack: Vec<f64> = Vec::new();
    let mut steps: Vec<String> = Vec::new();

    for token in expr.split_whitespace() {
        match token {
            "+" | "-" | "*" | "/" | "%" | "^" | "**" => {
                if stack.len() < 2 {
                    return Err(format!(
                        "RPN error: operator '{}' needs 2 operands, stack has {}",
                        token,
                        stack.len()
                    ));
                }
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                let result = match token {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => {
                        if b == 0.0 {
                            return Err("RPN: division by zero".to_string());
                        }
                        a / b
                    }
                    "%" => {
                        if b == 0.0 {
                            return Err("RPN: modulo by zero".to_string());
                        }
                        a.rem_euclid(b)
                    }
                    "^" | "**" => a.powf(b),
                    _ => unreachable!(),
                };
                steps.push(format!(
                    "{} {} {} = {}",
                    format_result(a),
                    token,
                    format_result(b),
                    format_result(result)
                ));
                stack.push(result);
            }
            "sqrt" => {
                let a = stack.pop().ok_or("RPN: sqrt needs 1 operand")?;
                let r = a.sqrt();
                steps.push(format!("sqrt({}) = {}", format_result(a), format_result(r)));
                stack.push(r);
            }
            "abs" => {
                let a = stack.pop().ok_or("RPN: abs needs 1 operand")?;
                let r = a.abs();
                steps.push(format!("abs({}) = {}", format_result(a), format_result(r)));
                stack.push(r);
            }
            "neg" => {
                let a = stack.pop().ok_or("RPN: neg needs 1 operand")?;
                steps.push(format!("neg({}) = {}", format_result(a), format_result(-a)));
                stack.push(-a);
            }
            "dup" => {
                let a = stack.last().copied().ok_or("RPN: dup needs 1 operand")?;
                stack.push(a);
            }
            "swap" => {
                if stack.len() < 2 {
                    return Err("RPN: swap needs 2 operands".to_string());
                }
                let n = stack.len();
                stack.swap(n - 1, n - 2);
            }
            "drop" => {
                stack.pop().ok_or("RPN: drop: stack empty")?;
            }
            num => {
                let n: f64 = match num.to_lowercase().as_str() {
                    "pi" | "π" => std::f64::consts::PI,
                    "e" => std::f64::consts::E,
                    "tau" => std::f64::consts::TAU,
                    _ => num
                        .parse()
                        .map_err(|_| format!("RPN: not a number or op: '{}'", num))?,
                };
                stack.push(n);
            }
        }
    }

    if stack.len() != 1 {
        return Err(format!(
            "RPN: expression left {} values on stack (expected 1): [{}]",
            stack.len(),
            stack
                .iter()
                .map(|n| format_result(*n))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let result = stack[0];
    let mut out = format!("calc_tools — rpn\n\n");
    out.push_str(&format!("  Expression : {}\n", expr));
    out.push_str(&format!("  Result     : {}\n", format_result(result)));
    if !steps.is_empty() {
        out.push_str("\n  Steps:\n");
        for (i, step) in steps.iter().enumerate() {
            out.push_str(&format!("    {}. {}\n", i + 1, step));
        }
    }
    Ok(out)
}

fn action_variables(args: &Value) -> Result<String, String> {
    // Evaluate multiple expressions with shared variable context
    // 'statements' is an array of strings like ["x = 5", "y = x * 2", "z = x + y"]
    let statements: Vec<String> =
        if let Some(arr) = args.get("statements").and_then(|v| v.as_array()) {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        } else if let Some(s) = args
            .get("statements")
            .or_else(|| args.get("expr"))
            .and_then(|v| v.as_str())
        {
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        } else {
            return Err("calc_tools: 'statements' array required for variables action".to_string());
        };

    let mut vars: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let mut out = format!("calc_tools — variables\n\n");
    let mut results: Vec<(String, f64)> = Vec::new();

    for stmt in &statements {
        let stmt = stmt.trim();
        if stmt.is_empty() || stmt.starts_with('#') || stmt.starts_with("//") {
            continue;
        }
        // Assignment: "name = expr" or just "expr"
        if let Some((lhs, rhs)) = stmt.split_once('=') {
            let var_name = lhs.trim();
            if var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                let val = eval_expr(rhs.trim(), &vars)
                    .map_err(|e| format!("Error in '{}': {}", stmt, e))?;
                vars.insert(var_name.to_string(), val);
                results.push((format!("{} = {}", var_name, format_result(val)), val));
                continue;
            }
        }
        // Plain expression
        let val = eval_expr(stmt, &vars).map_err(|e| format!("Error in '{}': {}", stmt, e))?;
        results.push((format!("{} → {}", stmt, format_result(val)), val));
    }

    out.push_str(&format!("{:<40}  {}\n", "Statement", "Value"));
    out.push_str(&format!("{}\n", "─".repeat(55)));
    for (label, val) in &results {
        let _ = val;
        out.push_str(&format!("{}\n", label));
    }
    Ok(out)
}

fn action_sequence(args: &Value) -> Result<String, String> {
    // Generate a numeric sequence from an expression involving 'n' or 'i'
    let expr = args
        .get("expr")
        .or_else(|| args.get("expression"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "calc_tools: 'expr' field is required for sequence (use 'n' as the index variable)"
                .to_string()
        })?;

    let start = args.get("start").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let step = args.get("step").and_then(|v| v.as_f64()).unwrap_or(1.0);

    let var_name = if expr.contains('i') && !expr.contains('n') {
        "i"
    } else {
        "n"
    };

    let mut out = format!("calc_tools — sequence\n\n");
    out.push_str(&format!(
        "  expr = {}  ({} = {}, step = {}, count = {})\n\n",
        expr, var_name, start, step, count
    ));
    out.push_str(&format!("  {:<8}  {}\n", var_name, "value"));
    out.push_str(&format!("  {}\n", "─".repeat(30)));

    let mut vars = std::collections::HashMap::new();
    let mut n = start;
    for _ in 0..count {
        vars.insert(var_name.to_string(), n);
        vars.insert("n".to_string(), n);
        vars.insert("i".to_string(), n);
        let val =
            eval_expr(expr, &vars).map_err(|e| format!("Error at {} = {}: {}", var_name, n, e))?;
        out.push_str(&format!(
            "  {:<8}  {}\n",
            format_result(n),
            format_result(val)
        ));
        n += step;
    }
    Ok(out)
}
