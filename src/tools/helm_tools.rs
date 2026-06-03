use serde_json::{json, Value};
use serde_yaml::Value as Yaml;
use std::collections::HashMap;

pub fn make_schema() -> Value {
    json!({
        "name": "helm_tools",
        "description": "Parse and inspect Helm chart files (Chart.yaml, values.yaml, templates) without external utilities.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["chart", "values", "deps", "validate", "templates"],
                    "description": "chart=parse Chart.yaml metadata, values=inspect values.yaml with type summary, deps=list chart dependencies, validate=chart validity checks, templates=list template files"
                },
                "chart_yaml": { "type": "string", "description": "Content of Chart.yaml inline" },
                "values_yaml": { "type": "string", "description": "Content of values.yaml inline" },
                "chart_file": { "type": "string", "description": "Path to Chart.yaml" },
                "values_file": { "type": "string", "description": "Path to values.yaml" },
                "chart_dir": { "type": "string", "description": "Path to the chart directory (auto-locates Chart.yaml and values.yaml inside)" }
            }
        }
    })
}

fn load_chart_yaml(args: &Value) -> Result<Option<Yaml>, String> {
    if let Some(dir) = args.get("chart_dir").and_then(|v| v.as_str()) {
        let path = format!("{}/Chart.yaml", dir.trim_end_matches('/'));
        match std::fs::read_to_string(&path) {
            Ok(s) => {
                return Ok(Some(
                    serde_yaml::from_str(&s)
                        .map_err(|e| format!("Chart.yaml parse error: {}", e))?,
                ))
            }
            Err(_) => {} // try alternate
        }
        let path2 = format!("{}/chart.yaml", dir.trim_end_matches('/'));
        if let Ok(s) = std::fs::read_to_string(&path2) {
            return Ok(Some(
                serde_yaml::from_str(&s).map_err(|e| format!("chart.yaml parse error: {}", e))?,
            ));
        }
        return Ok(None);
    }
    if let Some(f) = args.get("chart_file").and_then(|v| v.as_str()) {
        let s = std::fs::read_to_string(f).map_err(|e| format!("Cannot read {}: {}", f, e))?;
        return Ok(Some(
            serde_yaml::from_str(&s).map_err(|e| format!("Chart.yaml parse error: {}", e))?,
        ));
    }
    if let Some(y) = args.get("chart_yaml").and_then(|v| v.as_str()) {
        return Ok(Some(
            serde_yaml::from_str(y).map_err(|e| format!("Chart.yaml parse error: {}", e))?,
        ));
    }
    Ok(None)
}

fn load_values_yaml(args: &Value) -> Result<Option<Yaml>, String> {
    if let Some(dir) = args.get("chart_dir").and_then(|v| v.as_str()) {
        let path = format!("{}/values.yaml", dir.trim_end_matches('/'));
        if let Ok(s) = std::fs::read_to_string(&path) {
            return Ok(Some(
                serde_yaml::from_str(&s).map_err(|e| format!("values.yaml parse error: {}", e))?,
            ));
        }
        return Ok(None);
    }
    if let Some(f) = args.get("values_file").and_then(|v| v.as_str()) {
        let s = std::fs::read_to_string(f).map_err(|e| format!("Cannot read {}: {}", f, e))?;
        return Ok(Some(
            serde_yaml::from_str(&s).map_err(|e| format!("values.yaml parse error: {}", e))?,
        ));
    }
    if let Some(y) = args.get("values_yaml").and_then(|v| v.as_str()) {
        return Ok(Some(
            serde_yaml::from_str(y).map_err(|e| format!("values.yaml parse error: {}", e))?,
        ));
    }
    Ok(None)
}

fn yaml_str(v: &Yaml) -> String {
    match v {
        Yaml::String(s) => s.clone(),
        Yaml::Bool(b) => b.to_string(),
        Yaml::Number(n) => n.to_string(),
        Yaml::Null => "null".to_string(),
        _ => serde_yaml::to_string(v)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn yaml_type_label(v: &Yaml) -> &'static str {
    match v {
        Yaml::String(_) => "string",
        Yaml::Bool(_) => "bool",
        Yaml::Number(_) => "number",
        Yaml::Null => "null",
        Yaml::Sequence(_) => "list",
        Yaml::Mapping(_) => "map",
        Yaml::Tagged(_) => "tagged",
    }
}

fn count_values_types(v: &Yaml, counts: &mut HashMap<&'static str, usize>, depth: usize) {
    if depth > 5 {
        return;
    }
    match v {
        Yaml::Mapping(m) => {
            for (_, val) in m {
                *counts.entry(yaml_type_label(val)).or_insert(0) += 1;
                if matches!(val, Yaml::Mapping(_) | Yaml::Sequence(_)) {
                    count_values_types(val, counts, depth + 1);
                }
            }
        }
        Yaml::Sequence(seq) => {
            for item in seq {
                *counts.entry(yaml_type_label(item)).or_insert(0) += 1;
                count_values_types(item, counts, depth + 1);
            }
        }
        _ => {}
    }
}

fn list_top_keys(v: &Yaml) -> Vec<(String, &'static str, String)> {
    let mut result = Vec::new();
    if let Yaml::Mapping(m) = v {
        for (k, val) in m {
            let key = yaml_str(k);
            let typ = yaml_type_label(val);
            let preview = match val {
                Yaml::String(s) => {
                    let s: String = s.chars().take(60).collect();
                    s
                }
                Yaml::Bool(b) => b.to_string(),
                Yaml::Number(n) => n.to_string(),
                Yaml::Null => "null".to_string(),
                Yaml::Sequence(seq) => format!("[{} items]", seq.len()),
                Yaml::Mapping(m) => format!("{{{} keys}}", m.len()),
                _ => "…".to_string(),
            };
            result.push((key, typ, preview));
        }
    }
    result
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("chart");

    match action {
        "chart" => {
            let chart = load_chart_yaml(args)?.ok_or_else(|| {
                "No Chart.yaml provided. Use 'chart_yaml', 'chart_file', or 'chart_dir'."
                    .to_string()
            })?;
            action_chart(&chart)
        }
        "values" => {
            let values = load_values_yaml(args)?.ok_or_else(|| {
                "No values.yaml provided. Use 'values_yaml', 'values_file', or 'chart_dir'."
                    .to_string()
            })?;
            action_values(&values)
        }
        "deps" => {
            let chart = load_chart_yaml(args)?.ok_or_else(|| {
                "No Chart.yaml provided. Use 'chart_yaml', 'chart_file', or 'chart_dir'."
                    .to_string()
            })?;
            action_deps(&chart)
        }
        "validate" => {
            let chart = load_chart_yaml(args)?;
            let values = load_values_yaml(args)?;
            action_validate(chart.as_ref(), values.as_ref())
        }
        "templates" => {
            let dir = args
                .get("chart_dir")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Provide 'chart_dir' to list templates.".to_string())?;
            action_templates(dir)
        }
        _ => Err(format!(
            "Unknown action '{}'. Use: chart, values, deps, validate, templates",
            action
        )),
    }
}

fn action_chart(chart: &Yaml) -> Result<String, String> {
    let m = match chart.as_mapping() {
        Some(m) => m,
        None => return Err("Chart.yaml is not a mapping.".to_string()),
    };
    let get = |k: &str| m.get(k).map(yaml_str);

    let mut out = String::new();
    out.push_str("## Helm Chart\n\n");
    out.push_str(&format!(
        "**Name:**        {}\n",
        get("name").unwrap_or_else(|| "—".to_string())
    ));
    out.push_str(&format!(
        "**Version:**     {}\n",
        get("version").unwrap_or_else(|| "—".to_string())
    ));
    out.push_str(&format!(
        "**AppVersion:**  {}\n",
        get("appVersion").unwrap_or_else(|| "—".to_string())
    ));
    out.push_str(&format!(
        "**Type:**        {}\n",
        get("type").unwrap_or_else(|| "application".to_string())
    ));
    out.push_str(&format!(
        "**API Version:** {}\n",
        get("apiVersion").unwrap_or_else(|| "—".to_string())
    ));
    if let Some(desc) = get("description") {
        out.push_str(&format!("**Description:** {}\n", desc));
    }
    if let Some(home) = get("home") {
        out.push_str(&format!("**Home:**        {}\n", home));
    }

    // Keywords
    if let Some(Yaml::Sequence(kws)) = m.get("keywords") {
        let kw_list: Vec<_> = kws.iter().map(yaml_str).collect();
        out.push_str(&format!("**Keywords:**    {}\n", kw_list.join(", ")));
    }

    // Maintainers
    if let Some(Yaml::Sequence(maintainers)) = m.get("maintainers") {
        out.push_str("**Maintainers:**\n");
        for maint in maintainers {
            if let Yaml::Mapping(mm) = maint {
                let name = mm.get("name").map(yaml_str).unwrap_or_default();
                let email = mm
                    .get("email")
                    .map(|e| format!(" <{}>", yaml_str(e)))
                    .unwrap_or_default();
                out.push_str(&format!("  - {}{}\n", name, email));
            }
        }
    }

    // Dependencies summary
    if let Some(Yaml::Sequence(deps)) = m.get("dependencies") {
        out.push_str(&format!("**Dependencies:** {} chart(s)\n", deps.len()));
    }

    // Annotations
    if let Some(Yaml::Mapping(ann)) = m.get("annotations") {
        out.push_str("**Annotations:**\n");
        for (k, v) in ann.iter().take(5) {
            out.push_str(&format!("  {}: {}\n", yaml_str(k), yaml_str(v)));
        }
    }

    Ok(out)
}

fn action_values(values: &Yaml) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("## values.yaml\n\n");

    let top_keys = list_top_keys(values);
    if top_keys.is_empty() {
        return Ok("values.yaml is empty or not a mapping.\n".to_string());
    }

    let mut type_counts: HashMap<&'static str, usize> = HashMap::new();
    count_values_types(values, &mut type_counts, 0);

    // Type summary
    let mut tc_list: Vec<_> = type_counts.into_iter().collect();
    tc_list.sort_by(|a, b| b.1.cmp(&a.1));
    let type_str: Vec<String> = tc_list
        .iter()
        .map(|(t, c)| format!("{} ×{}", t, c))
        .collect();
    out.push_str(&format!(
        "Top-level keys: {}  Types: {}\n\n",
        top_keys.len(),
        type_str.join(", ")
    ));

    // List top-level keys
    out.push_str(&format!("{:<30} {:<8} {}\n", "Key", "Type", "Preview"));
    out.push_str(&format!(
        "{:<30} {:<8} {}\n",
        "─".repeat(29),
        "─".repeat(7),
        "─".repeat(40)
    ));
    for (key, typ, preview) in &top_keys {
        let k: String = key.chars().take(29).collect();
        let p: String = preview.chars().take(60).collect();
        let ellipsis = if preview.len() > 60 { "…" } else { "" };
        out.push_str(&format!("{:<30} {:<8} {}{}\n", k, typ, p, ellipsis));
    }

    // Check for common Helm value patterns
    out.push('\n');
    let keys: Vec<&str> = top_keys.iter().map(|(k, _, _)| k.as_str()).collect();
    let has_image = keys.contains(&"image");
    let has_resources = keys.contains(&"resources");
    let has_ingress = keys.contains(&"ingress");
    let has_service = keys.contains(&"service");
    let has_replicas = keys.contains(&"replicaCount") || keys.contains(&"replicas");
    let has_rbac = keys.contains(&"rbac");
    let has_sa = keys.contains(&"serviceAccount");

    let mut features = Vec::new();
    if has_image {
        features.push("image config");
    }
    if has_resources {
        features.push("resource limits");
    }
    if has_ingress {
        features.push("ingress");
    }
    if has_service {
        features.push("service");
    }
    if has_replicas {
        features.push("replica count");
    }
    if has_rbac {
        features.push("RBAC");
    }
    if has_sa {
        features.push("serviceAccount");
    }
    if !features.is_empty() {
        out.push_str(&format!("Detected: {}\n", features.join(", ")));
    }

    Ok(out)
}

fn action_deps(chart: &Yaml) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("## Chart Dependencies\n\n");

    let m = match chart.as_mapping() {
        Some(m) => m,
        None => return Err("Chart.yaml is not a mapping.".to_string()),
    };

    let deps = match m.get("dependencies") {
        Some(Yaml::Sequence(seq)) => seq,
        _ => return Ok("No dependencies defined in Chart.yaml.\n".to_string()),
    };

    out.push_str(&format!(
        "{:<25} {:<15} {:<30} {}\n",
        "Name", "Version", "Repository", "Condition"
    ));
    out.push_str(&format!(
        "{:<25} {:<15} {:<30} {}\n",
        "─".repeat(24),
        "─".repeat(14),
        "─".repeat(29),
        "─".repeat(20)
    ));

    for dep in deps {
        if let Yaml::Mapping(dm) = dep {
            let get = |k: &str| dm.get(k).map(yaml_str).unwrap_or_else(|| "—".to_string());
            let name = get("name");
            let version = get("version");
            let repo = get("repository");
            let condition = get("condition");
            let n: String = name.chars().take(24).collect();
            let v: String = version.chars().take(14).collect();
            let r: String = repo.chars().take(29).collect();
            let c: String = condition.chars().take(30).collect();
            out.push_str(&format!("{:<25} {:<15} {:<30} {}\n", n, v, r, c));

            // Tags
            if let Some(Yaml::Sequence(tags)) = dm.get("tags") {
                let tag_list: Vec<_> = tags.iter().map(yaml_str).collect();
                out.push_str(&format!("  tags: {}\n", tag_list.join(", ")));
            }
        }
    }

    out.push_str(&format!(
        "\n{} dependency(ies). Run 'helm dependency update' to fetch.\n",
        deps.len()
    ));
    out.push_str("Lock file: Chart.lock — check this into version control.\n");
    Ok(out)
}

fn action_validate(chart: Option<&Yaml>, values: Option<&Yaml>) -> Result<String, String> {
    let mut issues: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    if let Some(chart) = chart {
        if let Some(m) = chart.as_mapping() {
            let get = |k: &str| m.get(k).map(yaml_str);

            // Required fields
            if get("name").is_none() {
                issues.push("Chart.yaml: missing required field 'name'".to_string());
            }
            if get("version").is_none() {
                issues.push("Chart.yaml: missing required field 'version'".to_string());
            }
            if get("apiVersion").is_none() {
                warnings.push(
                    "Chart.yaml: missing 'apiVersion' (should be 'v2' for Helm 3)".to_string(),
                );
            }

            // Version format
            if let Some(ver) = get("version") {
                let parts: Vec<&str> = ver.split('.').collect();
                if parts.len() != 3 || parts.iter().any(|p| p.parse::<u64>().is_err()) {
                    issues.push(format!(
                        "Chart.yaml: version '{}' is not valid SemVer (X.Y.Z required)",
                        ver
                    ));
                }
            }

            // api version check
            if let Some(api) = get("apiVersion") {
                if api == "v1" {
                    warnings.push(
                        "Chart.yaml: apiVersion 'v1' is Helm 2 format — use 'v2' for Helm 3"
                            .to_string(),
                    );
                }
            }

            // Type
            if let Some(t) = get("type") {
                if t != "application" && t != "library" {
                    issues.push(format!(
                        "Chart.yaml: type '{}' is invalid — use 'application' or 'library'",
                        t
                    ));
                }
            }

            // Description recommended
            if get("description").is_none() {
                warnings.push("Chart.yaml: 'description' field is recommended".to_string());
            }
        } else {
            issues.push("Chart.yaml: not a YAML mapping".to_string());
        }
    } else {
        warnings.push("No Chart.yaml provided — skipping chart validation".to_string());
    }

    if let Some(values) = values {
        if values.as_mapping().is_none() {
            issues.push("values.yaml: root must be a YAML mapping".to_string());
        }
    } else {
        warnings.push("No values.yaml provided — skipping values validation".to_string());
    }

    let verdict = if issues.is_empty() {
        "VALID"
    } else {
        "INVALID"
    };
    let mut out = String::new();
    out.push_str(&format!("## Validate — {}\n\n", verdict));
    out.push_str(&format!(
        "Issues: {}  Warnings: {}\n\n",
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
    Ok(out)
}

fn action_templates(dir: &str) -> Result<String, String> {
    let templates_dir = format!("{}/templates", dir.trim_end_matches('/'));
    let entries =
        std::fs::read_dir(&templates_dir).map_err(|e| format!("Cannot read templates/: {}", e))?;

    let mut files: Vec<(String, u64)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        files.push((name, size));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::new();
    out.push_str(&format!("## Templates ({} file(s))\n\n", files.len()));
    if files.is_empty() {
        return Ok(out + "No template files found.\n");
    }

    let mut helpers = 0usize;
    let mut notes = 0usize;
    let mut k8s_types: HashMap<String, usize> = HashMap::new();

    for (name, size) in &files {
        let label = if name.starts_with('_') {
            helpers += 1;
            "[helper]"
        } else if name == "NOTES.txt" {
            notes += 1;
            "[notes]"
        } else {
            ""
        };
        out.push_str(&format!(
            "  {:5} B  {}{}\n",
            size,
            name,
            if !label.is_empty() {
                format!("  {}", label)
            } else {
                String::new()
            }
        ));

        // Guess k8s resource type from filename
        let lower = name.to_lowercase();
        for kind in &[
            "deployment",
            "service",
            "configmap",
            "secret",
            "ingress",
            "serviceaccount",
            "rbac",
            "clusterrole",
            "role",
            "daemonset",
            "statefulset",
            "job",
            "cronjob",
            "hpa",
            "pvc",
            "persistentvolumeclaim",
        ] {
            if lower.contains(kind) {
                *k8s_types.entry(kind.to_string()).or_insert(0) += 1;
            }
        }
    }

    out.push('\n');
    if !k8s_types.is_empty() {
        let mut types_list: Vec<_> = k8s_types.into_iter().collect();
        types_list.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        let type_str: Vec<String> = types_list
            .iter()
            .map(|(t, c)| format!("{} ×{}", t, c))
            .collect();
        out.push_str(&format!(
            "Detected resource types: {}\n",
            type_str.join(", ")
        ));
    }
    if helpers > 0 {
        out.push_str(&format!("Helper files (_*.yaml): {}\n", helpers));
    }
    if notes > 0 {
        out.push_str("NOTES.txt: present (shown after 'helm install')\n");
    }

    Ok(out)
}
