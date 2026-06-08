use serde_json::Value;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "gitlab_ci_tools",
        "description": "Parse, inspect, and validate .gitlab-ci.yml GitLab CI/CD pipeline configurations. Works offline — no network or GitLab server required.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "parse", "jobs", "list", "stages", "validate", "check"],
                    "description": "info/parse (default — pipeline overview with stages and job map), jobs/list (detailed per-job breakdown), stages (stage list with job counts), validate/check (issue detection)"
                },
                "text": { "type": "string", "description": ".gitlab-ci.yml content" },
                "yaml": { "type": "string", "description": "Alias for 'text'" },
                "ci": { "type": "string", "description": "Alias for 'text'" },
                "file": { "type": "string", "description": "Path to a .gitlab-ci.yml file" },
                "stage": { "type": "string", "description": "Filter jobs action to a specific stage name" },
                "templates": { "type": "boolean", "description": "Include hidden template jobs (starting with .) in jobs listing" }
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let text = if let Some(t) = args
        .get("text")
        .or_else(|| args.get("yaml"))
        .or_else(|| args.get("ci"))
        .and_then(|v| v.as_str())
    {
        t.to_string()
    } else if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(f).map_err(|e| format!("Failed to read file: {e}"))?
    } else {
        return Err("Provide 'text' with .gitlab-ci.yml content or 'file' path.".to_string());
    };

    let doc: Value = serde_yaml::from_str(&text).map_err(|e| format!("YAML parse error: {e}"))?;

    match action {
        "info" | "parse" => action_info(&doc),
        "jobs" | "list" => action_jobs(&doc, args),
        "stages" => action_stages(&doc),
        "validate" | "check" => action_validate(&doc),
        _ => Err(format!(
            "Unknown action '{}'. Use: info, jobs, stages, validate.",
            action
        )),
    }
}

// Global-level GitLab CI keys that are NOT job definitions
const GLOBAL_KEYS: &[&str] = &[
    "stages",
    "variables",
    "default",
    "include",
    "workflow",
    "image",
    "services",
    "before_script",
    "after_script",
    "cache",
    "artifacts",
];

fn is_job(key: &str, value: &Value) -> bool {
    if GLOBAL_KEYS.contains(&key) {
        return false;
    }
    // Hidden template jobs (start with .) are still jobs but flagged separately
    // A job must be a mapping (object)
    value.is_object()
}

fn is_template(key: &str) -> bool {
    key.starts_with('.')
}

struct JobSummary {
    name: String,
    stage: Option<String>,
    image: Option<String>,
    script_lines: usize,
    needs: Vec<String>,
    rules_count: usize,
    extends: Option<String>,
    is_template: bool,
    allow_failure: bool,
    parallel: Option<u64>,
    tags: Vec<String>,
    when: Option<String>,
}

fn summarize_job(name: &str, job: &Value) -> JobSummary {
    let stage = job
        .get("stage")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let image = job.get("image").and_then(|v| {
        if v.is_string() {
            v.as_str().map(|s| s.to_string())
        } else {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        }
    });

    let script_lines = job
        .get("script")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let needs: Vec<String> = job
        .get("needs")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    if v.is_string() {
                        v.as_str().map(|s| s.to_string())
                    } else {
                        v.get("job").and_then(|j| j.as_str()).map(|s| s.to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let rules_count = job
        .get("rules")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let extends = job.get("extends").and_then(|v| {
        if v.is_string() {
            v.as_str().map(|s| s.to_string())
        } else {
            v.as_array()
                .and_then(|a| a.first())
                .and_then(|first| first.as_str())
                .map(|s| s.to_string())
        }
    });

    let allow_failure = job
        .get("allow_failure")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let parallel = job.get("parallel").and_then(|v| v.as_u64());

    let tags: Vec<String> = job
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    let when = job
        .get("when")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    JobSummary {
        name: name.to_string(),
        stage,
        image,
        script_lines,
        needs,
        rules_count,
        extends,
        is_template: is_template(name),
        allow_failure,
        parallel,
        tags,
        when,
    }
}

fn action_info(doc: &Value) -> Result<String, String> {
    let obj = doc
        .as_object()
        .ok_or("Expected a YAML mapping at the top level.")?;

    let mut out = String::from("GitLab CI Configuration\n");
    out.push_str(&format!("{}\n\n", "═".repeat(45)));

    // Stages
    let declared_stages: Vec<String> = doc
        .get("stages")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    if declared_stages.is_empty() {
        out.push_str("Stages: (default: build, test, deploy)\n");
    } else {
        out.push_str(&format!("Stages: {}\n", declared_stages.join(" → ")));
    }

    // Count jobs vs templates
    let mut jobs = 0usize;
    let mut templates = 0usize;
    let mut used_stages: std::collections::HashSet<String> = Default::default();

    for (key, val) in obj {
        if is_job(key, val) {
            if is_template(key) {
                templates += 1;
            } else {
                jobs += 1;
            }
            if let Some(stage) = val.get("stage").and_then(|v| v.as_str()) {
                used_stages.insert(stage.to_string());
            }
        }
    }

    out.push_str(&format!("Jobs     : {}\n", jobs));
    if templates > 0 {
        out.push_str(&format!(
            "Templates: {} (hidden .job definitions)\n",
            templates
        ));
    }

    // Global image
    if let Some(img) = doc.get("image").and_then(|v| {
        if v.is_string() {
            v.as_str().map(|s| s.to_string())
        } else {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        }
    }) {
        out.push_str(&format!("Image    : {}\n", img));
    }

    // Global variables count
    if let Some(vars) = doc.get("variables").and_then(|v| v.as_object()) {
        out.push_str(&format!("Variables: {} global variable(s)\n", vars.len()));
    }

    // Include count
    if let Some(inc) = doc.get("include") {
        let count = if inc.is_array() {
            inc.as_array().map(|a| a.len()).unwrap_or(0)
        } else {
            1
        };
        out.push_str(&format!("Includes : {} external file(s)\n", count));
    }

    // Workflow rules
    if doc.get("workflow").is_some() {
        out.push_str("Workflow : pipeline-level rules defined\n");
    }

    // Stage usage analysis
    out.push('\n');

    // List jobs by stage
    let effective_stages: Vec<String> = if declared_stages.is_empty() {
        vec!["build".into(), "test".into(), "deploy".into()]
    } else {
        declared_stages.clone()
    };

    for stage in &effective_stages {
        let stage_jobs: Vec<String> = obj
            .iter()
            .filter(|(key, val)| {
                is_job(key, val)
                    && !is_template(key)
                    && val.get("stage").and_then(|v| v.as_str()).unwrap_or("test") == stage.as_str()
            })
            .map(|(key, _)| key.clone())
            .collect();

        if !stage_jobs.is_empty() {
            out.push_str(&format!("{:>10} : {}\n", stage, stage_jobs.join(", ")));
        }
    }

    // Jobs without a declared stage
    let unstaged: Vec<String> = obj
        .iter()
        .filter(|(key, val)| is_job(key, val) && !is_template(key) && val.get("stage").is_none())
        .map(|(key, _)| key.clone())
        .collect();
    if !unstaged.is_empty() {
        out.push_str(&format!("  (no stage): {}\n", unstaged.join(", ")));
    }

    Ok(out)
}

fn action_jobs(doc: &Value, args: &Value) -> Result<String, String> {
    let obj = doc.as_object().ok_or("Expected a YAML mapping.")?;
    let stage_filter = args.get("stage").and_then(|v| v.as_str());
    let include_templates = args
        .get("templates")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut out = String::new();

    for (key, val) in obj {
        if !is_job(key, val) {
            continue;
        }
        if is_template(key) && !include_templates {
            continue;
        }

        let summary = summarize_job(key, val);

        if let Some(sf) = stage_filter {
            if summary.stage.as_deref().unwrap_or("test") != sf {
                continue;
            }
        }

        let stage_str = summary.stage.as_deref().unwrap_or("test");
        let tmpl_tag = if summary.is_template {
            " [TEMPLATE]"
        } else {
            ""
        };
        let allow_fail_tag = if summary.allow_failure {
            " [ALLOW FAILURE]"
        } else {
            ""
        };
        out.push_str(&format!(
            "━━ {}{}{}\n",
            summary.name, tmpl_tag, allow_fail_tag
        ));
        out.push_str(&format!("   Stage  : {}\n", stage_str));

        if let Some(img) = &summary.image {
            out.push_str(&format!("   Image  : {}\n", img));
        }
        if let Some(ext) = &summary.extends {
            out.push_str(&format!("   Extends: {}\n", ext));
        }
        if let Some(when) = &summary.when {
            out.push_str(&format!("   When   : {}\n", when));
        }
        if !summary.tags.is_empty() {
            out.push_str(&format!("   Tags   : {}\n", summary.tags.join(", ")));
        }
        if let Some(p) = summary.parallel {
            out.push_str(&format!("   Parallel: {} instances\n", p));
        }
        out.push_str(&format!("   Script : {} line(s)\n", summary.script_lines));

        // before_script / after_script
        let before = val
            .get("before_script")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        let after = val
            .get("after_script")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        if before > 0 {
            out.push_str(&format!("   Before : {} line(s)\n", before));
        }
        if after > 0 {
            out.push_str(&format!("   After  : {} line(s)\n", after));
        }

        if !summary.needs.is_empty() {
            out.push_str(&format!("   Needs  : {}\n", summary.needs.join(", ")));
        }
        if summary.rules_count > 0 {
            out.push_str(&format!("   Rules  : {} rule(s)\n", summary.rules_count));
        }

        // Artifacts
        if let Some(art) = val.get("artifacts") {
            let paths: Vec<String> = art
                .get("paths")
                .and_then(|p| p.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();
            if !paths.is_empty() {
                out.push_str(&format!("   Artifacts: {}\n", paths.join(", ")));
            }
        }

        // Variables
        if let Some(vars) = val.get("variables").and_then(|v| v.as_object()) {
            out.push_str(&format!("   Vars   : {} variable(s)\n", vars.len()));
        }

        out.push('\n');
    }

    if out.is_empty() {
        return Ok(if let Some(sf) = stage_filter {
            format!("No jobs in stage '{}'.", sf)
        } else {
            "No jobs found.".to_string()
        });
    }

    Ok(out)
}

fn action_stages(doc: &Value) -> Result<String, String> {
    let obj = doc.as_object().ok_or("Expected a YAML mapping.")?;

    let declared: Vec<String> = doc
        .get("stages")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    let effective: Vec<String> = if declared.is_empty() {
        vec!["build".into(), "test".into(), "deploy".into()]
    } else {
        declared.clone()
    };

    let mut out = String::from("Pipeline Stages\n");
    out.push_str(&format!("{}\n\n", "═".repeat(40)));

    if declared.is_empty() {
        out.push_str("(No 'stages' declared — using GitLab defaults: build, test, deploy)\n\n");
    }

    for (i, stage) in effective.iter().enumerate() {
        let jobs: Vec<String> = obj
            .iter()
            .filter(|(key, val)| {
                is_job(key, val) && !is_template(key) && {
                    let job_stage = val.get("stage").and_then(|v| v.as_str()).unwrap_or("test");
                    job_stage == stage.as_str()
                }
            })
            .map(|(key, _)| key.clone())
            .collect();

        out.push_str(&format!("  {}. {} ({} job(s))\n", i + 1, stage, jobs.len()));
        for job in &jobs {
            out.push_str(&format!("     • {}\n", job));
        }
    }

    // Jobs without a matching declared stage
    let mut orphan_stages: std::collections::HashSet<String> = Default::default();
    for (key, val) in obj {
        if is_job(key, val) && !is_template(key) {
            if let Some(stage) = val.get("stage").and_then(|v| v.as_str()) {
                if !effective.contains(&stage.to_string()) {
                    orphan_stages.insert(stage.to_string());
                }
            }
        }
    }

    if !orphan_stages.is_empty() {
        out.push_str("\n⚠️  Jobs reference undeclared stages:\n");
        for s in &orphan_stages {
            out.push_str(&format!("     • {}\n", s));
        }
    }

    Ok(out)
}

fn action_validate(doc: &Value) -> Result<String, String> {
    let obj = doc.as_object().ok_or("Expected a YAML mapping.")?;

    let mut issues: Vec<String> = vec![];

    let declared_stages: Vec<String> = doc
        .get("stages")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    let effective_stages: Vec<String> = if declared_stages.is_empty() {
        vec!["build".into(), "test".into(), "deploy".into()]
    } else {
        declared_stages.clone()
    };

    // Collect all real job names for needs validation
    let all_job_names: Vec<String> = obj
        .keys()
        .filter(|k| is_job(k, &obj[*k]) && !is_template(k))
        .cloned()
        .collect();

    let mut job_count = 0;

    for (key, val) in obj {
        if !is_job(key, val) {
            continue;
        }
        if is_template(key) {
            continue; // Template jobs are optional patterns — skip strict checks
        }

        job_count += 1;

        // Each job must have a script (or trigger for child pipelines)
        let has_script = val.get("script").is_some();
        let has_trigger = val.get("trigger").is_some();
        if !has_script && !has_trigger {
            issues.push(format!("Job '{}': missing 'script' (or 'trigger')", key));
        }

        // Stage must be declared
        if let Some(stage) = val.get("stage").and_then(|v| v.as_str()) {
            if !effective_stages.contains(&stage.to_string()) {
                issues.push(format!(
                    "Job '{}': references undeclared stage '{}'",
                    key, stage
                ));
            }
        }

        // Needs must reference real jobs
        if let Some(needs_arr) = val.get("needs").and_then(|v| v.as_array()) {
            for need in needs_arr {
                let need_name = if need.is_string() {
                    need.as_str().map(|s| s.to_string())
                } else {
                    need.get("job")
                        .and_then(|j| j.as_str())
                        .map(|s| s.to_string())
                };
                if let Some(name) = need_name {
                    if !all_job_names.contains(&name) {
                        issues.push(format!(
                            "Job '{}': 'needs' references unknown job '{}'",
                            key, name
                        ));
                    }
                }
            }
        }

        // Warn on image: latest
        if let Some(img) = val.get("image").and_then(|v| {
            if v.is_string() {
                v.as_str().map(|s| s.to_string())
            } else {
                v.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            }
        }) {
            if img.ends_with(":latest") || (!img.contains(':') && img.contains('/')) {
                // Only warn on explicit :latest
                if img.ends_with(":latest") {
                    issues.push(format!(
                        "Job '{}': using ':latest' image tag — not reproducible ({})",
                        key, img
                    ));
                }
            }
        }

        // Warn on empty rules array
        if let Some(rules) = val.get("rules").and_then(|v| v.as_array()) {
            if rules.is_empty() {
                issues.push(format!(
                    "Job '{}': 'rules' is empty — job will never run",
                    key
                ));
            }
        }

        // Warn on both 'only'/'except' and 'rules' (deprecated mix)
        let has_only = val.get("only").is_some() || val.get("except").is_some();
        let has_rules = val.get("rules").is_some();
        if has_only && has_rules {
            issues.push(format!(
                "Job '{}': mixing deprecated 'only'/'except' with 'rules' — use 'rules' only",
                key
            ));
        }

        // Warn on deprecated 'only'/'except'
        if has_only && !has_rules {
            issues.push(format!(
                "Job '{}': uses deprecated 'only'/'except' — prefer 'rules'",
                key
            ));
        }
    }

    if job_count == 0 {
        issues.push("No jobs defined in the pipeline.".into());
    }

    // Global checks
    if declared_stages.is_empty() && job_count > 0 {
        // Not an error, just informational — GitLab has defaults
    }

    // Check for duplicate stage names
    let mut seen_stages: std::collections::HashSet<&str> = Default::default();
    for stage in &declared_stages {
        if !seen_stages.insert(stage.as_str()) {
            issues.push(format!("Duplicate stage name: '{}'", stage));
        }
    }

    let mut out = String::from("GitLab CI Validation\n");
    out.push_str(&format!("{}\n\n", "═".repeat(40)));
    out.push_str(&format!("Jobs     : {}\n", job_count));
    out.push_str(&format!("Stages   : {}\n\n", effective_stages.join(", ")));

    if issues.is_empty() {
        out.push_str("✅ VALID — no issues found\n");
    } else {
        out.push_str(&format!("⚠️  {} issue(s) found:\n\n", issues.len()));
        for issue in &issues {
            out.push_str(&format!("  • {}\n", issue));
        }
    }

    Ok(out)
}
