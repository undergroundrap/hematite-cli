use crate::agent::lsp::manager::LspManager;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

fn adjust_position(root: &PathBuf, path: &str, line: u32, character: u32) -> u32 {
    if character > 0 {
        return character;
    }

    let abs_path = root.join(path);
    if let Ok(content) = std::fs::read_to_string(&abs_path) {
        let lines: Vec<&str> = content.lines().collect();
        if let Some(l) = lines.get(line as usize) {
            if let Some(first) = l.find(|c: char| !c.is_whitespace()) {
                return first as u32;
            }
        }
    }
    character
}

pub async fn lsp_definitions(
    lsp: Arc<Mutex<LspManager>>,
    path: String,
    line: u32,
    character: u32,
) -> Result<String, String> {
    let mut manager = lsp.lock().await;
    if manager.get_client_for_path(&path).is_none() {
        let _ = manager.start_servers().await;
    }
    let _ = manager.ensure_opened(&path).await;
    let client = manager
        .get_client_for_path(&path)
        .ok_or_else(|| "No Language Server active for this file type.".to_string())?;

    let uri = manager.resolve_uri(&path);
    let character = adjust_position(&manager.workspace_root, &path, line, character);
    let params = json!({
        "textDocument": { "uri": uri },
        "position": { "line": line, "character": character }
    });

    let mut result = client
        .call("textDocument/definition", params.clone())
        .await?;

    // Index Recovery: Try line-1 if line N is empty (handling 1-indexed slips)
    if result.is_null() && line > 0 {
        let mut fallback_params = params.clone();
        fallback_params["position"]["line"] = json!(line - 1);
        let fallback_char = adjust_position(&manager.workspace_root, &path, line - 1, 0);
        fallback_params["position"]["character"] = json!(fallback_char);

        if let Ok(res) = client
            .call("textDocument/definition", fallback_params)
            .await
        {
            if !res.is_null() && !res.get("uri").is_none() {
                result = res;
            }
        }
    }

    format_location_result(result)
}

pub async fn lsp_references(
    lsp: Arc<Mutex<LspManager>>,
    path: String,
    line: u32,
    character: u32,
) -> Result<String, String> {
    let mut manager = lsp.lock().await;
    if manager.get_client_for_path(&path).is_none() {
        let _ = manager.start_servers().await;
    }
    let _ = manager.ensure_opened(&path).await;
    let client = manager
        .get_client_for_path(&path)
        .ok_or_else(|| "No Language Server active for this file type.".to_string())?;

    let uri = manager.resolve_uri(&path);
    let character = adjust_position(&manager.workspace_root, &path, line, character);
    let params = json!({
        "textDocument": { "uri": uri },
        "position": { "line": line, "character": character },
        "context": { "includeDeclaration": true }
    });

    let result = client.call("textDocument/references", params).await?;
    format_location_result(result)
}

pub async fn lsp_hover(
    lsp: Arc<Mutex<LspManager>>,
    path: String,
    line: u32,
    character: u32,
) -> Result<String, String> {
    let mut manager = lsp.lock().await;
    if manager.get_client_for_path(&path).is_none() {
        let _ = manager.start_servers().await;
    }
    let _ = manager.ensure_opened(&path).await;
    let client = manager
        .get_client_for_path(&path)
        .ok_or_else(|| "No Language Server active for this file type.".to_string())?;

    let uri = manager.resolve_uri(&path);
    let character = adjust_position(&manager.workspace_root, &path, line, character);
    let params = json!({
        "textDocument": { "uri": uri },
        "position": { "line": line, "character": character }
    });

    let mut result = client.call("textDocument/hover", params.clone()).await?;

    // Index Recovery: If line N returns nothing, try line N-1 (handling 1-indexed slips)
    if result.is_null() && line > 0 {
        let mut fallback_params = params.clone();
        fallback_params["position"]["line"] = json!(line - 1);
        // Also re-adjust character for the new line
        let fallback_char = adjust_position(&manager.workspace_root, &path, line - 1, 0);
        fallback_params["position"]["character"] = json!(fallback_char);

        if let Ok(res) = client.call("textDocument/hover", fallback_params).await {
            if !res.is_null() {
                result = res;
            }
        }
    }

    if result.is_null() {
        return Ok("No hover information available.".to_string());
    }

    let contents = result.get("contents").ok_or("Invalid hover response")?;
    // Handle both String and MarkupContent/Array
    if let Some(s) = contents.as_str() {
        Ok(s.to_string())
    } else if let Some(obj) = contents.get("value") {
        Ok(obj.as_str().unwrap_or("").to_string())
    } else {
        Ok(serde_json::to_string_pretty(contents).unwrap_or_default())
    }
}

fn format_location_result(res: Value) -> Result<String, String> {
    if res.is_null() {
        return Ok("No results found.".to_string());
    }

    let mut output = Vec::new();
    if let Some(arr) = res.as_array() {
        for loc in arr {
            output.push(format_location(loc));
        }
    } else {
        output.push(format_location(&res));
    }

    Ok(output.join("\n"))
}

fn format_location(loc: &Value) -> String {
    let uri = loc.get("uri").and_then(|v| v.as_str()).unwrap_or("unknown");
    let range = loc.get("range");
    let start = range.and_then(|r| r.get("start"));
    let line = start
        .and_then(|s| s.get("line").and_then(|v| v.as_u64()))
        .unwrap_or(0);
    let col = start
        .and_then(|s| s.get("character").and_then(|v| v.as_u64()))
        .unwrap_or(0);

    format!("{}:{}:{}", uri.replace("file:///", ""), line, col)
}

pub async fn lsp_search_symbol(
    lsp: Arc<Mutex<LspManager>>,
    query: String,
) -> Result<String, String> {
    let mut manager = lsp.lock().await;
    // Default to rust if nothing started yet for simple queries
    if manager.clients.is_empty() {
        let _ = manager.start_servers().await;
    }

    let client = manager
        .get_client("rust")
        .ok_or_else(|| "No Language Server active for workspace symbol search.".to_string())?;

    let params = json!({
        "query": query
    });

    let result = client.call("workspace/symbol", params).await?;
    if result.is_null() {
        return Ok("No symbols found matching your query.".to_string());
    }

    let mut output = Vec::new();
    if let Some(arr) = result.as_array() {
        for sym in arr {
            let name = sym
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let location = sym.get("location");
            if let Some(loc) = location {
                let formatted = format_location(loc);
                output.push(format!("{} -> {}", name, formatted));
            }
        }
    }

    if output.is_empty() {
        Ok("No symbols found matching your query.".to_string())
    } else {
        Ok(output.join("\n"))
    }
}

pub async fn lsp_rename_symbol(
    lsp: Arc<Mutex<LspManager>>,
    path: String,
    line: u32,
    character: u32,
    new_name: String,
) -> Result<String, String> {
    let mut manager = lsp.lock().await;
    let _ = manager.ensure_opened(&path).await;
    let client = manager
        .get_client_for_path(&path)
        .ok_or_else(|| "No LSP client for this file.".to_string())?;

    let uri = manager.resolve_uri(&path);
    let character = adjust_position(&manager.workspace_root, &path, line, character);
    let params = json!({
        "textDocument": { "uri": uri },
        "position": { "line": line, "character": character },
        "newName": new_name
    });

    let result = client.call("textDocument/rename", params).await?;
    if result.is_null() {
        return Ok("Rename failed or no changes returned.".to_string());
    }

    Ok(format!(
        "Rename successful. Workspace edit changes: \n{}",
        serde_json::to_string_pretty(&result).unwrap_or_default()
    ))
}

pub async fn lsp_get_diagnostics(
    lsp: Arc<Mutex<LspManager>>,
    path: String,
) -> Result<String, String> {
    let manager = lsp.lock().await;
    let client = manager
        .get_client_for_path(&path)
        .ok_or_else(|| "No LSP client for this file.".to_string())?;

    let uri = manager.resolve_uri(&path);
    let all_diags = client.diagnostics.lock().await;

    match all_diags.get(&uri) {
        Some(Value::Array(indices)) if !indices.is_empty() => {
            let mut out = format!("Diagnostics for {}:\n", path);
            for diag in indices {
                let msg = diag
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                let severity = diag.get("severity").and_then(|v| v.as_u64()).unwrap_or(1);
                let range = diag.get("range");
                let start_line = range
                    .and_then(|r| r.get("start"))
                    .and_then(|s| s.get("line"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let sev_label = match severity {
                    1 => "[ERROR]",
                    2 => "[WARNING]",
                    3 => "[INFO]",
                    _ => "[HINT]",
                };
                out.push_str(&format!("{} Line {}: {}\n", sev_label, start_line + 1, msg));
            }
            Ok(out)
        }
        _ => Ok(format!(
            "No diagnostics (errors/warnings) found for {}.",
            path
        )),
    }
}

pub fn get_lsp_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "lsp_definitions",
            "description": "Find the definition of a symbol at a specific file and position (line/char). \
                             Requires /lsp to be active. Much more precise than grep for code navigation.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to the file" },
                    "line": { "type": "integer", "description": "0-indexed line number" },
                    "character": { "type": "integer", "description": "0-indexed character offset" }
                },
                "required": ["path", "line", "character"]
            }
        }),
        json!({
            "name": "lsp_references",
            "description": "Find all references to a symbol at a specific file and position. \
                             Use this to find where a function or struct is used across the project.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to the file" },
                    "line": { "type": "integer", "description": "0-indexed line number" },
                    "character": { "type": "integer", "description": "0-indexed character offset" }
                },
                "required": ["path", "line", "character"]
            }
        }),
        json!({
            "name": "lsp_hover",
            "description": "Get type information, documentation, and metadata for a symbol at a specific position. \
                             Like hovering your mouse over a symbol in an IDE.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to the file" },
                    "line": { "type": "integer", "description": "0-indexed line number" },
                    "character": { "type": "integer", "description": "0-indexed character offset" }
                },
                "required": ["path", "line", "character"]
            }
        }),
        json!({
            "name": "lsp_search_symbol",
            "description": "Find the location (file/line) of any function, struct, or variable in the entire project workspace. \
                             This is the fastest 'Golden Path' for navigating to a symbol by name.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The name of the symbol to find (e.g. 'initialize_mcp')" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "lsp_rename_symbol",
            "description": "Rename a symbol reliably across the whole project using the Language Server. \
                             This handles all variable/function name changes safely.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to the file containing the symbol" },
                    "line": { "type": "integer", "description": "0-indexed line number" },
                    "character": { "type": "integer", "description": "0-indexed character offset" },
                    "new_name": { "type": "string", "description": "The new name for the symbol" }
                },
                "required": ["path", "line", "character", "new_name"]
            }
        }),
        json!({
            "name": "lsp_get_diagnostics",
            "description": "Get current compiler/linter errors and warnings for a file. \
                             Use this to verify your changes fixed a bug or to find where your code is broken.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to the file" }
                },
                "required": ["path"]
            }
        }),
    ]
}
