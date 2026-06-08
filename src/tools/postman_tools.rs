use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["parse", "requests", "folders", "vars", "summary"],
                "description": "Action: parse (default), requests, folders, vars, summary"
            },
            "json": { "type": "string", "description": "Inline Postman Collection v2.1 JSON" },
            "file": { "type": "string", "description": "Path to a .json Postman collection file" },
            "folder": { "type": "string", "description": "Filter by folder name (substring)" },
            "method": { "type": "string", "description": "Filter requests by HTTP method" },
            "query": { "type": "string", "description": "Filter by name/URL substring" },
            "limit": { "type": "integer", "description": "Max rows to return" }
        }
    })
}

fn load_json(args: &Value) -> Result<Value, String> {
    if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        let raw = std::fs::read_to_string(f).map_err(|e| format!("Cannot read '{}': {}", f, e))?;
        serde_json::from_str(&raw).map_err(|e| format!("JSON parse error: {}", e))
    } else if let Some(j) = args.get("json").and_then(|v| v.as_str()) {
        serde_json::from_str(j).map_err(|e| format!("JSON parse error: {}", e))
    } else {
        Err("Provide 'json' (inline) or 'file' (path to .json collection).".to_string())
    }
}

#[derive(Debug)]
struct Request {
    name: String,
    method: String,
    url: String,
    folder: String,
    auth: String,
    body_type: String,
    headers: usize,
    params: usize,
    tests: bool,
    pre_request: bool,
}

fn extract_url(item: &Value) -> String {
    let req = item.get("request");
    if let Some(r) = req {
        if let Some(url) = r.get("url") {
            if let Some(raw) = url.get("raw").and_then(|v| v.as_str()) {
                return raw.to_string();
            }
            if let Some(s) = url.as_str() {
                return s.to_string();
            }
        }
    }
    String::new()
}

fn extract_method(item: &Value) -> String {
    item.get("request")
        .and_then(|r| r.get("method"))
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase()
}

fn extract_auth(item: &Value) -> String {
    let req = match item.get("request") {
        Some(r) => r,
        None => return String::new(),
    };
    let auth = match req.get("auth") {
        Some(a) => a,
        None => return String::new(),
    };
    auth.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("none")
        .to_string()
}

fn extract_body_type(item: &Value) -> String {
    let req = match item.get("request") {
        Some(r) => r,
        None => return String::new(),
    };
    let body = match req.get("body") {
        Some(b) => b,
        None => return String::new(),
    };
    body.get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("none")
        .to_string()
}

fn extract_headers_count(item: &Value) -> usize {
    item.get("request")
        .and_then(|r| r.get("header"))
        .and_then(|h| h.as_array())
        .map(|a| a.len())
        .unwrap_or(0)
}

fn extract_params_count(item: &Value) -> usize {
    item.get("request")
        .and_then(|r| r.get("url"))
        .and_then(|u| u.get("query"))
        .and_then(|q| q.as_array())
        .map(|a| a.len())
        .unwrap_or(0)
}

fn has_event(item: &Value, listen: &str) -> bool {
    item.get("event")
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .any(|ev| ev.get("listen").and_then(|v| v.as_str()) == Some(listen))
        })
        .unwrap_or(false)
}

fn collect_items(col: &Value, folder: &str, out: &mut Vec<Request>) {
    if let Some(items) = col.get("item").and_then(|v| v.as_array()) {
        for item in items {
            let is_folder = item.get("item").is_some();
            if is_folder {
                let sub_name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(folder)");
                let new_folder = if folder.is_empty() {
                    sub_name.to_string()
                } else {
                    format!("{} / {}", folder, sub_name)
                };
                collect_items(item, &new_folder, out);
            } else {
                let name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(request)")
                    .to_string();
                out.push(Request {
                    name,
                    method: extract_method(item),
                    url: extract_url(item),
                    folder: folder.to_string(),
                    auth: extract_auth(item),
                    body_type: extract_body_type(item),
                    headers: extract_headers_count(item),
                    params: extract_params_count(item),
                    tests: has_event(item, "test"),
                    pre_request: has_event(item, "prerequest"),
                });
            }
        }
    }
}

fn collect_vars(col: &Value) -> Vec<(String, String)> {
    let mut vars = Vec::new();
    if let Some(arr) = col.get("variable").and_then(|v| v.as_array()) {
        for v in arr {
            let key = v
                .get("key")
                .and_then(|k| k.as_str())
                .unwrap_or("")
                .to_string();
            let val = v
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            vars.push((key, val));
        }
    }
    vars
}

fn collection_root(col: &Value) -> &Value {
    if col.get("info").is_some() {
        col
    } else {
        col.get("collection").unwrap_or(col)
    }
}

fn action_parse(args: &Value) -> Result<String, String> {
    let raw = load_json(args)?;
    let col = collection_root(&raw);
    let info = col.get("info");
    let name = info
        .and_then(|i| i.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let schema = info
        .and_then(|i| i.get("schema"))
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let desc = info
        .and_then(|i| i.get("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut requests: Vec<Request> = Vec::new();
    collect_items(col, "", &mut requests);

    let folder_filter = args.get("folder").and_then(|v| v.as_str());
    let method_filter = args
        .get("method")
        .and_then(|v| v.as_str())
        .map(|m| m.to_uppercase());
    let query_filter = args.get("query").and_then(|v| v.as_str());
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(u64::MAX);

    let filtered: Vec<&Request> = requests
        .iter()
        .filter(|r| {
            folder_filter.is_none_or(|f| {
                r.folder.to_lowercase().contains(&f.to_lowercase())
            })
        })
        .filter(|r| method_filter.as_deref().is_none_or(|m| r.method == m))
        .filter(|r| {
            query_filter.is_none_or(|q| {
                r.name.to_lowercase().contains(&q.to_lowercase())
                    || r.url.to_lowercase().contains(&q.to_lowercase())
            })
        })
        .take(limit as usize)
        .collect();

    let mut out = format!("## {}\n", name);
    if !desc.is_empty() {
        out.push_str(&format!(
            "   {}\n",
            desc.chars().take(120).collect::<String>()
        ));
    }
    out.push_str(&format!("   Schema:   {}\n", schema));
    out.push_str(&format!("   Requests: {}\n\n", filtered.len()));

    out.push_str(&format!(
        "{:<7} {:<30} {:<30} {:<10} {}\n",
        "METHOD", "NAME", "FOLDER", "AUTH", "URL"
    ));
    out.push_str(&format!("{}\n", "-".repeat(100)));

    for r in &filtered {
        let fname = if r.folder.is_empty() {
            "(root)".to_string()
        } else {
            r.folder.chars().take(28).collect()
        };
        let rname: String = r.name.chars().take(28).collect();
        let url: String = r.url.chars().take(50).collect();
        out.push_str(&format!(
            "{:<7} {:<30} {:<30} {:<10} {}\n",
            r.method, rname, fname, r.auth, url
        ));
    }
    Ok(out)
}

fn action_requests(args: &Value) -> Result<String, String> {
    let raw = load_json(args)?;
    let col = collection_root(&raw);
    let mut requests: Vec<Request> = Vec::new();
    collect_items(col, "", &mut requests);

    let query = args.get("query").and_then(|v| v.as_str());
    let method_filter = args
        .get("method")
        .and_then(|v| v.as_str())
        .map(|m| m.to_uppercase());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50);

    let filtered: Vec<&Request> = requests
        .iter()
        .filter(|r| method_filter.as_deref().is_none_or(|m| r.method == m))
        .filter(|r| {
            query.is_none_or(|q| {
                r.name.to_lowercase().contains(&q.to_lowercase())
                    || r.url.to_lowercase().contains(&q.to_lowercase())
            })
        })
        .take(limit as usize)
        .collect();

    if filtered.is_empty() {
        return Ok("No matching requests found.".to_string());
    }

    let mut out = String::new();
    for r in filtered {
        out.push_str(&format!("\n### {} {}\n", r.method, r.name));
        if !r.folder.is_empty() {
            out.push_str(&format!("   Folder:   {}\n", r.folder));
        }
        out.push_str(&format!("   URL:      {}\n", r.url));
        if !r.auth.is_empty() && r.auth != "noauth" {
            out.push_str(&format!("   Auth:     {}\n", r.auth));
        }
        if !r.body_type.is_empty() && r.body_type != "none" {
            out.push_str(&format!("   Body:     {}\n", r.body_type));
        }
        if r.headers > 0 {
            out.push_str(&format!("   Headers:  {}\n", r.headers));
        }
        if r.params > 0 {
            out.push_str(&format!("   Params:   {}\n", r.params));
        }
        if r.tests {
            out.push_str("   Tests:    ✓\n");
        }
        if r.pre_request {
            out.push_str("   Pre-req:  ✓\n");
        }
    }
    Ok(out.trim_start().to_string())
}

fn action_folders(args: &Value) -> Result<String, String> {
    let raw = load_json(args)?;
    let col = collection_root(&raw);
    let mut requests: Vec<Request> = Vec::new();
    collect_items(col, "", &mut requests);

    let mut folder_map: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for r in &requests {
        let key = if r.folder.is_empty() {
            "(root)".to_string()
        } else {
            r.folder.clone()
        };
        *folder_map.entry(key).or_insert(0) += 1;
    }

    let mut out = format!("{:<40} {}\n", "FOLDER", "REQUESTS");
    out.push_str(&format!("{}\n", "-".repeat(50)));
    for (folder, count) in &folder_map {
        out.push_str(&format!("{:<40} {}\n", folder, count));
    }
    out.push_str(&format!(
        "\nTotal folders: {}, Total requests: {}\n",
        folder_map.len(),
        requests.len()
    ));
    Ok(out)
}

fn action_vars(args: &Value) -> Result<String, String> {
    let raw = load_json(args)?;
    let col = collection_root(&raw);
    let vars = collect_vars(col);

    if vars.is_empty() {
        return Ok("No collection-level variables found.\n\nTip: Environment variables are stored separately in Postman environments, not in the collection file.".to_string());
    }

    let mut out = format!("{:<30} {}\n", "VARIABLE", "VALUE");
    out.push_str(&format!("{}\n", "-".repeat(60)));
    for (k, v) in &vars {
        let masked = if k.to_lowercase().contains("key")
            || k.to_lowercase().contains("token")
            || k.to_lowercase().contains("secret")
            || k.to_lowercase().contains("pass")
        {
            "[REDACTED]".to_string()
        } else {
            v.chars().take(40).collect()
        };
        out.push_str(&format!("{:<30} {}\n", k, masked));
    }
    Ok(out)
}

fn action_summary(args: &Value) -> Result<String, String> {
    let raw = load_json(args)?;
    let col = collection_root(&raw);
    let info = col.get("info");
    let name = info
        .and_then(|i| i.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    let mut requests: Vec<Request> = Vec::new();
    collect_items(col, "", &mut requests);

    let vars = collect_vars(col);
    let mut method_map: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut auth_map: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut with_tests = 0usize;
    let mut with_body = 0usize;
    let mut folders: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for r in &requests {
        *method_map.entry(r.method.clone()).or_insert(0) += 1;
        let auth = if r.auth.is_empty() || r.auth == "noauth" {
            "none".to_string()
        } else {
            r.auth.clone()
        };
        *auth_map.entry(auth).or_insert(0) += 1;
        if r.tests {
            with_tests += 1;
        }
        if !r.body_type.is_empty() && r.body_type != "none" {
            with_body += 1;
        }
        if !r.folder.is_empty() {
            folders.insert(r.folder.clone());
        }
    }

    let mut out = format!("## Collection: {}\n\n", name);
    out.push_str(&format!("  Requests:   {}\n", requests.len()));
    out.push_str(&format!("  Folders:    {}\n", folders.len()));
    out.push_str(&format!("  Variables:  {}\n", vars.len()));
    out.push_str(&format!("  With tests: {}\n", with_tests));
    out.push_str(&format!("  With body:  {}\n\n", with_body));

    out.push_str("## HTTP Methods\n\n");
    for (m, c) in &method_map {
        out.push_str(&format!("  {:8} {}\n", m, c));
    }

    out.push_str("\n## Auth Types\n\n");
    for (a, c) in &auth_map {
        out.push_str(&format!("  {:20} {}\n", a, c));
    }
    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "requests" => action_requests(args),
        "folders" => action_folders(args),
        "vars" => action_vars(args),
        "summary" => action_summary(args),
        _ => action_parse(args),
    }
}
