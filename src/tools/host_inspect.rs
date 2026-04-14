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
        "updates" | "windows_update" => inspect_updates(),
        "security" | "antivirus" | "defender" => inspect_security(),
        "pending_reboot" | "reboot_required" => inspect_pending_reboot(),
        "disk_health" | "smart" | "drive_health" => inspect_disk_health(),
        "battery" => inspect_battery(),
        "recent_crashes" | "crashes" | "bsod" => inspect_recent_crashes(max_entries),
        "scheduled_tasks" | "tasks" => inspect_scheduled_tasks(max_entries),
        "dev_conflicts" | "dev_environment" => inspect_dev_conflicts(),
        "connectivity" | "internet" | "internet_check" => inspect_connectivity(),
        "wifi" | "wi-fi" | "wireless" | "wlan" => inspect_wifi(),
        "connections" | "tcp_connections" | "active_connections" => inspect_connections(max_entries),
        "vpn" => inspect_vpn(),
        "proxy" | "proxy_settings" => inspect_proxy(),
        "firewall_rules" | "firewall-rules" => inspect_firewall_rules(max_entries),
        "traceroute" | "tracert" | "trace_route" | "trace" => {
            let host = args
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("8.8.8.8")
                .to_string();
            inspect_traceroute(&host, max_entries)
        }
        "dns_cache" | "dnscache" | "dns-cache" => inspect_dns_cache(max_entries),
        "arp" | "arp_table" => inspect_arp(),
        "route_table" | "routes" | "routing_table" => inspect_route_table(max_entries),
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
            "Unknown inspect_host topic '{}'. Use one of: summary, toolchains, path, env_doctor, fix_plan, network, services, processes, desktop, downloads, directory, disk, ports, repo_doctor, log_check, startup_items, health_report, storage, hardware, updates, security, pending_reboot, disk_health, battery, recent_crashes, scheduled_tasks, dev_conflicts, connectivity, wifi, connections, vpn, proxy, firewall_rules, os_config, resource_load.",
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

        let cpu_load = lines
            .next()
            .and_then(|l| l.parse::<u32>().ok())
            .unwrap_or(0);
        let mem_json = lines.collect::<Vec<_>>().join("");
        let mem_val: Value = serde_json::from_str(&mem_json).unwrap_or(Value::Null);

        let total_kb = mem_val["TotalVisibleMemorySize"].as_u64().unwrap_or(1);
        let free_kb = mem_val["FreePhysicalMemory"].as_u64().unwrap_or(0);
        let used_kb = total_kb.saturating_sub(free_kb);
        let mem_percent = if total_kb > 0 {
            (used_kb * 100) / total_kb
        } else {
            0
        };

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
        let fw_script =
            "Get-NetFirewallProfile | Format-Table -Property Name, Enabled -AutoSize | Out-String";
        if let Ok(fw_out) = Command::new("powershell")
            .args(["-NoProfile", "-Command", fw_script])
            .output()
        {
            let fw_str = String::from_utf8_lossy(&fw_out.stdout);
            out.push_str("=== Firewall Profiles ===\n");
            out.push_str(fw_str.trim());
            out.push_str("\n\n");
        }

        // System Uptime
        let uptime_script =
            "(Get-CimInstance -ClassName Win32_OperatingSystem).LastBootUpTime.ToString()";
        if let Ok(uptime_out) = Command::new("powershell")
            .args(["-NoProfile", "-Command", uptime_script])
            .output()
        {
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
                match Command::new("powershell")
                    .args(["-NoProfile", "-Command", &cmd])
                    .output()
                {
                    Ok(out) => Ok(format!(
                        "Executed remediation (winget install):\n{}",
                        String::from_utf8_lossy(&out.stdout)
                    )),
                    Err(e) => Err(format!("Failed to run winget: {}", e)),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err(
                    "install_package via wrapper is only supported on Windows currently (winget)"
                        .to_string(),
                )
            }
        }
        "restart_service" => {
            #[cfg(target_os = "windows")]
            {
                let cmd = format!("Restart-Service -Name {} -Force", target);
                match Command::new("powershell")
                    .args(["-NoProfile", "-Command", &cmd])
                    .output()
                {
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
                Err(
                    "restart_service via wrapper is only supported on Windows currently"
                        .to_string(),
                )
            }
        }
        "clear_temp" => {
            #[cfg(target_os = "windows")]
            {
                let cmd = "Remove-Item -Path \"$env:TEMP\\*\" -Recurse -Force -ErrorAction SilentlyContinue";
                match Command::new("powershell")
                    .args(["-NoProfile", "-Command", cmd])
                    .output()
                {
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

fn inspect_storage(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: storage\n\n");
    let _ = max_entries; // used by non-Windows branch

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
        match Command::new("df")
            .args(["-h", "--output=target,size,avail,pcent"])
            .output()
        {
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
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", cpu_script])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            let text = text.trim();
            let parts: Vec<&str> = text.split('|').collect();
            if parts.len() == 4 {
                out.push_str(&format!(
                    "CPU: {}\n  {} physical cores, {} logical processors, {:.1} GHz\n\n",
                    parts[0],
                    parts[1],
                    parts[2],
                    parts[3].parse::<f32>().unwrap_or(0.0)
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
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", ram_script])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            out.push_str(&format!("RAM: {}\n\n", text.trim().trim_matches('"')));
        }

        // GPU(s)
        let gpu_script = r#"Get-CimInstance Win32_VideoController | ForEach-Object {
    "$($_.Name)|$($_.DriverVersion)|$($_.CurrentHorizontalResolution)x$($_.CurrentVerticalResolution)"
}"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", gpu_script])
            .output()
        {
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
                        out.push_str(&format!(
                            "  {}\n    Driver: {}{}\n",
                            parts[0], parts[1], res
                        ));
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
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", mb_script])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            let text = text.trim().trim_matches('"');
            let parts: Vec<&str> = text.split('|').collect();
            if parts.len() == 2 {
                out.push_str(&format!(
                    "Motherboard: {}\n{}\n\n",
                    parts[0].trim(),
                    parts[1].trim()
                ));
            }
        }

        // Display(s)
        let disp_script = r#"Get-CimInstance Win32_DesktopMonitor | Where-Object {$_.ScreenWidth -gt 0} | ForEach-Object {
    "$($_.Name)|$($_.ScreenWidth)x$($_.ScreenHeight)"
}"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", disp_script])
            .output()
        {
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
            let model = content
                .lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(str::trim)
                .unwrap_or("unknown");
            let cores = content
                .lines()
                .filter(|l| l.starts_with("processor"))
                .count();
            out.push_str(&format!("CPU: {model}\n  {cores} logical processors\n\n"));
        }

        // RAM
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            let total_kb: u64 = content
                .lines()
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
            let gpu_lines: Vec<&str> = text
                .lines()
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
        if let Ok(o) = Command::new("dmidecode")
            .args(["-t", "baseboard", "-t", "bios"])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            out.push_str("Motherboard/BIOS:\n");
            for line in text
                .lines()
                .filter(|l| {
                    l.contains("Manufacturer:")
                        || l.contains("Product Name:")
                        || l.contains("Version:")
                })
                .take(6)
            {
                out.push_str(&format!("  {}\n", line.trim()));
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── updates ───────────────────────────────────────────────────────────────────

fn inspect_updates() -> Result<String, String> {
    let mut out = String::from("Host inspection: updates\n\n");

    #[cfg(target_os = "windows")]
    {
        // Last installed update via COM
        let script = r#"
try {
    $sess = New-Object -ComObject Microsoft.Update.Session
    $searcher = $sess.CreateUpdateSearcher()
    $count = $searcher.GetTotalHistoryCount()
    if ($count -gt 0) {
        $latest = $searcher.QueryHistory(0, 1) | Select-Object -First 1
        $latest.Date.ToString("yyyy-MM-dd HH:mm") + "|LAST_INSTALL"
    } else { "NONE|LAST_INSTALL" }
} catch { "ERROR:" + $_.Exception.Message + "|LAST_INSTALL" }
"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
        {
            let raw = String::from_utf8_lossy(&o.stdout);
            let text = raw.trim();
            if text.starts_with("ERROR:") {
                out.push_str("Last update install: (unable to query)\n");
            } else if text.contains("NONE") {
                out.push_str("Last update install: No update history found\n");
            } else {
                let date = text.replace("|LAST_INSTALL", "");
                out.push_str(&format!("Last update install: {date}\n"));
            }
        }

        // Pending updates count
        let pending_script = r#"
try {
    $sess = New-Object -ComObject Microsoft.Update.Session
    $searcher = $sess.CreateUpdateSearcher()
    $results = $searcher.Search("IsInstalled=0 and IsHidden=0 and Type='Software'")
    $results.Updates.Count.ToString() + "|PENDING"
} catch { "ERROR:" + $_.Exception.Message + "|PENDING" }
"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", pending_script])
            .output()
        {
            let raw = String::from_utf8_lossy(&o.stdout);
            let text = raw.trim();
            if text.starts_with("ERROR:") {
                out.push_str("Pending updates: (unable to query via COM — try opening Windows Update manually)\n");
            } else {
                let count: i64 = text.replace("|PENDING", "").trim().parse().unwrap_or(-1);
                if count == 0 {
                    out.push_str("Pending updates: Up to date — no updates waiting\n");
                } else if count > 0 {
                    out.push_str(&format!("Pending updates: {count} update(s) available\n"));
                    out.push_str(
                        "  → Open Windows Update (Settings > Windows Update) to install\n",
                    );
                }
            }
        }

        // Windows Update service state
        let svc_script = r#"
$svc = Get-Service -Name wuauserv -ErrorAction SilentlyContinue
if ($svc) { $svc.Status.ToString() } else { "NOT_FOUND" }
"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", svc_script])
            .output()
        {
            let raw = String::from_utf8_lossy(&o.stdout);
            let status = raw.trim();
            out.push_str(&format!("Windows Update service: {status}\n"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let apt_out = Command::new("apt").args(["list", "--upgradable"]).output();
        let mut found = false;
        if let Ok(o) = apt_out {
            let text = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = text
                .lines()
                .filter(|l| l.contains('/') && !l.contains("Listing"))
                .collect();
            if !lines.is_empty() {
                out.push_str(&format!(
                    "{} package(s) can be upgraded (apt)\n",
                    lines.len()
                ));
                out.push_str("  → Run: sudo apt upgrade\n");
                found = true;
            }
        }
        if !found {
            if let Ok(o) = Command::new("dnf")
                .args(["check-update", "--quiet"])
                .output()
            {
                let text = String::from_utf8_lossy(&o.stdout);
                let count = text
                    .lines()
                    .filter(|l| !l.is_empty() && !l.starts_with('!'))
                    .count();
                if count > 0 {
                    out.push_str(&format!("{count} package(s) can be upgraded (dnf)\n"));
                    out.push_str("  → Run: sudo dnf upgrade\n");
                } else {
                    out.push_str("System is up to date.\n");
                }
            } else {
                out.push_str("Could not query package manager for updates.\n");
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── security ──────────────────────────────────────────────────────────────────

fn inspect_security() -> Result<String, String> {
    let mut out = String::from("Host inspection: security\n\n");

    #[cfg(target_os = "windows")]
    {
        // Windows Defender status
        let defender_script = r#"
try {
    $status = Get-MpComputerStatus -ErrorAction Stop
    "RTP:" + $status.RealTimeProtectionEnabled + "|SCAN:" + $status.QuickScanEndTime.ToString("yyyy-MM-dd HH:mm") + "|VER:" + $status.AntivirusSignatureVersion + "|AGE:" + $status.AntivirusSignatureAge
} catch { "ERROR:" + $_.Exception.Message }
"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", defender_script])
            .output()
        {
            let raw = String::from_utf8_lossy(&o.stdout);
            let text = raw.trim();
            if text.starts_with("ERROR:") {
                out.push_str(&format!("Windows Defender: unable to query — {text}\n"));
            } else {
                let get = |key: &str| -> String {
                    text.split('|')
                        .find(|s| s.starts_with(key))
                        .and_then(|s| s.splitn(2, ':').nth(1))
                        .unwrap_or("unknown")
                        .to_string()
                };
                let rtp = get("RTP");
                let last_scan = {
                    // SCAN field has a colon in the time, so grab everything after "SCAN:"
                    text.split('|')
                        .find(|s| s.starts_with("SCAN:"))
                        .and_then(|s| s.get(5..))
                        .unwrap_or("unknown")
                        .to_string()
                };
                let def_ver = get("VER");
                let age_days: i64 = get("AGE").parse().unwrap_or(-1);

                let rtp_label = if rtp == "True" {
                    "ENABLED"
                } else {
                    "DISABLED [!]"
                };
                out.push_str(&format!(
                    "Windows Defender real-time protection: {rtp_label}\n"
                ));
                out.push_str(&format!("Last quick scan: {last_scan}\n"));
                out.push_str(&format!("Signature version: {def_ver}\n"));
                if age_days >= 0 {
                    let freshness = if age_days == 0 {
                        "up to date".to_string()
                    } else if age_days <= 3 {
                        format!("{age_days} day(s) old — OK")
                    } else if age_days <= 7 {
                        format!("{age_days} day(s) old — consider updating")
                    } else {
                        format!("{age_days} day(s) old — [!] STALE, run Windows Update")
                    };
                    out.push_str(&format!("Signature age: {freshness}\n"));
                }
                if rtp != "True" {
                    out.push_str(
                        "\n[!] Real-time protection is OFF — your PC is not actively protected.\n",
                    );
                    out.push_str(
                        "    → Open Windows Security > Virus & threat protection to re-enable.\n",
                    );
                }
            }
        }

        out.push('\n');

        // Windows Firewall state
        let fw_script = r#"
try {
    Get-NetFirewallProfile -ErrorAction Stop | ForEach-Object { $_.Name + ":" + $_.Enabled }
} catch { "ERROR:" + $_.Exception.Message }
"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", fw_script])
            .output()
        {
            let raw = String::from_utf8_lossy(&o.stdout);
            let text = raw.trim();
            if !text.starts_with("ERROR:") && !text.is_empty() {
                out.push_str("Windows Firewall:\n");
                for line in text.lines() {
                    if let Some((name, enabled)) = line.split_once(':') {
                        let state = if enabled.trim() == "True" {
                            "ON"
                        } else {
                            "OFF [!]"
                        };
                        out.push_str(&format!("  {name}: {state}\n"));
                    }
                }
                out.push('\n');
            }
        }

        // Windows activation status
        let act_script = r#"
try {
    $lic = Get-CimInstance SoftwareLicensingProduct -Filter "Name like 'Windows%' and LicenseStatus=1" -ErrorAction Stop | Select-Object -First 1
    if ($lic) { "ACTIVATED" } else { "NOT_ACTIVATED" }
} catch { "UNKNOWN" }
"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", act_script])
            .output()
        {
            let raw = String::from_utf8_lossy(&o.stdout);
            match raw.trim() {
                "ACTIVATED" => out.push_str("Windows activation: Activated\n"),
                "NOT_ACTIVATED" => out.push_str("Windows activation: [!] NOT ACTIVATED\n"),
                _ => out.push_str("Windows activation: Unable to determine\n"),
            }
        }

        // UAC state
        let uac_script = r#"
$val = Get-ItemPropertyValue 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' -Name EnableLUA -ErrorAction SilentlyContinue
if ($val -eq 1) { "ON" } else { "OFF" }
"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", uac_script])
            .output()
        {
            let raw = String::from_utf8_lossy(&o.stdout);
            let state = raw.trim();
            let label = if state == "ON" {
                "Enabled"
            } else {
                "DISABLED [!] — recommended to re-enable via secpol.msc"
            };
            out.push_str(&format!("UAC (User Account Control): {label}\n"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(o) = Command::new("ufw").arg("status").output() {
            let text = String::from_utf8_lossy(&o.stdout);
            out.push_str(&format!(
                "UFW: {}\n",
                text.lines().next().unwrap_or("unknown")
            ));
        }
        if let Ok(cfg) = std::fs::read_to_string("/etc/selinux/config") {
            if let Some(line) = cfg.lines().find(|l| l.starts_with("SELINUX=")) {
                out.push_str(&format!("{line}\n"));
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── pending_reboot ────────────────────────────────────────────────────────────

fn inspect_pending_reboot() -> Result<String, String> {
    let mut out = String::from("Host inspection: pending_reboot\n\n");

    #[cfg(target_os = "windows")]
    {
        let script = r#"
$reasons = @()
if (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired') {
    $reasons += "Windows Update requires a restart"
}
if (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending') {
    $reasons += "Windows component install/update requires a restart"
}
$pfro = Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager' -Name PendingFileRenameOperations -ErrorAction SilentlyContinue
if ($pfro -and $pfro.PendingFileRenameOperations) {
    $reasons += "Pending file rename operations (driver or system file replacement)"
}
if ($reasons.Count -eq 0) { "NO_REBOOT_NEEDED" } else { $reasons -join "|REASON|" }
"#;
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .map_err(|e| format!("pending_reboot: {e}"))?;

        let raw = String::from_utf8_lossy(&output.stdout);
        let text = raw.trim();

        if text == "NO_REBOOT_NEEDED" {
            out.push_str("No restart required — system is up to date and stable.\n");
        } else if text.is_empty() {
            out.push_str("Could not determine reboot status.\n");
        } else {
            out.push_str("[!] A system restart is pending:\n\n");
            for reason in text.split("|REASON|") {
                out.push_str(&format!("  • {}\n", reason.trim()));
            }
            out.push_str("\nRecommendation: Save your work and restart when convenient.\n");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if std::path::Path::new("/var/run/reboot-required").exists() {
            out.push_str("[!] A restart is required (see /var/run/reboot-required)\n");
            if let Ok(pkgs) = std::fs::read_to_string("/var/run/reboot-required.pkgs") {
                out.push_str("Packages requiring restart:\n");
                for p in pkgs.lines().take(10) {
                    out.push_str(&format!("  • {p}\n"));
                }
            }
        } else {
            out.push_str("No restart required.\n");
        }
    }

    Ok(out.trim_end().to_string())
}

// ── disk_health ───────────────────────────────────────────────────────────────

fn inspect_disk_health() -> Result<String, String> {
    let mut out = String::from("Host inspection: disk_health\n\n");

    #[cfg(target_os = "windows")]
    {
        let script = r#"
try {
    $disks = Get-PhysicalDisk -ErrorAction Stop
    foreach ($d in $disks) {
        $size_gb = [math]::Round($d.Size / 1GB, 0)
        $d.FriendlyName + "|" + $d.MediaType + "|" + $size_gb + "GB|" + $d.HealthStatus + "|" + $d.OperationalStatus
    }
} catch { "ERROR:" + $_.Exception.Message }
"#;
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .map_err(|e| format!("disk_health: {e}"))?;

        let raw = String::from_utf8_lossy(&output.stdout);
        let text = raw.trim();

        if text.starts_with("ERROR:") {
            out.push_str(&format!("Unable to query disk health: {text}\n"));
            out.push_str("This may require running as administrator.\n");
        } else if text.is_empty() {
            out.push_str("No physical disks found.\n");
        } else {
            out.push_str("Physical Drive Health:\n\n");
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(5, '|').collect();
                if parts.len() >= 4 {
                    let name = parts[0];
                    let media = parts[1];
                    let size = parts[2];
                    let health = parts[3];
                    let op_status = parts.get(4).unwrap_or(&"");
                    let health_label = match health.trim() {
                        "Healthy" => "OK",
                        "Warning" => "[!] WARNING",
                        "Unhealthy" => "[!!] UNHEALTHY — BACK UP YOUR DATA NOW",
                        other => other,
                    };
                    out.push_str(&format!("  {name}\n"));
                    out.push_str(&format!("    Type: {media} | Size: {size}\n"));
                    out.push_str(&format!("    Health: {health_label}\n"));
                    if !op_status.is_empty() {
                        out.push_str(&format!("    Status: {op_status}\n"));
                    }
                    out.push('\n');
                }
            }
        }

        // SMART failure prediction (best-effort, may need admin)
        let smart_script = r#"
try {
    Get-WmiObject -Class MSStorageDriver_FailurePredictStatus -Namespace root\wmi -ErrorAction Stop |
        ForEach-Object { $_.InstanceName + "|" + $_.PredictFailure }
} catch { "" }
"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", smart_script])
            .output()
        {
            let raw2 = String::from_utf8_lossy(&o.stdout);
            let text2 = raw2.trim();
            if !text2.is_empty() {
                let failures: Vec<&str> = text2.lines().filter(|l| l.contains("|True")).collect();
                if failures.is_empty() {
                    out.push_str("SMART failure prediction: No failures predicted\n");
                } else {
                    out.push_str("[!!] SMART failure predicted on one or more drives:\n");
                    for f in failures {
                        let name = f.split('|').next().unwrap_or(f);
                        out.push_str(&format!("  • {name}\n"));
                    }
                    out.push_str(
                        "\nBack up your data immediately and replace the failing drive.\n",
                    );
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(o) = Command::new("lsblk")
            .args(["-d", "-o", "NAME,SIZE,TYPE,ROTA,MODEL"])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            out.push_str("Block devices:\n");
            out.push_str(text.trim());
            out.push('\n');
        }
        if let Ok(scan) = Command::new("smartctl").args(["--scan"]).output() {
            let devices = String::from_utf8_lossy(&scan.stdout);
            for dev_line in devices.lines().take(4) {
                let dev = dev_line.split_whitespace().next().unwrap_or("");
                if dev.is_empty() {
                    continue;
                }
                if let Ok(o) = Command::new("smartctl").args(["-H", dev]).output() {
                    let health = String::from_utf8_lossy(&o.stdout);
                    if let Some(line) = health.lines().find(|l| l.contains("SMART overall-health"))
                    {
                        out.push_str(&format!("{dev}: {}\n", line.trim()));
                    }
                }
            }
        } else {
            out.push_str("(install smartmontools for SMART health data)\n");
        }
    }

    Ok(out.trim_end().to_string())
}

// ── battery ───────────────────────────────────────────────────────────────────

fn inspect_battery() -> Result<String, String> {
    let mut out = String::from("Host inspection: battery\n\n");

    #[cfg(target_os = "windows")]
    {
        let script = r#"
try {
    $bats = Get-CimInstance -ClassName Win32_Battery -ErrorAction Stop
    if (-not $bats) { "NO_BATTERY"; exit }
    foreach ($b in $bats) {
        $status = switch ($b.BatteryStatus) {
            1 { "Discharging (on battery)" }
            2 { "AC power - fully charged" }
            3 { "AC power - charging" }
            6 { "AC power - charging" }
            7 { "AC power - charging" }
            default { "Status $($b.BatteryStatus)" }
        }
        $b.Name + "|" + $b.EstimatedChargeRemaining + "|" + $status + "|" + $b.EstimatedRunTime
    }
} catch { "ERROR:" + $_.Exception.Message }
"#;
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .map_err(|e| format!("battery: {e}"))?;

        let raw = String::from_utf8_lossy(&output.stdout);
        let text = raw.trim();

        if text == "NO_BATTERY" {
            out.push_str("No battery detected — desktop or AC-only system.\n");
            return Ok(out.trim_end().to_string());
        }
        if text.starts_with("ERROR:") {
            out.push_str(&format!("Unable to query battery: {text}\n"));
            return Ok(out.trim_end().to_string());
        }

        for line in text.lines() {
            let parts: Vec<&str> = line.splitn(4, '|').collect();
            if parts.len() >= 3 {
                let name = parts[0];
                let charge: i64 = parts[1].parse().unwrap_or(-1);
                let status = parts[2];
                let time_rem: i64 = parts.get(3).and_then(|v| v.parse().ok()).unwrap_or(-1);

                out.push_str(&format!("Battery: {name}\n"));
                if charge >= 0 {
                    let bar_filled = (charge as usize * 20) / 100;
                    out.push_str(&format!(
                        "  Charge: [{}{}] {}%\n",
                        "#".repeat(bar_filled),
                        ".".repeat(20 - bar_filled),
                        charge
                    ));
                }
                out.push_str(&format!("  Status: {status}\n"));
                // Windows returns 71582788 as "unknown remaining time"
                if time_rem > 0 && time_rem < 71_582_788 {
                    let hours = time_rem / 60;
                    let mins = time_rem % 60;
                    out.push_str(&format!("  Estimated time remaining: {hours}h {mins}m\n"));
                }
                out.push('\n');
            }
        }

        // Battery wear level (requires admin for CIM battery namespace)
        let wear_script = r#"
try {
    $full = Get-CimInstance -Namespace root\cimv2 -ClassName BatteryFullChargedCapacity -ErrorAction Stop | Select-Object -First 1
    $static = Get-CimInstance -Namespace root\cimv2 -ClassName BatteryStaticData -ErrorAction Stop | Select-Object -First 1
    if ($full -and $static -and $static.DesignedCapacity -gt 0) {
        $pct = [math]::Round(($full.FullChargedCapacity / $static.DesignedCapacity) * 100, 1)
        $full.FullChargedCapacity.ToString() + "|" + $static.DesignedCapacity.ToString() + "|" + $pct.ToString()
    } else { "UNKNOWN" }
} catch { "UNKNOWN" }
"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", wear_script])
            .output()
        {
            let raw2 = String::from_utf8_lossy(&o.stdout);
            let t = raw2.trim();
            if t != "UNKNOWN" && !t.is_empty() {
                let parts: Vec<&str> = t.splitn(3, '|').collect();
                if parts.len() == 3 {
                    let full: i64 = parts[0].parse().unwrap_or(0);
                    let design: i64 = parts[1].parse().unwrap_or(0);
                    let pct: f64 = parts[2].parse().unwrap_or(0.0);
                    out.push_str(&format!(
                        "Battery wear level: {pct:.1}% of original capacity\n"
                    ));
                    out.push_str(&format!(
                        "  Current full charge: {full} mWh / Design: {design} mWh\n"
                    ));
                    if pct < 50.0 {
                        out.push_str("  [!] Significantly degraded — consider replacement\n");
                    } else if pct < 75.0 {
                        out.push_str("  [-] Noticeable wear\n");
                    } else {
                        out.push_str("  Battery health is good\n");
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let power_path = std::path::Path::new("/sys/class/power_supply");
        let mut found = false;
        if power_path.exists() {
            if let Ok(entries) = std::fs::read_dir(power_path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if let Ok(t) = std::fs::read_to_string(p.join("type")) {
                        if t.trim() == "Battery" {
                            found = true;
                            let name = p
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            out.push_str(&format!("Battery: {name}\n"));
                            let read = |f: &str| {
                                std::fs::read_to_string(p.join(f))
                                    .ok()
                                    .map(|s| s.trim().to_string())
                            };
                            if let Some(cap) = read("capacity") {
                                out.push_str(&format!("  Charge: {cap}%\n"));
                            }
                            if let Some(status) = read("status") {
                                out.push_str(&format!("  Status: {status}\n"));
                            }
                            if let (Some(full), Some(design)) =
                                (read("energy_full"), read("energy_full_design"))
                            {
                                if let (Ok(f), Ok(d)) = (full.parse::<f64>(), design.parse::<f64>())
                                {
                                    if d > 0.0 {
                                        out.push_str(&format!(
                                            "  Wear level: {:.1}% of design capacity\n",
                                            (f / d) * 100.0
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if !found {
            out.push_str("No battery found.\n");
        }
    }

    Ok(out.trim_end().to_string())
}

// ── recent_crashes ────────────────────────────────────────────────────────────

fn inspect_recent_crashes(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: recent_crashes\n\n");
    let n = max_entries.clamp(1, 30);

    #[cfg(target_os = "windows")]
    {
        // BSODs / unexpected shutdowns (EventID 41 = kernel power, 1001 = BugCheck)
        let bsod_script = format!(
            r#"
try {{
    $events = Get-WinEvent -FilterHashtable @{{LogName='System'; Id=41,1001}} -MaxEvents {n} -ErrorAction SilentlyContinue
    if ($events) {{
        $events | ForEach-Object {{
            $_.TimeCreated.ToString("yyyy-MM-dd HH:mm") + "|" + $_.Id + "|" + (($_.Message -split "[\r\n]")[0].Trim())
        }}
    }} else {{ "NO_BSOD" }}
}} catch {{ "ERROR:" + $_.Exception.Message }}"#
        );

        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", &bsod_script])
            .output()
        {
            let raw = String::from_utf8_lossy(&o.stdout);
            let text = raw.trim();
            if text == "NO_BSOD" {
                out.push_str("System crashes (BSOD/kernel): None in recent history\n");
            } else if text.starts_with("ERROR:") {
                out.push_str("System crashes: unable to query\n");
            } else {
                out.push_str("System crashes / unexpected shutdowns:\n");
                for line in text.lines() {
                    let parts: Vec<&str> = line.splitn(3, '|').collect();
                    if parts.len() >= 3 {
                        let time = parts[0];
                        let id = parts[1];
                        let msg = parts[2];
                        let label = if id == "41" {
                            "Unexpected shutdown"
                        } else {
                            "BSOD (BugCheck)"
                        };
                        out.push_str(&format!("  [{time}] {label}: {msg}\n"));
                    }
                }
                out.push('\n');
            }
        }

        // Application crashes (EventID 1000 = app crash, 1002 = app hang)
        let app_script = format!(
            r#"
try {{
    $crashes = Get-WinEvent -FilterHashtable @{{LogName='Application'; Id=1000,1002}} -MaxEvents {n} -ErrorAction SilentlyContinue
    if ($crashes) {{
        $crashes | ForEach-Object {{
            $_.TimeCreated.ToString("yyyy-MM-dd HH:mm") + "|" + (($_.Message -split "[\r\n]")[0].Trim())
        }}
    }} else {{ "NO_CRASHES" }}
}} catch {{ "ERROR_APP:" + $_.Exception.Message }}"#
        );

        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", &app_script])
            .output()
        {
            let raw = String::from_utf8_lossy(&o.stdout);
            let text = raw.trim();
            if text == "NO_CRASHES" {
                out.push_str("Application crashes: None in recent history\n");
            } else if text.starts_with("ERROR_APP:") {
                out.push_str("Application crashes: unable to query\n");
            } else {
                out.push_str("Application crashes:\n");
                for line in text.lines().take(n) {
                    let parts: Vec<&str> = line.splitn(2, '|').collect();
                    if parts.len() >= 2 {
                        out.push_str(&format!("  [{}] {}\n", parts[0], parts[1]));
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let n_str = n.to_string();
        if let Ok(o) = Command::new("journalctl")
            .args(["-k", "--no-pager", "-n", &n_str, "-p", "0..2"])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed.contains("No entries") {
                out.push_str("No kernel panics or critical crashes found.\n");
            } else {
                out.push_str("Kernel critical events:\n");
                out.push_str(trimmed);
                out.push('\n');
            }
        }
        if let Ok(o) = Command::new("coredumpctl")
            .args(["list", "--no-pager"])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            let count = text
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with("TIME"))
                .count();
            if count > 0 {
                out.push_str(&format!(
                    "\nCore dumps on file: {count}\n  → Run: coredumpctl list\n"
                ));
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── scheduled_tasks ───────────────────────────────────────────────────────────

fn inspect_scheduled_tasks(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: scheduled_tasks\n\n");
    let n = max_entries.clamp(1, 30);

    #[cfg(target_os = "windows")]
    {
        let script = format!(
            r#"
try {{
    $tasks = Get-ScheduledTask -ErrorAction Stop |
        Where-Object {{ $_.State -ne 'Disabled' }} |
        ForEach-Object {{
            $info = $_ | Get-ScheduledTaskInfo -ErrorAction SilentlyContinue
            $lastRun = if ($info -and $info.LastRunTime -and $info.LastRunTime.Year -gt 2000) {{
                $info.LastRunTime.ToString("yyyy-MM-dd HH:mm")
            }} else {{ "never" }}
            $exec = ($_.Actions | Select-Object -First 1).Execute
            if (-not $exec) {{ $exec = "(no exec)" }}
            $_.TaskName + "|" + $_.TaskPath + "|" + $_.State + "|" + $lastRun + "|" + $exec
        }}
    $tasks | Select-Object -First {n}
}} catch {{ "ERROR:" + $_.Exception.Message }}"#
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| format!("scheduled_tasks: {e}"))?;

        let raw = String::from_utf8_lossy(&output.stdout);
        let text = raw.trim();

        if text.starts_with("ERROR:") {
            out.push_str(&format!("Unable to query scheduled tasks: {text}\n"));
        } else if text.is_empty() {
            out.push_str("No active scheduled tasks found.\n");
        } else {
            out.push_str(&format!("Active scheduled tasks (up to {n}):\n\n"));
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(5, '|').collect();
                if parts.len() >= 4 {
                    let name = parts[0];
                    let path = parts[1];
                    let state = parts[2];
                    let last = parts[3];
                    let exec = parts.get(4).unwrap_or(&"").trim();
                    let display_path = path.trim_matches('\\');
                    let display_path = if display_path.is_empty() {
                        "Root"
                    } else {
                        display_path
                    };
                    out.push_str(&format!("  {name} [{display_path}]\n"));
                    out.push_str(&format!("    State: {state} | Last run: {last}\n"));
                    if !exec.is_empty() && exec != "(no exec)" {
                        let short = if exec.len() > 80 { &exec[..80] } else { exec };
                        out.push_str(&format!("    Runs: {short}\n"));
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(o) = Command::new("systemctl")
            .args(["list-timers", "--no-pager", "--all"])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            out.push_str("Systemd timers:\n");
            for l in text
                .lines()
                .filter(|l| {
                    !l.trim().is_empty() && !l.starts_with("NEXT") && !l.starts_with("timers")
                })
                .take(n)
            {
                out.push_str(&format!("  {l}\n"));
            }
            out.push('\n');
        }
        if let Ok(o) = Command::new("crontab").arg("-l").output() {
            let text = String::from_utf8_lossy(&o.stdout);
            let jobs: Vec<&str> = text
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                .collect();
            if !jobs.is_empty() {
                out.push_str("User crontab:\n");
                for j in jobs.iter().take(n) {
                    out.push_str(&format!("  {j}\n"));
                }
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── dev_conflicts ─────────────────────────────────────────────────────────────

fn inspect_dev_conflicts() -> Result<String, String> {
    let mut out = String::from("Host inspection: dev_conflicts\n\n");
    let mut conflicts: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    // ── Node.js / version managers ────────────────────────────────────────────
    {
        let node_ver = Command::new("node")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());
        let nvm_active = Command::new("nvm")
            .arg("current")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.contains("none") && !s.contains("No current"));
        let fnm_active = Command::new("fnm")
            .arg("current")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.contains("none"));
        let volta_active = Command::new("volta")
            .args(["which", "node"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        out.push_str("Node.js:\n");
        if let Some(ref v) = node_ver {
            out.push_str(&format!("  Active: {v}\n"));
        } else {
            out.push_str("  Not installed\n");
        }
        let managers: Vec<&str> = [
            nvm_active.as_deref(),
            fnm_active.as_deref(),
            volta_active.as_deref(),
        ]
        .iter()
        .filter_map(|x| *x)
        .collect();
        if managers.len() > 1 {
            conflicts.push(format!(
                "Multiple Node.js version managers detected (nvm/fnm/volta). Only one should be active to avoid PATH conflicts."
            ));
        } else if !managers.is_empty() {
            out.push_str(&format!("  Version manager: {}\n", managers[0]));
        }
        out.push('\n');
    }

    // ── Python ────────────────────────────────────────────────────────────────
    {
        let py3 = Command::new("python3")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
                let v = if stdout.is_empty() { stderr } else { stdout };
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            });
        let py = Command::new("python")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
                let v = if stdout.is_empty() { stderr } else { stdout };
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            });
        let pyenv = Command::new("pyenv")
            .arg("version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let conda_env = std::env::var("CONDA_DEFAULT_ENV").ok();

        out.push_str("Python:\n");
        match (&py3, &py) {
            (Some(v3), Some(v)) if v3 != v => {
                out.push_str(&format!("  python3: {v3}\n  python:  {v}\n"));
                if v.contains("2.") {
                    conflicts.push(
                        "python and python3 point to different major versions (2.x vs 3.x). Scripts using 'python' may break unexpectedly.".to_string()
                    );
                } else {
                    notes.push(
                        "python and python3 resolve to different minor versions.".to_string(),
                    );
                }
            }
            (Some(v3), None) => out.push_str(&format!("  python3: {v3}\n")),
            (None, Some(v)) => out.push_str(&format!("  python: {v}\n")),
            (Some(v3), Some(_)) => out.push_str(&format!("  {v3}\n")),
            (None, None) => out.push_str("  Not installed\n"),
        }
        if let Some(ref pe) = pyenv {
            out.push_str(&format!("  pyenv: {pe}\n"));
        }
        if let Some(env) = conda_env {
            if env == "base" {
                notes.push("Conda base environment is active — may shadow system Python. Run 'conda deactivate' if unexpected.".to_string());
            } else {
                out.push_str(&format!("  conda env: {env}\n"));
            }
        }
        out.push('\n');
    }

    // ── Rust / Cargo ──────────────────────────────────────────────────────────
    {
        let toolchain = Command::new("rustup")
            .args(["show", "active-toolchain"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let cargo_ver = Command::new("cargo")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());
        let rustc_ver = Command::new("rustc")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        out.push_str("Rust:\n");
        if let Some(ref t) = toolchain {
            out.push_str(&format!("  Active toolchain: {t}\n"));
        }
        if let Some(ref c) = cargo_ver {
            out.push_str(&format!("  {c}\n"));
        }
        if let Some(ref r) = rustc_ver {
            out.push_str(&format!("  {r}\n"));
        }
        if cargo_ver.is_none() && rustc_ver.is_none() {
            out.push_str("  Not installed\n");
        }

        // Detect system rust that might shadow rustup
        #[cfg(not(target_os = "windows"))]
        if let Ok(o) = Command::new("which").arg("rustc").output() {
            let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !path.is_empty() && !path.contains(".cargo") && !path.contains("rustup") {
                conflicts.push(format!(
                    "rustc found at non-rustup path '{path}' — may conflict with rustup-managed toolchain"
                ));
            }
        }
        out.push('\n');
    }

    // ── Git ───────────────────────────────────────────────────────────────────
    {
        let git_ver = Command::new("git")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());
        out.push_str("Git:\n");
        if let Some(ref v) = git_ver {
            out.push_str(&format!("  {v}\n"));
            let email = Command::new("git")
                .args(["config", "--global", "user.email"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string());
            if let Some(ref e) = email {
                if e.is_empty() {
                    notes.push("Git user.email is not configured globally — commits may fail or use wrong identity.".to_string());
                } else {
                    out.push_str(&format!("  user.email: {e}\n"));
                }
            }
            let gpg_sign = Command::new("git")
                .args(["config", "--global", "commit.gpgsign"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string());
            if gpg_sign.as_deref() == Some("true") {
                let key = Command::new("git")
                    .args(["config", "--global", "user.signingkey"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string());
                if key.as_deref().map(|k| k.is_empty()).unwrap_or(true) {
                    conflicts.push("Git commit signing is enabled but no signing key is configured — commits will fail.".to_string());
                }
            }
        } else {
            out.push_str("  Not installed\n");
        }
        out.push('\n');
    }

    // ── PATH duplicates ───────────────────────────────────────────────────────
    {
        let path_env = std::env::var("PATH").unwrap_or_default();
        let sep = if cfg!(windows) { ';' } else { ':' };
        let mut seen = HashSet::new();
        let mut dupes: Vec<String> = Vec::new();
        for p in path_env.split(sep) {
            let norm = p.trim().to_lowercase();
            if !norm.is_empty() && !seen.insert(norm) {
                dupes.push(p.to_string());
            }
        }
        if !dupes.is_empty() {
            let shown: Vec<&str> = dupes.iter().take(3).map(|s| s.as_str()).collect();
            notes.push(format!(
                "Duplicate PATH entries: {} {}",
                shown.join(", "),
                if dupes.len() > 3 {
                    format!("+{} more", dupes.len() - 3)
                } else {
                    String::new()
                }
            ));
        }
    }

    // ── Summary ───────────────────────────────────────────────────────────────
    if conflicts.is_empty() && notes.is_empty() {
        out.push_str("No conflicts detected — dev environment looks clean.\n");
    } else {
        if !conflicts.is_empty() {
            out.push_str("CONFLICTS:\n");
            for c in &conflicts {
                out.push_str(&format!("  [!] {c}\n"));
            }
            out.push('\n');
        }
        if !notes.is_empty() {
            out.push_str("NOTES:\n");
            for n in &notes {
                out.push_str(&format!("  [-] {n}\n"));
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── connectivity ──────────────────────────────────────────────────────────────

fn inspect_connectivity() -> Result<String, String> {
    let mut out = String::from("Host inspection: connectivity\n\n");

    #[cfg(target_os = "windows")]
    {
        let inet_script = r#"
try {
    $r = Test-NetConnection -ComputerName 8.8.8.8 -Port 53 -InformationLevel Quiet -WarningAction SilentlyContinue
    if ($r) { "REACHABLE" } else { "UNREACHABLE" }
} catch { "ERROR:" + $_.Exception.Message }
"#;
        if let Ok(o) = Command::new("powershell").args(["-NoProfile", "-Command", inet_script]).output() {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            match text.as_str() {
                "REACHABLE" => out.push_str("Internet: reachable\n"),
                "UNREACHABLE" => out.push_str("Internet: unreachable [!]\n"),
                _ => out.push_str(&format!("Internet: {}\n", text.trim_start_matches("ERROR:").trim())),
            }
        }

        let dns_script = r#"
try {
    Resolve-DnsName -Name "dns.google" -Type A -ErrorAction Stop | Out-Null
    "DNS:ok"
} catch { "DNS:fail:" + $_.Exception.Message }
"#;
        if let Ok(o) = Command::new("powershell").args(["-NoProfile", "-Command", dns_script]).output() {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if text == "DNS:ok" {
                out.push_str("DNS: resolving correctly\n");
            } else {
                let detail = text.trim_start_matches("DNS:fail:").trim();
                out.push_str(&format!("DNS: failed — {}\n", detail));
            }
        }

        let gw_script = r#"
(Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | Sort-Object RouteMetric | Select-Object -First 1).NextHop
"#;
        if let Ok(o) = Command::new("powershell").args(["-NoProfile", "-Command", gw_script]).output() {
            let gw = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !gw.is_empty() && gw != "0.0.0.0" {
                out.push_str(&format!("Default gateway: {}\n", gw));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let reachable = Command::new("ping").args(["-c", "1", "-W", "2", "8.8.8.8"]).output()
            .map(|o| o.status.success()).unwrap_or(false);
        out.push_str(if reachable { "Internet: reachable\n" } else { "Internet: unreachable\n" });
        let dns_ok = Command::new("getent").args(["hosts", "dns.google"]).output()
            .map(|o| o.status.success()).unwrap_or(false);
        out.push_str(if dns_ok { "DNS: resolving correctly\n" } else { "DNS: failed\n" });
        if let Ok(o) = Command::new("ip").args(["route", "show", "default"]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            if let Some(line) = text.lines().next() {
                out.push_str(&format!("Default gateway: {}\n", line.trim()));
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── wifi ──────────────────────────────────────────────────────────────────────

fn inspect_wifi() -> Result<String, String> {
    let mut out = String::from("Host inspection: wifi\n\n");

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("netsh").args(["wlan", "show", "interfaces"]).output()
            .map_err(|e| format!("wifi: {e}"))?;
        let text = String::from_utf8_lossy(&output.stdout).to_string();

        if text.contains("There is no wireless interface") || text.trim().is_empty() {
            out.push_str("No wireless interface detected on this machine.\n");
            return Ok(out.trim_end().to_string());
        }

        let fields = [
            ("SSID", "SSID"),
            ("State", "State"),
            ("Signal", "Signal"),
            ("Radio type", "Radio type"),
            ("Channel", "Channel"),
            ("Receive rate (Mbps)", "Download speed (Mbps)"),
            ("Transmit rate (Mbps)", "Upload speed (Mbps)"),
            ("Authentication", "Authentication"),
            ("Network type", "Network type"),
        ];

        let mut any = false;
        for line in text.lines() {
            let trimmed = line.trim();
            for (key, label) in &fields {
                if trimmed.starts_with(key) && trimmed.contains(':') {
                    let val = trimmed.splitn(2, ':').nth(1).unwrap_or("").trim();
                    if !val.is_empty() {
                        out.push_str(&format!("  {label}: {val}\n"));
                        any = true;
                    }
                }
            }
        }
        if !any {
            out.push_str("  (Wi-Fi adapter disconnected or no active connection)\n");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(o) = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device"]).output() {
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            let lines: Vec<&str> = text.lines().filter(|l| l.contains(":wifi:")).collect();
            if lines.is_empty() { out.push_str("No Wi-Fi devices found.\n"); }
            else { for l in lines { out.push_str(&format!("  {l}\n")); } }
        } else if let Ok(o) = Command::new("iwconfig").output() {
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            if !text.trim().is_empty() { out.push_str(text.trim()); out.push('\n'); }
        } else {
            out.push_str("No wireless tool available (install nmcli or wireless-tools).\n");
        }
    }

    Ok(out.trim_end().to_string())
}

// ── connections ───────────────────────────────────────────────────────────────

fn inspect_connections(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: connections\n\n");
    let n = max_entries.clamp(1, 25);

    #[cfg(target_os = "windows")]
    {
        let script = format!(r#"
try {{
    $procs = @{{}}
    Get-Process -ErrorAction SilentlyContinue | ForEach-Object {{ $procs[$_.Id] = $_.Name }}
    $all = Get-NetTCPConnection -State Established -ErrorAction Stop |
        Sort-Object RemoteAddress
    "TOTAL:" + $all.Count
    $all | Select-Object -First {n} | ForEach-Object {{
        $pname = if ($procs.ContainsKey($_.OwningProcess)) {{ $procs[$_.OwningProcess] }} else {{ "pid:" + $_.OwningProcess }}
        $pname + "|" + $_.LocalAddress + ":" + $_.LocalPort + "|" + $_.RemoteAddress + ":" + $_.RemotePort
    }}
}} catch {{ "ERROR:" + $_.Exception.Message }}"#);

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| format!("connections: {e}"))?;

        let raw = String::from_utf8_lossy(&output.stdout);
        let text = raw.trim();

        if text.starts_with("ERROR:") {
            out.push_str(&format!("Unable to query connections: {text}\n"));
        } else {
            let mut total = 0usize;
            let mut rows = Vec::new();
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("TOTAL:") {
                    total = rest.trim().parse().unwrap_or(0);
                } else {
                    rows.push(line);
                }
            }
            out.push_str(&format!("Established TCP connections: {total}\n\n"));
            for row in &rows {
                let parts: Vec<&str> = row.splitn(3, '|').collect();
                if parts.len() == 3 {
                    out.push_str(&format!("  {} | {} → {}\n", parts[0], parts[1], parts[2]));
                }
            }
            if total > n {
                out.push_str(&format!("\n  ... {} more connections not shown\n", total.saturating_sub(n)));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(o) = Command::new("ss").args(["-tnp", "state", "established"]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = text.lines().skip(1).filter(|l| !l.trim().is_empty()).collect();
            out.push_str(&format!("Established TCP connections: {}\n\n", lines.len()));
            for line in lines.iter().take(n) { out.push_str(&format!("  {}\n", line.trim())); }
            if lines.len() > n {
                out.push_str(&format!("\n  ... {} more not shown\n", lines.len() - n));
            }
        } else {
            out.push_str("ss not available — install iproute2\n");
        }
    }

    Ok(out.trim_end().to_string())
}

// ── vpn ───────────────────────────────────────────────────────────────────────

fn inspect_vpn() -> Result<String, String> {
    let mut out = String::from("Host inspection: vpn\n\n");

    #[cfg(target_os = "windows")]
    {
        let script = r#"
try {
    $vpn = Get-NetAdapter -ErrorAction Stop | Where-Object {
        $_.InterfaceDescription -match 'VPN|TAP|WireGuard|OpenVPN|Cisco|Palo Alto|GlobalProtect|Juniper|Pulse|NordVPN|ExpressVPN|Mullvad|ProtonVPN' -or
        $_.Name -match 'VPN|TAP|WireGuard|tun|ppp|wg\d'
    }
    if ($vpn) {
        foreach ($a in $vpn) {
            $a.Name + "|" + $a.InterfaceDescription + "|" + $a.Status + "|" + $a.MediaConnectionState
        }
    } else { "NONE" }
} catch { "ERROR:" + $_.Exception.Message }
"#;
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .map_err(|e| format!("vpn: {e}"))?;

        let raw = String::from_utf8_lossy(&output.stdout);
        let text = raw.trim();

        if text == "NONE" {
            out.push_str("No VPN adapters detected — no active VPN connection found.\n");
        } else if text.starts_with("ERROR:") {
            out.push_str(&format!("Unable to query adapters: {text}\n"));
        } else {
            out.push_str("VPN adapters:\n\n");
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(4, '|').collect();
                if parts.len() >= 3 {
                    let name = parts[0];
                    let desc = parts[1];
                    let status = parts[2];
                    let media = parts.get(3).unwrap_or(&"unknown");
                    let label = if status.trim() == "Up" { "CONNECTED" } else { "disconnected" };
                    out.push_str(&format!("  {name} [{label}]\n    {desc}\n    Status: {status} | Media: {media}\n\n"));
                }
            }
        }

        // Windows built-in VPN connections
        let ras_script = r#"
try {
    $c = Get-VpnConnection -ErrorAction Stop
    if ($c) { foreach ($v in $c) { $v.Name + "|" + $v.ConnectionStatus + "|" + $v.ServerAddress } }
    else { "NO_RAS" }
} catch { "NO_RAS" }
"#;
        if let Ok(o) = Command::new("powershell").args(["-NoProfile", "-Command", ras_script]).output() {
            let t = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if t != "NO_RAS" && !t.is_empty() {
                out.push_str("Windows VPN connections:\n");
                for line in t.lines() {
                    let parts: Vec<&str> = line.splitn(3, '|').collect();
                    if parts.len() >= 2 {
                        let name = parts[0];
                        let status = parts[1];
                        let server = parts.get(2).unwrap_or(&"");
                        out.push_str(&format!("  {name} → {server} [{status}]\n"));
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(o) = Command::new("ip").args(["link", "show"]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            let vpn_ifaces: Vec<&str> = text.lines()
                .filter(|l| l.contains("tun") || l.contains("tap") || l.contains(" wg") || l.contains("ppp"))
                .collect();
            if vpn_ifaces.is_empty() {
                out.push_str("No VPN interfaces (tun/tap/wg/ppp) detected.\n");
            } else {
                out.push_str(&format!("VPN-like interfaces ({}):\n", vpn_ifaces.len()));
                for l in vpn_ifaces { out.push_str(&format!("  {}\n", l.trim())); }
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── proxy ─────────────────────────────────────────────────────────────────────

fn inspect_proxy() -> Result<String, String> {
    let mut out = String::from("Host inspection: proxy\n\n");

    #[cfg(target_os = "windows")]
    {
        let script = r#"
$ie = Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings' -ErrorAction SilentlyContinue
if ($ie) {
    "ENABLE:" + $ie.ProxyEnable + "|SERVER:" + $ie.ProxyServer + "|OVERRIDE:" + $ie.ProxyOverride
} else { "NONE" }
"#;
        if let Ok(o) = Command::new("powershell").args(["-NoProfile", "-Command", script]).output() {
            let raw = String::from_utf8_lossy(&o.stdout);
            let text = raw.trim();
            if text != "NONE" && !text.is_empty() {
                let get = |key: &str| -> &str {
                    text.split('|')
                        .find(|s| s.starts_with(key))
                        .and_then(|s| s.splitn(2, ':').nth(1))
                        .unwrap_or("")
                };
                let enabled = get("ENABLE");
                let server = get("SERVER");
                let overrides = get("OVERRIDE");
                out.push_str("WinINET / IE proxy:\n");
                out.push_str(&format!("  Enabled: {}\n", if enabled == "1" { "yes" } else { "no" }));
                if !server.is_empty() && server != "None" {
                    out.push_str(&format!("  Proxy server: {server}\n"));
                }
                if !overrides.is_empty() && overrides != "None" {
                    out.push_str(&format!("  Bypass list: {overrides}\n"));
                }
                out.push('\n');
            }
        }

        if let Ok(o) = Command::new("netsh").args(["winhttp", "show", "proxy"]).output() {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            out.push_str("WinHTTP proxy:\n");
            for line in text.lines() {
                let l = line.trim();
                if !l.is_empty() { out.push_str(&format!("  {l}\n")); }
            }
            out.push('\n');
        }

        let mut env_found = false;
        for var in &["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "no_proxy", "NO_PROXY"] {
            if let Ok(val) = std::env::var(var) {
                if !env_found { out.push_str("Environment proxy variables:\n"); env_found = true; }
                out.push_str(&format!("  {var}: {val}\n"));
            }
        }
        if !env_found { out.push_str("No proxy environment variables set.\n"); }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut found = false;
        for var in &["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "no_proxy", "NO_PROXY", "ALL_PROXY", "all_proxy"] {
            if let Ok(val) = std::env::var(var) {
                if !found { out.push_str("Proxy environment variables:\n"); found = true; }
                out.push_str(&format!("  {var}: {val}\n"));
            }
        }
        if !found { out.push_str("No proxy environment variables set.\n"); }
        if let Ok(content) = std::fs::read_to_string("/etc/environment") {
            let proxy_lines: Vec<&str> = content.lines()
                .filter(|l| l.to_lowercase().contains("proxy"))
                .collect();
            if !proxy_lines.is_empty() {
                out.push_str("\nSystem proxy (/etc/environment):\n");
                for l in proxy_lines { out.push_str(&format!("  {l}\n")); }
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── firewall_rules ────────────────────────────────────────────────────────────

fn inspect_firewall_rules(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: firewall_rules\n\n");
    let n = max_entries.clamp(1, 20);

    #[cfg(target_os = "windows")]
    {
        let script = format!(r#"
try {{
    $rules = Get-NetFirewallRule -Enabled True -ErrorAction Stop |
        Where-Object {{
            $_.DisplayGroup -notmatch '^(@|Core Networking|Windows|File and Printer)' -and
            $_.Owner -eq $null
        }} | Select-Object -First {n} DisplayName, Direction, Action, Profile
    "TOTAL:" + $rules.Count
    $rules | ForEach-Object {{
        $dir = switch ($_.Direction) {{ 1 {{ "Inbound" }}; 2 {{ "Outbound" }}; default {{ "?" }} }}
        $act = switch ($_.Action) {{ 2 {{ "Allow" }}; 4 {{ "Block" }}; default {{ "?" }} }}
        $_.DisplayName + "|" + $dir + "|" + $act + "|" + $_.Profile
    }}
}} catch {{ "ERROR:" + $_.Exception.Message }}"#);

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| format!("firewall_rules: {e}"))?;

        let raw = String::from_utf8_lossy(&output.stdout);
        let text = raw.trim();

        if text.starts_with("ERROR:") {
            out.push_str(&format!("Unable to query firewall rules: {}\n", text.trim_start_matches("ERROR:").trim()));
            out.push_str("This query may require running as administrator.\n");
        } else if text.is_empty() {
            out.push_str("No non-default enabled firewall rules found.\n");
        } else {
            let mut total = 0usize;
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("TOTAL:") {
                    total = rest.trim().parse().unwrap_or(0);
                    out.push_str(&format!("Non-default enabled rules (showing up to {n}):\n\n"));
                } else {
                    let parts: Vec<&str> = line.splitn(4, '|').collect();
                    if parts.len() >= 3 {
                        let name = parts[0];
                        let dir = parts[1];
                        let action = parts[2];
                        let profile = parts.get(3).unwrap_or(&"Any");
                        let icon = if action == "Block" { "[!]" } else { "   " };
                        out.push_str(&format!("  {icon} [{dir}] {action}: {name} (profile: {profile})\n"));
                    }
                }
            }
            if total == 0 {
                out.push_str("No non-default enabled rules found.\n");
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(o) = Command::new("ufw").args(["status", "numbered"]).output() {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !text.is_empty() { out.push_str(&text); out.push('\n'); }
        } else if let Ok(o) = Command::new("iptables").args(["-L", "-n", "--line-numbers"]).output() {
            let text = String::from_utf8_lossy(&o.stdout);
            for l in text.lines().take(n * 2) { out.push_str(&format!("  {l}\n")); }
        } else {
            out.push_str("ufw and iptables not available or insufficient permissions.\n");
        }
    }

    Ok(out.trim_end().to_string())
}

// ── traceroute ────────────────────────────────────────────────────────────────

fn inspect_traceroute(host: &str, max_entries: usize) -> Result<String, String> {
    let mut out = format!("Host inspection: traceroute\n\nTarget: {host}\n\n");
    let hops = max_entries.clamp(5, 30);

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("tracert")
            .args(["-d", "-h", &hops.to_string(), host])
            .output()
            .map_err(|e| format!("tracert: {e}"))?;
        let raw = String::from_utf8_lossy(&output.stdout);
        let mut hop_count = 0usize;
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(|c: char| c.is_ascii_digit()) {
                hop_count += 1;
                out.push_str(&format!("  {trimmed}\n"));
            } else if trimmed.starts_with("Tracing") || trimmed.starts_with("Trace complete") {
                out.push_str(&format!("{trimmed}\n"));
            }
        }
        if hop_count == 0 {
            out.push_str("No hops returned — host may be unreachable or ICMP is blocked.\n");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let cmd = if std::path::Path::new("/usr/bin/traceroute").exists()
            || std::path::Path::new("/usr/sbin/traceroute").exists()
        {
            "traceroute"
        } else {
            "tracepath"
        };
        let output = Command::new(cmd)
            .args(["-m", &hops.to_string(), "-n", host])
            .output()
            .map_err(|e| format!("{cmd}: {e}"))?;
        let raw = String::from_utf8_lossy(&output.stdout);
        let mut hop_count = 0usize;
        for line in raw.lines().take(hops + 2) {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                hop_count += 1;
                out.push_str(&format!("  {trimmed}\n"));
            }
        }
        if hop_count == 0 {
            out.push_str("No hops returned — host may be unreachable or ICMP is blocked.\n");
        }
    }

    Ok(out.trim_end().to_string())
}

// ── dns_cache ─────────────────────────────────────────────────────────────────

fn inspect_dns_cache(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: dns_cache\n\n");
    let n = max_entries.clamp(10, 100);

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-DnsClientCache | Select-Object -First 200 Entry,RecordType,Data,TimeToLive | ConvertTo-Csv -NoTypeInformation",
            ])
            .output()
            .map_err(|e| format!("dns_cache: {e}"))?;

        let raw = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = raw.lines().skip(1).collect();
        let total = lines.len();

        if total == 0 {
            out.push_str("DNS cache is empty or could not be read.\n");
        } else {
            out.push_str(&format!("DNS cache entries (showing up to {n} of {total}):\n\n"));
            let mut shown = 0usize;
            for line in lines.iter().take(n) {
                let cols: Vec<&str> = line.splitn(4, ',').collect();
                if cols.len() >= 3 {
                    let entry = cols[0].trim_matches('"');
                    let rtype = cols[1].trim_matches('"');
                    let data  = cols[2].trim_matches('"');
                    let ttl   = cols.get(3).map(|s| s.trim_matches('"')).unwrap_or("?");
                    out.push_str(&format!("  {entry:<45} {rtype:<6} {data}  (TTL {ttl}s)\n"));
                    shown += 1;
                }
            }
            if total > shown {
                out.push_str(&format!("\n  ... and {} more entries\n", total - shown));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(o) = Command::new("resolvectl").args(["statistics"]).output() {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !text.is_empty() {
                out.push_str("systemd-resolved statistics:\n");
                for line in text.lines().take(n) {
                    out.push_str(&format!("  {line}\n"));
                }
                out.push('\n');
            }
        }
        if let Ok(o) = Command::new("dscacheutil").args(["-cachedump", "-entries", "Host"]).output() {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !text.is_empty() {
                out.push_str("DNS cache (macOS dscacheutil):\n");
                for line in text.lines().take(n) {
                    out.push_str(&format!("  {line}\n"));
                }
            } else {
                out.push_str("DNS cache is empty or not accessible on this platform.\n");
            }
        } else {
            out.push_str("DNS cache inspection not available (no resolvectl or dscacheutil found).\n");
        }
    }

    Ok(out.trim_end().to_string())
}

// ── arp ───────────────────────────────────────────────────────────────────────

fn inspect_arp() -> Result<String, String> {
    let mut out = String::from("Host inspection: arp\n\n");

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("arp").args(["-a"]).output().map_err(|e| format!("arp: {e}"))?;
        let raw = String::from_utf8_lossy(&output.stdout);
        let mut count = 0usize;
        for line in raw.lines() {
            let t = line.trim();
            if t.is_empty() { continue; }
            out.push_str(&format!("  {t}\n"));
            if t.contains("dynamic") || t.contains("static") { count += 1; }
        }
        out.push_str(&format!("\nTotal entries: {count}\n"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(o) = Command::new("arp").args(["-n"]).output() {
            let raw = String::from_utf8_lossy(&o.stdout);
            let mut count = 0usize;
            for line in raw.lines() {
                let t = line.trim();
                if !t.is_empty() { out.push_str(&format!("  {t}\n")); count += 1; }
            }
            out.push_str(&format!("\nTotal entries: {}\n", count.saturating_sub(1)));
        } else if let Ok(o) = Command::new("ip").args(["neigh"]).output() {
            let raw = String::from_utf8_lossy(&o.stdout);
            let mut count = 0usize;
            for line in raw.lines() {
                let t = line.trim();
                if !t.is_empty() { out.push_str(&format!("  {t}\n")); count += 1; }
            }
            out.push_str(&format!("\nTotal entries: {count}\n"));
        } else {
            out.push_str("arp and ip neigh not available.\n");
        }
    }

    Ok(out.trim_end().to_string())
}

// ── route_table ───────────────────────────────────────────────────────────────

fn inspect_route_table(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: route_table\n\n");
    let n = max_entries.clamp(10, 50);

    #[cfg(target_os = "windows")]
    {
        let script = r#"
try {
    $routes = Get-NetRoute -ErrorAction Stop |
        Where-Object { $_.RouteMetric -lt 9000 } |
        Sort-Object RouteMetric |
        Select-Object DestinationPrefix, NextHop, RouteMetric, InterfaceAlias
    "TOTAL:" + $routes.Count
    $routes | ForEach-Object {
        $_.DestinationPrefix + "|" + $_.NextHop + "|" + $_.RouteMetric + "|" + $_.InterfaceAlias
    }
} catch { "ERROR:" + $_.Exception.Message }
"#;
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .map_err(|e| format!("route_table: {e}"))?;
        let raw = String::from_utf8_lossy(&output.stdout);
        let text = raw.trim();

        if text.starts_with("ERROR:") {
            out.push_str(&format!("Unable to read route table: {}\n", text.trim_start_matches("ERROR:").trim()));
        } else {
            let mut shown = 0usize;
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("TOTAL:") {
                    let total: usize = rest.trim().parse().unwrap_or(0);
                    out.push_str(&format!("Routing table (showing up to {n} of {total} routes):\n\n"));
                    out.push_str(&format!("  {:<22} {:<18} {:>8}  Interface\n", "Destination", "Next Hop", "Metric"));
                    out.push_str(&format!("  {}\n", "-".repeat(70)));
                } else if shown < n {
                    let parts: Vec<&str> = line.splitn(4, '|').collect();
                    if parts.len() == 4 {
                        let dest   = parts[0];
                        let hop    = if parts[1].is_empty() || parts[1] == "0.0.0.0" || parts[1] == "::" { "on-link" } else { parts[1] };
                        let metric = parts[2];
                        let iface  = parts[3];
                        out.push_str(&format!("  {dest:<22} {hop:<18} {metric:>8}  {iface}\n"));
                        shown += 1;
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(o) = Command::new("ip").args(["route", "show"]).output() {
            let raw = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = raw.lines().collect();
            let total = lines.len();
            out.push_str(&format!("Routing table (showing up to {n} of {total} routes):\n\n"));
            for line in lines.iter().take(n) {
                out.push_str(&format!("  {line}\n"));
            }
            if total > n {
                out.push_str(&format!("\n  ... and {} more routes\n", total - n));
            }
        } else if let Ok(o) = Command::new("netstat").args(["-rn"]).output() {
            let raw = String::from_utf8_lossy(&o.stdout);
            for line in raw.lines().take(n) {
                out.push_str(&format!("  {line}\n"));
            }
        } else {
            out.push_str("ip route and netstat not available.\n");
        }
    }

    Ok(out.trim_end().to_string())
}
