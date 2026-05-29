use serde_json::Value;
use serde_yaml::Value as Yaml;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else {
        "info".to_string()
    };
    match action.as_str() {
        "info" => info_action(args),
        "containers" => containers_action(args),
        "volumes" => volumes_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: info, containers, volumes, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("yaml"))
        .or_else(|| args.get("manifest"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            "Missing 'text' — pass the Kubernetes manifest YAML content as a string".to_string()
        })
}

fn load_manifest(text: &str) -> Result<Yaml, String> {
    serde_yaml::from_str(text).map_err(|e| format!("Failed to parse manifest YAML: {}", e))
}

fn yaml_str(v: &Yaml) -> String {
    match v {
        Yaml::String(s) => s.clone(),
        Yaml::Number(n) => n.to_string(),
        Yaml::Bool(b) => b.to_string(),
        Yaml::Null => "".to_string(),
        _ => serde_yaml::to_string(v)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn kind(doc: &Yaml) -> String {
    doc.get("kind").map(yaml_str).unwrap_or_default()
}

fn api_version(doc: &Yaml) -> String {
    doc.get("apiVersion").map(yaml_str).unwrap_or_default()
}

fn meta_name(doc: &Yaml) -> String {
    doc.get("metadata")
        .and_then(|m| m.get("name"))
        .map(yaml_str)
        .unwrap_or_else(|| "(unnamed)".to_string())
}

fn meta_namespace(doc: &Yaml) -> Option<String> {
    doc.get("metadata")
        .and_then(|m| m.get("namespace"))
        .map(yaml_str)
        .filter(|s| !s.is_empty())
}

fn spec<'a>(doc: &'a Yaml) -> Option<&'a Yaml> {
    doc.get("spec")
}

fn pod_spec<'a>(doc: &'a Yaml) -> Option<&'a Yaml> {
    let k = kind(doc).to_lowercase();
    match k.as_str() {
        "pod" => spec(doc),
        "deployment" | "replicaset" | "statefulset" | "daemonset" | "job" | "cronjob" => {
            let template_path = if k == "cronjob" {
                doc.get("spec")
                    .and_then(|s| s.get("jobTemplate"))
                    .and_then(|j| j.get("spec"))
                    .and_then(|s| s.get("template"))
                    .and_then(|t| t.get("spec"))
            } else {
                doc.get("spec")
                    .and_then(|s| s.get("template"))
                    .and_then(|t| t.get("spec"))
            };
            template_path
        }
        _ => None,
    }
}

fn get_containers<'a>(ps: &'a Yaml) -> Vec<&'a Yaml> {
    ps.get("containers")
        .and_then(|c| c.as_sequence())
        .map(|seq| seq.iter().collect())
        .unwrap_or_default()
}

fn get_init_containers<'a>(ps: &'a Yaml) -> Vec<&'a Yaml> {
    ps.get("initContainers")
        .and_then(|c| c.as_sequence())
        .map(|seq| seq.iter().collect())
        .unwrap_or_default()
}

fn info_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = load_manifest(&text)?;

    let kind_val = kind(&doc);
    let api = api_version(&doc);
    let name = meta_name(&doc);
    let ns = meta_namespace(&doc);

    let mut out = format!("Kubernetes Manifest\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Kind:        {}\n",
        if kind_val.is_empty() {
            "(unknown)"
        } else {
            &kind_val
        }
    );
    out += &format!(
        "apiVersion:  {}\n",
        if api.is_empty() { "(unknown)" } else { &api }
    );
    out += &format!("Name:        {}\n", name);
    if let Some(ref n) = ns {
        out += &format!("Namespace:   {}\n", n);
    }

    // Labels
    if let Some(labels) = doc
        .get("metadata")
        .and_then(|m| m.get("labels"))
        .and_then(|l| l.as_mapping())
    {
        if !labels.is_empty() {
            out += &format!(
                "Labels:      {}\n",
                labels
                    .iter()
                    .map(|(k, v)| format!("{}={}", yaml_str(k), yaml_str(v)))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    // Annotations count
    if let Some(ann) = doc
        .get("metadata")
        .and_then(|m| m.get("annotations"))
        .and_then(|a| a.as_mapping())
    {
        out += &format!("Annotations: {}\n", ann.len());
    }

    out += "\n";

    // Kind-specific summary
    let k = kind_val.to_lowercase();
    match k.as_str() {
        "deployment" | "replicaset" | "statefulset" | "daemonset" => {
            if let Some(s) = spec(&doc) {
                if let Some(replicas) = s.get("replicas") {
                    out += &format!("Replicas:    {}\n", yaml_str(replicas));
                }
                if let Some(selector) = s
                    .get("selector")
                    .and_then(|sel| sel.get("matchLabels"))
                    .and_then(|ml| ml.as_mapping())
                {
                    let sel_str: Vec<String> = selector
                        .iter()
                        .map(|(k, v)| format!("{}={}", yaml_str(k), yaml_str(v)))
                        .collect();
                    out += &format!("Selector:    {}\n", sel_str.join(", "));
                }
                if let Some(strategy) = s.get("strategy").and_then(|st| st.get("type")) {
                    out += &format!("Strategy:    {}\n", yaml_str(strategy));
                }
            }
        }
        "service" => {
            if let Some(s) = spec(&doc) {
                if let Some(stype) = s.get("type") {
                    out += &format!("Type:        {}\n", yaml_str(stype));
                }
                if let Some(cluster_ip) = s.get("clusterIP") {
                    out += &format!("ClusterIP:   {}\n", yaml_str(cluster_ip));
                }
                if let Some(ports) = s.get("ports").and_then(|p| p.as_sequence()) {
                    out += &format!("Ports:       {}\n", ports.len());
                    for port in ports {
                        let p = port.get("port").map(yaml_str).unwrap_or_default();
                        let target = port.get("targetPort").map(yaml_str).unwrap_or_default();
                        let proto = port
                            .get("protocol")
                            .map(yaml_str)
                            .unwrap_or_else(|| "TCP".to_string());
                        let name_p = port.get("name").map(yaml_str).unwrap_or_default();
                        let label = if name_p.is_empty() {
                            String::new()
                        } else {
                            format!(" ({})", name_p)
                        };
                        out += &format!("  {}:{} {}{}\n", p, target, proto, label);
                    }
                }
            }
        }
        "configmap" => {
            if let Some(data) = doc.get("data").and_then(|d| d.as_mapping()) {
                out += &format!("Keys:        {}\n", data.len());
                for (k, _) in data.iter().take(10) {
                    out += &format!("  {}\n", yaml_str(k));
                }
            }
        }
        "ingress" => {
            if let Some(s) = spec(&doc) {
                if let Some(rules) = s.get("rules").and_then(|r| r.as_sequence()) {
                    out += &format!("Rules:       {}\n", rules.len());
                    for rule in rules {
                        if let Some(host) = rule.get("host") {
                            out += &format!("  Host: {}\n", yaml_str(host));
                        }
                    }
                }
                if let Some(tls) = s.get("tls").and_then(|t| t.as_sequence()) {
                    out += &format!("TLS:         {} block(s)\n", tls.len());
                }
            }
        }
        _ => {}
    }

    // Container summary
    if let Some(ps) = pod_spec(&doc) {
        let containers = get_containers(ps);
        let init_containers = get_init_containers(ps);
        if !containers.is_empty() {
            out += &format!("\nContainers:  {}\n", containers.len());
            for c in &containers {
                let cname = c
                    .get("name")
                    .map(yaml_str)
                    .unwrap_or_else(|| "(unnamed)".to_string());
                let image = c
                    .get("image")
                    .map(yaml_str)
                    .unwrap_or_else(|| "(no image)".to_string());
                out += &format!("  {} — {}\n", cname, image);
            }
        }
        if !init_containers.is_empty() {
            out += &format!("InitContainers: {}\n", init_containers.len());
        }
    }

    Ok(out)
}

fn containers_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = load_manifest(&text)?;

    let ps = match pod_spec(&doc) {
        Some(p) => p,
        None => return Ok(format!("Kind '{}' has no pod spec (containers). Try Deployment, Pod, StatefulSet, DaemonSet, Job, or CronJob.\n", kind(&doc))),
    };

    let mut all_containers: Vec<(bool, &Yaml)> = Vec::new();
    for c in get_init_containers(ps) {
        all_containers.push((true, c));
    }
    for c in get_containers(ps) {
        all_containers.push((false, c));
    }

    if all_containers.is_empty() {
        return Ok("No containers found.\n".to_string());
    }

    let mut out = format!(
        "Containers  [{} total]\n{}\n\n",
        all_containers.len(),
        "=".repeat(44)
    );

    for (is_init, c) in &all_containers {
        let cname = c
            .get("name")
            .map(yaml_str)
            .unwrap_or_else(|| "(unnamed)".to_string());
        let image = c
            .get("image")
            .map(yaml_str)
            .unwrap_or_else(|| "(no image)".to_string());
        let init_tag = if *is_init { " [init]" } else { "" };

        out += &format!("Container: {}{}\n", cname, init_tag);
        out += &format!("  Image:   {}\n", image);

        // Ports
        if let Some(ports) = c.get("ports").and_then(|p| p.as_sequence()) {
            let port_strs: Vec<String> = ports
                .iter()
                .map(|p| {
                    let cp = p.get("containerPort").map(yaml_str).unwrap_or_default();
                    let proto = p
                        .get("protocol")
                        .map(yaml_str)
                        .unwrap_or_else(|| "TCP".to_string());
                    let name_p = p.get("name").map(yaml_str).unwrap_or_default();
                    if name_p.is_empty() {
                        format!("{}/{}", cp, proto)
                    } else {
                        format!("{}/{} ({})", cp, proto, name_p)
                    }
                })
                .collect();
            out += &format!("  Ports:   {}\n", port_strs.join(", "));
        }

        // Resources
        if let Some(res) = c.get("resources") {
            let req_cpu = res.get("requests").and_then(|r| r.get("cpu")).map(yaml_str);
            let req_mem = res
                .get("requests")
                .and_then(|r| r.get("memory"))
                .map(yaml_str);
            let lim_cpu = res.get("limits").and_then(|l| l.get("cpu")).map(yaml_str);
            let lim_mem = res
                .get("limits")
                .and_then(|l| l.get("memory"))
                .map(yaml_str);
            if req_cpu.is_some() || req_mem.is_some() {
                out += &format!(
                    "  Requests: cpu={} mem={}\n",
                    req_cpu.as_deref().unwrap_or("-"),
                    req_mem.as_deref().unwrap_or("-")
                );
            }
            if lim_cpu.is_some() || lim_mem.is_some() {
                out += &format!(
                    "  Limits:   cpu={} mem={}\n",
                    lim_cpu.as_deref().unwrap_or("-"),
                    lim_mem.as_deref().unwrap_or("-")
                );
            }
        }

        // Env vars
        if let Some(env) = c.get("env").and_then(|e| e.as_sequence()) {
            out += &format!("  Env:     {} var(s)\n", env.len());
        }
        if let Some(env_from) = c.get("envFrom").and_then(|e| e.as_sequence()) {
            out += &format!("  EnvFrom: {} source(s)\n", env_from.len());
        }

        // Volume mounts
        if let Some(mounts) = c.get("volumeMounts").and_then(|v| v.as_sequence()) {
            out += &format!("  Mounts:  {}\n", mounts.len());
            for m in mounts.iter().take(5) {
                let mname = m.get("name").map(yaml_str).unwrap_or_default();
                let mp = m.get("mountPath").map(yaml_str).unwrap_or_default();
                let ro = m.get("readOnly").and_then(|v| v.as_bool()).unwrap_or(false);
                let ro_tag = if ro { " [ro]" } else { "" };
                out += &format!("    {} → {}{}\n", mname, mp, ro_tag);
            }
        }

        // Probes
        let has_liveness = c.get("livenessProbe").is_some();
        let has_readiness = c.get("readinessProbe").is_some();
        let has_startup = c.get("startupProbe").is_some();
        let probes: Vec<&str> = [
            if has_liveness { Some("liveness") } else { None },
            if has_readiness {
                Some("readiness")
            } else {
                None
            },
            if has_startup { Some("startup") } else { None },
        ]
        .iter()
        .filter_map(|x| *x)
        .collect();
        if !probes.is_empty() {
            out += &format!("  Probes:  {}\n", probes.join(", "));
        }

        // Security context
        if let Some(sc) = c.get("securityContext") {
            let run_as_root = sc.get("runAsNonRoot").and_then(|v| v.as_bool());
            let privileged = sc
                .get("privileged")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let read_only_fs = sc.get("readOnlyRootFilesystem").and_then(|v| v.as_bool());
            let mut sc_parts = Vec::new();
            if privileged {
                sc_parts.push("PRIVILEGED");
            }
            if let Some(false) = run_as_root {
                sc_parts.push("runAsNonRoot=false");
            }
            if let Some(true) = read_only_fs {
                sc_parts.push("readOnlyRootFilesystem");
            }
            if !sc_parts.is_empty() {
                out += &format!("  Security: {}\n", sc_parts.join(", "));
            }
        }

        out += "\n";
    }
    Ok(out)
}

fn volumes_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = load_manifest(&text)?;

    let ps = match pod_spec(&doc) {
        Some(p) => p,
        None => return Ok(format!("Kind '{}' has no pod spec.\n", kind(&doc))),
    };

    let volumes = match ps.get("volumes").and_then(|v| v.as_sequence()) {
        Some(v) => v,
        None => return Ok("No volumes defined in pod spec.\n".to_string()),
    };

    let mut out = format!("Volumes  [{} total]\n{}\n\n", volumes.len(), "=".repeat(44));

    for vol in volumes {
        let vname = vol
            .get("name")
            .map(yaml_str)
            .unwrap_or_else(|| "(unnamed)".to_string());
        let vol_type = detect_volume_type(vol);
        let detail = volume_detail(vol);
        out += &format!("{}\n  Type: {}\n", vname, vol_type);
        if !detail.is_empty() {
            out += &format!("  {}\n", detail);
        }
        out += "\n";
    }
    Ok(out)
}

fn detect_volume_type(vol: &Yaml) -> &'static str {
    if vol.get("configMap").is_some() {
        return "ConfigMap";
    }
    if vol.get("secret").is_some() {
        return "Secret";
    }
    if vol.get("emptyDir").is_some() {
        return "EmptyDir";
    }
    if vol.get("hostPath").is_some() {
        return "HostPath";
    }
    if vol.get("persistentVolumeClaim").is_some() {
        return "PVC";
    }
    if vol.get("nfs").is_some() {
        return "NFS";
    }
    if vol.get("projected").is_some() {
        return "Projected";
    }
    if vol.get("downwardAPI").is_some() {
        return "DownwardAPI";
    }
    if vol.get("gitRepo").is_some() {
        return "GitRepo (deprecated)";
    }
    "unknown"
}

fn volume_detail(vol: &Yaml) -> String {
    if let Some(cm) = vol.get("configMap") {
        let n = cm.get("name").map(yaml_str).unwrap_or_default();
        return format!("ConfigMap: {}", n);
    }
    if let Some(sec) = vol.get("secret") {
        let n = sec.get("secretName").map(yaml_str).unwrap_or_default();
        return format!("Secret: {}", n);
    }
    if let Some(hp) = vol.get("hostPath") {
        let p = hp.get("path").map(yaml_str).unwrap_or_default();
        return format!("HostPath: {}", p);
    }
    if let Some(pvc) = vol.get("persistentVolumeClaim") {
        let claim = pvc.get("claimName").map(yaml_str).unwrap_or_default();
        let ro = pvc
            .get("readOnly")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        return format!("Claim: {}{}", claim, if ro { " [readOnly]" } else { "" });
    }
    if let Some(nfs) = vol.get("nfs") {
        let server = nfs.get("server").map(yaml_str).unwrap_or_default();
        let path = nfs.get("path").map(yaml_str).unwrap_or_default();
        return format!("Server: {}, Path: {}", server, path);
    }
    String::new()
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = load_manifest(&text)?;
    let mut warnings: Vec<String> = Vec::new();

    let k = kind(&doc).to_lowercase();

    if kind(&doc).is_empty() {
        warnings.push("Missing 'kind' field".to_string());
    }
    if api_version(&doc).is_empty() {
        warnings.push("Missing 'apiVersion' field".to_string());
    }
    if doc.get("metadata").and_then(|m| m.get("name")).is_none() {
        warnings.push("Missing metadata.name".to_string());
    }

    // Pod-level checks
    if let Some(ps) = pod_spec(&doc) {
        let containers = get_containers(ps);
        let init_containers = get_init_containers(ps);

        let all_c: Vec<&Yaml> = init_containers
            .iter()
            .chain(containers.iter())
            .copied()
            .collect();

        if containers.is_empty() {
            warnings.push("No containers defined in pod spec".to_string());
        }

        // Host-level security
        if ps
            .get("hostNetwork")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            warnings.push(
                "hostNetwork: true — container shares the host's network namespace".to_string(),
            );
        }
        if ps.get("hostPID").and_then(|v| v.as_bool()).unwrap_or(false) {
            warnings.push("hostPID: true — container can see host processes".to_string());
        }

        for c in &all_c {
            let cname = c
                .get("name")
                .map(yaml_str)
                .unwrap_or_else(|| "(unnamed)".to_string());

            // Image latest tag
            if let Some(image) = c.get("image").map(yaml_str) {
                let has_tag =
                    image.contains('@') || (image.contains(':') && !image.ends_with(":latest"));
                if !has_tag {
                    warnings.push(format!(
                        "Container '{}' uses image '{}' without a pinned tag — use a specific version or digest",
                        cname, image
                    ));
                }
            }

            // Resource limits
            let has_limits = c.get("resources").and_then(|r| r.get("limits")).is_some();
            if !has_limits {
                warnings.push(format!(
                    "Container '{}': no resource limits defined — may consume unbounded CPU/memory",
                    cname
                ));
            }

            // Privileged
            let privileged = c
                .get("securityContext")
                .and_then(|sc| sc.get("privileged"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if privileged {
                warnings.push(format!(
                    "Container '{}': privileged: true — has full access to host kernel",
                    cname
                ));
            }

            // runAsNonRoot
            let run_as_non_root = c
                .get("securityContext")
                .and_then(|sc| sc.get("runAsNonRoot"))
                .and_then(|v| v.as_bool());
            let run_as_user = c
                .get("securityContext")
                .and_then(|sc| sc.get("runAsUser"))
                .and_then(|v| v.as_u64());
            if run_as_non_root != Some(true) && run_as_user.is_none() {
                warnings.push(format!(
                    "Container '{}': no runAsNonRoot or runAsUser set — may run as root",
                    cname
                ));
            }

            // Liveness/readiness probes (only on main containers, not init)
            let is_init = init_containers.iter().any(|ic| {
                ic.get("name").map(yaml_str).as_deref() == c.get("name").map(yaml_str).as_deref()
            });
            if !is_init {
                if c.get("livenessProbe").is_none() {
                    warnings.push(format!(
                        "Container '{}': no livenessProbe — Kubernetes can't detect and restart stuck containers",
                        cname
                    ));
                }
                if c.get("readinessProbe").is_none() {
                    warnings.push(format!(
                        "Container '{}': no readinessProbe — traffic may route to unready containers",
                        cname
                    ));
                }
            }
        }

        // HostPath volumes
        if let Some(vols) = ps.get("volumes").and_then(|v| v.as_sequence()) {
            for vol in vols {
                if vol.get("hostPath").is_some() {
                    let vname = vol.get("name").map(yaml_str).unwrap_or_default();
                    let path = vol
                        .get("hostPath")
                        .and_then(|hp| hp.get("path"))
                        .map(yaml_str)
                        .unwrap_or_default();
                    warnings.push(format!(
                        "Volume '{}': hostPath '{}' — mounts host filesystem; security risk",
                        vname, path
                    ));
                }
            }
        }
    }

    // Deployment-specific
    if k == "deployment" {
        if let Some(s) = spec(&doc) {
            let replicas = s.get("replicas").and_then(|r| r.as_u64()).unwrap_or(1);
            if replicas == 1 {
                warnings.push(
                    "replicas: 1 — single replica has no high-availability; consider replicas >= 2"
                        .to_string(),
                );
            }
        }
    }

    let mut out = format!("Kubernetes Manifest Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!("Kind: {}  Name: {}\n", kind(&doc), meta_name(&doc));
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
