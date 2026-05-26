use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    match action {
        "info" => info(args),
        "cidr" => cidr(args),
        "contains" => contains(args),
        "convert" => convert(args),
        "subnet" => subnet(args),
        other => Err(format!(
            "ip_tools: unknown action '{other}'. Valid: info, cidr, contains, convert, subnet"
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_ip(s: &str) -> Result<IpAddr, String> {
    s.trim()
        .parse::<IpAddr>()
        .map_err(|_| format!("ip_tools: cannot parse IP address '{s}'"))
}

fn parse_ipv4(s: &str) -> Result<Ipv4Addr, String> {
    match parse_ip(s)? {
        IpAddr::V4(v4) => Ok(v4),
        IpAddr::V6(_) => Err(format!("ip_tools: '{s}' is IPv6, expected IPv4")),
    }
}

fn ipv4_class(ip: Ipv4Addr) -> &'static str {
    let o = ip.octets()[0];
    match o {
        0..=127 => "A",
        128..=191 => "B",
        192..=223 => "C",
        224..=239 => "D (multicast)",
        240..=255 => "E (reserved)",
    }
}

fn ipv4_type(ip: Ipv4Addr) -> &'static str {
    if ip.is_loopback() {
        "Loopback"
    } else if ip.is_private() {
        "Private (RFC 1918)"
    } else if ip.is_link_local() {
        "Link-local (APIPA)"
    } else if ip.is_broadcast() {
        "Broadcast"
    } else if ip.is_multicast() {
        "Multicast"
    } else if ip.is_unspecified() {
        "Unspecified"
    } else {
        "Public"
    }
}

fn ipv4_to_binary(ip: Ipv4Addr) -> String {
    ip.octets()
        .iter()
        .map(|o| format!("{o:08b}"))
        .collect::<Vec<_>>()
        .join(".")
}

fn prefix_to_mask(prefix: u32) -> Ipv4Addr {
    if prefix == 0 {
        Ipv4Addr::new(0, 0, 0, 0)
    } else {
        let mask = !0u32 << (32 - prefix);
        Ipv4Addr::from(mask)
    }
}

fn mask_to_prefix(mask: Ipv4Addr) -> Result<u32, String> {
    let n = u32::from(mask);
    // A valid mask is all 1s followed by all 0s: count leading ones, verify no holes.
    let ones = n.count_ones();
    let expected = if ones == 0 {
        0u32
    } else {
        !0u32 << (32 - ones)
    };
    if n != expected {
        return Err(format!("ip_tools: '{}' is not a valid subnet mask", mask));
    }
    Ok(ones)
}

fn network_addr(ip: Ipv4Addr, prefix: u32) -> Ipv4Addr {
    let n = u32::from(ip) & u32::from(prefix_to_mask(prefix));
    Ipv4Addr::from(n)
}

fn broadcast_addr(ip: Ipv4Addr, prefix: u32) -> Ipv4Addr {
    let n = u32::from(ip);
    let mask = u32::from(prefix_to_mask(prefix));
    Ipv4Addr::from(n | !mask)
}

fn usable_hosts(prefix: u32) -> u64 {
    if prefix >= 31 {
        return if prefix == 31 { 2 } else { 1 };
    }
    2u64.pow(32 - prefix) - 2
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn info(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("ip_tools info: 'input' is required")?;

    let ip = parse_ip(input)?;
    let mut out = format!("IP INFO\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input   : {input}\n"));

    match ip {
        IpAddr::V4(v4) => {
            out.push_str(&format!("Version : IPv4\n"));
            out.push_str(&format!("Class   : {}\n", ipv4_class(v4)));
            out.push_str(&format!("Type    : {}\n", ipv4_type(v4)));
            out.push_str(&format!("Binary  : {}\n", ipv4_to_binary(v4)));
            let n = u32::from(v4);
            out.push_str(&format!("Decimal : {n}\n"));
            out.push_str(&format!("Hex     : 0x{n:08X}\n"));
            let o = v4.octets();
            out.push_str(&format!(
                "Octets  : {} . {} . {} . {}\n",
                o[0], o[1], o[2], o[3]
            ));
            out.push_str(&format!("Loopback   : {}\n", v4.is_loopback()));
            out.push_str(&format!("Private    : {}\n", v4.is_private()));
            out.push_str(&format!("Multicast  : {}\n", v4.is_multicast()));
            out.push_str(&format!("IPv4-mapped IPv6 : ::ffff:{}\n", input));
        }
        IpAddr::V6(v6) => {
            out.push_str(&format!("Version    : IPv6\n"));
            out.push_str(&format!("Expanded   : {}\n", expand_ipv6(v6)));
            out.push_str(&format!("Compressed : {v6}\n"));
            out.push_str(&format!("Loopback   : {}\n", v6.is_loopback()));
            out.push_str(&format!("Multicast  : {}\n", v6.is_multicast()));
            out.push_str(&format!(
                "IPv4-compat: {}\n",
                v6.to_ipv4()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "n/a".to_string())
            ));
        }
    }
    Ok(out)
}

fn expand_ipv6(v6: Ipv6Addr) -> String {
    v6.segments()
        .iter()
        .map(|s| format!("{s:04x}"))
        .collect::<Vec<_>>()
        .join(":")
}

fn cidr(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("ip_tools cidr: 'input' is required (e.g. '192.168.1.0/24')")?;

    let (ip_part, prefix_str) = input.split_once('/').ok_or_else(|| {
        format!("ip_tools cidr: expected CIDR notation like '192.168.1.0/24', got '{input}'")
    })?;

    let ip = parse_ipv4(ip_part)?;
    let prefix: u32 = prefix_str
        .trim()
        .parse()
        .map_err(|_| format!("ip_tools cidr: invalid prefix length '{prefix_str}'"))?;
    if prefix > 32 {
        return Err(format!(
            "ip_tools cidr: prefix {prefix} out of range [0, 32]"
        ));
    }

    let network = network_addr(ip, prefix);
    let broadcast = broadcast_addr(ip, prefix);
    let mask = prefix_to_mask(prefix);
    let hosts = usable_hosts(prefix);

    let first_host = if prefix < 31 {
        let n = u32::from(network) + 1;
        Ipv4Addr::from(n).to_string()
    } else {
        network.to_string()
    };

    let last_host = if prefix < 31 {
        let n = u32::from(broadcast) - 1;
        Ipv4Addr::from(n).to_string()
    } else {
        broadcast.to_string()
    };

    let mut out = format!("CIDR INFO\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input       : {input}\n"));
    out.push_str(&format!("Network     : {network}/{prefix}\n"));
    out.push_str(&format!("Subnet mask : {mask}\n"));
    out.push_str(&format!("Broadcast   : {broadcast}\n"));
    out.push_str(&format!("First host  : {first_host}\n"));
    out.push_str(&format!("Last host   : {last_host}\n"));
    out.push_str(&format!("Usable hosts: {hosts}\n"));
    out.push_str(&format!(
        "Total IPs   : {}\n",
        if prefix <= 32 {
            2u64.pow(32 - prefix)
        } else {
            1
        }
    ));
    out.push_str(&format!(
        "Wildcard    : {}\n",
        Ipv4Addr::from(!u32::from(mask))
    ));
    out.push_str(&format!("Network binary  : {}\n", ipv4_to_binary(network)));
    out.push_str(&format!("Mask binary     : {}\n", ipv4_to_binary(mask)));
    Ok(out)
}

fn contains(args: &serde_json::Value) -> Result<String, String> {
    let ip_str = args
        .get("ip")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("ip_tools contains: 'ip' is required")?;
    let cidr_str = args
        .get("cidr")
        .or_else(|| args.get("network"))
        .and_then(|v| v.as_str())
        .ok_or("ip_tools contains: 'cidr' is required (e.g. '192.168.1.0/24')")?;

    let ip = parse_ipv4(ip_str)?;
    let (net_ip_str, prefix_str) = cidr_str
        .split_once('/')
        .ok_or_else(|| format!("ip_tools contains: expected CIDR notation, got '{cidr_str}'"))?;
    let net_ip = parse_ipv4(net_ip_str)?;
    let prefix: u32 = prefix_str
        .trim()
        .parse()
        .map_err(|_| format!("ip_tools contains: invalid prefix '{prefix_str}'"))?;

    let network = network_addr(net_ip, prefix);
    let broadcast = broadcast_addr(net_ip, prefix);
    let ip_n = u32::from(ip);
    let in_range = ip_n >= u32::from(network) && ip_n <= u32::from(broadcast);

    let mut out = format!("IP CONTAINS\n{}\n", "─".repeat(50));
    out.push_str(&format!("IP      : {ip_str}\n"));
    out.push_str(&format!("Network : {cidr_str}\n"));
    out.push_str(&format!("Range   : {network} – {broadcast}\n"));
    out.push_str(&format!(
        "Result  : {}\n",
        if in_range {
            "YES — IP is within the network"
        } else {
            "NO — IP is outside the network"
        }
    ));
    Ok(out)
}

fn convert(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("ip_tools convert: 'input' is required")?;

    let trimmed = input.trim();

    // Try to parse as decimal integer → IPv4
    if let Ok(n) = trimmed.parse::<u32>() {
        let ip = Ipv4Addr::from(n);
        let mut out = format!("IP CONVERT\n{}\n", "─".repeat(50));
        out.push_str(&format!("Input   : {input} (decimal integer)\n"));
        out.push_str(&format!("IPv4    : {ip}\n"));
        out.push_str(&format!("Hex     : 0x{n:08X}\n"));
        out.push_str(&format!("Binary  : {}\n", ipv4_to_binary(ip)));
        return Ok(out);
    }

    // Try hex 0xNNNNNNNN
    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        let hex = &trimmed[2..];
        if let Ok(n) = u32::from_str_radix(hex, 16) {
            let ip = Ipv4Addr::from(n);
            let mut out = format!("IP CONVERT\n{}\n", "─".repeat(50));
            out.push_str(&format!("Input   : {input} (hexadecimal)\n"));
            out.push_str(&format!("IPv4    : {ip}\n"));
            out.push_str(&format!("Decimal : {n}\n"));
            out.push_str(&format!("Binary  : {}\n", ipv4_to_binary(ip)));
            return Ok(out);
        }
    }

    // Try dotted-decimal or IPv6
    let ip = parse_ip(trimmed)?;
    let mut out = format!("IP CONVERT\n{}\n", "─".repeat(50));
    match ip {
        IpAddr::V4(v4) => {
            let n = u32::from(v4);
            out.push_str(&format!("Input   : {input}\n"));
            out.push_str(&format!("Decimal : {n}\n"));
            out.push_str(&format!("Hex     : 0x{n:08X}\n"));
            out.push_str(&format!("Binary  : {}\n", ipv4_to_binary(v4)));
            out.push_str(&format!("IPv4-mapped IPv6 : ::ffff:{v4}\n"));
        }
        IpAddr::V6(v6) => {
            out.push_str(&format!("Input      : {input}\n"));
            out.push_str(&format!("Compressed : {v6}\n"));
            out.push_str(&format!("Expanded   : {}\n", expand_ipv6(v6)));
            if let Some(v4) = v6.to_ipv4() {
                out.push_str(&format!("IPv4       : {v4}\n"));
            }
        }
    }
    Ok(out)
}

fn subnet(args: &serde_json::Value) -> Result<String, String> {
    let ip_str = args
        .get("ip")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("ip_tools subnet: 'ip' is required")?;

    let mask_str = args
        .get("mask")
        .and_then(|v| v.as_str())
        .ok_or("ip_tools subnet: 'mask' is required (e.g. '255.255.255.0')")?;

    let ip = parse_ipv4(ip_str)?;
    let mask = parse_ipv4(mask_str)?;
    let prefix = mask_to_prefix(mask)?;

    let network = network_addr(ip, prefix);
    let broadcast = broadcast_addr(ip, prefix);
    let hosts = usable_hosts(prefix);

    let mut out = format!("SUBNET INFO\n{}\n", "─".repeat(50));
    out.push_str(&format!("IP          : {ip_str}\n"));
    out.push_str(&format!("Subnet mask : {mask_str}\n"));
    out.push_str(&format!("Prefix      : /{prefix}\n"));
    out.push_str(&format!("Network     : {network}/{prefix}\n"));
    out.push_str(&format!("Broadcast   : {broadcast}\n"));
    out.push_str(&format!("Usable hosts: {hosts}\n"));
    Ok(out)
}
