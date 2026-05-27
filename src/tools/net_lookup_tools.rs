use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("port");
    match action {
        "port" => port_action(args),
        "service" => service_action(args),
        "search" => search_action(args),
        "protocol" => protocol_action(args),
        other => Err(format!(
            "net_lookup_tools: unknown action '{other}'. Valid: port, service, search, protocol"
        )),
    }
}

/// (port_number, protocol, service_name, description)
static PORTS: &[(u16, &str, &str, &str)] = &[
    (7, "tcp", "echo", "Echo Protocol"),
    (9, "tcp", "discard", "Discard Protocol"),
    (13, "tcp", "daytime", "Daytime Protocol"),
    (20, "tcp", "ftp-data", "FTP Data Transfer"),
    (21, "tcp", "ftp", "FTP Control"),
    (22, "tcp", "ssh", "Secure Shell"),
    (23, "tcp", "telnet", "Telnet"),
    (25, "tcp", "smtp", "Simple Mail Transfer Protocol"),
    (37, "tcp", "time", "Time Protocol"),
    (43, "tcp", "whois", "WHOIS"),
    (53, "tcp", "dns", "Domain Name System"),
    (53, "udp", "dns", "Domain Name System"),
    (
        67,
        "udp",
        "dhcp",
        "Dynamic Host Configuration Protocol (server)",
    ),
    (
        68,
        "udp",
        "dhcp",
        "Dynamic Host Configuration Protocol (client)",
    ),
    (69, "udp", "tftp", "Trivial File Transfer Protocol"),
    (70, "tcp", "gopher", "Gopher"),
    (79, "tcp", "finger", "Finger Protocol"),
    (80, "tcp", "http", "Hypertext Transfer Protocol"),
    (88, "tcp", "kerberos", "Kerberos Authentication"),
    (109, "tcp", "pop2", "Post Office Protocol v2"),
    (110, "tcp", "pop3", "Post Office Protocol v3"),
    (111, "tcp", "rpc", "Remote Procedure Call / portmap"),
    (119, "tcp", "nntp", "Network News Transfer Protocol"),
    (123, "udp", "ntp", "Network Time Protocol"),
    (135, "tcp", "msrpc", "Microsoft RPC / DCE"),
    (137, "udp", "netbios-ns", "NetBIOS Name Service"),
    (138, "udp", "netbios-dgm", "NetBIOS Datagram Service"),
    (139, "tcp", "netbios-ssn", "NetBIOS Session Service"),
    (143, "tcp", "imap", "Internet Message Access Protocol"),
    (161, "udp", "snmp", "Simple Network Management Protocol"),
    (162, "udp", "snmptrap", "SNMP Trap"),
    (179, "tcp", "bgp", "Border Gateway Protocol"),
    (194, "tcp", "irc", "Internet Relay Chat"),
    (389, "tcp", "ldap", "Lightweight Directory Access Protocol"),
    (443, "tcp", "https", "HTTP Secure (TLS)"),
    (445, "tcp", "smb", "Server Message Block / Microsoft-DS"),
    (465, "tcp", "smtps", "SMTP over TLS (Submission)"),
    (500, "udp", "isakmp", "Internet Security Association / IKE"),
    (514, "udp", "syslog", "Syslog"),
    (515, "tcp", "lpd", "Line Printer Daemon"),
    (520, "udp", "rip", "Routing Information Protocol"),
    (546, "udp", "dhcpv6", "DHCPv6 Client"),
    (547, "udp", "dhcpv6", "DHCPv6 Server"),
    (
        587,
        "tcp",
        "submission",
        "Email Message Submission (SMTP+STARTTLS)",
    ),
    (631, "tcp", "ipp", "Internet Printing Protocol"),
    (636, "tcp", "ldaps", "LDAP over TLS"),
    (993, "tcp", "imaps", "IMAP over TLS"),
    (995, "tcp", "pop3s", "POP3 over TLS"),
    (1080, "tcp", "socks", "SOCKS Proxy"),
    (1194, "udp", "openvpn", "OpenVPN"),
    (1234, "tcp", "lmstudio", "LM Studio API (local AI)"),
    (1433, "tcp", "mssql", "Microsoft SQL Server"),
    (1521, "tcp", "oracle", "Oracle Database"),
    (1723, "tcp", "pptp", "Point-to-Point Tunneling Protocol"),
    (1883, "tcp", "mqtt", "MQTT Message Broker"),
    (2049, "tcp", "nfs", "Network File System"),
    (2181, "tcp", "zookeeper", "Apache ZooKeeper"),
    (2375, "tcp", "docker", "Docker API (unencrypted)"),
    (2376, "tcp", "docker-tls", "Docker API (TLS)"),
    (3000, "tcp", "dev-http", "Common development HTTP server"),
    (3306, "tcp", "mysql", "MySQL / MariaDB"),
    (3389, "tcp", "rdp", "Remote Desktop Protocol"),
    (4200, "tcp", "angular", "Angular CLI dev server"),
    (4443, "tcp", "https-alt", "HTTPS alternate port"),
    (5000, "tcp", "dev-http", "Flask / common development server"),
    (5173, "tcp", "vite", "Vite dev server"),
    (5432, "tcp", "postgresql", "PostgreSQL"),
    (5671, "tcp", "amqps", "AMQP over TLS (RabbitMQ)"),
    (
        5672,
        "tcp",
        "amqp",
        "Advanced Message Queuing Protocol (RabbitMQ)",
    ),
    (5900, "tcp", "vnc", "Virtual Network Computing"),
    (6379, "tcp", "redis", "Redis"),
    (6443, "tcp", "kubernetes-api", "Kubernetes API server"),
    (7474, "tcp", "neo4j", "Neo4j Graph Database"),
    (8000, "tcp", "dev-http", "Common development HTTP server"),
    (8080, "tcp", "http-alt", "HTTP alternate / proxy"),
    (8081, "tcp", "http-alt", "HTTP alternate"),
    (8443, "tcp", "https-alt", "HTTPS alternate"),
    (8888, "tcp", "jupyter", "Jupyter Notebook"),
    (9000, "tcp", "sonarqube", "SonarQube / PHP-FPM"),
    (9001, "tcp", "supervisord", "Supervisor / Elasticsearch"),
    (9090, "tcp", "prometheus", "Prometheus metrics server"),
    (9092, "tcp", "kafka", "Apache Kafka"),
    (9200, "tcp", "elasticsearch", "Elasticsearch REST API"),
    (
        9300,
        "tcp",
        "elasticsearch",
        "Elasticsearch cluster transport",
    ),
    (11211, "tcp", "memcached", "Memcached"),
    (11434, "tcp", "ollama", "Ollama API (local AI)"),
    (15672, "tcp", "rabbitmq-mgmt", "RabbitMQ Management UI"),
    (16686, "tcp", "jaeger", "Jaeger Tracing UI"),
    (19999, "tcp", "netdata", "Netdata monitoring"),
    (24224, "tcp", "fluentd", "Fluentd / Fluent Bit"),
    (27017, "tcp", "mongodb", "MongoDB"),
    (
        28017,
        "tcp",
        "mongodb-http",
        "MongoDB HTTP interface (deprecated)",
    ),
    (50070, "tcp", "hadoop", "Hadoop NameNode WebUI"),
    (51820, "udp", "wireguard", "WireGuard VPN"),
];

/// (proto_number, name, description)
static PROTOCOLS: &[(u8, &str, &str)] = &[
    (0, "hopopt", "IPv6 Hop-by-Hop Option"),
    (1, "icmp", "Internet Control Message Protocol"),
    (2, "igmp", "Internet Group Management Protocol"),
    (4, "ipip", "IP in IP encapsulation"),
    (6, "tcp", "Transmission Control Protocol"),
    (8, "egp", "Exterior Gateway Protocol"),
    (9, "igp", "Interior Gateway Protocol"),
    (17, "udp", "User Datagram Protocol"),
    (27, "rdp", "Reliable Data Protocol"),
    (33, "dccp", "Datagram Congestion Control Protocol"),
    (41, "ipv6", "IPv6 encapsulation"),
    (43, "ipv6-route", "IPv6 Routing Header"),
    (44, "ipv6-frag", "IPv6 Fragment Header"),
    (47, "gre", "Generic Routing Encapsulation"),
    (50, "esp", "Encapsulating Security Payload (IPsec)"),
    (51, "ah", "Authentication Header (IPsec)"),
    (58, "icmpv6", "ICMP for IPv6"),
    (88, "eigrp", "Enhanced Interior Gateway Routing Protocol"),
    (89, "ospf", "Open Shortest Path First"),
    (94, "ipip", "IP-in-IP"),
    (97, "etherip", "Ethernet-within-IP Encapsulation"),
    (103, "pim", "Protocol Independent Multicast"),
    (112, "vrrp", "Virtual Router Redundancy Protocol"),
    (115, "l2tp", "Layer 2 Tunneling Protocol"),
    (124, "isis", "IS-IS over IPv4"),
    (132, "sctp", "Stream Control Transmission Protocol"),
    (133, "fc", "Fibre Channel"),
    (136, "udplite", "UDP-Lite"),
    (137, "mpls-in-ip", "MPLS in IP"),
];

fn port_action(args: &Value) -> Result<String, String> {
    let port_val = args
        .get("port")
        .or_else(|| args.get("number"))
        .ok_or("net_lookup_tools port: 'port' number required")?;

    let port = port_val
        .as_u64()
        .or_else(|| port_val.as_str().and_then(|s| s.parse().ok()))
        .ok_or("net_lookup_tools port: 'port' must be a number")? as u16;

    let proto_filter = args
        .get("protocol")
        .or_else(|| args.get("proto"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let matches: Vec<_> = PORTS
        .iter()
        .filter(|(p, proto, _, _)| *p == port && proto_filter.as_ref().map_or(true, |f| proto == f))
        .collect();

    if matches.is_empty() {
        return Ok(format!(
            "Port {port}/{proto} — not found in well-known port table\n",
            proto = proto_filter.as_deref().unwrap_or("any")
        ));
    }

    let mut out = format!("Port {port}\n\n");
    for (_, proto, svc, desc) in &matches {
        out.push_str(&format!(
            "  {proto:>5}/{proto:<3}  {svc:<20}  {desc}\n",
            proto = proto.to_uppercase()
        ));
    }
    Ok(out)
}

fn service_action(args: &Value) -> Result<String, String> {
    let name = args
        .get("name")
        .or_else(|| args.get("service"))
        .and_then(|v| v.as_str())
        .ok_or("net_lookup_tools service: 'name' required")?
        .to_lowercase();

    let matches: Vec<_> = PORTS
        .iter()
        .filter(|(_, _, svc, _)| svc.to_lowercase() == name || svc.to_lowercase().contains(&name))
        .collect();

    if matches.is_empty() {
        return Ok(format!(
            "Service '{name}' — not found in well-known port table\n"
        ));
    }

    let mut out = format!("Service: {name}\n\n");
    for (port, proto, svc, desc) in &matches {
        out.push_str(&format!(
            "  {:>5}/{:<3}  {svc:<20}  {desc}\n",
            port,
            proto.to_uppercase()
        ));
    }
    Ok(out)
}

fn search_action(args: &Value) -> Result<String, String> {
    let query = args
        .get("query")
        .or_else(|| args.get("q"))
        .and_then(|v| v.as_str())
        .ok_or("net_lookup_tools search: 'query' or 'q' required")?
        .to_lowercase();

    let matches: Vec<_> = PORTS
        .iter()
        .filter(|(_, _, svc, desc)| {
            svc.to_lowercase().contains(&query) || desc.to_lowercase().contains(&query)
        })
        .collect();

    if matches.is_empty() {
        return Ok(format!("No services matching '{query}'\n"));
    }

    let mut out = format!("{} result(s) for '{query}':\n\n", matches.len());
    for (port, proto, svc, desc) in &matches {
        out.push_str(&format!(
            "  {:>5}/{:<3}  {svc:<20}  {desc}\n",
            port,
            proto.to_uppercase()
        ));
    }
    Ok(out)
}

fn protocol_action(args: &Value) -> Result<String, String> {
    // If number given, look up the protocol
    if let Some(num_val) = args.get("number").or_else(|| args.get("num")) {
        let num = num_val
            .as_u64()
            .or_else(|| num_val.as_str().and_then(|s| s.parse().ok()))
            .ok_or("net_lookup_tools protocol: 'number' must be a u8")? as u8;
        return match PROTOCOLS.iter().find(|(n, _, _)| *n == num) {
            Some((n, name, desc)) => Ok(format!("Protocol {n}: {name}\n  {desc}\n")),
            None => Ok(format!("Protocol {num}: not found in IANA table\n")),
        };
    }

    // If name given, reverse look up
    if let Some(name) = args
        .get("name")
        .or_else(|| args.get("proto"))
        .and_then(|v| v.as_str())
    {
        let lower = name.to_lowercase();
        let found: Vec<_> = PROTOCOLS
            .iter()
            .filter(|(_, n, d)| {
                n.to_lowercase().contains(&lower) || d.to_lowercase().contains(&lower)
            })
            .collect();
        if found.is_empty() {
            return Ok(format!("Protocol '{name}': not found\n"));
        }
        let mut out = format!("Protocol '{name}' matches:\n\n");
        for (n, name, desc) in &found {
            out.push_str(&format!("  {:>3}  {name:<12}  {desc}\n", n));
        }
        return Ok(out);
    }

    // Default: list all
    let mut out = format!("IP Protocol Numbers ({} entries)\n\n", PROTOCOLS.len());
    for (n, name, desc) in PROTOCOLS {
        out.push_str(&format!("  {:>3}  {name:<12}  {desc}\n", n));
    }
    Ok(out)
}
