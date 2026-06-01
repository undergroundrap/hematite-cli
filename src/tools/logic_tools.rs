use serde_json::{json, Value};

pub fn logic_tools_schema() -> Value {
    json!({
        "name": "logic_tools",
        "description": "Propositional logic operations without external utilities. Actions: truth_table (generate full truth table for an expression), evaluate (evaluate expression with given variable assignments), sat (check if satisfiable — find assignments that make it true), tautology (check if always true), contradition (check if always false), simplify (apply basic boolean identities to reduce expression), cnf (convert to Conjunctive Normal Form), dnf (convert to Disjunctive Normal Form). Operators: and/&&, or/||, not/!, xor/^, implies/->, iff/<->. Variables: any identifier A–Z or a–z.",
        "parameters": {
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "Propositional logic expression (e.g. 'A and (B or not C)', 'P -> Q', 'A xor B')"
                },
                "action": {
                    "type": "string",
                    "enum": ["truth_table", "evaluate", "sat", "tautology", "contradiction", "simplify", "cnf", "dnf"],
                    "description": "Action to perform (default: truth_table)"
                },
                "variables": {
                    "type": "object",
                    "description": "Variable assignments for 'evaluate': {\"A\": true, \"B\": false}"
                }
            },
            "required": ["expression"]
        }
    })
}

// ── Tokenizer ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Var(String),
    And,
    Or,
    Not,
    Xor,
    Implies,
    Iff,
    LParen,
    RParen,
    True,
    False,
}

fn tokenize(expr: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\n' => {
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
            '!' => {
                tokens.push(Token::Not);
                i += 1;
            }
            '^' => {
                tokens.push(Token::Xor);
                i += 1;
            }
            '&' => {
                if i + 1 < chars.len() && chars[i + 1] == '&' {
                    i += 1;
                }
                tokens.push(Token::And);
                i += 1;
            }
            '|' => {
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    i += 1;
                }
                tokens.push(Token::Or);
                i += 1;
            }
            '-' => {
                if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::Implies);
                    i += 2;
                } else {
                    return Err(format!("Unexpected '-' at position {}", i));
                }
            }
            '<' => {
                if i + 2 < chars.len() && chars[i + 1] == '-' && chars[i + 2] == '>' {
                    tokens.push(Token::Iff);
                    i += 3;
                } else {
                    return Err(format!("Unexpected '<' at position {}", i));
                }
            }
            _ if c.is_alphabetic() => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                match word.to_lowercase().as_str() {
                    "and" => tokens.push(Token::And),
                    "or" => tokens.push(Token::Or),
                    "not" => tokens.push(Token::Not),
                    "xor" => tokens.push(Token::Xor),
                    "implies" => tokens.push(Token::Implies),
                    "iff" | "xnor" => tokens.push(Token::Iff),
                    "true" | "t" | "1" => tokens.push(Token::True),
                    "false" | "f" | "0" => tokens.push(Token::False),
                    _ => tokens.push(Token::Var(word)),
                }
            }
            '1' => {
                tokens.push(Token::True);
                i += 1;
            }
            '0' => {
                tokens.push(Token::False);
                i += 1;
            }
            _ => return Err(format!("Unexpected character '{}'", c)),
        }
    }
    Ok(tokens)
}

// ── Parser ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Expr {
    Var(String),
    Lit(bool),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Xor(Box<Expr>, Box<Expr>),
    Implies(Box<Expr>, Box<Expr>),
    Iff(Box<Expr>, Box<Expr>),
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn consume(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t
    }

    // iff: lowest precedence
    fn parse_iff(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_implies()?;
        while self.peek() == Some(&Token::Iff) {
            self.consume();
            let right = self.parse_implies()?;
            left = Expr::Iff(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_implies(&mut self) -> Result<Expr, String> {
        let left = self.parse_or()?;
        if self.peek() == Some(&Token::Implies) {
            self.consume();
            let right = self.parse_implies()?; // right-associative
            Ok(Expr::Implies(Box::new(left), Box::new(right)))
        } else {
            Ok(left)
        }
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_xor()?;
        while self.peek() == Some(&Token::Or) {
            self.consume();
            let right = self.parse_xor()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_xor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while self.peek() == Some(&Token::Xor) {
            self.consume();
            let right = self.parse_and()?;
            left = Expr::Xor(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_not()?;
        while self.peek() == Some(&Token::And) {
            self.consume();
            let right = self.parse_not()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, String> {
        if self.peek() == Some(&Token::Not) {
            self.consume();
            let inner = self.parse_not()?;
            Ok(Expr::Not(Box::new(inner)))
        } else {
            self.parse_atom()
        }
    }

    fn parse_atom(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Token::LParen) => {
                self.consume();
                let inner = self.parse_iff()?;
                if self.peek() != Some(&Token::RParen) {
                    return Err("Expected closing ')'".to_string());
                }
                self.consume();
                Ok(inner)
            }
            Some(Token::Var(name)) => {
                let name = name.clone();
                self.consume();
                Ok(Expr::Var(name))
            }
            Some(Token::True) => {
                self.consume();
                Ok(Expr::Lit(true))
            }
            Some(Token::False) => {
                self.consume();
                Ok(Expr::Lit(false))
            }
            Some(t) => Err(format!("Unexpected token {:?}", t)),
            None => Err("Unexpected end of expression".to_string()),
        }
    }
}

fn parse_expr(expr: &str) -> Result<Expr, String> {
    let tokens = tokenize(expr)?;
    let mut parser = Parser::new(tokens);
    let e = parser.parse_iff()?;
    if parser.pos < parser.tokens.len() {
        return Err(format!("Unexpected token at position {}", parser.pos));
    }
    Ok(e)
}

// ── Evaluation ────────────────────────────────────────────────────────────────

fn collect_vars(expr: &Expr, vars: &mut Vec<String>) {
    match expr {
        Expr::Var(n) => {
            if !vars.contains(n) {
                vars.push(n.clone());
            }
        }
        Expr::Lit(_) => {}
        Expr::Not(e) => collect_vars(e, vars),
        Expr::And(a, b)
        | Expr::Or(a, b)
        | Expr::Xor(a, b)
        | Expr::Implies(a, b)
        | Expr::Iff(a, b) => {
            collect_vars(a, vars);
            collect_vars(b, vars);
        }
    }
}

fn eval(expr: &Expr, assignment: &std::collections::HashMap<String, bool>) -> bool {
    match expr {
        Expr::Var(n) => *assignment.get(n).unwrap_or(&false),
        Expr::Lit(b) => *b,
        Expr::Not(e) => !eval(e, assignment),
        Expr::And(a, b) => eval(a, assignment) && eval(b, assignment),
        Expr::Or(a, b) => eval(a, assignment) || eval(b, assignment),
        Expr::Xor(a, b) => eval(a, assignment) ^ eval(b, assignment),
        Expr::Implies(a, b) => !eval(a, assignment) || eval(b, assignment),
        Expr::Iff(a, b) => eval(a, assignment) == eval(b, assignment),
    }
}

fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Var(n) => n.clone(),
        Expr::Lit(b) => b.to_string(),
        Expr::Not(e) => format!("¬{}", expr_to_string(e)),
        Expr::And(a, b) => format!("({} ∧ {})", expr_to_string(a), expr_to_string(b)),
        Expr::Or(a, b) => format!("({} ∨ {})", expr_to_string(a), expr_to_string(b)),
        Expr::Xor(a, b) => format!("({} ⊕ {})", expr_to_string(a), expr_to_string(b)),
        Expr::Implies(a, b) => format!("({} → {})", expr_to_string(a), expr_to_string(b)),
        Expr::Iff(a, b) => format!("({} ↔ {})", expr_to_string(a), expr_to_string(b)),
    }
}

fn all_assignments(vars: &[String]) -> Vec<std::collections::HashMap<String, bool>> {
    let n = vars.len();
    let count = 1usize << n;
    (0..count)
        .map(|i| {
            vars.iter()
                .enumerate()
                .map(|(j, v)| (v.clone(), (i >> (n - 1 - j)) & 1 == 1))
                .collect()
        })
        .collect()
}

// ── Normal forms ──────────────────────────────────────────────────────────────

fn to_nnf(expr: &Expr, negated: bool) -> Expr {
    match (expr, negated) {
        (Expr::Lit(b), false) => Expr::Lit(*b),
        (Expr::Lit(b), true) => Expr::Lit(!b),
        (Expr::Var(n), false) => Expr::Var(n.clone()),
        (Expr::Var(n), true) => Expr::Not(Box::new(Expr::Var(n.clone()))),
        (Expr::Not(e), _) => to_nnf(e, !negated),
        (Expr::And(a, b), false) => {
            Expr::And(Box::new(to_nnf(a, false)), Box::new(to_nnf(b, false)))
        }
        (Expr::And(a, b), true) => Expr::Or(Box::new(to_nnf(a, true)), Box::new(to_nnf(b, true))),
        (Expr::Or(a, b), false) => Expr::Or(Box::new(to_nnf(a, false)), Box::new(to_nnf(b, false))),
        (Expr::Or(a, b), true) => Expr::And(Box::new(to_nnf(a, true)), Box::new(to_nnf(b, true))),
        (Expr::Implies(a, b), false) => {
            Expr::Or(Box::new(to_nnf(a, true)), Box::new(to_nnf(b, false)))
        }
        (Expr::Implies(a, b), true) => {
            Expr::And(Box::new(to_nnf(a, false)), Box::new(to_nnf(b, true)))
        }
        (Expr::Iff(a, b), false) => {
            let ab = Expr::And(Box::new(to_nnf(a, false)), Box::new(to_nnf(b, false)));
            let ba = Expr::And(Box::new(to_nnf(a, true)), Box::new(to_nnf(b, true)));
            Expr::Or(Box::new(ab), Box::new(ba))
        }
        (Expr::Iff(a, b), true) => {
            let ab = Expr::And(Box::new(to_nnf(a, false)), Box::new(to_nnf(b, true)));
            let ba = Expr::And(Box::new(to_nnf(a, true)), Box::new(to_nnf(b, false)));
            Expr::Or(Box::new(ab), Box::new(ba))
        }
        (Expr::Xor(a, b), n) => {
            // a XOR b = (a OR b) AND NOT(a AND b) = (a AND NOT b) OR (NOT a AND b)
            let iff = Expr::Iff(a.clone(), b.clone());
            to_nnf(&Expr::Not(Box::new(iff)), !n)
        }
    }
}

// Distribute OR over AND to get CNF
fn distribute_or_over_and(a: &Expr, b: &Expr) -> Expr {
    match a {
        Expr::And(a1, a2) => Expr::And(
            Box::new(distribute_or_over_and(a1, b)),
            Box::new(distribute_or_over_and(a2, b)),
        ),
        _ => match b {
            Expr::And(b1, b2) => Expr::And(
                Box::new(distribute_or_over_and(a, b1)),
                Box::new(distribute_or_over_and(a, b2)),
            ),
            _ => Expr::Or(Box::new(a.clone()), Box::new(b.clone())),
        },
    }
}

fn to_cnf_from_nnf(expr: &Expr) -> Expr {
    match expr {
        Expr::And(a, b) => Expr::And(Box::new(to_cnf_from_nnf(a)), Box::new(to_cnf_from_nnf(b))),
        Expr::Or(a, b) => {
            let ca = to_cnf_from_nnf(a);
            let cb = to_cnf_from_nnf(b);
            distribute_or_over_and(&ca, &cb)
        }
        _ => expr.clone(),
    }
}

// Distribute AND over OR to get DNF
fn distribute_and_over_or(a: &Expr, b: &Expr) -> Expr {
    match a {
        Expr::Or(a1, a2) => Expr::Or(
            Box::new(distribute_and_over_or(a1, b)),
            Box::new(distribute_and_over_or(a2, b)),
        ),
        _ => match b {
            Expr::Or(b1, b2) => Expr::Or(
                Box::new(distribute_and_over_or(a, b1)),
                Box::new(distribute_and_over_or(a, b2)),
            ),
            _ => Expr::And(Box::new(a.clone()), Box::new(b.clone())),
        },
    }
}

fn to_dnf_from_nnf(expr: &Expr) -> Expr {
    match expr {
        Expr::Or(a, b) => Expr::Or(Box::new(to_dnf_from_nnf(a)), Box::new(to_dnf_from_nnf(b))),
        Expr::And(a, b) => {
            let da = to_dnf_from_nnf(a);
            let db = to_dnf_from_nnf(b);
            distribute_and_over_or(&da, &db)
        }
        _ => expr.clone(),
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_truth_table(expr_str: &str) -> Result<String, String> {
    let expr = parse_expr(expr_str)?;
    let mut vars = Vec::new();
    collect_vars(&expr, &mut vars);
    vars.sort();

    if vars.len() > 8 {
        return Err(format!(
            "Too many variables ({}) — truth table would have {} rows. Max 8 variables.",
            vars.len(),
            1 << vars.len()
        ));
    }

    let canonical = expr_to_string(&expr);
    let assignments = all_assignments(&vars);

    let header_vars: Vec<String> = vars.iter().map(|v| format!("{:>5}", v)).collect();
    let col_result = "Result";

    let mut out = format!("Expression: {}\n\n", canonical);
    out += &format!("{}  {}\n", header_vars.join("  "), col_result);
    out += &format!(
        "{}  {}\n",
        vars.iter().map(|_| "-----").collect::<Vec<_>>().join("  "),
        "------"
    );

    let mut true_count = 0usize;
    let total = assignments.len();
    for assignment in &assignments {
        let result = eval(&expr, assignment);
        if result {
            true_count += 1;
        }
        let var_vals: Vec<String> = vars
            .iter()
            .map(|v| {
                let val = assignment[v];
                format!("{:>5}", if val { "T" } else { "F" })
            })
            .collect();
        out += &format!(
            "{}  {}\n",
            var_vals.join("  "),
            if result { "T" } else { "F" }
        );
    }

    out += &format!(
        "\nTrue for {} / {} assignments ({:.0}%)\n",
        true_count,
        total,
        100.0 * true_count as f64 / total as f64
    );

    if true_count == 0 {
        out += "Classification: CONTRADICTION (always false)\n";
    } else if true_count == total {
        out += "Classification: TAUTOLOGY (always true)\n";
    } else {
        out += "Classification: CONTINGENCY\n";
    }

    Ok(out)
}

fn action_evaluate(expr_str: &str, vars_val: &Value) -> Result<String, String> {
    let expr = parse_expr(expr_str)?;
    let mut assignment = std::collections::HashMap::new();
    if let Some(obj) = vars_val.as_object() {
        for (k, v) in obj {
            let val = match v {
                Value::Bool(b) => *b,
                Value::String(s) => match s.to_lowercase().as_str() {
                    "true" | "t" | "1" | "yes" => true,
                    _ => false,
                },
                Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
                _ => false,
            };
            assignment.insert(k.clone(), val);
        }
    }

    let mut missing = Vec::new();
    let mut all_vars = Vec::new();
    collect_vars(&expr, &mut all_vars);
    for v in &all_vars {
        if !assignment.contains_key(v) {
            missing.push(v.clone());
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "Missing variable assignments: {}",
            missing.join(", ")
        ));
    }

    let result = eval(&expr, &assignment);
    let canonical = expr_to_string(&expr);

    let mut out = format!("Expression: {}\n\n", canonical);
    out += "Assignments:\n";
    let mut sorted_vars: Vec<_> = assignment.iter().collect();
    sorted_vars.sort_by_key(|(k, _)| k.clone());
    for (k, v) in &sorted_vars {
        out += &format!("  {} = {}\n", k, v);
    }
    out += &format!("\nResult: {}\n", if result { "TRUE" } else { "FALSE" });
    Ok(out)
}

fn action_sat(expr_str: &str) -> Result<String, String> {
    let expr = parse_expr(expr_str)?;
    let mut vars = Vec::new();
    collect_vars(&expr, &mut vars);
    vars.sort();

    if vars.len() > 20 {
        return Err(format!(
            "Too many variables ({}) for SAT check. Max 20.",
            vars.len()
        ));
    }

    let canonical = expr_to_string(&expr);
    let mut out = format!("Expression: {}\n\n", canonical);

    let assignments = all_assignments(&vars);
    let mut satisfying = Vec::new();
    for assignment in &assignments {
        if eval(&expr, assignment) {
            satisfying.push(assignment);
            if satisfying.len() >= 3 {
                break;
            } // just show a few examples
        }
    }

    if satisfying.is_empty() {
        out += "UNSATISFIABLE — no assignment makes this expression true.\n";
    } else {
        out += "SATISFIABLE\n\nSatisfying assignment(s):\n";
        for a in &satisfying {
            let mut sorted: Vec<_> = a.iter().collect();
            sorted.sort_by_key(|(k, _)| k.clone());
            let parts: Vec<String> = sorted
                .iter()
                .map(|(k, v)| format!("{}={}", k, if **v { "T" } else { "F" }))
                .collect();
            out += &format!("  {{{}}}\n", parts.join(", "));
        }
        // Count total
        let total: usize = assignments.iter().filter(|a| eval(&expr, a)).count();
        out += &format!(
            "\n{} satisfying assignment(s) out of {}\n",
            total,
            assignments.len()
        );
    }
    Ok(out)
}

fn action_tautology(expr_str: &str) -> Result<String, String> {
    let expr = parse_expr(expr_str)?;
    let mut vars = Vec::new();
    collect_vars(&expr, &mut vars);
    vars.sort();

    let canonical = expr_to_string(&expr);
    let assignments = all_assignments(&vars);
    let mut counterexample = None;
    for a in &assignments {
        if !eval(&expr, a) {
            counterexample = Some(a.clone());
            break;
        }
    }

    let mut out = format!("Expression: {}\n\n", canonical);
    if counterexample.is_none() {
        out += "TAUTOLOGY — true for all variable assignments.\n";
    } else {
        out += "NOT A TAUTOLOGY\n\nCounterexample:\n";
        let a = counterexample.unwrap();
        let mut sorted: Vec<_> = a.iter().collect();
        sorted.sort_by_key(|(k, _)| k.clone());
        for (k, v) in &sorted {
            out += &format!("  {} = {}\n", k, if **v { "T" } else { "F" });
        }
        out += "  Result: F\n";
    }
    Ok(out)
}

fn action_contradiction(expr_str: &str) -> Result<String, String> {
    let expr = parse_expr(expr_str)?;
    let mut vars = Vec::new();
    collect_vars(&expr, &mut vars);
    vars.sort();

    let canonical = expr_to_string(&expr);
    let assignments = all_assignments(&vars);
    let mut witness = None;
    for a in &assignments {
        if eval(&expr, a) {
            witness = Some(a.clone());
            break;
        }
    }

    let mut out = format!("Expression: {}\n\n", canonical);
    if witness.is_none() {
        out += "CONTRADICTION — false for all variable assignments.\n";
    } else {
        out += "NOT A CONTRADICTION\n\nAssignment that makes it true:\n";
        let a = witness.unwrap();
        let mut sorted: Vec<_> = a.iter().collect();
        sorted.sort_by_key(|(k, _)| k.clone());
        for (k, v) in &sorted {
            out += &format!("  {} = {}\n", k, if **v { "T" } else { "F" });
        }
        out += "  Result: T\n";
    }
    Ok(out)
}

fn action_simplify(expr_str: &str) -> Result<String, String> {
    let expr = parse_expr(expr_str)?;
    // Simple approach: use truth table to reconstruct minimal DNF
    let mut vars = Vec::new();
    collect_vars(&expr, &mut vars);
    vars.sort();
    let canonical = expr_to_string(&expr);

    if vars.len() > 8 {
        return Err("Too many variables for simplification (max 8)".to_string());
    }

    let assignments = all_assignments(&vars);
    let true_rows: Vec<_> = assignments.iter().filter(|a| eval(&expr, a)).collect();

    let mut out = format!("Expression: {}\n", canonical);
    out += &format!("NNF:        {}\n\n", expr_to_string(&to_nnf(&expr, false)));

    if true_rows.is_empty() {
        out += "Simplified: FALSE\n";
    } else if true_rows.len() == assignments.len() {
        out += "Simplified: TRUE\n";
    } else {
        out += &format!(
            "True for {}/{} assignments.\n",
            true_rows.len(),
            assignments.len()
        );
        out += "Minterm expansion (DNF from truth table):\n";
        let minterms: Vec<String> = true_rows
            .iter()
            .map(|a| {
                let mut sorted: Vec<_> = a.iter().collect();
                sorted.sort_by_key(|(k, _)| k.clone());
                let terms: Vec<String> = sorted
                    .iter()
                    .map(|(k, &v)| if v { k.to_string() } else { format!("¬{}", k) })
                    .collect();
                format!("({})", terms.join(" ∧ "))
            })
            .collect();
        out += &format!("{}\n", minterms.join("\n∨ "));
    }
    Ok(out)
}

fn action_cnf(expr_str: &str) -> Result<String, String> {
    let expr = parse_expr(expr_str)?;
    let canonical = expr_to_string(&expr);
    let nnf = to_nnf(&expr, false);
    let cnf = to_cnf_from_nnf(&nnf);
    let cnf_str = expr_to_string(&cnf);

    let mut out = format!("Original: {}\n", canonical);
    out += &format!("NNF:      {}\n", expr_to_string(&nnf));
    out += &format!("CNF:      {}\n", cnf_str);
    out += "\nCNF form has all ANDs at the top level and ORs within each clause.\n";
    Ok(out)
}

fn action_dnf(expr_str: &str) -> Result<String, String> {
    let expr = parse_expr(expr_str)?;
    let canonical = expr_to_string(&expr);
    let nnf = to_nnf(&expr, false);
    let dnf = to_dnf_from_nnf(&nnf);
    let dnf_str = expr_to_string(&dnf);

    let mut out = format!("Original: {}\n", canonical);
    out += &format!("NNF:      {}\n", expr_to_string(&nnf));
    out += &format!("DNF:      {}\n", dnf_str);
    out += "\nDNF form has all ORs at the top level and ANDs within each clause.\n";
    Ok(out)
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let expr_str = args
        .get("expression")
        .and_then(|v| v.as_str())
        .ok_or("'expression' is required")?;

    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("truth_table");

    match action {
        "truth_table" => action_truth_table(expr_str),
        "evaluate" => {
            let vars = args.get("variables").cloned().unwrap_or(Value::Object(Default::default()));
            action_evaluate(expr_str, &vars)
        }
        "sat" => action_sat(expr_str),
        "tautology" => action_tautology(expr_str),
        "contradiction" => action_contradiction(expr_str),
        "simplify" => action_simplify(expr_str),
        "cnf" => action_cnf(expr_str),
        "dnf" => action_dnf(expr_str),
        _ => Err(format!(
            "Unknown action '{}'. Valid: truth_table, evaluate, sat, tautology, contradiction, simplify, cnf, dnf",
            action
        )),
    }
}
