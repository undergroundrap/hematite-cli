use serde_json::{json, Value};

pub fn stack_tools_schema() -> Value {
    json!({
        "name": "stack_tools",
        "description": "Stack, queue, and deque simulation with step-by-step operation trace. Actions: stack (LIFO — push/pop/peek), queue (FIFO — enqueue/dequeue/peek), deque (double-ended — push_front/push_back/pop_front/pop_back/peek), evaluate (evaluate an expression using a stack: RPN or infix with operator precedence), balance (check bracket/parenthesis balance using a stack).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["stack", "queue", "deque", "evaluate", "balance"],
                    "description": "Action to perform (default: stack)"
                },
                "operations": {
                    "type": ["array", "string"],
                    "description": "List of operations like ['push 1', 'push 2', 'pop', 'peek'] for stack/queue/deque; or 'push 1, push 2, pop'"
                },
                "expression": {
                    "type": "string",
                    "description": "Expression string for 'evaluate' (RPN like '3 4 + 2 *' or infix like '(3+4)*2') or bracket string for 'balance'"
                },
                "initial": {
                    "type": ["array", "string"],
                    "description": "Initial items in the structure before operations (optional)"
                }
            }
        }
    })
}

// ── Operation parser ──────────────────────────────────────────────────────────

fn parse_ops(v: &Value) -> Result<Vec<String>, String> {
    if let Some(arr) = v.as_array() {
        arr.iter()
            .map(|x| {
                x.as_str()
                    .map(|s| s.trim().to_string())
                    .ok_or_else(|| "non-string op".to_string())
            })
            .collect()
    } else if let Some(s) = v.as_str() {
        Ok(s.split(',')
            .map(|op| op.trim().to_string())
            .filter(|op| !op.is_empty())
            .collect())
    } else {
        Err("pass 'operations' as JSON array or comma-separated string".to_string())
    }
}

fn parse_initial(v: &Value) -> Vec<String> {
    if let Some(arr) = v.as_array() {
        arr.iter()
            .filter_map(|x| {
                x.as_str()
                    .map(|s| s.to_string())
                    .or_else(|| Some(x.to_string()))
            })
            .collect()
    } else if let Some(s) = v.as_str() {
        s.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect()
    } else {
        Vec::new()
    }
}

fn fmt_stack(s: &[String]) -> String {
    if s.is_empty() {
        return "[]".to_string();
    }
    format!(
        "[{}] ← top",
        s.iter().map(|x| x.as_str()).collect::<Vec<_>>().join(", ")
    )
}

fn fmt_queue(q: &[String]) -> String {
    if q.is_empty() {
        return "[]".to_string();
    }
    format!(
        "front → [{}] ← back",
        q.iter().map(|x| x.as_str()).collect::<Vec<_>>().join(", ")
    )
}

fn fmt_deque(d: &[String]) -> String {
    if d.is_empty() {
        return "[]".to_string();
    }
    format!(
        "front ↔ [{}] ↔ back",
        d.iter().map(|x| x.as_str()).collect::<Vec<_>>().join(", ")
    )
}

// ── Stack action ──────────────────────────────────────────────────────────────

fn action_stack(args: &Value) -> Result<String, String> {
    let initial = args.get("initial").map(parse_initial).unwrap_or_default();
    let ops_val = args.get("operations").ok_or("pass 'operations' list")?;
    let ops = parse_ops(ops_val)?;

    let mut stack: Vec<String> = initial.clone();
    let mut lines = Vec::new();
    lines.push(format!("STACK (LIFO)"));
    lines.push(format!("Initial: {}", fmt_stack(&stack)));
    lines.push(String::new());

    for op in &ops {
        let op_lower = op.to_lowercase();
        let op_lower = op_lower.trim();
        if op_lower.starts_with("push") {
            let val = op.trim()[4..].trim().to_string();
            if val.is_empty() {
                lines.push(format!("  push   → ERROR: no value specified"));
                continue;
            }
            stack.push(val.clone());
            lines.push(format!("  push({}) → {}", val, fmt_stack(&stack)));
        } else if op_lower == "pop" {
            match stack.pop() {
                Some(v) => lines.push(format!(
                    "  pop    → removed '{}' → {}",
                    v,
                    fmt_stack(&stack)
                )),
                None => lines.push("  pop    → ERROR: stack is empty".to_string()),
            }
        } else if op_lower == "peek" || op_lower == "top" {
            match stack.last() {
                Some(v) => lines.push(format!(
                    "  peek   → '{}' (not removed) → {}",
                    v,
                    fmt_stack(&stack)
                )),
                None => lines.push("  peek   → ERROR: stack is empty".to_string()),
            }
        } else if op_lower == "size" || op_lower == "len" {
            lines.push(format!("  size   → {} element(s)", stack.len()));
        } else if op_lower == "clear" || op_lower == "empty" {
            stack.clear();
            lines.push(format!("  clear  → {}", fmt_stack(&stack)));
        } else {
            lines.push(format!(
                "  '{}' → UNKNOWN (valid: push <val>, pop, peek, size, clear)",
                op
            ));
        }
    }

    lines.push(String::new());
    lines.push(format!("Final: {}", fmt_stack(&stack)));
    Ok(lines.join("\n"))
}

// ── Queue action ──────────────────────────────────────────────────────────────

fn action_queue(args: &Value) -> Result<String, String> {
    let initial = args.get("initial").map(parse_initial).unwrap_or_default();
    let ops_val = args.get("operations").ok_or("pass 'operations' list")?;
    let ops = parse_ops(ops_val)?;

    let mut queue: Vec<String> = initial.clone();
    let mut lines = Vec::new();
    lines.push("QUEUE (FIFO)".to_string());
    lines.push(format!("Initial: {}", fmt_queue(&queue)));
    lines.push(String::new());

    for op in &ops {
        let op_lower = op.to_lowercase();
        let op_lower = op_lower.trim();
        if op_lower.starts_with("enqueue") || op_lower.starts_with("push") {
            let trim_len = if op_lower.starts_with("enqueue") {
                7
            } else {
                4
            };
            let val = op.trim()[trim_len..].trim().to_string();
            if val.is_empty() {
                lines.push("  enqueue → ERROR: no value specified".to_string());
                continue;
            }
            queue.push(val.clone());
            lines.push(format!("  enqueue({}) → {}", val, fmt_queue(&queue)));
        } else if op_lower == "dequeue" || op_lower == "pop" {
            if queue.is_empty() {
                lines.push("  dequeue → ERROR: queue is empty".to_string());
            } else {
                let v = queue.remove(0);
                lines.push(format!(
                    "  dequeue → removed '{}' → {}",
                    v,
                    fmt_queue(&queue)
                ));
            }
        } else if op_lower == "peek" || op_lower == "front" {
            match queue.first() {
                Some(v) => lines.push(format!(
                    "  peek    → '{}' (not removed) → {}",
                    v,
                    fmt_queue(&queue)
                )),
                None => lines.push("  peek    → ERROR: queue is empty".to_string()),
            }
        } else if op_lower == "size" || op_lower == "len" {
            lines.push(format!("  size    → {} element(s)", queue.len()));
        } else if op_lower == "clear" || op_lower == "empty" {
            queue.clear();
            lines.push(format!("  clear   → {}", fmt_queue(&queue)));
        } else {
            lines.push(format!(
                "  '{}' → UNKNOWN (valid: enqueue <val>, dequeue, peek, size, clear)",
                op
            ));
        }
    }

    lines.push(String::new());
    lines.push(format!("Final: {}", fmt_queue(&queue)));
    Ok(lines.join("\n"))
}

// ── Deque action ──────────────────────────────────────────────────────────────

fn action_deque(args: &Value) -> Result<String, String> {
    let initial = args.get("initial").map(parse_initial).unwrap_or_default();
    let ops_val = args.get("operations").ok_or("pass 'operations' list")?;
    let ops = parse_ops(ops_val)?;

    let mut deque: Vec<String> = initial.clone();
    let mut lines = Vec::new();
    lines.push("DEQUE (double-ended queue)".to_string());
    lines.push(format!("Initial: {}", fmt_deque(&deque)));
    lines.push(String::new());

    for op in &ops {
        let op_lower = op.to_lowercase();
        let op_lower = op_lower.trim();
        if op_lower.starts_with("push_front")
            || op_lower.starts_with("prepend")
            || op_lower.starts_with("push_left")
        {
            let skip = if op_lower.starts_with("push_front") {
                10
            } else if op_lower.starts_with("prepend") {
                7
            } else {
                9
            };
            let val = op.trim()[skip..].trim().to_string();
            deque.insert(0, val.clone());
            lines.push(format!("  push_front({}) → {}", val, fmt_deque(&deque)));
        } else if op_lower.starts_with("push_back")
            || op_lower.starts_with("append")
            || op_lower.starts_with("push_right")
            || op_lower.starts_with("push ")
        {
            let skip = if op_lower.starts_with("push_back") {
                9
            } else if op_lower.starts_with("append") {
                6
            } else if op_lower.starts_with("push_right") {
                10
            } else {
                5
            };
            let val = op.trim()[skip..].trim().to_string();
            deque.push(val.clone());
            lines.push(format!("  push_back({}) → {}", val, fmt_deque(&deque)));
        } else if op_lower == "pop_front" || op_lower == "pop_left" {
            if deque.is_empty() {
                lines.push("  pop_front → ERROR: deque is empty".to_string());
            } else {
                let v = deque.remove(0);
                lines.push(format!(
                    "  pop_front → removed '{}' → {}",
                    v,
                    fmt_deque(&deque)
                ));
            }
        } else if op_lower == "pop_back" || op_lower == "pop" || op_lower == "pop_right" {
            match deque.pop() {
                Some(v) => lines.push(format!(
                    "  pop_back → removed '{}' → {}",
                    v,
                    fmt_deque(&deque)
                )),
                None => lines.push("  pop_back → ERROR: deque is empty".to_string()),
            }
        } else if op_lower == "peek_front" || op_lower == "front" {
            match deque.first() {
                Some(v) => lines.push(format!("  peek_front → '{}' → {}", v, fmt_deque(&deque))),
                None => lines.push("  peek_front → ERROR: deque is empty".to_string()),
            }
        } else if op_lower == "peek_back" || op_lower == "back" {
            match deque.last() {
                Some(v) => lines.push(format!("  peek_back → '{}' → {}", v, fmt_deque(&deque))),
                None => lines.push("  peek_back → ERROR: deque is empty".to_string()),
            }
        } else if op_lower == "size" || op_lower == "len" {
            lines.push(format!("  size → {} element(s)", deque.len()));
        } else if op_lower == "clear" {
            deque.clear();
            lines.push(format!("  clear → {}", fmt_deque(&deque)));
        } else {
            lines.push(format!("  '{}' → UNKNOWN (valid: push_front/push_back/pop_front/pop_back/peek_front/peek_back/size/clear)", op));
        }
    }

    lines.push(String::new());
    lines.push(format!("Final: {}", fmt_deque(&deque)));
    Ok(lines.join("\n"))
}

// ── RPN / infix evaluator ─────────────────────────────────────────────────────

fn action_evaluate(args: &Value) -> Result<String, String> {
    let expr = args
        .get("expression")
        .and_then(|v| v.as_str())
        .ok_or("pass 'expression' to evaluate")?
        .trim();

    // detect RPN: tokens are all numbers or operators, no parentheses on operators
    let tokens: Vec<&str> = expr.split_whitespace().collect();
    let looks_rpn = tokens.iter().all(|t| {
        t.parse::<f64>().is_ok() || matches!(*t, "+" | "-" | "*" | "/" | "%" | "^" | "**")
    });

    if looks_rpn && tokens.len() > 1 {
        eval_rpn(expr)
    } else {
        eval_infix(expr)
    }
}

fn eval_rpn(expr: &str) -> Result<String, String> {
    let tokens: Vec<&str> = expr.split_whitespace().collect();
    let mut stack: Vec<f64> = Vec::new();
    let mut trace = Vec::new();
    trace.push(format!("RPN evaluation of: {}", expr));
    trace.push(String::new());

    for token in &tokens {
        if let Ok(n) = token.parse::<f64>() {
            stack.push(n);
            trace.push(format!("  push {}  → stack: {:?}", n, stack));
        } else {
            if stack.len() < 2 {
                return Err(format!(
                    "RPN error: operator '{}' needs 2 operands but stack has {}",
                    token,
                    stack.len()
                ));
            }
            let b = stack.pop().unwrap();
            let a = stack.pop().unwrap();
            let result = match *token {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "/" => {
                    if b == 0.0 {
                        return Err("division by zero".to_string());
                    }
                    a / b
                }
                "%" => {
                    if b == 0.0 {
                        return Err("modulo by zero".to_string());
                    }
                    a % b
                }
                "^" | "**" => a.powf(b),
                _ => return Err(format!("unknown operator '{}'", token)),
            };
            stack.push(result);
            trace.push(format!(
                "  {} {} {} = {}  → stack: {:?}",
                a, token, b, result, stack
            ));
        }
    }

    if stack.len() != 1 {
        return Err(format!(
            "RPN error: expression left {} items on stack",
            stack.len()
        ));
    }

    let result = stack[0];
    trace.push(String::new());
    let result_str = if result.fract() == 0.0 && result.abs() < 1e12 {
        format!("{}", result as i64)
    } else {
        format!("{}", result)
    };
    trace.push(format!("Result: {}", result_str));
    Ok(trace.join("\n"))
}

fn precedence(op: char) -> u8 {
    match op {
        '+' | '-' => 1,
        '*' | '/' | '%' => 2,
        '^' => 3,
        _ => 0,
    }
}

fn apply_op(op: char, b: f64, a: f64) -> Result<f64, String> {
    match op {
        '+' => Ok(a + b),
        '-' => Ok(a - b),
        '*' => Ok(a * b),
        '/' => {
            if b == 0.0 {
                return Err("division by zero".to_string());
            }
            Ok(a / b)
        }
        '%' => {
            if b == 0.0 {
                return Err("modulo by zero".to_string());
            }
            Ok(a % b)
        }
        '^' => Ok(a.powf(b)),
        _ => Err(format!("unknown operator '{}'", op)),
    }
}

fn eval_infix(expr: &str) -> Result<String, String> {
    // Simple shunting-yard to evaluate infix expressions
    let mut output: Vec<f64> = Vec::new();
    let mut ops: Vec<char> = Vec::new();
    let mut trace = Vec::new();
    trace.push(format!("Infix evaluation of: {}", expr));
    trace.push(String::new());

    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit()
            || (c == '-'
                && (i == 0 || matches!(chars[i - 1], '(' | '+' | '-' | '*' | '/' | '%' | '^')))
        {
            let start = i;
            if c == '-' {
                i += 1;
            }
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let num_str: String = chars[start..i].iter().collect();
            let n: f64 = num_str
                .parse()
                .map_err(|_| format!("invalid number '{}'", num_str))?;
            output.push(n);
            trace.push(format!("  number {} → operand stack: {:?}", n, output));
            continue;
        }
        if c == '(' {
            ops.push(c);
        } else if c == ')' {
            while ops.last() != Some(&'(') {
                let op = ops.pop().ok_or("mismatched parentheses")?;
                let b = output.pop().ok_or("missing operand")?;
                let a = output.pop().ok_or("missing operand")?;
                let r = apply_op(op, b, a)?;
                output.push(r);
                trace.push(format!(
                    "  apply '{}' → {} {} {} = {}  operands: {:?}",
                    op, a, op, b, r, output
                ));
            }
            ops.pop(); // remove '('
        } else if matches!(c, '+' | '-' | '*' | '/' | '%' | '^') {
            while let Some(&top) = ops.last() {
                if top != '(' && precedence(top) >= precedence(c) {
                    ops.pop();
                    let b = output.pop().ok_or("missing operand")?;
                    let a = output.pop().ok_or("missing operand")?;
                    let r = apply_op(top, b, a)?;
                    output.push(r);
                    trace.push(format!(
                        "  apply '{}' → {} {} {} = {}  operands: {:?}",
                        top, a, top, b, r, output
                    ));
                } else {
                    break;
                }
            }
            ops.push(c);
        } else {
            return Err(format!("unexpected character '{}'", c));
        }
        i += 1;
    }

    while let Some(op) = ops.pop() {
        if op == '(' {
            return Err("mismatched parentheses".to_string());
        }
        let b = output.pop().ok_or("missing operand")?;
        let a = output.pop().ok_or("missing operand")?;
        let r = apply_op(op, b, a)?;
        output.push(r);
        trace.push(format!(
            "  apply '{}' → {} {} {} = {}  operands: {:?}",
            op, a, op, b, r, output
        ));
    }

    if output.len() != 1 {
        return Err("malformed expression".to_string());
    }

    let result = output[0];
    trace.push(String::new());
    let result_str = if result.fract() == 0.0 && result.abs() < 1e12 {
        format!("{}", result as i64)
    } else {
        format!("{}", result)
    };
    trace.push(format!("Result: {}", result_str));
    Ok(trace.join("\n"))
}

// ── Balance check ─────────────────────────────────────────────────────────────

fn action_balance(args: &Value) -> Result<String, String> {
    let expr = args
        .get("expression")
        .and_then(|v| v.as_str())
        .ok_or("pass 'expression' to check bracket balance")?;

    let open_brackets = ['(', '[', '{'];
    let close_brackets = [')', ']', '}'];
    let mut stack: Vec<(char, usize)> = Vec::new();
    let mut trace = Vec::new();
    trace.push(format!("Balance check: {}", expr));
    trace.push(String::new());

    for (pos, ch) in expr.chars().enumerate() {
        if open_brackets.contains(&ch) {
            stack.push((ch, pos));
            trace.push(format!(
                "  pos {}: '{}' — push  → stack: {:?}",
                pos,
                ch,
                stack.iter().map(|(c, _)| c).collect::<Vec<_>>()
            ));
        } else if let Some(idx) = close_brackets.iter().position(|&c| c == ch) {
            let expected_open = open_brackets[idx];
            match stack.pop() {
                Some((top, open_pos)) if top == expected_open => {
                    trace.push(format!(
                        "  pos {}: '{}' — match with '{}' at pos {}  → stack: {:?}",
                        pos,
                        ch,
                        top,
                        open_pos,
                        stack.iter().map(|(c, _)| c).collect::<Vec<_>>()
                    ));
                }
                Some((top, open_pos)) => {
                    trace.push(format!(
                        "  pos {}: '{}' — MISMATCH with '{}' at pos {}",
                        pos, ch, top, open_pos
                    ));
                    return Ok(trace.join("\n")
                        + &format!(
                            "\n\nResult: UNBALANCED — '{}' at pos {} does not match '{}' at pos {}",
                            ch, pos, top, open_pos
                        ));
                }
                None => {
                    trace.push(format!(
                        "  pos {}: '{}' — NO MATCHING OPEN bracket",
                        pos, ch
                    ));
                    return Ok(trace.join("\n")
                        + &format!(
                            "\n\nResult: UNBALANCED — extra '{}' at pos {} with no matching open",
                            ch, pos
                        ));
                }
            }
        }
    }

    if stack.is_empty() {
        trace.push(String::new());
        trace.push("Result: BALANCED ✓".to_string());
    } else {
        let unmatched: Vec<String> = stack
            .iter()
            .map(|(c, pos)| format!("'{}' at pos {}", c, pos))
            .collect();
        trace.push(String::new());
        trace.push(format!(
            "Result: UNBALANCED — unclosed: {}",
            unmatched.join(", ")
        ));
    }
    Ok(trace.join("\n"))
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("stack");
    match action {
        "stack" => action_stack(args),
        "queue" => action_queue(args),
        "deque" => action_deque(args),
        "evaluate" => action_evaluate(args),
        "balance" => action_balance(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: stack, queue, deque, evaluate, balance",
            action
        )),
    }
}
