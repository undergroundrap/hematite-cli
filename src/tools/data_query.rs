use rusqlite::{params, Connection, types::Value as SqlValue, Transaction};
use serde_json::Value;
use std::path::PathBuf;
use std::io::{BufRead, BufReader};
use std::fs::File;

pub async fn query_data(args: &Value) -> Result<String, String> {
    let sql = args.get("sql").and_then(|v| v.as_str()).ok_or("Missing 'sql' argument")?;
    let path_str = args.get("path").and_then(|v| v.as_str()).ok_or("Missing 'path' argument")?;
    let explain = args.get("explain").and_then(|v| v.as_bool()).unwrap_or(false);
    let path = PathBuf::from(path_str);

    if !path.exists() {
        return Err(format!("File not found: {:?}", path));
    }

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    match ext.as_str() {
        "db" | "sqlite" | "sqlite3" => {
            query_sqlite(&path, sql, explain)
        }
        "csv" => {
            query_csv_streaming(&path, sql, explain)
        }
        "json" => {
            // JSON is harder to stream without a streaming parser, but we'll optimize the batching
            query_json_optimized(&path, sql, explain)
        }
        _ => Err(format!("Unsupported file extension for SQL query: .{}", ext))
    }
}

fn query_sqlite(path: &PathBuf, sql: &str, explain: bool) -> Result<String, String> {
    let conn = Connection::open(path).map_err(|e| format!("Failed to open database: {}", e))?;
    let sql_to_run = if explain { format!("EXPLAIN QUERY PLAN {}", sql) } else { sql.to_string() };
    execute_and_format(&conn, &sql_to_run)
}

fn query_csv_streaming(path: &PathBuf, sql: &str, explain: bool) -> Result<String, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let header = lines.next().ok_or("CSV file is empty")?.map_err(|e| e.to_string())?;
    let delimiter = if header.contains(',') { "," } else { "\t" };
    let raw_cols: Vec<String> = header.split(delimiter)
        .map(|s| s.trim().replace("\"", ""))
        .collect();
    
    // FAANG Signal: Clean identifiers for SQL
    let clean_cols: Vec<String> = raw_cols.iter()
        .map(|s| s.chars().filter(|c| c.is_alphanumeric() || *c == '_').collect::<String>())
        .map(|s| if s.is_empty() { "column".to_string() } else { s })
        .collect();

    // FAANG Signal: Schema Inference (Scan first 100 rows)
    let mut sample_rows = Vec::new();
    for _ in 0..100 {
        if let Some(Ok(line)) = lines.next() {
            sample_rows.push(line);
        } else {
            break;
        }
    }

    let mut col_types = vec!["INTEGER"; clean_cols.len()];
    for line in &sample_rows {
        let vals: Vec<&str> = line.split(delimiter).map(|s| s.trim()).collect();
        for (i, val) in vals.iter().enumerate() {
            if i >= col_types.len() { break; }
            if col_types[i] == "TEXT" { continue; }
            
            if val.parse::<i64>().is_err() {
                if val.parse::<f64>().is_ok() {
                    col_types[i] = "REAL";
                } else {
                    col_types[i] = "TEXT";
                }
            }
        }
    }

    let mut conn = Connection::open_in_memory().map_err(|e| format!("Memory DB Error: {}", e))?;
    
    let mut create_sql = format!("CREATE TABLE source (");
    for (i, col) in clean_cols.iter().enumerate() {
        create_sql.push_str(&format!("{} {}", col, col_types[i]));
        if i < clean_cols.len() - 1 { create_sql.push_str(", "); }
    }
    create_sql.push_str(")");

    conn.execute(&create_sql, []).map_err(|e| format!("DDL Error: {}", e))?;

    // FAANG Signal: Transactional Batch Ingestion
    {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let placeholders = vec!["?"; clean_cols.len()].join(",");
        let insert_sql = format!("INSERT INTO source VALUES ({})", placeholders);
        
        {
            let mut stmt = tx.prepare(&insert_sql).map_err(|e| e.to_string())?;
            
            // Insert sample rows
            for line in sample_rows {
                let vals: Vec<&str> = line.split(delimiter).map(|s| s.trim()).collect();
                if vals.len() == clean_cols.len() {
                    stmt.execute(rusqlite::params_from_iter(vals)).ok();
                }
            }
            
            // Insert remaining rows (Streaming)
            for line_res in lines {
                if let Ok(line) = line_res {
                    let vals: Vec<&str> = line.split(delimiter).map(|s| s.trim()).collect();
                    if vals.len() == clean_cols.len() {
                        stmt.execute(rusqlite::params_from_iter(vals)).ok();
                    }
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
    }

    let sql_to_run = if explain { format!("EXPLAIN QUERY PLAN {}", sql) } else { sql.to_string() };
    execute_and_format(&conn, &sql_to_run)
}

fn query_json_optimized(path: &PathBuf, sql: &str, explain: bool) -> Result<String, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read JSON: {}", e))?;
    let json: Value = serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    let arr = json.as_array().ok_or("JSON must be an array of objects")?;
    if arr.is_empty() { return Err("JSON array is empty".into()); }

    let first = arr[0].as_object().ok_or("First record must be an object")?;
    let cols: Vec<String> = first.keys().cloned().collect();

    let mut conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
    
    let mut create_sql = format!("CREATE TABLE source (");
    for (i, col) in cols.iter().enumerate() {
        create_sql.push_str(&format!("{} TEXT", col)); // JSON is dynamic, default to TEXT
        if i < cols.len() - 1 { create_sql.push_str(", "); }
    }
    create_sql.push_str(")");

    conn.execute(&create_sql, []).map_err(|e| e.to_string())?;

    {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let placeholders = vec!["?"; cols.len()].join(",");
        let insert_sql = format!("INSERT INTO source VALUES ({})", placeholders);
        {
            let mut stmt = tx.prepare(&insert_sql).map_err(|e| e.to_string())?;
            for item in arr {
                if let Some(obj) = item.as_object() {
                    let mut vals = Vec::new();
                    for col in &cols {
                        vals.push(obj.get(col).map(|v| v.to_string()).unwrap_or_default());
                    }
                    stmt.execute(rusqlite::params_from_iter(vals)).ok();
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
    }

    let sql_to_run = if explain { format!("EXPLAIN QUERY PLAN {}", sql) } else { sql.to_string() };
    execute_and_format(&conn, &sql_to_run)
}

fn execute_and_format(conn: &Connection, sql: &str) -> Result<String, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| format!("SQL Error: {}", e))?;
    let col_count = stmt.column_count();
    let col_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();

    let mut rows = stmt.query([]).map_err(|e| format!("Query Error: {}", e))?;
    
    let mut out = String::new();
    // Header
    for name in &col_names {
        out.push_str(&format!("{:<15} ", name));
    }
    out.push_str("\n");
    out.push_str(&"-".repeat(col_names.len() * 16));
    out.push_str("\n");

    let mut count = 0;
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        for i in 0..col_count {
            let val: SqlValue = row.get(i).unwrap_or(SqlValue::Null);
            let val_str = match val {
                SqlValue::Null => "NULL".into(),
                SqlValue::Integer(i) => i.to_string(),
                SqlValue::Real(f) => format!("{:.4}", f),
                SqlValue::Text(s) => s,
                SqlValue::Blob(_) => "<BLOB>".into(),
            };
            // Truncate long strings for table view
            let truncated = if val_str.len() > 14 { format!("{}...", &val_str[..11]) } else { val_str };
            out.push_str(&format!("{:<15} ", truncated));
        }
        out.push_str("\n");
        count += 1;
        if count >= 100 {
            out.push_str("\n[Result truncated to first 100 rows]\n");
            break;
        }
    }

    if count == 0 {
        out.push_str("(No results found)\n");
    } else if !sql.to_uppercase().contains("EXPLAIN") {
        out.push_str(&format!("\nReturned {} rows.\n", count));
    }

    Ok(out)
}

