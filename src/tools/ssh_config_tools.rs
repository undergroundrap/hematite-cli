use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("host").is_some() {
        "get".to_string()
    } else {
        "list".to_string()
    };
    match action.as_str() {
        "list" => list_action(args),
        "get" => get_action(args),
        "explain" => explain_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: list, get, explain, validate",
            action
        )),
    }
}

fn get_config(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("config"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the SSH config content as a string".to_string())
}

#[derive(Debug, Clone)]
struct SshHost {
    pattern: String,
    options: Vec<(String, String)>,
}

fn parse_ssh_config(text: &str) -> Vec<SshHost> {
    let mut hosts: Vec<SshHost> = Vec::new();
    let mut current: Option<SshHost> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Host block start
        let lower = trimmed.to_lowercase();
        if lower.starts_with("host ") {
            if let Some(h) = current.take() {
                hosts.push(h);
            }
            let pattern = trimmed[5..].trim().to_string();
            current = Some(SshHost {
                pattern,
                options: Vec::new(),
            });
            continue;
        }
        if lower == "host" {
            if let Some(h) = current.take() {
                hosts.push(h);
            }
            current = Some(SshHost {
                pattern: String::new(),
                options: Vec::new(),
            });
            continue;
        }

        // Option inside a block
        if let Some(ref mut host) = current {
            if let Some(space_pos) = trimmed.find(|c: char| c.is_whitespace()) {
                let key = trimmed[..space_pos].to_string();
                let val = trimmed[space_pos..].trim().to_string();
                host.options.push((key, val));
            }
        } else {
            // Global options before any Host block
            if hosts.is_empty() {
                // Treat as a global "*" block
                if let Some(space_pos) = trimmed.find(|c: char| c.is_whitespace()) {
                    let key = trimmed[..space_pos].to_string();
                    let val = trimmed[space_pos..].trim().to_string();
                    // Create implicit global host
                    let global = hosts.iter_mut().find(|h| h.pattern == "*");
                    if let Some(g) = global {
                        g.options.push((key, val));
                    } else {
                        hosts.push(SshHost {
                            pattern: "*".to_string(),
                            options: vec![(key, val)],
                        });
                    }
                }
            }
        }
    }

    if let Some(h) = current {
        hosts.push(h);
    }

    hosts
}

fn get_option<'a>(host: &'a SshHost, key: &str) -> Option<&'a str> {
    host.options
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

fn explain_option(key: &str, value: &str) -> String {
    match key.to_lowercase().as_str() {
        "hostname" => format!("Real hostname to connect to: {}", value),
        "user" => format!("Login user: {}", value),
        "port" => format!("SSH port: {} (default is 22)", value),
        "identityfile" => format!("Private key file: {}", value),
        "identitiesonly" => format!("Use only specified identity files: {}", value),
        "forwardagent" => format!(
            "Forward SSH agent: {} — allows using local keys on remote hosts",
            value
        ),
        "serveralivecountmax" => format!(
            "Kill connection after {} unanswered keepalive packets",
            value
        ),
        "serveraliveinterval" => format!("Send keepalive every {} seconds", value),
        "proxyjump" => format!(
            "Jump through host: {} — SSH will tunnel via this host",
            value
        ),
        "proxycommand" => format!("Run command to connect: {}", value),
        "stricthostkeychecking" => {
            let desc = match value.to_lowercase().as_str() {
                "yes" => "refuse connections to unknown hosts",
                "no" => "automatically accept new host keys (less secure)",
                "accept-new" => "accept new keys, reject changed keys",
                _ => "custom behavior",
            };
            format!("Host key checking: {} — {}", value, desc)
        }
        "compression" => format!("Data compression: {}", value),
        "controlmaster" => format!(
            "Connection multiplexing: {} — shares one TCP connection",
            value
        ),
        "controlpath" => format!("Socket path for connection sharing: {}", value),
        "controlpersist" => format!(
            "Keep master connection open for {} after last client exits",
            value
        ),
        "addkeystoagent" => format!("Add keys to SSH agent automatically: {}", value),
        "usekeys" | "preferredauthentications" => format!("Auth methods: {}", value),
        "loglevel" => format!("Logging verbosity: {}", value),
        "requesttty" => format!("TTY allocation: {}", value),
        "remoteforward" | "localforward" => format!("Port forward: {}", value),
        "dynamicforward" => format!("SOCKS proxy on local port: {}", value),
        "batchmode" => format!("Batch mode (no prompts): {}", value),
        "connecttimeout" => format!("Connection timeout: {} seconds", value),
        "tcpkeepalive" => format!("TCP keepalive: {}", value),
        "hashknownhosts" => format!("Hash known_hosts entries: {}", value),
        "kexalgorithms" => format!("Key exchange algorithms: {}", value),
        "ciphers" => format!("Encryption ciphers: {}", value),
        "macs" => format!("Message authentication codes: {}", value),
        "hostkeyalgorithms" => format!("Accepted host key types: {}", value),
        _ => format!("{} = {}", key, value),
    }
}

fn list_action(args: &Value) -> Result<String, String> {
    let text = get_config(args)?;
    let hosts = parse_ssh_config(&text);

    let mut out = format!(
        "SSH Config  [{} host block(s)]\n{}\n\n",
        hosts.len(),
        "=".repeat(44)
    );

    for h in &hosts {
        let hostname = get_option(h, "HostName").unwrap_or("(same as pattern)");
        let user = get_option(h, "User").unwrap_or("(current user)");
        let port = get_option(h, "Port").unwrap_or("22");
        let identity = get_option(h, "IdentityFile").unwrap_or("(default)");
        let jump = get_option(h, "ProxyJump");
        out += &format!("Host {}\n", h.pattern);
        out += &format!("  HostName:     {}\n", hostname);
        out += &format!("  User:         {}\n", user);
        out += &format!("  Port:         {}\n", port);
        out += &format!("  IdentityFile: {}\n", identity);
        if let Some(j) = jump {
            out += &format!("  ProxyJump:    {}\n", j);
        }
        out += &format!("  ({} option(s) total)\n\n", h.options.len());
    }
    Ok(out)
}

fn get_action(args: &Value) -> Result<String, String> {
    let text = get_config(args)?;
    let query = args
        .get("host")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'host' — the Host pattern to look up")?
        .to_lowercase();

    let hosts = parse_ssh_config(&text);
    let host = hosts
        .iter()
        .find(|h| h.pattern.to_lowercase().contains(&query) || h.pattern.to_lowercase() == query)
        .ok_or_else(|| {
            format!(
                "Host '{}' not found. Use action='list' to see all hosts.",
                query
            )
        })?;

    let mut out = format!("Host {}\n{}\n\n", host.pattern, "=".repeat(44));
    for (k, v) in &host.options {
        out += &format!("{:<24} {}\n", k, v);
    }
    Ok(out)
}

fn explain_action(args: &Value) -> Result<String, String> {
    let text = get_config(args)?;
    let hosts = parse_ssh_config(&text);

    let filter = args.get("host").and_then(|v| v.as_str());

    let mut out = format!("SSH Config Explained\n{}\n\n", "=".repeat(44));

    for h in &hosts {
        if let Some(f) = filter {
            if !h.pattern.to_lowercase().contains(&f.to_lowercase()) {
                continue;
            }
        }
        out += &format!("Host {}\n", h.pattern);
        for (k, v) in &h.options {
            out += &format!("  {}\n", explain_option(k, v));
        }
        out += "\n";
    }
    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_config(args)?;
    let hosts = parse_ssh_config(&text);
    let mut warnings: Vec<String> = Vec::new();

    let mut seen_patterns: Vec<String> = Vec::new();
    for h in &hosts {
        // Check for duplicate host patterns
        if seen_patterns.contains(&h.pattern) {
            warnings.push(format!("Duplicate Host pattern: '{}'", h.pattern));
        }
        seen_patterns.push(h.pattern.clone());

        // Warn on StrictHostKeyChecking = no
        if get_option(h, "StrictHostKeyChecking")
            .map(|v| v.to_lowercase() == "no")
            .unwrap_or(false)
        {
            warnings.push(format!(
                "Host '{}': StrictHostKeyChecking=no accepts any host key (MITM risk)",
                h.pattern
            ));
        }

        // Warn on port not 22
        if let Some(port) = get_option(h, "Port") {
            if port != "22" && !port.chars().all(|c| c.is_ascii_digit()) {
                warnings.push(format!(
                    "Host '{}': Port '{}' does not appear numeric",
                    h.pattern, port
                ));
            }
        }

        // IdentityFile with ~ not expanded — informational
        if let Some(id) = get_option(h, "IdentityFile") {
            if !id.starts_with('~') && !id.starts_with('/') && !id.starts_with('%') {
                warnings.push(format!(
                    "Host '{}': IdentityFile '{}' is a relative path — may not resolve correctly",
                    h.pattern, id
                ));
            }
        }
    }

    let mut out = format!("SSH Config Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!("{} host block(s) parsed.\n", hosts.len());
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
