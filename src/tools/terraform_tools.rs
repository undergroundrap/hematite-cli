use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => info_action(args),
        "resources" => resources_action(args),
        "variables" => variables_action(args),
        "outputs" => outputs_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: info, resources, variables, outputs, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("hcl"))
        .or_else(|| args.get("tf"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the Terraform HCL content as a string".to_string())
}

#[derive(Debug, Clone)]
struct HclBlock {
    block_type: String,
    labels: Vec<String>,
    body: String,
}

fn parse_hcl(text: &str) -> Vec<HclBlock> {
    let mut blocks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        // Skip whitespace
        while i < n && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= n {
            break;
        }

        // Skip line comments
        if chars[i] == '#' || (i + 1 < n && chars[i] == '/' && chars[i + 1] == '/') {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Skip block comments
        if i + 1 < n && chars[i] == '/' && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        // Must start with an identifier character
        if !chars[i].is_alphabetic() && chars[i] != '_' {
            i += 1;
            continue;
        }

        // Read block type identifier
        let id_start = i;
        while i < n && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '-') {
            i += 1;
        }
        let block_type: String = chars[id_start..i].iter().collect();

        // Skip inline whitespace (not newlines)
        while i < n && (chars[i] == ' ' || chars[i] == '\t') {
            i += 1;
        }

        // If next non-space is '=' (and not '=='), it's an attribute — skip value
        if i < n && chars[i] == '=' && (i + 1 >= n || chars[i + 1] != '=') {
            i += 1;
            skip_hcl_value(&chars, &mut i);
            continue;
        }

        // Read quoted string labels
        let mut labels = Vec::new();
        while i < n && chars[i] != '{' && chars[i] != '\n' {
            if chars[i] == '"' {
                i += 1;
                let mut s = String::new();
                while i < n && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < n {
                        i += 1;
                        s.push(chars[i]);
                    } else {
                        s.push(chars[i]);
                    }
                    i += 1;
                }
                if i < n {
                    i += 1;
                }
                labels.push(s);
            } else {
                i += 1;
            }
        }

        if i < n && chars[i] == '{' {
            i += 1;
            let body = collect_hcl_body(&chars, &mut i);
            if i < n && chars[i] == '}' {
                i += 1;
            }
            blocks.push(HclBlock {
                block_type,
                labels,
                body,
            });
        } else {
            // Not a block; skip line
            while i < n && chars[i] != '\n' {
                i += 1;
            }
        }
    }

    blocks
}

fn skip_hcl_value(chars: &[char], i: &mut usize) {
    let n = chars.len();
    let mut depth = 0i32;
    while *i < n {
        let c = chars[*i];
        if c == '\n' && depth == 0 {
            break;
        }
        if c == '"' {
            *i += 1;
            while *i < n && chars[*i] != '"' {
                if chars[*i] == '\\' && *i + 1 < n {
                    *i += 1;
                }
                *i += 1;
            }
            if *i < n {
                *i += 1;
            }
        } else if c == '{' || c == '[' {
            depth += 1;
            *i += 1;
        } else if c == '}' || c == ']' {
            depth -= 1;
            if depth < 0 {
                break;
            }
            *i += 1;
        } else {
            *i += 1;
        }
    }
}

fn collect_hcl_body(chars: &[char], i: &mut usize) -> String {
    let n = chars.len();
    let mut depth = 1i32;
    let mut body = String::new();

    while *i < n && depth > 0 {
        let c = chars[*i];
        if c == '"' {
            body.push(c);
            *i += 1;
            while *i < n && chars[*i] != '"' {
                if chars[*i] == '\\' && *i + 1 < n {
                    body.push(chars[*i]);
                    *i += 1;
                }
                body.push(chars[*i]);
                *i += 1;
            }
            if *i < n {
                body.push(chars[*i]);
                *i += 1;
            }
        } else if c == '#' || (c == '/' && *i + 1 < n && chars[*i + 1] == '/') {
            while *i < n && chars[*i] != '\n' {
                *i += 1;
            }
        } else if c == '{' {
            depth += 1;
            body.push(c);
            *i += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                break;
            }
            body.push(c);
            *i += 1;
        } else {
            body.push(c);
            *i += 1;
        }
    }

    body
}

fn get_attr<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        // Match `key =` or `key=`
        let rest = if let Some(r) = line.strip_prefix(key) {
            r
        } else {
            continue;
        };
        let rest = rest.trim_start();
        let rest = if let Some(r) = rest.strip_prefix('=') {
            // guard against ==
            if r.starts_with('=') {
                continue;
            }
            r
        } else {
            continue;
        };
        let val = rest.trim();
        // Strip outer quotes
        if val.starts_with('"') && val.len() >= 2 {
            let inner = &val[1..];
            if let Some(end) = inner.find('"') {
                return Some(&inner[..end]);
            }
        }
        return Some(val);
    }
    None
}

struct TfSummary {
    required_version: Option<String>,
    providers: Vec<(String, String, String)>, // (name, source, version)
    resource_count: usize,
    data_count: usize,
    module_count: usize,
    variable_count: usize,
    output_count: usize,
    local_count: usize,
}

fn summarize(blocks: &[HclBlock]) -> TfSummary {
    let mut s = TfSummary {
        required_version: None,
        providers: Vec::new(),
        resource_count: 0,
        data_count: 0,
        module_count: 0,
        variable_count: 0,
        output_count: 0,
        local_count: 0,
    };

    for b in blocks {
        match b.block_type.as_str() {
            "terraform" => {
                s.required_version = get_attr(&b.body, "required_version").map(|v| v.to_string());
                // Parse nested required_providers block
                let nested = parse_hcl(&b.body);
                for nb in &nested {
                    if nb.block_type == "required_providers" {
                        // Each provider is a block or attribute inside required_providers body
                        // HCL format: `name = { source = "x" version = "y" }`
                        // Our parser sees these as assignments not blocks; use body parsing
                        let inner = parse_hcl(&nb.body);
                        for pb in &inner {
                            let src = get_attr(&pb.body, "source").unwrap_or("").to_string();
                            let ver = get_attr(&pb.body, "version").unwrap_or("").to_string();
                            s.providers.push((pb.block_type.clone(), src, ver));
                        }
                        // Also handle flat `name = { source = "x" version = "y" }` style
                        // by scanning nb.body directly for name = { ... } patterns
                        if inner.is_empty() {
                            // Fallback: scan body lines for `name = {` pattern
                            for prov_block in parse_providers_from_body(&nb.body) {
                                s.providers.push(prov_block);
                            }
                        }
                    }
                }
            }
            "provider" => {
                let name = b.labels.first().cloned().unwrap_or_default();
                let ver = get_attr(&b.body, "version").unwrap_or("").to_string();
                s.providers.push((name, String::new(), ver));
            }
            "resource" => s.resource_count += 1,
            "data" => s.data_count += 1,
            "module" => s.module_count += 1,
            "variable" => s.variable_count += 1,
            "output" => s.output_count += 1,
            "locals" => s.local_count += 1,
            _ => {}
        }
    }

    s
}

fn parse_providers_from_body(body: &str) -> Vec<(String, String, String)> {
    let chars: Vec<char> = body.chars().collect();
    let n = chars.len();
    let mut result = Vec::new();
    let mut i = 0;

    while i < n {
        while i < n && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= n {
            break;
        }

        // Read identifier
        if !chars[i].is_alphabetic() && chars[i] != '_' {
            i += 1;
            continue;
        }
        let id_start = i;
        while i < n && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '-') {
            i += 1;
        }
        let name: String = chars[id_start..i].iter().collect();

        while i < n && (chars[i] == ' ' || chars[i] == '\t') {
            i += 1;
        }

        if i < n && chars[i] == '=' {
            i += 1;
            while i < n && (chars[i] == ' ' || chars[i] == '\t') {
                i += 1;
            }
            if i < n && chars[i] == '{' {
                i += 1;
                let pb = collect_hcl_body(&chars, &mut i);
                if i < n && chars[i] == '}' {
                    i += 1;
                }
                let src = get_attr(&pb, "source").unwrap_or("").to_string();
                let ver = get_attr(&pb, "version").unwrap_or("").to_string();
                result.push((name, src, ver));
            } else {
                skip_hcl_value(&chars, &mut i);
            }
        } else {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
        }
    }

    result
}

fn info_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let blocks = parse_hcl(&text);
    let s = summarize(&blocks);

    let mut out = format!("Terraform Configuration\n{}\n\n", "=".repeat(44));

    if let Some(ref rv) = s.required_version {
        out += &format!("required_version: {}\n", rv);
    } else {
        out += "required_version: (not set)\n";
    }

    out += &format!(
        "\nResources:  {}\nData:       {}\nModules:    {}\nVariables:  {}\nOutputs:    {}\nLocals:     {}\n",
        s.resource_count, s.data_count, s.module_count,
        s.variable_count, s.output_count, s.local_count
    );

    if !s.providers.is_empty() {
        out += &format!("\nProviders ({}):\n", s.providers.len());
        for (name, src, ver) in &s.providers {
            let src_part = if src.is_empty() {
                String::new()
            } else {
                format!(" ({})", src)
            };
            let ver_part = if ver.is_empty() {
                String::new()
            } else {
                format!(" @ {}", ver)
            };
            out += &format!("  {}{}{}\n", name, src_part, ver_part);
        }
    }

    // List modules with source
    let modules: Vec<_> = blocks.iter().filter(|b| b.block_type == "module").collect();
    if !modules.is_empty() {
        out += &format!("\nModules ({}):\n", modules.len());
        for m in &modules {
            let name = m.labels.first().cloned().unwrap_or_default();
            let src = get_attr(&m.body, "source").unwrap_or("(no source)");
            let ver = get_attr(&m.body, "version");
            let ver_str = ver.map(|v| format!(" @ {}", v)).unwrap_or_default();
            out += &format!("  {} — {}{}\n", name, src, ver_str);
        }
    }

    Ok(out)
}

fn resources_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let filter = args.get("filter").and_then(|v| v.as_str());
    let blocks = parse_hcl(&text);

    let resources: Vec<_> = blocks
        .iter()
        .filter(|b| b.block_type == "resource" || b.block_type == "data")
        .filter(|b| {
            if let Some(f) = filter {
                let label = b.labels.join(" ").to_lowercase();
                label.contains(&f.to_lowercase())
            } else {
                true
            }
        })
        .collect();

    if resources.is_empty() {
        return Ok("No resource or data blocks found.\n".to_string());
    }

    let mut out = format!(
        "Resources / Data  [{} total]\n{}\n\n",
        resources.len(),
        "=".repeat(44)
    );

    for b in &resources {
        let rtype = b
            .labels
            .first()
            .cloned()
            .unwrap_or_else(|| "(unknown)".to_string());
        let rname = b
            .labels
            .get(1)
            .cloned()
            .unwrap_or_else(|| "(unnamed)".to_string());
        let kind_tag = if b.block_type == "data" { "data." } else { "" };
        out += &format!("{}{}.{}\n", kind_tag, rtype, rname);

        // Show notable attributes
        let notable = [
            "ami",
            "instance_type",
            "name",
            "location",
            "region",
            "source",
            "bucket",
            "image",
            "machine_type",
            "size",
        ];
        for key in &notable {
            if let Some(v) = get_attr(&b.body, key) {
                out += &format!("  {}: {}\n", key, v);
            }
        }
        // Count tags
        let tag_lines = b
            .body
            .lines()
            .filter(|l| l.trim_start().starts_with("tags"))
            .count();
        if tag_lines > 0 {
            out += "  tags: defined\n";
        }
        out += "\n";
    }

    Ok(out)
}

fn variables_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let blocks = parse_hcl(&text);

    let vars: Vec<_> = blocks
        .iter()
        .filter(|b| b.block_type == "variable")
        .collect();

    if vars.is_empty() {
        return Ok("No variable blocks found.\n".to_string());
    }

    let mut out = format!("Variables  [{} total]\n{}\n\n", vars.len(), "=".repeat(44));

    for v in &vars {
        let name = v.labels.first().cloned().unwrap_or_default();
        let vtype = get_attr(&v.body, "type").unwrap_or("any");
        let default = get_attr(&v.body, "default");
        let description = get_attr(&v.body, "description");
        let sensitive = get_attr(&v.body, "sensitive")
            .map(|s| s == "true")
            .unwrap_or(false);

        out += &format!("var.{}\n", name);
        out += &format!("  type:     {}\n", vtype);
        if let Some(d) = description {
            let snippet: String = d.chars().take(60).collect();
            out += &format!("  desc:     {}\n", snippet);
        }
        if let Some(d) = default {
            out += &format!("  default:  {}\n", d);
        } else {
            out += "  default:  (required)\n";
        }
        if sensitive {
            out += "  [SENSITIVE]\n";
        }
        out += "\n";
    }

    Ok(out)
}

fn outputs_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let blocks = parse_hcl(&text);

    let outputs: Vec<_> = blocks.iter().filter(|b| b.block_type == "output").collect();

    if outputs.is_empty() {
        return Ok("No output blocks found.\n".to_string());
    }

    let mut out = format!("Outputs  [{} total]\n{}\n\n", outputs.len(), "=".repeat(44));

    for o in &outputs {
        let name = o.labels.first().cloned().unwrap_or_default();
        let value = get_attr(&o.body, "value").unwrap_or("(complex expression)");
        let description = get_attr(&o.body, "description");
        let sensitive = get_attr(&o.body, "sensitive")
            .map(|s| s == "true")
            .unwrap_or(false);
        let sensitive_tag = if sensitive { " [SENSITIVE]" } else { "" };

        out += &format!("output.{}{}\n", name, sensitive_tag);
        if let Some(d) = description {
            out += &format!("  desc:  {}\n", d);
        }
        let snippet: String = value.chars().take(80).collect();
        let ellipsis = if value.len() > 80 { "…" } else { "" };
        out += &format!("  value: {}{}\n\n", snippet, ellipsis);
    }

    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let blocks = parse_hcl(&text);
    let s = summarize(&blocks);
    let mut warnings: Vec<String> = Vec::new();

    // Missing required_version
    if s.required_version.is_none() {
        warnings.push(
            "No required_version in terraform{} block — pin the Terraform version for reproducible plans".to_string(),
        );
    }

    // Provider version constraints
    for (name, _src, ver) in &s.providers {
        if ver.is_empty() {
            warnings.push(format!(
                "Provider '{}': no version constraint — pin provider versions to avoid unexpected upgrades",
                name
            ));
        } else if ver == ">= 0" || ver == "*" {
            warnings.push(format!(
                "Provider '{}': version '{}' is too permissive — use ~> for minor-version locking",
                name, ver
            ));
        }
    }

    // Empty resource/variable counts
    if blocks.is_empty() {
        warnings.push(
            "No HCL blocks found — check that valid Terraform configuration was passed".to_string(),
        );
    }

    // Check for hardcoded credentials in resource bodies
    let secret_patterns = [
        "access_key",
        "secret_key",
        "password",
        "private_key",
        "token",
        "api_key",
    ];
    for b in &blocks {
        for line in b.body.lines() {
            let line_lower = line.trim().to_lowercase();
            if line_lower.starts_with('#') || line_lower.starts_with("//") {
                continue;
            }
            for pat in &secret_patterns {
                if line_lower.contains(pat) {
                    // Check if the value looks like a hardcoded literal (not a var/data reference)
                    if let Some(eq_pos) = line.find('=') {
                        let val = line[eq_pos + 1..].trim();
                        if val.starts_with('"')
                            && !val.contains("var.")
                            && !val.contains("data.")
                            && !val.contains("${")
                        {
                            let label = b.labels.join(".");
                            warnings.push(format!(
                                "Possible hardcoded credential: {}.{} contains literal value for '{}' — use var.* or a secrets manager",
                                b.block_type, label, pat
                            ));
                            break;
                        }
                    }
                }
            }
        }
    }

    // Outputs with sensitive-looking names but no sensitive = true
    for b in blocks.iter().filter(|b| b.block_type == "output") {
        let name = b.labels.first().cloned().unwrap_or_default().to_lowercase();
        let is_sensitive = get_attr(&b.body, "sensitive")
            .map(|s| s == "true")
            .unwrap_or(false);
        if !is_sensitive {
            let sensitive_names = [
                "password",
                "secret",
                "key",
                "token",
                "credential",
                "private",
            ];
            if sensitive_names.iter().any(|p| name.contains(p)) {
                warnings.push(format!(
                    "Output '{}' looks sensitive but sensitive = true is not set — Terraform will show its value in state and plan output",
                    name
                ));
            }
        }
    }

    // Variables with sensitive-looking names but no sensitive = true
    for b in blocks.iter().filter(|b| b.block_type == "variable") {
        let name = b.labels.first().cloned().unwrap_or_default().to_lowercase();
        let is_sensitive = get_attr(&b.body, "sensitive")
            .map(|s| s == "true")
            .unwrap_or(false);
        if !is_sensitive {
            let sensitive_names = [
                "password",
                "secret",
                "api_key",
                "token",
                "private_key",
                "credential",
            ];
            if sensitive_names.iter().any(|p| name.contains(p)) {
                warnings.push(format!(
                    "Variable '{}' looks sensitive but sensitive = true is not set — value may appear in plan output",
                    name
                ));
            }
        }
    }

    let mut out = format!("Terraform Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!(
        "{} resource(s), {} variable(s), {} output(s) checked.\n",
        s.resource_count, s.variable_count, s.output_count
    );
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
