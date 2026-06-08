use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("server").is_some() {
        "inspect".to_string()
    } else {
        "list".to_string()
    };
    match action.as_str() {
        "list" => list_action(args),
        "inspect" => inspect_action(args),
        "locations" => locations_action(args),
        "directives" => directives_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: list, inspect, locations, directives, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("config"))
        .or_else(|| args.get("conf"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the nginx config content as a string".to_string())
}

#[derive(Debug, Clone)]
struct NginxBlock {
    kind: String,
    params: String,
    directives: Vec<(String, String)>,
    children: Vec<NginxBlock>,
}

impl NginxBlock {
    fn new(kind: &str, params: &str) -> Self {
        NginxBlock {
            kind: kind.to_string(),
            params: params.trim().to_string(),
            directives: Vec::new(),
            children: Vec::new(),
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.directives
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    fn get_all(&self, key: &str) -> Vec<&str> {
        self.directives
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
            .collect()
    }
}

// Tokenizer: strip comments, handle quoted strings
fn tokenize(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut in_comment = false;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if in_comment {
            if c == '\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if c == '#' && !in_quote {
            in_comment = true;
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            in_quote = !in_quote;
            current.push(c);
            i += 1;
            continue;
        }
        if in_quote {
            current.push(c);
            i += 1;
            continue;
        }
        if c == '{' || c == '}' || c == ';' {
            if !current.trim().is_empty() {
                tokens.push(current.trim().to_string());
                current = String::new();
            }
            tokens.push(c.to_string());
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            if !current.trim().is_empty() {
                tokens.push(current.trim().to_string());
                current = String::new();
            }
        } else {
            current.push(c);
        }
        i += 1;
    }
    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }
    tokens
}

fn parse_block(tokens: &[String], pos: &mut usize) -> NginxBlock {
    let mut block = NginxBlock::new("root", "");
    while *pos < tokens.len() {
        let tok = &tokens[*pos];
        if tok == "}" {
            *pos += 1;
            break;
        }
        // Collect directive tokens until ; or {
        let mut parts: Vec<String> = Vec::new();
        while *pos < tokens.len() {
            let t = tokens[*pos].clone();
            if t == "{" {
                *pos += 1;
                // It's a named block
                let kind = parts.first().cloned().unwrap_or_default();
                let params = parts[1..].join(" ");
                let mut child = parse_block(tokens, pos);
                child.kind = kind;
                child.params = params;
                block.children.push(child);
                parts.clear();
                break;
            } else if t == "}" {
                // Unexpected — stop
                break;
            } else if t == ";" {
                *pos += 1;
                if parts.len() >= 2 {
                    let key = parts[0].clone();
                    let val = parts[1..].join(" ");
                    block.directives.push((key, val));
                } else if parts.len() == 1 {
                    block.directives.push((parts[0].clone(), String::new()));
                }
                parts.clear();
                break;
            } else {
                parts.push(t);
                *pos += 1;
            }
        }
    }
    block
}

fn parse_nginx_conf(text: &str) -> NginxBlock {
    let tokens = tokenize(text);
    let mut pos = 0;
    parse_block(&tokens, &mut pos)
}

fn collect_blocks<'a>(root: &'a NginxBlock, kind: &str) -> Vec<&'a NginxBlock> {
    let mut result = Vec::new();
    for child in &root.children {
        if child.kind.eq_ignore_ascii_case(kind) {
            result.push(child);
        }
        // Also look inside http{} blocks
        result.extend(collect_blocks(child, kind));
    }
    result
}

fn server_label(server: &NginxBlock) -> String {
    let names: Vec<&str> = server.get_all("server_name");
    let listen: Vec<&str> = server.get_all("listen");
    let name = if names.is_empty() || names.iter().all(|n| *n == "_") {
        "(default)".to_string()
    } else {
        names.join(", ")
    };
    let port = if listen.is_empty() {
        "80".to_string()
    } else {
        listen.join(", ")
    };
    format!("{} [{}]", name, port)
}

fn list_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let root = parse_nginx_conf(&text);
    let servers = collect_blocks(&root, "server");

    let mut out = format!(
        "Nginx Config  [{} server block(s)]\n{}\n\n",
        servers.len(),
        "=".repeat(52)
    );

    if servers.is_empty() {
        out += "No server blocks found. Check the config format.\n";
        return Ok(out);
    }

    for (i, server) in servers.iter().enumerate() {
        let label = server_label(server);
        let root_dir = server.get("root").unwrap_or("(none)");
        let proxy = server.get("proxy_pass");
        let ssl = server
            .directives
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("ssl") && v.eq_ignore_ascii_case("on"))
            || server
                .get_all("listen")
                .iter()
                .any(|l| l.contains("ssl") || l.contains("443"));
        let locations = collect_blocks(server, "location");

        out += &format!("Server #{}: {}\n", i + 1, label);
        out += &format!("  Root:      {}\n", root_dir);
        if let Some(p) = proxy {
            out += &format!("  Proxy:     {}\n", p);
        }
        out += &format!("  SSL:       {}\n", if ssl { "yes" } else { "no" });
        out += &format!("  Locations: {}\n\n", locations.len());
    }
    Ok(out)
}

fn inspect_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let query = args
        .get("server")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'server' — server_name or index number to inspect")?;
    let root = parse_nginx_conf(&text);
    let servers = collect_blocks(&root, "server");

    if servers.is_empty() {
        return Err("No server blocks found in config".to_string());
    }

    // Try numeric index first
    let server = if let Ok(idx) = query.parse::<usize>() {
        servers
            .get(idx.saturating_sub(1))
            .copied()
            .ok_or_else(|| format!("Server index {} out of range (1–{})", idx, servers.len()))?
    } else {
        servers
            .iter()
            .find(|s| {
                s.get_all("server_name")
                    .iter()
                    .any(|n| n.to_lowercase().contains(&query.to_lowercase()))
            })
            .copied()
            .ok_or_else(|| {
                format!(
                    "Server '{}' not found. Use action='list' to see all servers.",
                    query
                )
            })?
    };

    let label = server_label(server);
    let mut out = format!("Server: {}\n{}\n\n", label, "=".repeat(44));

    // Top-level directives
    for (k, v) in &server.directives {
        out += &format!("{:<24} {}\n", k, v);
    }

    // Location blocks
    let locations = collect_blocks(server, "location");
    if !locations.is_empty() {
        out += &format!("\nLocations ({}):\n", locations.len());
        for loc in &locations {
            let proxy = loc.get("proxy_pass");
            let root_dir = loc.get("root");
            let alias = loc.get("alias");
            let try_files = loc.get("try_files");
            out += &format!("  location {} {{\n", loc.params);
            if let Some(p) = proxy {
                out += &format!("    proxy_pass     {}\n", p);
            }
            if let Some(r) = root_dir {
                out += &format!("    root           {}\n", r);
            }
            if let Some(a) = alias {
                out += &format!("    alias          {}\n", a);
            }
            if let Some(t) = try_files {
                out += &format!("    try_files      {}\n", t);
            }
            for (k, v) in &loc.directives {
                let kl = k.to_lowercase();
                if kl != "proxy_pass" && kl != "root" && kl != "alias" && kl != "try_files" {
                    out += &format!("    {:<20} {}\n", k, v);
                }
            }
            out += "  }\n";
        }
    }
    Ok(out)
}

fn locations_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let filter = args.get("server").and_then(|v| v.as_str());
    let root = parse_nginx_conf(&text);
    let servers = collect_blocks(&root, "server");

    let mut out = format!("Location Blocks\n{}\n\n", "=".repeat(44));

    for server in &servers {
        if let Some(f) = filter {
            let names = server.get_all("server_name");
            if !names
                .iter()
                .any(|n| n.to_lowercase().contains(&f.to_lowercase()))
            {
                continue;
            }
        }

        let label = server_label(server);
        let locations = collect_blocks(server, "location");
        if locations.is_empty() {
            continue;
        }

        out += &format!("Server: {}\n", label);
        for loc in &locations {
            let proxy = loc.get("proxy_pass").map(|p| format!("→ {}", p));
            let root_dir = loc
                .get("root")
                .map(|r| format!("root: {}", r))
                .or_else(|| loc.get("alias").map(|a| format!("alias: {}", a)));
            let desc = proxy.or(root_dir).unwrap_or_else(|| "(inline)".to_string());
            out += &format!("  location {:<30} {}\n", loc.params, desc);
        }
        out += "\n";
    }
    Ok(out)
}

fn directives_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let root = parse_nginx_conf(&text);

    let mut out = format!("Nginx Directives (Global/HTTP)\n{}\n\n", "=".repeat(44));

    // Root-level directives
    if !root.directives.is_empty() {
        out += "Global:\n";
        for (k, v) in &root.directives {
            out += &format!("  {:<28} {}\n", k, v);
        }
        out += "\n";
    }

    // http block directives
    let http_blocks: Vec<&NginxBlock> = root
        .children
        .iter()
        .filter(|b| b.kind.eq_ignore_ascii_case("http"))
        .collect();
    for http in http_blocks {
        if !http.directives.is_empty() {
            out += "http {}:\n";
            for (k, v) in &http.directives {
                out += &format!("  {:<28} {}\n", k, v);
            }
            out += "\n";
        }
        // upstream blocks
        let upstreams = collect_blocks(http, "upstream");
        if !upstreams.is_empty() {
            out += &format!("Upstreams ({}):\n", upstreams.len());
            for us in &upstreams {
                out += &format!("  upstream {} {{\n", us.params);
                for (k, v) in &us.directives {
                    out += &format!("    {:<24} {}\n", k, v);
                }
                out += "  }\n";
            }
            out += "\n";
        }
    }
    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let root = parse_nginx_conf(&text);
    let servers = collect_blocks(&root, "server");
    let mut warnings: Vec<String> = Vec::new();

    if servers.is_empty() {
        warnings.push(
            "No server blocks found — check for missing http{} wrapper or syntax errors"
                .to_string(),
        );
    }

    let mut default_server_count = 0;
    for (i, server) in servers.iter().enumerate() {
        let label = server_label(server);

        // Warn on missing server_name
        let names = server.get_all("server_name");
        if names.is_empty() {
            warnings.push(format!(
                "Server #{} {}: no server_name directive — acts as catch-all",
                i + 1,
                label
            ));
        }
        if names.contains(&"_") {
            default_server_count += 1;
        }

        // Warn on root without index
        if server.get("root").is_some() && server.get("index").is_none() {
            let has_index = collect_blocks(server, "location")
                .iter()
                .any(|l| l.get("index").is_some());
            if !has_index {
                warnings.push(format!(
                    "Server #{} {}: 'root' set but no 'index' directive — directory listing may fail",
                    i + 1,
                    label
                ));
            }
        }

        // Warn on SSL listen without ssl_certificate
        let listen_ssl = server
            .get_all("listen")
            .iter()
            .any(|l| l.contains("ssl") || l.contains("443"));
        if listen_ssl && server.get("ssl_certificate").is_none() {
            warnings.push(format!(
                "Server #{} {}: listens on SSL port but no ssl_certificate directive",
                i + 1,
                label
            ));
        }

        // Warn on proxy without proxy_set_header Host
        let locations = collect_blocks(server, "location");
        for loc in &locations {
            if loc.get("proxy_pass").is_some() {
                let has_host_header = loc.directives.iter().any(|(k, v)| {
                    k.eq_ignore_ascii_case("proxy_set_header")
                        && v.to_lowercase().starts_with("host")
                });
                if !has_host_header {
                    warnings.push(format!(
                        "Server #{} {} location {}: proxy_pass without 'proxy_set_header Host' — upstream may not see correct Host header",
                        i + 1, label, loc.params
                    ));
                }
            }
        }
    }

    if default_server_count > 1 {
        warnings.push(format!(
            "{} servers with server_name '_' — only one default server is effective",
            default_server_count
        ));
    }

    let mut out = format!("Nginx Config Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!("{} server block(s) parsed.\n", servers.len());
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
