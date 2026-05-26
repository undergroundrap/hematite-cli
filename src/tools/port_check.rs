use serde_json::Value;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

const DEFAULT_TIMEOUT_MS: u64 = 3000;

// Well-known port annotations
const WELL_KNOWN: &[(u16, &str)] = &[
    (21, "FTP"),
    (22, "SSH"),
    (23, "Telnet"),
    (25, "SMTP"),
    (53, "DNS"),
    (80, "HTTP"),
    (110, "POP3"),
    (143, "IMAP"),
    (443, "HTTPS"),
    (465, "SMTPS"),
    (587, "SMTP submission"),
    (993, "IMAPS"),
    (995, "POP3S"),
    (1234, "LM Studio"),
    (1433, "SQL Server"),
    (1521, "Oracle DB"),
    (3000, "Dev server / Grafana"),
    (3306, "MySQL"),
    (3389, "RDP"),
    (5432, "PostgreSQL"),
    (5672, "RabbitMQ AMQP"),
    (6379, "Redis"),
    (6443, "Kubernetes API"),
    (7474, "Neo4j"),
    (8000, "Dev server"),
    (8080, "HTTP alt / Proxy"),
    (8443, "HTTPS alt"),
    (8888, "Jupyter"),
    (9000, "SonarQube / PHP-FPM"),
    (9200, "Elasticsearch"),
    (9300, "Elasticsearch cluster"),
    (11434, "Ollama"),
    (15672, "RabbitMQ management"),
    (27017, "MongoDB"),
    (27018, "MongoDB shard"),
    (50000, "Jenkins"),
    (51820, "WireGuard"),
];

pub async fn execute(args: &Value) -> Result<String, String> {
    let host = args
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("localhost");

    let port = args
        .get("port")
        .and_then(|v| v.as_u64())
        .ok_or("port_check: 'port' is required (integer)")? as u16;

    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT_MS);

    let timeout = Duration::from_millis(timeout_ms);

    // Resolve hostname
    let addr_str = format!("{host}:{port}");
    let addrs: Vec<SocketAddr> = addr_str
        .to_socket_addrs()
        .map_err(|e| format!("port_check: DNS resolution failed for '{host}': {e}"))?
        .collect();

    if addrs.is_empty() {
        return Err(format!(
            "port_check: could not resolve '{host}' — check hostname or DNS"
        ));
    }

    let resolved_ip = addrs[0].ip();

    // Try each resolved address
    let mut last_error = String::new();
    let open = addrs
        .iter()
        .any(|addr| match TcpStream::connect_timeout(addr, timeout) {
            Ok(_) => true,
            Err(e) => {
                last_error = e.to_string();
                false
            }
        });

    let service = WELL_KNOWN
        .iter()
        .find(|(p, _)| *p == port)
        .map(|(_, name)| format!(" ({name})"))
        .unwrap_or_default();

    let status = if open { "OPEN" } else { "CLOSED / FILTERED" };

    let mut out = format!(
        "port_check: {host}:{port}{service} — {status}\n\
         Resolved   : {resolved_ip}\n\
         Timeout    : {timeout_ms}ms\n"
    );

    if open {
        out.push_str(
            "Result     : Connection succeeded — port is reachable and accepting connections.\n",
        );
    } else {
        out.push_str(&format!(
            "Result     : Connection refused or timed out — port is not reachable.\n\
             Reason     : {last_error}\n"
        ));
        // Helpful hints per port
        if let Some(hint) = hint_for_port(port, host) {
            out.push_str(&format!("Hint       : {hint}\n"));
        }
    }

    Ok(out)
}

fn hint_for_port(port: u16, host: &str) -> Option<&'static str> {
    let is_local = host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host.starts_with("192.168.")
        || host.starts_with("10.");

    match port {
        22 => Some("SSH: check that sshd is running (`sc query sshd` on Windows, `systemctl status ssh` on Linux)"),
        80 | 443 | 8080 | 8443 => Some("HTTP/HTTPS: check that your web server or dev server is running"),
        3306 => Some("MySQL: check that mysqld is running and listening on 0.0.0.0, not 127.0.0.1 only"),
        5432 => Some("PostgreSQL: check pg_hba.conf and postgresql.conf listen_addresses"),
        6379 => Some("Redis: check that Redis is running (`redis-cli ping`) and not bound to 127.0.0.1 only"),
        27017 => Some("MongoDB: check that mongod is running and bindIp includes the target address"),
        1234 => Some("LM Studio: open LM Studio, load a model, and enable the local server"),
        11434 => Some("Ollama: run `ollama serve` or check that the Ollama service is running"),
        3389 => Some("RDP: check that Remote Desktop is enabled in System Properties > Remote"),
        9200 | 9300 => Some("Elasticsearch: check that the Elasticsearch service is running"),
        8888 => Some("Jupyter: run `jupyter notebook` or `jupyter lab` to start the server"),
        _ if is_local => Some("Check that the service is running on localhost and not blocked by the firewall"),
        _ => Some("Check the remote firewall rules and that the service is running on the target host"),
    }
}
