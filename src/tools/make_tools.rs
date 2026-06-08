use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("target").is_some() {
        "explain".to_string()
    } else {
        "list".to_string()
    };
    match action.as_str() {
        "list" => list_action(args),
        "explain" => explain_action(args),
        "deps" => deps_action(args),
        "vars" => vars_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: list, explain, deps, vars",
            action
        )),
    }
}

fn get_makefile(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("makefile"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the Makefile content as a string".to_string())
}

#[derive(Debug, Clone)]
struct MakeTarget {
    name: String,
    deps: Vec<String>,
    commands: Vec<String>,
    comment: Option<String>,
    is_phony: bool,
}

#[derive(Debug, Clone)]
struct MakeVar {
    name: String,
    value: String,
    kind: &'static str, // "=", ":=", "?=", "+="
}

fn parse_makefile(text: &str) -> (Vec<MakeTarget>, Vec<MakeVar>, Vec<String>) {
    let mut targets: Vec<MakeTarget> = Vec::new();
    let mut vars: Vec<MakeVar> = Vec::new();
    let mut phony_names: Vec<String> = Vec::new();
    let mut pending_comment: Option<String> = None;
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0usize;

    while i < lines.len() {
        let raw = lines[i];
        let trimmed = raw.trim();

        // Continuation lines are handled in-context
        if trimmed.is_empty() {
            pending_comment = None;
            i += 1;
            continue;
        }

        // Comment
        if let Some(comment_rest) = trimmed.strip_prefix('#') {
            let comment_text = comment_rest.trim().to_string();
            if !comment_text.is_empty() {
                pending_comment = Some(comment_text);
            }
            i += 1;
            continue;
        }

        // .PHONY declaration
        if let Some(phony_rest) = trimmed.strip_prefix(".PHONY:") {
            let rest = &phony_rest.trim().to_string();
            for name in rest.split_whitespace() {
                phony_names.push(name.to_string());
            }
            i += 1;
            continue;
        }

        // Variable assignment: name = / := / ?= / +=
        let var_kinds = [" := ", " = ", " ?= ", " += "];
        if let Some(kind) = var_kinds.iter().find(|k| trimmed.contains(*k)) {
            let kk = kind.trim_start();
            if let Some(eq_pos) = trimmed.find(kind) {
                let name = trimmed[..eq_pos].trim().to_string();
                let value = trimmed[eq_pos + kind.len()..].trim().to_string();
                // filter out things that look like target rules with colons
                if !name.contains(':') && !name.is_empty() {
                    vars.push(MakeVar {
                        name,
                        value,
                        kind: kk,
                    });
                    i += 1;
                    continue;
                }
            }
        }

        // Target rule: name(s): deps
        // Tabs at line start = command, must start without leading tab for target line
        if !raw.starts_with('\t') && trimmed.contains(':') && !trimmed.starts_with('#') {
            let colon_pos = if let Some(p) = trimmed.find(':') {
                p
            } else {
                i += 1;
                continue;
            };
            // Skip :: (double colon) targets — treat same
            let target_part = trimmed[..colon_pos].trim();
            let rest = trimmed[colon_pos + 1..].trim_start_matches(':').trim();

            if target_part.is_empty() || target_part.contains('=') {
                i += 1;
                continue;
            }

            let deps: Vec<String> = rest
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();

            // Collect command lines (lines starting with tab)
            let mut commands: Vec<String> = Vec::new();
            i += 1;
            while i < lines.len() {
                let cmd_line = lines[i];
                if cmd_line.starts_with('\t') {
                    let cmd = cmd_line.trim().to_string();
                    // strip leading @ and - prefixes
                    let cmd_clean = cmd
                        .trim_start_matches('@')
                        .trim_start_matches('-')
                        .trim()
                        .to_string();
                    if !cmd_clean.is_empty() {
                        commands.push(cmd_clean);
                    }
                    i += 1;
                } else {
                    break;
                }
            }

            let target_name = target_part.to_string();
            targets.push(MakeTarget {
                name: target_name,
                deps,
                commands,
                comment: pending_comment.take(),
                is_phony: false,
            });
            continue;
        }

        pending_comment = None;
        i += 1;
    }

    // Mark phony targets
    for t in targets.iter_mut() {
        if phony_names.contains(&t.name) {
            t.is_phony = true;
        }
    }

    (targets, vars, phony_names)
}

fn list_action(args: &Value) -> Result<String, String> {
    let text = get_makefile(args)?;
    let (targets, vars, _) = parse_makefile(&text);

    let mut out = format!(
        "Makefile  [{} target(s)  {} variable(s)]\n{}\n\n",
        targets.len(),
        vars.len(),
        "=".repeat(44)
    );

    if targets.is_empty() {
        out += "No targets found.\n";
    } else {
        out += &format!("{:<24} {:<8} {}\n", "Target", "Phony", "Dependencies");
        out += &format!("{}\n", "-".repeat(60));
        for t in &targets {
            let phony_str = if t.is_phony { "yes" } else { "no" };
            let deps_str = if t.deps.is_empty() {
                "(none)".to_string()
            } else {
                t.deps.join(", ")
            };
            out += &format!("{:<24} {:<8} {}\n", t.name, phony_str, deps_str);
            if let Some(c) = &t.comment {
                out += &format!("  # {}\n", c);
            }
        }
    }
    Ok(out)
}

fn explain_action(args: &Value) -> Result<String, String> {
    let text = get_makefile(args)?;
    let target_name = args
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'target' — the target name to explain")?;

    let (targets, _, _) = parse_makefile(&text);
    let target = targets
        .iter()
        .find(|t| t.name == target_name)
        .ok_or_else(|| {
            format!(
                "Target '{}' not found. Use action='list' to see all targets.",
                target_name
            )
        })?;

    let mut out = format!("Target: {}\n{}\n\n", target.name, "=".repeat(44));
    if let Some(c) = &target.comment {
        out += &format!("Description: {}\n\n", c);
    }
    out += &format!("Phony:  {}\n", if target.is_phony { "yes" } else { "no" });
    if target.deps.is_empty() {
        out += "Deps:   (none)\n";
    } else {
        out += &format!("Deps:   {}\n", target.deps.join(", "));
    }
    out += &format!("Commands: {}\n", target.commands.len());
    for (i, cmd) in target.commands.iter().enumerate() {
        out += &format!("  {}. {}\n", i + 1, cmd);
    }
    Ok(out)
}

fn deps_action(args: &Value) -> Result<String, String> {
    let text = get_makefile(args)?;
    let (targets, _, _) = parse_makefile(&text);

    let filter = args.get("target").and_then(|v| v.as_str());

    let mut out = format!("Makefile Dependencies\n{}\n\n", "=".repeat(44));

    for t in &targets {
        if let Some(f) = filter {
            if t.name != f {
                continue;
            }
        }
        if t.deps.is_empty() {
            out += &format!("{} (no deps)\n", t.name);
        } else {
            out += &format!("{} <-- {}\n", t.name, t.deps.join(", "));
        }
    }
    Ok(out)
}

fn vars_action(args: &Value) -> Result<String, String> {
    let text = get_makefile(args)?;
    let (_, vars, _) = parse_makefile(&text);

    let mut out = format!(
        "Makefile Variables  [{} total]\n{}\n\n",
        vars.len(),
        "=".repeat(44)
    );

    if vars.is_empty() {
        out += "No variables found.\n";
    } else {
        out += &format!("{:<24} {:<4} {}\n", "Variable", "Op", "Value");
        out += &format!("{}\n", "-".repeat(60));
        for v in &vars {
            let val_display = if v.value.len() > 40 {
                format!("{}...", &v.value[..37])
            } else {
                v.value.clone()
            };
            out += &format!("{:<24} {:<4} {}\n", v.name, v.kind, val_display);
        }
    }
    Ok(out)
}
