use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["parse", "hosts", "ports", "services", "summary"],
                "description": "Action: parse (default), hosts, ports, services, summary"
            },
            "xml": { "type": "string", "description": "Inline nmap XML output" },
            "file": { "type": "string", "description": "Path to nmap XML file" },
            "host": { "type": "string", "description": "Filter by host IP or hostname" },
            "state": { "type": "string", "description": "Filter ports by state: open/closed/filtered" },
            "limit": { "type": "integer", "description": "Max rows to return" }
        }
    })
}

#[derive(Debug, Default)]
struct NmapHost {
    ip: String,
    hostname: String,
    status: String,
    os_guess: String,
    ports: Vec<NmapPort>,
}

#[derive(Debug, Default)]
struct NmapPort {
    number: u16,
    protocol: String,
    state: String,
    service: String,
    version: String,
    extra_info: String,
}

fn load_xml(args: &Value) -> Result<String, String> {
    if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(f).map_err(|e| format!("Cannot read file '{}': {}", f, e))
    } else if let Some(x) = args.get("xml").and_then(|v| v.as_str()) {
        Ok(x.to_string())
    } else {
        Err("Provide 'xml' (inline) or 'file' (path to .xml).".to_string())
    }
}

fn parse_xml(xml: &str) -> Vec<NmapHost> {
    let mut hosts: Vec<NmapHost> = Vec::new();
    let mut current: Option<NmapHost> = None;
    let mut in_host = false;
    let mut in_port = false;
    let mut current_port: Option<NmapPort> = None;
    let mut in_script = false;

    for line in xml.lines() {
        let t = line.trim();

        if t.starts_with("<host ") || t == "<host>" {
            in_host = true;
            current = Some(NmapHost::default());
        } else if t == "</host>" {
            if let Some(mut h) = current.take() {
                if let Some(p) = current_port.take() {
                    h.ports.push(p);
                }
                hosts.push(h);
            }
            in_host = false;
            in_port = false;
            current_port = None;
        } else if in_host {
            if t.starts_with("<status ") {
                if let Some(state) = attr_val(t, "state") {
                    if let Some(h) = current.as_mut() {
                        h.status = state;
                    }
                }
            } else if t.starts_with("<address ") {
                if attr_val(t, "addrtype").as_deref() != Some("mac") {
                    if let Some(addr) = attr_val(t, "addr") {
                        if let Some(h) = current.as_mut() {
                            if h.ip.is_empty() {
                                h.ip = addr;
                            }
                        }
                    }
                }
            } else if t.starts_with("<hostname ") {
                if let Some(name) = attr_val(t, "name") {
                    if let Some(h) = current.as_mut() {
                        if h.hostname.is_empty() {
                            h.hostname = name;
                        }
                    }
                }
            } else if t.starts_with("<osmatch ") {
                if let Some(name) = attr_val(t, "name") {
                    if let Some(h) = current.as_mut() {
                        if h.os_guess.is_empty() {
                            h.os_guess = name;
                        }
                    }
                }
            } else if t.starts_with("<port ") {
                if let Some(p) = current_port.take() {
                    if let Some(h) = current.as_mut() {
                        h.ports.push(p);
                    }
                }
                let mut port = NmapPort::default();
                if let Some(pn) = attr_val(t, "portid") {
                    port.number = pn.parse().unwrap_or(0);
                }
                if let Some(proto) = attr_val(t, "protocol") {
                    port.protocol = proto;
                }
                in_port = true;
                in_script = false;
                current_port = Some(port);
            } else if t == "</port>" {
                if let Some(p) = current_port.take() {
                    if let Some(h) = current.as_mut() {
                        h.ports.push(p);
                    }
                }
                in_port = false;
                in_script = false;
            } else if in_port && !in_script {
                if t.starts_with("<state ") {
                    if let Some(state) = attr_val(t, "state") {
                        if let Some(p) = current_port.as_mut() {
                            p.state = state;
                        }
                    }
                } else if t.starts_with("<service ") {
                    if let Some(p) = current_port.as_mut() {
                        p.service = attr_val(t, "name").unwrap_or_default();
                        let mut version_parts = Vec::new();
                        if let Some(prod) = attr_val(t, "product") {
                            version_parts.push(prod);
                        }
                        if let Some(ver) = attr_val(t, "version") {
                            version_parts.push(ver);
                        }
                        p.version = version_parts.join(" ");
                        p.extra_info = attr_val(t, "extrainfo").unwrap_or_default();
                    }
                } else if t.starts_with("<script ") {
                    in_script = true;
                } else if t == "</script>" {
                    in_script = false;
                }
            }
        }
    }
    hosts
}

fn attr_val(tag: &str, attr: &str) -> Option<String> {
    let search = format!("{}=\"", attr);
    let pos = tag.find(&search)?;
    let rest = &tag[pos + search.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn filter_host<'a>(hosts: &'a [NmapHost], filter: Option<&str>) -> Vec<&'a NmapHost> {
    match filter {
        None => hosts.iter().collect(),
        Some(f) => hosts
            .iter()
            .filter(|h| h.ip.contains(f) || h.hostname.contains(f))
            .collect(),
    }
}

fn state_icon(s: &str) -> &'static str {
    match s {
        "open" => "✓",
        "closed" => "✗",
        "filtered" => "?",
        _ => "-",
    }
}

fn action_parse(args: &Value) -> Result<String, String> {
    let xml = load_xml(args)?;
    let hosts = parse_xml(&xml);
    let host_filter = args.get("host").and_then(|v| v.as_str());
    let state_filter = args.get("state").and_then(|v| v.as_str());
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(u64::MAX);

    let filtered = filter_host(&hosts, host_filter);

    if filtered.is_empty() {
        return Ok("No hosts found in nmap output.".to_string());
    }

    let mut out = String::new();
    for h in filtered.into_iter().take(limit as usize) {
        let display = if h.hostname.is_empty() {
            h.ip.clone()
        } else {
            format!("{} ({})", h.ip, h.hostname)
        };
        out.push_str(&format!("\n## {} — {}\n", display, h.status.to_uppercase()));
        if !h.os_guess.is_empty() {
            out.push_str(&format!("   OS: {}\n", h.os_guess));
        }
        let ports: Vec<&NmapPort> = h
            .ports
            .iter()
            .filter(|p| state_filter.is_none_or(|s| p.state == s))
            .collect();
        if ports.is_empty() {
            out.push_str("   No matching ports.\n");
        } else {
            out.push_str(&format!(
                "   {:<6} {:<5} {:<10} {:<12} {}\n",
                "PORT", "PROTO", "STATE", "SERVICE", "VERSION"
            ));
            out.push_str(&format!("   {}\n", "-".repeat(60)));
            for p in ports {
                let icon = state_icon(&p.state);
                let ver = if p.version.is_empty() {
                    p.extra_info.clone()
                } else {
                    p.version.clone()
                };
                out.push_str(&format!(
                    "   {:<6} {:<5} {} {:<8} {:<12} {}\n",
                    p.number, p.protocol, icon, p.state, p.service, ver
                ));
            }
        }
    }
    Ok(out.trim_start().to_string())
}

fn action_hosts(args: &Value) -> Result<String, String> {
    let xml = load_xml(args)?;
    let hosts = parse_xml(&xml);
    let host_filter = args.get("host").and_then(|v| v.as_str());
    let filtered = filter_host(&hosts, host_filter);

    if filtered.is_empty() {
        return Ok("No hosts found.".to_string());
    }

    let total = filtered.len();
    let mut out = format!(
        "{:<20} {:<30} {:<8} {:<5} {}\n",
        "IP", "HOSTNAME", "STATUS", "PORTS", "OS"
    );
    out.push_str(&format!("{}\n", "-".repeat(80)));
    for h in filtered {
        let open = h.ports.iter().filter(|p| p.state == "open").count();
        let hostname = if h.hostname.is_empty() {
            "-".to_string()
        } else {
            h.hostname.clone()
        };
        let os = if h.os_guess.is_empty() {
            "-".to_string()
        } else {
            h.os_guess.chars().take(24).collect()
        };
        out.push_str(&format!(
            "{:<20} {:<30} {:<8} {:<5} {}\n",
            h.ip, hostname, h.status, open, os
        ));
    }
    out.push_str(&format!("\nTotal: {} host(s)\n", total));
    Ok(out)
}

fn action_ports(args: &Value) -> Result<String, String> {
    let xml = load_xml(args)?;
    let hosts = parse_xml(&xml);
    let host_filter = args.get("host").and_then(|v| v.as_str());
    let state_filter = args.get("state").and_then(|v| v.as_str()).unwrap_or("open");
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(200);

    let filtered = filter_host(&hosts, host_filter);
    let mut rows: Vec<(String, u16, &str, &str, String)> = Vec::new();

    for h in filtered {
        for p in &h.ports {
            if p.state == state_filter {
                let display = if h.hostname.is_empty() {
                    h.ip.clone()
                } else {
                    format!("{}/{}", h.ip, h.hostname)
                };
                let ver = if p.version.is_empty() {
                    p.extra_info.clone()
                } else {
                    p.version.clone()
                };
                rows.push((display, p.number, &p.protocol, &p.service, ver));
            }
        }
    }

    rows.sort_by_key(|r| r.1);
    rows.truncate(limit as usize);

    if rows.is_empty() {
        return Ok(format!("No {} ports found.", state_filter));
    }

    let mut out = format!(
        "{:<6} {:<5} {:<24} {:<14} {}\n",
        "PORT", "PROTO", "HOST", "SERVICE", "VERSION"
    );
    out.push_str(&format!("{}\n", "-".repeat(70)));
    for (host, port, proto, svc, ver) in &rows {
        out.push_str(&format!(
            "{:<6} {:<5} {:<24} {:<14} {}\n",
            port, proto, host, svc, ver
        ));
    }
    out.push_str(&format!("\n{} {} port(s)\n", rows.len(), state_filter));
    Ok(out)
}

fn action_services(args: &Value) -> Result<String, String> {
    let xml = load_xml(args)?;
    let hosts = parse_xml(&xml);
    let mut service_map: std::collections::BTreeMap<String, Vec<(String, u16)>> =
        std::collections::BTreeMap::new();

    for h in &hosts {
        for p in &h.ports {
            if p.state == "open" {
                let svc = if p.service.is_empty() {
                    "unknown".to_string()
                } else {
                    p.service.clone()
                };
                let display = if h.hostname.is_empty() {
                    h.ip.clone()
                } else {
                    h.hostname.clone()
                };
                service_map
                    .entry(svc)
                    .or_default()
                    .push((display, p.number));
            }
        }
    }

    if service_map.is_empty() {
        return Ok("No open ports found.".to_string());
    }

    let mut out = format!("{:<20} {:<6} {}\n", "SERVICE", "COUNT", "HOSTS:PORTS");
    out.push_str(&format!("{}\n", "-".repeat(70)));
    for (svc, entries) in &service_map {
        let count = entries.len();
        let preview: Vec<String> = entries
            .iter()
            .take(4)
            .map(|(h, p)| format!("{}:{}", h, p))
            .collect();
        let mut preview_str = preview.join("  ");
        if entries.len() > 4 {
            preview_str.push_str(&format!("  (+{})", entries.len() - 4));
        }
        out.push_str(&format!("{:<20} {:<6} {}\n", svc, count, preview_str));
    }
    Ok(out)
}

fn action_summary(args: &Value) -> Result<String, String> {
    let xml = load_xml(args)?;
    let hosts = parse_xml(&xml);

    let total = hosts.len();
    let up = hosts.iter().filter(|h| h.status == "up").count();
    let down = hosts.iter().filter(|h| h.status == "down").count();
    let open_ports: usize = hosts
        .iter()
        .flat_map(|h| h.ports.iter())
        .filter(|p| p.state == "open")
        .count();
    let filtered_ports: usize = hosts
        .iter()
        .flat_map(|h| h.ports.iter())
        .filter(|p| p.state == "filtered")
        .count();

    let mut service_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for h in &hosts {
        for p in &h.ports {
            if p.state == "open" && !p.service.is_empty() {
                *service_counts.entry(p.service.clone()).or_insert(0) += 1;
            }
        }
    }
    let mut service_vec: Vec<(&String, &usize)> = service_counts.iter().collect();
    service_vec.sort_by(|a, b| b.1.cmp(a.1));

    let with_os = hosts.iter().filter(|h| !h.os_guess.is_empty()).count();

    let mut out = String::new();
    out.push_str("## Nmap Scan Summary\n\n");
    out.push_str(&format!("  Hosts total:    {}\n", total));
    out.push_str(&format!("  Hosts up:       {}\n", up));
    if down > 0 {
        out.push_str(&format!("  Hosts down:     {}\n", down));
    }
    out.push_str(&format!("  Open ports:     {}\n", open_ports));
    if filtered_ports > 0 {
        out.push_str(&format!("  Filtered ports: {}\n", filtered_ports));
    }
    out.push_str(&format!("  OS identified:  {}\n", with_os));
    if !service_vec.is_empty() {
        out.push_str("\n## Top Services\n\n");
        for (svc, cnt) in service_vec.iter().take(10) {
            out.push_str(&format!("  {:20} {}\n", svc, cnt));
        }
    }
    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "hosts" => action_hosts(args),
        "ports" => action_ports(args),
        "services" => action_services(args),
        "summary" => action_summary(args),
        _ => action_parse(args),
    }
}
