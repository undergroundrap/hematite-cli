use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_MAX_ENTRIES: usize = 10;
const MAX_ENTRIES_CAP: usize = 25;
const DIRECTORY_SCAN_NODE_BUDGET: usize = 25_000;

pub async fn inspect_host(args: &Value) -> Result<String, String> {
    let topic = args
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("summary");
    let max_entries = parse_max_entries(args);

    match topic {
        "summary" => inspect_summary(max_entries),
        "toolchains" => inspect_toolchains(),
        "path" => inspect_path(max_entries),
        "env_doctor" => inspect_env_doctor(max_entries),
        "fix_plan" => inspect_fix_plan(parse_issue_text(args), parse_port_filter(args), max_entries).await,
        "network" => inspect_network(max_entries),
        "services" => inspect_services(parse_name_filter(args), max_entries),
        "processes" => inspect_processes(parse_name_filter(args), max_entries),
        "desktop" => inspect_known_directory("Desktop", desktop_dir(), max_entries).await,
        "downloads" => inspect_known_directory("Downloads", downloads_dir(), max_entries).await,
        "disk" => {
            let path = resolve_optional_path(args)?;
            inspect_disk(path, max_entries).await
        }
        "ports" => inspect_ports(parse_port_filter(args), max_entries),
        "log_check" => inspect_log_check(max_entries),
        "startup_items" => inspect_startup_items(max_entries),
        "health_report" | "system_health" => inspect_health_report(),
        "storage" => inspect_storage(max_entries),
        "hardware" => inspect_hardware(),
        "os_config" | "system_config" => inspect_os_config(),
        "resource_load" | "performance" | "system_load" | "performance_diagnosis" => inspect_resource_load(),
        "repo_doctor" => {
            let path = resolve_optional_path(args)?;
            inspect_repo_doctor(path, max_entries)
        }
        "directory" => {
            let raw_path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    "Missing required argument: 'path' for inspect_host(topic: \"directory\")"
                        .to_string()
                })?;
            let resolved = resolve_path(raw_path)?;
            inspect_directory("Directory", resolved, max_entries).await
        }
        other => Err(format!(
            "Unknown inspect_host topic '{}'. Use one of: summary, toolchains, path, env_doctor, fix_plan, network, services, processes, desktop, downloads, directory, disk, ports, repo_doctor, log_check, startup_items, health_report, storage, hardware, os_config, resource_load.",
            other
        )),
    }
}

fn parse_max_entries(args: &Value) -> usize {
    args.get("max_entries")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_MAX_ENTRIES)
        .clamp(1, MAX_ENTRIES_CAP)
}

fn parse_port_filter(args: &Value) -> Option<u16> {
    args.get("port")
        .and_then(|v| v.as_u64())
        .and_then(|n| u16::try_from(n).ok())
}

fn parse_name_filter(args: &Value) -> Option<String> {
    args.get("name")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn parse_issue_text(args: &Value) -> Option<String> {
    args.get("issue")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn resolve_optional_path(args: &Value) -> Result<PathBuf, String> {
    match args.get("path").and_then(|v| v.as_str()) {
        Some(raw_path) => resolve_path(raw_path),
        None => {
            std::env::current_dir().map_err(|e| format!("Failed to get current directory: {e}"))
        }
    }
}

fn inspect_summary(max_entries: usize) -> Result<String, String> {
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {e}"))?;
    let workspace_root = crate::tools::file_ops::workspace_root();
    let workspace_mode = workspace_mode_label(&workspace_root);
    let path_stats = analyze_path_env();
    let toolchains = collect_toolchains();

    let mut out = String::from("Host inspection: summary\n\n");
    out.push_str(&format!("- OS: {}\n", std::env::consts::OS));
    out.push_str(&format!("- Current directory: {}\n", current_dir.display()));
    out.push_str(&format!("- Workspace root: {}\n", workspace_root.display()));
    out.push_str(&format!("- Workspace mode: {}\n", workspace_mode));
    out.push_str(&format!("- Preferred shell: {}\n", preferred_shell_label()));
    out.push_str(&format!(
        "- PATH entries: {} total, {} unique, {} duplicates, {} missing\n",
        path_stats.total_entries,
        path_stats.unique_entries,
        path_stats.duplicate_entries.len(),
        path_stats.missing_entries.len()
    ));

    if toolchains.found.is_empty() {
        out.push_str(
            "- Toolchains found: none of the common developer tools were detected on PATH\n",
        );
    } else {
        out.push_str("- Toolchains found:\n");
        for (label, version) in toolchains.found.iter().take(max_entries.min(8)) {
            out.push_str(&format!("  - {}: {}\n", label, version));
        }
        if toolchains.found.len() > max_entries.min(8) {
            out.push_str(&format!(
                "  - ... {} more found tools omitted\n",
                toolchains.found.len() - max_entries.min(8)
            ));
        }
    }

    if !toolchains.missing.is_empty() {
        out.push_str(&format!(
            "- Common tools not detected on PATH: {}\n",
            toolchains.missing.join(", ")
        ));
    }

    for (label, path) in [("Desktop", desktop_dir()), ("Downloads", downloads_dir())] {
        match path {
            Some(path) if path.exists() => match count_top_level_items(&path) {
                Ok(count) => out.push_str(&format!(
                    "- {}: {} top-level items at {}\n",
                    label,
                    count,
                    path.display()
                )),
                Err(e) => out.push_str(&format!(
                    "- {}: exists at {} but could not inspect ({})\n",
                    label,
                    path.display(),
                    e
                )),
            },
            Some(path) => out.push_str(&format!(
                "- {}: expected at {} but not found\n",
                label,
                path.display()
            )),
            None => out.push_str(&format!("- {}: location unavailable on this host\n", label)),
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_toolchains() -> Result<String, String> {
    let report = collect_toolchains();
    let mut out = String::from("Host inspection: toolchains\n\n");

    if report.found.is_empty() {
        out.push_str("- No common developer tools were detected on PATH.");
    } else {
        out.push_str("Detected developer tools:\n");
        for (label, version) in report.found {
            out.push_str(&format!("- {}: {}\n", label, version));
        }
    }

    if !report.missing.is_empty() {
        out.push_str("\nNot detected on PATH:\n");
        for label in report.missing {
            out.push_str(&format!("- {}\n", label));
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_path(max_entries: usize) -> Result<String, String> {
    let path_stats = analyze_path_env();
    let mut out = String::from("Host inspection: PATH\n\n");
    out.push_str(&format!("- Total entries: {}\n", path_stats.total_entries));
    out.push_str(&format!(
        "- Unique entries: {}\n",
        path_stats.unique_entries
    ));
    out.push_str(&format!(
        "- Duplicate entries: {}\n",
        path_stats.duplicate_entries.len()
    ));
    out.push_str(&format!(
        "- Missing paths: {}\n",
        path_stats.missing_entries.len()
    ));

    out.push_str("\nPATH entries:\n");
    for entry in path_stats.entries.iter().take(max_entries) {
        out.push_str(&format!("- {}\n", entry));
    }
    if path_stats.entries.len() > max_entries {
        out.push_str(&format!(
            "- ... {} more entries omitted\n",
            path_stats.entries.len() - max_entries
        ));
    }

    if !path_stats.duplicate_entries.is_empty() {
        out.push_str("\nDuplicate entries:\n");
        for entry in path_stats.duplicate_entries.iter().take(max_entries) {
            out.push_str(&format!("- {}\n", entry));
        }
        if path_stats.duplicate_entries.len() > max_entries {
            out.push_str(&format!(
                "- ... {} more duplicates omitted\n",
                path_stats.duplicate_entries.len() - max_entries
            ));
        }
    }

    if !path_stats.missing_entries.is_empty() {
        out.push_str("\nMissing directories:\n");
        for entry in path_stats.missing_entries.iter().take(max_entries) {
            out.push_str(&format!("- {}\n", entry));
        }
        if path_stats.missing_entries.len() > max_entries {
            out.push_str(&format!(
                "- ... {} more missing entries omitted\n",
                path_stats.missing_entries.len() - max_entries
            ));
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_env_doctor(max_entries: usize) -> Result<String, String> {
    let path_stats = analyze_path_env();
    let toolchains = collect_toolchains();
    let package_managers = collect_package_managers();
    let findings = build_env_doctor_findings(&toolchains, &package_managers, &path_stats);

    let mut out = String::from("Host inspection: env_doctor\n\n");
    out.push_str(&format!(
        "- PATH health: {} duplicates, {} missing entries\n",
        path_stats.duplicate_entries.len(),
        path_stats.missing_entries.len()
    ));
    out.push_str(&format!("- Toolchains found: {}\n", toolchains.found.len()));
    out.push_str(&format!(
        "- Package managers found: {}\n",
        package_managers.found.len()
    ));

    if !package_managers.found.is_empty() {
        out.push_str("\nPackage managers:\n");
        for (label, version) in package_managers.found.iter().take(max_entries) {
            out.push_str(&format!("- {}: {}\n", label, version));
        }
        if package_managers.found.len() > max_entries {
            out.push_str(&format!(
                "- ... {} more package managers omitted\n",
                package_managers.found.len() - max_entries
            ));
        }
    }

    if !path_stats.duplicate_entries.is_empty() {
        out.push_str("\nDuplicate PATH entries:\n");
        for entry in path_stats.duplicate_entries.iter().take(max_entries.min(5)) {
            out.push_str(&format!("- {}\n", entry));
        }
        if path_stats.duplicate_entries.len() > max_entries.min(5) {
            out.push_str(&format!(
                "- ... {} more duplicate entries omitted\n",
                path_stats.duplicate_entries.len() - max_entries.min(5)
            ));
        }
    }

    if !path_stats.missing_entries.is_empty() {
        out.push_str("\nMissing PATH entries:\n");
        for entry in path_stats.missing_entries.iter().take(max_entries.min(5)) {
            out.push_str(&format!("- {}\n", entry));
        }
        if path_stats.missing_entries.len() > max_entries.min(5) {
            out.push_str(&format!(
                "- ... {} more missing entries omitted\n",
                path_stats.missing_entries.len() - max_entries.min(5)
            ));
        }
    }

    if !findings.is_empty() {
        out.push_str("\nFindings:\n");
        for finding in findings.iter().take(max_entries.max(5)) {
            out.push_str(&format!("- {}\n", finding));
        }
        if findings.len() > max_entries.max(5) {
            out.push_str(&format!(
                "- ... {} more findings omitted\n",
                findings.len() - max_entries.max(5)
            ));
        }
    } else {
        out.push_str("\nFindings:\n- No obvious environment drift was detected from PATH and package-manager checks.");
    }

    out.push_str(
        "\nGuidance:\n- This report already includes the PATH and package-manager health details. Do not call `inspect_host(path)` next unless the user explicitly asks for the raw PATH list.",
    );

    Ok(out.trim_end().to_string())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FixPlanKind {
    EnvPath,
    PortConflict,
    LmStudio,
    Generic,
}

async fn inspect_fix_plan(
    issue: Option<String>,
    port_filter: Option<u16>,
    max_entries: usize,
) -> Result<String, String> {
    let issue = issue.unwrap_or_else(|| {
        "Help me fix PATH, toolchain, port-conflict, or LM Studio connectivity problems."
            .to_string()
    });
    let plan_kind = classify_fix_plan_kind(&issue, port_filter);
    match plan_kind {
        FixPlanKind::EnvPath => inspect_env_fix_plan(&issue, max_entries),
        FixPlanKind::PortConflict => inspect_port_fix_plan(&issue, port_filter, max_entries),
        FixPlanKind::LmStudio => inspect_lm_studio_fix_plan(&issue, max_entries).await,
        FixPlanKind::Generic => inspect_generic_fix_plan(&issue),
    }
}

fn classify_fix_plan_kind(issue: &str, port_filter: Option<u16>) -> FixPlanKind {
    let lower = issue.to_ascii_lowercase();
    if port_filter.is_some()
        || lower.contains("port ")
        || lower.contains("address already in use")
        || lower.contains("already in use")
        || lower.contains("what owns port")
        || lower.contains("listening on port")
    {
        FixPlanKind::PortConflict
    } else if lower.contains("lm studio")
        || lower.contains("localhost:1234")
        || lower.contains("/v1/models")
        || lower.contains("no coding model loaded")
        || lower.contains("embedding model")
        || lower.contains("server on port 1234")
        || lower.contains("runtime refresh")
    {
        FixPlanKind::LmStudio
    } else if lower.contains("cargo")
        || lower.contains("rustc")
        || lower.contains("path")
        || lower.contains("package manager")
        || lower.contains("package managers")
        || lower.contains("toolchain")
        || lower.contains("winget")
        || lower.contains("choco")
        || lower.contains("scoop")
        || lower.contains("python")
        || lower.contains("node")
    {
        FixPlanKind::EnvPath
    } else {
        FixPlanKind::Generic
    }
}

fn inspect_env_fix_plan(issue: &str, max_entries: usize) -> Result<String, String> {
    let path_stats = analyze_path_env();
    let toolchains = collect_toolchains();
    let package_managers = collect_package_managers();
    let findings = build_env_doctor_findings(&toolchains, &package_managers, &path_stats);
    let found_tools = toolchains
        .found
        .iter()
        .map(|(label, _)| label.as_str())
        .collect::<HashSet<_>>();
    let found_managers = package_managers
        .found
        .iter()
        .map(|(label, _)| label.as_str())
        .collect::<HashSet<_>>();

    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: environment/path\n");
    out.push_str(&format!(
        "- PATH health: {} duplicates, {} missing entries\n",
        path_stats.duplicate_entries.len(),
        path_stats.missing_entries.len()
    ));
    out.push_str(&format!("- Toolchains found: {}\n", toolchains.found.len()));
    out.push_str(&format!(
        "- Package managers found: {}\n",
        package_managers.found.len()
    ));

    out.push_str("\nLikely causes:\n");
    if found_tools.contains("rustc") && !found_managers.contains("cargo") {
        out.push_str(
            "- Rust is present but Cargo is not. The most common cause is a missing Rustup bin path such as `%USERPROFILE%\\.cargo\\bin` on Windows or `$HOME/.cargo/bin` on Unix.\n",
        );
    }
    if path_stats.duplicate_entries.is_empty()
        && path_stats.missing_entries.is_empty()
        && !findings.is_empty()
    {
        for finding in findings.iter().take(max_entries.max(4)) {
            out.push_str(&format!("- {}\n", finding));
        }
    } else {
        if !path_stats.duplicate_entries.is_empty() {
            out.push_str("- Duplicate PATH rows create clutter and can hide which install path is actually winning.\n");
        }
        if !path_stats.missing_entries.is_empty() {
            out.push_str("- Stale PATH rows point at directories that no longer exist, which makes environment drift harder to reason about.\n");
        }
    }
    if found_tools.contains("node")
        && !found_managers.contains("npm")
        && !found_managers.contains("pnpm")
    {
        out.push_str("- Node is present without a detected package manager. That usually means a partial install or PATH drift.\n");
    }
    if found_tools.contains("python")
        && !found_managers.contains("pip")
        && !found_managers.contains("uv")
        && !found_managers.contains("pipx")
    {
        out.push_str("- Python is present without a detected package manager. That usually means the launcher works but Scripts/bin is not discoverable.\n");
    }

    out.push_str("\nFix plan:\n");
    out.push_str("- Verify the command resolution first with `where cargo`, `where rustc`, `where python`, or `Get-Command cargo` so you know whether the tool is missing or just hidden behind PATH drift.\n");
    if found_tools.contains("rustc") && !found_managers.contains("cargo") {
        out.push_str("- Add the Rustup bin directory to your user PATH, then restart the terminal. On Windows that is usually `%USERPROFILE%\\.cargo\\bin`.\n");
    } else if !found_tools.contains("rustc") && !found_managers.contains("cargo") {
        out.push_str("- If Rust is not installed at all, install Rustup first, then reopen the terminal. On Windows the clean path is `winget install Rustlang.Rustup`.\n");
    }
    if !path_stats.duplicate_entries.is_empty() || !path_stats.missing_entries.is_empty() {
        out.push_str("- Clean duplicate or dead PATH rows in Environment Variables so the winning toolchain path is obvious and stable.\n");
    }
    if found_tools.contains("node")
        && !found_managers.contains("npm")
        && !found_managers.contains("pnpm")
    {
        out.push_str("- Repair the Node install or reinstall Node so `npm` is restored. If you prefer `pnpm`, install it after Node is healthy.\n");
    }
    if found_tools.contains("python")
        && !found_managers.contains("pip")
        && !found_managers.contains("uv")
        && !found_managers.contains("pipx")
    {
        out.push_str("- Repair Python or install a Python package manager explicitly. `py -m ensurepip --upgrade` is the least-invasive first check on Windows.\n");
    }

    if !path_stats.duplicate_entries.is_empty() {
        out.push_str("\nExample duplicate PATH rows:\n");
        for entry in path_stats.duplicate_entries.iter().take(max_entries.min(5)) {
            out.push_str(&format!("- {}\n", entry));
        }
    }
    if !path_stats.missing_entries.is_empty() {
        out.push_str("\nExample missing PATH rows:\n");
        for entry in path_stats.missing_entries.iter().take(max_entries.min(5)) {
            out.push_str(&format!("- {}\n", entry));
        }
    }

    out.push_str(
        "\nWhy this works:\n- PATH problems are usually resolution problems, not mysterious tool failures. Verify the executable path, repair the install only when needed, then restart the shell so the environment is rebuilt cleanly.",
    );
    Ok(out.trim_end().to_string())
}

fn inspect_port_fix_plan(
    issue: &str,
    port_filter: Option<u16>,
    max_entries: usize,
) -> Result<String, String> {
    let requested_port = port_filter.or_else(|| first_port_in_text(issue));
    let listeners = collect_listening_ports().unwrap_or_default();
    let mut matching = listeners;
    if let Some(port) = requested_port {
        matching.retain(|entry| entry.port == port);
    }
    let processes = collect_processes().unwrap_or_default();

    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: port_conflict\n");
    if let Some(port) = requested_port {
        out.push_str(&format!("- Requested port: {}\n", port));
    } else {
        out.push_str("- Requested port: not parsed from the issue text\n");
    }
    out.push_str(&format!("- Matching listeners found: {}\n", matching.len()));

    if !matching.is_empty() {
        out.push_str("\nCurrent listeners:\n");
        for entry in matching.iter().take(max_entries.min(5)) {
            let process_name = entry
                .pid
                .as_deref()
                .and_then(|pid| pid.parse::<u32>().ok())
                .and_then(|pid| {
                    processes
                        .iter()
                        .find(|process| process.pid == pid)
                        .map(|process| process.name.as_str())
                })
                .unwrap_or("unknown");
            let pid = entry.pid.as_deref().unwrap_or("unknown");
            out.push_str(&format!(
                "- {} {} ({}) pid {} process {}\n",
                entry.protocol, entry.local, entry.state, pid, process_name
            ));
        }
    }

    out.push_str("\nFix plan:\n");
    out.push_str("- Identify whether the existing listener is expected. If it is your dev server, reuse it or change your app config instead of killing it blindly.\n");
    if !matching.is_empty() {
        out.push_str("- If the listener is stale, stop the owning process by PID or close the parent app cleanly. On Windows, `taskkill /PID <pid> /F` is the blunt option, but closing the app normally is safer.\n");
    } else {
        out.push_str("- Re-run a listener check right before changing anything. Port conflicts can disappear if a stale dev process exits between checks.\n");
    }
    out.push_str("- If the port is intentionally occupied, move your app to another port instead of fighting the existing process.\n");
    out.push_str("- If the port keeps getting reclaimed after you kill it, inspect startup services or background tools rather than repeating `taskkill` loops.\n");
    out.push_str(
        "\nWhy this works:\n- Port conflicts are ownership problems. Once you know which PID owns the listener, the clean fix is either stop that owner or move your app to a different port.",
    );
    Ok(out.trim_end().to_string())
}

async fn inspect_lm_studio_fix_plan(issue: &str, max_entries: usize) -> Result<String, String> {
    let config = crate::agent::config::load_config();
    let configured_api = config
        .api_url
        .unwrap_or_else(|| "http://localhost:1234/v1".to_string());
    let models_url = format!("{}/models", configured_api.trim_end_matches('/'));
    let reachability = probe_http_endpoint(&models_url).await;
    let embed_model = detect_loaded_embed_model(&configured_api).await;

    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: lm_studio\n");
    out.push_str(&format!("- Configured API URL: {}\n", configured_api));
    out.push_str(&format!("- Probe URL: {}\n", models_url));
    match &reachability {
        EndpointProbe::Reachable(status) => {
            out.push_str(&format!("- Endpoint reachable: yes (HTTP {})\n", status))
        }
        EndpointProbe::Unreachable(detail) => {
            out.push_str(&format!("- Endpoint reachable: no ({})\n", detail))
        }
    }
    out.push_str(&format!(
        "- Embedding model loaded: {}\n",
        embed_model.as_deref().unwrap_or("none detected")
    ));

    out.push_str("\nFix plan:\n");
    match reachability {
        EndpointProbe::Reachable(_) => {
            out.push_str("- LM Studio is reachable, so the first fix step is model state, not networking. Check whether a chat model is actually loaded and whether the local server is still serving the model you expect.\n");
        }
        EndpointProbe::Unreachable(_) => {
            out.push_str("- Start LM Studio and make sure the local server is running on the configured endpoint. Hematite defaults to `http://localhost:1234/v1` unless `.hematite/settings.json` overrides `api_url`.\n");
        }
    }
    out.push_str("- If Hematite is pointed at the wrong endpoint, fix `api_url` in `.hematite/settings.json` and restart or run `/runtime-refresh`.\n");
    out.push_str("- If chat works but semantic search does not, load the embedding model as a second resident model in LM Studio. Hematite expects a `nomic-embed` style model there.\n");
    out.push_str("- If LM Studio keeps responding with no model loaded, load the coding model first, then start the server again before blaming Hematite.\n");
    out.push_str("- If the server is up but turns still fail, narrow the prompt or refresh the runtime profile so Hematite picks up the live model and context budget.\n");
    if let Some(model) = embed_model {
        out.push_str(&format!(
            "- Current embedding model already visible: {}. That means the embeddings lane is configured, so focus on the chat model or endpoint next.\n",
            model
        ));
    }
    if max_entries > 0 {
        out.push_str(
            "\nWhy this works:\n- LM Studio failures usually collapse into three buckets: wrong endpoint, server not running, or models not loaded. Confirm the endpoint first, then fix model state instead of guessing.",
        );
    }
    Ok(out.trim_end().to_string())
}

fn inspect_generic_fix_plan(issue: &str) -> Result<String, String> {
    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: generic\n");
    out.push_str(
        "\nGuidance:\n- Use `fix_plan` for one of the current structured remediation lanes: PATH/toolchain drift, port conflicts, or LM Studio connectivity.\n- If your issue is outside those lanes, run the closest `inspect_host` topic first to ground the diagnosis before proposing changes.",
    );
    Ok(out.trim_end().to_string())
}

fn inspect_resource_load() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_Processor).LoadPercentage; Get-CimInstance Win32_OperatingSystem | Select-Object TotalVisibleMemorySize, FreePhysicalMemory | ConvertTo-Json -Compress",
            ])
            .output()
            .map_err(|e| format!("Failed to run powershell: {e}"))?;

        let text = String::from_utf8_lossy(&output.stdout);
        let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
        
        let cpu_load = lines.next().and_then(|l| l.parse::<u32>().ok()).unwrap_or(0);
        let mem_json = lines.collect::<Vec<_>>().join("");
        let mem_val: Value = serde_json::from_str(&mem_json).unwrap_or(Value::Null);

        let total_kb = mem_val["TotalVisibleMemorySize"].as_u64().unwrap_or(1);
        let free_kb = mem_val["FreePhysicalMemory"].as_u64().unwrap_or(0);
        let used_kb = total_kb.saturating_sub(free_kb);
        let mem_percent = if total_kb > 0 { (used_kb * 100) / total_kb } else { 0 };

        let mut out = String::from("Host inspection: resource_load\n\n");
        out.push_str("**System Performance Summary:**\n");
        out.push_str(&format!("- CPU Load: {}%\n", cpu_load));
        out.push_str(&format!(
            "- Memory Usage: {} / {} ({}%)\n",
            human_bytes(used_kb * 1024),
            human_bytes(total_kb * 1024),
            mem_percent
        ));

        if cpu_load > 85 {
            out.push_str("\n[Warning] CPU load is extremely high. System may be unresponsive.\n");
        }
        if mem_percent > 90 {
            out.push_str("\n[Warning] Memory usage is near capacity. Swap activity may slow down the machine.\n");
        }

        Ok(out)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok("Resource load inspection is not yet implemented for this platform.".to_string())
    }
}

#[derive(Debug)]
enum EndpointProbe {
    Reachable(u16),
    Unreachable(String),
}

async fn probe_http_endpoint(url: &str) -> EndpointProbe {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(client) => client,
        Err(err) => return EndpointProbe::Unreachable(err.to_string()),
    };

    match client.get(url).send().await {
        Ok(resp) => EndpointProbe::Reachable(resp.status().as_u16()),
        Err(err) => return EndpointProbe::Unreachable(err.to_string()),
    }
}

async fn detect_loaded_embed_model(configured_api: &str) -> Option<String> {
    let base = configured_api.trim_end_matches("/v1").trim_end_matches('/');
    let url = format!("{}/api/v0/models", base);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()?;

    #[derive(serde::Deserialize)]
    struct ModelList {
        data: Vec<ModelEntry>,
    }
    #[derive(serde::Deserialize)]
    struct ModelEntry {
        id: String,
        #[serde(rename = "type", default)]
        model_type: String,
        #[serde(default)]
        state: String,
    }

    let response = client.get(url).send().await.ok()?;
    let models = response.json::<ModelList>().await.ok()?;
    models
        .data
        .into_iter()
        .find(|model| model.model_type == "embeddings" && model.state == "loaded")
        .map(|model| model.id)
}

fn first_port_in_text(text: &str) -> Option<u16> {
    text.split(|c: char| !c.is_ascii_digit())
        .find(|fragment| !fragment.is_empty())
        .and_then(|fragment| fragment.parse::<u16>().ok())
}

fn inspect_processes(name_filter: Option<String>, max_entries: usize) -> Result<String, String> {
    let mut processes = collect_processes()?;
    if let Some(filter) = name_filter.as_deref() {
        let lowered = filter.to_ascii_lowercase();
        processes.retain(|entry| entry.name.to_ascii_lowercase().contains(&lowered));
    }
    processes.sort_by(|a, b| {
        b.memory_bytes
            .cmp(&a.memory_bytes)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.pid.cmp(&b.pid))
    });

    let total_memory: u64 = processes.iter().map(|entry| entry.memory_bytes).sum();

    let mut out = String::from("Host inspection: processes\n\n");
    if let Some(filter) = name_filter.as_deref() {
        out.push_str(&format!("- Filter name: {}\n", filter));
    }
    out.push_str(&format!("- Processes found: {}\n", processes.len()));
    out.push_str(&format!(
        "- Total reported working set: {}\n",
        human_bytes(total_memory)
    ));

    if processes.is_empty() {
        out.push_str("\nNo running processes matched.");
        return Ok(out);
    }

    out.push_str("\nTop processes by resource usage:\n");
    for entry in processes.iter().take(max_entries) {
        let cpu_str = entry
            .cpu_seconds
            .map(|s| format!(" [CPU: {:.1}s]", s))
            .unwrap_or_default();
        out.push_str(&format!(
            "- {} (pid {}) - {}{}{}\n",
            entry.name,
            entry.pid,
            human_bytes(entry.memory_bytes),
            cpu_str,
            entry
                .detail
                .as_deref()
                .map(|detail| format!(" [{}]", detail))
                .unwrap_or_default()
        ));
    }
    if processes.len() > max_entries {
        out.push_str(&format!(
            "- ... {} more processes omitted\n",
            processes.len() - max_entries
        ));
    }

    Ok(out.trim_end().to_string())
}

fn inspect_network(max_entries: usize) -> Result<String, String> {
    let adapters = collect_network_adapters()?;
    let active_count = adapters
        .iter()
        .filter(|adapter| adapter.is_active())
        .count();
    let exposure = listener_exposure_summary(collect_listening_ports().ok().unwrap_or_default());

    let mut out = String::from("Host inspection: network\n\n");
    out.push_str(&format!("- Adapters found: {}\n", adapters.len()));
    out.push_str(&format!("- Active adapters: {}\n", active_count));
    out.push_str(&format!(
        "- Listener exposure: {} loopback-only, {} wildcard/public, {} specific-bind\n",
        exposure.loopback_only, exposure.wildcard_public, exposure.specific_bind
    ));

    if adapters.is_empty() {
        out.push_str("\nNo adapter details were detected.");
        return Ok(out);
    }

    out.push_str("\nAdapter summary:\n");
    for adapter in adapters.iter().take(max_entries) {
        let status = if adapter.is_active() {
            "active"
        } else if adapter.disconnected {
            "disconnected"
        } else {
            "idle"
        };
        let mut details = vec![status.to_string()];
        if !adapter.ipv4.is_empty() {
            details.push(format!("ipv4 {}", adapter.ipv4.join(", ")));
        }
        if !adapter.ipv6.is_empty() {
            details.push(format!("ipv6 {}", adapter.ipv6.join(", ")));
        }
        if !adapter.gateways.is_empty() {
            details.push(format!("gateway {}", adapter.gateways.join(", ")));
        }
        if !adapter.dns_servers.is_empty() {
            details.push(format!("dns {}", adapter.dns_servers.join(", ")));
        }
        out.push_str(&format!("- {} - {}\n", adapter.name, details.join(" | ")));
    }
    if adapters.len() > max_entries {
        out.push_str(&format!(
            "- ... {} more adapters omitted\n",
            adapters.len() - max_entries
        ));
    }

    Ok(out.trim_end().to_string())
}

fn inspect_services(name_filter: Option<String>, max_entries: usize) -> Result<String, String> {
    let mut services = collect_services()?;
    if let Some(filter) = name_filter.as_deref() {
        let lowered = filter.to_ascii_lowercase();
        services.retain(|entry| {
            entry.name.to_ascii_lowercase().contains(&lowered)
                || entry
                    .display_name
                    .as_deref()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains(&lowered)
        });
    }

    services.sort_by(|a, b| {
        service_status_rank(&a.status)
            .cmp(&service_status_rank(&b.status))
            .then_with(|| a.name.cmp(&b.name))
    });

    let running = services
        .iter()
        .filter(|entry| {
            entry.status.eq_ignore_ascii_case("running")
                || entry.status.eq_ignore_ascii_case("active")
        })
        .count();
    let failed = services
        .iter()
        .filter(|entry| {
            entry.status.eq_ignore_ascii_case("failed")
                || entry.status.eq_ignore_ascii_case("error")
                || entry.status.eq_ignore_ascii_case("stopped")
        })
        .count();

    let mut out = String::from("Host inspection: services\n\n");
    if let Some(filter) = name_filter.as_deref() {
        out.push_str(&format!("- Filter name: {}\n", filter));
    }
    out.push_str(&format!("- Services found: {}\n", services.len()));
    out.push_str(&format!("- Running/active: {}\n", running));
    out.push_str(&format!("- Failed/stopped: {}\n", failed));

    if services.is_empty() {
        out.push_str("\nNo services matched.");
        return Ok(out);
    }

    out.push_str("\nService summary:\n");
    for entry in services.iter().take(max_entries) {
        let startup = entry
            .startup
            .as_deref()
            .map(|value| format!(" | startup {}", value))
            .unwrap_or_default();
        let display = entry
            .display_name
            .as_deref()
            .filter(|value| *value != &entry.name)
            .map(|value| format!(" [{}]", value))
            .unwrap_or_default();
        out.push_str(&format!(
            "- {}{} - {}{}\n",
            entry.name, display, entry.status, startup
        ));
    }
    if services.len() > max_entries {
        out.push_str(&format!(
            "- ... {} more services omitted\n",
            services.len() - max_entries
        ));
    }

    Ok(out.trim_end().to_string())
}

async fn inspect_disk(path: PathBuf, max_entries: usize) -> Result<String, String> {
    inspect_directory("Disk", path, max_entries).await
}

fn inspect_ports(port_filter: Option<u16>, max_entries: usize) -> Result<String, String> {
    let mut listeners = collect_listening_ports()?;
    if let Some(port) = port_filter {
        listeners.retain(|entry| entry.port == port);
    }
    listeners.sort_by(|a, b| a.port.cmp(&b.port).then_with(|| a.local.cmp(&b.local)));

    let mut out = String::from("Host inspection: ports\n\n");
    if let Some(port) = port_filter {
        out.push_str(&format!("- Filter port: {}\n", port));
    }
    out.push_str(&format!(
        "- Listening endpoints found: {}\n",
        listeners.len()
    ));

    if listeners.is_empty() {
        out.push_str("\nNo listening endpoints matched.");
        return Ok(out);
    }

    out.push_str("\nListening endpoints:\n");
    for entry in listeners.iter().take(max_entries) {
        let pid = entry
            .pid
            .as_deref()
            .map(|pid| format!(" pid {}", pid))
            .unwrap_or_default();
        out.push_str(&format!(
            "- {} {} ({}){}\n",
            entry.protocol, entry.local, entry.state, pid
        ));
    }
    if listeners.len() > max_entries {
        out.push_str(&format!(
            "- ... {} more listening endpoints omitted\n",
            listeners.len() - max_entries
        ));
    }

    Ok(out.trim_end().to_string())
}

fn inspect_repo_doctor(path: PathBuf, max_entries: usize) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path.display()));
    }

    let markers = collect_project_markers(&path);
    let hematite_state = collect_hematite_state(&path);
    let git_state = inspect_git_state(&path);
    let release_state = inspect_release_artifacts(&path);

    let mut out = String::from("Host inspection: repo_doctor\n\n");
    out.push_str(&format!("- Path: {}\n", path.display()));
    out.push_str(&format!(
        "- Workspace mode: {}\n",
        workspace_mode_for_path(&path)
    ));

    if markers.is_empty() {
        out.push_str("- Project markers: none of Cargo.toml, package.json, pyproject.toml, go.mod, justfile, Makefile, or .git were found at this path\n");
    } else {
        out.push_str("- Project markers:\n");
        for marker in markers.iter().take(max_entries) {
            out.push_str(&format!("  - {}\n", marker));
        }
    }

    match git_state {
        Some(git) => {
            out.push_str(&format!("- Git root: {}\n", git.root.display()));
            out.push_str(&format!("- Git branch: {}\n", git.branch));
            out.push_str(&format!("- Git status: {}\n", git.status_label()));
        }
        None => out.push_str("- Git: not inside a detected work tree\n"),
    }

    out.push_str(&format!(
        "- Hematite docs/imports/reports: {}/{}/{}\n",
        hematite_state.docs_count, hematite_state.import_count, hematite_state.report_count
    ));
    if hematite_state.workspace_profile {
        out.push_str("- Workspace profile: present\n");
    } else {
        out.push_str("- Workspace profile: absent\n");
    }

    if let Some(release) = release_state {
        out.push_str(&format!("- Cargo version: {}\n", release.version));
        out.push_str(&format!(
            "- Windows artifacts for current version: {}/{}/{}\n",
            bool_label(release.portable_dir),
            bool_label(release.portable_zip),
            bool_label(release.setup_exe)
        ));
    }

    Ok(out.trim_end().to_string())
}

async fn inspect_known_directory(
    label: &str,
    path: Option<PathBuf>,
    max_entries: usize,
) -> Result<String, String> {
    let path = path.ok_or_else(|| format!("{} location is unavailable on this host.", label))?;
    inspect_directory(label, path, max_entries).await
}

async fn inspect_directory(
    label: &str,
    path: PathBuf,
    max_entries: usize,
) -> Result<String, String> {
    let label = label.to_string();
    tokio::task::spawn_blocking(move || inspect_directory_sync(&label, &path, max_entries))
        .await
        .map_err(|e| format!("inspect_host task failed: {e}"))?
}

fn inspect_directory_sync(label: &str, path: &Path, max_entries: usize) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path.display()));
    }

    let mut top_level_entries = Vec::new();
    for entry in fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory {}: {e}", path.display()))?
    {
        match entry {
            Ok(entry) => top_level_entries.push(entry),
            Err(_) => continue,
        }
    }
    top_level_entries.sort_by_key(|entry| entry.file_name());

    let top_level_count = top_level_entries.len();
    let mut sample_names = Vec::new();
    let mut largest_entries = Vec::new();
    let mut aggregate = PathAggregate::default();
    let mut budget = DIRECTORY_SCAN_NODE_BUDGET;

    for entry in top_level_entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if sample_names.len() < max_entries {
            sample_names.push(name.clone());
        }
        let kind = match entry.file_type() {
            Ok(ft) if ft.is_dir() => "dir",
            Ok(ft) if ft.is_symlink() => "symlink",
            _ => "file",
        };
        let stats = measure_path(&entry.path(), &mut budget);
        aggregate.merge(&stats);
        largest_entries.push(LargestEntry {
            name,
            kind,
            bytes: stats.total_bytes,
        });
    }

    largest_entries.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));

    let mut out = format!("Directory inspection: {}\n\n", label);
    out.push_str(&format!("- Path: {}\n", path.display()));
    out.push_str(&format!("- Top-level items: {}\n", top_level_count));
    out.push_str(&format!("- Recursive files: {}\n", aggregate.file_count));
    out.push_str(&format!(
        "- Recursive directories: {}\n",
        aggregate.dir_count
    ));
    out.push_str(&format!(
        "- Total size: {}{}\n",
        human_bytes(aggregate.total_bytes),
        if aggregate.partial {
            " (partial scan)"
        } else {
            ""
        }
    ));
    if aggregate.skipped_entries > 0 {
        out.push_str(&format!(
            "- Skipped entries: {} (permissions, symlinks, or scan budget)\n",
            aggregate.skipped_entries
        ));
    }

    if !largest_entries.is_empty() {
        out.push_str("\nLargest top-level entries:\n");
        for entry in largest_entries.iter().take(max_entries) {
            out.push_str(&format!(
                "- {} [{}] - {}\n",
                entry.name,
                entry.kind,
                human_bytes(entry.bytes)
            ));
        }
    }

    if !sample_names.is_empty() {
        out.push_str("\nSample names:\n");
        for name in sample_names {
            out.push_str(&format!("- {}\n", name));
        }
    }

    Ok(out.trim_end().to_string())
}

fn resolve_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Path must not be empty.".to_string());
    }

    if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
    {
        let home = home::home_dir().ok_or_else(|| "Home directory is unavailable.".to_string())?;
        return Ok(home.join(rest));
    }

    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        Ok(path)
    } else {
        let cwd =
            std::env::current_dir().map_err(|e| format!("Failed to get current directory: {e}"))?;
        Ok(cwd.join(path))
    }
}

fn workspace_mode_label(workspace_root: &Path) -> &'static str {
    workspace_mode_for_path(workspace_root)
}

fn workspace_mode_for_path(path: &Path) -> &'static str {
    if is_project_marker_path(path) {
        "project"
    } else if path.join(".hematite").join("docs").exists()
        || path.join(".hematite").join("imports").exists()
        || path.join(".hematite").join("reports").exists()
    {
        "docs-only"
    } else {
        "general directory"
    }
}

fn is_project_marker_path(path: &Path) -> bool {
    [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "composer.json",
        "requirements.txt",
        "Makefile",
        "justfile",
    ]
    .iter()
    .any(|name| path.join(name).exists())
        || path.join(".git").exists()
}

fn preferred_shell_label() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "PowerShell"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "sh"
    }
}

fn desktop_dir() -> Option<PathBuf> {
    home::home_dir().map(|home| home.join("Desktop"))
}

fn downloads_dir() -> Option<PathBuf> {
    home::home_dir().map(|home| home.join("Downloads"))
}

fn count_top_level_items(path: &Path) -> Result<usize, String> {
    let mut count = 0usize;
    for entry in
        fs::read_dir(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?
    {
        if entry.is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

#[derive(Default)]
struct PathAggregate {
    total_bytes: u64,
    file_count: u64,
    dir_count: u64,
    skipped_entries: u64,
    partial: bool,
}

impl PathAggregate {
    fn merge(&mut self, other: &PathAggregate) {
        self.total_bytes += other.total_bytes;
        self.file_count += other.file_count;
        self.dir_count += other.dir_count;
        self.skipped_entries += other.skipped_entries;
        self.partial |= other.partial;
    }
}

struct LargestEntry {
    name: String,
    kind: &'static str,
    bytes: u64,
}

fn measure_path(path: &Path, budget: &mut usize) -> PathAggregate {
    if *budget == 0 {
        return PathAggregate {
            partial: true,
            skipped_entries: 1,
            ..PathAggregate::default()
        };
    }
    *budget -= 1;

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            return PathAggregate {
                skipped_entries: 1,
                ..PathAggregate::default()
            }
        }
    };

    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return PathAggregate {
            skipped_entries: 1,
            ..PathAggregate::default()
        };
    }

    if metadata.is_file() {
        return PathAggregate {
            total_bytes: metadata.len(),
            file_count: 1,
            ..PathAggregate::default()
        };
    }

    if !metadata.is_dir() {
        return PathAggregate::default();
    }

    let mut aggregate = PathAggregate {
        dir_count: 1,
        ..PathAggregate::default()
    };

    let read_dir = match fs::read_dir(path) {
        Ok(read_dir) => read_dir,
        Err(_) => {
            aggregate.skipped_entries += 1;
            return aggregate;
        }
    };

    for child in read_dir {
        match child {
            Ok(child) => {
                let child_stats = measure_path(&child.path(), budget);
                aggregate.merge(&child_stats);
            }
            Err(_) => aggregate.skipped_entries += 1,
        }
    }

    aggregate
}

struct PathAnalysis {
    total_entries: usize,
    unique_entries: usize,
    entries: Vec<String>,
    duplicate_entries: Vec<String>,
    missing_entries: Vec<String>,
}

fn analyze_path_env() -> PathAnalysis {
    let mut entries = Vec::new();
    let mut duplicate_entries = Vec::new();
    let mut missing_entries = Vec::new();
    let mut seen = HashSet::new();

    let raw_path = std::env::var_os("PATH").unwrap_or_default();
    for path in std::env::split_paths(&raw_path) {
        let display = path.display().to_string();
        if display.trim().is_empty() {
            continue;
        }

        let normalized = normalize_path_entry(&display);
        if !seen.insert(normalized) {
            duplicate_entries.push(display.clone());
        }
        if !path.exists() {
            missing_entries.push(display.clone());
        }
        entries.push(display);
    }

    let total_entries = entries.len();
    let unique_entries = seen.len();

    PathAnalysis {
        total_entries,
        unique_entries,
        entries,
        duplicate_entries,
        missing_entries,
    }
}

fn normalize_path_entry(value: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        value
            .replace('/', "\\")
            .trim_end_matches(['\\', '/'])
            .to_ascii_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        value.trim_end_matches('/').to_string()
    }
}

struct ToolchainReport {
    found: Vec<(String, String)>,
    missing: Vec<String>,
}

struct PackageManagerReport {
    found: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
struct ProcessEntry {
    name: String,
    pid: u32,
    memory_bytes: u64,
    cpu_seconds: Option<f64>,
    detail: Option<String>,
}

#[derive(Debug, Clone)]
struct ServiceEntry {
    name: String,
    status: String,
    startup: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct NetworkAdapter {
    name: String,
    ipv4: Vec<String>,
    ipv6: Vec<String>,
    gateways: Vec<String>,
    dns_servers: Vec<String>,
    disconnected: bool,
}

impl NetworkAdapter {
    fn is_active(&self) -> bool {
        !self.disconnected
            && (!self.ipv4.is_empty() || !self.ipv6.is_empty() || !self.gateways.is_empty())
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ListenerExposureSummary {
    loopback_only: usize,
    wildcard_public: usize,
    specific_bind: usize,
}

#[derive(Debug, Clone)]
struct ListeningPort {
    protocol: String,
    local: String,
    port: u16,
    state: String,
    pid: Option<String>,
}

fn collect_listening_ports() -> Result<Vec<ListeningPort>, String> {
    #[cfg(target_os = "windows")]
    {
        collect_windows_listening_ports()
    }
    #[cfg(not(target_os = "windows"))]
    {
        collect_unix_listening_ports()
    }
}

fn collect_network_adapters() -> Result<Vec<NetworkAdapter>, String> {
    #[cfg(target_os = "windows")]
    {
        collect_windows_network_adapters()
    }
    #[cfg(not(target_os = "windows"))]
    {
        collect_unix_network_adapters()
    }
}

fn collect_services() -> Result<Vec<ServiceEntry>, String> {
    #[cfg(target_os = "windows")]
    {
        collect_windows_services()
    }
    #[cfg(not(target_os = "windows"))]
    {
        collect_unix_services()
    }
}

#[cfg(target_os = "windows")]
fn collect_windows_listening_ports() -> Result<Vec<ListeningPort>, String> {
    let output = Command::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .output()
        .map_err(|e| format!("Failed to run netstat: {e}"))?;
    if !output.status.success() {
        return Err("netstat returned a non-success status.".to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut listeners = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("TCP") {
            continue;
        }
        let cols: Vec<&str> = trimmed.split_whitespace().collect();
        if cols.len() < 5 || cols[3] != "LISTENING" {
            continue;
        }
        let Some(port) = extract_port_from_socket(cols[1]) else {
            continue;
        };
        listeners.push(ListeningPort {
            protocol: cols[0].to_string(),
            local: cols[1].to_string(),
            port,
            state: cols[3].to_string(),
            pid: Some(cols[4].to_string()),
        });
    }

    Ok(listeners)
}

#[cfg(not(target_os = "windows"))]
fn collect_unix_listening_ports() -> Result<Vec<ListeningPort>, String> {
    let output = Command::new("ss")
        .args(["-ltn"])
        .output()
        .map_err(|e| format!("Failed to run ss: {e}"))?;
    if !output.status.success() {
        return Err("ss returned a non-success status.".to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut listeners = Vec::new();
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        let Some(port) = extract_port_from_socket(cols[3]) else {
            continue;
        };
        listeners.push(ListeningPort {
            protocol: "tcp".to_string(),
            local: cols[3].to_string(),
            port,
            state: cols[0].to_string(),
            pid: None,
        });
    }

    Ok(listeners)
}

fn collect_processes() -> Result<Vec<ProcessEntry>, String> {
    #[cfg(target_os = "windows")]
    {
        collect_windows_processes()
    }
    #[cfg(not(target_os = "windows"))]
    {
        collect_unix_processes()
    }
}

#[cfg(target_os = "windows")]
fn collect_windows_services() -> Result<Vec<ServiceEntry>, String> {
    let command = "Get-CimInstance Win32_Service | Select-Object Name,State,StartMode,DisplayName | ConvertTo-Json -Compress";
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", command])
        .output()
        .map_err(|e| format!("Failed to run PowerShell service inspection: {e}"))?;
    if !output.status.success() {
        return Err("PowerShell service inspection returned a non-success status.".to_string());
    }

    parse_windows_services_json(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(not(target_os = "windows"))]
fn collect_unix_services() -> Result<Vec<ServiceEntry>, String> {
    let status_output = Command::new("systemctl")
        .args([
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--no-legend",
            "--plain",
        ])
        .output()
        .map_err(|e| format!("Failed to run systemctl list-units: {e}"))?;
    if !status_output.status.success() {
        return Err("systemctl list-units returned a non-success status.".to_string());
    }

    let startup_output = Command::new("systemctl")
        .args([
            "list-unit-files",
            "--type=service",
            "--no-legend",
            "--no-pager",
            "--plain",
        ])
        .output()
        .map_err(|e| format!("Failed to run systemctl list-unit-files: {e}"))?;
    if !startup_output.status.success() {
        return Err("systemctl list-unit-files returned a non-success status.".to_string());
    }

    Ok(parse_unix_services(
        &String::from_utf8_lossy(&status_output.stdout),
        &String::from_utf8_lossy(&startup_output.stdout),
    ))
}

#[cfg(target_os = "windows")]
fn collect_windows_network_adapters() -> Result<Vec<NetworkAdapter>, String> {
    let output = Command::new("ipconfig")
        .args(["/all"])
        .output()
        .map_err(|e| format!("Failed to run ipconfig: {e}"))?;
    if !output.status.success() {
        return Err("ipconfig returned a non-success status.".to_string());
    }

    Ok(parse_windows_ipconfig_all(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

#[cfg(not(target_os = "windows"))]
fn collect_unix_network_adapters() -> Result<Vec<NetworkAdapter>, String> {
    let addr_output = Command::new("ip")
        .args(["-o", "addr", "show", "up"])
        .output()
        .map_err(|e| format!("Failed to run ip addr: {e}"))?;
    if !addr_output.status.success() {
        return Err("ip addr returned a non-success status.".to_string());
    }

    let route_output = Command::new("ip")
        .args(["route", "show", "default"])
        .output()
        .map_err(|e| format!("Failed to run ip route: {e}"))?;
    if !route_output.status.success() {
        return Err("ip route returned a non-success status.".to_string());
    }

    let mut adapters = parse_unix_ip_addr(&String::from_utf8_lossy(&addr_output.stdout));
    apply_unix_default_routes(
        &mut adapters,
        &String::from_utf8_lossy(&route_output.stdout),
    );
    apply_unix_dns_servers(&mut adapters);
    Ok(adapters)
}

#[cfg(target_os = "windows")]
fn collect_windows_processes() -> Result<Vec<ProcessEntry>, String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-Process | Select-Object Name, Id, WorkingSet64, CPU | ConvertTo-Json -Compress",
        ])
        .output()
        .map_err(|e| format!("Failed to run powershell Get-Process: {e}"))?;

    if !output.status.success() {
        return Err("powershell Get-Process returned a non-success status.".to_string());
    }

    let json_text = String::from_utf8_lossy(&output.stdout);
    let values: Value = serde_json::from_str(&json_text)
        .map_err(|e| format!("Failed to parse process JSON: {e}"))?;

    let mut out = Vec::new();
    if let Some(arr) = values.as_array() {
        for v in arr {
            let name = v["Name"].as_str().unwrap_or("unknown").to_string();
            let pid = v["Id"].as_u64().unwrap_or(0) as u32;
            let memory_bytes = v["WorkingSet64"].as_u64().unwrap_or(0);
            let cpu_seconds = v["CPU"].as_f64();
            out.push(ProcessEntry {
                name,
                pid,
                memory_bytes,
                cpu_seconds,
                detail: None,
            });
        }
    } else if let Some(v) = values.as_object() {
        let name = v["Name"].as_str().unwrap_or("unknown").to_string();
        let pid = v["Id"].as_u64().unwrap_or(0) as u32;
        let memory_bytes = v["WorkingSet64"].as_u64().unwrap_or(0);
        let cpu_seconds = v["CPU"].as_f64();
        out.push(ProcessEntry {
            name,
            pid,
            memory_bytes,
            cpu_seconds,
            detail: None,
        });
    }

    Ok(out)
}

#[cfg(not(target_os = "windows"))]
fn collect_unix_processes() -> Result<Vec<ProcessEntry>, String> {
    let output = Command::new("ps")
        .args(["-eo", "pid=,rss=,comm="])
        .output()
        .map_err(|e| format!("Failed to run ps: {e}"))?;
    if !output.status.success() {
        return Err("ps returned a non-success status.".to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut processes = Vec::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 3 {
            continue;
        }
        let (Some(pid), Some(rss_kib)) = (cols[0].parse::<u32>().ok(), cols[1].parse::<u64>().ok())
        else {
            continue;
        };
        processes.push(ProcessEntry {
            name: cols[2..].join(" "),
            pid,
            memory_bytes: rss_kib * 1024,
            cpu_seconds: None,
            detail: None,
        });
    }

    Ok(processes)
}

fn extract_port_from_socket(value: &str) -> Option<u16> {
    let cleaned = value.trim().trim_matches(['[', ']']);
    let port_str = cleaned.rsplit(':').next()?;
    port_str.parse::<u16>().ok()
}

fn listener_exposure_summary(listeners: Vec<ListeningPort>) -> ListenerExposureSummary {
    let mut summary = ListenerExposureSummary::default();
    for entry in listeners {
        let local = entry.local.to_ascii_lowercase();
        if is_loopback_listener(&local) {
            summary.loopback_only += 1;
        } else if is_wildcard_listener(&local) {
            summary.wildcard_public += 1;
        } else {
            summary.specific_bind += 1;
        }
    }
    summary
}

fn service_status_rank(status: &str) -> u8 {
    let lower = status.to_ascii_lowercase();
    if lower == "failed" || lower == "error" {
        0
    } else if lower == "running" || lower == "active" {
        1
    } else if lower == "starting" || lower == "activating" {
        2
    } else {
        3
    }
}

fn is_loopback_listener(local: &str) -> bool {
    local.starts_with("127.")
        || local.starts_with("[::1]")
        || local.starts_with("::1")
        || local.starts_with("localhost:")
}

fn is_wildcard_listener(local: &str) -> bool {
    local.starts_with("0.0.0.0:")
        || local.starts_with("[::]:")
        || local.starts_with(":::")
        || local == "*:*"
}

struct GitState {
    root: PathBuf,
    branch: String,
    dirty_entries: usize,
}

impl GitState {
    fn status_label(&self) -> String {
        if self.dirty_entries == 0 {
            "clean".to_string()
        } else {
            format!("dirty ({} changed path(s))", self.dirty_entries)
        }
    }
}

fn inspect_git_state(path: &Path) -> Option<GitState> {
    let root = capture_first_line(
        "git",
        &["-C", path.to_str()?, "rev-parse", "--show-toplevel"],
    )?;
    let branch = capture_first_line("git", &["-C", path.to_str()?, "branch", "--show-current"])
        .unwrap_or_else(|| "detached".to_string());
    let output = Command::new("git")
        .args(["-C", path.to_str()?, "status", "--short"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let dirty_entries = String::from_utf8_lossy(&output.stdout).lines().count();
    Some(GitState {
        root: PathBuf::from(root),
        branch,
        dirty_entries,
    })
}

struct HematiteState {
    docs_count: usize,
    import_count: usize,
    report_count: usize,
    workspace_profile: bool,
}

fn collect_hematite_state(path: &Path) -> HematiteState {
    let root = path.join(".hematite");
    HematiteState {
        docs_count: count_entries_if_exists(&root.join("docs")),
        import_count: count_entries_if_exists(&root.join("imports")),
        report_count: count_entries_if_exists(&root.join("reports")),
        workspace_profile: root.join("workspace_profile.json").exists(),
    }
}

fn count_entries_if_exists(path: &Path) -> usize {
    if !path.exists() || !path.is_dir() {
        return 0;
    }
    fs::read_dir(path)
        .ok()
        .map(|iter| iter.filter(|entry| entry.is_ok()).count())
        .unwrap_or(0)
}

fn collect_project_markers(path: &Path) -> Vec<String> {
    [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "justfile",
        "Makefile",
        ".git",
    ]
    .iter()
    .filter_map(|name| path.join(name).exists().then(|| (*name).to_string()))
    .collect()
}

struct ReleaseArtifactState {
    version: String,
    portable_dir: bool,
    portable_zip: bool,
    setup_exe: bool,
}

fn inspect_release_artifacts(path: &Path) -> Option<ReleaseArtifactState> {
    let cargo_toml = path.join("Cargo.toml");
    if !cargo_toml.exists() {
        return None;
    }
    let cargo_text = fs::read_to_string(cargo_toml).ok()?;
    let version = [regex_line_capture(
        &cargo_text,
        r#"(?m)^version\s*=\s*"([^"]+)""#,
    )?]
    .concat();
    let dist_windows = path.join("dist").join("windows");
    let prefix = format!("Hematite-{}", version);
    Some(ReleaseArtifactState {
        version,
        portable_dir: dist_windows.join(format!("{}-portable", prefix)).exists(),
        portable_zip: dist_windows
            .join(format!("{}-portable.zip", prefix))
            .exists(),
        setup_exe: dist_windows.join(format!("{}-Setup.exe", prefix)).exists(),
    })
}

fn regex_line_capture(text: &str, pattern: &str) -> Option<String> {
    let regex = regex::Regex::new(pattern).ok()?;
    let captures = regex.captures(text)?;
    captures.get(1).map(|m| m.as_str().to_string())
}

fn bool_label(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn collect_toolchains() -> ToolchainReport {
    let checks = [
        ToolCheck::new("git", &[CommandProbe::new("git", &["--version"])]),
        ToolCheck::new("rustc", &[CommandProbe::new("rustc", &["--version"])]),
        ToolCheck::new("cargo", &[CommandProbe::new("cargo", &["--version"])]),
        ToolCheck::new("node", &[CommandProbe::new("node", &["--version"])]),
        ToolCheck::new(
            "npm",
            &[
                CommandProbe::new("npm", &["--version"]),
                CommandProbe::new("npm.cmd", &["--version"]),
            ],
        ),
        ToolCheck::new(
            "pnpm",
            &[
                CommandProbe::new("pnpm", &["--version"]),
                CommandProbe::new("pnpm.cmd", &["--version"]),
            ],
        ),
        ToolCheck::new(
            "python",
            &[
                CommandProbe::new("python", &["--version"]),
                CommandProbe::new("python3", &["--version"]),
                CommandProbe::new("py", &["-3", "--version"]),
                CommandProbe::new("py", &["--version"]),
            ],
        ),
        ToolCheck::new("deno", &[CommandProbe::new("deno", &["--version"])]),
        ToolCheck::new("go", &[CommandProbe::new("go", &["version"])]),
        ToolCheck::new("dotnet", &[CommandProbe::new("dotnet", &["--version"])]),
        ToolCheck::new("uv", &[CommandProbe::new("uv", &["--version"])]),
    ];

    let mut found = Vec::new();
    let mut missing = Vec::new();

    for check in checks {
        match check.detect() {
            Some(version) => found.push((check.label.to_string(), version)),
            None => missing.push(check.label.to_string()),
        }
    }

    ToolchainReport { found, missing }
}

fn collect_package_managers() -> PackageManagerReport {
    let checks = [
        ToolCheck::new("cargo", &[CommandProbe::new("cargo", &["--version"])]),
        ToolCheck::new(
            "npm",
            &[
                CommandProbe::new("npm", &["--version"]),
                CommandProbe::new("npm.cmd", &["--version"]),
            ],
        ),
        ToolCheck::new(
            "pnpm",
            &[
                CommandProbe::new("pnpm", &["--version"]),
                CommandProbe::new("pnpm.cmd", &["--version"]),
            ],
        ),
        ToolCheck::new(
            "pip",
            &[
                CommandProbe::new("python", &["-m", "pip", "--version"]),
                CommandProbe::new("python3", &["-m", "pip", "--version"]),
                CommandProbe::new("py", &["-3", "-m", "pip", "--version"]),
                CommandProbe::new("py", &["-m", "pip", "--version"]),
                CommandProbe::new("pip", &["--version"]),
            ],
        ),
        ToolCheck::new("pipx", &[CommandProbe::new("pipx", &["--version"])]),
        ToolCheck::new("uv", &[CommandProbe::new("uv", &["--version"])]),
        ToolCheck::new("winget", &[CommandProbe::new("winget", &["--version"])]),
        ToolCheck::new(
            "choco",
            &[
                CommandProbe::new("choco", &["--version"]),
                CommandProbe::new("choco.exe", &["--version"]),
            ],
        ),
        ToolCheck::new("scoop", &[CommandProbe::new("scoop", &["--version"])]),
    ];

    let mut found = Vec::new();
    for check in checks {
        match check.detect() {
            Some(version) => found.push((check.label.to_string(), version)),
            None => {}
        }
    }

    PackageManagerReport { found }
}

#[derive(Clone)]
struct ToolCheck {
    label: &'static str,
    probes: Vec<CommandProbe>,
}

impl ToolCheck {
    fn new(label: &'static str, probes: &[CommandProbe]) -> Self {
        Self {
            label,
            probes: probes.to_vec(),
        }
    }

    fn detect(&self) -> Option<String> {
        for probe in &self.probes {
            if let Some(output) = capture_first_line(probe.program, probe.args) {
                return Some(output);
            }
        }
        None
    }
}

#[derive(Clone, Copy)]
struct CommandProbe {
    program: &'static str,
    args: &'static [&'static str],
}

impl CommandProbe {
    const fn new(program: &'static str, args: &'static [&'static str]) -> Self {
        Self { program, args }
    }
}

fn build_env_doctor_findings(
    toolchains: &ToolchainReport,
    package_managers: &PackageManagerReport,
    path_stats: &PathAnalysis,
) -> Vec<String> {
    let found_tools = toolchains
        .found
        .iter()
        .map(|(label, _)| label.as_str())
        .collect::<HashSet<_>>();
    let found_managers = package_managers
        .found
        .iter()
        .map(|(label, _)| label.as_str())
        .collect::<HashSet<_>>();

    let mut findings = Vec::new();

    if path_stats.duplicate_entries.len() > 0 {
        findings.push(format!(
            "PATH contains {} duplicate entries. That is usually harmless but worth cleaning up.",
            path_stats.duplicate_entries.len()
        ));
    }
    if path_stats.missing_entries.len() > 0 {
        findings.push(format!(
            "PATH contains {} entries that do not exist on disk.",
            path_stats.missing_entries.len()
        ));
    }
    if found_tools.contains("rustc") && !found_managers.contains("cargo") {
        findings.push(
            "Rust is present but Cargo was not detected. That is an incomplete Rust toolchain."
                .to_string(),
        );
    }
    if found_tools.contains("node")
        && !found_managers.contains("npm")
        && !found_managers.contains("pnpm")
    {
        findings.push(
            "Node is present but no JavaScript package manager was detected (npm or pnpm)."
                .to_string(),
        );
    }
    if found_tools.contains("python")
        && !found_managers.contains("pip")
        && !found_managers.contains("uv")
        && !found_managers.contains("pipx")
    {
        findings.push(
            "Python is present but no Python package manager was detected (pip, uv, or pipx)."
                .to_string(),
        );
    }
    let windows_manager_count = ["winget", "choco", "scoop"]
        .iter()
        .filter(|label| found_managers.contains(**label))
        .count();
    if windows_manager_count > 1 {
        findings.push(
            "Multiple Windows package managers are installed. That is workable, but it can create overlap in update paths."
                .to_string(),
        );
    }
    if findings.is_empty() && !found_managers.is_empty() {
        findings.push(
            "Core package-manager coverage looks healthy for a normal developer workstation."
                .to_string(),
        );
    }

    findings
}

fn capture_first_line(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).into_owned()
    } else {
        String::from_utf8_lossy(&output.stdout).into_owned()
    };

    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.to_string())
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_index = 0usize;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}

#[cfg(target_os = "windows")]
fn parse_windows_ipconfig_all(text: &str) -> Vec<NetworkAdapter> {
    let mut adapters = Vec::new();
    let mut current: Option<NetworkAdapter> = None;
    let mut pending_dns = false;

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            pending_dns = false;
            continue;
        }

        if !line.starts_with(' ') && trimmed.ends_with(':') && trimmed.contains("adapter") {
            if let Some(adapter) = current.take() {
                adapters.push(adapter);
            }
            current = Some(NetworkAdapter {
                name: trimmed.trim_end_matches(':').to_string(),
                ..NetworkAdapter::default()
            });
            pending_dns = false;
            continue;
        }

        let Some(adapter) = current.as_mut() else {
            continue;
        };

        if trimmed.contains("Media State") && trimmed.contains("disconnected") {
            adapter.disconnected = true;
        }

        if let Some(value) = value_after_colon(trimmed) {
            let normalized = normalize_ipconfig_value(value);
            if trimmed.starts_with("IPv4 Address") && !normalized.is_empty() {
                adapter.ipv4.push(normalized);
                pending_dns = false;
            } else if trimmed.starts_with("IPv6 Address")
                || trimmed.starts_with("Temporary IPv6 Address")
                || trimmed.starts_with("Link-local IPv6 Address")
            {
                if !normalized.is_empty() {
                    adapter.ipv6.push(normalized);
                }
                pending_dns = false;
            } else if trimmed.starts_with("Default Gateway") {
                if !normalized.is_empty() {
                    adapter.gateways.push(normalized);
                }
                pending_dns = false;
            } else if trimmed.starts_with("DNS Servers") {
                if !normalized.is_empty() {
                    adapter.dns_servers.push(normalized);
                }
                pending_dns = true;
            } else {
                pending_dns = false;
            }
        } else if pending_dns {
            let normalized = normalize_ipconfig_value(trimmed);
            if !normalized.is_empty() {
                adapter.dns_servers.push(normalized);
            }
        }
    }

    if let Some(adapter) = current.take() {
        adapters.push(adapter);
    }

    for adapter in &mut adapters {
        dedup_vec(&mut adapter.ipv4);
        dedup_vec(&mut adapter.ipv6);
        dedup_vec(&mut adapter.gateways);
        dedup_vec(&mut adapter.dns_servers);
    }

    adapters
}

#[cfg(not(target_os = "windows"))]
fn parse_unix_ip_addr(text: &str) -> Vec<NetworkAdapter> {
    let mut adapters = std::collections::BTreeMap::<String, NetworkAdapter>::new();

    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        let name = cols[1].trim_end_matches(':').to_string();
        let family = cols[2];
        let addr = cols[3].split('/').next().unwrap_or("").to_string();
        let entry = adapters
            .entry(name.clone())
            .or_insert_with(|| NetworkAdapter {
                name,
                ..NetworkAdapter::default()
            });
        match family {
            "inet" if !addr.is_empty() => entry.ipv4.push(addr),
            "inet6" if !addr.is_empty() => entry.ipv6.push(addr),
            _ => {}
        }
    }

    adapters.into_values().collect()
}

#[cfg(not(target_os = "windows"))]
fn apply_unix_default_routes(adapters: &mut [NetworkAdapter], text: &str) {
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 {
            continue;
        }
        let gateway = cols
            .windows(2)
            .find(|pair| pair[0] == "via")
            .map(|pair| pair[1].to_string());
        let dev = cols
            .windows(2)
            .find(|pair| pair[0] == "dev")
            .map(|pair| pair[1]);
        if let (Some(gateway), Some(dev)) = (gateway, dev) {
            if let Some(adapter) = adapters.iter_mut().find(|adapter| adapter.name == dev) {
                adapter.gateways.push(gateway);
            }
        }
    }

    for adapter in adapters {
        dedup_vec(&mut adapter.gateways);
    }
}

#[cfg(not(target_os = "windows"))]
fn apply_unix_dns_servers(adapters: &mut [NetworkAdapter]) {
    let Ok(text) = fs::read_to_string("/etc/resolv.conf") else {
        return;
    };
    let mut dns_servers = text
        .lines()
        .filter_map(|line| line.strip_prefix("nameserver "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    dedup_vec(&mut dns_servers);
    if dns_servers.is_empty() {
        return;
    }
    for adapter in adapters.iter_mut().filter(|adapter| adapter.is_active()) {
        adapter.dns_servers = dns_servers.clone();
    }
}


fn value_after_colon(line: &str) -> Option<&str> {
    line.split_once(':').map(|(_, value)| value.trim())
}

fn normalize_ipconfig_value(value: &str) -> String {
    value
        .trim()
        .trim_matches(['(', ')'])
        .trim_end_matches("(Preferred)")
        .trim()
        .to_string()
}

fn dedup_vec(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

#[cfg(target_os = "windows")]
fn parse_windows_services_json(text: &str) -> Result<Vec<ServiceEntry>, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let value: Value = serde_json::from_str(trimmed)
        .map_err(|e| format!("Failed to parse PowerShell service JSON: {e}"))?;
    let entries = match value {
        Value::Array(items) => items,
        other => vec![other],
    };

    let mut services = Vec::new();
    for entry in entries {
        let Some(name) = entry.get("Name").and_then(|v| v.as_str()) else {
            continue;
        };
        services.push(ServiceEntry {
            name: name.to_string(),
            status: entry
                .get("State")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            startup: entry
                .get("StartMode")
                .and_then(|v| v.as_str())
                .map(|value| value.to_string()),
            display_name: entry
                .get("DisplayName")
                .and_then(|v| v.as_str())
                .map(|value| value.to_string()),
        });
    }

    Ok(services)
}

#[cfg(not(target_os = "windows"))]
fn parse_unix_services(status_text: &str, startup_text: &str) -> Vec<ServiceEntry> {
    let mut startup_modes = std::collections::HashMap::<String, String>::new();
    for line in startup_text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 {
            continue;
        }
        startup_modes.insert(cols[0].to_string(), cols[1].to_string());
    }

    let mut services = Vec::new();
    for line in status_text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        let unit = cols[0];
        let load = cols[1];
        let active = cols[2];
        let sub = cols[3];
        let description = if cols.len() > 4 {
            Some(cols[4..].join(" "))
        } else {
            None
        };
        services.push(ServiceEntry {
            name: unit.to_string(),
            status: format!("{}/{}", active, sub),
            startup: startup_modes
                .get(unit)
                .cloned()
                .or_else(|| Some(load.to_string())),
            display_name: description,
        });
    }

    services
}

// ── health_report ─────────────────────────────────────────────────────────────

/// Synthesized system health report — runs multiple checks and returns a
/// plain-English tiered verdict suitable for both developers and non-technical
/// users who just want to know if their machine is okay.
fn inspect_health_report() -> Result<String, String> {
    let mut needs_fix: Vec<String> = Vec::new();
    let mut watch: Vec<String> = Vec::new();
    let mut good: Vec<String> = Vec::new();
    let mut tips: Vec<String> = Vec::new();

    health_check_disk(&mut needs_fix, &mut watch, &mut good);
    health_check_memory(&mut watch, &mut good);
    health_check_tools(&mut watch, &mut good, &mut tips);
    health_check_recent_errors(&mut watch, &mut tips);

    let overall = if !needs_fix.is_empty() {
        "ACTION REQUIRED"
    } else if !watch.is_empty() {
        "WORTH A LOOK"
    } else {
        "ALL GOOD"
    };

    let mut out = format!("System Health Report — {overall}\n\n");

    if !needs_fix.is_empty() {
        out.push_str("Needs fixing:\n");
        for item in &needs_fix {
            out.push_str(&format!("  [!] {item}\n"));
        }
        out.push('\n');
    }
    if !watch.is_empty() {
        out.push_str("Worth watching:\n");
        for item in &watch {
            out.push_str(&format!("  [-] {item}\n"));
        }
        out.push('\n');
    }
    if !good.is_empty() {
        out.push_str("Looking good:\n");
        for item in &good {
            out.push_str(&format!("  [+] {item}\n"));
        }
        out.push('\n');
    }
    if !tips.is_empty() {
        out.push_str("To dig deeper:\n");
        for tip in &tips {
            out.push_str(&format!("  {tip}\n"));
        }
    }

    Ok(out.trim_end().to_string())
}

fn health_check_disk(needs_fix: &mut Vec<String>, watch: &mut Vec<String>, good: &mut Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let script = r#"try {
    $d = Get-PSDrive C -ErrorAction Stop
    "$($d.Free)|$($d.Used)"
} catch { "ERR" }"#;
        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            let text = text.trim();
            if !text.starts_with("ERR") {
                let parts: Vec<&str> = text.split('|').collect();
                if parts.len() == 2 {
                    let free_bytes: u64 = parts[0].trim().parse().unwrap_or(0);
                    let used_bytes: u64 = parts[1].trim().parse().unwrap_or(0);
                    let total = free_bytes + used_bytes;
                    let free_gb = free_bytes / 1_073_741_824;
                    let pct_free = if total > 0 {
                        (free_bytes as f64 / total as f64 * 100.0) as u64
                    } else {
                        0
                    };
                    let msg = format!("Disk: {free_gb} GB free on C: ({pct_free}% available)");
                    if free_gb < 5 {
                        needs_fix.push(format!(
                            "{msg} — very low. Free up space or your system may slow down or stop working."
                        ));
                    } else if free_gb < 15 {
                        watch.push(format!("{msg} — getting low, consider cleaning up."));
                    } else {
                        good.push(msg);
                    }
                    return;
                }
            }
        }
        watch.push("Disk: could not read free space from C: drive.".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(out) = Command::new("df").args(["-BG", "/"]).output() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines().skip(1) {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() >= 5 {
                    let avail_str = cols[3].trim_end_matches('G');
                    let use_pct = cols[4].trim_end_matches('%');
                    let avail_gb: u64 = avail_str.parse().unwrap_or(0);
                    let used_pct: u64 = use_pct.parse().unwrap_or(0);
                    let msg = format!("Disk: {avail_gb} GB free on / ({used_pct}% used)");
                    if avail_gb < 5 {
                        needs_fix.push(format!(
                            "{msg} — very low. Free up space to prevent system issues."
                        ));
                    } else if avail_gb < 15 {
                        watch.push(format!("{msg} — getting low."));
                    } else {
                        good.push(msg);
                    }
                    return;
                }
            }
        }
        watch.push("Disk: could not determine free space.".to_string());
    }
}

fn health_check_memory(watch: &mut Vec<String>, good: &mut Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let script = r#"try {
    $os = Get-CimInstance Win32_OperatingSystem -ErrorAction Stop
    "$($os.FreePhysicalMemory)|$($os.TotalVisibleMemorySize)"
} catch { "ERR" }"#;
        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            let text = text.trim();
            if !text.starts_with("ERR") {
                let parts: Vec<&str> = text.split('|').collect();
                if parts.len() == 2 {
                    let free_kb: u64 = parts[0].trim().parse().unwrap_or(0);
                    let total_kb: u64 = parts[1].trim().parse().unwrap_or(0);
                    if total_kb > 0 {
                        let free_gb = free_kb / 1_048_576;
                        let total_gb = total_kb / 1_048_576;
                        let free_pct = free_kb * 100 / total_kb;
                        let msg = format!(
                            "RAM: {free_gb} GB free of {total_gb} GB ({free_pct}% available)"
                        );
                        if free_pct < 10 {
                            watch.push(format!(
                                "{msg} — very low. Close unused apps to free up memory."
                            ));
                        } else if free_pct < 25 {
                            watch.push(format!("{msg} — running a bit low."));
                        } else {
                            good.push(msg);
                        }
                        return;
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            let mut total_kb = 0u64;
            let mut avail_kb = 0u64;
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    total_kb = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                } else if line.starts_with("MemAvailable:") {
                    avail_kb = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                }
            }
            if total_kb > 0 {
                let free_gb = avail_kb / 1_048_576;
                let total_gb = total_kb / 1_048_576;
                let free_pct = avail_kb * 100 / total_kb;
                let msg =
                    format!("RAM: {free_gb} GB free of {total_gb} GB ({free_pct}% available)");
                if free_pct < 10 {
                    watch.push(format!("{msg} — very low. Close unused apps."));
                } else if free_pct < 25 {
                    watch.push(format!("{msg} — running a bit low."));
                } else {
                    good.push(msg);
                }
            }
        }
    }
}

fn health_check_tools(watch: &mut Vec<String>, good: &mut Vec<String>, tips: &mut Vec<String>) {
    let tool_checks: &[(&str, &str, &str)] = &[
        ("git", "--version", "Git"),
        ("cargo", "--version", "Rust / Cargo"),
        ("node", "--version", "Node.js"),
        ("python", "--version", "Python"),
        ("python3", "--version", "Python 3"),
        ("npm", "--version", "npm"),
    ];

    let mut found: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut python_found = false;

    for (cmd, arg, label) in tool_checks {
        if cmd.starts_with("python") && python_found {
            continue;
        }
        let ok = Command::new(cmd)
            .arg(arg)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            found.push((*label).to_string());
            if cmd.starts_with("python") {
                python_found = true;
            }
        } else if !cmd.starts_with("python") || !python_found {
            missing.push((*label).to_string());
        }
    }

    if !found.is_empty() {
        good.push(format!("Dev tools found: {}", found.join(", ")));
    }
    if !missing.is_empty() {
        watch.push(format!(
            "Not installed (or not on PATH): {} — only matters if you need them",
            missing.join(", ")
        ));
        tips.push(
            "Run inspect_host(topic=\"toolchains\") for exact version details on all dev tools."
                .to_string(),
        );
    }
}

fn health_check_recent_errors(watch: &mut Vec<String>, tips: &mut Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let script = r#"try {
    $cutoff = (Get-Date).AddHours(-24)
    $count = (Get-WinEvent -FilterHashtable @{LogName='Application','System'; Level=1,2,3; StartTime=$cutoff} -MaxEvents 200 -ErrorAction SilentlyContinue | Measure-Object).Count
    $count
} catch { "0" }"#;
        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            let count: u64 = text.trim().parse().unwrap_or(0);
            if count > 0 {
                watch.push(format!(
                    "{count} critical/error event{} in Windows event logs in the last 24 hours.",
                    if count == 1 { "" } else { "s" }
                ));
                tips.push(
                    "Run inspect_host(topic=\"log_check\") to see the actual error messages."
                        .to_string(),
                );
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(out) = Command::new("journalctl")
            .args(["-p", "3", "-n", "1", "--no-pager", "--quiet"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if !text.trim().is_empty() {
                watch.push("Critical/error entries found in the system journal.".to_string());
                tips.push(
                    "Run inspect_host(topic=\"log_check\") to see recent errors.".to_string(),
                );
            }
        }
    }
}

// ── log_check ─────────────────────────────────────────────────────────────────

fn inspect_log_check(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: log_check\n\n");

    #[cfg(target_os = "windows")]
    {
        // Pull recent critical/error events from Windows Application and System logs.
        let n = max_entries.clamp(1, 50);
        let script = format!(
            r#"try {{
    $events = Get-WinEvent -FilterHashtable @{{LogName='Application','System'; Level=1,2,3}} -MaxEvents 100 -ErrorAction SilentlyContinue
    if (-not $events) {{ "NO_EVENTS"; exit }}
    $events | Select-Object -First {n} | ForEach-Object {{
        $line = $_.TimeCreated.ToString('yyyy-MM-dd HH:mm:ss') + '|' + $_.LevelDisplayName + '|' + $_.ProviderName + '|' + (($_.Message -split '[\r\n]')[0].Trim())
        $line
    }}
}} catch {{ "ERROR:" + $_.Exception.Message }}"#,
            n = n
        );
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| format!("log_check: failed to run PowerShell: {e}"))?;

        let raw = String::from_utf8_lossy(&output.stdout);
        let text = raw.trim();

        if text.is_empty() || text == "NO_EVENTS" {
            out.push_str("No critical or error events found in Application/System logs.\n");
            return Ok(out.trim_end().to_string());
        }
        if text.starts_with("ERROR:") {
            out.push_str(&format!("Warning: event log query returned: {text}\n"));
            return Ok(out.trim_end().to_string());
        }

        let mut count = 0usize;
        for line in text.lines() {
            let parts: Vec<&str> = line.splitn(4, '|').collect();
            if parts.len() == 4 {
                let (time, level, source, msg) = (parts[0], parts[1], parts[2], parts[3]);
                out.push_str(&format!("[{time}] [{level}] {source}: {msg}\n"));
                count += 1;
            }
        }
        out.push_str(&format!(
            "\nEvents shown: {count} (critical/error from Application + System logs)\n"
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Use journalctl on Linux/macOS if available.
        let n = max_entries.clamp(1, 50).to_string();
        let output = Command::new("journalctl")
            .args(["-p", "3", "-n", &n, "--no-pager", "--output=short-precise"])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                let trimmed = text.trim();
                if trimmed.is_empty() || trimmed.contains("No entries") {
                    out.push_str("No critical or error entries found in the system journal.\n");
                } else {
                    out.push_str(trimmed);
                    out.push('\n');
                    out.push_str("\n(source: journalctl -p 3 = critical/alert/emergency/error)\n");
                }
            }
            _ => {
                // Fallback: check /var/log/syslog or /var/log/messages
                let log_paths = ["/var/log/syslog", "/var/log/messages"];
                let mut found = false;
                for log_path in &log_paths {
                    if let Ok(content) = std::fs::read_to_string(log_path) {
                        let lines: Vec<&str> = content.lines().collect();
                        let tail: Vec<&str> = lines
                            .iter()
                            .rev()
                            .filter(|l| {
                                let l_lower = l.to_ascii_lowercase();
                                l_lower.contains("error") || l_lower.contains("crit")
                            })
                            .take(max_entries)
                            .copied()
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect();
                        if !tail.is_empty() {
                            out.push_str(&format!("Source: {log_path}\n"));
                            for l in &tail {
                                out.push_str(l);
                                out.push('\n');
                            }
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    out.push_str(
                        "journalctl not found and no readable syslog detected on this system.\n",
                    );
                }
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── startup_items ─────────────────────────────────────────────────────────────

fn inspect_startup_items(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: startup_items\n\n");

    #[cfg(target_os = "windows")]
    {
        // Query both HKLM and HKCU Run keys.
        let script = r#"
$hives = @(
    @{Hive='HKLM'; Path='HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run'},
    @{Hive='HKCU'; Path='HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run'},
    @{Hive='HKLM (32-bit)'; Path='HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run'}
)
foreach ($h in $hives) {
    try {
        $props = Get-ItemProperty -Path $h.Path -ErrorAction Stop
        $props.PSObject.Properties | Where-Object { $_.Name -notlike 'PS*' } | ForEach-Object {
            "$($h.Hive)|$($_.Name)|$($_.Value)"
        }
    } catch {}
}
"#;
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .map_err(|e| format!("startup_items: failed to run PowerShell: {e}"))?;

        let raw = String::from_utf8_lossy(&output.stdout);
        let text = raw.trim();

        let entries: Vec<(String, String, String)> = text
            .lines()
            .filter_map(|l| {
                let parts: Vec<&str> = l.splitn(3, '|').collect();
                if parts.len() == 3 {
                    Some((
                        parts[0].to_string(),
                        parts[1].to_string(),
                        parts[2].to_string(),
                    ))
                } else {
                    None
                }
            })
            .take(max_entries)
            .collect();

        if entries.is_empty() {
            out.push_str("No startup entries found in the Windows Run registry keys.\n");
        } else {
            out.push_str("Registry run keys (programs that start with Windows):\n\n");
            let mut last_hive = String::new();
            for (hive, name, value) in &entries {
                if *hive != last_hive {
                    out.push_str(&format!("[{}]\n", hive));
                    last_hive = hive.clone();
                }
                // Truncate very long values (paths with many args)
                let display = if value.len() > 100 {
                    format!("{}…", &value[..100])
                } else {
                    value.clone()
                };
                out.push_str(&format!("  {name}: {display}\n"));
            }
            out.push_str(&format!("\nTotal startup entries: {}\n", entries.len()));
        }

        // Also show Startup folder items.
        let startup_script = r#"
$paths = @(
    [System.Environment]::GetFolderPath('Startup'),
    [System.Environment]::GetFolderPath('CommonStartup')
)
foreach ($p in $paths) {
    if (Test-Path $p) {
        $items = Get-ChildItem $p -File -ErrorAction SilentlyContinue
        if ($items) {
            "$p"
            $items | ForEach-Object { "  " + $_.Name }
        }
    }
}
"#;
        if let Ok(folder_out) = Command::new("powershell")
            .args(["-NoProfile", "-Command", startup_script])
            .output()
        {
            let folder_text = String::from_utf8_lossy(&folder_out.stdout);
            let trimmed = folder_text.trim();
            if !trimmed.is_empty() {
                out.push_str("\nStartup folders:\n");
                out.push_str(trimmed);
                out.push('\n');
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Linux: systemd enabled services + cron @reboot entries.
        let output = Command::new("systemctl")
            .args([
                "list-unit-files",
                "--type=service",
                "--state=enabled",
                "--no-legend",
                "--no-pager",
                "--plain",
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                let services: Vec<&str> = text
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .take(max_entries)
                    .collect();
                if services.is_empty() {
                    out.push_str("No enabled systemd services found.\n");
                } else {
                    out.push_str("Enabled systemd services (run at boot):\n\n");
                    for s in &services {
                        out.push_str(&format!("  {s}\n"));
                    }
                    out.push_str(&format!(
                        "\nShowing {} of enabled services.\n",
                        services.len()
                    ));
                }
            }
            _ => {
                out.push_str(
                    "systemctl not found on this system. Cannot enumerate startup services.\n",
                );
            }
        }

        // Check @reboot cron entries.
        if let Ok(cron_out) = Command::new("crontab").args(["-l"]).output() {
            let cron_text = String::from_utf8_lossy(&cron_out.stdout);
            let reboot_entries: Vec<&str> = cron_text
                .lines()
                .filter(|l| l.trim_start().starts_with("@reboot"))
                .collect();
            if !reboot_entries.is_empty() {
                out.push_str("\nCron @reboot entries:\n");
                for e in reboot_entries {
                    out.push_str(&format!("  {e}\n"));
                }
            }
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_os_config() -> Result<String, String> {
    let mut out = String::from("Host inspection: OS Configuration\n\n");

    #[cfg(target_os = "windows")]
    {
        // Power Plan
        if let Ok(power_out) = Command::new("powercfg").args(["/getactivescheme"]).output() {
            let power_str = String::from_utf8_lossy(&power_out.stdout);
            out.push_str("=== Power Plan ===\n");
            out.push_str(power_str.trim());
            out.push_str("\n\n");
        }

        // Firewall Status
        let fw_script = "Get-NetFirewallProfile | Format-Table -Property Name, Enabled -AutoSize | Out-String";
        if let Ok(fw_out) = Command::new("powershell").args(["-NoProfile", "-Command", fw_script]).output() {
            let fw_str = String::from_utf8_lossy(&fw_out.stdout);
            out.push_str("=== Firewall Profiles ===\n");
            out.push_str(fw_str.trim());
            out.push_str("\n\n");
        }

        // System Uptime
        let uptime_script = "(Get-CimInstance -ClassName Win32_OperatingSystem).LastBootUpTime.ToString()";
        if let Ok(uptime_out) = Command::new("powershell").args(["-NoProfile", "-Command", uptime_script]).output() {
            let uptime_str = String::from_utf8_lossy(&uptime_out.stdout);
            out.push_str("=== System Uptime (Last Boot) ===\n");
            out.push_str(uptime_str.trim());
            out.push_str("\n\n");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Uptime
        if let Ok(uptime_out) = Command::new("uptime").args(["-p"]).output() {
            let uptime_str = String::from_utf8_lossy(&uptime_out.stdout);
            out.push_str("=== System Uptime ===\n");
            out.push_str(uptime_str.trim());
            out.push_str("\n\n");
        }
        
        // Firewall (ufw status if available)
        if let Ok(ufw_out) = Command::new("ufw").arg("status").output() {
            let ufw_str = String::from_utf8_lossy(&ufw_out.stdout);
            if !ufw_str.trim().is_empty() {
                out.push_str("=== Firewall (UFW) ===\n");
                out.push_str(ufw_str.trim());
                out.push_str("\n\n");
            }
        }
    }
    Ok(out.trim_end().to_string())
}

pub async fn resolve_host_issue(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: 'action'".to_string())?;

    let target = args
        .get("target")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    if target.is_empty() && action != "clear_temp" {
        return Err("Missing required argument: 'target' for this action".to_string());
    }

    match action {
        "install_package" => {
            #[cfg(target_os = "windows")]
            {
                let cmd = format!("winget install --id {} -e --accept-package-agreements --accept-source-agreements", target);
                match Command::new("powershell").args(["-NoProfile", "-Command", &cmd]).output() {
                    Ok(out) => Ok(format!("Executed remediation (winget install):\n{}", String::from_utf8_lossy(&out.stdout))),
                    Err(e) => Err(format!("Failed to run winget: {}", e)),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err("install_package via wrapper is only supported on Windows currently (winget)".to_string())
            }
        }
        "restart_service" => {
            #[cfg(target_os = "windows")]
            {
                let cmd = format!("Restart-Service -Name {} -Force", target);
                match Command::new("powershell").args(["-NoProfile", "-Command", &cmd]).output() {
                    Ok(out) => {
                        let err_str = String::from_utf8_lossy(&out.stderr);
                        if !err_str.is_empty() {
                            return Err(format!("Error restarting service:\n{}", err_str));
                        }
                        Ok(format!("Successfully restarted service: {}", target))
                    }
                    Err(e) => Err(format!("Failed to restart service: {}", e)),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err("restart_service via wrapper is only supported on Windows currently".to_string())
            }
        }
        "clear_temp" => {
            #[cfg(target_os = "windows")]
            {
                let cmd = "Remove-Item -Path \"$env:TEMP\\*\" -Recurse -Force -ErrorAction SilentlyContinue";
                match Command::new("powershell").args(["-NoProfile", "-Command", cmd]).output() {
                    Ok(_) => Ok("Successfully cleared temporary files".to_string()),
                    Err(e) => Err(format!("Failed to clear temp: {}", e)),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err("clear_temp via wrapper is only supported on Windows currently".to_string())
            }
        }
        other => Err(format!("Unknown remediation action: {}", other)),
    }
}

// ── storage ───────────────────────────────────────────────────────────────────

fn inspect_storage(_max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: storage\n\n");

    // ── Drive overview ────────────────────────────────────────────────────────
    out.push_str("Drives:\n");

    #[cfg(target_os = "windows")]
    {
        let script = r#"Get-PSDrive -PSProvider 'FileSystem' | ForEach-Object {
    $free = $_.Free
    $used = $_.Used
    if ($free -eq $null) { $free = 0 }
    if ($used -eq $null) { $used = 0 }
    $total = $free + $used
    "$($_.Name)|$free|$used|$total"
}"#;
        match Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
        {
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                let mut drive_count = 0usize;
                for line in text.lines() {
                    let parts: Vec<&str> = line.trim().split('|').collect();
                    if parts.len() == 4 {
                        let name = parts[0];
                        let free: u64 = parts[1].parse().unwrap_or(0);
                        let total: u64 = parts[3].parse().unwrap_or(0);
                        if total == 0 {
                            continue;
                        }
                        let free_gb = free / 1_073_741_824;
                        let total_gb = total / 1_073_741_824;
                        let used_pct = ((total - free) as f64 / total as f64 * 100.0) as u64;
                        let bar_len = 20usize;
                        let filled = (used_pct as usize * bar_len / 100).min(bar_len);
                        let bar: String = "#".repeat(filled) + &".".repeat(bar_len - filled);
                        let warn = if free_gb < 5 {
                            " [!] CRITICALLY LOW"
                        } else if free_gb < 15 {
                            " [-] LOW"
                        } else {
                            ""
                        };
                        out.push_str(&format!(
                            "  {name}:  [{bar}] {used_pct}% used — {free_gb} GB free of {total_gb} GB{warn}\n"
                        ));
                        drive_count += 1;
                    }
                }
                if drive_count == 0 {
                    out.push_str("  (could not enumerate drives)\n");
                }
            }
            Err(e) => out.push_str(&format!("  (drive scan failed: {e})\n")),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        match Command::new("df").args(["-h", "--output=target,size,avail,pcent"]).output() {
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                let mut count = 0usize;
                for line in text.lines().skip(1) {
                    let cols: Vec<&str> = line.split_whitespace().collect();
                    if cols.len() >= 4 && !cols[0].starts_with("tmpfs") {
                        out.push_str(&format!(
                            "  {}  size: {}  avail: {}  used: {}\n",
                            cols[0], cols[1], cols[2], cols[3]
                        ));
                        count += 1;
                        if count >= max_entries {
                            break;
                        }
                    }
                }
            }
            Err(e) => out.push_str(&format!("  (df failed: {e})\n")),
        }
    }

    // ── Large developer cache directories ─────────────────────────────────────
    out.push_str("\nLarge developer cache directories (if present):\n");

    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        let check_dirs: &[(&str, &str)] = &[
            ("Temp", r"AppData\Local\Temp"),
            ("npm cache", r"AppData\Roaming\npm-cache"),
            ("Cargo registry", r".cargo\registry"),
            ("Cargo git", r".cargo\git"),
            ("pip cache", r"AppData\Local\pip\cache"),
            ("Yarn cache", r"AppData\Local\Yarn\Cache"),
            (".rustup toolchains", r".rustup\toolchains"),
            ("node_modules (home)", r"node_modules"),
        ];

        let mut found_any = false;
        for (label, rel) in check_dirs {
            let full = format!(r"{}\{}", home, rel);
            let path = std::path::Path::new(&full);
            if path.exists() {
                // Quick size estimate via PowerShell (non-blocking cap at 5s)
                let size_script = format!(
                    r#"try {{ $s = (Get-ChildItem -Path '{}' -Recurse -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum; [math]::Round($s/1MB,0) }} catch {{ '?' }}"#,
                    full.replace('\'', "''")
                );
                let size_mb = Command::new("powershell")
                    .args(["-NoProfile", "-Command", &size_script])
                    .output()
                    .ok()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_else(|| "?".to_string());
                out.push_str(&format!("  {label}: {size_mb} MB  ({full})\n"));
                found_any = true;
            }
        }
        if !found_any {
            out.push_str("  (none of the common cache directories found)\n");
        }

        out.push_str("\nTip: to reclaim space, run inspect_host(topic=\"fix_plan\", issue=\"free up disk space\")\n");
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        let check_dirs: &[(&str, &str)] = &[
            ("npm cache", ".npm"),
            ("Cargo registry", ".cargo/registry"),
            ("pip cache", ".cache/pip"),
            (".rustup toolchains", ".rustup/toolchains"),
            ("Yarn cache", ".cache/yarn"),
        ];
        let mut found_any = false;
        for (label, rel) in check_dirs {
            let full = format!("{}/{}", home, rel);
            if std::path::Path::new(&full).exists() {
                let size = Command::new("du")
                    .args(["-sh", &full])
                    .output()
                    .ok()
                    .map(|o| {
                        let s = String::from_utf8_lossy(&o.stdout);
                        s.split_whitespace().next().unwrap_or("?").to_string()
                    })
                    .unwrap_or_else(|| "?".to_string());
                out.push_str(&format!("  {label}: {size}  ({full})\n"));
                found_any = true;
            }
        }
        if !found_any {
            out.push_str("  (none of the common cache directories found)\n");
        }
    }

    Ok(out.trim_end().to_string())
}

// ── hardware ──────────────────────────────────────────────────────────────────

fn inspect_hardware() -> Result<String, String> {
    let mut out = String::from("Host inspection: hardware\n\n");

    #[cfg(target_os = "windows")]
    {
        // CPU
        let cpu_script = r#"Get-CimInstance Win32_Processor | ForEach-Object {
    "$($_.Name.Trim())|$($_.NumberOfCores)|$($_.NumberOfLogicalProcessors)|$([math]::Round($_.MaxClockSpeed/1000,1))"
} | Select-Object -First 1"#;
        if let Ok(o) = Command::new("powershell").args(["-NoProfile", "-Command", cpu_script]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            let text = text.trim();
            let parts: Vec<&str> = text.split('|').collect();
            if parts.len() == 4 {
                out.push_str(&format!(
                    "CPU: {}\n  {} physical cores, {} logical processors, {:.1} GHz\n\n",
                    parts[0], parts[1], parts[2], parts[3].parse::<f32>().unwrap_or(0.0)
                ));
            } else {
                out.push_str(&format!("CPU: {text}\n\n"));
            }
        }

        // RAM (total installed + speed)
        let ram_script = r#"$sticks = Get-CimInstance Win32_PhysicalMemory
$total = ($sticks | Measure-Object Capacity -Sum).Sum / 1GB
$speed = ($sticks | Select-Object -First 1).Speed
"$([math]::Round($total,0)) GB @ $($speed) MHz ($($sticks.Count) stick(s))""#;
        if let Ok(o) = Command::new("powershell").args(["-NoProfile", "-Command", ram_script]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            out.push_str(&format!("RAM: {}\n\n", text.trim().trim_matches('"')));
        }

        // GPU(s)
        let gpu_script = r#"Get-CimInstance Win32_VideoController | ForEach-Object {
    "$($_.Name)|$($_.DriverVersion)|$($_.CurrentHorizontalResolution)x$($_.CurrentVerticalResolution)"
}"#;
        if let Ok(o) = Command::new("powershell").args(["-NoProfile", "-Command", gpu_script]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = text.lines().collect();
            if !lines.is_empty() {
                out.push_str("GPU(s):\n");
                for line in lines.iter().filter(|l| !l.trim().is_empty()) {
                    let parts: Vec<&str> = line.trim().split('|').collect();
                    if parts.len() == 3 {
                        let res = if parts[2] == "x" || parts[2].starts_with('0') {
                            String::new()
                        } else {
                            format!(" — {}@display", parts[2])
                        };
                        out.push_str(&format!("  {}\n    Driver: {}{}\n", parts[0], parts[1], res));
                    } else {
                        out.push_str(&format!("  {}\n", line.trim()));
                    }
                }
                out.push('\n');
            }
        }

        // Motherboard + BIOS
        let mb_script = r#"$mb = Get-CimInstance Win32_BaseBoard
$bios = Get-CimInstance Win32_BIOS
"$($mb.Manufacturer.Trim()) $($mb.Product.Trim())|BIOS: $($bios.Manufacturer.Trim()) $($bios.SMBIOSBIOSVersion.Trim()) ($($bios.ReleaseDate))""#;
        if let Ok(o) = Command::new("powershell").args(["-NoProfile", "-Command", mb_script]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            let text = text.trim().trim_matches('"');
            let parts: Vec<&str> = text.split('|').collect();
            if parts.len() == 2 {
                out.push_str(&format!("Motherboard: {}\n{}\n\n", parts[0].trim(), parts[1].trim()));
            }
        }

        // Display(s)
        let disp_script = r#"Get-CimInstance Win32_DesktopMonitor | Where-Object {$_.ScreenWidth -gt 0} | ForEach-Object {
    "$($_.Name)|$($_.ScreenWidth)x$($_.ScreenHeight)"
}"#;
        if let Ok(o) = Command::new("powershell").args(["-NoProfile", "-Command", disp_script]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
            if !lines.is_empty() {
                out.push_str("Display(s):\n");
                for line in &lines {
                    let parts: Vec<&str> = line.trim().split('|').collect();
                    if parts.len() == 2 {
                        out.push_str(&format!("  {} — {}\n", parts[0].trim(), parts[1]));
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // CPU via /proc/cpuinfo
        if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
            let model = content.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(str::trim)
                .unwrap_or("unknown");
            let cores = content.lines().filter(|l| l.starts_with("processor")).count();
            out.push_str(&format!("CPU: {model}\n  {cores} logical processors\n\n"));
        }

        // RAM
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            let total_kb: u64 = content.lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let total_gb = total_kb / 1_048_576;
            out.push_str(&format!("RAM: {total_gb} GB total\n\n"));
        }

        // GPU via lspci
        if let Ok(o) = Command::new("lspci").args(["-vmm"]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            let gpu_lines: Vec<&str> = text.lines()
                .filter(|l| l.contains("VGA") || l.contains("Display") || l.contains("3D"))
                .collect();
            if !gpu_lines.is_empty() {
                out.push_str("GPU(s):\n");
                for l in gpu_lines {
                    out.push_str(&format!("  {l}\n"));
                }
                out.push('\n');
            }
        }

        // DMI/BIOS info
        if let Ok(o) = Command::new("dmidecode").args(["-t", "baseboard", "-t", "bios"]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            out.push_str("Motherboard/BIOS:\n");
            for line in text.lines().filter(|l| {
                l.contains("Manufacturer:") || l.contains("Product Name:") || l.contains("Version:")
            }).take(6) {
                out.push_str(&format!("  {}\n", line.trim()));
            }
        }
    }

    Ok(out.trim_end().to_string())
}
