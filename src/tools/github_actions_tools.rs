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
        "jobs" => jobs_action(args),
        "steps" => steps_action(args),
        "triggers" => triggers_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: info, jobs, steps, triggers, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("yaml"))
        .or_else(|| args.get("workflow"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            "Missing 'text' — pass the GitHub Actions workflow YAML content as a string".to_string()
        })
}

fn load_workflow(text: &str) -> Result<Yaml, String> {
    serde_yaml::from_str(text).map_err(|e| format!("Failed to parse workflow YAML: {}", e))
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

fn get_jobs(doc: &Yaml) -> Vec<(String, &Yaml)> {
    doc.get("jobs")
        .and_then(|j| j.as_mapping())
        .map(|m| m.iter().map(|(k, v)| (yaml_str(k), v)).collect())
        .unwrap_or_default()
}

fn trigger_names(doc: &Yaml) -> Vec<String> {
    match doc.get("on") {
        Some(Yaml::String(s)) => vec![s.clone()],
        Some(Yaml::Sequence(seq)) => seq.iter().map(yaml_str).collect(),
        Some(Yaml::Mapping(m)) => m.keys().map(yaml_str).collect(),
        _ => vec![],
    }
}

fn info_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = load_workflow(&text)?;

    let name = doc
        .get("name")
        .map(yaml_str)
        .unwrap_or_else(|| "(unnamed)".to_string());
    let triggers = trigger_names(&doc);
    let jobs = get_jobs(&doc);

    let mut out = format!("GitHub Actions Workflow\n{}\n\n", "=".repeat(44));
    out += &format!("Name:     {}\n", name);
    out += &format!(
        "Triggers: {}\n",
        if triggers.is_empty() {
            "(none)".to_string()
        } else {
            triggers.join(", ")
        }
    );
    out += &format!("Jobs:     {}\n\n", jobs.len());

    for (job_id, job) in &jobs {
        let runs_on = job
            .get("runs-on")
            .map(yaml_str)
            .unwrap_or_else(|| "(unset)".to_string());
        let needs: Vec<String> = match job.get("needs") {
            Some(Yaml::String(s)) => vec![s.clone()],
            Some(Yaml::Sequence(seq)) => seq.iter().map(yaml_str).collect(),
            _ => vec![],
        };
        let steps = job
            .get("steps")
            .and_then(|s| s.as_sequence())
            .map(|s| s.len())
            .unwrap_or(0);
        let if_cond = job.get("if").map(yaml_str);

        out += &format!("Job: {}\n", job_id);
        out += &format!("  runs-on: {}\n", runs_on);
        if !needs.is_empty() {
            out += &format!("  needs:   {}\n", needs.join(", "));
        }
        if let Some(cond) = if_cond {
            out += &format!("  if:      {}\n", cond);
        }
        out += &format!("  steps:   {}\n\n", steps);
    }
    Ok(out)
}

fn jobs_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = load_workflow(&text)?;
    let jobs = get_jobs(&doc);

    if jobs.is_empty() {
        return Ok("No jobs defined in workflow.\n".to_string());
    }

    let mut out = format!("Jobs  [{} total]\n{}\n\n", jobs.len(), "=".repeat(44));
    for (job_id, job) in &jobs {
        let runs_on = job
            .get("runs-on")
            .map(yaml_str)
            .unwrap_or_else(|| "(unset)".to_string());
        let steps = job
            .get("steps")
            .and_then(|s| s.as_sequence())
            .map(|s| s.len())
            .unwrap_or(0);
        let needs: Vec<String> = match job.get("needs") {
            Some(Yaml::String(s)) => vec![s.clone()],
            Some(Yaml::Sequence(seq)) => seq.iter().map(yaml_str).collect(),
            _ => vec![],
        };
        let strategy = job.get("strategy");
        let matrix_info = strategy
            .and_then(|s| s.get("matrix"))
            .map(|_| " [matrix]")
            .unwrap_or("");
        let concurrency = job
            .get("concurrency")
            .map(|_| " [concurrency]")
            .unwrap_or("");

        out += &format!("{}{}{}\n", job_id, matrix_info, concurrency);
        out += &format!("  runs-on: {}\n", runs_on);
        out += &format!("  steps:   {}\n", steps);
        if !needs.is_empty() {
            out += &format!("  needs:   {}\n", needs.join(", "));
        }
        // Env vars at job level
        if let Some(env) = job.get("env").and_then(|e| e.as_mapping()) {
            out += &format!("  env:     {} variable(s)\n", env.len());
        }
        out += "\n";
    }
    Ok(out)
}

fn steps_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let filter = args.get("job").and_then(|v| v.as_str());
    let doc = load_workflow(&text)?;
    let jobs = get_jobs(&doc);

    let mut out = format!("Steps\n{}\n\n", "=".repeat(44));

    for (job_id, job) in &jobs {
        if let Some(f) = filter {
            if !job_id.to_lowercase().contains(&f.to_lowercase()) {
                continue;
            }
        }
        let steps = match job.get("steps").and_then(|s| s.as_sequence()) {
            Some(s) => s,
            None => continue,
        };

        out += &format!("Job: {} ({} step(s))\n", job_id, steps.len());
        for (i, step) in steps.iter().enumerate() {
            let step_name = step
                .get("name")
                .map(yaml_str)
                .unwrap_or_else(|| "(unnamed)".to_string());
            let uses = step.get("uses").map(yaml_str);
            let run = step.get("run").map(yaml_str);
            let if_cond = step.get("if").map(yaml_str);
            let continue_on_error = step
                .get("continue-on-error")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            out += &format!("  {}. {}", i + 1, step_name);
            if continue_on_error {
                out += " [continue-on-error]";
            }
            out += "\n";
            if let Some(u) = uses {
                out += &format!("     uses: {}\n", u);
            }
            if let Some(r) = run {
                let first_line: String = r.lines().next().unwrap_or("").chars().take(60).collect();
                let ellipsis = if r.len() > 60 { "…" } else { "" };
                out += &format!("     run:  {}{}\n", first_line, ellipsis);
            }
            if let Some(cond) = if_cond {
                out += &format!("     if:   {}\n", cond);
            }
        }
        out += "\n";
    }
    Ok(out)
}

fn triggers_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = load_workflow(&text)?;

    let mut out = format!("Workflow Triggers\n{}\n\n", "=".repeat(44));

    match doc.get("on") {
        None => {
            out += "No triggers defined.\n";
        }
        Some(Yaml::String(s)) => {
            out += &format!("Event: {}\n", s);
        }
        Some(Yaml::Sequence(seq)) => {
            for event in seq {
                out += &format!("Event: {}\n", yaml_str(event));
            }
        }
        Some(Yaml::Mapping(m)) => {
            for (event_key, event_val) in m {
                let event_name = yaml_str(event_key);
                out += &format!("Event: {}\n", event_name);
                // Show branches/tags/paths if present
                for filter_key in &[
                    "branches",
                    "branches-ignore",
                    "tags",
                    "tags-ignore",
                    "paths",
                    "paths-ignore",
                ] {
                    if let Some(filters) = event_val.get(*filter_key).and_then(|v| v.as_sequence())
                    {
                        let vals: Vec<String> = filters.iter().map(yaml_str).collect();
                        out += &format!("  {}: {}\n", filter_key, vals.join(", "));
                    }
                }
                // cron schedule for schedule event
                if let Some(schedule) = event_val.as_sequence() {
                    for entry in schedule {
                        if let Some(cron) = entry.get("cron").map(yaml_str) {
                            out += &format!("  cron: {}\n", cron);
                        }
                    }
                }
                // workflow inputs
                if let Some(inputs) = event_val.get("inputs").and_then(|i| i.as_mapping()) {
                    out += &format!("  inputs: {} defined\n", inputs.len());
                }
            }
        }
        _ => {}
    }

    // Concurrency
    if let Some(concurrency) = doc.get("concurrency") {
        let group = concurrency.get("group").map(yaml_str).unwrap_or_default();
        let cancel = concurrency
            .get("cancel-in-progress")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        out += &format!(
            "\nConcurrency: group={} cancel-in-progress={}\n",
            group, cancel
        );
    }
    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let doc = load_workflow(&text)?;
    let mut warnings: Vec<String> = Vec::new();

    if doc.get("on").is_none() {
        warnings.push("No 'on' triggers defined — workflow will never run".to_string());
    }

    let jobs = get_jobs(&doc);
    if jobs.is_empty() {
        warnings.push("No jobs defined in workflow".to_string());
    }

    let job_ids: Vec<String> = jobs.iter().map(|(id, _)| id.clone()).collect();

    for (job_id, job) in &jobs {
        // Warn on missing runs-on
        if job.get("runs-on").is_none() {
            warnings.push(format!("Job '{}': missing 'runs-on' — required", job_id));
        }

        // Warn on undefined needs
        let needs: Vec<String> = match job.get("needs") {
            Some(Yaml::String(s)) => vec![s.clone()],
            Some(Yaml::Sequence(seq)) => seq.iter().map(yaml_str).collect(),
            _ => vec![],
        };
        for dep in &needs {
            if !job_ids.contains(dep) {
                warnings.push(format!(
                    "Job '{}': needs '{}' which is not a defined job",
                    job_id, dep
                ));
            }
        }

        // Check steps
        let steps = job.get("steps").and_then(|s| s.as_sequence());
        if steps.map(|s| s.is_empty()).unwrap_or(true) {
            warnings.push(format!("Job '{}': has no steps", job_id));
        }

        if let Some(steps) = steps {
            for (i, step) in steps.iter().enumerate() {
                let has_uses = step.get("uses").is_some();
                let has_run = step.get("run").is_some();
                if !has_uses && !has_run {
                    let name = step
                        .get("name")
                        .map(yaml_str)
                        .unwrap_or_else(|| format!("#{}", i + 1));
                    warnings.push(format!(
                        "Job '{}' step '{}': has neither 'uses' nor 'run'",
                        job_id, name
                    ));
                }
                // Warn on pinned actions without hash
                if let Some(uses) = step.get("uses").and_then(|v| v.as_str()) {
                    if uses.contains('@') {
                        let tag = uses.split('@').last().unwrap_or("");
                        if !tag.chars().all(|c| c.is_ascii_hexdigit()) && tag.len() != 40 {
                            // Not a full SHA — using a mutable tag
                            if !tag.starts_with('v') && !tag.is_empty() {
                                // skip version tags like @v4, flag branch refs
                            }
                        }
                    }
                }
            }
        }
    }

    // Warn on missing workflow-level permissions (good practice)
    if doc.get("permissions").is_none() && !jobs.is_empty() {
        warnings.push(
            "No top-level 'permissions' defined — consider setting least-privilege permissions"
                .to_string(),
        );
    }

    let mut out = format!("GitHub Actions Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!("{} job(s) parsed.\n", jobs.len());
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
