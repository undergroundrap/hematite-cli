use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["parse", "chains", "rules", "ports", "summary"],
                "description": "Action: parse (default), chains, rules, ports, summary"
            },
            "text": { "type": "string", "description": "Inline iptables-save output" },
            "iptables": { "type": "string", "description": "Inline iptables-save output (alias)" },
            "file": { "type": "string", "description": "Path to an iptables-save file" },
            "table": { "type": "string", "description": "Filter by table: filter/nat/mangle/raw" },
            "chain": { "type": "string", "description": "Filter by chain name" },
            "target": { "type": "string", "description": "Filter by rule target: ACCEPT/DROP/REJECT/etc." },
            "query": { "type": "string", "description": "Filter rules by text substring" }
        }
    })
}

fn load_text(args: &Value) -> Result<String, String> {
    if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(f).map_err(|e| format!("Cannot read '{}': {}", f, e))
    } else if let Some(t) = args.get("iptables").and_then(|v| v.as_str()) {
        Ok(t.to_string())
    } else if let Some(t) = args.get("text").and_then(|v| v.as_str()) {
        Ok(t.to_string())
    } else {
        Err("Provide 'iptables'/'text' (inline iptables-save output) or 'file'.".to_string())
    }
}

#[derive(Debug, Default)]
struct IpTable {
    name: String,
    chains: Vec<Chain>,
}

#[derive(Clone, Debug, Default)]
struct Chain {
    name: String,
    policy: String,
    packet_count: u64,
    byte_count: u64,
    rules: Vec<Rule>,
}

#[derive(Clone, Debug, Default)]
struct Rule {
    raw: String,
    chain: String,
    target: String,
    protocol: String,
    src: String,
    dst: String,
    in_iface: String,
    out_iface: String,
    dport: String,
    sport: String,
    comment: String,
    extra: String,
}

fn parse_count(s: &str) -> u64 {
    let s = s.trim_start_matches('[').trim_end_matches(']');
    s.split(':')
        .next()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

fn parse_byte(s: &str) -> u64 {
    let s = s.trim_start_matches('[').trim_end_matches(']');
    s.split(':')
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

fn extract_flag<'a>(parts: &'a [&str], flag: &str) -> Option<&'a str> {
    parts.windows(2).find(|w| w[0] == flag).map(|w| w[1])
}

fn parse_rule_line(line: &str, table_chain_names: &[String]) -> Option<Rule> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() || parts[0] != "-A" {
        return None;
    }

    let chain = parts.get(1)?.to_string();
    if !table_chain_names.contains(&chain) {
        return None;
    }

    let mut rule = Rule {
        raw: line.to_string(),
        chain: chain.clone(),
        ..Default::default()
    };

    let mut i = 2usize;
    while i < parts.len() {
        match parts[i] {
            "-j" | "--jump" => {
                if i + 1 < parts.len() {
                    rule.target = parts[i + 1].to_string();
                    i += 2;
                    continue;
                }
            }
            "-p" | "--protocol" => {
                if i + 1 < parts.len() {
                    rule.protocol = parts[i + 1].to_string();
                    i += 2;
                    continue;
                }
            }
            "-s" | "--source" => {
                if i + 1 < parts.len() {
                    rule.src = parts[i + 1].to_string();
                    i += 2;
                    continue;
                }
            }
            "-d" | "--destination" => {
                if i + 1 < parts.len() {
                    rule.dst = parts[i + 1].to_string();
                    i += 2;
                    continue;
                }
            }
            "-i" | "--in-interface" => {
                if i + 1 < parts.len() {
                    rule.in_iface = parts[i + 1].to_string();
                    i += 2;
                    continue;
                }
            }
            "-o" | "--out-interface" => {
                if i + 1 < parts.len() {
                    rule.out_iface = parts[i + 1].to_string();
                    i += 2;
                    continue;
                }
            }
            "--dport" | "--destination-port" => {
                if i + 1 < parts.len() {
                    rule.dport = parts[i + 1].to_string();
                    i += 2;
                    continue;
                }
            }
            "--sport" | "--source-port" => {
                if i + 1 < parts.len() {
                    rule.sport = parts[i + 1].to_string();
                    i += 2;
                    continue;
                }
            }
            "--comment" => {
                if i + 1 < parts.len() {
                    rule.comment = parts[i + 1].trim_matches('"').to_string();
                    i += 2;
                    continue;
                }
            }
            _ => {
                rule.extra.push(' ');
                rule.extra.push_str(parts[i]);
            }
        }
        i += 1;
    }

    let _ = extract_flag(&parts, "--dports");
    if rule.target.is_empty() {
        rule.target = "(no-jump)".to_string();
    }
    Some(rule)
}

fn parse_tables(text: &str) -> Vec<IpTable> {
    let mut tables: Vec<IpTable> = Vec::new();
    let mut current_table: Option<IpTable> = None;
    let mut chain_names: Vec<String> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if line.starts_with('*') {
            if let Some(t) = current_table.take() {
                tables.push(t);
            }
            chain_names.clear();
            current_table = Some(IpTable {
                name: line[1..].to_string(),
                ..Default::default()
            });
        } else if line == "COMMIT" {
            if let Some(t) = current_table.take() {
                tables.push(t);
            }
            chain_names.clear();
        } else if line.starts_with(':') {
            // :CHAIN POLICY [packets:bytes]
            let parts: Vec<&str> = line.split_whitespace().collect();
            let name = parts
                .get(0)
                .map(|s| s.trim_start_matches(':'))
                .unwrap_or("")
                .to_string();
            let policy = parts.get(1).unwrap_or(&"-").to_string();
            let counts = parts.get(2).unwrap_or(&"[0:0]");
            chain_names.push(name.clone());
            if let Some(t) = current_table.as_mut() {
                t.chains.push(Chain {
                    name: name.clone(),
                    policy: policy.to_string(),
                    packet_count: parse_count(counts),
                    byte_count: parse_byte(counts),
                    rules: Vec::new(),
                });
            }
        } else if line.starts_with("-A") {
            if let Some(t) = current_table.as_mut() {
                if let Some(rule) = parse_rule_line(line, &chain_names) {
                    let chain_name = rule.chain.clone();
                    if let Some(chain) = t.chains.iter_mut().find(|c| c.name == chain_name) {
                        chain.rules.push(rule);
                    }
                }
            }
        }
    }
    tables
}

fn target_icon(t: &str) -> &'static str {
    match t {
        "ACCEPT" => "✓",
        "DROP" | "REJECT" => "✗",
        "LOG" => "L",
        "RETURN" => "↩",
        "MASQUERADE" | "SNAT" | "DNAT" => "⇄",
        _ => "→",
    }
}

fn action_parse(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let tables = parse_tables(&text);
    let table_filter = args.get("table").and_then(|v| v.as_str());
    let chain_filter = args
        .get("chain")
        .and_then(|v| v.as_str())
        .map(|s| s.to_uppercase());
    let target_filter = args
        .get("target")
        .and_then(|v| v.as_str())
        .map(|s| s.to_uppercase());
    let query = args.get("query").and_then(|v| v.as_str());

    if tables.is_empty() {
        return Ok(
            "No iptables tables found. Provide output from `iptables-save` or `ip6tables-save`."
                .to_string(),
        );
    }

    let mut out = String::new();
    for t in tables
        .iter()
        .filter(|t| table_filter.map_or(true, |f| t.name == f))
    {
        out.push_str(&format!("\n## Table: {}\n", t.name.to_uppercase()));
        for chain in &t.chains {
            if chain_filter.as_deref().map_or(false, |f| chain.name != f) {
                continue;
            }
            let filtered_rules: Vec<&Rule> = chain
                .rules
                .iter()
                .filter(|r| target_filter.as_deref().map_or(true, |tf| r.target == tf))
                .filter(|r| {
                    query.map_or(true, |q| r.raw.to_lowercase().contains(&q.to_lowercase()))
                })
                .collect();

            let policy_disp = if chain.policy == "-" {
                "(no policy)".to_string()
            } else {
                chain.policy.clone()
            };
            out.push_str(&format!(
                "\n### Chain {} [policy: {}] ({} rules)\n",
                chain.name,
                policy_disp,
                filtered_rules.len()
            ));
            if filtered_rules.is_empty() {
                continue;
            }

            for r in &filtered_rules {
                let icon = target_icon(&r.target);
                let mut desc_parts = Vec::new();
                if !r.protocol.is_empty() {
                    desc_parts.push(r.protocol.clone());
                }
                if !r.src.is_empty() && r.src != "0.0.0.0/0" {
                    desc_parts.push(format!("src:{}", r.src));
                }
                if !r.dst.is_empty() && r.dst != "0.0.0.0/0" {
                    desc_parts.push(format!("dst:{}", r.dst));
                }
                if !r.dport.is_empty() {
                    desc_parts.push(format!("dport:{}", r.dport));
                }
                if !r.sport.is_empty() {
                    desc_parts.push(format!("sport:{}", r.sport));
                }
                if !r.in_iface.is_empty() {
                    desc_parts.push(format!("in:{}", r.in_iface));
                }
                if !r.out_iface.is_empty() {
                    desc_parts.push(format!("out:{}", r.out_iface));
                }
                if !r.comment.is_empty() {
                    desc_parts.push(format!("# {}", r.comment));
                }
                let desc = if desc_parts.is_empty() {
                    "(any)".to_string()
                } else {
                    desc_parts.join("  ")
                };
                out.push_str(&format!("  {} {:<10} {}\n", icon, r.target, desc));
            }
        }
    }
    Ok(out.trim_start().to_string())
}

fn action_chains(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let tables = parse_tables(&text);
    let table_filter = args.get("table").and_then(|v| v.as_str());

    let mut out = format!(
        "{:<8} {:<20} {:<10} {:<8} {}\n",
        "TABLE", "CHAIN", "POLICY", "RULES", "PACKETS"
    );
    out.push_str(&format!("{}\n", "-".repeat(65)));

    for t in tables
        .iter()
        .filter(|t| table_filter.map_or(true, |f| t.name == f))
    {
        for chain in &t.chains {
            let policy = if chain.policy == "-" {
                "-".to_string()
            } else {
                chain.policy.clone()
            };
            out.push_str(&format!(
                "{:<8} {:<20} {:<10} {:<8} {}\n",
                t.name,
                chain.name,
                policy,
                chain.rules.len(),
                chain.packet_count
            ));
        }
    }
    Ok(out)
}

fn action_rules(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let tables = parse_tables(&text);
    let table_filter = args.get("table").and_then(|v| v.as_str());
    let chain_filter = args
        .get("chain")
        .and_then(|v| v.as_str())
        .map(|s| s.to_uppercase());
    let target_filter = args
        .get("target")
        .and_then(|v| v.as_str())
        .map(|s| s.to_uppercase());
    let query = args.get("query").and_then(|v| v.as_str());

    let mut all_rules: Vec<(String, &Rule)> = Vec::new();
    for t in tables
        .iter()
        .filter(|t| table_filter.map_or(true, |f| t.name == f))
    {
        for chain in &t.chains {
            if chain_filter.as_deref().map_or(false, |f| chain.name != f) {
                continue;
            }
            for r in &chain.rules {
                if target_filter.as_deref().map_or(false, |tf| r.target != tf) {
                    continue;
                }
                if query.map_or(false, |q| !r.raw.to_lowercase().contains(&q.to_lowercase())) {
                    continue;
                }
                all_rules.push((t.name.clone(), r));
            }
        }
    }

    if all_rules.is_empty() {
        return Ok("No matching rules found.".to_string());
    }

    let mut out = format!(
        "{:<8} {:<12} {:<10} {}\n",
        "TABLE", "CHAIN", "TARGET", "MATCH"
    );
    out.push_str(&format!("{}\n", "-".repeat(80)));
    for (table, r) in &all_rules {
        let mut parts = Vec::new();
        if !r.protocol.is_empty() {
            parts.push(r.protocol.as_str());
        }
        if !r.dport.is_empty() {
            parts.push(&r.dport);
        }
        if !r.src.is_empty() && r.src != "0.0.0.0/0" {
            parts.push(&r.src);
        }
        let match_str = if parts.is_empty() {
            "any".to_string()
        } else {
            parts.join(" ")
        };
        out.push_str(&format!(
            "{:<8} {:<12} {:<10} {}\n",
            table, r.chain, r.target, match_str
        ));
    }
    out.push_str(&format!("\nTotal: {} rule(s)\n", all_rules.len()));
    Ok(out)
}

fn action_ports(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let tables = parse_tables(&text);

    let mut port_rules: Vec<(String, String, String, String, String)> = Vec::new();
    for t in &tables {
        for chain in &t.chains {
            for r in &chain.rules {
                if !r.dport.is_empty() || !r.sport.is_empty() {
                    port_rules.push((
                        t.name.clone(),
                        chain.name.clone(),
                        r.protocol.clone(),
                        r.dport.clone(),
                        r.target.clone(),
                    ));
                }
            }
        }
    }

    if port_rules.is_empty() {
        return Ok("No port-specific rules found.".to_string());
    }

    let mut out = format!(
        "{:<8} {:<12} {:<6} {:<20} {}\n",
        "TABLE", "CHAIN", "PROTO", "DPORT", "TARGET"
    );
    out.push_str(&format!("{}\n", "-".repeat(65)));
    for (table, chain, proto, dport, target) in &port_rules {
        out.push_str(&format!(
            "{:<8} {:<12} {:<6} {:<20} {}\n",
            table, chain, proto, dport, target
        ));
    }
    Ok(out)
}

fn action_summary(args: &Value) -> Result<String, String> {
    let text = load_text(args)?;
    let tables = parse_tables(&text);

    let mut out = String::new();
    out.push_str("## iptables Summary\n\n");

    for t in &tables {
        let total_rules: usize = t.chains.iter().map(|c| c.rules.len()).sum();
        out.push_str(&format!(
            "### Table: {} ({} chains, {} rules)\n",
            t.name.to_uppercase(),
            t.chains.len(),
            total_rules
        ));
        for chain in &t.chains {
            let accept = chain.rules.iter().filter(|r| r.target == "ACCEPT").count();
            let drop = chain
                .rules
                .iter()
                .filter(|r| r.target == "DROP" || r.target == "REJECT")
                .count();
            let log = chain.rules.iter().filter(|r| r.target == "LOG").count();
            let policy = if chain.policy == "-" {
                "-".to_string()
            } else {
                chain.policy.clone()
            };
            out.push_str(&format!(
                "  {:20} policy={:<8} rules={:<4} ACCEPT={} DROP/REJECT={} LOG={}\n",
                chain.name,
                policy,
                chain.rules.len(),
                accept,
                drop,
                log
            ));
        }
        out.push('\n');
    }

    // Risk observations
    let all_rules: Vec<&Rule> = tables
        .iter()
        .flat_map(|t| t.chains.iter().flat_map(|c| c.rules.iter()))
        .collect();
    let open_all: Vec<&Rule> = all_rules
        .iter()
        .filter(|r| {
            r.target == "ACCEPT" && r.src.is_empty() && r.dst.is_empty() && r.dport.is_empty()
        })
        .copied()
        .collect();
    let forward_chain: Vec<&Rule> = tables
        .iter()
        .filter(|t| t.name == "filter")
        .flat_map(|t| t.chains.iter())
        .filter(|c| c.name == "FORWARD")
        .flat_map(|c| c.rules.iter())
        .collect();
    let forward_accepts = forward_chain
        .iter()
        .filter(|r| r.target == "ACCEPT")
        .count();

    if !open_all.is_empty() {
        out.push_str(&format!(
            "⚠ {} broad ACCEPT rule(s) with no source/dest/port restriction\n",
            open_all.len()
        ));
    }
    if forward_accepts > 0 {
        out.push_str(&format!(
            "ℹ FORWARD chain has {} ACCEPT rule(s) — this host may be routing traffic\n",
            forward_accepts
        ));
    }
    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "chains" => action_chains(args),
        "rules" => action_rules(args),
        "ports" => action_ports(args),
        "summary" => action_summary(args),
        _ => action_parse(args),
    }
}
