use serde_json::{json, Value};
use serde_yaml::Value as Yaml;

pub fn make_schema() -> Value {
    json!({
        "name": "ansible_tools",
        "description": "Parse and inspect Ansible playbooks and inventory files without external utilities.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "tasks", "vars", "handlers", "validate"],
                    "description": "parse=playbook overview with play/role/task counts, tasks=all tasks with module/name/tags, vars=all variable definitions, handlers=all handlers, validate=basic linting checks"
                },
                "yaml": { "type": "string", "description": "Ansible playbook YAML content" },
                "file": { "type": "string", "description": "Path to playbook .yml/.yaml file" },
                "tag": { "type": "string", "description": "Filter tasks by tag (for 'tasks' action)" }
            }
        }
    })
}

fn load_yaml(args: &Value) -> Result<Yaml, String> {
    let src = if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(f).map_err(|e| format!("Cannot read {}: {}", f, e))?
    } else if let Some(y) = args.get("yaml").and_then(|v| v.as_str()) {
        y.to_string()
    } else {
        return Err("Provide 'yaml' (inline content) or 'file' (path to playbook).".to_string());
    };
    serde_yaml::from_str(&src).map_err(|e| format!("YAML parse error: {}", e))
}

fn yaml_str(v: &Yaml) -> String {
    match v {
        Yaml::String(s) => s.clone(),
        Yaml::Bool(b) => b.to_string(),
        Yaml::Number(n) => n.to_string(),
        Yaml::Null => "~".to_string(),
        other => serde_yaml::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

struct TaskInfo {
    name: String,
    module: String,
    tags: Vec<String>,
    when: Option<String>,
    notify: Vec<String>,
    loop_with: bool,
    delegate_to: Option<String>,
}

fn extract_task(t: &Yaml) -> Option<TaskInfo> {
    let map = t.as_mapping()?;
    let name = map
        .get("name")
        .map(yaml_str)
        .unwrap_or_else(|| "(unnamed)".to_string());
    let when = map.get("when").map(yaml_str);
    let delegate_to = map.get("delegate_to").map(yaml_str);
    let loop_with =
        map.contains_key("loop") || map.contains_key("with_items") || map.contains_key("with_list");
    let notify = map
        .get("notify")
        .map(|v| match v {
            Yaml::Sequence(seq) => seq.iter().map(yaml_str).collect(),
            other => vec![yaml_str(other)],
        })
        .unwrap_or_default();
    let tags = map
        .get("tags")
        .map(|v| match v {
            Yaml::Sequence(seq) => seq.iter().map(yaml_str).collect(),
            other => vec![yaml_str(other)],
        })
        .unwrap_or_default();

    // Find the module: any key that isn't a task-control key
    let control_keys = [
        "name",
        "when",
        "notify",
        "tags",
        "register",
        "loop",
        "with_items",
        "with_list",
        "with_dict",
        "with_subelements",
        "with_sequence",
        "become",
        "become_user",
        "ignore_errors",
        "failed_when",
        "changed_when",
        "delegate_to",
        "delegate_facts",
        "environment",
        "no_log",
        "run_once",
        "any_errors_fatal",
        "timeout",
        "vars",
        "listen",
        "block",
        "rescue",
        "always",
        "include_tasks",
        "import_tasks",
        "include_role",
        "import_role",
    ];
    let module = map
        .keys()
        .filter_map(|k| {
            let ks = yaml_str(k);
            if control_keys.contains(&ks.as_str()) {
                None
            } else {
                Some(ks)
            }
        })
        .next()
        .unwrap_or_else(|| "unknown".to_string());

    // Handle block/include specially
    let module = if map.contains_key("block") {
        "block".to_string()
    } else if map.contains_key("include_tasks") {
        "include_tasks".to_string()
    } else if map.contains_key("import_tasks") {
        "import_tasks".to_string()
    } else if map.contains_key("include_role") {
        "include_role".to_string()
    } else if map.contains_key("import_role") {
        "import_role".to_string()
    } else {
        module
    };

    Some(TaskInfo {
        name,
        module,
        tags,
        when,
        notify,
        loop_with,
        delegate_to,
    })
}

fn collect_tasks(task_list: &Yaml, out: &mut Vec<TaskInfo>) {
    if let Some(seq) = task_list.as_sequence() {
        for t in seq {
            // Handle block/rescue/always
            if let Some(map) = t.as_mapping() {
                if let Some(block) = map.get("block") {
                    collect_tasks(block, out);
                }
                if let Some(rescue) = map.get("rescue") {
                    collect_tasks(rescue, out);
                }
                if let Some(always) = map.get("always") {
                    collect_tasks(always, out);
                }
            }
            if let Some(ti) = extract_task(t) {
                out.push(ti);
            }
        }
    }
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    let doc = load_yaml(args)?;

    // A playbook is a sequence of plays
    let plays = match &doc {
        Yaml::Sequence(seq) => seq.clone(),
        _ => {
            return Err(
                "Expected a playbook (YAML sequence of plays) at the top level.".to_string(),
            )
        }
    };

    match action {
        "parse" => action_parse(&plays),
        "tasks" => {
            let tag_filter = args.get("tag").and_then(|v| v.as_str());
            action_tasks(&plays, tag_filter)
        }
        "vars" => action_vars(&plays),
        "handlers" => action_handlers(&plays),
        "validate" => action_validate(&plays),
        _ => Err(format!(
            "Unknown action '{}'. Use: parse, tasks, vars, handlers, validate",
            action
        )),
    }
}

fn action_parse(plays: &[Yaml]) -> Result<String, String> {
    let mut out = String::new();
    out.push_str(&format!(
        "## Ansible Playbook — {} play(s)\n\n",
        plays.len()
    ));

    for (i, play) in plays.iter().enumerate() {
        let map = match play.as_mapping() {
            Some(m) => m,
            None => continue,
        };
        let name = map
            .get("name")
            .map(yaml_str)
            .unwrap_or_else(|| format!("Play {}", i + 1));
        let hosts = map
            .get("hosts")
            .map(yaml_str)
            .unwrap_or_else(|| "?".to_string());
        let become_flag = map
            .get("become")
            .map(yaml_str)
            .unwrap_or_else(|| "false".to_string());

        let mut tasks: Vec<TaskInfo> = Vec::new();
        if let Some(tl) = map.get("tasks") {
            collect_tasks(tl, &mut tasks);
        }
        if let Some(pl) = map.get("pre_tasks") {
            collect_tasks(pl, &mut tasks);
        }
        if let Some(pl) = map.get("post_tasks") {
            collect_tasks(pl, &mut tasks);
        }

        let mut handlers: Vec<TaskInfo> = Vec::new();
        if let Some(hl) = map.get("handlers") {
            collect_tasks(hl, &mut handlers);
        }

        let roles: Vec<String> = map
            .get("roles")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .map(|r| match r {
                        Yaml::String(s) => s.clone(),
                        Yaml::Mapping(m) => {
                            m.get("role").map(yaml_str).unwrap_or_else(|| yaml_str(r))
                        }
                        other => yaml_str(other),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let gather_facts = map
            .get("gather_facts")
            .map(yaml_str)
            .unwrap_or_else(|| "true".to_string());
        let vars_count = map
            .get("vars")
            .and_then(|v| v.as_mapping())
            .map(|m| m.len())
            .unwrap_or(0);

        out.push_str(&format!("### {} — hosts: {}\n", name, hosts));
        out.push_str(&format!(
            "Tasks: {}  Handlers: {}  Roles: {}  Vars: {}\n",
            tasks.len(),
            handlers.len(),
            roles.len(),
            vars_count
        ));
        out.push_str(&format!(
            "become: {}  gather_facts: {}\n",
            become_flag, gather_facts
        ));
        if !roles.is_empty() {
            out.push_str(&format!("Roles: {}\n", roles.join(", ")));
        }

        // Module frequency
        let mut mod_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for t in &tasks {
            *mod_counts.entry(t.module.clone()).or_insert(0) += 1;
        }
        let mut mod_list: Vec<_> = mod_counts.into_iter().collect();
        mod_list.sort_by_key(|b| std::cmp::Reverse(b.1));
        if !mod_list.is_empty() {
            let top: Vec<String> = mod_list
                .iter()
                .take(6)
                .map(|(m, c)| format!("{} ×{}", m, c))
                .collect();
            out.push_str(&format!("Top modules: {}\n", top.join(", ")));
        }
        out.push('\n');
    }
    Ok(out)
}

fn action_tasks(plays: &[Yaml], tag_filter: Option<&str>) -> Result<String, String> {
    let mut out = String::new();
    let filter_note = tag_filter
        .map(|t| format!(" (tag: {})", t))
        .unwrap_or_default();
    out.push_str(&format!("## Tasks{}\n\n", filter_note));
    let mut total = 0;
    for play in plays {
        let map = match play.as_mapping() {
            Some(m) => m,
            None => continue,
        };
        let play_name = map
            .get("name")
            .map(yaml_str)
            .unwrap_or_else(|| "unnamed play".to_string());
        let mut tasks: Vec<TaskInfo> = Vec::new();
        if let Some(tl) = map.get("pre_tasks") {
            collect_tasks(tl, &mut tasks);
        }
        if let Some(tl) = map.get("tasks") {
            collect_tasks(tl, &mut tasks);
        }
        if let Some(tl) = map.get("post_tasks") {
            collect_tasks(tl, &mut tasks);
        }

        let filtered: Vec<_> = tasks
            .iter()
            .filter(|t| {
                if let Some(tag) = tag_filter {
                    t.tags.iter().any(|tg| tg == tag)
                } else {
                    true
                }
            })
            .collect();

        if filtered.is_empty() {
            continue;
        }
        out.push_str(&format!("**{}**\n", play_name));
        for t in &filtered {
            total += 1;
            let tag_str = if t.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", t.tags.join(","))
            };
            let loop_str = if t.loop_with { " ↺" } else { "" };
            let when_str = if t.when.is_some() { " (when)" } else { "" };
            let del_str = t
                .delegate_to
                .as_deref()
                .map(|d| format!(" →{}", d))
                .unwrap_or_default();
            out.push_str(&format!(
                "  {}{}: {}{}{}{}\n",
                t.module, tag_str, t.name, loop_str, when_str, del_str
            ));
            if !t.notify.is_empty() {
                out.push_str(&format!("    notify: {}\n", t.notify.join(", ")));
            }
        }
        out.push('\n');
    }
    out.push_str(&format!("{} task(s)\n", total));
    Ok(out)
}

fn action_vars(plays: &[Yaml]) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("## Variables\n\n");
    let mut total = 0;
    for play in plays {
        let map = match play.as_mapping() {
            Some(m) => m,
            None => continue,
        };
        let play_name = map
            .get("name")
            .map(yaml_str)
            .unwrap_or_else(|| "unnamed play".to_string());
        if let Some(vars) = map.get("vars").and_then(|v| v.as_mapping()) {
            if !vars.is_empty() {
                out.push_str(&format!("**{}**\n", play_name));
                for (k, v) in vars {
                    total += 1;
                    let val = yaml_str(v);
                    let display: String = val.chars().take(80).collect();
                    let ellipsis = if val.len() > 80 { "…" } else { "" };
                    out.push_str(&format!("  {}: {}{}\n", yaml_str(k), display, ellipsis));
                }
                out.push('\n');
            }
        }
        // vars_files
        if let Some(files) = map.get("vars_files").and_then(|v| v.as_sequence()) {
            out.push_str(&format!("**{}** — vars_files:\n", play_name));
            for f in files {
                out.push_str(&format!("  - {}\n", yaml_str(f)));
            }
            out.push('\n');
        }
    }
    if total == 0 {
        out.push_str("No inline variables defined. Check vars_files or group_vars/host_vars.\n");
    } else {
        out.push_str(&format!("{} variable(s) total\n", total));
    }
    Ok(out)
}

fn action_handlers(plays: &[Yaml]) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("## Handlers\n\n");
    let mut total = 0;
    for play in plays {
        let map = match play.as_mapping() {
            Some(m) => m,
            None => continue,
        };
        let play_name = map
            .get("name")
            .map(yaml_str)
            .unwrap_or_else(|| "unnamed play".to_string());
        let mut handlers: Vec<TaskInfo> = Vec::new();
        if let Some(hl) = map.get("handlers") {
            collect_tasks(hl, &mut handlers);
        }
        if !handlers.is_empty() {
            out.push_str(&format!("**{}**\n", play_name));
            for h in &handlers {
                total += 1;
                let listen = "(any notify will match by name)";
                out.push_str(&format!("  {}: {}  {}\n", h.module, h.name, listen));
            }
            out.push('\n');
        }
    }
    if total == 0 {
        out.push_str("No handlers defined.\n");
    } else {
        out.push_str(&format!("{} handler(s) total\n", total));
    }
    Ok(out)
}

fn action_validate(plays: &[Yaml]) -> Result<String, String> {
    let mut issues: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut ok_count = 0;

    for (i, play) in plays.iter().enumerate() {
        let map = match play.as_mapping() {
            Some(m) => m,
            None => {
                issues.push(format!("Play {} is not a mapping", i));
                continue;
            }
        };
        let play_name = map
            .get("name")
            .map(yaml_str)
            .unwrap_or_else(|| format!("Play {}", i + 1));

        if map.get("hosts").is_none() {
            issues.push(format!("[{}] Missing 'hosts' directive", play_name));
        } else {
            ok_count += 1;
        }

        let has_tasks = map.get("tasks").is_some();
        let has_roles = map.get("roles").is_some();
        let has_import = map.get("import_playbook").is_some();
        if !has_tasks && !has_roles && !has_import {
            warnings.push(format!(
                "[{}] No tasks, roles, or import_playbook — play does nothing",
                play_name
            ));
        }

        // Check for tasks with no name
        let mut unnamed = 0;
        let mut tasks: Vec<TaskInfo> = Vec::new();
        if let Some(tl) = map.get("tasks") {
            collect_tasks(tl, &mut tasks);
        }
        for t in &tasks {
            if t.name == "(unnamed)" {
                unnamed += 1;
            }
        }
        if unnamed > 0 {
            warnings.push(format!(
                "[{}] {} task(s) have no 'name' field",
                play_name, unnamed
            ));
        }

        // Check become without become_user is fine, but warn if become_user without become
        if map.get("become_user").is_some() && map.get("become").is_none() {
            warnings.push(format!(
                "[{}] 'become_user' set but 'become' not explicitly enabled",
                play_name
            ));
        }
    }

    let mut out = String::new();
    let verdict = if issues.is_empty() {
        "VALID"
    } else {
        "INVALID"
    };
    out.push_str(&format!("## Validate — {}\n\n", verdict));
    out.push_str(&format!(
        "Plays checked: {}  Issues: {}  Warnings: {}\n\n",
        plays.len(),
        issues.len(),
        warnings.len()
    ));

    if !issues.is_empty() {
        out.push_str("**Issues (must fix):**\n");
        for issue in &issues {
            out.push_str(&format!("  ✗ {}\n", issue));
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
    let _ = ok_count;
    Ok(out)
}
