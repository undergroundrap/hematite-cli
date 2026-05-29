use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else {
        "info".to_string()
    };
    match action.as_str() {
        "info" => info_action(args),
        "layers" => layers_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: info, layers, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("dockerfile"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the Dockerfile content as a string".to_string())
}

#[derive(Debug, Clone)]
struct Instruction {
    directive: String,
    args: String,
}

fn parse_dockerfile(text: &str) -> Vec<Instruction> {
    let mut instructions = Vec::new();
    let mut current: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            if current.is_some() {
                // Flush continuation — comment breaks it
                flush(&mut current, &mut instructions);
            }
            continue;
        }

        if let Some(ref mut buf) = current {
            // Previous line ended with backslash — continuation
            if buf.ends_with('\\') {
                buf.pop();
                buf.push(' ');
                buf.push_str(trimmed);
            } else {
                flush(&mut current, &mut instructions);
                current = Some(trimmed.to_string());
            }
        } else {
            current = Some(trimmed.to_string());
        }

        // Check if this accumulated line ends with continuation
        if let Some(ref buf) = current {
            if !buf.ends_with('\\') {
                flush(&mut current, &mut instructions);
            }
        }
    }
    if current.is_some() {
        flush(&mut current, &mut instructions);
    }
    instructions
}

fn flush(current: &mut Option<String>, out: &mut Vec<Instruction>) {
    if let Some(line) = current.take() {
        let line = line.trim().to_string();
        if line.is_empty() {
            return;
        }
        let (directive, rest) = if let Some(sp) = line.find(char::is_whitespace) {
            (line[..sp].to_uppercase(), line[sp..].trim().to_string())
        } else {
            (line.to_uppercase(), String::new())
        };
        out.push(Instruction {
            directive,
            args: rest,
        });
    }
}

struct DockerfileInfo {
    stages: Vec<(String, String, Option<String>)>, // (image, tag, alias)
    expose_ports: Vec<String>,
    labels: Vec<(String, String)>,
    workdir: Option<String>,
    user: Option<String>,
    cmd: Option<String>,
    entrypoint: Option<String>,
    healthcheck: Option<String>,
    env_count: usize,
    arg_count: usize,
    copy_count: usize,
    add_count: usize,
    run_count: usize,
}

fn analyze(instructions: &[Instruction]) -> DockerfileInfo {
    let mut info = DockerfileInfo {
        stages: Vec::new(),
        expose_ports: Vec::new(),
        labels: Vec::new(),
        workdir: None,
        user: None,
        cmd: None,
        entrypoint: None,
        healthcheck: None,
        env_count: 0,
        arg_count: 0,
        copy_count: 0,
        add_count: 0,
        run_count: 0,
    };

    for inst in instructions {
        match inst.directive.as_str() {
            "FROM" => {
                let parts: Vec<&str> = inst.args.splitn(3, ' ').collect();
                let image_ref = parts.first().copied().unwrap_or("");
                let alias = if parts.len() == 3 && parts[1].eq_ignore_ascii_case("as") {
                    Some(parts[2].to_string())
                } else {
                    None
                };
                let (image, tag) = if let Some(colon) = image_ref.rfind(':') {
                    (
                        image_ref[..colon].to_string(),
                        image_ref[colon + 1..].to_string(),
                    )
                } else {
                    (image_ref.to_string(), "latest".to_string())
                };
                info.stages.push((image, tag, alias));
            }
            "EXPOSE" => {
                for port in inst.args.split_whitespace() {
                    info.expose_ports.push(port.to_string());
                }
            }
            "LABEL" => {
                // key=value pairs, possibly multiple on one line
                for pair in split_label_pairs(&inst.args) {
                    info.labels.push(pair);
                }
            }
            "WORKDIR" => info.workdir = Some(inst.args.clone()),
            "USER" => info.user = Some(inst.args.clone()),
            "CMD" => info.cmd = Some(inst.args.clone()),
            "ENTRYPOINT" => info.entrypoint = Some(inst.args.clone()),
            "HEALTHCHECK" => info.healthcheck = Some(inst.args.clone()),
            "ENV" => info.env_count += 1,
            "ARG" => info.arg_count += 1,
            "COPY" => info.copy_count += 1,
            "ADD" => info.add_count += 1,
            "RUN" => info.run_count += 1,
            _ => {}
        }
    }
    info
}

fn split_label_pairs(s: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let s = s.trim();
    // Simple split on key=value, handling quoted values
    let mut i = 0;
    let chars: Vec<char> = s.chars().collect();
    while i < chars.len() {
        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        // Collect key (up to '=')
        let mut key = String::new();
        while i < chars.len() && chars[i] != '=' && !chars[i].is_whitespace() {
            key.push(chars[i]);
            i += 1;
        }
        if i >= chars.len() || chars[i] != '=' {
            break;
        }
        i += 1; // skip '='
                // Collect value
        let mut val = String::new();
        if i < chars.len() && chars[i] == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                val.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
        } else {
            while i < chars.len() && !chars[i].is_whitespace() {
                val.push(chars[i]);
                i += 1;
            }
        }
        if !key.is_empty() {
            pairs.push((key, val));
        }
    }
    pairs
}

fn info_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let instructions = parse_dockerfile(&text);
    let info = analyze(&instructions);

    let mut out = format!("Dockerfile\n{}\n\n", "=".repeat(44));

    let stage_count = info.stages.len();
    if stage_count == 0 {
        out += "No FROM instructions found.\n";
        return Ok(out);
    }

    out += &format!("Stages: {}\n\n", stage_count);
    for (i, (image, tag, alias)) in info.stages.iter().enumerate() {
        let alias_str = alias
            .as_deref()
            .map(|a| format!(" AS {}", a))
            .unwrap_or_default();
        out += &format!("  Stage {}: {}:{}{}\n", i + 1, image, tag, alias_str);
    }
    out += "\n";

    if let Some(ref wd) = info.workdir {
        out += &format!("WorkDir:    {}\n", wd);
    }
    if let Some(ref user) = info.user {
        out += &format!("User:       {}\n", user);
    }
    if !info.expose_ports.is_empty() {
        out += &format!("Expose:     {}\n", info.expose_ports.join(", "));
    }
    if let Some(ref cmd) = info.cmd {
        let snippet: String = cmd.chars().take(60).collect();
        let ellipsis = if cmd.len() > 60 { "..." } else { "" };
        out += &format!("CMD:        {}{}\n", snippet, ellipsis);
    }
    if let Some(ref ep) = info.entrypoint {
        let snippet: String = ep.chars().take(60).collect();
        let ellipsis = if ep.len() > 60 { "..." } else { "" };
        out += &format!("Entrypoint: {}{}\n", snippet, ellipsis);
    }
    if info.healthcheck.is_some() {
        out += "Healthcheck: defined\n";
    }

    out += "\n";
    out += &format!("RUN:        {}\n", info.run_count);
    out += &format!("COPY:       {}\n", info.copy_count);
    if info.add_count > 0 {
        out += &format!("ADD:        {}\n", info.add_count);
    }
    out += &format!("ENV:        {}\n", info.env_count);
    out += &format!("ARG:        {}\n", info.arg_count);

    if !info.labels.is_empty() {
        out += &format!("\nLabels ({}):\n", info.labels.len());
        for (k, v) in info.labels.iter().take(8) {
            out += &format!("  {:<24} {}\n", k, v);
        }
        if info.labels.len() > 8 {
            out += &format!("  ... ({} more)\n", info.labels.len() - 8);
        }
    }

    Ok(out)
}

fn layers_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let instructions = parse_dockerfile(&text);

    let mut out = format!(
        "Layers  [{} instruction(s)]\n{}\n\n",
        instructions.len(),
        "=".repeat(44)
    );

    for (i, inst) in instructions.iter().enumerate() {
        let snippet: String = inst.args.chars().take(72).collect();
        let ellipsis = if inst.args.len() > 72 { "…" } else { "" };
        out += &format!(
            "{:>3}. {:<12} {}{}\n",
            i + 1,
            inst.directive,
            snippet,
            ellipsis
        );
    }
    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let instructions = parse_dockerfile(&text);
    let info = analyze(&instructions);
    let mut warnings: Vec<String> = Vec::new();

    if info.stages.is_empty() {
        warnings.push("No FROM instruction found".to_string());
        return finish_validate(warnings, &info);
    }

    // Check last stage for 'latest' tag
    for (image, tag, alias) in &info.stages {
        if tag == "latest" {
            let label = alias.as_deref().unwrap_or(image.as_str());
            warnings.push(format!(
                "Stage '{}' uses the 'latest' tag — pin to a specific version for reproducible builds",
                label
            ));
        }
    }

    // Multi-stage: warn if more than one stage and last lacks alias
    if info.stages.len() > 1 {
        let last = info.stages.last().unwrap();
        if last.2.is_none() {
            // No alias on last stage is fine; skip
        }
    }

    // User check
    let runs_as_root = info.user.is_none()
        || info
            .user
            .as_deref()
            .map(|u| u == "root" || u == "0")
            .unwrap_or(false);
    if runs_as_root {
        warnings.push(
            "No USER instruction (or USER root) — container will run as root. Add a non-root USER for security."
                .to_string(),
        );
    }

    // ADD vs COPY
    if info.add_count > 0 {
        warnings.push(format!(
            "{} ADD instruction(s) — prefer COPY unless you need auto-extraction or remote URLs",
            info.add_count
        ));
    }

    // No HEALTHCHECK
    if info.healthcheck.is_none() && info.stages.len() == 1 {
        warnings.push(
            "No HEALTHCHECK instruction — Docker can't detect unhealthy containers without one"
                .to_string(),
        );
    }

    // Secrets in ENV or ARG (look for secret-shaped names)
    let secret_patterns = [
        "password",
        "secret",
        "token",
        "api_key",
        "apikey",
        "private_key",
        "passwd",
        "auth",
    ];
    for inst in &instructions {
        if inst.directive == "ENV" || inst.directive == "ARG" {
            let lower = inst.args.to_lowercase();
            for pat in &secret_patterns {
                if lower.contains(pat) {
                    warnings.push(format!(
                        "{} contains a secret-shaped name ('{}') — use --secret or build args at runtime instead",
                        inst.directive, pat
                    ));
                    break;
                }
            }
        }
    }

    // curl | sh pattern
    for inst in &instructions {
        if inst.directive == "RUN" {
            let lower = inst.args.to_lowercase();
            if (lower.contains("curl") || lower.contains("wget")) && lower.contains("| sh")
                || lower.contains("| bash")
            {
                warnings.push(
                    "RUN contains curl/wget piped to shell — this is a security risk; verify checksums instead".to_string(),
                );
            }
        }
    }

    // No CMD or ENTRYPOINT
    if info.cmd.is_none() && info.entrypoint.is_none() {
        warnings.push(
            "No CMD or ENTRYPOINT — image has no default command; base images are exempt"
                .to_string(),
        );
    }

    finish_validate(warnings, &info)
}

fn finish_validate(warnings: Vec<String>, info: &DockerfileInfo) -> Result<String, String> {
    let mut out = format!("Dockerfile Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!(
        "{} stage(s), {} instruction(s) total.\n",
        info.stages.len(),
        info.run_count
            + info.copy_count
            + info.add_count
            + info.env_count
            + info.arg_count
            + info.expose_ports.len()
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
