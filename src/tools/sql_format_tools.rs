use serde_json::Value;
use std::fmt::Write as FmtWrite;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("format");
    let sql = get_sql(args)?;
    match action {
        "format" | "" => action_format(&sql, args),
        "minify" => action_minify(&sql),
        "extract" => action_extract(&sql, args),
        "split" => action_split(&sql),
        _ => Err(format!(
            "Unknown action '{}'. Available: format, minify, extract, split",
            action
        )),
    }
}

fn get_sql(args: &Value) -> Result<String, String> {
    if let Some(s) = args
        .get("sql")
        .or_else(|| args.get("query"))
        .or_else(|| args.get("text"))
    {
        if let Some(text) = s.as_str() {
            return Ok(text.to_string());
        }
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path).map_err(|e| format!("Cannot read '{}': {e}", path));
    }
    Err("Pass 'sql' with the SQL text, or 'file' with a path to a .sql file.".into())
}

// ── Tokenizer ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum TokKind {
    Keyword,
    Ident,
    Number,
    Str,
    Op,
    Comma,
    Lparen,
    Rparen,
    Semi,
    LineComment,
    BlockComment,
    Whitespace,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokKind,
    text: String,
}

fn tokenize(sql: &str) -> Vec<Token> {
    let chars: Vec<char> = sql.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < n {
        // Line comment
        if i + 1 < n && chars[i] == '-' && chars[i + 1] == '-' {
            let start = i;
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            tokens.push(Token {
                kind: TokKind::LineComment,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }
        // Block comment
        if i + 1 < n && chars[i] == '/' && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2; // consume */
            tokens.push(Token {
                kind: TokKind::BlockComment,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }
        // String literals (single-quoted and double-quoted)
        if chars[i] == '\'' || chars[i] == '"' {
            let q = chars[i];
            let start = i;
            i += 1;
            while i < n {
                if chars[i] == q {
                    i += 1;
                    if i < n && chars[i] == q {
                        i += 1; // escaped quote
                    } else {
                        break;
                    }
                } else if chars[i] == '\\' {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            tokens.push(Token {
                kind: TokKind::Str,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }
        // Backtick identifiers
        if chars[i] == '`' {
            let start = i;
            i += 1;
            while i < n && chars[i] != '`' {
                i += 1;
            }
            i += 1;
            tokens.push(Token {
                kind: TokKind::Ident,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }
        // Bracket identifiers [name]
        if chars[i] == '[' {
            let start = i;
            i += 1;
            while i < n && chars[i] != ']' {
                i += 1;
            }
            i += 1;
            tokens.push(Token {
                kind: TokKind::Ident,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }
        // Whitespace
        if chars[i].is_whitespace() {
            let start = i;
            while i < n && chars[i].is_whitespace() {
                i += 1;
            }
            tokens.push(Token {
                kind: TokKind::Whitespace,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }
        // Numbers
        if chars[i].is_ascii_digit()
            || (chars[i] == '.' && i + 1 < n && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            while i < n
                && (chars[i].is_ascii_digit()
                    || chars[i] == '.'
                    || chars[i] == 'e'
                    || chars[i] == 'E'
                    || chars[i] == '-'
                    || chars[i] == '+')
            {
                i += 1;
            }
            tokens.push(Token {
                kind: TokKind::Number,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }
        // Identifiers / Keywords
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let kind = if is_keyword(&text.to_uppercase()) {
                TokKind::Keyword
            } else {
                TokKind::Ident
            };
            tokens.push(Token { kind, text });
            continue;
        }
        // Special single chars
        let tok = match chars[i] {
            ',' => Token {
                kind: TokKind::Comma,
                text: ",".into(),
            },
            '(' => Token {
                kind: TokKind::Lparen,
                text: "(".into(),
            },
            ')' => Token {
                kind: TokKind::Rparen,
                text: ")".into(),
            },
            ';' => Token {
                kind: TokKind::Semi,
                text: ";".into(),
            },
            _ => Token {
                kind: TokKind::Op,
                text: chars[i].to_string(),
            },
        };
        tokens.push(tok);
        i += 1;
    }
    tokens
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "SELECT"
            | "FROM"
            | "WHERE"
            | "AND"
            | "OR"
            | "NOT"
            | "IN"
            | "EXISTS"
            | "BETWEEN"
            | "LIKE"
            | "IS"
            | "NULL"
            | "TRUE"
            | "FALSE"
            | "INSERT"
            | "INTO"
            | "VALUES"
            | "UPDATE"
            | "SET"
            | "DELETE"
            | "TRUNCATE"
            | "CREATE"
            | "ALTER"
            | "DROP"
            | "TABLE"
            | "VIEW"
            | "INDEX"
            | "SCHEMA"
            | "DATABASE"
            | "JOIN"
            | "INNER"
            | "OUTER"
            | "LEFT"
            | "RIGHT"
            | "FULL"
            | "CROSS"
            | "ON"
            | "USING"
            | "GROUP"
            | "BY"
            | "HAVING"
            | "ORDER"
            | "LIMIT"
            | "OFFSET"
            | "DISTINCT"
            | "ALL"
            | "UNION"
            | "INTERSECT"
            | "EXCEPT"
            | "AS"
            | "WITH"
            | "RECURSIVE"
            | "CASE"
            | "WHEN"
            | "THEN"
            | "ELSE"
            | "END"
            | "IF"
            | "OVER"
            | "PARTITION"
            | "ROWS"
            | "RANGE"
            | "UNBOUNDED"
            | "PRECEDING"
            | "FOLLOWING"
            | "CURRENT"
            | "ROW"
            | "ASC"
            | "DESC"
            | "NULLS"
            | "FIRST"
            | "LAST"
            | "PRIMARY"
            | "KEY"
            | "FOREIGN"
            | "REFERENCES"
            | "UNIQUE"
            | "CHECK"
            | "DEFAULT"
            | "CONSTRAINT"
            | "AUTO_INCREMENT"
            | "AUTOINCREMENT"
            | "IDENTITY"
            | "SERIAL"
            | "SEQUENCE"
            | "BEGIN"
            | "COMMIT"
            | "ROLLBACK"
            | "TRANSACTION"
            | "SAVEPOINT"
            | "RELEASE"
            | "GRANT"
            | "REVOKE"
            | "PRIVILEGES"
            | "ROLE"
            | "EXPLAIN"
            | "ANALYZE"
            | "VERBOSE"
            | "RETURNING"
            | "CONFLICT"
            | "REPLACE"
            | "TRIGGER"
            | "PROCEDURE"
            | "FUNCTION"
            | "RETURNS"
            | "DECLARE"
            | "LANGUAGE"
            | "INT"
            | "INTEGER"
            | "BIGINT"
            | "SMALLINT"
            | "TINYINT"
            | "FLOAT"
            | "DOUBLE"
            | "DECIMAL"
            | "NUMERIC"
            | "CHAR"
            | "VARCHAR"
            | "TEXT"
            | "BLOB"
            | "CLOB"
            | "BYTEA"
            | "BOOLEAN"
            | "BOOL"
            | "DATE"
            | "TIME"
            | "TIMESTAMP"
            | "DATETIME"
            | "JSON"
            | "JSONB"
            | "COALESCE"
            | "NULLIF"
            | "GREATEST"
            | "LEAST"
            | "COUNT"
            | "SUM"
            | "AVG"
            | "MIN"
            | "MAX"
            | "CAST"
            | "CONVERT"
            | "EXTRACT"
            | "DATE_TRUNC"
            | "INTERVAL"
            | "NOW"
            | "CURRENT_TIMESTAMP"
            | "CURRENT_DATE"
    )
}

// ── Major keywords that start a new clause on their own line ────────────────

fn is_clause_start(s: &str) -> bool {
    matches!(
        s,
        "SELECT"
            | "FROM"
            | "WHERE"
            | "GROUP"
            | "HAVING"
            | "ORDER"
            | "LIMIT"
            | "OFFSET"
            | "UNION"
            | "INTERSECT"
            | "EXCEPT"
            | "INSERT"
            | "UPDATE"
            | "DELETE"
            | "SET"
            | "VALUES"
            | "CREATE"
            | "ALTER"
            | "DROP"
            | "TRUNCATE"
            | "WITH"
            | "BEGIN"
            | "COMMIT"
            | "ROLLBACK"
            | "RETURNING"
            | "ON"
            | "CONFLICT"
            | "INTO"
    )
}

fn is_join_keyword(s: &str) -> bool {
    matches!(
        s,
        "JOIN" | "INNER" | "OUTER" | "LEFT" | "RIGHT" | "FULL" | "CROSS"
    )
}

// ── Formatter ────────────────────────────────────────────────────────────────

fn action_format(sql: &str, args: &Value) -> Result<String, String> {
    let indent_str = args.get("indent").and_then(|v| v.as_str()).unwrap_or("  ");
    let uppercase = args
        .get("uppercase")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let tokens = tokenize(sql);
    let non_ws: Vec<&Token> = tokens
        .iter()
        .filter(|t| !matches!(t.kind, TokKind::Whitespace))
        .collect();

    let mut out = String::new();
    let mut depth: usize = 0;
    let mut i = 0;
    let n = non_ws.len();

    let kw = |t: &Token| -> String {
        if uppercase && t.kind == TokKind::Keyword {
            t.text.to_uppercase()
        } else {
            t.text.clone()
        }
    };

    while i < n {
        let tok = non_ws[i];

        match tok.kind {
            TokKind::Semi => {
                out.push(';');
                out.push('\n');
                depth = 0;
            }
            TokKind::Comma => {
                // In SELECT/GROUP BY context, comma triggers newline + indent
                out.push(',');
                out.push('\n');
                out.push_str(&indent_str.repeat(depth + 1));
            }
            TokKind::Lparen => {
                out.push('(');
                // Check if this is a subquery (followed by SELECT or WITH)
                if i + 1 < n
                    && matches!(
                        non_ws[i + 1].text.to_uppercase().as_str(),
                        "SELECT" | "WITH" | "VALUES"
                    )
                {
                    depth += 1;
                    out.push('\n');
                    out.push_str(&indent_str.repeat(depth));
                }
            }
            TokKind::Rparen => {
                // Check if we increased depth for a subquery
                if depth > 0 {
                    let prev_depth = depth;
                    // Look back to see if we opened a subquery paren
                    depth = depth.saturating_sub(1);
                    if prev_depth > 0 {
                        out.push('\n');
                        out.push_str(&indent_str.repeat(depth));
                    }
                }
                out.push(')');
            }
            TokKind::Keyword => {
                let upper = tok.text.to_uppercase();
                if is_clause_start(&upper) {
                    // These go at the beginning of a new line
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                    let keyword_text = kw(tok);
                    // Some keywords need a following keyword on same line (ORDER BY, GROUP BY, etc.)
                    if upper == "GROUP" || upper == "ORDER" || upper == "PARTITION" {
                        let by_follows = i + 1 < n && non_ws[i + 1].text.to_uppercase() == "BY";
                        if by_follows {
                            let combined = format!("{} {}", keyword_text, kw(non_ws[i + 1]));
                            out.push_str(&format!("{}{}", indent_str.repeat(depth), combined));
                            i += 2;
                            out.push('\n');
                            out.push_str(&indent_str.repeat(depth + 1));
                            continue;
                        }
                    }
                    out.push_str(&format!("{}{}", indent_str.repeat(depth), keyword_text));
                    out.push('\n');
                    out.push_str(&indent_str.repeat(depth + 1));
                } else if is_join_keyword(&upper) {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                    // Collect the full JOIN phrase (e.g. LEFT OUTER JOIN)
                    let mut join_parts = vec![kw(tok)];
                    let mut j = i + 1;
                    while j < n
                        && (is_join_keyword(&non_ws[j].text.to_uppercase())
                            || non_ws[j].text.to_uppercase() == "JOIN")
                    {
                        join_parts.push(kw(non_ws[j]));
                        j += 1;
                    }
                    i = j.saturating_sub(1);
                    let join_kw = join_parts.join(" ");
                    out.push_str(&format!("{}{}", indent_str.repeat(depth), join_kw));
                    out.push(' ');
                } else if upper == "AND" || upper == "OR" {
                    out.push('\n');
                    out.push_str(&indent_str.repeat(depth + 1));
                    out.push_str(&kw(tok));
                    out.push(' ');
                } else if upper == "AS" || upper == "ON" || upper == "USING" {
                    out.push(' ');
                    out.push_str(&kw(tok));
                    out.push(' ');
                } else if upper == "CASE" {
                    out.push_str(&kw(tok));
                    depth += 1;
                } else if upper == "WHEN" || upper == "ELSE" {
                    out.push('\n');
                    out.push_str(&indent_str.repeat(depth));
                    out.push_str(&kw(tok));
                    out.push(' ');
                } else if upper == "THEN" {
                    out.push(' ');
                    out.push_str(&kw(tok));
                    out.push(' ');
                } else if upper == "END" {
                    depth = depth.saturating_sub(1);
                    out.push('\n');
                    out.push_str(&indent_str.repeat(depth));
                    out.push_str(&kw(tok));
                } else {
                    // All other keywords: just emit with a space
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                    out.push_str(&kw(tok));
                    out.push(' ');
                }
            }
            TokKind::LineComment | TokKind::BlockComment => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str(&indent_str.repeat(depth));
                out.push_str(&tok.text);
                out.push('\n');
                out.push_str(&indent_str.repeat(depth));
            }
            _ => {
                // Ident, Number, Str, Op
                if !out.ends_with(' ') && !out.ends_with('\n') && !out.ends_with('(') {
                    out.push(' ');
                }
                out.push_str(&tok.text);
            }
        }
        i += 1;
    }

    // Clean up trailing whitespace on each line
    let cleaned: String = out
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    let cleaned = cleaned.trim_start_matches('\n').to_string();

    let original_len = sql.len();
    let formatted_len = cleaned.len();
    let mut result = String::new();
    writeln!(result, "Formatted SQL\n{}", "─".repeat(60)).ok();
    writeln!(
        result,
        "Original size: {} chars  →  Formatted: {} chars",
        original_len, formatted_len
    )
    .ok();
    writeln!(result, "{}", "─".repeat(60)).ok();
    writeln!(result).ok();
    result.push_str(&cleaned);
    Ok(result)
}

fn action_minify(sql: &str) -> Result<String, String> {
    let tokens = tokenize(sql);
    let mut out = String::new();
    let mut prev_was_ident = false;

    for tok in &tokens {
        match tok.kind {
            TokKind::Whitespace => continue,
            TokKind::LineComment | TokKind::BlockComment => continue,
            TokKind::Comma | TokKind::Semi | TokKind::Lparen | TokKind::Rparen | TokKind::Op => {
                out.push_str(&tok.text);
                prev_was_ident = false;
            }
            _ => {
                if prev_was_ident {
                    out.push(' ');
                }
                out.push_str(&tok.text);
                prev_was_ident = matches!(
                    tok.kind,
                    TokKind::Keyword | TokKind::Ident | TokKind::Number | TokKind::Str
                );
            }
        }
    }

    let original = sql.len();
    let minified = out.len();
    let savings = if original > 0 {
        100 - (minified * 100 / original)
    } else {
        0
    };
    let mut result = String::new();
    writeln!(result, "Minified SQL").ok();
    writeln!(result, "{}", "─".repeat(60)).ok();
    writeln!(
        result,
        "Original: {} chars  →  Minified: {} chars  ({}% reduction)",
        original, minified, savings
    )
    .ok();
    writeln!(result, "{}\n", "─".repeat(60)).ok();
    result.push_str(&out);
    Ok(result)
}

fn action_extract(sql: &str, args: &Value) -> Result<String, String> {
    let what = args
        .get("what")
        .and_then(|v| v.as_str())
        .unwrap_or("tables");
    let tokens = tokenize(sql);
    let non_ws: Vec<&Token> = tokens
        .iter()
        .filter(|t| {
            !matches!(
                t.kind,
                TokKind::Whitespace | TokKind::LineComment | TokKind::BlockComment
            )
        })
        .collect();
    let n = non_ws.len();

    match what {
        "tables" | "table" => {
            let mut tables = Vec::new();
            for i in 0..n {
                let upper = non_ws[i].text.to_uppercase();
                if matches!(
                    upper.as_str(),
                    "FROM" | "JOIN" | "INTO" | "UPDATE" | "TABLE"
                ) && i + 1 < n
                    && non_ws[i + 1].kind == TokKind::Ident
                {
                    let name = non_ws[i + 1].text.clone();
                    if !tables.contains(&name) {
                        tables.push(name);
                    }
                }
            }
            let mut out = format!(
                "Tables Referenced ({} found)\n{}\n",
                tables.len(),
                "─".repeat(40)
            );
            for t in &tables {
                writeln!(out, "  {t}").ok();
            }
            Ok(out)
        }
        "columns" | "column" => {
            // Extract columns from SELECT … FROM
            let mut cols = Vec::new();
            let mut in_select = false;
            for token in &non_ws {
                let upper = token.text.to_uppercase();
                if upper == "SELECT" {
                    in_select = true;
                    continue;
                }
                if in_select && matches!(upper.as_str(), "FROM" | "INTO") {
                    in_select = false;
                    continue;
                }
                if in_select && token.kind == TokKind::Ident {
                    let name = token.text.clone();
                    // Skip alias keywords
                    if !matches!(
                        name.to_uppercase().as_str(),
                        "AS" | "DISTINCT" | "ALL" | "TOP"
                    ) && !cols.contains(&name)
                    {
                        cols.push(name);
                    }
                }
            }
            let mut out = format!(
                "Columns in SELECT ({} found)\n{}\n",
                cols.len(),
                "─".repeat(40)
            );
            for c in &cols {
                writeln!(out, "  {c}").ok();
            }
            Ok(out)
        }
        "aliases" | "alias" => {
            let mut aliases = Vec::new();
            for i in 0..n.saturating_sub(1) {
                if non_ws[i].text.to_uppercase() == "AS" && i + 1 < n {
                    aliases.push(non_ws[i + 1].text.clone());
                }
            }
            let mut out = format!("Aliases ({} found)\n{}\n", aliases.len(), "─".repeat(40));
            for a in &aliases {
                writeln!(out, "  {a}").ok();
            }
            Ok(out)
        }
        "comments" | "comment" => {
            let comments: Vec<&str> = tokens
                .iter()
                .filter(|t| matches!(t.kind, TokKind::LineComment | TokKind::BlockComment))
                .map(|t| t.text.as_str())
                .collect();
            let mut out = format!("Comments ({} found)\n{}\n", comments.len(), "─".repeat(40));
            for c in &comments {
                writeln!(out, "  {c}").ok();
            }
            Ok(out)
        }
        _ => Err(format!(
            "Unknown 'what' value: '{}'. Use: tables, columns, aliases, comments",
            what
        )),
    }
}

fn action_split(sql: &str) -> Result<String, String> {
    let tokens = tokenize(sql);
    let mut statements: Vec<String> = Vec::new();
    let mut current = String::new();

    for tok in &tokens {
        if tok.kind == TokKind::Semi {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                statements.push(trimmed);
            }
            current.clear();
        } else {
            current.push_str(&tok.text);
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        statements.push(trimmed);
    }

    let mut out = format!(
        "Split SQL — {} statement(s) found\n{}\n\n",
        statements.len(),
        "─".repeat(60)
    );
    for (i, stmt) in statements.iter().enumerate() {
        writeln!(
            out,
            "── Statement {} ─────────────────────────────────────────",
            i + 1
        )
        .ok();
        writeln!(out, "{stmt};\n").ok();
    }
    Ok(out)
}
