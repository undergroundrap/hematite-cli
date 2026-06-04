use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "description": "parse (default) | build | validate | batch"
            },
            "message": {
                "type": "string",
                "description": "Raw JSON-RPC message string to parse or validate"
            },
            "kind": {
                "type": "string",
                "description": "For build: request | notification | response | error"
            },
            "method": {
                "type": "string",
                "description": "Method name for request or notification"
            },
            "id": {
                "type": ["string", "number", "null"],
                "description": "Request/response id — string, number, or null"
            },
            "params": {
                "description": "Params for request or notification — object or array"
            },
            "result": {
                "description": "Result value for a success response"
            },
            "error_code": {
                "type": "integer",
                "description": "Error code integer for error response (e.g. -32600)"
            },
            "error_message": {
                "type": "string",
                "description": "Error message string for error response"
            },
            "error_data": {
                "description": "Optional additional error data"
            },
            "messages": {
                "type": "array",
                "description": "Array of message objects for building a batch"
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("parse");
    match action {
        "parse" | "inspect" | "decode" => do_parse(args),
        "build" | "create" | "generate" => do_build(args),
        "validate" | "check" => do_validate(args),
        "batch" => do_batch(args),
        _ => Ok(format!(
            "Unknown action '{}'. Use: parse, build, validate, batch.",
            action
        )),
    }
}

// ── Message type detection ────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum MsgKind {
    Request,
    Notification,
    SuccessResponse,
    ErrorResponse,
    Batch,
    Unknown,
}

fn detect_kind(v: &Value) -> MsgKind {
    if v.is_array() {
        return MsgKind::Batch;
    }
    let obj = match v.as_object() {
        Some(o) => o,
        None => return MsgKind::Unknown,
    };
    if obj.contains_key("method") {
        if obj.contains_key("id") {
            MsgKind::Request
        } else {
            MsgKind::Notification
        }
    } else if obj.contains_key("result") {
        MsgKind::SuccessResponse
    } else if obj.contains_key("error") {
        MsgKind::ErrorResponse
    } else {
        MsgKind::Unknown
    }
}

fn kind_label(k: &MsgKind) -> &'static str {
    match k {
        MsgKind::Request => "Request",
        MsgKind::Notification => "Notification",
        MsgKind::SuccessResponse => "Success Response",
        MsgKind::ErrorResponse => "Error Response",
        MsgKind::Batch => "Batch",
        MsgKind::Unknown => "Unknown",
    }
}

// ── Standard error codes ──────────────────────────────────────────────────

fn standard_error_name(code: i64) -> Option<&'static str> {
    match code {
        -32700 => Some("Parse error — invalid JSON received"),
        -32600 => Some("Invalid Request — not a valid JSON-RPC object"),
        -32601 => Some("Method not found — method does not exist"),
        -32602 => Some("Invalid params — invalid method parameters"),
        -32603 => Some("Internal error — internal JSON-RPC error"),
        -32099..=-32000 => Some("Server error — implementation-defined"),
        _ => None,
    }
}

// ── Action: parse ──────────────────────────────────────────────────────────

fn load_message(args: &Value) -> Result<Value, String> {
    let raw = args
        .get("message")
        .or_else(|| args.get("text"))
        .or_else(|| args.get("json"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Pass 'message' with a JSON-RPC string.".to_string())?;
    serde_json::from_str(raw).map_err(|e| format!("Invalid JSON: {}", e))
}

fn format_id(id: &Value) -> String {
    match id {
        Value::Null => "null".to_string(),
        Value::String(s) => format!("\"{}\" (string)", s),
        Value::Number(n) => format!("{} (number)", n),
        _ => format!("{} (non-standard type)", id),
    }
}

fn summarize_params(params: &Value) -> String {
    match params {
        Value::Array(arr) => format!(
            "positional array — {} item{}",
            arr.len(),
            if arr.len() == 1 { "" } else { "s" }
        ),
        Value::Object(obj) => format!(
            "named object — {} key{}: {}",
            obj.len(),
            if obj.len() == 1 { "" } else { "s" },
            obj.keys().cloned().collect::<Vec<_>>().join(", ")
        ),
        _ => format!("non-standard: {}", params),
    }
}

fn do_parse(args: &Value) -> Result<String, String> {
    let v = load_message(args)?;
    parse_single(&v)
}

fn parse_single(v: &Value) -> Result<String, String> {
    let kind = detect_kind(v);
    let mut out = format!("JSON-RPC 2.0 — {}\n{}\n\n", kind_label(&kind), "─".repeat(45));

    if let Some(obj) = v.as_object() {
        match obj.get("jsonrpc") {
            Some(ver) => {
                let ver_str = ver.as_str().unwrap_or("(non-string)");
                let ok = if ver_str == "2.0" { "✓" } else { "✗ (expected \"2.0\")" };
                out.push_str(&format!("  jsonrpc : \"{}\" {}\n", ver_str, ok));
            }
            None => {
                out.push_str("  jsonrpc : MISSING ✗\n");
            }
        }

        if let Some(id) = obj.get("id") {
            out.push_str(&format!("  id      : {}\n", format_id(id)));
        }

        if let Some(method) = obj.get("method") {
            out.push_str(&format!(
                "  method  : {}\n",
                method.as_str().unwrap_or("(non-string)")
            ));
        }

        if let Some(params) = obj.get("params") {
            out.push_str(&format!("  params  : {}\n", summarize_params(params)));
        }

        if let Some(result) = obj.get("result") {
            let preview = preview_value(result, 80);
            out.push_str(&format!("  result  : {}\n", preview));
        }

        if let Some(error) = obj.get("error") {
            out.push_str("  error   :\n");
            if let Some(err_obj) = error.as_object() {
                if let Some(code) = err_obj.get("code").and_then(|c| c.as_i64()) {
                    let name = standard_error_name(code)
                        .map(|n| format!(" — {}", n))
                        .unwrap_or_default();
                    out.push_str(&format!("    code    : {}{}\n", code, name));
                } else {
                    out.push_str(&format!(
                        "    code    : {}\n",
                        err_obj.get("code").unwrap_or(&Value::Null)
                    ));
                }
                if let Some(msg) = err_obj.get("message").and_then(|m| m.as_str()) {
                    out.push_str(&format!("    message : \"{}\"\n", msg));
                }
                if let Some(data) = err_obj.get("data") {
                    let preview = preview_value(data, 60);
                    out.push_str(&format!("    data    : {}\n", preview));
                }
            } else {
                out.push_str(&format!("    (non-object: {})\n", error));
            }
        }

        let known = ["jsonrpc", "id", "method", "params", "result", "error"];
        let extras: Vec<_> = obj.keys().filter(|k| !known.contains(&k.as_str())).collect();
        if !extras.is_empty() {
            out.push_str(&format!(
                "\n  Extra keys: {}\n",
                extras.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }

        out.push('\n');
        match kind {
            MsgKind::Request => {
                out.push_str("Description: Client → Server. Server must reply with a response\n");
                out.push_str("             using the same 'id'.\n");
            }
            MsgKind::Notification => {
                out.push_str(
                    "Description: Client → Server (fire-and-forget). No response expected.\n",
                );
            }
            MsgKind::SuccessResponse => {
                out.push_str(
                    "Description: Server → Client. Method succeeded; result is the return value.\n",
                );
            }
            MsgKind::ErrorResponse => {
                out.push_str(
                    "Description: Server → Client. Method failed; error describes the problem.\n",
                );
            }
            MsgKind::Unknown => {
                out.push_str(
                    "Description: Could not determine message type — missing required fields.\n",
                );
            }
            MsgKind::Batch => {}
        }
    } else if let Some(arr) = v.as_array() {
        out.push_str(&format!(
            "  {} item{} in batch\n\n",
            arr.len(),
            if arr.len() == 1 { "" } else { "s" }
        ));
        for (i, item) in arr.iter().enumerate() {
            let sub_kind = detect_kind(item);
            out.push_str(&format!("  [{}] {} ", i, kind_label(&sub_kind)));
            if let Some(obj) = item.as_object() {
                if let Some(method) = obj.get("method") {
                    out.push_str(&format!("method={}", method.as_str().unwrap_or("?")));
                }
                if let Some(id) = obj.get("id") {
                    out.push_str(&format!(" id={}", format_id_short(id)));
                }
            }
            out.push('\n');
        }
        out.push_str(
            "\nDescription: Batch — send multiple requests/notifications in one HTTP call.\n",
        );
        out.push_str("             Responses (if any) arrive as a batch array too.\n");
    }

    Ok(out)
}

fn preview_value(v: &Value, max: usize) -> String {
    let s = v.to_string();
    if s.len() <= max {
        s
    } else {
        format!("{}… ({} bytes total)", &s[..max], s.len())
    }
}

fn format_id_short(id: &Value) -> String {
    match id {
        Value::Null => "null".to_string(),
        Value::String(s) => format!("\"{}\"", s),
        Value::Number(n) => n.to_string(),
        _ => id.to_string(),
    }
}

// ── Action: validate ───────────────────────────────────────────────────────

fn do_validate(args: &Value) -> Result<String, String> {
    let v = load_message(args)?;
    let issues = validate_message(&v);

    if issues.is_empty() {
        Ok(format!(
            "JSON-RPC 2.0 — {}\n\nVALID ✓  No spec violations found.\n",
            kind_label(&detect_kind(&v))
        ))
    } else {
        let mut out = format!(
            "JSON-RPC 2.0 — {}\n\nINVALID ✗  {} issue{} found:\n\n",
            kind_label(&detect_kind(&v)),
            issues.len(),
            if issues.len() == 1 { "" } else { "s" }
        );
        for (i, issue) in issues.iter().enumerate() {
            out.push_str(&format!("  {}. {}\n", i + 1, issue));
        }
        Ok(out)
    }
}

fn validate_message(v: &Value) -> Vec<String> {
    let mut issues = Vec::new();

    if v.is_array() {
        let arr = v.as_array().unwrap();
        if arr.is_empty() {
            issues.push("Batch array must not be empty (spec §6)".to_string());
        }
        for (i, item) in arr.iter().enumerate() {
            let sub = validate_single_message(item);
            for s in sub {
                issues.push(format!("[{}] {}", i, s));
            }
        }
        return issues;
    }

    issues.extend(validate_single_message(v));
    issues
}

fn validate_single_message(v: &Value) -> Vec<String> {
    let mut issues = Vec::new();

    let obj = match v.as_object() {
        Some(o) => o,
        None => {
            issues.push("Message must be a JSON object".to_string());
            return issues;
        }
    };

    match obj.get("jsonrpc") {
        None => issues.push("Missing required field 'jsonrpc'".to_string()),
        Some(ver) => {
            if ver.as_str() != Some("2.0") {
                issues.push(format!(
                    "'jsonrpc' must be exactly \"2.0\", got: {}",
                    ver
                ));
            }
        }
    }

    let has_method = obj.contains_key("method");
    let has_result = obj.contains_key("result");
    let has_error = obj.contains_key("error");
    let has_id = obj.contains_key("id");

    if has_method {
        if let Some(method) = obj.get("method") {
            if !method.is_string() {
                issues.push("'method' must be a string".to_string());
            }
        }
        if let Some(params) = obj.get("params") {
            if !params.is_array() && !params.is_object() {
                issues.push(
                    "'params' must be an array or object (not null/scalar)".to_string(),
                );
            }
        }
        if has_id {
            let id = &obj["id"];
            if !id.is_string() && !id.is_number() && !id.is_null() {
                issues.push("'id' must be a string, number, or null".to_string());
            }
        }
    } else if has_result {
        if !has_id {
            issues.push("Response must include 'id'".to_string());
        } else {
            let id = &obj["id"];
            if !id.is_string() && !id.is_number() && !id.is_null() {
                issues.push("'id' must be a string, number, or null".to_string());
            }
        }
        if has_error {
            issues.push(
                "Response must not contain both 'result' and 'error'".to_string(),
            );
        }
    } else if has_error {
        if !has_id {
            issues.push(
                "Error response must include 'id' (use null if unknown)".to_string(),
            );
        }
        let err = &obj["error"];
        match err.as_object() {
            None => issues.push("'error' must be an object".to_string()),
            Some(err_obj) => {
                match err_obj.get("code") {
                    None => issues.push("'error.code' is required (integer)".to_string()),
                    Some(c) => {
                        if !c.is_number()
                            || c.as_f64().map(|f| f.fract() != 0.0).unwrap_or(false)
                        {
                            issues.push("'error.code' must be an integer".to_string());
                        }
                    }
                }
                match err_obj.get("message") {
                    None => issues.push("'error.message' is required (string)".to_string()),
                    Some(m) => {
                        if !m.is_string() {
                            issues.push("'error.message' must be a string".to_string());
                        }
                    }
                }
            }
        }
    } else {
        issues.push(
            "Cannot determine message type: missing 'method', 'result', or 'error'".to_string(),
        );
    }

    issues
}

// ── Action: build ──────────────────────────────────────────────────────────

fn do_build(args: &Value) -> Result<String, String> {
    let kind = args.get("kind").and_then(|v| v.as_str()).unwrap_or("request");

    let msg = match kind {
        "request" => build_request(args)?,
        "notification" => build_notification(args)?,
        "response" | "success" => build_response(args),
        "error" => build_error(args)?,
        _ => {
            return Ok(format!(
                "Unknown kind '{}'. Use: request, notification, response, error.",
                kind
            ))
        }
    };

    let pretty = serde_json::to_string_pretty(&msg).map_err(|e| e.to_string())?;
    let issues = validate_single_message(&msg);

    let mut out = format!(
        "JSON-RPC 2.0 — {} built\n{}\n\n{}\n",
        kind_label(&detect_kind(&msg)),
        "─".repeat(45),
        pretty
    );

    if issues.is_empty() {
        out.push_str("\n✓ Passes spec validation\n");
    } else {
        out.push_str("\n⚠ Validation warnings:\n");
        for issue in &issues {
            out.push_str(&format!("  • {}\n", issue));
        }
    }

    Ok(out)
}

fn build_request(args: &Value) -> Result<Value, String> {
    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Pass 'method' string for a request.".to_string())?;

    let id = args.get("id").cloned().unwrap_or(json!(1));

    let mut obj = json!({
        "jsonrpc": "2.0",
        "method": method,
        "id": id
    });

    if let Some(params) = args.get("params").cloned() {
        obj["params"] = params;
    }

    Ok(obj)
}

fn build_notification(args: &Value) -> Result<Value, String> {
    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Pass 'method' string for a notification.".to_string())?;

    let mut obj = json!({
        "jsonrpc": "2.0",
        "method": method
    });

    if let Some(params) = args.get("params").cloned() {
        obj["params"] = params;
    }

    Ok(obj)
}

fn build_response(args: &Value) -> Value {
    let id = args.get("id").cloned().unwrap_or(json!(1));
    let result = args.get("result").cloned().unwrap_or(Value::Null);
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn build_error(args: &Value) -> Result<Value, String> {
    let id = args.get("id").cloned().unwrap_or(Value::Null);

    let code = args
        .get("error_code")
        .or_else(|| args.get("code"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| {
            "Pass 'error_code' (integer) for an error response.".to_string()
        })?;

    let message = args
        .get("error_message")
        .or_else(|| args.get("message"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "Pass 'error_message' (string) for an error response.".to_string()
        })?;

    let mut err_obj = json!({
        "code": code,
        "message": message
    });

    if let Some(data) = args.get("error_data").cloned() {
        err_obj["data"] = data;
    }

    Ok(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": err_obj
    }))
}

// ── Action: batch ──────────────────────────────────────────────────────────

fn do_batch(args: &Value) -> Result<String, String> {
    if args.get("message").is_some()
        || args.get("text").is_some()
        || args.get("json").is_some()
    {
        let v = load_message(args)?;
        if !v.is_array() {
            return Ok("Input is not a JSON array. A batch must be a JSON array.".to_string());
        }
        return parse_batch(&v);
    }

    if let Some(messages) = args.get("messages").and_then(|v| v.as_array()) {
        let batch = Value::Array(messages.clone());
        let issues = validate_message(&batch);
        let pretty = serde_json::to_string_pretty(&batch).map_err(|e| e.to_string())?;
        let mut out = format!(
            "JSON-RPC 2.0 Batch — {} message{}\n{}\n\n{}\n",
            messages.len(),
            if messages.len() == 1 { "" } else { "s" },
            "─".repeat(45),
            pretty
        );
        if issues.is_empty() {
            out.push_str("\n✓ Passes spec validation\n");
        } else {
            out.push_str("\n⚠ Validation issues:\n");
            for issue in &issues {
                out.push_str(&format!("  • {}\n", issue));
            }
        }
        return Ok(out);
    }

    Ok(
        "Pass 'message' to parse a batch, or 'messages' (array) to build one.\n\
         Example: messages=[{\"jsonrpc\":\"2.0\",\"method\":\"add\",\"params\":[1,2],\"id\":1}]"
            .to_string(),
    )
}

fn parse_batch(v: &Value) -> Result<String, String> {
    let arr = v.as_array().unwrap();

    if arr.is_empty() {
        return Ok("Empty batch array. Per spec §6, batch must not be empty.".to_string());
    }

    let mut out = format!(
        "JSON-RPC 2.0 Batch — {} item{}\n{}\n\n",
        arr.len(),
        if arr.len() == 1 { "" } else { "s" },
        "─".repeat(45)
    );

    let mut requests = 0usize;
    let mut notifications = 0usize;
    let mut responses = 0usize;
    let mut errors = 0usize;

    for (i, item) in arr.iter().enumerate() {
        let kind = detect_kind(item);
        match kind {
            MsgKind::Request => requests += 1,
            MsgKind::Notification => notifications += 1,
            MsgKind::SuccessResponse => responses += 1,
            MsgKind::ErrorResponse => errors += 1,
            _ => {}
        }

        out.push_str(&format!("  [{}] {}", i, kind_label(&kind)));
        if let Some(obj) = item.as_object() {
            if let Some(m) = obj.get("method").and_then(|v| v.as_str()) {
                out.push_str(&format!("  method={}", m));
            }
            if let Some(id) = obj.get("id") {
                out.push_str(&format!("  id={}", format_id_short(id)));
            }
            if let Some(params) = obj.get("params") {
                match params {
                    Value::Array(a) => out.push_str(&format!("  params=[{}]", a.len())),
                    Value::Object(o) => out.push_str(&format!("  params={{{}}}", o.len())),
                    _ => {}
                }
            }
        }
        out.push('\n');

        let issues = validate_single_message(item);
        if !issues.is_empty() {
            for issue in &issues {
                out.push_str(&format!("       ✗ {}\n", issue));
            }
        }
    }

    out.push_str(&format!(
        "\nSummary: {} request{}, {} notification{}, {} response{}, {} error response{}\n",
        requests,
        if requests == 1 { "" } else { "s" },
        notifications,
        if notifications == 1 { "" } else { "s" },
        responses,
        if responses == 1 { "" } else { "s" },
        errors,
        if errors == 1 { "" } else { "s" }
    ));

    if requests > 0 || notifications > 0 {
        out.push_str("\nNote: For batch requests, the server may return responses out of order.\n");
        out.push_str("      Match responses to requests by 'id'.\n");
    }

    Ok(out)
}
