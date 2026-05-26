use rusqlite::{Connection, OpenFlags};

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("tables");

    match action {
        "tables" => tables(args),
        "schema" => schema(args),
        "query" => query(args),
        "info" => info(args),
        "export" => export(args),
        other => Err(format!(
            "sqlite_tools: unknown action '{other}'. Valid: tables, schema, query, info, export"
        )),
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn open_db(args: &serde_json::Value) -> Result<Connection, String> {
    let file = args
        .get("file")
        .or_else(|| args.get("db"))
        .or_else(|| args.get("database"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("sqlite_tools: 'file' is required (path to .db or .sqlite file)")?;

    let path = if std::path::Path::new(file).is_absolute() {
        std::path::PathBuf::from(file)
    } else {
        let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
            std::path::PathBuf::from(r)
        } else {
            crate::tools::file_ops::workspace_root()
        };
        root.join(file)
    };

    if !path.exists() {
        return Err(format!("sqlite_tools: file not found: {}", path.display()));
    }

    Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("sqlite_tools: cannot open '{}': {e}", path.display()))
}

fn is_safe_sql(sql: &str) -> Result<(), String> {
    let normalized = sql.trim().to_uppercase();
    // Allow SELECT and read-only PRAGMAs; block everything else.
    let allowed = normalized.starts_with("SELECT")
        || normalized.starts_with("EXPLAIN")
        || normalized.starts_with("WITH ") // CTEs
        || normalized.starts_with("PRAGMA ");
    if !allowed {
        return Err(format!(
            "sqlite_tools: only SELECT, EXPLAIN, WITH, and PRAGMA statements are allowed \
             (got: {}...)",
            &normalized[..normalized.len().min(20)]
        ));
    }
    // Reject any write keywords that might appear inside allowed prefixes
    for keyword in &[
        "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER", "ATTACH", "DETACH", "VACUUM",
        "REINDEX", "ANALYZE",
    ] {
        // Check that the keyword appears as a standalone word (not part of a column name)
        let pat = format!(" {} ", keyword);
        let pat2 = format!("\t{}", keyword);
        let pat3 = format!(";{}", keyword);
        if normalized.contains(&pat)
            || normalized.contains(&pat2)
            || normalized.contains(&pat3)
            || normalized.starts_with(keyword)
        {
            return Err(format!(
                "sqlite_tools: '{keyword}' is not allowed — read-only mode only"
            ));
        }
    }
    Ok(())
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn tables(args: &serde_json::Value) -> Result<String, String> {
    let conn = open_db(args)?;

    let mut stmt = conn
        .prepare(
            "SELECT type, name, sql FROM sqlite_master \
             WHERE type IN ('table','view','index','trigger') \
             ORDER BY type, name",
        )
        .map_err(|e| format!("sqlite_tools: query error: {e}"))?;

    let mut tables: Vec<(String, String)> = Vec::new();
    let mut views: Vec<String> = Vec::new();
    let mut indexes: Vec<String> = Vec::new();

    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("sqlite_tools: row error: {e}"))?;

    for row in rows {
        let (kind, name) = row.map_err(|e| format!("sqlite_tools: {e}"))?;
        match kind.as_str() {
            "table" => {
                // Get row count for each table
                let count: i64 = conn
                    .query_row(&format!("SELECT COUNT(*) FROM \"{name}\""), [], |r| {
                        r.get(0)
                    })
                    .unwrap_or(-1);
                tables.push((name, count.to_string()));
            }
            "view" => views.push(name),
            "index" => indexes.push(name),
            _ => {}
        }
    }

    let db_name = args
        .get("file")
        .or_else(|| args.get("db"))
        .and_then(|v| v.as_str())
        .unwrap_or("database");

    let mut out = format!("SQLITE TABLES: {db_name}\n{}\n", "─".repeat(50));
    if tables.is_empty() {
        out.push_str("No tables found.\n");
    } else {
        out.push_str(&format!("{:<40} {:>10}\n", "Table", "Rows"));
        out.push_str(&format!("{}\n", "─".repeat(52)));
        for (name, count) in &tables {
            out.push_str(&format!("{:<40} {:>10}\n", name, count));
        }
        out.push_str(&format!("\n{} table(s)\n", tables.len()));
    }
    if !views.is_empty() {
        out.push_str(&format!("\nViews ({}):\n", views.len()));
        for v in &views {
            out.push_str(&format!("  {v}\n"));
        }
    }
    if !indexes.is_empty() {
        out.push_str(&format!("\nIndexes: {}\n", indexes.len()));
    }
    Ok(out)
}

fn schema(args: &serde_json::Value) -> Result<String, String> {
    let conn = open_db(args)?;
    let table = args.get("table").and_then(|v| v.as_str());

    let db_name = args
        .get("file")
        .or_else(|| args.get("db"))
        .and_then(|v| v.as_str())
        .unwrap_or("database");

    let mut out = format!("SQLITE SCHEMA: {db_name}\n{}\n", "─".repeat(50));

    if let Some(tbl) = table {
        let sql: Option<String> = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name = ?1",
                [tbl],
                |r| r.get(0),
            )
            .ok();
        match sql {
            Some(s) => {
                out.push_str(&format!("{s}\n"));
                // Also show column info via PRAGMA
                out.push_str(&format!("\nColumns for \"{tbl}\":\n"));
                let mut stmt = conn
                    .prepare(&format!("PRAGMA table_info(\"{tbl}\")"))
                    .map_err(|e| format!("sqlite_tools: pragma error: {e}"))?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,    // cid
                            row.get::<_, String>(1)?, // name
                            row.get::<_, String>(2)?, // type
                            row.get::<_, i64>(3)?,    // notnull
                            row.get::<_, i64>(5)?,    // pk
                        ))
                    })
                    .map_err(|e| format!("sqlite_tools: {e}"))?;
                out.push_str(&format!(
                    "  {:<5} {:<30} {:<15} {:<8} {}\n",
                    "cid", "name", "type", "not_null", "pk"
                ));
                out.push_str(&format!("  {}\n", "─".repeat(65)));
                for row in rows {
                    let (cid, name, typ, notnull, pk) =
                        row.map_err(|e| format!("sqlite_tools: {e}"))?;
                    out.push_str(&format!(
                        "  {:<5} {:<30} {:<15} {:<8} {}\n",
                        cid,
                        name,
                        typ,
                        if notnull == 1 { "YES" } else { "no" },
                        if pk > 0 {
                            format!("PK{pk}")
                        } else {
                            String::new()
                        }
                    ));
                }
            }
            None => {
                return Err(format!("sqlite_tools: table '{tbl}' not found"));
            }
        }
    } else {
        // Show all tables
        let mut stmt = conn
            .prepare("SELECT type, name, sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY type, name")
            .map_err(|e| format!("sqlite_tools: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| format!("sqlite_tools: {e}"))?;
        for row in rows {
            let (kind, name, sql) = row.map_err(|e| format!("sqlite_tools: {e}"))?;
            out.push_str(&format!("-- {kind}: {name}\n{sql};\n\n"));
        }
    }
    Ok(out)
}

fn query(args: &serde_json::Value) -> Result<String, String> {
    let sql = args
        .get("sql")
        .or_else(|| args.get("query"))
        .and_then(|v| v.as_str())
        .ok_or("sqlite_tools query: 'sql' is required")?;

    is_safe_sql(sql)?;

    let conn = open_db(args)?;
    let max_rows = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("sqlite_tools: SQL error: {e}"))?;

    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();

    // Collect rows
    let mut rows_data: Vec<Vec<String>> = Vec::new();
    let rows = stmt
        .query([])
        .map_err(|e| format!("sqlite_tools: query error: {e}"))?;

    let mut rows = rows;
    let mut truncated = false;
    loop {
        match rows.next() {
            Ok(Some(row)) => {
                if rows_data.len() >= max_rows {
                    truncated = true;
                    break;
                }
                let vals: Vec<String> = (0..col_count)
                    .map(|i| {
                        use rusqlite::types::ValueRef;
                        match row.get_ref_unwrap(i) {
                            ValueRef::Null => "NULL".to_string(),
                            ValueRef::Integer(n) => n.to_string(),
                            ValueRef::Real(f) => format!("{f}"),
                            ValueRef::Text(s) => {
                                let s = String::from_utf8_lossy(s).to_string();
                                if s.len() > 80 {
                                    format!("{}...", &s[..80])
                                } else {
                                    s
                                }
                            }
                            ValueRef::Blob(b) => format!("<blob {} bytes>", b.len()),
                        }
                    })
                    .collect();
                rows_data.push(vals);
            }
            Ok(None) => break,
            Err(e) => return Err(format!("sqlite_tools: row error: {e}")),
        }
    }

    // Calculate column widths
    let mut widths: Vec<usize> = col_names.iter().map(|n| n.len().max(4)).collect();
    for row in &rows_data {
        for (i, val) in row.iter().enumerate() {
            widths[i] = widths[i].max(val.len().min(40));
        }
    }

    let mut out = format!("SQLITE QUERY\n{}\n", "─".repeat(50));
    out.push_str(&format!("SQL: {sql}\n\n"));

    // Header
    for (i, name) in col_names.iter().enumerate() {
        out.push_str(&format!("{:<width$}  ", name, width = widths[i]));
    }
    out.push('\n');
    for w in &widths {
        out.push_str(&format!("{:<width$}  ", "─".repeat(*w), width = w));
    }
    out.push('\n');

    // Rows
    for row in &rows_data {
        for (i, val) in row.iter().enumerate() {
            out.push_str(&format!("{:<width$}  ", val, width = widths[i]));
        }
        out.push('\n');
    }

    out.push_str(&format!("\n{} row(s)", rows_data.len()));
    if truncated {
        out.push_str(&format!(
            " (limit {max_rows} — use 'limit' arg to increase)"
        ));
    }
    out.push('\n');
    Ok(out)
}

fn info(args: &serde_json::Value) -> Result<String, String> {
    let path_str = args
        .get("file")
        .or_else(|| args.get("db"))
        .or_else(|| args.get("database"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("sqlite_tools: 'file' is required")?;

    let path = if std::path::Path::new(path_str).is_absolute() {
        std::path::PathBuf::from(path_str)
    } else {
        let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
            std::path::PathBuf::from(r)
        } else {
            crate::tools::file_ops::workspace_root()
        };
        root.join(path_str)
    };

    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    let conn = open_db(args)?;

    let page_size: i64 = conn
        .query_row("PRAGMA page_size", [], |r| r.get(0))
        .unwrap_or(0);
    let page_count: i64 = conn
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .unwrap_or(0);
    let encoding: String = conn
        .query_row("PRAGMA encoding", [], |r| r.get(0))
        .unwrap_or_else(|_| "unknown".to_string());
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap_or_else(|_| "unknown".to_string());
    let user_version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap_or(0);
    let sqlite_version: String = conn
        .query_row("SELECT sqlite_version()", [], |r| r.get(0))
        .unwrap_or_else(|_| "unknown".to_string());

    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let mut out = format!(
        "SQLITE INFO: {}\n{}\n",
        path.file_name().unwrap_or_default().to_string_lossy(),
        "─".repeat(50)
    );
    out.push_str(&format!("File size     : {}\n", human_size(file_size)));
    out.push_str(&format!("SQLite version: {sqlite_version}\n"));
    out.push_str(&format!("Page size     : {page_size} bytes\n"));
    out.push_str(&format!("Page count    : {page_count}\n"));
    out.push_str(&format!("Encoding      : {encoding}\n"));
    out.push_str(&format!("Journal mode  : {journal_mode}\n"));
    out.push_str(&format!("User version  : {user_version}\n"));
    out.push_str(&format!("Tables        : {table_count}\n"));
    Ok(out)
}

fn export(args: &serde_json::Value) -> Result<String, String> {
    let table = args
        .get("table")
        .and_then(|v| v.as_str())
        .ok_or("sqlite_tools export: 'table' is required")?;
    let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("csv");
    let max_rows = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(1000) as usize;

    let conn = open_db(args)?;

    let sql = format!("SELECT * FROM \"{table}\" LIMIT {}", max_rows + 1);
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("sqlite_tools export: {e}"))?;

    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("col").to_string())
        .collect();

    let mut rows_data: Vec<Vec<String>> = Vec::new();
    let rows = stmt
        .query([])
        .map_err(|e| format!("sqlite_tools export: {e}"))?;
    let mut rows = rows;
    let mut truncated = false;
    loop {
        match rows.next() {
            Ok(Some(row)) => {
                if rows_data.len() >= max_rows {
                    truncated = true;
                    break;
                }
                let vals: Vec<String> = (0..col_count)
                    .map(|i| {
                        use rusqlite::types::ValueRef;
                        match row.get_ref_unwrap(i) {
                            ValueRef::Null => String::new(),
                            ValueRef::Integer(n) => n.to_string(),
                            ValueRef::Real(f) => format!("{f}"),
                            ValueRef::Text(s) => String::from_utf8_lossy(s).to_string(),
                            ValueRef::Blob(b) => format!("<blob {} bytes>", b.len()),
                        }
                    })
                    .collect();
                rows_data.push(vals);
            }
            Ok(None) => break,
            Err(e) => return Err(format!("sqlite_tools export: {e}")),
        }
    }

    let mut out = format!("SQLITE EXPORT: {table} ({format})\n{}\n", "─".repeat(50));

    match format {
        "json" => {
            let json_rows: Vec<serde_json::Value> = rows_data
                .iter()
                .map(|row| {
                    let mut map = serde_json::Map::new();
                    for (i, val) in row.iter().enumerate() {
                        map.insert(col_names[i].clone(), serde_json::Value::String(val.clone()));
                    }
                    serde_json::Value::Object(map)
                })
                .collect();
            out.push_str(
                &serde_json::to_string_pretty(&serde_json::Value::Array(json_rows))
                    .unwrap_or_default(),
            );
            out.push('\n');
        }
        _ => {
            // CSV
            fn csv_field(s: &str) -> String {
                if s.contains(',') || s.contains('"') || s.contains('\n') {
                    format!("\"{}\"", s.replace('"', "\"\""))
                } else {
                    s.to_string()
                }
            }
            out.push_str(
                &col_names
                    .iter()
                    .map(|n| csv_field(n))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            out.push('\n');
            for row in &rows_data {
                out.push_str(
                    &row.iter()
                        .map(|v| csv_field(v))
                        .collect::<Vec<_>>()
                        .join(","),
                );
                out.push('\n');
            }
        }
    }

    if truncated {
        out.push_str(&format!(
            "\n(truncated to {max_rows} rows — use 'limit' arg to change)\n"
        ));
    }
    Ok(out)
}
