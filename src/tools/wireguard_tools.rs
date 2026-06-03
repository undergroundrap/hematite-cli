use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde_json::Value;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "wireguard_tools",
        "description": "Parse and validate WireGuard VPN configuration files. Shows [Interface] settings and [Peer] table, validates keys (Curve25519 base64), AllowedIPs CIDRs, and endpoint host:port format. Works offline — no network calls.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "peers", "validate", "keys"],
                    "description": "info (default — interface summary and peer table), peers (detailed per-peer info), validate (check required fields, key validity, CIDR format, port ranges), keys (list all keys with validity check — never shows PrivateKey value)"
                },
                "config": {
                    "type": "string",
                    "description": "WireGuard config content (INI-like with [Interface] and [Peer] sections)"
                },
                "text": {
                    "type": "string",
                    "description": "Alias for 'config'"
                },
                "file": {
                    "type": "string",
                    "description": "Path to a WireGuard .conf file"
                }
            }
        }
    })
}

#[derive(Default)]
struct Interface {
    private_key: Option<String>,
    address: Vec<String>,
    listen_port: Option<String>,
    dns: Vec<String>,
    mtu: Option<String>,
    pre_up: Vec<String>,
    post_up: Vec<String>,
    pre_down: Vec<String>,
    post_down: Vec<String>,
}

#[derive(Default)]
struct Peer {
    public_key: Option<String>,
    preshared_key: Option<String>,
    allowed_ips: Vec<String>,
    endpoint: Option<String>,
    persistent_keepalive: Option<String>,
    name: Option<String>, // from # comment before [Peer]
}

struct Config {
    interface: Interface,
    peers: Vec<Peer>,
}

fn parse_config(text: &str) -> Result<Config, String> {
    let mut iface = Interface::default();
    let mut peers: Vec<Peer> = Vec::new();
    let mut current_peer: Option<Peer> = None;
    let mut in_interface = false;
    let mut pending_name: Option<String> = None;

    for line in text.lines() {
        let line = line.trim();

        if line.is_empty() {
            pending_name = None;
            continue;
        }

        if let Some(comment) = line.strip_prefix('#') {
            // Comments above [Peer] are often friendly names
            pending_name = Some(comment.trim().to_string());
            continue;
        }

        if line.eq_ignore_ascii_case("[Interface]") {
            if let Some(p) = current_peer.take() {
                peers.push(p);
            }
            in_interface = true;
            pending_name = None;
            continue;
        }

        if line.eq_ignore_ascii_case("[Peer]") {
            if let Some(p) = current_peer.take() {
                peers.push(p);
            }
            let mut p = Peer::default();
            p.name = pending_name.take();
            current_peer = Some(p);
            in_interface = false;
            continue;
        }

        pending_name = None;

        let (key, val) = match line.find('=') {
            Some(i) => (line[..i].trim(), line[i + 1..].trim()),
            None => continue,
        };

        if in_interface {
            match key.to_lowercase().as_str() {
                "privatekey" => iface.private_key = Some(val.to_string()),
                "address" => iface
                    .address
                    .extend(val.split(',').map(|s| s.trim().to_string())),
                "listenport" => iface.listen_port = Some(val.to_string()),
                "dns" => iface
                    .dns
                    .extend(val.split(',').map(|s| s.trim().to_string())),
                "mtu" => iface.mtu = Some(val.to_string()),
                "preup" => iface.pre_up.push(val.to_string()),
                "postup" => iface.post_up.push(val.to_string()),
                "predown" => iface.pre_down.push(val.to_string()),
                "postdown" => iface.post_down.push(val.to_string()),
                _ => {}
            }
        } else if let Some(ref mut p) = current_peer {
            match key.to_lowercase().as_str() {
                "publickey" => p.public_key = Some(val.to_string()),
                "presharedkey" => p.preshared_key = Some(val.to_string()),
                "allowedips" => p
                    .allowed_ips
                    .extend(val.split(',').map(|s| s.trim().to_string())),
                "endpoint" => p.endpoint = Some(val.to_string()),
                "persistentkeepalive" => p.persistent_keepalive = Some(val.to_string()),
                _ => {}
            }
        }
    }

    if let Some(p) = current_peer {
        peers.push(p);
    }

    Ok(Config {
        interface: iface,
        peers,
    })
}

fn is_valid_wg_key(key: &str) -> bool {
    let key = key.trim();
    if key.len() != 44 {
        return false;
    }
    B64.decode(key).map(|b| b.len() == 32).unwrap_or(false)
}

fn validate_cidr(cidr: &str) -> bool {
    // Accept 0.0.0.0/0, ::/0, and standard CIDR notation
    let cidr = cidr.trim();
    if let Some(slash) = cidr.find('/') {
        let ip_part = &cidr[..slash];
        let prefix_part = &cidr[slash + 1..];
        let prefix: u32 = match prefix_part.parse() {
            Ok(p) => p,
            Err(_) => return false,
        };
        // IPv4
        if ip_part.contains('.') {
            let octs: Vec<&str> = ip_part.split('.').collect();
            if octs.len() != 4 {
                return false;
            }
            let ok = octs.iter().all(|o| o.parse::<u8>().is_ok());
            return ok && prefix <= 32;
        }
        // IPv6 (basic check)
        return prefix <= 128;
    }
    false
}

fn validate_endpoint(ep: &str) -> bool {
    // host:port or [ipv6]:port
    let ep = ep.trim();
    if let Some(colon) = ep.rfind(':') {
        let port_str = &ep[colon + 1..];
        let port: u16 = match port_str.parse() {
            Ok(p) => p,
            Err(_) => return false,
        };
        port > 0 // port 0 is not valid for WireGuard
    } else {
        false
    }
}

fn load_input(args: &Value) -> Result<String, String> {
    if let Some(t) = args["config"].as_str().or(args["text"].as_str()) {
        return Ok(t.to_string());
    }
    if let Some(path) = args["file"].as_str() {
        return std::fs::read_to_string(path).map_err(|e| format!("cannot read '{path}': {e}"));
    }
    Err("provide 'config', 'text', or 'file'".to_string())
}

fn action_info(cfg: &Config) -> String {
    let mut out = String::from("═══ [Interface] ═══════════════════════════════════\n");

    out.push_str(&format!(
        "PrivateKey:   {}\n",
        if cfg.interface.private_key.is_some() {
            "[set — redacted]"
        } else {
            "[MISSING]"
        }
    ));

    if cfg.interface.address.is_empty() {
        out.push_str("Address:      [not set]\n");
    } else {
        out.push_str(&format!(
            "Address:      {}\n",
            cfg.interface.address.join(", ")
        ));
    }

    if let Some(ref port) = cfg.interface.listen_port {
        out.push_str(&format!("ListenPort:   {port}\n"));
    }

    if !cfg.interface.dns.is_empty() {
        out.push_str(&format!("DNS:          {}\n", cfg.interface.dns.join(", ")));
    }

    if let Some(ref mtu) = cfg.interface.mtu {
        out.push_str(&format!("MTU:          {mtu}\n"));
    }

    if !cfg.interface.pre_up.is_empty()
        || !cfg.interface.post_up.is_empty()
        || !cfg.interface.pre_down.is_empty()
        || !cfg.interface.post_down.is_empty()
    {
        out.push_str("Hooks:        PreUp/PostUp/PreDown/PostDown configured\n");
    }

    out.push_str(&format!("\n{} peer(s)\n", cfg.peers.len()));

    if cfg.peers.is_empty() {
        return out;
    }

    let w_key = 20;
    let w_ep = 30;
    out.push_str(&format!(
        "\n{:<w_key$}  {:<w_ep$}  AllowedIPs\n",
        "PublicKey (prefix)", "Endpoint"
    ));
    out.push_str(&format!("{}\n", "─".repeat(80)));

    for peer in &cfg.peers {
        let pk_prefix = peer
            .public_key
            .as_deref()
            .map(|k| {
                if k.len() >= 8 {
                    format!("{}…", &k[..8])
                } else {
                    k.to_string()
                }
            })
            .unwrap_or_else(|| "[MISSING]".to_string());

        let ep = peer.endpoint.as_deref().unwrap_or("—");

        let allowed = if peer.allowed_ips.is_empty() {
            "[MISSING]".to_string()
        } else {
            peer.allowed_ips.join(", ")
        };

        let name_prefix = peer
            .name
            .as_deref()
            .map(|n| format!("[{n}] "))
            .unwrap_or_default();

        if !name_prefix.is_empty() {
            out.push_str(&format!("{name_prefix}\n"));
        }
        out.push_str(&format!(
            "{:<w_key$}  {:<w_ep$}  {}\n",
            pk_prefix, ep, allowed
        ));
    }

    out
}

fn action_peers(cfg: &Config) -> String {
    if cfg.peers.is_empty() {
        return "No [Peer] sections found.\n".to_string();
    }

    let mut out = String::new();
    for (i, peer) in cfg.peers.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let header = peer
            .name
            .as_deref()
            .map(|n| format!("─── Peer {} — {} ", i + 1, n))
            .unwrap_or_else(|| format!("─── Peer {} ", i + 1));
        out.push_str(&format!(
            "{}{}\n",
            header,
            "─".repeat(50usize.saturating_sub(header.len()))
        ));

        if let Some(ref pk) = peer.public_key {
            let valid = if is_valid_wg_key(pk) {
                "✓"
            } else {
                "✗ INVALID"
            };
            out.push_str(&format!("PublicKey:          {} {valid}\n", pk));
        } else {
            out.push_str("PublicKey:          [MISSING]\n");
        }

        if let Some(ref psk) = peer.preshared_key {
            let valid = if is_valid_wg_key(psk) {
                "✓"
            } else {
                "✗ INVALID"
            };
            out.push_str(&format!("PresharedKey:       [set] {valid}\n"));
        }

        if let Some(ref ep) = peer.endpoint {
            let valid = if validate_endpoint(ep) {
                ""
            } else {
                " ← invalid format"
            };
            out.push_str(&format!("Endpoint:           {ep}{valid}\n"));
        }

        if peer.allowed_ips.is_empty() {
            out.push_str("AllowedIPs:         [MISSING]\n");
        } else {
            for (j, cidr) in peer.allowed_ips.iter().enumerate() {
                let valid = if validate_cidr(cidr) {
                    ""
                } else {
                    " ← invalid CIDR"
                };
                let label = if j == 0 {
                    "AllowedIPs:         "
                } else {
                    "                    "
                };
                out.push_str(&format!("{label}{cidr}{valid}\n"));
            }
        }

        if let Some(ref ka) = peer.persistent_keepalive {
            out.push_str(&format!("PersistentKeepalive: {ka}s\n"));
        }
    }

    out
}

fn action_validate(cfg: &Config) -> String {
    let mut issues: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Interface checks
    if cfg.interface.private_key.is_none() {
        issues.push("[Interface] PrivateKey is missing".to_string());
    } else if let Some(ref pk) = cfg.interface.private_key {
        if !is_valid_wg_key(pk) {
            issues.push(
                "[Interface] PrivateKey is not a valid 32-byte Curve25519 key (44 base64 chars)"
                    .to_string(),
            );
        }
    }

    if cfg.interface.address.is_empty() {
        warnings.push("[Interface] Address is not set — needed for most setups".to_string());
    } else {
        for addr in &cfg.interface.address {
            if !validate_cidr(addr) {
                issues.push(format!("[Interface] Address '{addr}' is not a valid CIDR"));
            }
        }
    }

    if let Some(ref port) = cfg.interface.listen_port {
        match port.parse::<u16>() {
            Ok(0) => issues.push("[Interface] ListenPort 0 is not valid".to_string()),
            Err(_) => issues.push(format!(
                "[Interface] ListenPort '{port}' is not a valid port number"
            )),
            Ok(_) => {}
        }
    }

    if cfg.peers.is_empty() {
        warnings.push("No [Peer] sections — config has no peers".to_string());
    }

    for (i, peer) in cfg.peers.iter().enumerate() {
        let id = peer
            .name
            .as_deref()
            .map(|n| format!("Peer '{n}'"))
            .unwrap_or_else(|| format!("Peer {}", i + 1));

        match &peer.public_key {
            None => issues.push(format!("[{id}] PublicKey is missing")),
            Some(pk) if !is_valid_wg_key(pk) => issues.push(format!(
                "[{id}] PublicKey is not a valid 32-byte key (44 base64 chars)"
            )),
            _ => {}
        }

        if let Some(ref psk) = peer.preshared_key {
            if !is_valid_wg_key(psk) {
                issues.push(format!("[{id}] PresharedKey is not a valid 32-byte key"));
            }
        }

        if peer.allowed_ips.is_empty() {
            issues.push(format!("[{id}] AllowedIPs is missing"));
        } else {
            for cidr in &peer.allowed_ips {
                if !validate_cidr(cidr) {
                    issues.push(format!(
                        "[{id}] AllowedIPs '{cidr}' is not valid CIDR notation"
                    ));
                }
            }
        }

        if let Some(ref ep) = peer.endpoint {
            if !validate_endpoint(ep) {
                issues.push(format!(
                    "[{id}] Endpoint '{ep}' is not valid host:port format"
                ));
            }
        }

        if let Some(ref ka) = peer.persistent_keepalive {
            match ka.parse::<u32>() {
                Err(_) => issues.push(format!(
                    "[{id}] PersistentKeepalive '{ka}' is not a valid number"
                )),
                Ok(0) => warnings.push(format!(
                    "[{id}] PersistentKeepalive 0 disables keepalives (same as not set)"
                )),
                Ok(n) if n > 65535 => {
                    warnings.push(format!("[{id}] PersistentKeepalive {n} is unusually large"))
                }
                _ => {}
            }
        }

        // Warn if no endpoint (ok for server-side peers, but flag it)
        if peer.endpoint.is_none() {
            warnings.push(format!("[{id}] no Endpoint — this peer cannot initiate connections (ok for server configs)"));
        }
    }

    let mut out = String::new();

    if issues.is_empty() && warnings.is_empty() {
        out.push_str("Verdict: VALID\nNo issues found.\n");
        return out;
    }

    if !issues.is_empty() {
        out.push_str(&format!("Errors ({}):\n", issues.len()));
        for iss in &issues {
            out.push_str(&format!("  ✗ {iss}\n"));
        }
    }

    if !warnings.is_empty() {
        out.push_str(&format!("\nWarnings ({}):\n", warnings.len()));
        for w in &warnings {
            out.push_str(&format!("  ⚠ {w}\n"));
        }
    }

    out.push('\n');
    if issues.is_empty() {
        out.push_str("Verdict: VALID (with warnings)\n");
    } else {
        out.push_str("Verdict: INVALID\n");
    }

    out
}

fn action_keys(cfg: &Config) -> String {
    let mut out = String::new();

    out.push_str("─── Interface ──────────────────────────────────────\n");
    match &cfg.interface.private_key {
        None => out.push_str("PrivateKey: [not set]\n"),
        Some(pk) => {
            let status = if is_valid_wg_key(pk) {
                "✓ valid 32-byte key"
            } else {
                "✗ invalid format"
            };
            out.push_str(&format!("PrivateKey: [REDACTED] — {status}\n"));
        }
    }
    out.push('\n');

    if cfg.peers.is_empty() {
        out.push_str("No peers.\n");
        return out;
    }

    out.push_str("─── Peers ──────────────────────────────────────────\n");
    for (i, peer) in cfg.peers.iter().enumerate() {
        let label = peer
            .name
            .as_deref()
            .map(|n| format!("Peer {} ({n})", i + 1))
            .unwrap_or_else(|| format!("Peer {}", i + 1));
        out.push_str(&format!("{label}:\n"));

        match &peer.public_key {
            None => out.push_str("  PublicKey:    [not set]\n"),
            Some(pk) => {
                let status = if is_valid_wg_key(pk) {
                    "✓"
                } else {
                    "✗ invalid"
                };
                out.push_str(&format!("  PublicKey:    {pk} {status}\n"));
            }
        }

        match &peer.preshared_key {
            None => {}
            Some(psk) => {
                let status = if is_valid_wg_key(psk) {
                    "✓ valid"
                } else {
                    "✗ invalid"
                };
                out.push_str(&format!("  PresharedKey: [REDACTED] — {status}\n"));
            }
        }
    }

    out
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args["action"].as_str().unwrap_or("info");
    let text = load_input(args)?;

    if text.trim().is_empty() {
        return Err("empty config".to_string());
    }

    let cfg = parse_config(&text)?;

    if cfg.interface.private_key.is_none() && cfg.peers.is_empty() {
        // Check if this even looks like a WireGuard config
        if !text.contains("[Interface]") && !text.contains("[Peer]") {
            return Err(
                "does not look like a WireGuard config — expected [Interface] or [Peer] sections"
                    .to_string(),
            );
        }
    }

    match action {
        "info" => Ok(action_info(&cfg)),
        "peers" => Ok(action_peers(&cfg)),
        "validate" => Ok(action_validate(&cfg)),
        "keys" => Ok(action_keys(&cfg)),
        _ => Err(format!("unknown action '{action}'")),
    }
}
