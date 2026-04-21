use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_WORKFLOW_TIMEOUT_MS: u64 = 600_000;
const DEFAULT_VERIFY_TIMEOUT_MS: u64 = 1_800_000;
const DEFAULT_WEBSITE_BOOT_TIMEOUT_MS: u64 = 120_000;
const DEFAULT_WEBSITE_REQUEST_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_WEBSITE_VALIDATE_TIMEOUT_MS: u64 = 30_000;
const WEBSITE_LOG_TAIL_BYTES: u64 = 4_096;

pub async fn run_workspace_workflow(args: &Value) -> Result<String, String> {
    let root = require_project_workspace_root()?;
    let workflow = required_string(args, "workflow")?;

    match workflow {
        "website_start" => start_website_server(args, &root).await,
        "website_probe" => probe_website_server(args, &root).await,
        "website_validate" => validate_website_server(args, &root).await,
        "website_status" => website_server_status(args, &root).await,
        "website_stop" => stop_website_server(args, &root).await,
        _ => {
            let invocation = WorkspaceInvocation::from_args(args, &root)?;
            let output = crate::tools::shell::execute_command_in_dir(
                &invocation.command,
                &root,
                invocation.timeout_ms,
                false,
            )
            .await?;

            Ok(format!(
                "Workspace workflow: {}\nWorkspace root: {}\nCommand: {}\n\n{}",
                invocation.workflow_label,
                root.display(),
                invocation.command,
                output.trim()
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceInvocation {
    workflow_label: String,
    command: String,
    timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebsiteLaunchPlan {
    mode: String,
    script: String,
    command: String,
    url: String,
    framework_hint: String,
    boot_timeout_ms: u64,
    request_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct WebsiteServerState {
    label: String,
    mode: String,
    script: String,
    command: String,
    url: String,
    framework_hint: String,
    pid: u32,
    log_path: String,
    workspace_root: String,
    started_at_epoch_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebsiteProbeSummary {
    url: String,
    status: u16,
    content_type: Option<String>,
    title: Option<String>,
    body_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebsiteResponseSnapshot {
    summary: WebsiteProbeSummary,
    body: String,
}

impl WorkspaceInvocation {
    fn from_args(args: &Value, root: &Path) -> Result<Self, String> {
        let workflow = required_string(args, "workflow")?;
        let timeout_ms = args
            .get("timeout_ms")
            .and_then(|value| value.as_u64())
            .unwrap_or(default_timeout_ms(workflow));

        let command = match workflow {
            "build" => default_command_for_action(root, "build")?,
            "test" => default_command_for_action(root, "test")?,
            "lint" => default_command_for_action(root, "lint")?,
            "fix" => default_command_for_action(root, "fix")?,
            "package_script" => build_package_script_command(root, required_string(args, "name")?)?,
            "task" => format!("task {}", required_string(args, "name")?),
            "just" => format!("just {}", required_string(args, "name")?),
            "make" => format!("make {}", required_string(args, "name")?),
            "script_path" => build_script_path_command(root, required_string(args, "path")?)?,
            "command" => required_string(args, "command")?.to_string(),
            other => {
                return Err(format!(
                    "Unknown workflow '{}'. Use one of: build, test, lint, fix, package_script, task, just, make, script_path, command, website_start, website_probe, website_validate, website_status, website_stop.",
                    other
                ))
            }
        };

        Ok(Self {
            workflow_label: workflow.to_string(),
            command,
            timeout_ms,
        })
    }
}

fn require_project_workspace_root() -> Result<PathBuf, String> {
    Ok(crate::tools::file_ops::workspace_root())
}

fn required_string<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Missing required argument: '{}'", key))
}

fn optional_string<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn optional_string_vec(args: &Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(|value| value.as_array())
        .into_iter()
        .flat_map(|items| items.iter())
        .filter_map(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn default_timeout_ms(workflow: &str) -> u64 {
    match workflow {
        "build" | "test" | "lint" | "fix" => DEFAULT_VERIFY_TIMEOUT_MS,
        "website_start" => DEFAULT_WEBSITE_BOOT_TIMEOUT_MS,
        "website_probe" | "website_status" => DEFAULT_WEBSITE_REQUEST_TIMEOUT_MS,
        "website_validate" => DEFAULT_WEBSITE_VALIDATE_TIMEOUT_MS,
        _ => DEFAULT_WORKFLOW_TIMEOUT_MS,
    }
}

fn default_command_for_action(root: &Path, action: &str) -> Result<String, String> {
    let profile = crate::agent::workspace_profile::load_workspace_profile(root)
        .unwrap_or_else(|| crate::agent::workspace_profile::detect_workspace_profile(root));

    match action {
        "build" => profile
            .build_hint
            .ok_or_else(|| missing_workspace_command_message(action, root)),
        "test" => profile
            .test_hint
            .ok_or_else(|| missing_workspace_command_message(action, root)),
        "lint" => detect_lint_command(root),
        "fix" => detect_fix_command(root),
        other => Err(format!("Unsupported workspace action '{}'.", other)),
    }
}

fn missing_workspace_command_message(action: &str, root: &Path) -> String {
    format!(
        "Hematite could not infer a `{}` command for the locked workspace at {}. Add a workspace verify profile in `.hematite/settings.json`, or ask for an explicit command/script instead.",
        action,
        root.display()
    )
}

fn detect_lint_command(root: &Path) -> Result<String, String> {
    if root.join("Cargo.toml").exists() {
        Ok("cargo clippy --all-targets --all-features -- -D warnings".to_string())
    } else if root.join("package.json").exists() {
        Ok(format!(
            "{} run lint --if-present",
            detect_node_package_manager(root)
        ))
    } else if root.join("go.mod").exists() {
        Err(missing_workspace_command_message("lint", root))
    } else {
        Err(missing_workspace_command_message("lint", root))
    }
}

fn detect_fix_command(root: &Path) -> Result<String, String> {
    if root.join("Cargo.toml").exists() {
        Ok("cargo fmt".to_string())
    } else if root.join("package.json").exists() {
        Ok(format!(
            "{} run fix --if-present",
            detect_node_package_manager(root)
        ))
    } else {
        Err(missing_workspace_command_message("fix", root))
    }
}

fn build_package_script_command(root: &Path, name: &str) -> Result<String, String> {
    let package = read_package_json(root)?;
    let has_script = package
        .get("scripts")
        .and_then(|value| value.get(name))
        .is_some();
    if !has_script {
        return Err(format!(
            "package.json does not define a script named `{}` in {}.",
            name,
            root.display()
        ));
    }

    let package_manager = detect_node_package_manager(root);
    let command = match package_manager.as_str() {
        "yarn" => format!("yarn {}", name),
        "bun" => format!("bun run {}", name),
        manager => format!("{} run {}", manager, name),
    };
    Ok(command)
}

fn read_package_json(root: &Path) -> Result<Value, String> {
    let package_json = root.join("package.json");
    if !package_json.exists() {
        return Err(format!(
            "This workflow requires package.json in the locked workspace root ({}).",
            root.display()
        ));
    }

    let content = fs::read_to_string(&package_json)
        .map_err(|e| format!("Failed to read {}: {}", package_json.display(), e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", package_json.display(), e))
}

fn package_scripts(package: &Value) -> Map<String, Value> {
    package
        .get("scripts")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default()
}

fn build_script_path_command(root: &Path, relative_path: &str) -> Result<String, String> {
    let candidate = root.join(relative_path);
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("Failed to resolve workspace root {}: {}", root.display(), e))?;
    let canonical_path = candidate.canonicalize().map_err(|e| {
        format!(
            "Could not resolve script path `{}` from workspace root {}: {}",
            relative_path,
            root.display(),
            e
        )
    })?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(format!(
            "Script path `{}` resolves outside the locked workspace root {}.",
            relative_path,
            root.display()
        ));
    }

    let display_path = normalize_relative_path(&canonical_path, &canonical_root)?;
    let lower = display_path.to_ascii_lowercase();
    if lower.ends_with(".ps1") {
        Ok(format!(
            "pwsh -ExecutionPolicy Bypass -File {}",
            quote_command_arg(&display_path)
        ))
    } else if lower.ends_with(".cmd") || lower.ends_with(".bat") {
        Ok(format!("cmd /C {}", quote_command_arg(&display_path)))
    } else if lower.ends_with(".sh") {
        Ok(format!("bash {}", quote_command_arg(&display_path)))
    } else if lower.ends_with(".py") {
        Ok(format!("python {}", quote_command_arg(&display_path)))
    } else if lower.ends_with(".js") || lower.ends_with(".cjs") || lower.ends_with(".mjs") {
        Ok(format!("node {}", quote_command_arg(&display_path)))
    } else {
        Ok(display_path)
    }
}

fn normalize_relative_path(path: &Path, root: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|e| format!("Failed to normalize script path: {}", e))?;
    Ok(format!(
        ".{}{}",
        std::path::MAIN_SEPARATOR,
        relative.display()
    ))
}

fn quote_command_arg(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn detect_node_package_manager(root: &Path) -> String {
    if root.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if root.join("yarn.lock").exists() {
        "yarn".to_string()
    } else if root.join("bun.lockb").exists() || root.join("bun.lock").exists() {
        "bun".to_string()
    } else {
        "npm".to_string()
    }
}

async fn start_website_server(args: &Value, root: &Path) -> Result<String, String> {
    let label = optional_string(args, "label").unwrap_or("default");
    let state_path = website_state_path(root, label);
    if let Some(existing) = load_website_server_state(&state_path)? {
        if is_process_alive(existing.pid).await {
            return Err(format!(
                "A website server labeled `{}` is already running.\nURL: {}\nPID: {}\nLog: {}\nUse workflow=website_status or workflow=website_stop first.",
                existing.label, existing.url, existing.pid, existing.log_path
            ));
        }
        let _ = fs::remove_file(&state_path);
    }

    let plan = detect_website_launch_plan(args, root)?;
    let runtime_dir = website_runtime_dir(root);
    fs::create_dir_all(&runtime_dir)
        .map_err(|e| format!("Failed to create {}: {}", runtime_dir.display(), e))?;
    let log_path = website_log_path(root, label);
    let stdout_log = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&log_path)
        .map_err(|e| format!("Failed to create {}: {}", log_path.display(), e))?;
    let stderr_log = stdout_log
        .try_clone()
        .map_err(|e| format!("Failed to clone log handle {}: {}", log_path.display(), e))?;

    let mut command = build_shell_command(&plan.command).await;
    command
        .current_dir(root)
        .stdout(Stdio::from(stdout_log))
        .stderr(Stdio::from(stderr_log));

    let sandbox_root = crate::tools::file_ops::hematite_dir().join("sandbox");
    let _ = fs::create_dir_all(&sandbox_root);
    command.env("HOME", &sandbox_root);
    command.env("TMPDIR", &sandbox_root);
    command.env("CI", "1");
    command.env("BROWSER", "none");

    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to start website server: {}", e))?;
    let pid = child
        .id()
        .ok_or_else(|| "Website server started without a visible process id.".to_string())?;

    let started_at_epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Clock error: {}", e))?
        .as_millis() as u64;

    let state = WebsiteServerState {
        label: label.to_string(),
        mode: plan.mode.clone(),
        script: plan.script.clone(),
        command: plan.command.clone(),
        url: plan.url.clone(),
        framework_hint: plan.framework_hint.clone(),
        pid,
        log_path: log_path.display().to_string(),
        workspace_root: root.display().to_string(),
        started_at_epoch_ms,
    };

    let probe = wait_for_website_readiness(
        &mut child,
        &plan.url,
        plan.boot_timeout_ms,
        plan.request_timeout_ms,
        &log_path,
    )
    .await
    .map_err(|message| {
        let _ = fs::remove_file(&state_path);
        message
    })?;

    save_website_server_state(&state_path, &state)?;

    Ok(format!(
        "Workspace workflow: website_start\nWorkspace root: {}\nMode: {}\nLabel: {}\nScript: {}\nCommand: {}\nFramework hint: {}\nURL: {}\nPID: {}\nLog: {}\n\nReady: HTTP {}{}\n{}",
        root.display(),
        state.mode,
        state.label,
        state.script,
        state.command,
        state.framework_hint,
        state.url,
        state.pid,
        state.log_path,
        probe.status,
        probe
            .title
            .as_ref()
            .map(|title| format!(" ({title})"))
            .unwrap_or_default(),
        format_probe_details(&probe)
    ))
}

fn detect_website_launch_plan(args: &Value, root: &Path) -> Result<WebsiteLaunchPlan, String> {
    let package = read_package_json(root)?;
    let scripts = package_scripts(&package);
    let runtime_contract = load_runtime_contract(root);
    let mode = optional_string(args, "mode")
        .unwrap_or("dev")
        .to_ascii_lowercase();
    let script = if let Some(explicit) = optional_string(args, "script") {
        explicit.to_string()
    } else {
        detect_website_script_name(&scripts, &mode)?
    };
    if !scripts.contains_key(&script) {
        return Err(format!(
            "package.json does not define a website script named `{}` in {}.",
            script,
            root.display()
        ));
    }

    let framework_hint = infer_website_framework(&package);
    let port = args
        .get("port")
        .and_then(|value| value.as_u64())
        .and_then(|value| u16::try_from(value).ok())
        .or_else(|| infer_website_default_port(&package, &mode));
    let host = optional_string(args, "host").unwrap_or("127.0.0.1");
    let url = if let Some(explicit_url) = optional_string(args, "url") {
        normalize_http_url(explicit_url)
    } else if let Some(url_hint) = runtime_contract
        .as_ref()
        .and_then(|contract| contract.local_url_hint.clone())
    {
        url_hint
    } else {
        let inferred_port = port.unwrap_or(if mode == "preview" { 4173 } else { 3000 });
        format!("http://{}:{}/", host, inferred_port)
    };

    Ok(WebsiteLaunchPlan {
        mode,
        script: script.clone(),
        command: build_package_script_command(root, &script)?,
        url,
        framework_hint,
        boot_timeout_ms: args
            .get("timeout_ms")
            .and_then(|value| value.as_u64())
            .unwrap_or(DEFAULT_WEBSITE_BOOT_TIMEOUT_MS),
        request_timeout_ms: args
            .get("request_timeout_ms")
            .and_then(|value| value.as_u64())
            .unwrap_or(DEFAULT_WEBSITE_REQUEST_TIMEOUT_MS),
    })
}

fn detect_website_script_name(scripts: &Map<String, Value>, mode: &str) -> Result<String, String> {
    let candidates = match mode {
        "dev" => ["dev", "start", "serve"],
        "preview" => ["preview", "serve", "start"],
        "start" => ["start", "serve", "dev"],
        other => {
            return Err(format!(
                "Unknown website mode `{}`. Use one of: dev, preview, start.",
                other
            ))
        }
    };

    candidates
        .iter()
        .find(|candidate| scripts.contains_key(**candidate))
        .map(|candidate| candidate.to_string())
        .ok_or_else(|| {
            format!(
                "Could not infer a website {} script from package.json. Define one of [{}], or pass `script` explicitly.",
                mode,
                candidates.join(", ")
            )
        })
}

fn infer_website_framework(package: &Value) -> String {
    let deps = dependency_names(package);
    let script_text = package_scripts(package)
        .into_values()
        .filter_map(|value| value.as_str().map(|text| text.to_ascii_lowercase()))
        .collect::<Vec<_>>()
        .join("\n");

    if deps.contains("next") || script_text.contains("next ") {
        "next".to_string()
    } else if deps.contains("vite") || script_text.contains("vite") {
        "vite".to_string()
    } else if deps.contains("astro") || script_text.contains("astro ") {
        "astro".to_string()
    } else if deps.contains("@angular/core") || script_text.contains("ng serve") {
        "angular".to_string()
    } else if deps.contains("gatsby") || script_text.contains("gatsby ") {
        "gatsby".to_string()
    } else if deps.contains("react-scripts") || script_text.contains("react-scripts") {
        "react-scripts".to_string()
    } else if deps.contains("@sveltejs/kit") || script_text.contains("svelte-kit") {
        "sveltekit".to_string()
    } else if deps.contains("nuxt") || script_text.contains("nuxt ") {
        "nuxt".to_string()
    } else {
        "generic-node-site".to_string()
    }
}

fn infer_website_default_port(package: &Value, mode: &str) -> Option<u16> {
    match infer_website_framework(package).as_str() {
        "vite" | "sveltekit" => Some(if mode == "preview" { 4173 } else { 5173 }),
        "astro" => Some(4321),
        "gatsby" => Some(8000),
        "angular" => Some(4200),
        "next" | "react-scripts" | "nuxt" => Some(3000),
        _ => None,
    }
}

fn dependency_names(package: &Value) -> std::collections::BTreeSet<String> {
    let mut deps = std::collections::BTreeSet::new();
    for field in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(map) = package.get(field).and_then(|value| value.as_object()) {
            for name in map.keys() {
                deps.insert(name.to_ascii_lowercase());
            }
        }
    }
    deps
}

async fn wait_for_website_readiness(
    child: &mut tokio::process::Child,
    url: &str,
    boot_timeout_ms: u64,
    request_timeout_ms: u64,
    log_path: &Path,
) -> Result<WebsiteProbeSummary, String> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(boot_timeout_ms);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(request_timeout_ms))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("Failed to build readiness probe client: {}", e))?;

    loop {
        let probe_error = match probe_website_once(&client, url).await {
            Ok(summary) => return Ok(summary),
            Err(err) => err,
        };

        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!(
                    "Website server exited before it became ready (status: {}).\nLast probe error: {}\n{}",
                    status,
                    probe_error,
                    format_log_tail_for_path("Recent log tail", Some(log_path))
                ));
            }
            Ok(None) => {}
            Err(err) => {
                return Err(format!("Failed to inspect website server status: {}", err));
            }
        }

        if tokio::time::Instant::now() >= deadline {
            let _ = child.kill().await;
            return Err(format!(
                "Website server did not become ready within {} ms.\nLast probe error: {}\n{}",
                boot_timeout_ms,
                probe_error,
                format_log_tail_for_path("Recent log tail", Some(log_path))
            ));
        }

        tokio::time::sleep(Duration::from_millis(750)).await;
    }
}

async fn probe_website_server(args: &Value, root: &Path) -> Result<String, String> {
    let label = optional_string(args, "label").unwrap_or("default");
    let state = load_website_server_state(&website_state_path(root, label))?;
    let (url, log_path) = if let Some(state) = state {
        (state.url, Some(state.log_path))
    } else if let Some(url) = optional_string(args, "url") {
        (normalize_http_url(url), None)
    } else {
        return Err(format!(
            "No tracked website server labeled `{}`. Pass `url` to probe an arbitrary local site, or start one with workflow=website_start.",
            label
        ));
    };

    let request_timeout_ms = args
        .get("timeout_ms")
        .and_then(|value| value.as_u64())
        .unwrap_or(DEFAULT_WEBSITE_REQUEST_TIMEOUT_MS);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(request_timeout_ms))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("Failed to build probe client: {}", e))?;
    let probe = probe_website_once(&client, &url).await.map_err(|e| {
        if let Some(path) = log_path.as_deref() {
            format!("{}\n{}", e, format_log_tail("Recent log tail", Some(path)))
        } else {
            e
        }
    })?;

    Ok(format!(
        "Workspace workflow: website_probe\nWorkspace root: {}\nURL: {}\n\nHTTP {}{}\n{}",
        root.display(),
        probe.url,
        probe.status,
        probe
            .title
            .as_ref()
            .map(|title| format!(" ({title})"))
            .unwrap_or_default(),
        format_probe_details(&probe)
    ))
}

async fn validate_website_server(args: &Value, root: &Path) -> Result<String, String> {
    let label = optional_string(args, "label").unwrap_or("default");
    let (base_url, log_path) = resolve_website_target(args, root, label)?;
    let routes = default_website_routes(args, root);
    let asset_limit = args
        .get("asset_limit")
        .and_then(|value| value.as_u64())
        .unwrap_or(8)
        .min(24) as usize;
    let request_timeout_ms = args
        .get("timeout_ms")
        .and_then(|value| value.as_u64())
        .unwrap_or(DEFAULT_WEBSITE_VALIDATE_TIMEOUT_MS);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(request_timeout_ms))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("Failed to build validation client: {}", e))?;

    let mut route_lines = Vec::new();
    let mut asset_lines = Vec::new();
    let mut issues = Vec::new();
    let mut assets = std::collections::BTreeSet::new();

    for route in &routes {
        let route_url = resolve_website_url(&base_url, route)?;
        match fetch_website_snapshot(&client, &route_url).await {
            Ok(snapshot) => {
                let summary = &snapshot.summary;
                route_lines.push(format!(
                    "- {} -> HTTP {}{}",
                    route,
                    summary.status,
                    summary
                        .title
                        .as_ref()
                        .map(|title| format!(" ({title})"))
                        .unwrap_or_default()
                ));
                let content_type = summary.content_type.as_deref().unwrap_or_default();
                if content_type.contains("text/html") {
                    if summary.title.is_none() {
                        issues.push(format!("Route {} returned HTML without a <title>.", route));
                    }
                    for asset in extract_local_asset_urls(&route_url, &snapshot.body)
                        .into_iter()
                        .take(asset_limit)
                    {
                        assets.insert(asset);
                    }
                }
            }
            Err(err) => {
                issues.push(format!("Route {} failed validation: {}", route, err));
            }
        }
    }

    for asset_url in assets.iter().take(asset_limit) {
        match probe_website_once(&client, asset_url).await {
            Ok(summary) => asset_lines.push(format!(
                "- {} -> HTTP {} ({})",
                asset_url,
                summary.status,
                summary
                    .content_type
                    .as_deref()
                    .unwrap_or("unknown content type")
            )),
            Err(err) => issues.push(format!("Asset {} failed validation: {}", asset_url, err)),
        }
    }

    let result = if issues.is_empty() { "PASS" } else { "FAIL" };
    let mut out = format!(
        "Workspace workflow: website_validate\nWorkspace root: {}\nBase URL: {}\nRoutes checked: {}\nAssets checked: {}\nResult: {}",
        root.display(),
        base_url,
        routes.len(),
        asset_lines.len(),
        result
    );
    if !route_lines.is_empty() {
        out.push_str("\n\nRoutes\n");
        out.push_str(&route_lines.join("\n"));
    }
    if !asset_lines.is_empty() {
        out.push_str("\n\nAssets\n");
        out.push_str(&asset_lines.join("\n"));
    }
    if !issues.is_empty() {
        out.push_str("\n\nIssues\n");
        out.push_str(
            &issues
                .into_iter()
                .map(|issue| format!("- {}", issue))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    if let Some(path) = log_path.as_deref() {
        out.push_str("\n\n");
        out.push_str(&format_log_tail("Recent log tail", Some(path)));
    }
    Ok(out)
}

fn resolve_website_target(
    args: &Value,
    root: &Path,
    label: &str,
) -> Result<(String, Option<String>), String> {
    let state = load_website_server_state(&website_state_path(root, label))?;
    if let Some(state) = state {
        return Ok((state.url, Some(state.log_path)));
    }
    if let Some(url) = optional_string(args, "url") {
        return Ok((normalize_http_url(url), None));
    }
    if let Some(url_hint) = load_runtime_contract(root).and_then(|contract| contract.local_url_hint)
    {
        return Ok((url_hint, None));
    }
    Err(format!(
        "No tracked website server labeled `{}` and no explicit url. Start the site with workflow=website_start or pass `url`.",
        label
    ))
}

fn default_website_routes(args: &Value, root: &Path) -> Vec<String> {
    let mut routes = optional_string_vec(args, "routes");
    if !routes.is_empty() {
        return normalize_route_hints(routes);
    }
    if let Some(contract) = load_runtime_contract(root) {
        routes = contract.route_hints;
    }
    if routes.is_empty() {
        routes.push("/".to_string());
    }
    normalize_route_hints(routes)
}

fn normalize_route_hints(routes: Vec<String>) -> Vec<String> {
    let mut normalized = std::collections::BTreeSet::new();
    for route in routes {
        let trimmed = route.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            normalized.insert(trimmed.to_string());
        } else if trimmed.starts_with('/') {
            normalized.insert(trimmed.to_string());
        } else {
            normalized.insert(format!("/{}", trimmed));
        }
    }
    if normalized.is_empty() {
        normalized.insert("/".to_string());
    }
    normalized.into_iter().collect()
}

fn resolve_website_url(base_url: &str, route: &str) -> Result<String, String> {
    if route.starts_with("http://") || route.starts_with("https://") {
        return Ok(route.to_string());
    }
    let base = reqwest::Url::parse(base_url)
        .map_err(|e| format!("Invalid base URL {}: {}", base_url, e))?;
    base.join(route).map(|url| url.to_string()).map_err(|e| {
        format!(
            "Failed to resolve route {} against {}: {}",
            route, base_url, e
        )
    })
}

async fn website_server_status(args: &Value, root: &Path) -> Result<String, String> {
    let label = optional_string(args, "label").unwrap_or("default");
    let state_path = website_state_path(root, label);
    let Some(state) = load_website_server_state(&state_path)? else {
        return Err(format!(
            "No tracked website server labeled `{}`. Start one with workflow=website_start.",
            label
        ));
    };

    let alive = is_process_alive(state.pid).await;
    let request_timeout_ms = args
        .get("timeout_ms")
        .and_then(|value| value.as_u64())
        .unwrap_or(DEFAULT_WEBSITE_REQUEST_TIMEOUT_MS);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(request_timeout_ms))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("Failed to build status probe client: {}", e))?;
    let probe = probe_website_once(&client, &state.url).await.ok();

    let mut out = format!(
        "Workspace workflow: website_status\nWorkspace root: {}\nLabel: {}\nMode: {}\nScript: {}\nCommand: {}\nFramework hint: {}\nURL: {}\nPID: {}\nAlive: {}\nLog: {}",
        root.display(),
        state.label,
        state.mode,
        state.script,
        state.command,
        state.framework_hint,
        state.url,
        state.pid,
        if alive { "yes" } else { "no" },
        state.log_path
    );
    if let Some(probe) = probe {
        out.push_str(&format!(
            "\n\nHTTP {}{}\n{}",
            probe.status,
            probe
                .title
                .as_ref()
                .map(|title| format!(" ({title})"))
                .unwrap_or_default(),
            format_probe_details(&probe)
        ));
    } else {
        out.push_str("\n\nHTTP probe: unavailable");
    }
    out.push_str("\n");
    out.push_str(&format_log_tail("Recent log tail", Some(&state.log_path)));
    Ok(out)
}

async fn stop_website_server(args: &Value, root: &Path) -> Result<String, String> {
    let label = optional_string(args, "label").unwrap_or("default");
    let state_path = website_state_path(root, label);
    let Some(state) = load_website_server_state(&state_path)? else {
        return Err(format!(
            "No tracked website server labeled `{}`. Nothing to stop.",
            label
        ));
    };

    let was_alive = is_process_alive(state.pid).await;
    if was_alive {
        kill_process(state.pid).await?;
    }
    let _ = fs::remove_file(&state_path);

    Ok(format!(
        "Workspace workflow: website_stop\nWorkspace root: {}\nLabel: {}\nPID: {}\nWas alive: {}\nURL: {}\nLog: {}\n\n{}",
        root.display(),
        state.label,
        state.pid,
        if was_alive { "yes" } else { "no" },
        state.url,
        state.log_path,
        format_log_tail("Recent log tail", Some(&state.log_path))
    ))
}

async fn probe_website_once(
    client: &reqwest::Client,
    url: &str,
) -> Result<WebsiteProbeSummary, String> {
    Ok(fetch_website_snapshot(client, url).await?.summary)
}

async fn fetch_website_snapshot(
    client: &reqwest::Client,
    url: &str,
) -> Result<WebsiteResponseSnapshot, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP probe failed for {}: {}", url, e))?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body from {}: {}", url, e))?;
    if !status.is_success() {
        return Err(format!(
            "HTTP probe returned {} for {}.",
            status.as_u16(),
            url
        ));
    }

    Ok(WebsiteResponseSnapshot {
        summary: WebsiteProbeSummary {
            url: url.to_string(),
            status: status.as_u16(),
            content_type,
            title: extract_html_title(&body),
            body_preview: html_preview_text(&body),
        },
        body,
    })
}

fn extract_html_title(body: &str) -> Option<String> {
    let re = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").ok()?;
    re.captures(body)
        .and_then(|captures| captures.get(1).map(|value| value.as_str()))
        .map(compact_whitespace)
        .filter(|title| !title.is_empty())
}

fn html_preview_text(body: &str) -> String {
    let strip_re = Regex::new(r"(?is)<script[^>]*>.*?</script>|<style[^>]*>.*?</style>|<[^>]+>")
        .expect("valid strip regex");
    let stripped = strip_re.replace_all(body, " ");
    let compact = compact_whitespace(&stripped);
    compact.chars().take(240).collect()
}

fn compact_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn format_probe_details(probe: &WebsiteProbeSummary) -> String {
    let mut lines = Vec::new();
    if let Some(content_type) = probe.content_type.as_deref() {
        lines.push(format!("Content-Type: {}", content_type));
    }
    if let Some(title) = probe.title.as_deref() {
        lines.push(format!("Title: {}", title));
    }
    if !probe.body_preview.is_empty() {
        lines.push(format!("Body preview: {}", probe.body_preview));
    }
    if lines.is_empty() {
        "(no probe details)".to_string()
    } else {
        lines.join("\n")
    }
}

fn normalize_http_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{}", trimmed)
    }
}

fn extract_local_asset_urls(page_url: &str, body: &str) -> Vec<String> {
    let Ok(page) = reqwest::Url::parse(page_url) else {
        return Vec::new();
    };
    let regex = Regex::new(r#"(?is)(?:src|href)=["']([^"'#]+)["']"#).expect("valid asset regex");
    let mut assets = std::collections::BTreeSet::new();
    for captures in regex.captures_iter(body) {
        let Some(raw) = captures.get(1).map(|value| value.as_str().trim()) else {
            continue;
        };
        let lower = raw.to_ascii_lowercase();
        if lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("data:")
            || lower.starts_with("mailto:")
            || lower.starts_with("tel:")
            || lower.starts_with("javascript:")
        {
            continue;
        }
        if !looks_like_static_asset(raw) {
            continue;
        }
        if let Ok(joined) = page.join(raw) {
            assets.insert(joined.to_string());
        }
    }
    assets.into_iter().collect()
}

fn looks_like_static_asset(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        ".css",
        ".js",
        ".mjs",
        ".ico",
        ".png",
        ".jpg",
        ".jpeg",
        ".svg",
        ".webp",
        ".gif",
        ".woff",
        ".woff2",
        ".map",
        ".json",
        ".webmanifest",
    ]
    .iter()
    .any(|suffix| lower.contains(suffix))
}

fn load_runtime_contract(root: &Path) -> Option<crate::agent::workspace_profile::RuntimeContract> {
    crate::agent::workspace_profile::load_workspace_profile(root)
        .unwrap_or_else(|| crate::agent::workspace_profile::detect_workspace_profile(root))
        .runtime_contract
}

fn website_runtime_dir(root: &Path) -> PathBuf {
    if crate::tools::file_ops::is_os_shortcut_directory(root) {
        crate::tools::file_ops::hematite_dir().join("website-runtime")
    } else {
        root.join(".hematite").join("website-runtime")
    }
}

fn website_state_path(root: &Path, label: &str) -> PathBuf {
    website_runtime_dir(root).join(format!("{}.json", slugify_label(label)))
}

fn website_log_path(root: &Path, label: &str) -> PathBuf {
    website_runtime_dir(root).join(format!("{}.log", slugify_label(label)))
}

fn slugify_label(input: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in input.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            slug.push(lower);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "default".to_string()
    } else {
        trimmed.to_string()
    }
}

fn save_website_server_state(path: &Path, state: &WebsiteServerState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }
    let payload = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to encode website state: {}", e))?;
    fs::write(path, payload).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

fn load_website_server_state(path: &Path) -> Result<Option<WebsiteServerState>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let state = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;
    Ok(Some(state))
}

fn format_log_tail(label: &str, path: Option<&str>) -> String {
    match path {
        Some(path) => match read_log_tail(Path::new(path)) {
            Ok(tail) if tail.is_empty() => format!("{}: (empty)", label),
            Ok(tail) => format!("{}:\n{}", label, tail),
            Err(err) => format!("{}: unavailable ({})", label, err),
        },
        None => format!("{}: unavailable", label),
    }
}

fn format_log_tail_for_path(label: &str, path: Option<&Path>) -> String {
    match path {
        Some(path) => match read_log_tail(path) {
            Ok(tail) if tail.is_empty() => format!("{}: (empty)", label),
            Ok(tail) => format!("{}:\n{}", label, tail),
            Err(err) => format!("{}: unavailable ({})", label, err),
        },
        None => format!("{}: unavailable", label),
    }
}

fn read_log_tail(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|e| format!("failed to open {}: {}", path.display(), e))?;
    let len = file
        .metadata()
        .map_err(|e| format!("failed to inspect {}: {}", path.display(), e))?
        .len();
    let start = len.saturating_sub(WEBSITE_LOG_TAIL_BYTES);
    file.seek(SeekFrom::Start(start))
        .map_err(|e| format!("failed to seek {}: {}", path.display(), e))?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    Ok(buffer.trim().to_string())
}

async fn build_shell_command(command: &str) -> tokio::process::Command {
    #[cfg(target_os = "windows")]
    {
        let normalized = command
            .replace("/dev/null", "$null")
            .replace("1>/dev/null", "2>$null")
            .replace("2>/dev/null", "2>$null");

        if which("pwsh").await {
            let mut cmd = tokio::process::Command::new("pwsh");
            cmd.args(["-NoProfile", "-NonInteractive", "-Command", &normalized]);
            cmd
        } else {
            let mut cmd = tokio::process::Command::new("powershell");
            cmd.args(["-NoProfile", "-NonInteractive", "-Command", &normalized]);
            cmd
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = tokio::process::Command::new("sh");
        cmd.args(["-c", command]);
        cmd
    }
}

async fn which(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    let check = format!("{}.exe", name);
    #[cfg(not(target_os = "windows"))]
    let check = name;

    tokio::process::Command::new("where")
        .arg(check)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}

async fn is_process_alive(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        tokio::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await
            .ok()
            .map(|output| {
                let text = String::from_utf8_lossy(&output.stdout);
                text.lines().any(|line| {
                    line.split_whitespace()
                        .any(|token| token == pid.to_string())
                })
            })
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        tokio::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

async fn kill_process(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let output = tokio::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output()
            .await
            .map_err(|e| format!("Failed to stop PID {}: {}", pid, e))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Failed to stop PID {}: {}",
                pid,
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let status = tokio::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .await
            .map_err(|e| format!("Failed to stop PID {}: {}", pid, e))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("Failed to stop PID {}.", pid))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_package(root: &Path, json: &str) {
        fs::write(root.join("package.json"), json).unwrap();
    }

    #[test]
    fn package_script_uses_detected_package_manager() {
        let package_root = std::env::temp_dir().join(format!(
            "hematite-workspace-workflow-node-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&package_root).unwrap();
        std::fs::write(
            package_root.join("package.json"),
            r#"{ "scripts": { "dev": "vite" } }"#,
        )
        .unwrap();
        std::fs::write(package_root.join("pnpm-lock.yaml"), "").unwrap();

        let command = build_package_script_command(&package_root, "dev").unwrap();
        assert_eq!(command, "pnpm run dev");

        let _ = std::fs::remove_file(package_root.join("package.json"));
        let _ = std::fs::remove_file(package_root.join("pnpm-lock.yaml"));
        let _ = std::fs::remove_dir(package_root);
    }

    #[test]
    fn script_path_stays_inside_workspace_root() {
        let script_dir = std::env::temp_dir().join(format!(
            "hematite-workspace-workflow-scripts-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(script_dir.join("scripts")).unwrap();
        std::fs::write(script_dir.join("scripts").join("dev.ps1"), "Write-Host hi").unwrap();

        let command = build_script_path_command(&script_dir, "scripts/dev.ps1").unwrap();
        assert!(command.contains("pwsh -ExecutionPolicy Bypass -File"));

        let _ = std::fs::remove_file(script_dir.join("scripts").join("dev.ps1"));
        let _ = std::fs::remove_dir(script_dir.join("scripts"));
        let _ = std::fs::remove_dir(script_dir);
    }

    #[test]
    fn detect_website_launch_plan_prefers_dev_script_and_vite_port() {
        let dir = tempfile::tempdir().unwrap();
        write_package(
            dir.path(),
            r#"{
                "scripts": { "dev": "vite", "preview": "vite preview" },
                "devDependencies": { "vite": "^5.0.0" }
            }"#,
        );
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

        let plan = detect_website_launch_plan(&serde_json::json!({}), dir.path()).unwrap();
        assert_eq!(plan.script, "dev");
        assert_eq!(plan.command, "pnpm run dev");
        assert_eq!(plan.framework_hint, "vite");
        assert_eq!(plan.url, "http://127.0.0.1:5173/");
    }

    #[test]
    fn detect_website_launch_plan_honors_preview_mode() {
        let dir = tempfile::tempdir().unwrap();
        write_package(
            dir.path(),
            r#"{
                "scripts": { "preview": "vite preview" },
                "devDependencies": { "vite": "^5.0.0" }
            }"#,
        );

        let plan =
            detect_website_launch_plan(&serde_json::json!({ "mode": "preview" }), dir.path())
                .unwrap();
        assert_eq!(plan.script, "preview");
        assert_eq!(plan.url, "http://127.0.0.1:4173/");
    }

    #[test]
    fn extract_html_title_and_preview_are_clean() {
        let html = r#"
            <html>
              <head><title>  Demo Site  </title></head>
              <body><h1>Hello</h1><script>ignore()</script><p>Readable preview text.</p></body>
            </html>
        "#;
        assert_eq!(extract_html_title(html).as_deref(), Some("Demo Site"));
        let preview = html_preview_text(html);
        assert!(preview.contains("Hello"));
        assert!(preview.contains("Readable preview text."));
        assert!(!preview.contains("ignore()"));
    }

    #[test]
    fn extract_local_asset_urls_resolves_relative_assets() {
        let html = r#"
            <html>
              <head>
                <link rel="stylesheet" href="/assets/app.css">
                <script src="./main.js"></script>
              </head>
              <body>
                <img src="images/logo.png">
                <a href="https://example.com">external</a>
              </body>
            </html>
        "#;
        let assets = extract_local_asset_urls("http://127.0.0.1:5173/about/", html);
        assert!(assets
            .iter()
            .any(|asset| asset == "http://127.0.0.1:5173/assets/app.css"));
        assert!(assets
            .iter()
            .any(|asset| asset == "http://127.0.0.1:5173/about/main.js"));
        assert!(assets
            .iter()
            .any(|asset| asset == "http://127.0.0.1:5173/about/images/logo.png"));
        assert!(!assets.iter().any(|asset| asset.contains("example.com")));
    }

    #[test]
    fn normalize_route_hints_deduplicates_and_prefixes_slashes() {
        let routes = normalize_route_hints(vec![
            "".to_string(),
            "pricing".to_string(),
            "/pricing".to_string(),
            "/".to_string(),
        ]);
        assert_eq!(routes, vec!["/".to_string(), "/pricing".to_string()]);
    }

    #[tokio::test]
    async fn probe_website_once_reads_local_title() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                use std::io::Read;
                let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 67\r\nConnection: close\r\n\r\n<html><head><title>Probe Test</title></head><body>hello</body></html>";
                let mut request = [0_u8; 1024];
                let _ = stream.read(&mut request);
                use std::io::Write;
                let _ = stream.write_all(response);
            }
        });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let probe = probe_website_once(&client, &format!("http://{}/", addr))
            .await
            .unwrap();
        assert_eq!(probe.status, 200);
        assert_eq!(probe.title.as_deref(), Some("Probe Test"));
        assert!(probe.body_preview.contains("hello"));
    }
}
