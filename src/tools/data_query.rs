use rusqlite::{params, Connection, types::Value as SqlValue};
use serde_json::Value;
use std::path::PathBuf;

pub async fn query_data(args: &Value) -> Result<String, String> {
    let sql = args.get("sql").and_then(|v| v.as_str()).ok_or("Missing 'sql' argument")?;
    let path_str = args.get("path").and_then(|v| v.as_str()).ok_or("Missing 'path' argument")?;
    let path = PathBuf::from(path_str);

    if !path.exists() {
        return Err(format!("File not found: {:?}", path));
    }

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    match ext.as_str() {
        "db" | "sqlite" | "sqlite3" => {
            query_sqlite(&path, sql)
        }
        "csv" => {
            query_csv(&path, sql)
        }
        "json" => {
            query_json(&path, sql)
        }
        _ => Err(format!("Unsupported file extension for SQL query: .{}", ext))
    }
}

fn query_sqlite(path: &PathBuf, sql: &str) -> Result<String, String> {
    let conn = Connection::open(path).map_err(|e| format!("Failed to open database: {}", e))?;
    execute_and_format(&conn, sql)
}

fn query_csv(path: &PathBuf, sql: &str) -> Result<String, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read CSV: {}", e))?;
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Err("CSV file is empty".into());
    }

    let conn = Connection::open_in_memory().map_err(|e| format!("Failed to create in-memory DB: {}", e))?;
    
    let header = lines[0];
    let delimiter = if header.contains(',') { "," } else { "\t" };
    // Simple split-based column extraction. Handles quotes poorly but works for standard CSVs.
    let cols: Vec<String> = header.split(delimiter)
        .map(|s| s.trim().replace(" ", "_").replace("\"", ""))
        .collect();

    let mut create_sql = format!("CREATE TABLE source (");
    for (i, col) in cols.iter().enumerate() {
        create_sql.push_str(&format!("{} TEXT", col));
        if i < cols.len() - 1 { create_sql.push_str(", "); }
    }
    create_sql.push_str(")");

    conn.execute(&create_sql, []).map_err(|e| format!("Failed to create table: {}", e))?;

    let placeholders = vec!["?"; cols.len()].join(",");
    let insert_sql = format!("INSERT INTO source VALUES ({})", placeholders);

    for line in lines.iter().skip(1) {
        let vals: Vec<&str> = line.split(delimiter).map(|s| s.trim()).collect();
        if vals.len() == cols.len() {
             // Convert vals to ToSql
             let mut stmt = conn.prepare(&insert_sql).map_err(|e| e.to_string())?;
             stmt.execute(rusqlite::params_from_iter(vals)).ok();
        }
    }

    execute_and_format(&conn, sql)
}

fn query_json(path: &PathBuf, sql: &str) -> Result<String, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read JSON: {}", e))?;
    let json: Value = serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    let arr = if let Some(a) = json.as_array() {
        a
    } else {
        return Err("JSON must be an array of objects for SQL query".into());
    };

    if arr.is_empty() {
        return Err("JSON array is empty".into());
    }

    let first = arr[0].as_object().ok_or("JSON must be an array of objects")?;
    let cols: Vec<String> = first.keys().cloned().collect();

    let conn = Connection::open_in_memory().map_err(|e| format!("Failed to create in-memory DB: {}", e))?;
    
    let mut create_sql = format!("CREATE TABLE source (");
    for (i, col) in cols.iter().enumerate() {
        create_sql.push_str(&format!("{} TEXT", col));
        if i < cols.len() - 1 { create_sql.push_str(", "); }
    }
    create_sql.push_str(")");

    conn.execute(&create_sql, []).map_err(|e| format!("Failed to create table: {}", e))?;

    let placeholders = vec!["?"; cols.len()].join(",");
    let insert_sql = format!("INSERT INTO source VALUES ({})", placeholders);

    for item in arr {
        if let Some(obj) = item.as_object() {
            let mut vals = Vec::new();
            for col in &cols {
                vals.push(obj.get(col).map(|v| v.to_string()).unwrap_or_default());
            }
            let mut stmt = conn.prepare(&insert_sql).map_err(|e| e.to_string())?;
            stmt.execute(rusqlite::params_from_iter(vals)).ok();
        }
    }

    execute_and_format(&conn, sql)
}

fn execute_and_format(conn: &Connection, sql: &str) -> Result<String, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| format!("SQL Prepare Error: {}", e))?;
    let col_count = stmt.column_count();
    let col_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();

    let mut rows = stmt.query([]).map_err(|e| format!("SQL Query Error: {}", e))?;
    
    let mut out = String::new();
    // Header
    for name in &col_names {
        out.push_str(&format!("{:<15} ", name));
    }
    out.push_str("\n");
    out.push_str(&"-".repeat(col_names.len() * 16));
    out.push_str("\n");

    let mut count = 0;
    while let Some(row) = rows.next().map_err(|e| format!("SQL Row Error: {}", e))? {
        for i in 0..col_count {
            let val: SqlValue = row.get(i).unwrap_or(SqlValue::Null);
            let val_str = match val {
                SqlValue::Null => "NULL".into(),
                SqlValue::Integer(i) => i.to_string(),
                SqlValue::Real(f) => f.to_string(),
                SqlValue::Text(s) => s,
                SqlValue::Blob(_) => "<BLOB>".into(),
            };
            out.push_str(&format!("{:<15} ", val_str));
        }
        out.push_str("\n");
        count += 1;
        if count >= 100 {
            out.push_str("\n... (truncated to 100 rows) ...\n");
            break;
        }
    }

    if count == 0 {
        out.push_str("(No rows returned)\n");
    } else {
        out.push_str(&format!("\nTotal rows returned: {}\n", count));
    }

    Ok(out)
}
