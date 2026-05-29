use serde_json::Value;
use serde_yaml::Value as Yaml;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else {
        "info".to_string()
    };
    match action.as_str() {
        "info" => info_action(args),
        "endpoints" => endpoints_action(args),
        "schemas" => schemas_action(args),
        "search" => search_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: info, endpoints, schemas, search, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("yaml"))
        .or_else(|| args.get("json"))
        .or_else(|| args.get("spec"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            "Missing 'text' — pass the OpenAPI/Swagger spec content as a string".to_string()
        })
}

fn load_spec(text: &str) -> Result<Yaml, String> {
    // Try YAML first (superset of JSON)
    serde_yaml::from_str(text).map_err(|e| format!("Failed to parse spec: {}", e))
}

fn yaml_str(v: &Yaml) -> String {
    match v {
        Yaml::String(s) => s.clone(),
        Yaml::Number(n) => n.to_string(),
        Yaml::Bool(b) => b.to_string(),
        Yaml::Null => "".to_string(),
        _ => serde_yaml::to_string(v)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn spec_version(doc: &Yaml) -> (String, bool) {
    // OpenAPI 3.x
    if let Some(v) = doc.get("openapi") {
        return (yaml_str(v), true);
    }
    // Swagger 2.x
    if let Some(v) = doc.get("swagger") {
        return (yaml_str(v), false);
    }
    ("unknown".to_string(), false)
}

fn get_paths<'a>(doc: &'a Yaml) -> Option<&'a serde_yaml::Mapping> {
    doc.get("paths")?.as_mapping()
}

fn get_schemas<'a>(doc: &'a Yaml) -> Option<&'a serde_yaml::Mapping> {
    // OpenAPI 3.x
    if let Some(schemas) = doc
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_mapping())
    {
        return Some(schemas);
    }
    // Swagger 2.x
    doc.get("definitions")?.as_mapping()
}

struct EndpointInfo {
    method: String,
    path: String,
    summary: String,
    operation_id: String,
    tags: Vec<String>,
    deprecated: bool,
}

fn collect_all_endpoints(doc: &Yaml) -> Vec<EndpointInfo> {
    let mut endpoints = Vec::new();
    let methods = [
        "get", "post", "put", "patch", "delete", "head", "options", "trace",
    ];

    if let Some(paths) = get_paths(doc) {
        for (path_key, path_item) in paths {
            let path = yaml_str(path_key);
            for method in &methods {
                if let Some(op) = path_item.get(*method) {
                    let summary = op
                        .get("summary")
                        .map(yaml_str)
                        .unwrap_or_else(|| "(no summary)".to_string());
                    let operation_id = op.get("operationId").map(yaml_str).unwrap_or_default();
                    let deprecated = op
                        .get("deprecated")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let tags: Vec<String> = op
                        .get("tags")
                        .and_then(|t| t.as_sequence())
                        .map(|seq| seq.iter().map(yaml_str).collect())
                        .unwrap_or_default();
                    endpoints.push(EndpointInfo {
                        method: method.to_uppercase(),
                        path: path.clone(),
                        summary,
                        operation_id,
                        tags,
                        deprecated,
                    });
                }
            }
        }
    }
    endpoints
}

fn info_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = load_spec(&text)?;

    let (version, is_oas3) = spec_version(&doc);
    let format = if is_oas3 { "OpenAPI" } else { "Swagger" };

    let info = doc.get("info");
    let title = info
        .and_then(|i| i.get("title"))
        .map(yaml_str)
        .unwrap_or_else(|| "(untitled)".to_string());
    let api_version = info
        .and_then(|i| i.get("version"))
        .map(yaml_str)
        .unwrap_or_else(|| "(unversioned)".to_string());
    let description = info
        .and_then(|i| i.get("description"))
        .map(yaml_str)
        .unwrap_or_default();

    let endpoint_count = collect_all_endpoints(&doc).len();
    let schema_count = get_schemas(&doc).map(|m| m.len()).unwrap_or(0);

    let mut out = format!("{} {} Spec\n{}\n\n", format, version, "=".repeat(44));
    out += &format!("Title:      {}\n", title);
    out += &format!("API Ver:    {}\n", api_version);
    if !description.is_empty() {
        let snippet: String = description.chars().take(120).collect();
        let ellipsis = if description.len() > 120 { "…" } else { "" };
        out += &format!("Desc:       {}{}\n", snippet, ellipsis);
    }
    out += "\n";

    // Servers (OAS 3.x)
    if let Some(servers) = doc.get("servers").and_then(|s| s.as_sequence()) {
        out += &format!("Servers ({}):\n", servers.len());
        for srv in servers.iter().take(5) {
            let url = srv.get("url").map(yaml_str).unwrap_or_default();
            let desc = srv.get("description").map(yaml_str).unwrap_or_default();
            if desc.is_empty() {
                out += &format!("  {}\n", url);
            } else {
                out += &format!("  {} — {}\n", url, desc);
            }
        }
        out += "\n";
    }
    // Base path (Swagger 2.x)
    if let Some(host) = doc.get("host").map(yaml_str) {
        let base = doc.get("basePath").map(yaml_str).unwrap_or_default();
        out += &format!("Host:       {}{}\n\n", host, base);
    }

    out += &format!("Endpoints:  {}\n", endpoint_count);
    out += &format!("Schemas:    {}\n", schema_count);

    // Tags summary
    let mut tags: std::collections::HashSet<String> = std::collections::HashSet::new();
    for ep in collect_all_endpoints(&doc) {
        for tag in ep.tags {
            tags.insert(tag);
        }
    }
    if !tags.is_empty() {
        let mut sorted: Vec<_> = tags.into_iter().collect();
        sorted.sort();
        out += &format!("Tags:       {}\n", sorted.join(", "));
    }

    // Auth schemes
    let security_schemes = if is_oas3 {
        doc.get("components")
            .and_then(|c| c.get("securitySchemes"))
            .and_then(|s| s.as_mapping())
            .map(|m| m.keys().map(yaml_str).collect::<Vec<_>>())
    } else {
        doc.get("securityDefinitions")
            .and_then(|s| s.as_mapping())
            .map(|m| m.keys().map(yaml_str).collect::<Vec<_>>())
    };
    if let Some(schemes) = security_schemes {
        if !schemes.is_empty() {
            out += &format!("Auth:       {}\n", schemes.join(", "));
        }
    }

    Ok(out)
}

fn endpoints_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = load_spec(&text)?;
    let filter_tag = args.get("tag").and_then(|v| v.as_str());
    let endpoints = collect_all_endpoints(&doc);

    if endpoints.is_empty() {
        return Ok("No endpoints found in spec.\n".to_string());
    }

    let mut out = format!(
        "Endpoints  [{} total]\n{}\n\n",
        endpoints.len(),
        "=".repeat(52)
    );

    // Group by tag if requested, else flat list
    if filter_tag.is_some() {
        let tag = filter_tag.unwrap().to_lowercase();
        for ep in &endpoints {
            if !ep.tags.iter().any(|t| t.to_lowercase() == tag) {
                continue;
            }
            let deprecated = if ep.deprecated { " [DEPRECATED]" } else { "" };
            out += &format!(
                "{:<8} {:<40} {}{}\n",
                ep.method, ep.path, ep.summary, deprecated
            );
            if !ep.operation_id.is_empty() {
                out += &format!("         operationId: {}\n", ep.operation_id);
            }
        }
    } else {
        for ep in &endpoints {
            let deprecated = if ep.deprecated { " [DEPRECATED]" } else { "" };
            let tags = if ep.tags.is_empty() {
                String::new()
            } else {
                format!("  [{}]", ep.tags.join(", "))
            };
            out += &format!(
                "{:<8} {:<40} {}{}{}\n",
                ep.method, ep.path, ep.summary, tags, deprecated
            );
        }
    }
    Ok(out)
}

fn schemas_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let name_filter = args.get("schema").and_then(|v| v.as_str());
    let doc = load_spec(&text)?;

    let schemas = match get_schemas(&doc) {
        Some(s) => s,
        None => return Ok("No schemas/definitions found in spec.\n".to_string()),
    };

    let mut out = format!("Schemas  [{} total]\n{}\n\n", schemas.len(), "=".repeat(44));

    for (key, schema) in schemas {
        let name = yaml_str(key);
        if let Some(f) = name_filter {
            if !name.to_lowercase().contains(&f.to_lowercase()) {
                continue;
            }
        }

        let schema_type = schema.get("type").map(yaml_str).unwrap_or_default();
        let description = schema.get("description").map(yaml_str).unwrap_or_default();

        out += &format!("{}", name);
        if !schema_type.is_empty() {
            out += &format!(" ({})", schema_type);
        }
        out += "\n";

        if !description.is_empty() {
            let snippet: String = description.chars().take(80).collect();
            let ellipsis = if description.len() > 80 { "…" } else { "" };
            out += &format!("  {}{}\n", snippet, ellipsis);
        }

        // Show properties
        if let Some(props) = schema.get("properties").and_then(|p| p.as_mapping()) {
            let required: Vec<String> = schema
                .get("required")
                .and_then(|r| r.as_sequence())
                .map(|seq| seq.iter().map(yaml_str).collect())
                .unwrap_or_default();

            out += &format!("  Properties ({}):\n", props.len());
            for (prop_key, prop_val) in props.iter().take(10) {
                let prop_name = yaml_str(prop_key);
                let prop_type = prop_val.get("type").map(yaml_str).unwrap_or_else(|| {
                    prop_val
                        .get("$ref")
                        .map(|r| yaml_str(r).rsplit('/').next().unwrap_or("ref").to_string())
                        .unwrap_or_else(|| "any".to_string())
                });
                let req_flag = if required.contains(&prop_name) {
                    "*"
                } else {
                    " "
                };
                out += &format!("    {}{:<24} {}\n", req_flag, prop_name, prop_type);
            }
            if props.len() > 10 {
                out += &format!("    … ({} more)\n", props.len() - 10);
            }
        }
        out += "\n";
    }
    Ok(out)
}

fn search_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let query = args
        .get("query")
        .or_else(|| args.get("q"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'query' — search term to filter endpoints")?
        .to_lowercase();
    let doc = load_spec(&text)?;
    let endpoints = collect_all_endpoints(&doc);

    let matches: Vec<_> = endpoints
        .iter()
        .filter(|ep| {
            ep.path.to_lowercase().contains(&query)
                || ep.summary.to_lowercase().contains(&query)
                || ep.operation_id.to_lowercase().contains(&query)
                || ep.tags.iter().any(|t| t.to_lowercase().contains(&query))
                || ep.method.to_lowercase() == query
        })
        .collect();

    if matches.is_empty() {
        return Ok(format!("No endpoints matching '{}'.\n", query));
    }

    let mut out = format!(
        "Search: '{}' — {} match(es)\n{}\n\n",
        query,
        matches.len(),
        "=".repeat(44)
    );
    for ep in &matches {
        let deprecated = if ep.deprecated { " [DEPRECATED]" } else { "" };
        out += &format!(
            "{:<8} {:<40} {}{}\n",
            ep.method, ep.path, ep.summary, deprecated
        );
        if !ep.operation_id.is_empty() {
            out += &format!("         operationId: {}\n", ep.operation_id);
        }
    }
    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = load_spec(&text)?;
    let mut warnings: Vec<String> = Vec::new();

    let (version, _) = spec_version(&doc);
    if version == "unknown" {
        warnings.push("Missing 'openapi' or 'swagger' version field".to_string());
    }

    if doc.get("info").is_none() {
        warnings.push("Missing 'info' section (title and version required)".to_string());
    }

    let endpoints = collect_all_endpoints(&doc);
    if endpoints.is_empty() {
        warnings.push("No paths/endpoints defined".to_string());
    }

    let mut no_summary = 0;
    let mut no_operation_id = 0;
    let mut deprecated_count = 0;
    let mut operation_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut duplicate_op_ids: Vec<String> = Vec::new();

    for ep in &endpoints {
        if ep.summary == "(no summary)" || ep.summary.is_empty() {
            no_summary += 1;
        }
        if ep.operation_id.is_empty() {
            no_operation_id += 1;
        } else if !operation_ids.insert(ep.operation_id.clone()) {
            duplicate_op_ids.push(ep.operation_id.clone());
        }
        if ep.deprecated {
            deprecated_count += 1;
        }
    }

    if no_summary > 0 {
        warnings.push(format!(
            "{} endpoint(s) missing 'summary' — hard to understand without description",
            no_summary
        ));
    }
    if no_operation_id > 0 {
        warnings.push(format!(
            "{} endpoint(s) missing 'operationId' — SDK generation may produce generic names",
            no_operation_id
        ));
    }
    if !duplicate_op_ids.is_empty() {
        duplicate_op_ids.dedup();
        warnings.push(format!(
            "Duplicate operationId(s): {} — must be unique across the spec",
            duplicate_op_ids.join(", ")
        ));
    }
    if deprecated_count > 0 {
        warnings.push(format!(
            "{} deprecated endpoint(s) — consider removing or providing migration path",
            deprecated_count
        ));
    }

    let mut out = format!("OpenAPI Spec Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!("Spec version: {}\n", version);
    out += &format!("{} endpoint(s) checked.\n", endpoints.len());
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
