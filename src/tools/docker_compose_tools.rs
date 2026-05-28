use serde_json::Value;
use serde_yaml::Value as Yaml;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("service").is_some() {
        "inspect".to_string()
    } else {
        "services".to_string()
    };
    match action.as_str() {
        "services" => services_action(args),
        "inspect" => inspect_action(args),
        "ports" => ports_action(args),
        "volumes" => volumes_action(args),
        "networks" => networks_action(args),
        "env" => env_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: services, inspect, ports, volumes, networks, env, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("yaml"))
        .or_else(|| args.get("compose"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            "Missing 'text' — pass the docker-compose.yml content as a string".to_string()
        })
}

fn parse_doc(text: &str) -> Result<Yaml, String> {
    serde_yaml::from_str(text).map_err(|e| format!("Failed to parse YAML: {}", e))
}

fn yaml_str(v: &Yaml) -> String {
    match v {
        Yaml::String(s) => s.clone(),
        Yaml::Number(n) => n.to_string(),
        Yaml::Bool(b) => b.to_string(),
        Yaml::Null => "~".to_string(),
        _ => serde_yaml::to_string(v)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn service_names(doc: &Yaml) -> Vec<String> {
    doc.get("services")
        .and_then(|s| s.as_mapping())
        .map(|m| {
            m.keys()
                .filter_map(|k| k.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn get_service<'a>(doc: &'a Yaml, name: &str) -> Option<&'a Yaml> {
    doc.get("services")?.get(name)
}

fn service_image(svc: &Yaml) -> Option<String> {
    svc.get("image").map(yaml_str)
}

fn service_build(svc: &Yaml) -> Option<String> {
    let b = svc.get("build")?;
    match b {
        Yaml::String(s) => Some(s.clone()),
        Yaml::Mapping(_) => {
            let ctx = b.get("context").map(yaml_str).unwrap_or_default();
            let dockerfile = b.get("dockerfile").map(yaml_str);
            if let Some(df) = dockerfile {
                Some(format!("{} ({})", ctx, df))
            } else {
                Some(ctx)
            }
        }
        _ => None,
    }
}

fn service_ports(svc: &Yaml) -> Vec<String> {
    svc.get("ports")
        .and_then(|p| p.as_sequence())
        .map(|seq| seq.iter().map(yaml_str).collect())
        .unwrap_or_default()
}

fn service_volumes(svc: &Yaml) -> Vec<String> {
    svc.get("volumes")
        .and_then(|v| v.as_sequence())
        .map(|seq| seq.iter().map(yaml_str).collect())
        .unwrap_or_default()
}

fn service_networks(svc: &Yaml) -> Vec<String> {
    match svc.get("networks") {
        Some(Yaml::Sequence(seq)) => seq.iter().map(yaml_str).collect(),
        Some(Yaml::Mapping(m)) => m
            .keys()
            .filter_map(|k| k.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

fn service_env(svc: &Yaml) -> Vec<String> {
    match svc.get("environment") {
        Some(Yaml::Sequence(seq)) => seq.iter().map(yaml_str).collect(),
        Some(Yaml::Mapping(m)) => m
            .iter()
            .map(|(k, v)| format!("{}={}", yaml_str(k), yaml_str(v)))
            .collect(),
        _ => Vec::new(),
    }
}

fn service_depends(svc: &Yaml) -> Vec<String> {
    match svc.get("depends_on") {
        Some(Yaml::Sequence(seq)) => seq.iter().map(yaml_str).collect(),
        Some(Yaml::Mapping(m)) => m
            .keys()
            .filter_map(|k| k.as_str().map(|s| s.to_string()))
            .collect(),
        Some(Yaml::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn services_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = parse_doc(&text)?;
    let names = service_names(&doc);

    let version = doc.get("version").map(yaml_str);
    let mut out = format!(
        "Docker Compose  [version: {}  services: {}]\n{}\n\n",
        version.as_deref().unwrap_or("unspecified"),
        names.len(),
        "=".repeat(52)
    );

    for name in &names {
        if let Some(svc) = get_service(&doc, name) {
            let image = service_image(svc)
                .or_else(|| service_build(svc).map(|b| format!("(build: {})", b)))
                .unwrap_or_else(|| "(no image/build)".to_string());
            let ports = service_ports(svc);
            let restart = svc.get("restart").map(yaml_str);
            let depends = service_depends(svc);

            out += &format!("Service: {}\n", name);
            out += &format!("  Image:    {}\n", image);
            if !ports.is_empty() {
                out += &format!("  Ports:    {}\n", ports.join(", "));
            }
            if let Some(r) = restart {
                out += &format!("  Restart:  {}\n", r);
            }
            if !depends.is_empty() {
                out += &format!("  Depends:  {}\n", depends.join(", "));
            }
            let vol_count = service_volumes(svc).len();
            let env_count = service_env(svc).len();
            out += &format!("  ({} volume(s), {} env var(s))\n\n", vol_count, env_count);
        }
    }

    // Top-level networks and volumes
    if let Some(nets) = doc.get("networks").and_then(|n| n.as_mapping()) {
        let net_names: Vec<_> = nets.keys().filter_map(|k| k.as_str()).collect();
        if !net_names.is_empty() {
            out += &format!("Top-level Networks: {}\n", net_names.join(", "));
        }
    }
    if let Some(vols) = doc.get("volumes").and_then(|v| v.as_mapping()) {
        let vol_names: Vec<_> = vols.keys().filter_map(|k| k.as_str()).collect();
        if !vol_names.is_empty() {
            out += &format!("Top-level Volumes:  {}\n", vol_names.join(", "));
        }
    }

    Ok(out)
}

fn inspect_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let query = args
        .get("service")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'service' — the service name to inspect")?;
    let doc = parse_doc(&text)?;
    let names = service_names(&doc);

    let name = names
        .iter()
        .find(|n| n.to_lowercase().contains(&query.to_lowercase()))
        .ok_or_else(|| {
            format!(
                "Service '{}' not found. Known services: {}",
                query,
                names.join(", ")
            )
        })?
        .clone();

    let svc = get_service(&doc, &name).unwrap();
    let mut out = format!("Service: {}\n{}\n\n", name, "=".repeat(44));

    if let Some(img) = service_image(svc) {
        out += &format!("Image:       {}\n", img);
    }
    if let Some(bld) = service_build(svc) {
        out += &format!("Build:       {}\n", bld);
    }
    if let Some(cmd) = svc.get("command").map(yaml_str) {
        out += &format!("Command:     {}\n", cmd);
    }
    if let Some(ep) = svc.get("entrypoint").map(yaml_str) {
        out += &format!("Entrypoint:  {}\n", ep);
    }
    if let Some(r) = svc.get("restart").map(yaml_str) {
        out += &format!("Restart:     {}\n", r);
    }

    let ports = service_ports(svc);
    if !ports.is_empty() {
        out += &format!("\nPorts ({}):\n", ports.len());
        for p in &ports {
            out += &format!("  {}\n", p);
        }
    }

    let vols = service_volumes(svc);
    if !vols.is_empty() {
        out += &format!("\nVolumes ({}):\n", vols.len());
        for v in &vols {
            out += &format!("  {}\n", v);
        }
    }

    let nets = service_networks(svc);
    if !nets.is_empty() {
        out += &format!("\nNetworks ({}):\n", nets.len());
        for n in &nets {
            out += &format!("  {}\n", n);
        }
    }

    let env = service_env(svc);
    if !env.is_empty() {
        out += &format!("\nEnvironment ({}):\n", env.len());
        for e in &env {
            out += &format!("  {}\n", e);
        }
    }

    let deps = service_depends(svc);
    if !deps.is_empty() {
        out += &format!("\nDepends On: {}\n", deps.join(", "));
    }

    if let Some(hc) = svc.get("healthcheck") {
        if let Some(test) = hc.get("test").map(yaml_str) {
            out += &format!("\nHealthcheck: {}\n", test);
        }
    }

    Ok(out)
}

fn ports_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = parse_doc(&text)?;
    let names = service_names(&doc);

    let mut out = format!("Port Mappings\n{}\n\n", "=".repeat(44));
    let mut found_any = false;

    for name in &names {
        if let Some(svc) = get_service(&doc, name) {
            let ports = service_ports(svc);
            if !ports.is_empty() {
                found_any = true;
                out += &format!("{}:\n", name);
                for p in &ports {
                    // Annotate host:container pattern
                    let annotation = annotate_port(p);
                    out += &format!("  {}  {}\n", p, annotation);
                }
                out += "\n";
            }
        }
    }

    if !found_any {
        out += "No port mappings defined.\n";
    }
    Ok(out)
}

fn annotate_port(p: &str) -> String {
    // Handles "8080:80", "127.0.0.1:8080:80", "8080", "80/udp"
    let stripped = p.trim_start_matches('"').trim_end_matches('"');
    let parts: Vec<&str> = stripped.split(':').collect();
    let container_part = parts.last().unwrap_or(&stripped);
    let port_num: u16 = container_part
        .split('/')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    match port_num {
        80 => "(HTTP)",
        443 => "(HTTPS)",
        3000 => "(Node/React dev)",
        3306 => "(MySQL)",
        5432 => "(PostgreSQL)",
        5672 => "(RabbitMQ AMQP)",
        6379 => "(Redis)",
        8080 => "(HTTP alt)",
        8443 => "(HTTPS alt)",
        9200 => "(Elasticsearch)",
        27017 => "(MongoDB)",
        _ => "",
    }
    .to_string()
}

fn volumes_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = parse_doc(&text)?;
    let names = service_names(&doc);

    let mut out = format!("Volume Mounts\n{}\n\n", "=".repeat(44));

    // Named top-level volumes
    if let Some(vols) = doc.get("volumes").and_then(|v| v.as_mapping()) {
        out += "Named Volumes (top-level):\n";
        for (k, _) in vols {
            out += &format!("  {}\n", yaml_str(k));
        }
        out += "\n";
    }

    // Per-service mounts
    for name in &names {
        if let Some(svc) = get_service(&doc, name) {
            let vols = service_volumes(svc);
            if !vols.is_empty() {
                out += &format!("{}:\n", name);
                for v in &vols {
                    let kind = if v.starts_with('/') || v.starts_with('.') || v.starts_with('~') {
                        "(bind mount)"
                    } else if v.contains(':') {
                        let left = v.split(':').next().unwrap_or("");
                        if left.starts_with('/') || left.starts_with('.') {
                            "(bind mount)"
                        } else {
                            "(named volume)"
                        }
                    } else {
                        "(named volume)"
                    };
                    out += &format!("  {}  {}\n", v, kind);
                }
                out += "\n";
            }
        }
    }
    Ok(out)
}

fn networks_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = parse_doc(&text)?;
    let names = service_names(&doc);

    let mut out = format!("Networks\n{}\n\n", "=".repeat(44));

    // Top-level network definitions
    if let Some(nets) = doc.get("networks").and_then(|n| n.as_mapping()) {
        out += "Defined Networks:\n";
        for (k, v) in nets {
            let net_name = yaml_str(k);
            let driver = v
                .get("driver")
                .map(yaml_str)
                .unwrap_or_else(|| "bridge".to_string());
            out += &format!("  {} (driver: {})\n", net_name, driver);
        }
        out += "\n";
    }

    // Which services use which networks
    out += "Service → Network Membership:\n";
    for name in &names {
        if let Some(svc) = get_service(&doc, name) {
            let nets = service_networks(svc);
            if !nets.is_empty() {
                out += &format!("  {} → {}\n", name, nets.join(", "));
            } else {
                out += &format!("  {} → (default)\n", name);
            }
        }
    }
    Ok(out)
}

fn env_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let filter = args.get("service").and_then(|v| v.as_str());
    let doc = parse_doc(&text)?;
    let names = service_names(&doc);

    let mut out = format!("Environment Variables\n{}\n\n", "=".repeat(44));

    for name in &names {
        if let Some(f) = filter {
            if !name.to_lowercase().contains(&f.to_lowercase()) {
                continue;
            }
        }
        if let Some(svc) = get_service(&doc, name) {
            let env = service_env(svc);
            if env.is_empty() {
                if filter.is_some() {
                    out += &format!("{}: (no environment variables)\n", name);
                }
                continue;
            }
            out += &format!("{}:\n", name);
            for e in &env {
                // Redact if value looks like a secret
                let display = if e.contains('=') {
                    let mut parts = e.splitn(2, '=');
                    let key = parts.next().unwrap_or("");
                    let val = parts.next().unwrap_or("");
                    let key_upper = key.to_uppercase();
                    if key_upper.contains("PASSWORD")
                        || key_upper.contains("SECRET")
                        || key_upper.contains("TOKEN")
                        || key_upper.contains("KEY")
                        || key_upper.contains("CREDENTIAL")
                    {
                        format!("{}=[REDACTED]", key)
                    } else {
                        format!("{}={}", key, val)
                    }
                } else {
                    e.clone()
                };
                out += &format!("  {}\n", display);
            }
            if let Some(ef) = svc.get("env_file") {
                let files: Vec<String> = match ef {
                    Yaml::String(s) => vec![s.clone()],
                    Yaml::Sequence(seq) => seq.iter().map(yaml_str).collect(),
                    _ => vec![],
                };
                for f in &files {
                    out += &format!("  [env_file: {}]\n", f);
                }
            }
            out += "\n";
        }
    }
    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = parse_doc(&text)?;
    let names = service_names(&doc);
    let mut warnings: Vec<String> = Vec::new();

    if names.is_empty() {
        warnings.push("No services defined under 'services:' key".to_string());
    }

    let mut seen = std::collections::HashSet::new();
    for name in &names {
        if !seen.insert(name) {
            warnings.push(format!("Duplicate service name: '{}'", name));
        }
        if let Some(svc) = get_service(&doc, name) {
            let has_image = service_image(svc).is_some();
            let has_build = svc.get("build").is_some();
            if !has_image && !has_build {
                warnings.push(format!(
                    "Service '{}': no 'image' or 'build' — cannot start without one",
                    name
                ));
            }

            // Warn on missing restart policy
            if svc.get("restart").is_none() {
                warnings.push(format!(
                    "Service '{}': no 'restart' policy — consider 'unless-stopped' or 'always'",
                    name
                ));
            }

            // Check depends_on references valid services
            for dep in service_depends(svc) {
                if !names.contains(&dep) {
                    warnings.push(format!(
                        "Service '{}': depends_on '{}' which is not defined",
                        name, dep
                    ));
                }
            }

            // Warn on privileged mode
            if svc
                .get("privileged")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                warnings.push(format!(
                    "Service '{}': running in privileged mode — grants full host access",
                    name
                ));
            }

            // Warn on host network mode
            if svc
                .get("network_mode")
                .and_then(|v| v.as_str())
                .map(|s| s == "host")
                .unwrap_or(false)
            {
                warnings.push(format!(
                    "Service '{}': network_mode=host bypasses container network isolation",
                    name
                ));
            }
        }
    }

    let mut out = format!("Docker Compose Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!("{} service(s) parsed.\n", names.len());
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
