use serde_json::{json, Value};
use std::collections::HashMap;

pub fn make_schema() -> Value {
    json!({
        "name": "haproxy_tools",
        "description": "Parse, inspect, and validate HAProxy configuration files without external utilities.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "frontends", "backends", "servers", "validate"],
                    "description": "parse=overview of all sections, frontends=list frontends with bind/default_backend, backends=list backends with balance/server counts, servers=all servers per backend with address/port/options, validate=configuration checks"
                },
                "config": { "type": "string", "description": "HAProxy configuration content inline" },
                "file": { "type": "string", "description": "Path to haproxy.cfg file" },
                "backend": { "type": "string", "description": "Filter 'servers' to a specific backend name" }
            }
        }
    })
}

#[derive(Debug, Clone)]
struct Section {
    kind: String,
    name: String,
    directives: Vec<(String, String)>,
}

fn load_config(args: &Value) -> Result<String, String> {
    if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(f).map_err(|e| format!("Cannot read {}: {}", f, e))
    } else if let Some(c) = args.get("config").and_then(|v| v.as_str()) {
        Ok(c.to_string())
    } else {
        Err("Provide 'config' (inline content) or 'file' (path to haproxy.cfg).".to_string())
    }
}

fn parse_config(src: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<Section> = None;

    for raw_line in src.lines() {
        // Strip inline comments
        let line = if let Some(pos) = raw_line.find(" #").or_else(|| raw_line.find("\t#")) {
            &raw_line[..pos]
        } else {
            raw_line
        };
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Section headers
        let parts: Vec<&str> = line
            .splitn(3, char::is_whitespace)
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            continue;
        }

        let first = parts[0].to_lowercase();
        match first.as_str() {
            "global" | "defaults" => {
                if let Some(sec) = current.take() {
                    sections.push(sec);
                }
                current = Some(Section {
                    kind: first.clone(),
                    name: first,
                    directives: Vec::new(),
                });
            }
            "frontend" | "backend" | "listen" | "resolvers" | "userlist" | "peers" | "mailers" => {
                if let Some(sec) = current.take() {
                    sections.push(sec);
                }
                let name = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
                current = Some(Section {
                    kind: first,
                    name,
                    directives: Vec::new(),
                });
            }
            _ => {
                // Directive inside current section
                if let Some(ref mut sec) = current {
                    let key = parts[0].to_lowercase();
                    let val = if parts.len() > 1 {
                        line[parts[0].len()..].trim().to_string()
                    } else {
                        String::new()
                    };
                    sec.directives.push((key, val));
                }
            }
        }
    }
    if let Some(sec) = current {
        sections.push(sec);
    }
    sections
}

fn get_directive<'a>(sec: &'a Section, key: &str) -> Option<&'a str> {
    sec.directives
        .iter()
        .rev()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn get_all_directives<'a>(sec: &'a Section, key: &str) -> Vec<&'a str> {
    sec.directives
        .iter()
        .filter(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .collect()
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    let src = load_config(args)?;
    let sections = parse_config(&src);

    match action {
        "parse" => action_parse(&sections),
        "frontends" => action_frontends(&sections),
        "backends" => action_backends(&sections),
        "servers" => {
            let filter = args.get("backend").and_then(|v| v.as_str());
            action_servers(&sections, filter)
        }
        "validate" => action_validate(&sections),
        _ => Err(format!(
            "Unknown action '{}'. Use: parse, frontends, backends, servers, validate",
            action
        )),
    }
}

fn action_parse(sections: &[Section]) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("## HAProxy Configuration\n\n");

    let globals = sections.iter().filter(|s| s.kind == "global").count();
    let defaults = sections.iter().filter(|s| s.kind == "defaults").count();
    let frontends: Vec<_> = sections
        .iter()
        .filter(|s| s.kind == "frontend" || s.kind == "listen")
        .collect();
    let backends: Vec<_> = sections
        .iter()
        .filter(|s| s.kind == "backend" || s.kind == "listen")
        .collect();

    out.push_str(&format!(
        "Sections: {} global, {} defaults, {} frontend(s), {} backend(s)\n\n",
        globals,
        defaults,
        frontends.len(),
        backends.len()
    ));

    // Global highlights
    if let Some(g) = sections.iter().find(|s| s.kind == "global") {
        let maxconn = get_directive(g, "maxconn").unwrap_or("not set");
        let log = get_directive(g, "log").unwrap_or("not set");
        let user = get_directive(g, "user").unwrap_or("root");
        let group = get_directive(g, "group").unwrap_or("root");
        out.push_str(&format!(
            "**Global:** maxconn={}, user={}, group={}, log={}\n\n",
            maxconn, user, group, log
        ));
    }

    // Defaults highlights
    if let Some(d) = sections.iter().find(|s| s.kind == "defaults") {
        let mode = get_directive(d, "mode").unwrap_or("tcp");
        let timeout_connect = get_directive(d, "timeout connect").unwrap_or("—");
        let timeout_client = get_directive(d, "timeout client").unwrap_or("—");
        let timeout_server = get_directive(d, "timeout server").unwrap_or("—");
        let retries = get_directive(d, "retries").unwrap_or("3");
        out.push_str(&format!(
            "**Defaults:** mode={}, retries={}, timeout connect={}, client={}, server={}\n\n",
            mode, retries, timeout_connect, timeout_client, timeout_server
        ));
    }

    // Frontend list
    let fe: Vec<_> = sections.iter().filter(|s| s.kind == "frontend").collect();
    if !fe.is_empty() {
        out.push_str("**Frontends:**\n");
        for f in &fe {
            let binds = get_all_directives(f, "bind").join(", ");
            let default_backend = get_directive(f, "default_backend").unwrap_or("—");
            let mode = get_directive(f, "mode").unwrap_or("default");
            out.push_str(&format!(
                "  {} — bind: {}  default_backend: {}  mode: {}\n",
                f.name, binds, default_backend, mode
            ));
        }
        out.push('\n');
    }

    // Backend list
    let be: Vec<_> = sections.iter().filter(|s| s.kind == "backend").collect();
    if !be.is_empty() {
        out.push_str("**Backends:**\n");
        for b in &be {
            let balance = get_directive(b, "balance").unwrap_or("roundrobin");
            let server_count = get_all_directives(b, "server").len();
            let mode = get_directive(b, "mode").unwrap_or("default");
            out.push_str(&format!(
                "  {} — balance: {}  servers: {}  mode: {}\n",
                b.name, balance, server_count, mode
            ));
        }
        out.push('\n');
    }

    // Listen sections
    let li: Vec<_> = sections.iter().filter(|s| s.kind == "listen").collect();
    if !li.is_empty() {
        out.push_str("**Listen (combined frontend+backend):**\n");
        for l in &li {
            let binds = get_all_directives(l, "bind").join(", ");
            let server_count = get_all_directives(l, "server").len();
            out.push_str(&format!(
                "  {} — bind: {}  servers: {}\n",
                l.name, binds, server_count
            ));
        }
        out.push('\n');
    }

    Ok(out)
}

fn action_frontends(sections: &[Section]) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("## Frontends\n\n");
    let fe: Vec<_> = sections
        .iter()
        .filter(|s| s.kind == "frontend" || s.kind == "listen")
        .collect();
    if fe.is_empty() {
        return Ok("No frontend or listen sections found.\n".to_string());
    }
    for f in &fe {
        out.push_str(&format!("### {}\n", f.name));
        for bind in get_all_directives(f, "bind") {
            out.push_str(&format!("  bind {}\n", bind));
        }
        if let Some(db) = get_directive(f, "default_backend") {
            out.push_str(&format!("  default_backend {}\n", db));
        }
        if let Some(mode) = get_directive(f, "mode") {
            out.push_str(&format!("  mode {}\n", mode));
        }
        // ACLs
        let acls = get_all_directives(f, "acl");
        if !acls.is_empty() {
            out.push_str(&format!("  ACLs: {}\n", acls.len()));
            for acl in acls.iter().take(5) {
                out.push_str(&format!("    acl {}\n", acl));
            }
            if acls.len() > 5 {
                out.push_str(&format!("    … {} more\n", acls.len() - 5));
            }
        }
        // use_backend rules
        for ub in get_all_directives(f, "use_backend") {
            out.push_str(&format!("  use_backend {}\n", ub));
        }
        // Stats
        let maxconn = get_directive(f, "maxconn");
        let timeout_client = get_directive(f, "timeout client");
        if let Some(v) = maxconn {
            out.push_str(&format!("  maxconn {}\n", v));
        }
        if let Some(v) = timeout_client {
            out.push_str(&format!("  timeout client {}\n", v));
        }
        out.push('\n');
    }
    Ok(out)
}

fn action_backends(sections: &[Section]) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("## Backends\n\n");
    let be: Vec<_> = sections
        .iter()
        .filter(|s| s.kind == "backend" || s.kind == "listen")
        .collect();
    if be.is_empty() {
        return Ok("No backend or listen sections found.\n".to_string());
    }
    for b in &be {
        let balance = get_directive(b, "balance").unwrap_or("roundrobin");
        let mode = get_directive(b, "mode").unwrap_or("default");
        let servers = get_all_directives(b, "server");
        let health = get_all_directives(b, "option")
            .iter()
            .filter(|o| o.contains("check") || o.contains("httpchk"))
            .count();
        out.push_str(&format!("### {}\n", b.name));
        out.push_str(&format!(
            "  balance: {}  mode: {}  servers: {}  health-check options: {}\n",
            balance,
            mode,
            servers.len(),
            health
        ));
        if let Some(t) = get_directive(b, "timeout server") {
            out.push_str(&format!("  timeout server: {}\n", t));
        }
        if let Some(t) = get_directive(b, "timeout connect") {
            out.push_str(&format!("  timeout connect: {}\n", t));
        }
        // Cookie
        if let Some(c) = get_directive(b, "cookie") {
            out.push_str(&format!("  cookie: {}\n", c));
        }
        // Options
        let opts = get_all_directives(b, "option");
        if !opts.is_empty() {
            let opt_str: Vec<_> = opts.iter().take(4).copied().collect();
            out.push_str(&format!("  options: {}\n", opt_str.join(", ")));
        }
        out.push('\n');
    }
    Ok(out)
}

fn parse_server_line(line: &str) -> (String, String, String) {
    let parts: Vec<&str> = line
        .splitn(3, char::is_whitespace)
        .filter(|s| !s.is_empty())
        .collect();
    let name = parts.first().unwrap_or(&"").to_string();
    let addr = parts.get(1).unwrap_or(&"").to_string();
    let opts = parts.get(2).unwrap_or(&"").to_string();
    (name, addr, opts)
}

fn action_servers(sections: &[Section], filter: Option<&str>) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("## Servers\n\n");
    let be: Vec<_> = sections
        .iter()
        .filter(|s| s.kind == "backend" || s.kind == "listen")
        .filter(|s| filter.map(|f| s.name == f).unwrap_or(true))
        .collect();

    if be.is_empty() {
        return Ok(format!(
            "No backends found{}.\n",
            filter
                .map(|f| format!(" named '{}'", f))
                .unwrap_or_default()
        ));
    }

    for b in &be {
        let servers = get_all_directives(b, "server");
        out.push_str(&format!("**{}** ({} server(s))\n", b.name, servers.len()));
        out.push_str(&format!(
            "{:<20} {:<25} {}\n",
            "Name", "Address:Port", "Options"
        ));
        out.push_str(&format!(
            "{:<20} {:<25} {}\n",
            "─".repeat(19),
            "─".repeat(24),
            "─".repeat(30)
        ));
        for sv in &servers {
            let (name, addr, opts) = parse_server_line(sv);
            let opts_brief: String = opts.chars().take(50).collect();
            let ellipsis = if opts.len() > 50 { "…" } else { "" };
            out.push_str(&format!(
                "{:<20} {:<25} {}{}\n",
                name, addr, opts_brief, ellipsis
            ));
        }
        out.push('\n');
    }
    Ok(out)
}

fn action_validate(sections: &[Section]) -> Result<String, String> {
    let mut issues: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Check global exists
    if !sections.iter().any(|s| s.kind == "global") {
        warnings.push("No 'global' section found".to_string());
    }
    if !sections.iter().any(|s| s.kind == "defaults") {
        warnings
            .push("No 'defaults' section found — all timeouts must be set per-section".to_string());
    }

    // Collect backend names
    let backend_names: std::collections::HashSet<String> = sections
        .iter()
        .filter(|s| s.kind == "backend" || s.kind == "listen")
        .map(|s| s.name.clone())
        .collect();

    let frontend_names: std::collections::HashSet<String> = sections
        .iter()
        .filter(|s| s.kind == "frontend" || s.kind == "listen")
        .map(|s| s.name.clone())
        .collect();

    for fe in sections.iter().filter(|s| s.kind == "frontend") {
        // default_backend must exist
        if let Some(db) = get_directive(fe, "default_backend") {
            if !backend_names.contains(db) {
                issues.push(format!(
                    "frontend '{}': default_backend '{}' is not defined",
                    fe.name, db
                ));
            }
        } else {
            // No default_backend — must have use_backend for all paths
            let ubs = get_all_directives(fe, "use_backend").len();
            if ubs == 0 {
                warnings.push(format!(
                    "frontend '{}': no default_backend and no use_backend rules",
                    fe.name
                ));
            }
        }

        // Check use_backend references exist
        for ub in get_all_directives(fe, "use_backend") {
            let backend_ref = ub.split_whitespace().next().unwrap_or("");
            if !backend_ref.is_empty() && !backend_names.contains(backend_ref) {
                issues.push(format!(
                    "frontend '{}': use_backend '{}' is not defined",
                    fe.name, backend_ref
                ));
            }
        }

        // Check bind is present
        if get_all_directives(fe, "bind").is_empty() {
            issues.push(format!("frontend '{}': no 'bind' directive", fe.name));
        }
    }

    for be in sections.iter().filter(|s| s.kind == "backend") {
        let servers = get_all_directives(be, "server");
        if servers.is_empty() {
            warnings.push(format!("backend '{}': no servers defined", be.name));
        }
        // Check health check consistency
        let has_httpchk = get_all_directives(be, "option")
            .iter()
            .any(|o| o.contains("httpchk"));
        let has_check = servers.iter().any(|sv| sv.contains("check"));
        if has_httpchk && !has_check {
            warnings.push(format!(
                "backend '{}': option httpchk set but servers don't have 'check' keyword",
                be.name
            ));
        }
        // Balance algorithm check
        let balance = get_directive(be, "balance").unwrap_or("roundrobin");
        let valid_algos = [
            "roundrobin",
            "leastconn",
            "first",
            "source",
            "uri",
            "url_param",
            "hdr",
            "random",
            "rdp-cookie",
            "static-rr",
            "sticky-cache",
        ];
        let algo_name = balance.split_whitespace().next().unwrap_or(balance);
        if !valid_algos.contains(&algo_name) {
            warnings.push(format!(
                "backend '{}': unknown balance algorithm '{}'",
                be.name, algo_name
            ));
        }
    }

    // Duplicate section names
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for s in sections {
        if s.kind != "global" && s.kind != "defaults" {
            *name_counts
                .entry(format!("{}:{}", s.kind, s.name))
                .or_insert(0) += 1;
        }
    }
    for (key, count) in &name_counts {
        if *count > 1 {
            issues.push(format!(
                "Duplicate section: {} (appears {} times)",
                key, count
            ));
        }
    }

    let verdict = if issues.is_empty() {
        "VALID"
    } else {
        "INVALID"
    };
    let mut out = String::new();
    out.push_str(&format!("## Validate — {}\n\n", verdict));
    out.push_str(&format!(
        "Sections: {}  Issues: {}  Warnings: {}\n\n",
        sections.len(),
        issues.len(),
        warnings.len()
    ));

    if !issues.is_empty() {
        out.push_str("**Issues (must fix):**\n");
        for i in &issues {
            out.push_str(&format!("  ✗ {}\n", i));
        }
        out.push('\n');
    }
    if !warnings.is_empty() {
        out.push_str("**Warnings:**\n");
        for w in &warnings {
            out.push_str(&format!("  ⚠ {}\n", w));
        }
        out.push('\n');
    }
    if issues.is_empty() && warnings.is_empty() {
        out.push_str("✓ No issues found.\n");
    }
    // Unused backends
    let used_backends: std::collections::HashSet<String> = sections
        .iter()
        .filter(|s| s.kind == "frontend")
        .flat_map(|s| {
            let mut refs: Vec<String> = Vec::new();
            if let Some(db) = get_directive(s, "default_backend") {
                refs.push(db.to_string());
            }
            for ub in get_all_directives(s, "use_backend") {
                if let Some(name) = ub.split_whitespace().next() {
                    refs.push(name.to_string());
                }
            }
            refs
        })
        .collect();
    let unreferenced: Vec<_> = backend_names
        .iter()
        .filter(|n| !used_backends.contains(*n) && !frontend_names.contains(*n))
        .collect();
    if !unreferenced.is_empty() {
        out.push_str("**Unreferenced backends (no frontend points to them):**\n");
        for u in &unreferenced {
            out.push_str(&format!("  ⚠ {}\n", u));
        }
    }
    Ok(out)
}
