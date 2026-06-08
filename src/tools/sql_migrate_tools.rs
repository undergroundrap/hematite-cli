use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("analyze");

    let text = get_text(args)?;

    match action {
        "analyze" => action_analyze(&text),
        "risk" => action_risk(&text),
        "ops" => action_ops(&text),
        "validate" => action_validate(&text),
        other => Err(format!(
            "Unknown action '{}'. Valid: analyze, risk, ops, validate",
            other
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("sql"))
        .or_else(|| args.get("migration"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass SQL migration content as a string".to_string())
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    fn label(&self) -> &'static str {
        match self {
            RiskLevel::Safe => "SAFE",
            RiskLevel::Low => "LOW",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::High => "HIGH",
            RiskLevel::Critical => "CRITICAL",
        }
    }

    fn score(&self) -> u8 {
        match self {
            RiskLevel::Safe => 0,
            RiskLevel::Low => 1,
            RiskLevel::Medium => 2,
            RiskLevel::High => 3,
            RiskLevel::Critical => 4,
        }
    }
}

#[derive(Debug, Clone)]
struct MigrationOp {
    statement: String,
    kind: String,
    risk: RiskLevel,
    notes: Vec<String>,
    line: usize,
}

// ── Statement splitter and parser ─────────────────────────────────────────────

fn split_statements(sql: &str) -> Vec<(usize, String)> {
    let mut stmts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut str_char = ' ';
    let mut line = 1usize;
    let mut stmt_start_line = 1usize;
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Track lines
        if c == '\n' {
            line += 1;
        }

        // Line comments
        if !in_str && c == '-' && chars.get(i + 1) == Some(&'-') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Block comments
        if !in_str && c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2;
            continue;
        }

        // String literals
        if !in_str && (c == '\'' || c == '"') {
            in_str = true;
            str_char = c;
            current.push(c);
            i += 1;
            continue;
        }
        if in_str && c == str_char {
            // escaped quote?
            if chars.get(i + 1) == Some(&str_char) {
                current.push(c);
                current.push(c);
                i += 2;
                continue;
            }
            in_str = false;
            current.push(c);
            i += 1;
            continue;
        }

        if !in_str {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
            }

            if c == ';' && depth == 0 {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    stmts.push((stmt_start_line, trimmed));
                }
                current.clear();
                stmt_start_line = line + 1;
                i += 1;
                continue;
            }
        }

        current.push(c);
        i += 1;
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        stmts.push((stmt_start_line, trimmed));
    }
    stmts
}

fn first_words(sql: &str, n: usize) -> Vec<String> {
    sql.split_whitespace()
        .take(n)
        .map(|s| s.to_uppercase())
        .collect()
}

fn extract_table_name(words: &[String], after: &str) -> String {
    let after_up = after.to_uppercase();
    for (i, w) in words.iter().enumerate() {
        if w == &after_up {
            return words
                .get(i + 1)
                .cloned()
                .unwrap_or_default()
                .trim_matches(|c: char| c == '"' || c == '`' || c == '[' || c == ']')
                .to_string();
        }
    }
    String::new()
}

fn sql_upper(sql: &str) -> String {
    sql.to_uppercase()
}

fn analyze_statement(line: usize, stmt: &str) -> MigrationOp {
    let words = first_words(stmt, 8);
    let upper = sql_upper(stmt);
    let first = words.first().map(|s| s.as_str()).unwrap_or("");
    let second = words.get(1).map(|s| s.as_str()).unwrap_or("");

    match (first, second) {
        // ── DDL: CREATE ──────────────────────────────────────────────────────
        ("CREATE", "TABLE") => {
            let table = extract_table_name(&words, "TABLE");
            let if_not_exists = upper.contains("IF NOT EXISTS");
            let mut notes = Vec::new();
            if !if_not_exists {
                notes.push("Consider IF NOT EXISTS for idempotent deployments".to_string());
            }
            MigrationOp {
                statement: format!("CREATE TABLE {}", table),
                kind: "CREATE TABLE".to_string(),
                risk: RiskLevel::Safe,
                notes,
                line,
            }
        }
        ("CREATE", "INDEX") | ("CREATE", "UNIQUE") => {
            let table = extract_table_name(&words, "ON");
            let concurrent = upper.contains("CONCURRENTLY");
            let mut notes = Vec::new();
            if !concurrent {
                notes.push("Consider CONCURRENTLY to avoid table lock on large tables".to_string());
            }
            MigrationOp {
                statement: format!("CREATE INDEX ON {}", table),
                kind: "CREATE INDEX".to_string(),
                risk: if concurrent {
                    RiskLevel::Low
                } else {
                    RiskLevel::Medium
                },
                notes,
                line,
            }
        }
        ("CREATE", "VIEW") | ("CREATE", "OR") => MigrationOp {
            statement: "CREATE VIEW".to_string(),
            kind: "CREATE VIEW".to_string(),
            risk: RiskLevel::Safe,
            notes: vec![],
            line,
        },
        ("CREATE", kind) => MigrationOp {
            statement: format!("CREATE {}", kind),
            kind: format!("CREATE {}", kind),
            risk: RiskLevel::Safe,
            notes: vec![],
            line,
        },

        // ── DDL: DROP ────────────────────────────────────────────────────────
        ("DROP", "TABLE") => {
            let table = extract_table_name(&words, "TABLE");
            let if_exists = upper.contains("IF EXISTS");
            let cascade = upper.contains("CASCADE");
            let mut notes = Vec::new();
            if !if_exists {
                notes.push("Add IF EXISTS for safer deployments".to_string());
            }
            if cascade {
                notes.push("CASCADE will drop dependent objects — verify intentional".to_string());
            }
            notes.push("Destructive and irreversible — ensure data is backed up".to_string());
            MigrationOp {
                statement: format!("DROP TABLE {}", table),
                kind: "DROP TABLE".to_string(),
                risk: RiskLevel::Critical,
                notes,
                line,
            }
        }
        ("DROP", "INDEX") => {
            let concurrent = upper.contains("CONCURRENTLY");
            let mut notes = Vec::new();
            if !concurrent {
                notes.push("Consider CONCURRENTLY to avoid read lock on PostgreSQL".to_string());
            }
            MigrationOp {
                statement: "DROP INDEX".to_string(),
                kind: "DROP INDEX".to_string(),
                risk: RiskLevel::Low,
                notes,
                line,
            }
        }
        ("DROP", "COLUMN") | ("ALTER", _) if upper.contains("DROP COLUMN") => {
            let mut notes = vec![
                "Column drops are irreversible — data is permanently lost".to_string(),
                "Ensure no application code still references this column before deploying"
                    .to_string(),
            ];
            if upper.contains("CASCADE") {
                notes.push("CASCADE will also drop dependent views/constraints".to_string());
            }
            MigrationOp {
                statement: "DROP COLUMN".to_string(),
                kind: "DROP COLUMN".to_string(),
                risk: RiskLevel::Critical,
                notes,
                line,
            }
        }
        ("DROP", kind) => MigrationOp {
            statement: format!("DROP {}", kind),
            kind: format!("DROP {}", kind),
            risk: RiskLevel::High,
            notes: vec!["Destructive operation — verify intentional".to_string()],
            line,
        },

        // ── DDL: ALTER ───────────────────────────────────────────────────────
        ("ALTER", "TABLE") => {
            let table = extract_table_name(&words, "TABLE");
            let sub_op = words.get(3).map(|s| s.as_str()).unwrap_or("");

            match sub_op {
                "ADD" => {
                    let col = upper.contains("ADD COLUMN") || upper.contains("ADD ");
                    let has_default = upper.contains("DEFAULT");
                    let not_null = upper.contains("NOT NULL");
                    let mut notes = Vec::new();
                    if not_null && !has_default {
                        notes.push(
                            "Adding NOT NULL column without DEFAULT requires table rewrite — may lock large tables".to_string(),
                        );
                    }
                    if not_null && has_default {
                        notes.push(
                            "Adding NOT NULL with DEFAULT: PostgreSQL 11+ handles this online; older versions rewrite the table".to_string(),
                        );
                    }
                    let _ = col;
                    MigrationOp {
                        statement: format!("ALTER TABLE {} ADD COLUMN", table),
                        kind: "ALTER TABLE ADD COLUMN".to_string(),
                        risk: if not_null && !has_default {
                            RiskLevel::High
                        } else {
                            RiskLevel::Low
                        },
                        notes,
                        line,
                    }
                }
                "RENAME" => {
                    let mut notes = vec![
                        "Column/table renames require all application code referencing this name to be updated simultaneously".to_string(),
                    ];
                    if upper.contains("RENAME TO") && !upper.contains("RENAME COLUMN") {
                        notes.push(
                            "Table rename: update ORM models, views, and any raw SQL references"
                                .to_string(),
                        );
                    }
                    MigrationOp {
                        statement: format!("ALTER TABLE {} RENAME", table),
                        kind: "ALTER TABLE RENAME".to_string(),
                        risk: RiskLevel::High,
                        notes,
                        line,
                    }
                }
                "ALTER" => {
                    let mut notes = Vec::new();
                    if upper.contains("SET NOT NULL") {
                        notes.push(
                            "SET NOT NULL requires a full table scan to verify existing rows — slow on large tables".to_string(),
                        );
                    }
                    if upper.contains("TYPE") {
                        notes.push(
                            "Changing column type may require table rewrite and can break dependent views".to_string(),
                        );
                    }
                    MigrationOp {
                        statement: format!("ALTER TABLE {} ALTER COLUMN", table),
                        kind: "ALTER TABLE ALTER COLUMN".to_string(),
                        risk: RiskLevel::Medium,
                        notes,
                        line,
                    }
                }
                _ => MigrationOp {
                    statement: format!("ALTER TABLE {}", table),
                    kind: "ALTER TABLE".to_string(),
                    risk: RiskLevel::Medium,
                    notes: vec![],
                    line,
                },
            }
        }

        // ── DML ──────────────────────────────────────────────────────────────
        ("UPDATE", _) => {
            let has_where = upper.contains(" WHERE ");
            let mut notes = Vec::new();
            if !has_where {
                notes
                    .push("UPDATE without WHERE updates ALL rows — verify intentional".to_string());
            }
            MigrationOp {
                statement: format!("UPDATE {}", words.get(1).cloned().unwrap_or_default()),
                kind: "UPDATE".to_string(),
                risk: if has_where {
                    RiskLevel::Medium
                } else {
                    RiskLevel::High
                },
                notes,
                line,
            }
        }
        ("DELETE", "FROM") | ("DELETE", _) => {
            let has_where = upper.contains(" WHERE ");
            let mut notes = Vec::new();
            if !has_where {
                notes
                    .push("DELETE without WHERE deletes ALL rows — verify intentional".to_string());
            }
            notes.push("Data loss is irreversible unless wrapped in a transaction".to_string());
            MigrationOp {
                statement: "DELETE".to_string(),
                kind: "DELETE".to_string(),
                risk: if has_where {
                    RiskLevel::Medium
                } else {
                    RiskLevel::Critical
                },
                notes,
                line,
            }
        }
        ("INSERT", "INTO") => {
            let table = extract_table_name(&words, "INTO");
            let has_on_conflict = upper.contains("ON CONFLICT");
            let mut notes = Vec::new();
            if !has_on_conflict {
                notes.push("Consider ON CONFLICT clause for idempotent seed data".to_string());
            }
            MigrationOp {
                statement: format!("INSERT INTO {}", table),
                kind: "INSERT".to_string(),
                risk: RiskLevel::Low,
                notes,
                line,
            }
        }
        ("TRUNCATE", _) => {
            let cascade = upper.contains("CASCADE");
            let mut notes = vec![
                "TRUNCATE removes all rows instantly — no WHERE clause, no rollback in some engines".to_string(),
            ];
            if cascade {
                notes.push("CASCADE truncates dependent tables — verify intentional".to_string());
            }
            MigrationOp {
                statement: format!("TRUNCATE {}", words.get(1).cloned().unwrap_or_default()),
                kind: "TRUNCATE".to_string(),
                risk: RiskLevel::Critical,
                notes,
                line,
            }
        }

        // ── Transaction control ───────────────────────────────────────────────
        ("BEGIN", _) | ("START", _) => MigrationOp {
            statement: "BEGIN TRANSACTION".to_string(),
            kind: "BEGIN".to_string(),
            risk: RiskLevel::Safe,
            notes: vec![],
            line,
        },
        ("COMMIT", _) => MigrationOp {
            statement: "COMMIT".to_string(),
            kind: "COMMIT".to_string(),
            risk: RiskLevel::Safe,
            notes: vec![],
            line,
        },
        ("ROLLBACK", _) => MigrationOp {
            statement: "ROLLBACK".to_string(),
            kind: "ROLLBACK".to_string(),
            risk: RiskLevel::Safe,
            notes: vec![],
            line,
        },

        // ── Other ─────────────────────────────────────────────────────────────
        ("GRANT", _) | ("REVOKE", _) => MigrationOp {
            statement: format!("{} permission change", first),
            kind: first.to_string(),
            risk: RiskLevel::Medium,
            notes: vec!["Permission changes affect all sessions immediately".to_string()],
            line,
        },
        ("VACUUM", _) | ("ANALYZE", _) | ("REINDEX", _) => MigrationOp {
            statement: first.to_string(),
            kind: first.to_string(),
            risk: RiskLevel::Low,
            notes: vec!["Maintenance operation — may acquire brief locks".to_string()],
            line,
        },
        _ => MigrationOp {
            statement: stmt.chars().take(60).collect::<String>(),
            kind: first.to_string(),
            risk: RiskLevel::Safe,
            notes: vec![],
            line,
        },
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_analyze(text: &str) -> Result<String, String> {
    let stmts = split_statements(text);
    if stmts.is_empty() {
        return Ok("sql_migrate_tools — analyze\n\nNo SQL statements found.".to_string());
    }

    let ops: Vec<MigrationOp> = stmts
        .iter()
        .map(|(line, stmt)| analyze_statement(*line, stmt))
        .collect();

    let max_risk = ops.iter().map(|o| o.risk.score()).max().unwrap_or(0);
    let overall = match max_risk {
        0 => "SAFE",
        1 => "LOW RISK",
        2 => "MEDIUM RISK",
        3 => "HIGH RISK",
        _ => "CRITICAL RISK",
    };

    let critical: Vec<&MigrationOp> = ops.iter().filter(|o| o.risk.score() >= 4).collect();
    let high: Vec<&MigrationOp> = ops.iter().filter(|o| o.risk.score() == 3).collect();

    let mut out = format!(
        "sql_migrate_tools — analyze\n\
         Overall risk: {} | {} statement(s) | {} critical | {} high-risk\n\n",
        overall,
        ops.len(),
        critical.len(),
        high.len()
    );

    for op in &ops {
        out.push_str(&format!(
            "  [{}] line {}: {}\n",
            op.risk.label(),
            op.line,
            op.statement
        ));
        for note in &op.notes {
            out.push_str(&format!("       ↳ {}\n", note));
        }
    }

    Ok(out)
}

fn action_risk(text: &str) -> Result<String, String> {
    let stmts = split_statements(text);
    let ops: Vec<MigrationOp> = stmts
        .iter()
        .map(|(line, stmt)| analyze_statement(*line, stmt))
        .collect();

    let risky: Vec<&MigrationOp> = ops.iter().filter(|o| o.risk.score() >= 2).collect();

    if risky.is_empty() {
        return Ok(
            "sql_migrate_tools — risk\n\nNo medium/high/critical risk operations found."
                .to_string(),
        );
    }

    let mut out = format!(
        "sql_migrate_tools — risk\n\
         {} risky operation(s) found:\n\n",
        risky.len()
    );

    for op in &risky {
        out.push_str(&format!(
            "[{}] line {}: {}\n",
            op.risk.label(),
            op.line,
            op.statement
        ));
        for note in &op.notes {
            out.push_str(&format!("  ↳ {}\n", note));
        }
        out.push('\n');
    }

    Ok(out)
}

fn action_ops(text: &str) -> Result<String, String> {
    let stmts = split_statements(text);
    let ops: Vec<MigrationOp> = stmts
        .iter()
        .map(|(line, stmt)| analyze_statement(*line, stmt))
        .collect();

    // Group by kind
    let mut kinds: Vec<(String, usize)> = Vec::new();
    for op in &ops {
        if let Some(entry) = kinds.iter_mut().find(|(k, _)| k == &op.kind) {
            entry.1 += 1;
        } else {
            kinds.push((op.kind.clone(), 1));
        }
    }

    let mut out = format!(
        "sql_migrate_tools — ops\n\
         {} statement(s) in {} operation type(s):\n\n",
        ops.len(),
        kinds.len()
    );

    for (kind, count) in &kinds {
        out.push_str(&format!("  {}  ×{}\n", kind, count));
    }

    out.push_str("\nDetailed listing:\n");
    for op in &ops {
        out.push_str(&format!(
            "  line {:4}: [{}] {}\n",
            op.line,
            op.risk.label(),
            op.statement
        ));
    }

    Ok(out)
}

fn action_validate(text: &str) -> Result<String, String> {
    let stmts = split_statements(text);
    let ops: Vec<MigrationOp> = stmts
        .iter()
        .map(|(line, stmt)| analyze_statement(*line, stmt))
        .collect();

    let mut warnings: Vec<String> = Vec::new();

    // Check for transaction wrapping
    let has_begin = ops.iter().any(|o| o.kind == "BEGIN");
    let has_commit = ops.iter().any(|o| o.kind == "COMMIT");
    let has_destructive = ops
        .iter()
        .any(|o| matches!(o.risk, RiskLevel::High | RiskLevel::Critical));

    if has_destructive && !has_begin {
        warnings.push(
            "Destructive operations are not wrapped in a transaction — add BEGIN/COMMIT for atomicity".to_string(),
        );
    }
    if has_begin && !has_commit {
        warnings.push("BEGIN without matching COMMIT found".to_string());
    }

    // Collect all notes as warnings
    for op in &ops {
        for note in &op.notes {
            warnings.push(format!("line {}: [{}] {}", op.line, op.kind, note));
        }
    }

    // Check for concurrent index creation inside a transaction (illegal in PostgreSQL)
    let has_concurrent = ops
        .iter()
        .any(|o| o.kind == "CREATE INDEX" && o.risk.score() <= 1);
    if has_concurrent && has_begin {
        warnings.push(
            "CONCURRENTLY index creation cannot run inside a transaction block — move it outside BEGIN/COMMIT".to_string(),
        );
    }

    let verdict = if ops.iter().any(|o| o.risk.score() >= 4) {
        "CRITICAL"
    } else if ops.iter().any(|o| o.risk.score() >= 3) {
        "HIGH RISK"
    } else if warnings.is_empty() {
        "VALID"
    } else {
        "WARNINGS"
    };

    let mut out = format!(
        "sql_migrate_tools — validate\n\
         Status: {} | {} statement(s) | {} warning(s)\n",
        verdict,
        ops.len(),
        warnings.len()
    );

    if warnings.is_empty() {
        out.push_str("\nNo issues found.");
    } else {
        out.push('\n');
        for w in &warnings {
            out.push_str(&format!("  WARNING: {}\n", w));
        }
    }

    Ok(out)
}
