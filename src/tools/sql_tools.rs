use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "parse" => parse_action(args),
        "tables" => tables_action(args),
        "explain" => explain_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: parse, tables, explain, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("sql"))
        .or_else(|| args.get("query"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the SQL content as a string".to_string())
}

// ── SQL tokeniser (enough for structural analysis) ───────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Word(String),
    Punct(char),
    Str(String),
    Comment(String),
}

fn tokenise(sql: &str) -> Vec<Tok> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Skip whitespace
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        // Line comment
        if i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '-' {
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            tokens.push(Tok::Comment(chars[start..i].iter().collect()));
            continue;
        }
        // Block comment
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            i += 2;
            let mut buf = String::from("/*");
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                buf.push(chars[i]);
                i += 1;
            }
            buf.push_str("*/");
            i += 2;
            tokens.push(Tok::Comment(buf));
            continue;
        }
        // String literal: ' or "
        if chars[i] == '\'' || chars[i] == '"' {
            let delim = chars[i];
            i += 1;
            let mut buf = String::new();
            while i < chars.len() {
                if chars[i] == delim {
                    if i + 1 < chars.len() && chars[i + 1] == delim {
                        buf.push(delim);
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    buf.push(chars[i]);
                    i += 1;
                }
            }
            tokens.push(Tok::Str(buf));
            continue;
        }
        // Backtick identifier
        if chars[i] == '`' {
            i += 1;
            let mut buf = String::new();
            while i < chars.len() && chars[i] != '`' {
                buf.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            tokens.push(Tok::Word(buf));
            continue;
        }
        // Bracket identifier [name]
        if chars[i] == '[' {
            i += 1;
            let mut buf = String::new();
            while i < chars.len() && chars[i] != ']' {
                buf.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            tokens.push(Tok::Word(buf));
            continue;
        }
        // Word / keyword / number
        if chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$' || chars[i] == '#' {
            let mut buf = String::new();
            while i < chars.len()
                && (chars[i].is_alphanumeric()
                    || chars[i] == '_'
                    || chars[i] == '$'
                    || chars[i] == '#'
                    || chars[i] == '.')
            {
                buf.push(chars[i]);
                i += 1;
            }
            tokens.push(Tok::Word(buf));
            continue;
        }
        // Punctuation
        tokens.push(Tok::Punct(chars[i]));
        i += 1;
    }
    tokens
}

fn words_only(tokens: &[Tok]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(|t| {
            if let Tok::Word(w) = t {
                Some(w.to_uppercase())
            } else {
                None
            }
        })
        .collect()
}

// ── Statement splitter ───────────────────────────────────────────────────────

fn split_statements(sql: &str) -> Vec<String> {
    let mut stmts = Vec::new();
    let mut buf = String::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut str_char = ' ';
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_str {
            buf.push(c);
            if c == str_char {
                if i + 1 < chars.len() && chars[i + 1] == str_char {
                    buf.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                in_str = false;
            }
        } else if c == '\'' || c == '"' {
            in_str = true;
            str_char = c;
            buf.push(c);
        } else if c == '(' {
            depth += 1;
            buf.push(c);
        } else if c == ')' {
            depth -= 1;
            buf.push(c);
        } else if c == ';' && depth == 0 {
            let s = buf.trim().to_string();
            if !s.is_empty() {
                stmts.push(s);
            }
            buf.clear();
        } else {
            buf.push(c);
        }
        i += 1;
    }
    let s = buf.trim().to_string();
    if !s.is_empty() {
        stmts.push(s);
    }
    stmts
}

fn stmt_keyword(stmt: &str) -> String {
    let tokens = tokenise(stmt);
    let ws = words_only(&tokens);
    // skip leading comments — grab first real keyword pair
    if ws.len() >= 2 {
        match ws[0].as_str() {
            "CREATE" | "DROP" | "ALTER" | "TRUNCATE" => {
                return format!("{} {}", ws[0], ws[1]);
            }
            _ => {}
        }
    }
    ws.first().cloned().unwrap_or_default()
}

// ── CREATE TABLE extractor ───────────────────────────────────────────────────

#[derive(Debug)]
struct Column {
    name: String,
    data_type: String,
    not_null: bool,
    primary_key: bool,
    default: Option<String>,
    references: Option<String>,
}

#[derive(Debug)]
struct Table {
    name: String,
    columns: Vec<Column>,
    primary_keys: Vec<String>, // from table-level PK constraint
    foreign_keys: Vec<(String, String, String)>, // (col, ref_table, ref_col)
    indexes: Vec<String>,
}

fn extract_tables(sql: &str) -> Vec<Table> {
    let stmts = split_statements(sql);
    let mut tables = Vec::new();

    for stmt in &stmts {
        let upper = stmt.to_uppercase();
        if !upper.trim_start().starts_with("CREATE TABLE") {
            continue;
        }
        if let Some(t) = parse_create_table(stmt) {
            tables.push(t);
        }
    }
    tables
}

fn parse_create_table(stmt: &str) -> Option<Table> {
    // Find table name: CREATE [TEMP] TABLE [IF NOT EXISTS] name (...)
    let tokens = tokenise(stmt);
    let ws = words_only(&tokens);
    // Find TABLE keyword position in words
    let mut tbl_idx = None;
    for (i, w) in ws.iter().enumerate() {
        if w == "TABLE" {
            tbl_idx = Some(i);
            break;
        }
    }
    let tbl_idx = tbl_idx?;
    // Skip IF NOT EXISTS
    let name_idx = if ws.get(tbl_idx + 1).map(|s| s.as_str()) == Some("IF") {
        tbl_idx + 4
    } else {
        tbl_idx + 1
    };
    let raw_name = ws.get(name_idx)?.clone();
    // Strip schema prefix if present (schema.name → name)
    let table_name = raw_name.split('.').last().unwrap_or(&raw_name).to_string();

    // Extract the body between the outermost parens
    let body = extract_paren_body(stmt)?;
    let columns = parse_column_defs(&body);
    let primary_keys = extract_table_pk(&body);
    let foreign_keys = extract_table_fks(&body);

    Some(Table {
        name: table_name,
        columns,
        primary_keys,
        foreign_keys,
        indexes: Vec::new(),
    })
}

fn extract_paren_body(stmt: &str) -> Option<String> {
    let start = stmt.find('(')?;
    let chars: Vec<char> = stmt.chars().collect();
    let mut depth = 0i32;
    let mut i = start;
    let mut end = start;
    while i < chars.len() {
        if chars[i] == '(' {
            depth += 1;
        } else if chars[i] == ')' {
            depth -= 1;
            if depth == 0 {
                end = i;
                break;
            }
        }
        i += 1;
    }
    Some(chars[start + 1..end].iter().collect())
}

fn split_col_defs(body: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut str_char = ' ';
    for c in body.chars() {
        if in_str {
            buf.push(c);
            if c == str_char {
                in_str = false;
            }
            continue;
        }
        if c == '\'' || c == '"' {
            in_str = true;
            str_char = c;
            buf.push(c);
            continue;
        }
        if c == '(' {
            depth += 1;
            buf.push(c);
        } else if c == ')' {
            depth -= 1;
            buf.push(c);
        } else if c == ',' && depth == 0 {
            let s = buf.trim().to_string();
            if !s.is_empty() {
                parts.push(s);
            }
            buf.clear();
        } else {
            buf.push(c);
        }
    }
    let s = buf.trim().to_string();
    if !s.is_empty() {
        parts.push(s);
    }
    parts
}

fn parse_column_defs(body: &str) -> Vec<Column> {
    let defs = split_col_defs(body);
    let mut cols = Vec::new();
    for def in &defs {
        let upper = def.to_uppercase();
        let upper_trimmed = upper.trim_start();
        // Skip table-level constraints
        if upper_trimmed.starts_with("PRIMARY")
            || upper_trimmed.starts_with("UNIQUE")
            || upper_trimmed.starts_with("FOREIGN")
            || upper_trimmed.starts_with("CHECK")
            || upper_trimmed.starts_with("CONSTRAINT")
            || upper_trimmed.starts_with("INDEX")
            || upper_trimmed.starts_with("KEY")
        {
            continue;
        }
        let tokens = tokenise(def);
        let ws: Vec<&str> = tokens
            .iter()
            .filter_map(|t| {
                if let Tok::Word(w) = t {
                    Some(w.as_str())
                } else {
                    None
                }
            })
            .collect();
        if ws.len() < 2 {
            continue;
        }
        let col_name = ws[0].to_string();
        let data_type = ws[1].to_string();
        let upper_def = def.to_uppercase();
        let not_null = upper_def.contains("NOT NULL");
        let primary_key = upper_def.contains("PRIMARY KEY");
        let default = extract_default_value(def);
        let references = extract_inline_references(def);

        cols.push(Column {
            name: col_name,
            data_type,
            not_null,
            primary_key,
            default,
            references,
        });
    }
    cols
}

fn extract_default_value(def: &str) -> Option<String> {
    let upper = def.to_uppercase();
    let idx = upper.find("DEFAULT")?;
    let after = &def[idx + 7..].trim_start();
    // Read until whitespace or comma or end (outside parens)
    let mut val = String::new();
    let mut depth = 0i32;
    for c in after.chars() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            if depth == 0 {
                break;
            }
            depth -= 1;
        } else if c == ',' && depth == 0 {
            break;
        } else if c.is_whitespace() && depth == 0 && !val.is_empty() {
            break;
        }
        val.push(c);
    }
    let val = val.trim().to_string();
    if val.is_empty() {
        None
    } else {
        Some(val)
    }
}

fn extract_inline_references(def: &str) -> Option<String> {
    let upper = def.to_uppercase();
    let idx = upper.find("REFERENCES")?;
    let after = &def[idx + 10..].trim_start();
    // Read until '(' or end
    let name: String = after
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.' || *c == '"' || *c == '`')
        .collect();
    let name = name
        .trim_matches(|c: char| c == '"' || c == '`')
        .to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn extract_table_pk(body: &str) -> Vec<String> {
    let defs = split_col_defs(body);
    let mut pks = Vec::new();
    for def in &defs {
        let upper = def.to_uppercase();
        if upper.trim_start().starts_with("PRIMARY KEY") {
            if let Some(body) = extract_paren_body(def) {
                for part in body.split(',') {
                    let name = part.trim().trim_matches(|c: char| {
                        c == '"' || c == '`' || c == '\'' || c == '[' || c == ']'
                    });
                    if !name.is_empty() {
                        pks.push(name.to_string());
                    }
                }
            }
        }
    }
    pks
}

fn extract_table_fks(body: &str) -> Vec<(String, String, String)> {
    let defs = split_col_defs(body);
    let mut fks = Vec::new();
    for def in &defs {
        let upper = def.to_uppercase();
        if !upper.trim_start().contains("FOREIGN KEY") {
            continue;
        }
        // FOREIGN KEY (col) REFERENCES table (col)
        let tokens = tokenise(def);
        let ws_raw: Vec<String> = tokens
            .iter()
            .filter_map(|t| {
                if let Tok::Word(w) = t {
                    Some(w.clone())
                } else {
                    None
                }
            })
            .collect();
        // Find FOREIGN, KEY, then paren groups
        let fk_col = extract_paren_body(def)
            .map(|b| b.trim().to_string())
            .unwrap_or_default();

        // Find REFERENCES
        let upper_tok: Vec<String> = ws_raw.iter().map(|s| s.to_uppercase()).collect();
        if let Some(ref_idx) = upper_tok.iter().position(|s| s == "REFERENCES") {
            let ref_table = ws_raw.get(ref_idx + 1).cloned().unwrap_or_default();
            let ref_col = {
                // find second paren body
                let after_ref = &def.to_uppercase();
                let first_paren = after_ref.find('(');
                let rest = first_paren
                    .map(|p| &def[p + fk_col.len() + 2..])
                    .unwrap_or("");
                extract_paren_body(rest)
                    .map(|b| b.trim().to_string())
                    .unwrap_or_default()
            };
            if !fk_col.is_empty() && !ref_table.is_empty() {
                fks.push((fk_col, ref_table, ref_col));
            }
        }
    }
    fks
}

// ── Query analysis helpers ───────────────────────────────────────────────────

fn count_joins(sql: &str) -> usize {
    let upper = sql.to_uppercase();
    let mut count = 0;
    let mut pos = 0;
    while let Some(idx) = upper[pos..].find("JOIN") {
        count += 1;
        pos += idx + 4;
    }
    count
}

fn extract_referenced_tables(stmt: &str) -> Vec<String> {
    let tokens = tokenise(stmt);
    let ws: Vec<String> = tokens
        .iter()
        .filter_map(|t| {
            if let Tok::Word(w) = t {
                Some(w.to_uppercase())
            } else {
                None
            }
        })
        .collect();
    let mut tables = Vec::new();
    let keywords = ["FROM", "JOIN", "INTO", "UPDATE", "TABLE"];
    for (i, w) in ws.iter().enumerate() {
        if keywords.contains(&w.as_str()) {
            if let Some(next) = ws.get(i + 1) {
                if !is_keyword(next) && next.len() > 1 {
                    tables.push(next.to_string());
                }
            }
        }
    }
    tables.dedup();
    tables
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "SELECT"
            | "FROM"
            | "WHERE"
            | "JOIN"
            | "INNER"
            | "LEFT"
            | "RIGHT"
            | "OUTER"
            | "FULL"
            | "CROSS"
            | "ON"
            | "AND"
            | "OR"
            | "NOT"
            | "IN"
            | "IS"
            | "NULL"
            | "AS"
            | "SET"
            | "INTO"
            | "VALUES"
            | "UPDATE"
            | "DELETE"
            | "INSERT"
            | "ORDER"
            | "GROUP"
            | "BY"
            | "HAVING"
            | "LIMIT"
            | "OFFSET"
            | "UNION"
            | "ALL"
            | "DISTINCT"
            | "CASE"
            | "WHEN"
            | "THEN"
            | "ELSE"
            | "END"
            | "EXISTS"
            | "ANY"
            | "SOME"
            | "BETWEEN"
            | "LIKE"
            | "ILIKE"
            | "USING"
            | "NATURAL"
            | "TABLE"
            | "CREATE"
            | "DROP"
            | "ALTER"
            | "INDEX"
            | "VIEW"
            | "RETURNING"
            | "WITH"
            | "CTE"
    )
}

// ── Actions ──────────────────────────────────────────────────────────────────

fn parse_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let stmts = split_statements(&text);

    if stmts.is_empty() {
        return Ok("No SQL statements found.\n".to_string());
    }

    let mut out = format!(
        "SQL Analysis  [{} statement(s)]\n{}\n\n",
        stmts.len(),
        "=".repeat(44)
    );

    // Count by type
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for stmt in &stmts {
        let kw = stmt_keyword(stmt);
        *counts.entry(kw).or_insert(0) += 1;
    }
    let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out += "Statement types:\n";
    for (kw, n) in &sorted {
        out += &format!("  {:<20} {}\n", kw, n);
    }
    out += "\n";

    // Per-statement summary
    out += "Statements:\n";
    for (i, stmt) in stmts.iter().enumerate() {
        let kw = stmt_keyword(stmt);
        let tables = extract_referenced_tables(stmt);
        let joins = count_joins(stmt);
        let has_subquery = stmt.to_uppercase().contains("SELECT")
            && stmt.to_uppercase()[stmt.to_uppercase().find("SELECT").unwrap() + 6..]
                .contains("SELECT");
        let snippet: String = stmt.chars().take(60).collect();
        let ellipsis = if stmt.len() > 60 { "…" } else { "" };
        out += &format!("  {:>2}. [{:<16}] {}{}\n", i + 1, kw, snippet, ellipsis);
        if !tables.is_empty() {
            out += &format!("      Tables: {}\n", tables.join(", "));
        }
        if joins > 0 {
            out += &format!("      Joins: {}\n", joins);
        }
        if has_subquery {
            out += "      Contains subquery\n";
        }
    }

    Ok(out)
}

fn tables_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let tables = extract_tables(&text);

    if tables.is_empty() {
        return Ok("No CREATE TABLE statements found.\n".to_string());
    }

    let mut out = format!(
        "Tables  [{} table(s)]\n{}\n\n",
        tables.len(),
        "=".repeat(44)
    );

    for table in &tables {
        out += &format!("Table: {}\n", table.name);
        if !table.primary_keys.is_empty() {
            out += &format!("  PK: {}\n", table.primary_keys.join(", "));
        }
        out += &format!("  Columns: {}\n", table.columns.len());
        for col in &table.columns {
            let mut flags = Vec::new();
            if col.primary_key {
                flags.push("PK");
            }
            if col.not_null {
                flags.push("NOT NULL");
            }
            if let Some(ref d) = col.default {
                flags.push("DEFAULT");
                let _ = d;
            }
            if col.references.is_some() {
                flags.push("FK");
            }
            let flag_str = if flags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", flags.join(", "))
            };
            out += &format!("    {:<24} {}{}\n", col.name, col.data_type, flag_str);
        }
        if !table.foreign_keys.is_empty() {
            out += "  Foreign keys:\n";
            for (col, ref_table, ref_col) in &table.foreign_keys {
                out += &format!(
                    "    {} → {}({})\n",
                    col,
                    ref_table,
                    if ref_col.is_empty() { "?" } else { ref_col }
                );
            }
        }
        out += "\n";
    }

    Ok(out)
}

fn explain_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let stmts = split_statements(&text);

    if stmts.is_empty() {
        return Ok("No SQL statements found.\n".to_string());
    }

    let mut out = format!("SQL Explanation\n{}\n\n", "=".repeat(44));

    for (i, stmt) in stmts.iter().enumerate() {
        let tokens = tokenise(stmt);
        let ws: Vec<String> = words_only(&tokens);
        let first = ws.first().map(|s| s.as_str()).unwrap_or("");

        let explanation = match first {
            "SELECT" => explain_select(stmt, &ws),
            "INSERT" => explain_insert(&ws),
            "UPDATE" => explain_update(&ws),
            "DELETE" => explain_delete(&ws),
            "CREATE" => explain_create(&ws),
            "DROP" => explain_drop(&ws),
            "ALTER" => explain_alter(&ws),
            "TRUNCATE" => {
                let tbl = ws.get(2).or_else(|| ws.get(1)).cloned().unwrap_or_default();
                format!("Remove all rows from table '{}' (fast, no logging per-row)", tbl)
            }
            "WITH" => "Common Table Expression (CTE) — defines a named temporary result set for use in the following SELECT/INSERT/UPDATE/DELETE".to_string(),
            "GRANT" => "Grant privileges to a role or user".to_string(),
            "REVOKE" => "Revoke previously granted privileges".to_string(),
            "BEGIN" | "START" => "Begin a transaction block".to_string(),
            "COMMIT" => "Commit (persist) the current transaction".to_string(),
            "ROLLBACK" => "Roll back (undo) the current transaction".to_string(),
            _ => format!("'{}' statement", first),
        };

        if stmts.len() > 1 {
            out += &format!("Statement {}: {}\n\n", i + 1, explanation);
        } else {
            out += &format!("{}\n\n", explanation);
        }
    }

    Ok(out)
}

fn explain_select(stmt: &str, ws: &[String]) -> String {
    let upper = stmt.to_uppercase();
    let tables = extract_referenced_tables(stmt);
    let joins = count_joins(stmt);
    let has_where = upper.contains("WHERE");
    let has_group = upper.contains("GROUP BY");
    let has_having = upper.contains("HAVING");
    let has_order = upper.contains("ORDER BY");
    let has_limit = upper.contains("LIMIT");
    let has_distinct = ws.get(1).map(|s| s.as_str()) == Some("DISTINCT");
    let has_subquery = {
        let after_first = upper.find("SELECT").map(|p| &upper[p + 6..]).unwrap_or("");
        after_first.contains("SELECT")
    };
    let has_union = upper.contains("UNION");
    let has_cte = upper.contains("WITH") && upper.contains("AS");

    let mut parts = Vec::new();
    if has_cte {
        parts.push("uses CTEs".to_string());
    }
    if has_distinct {
        parts.push("DISTINCT rows".to_string());
    }
    match tables.len() {
        0 => {}
        1 => parts.push(format!("from '{}'", tables[0].to_lowercase())),
        n => parts.push(format!(
            "from {} tables ({})",
            n,
            tables
                .iter()
                .map(|t| t.to_lowercase())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
    if joins > 0 {
        parts.push(format!("{} join(s)", joins));
    }
    if has_where {
        parts.push("filtered by WHERE".to_string());
    }
    if has_group {
        parts.push("grouped".to_string());
    }
    if has_having {
        parts.push("HAVING filter on groups".to_string());
    }
    if has_order {
        parts.push("sorted".to_string());
    }
    if has_limit {
        parts.push("limited".to_string());
    }
    if has_subquery {
        parts.push("contains subquery".to_string());
    }
    if has_union {
        parts.push("UNION of result sets".to_string());
    }

    if parts.is_empty() {
        "SELECT — retrieve rows from the database".to_string()
    } else {
        format!("SELECT — retrieve rows; {}", parts.join("; "))
    }
}

fn explain_insert(ws: &[String]) -> String {
    let tbl = ws.get(2).or_else(|| ws.get(1)).cloned().unwrap_or_default();
    let has_select = ws.iter().any(|w| w == "SELECT");
    if has_select {
        format!(
            "INSERT INTO '{}' — insert rows from a SELECT subquery",
            tbl.to_lowercase()
        )
    } else {
        format!(
            "INSERT INTO '{}' — insert one or more rows with VALUES",
            tbl.to_lowercase()
        )
    }
}

fn explain_update(ws: &[String]) -> String {
    let tbl = ws.get(1).cloned().unwrap_or_default();
    format!("UPDATE '{}' — modify existing rows", tbl.to_lowercase())
}

fn explain_delete(ws: &[String]) -> String {
    let tbl = ws.get(2).or_else(|| ws.get(1)).cloned().unwrap_or_default();
    format!(
        "DELETE FROM '{}' — remove rows (use WHERE or all rows are deleted)",
        tbl.to_lowercase()
    )
}

fn explain_create(ws: &[String]) -> String {
    let what = ws.get(1).map(|s| s.as_str()).unwrap_or("");
    let name = ws.get(2).or_else(|| ws.get(3)).cloned().unwrap_or_default();
    match what {
        "TABLE" => format!(
            "CREATE TABLE '{}' — define a new table schema",
            name.to_lowercase()
        ),
        "INDEX" => format!(
            "CREATE INDEX on '{}' — add an index for faster lookups",
            name.to_lowercase()
        ),
        "VIEW" => format!(
            "CREATE VIEW '{}' — define a virtual table from a SELECT",
            name.to_lowercase()
        ),
        "UNIQUE" => {
            let name2 = ws.get(3).cloned().unwrap_or_default();
            format!(
                "CREATE UNIQUE INDEX '{}' — unique constraint index",
                name2.to_lowercase()
            )
        }
        "DATABASE" => format!("CREATE DATABASE '{}'", name.to_lowercase()),
        "SCHEMA" => format!("CREATE SCHEMA '{}'", name.to_lowercase()),
        "SEQUENCE" => format!(
            "CREATE SEQUENCE '{}' — auto-increment number generator",
            name.to_lowercase()
        ),
        "TRIGGER" => format!(
            "CREATE TRIGGER '{}' — automatic action on table events",
            name.to_lowercase()
        ),
        "FUNCTION" | "PROCEDURE" => format!(
            "CREATE {} '{}' — stored procedural logic",
            what,
            name.to_lowercase()
        ),
        _ => format!("CREATE {} '{}'", what, name.to_lowercase()),
    }
}

fn explain_drop(ws: &[String]) -> String {
    let what = ws.get(1).map(|s| s.as_str()).unwrap_or("");
    let name = ws.get(2).or_else(|| ws.get(3)).cloned().unwrap_or_default();
    format!(
        "DROP {} '{}' — permanently remove the {} from the database",
        what,
        name.to_lowercase(),
        what.to_lowercase()
    )
}

fn explain_alter(ws: &[String]) -> String {
    let what = ws.get(1).map(|s| s.as_str()).unwrap_or("");
    let name = ws.get(2).cloned().unwrap_or_default();
    let op = ws.get(3).map(|s| s.as_str()).unwrap_or("");
    match op {
        "ADD" => format!(
            "ALTER TABLE '{}' ADD — add a new column or constraint",
            name.to_lowercase()
        ),
        "DROP" => format!(
            "ALTER TABLE '{}' DROP — remove a column or constraint",
            name.to_lowercase()
        ),
        "RENAME" => format!(
            "ALTER TABLE '{}' RENAME — rename the table or a column",
            name.to_lowercase()
        ),
        "MODIFY" | "ALTER" => format!(
            "ALTER TABLE '{}' MODIFY — change column type or constraint",
            name.to_lowercase()
        ),
        _ => format!(
            "ALTER {} '{}' — modify the {} definition",
            what,
            name.to_lowercase(),
            what.to_lowercase()
        ),
    }
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let stmts = split_statements(&text);
    let mut warnings: Vec<String> = Vec::new();

    if stmts.is_empty() {
        return Ok("No SQL statements found.\n".to_string());
    }

    for (i, stmt) in stmts.iter().enumerate() {
        let upper = stmt.to_uppercase();
        let stmt_label = format!("Stmt {}", i + 1);

        // SELECT * is a bad practice
        if upper.contains("SELECT *") || upper.contains("SELECT\t*") {
            warnings.push(format!(
                "[{}] SELECT * — prefer explicit column names for clarity and future-proofing",
                stmt_label
            ));
        }

        // DELETE or UPDATE without WHERE
        let is_delete = upper.trim_start().starts_with("DELETE");
        let is_update = upper.trim_start().starts_with("UPDATE");
        if (is_delete || is_update) && !upper.contains("WHERE") {
            warnings.push(format!(
                "[{}] {} without WHERE clause — this will affect ALL rows",
                stmt_label,
                if is_delete { "DELETE" } else { "UPDATE" }
            ));
        }

        // DROP TABLE without IF EXISTS
        if upper.contains("DROP TABLE") && !upper.contains("IF EXISTS") {
            warnings.push(format!(
                "[{}] DROP TABLE without IF EXISTS — will error if table doesn't exist",
                stmt_label
            ));
        }

        // SELECT inside a loop-style pattern (subquery in FROM without alias)
        if upper.contains("FROM (SELECT") || upper.contains("FROM(SELECT") {
            warnings.push(format!(
                "[{}] Derived table (subquery in FROM) — ensure it has an alias, or consider a CTE for clarity",
                stmt_label
            ));
        }

        // Implicit CROSS JOIN (multiple tables in FROM without JOIN keyword)
        if upper.starts_with("SELECT") {
            let from_idx = upper.find("FROM");
            let where_idx = upper.find("WHERE").unwrap_or(upper.len());
            if let Some(fi) = from_idx {
                let from_clause = &upper[fi + 4..where_idx.min(upper.len())];
                if !from_clause.contains("JOIN") {
                    let comma_count = from_clause.chars().filter(|&c| c == ',').count();
                    if comma_count >= 1 {
                        warnings.push(format!(
                            "[{}] Implicit cross join (comma-separated FROM tables) — use explicit JOIN syntax instead",
                            stmt_label
                        ));
                    }
                }
            }
        }

        // NOT IN with NULL risk
        if upper.contains("NOT IN") {
            warnings.push(format!(
                "[{}] NOT IN may return unexpected results if the subquery contains NULL — consider NOT EXISTS instead",
                stmt_label
            ));
        }

        // LIKE with leading wildcard is slow
        if upper.contains("LIKE '%") || upper.contains("LIKE \"%") {
            warnings.push(format!(
                "[{}] LIKE with leading wildcard ('%...') prevents index use — consider full-text search if this is frequent",
                stmt_label
            ));
        }

        // Tables: check CREATE TABLE for missing PK
        if upper.trim_start().starts_with("CREATE TABLE") {
            if let Some(table) = parse_create_table(stmt) {
                let has_pk =
                    !table.primary_keys.is_empty() || table.columns.iter().any(|c| c.primary_key);
                if !has_pk {
                    warnings.push(format!(
                        "[{}] Table '{}' has no PRIMARY KEY defined",
                        stmt_label, table.name
                    ));
                }
            }
        }
    }

    let mut out = format!("SQL Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!("{} statement(s) analysed.\n", stmts.len());
    if warnings.is_empty() {
        out += "No issues found.\n";
    } else {
        out += &format!("\n{} warning(s):\n", warnings.len());
        for w in &warnings {
            out += &format!("  [WARN] {}\n", w);
        }
    }
    Ok(out)
}
