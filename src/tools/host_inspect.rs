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
        "startup_items" | "startup" | "boot" | "autorun" => inspect_startup_items(max_entries),
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
        "env" | "environment" | "environment_variables" | "env_vars" => inspect_env(max_entries),
        "hosts_file" | "hosts" | "etc_hosts" => inspect_hosts_file(),
        "docker" | "containers" | "docker_status" => inspect_docker(max_entries),
        "wsl" | "wsl_distros" | "subsystem" => inspect_wsl(),
        "ssh" | "ssh_config" | "ssh_status" => inspect_ssh(),
        "installed_software" | "installed" | "programs" | "software" | "packages" => inspect_installed_software(max_entries),
        "git_config" | "git_global" => inspect_git_config(),
        "databases" | "database" | "db_services" | "db" => inspect_databases(),
        "user_accounts" | "users" | "local_users" | "accounts" => inspect_user_accounts(max_entries),
        "audit_policy" | "audit" | "auditpol" => inspect_audit_policy(),
        "shares" | "smb_shares" | "network_shares" | "mapped_drives" => inspect_shares(max_entries),
        "dns_servers" | "dns_config" | "dns_resolver" | "nameservers" => inspect_dns_servers(),
        "bitlocker" | "encryption" | "drive_encryption" | "bitlocker_status" => inspect_bitlocker(),
        "rdp" | "remote_desktop" | "rdp_status" => inspect_rdp(),
        "shadow_copies" | "vss" | "volume_shadow" | "backups" | "snapshots" => inspect_shadow_copies(),
        "pagefile" | "page_file" | "virtual_memory" | "swap" => inspect_pagefile(),
        "windows_features" | "optional_features" | "installed_features" | "features" => inspect_windows_features(max_entries),
        "printers" | "printer" | "print_queue" | "printing" => inspect_printers(max_entries),
        "winrm" | "remote_management" | "psremoting" => inspect_winrm(),
        "network_stats" | "adapter_stats" | "nic_stats" | "interface_stats" => inspect_network_stats(max_entries),
        "udp_ports" | "udp_listeners" | "udp" => inspect_udp_ports(max_entries),
        "gpo" | "group_policy" | "applied_policies" => inspect_gpo(),
        "certificates" | "certs" | "ssl_certs" => inspect_certificates(max_entries),
        "integrity" | "sfc" | "dism" | "system_health_deep" => inspect_integrity(),
        "domain" | "active_directory" | "ad_context" | "workgroup" => inspect_domain(),
        "device_health" | "hardware_errors" | "yellow_bangs" => inspect_device_health(),
        "drivers" | "system_drivers" | "driver_list" => inspect_drivers(max_entries),
        "peripherals" | "usb" | "input_devices" | "connected_hardware" => inspect_peripherals(max_entries),
        "sessions" | "logins" | "active_sessions" => inspect_sessions(max_entries),
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
        "disk_benchmark" | "stress_test" | "io_intensity" => {
            let path = resolve_optional_path(args)?;
            inspect_disk_benchmark(path).await
        }
        other => Err(format!(
            "Unknown inspect_host topic '{}'. Use one of: summary, toolchains, path, env_doctor, fix_plan, network, services, processes, desktop, downloads, directory, disk_benchmark, disk, ports, repo_doctor, log_check, startup_items, health_report, storage, hardware, updates, security, pending_reboot, disk_health, battery, recent_crashes, scheduled_tasks, dev_conflicts, connectivity, wifi, connections, vpn, proxy, firewall_rules, traceroute, dns_cache, arp, route_table, os_config, resource_load, env, hosts_file, docker, wsl, ssh, installed_software, git_config, databases, user_accounts, audit_policy, shares, dns_servers, bitlocker, rdp, shadow_copies, pagefile, windows_features, printers, winrm, network_stats, udp_ports, gpo, certificates, integrity, domain, device_health, drivers, peripherals, sessions.",
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
    DriverInstall,
    GroupPolicy,
    FirewallRule,
    SshKey,
    WslSetup,
    ServiceConfig,
    WindowsActivation,
    RegistryEdit,
    ScheduledTaskCreate,
    DiskCleanup,
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
        FixPlanKind::DriverInstall => inspect_driver_install_fix_plan(&issue),
        FixPlanKind::GroupPolicy => inspect_group_policy_fix_plan(&issue),
        FixPlanKind::FirewallRule => inspect_firewall_rule_fix_plan(&issue),
        FixPlanKind::SshKey => inspect_ssh_key_fix_plan(&issue),
        FixPlanKind::WslSetup => inspect_wsl_setup_fix_plan(&issue),
        FixPlanKind::ServiceConfig => inspect_service_config_fix_plan(&issue),
        FixPlanKind::WindowsActivation => inspect_windows_activation_fix_plan(&issue),
        FixPlanKind::RegistryEdit => inspect_registry_edit_fix_plan(&issue),
        FixPlanKind::ScheduledTaskCreate => inspect_scheduled_task_fix_plan(&issue),
        FixPlanKind::DiskCleanup => inspect_disk_cleanup_fix_plan(&issue),
        FixPlanKind::Generic => inspect_generic_fix_plan(&issue),
    }
}

fn classify_fix_plan_kind(issue: &str, port_filter: Option<u16>) -> FixPlanKind {
    let lower = issue.to_ascii_lowercase();
    // FirewallRule must be checked before PortConflict — "open port 80 in the firewall"
    // is firewall rule creation, not a port ownership conflict.
    if lower.contains("firewall rule")
        || lower.contains("inbound rule")
        || lower.contains("outbound rule")
        || (lower.contains("firewall") && (lower.contains("allow") || lower.contains("block") || lower.contains("create") || lower.contains("open")))
    {
        FixPlanKind::FirewallRule
    } else if port_filter.is_some()
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
    } else if lower.contains("driver")
        || lower.contains("gpu driver")
        || lower.contains("nvidia driver")
        || lower.contains("amd driver")
        || lower.contains("install driver")
        || lower.contains("update driver")
    {
        FixPlanKind::DriverInstall
    } else if lower.contains("group policy")
        || lower.contains("gpedit")
        || lower.contains("local policy")
        || lower.contains("secpol")
        || lower.contains("administrative template")
    {
        FixPlanKind::GroupPolicy
    } else if lower.contains("ssh key")
        || lower.contains("ssh-keygen")
        || lower.contains("generate ssh")
        || lower.contains("authorized_keys")
        || lower.contains("id_rsa")
        || lower.contains("id_ed25519")
    {
        FixPlanKind::SshKey
    } else if lower.contains("wsl")
        || lower.contains("windows subsystem for linux")
        || lower.contains("install ubuntu")
        || lower.contains("install linux on windows")
        || lower.contains("wsl2")
    {
        FixPlanKind::WslSetup
    } else if lower.contains("service")
        && (lower.contains("start ")
            || lower.contains("stop ")
            || lower.contains("restart ")
            || lower.contains("enable ")
            || lower.contains("disable ")
            || lower.contains("configure service"))
    {
        FixPlanKind::ServiceConfig
    } else if lower.contains("activate windows")
        || lower.contains("windows activation")
        || lower.contains("product key")
        || lower.contains("kms")
        || lower.contains("not activated")
    {
        FixPlanKind::WindowsActivation
    } else if lower.contains("registry")
        || lower.contains("regedit")
        || lower.contains("hklm")
        || lower.contains("hkcu")
        || lower.contains("reg add")
        || lower.contains("reg delete")
        || lower.contains("registry key")
    {
        FixPlanKind::RegistryEdit
    } else if lower.contains("scheduled task")
        || lower.contains("task scheduler")
        || lower.contains("schtasks")
        || lower.contains("create task")
        || lower.contains("run on startup")
        || lower.contains("run on schedule")
        || lower.contains("cron")
    {
        FixPlanKind::ScheduledTaskCreate
    } else if lower.contains("disk cleanup")
        || lower.contains("free up disk")
        || lower.contains("free up space")
        || lower.contains("clear cache")
        || lower.contains("disk full")
        || lower.contains("low disk space")
        || lower.contains("reclaim space")
    {
        FixPlanKind::DiskCleanup
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

fn inspect_driver_install_fix_plan(issue: &str) -> Result<String, String> {
    // Read GPU info from the hardware topic output for grounding
    #[cfg(target_os = "windows")]
    let gpu_info = {
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-CimInstance Win32_VideoController | Select-Object Name,DriverVersion,DriverDate | ForEach-Object { \"GPU: $($_.Name) | Driver: $($_.DriverVersion) | Date: $($_.DriverDate)\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();
        out.trim().to_string()
    };
    #[cfg(not(target_os = "windows"))]
    let gpu_info = String::from("(GPU detection not available on this platform)");

    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: driver_install\n");
    if !gpu_info.is_empty() {
        out.push_str(&format!("\nDetected GPU(s):\n{}\n", gpu_info));
    }
    out.push_str("\nFix plan — Installing or updating GPU drivers:\n");
    out.push_str("1. Identify your GPU make from the detection above (NVIDIA, AMD, or Intel).\n");
    out.push_str("2. Open Device Manager: press Win+X → Device Manager → expand Display Adapters.\n");
    out.push_str("3. Right-click your GPU → Properties → Driver tab — note the current driver version and date.\n");
    out.push_str("4. Download the latest driver directly from the manufacturer:\n");
    out.push_str("   - NVIDIA: geforce.com/drivers (use GeForce Experience for auto-detection)\n");
    out.push_str("   - AMD: amd.com/support (use Auto-Detect tool)\n");
    out.push_str("   - Intel: intel.com/content/www/us/en/download-center/home.html\n");
    out.push_str("5. Run the downloaded installer. Choose 'Express Install' (keeps settings) or 'Custom / Clean Install' (wipes old driver state — recommended if fixing corruption).\n");
    out.push_str("6. Reboot when prompted — driver installs always require a restart.\n");
    out.push_str("\nVerification:\n");
    out.push_str("- After reboot, run in PowerShell:\n  Get-CimInstance Win32_VideoController | Select-Object Name,DriverVersion,DriverDate\n");
    out.push_str("- The DriverVersion should match what you installed.\n");
    out.push_str("\nWhy this works:\nManufacturer installers handle INF signing, kernel-mode driver registration, and WDDM version negotiation automatically. Manual Device Manager updates often miss supporting components.");
    Ok(out.trim_end().to_string())
}

fn inspect_group_policy_fix_plan(issue: &str) -> Result<String, String> {
    // Check Windows edition — Group Policy editor is not available on Home editions
    #[cfg(target_os = "windows")]
    let edition = {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "(Get-CimInstance Win32_OperatingSystem).Caption",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    #[cfg(not(target_os = "windows"))]
    let edition = String::from("(Windows edition detection not available)");

    let is_home = edition.to_lowercase().contains("home");

    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: group_policy\n");
    out.push_str(&format!("- Windows edition detected: {}\n", if edition.is_empty() { "unknown".to_string() } else { edition.clone() }));

    if is_home {
        out.push_str("\nWARNING: Windows Home does not include the Local Group Policy Editor (gpedit.msc).\n");
        out.push_str("Options on Home edition:\n");
        out.push_str("1. Use the Registry Editor (regedit) as an alternative — most Group Policy settings map to registry keys under HKLM\\SOFTWARE\\Policies or HKCU\\SOFTWARE\\Policies.\n");
        out.push_str("2. Install the gpedit.msc enabler script (third-party — use with caution).\n");
        out.push_str("3. Upgrade to Windows Pro if you need full Group Policy support.\n");
    } else {
        out.push_str("\nFix plan — Editing Local Group Policy:\n");
        out.push_str("1. Press Win+R → type gpedit.msc → press Enter (requires administrator).\n");
        out.push_str("2. Navigate the tree: Computer Configuration (machine-wide) or User Configuration (current user).\n");
        out.push_str("3. Drill into Administrative Templates → find the policy you want.\n");
        out.push_str("4. Double-click a policy → set to Enabled, Disabled, or Not Configured.\n");
        out.push_str("5. Click OK — most policies apply on next logon or after gpupdate.\n");
        out.push_str("6. To force immediate application, run in an elevated PowerShell:\n  gpupdate /force\n");
    }
    out.push_str("\nVerification:\n");
    out.push_str("- Run `gpresult /r` in an elevated command prompt to see applied policies.\n");
    out.push_str("- Or: `Get-GPResultantSetOfPolicy` in PowerShell (requires RSAT on domain machines).\n");
    out.push_str("\nWhy this works:\nGroup Policy writes settings to well-known registry paths that Windows reads at logon and on policy refresh cycles. gpupdate /force triggers an immediate refresh without requiring a restart.");
    Ok(out.trim_end().to_string())
}

fn inspect_firewall_rule_fix_plan(issue: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    let profile_state = {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-NetFirewallProfile | Select-Object Name,Enabled | ForEach-Object { \"$($_.Name): $($_.Enabled)\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    #[cfg(not(target_os = "windows"))]
    let profile_state = String::new();

    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: firewall_rule\n");
    if !profile_state.is_empty() {
        out.push_str(&format!("\nFirewall profile state:\n{}\n", profile_state));
    }
    out.push_str("\nFix plan — Creating or modifying a Windows Firewall rule (PowerShell, run as Administrator):\n");
    out.push_str("\nTo ALLOW inbound traffic on a port:\n");
    out.push_str("  New-NetFirewallRule -DisplayName \"My App Port 8080\" -Direction Inbound -Protocol TCP -LocalPort 8080 -Action Allow -Profile Any\n");
    out.push_str("\nTo BLOCK outbound traffic to an address:\n");
    out.push_str("  New-NetFirewallRule -DisplayName \"Block Example\" -Direction Outbound -RemoteAddress 1.2.3.4 -Action Block\n");
    out.push_str("\nTo ALLOW an application through the firewall:\n");
    out.push_str("  New-NetFirewallRule -DisplayName \"My App\" -Direction Inbound -Program \"C:\\Path\\To\\App.exe\" -Action Allow\n");
    out.push_str("\nTo REMOVE a rule you created:\n");
    out.push_str("  Remove-NetFirewallRule -DisplayName \"My App Port 8080\"\n");
    out.push_str("\nTo see existing custom rules:\n");
    out.push_str("  Get-NetFirewallRule | Where-Object { $_.Enabled -eq 'True' -and $_.PolicyStoreSourceType -ne 'GroupPolicy' } | Select-Object DisplayName,Direction,Action\n");
    out.push_str("\nVerification:\n");
    out.push_str("- After creating the rule, test reachability from another machine or use:\n  Test-NetConnection -ComputerName localhost -Port 8080\n");
    out.push_str("\nWhy this works:\nNew-NetFirewallRule writes directly to the Windows Filtering Platform (WFP) rule store — the same engine used by the Firewall GUI, but scriptable and reproducible.");
    Ok(out.trim_end().to_string())
}

fn inspect_ssh_key_fix_plan(issue: &str) -> Result<String, String> {
    let home = dirs_home().unwrap_or_else(|| std::path::PathBuf::from("~"));
    let ssh_dir = home.join(".ssh");
    let has_ssh_dir = ssh_dir.exists();
    let has_ed25519 = ssh_dir.join("id_ed25519").exists();
    let has_rsa = ssh_dir.join("id_rsa").exists();
    let has_authorized_keys = ssh_dir.join("authorized_keys").exists();

    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: ssh_key\n");
    out.push_str(&format!("- ~/.ssh directory exists: {}\n", has_ssh_dir));
    out.push_str(&format!("- id_ed25519 key found: {}\n", has_ed25519));
    out.push_str(&format!("- id_rsa key found: {}\n", has_rsa));
    out.push_str(&format!("- authorized_keys found: {}\n", has_authorized_keys));

    if has_ed25519 {
        out.push_str("\nYou already have an Ed25519 key. If you want to use it, skip to the 'Add to agent' step.\n");
    }

    out.push_str("\nFix plan — Generating an SSH key pair:\n");
    out.push_str("1. Open PowerShell (or Terminal) — no elevation needed.\n");
    out.push_str("2. Generate an Ed25519 key (preferred over RSA):\n");
    out.push_str("   ssh-keygen -t ed25519 -C \"your@email.com\"\n");
    out.push_str("   - Accept the default path (~/.ssh/id_ed25519) unless you need a custom name.\n");
    out.push_str("   - Set a passphrase (recommended) or press Enter twice for no passphrase.\n");
    out.push_str("3. Start the SSH agent and add your key:\n");
    out.push_str("   # Windows (PowerShell, run as Admin once to enable the service):\n");
    out.push_str("   Set-Service -Name ssh-agent -StartupType Automatic\n");
    out.push_str("   Start-Service ssh-agent\n");
    out.push_str("   # Then add the key (normal PowerShell):\n");
    out.push_str("   ssh-add ~/.ssh/id_ed25519\n");
    out.push_str("4. Copy your PUBLIC key to the target server's authorized_keys:\n");
    out.push_str("   # Print your public key:\n");
    out.push_str("   cat ~/.ssh/id_ed25519.pub\n");
    out.push_str("   # On the target server, append it:\n");
    out.push_str("   echo \"<paste public key>\" >> ~/.ssh/authorized_keys\n");
    out.push_str("   chmod 600 ~/.ssh/authorized_keys\n");
    out.push_str("5. Test the connection:\n");
    out.push_str("   ssh user@server-address\n");
    out.push_str("\nFor GitHub/GitLab:\n");
    out.push_str("- Copy the public key: Get-Content ~/.ssh/id_ed25519.pub | Set-Clipboard\n");
    out.push_str("- Paste it into GitHub Settings → SSH and GPG keys → New SSH key\n");
    out.push_str("- Test: ssh -T git@github.com\n");
    out.push_str("\nWhy this works:\nEd25519 keys use elliptic-curve cryptography — shorter than RSA, harder to brute-force, and supported by all modern SSH servers. The agent caches the decrypted key so you only enter the passphrase once per session.");
    Ok(out.trim_end().to_string())
}

fn inspect_wsl_setup_fix_plan(issue: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    let wsl_status = {
        let out = Command::new("wsl")
            .args(["--status"])
            .output()
            .ok()
            .and_then(|o| {
                let stdout = String::from_utf8(o.stdout).unwrap_or_default();
                let stderr = String::from_utf8(o.stderr).unwrap_or_default();
                Some(format!("{}{}", stdout, stderr))
            })
            .unwrap_or_default();
        out.trim().to_string()
    };
    #[cfg(not(target_os = "windows"))]
    let wsl_status = String::new();

    let wsl_installed = !wsl_status.is_empty() && !wsl_status.to_lowercase().contains("not installed");

    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: wsl_setup\n");
    out.push_str(&format!("- WSL already installed: {}\n", wsl_installed));
    if !wsl_status.is_empty() {
        out.push_str(&format!("- WSL status:\n{}\n", wsl_status));
    }

    if wsl_installed {
        out.push_str("\nWSL is already installed. To install a new Linux distro:\n");
        out.push_str("1. Run in PowerShell (Admin): wsl --install -d Ubuntu\n");
        out.push_str("   Available distros: wsl --list --online\n");
        out.push_str("2. After install, launch from Start menu or type 'ubuntu' in PowerShell.\n");
        out.push_str("3. Create your Linux username and password when prompted.\n");
    } else {
        out.push_str("\nFix plan — Installing WSL2 (Windows Subsystem for Linux):\n");
        out.push_str("1. Open PowerShell as Administrator.\n");
        out.push_str("2. Install WSL with the default Ubuntu distro:\n");
        out.push_str("   wsl --install\n");
        out.push_str("   (This enables the required Windows features, downloads WSL2, and installs Ubuntu)\n");
        out.push_str("3. Reboot when prompted — WSL requires a restart after the first install.\n");
        out.push_str("4. After reboot, Ubuntu will launch automatically and ask you to create a username and password.\n");
        out.push_str("5. Set WSL2 as the default version (should already be set, but confirm):\n");
        out.push_str("   wsl --set-default-version 2\n");
        out.push_str("\nTo install a different distro instead of Ubuntu:\n");
        out.push_str("   wsl --install -d Debian\n");
        out.push_str("   wsl --list --online   # to see all available distros\n");
    }
    out.push_str("\nVerification:\n");
    out.push_str("- Run: wsl --list --verbose\n");
    out.push_str("- You should see your distro with State: Running and Version: 2\n");
    out.push_str("\nWhy this works:\nWSL2 runs a real Linux kernel inside a lightweight Hyper-V VM. The `wsl --install` command handles all the Windows feature enablement, kernel download, and distro bootstrapping automatically.");
    Ok(out.trim_end().to_string())
}

fn inspect_service_config_fix_plan(issue: &str) -> Result<String, String> {
    let lower = issue.to_ascii_lowercase();
    // Extract service name hints from the issue text
    let service_hint = if lower.contains("ssh") {
        Some("sshd")
    } else if lower.contains("mysql") {
        Some("MySQL80")
    } else if lower.contains("postgres") || lower.contains("postgresql") {
        Some("postgresql")
    } else if lower.contains("redis") {
        Some("Redis")
    } else if lower.contains("nginx") {
        Some("nginx")
    } else if lower.contains("apache") {
        Some("Apache2.4")
    } else {
        None
    };

    #[cfg(target_os = "windows")]
    let service_state = if let Some(svc) = service_hint {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("Get-Service -Name '{}' -ErrorAction SilentlyContinue | Select-Object Name,Status,StartType | ForEach-Object {{ \"Service: $($_.Name) | Status: $($_.Status) | StartType: $($_.StartType)\" }}", svc),
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default()
            .trim()
            .to_string()
    } else {
        String::new()
    };
    #[cfg(not(target_os = "windows"))]
    let service_state = String::new();

    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: service_config\n");
    if let Some(svc) = service_hint {
        out.push_str(&format!("- Service detected in request: {}\n", svc));
    }
    if !service_state.is_empty() {
        out.push_str(&format!("- Current state: {}\n", service_state));
    }

    out.push_str("\nFix plan — Managing Windows services (PowerShell, run as Administrator):\n");
    out.push_str("\nStart a service:\n");
    out.push_str("  Start-Service -Name \"ServiceName\"\n");
    out.push_str("\nStop a service:\n");
    out.push_str("  Stop-Service -Name \"ServiceName\"\n");
    out.push_str("\nRestart a service:\n");
    out.push_str("  Restart-Service -Name \"ServiceName\"\n");
    out.push_str("\nEnable a service to start automatically:\n");
    out.push_str("  Set-Service -Name \"ServiceName\" -StartupType Automatic\n");
    out.push_str("\nDisable a service (stops it from auto-starting):\n");
    out.push_str("  Set-Service -Name \"ServiceName\" -StartupType Disabled\n");
    out.push_str("\nFind the exact service name:\n");
    out.push_str("  Get-Service | Where-Object { $_.DisplayName -like '*mysql*' }\n");
    out.push_str("\nVerification:\n");
    out.push_str("  Get-Service -Name \"ServiceName\" | Select-Object Name,Status,StartType\n");
    if let Some(svc) = service_hint {
        out.push_str(&format!("\nFor your detected service ({}):\n  Get-Service -Name '{}'\n", svc, svc));
    }
    out.push_str("\nWhy this works:\nPowerShell's service cmdlets talk directly to the Windows Service Control Manager (SCM) — the same authority that manages auto-start, stop, and dependency resolution for all registered Windows services.");
    Ok(out.trim_end().to_string())
}

fn inspect_windows_activation_fix_plan(issue: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    let activation_status = {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-CimInstance SoftwareLicensingProduct -Filter \"Name like 'Windows%'\" | Where-Object { $_.PartialProductKey } | Select-Object Name,LicenseStatus | ForEach-Object { \"Product: $($_.Name) | Status: $(if ($_.LicenseStatus -eq 1) { 'LICENSED' } else { 'NOT LICENSED (code ' + $_.LicenseStatus + ')' })\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    #[cfg(not(target_os = "windows"))]
    let activation_status = String::new();

    let is_licensed = activation_status.to_lowercase().contains("licensed")
        && !activation_status.to_lowercase().contains("not licensed");

    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: windows_activation\n");
    if !activation_status.is_empty() {
        out.push_str(&format!("- Current activation state:\n{}\n", activation_status));
    }

    if is_licensed {
        out.push_str("\nWindows appears to be activated. If you are still seeing activation prompts, try:\n");
        out.push_str("1. Run in elevated PowerShell: slmgr /ato\n");
        out.push_str("   (Forces an online activation attempt)\n");
        out.push_str("2. Check activation details: slmgr /dli\n");
    } else {
        out.push_str("\nFix plan — Activating Windows:\n");
        out.push_str("1. Check your current status first:\n");
        out.push_str("   slmgr /dli   (basic info)\n");
        out.push_str("   slmgr /dlv   (detailed — shows remaining rearms, grace period)\n");
        out.push_str("\n2. If you have a retail product key:\n");
        out.push_str("   slmgr /ipk XXXXX-XXXXX-XXXXX-XXXXX-XXXXX   (install key)\n");
        out.push_str("   slmgr /ato                                   (activate online)\n");
        out.push_str("\n3. If you had a digital license (linked to your Microsoft account):\n");
        out.push_str("   - Go to Settings → System → Activation\n");
        out.push_str("   - Click 'Troubleshoot' → 'I changed hardware on this device recently'\n");
        out.push_str("   - Sign in with the Microsoft account that holds the license\n");
        out.push_str("\n4. If using a volume license (organization/enterprise):\n");
        out.push_str("   - Contact your IT department for the KMS server address\n");
        out.push_str("   - Set KMS host: slmgr /skms kms.yourorg.com\n");
        out.push_str("   - Activate:    slmgr /ato\n");
    }
    out.push_str("\nVerification:\n");
    out.push_str("  slmgr /dli   — should show 'License Status: Licensed'\n");
    out.push_str("  Or: Settings → System → Activation → 'Windows is activated'\n");
    out.push_str("\nWhy this works:\nslmgr.vbs is the Software License Manager — Microsoft's official command-line tool for all Windows license operations. It talks directly to the Software Protection Platform service.");
    Ok(out.trim_end().to_string())
}

fn inspect_registry_edit_fix_plan(issue: &str) -> Result<String, String> {
    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: registry_edit\n");
    out.push_str("\nCAUTION: Registry edits affect core Windows behavior. Always back up before editing.\n");
    out.push_str("\nFix plan — Safely editing the Windows Registry:\n");
    out.push_str("\n1. Back up before you touch anything:\n");
    out.push_str("   # Export the key you're about to change (PowerShell):\n");
    out.push_str("   reg export \"HKLM\\SOFTWARE\\MyKey\" C:\\backup\\MyKey_backup.reg\n");
    out.push_str("   # Or export the whole registry (takes a while):\n");
    out.push_str("   reg export HKLM C:\\backup\\HKLM_full.reg\n");
    out.push_str("\n2. Read a value (PowerShell, no elevation needed for HKCU):\n");
    out.push_str("   Get-ItemProperty -Path 'HKLM:\\SOFTWARE\\MyKey' -Name 'MyValue'\n");
    out.push_str("\n3. Create or update a DWORD value (PowerShell, Admin for HKLM):\n");
    out.push_str("   Set-ItemProperty -Path 'HKLM:\\SOFTWARE\\MyKey' -Name 'MyValue' -Value 1 -Type DWord\n");
    out.push_str("\n4. Create a new key:\n");
    out.push_str("   New-Item -Path 'HKLM:\\SOFTWARE\\MyNewKey' -Force\n");
    out.push_str("\n5. Delete a value:\n");
    out.push_str("   Remove-ItemProperty -Path 'HKLM:\\SOFTWARE\\MyKey' -Name 'MyValue'\n");
    out.push_str("\n6. Restore from backup if something breaks:\n");
    out.push_str("   reg import C:\\backup\\MyKey_backup.reg\n");
    out.push_str("\nCommon registry hives:\n");
    out.push_str("  HKLM = HKEY_LOCAL_MACHINE  (machine-wide, requires Admin)\n");
    out.push_str("  HKCU = HKEY_CURRENT_USER   (current user, no elevation needed)\n");
    out.push_str("  HKCR = HKEY_CLASSES_ROOT    (file associations)\n");
    out.push_str("\nVerification:\n");
    out.push_str("  Get-ItemProperty -Path 'HKLM:\\SOFTWARE\\MyKey' | Select-Object MyValue\n");
    out.push_str("\nWhy this works:\nPowerShell's registry provider (HKLM:, HKCU:) is the safest scripted way to edit the registry — it validates paths and types, unlike raw reg.exe which accepts anything silently.");
    Ok(out.trim_end().to_string())
}

fn inspect_scheduled_task_fix_plan(issue: &str) -> Result<String, String> {
    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: scheduled_task_create\n");
    out.push_str("\nFix plan — Creating a Scheduled Task (PowerShell, run as Administrator):\n");
    out.push_str("\nExample: Run a script at 9 AM every day\n");
    out.push_str("  $action  = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument '-File C:\\Scripts\\MyScript.ps1'\n");
    out.push_str("  $trigger = New-ScheduledTaskTrigger -Daily -At '09:00AM'\n");
    out.push_str("  Register-ScheduledTask -TaskName 'MyDailyTask' -Action $action -Trigger $trigger -RunLevel Highest\n");
    out.push_str("\nExample: Run at Windows startup\n");
    out.push_str("  $trigger = New-ScheduledTaskTrigger -AtStartup\n");
    out.push_str("  Register-ScheduledTask -TaskName 'MyStartupTask' -Action $action -Trigger $trigger -RunLevel Highest\n");
    out.push_str("\nExample: Run at user logon\n");
    out.push_str("  $trigger = New-ScheduledTaskTrigger -AtLogon\n");
    out.push_str("  Register-ScheduledTask -TaskName 'MyLogonTask' -Action $action -Trigger $trigger\n");
    out.push_str("\nExample: Run every 30 minutes\n");
    out.push_str("  $trigger = New-ScheduledTaskTrigger -RepetitionInterval (New-TimeSpan -Minutes 30) -Once -At (Get-Date)\n");
    out.push_str("\nView all tasks:\n");
    out.push_str("  Get-ScheduledTask | Select-Object TaskName,State | Sort-Object TaskName\n");
    out.push_str("\nDelete a task:\n");
    out.push_str("  Unregister-ScheduledTask -TaskName 'MyDailyTask' -Confirm:$false\n");
    out.push_str("\nRun a task immediately:\n");
    out.push_str("  Start-ScheduledTask -TaskName 'MyDailyTask'\n");
    out.push_str("\nVerification:\n");
    out.push_str("  Get-ScheduledTask -TaskName 'MyDailyTask' | Select-Object TaskName,State,LastRunTime,NextRunTime\n");
    out.push_str("\nWhy this works:\nPowerShell's ScheduledTask cmdlets use the Task Scheduler COM interface — the same engine as the Task Scheduler GUI (taskschd.msc). Tasks persist in the Windows Task Scheduler database across reboots.");
    Ok(out.trim_end().to_string())
}

fn inspect_disk_cleanup_fix_plan(issue: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    let disk_info = {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-PSDrive -PSProvider FileSystem | Select-Object Name,@{N='Used_GB';E={[Math]::Round($_.Used/1GB,1)}},@{N='Free_GB';E={[Math]::Round($_.Free/1GB,1)}} | Where-Object { $_.Used_GB -gt 0 } | ForEach-Object { \"Drive $($_.Name): Used $($_.Used_GB) GB, Free $($_.Free_GB) GB\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    #[cfg(not(target_os = "windows"))]
    let disk_info = String::new();

    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: disk_cleanup\n");
    if !disk_info.is_empty() {
        out.push_str(&format!("\nCurrent drive usage:\n{}\n", disk_info));
    }
    out.push_str("\nFix plan — Reclaiming disk space (ordered by impact):\n");
    out.push_str("\n1. Run Windows Disk Cleanup (built-in, GUI):\n");
    out.push_str("   cleanmgr /sageset:1    (configure what to clean)\n");
    out.push_str("   cleanmgr /sagerun:1    (run the cleanup)\n");
    out.push_str("   Tick 'Windows Update Cleanup' for the biggest reclaim (often 5-20 GB).\n");
    out.push_str("\n2. Clear the Windows Update cache (PowerShell, Admin):\n");
    out.push_str("   Stop-Service wuauserv\n");
    out.push_str("   Remove-Item C:\\Windows\\SoftwareDistribution\\Download\\* -Recurse -Force\n");
    out.push_str("   Start-Service wuauserv\n");
    out.push_str("\n3. Clear Windows Temp folder:\n");
    out.push_str("   Remove-Item $env:TEMP\\* -Recurse -Force -ErrorAction SilentlyContinue\n");
    out.push_str("   Remove-Item C:\\Windows\\Temp\\* -Recurse -Force -ErrorAction SilentlyContinue\n");
    out.push_str("\n4. Developer cache directories (often the biggest culprits):\n");
    out.push_str("   - Rust build artifacts: cargo clean  (inside each project)\n");
    out.push_str("   - npm cache:  npm cache clean --force\n");
    out.push_str("   - pip cache:  pip cache purge\n");
    out.push_str("   - Docker:     docker system prune -a  (removes all unused images/containers)\n");
    out.push_str("   - Cargo registry cache: Remove-Item ~\\.cargo\\registry -Recurse -Force  (will redownload on next build)\n");
    out.push_str("\n5. Check for large files:\n");
    out.push_str("   Get-ChildItem C:\\ -Recurse -ErrorAction SilentlyContinue | Sort-Object Length -Descending | Select-Object -First 20 FullName,@{N='MB';E={[Math]::Round($_.Length/1MB,1)}}\n");
    out.push_str("\nVerification:\n");
    out.push_str("  Get-PSDrive C | Select-Object @{N='Free_GB';E={[Math]::Round($_.Free/1GB,1)}}\n");
    out.push_str("\nWhy this works:\nWindows accumulates update packages, temp files, and developer build artifacts over months. Targeting those specific locations gives the most space back with the least risk of breaking anything.");
    Ok(out.trim_end().to_string())
}

fn inspect_generic_fix_plan(issue: &str) -> Result<String, String> {
    let mut out = String::from("Host inspection: fix_plan\n\n");
    out.push_str(&format!("- Requested issue: {}\n", issue));
    out.push_str("- Fix-plan type: generic\n");
    out.push_str(
        "\nGuidance:\n- Use `fix_plan` with a descriptive issue string to get a grounded, machine-specific walkthrough.\n\
         Structured lanes available:\n\
         - PATH/toolchain drift (cargo, rustc, node, python, winget, choco, scoop)\n\
         - Port conflict (address already in use, what owns port)\n\
         - LM Studio connectivity (localhost:1234, no coding model loaded, embedding model)\n\
         - Driver install (GPU driver, nvidia driver, install driver, update driver)\n\
         - Group Policy (gpedit, local policy, administrative template)\n\
         - Firewall rule (inbound rule, outbound rule, open port, allow port, block port)\n\
         - SSH key (ssh-keygen, generate ssh, authorized_keys)\n\
         - WSL setup (wsl2, windows subsystem for linux, install ubuntu)\n\
         - Service config (start/stop/restart/enable/disable a service)\n\
         - Windows activation (product key, not activated, kms)\n\
         - Registry edit (regedit, reg add, hklm, hkcu, registry key)\n\
         - Scheduled task (task scheduler, schtasks, run on startup, cron)\n\
         - Disk cleanup (free up disk, clear cache, disk full, reclaim space)\n\
         - If your issue is outside these lanes, run the closest `inspect_host` topic first to ground the diagnosis.",
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
        let io_str = if let (Some(r), Some(w)) = (entry.read_ops, entry.write_ops) {
            format!(" [I/O R:{}/W:{}]", r, w)
        } else {
            " [I/O unknown]".to_string()
        };
        out.push_str(&format!(
            "- {} (pid {}) - {}{}{}{}\n",
            entry.name,
            entry.pid,
            human_bytes(entry.memory_bytes),
            cpu_str,
            io_str,
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
        let full_path = cwd.join(&path);

        // Heuristic: If it's a relative path to .hematite or hematite.exe and doesn't exist here,
        // check the user's home directory.
        if !full_path.exists()
            && (trimmed.starts_with(".hematite") || trimmed.starts_with("hematite.exe"))
        {
            if let Some(home) = home::home_dir() {
                let home_path = home.join(trimmed);
                if home_path.exists() {
                    return Ok(home_path);
                }
            }
        }

        Ok(full_path)
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
    read_ops: Option<u64>,
    write_ops: Option<u64>,
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
            "Get-Process | Select-Object Name, Id, WorkingSet64, CPU, ReadOperationCount, WriteOperationCount | ConvertTo-Json -Compress",
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
            let read_ops = v["ReadOperationCount"].as_u64();
            let write_ops = v["WriteOperationCount"].as_u64();
            out.push(ProcessEntry {
                name,
                pid,
                memory_bytes,
                cpu_seconds,
                read_ops,
                write_ops,
                detail: None,
            });
        }
    } else if let Some(v) = values.as_object() {
        let name = v["Name"].as_str().unwrap_or("unknown").to_string();
        let pid = v["Id"].as_u64().unwrap_or(0) as u32;
        let memory_bytes = v["WorkingSet64"].as_u64().unwrap_or(0);
        let cpu_seconds = v["CPU"].as_f64();
        let read_ops = v["ReadOperationCount"].as_u64();
        let write_ops = v["WriteOperationCount"].as_u64();
        out.push(ProcessEntry {
            name,
            pid,
            memory_bytes,
            cpu_seconds,
            read_ops,
            write_ops,
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
            read_ops: None,
            write_ops: None,
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

        // 3. Unified Startup Command check (Task Manager style)
        let unified_script = r#"Get-CimInstance Win32_StartupCommand | ForEach-Object { "  $($_.Name): $($_.Command) ($($_.Location))" }"#;
        if let Ok(unified_out) = Command::new("powershell")
            .args(["-NoProfile", "-Command", unified_script])
            .output()
        {
            let unified_text = String::from_utf8_lossy(&unified_out.stdout);
            let trimmed = unified_text.trim();
            if !trimmed.is_empty() {
                out.push_str("\n=== Unified Startup Commands (WMI) ===\n");
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

        // ── Real-time Performance (Latency) ──────────────────────────────────
        let latency_script = "Get-CimInstance Win32_PerfFormattedData_PerfDisk_PhysicalDisk -Filter \"Name='_Total'\" | Select-Object -ExpandProperty AvgDiskQueueLength";
        match Command::new("powershell")
            .args(["-NoProfile", "-Command", latency_script])
            .output()
        {
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !text.is_empty() {
                    out.push_str("\nReal-time Disk Intensity:\n");
                    out.push_str(&format!("  Average Disk Queue Length: {text}\n"));
                    if let Ok(q) = text.parse::<f64>() {
                        if q > 2.0 {
                            out.push_str("  [!] WARNING: High disk latency detected (Queue Length > 2.0)\n");
                        } else {
                            out.push_str("  [~] Disk latency is within healthy bounds.\n");
                        }
                    }
                }
            }
            Err(_) => {}
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

        // Motherboard + BIOS + Virtualization
        let mb_script = r#"$mb = Get-CimInstance Win32_BaseBoard
$bios = Get-CimInstance Win32_BIOS
$cs = Get-CimInstance Win32_ComputerSystem
$proc = Get-CimInstance Win32_Processor | Select-Object -First 1
$virt = "Hypervisor: $($cs.HypervisorPresent)|SLAT: $($proc.SecondLevelAddressTranslationExtensions)"
"$($mb.Manufacturer.Trim()) $($mb.Product.Trim())|BIOS: $($bios.Manufacturer.Trim()) $($bios.SMBIOSBIOSVersion.Trim()) ($($bios.ReleaseDate))|$virt""#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", mb_script])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            let text = text.trim().trim_matches('"');
            let parts: Vec<&str> = text.split('|').collect();
            if parts.len() == 4 {
                out.push_str(&format!(
                    "Motherboard: {}\n{}\nVirtualization: {}, {}\n\n",
                    parts[0].trim(),
                    parts[1].trim(),
                    parts[2].trim(),
                    parts[3].trim()
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

// ── env ───────────────────────────────────────────────────────────────────────

fn inspect_env(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: env\n\n");
    let n = max_entries.clamp(10, 50);

    fn looks_like_secret(name: &str) -> bool {
        let n = name.to_uppercase();
        n.contains("KEY") || n.contains("SECRET") || n.contains("TOKEN")
            || n.contains("PASSWORD") || n.contains("PASSWD") || n.contains("CREDENTIAL")
            || n.contains("AUTH") || n.contains("CERT") || n.contains("PRIVATE")
    }

    let known_dev_vars: &[&str] = &[
        "CARGO_HOME", "RUSTUP_HOME", "GOPATH", "GOROOT", "GOBIN",
        "JAVA_HOME", "ANDROID_HOME", "ANDROID_SDK_ROOT",
        "PYTHONPATH", "PYTHONHOME", "VIRTUAL_ENV", "CONDA_DEFAULT_ENV", "CONDA_PREFIX",
        "NODE_PATH", "NVM_DIR", "NVM_BIN", "PNPM_HOME",
        "DENO_INSTALL", "DENO_DIR",
        "DOTNET_ROOT", "NUGET_PACKAGES",
        "CMAKE_HOME", "VCPKG_ROOT",
        "AWS_PROFILE", "AWS_REGION", "AWS_DEFAULT_REGION",
        "GCP_PROJECT", "GOOGLE_CLOUD_PROJECT", "GOOGLE_APPLICATION_CREDENTIALS",
        "AZURE_SUBSCRIPTION_ID",
        "DATABASE_URL", "REDIS_URL", "MONGO_URI",
        "EDITOR", "VISUAL", "SHELL", "TERM",
        "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME",
        "HOME", "USERPROFILE", "APPDATA", "LOCALAPPDATA", "TEMP", "TMP",
        "COMPUTERNAME", "USERNAME", "USERDOMAIN",
        "PROCESSOR_ARCHITECTURE", "NUMBER_OF_PROCESSORS",
        "OS", "HOMEDRIVE", "HOMEPATH",
        "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY", "ALL_PROXY",
        "http_proxy", "https_proxy", "no_proxy",
        "DOCKER_HOST", "DOCKER_BUILDKIT",
        "COMPOSE_PROJECT_NAME",
        "KUBECONFIG", "KUBE_CONTEXT",
        "CI", "GITHUB_ACTIONS", "GITLAB_CI",
        "LMSTUDIO_HOME", "HEMATITE_URL",
    ];

    let mut all_vars: Vec<(String, String)> = std::env::vars().collect();
    all_vars.sort_by(|a, b| a.0.cmp(&b.0));
    let total = all_vars.len();

    let mut dev_found: Vec<String> = Vec::new();
    let mut secret_found: Vec<String> = Vec::new();

    for (k, v) in &all_vars {
        if k == "PATH" { continue; }
        if looks_like_secret(k) {
            secret_found.push(format!("{k} = [SET, {} chars]", v.len()));
        } else {
            let k_upper = k.to_uppercase();
            let is_known = known_dev_vars.iter().any(|kv| k_upper.as_str() == kv.to_uppercase().as_str());
            if is_known {
                let display = if v.len() > 120 {
                    format!("{k} = {}…", &v[..117])
                } else {
                    format!("{k} = {v}")
                };
                dev_found.push(display);
            }
        }
    }

    out.push_str(&format!("Total environment variables: {total}\n\n"));

    if let Ok(p) = std::env::var("PATH") {
        let sep = if cfg!(target_os = "windows") { ';' } else { ':' };
        let count = p.split(sep).count();
        out.push_str(&format!("PATH: {count} entries (use topic=path for full audit)\n\n"));
    }

    if !secret_found.is_empty() {
        out.push_str(&format!(
            "=== Secret/credential variables ({} detected, values hidden) ===\n",
            secret_found.len()
        ));
        for s in secret_found.iter().take(n) {
            out.push_str(&format!("  {s}\n"));
        }
        out.push('\n');
    }

    if !dev_found.is_empty() {
        out.push_str(&format!("=== Developer & tool variables ({}) ===\n", dev_found.len()));
        for d in dev_found.iter().take(n) {
            out.push_str(&format!("  {d}\n"));
        }
        out.push('\n');
    }

    let other_count = all_vars.iter().filter(|(k, _)| {
        k != "PATH"
            && !looks_like_secret(k)
            && !known_dev_vars.iter().any(|kv| k.to_uppercase().as_str() == kv.to_uppercase().as_str())
    }).count();
    if other_count > 0 {
        out.push_str(&format!(
            "Other variables: {other_count} (use 'env' in shell to see all)\n"
        ));
    }

    Ok(out.trim_end().to_string())
}

// ── hosts_file ────────────────────────────────────────────────────────────────

fn inspect_hosts_file() -> Result<String, String> {
    let mut out = String::from("Host inspection: hosts_file\n\n");

    let hosts_path = if cfg!(target_os = "windows") {
        std::path::PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
    } else {
        std::path::PathBuf::from("/etc/hosts")
    };

    out.push_str(&format!("Path: {}\n\n", hosts_path.display()));

    match fs::read_to_string(&hosts_path) {
        Ok(content) => {
            let mut active_entries: Vec<String> = Vec::new();
            let mut comment_lines = 0usize;
            let mut blank_lines = 0usize;

            for line in content.lines() {
                let t = line.trim();
                if t.is_empty() {
                    blank_lines += 1;
                } else if t.starts_with('#') {
                    comment_lines += 1;
                } else {
                    active_entries.push(line.to_string());
                }
            }

            out.push_str(&format!(
                "Active entries: {}  |  Comment lines: {}  |  Blank lines: {}\n\n",
                active_entries.len(),
                comment_lines,
                blank_lines
            ));

            if active_entries.is_empty() {
                out.push_str(
                    "No active host entries (file contains only comments/blanks — standard default state).\n",
                );
            } else {
                out.push_str("=== Active entries ===\n");
                for entry in &active_entries {
                    out.push_str(&format!("  {entry}\n"));
                }
                out.push('\n');

                let custom: Vec<&String> = active_entries
                    .iter()
                    .filter(|e| {
                        let t = e.trim_start();
                        !t.starts_with("127.")
                            && !t.starts_with("::1")
                            && !t.starts_with("0.0.0.0")
                    })
                    .collect();
                if !custom.is_empty() {
                    out.push_str(&format!(
                        "[!] Custom (non-loopback) entries: {}\n",
                        custom.len()
                    ));
                    for e in &custom {
                        out.push_str(&format!("  {e}\n"));
                    }
                } else {
                    out.push_str(
                        "All active entries are standard loopback or block entries.\n",
                    );
                }
            }

            out.push_str("\n=== Full file ===\n");
            for line in content.lines() {
                out.push_str(&format!("  {line}\n"));
            }
        }
        Err(e) => {
            out.push_str(&format!("Could not read hosts file: {e}\n"));
            if cfg!(target_os = "windows") {
                out.push_str(
                    "On Windows, run Hematite as Administrator if permission is denied.\n",
                );
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── docker ────────────────────────────────────────────────────────────────────

fn inspect_docker(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: docker\n\n");
    let n = max_entries.clamp(5, 25);

    let version_output = Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output();

    match version_output {
        Err(_) => {
            out.push_str("Docker: not found on PATH.\n");
            out.push_str(
                "Install Docker Desktop: https://www.docker.com/products/docker-desktop\n",
            );
            return Ok(out.trim_end().to_string());
        }
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            if stderr.contains("cannot connect")
                || stderr.contains("Is the docker daemon running")
                || stderr.contains("pipe")
                || stderr.contains("socket")
            {
                out.push_str("Docker: installed but daemon is NOT running.\n");
                out.push_str("Start Docker Desktop or run: sudo systemctl start docker\n");
            } else {
                out.push_str(&format!("Docker: error — {}\n", stderr.trim()));
            }
            return Ok(out.trim_end().to_string());
        }
        Ok(o) => {
            let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
            out.push_str(&format!("Docker Engine: {version}\n"));
        }
    }

    if let Ok(o) = Command::new("docker")
        .args([
            "info",
            "--format",
            "Containers: {{.Containers}} (running: {{.ContainersRunning}}, stopped: {{.ContainersStopped}})\nImages: {{.Images}}\nStorage driver: {{.Driver}}\nOS/Arch: {{.OSType}}/{{.Architecture}}\nCPUs: {{.NCPU}}",
        ])
        .output()
    {
        let info = String::from_utf8_lossy(&o.stdout);
        for line in info.lines() {
            let t = line.trim();
            if !t.is_empty() {
                out.push_str(&format!("  {t}\n"));
            }
        }
        out.push('\n');
    }

    if let Ok(o) = Command::new("docker")
        .args([
            "ps",
            "--format",
            "table {{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}",
        ])
        .output()
    {
        let raw = String::from_utf8_lossy(&o.stdout);
        let lines: Vec<&str> = raw.lines().collect();
        if lines.len() <= 1 {
            out.push_str("Running containers: none\n\n");
        } else {
            out.push_str(&format!(
                "=== Running containers ({}) ===\n",
                lines.len().saturating_sub(1)
            ));
            for line in lines.iter().take(n + 1) {
                out.push_str(&format!("  {line}\n"));
            }
            if lines.len() > n + 1 {
                out.push_str(&format!("  ... and {} more\n", lines.len() - n - 1));
            }
            out.push('\n');
        }
    }

    if let Ok(o) = Command::new("docker")
        .args([
            "images",
            "--format",
            "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}",
        ])
        .output()
    {
        let raw = String::from_utf8_lossy(&o.stdout);
        let lines: Vec<&str> = raw.lines().collect();
        if lines.len() > 1 {
            out.push_str(&format!(
                "=== Local images ({}) ===\n",
                lines.len().saturating_sub(1)
            ));
            for line in lines.iter().take(n + 1) {
                out.push_str(&format!("  {line}\n"));
            }
            if lines.len() > n + 1 {
                out.push_str(&format!("  ... and {} more\n", lines.len() - n - 1));
            }
            out.push('\n');
        }
    }

    if let Ok(o) = Command::new("docker")
        .args([
            "compose",
            "ls",
            "--format",
            "table {{.Name}}\t{{.Status}}\t{{.ConfigFiles}}",
        ])
        .output()
    {
        let raw = String::from_utf8_lossy(&o.stdout);
        let lines: Vec<&str> = raw.lines().collect();
        if lines.len() > 1 {
            out.push_str(&format!(
                "=== Compose projects ({}) ===\n",
                lines.len().saturating_sub(1)
            ));
            for line in lines.iter().take(n + 1) {
                out.push_str(&format!("  {line}\n"));
            }
            out.push('\n');
        }
    }

    if let Ok(o) = Command::new("docker").args(["context", "show"]).output() {
        let ctx = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !ctx.is_empty() {
            out.push_str(&format!("Active context: {ctx}\n"));
        }
    }

    Ok(out.trim_end().to_string())
}

// ── wsl ───────────────────────────────────────────────────────────────────────

fn inspect_wsl() -> Result<String, String> {
    let mut out = String::from("Host inspection: wsl\n\n");

    #[cfg(target_os = "windows")]
    {
        if let Ok(o) = Command::new("wsl").args(["--version"]).output() {
            let raw = String::from_utf8_lossy(&o.stdout);
            let cleaned: String = raw.chars().filter(|c| *c != '\0').collect();
            for line in cleaned.lines().take(4) {
                let t = line.trim();
                if !t.is_empty() {
                    out.push_str(&format!("  {t}\n"));
                }
            }
            out.push('\n');
        }

        let list_output = Command::new("wsl").args(["--list", "--verbose"]).output();
        match list_output {
            Err(e) => {
                out.push_str(&format!("WSL: wsl.exe error: {e}\n"));
                out.push_str("WSL may not be installed. Enable with: wsl --install\n");
            }
            Ok(o) if !o.status.success() => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let cleaned: String = stderr.chars().filter(|c| *c != '\0').collect();
                out.push_str(&format!("WSL: error — {}\n", cleaned.trim()));
                out.push_str("Run: wsl --install\n");
            }
            Ok(o) => {
                let raw = String::from_utf8_lossy(&o.stdout);
                let cleaned: String = raw.chars().filter(|c| *c != '\0').collect();
                let lines: Vec<&str> = cleaned
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .collect();
                let distro_lines: Vec<&str> = lines
                    .iter()
                    .filter(|l| {
                        let t = l.trim();
                        !t.is_empty()
                            && !t.to_uppercase().starts_with("NAME")
                            && !t.starts_with("---")
                    })
                    .copied()
                    .collect();

                if distro_lines.is_empty() {
                    out.push_str("WSL: installed but no distributions found.\n");
                    out.push_str("Install a distro: wsl --install -d Ubuntu\n");
                } else {
                    out.push_str("=== WSL Distributions ===\n");
                    for line in &lines {
                        out.push_str(&format!("  {}\n", line.trim()));
                    }
                    out.push_str(&format!("\nTotal distributions: {}\n", distro_lines.len()));
                }
            }
        }

        if let Ok(o) = Command::new("wsl").args(["--status"]).output() {
            let raw = String::from_utf8_lossy(&o.stdout);
            let cleaned: String = raw.chars().filter(|c| *c != '\0').collect();
            let status_lines: Vec<&str> = cleaned
                .lines()
                .filter(|l| !l.trim().is_empty())
                .take(8)
                .collect();
            if !status_lines.is_empty() {
                out.push_str("\n=== WSL status ===\n");
                for line in status_lines {
                    out.push_str(&format!("  {}\n", line.trim()));
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("WSL (Windows Subsystem for Linux) is a Windows-only feature.\n");
        out.push_str(
            "On Linux/macOS, use native virtualization (KVM, UTM, Parallels) instead.\n",
        );
    }

    Ok(out.trim_end().to_string())
}

// ── ssh ───────────────────────────────────────────────────────────────────────

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

fn inspect_ssh() -> Result<String, String> {
    let mut out = String::from("Host inspection: ssh\n\n");

    if let Ok(o) = Command::new("ssh").args(["-V"]).output() {
        let ver = if o.stdout.is_empty() {
            String::from_utf8_lossy(&o.stderr).trim().to_string()
        } else {
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        };
        if !ver.is_empty() {
            out.push_str(&format!("SSH client: {ver}\n"));
        }
    } else {
        out.push_str("SSH client: not found on PATH.\n");
    }

    #[cfg(target_os = "windows")]
    {
        let script = r#"
$svc = Get-Service -Name sshd -ErrorAction SilentlyContinue
if ($svc) { "SSHD:" + $svc.Status + " | StartType:" + $svc.StartType }
else { "SSHD:not_installed" }
"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if text.contains("not_installed") {
                out.push_str("SSH server (sshd): not installed\n");
            } else {
                out.push_str(&format!(
                    "SSH server (sshd): {}\n",
                    text.trim_start_matches("SSHD:")
                ));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(o) = Command::new("systemctl")
            .args(["is-active", "sshd"])
            .output()
        {
            let status = String::from_utf8_lossy(&o.stdout).trim().to_string();
            out.push_str(&format!("SSH server (sshd): {status}\n"));
        } else if let Ok(o) = Command::new("systemctl")
            .args(["is-active", "ssh"])
            .output()
        {
            let status = String::from_utf8_lossy(&o.stdout).trim().to_string();
            out.push_str(&format!("SSH server (ssh): {status}\n"));
        }
    }

    out.push('\n');

    if let Some(ssh_dir) = dirs_home().map(|h| h.join(".ssh")) {
        if ssh_dir.exists() {
            out.push_str(&format!("~/.ssh: {}\n", ssh_dir.display()));

            let kh = ssh_dir.join("known_hosts");
            if kh.exists() {
                let count = fs::read_to_string(&kh)
                    .map(|c| {
                        c.lines()
                            .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                            .count()
                    })
                    .unwrap_or(0);
                out.push_str(&format!("  known_hosts: {count} entries\n"));
            } else {
                out.push_str("  known_hosts: not present\n");
            }

            let ak = ssh_dir.join("authorized_keys");
            if ak.exists() {
                let count = fs::read_to_string(&ak)
                    .map(|c| {
                        c.lines()
                            .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                            .count()
                    })
                    .unwrap_or(0);
                out.push_str(&format!("  authorized_keys: {count} public keys\n"));
            } else {
                out.push_str("  authorized_keys: not present\n");
            }

            let key_names = [
                "id_rsa",
                "id_ed25519",
                "id_ecdsa",
                "id_dsa",
                "id_ecdsa_sk",
                "id_ed25519_sk",
            ];
            let found_keys: Vec<&str> = key_names
                .iter()
                .filter(|k| ssh_dir.join(k).exists())
                .copied()
                .collect();
            if !found_keys.is_empty() {
                out.push_str(&format!("  Private keys: {}\n", found_keys.join(", ")));
            } else {
                out.push_str("  Private keys: none found\n");
            }

            let config_path = ssh_dir.join("config");
            if config_path.exists() {
                out.push_str("\n=== SSH config hosts ===\n");
                match fs::read_to_string(&config_path) {
                    Ok(content) => {
                        let mut hosts: Vec<(String, Vec<String>)> = Vec::new();
                        let mut current: Option<(String, Vec<String>)> = None;
                        for line in content.lines() {
                            let t = line.trim();
                            if t.is_empty() || t.starts_with('#') {
                                continue;
                            }
                            if let Some(host) = t.strip_prefix("Host ") {
                                if let Some(prev) = current.take() {
                                    hosts.push(prev);
                                }
                                current = Some((host.trim().to_string(), Vec::new()));
                            } else if let Some((_, ref mut details)) = current {
                                let tu = t.to_uppercase();
                                if tu.starts_with("HOSTNAME ")
                                    || tu.starts_with("USER ")
                                    || tu.starts_with("PORT ")
                                    || tu.starts_with("IDENTITYFILE ")
                                {
                                    details.push(t.to_string());
                                }
                            }
                        }
                        if let Some(prev) = current {
                            hosts.push(prev);
                        }

                        if hosts.is_empty() {
                            out.push_str("  No Host entries found.\n");
                        } else {
                            for (h, details) in &hosts {
                                if details.is_empty() {
                                    out.push_str(&format!("  Host {h}\n"));
                                } else {
                                    out.push_str(&format!(
                                        "  Host {h}  [{}]\n",
                                        details.join(", ")
                                    ));
                                }
                            }
                            out.push_str(&format!(
                                "\n  Total configured hosts: {}\n",
                                hosts.len()
                            ));
                        }
                    }
                    Err(e) => out.push_str(&format!("  Could not read config: {e}\n")),
                }
            } else {
                out.push_str("  SSH config: not present\n");
            }
        } else {
            out.push_str("~/.ssh: directory not found (no SSH keys configured).\n");
        }
    }

    Ok(out.trim_end().to_string())
}

// ── installed_software ────────────────────────────────────────────────────────

fn inspect_installed_software(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: installed_software\n\n");
    let n = max_entries.clamp(10, 50);

    #[cfg(target_os = "windows")]
    {
        let winget_out = Command::new("winget")
            .args(["list", "--accept-source-agreements"])
            .output();

        if let Ok(o) = winget_out {
            if o.status.success() {
                let raw = String::from_utf8_lossy(&o.stdout);
                let mut header_done = false;
                let mut packages: Vec<&str> = Vec::new();
                for line in raw.lines() {
                    let t = line.trim();
                    if t.starts_with("---") {
                        header_done = true;
                        continue;
                    }
                    if header_done && !t.is_empty() {
                        packages.push(line);
                    }
                }
                let total = packages.len();
                out.push_str(&format!(
                    "=== Installed software via winget ({total} packages) ===\n\n"
                ));
                for line in packages.iter().take(n) {
                    out.push_str(&format!("  {line}\n"));
                }
                if total > n {
                    out.push_str(&format!("\n  ... and {} more packages\n", total - n));
                }
                out.push_str("\nFor full list: winget list\n");
                return Ok(out.trim_end().to_string());
            }
        }

        // Fallback: registry scan
        let script = format!(
            r#"
$apps = @()
$reg_paths = @(
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
foreach ($p in $reg_paths) {{
    try {{
        $apps += Get-ItemProperty $p -ErrorAction SilentlyContinue |
            Where-Object {{ $_.DisplayName }} |
            Select-Object DisplayName, DisplayVersion, Publisher
    }} catch {{}}
}}
$sorted = $apps | Sort-Object DisplayName -Unique
"TOTAL:" + $sorted.Count
$sorted | Select-Object -First {n} | ForEach-Object {{
    $_.DisplayName + "|" + $_.DisplayVersion + "|" + $_.Publisher
}}
"#
        );
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
        {
            let raw = String::from_utf8_lossy(&o.stdout);
            out.push_str("=== Installed software (registry scan) ===\n");
            out.push_str(&format!(
                "  {:<50} {:<18} Publisher\n",
                "Name", "Version"
            ));
            out.push_str(&format!("  {}\n", "-".repeat(90)));
            for line in raw.lines() {
                if let Some(rest) = line.strip_prefix("TOTAL:") {
                    let total: usize = rest.trim().parse().unwrap_or(0);
                    out.push_str(&format!(
                        "  (Total: {total}, showing first {n})\n\n"
                    ));
                } else if !line.trim().is_empty() {
                    let parts: Vec<&str> = line.splitn(3, '|').collect();
                    let name = parts.first().map(|s| s.trim()).unwrap_or("");
                    let ver = parts.get(1).map(|s| s.trim()).unwrap_or("");
                    let pub_ = parts.get(2).map(|s| s.trim()).unwrap_or("");
                    out.push_str(&format!("  {:<50} {:<18} {pub_}\n", name, ver));
                }
            }
        } else {
            out.push_str(
                "Could not query installed software (winget and registry scan both failed).\n",
            );
        }
    }

    #[cfg(target_os = "linux")]
    {
        let mut found = false;
        if let Ok(o) = Command::new("dpkg").args(["--get-selections"]).output() {
            if o.status.success() {
                let raw = String::from_utf8_lossy(&o.stdout);
                let installed: Vec<&str> =
                    raw.lines().filter(|l| l.contains("install")).collect();
                let total = installed.len();
                out.push_str(&format!(
                    "=== Installed packages via dpkg ({total}) ===\n"
                ));
                for line in installed.iter().take(n) {
                    out.push_str(&format!("  {}\n", line.trim()));
                }
                if total > n {
                    out.push_str(&format!("  ... and {} more\n", total - n));
                }
                out.push_str("\nFor full list: dpkg --get-selections | grep install\n");
                found = true;
            }
        }
        if !found {
            if let Ok(o) = Command::new("rpm")
                .args(["-qa", "--queryformat", "%{NAME} %{VERSION}\n"])
                .output()
            {
                if o.status.success() {
                    let raw = String::from_utf8_lossy(&o.stdout);
                    let lines: Vec<&str> = raw.lines().collect();
                    let total = lines.len();
                    out.push_str(&format!("=== Installed packages via rpm ({total}) ===\n"));
                    for line in lines.iter().take(n) {
                        out.push_str(&format!("  {line}\n"));
                    }
                    if total > n {
                        out.push_str(&format!("  ... and {} more\n", total - n));
                    }
                    found = true;
                }
            }
        }
        if !found {
            if let Ok(o) = Command::new("pacman").args(["-Q"]).output() {
                if o.status.success() {
                    let raw = String::from_utf8_lossy(&o.stdout);
                    let lines: Vec<&str> = raw.lines().collect();
                    let total = lines.len();
                    out.push_str(&format!(
                        "=== Installed packages via pacman ({total}) ===\n"
                    ));
                    for line in lines.iter().take(n) {
                        out.push_str(&format!("  {line}\n"));
                    }
                    if total > n {
                        out.push_str(&format!("  ... and {} more\n", total - n));
                    }
                    found = true;
                }
            }
        }
        if !found {
            out.push_str("No package manager found (tried dpkg, rpm, pacman).\n");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(o) = Command::new("brew").args(["list", "--versions"]).output() {
            if o.status.success() {
                let raw = String::from_utf8_lossy(&o.stdout);
                let lines: Vec<&str> = raw.lines().collect();
                let total = lines.len();
                out.push_str(&format!("=== Homebrew packages ({total}) ===\n"));
                for line in lines.iter().take(n) {
                    out.push_str(&format!("  {line}\n"));
                }
                if total > n {
                    out.push_str(&format!("  ... and {} more\n", total - n));
                }
                out.push_str("\nFor full list: brew list --versions\n");
            }
        } else {
            out.push_str("Homebrew not found.\n");
        }
        if let Ok(o) = Command::new("mas").args(["list"]).output() {
            if o.status.success() {
                let raw = String::from_utf8_lossy(&o.stdout);
                let lines: Vec<&str> = raw.lines().collect();
                out.push_str(&format!(
                    "\n=== Mac App Store apps ({}) ===\n",
                    lines.len()
                ));
                for line in lines.iter().take(n) {
                    out.push_str(&format!("  {line}\n"));
                }
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── git_config ────────────────────────────────────────────────────────────────

fn inspect_git_config() -> Result<String, String> {
    let mut out = String::from("Host inspection: git_config\n\n");

    if let Ok(o) = Command::new("git").args(["--version"]).output() {
        let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
        out.push_str(&format!("Git: {ver}\n\n"));
    } else {
        out.push_str("Git: not found on PATH.\n");
        return Ok(out.trim_end().to_string());
    }

    if let Ok(o) = Command::new("git")
        .args(["config", "--global", "--list"])
        .output()
    {
        if o.status.success() {
            let raw = String::from_utf8_lossy(&o.stdout);
            let mut pairs: Vec<(String, String)> = raw
                .lines()
                .filter_map(|l| {
                    let mut parts = l.splitn(2, '=');
                    let k = parts.next()?.trim().to_string();
                    let v = parts.next().unwrap_or("").trim().to_string();
                    Some((k, v))
                })
                .collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));

            out.push_str("=== Global git config ===\n");

            let sections: &[(&str, &[&str])] = &[
                ("Identity",       &["user.name", "user.email", "user.signingkey"]),
                ("Core",           &["core.editor", "core.autocrlf", "core.eol", "core.ignorecase", "core.filemode"]),
                ("Commit/Signing", &["commit.gpgsign", "tag.gpgsign", "gpg.format", "gpg.ssh.allowedsignersfile"]),
                ("Push/Pull",      &["push.default", "push.autosetupremote", "pull.rebase", "pull.ff"]),
                ("Credential",     &["credential.helper"]),
                ("Branch",         &["init.defaultbranch", "branch.autosetuprebase"]),
            ];

            let mut shown_keys: HashSet<String> = HashSet::new();
            for (section, keys) in sections {
                let mut section_lines: Vec<String> = Vec::new();
                for key in *keys {
                    if let Some((k, v)) = pairs.iter().find(|(kk, _)| kk == key) {
                        section_lines.push(format!("  {k} = {v}"));
                        shown_keys.insert(k.clone());
                    }
                }
                if !section_lines.is_empty() {
                    out.push_str(&format!("\n[{section}]\n"));
                    for line in section_lines {
                        out.push_str(&format!("{line}\n"));
                    }
                }
            }

            let other: Vec<&(String, String)> = pairs
                .iter()
                .filter(|(k, _)| !shown_keys.contains(k) && !k.starts_with("alias."))
                .collect();
            if !other.is_empty() {
                out.push_str("\n[Other]\n");
                for (k, v) in other.iter().take(20) {
                    out.push_str(&format!("  {k} = {v}\n"));
                }
                if other.len() > 20 {
                    out.push_str(&format!("  ... and {} more\n", other.len() - 20));
                }
            }

            out.push_str(&format!("\nTotal global config keys: {}\n", pairs.len()));
        } else {
            out.push_str("No global git config found.\n");
            out.push_str("Set up with:\n");
            out.push_str("  git config --global user.name \"Your Name\"\n");
            out.push_str("  git config --global user.email \"you@example.com\"\n");
        }
    }

    if let Ok(o) = Command::new("git")
        .args(["config", "--local", "--list"])
        .output()
    {
        if o.status.success() {
            let raw = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
            if !lines.is_empty() {
                out.push_str(&format!(
                    "\n=== Local repo config ({} keys) ===\n",
                    lines.len()
                ));
                for line in lines.iter().take(15) {
                    out.push_str(&format!("  {line}\n"));
                }
                if lines.len() > 15 {
                    out.push_str(&format!("  ... and {} more\n", lines.len() - 15));
                }
            }
        }
    }

    if let Ok(o) = Command::new("git")
        .args(["config", "--global", "--get-regexp", r"alias\."])
        .output()
    {
        if o.status.success() {
            let raw = String::from_utf8_lossy(&o.stdout);
            let aliases: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
            if !aliases.is_empty() {
                out.push_str(&format!("\n=== Git aliases ({}) ===\n", aliases.len()));
                for a in aliases.iter().take(20) {
                    out.push_str(&format!("  {a}\n"));
                }
                if aliases.len() > 20 {
                    out.push_str(&format!("  ... and {} more\n", aliases.len() - 20));
                }
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── databases ─────────────────────────────────────────────────────────────────

fn inspect_databases() -> Result<String, String> {
    let mut out = String::from("Host inspection: databases\n\n");
    out.push_str("Scanning for local database engines (service state, port, version)...\n\n");

    struct DbEngine {
        name: &'static str,
        service_names: &'static [&'static str],
        default_port: u16,
        cli_name: &'static str,
        cli_version_args: &'static [&'static str],
    }

    let engines: &[DbEngine] = &[
        DbEngine {
            name: "PostgreSQL",
            service_names: &["postgresql", "postgresql-x64-14", "postgresql-x64-15", "postgresql-x64-16", "postgresql-x64-17"],

            default_port: 5432,
            cli_name: "psql",
            cli_version_args: &["--version"],
        },
        DbEngine {
            name: "MySQL",
            service_names: &["mysql", "mysql80", "mysql57"],

            default_port: 3306,
            cli_name: "mysql",
            cli_version_args: &["--version"],
        },
        DbEngine {
            name: "MariaDB",
            service_names: &["mariadb", "mariadb.exe"],

            default_port: 3306,
            cli_name: "mariadb",
            cli_version_args: &["--version"],
        },
        DbEngine {
            name: "MongoDB",
            service_names: &["mongodb", "mongod"],

            default_port: 27017,
            cli_name: "mongod",
            cli_version_args: &["--version"],
        },
        DbEngine {
            name: "Redis",
            service_names: &["redis", "redis-server"],

            default_port: 6379,
            cli_name: "redis-server",
            cli_version_args: &["--version"],
        },
        DbEngine {
            name: "SQL Server",
            service_names: &["mssqlserver", "mssql$sqlexpress", "mssql$localdb"],

            default_port: 1433,
            cli_name: "sqlcmd",
            cli_version_args: &["-?"],
        },
        DbEngine {
            name: "SQLite",
            service_names: &[],  // no service — file-based

            default_port: 0,     // no port — file-based
            cli_name: "sqlite3",
            cli_version_args: &["--version"],
        },
        DbEngine {
            name: "CouchDB",
            service_names: &["couchdb", "apache-couchdb"],

            default_port: 5984,
            cli_name: "couchdb",
            cli_version_args: &["--version"],
        },
        DbEngine {
            name: "Cassandra",
            service_names: &["cassandra"],

            default_port: 9042,
            cli_name: "cqlsh",
            cli_version_args: &["--version"],
        },
        DbEngine {
            name: "Elasticsearch",
            service_names: &["elasticsearch-service-x64", "elasticsearch"],

            default_port: 9200,
            cli_name: "elasticsearch",
            cli_version_args: &["--version"],
        },
    ];

    // Helper: check if port is listening
    fn port_listening(port: u16) -> bool {
        if port == 0 { return false; }
        // Use netstat-style check via connecting
        std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            std::time::Duration::from_millis(150),
        ).is_ok()
    }

    let mut found_any = false;

    for engine in engines {
        let mut status_parts: Vec<String> = Vec::new();
        let mut detected = false;

        // 1. CLI version check (fastest — works cross-platform)
        let version = Command::new(engine.cli_name)
            .args(engine.cli_version_args)
            .output()
            .ok()
            .and_then(|o| {
                let combined = if o.stdout.is_empty() {
                    String::from_utf8_lossy(&o.stderr).trim().to_string()
                } else {
                    String::from_utf8_lossy(&o.stdout).trim().to_string()
                };
                // Take just the first line
                combined.lines().next().map(|l| l.trim().to_string())
            });

        if let Some(ref ver) = version {
            if !ver.is_empty() {
                status_parts.push(format!("version: {ver}"));
                detected = true;
            }
        }

        // 2. Port check
        if engine.default_port > 0 && port_listening(engine.default_port) {
            status_parts.push(format!("listening on :{}", engine.default_port));
            detected = true;
        } else if engine.default_port > 0 && detected {
            status_parts.push(format!("not listening on :{}", engine.default_port));
        }

        // 3. Windows service check
        #[cfg(target_os = "windows")]
        {
            if !engine.service_names.is_empty() {
                let service_list = engine.service_names.join("','");
                let script = format!(
                    r#"$names = @('{}'); foreach ($n in $names) {{ $s = Get-Service -Name $n -ErrorAction SilentlyContinue; if ($s) {{ $n + ':' + $s.Status; break }} }}"#,
                    service_list
                );
                if let Ok(o) = Command::new("powershell")
                    .args(["-NoProfile", "-Command", &script])
                    .output()
                {
                    let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if !text.is_empty() {
                        let parts: Vec<&str> = text.splitn(2, ':').collect();
                        let svc_name = parts.first().map(|s| s.trim()).unwrap_or("");
                        let svc_state = parts.get(1).map(|s| s.trim()).unwrap_or("unknown");
                        status_parts.push(format!("service '{svc_name}': {svc_state}"));
                        detected = true;
                    }
                }
            }
        }

        // 4. Linux/macOS systemctl / launchctl check
        #[cfg(not(target_os = "windows"))]
        {
            for svc in engine.service_names {
                if let Ok(o) = Command::new("systemctl").args(["is-active", svc]).output() {
                    let state = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if !state.is_empty() && state != "inactive" {
                        status_parts.push(format!("systemd '{svc}': {state}"));
                        detected = true;
                        break;
                    }
                }
            }
        }

        if detected {
            found_any = true;
            let label = if engine.default_port > 0 {
                format!("{} (default port: {})", engine.name, engine.default_port)
            } else {
                format!("{} (file-based, no port)", engine.name)
            };
            out.push_str(&format!("[FOUND] {label}\n"));
            for part in &status_parts {
                out.push_str(&format!("  {part}\n"));
            }
            out.push('\n');
        }
    }

    if !found_any {
        out.push_str("No local database engines detected.\n");
        out.push_str("(Checked: PostgreSQL, MySQL, MariaDB, MongoDB, Redis, SQL Server, SQLite, CouchDB, Cassandra, Elasticsearch)\n\n");
        out.push_str("Note: databases running inside Docker containers are listed under topic='docker'.\n");
    } else {
        out.push_str("---\n");
        out.push_str("Note: databases running inside Docker containers are listed under topic='docker'.\n");
        out.push_str("This topic checks service state and port reachability only — no credentials or queries are used.\n");
    }

    Ok(out.trim_end().to_string())
}

// ── user_accounts ─────────────────────────────────────────────────────────────

fn inspect_user_accounts(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: user_accounts\n\n");

    #[cfg(target_os = "windows")]
    {
        let users_out = Command::new("powershell")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "Get-LocalUser | ForEach-Object { $logon = if ($_.LastLogon) { $_.LastLogon.ToString('yyyy-MM-dd HH:mm') } else { 'never' }; \"  $($_.Name) | Enabled: $($_.Enabled) | LastLogon: $logon | PwdRequired: $($_.PasswordRequired) | $($_.Description)\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        out.push_str("=== Local User Accounts ===\n");
        if users_out.trim().is_empty() {
            out.push_str("  (requires elevation or Get-LocalUser unavailable)\n");
        } else {
            for line in users_out.lines().take(max_entries) {
                if !line.trim().is_empty() { out.push_str(line); out.push('\n'); }
            }
        }

        let admins_out = Command::new("powershell")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "Get-LocalGroupMember -Group 'Administrators' 2>$null | ForEach-Object { \"  $($_.ObjectClass): $($_.Name)\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        out.push_str("\n=== Administrators Group Members ===\n");
        if admins_out.trim().is_empty() {
            out.push_str("  (unable to retrieve)\n");
        } else {
            out.push_str(admins_out.trim());
            out.push('\n');
        }

        let sessions_out = Command::new("powershell")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "query user 2>$null",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        out.push_str("\n=== Active Logon Sessions ===\n");
        if sessions_out.trim().is_empty() {
            out.push_str("  (none or requires elevation)\n");
        } else {
            for line in sessions_out.lines().take(max_entries) {
                if !line.trim().is_empty() { out.push_str(&format!("  {}\n", line)); }
            }
        }

        let is_admin = Command::new("powershell")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_lowercase())
            .unwrap_or_default();

        out.push_str("\n=== Current Session Elevation ===\n");
        out.push_str(&format!("  Running as Administrator: {}\n",
            if is_admin.contains("true") { "YES" } else { "no" }));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let who_out = Command::new("who").output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        out.push_str("=== Active Sessions ===\n");
        if who_out.trim().is_empty() {
            out.push_str("  (none)\n");
        } else {
            for line in who_out.lines().take(max_entries) {
                out.push_str(&format!("  {}\n", line));
            }
        }
        let id_out = Command::new("id").output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        out.push_str(&format!("\n=== Current User ===\n  {}\n", id_out.trim()));
    }

    Ok(out.trim_end().to_string())
}

// ── audit_policy ──────────────────────────────────────────────────────────────

fn inspect_audit_policy() -> Result<String, String> {
    let mut out = String::from("Host inspection: audit_policy\n\n");

    #[cfg(target_os = "windows")]
    {
        let auditpol_out = Command::new("auditpol")
            .args(["/get", "/category:*"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        if auditpol_out.trim().is_empty() || auditpol_out.to_lowercase().contains("access is denied") {
            out.push_str("Audit policy requires Administrator elevation to read.\n");
            out.push_str("Run Hematite as Administrator, or check manually: auditpol /get /category:*\n");
        } else {
            out.push_str("=== Windows Audit Policy ===\n");
            let mut any_enabled = false;
            for line in auditpol_out.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }
                if trimmed.contains("Success") || trimmed.contains("Failure") {
                    out.push_str(&format!("  [ENABLED] {}\n", trimmed));
                    any_enabled = true;
                } else {
                    out.push_str(&format!("  {}\n", trimmed));
                }
            }
            if !any_enabled {
                out.push_str("\n[WARNING] No audit categories are enabled — security events will not be logged.\n");
                out.push_str("Minimum recommended: enable Logon/Logoff and Account Logon success+failure.\n");
            }
        }

        let evtlog = Command::new("powershell")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "Get-Service EventLog -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Status",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        out.push_str(&format!("\n=== Windows Event Log Service ===\n  Status: {}\n",
            if evtlog.is_empty() { "unknown".to_string() } else { evtlog }));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let auditd_status = Command::new("systemctl")
            .args(["is-active", "auditd"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "not found".to_string());

        out.push_str(&format!("=== auditd service ===\n  Status: {}\n", auditd_status));

        if auditd_status == "active" {
            let rules = Command::new("auditctl").args(["-l"]).output().ok()
                .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
            out.push_str("\n=== Active Audit Rules ===\n");
            if rules.trim().is_empty() || rules.contains("No rules") {
                out.push_str("  No rules configured.\n");
            } else {
                for line in rules.lines() {
                    out.push_str(&format!("  {}\n", line));
                }
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── shares ────────────────────────────────────────────────────────────────────

fn inspect_shares(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: shares\n\n");

    #[cfg(target_os = "windows")]
    {
        let smb_out = Command::new("powershell")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "Get-SmbShare | ForEach-Object { \"  $($_.Name) | Path: $($_.Path) | State: $($_.ShareState) | Encrypted: $($_.EncryptData) | $($_.Description)\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        out.push_str("=== SMB Shares (exposed by this machine) ===\n");
        let smb_lines: Vec<&str> = smb_out.lines().filter(|l| !l.trim().is_empty()).take(max_entries).collect();
        if smb_lines.is_empty() {
            out.push_str("  No SMB shares or unable to retrieve.\n");
        } else {
            for line in &smb_lines {
                let name = line.trim().split('|').next().unwrap_or("").trim();
                if name.ends_with('$') {
                    out.push_str(&format!("  {}\n", line.trim()));
                } else {
                    out.push_str(&format!("  [CUSTOM] {}\n", line.trim()));
                }
            }
        }

        let smb_security = Command::new("powershell")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "Get-SmbServerConfiguration | ForEach-Object { \"  SMB1: $($_.EnableSMB1Protocol) | SMB2: $($_.EnableSMB2Protocol) | Signing Required: $($_.RequireSecuritySignature) | Encryption: $($_.EncryptData)\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        out.push_str("\n=== SMB Server Security Settings ===\n");
        if smb_security.trim().is_empty() {
            out.push_str("  (unable to retrieve)\n");
        } else {
            out.push_str(smb_security.trim());
            out.push('\n');
            if smb_security.to_lowercase().contains("smb1: true") {
                out.push_str("  [WARNING] SMB1 is ENABLED — disable it: Set-SmbServerConfiguration -EnableSMB1Protocol $false -Force\n");
            }
        }

        let drives_out = Command::new("powershell")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "Get-PSDrive -PSProvider FileSystem | Where-Object { $_.DisplayRoot } | ForEach-Object { \"  $($_.Name): -> $($_.DisplayRoot)\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        out.push_str("\n=== Mapped Network Drives ===\n");
        if drives_out.trim().is_empty() {
            out.push_str("  None.\n");
        } else {
            for line in drives_out.lines().take(max_entries) {
                if !line.trim().is_empty() { out.push_str(line); out.push('\n'); }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let smb_conf = std::fs::read_to_string("/etc/samba/smb.conf").unwrap_or_default();
        out.push_str("=== Samba Config (/etc/samba/smb.conf) ===\n");
        if smb_conf.is_empty() {
            out.push_str("  Not found or Samba not installed.\n");
        } else {
            for line in smb_conf.lines().take(max_entries) {
                out.push_str(&format!("  {}\n", line));
            }
        }
        let nfs_exports = std::fs::read_to_string("/etc/exports").unwrap_or_default();
        out.push_str("\n=== NFS Exports (/etc/exports) ===\n");
        if nfs_exports.is_empty() {
            out.push_str("  Not configured.\n");
        } else {
            for line in nfs_exports.lines().take(max_entries) {
                out.push_str(&format!("  {}\n", line));
            }
        }
    }

    Ok(out.trim_end().to_string())
}

// ── dns_servers ───────────────────────────────────────────────────────────────

fn inspect_dns_servers() -> Result<String, String> {
    let mut out = String::from("Host inspection: dns_servers\n\n");

    #[cfg(target_os = "windows")]
    {
        let dns_out = Command::new("powershell")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "Get-DnsClientServerAddress | Where-Object { $_.ServerAddresses.Count -gt 0 } | ForEach-Object { $addrs = $_.ServerAddresses -join ', '; \"  $($_.InterfaceAlias) (AF $($_.AddressFamily)): $addrs\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        out.push_str("=== Configured DNS Resolvers (per adapter) ===\n");
        if dns_out.trim().is_empty() {
            out.push_str("  (unable to retrieve)\n");
        } else {
            for line in dns_out.lines() {
                if line.trim().is_empty() { continue; }
                let mut annotation = "";
                if line.contains("8.8.8.8") || line.contains("8.8.4.4") {
                    annotation = "  <- Google Public DNS";
                } else if line.contains("1.1.1.1") || line.contains("1.0.0.1") {
                    annotation = "  <- Cloudflare DNS";
                } else if line.contains("9.9.9.9") {
                    annotation = "  <- Quad9";
                } else if line.contains("208.67.222") || line.contains("208.67.220") {
                    annotation = "  <- OpenDNS";
                }
                out.push_str(line);
                out.push_str(annotation);
                out.push('\n');
            }
        }

        let doh_out = Command::new("powershell")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "Get-DnsClientDohServerAddress 2>$null | ForEach-Object { \"  $($_.ServerAddress): $($_.DohTemplate)\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        out.push_str("\n=== DNS over HTTPS (DoH) ===\n");
        if doh_out.trim().is_empty() {
            out.push_str("  Not configured (plain DNS).\n");
        } else {
            out.push_str(doh_out.trim());
            out.push('\n');
        }

        let suffixes = Command::new("powershell")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "Get-DnsClientGlobalSetting | Select-Object -ExpandProperty SuffixSearchList | ForEach-Object { \"  $_\" }",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        if !suffixes.trim().is_empty() {
            out.push_str("\n=== DNS Search Suffix List ===\n");
            out.push_str(suffixes.trim());
            out.push('\n');
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let resolv = std::fs::read_to_string("/etc/resolv.conf").unwrap_or_default();
        out.push_str("=== /etc/resolv.conf ===\n");
        if resolv.is_empty() {
            out.push_str("  Not found.\n");
        } else {
            for line in resolv.lines() {
                if !line.trim().is_empty() && !line.starts_with('#') {
                    out.push_str(&format!("  {}\n", line));
                }
            }
        }
        let resolved_out = Command::new("resolvectl").args(["status", "--no-pager"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if !resolved_out.is_empty() {
            out.push_str("\n=== systemd-resolved ===\n");
            for line in resolved_out.lines().take(30) {
                out.push_str(&format!("  {}\n", line));
            }
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_bitlocker() -> Result<String, String> {
    let mut out = String::from("Host inspection: bitlocker\n\n");

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = "Get-BitLockerVolume | Select-Object MountPoint, VolumeStatus, ProtectionStatus, EncryptionPercentage | ForEach-Object { \"$($_.MountPoint) [$($_.VolumeStatus)] Protection:$($_.ProtectionStatus) ($($_.EncryptionPercentage)%)\" }";
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps_cmd])
            .output()
            .map_err(|e| format!("Failed to execute PowerShell: {e}"))?;

        let stdout = String::from_utf8(output.stdout).unwrap_or_default();
        let stderr = String::from_utf8(output.stderr).unwrap_or_default();

        if !stdout.trim().is_empty() {
            out.push_str("=== BitLocker Volumes ===\n");
            for line in stdout.lines() {
                out.push_str(&format!("  {}\n", line));
            }
        } else if !stderr.trim().is_empty() {
            if stderr.contains("Access is denied") {
                out.push_str("Error: Access denied. BitLocker diagnostics require Administrator elevation.\n");
            } else {
                out.push_str(&format!("Error retrieving BitLocker info: {}\n", stderr.trim()));
            }
        } else {
            out.push_str("No BitLocker volumes detected or access denied.\n");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("BitLocker is a Windows-specific technology. Checking for LUKS/dm-crypt...\n\n");
        let lsblk = Command::new("lsblk").args(["-f", "-o", "NAME,FSTYPE,MOUNTPOINT"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if lsblk.contains("crypto_LUKS") {
            out.push_str("=== LUKS Encrypted Volumes ===\n");
            for line in lsblk.lines().filter(|l| l.contains("crypto_LUKS")) {
                out.push_str(&format!("  {}\n", line));
            }
        } else {
            out.push_str("No LUKS encrypted volumes detected via lsblk.\n");
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_rdp() -> Result<String, String> {
    let mut out = String::from("Host inspection: rdp\n\n");

    #[cfg(target_os = "windows")]
    {
        let reg_path = "HKLM:\\System\\CurrentControlSet\\Control\\Terminal Server";
        let f_deny = Command::new("powershell").args(["-NoProfile", "-Command", &format!("(Get-ItemProperty '{}').fDenyTSConnections", reg_path)])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default().trim().to_string();

        let status = if f_deny == "0" { "ENABLED" } else { "DISABLED" };
        out.push_str(&format!("=== RDP Status: {} ===\n", status));

        let port = Command::new("powershell").args(["-NoProfile", "-Command", "Get-ItemProperty 'HKLM:\\System\\CurrentControlSet\\Control\\Terminal Server\\WinStations\\RDP-Tcp' -Name PortNumber | Select-Object -ExpandProperty PortNumber"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default().trim().to_string();
        out.push_str(&format!("  Port: {}\n", if port.is_empty() { "3389 (default)" } else { &port }));

        let nla = Command::new("powershell").args(["-NoProfile", "-Command", &format!("(Get-ItemProperty '{}').UserAuthentication", reg_path)])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default().trim().to_string();
        out.push_str(&format!("  NLA Required: {}\n", if nla == "1" { "Yes" } else { "No" }));

        out.push_str("\n=== Active Sessions ===\n");
        let qwinsta = Command::new("qwinsta").output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if qwinsta.trim().is_empty() {
            out.push_str("  No active sessions listed.\n");
        } else {
            for line in qwinsta.lines() {
                out.push_str(&format!("  {}\n", line));
            }
        }

        out.push_str("\n=== Firewall Rule Check ===\n");
        let fw = Command::new("powershell").args(["-NoProfile", "-Command", "Get-NetFirewallRule -DisplayName '*Remote Desktop*' -Enabled True | Select-Object DisplayName, Action, Direction | ForEach-Object { \"  $($_.DisplayName): $($_.Action) ($($_.Direction))\" }"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if fw.trim().is_empty() {
            out.push_str("  No enabled RDP firewall rules found.\n");
        } else {
            out.push_str(fw.trim_end());
            out.push('\n');
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("Checking for common RDP/VNC listeners (3389, 5900-5905)...\n");
        let ss = Command::new("ss").args(["-tlnp"]).output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();
        let matches: Vec<&str> = ss.lines().filter(|l| l.contains(":3389") || l.contains(":590")).collect();
        if matches.is_empty() {
            out.push_str("  No RDP/VNC listeners detected via 'ss'.\n");
        } else {
            for m in matches {
                out.push_str(&format!("  {}\n", m));
            }
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_shadow_copies() -> Result<String, String> {
    let mut out = String::from("Host inspection: shadow_copies\n\n");

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("vssadmin").args(["list", "shadows"]).output()
            .map_err(|e| format!("Failed to run vssadmin: {e}"))?;
        let stdout = String::from_utf8(output.stdout).unwrap_or_default();

        if stdout.contains("No items found") || stdout.trim().is_empty() {
            out.push_str("No Volume Shadow Copies found.\n");
        } else {
            out.push_str("=== Volume Shadow Copies ===\n");
            for line in stdout.lines().take(50) {
                if line.contains("Creation Time:") || line.contains("Contents:") || line.contains("Volume Name:") {
                    out.push_str(&format!("  {}\n", line.trim()));
                }
            }
        }

        out.push_str("\n=== Shadow Copy Storage ===\n");
        let storage_out = Command::new("vssadmin").args(["list", "shadowstorage"]).output().ok();
        if let Some(o) = storage_out {
            let stdout = String::from_utf8(o.stdout).unwrap_or_default();
            for line in stdout.lines() {
                if line.contains("Used Shadow Copy Storage space:") || line.contains("Max Shadow Copy Storage space:") {
                    out.push_str(&format!("  {}\n", line.trim()));
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("Checking for LVM snapshots or Btrfs subvolumes...\n\n");
        let lvs = Command::new("lvs").output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if !lvs.is_empty() {
            out.push_str("=== LVM Volumes (checking for snapshots) ===\n");
            out.push_str(&lvs);
        } else {
            out.push_str("No LVM volumes detected.\n");
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_pagefile() -> Result<String, String> {
    let mut out = String::from("Host inspection: pagefile\n\n");

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = "Get-CimInstance Win32_PageFileUsage | Select-Object Name, AllocatedBaseSize, CurrentUsage, PeakUsage | ForEach-Object { \"  $($_.Name): $($_.AllocatedBaseSize)MB total, $($_.CurrentUsage)MB used (Peak: $($_.PeakUsage)MB)\" }";
        let output = Command::new("powershell").args(["-NoProfile", "-Command", ps_cmd])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();

        if output.trim().is_empty() {
            out.push_str("No page files detected (system may be running without a page file or managed differently).\n");
            let managed = Command::new("powershell").args(["-NoProfile", "-Command", "(Get-CimInstance Win32_ComputerSystem).AutomaticManagedPagefile"])
                .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default().trim().to_string();
            out.push_str(&format!("Automatic Managed Pagefile: {}\n", managed));
        } else {
            out.push_str("=== Page File Usage ===\n");
            out.push_str(&output);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("=== Swap Usage (Linux/macOS) ===\n");
        let swap = Command::new("swapon").args(["--show"]).output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if swap.is_empty() {
            let free = Command::new("free").args(["-h"]).output().ok()
                .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
            out.push_str(&free);
        } else {
            out.push_str(&swap);
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_windows_features(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: windows_features\n\n");

    #[cfg(target_os = "windows")]
    {
        out.push_str("=== Quick Check: Notable Features ===\n");
        let quick_ps = "Get-WindowsOptionalFeature -Online | Where-Object { $_.FeatureName -match 'IIS|Hyper-V|VirtualMachinePlatform|Subsystem-Linux' -and $_.State -eq 'Enabled' } | Select-Object -ExpandProperty FeatureName";
        let output = Command::new("powershell").args(["-NoProfile", "-Command", quick_ps]).output().ok();
        
        if let Some(o) = output {
            let stdout = String::from_utf8(o.stdout).unwrap_or_default();
            let stderr = String::from_utf8(o.stderr).unwrap_or_default();
            
            if !stdout.trim().is_empty() {
                for f in stdout.lines() {
                    out.push_str(&format!("  [ENABLED] {}\n", f));
                }
            } else if stderr.contains("Access is denied") || stderr.contains("requires elevation") {
                out.push_str("  Error: Access denied. Listing Windows Features requires Administrator elevation.\n");
            } else if quick_ps.contains("-Online") && stdout.trim().is_empty() {
                out.push_str("  No major features (IIS, Hyper-V, WSL) appear enabled in the quick check.\n");
            }
        }

        out.push_str(&format!("\n=== All Enabled Features (capped at {}) ===\n", max_entries));
        let all_ps = format!("Get-WindowsOptionalFeature -Online | Where-Object {{$_.State -eq 'Enabled'}} | Select-Object -First {} -ExpandProperty FeatureName", max_entries);
        let all_out = Command::new("powershell").args(["-NoProfile", "-Command", &all_ps]).output().ok();
        if let Some(o) = all_out {
            let stdout = String::from_utf8(o.stdout).unwrap_or_default();
            if !stdout.trim().is_empty() {
                out.push_str(&stdout);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = max_entries;
        out.push_str("Windows Optional Features are Windows-specific. On Linux, check your package manager.\n");
    }

    Ok(out.trim_end().to_string())
}

fn inspect_printers(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: printers\n\n");

    #[cfg(target_os = "windows")]
    {
        let list = Command::new("powershell").args(["-NoProfile", "-Command", &format!("Get-Printer | Select-Object Name, DriverName, PortName, JobCount | Select-Object -First {} | ForEach-Object {{ \"  $($_.Name) [$($_.DriverName)] (Port: $($_.PortName), Jobs: $($_.JobCount))\" }}", max_entries)])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if list.trim().is_empty() {
            out.push_str("No printers detected.\n");
        } else {
            out.push_str("=== Installed Printers ===\n");
            out.push_str(&list);
        }

        let jobs = Command::new("powershell").args(["-NoProfile", "-Command", "Get-PrintJob | Select-Object PrinterName, ID, DocumentName, Status | ForEach-Object { \"  [$($_.PrinterName)] Job $($_.ID): $($_.DocumentName) - $($_.Status)\" }"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if !jobs.trim().is_empty() {
            out.push_str("\n=== Active Print Jobs ===\n");
            out.push_str(&jobs);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = max_entries;
        out.push_str("Checking LPSTAT for printers...\n");
        let lpstat = Command::new("lpstat").args(["-p", "-d"]).output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if lpstat.is_empty() {
            out.push_str("  No CUPS/LP printers found.\n");
        } else {
            out.push_str(&lpstat);
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_winrm() -> Result<String, String> {
    let mut out = String::from("Host inspection: winrm\n\n");

    #[cfg(target_os = "windows")]
    {
        let svc = Command::new("powershell").args(["-NoProfile", "-Command", "(Get-Service WinRM).Status"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default().trim().to_string();
        out.push_str(&format!("WinRM Service Status: {}\n\n", if svc.is_empty() { "NOT_FOUND" } else { &svc }));

        out.push_str("=== WinRM Listeners ===\n");
        let output = Command::new("powershell").args(["-NoProfile", "-Command", "winrm enumerate winrm/config/listener 2>$null"]).output().ok();
        if let Some(o) = output {
            let stdout = String::from_utf8(o.stdout).unwrap_or_default();
            let stderr = String::from_utf8(o.stderr).unwrap_or_default();
            
            if !stdout.trim().is_empty() {
                for line in stdout.lines() {
                    if line.contains("Address =") || line.contains("Transport =") || line.contains("Port =") {
                        out.push_str(&format!("  {}\n", line.trim()));
                    }
                }
            } else if stderr.contains("Access is denied") {
                out.push_str("  Error: Access denied to WinRM configuration.\n");
            } else {
                out.push_str("  No listeners configured.\n");
            }
        }

        out.push_str("\n=== PowerShell Remoting Test (Local) ===\n");
        let test_out = Command::new("powershell").args(["-NoProfile", "-Command", "Test-WSMan -ErrorAction SilentlyContinue | Select-Object ProductVersion, Stack | ForEach-Object { \"  SUCCESS: OS Version $($_.ProductVersion) (Stack $($_.Stack))\" }"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if test_out.trim().is_empty() {
            out.push_str("  WinRM not responding to local WS-Man requests.\n");
        } else {
            out.push_str(&test_out);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("WinRM is primarily a Windows technology. Checking for listening port 5985/5986...\n");
        let ss = Command::new("ss").args(["-tln"]).output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if ss.contains(":5985") || ss.contains(":5986") {
            out.push_str("  WinRM ports (5985/5986) are listening.\n");
        } else {
            out.push_str("  WinRM ports not detected.\n");
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_network_stats(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: network_stats\n\n");

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = format!("Get-NetAdapterStatistics | Select-Object Name, ReceivedBytes, SentBytes, ReceivedPacketErrors, OutboundPacketErrors | Select-Object -First {} | ForEach-Object {{ \"  $($_.Name): RX:$([math]::round($($_.ReceivedBytes)/1MB, 2))MB, TX:$([math]::round($($_.SentBytes)/1MB, 2))MB, Errors(RX/TX): $($_.ReceivedPacketErrors)/$($_.OutboundPacketErrors)\" }}", max_entries);
        let output = Command::new("powershell").args(["-NoProfile", "-Command", &ps_cmd])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if output.trim().is_empty() {
            out.push_str("No network adapter statistics available.\n");
        } else {
            out.push_str("=== Adapter Throughput & Errors ===\n");
            out.push_str(&output);
        }

        let discards = Command::new("powershell").args(["-NoProfile", "-Command", "Get-NetAdapterStatistics | Select-Object Name, ReceivedPacketDiscards, OutboundPacketDiscards | ForEach-Object { if($_.ReceivedPacketDiscards -gt 0 -or $_.OutboundPacketDiscards -gt 0) { \"  $($_.Name): Discards(RX/TX): $($_.ReceivedPacketDiscards)/$($_.OutboundPacketDiscards)\" } }"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if !discards.trim().is_empty() {
            out.push_str("\n=== Packet Discards (Non-Zero Only) ===\n");
            out.push_str(&discards);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = max_entries;
        out.push_str("=== Network Stats (ip -s link) ===\n");
        let ip_s = Command::new("ip").args(["-s", "link"]).output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if ip_s.is_empty() {
            let netstat = Command::new("netstat").args(["-i"]).output().ok()
                .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
            out.push_str(&netstat);
        } else {
            out.push_str(&ip_s);
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_udp_ports(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: udp_ports\n\n");

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = format!("Get-NetUDPEndpoint | Select-Object LocalAddress, LocalPort, OwningProcess | Select-Object -First {} | ForEach-Object {{ $proc = (Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue).Name; \"  $($_.LocalAddress):$($_.LocalPort) (PID: $($_.OwningProcess) - $($proc))\" }}", max_entries);
        let output = Command::new("powershell").args(["-NoProfile", "-Command", &ps_cmd]).output().ok();

        if let Some(o) = output {
            let stdout = String::from_utf8(o.stdout).unwrap_or_default();
            let stderr = String::from_utf8(o.stderr).unwrap_or_default();
            
            if !stdout.trim().is_empty() {
                out.push_str("=== UDP Listeners (Local:Port) ===\n");
                for line in stdout.lines() {
                    let mut note = "";
                    if line.contains(":53 ") { note = " [DNS]"; }
                    else if line.contains(":67 ") || line.contains(":68 ") { note = " [DHCP]"; }
                    else if line.contains(":123 ") { note = " [NTP]"; }
                    else if line.contains(":161 ") { note = " [SNMP]"; }
                    else if line.contains(":1900 ") { note = " [SSDP/UPnP]"; }
                    else if line.contains(":5353 ") { note = " [mDNS]"; }

                    out.push_str(&format!("{}{}\n", line, note));
                }
            } else if stderr.contains("Access is denied") {
                out.push_str("Error: Access denied. Full UDP listener details require Administrator elevation.\n");
            } else {
                out.push_str("No UDP listeners detected.\n");
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let ss_out = Command::new("ss").args(["-ulnp"]).output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        out.push_str("=== UDP Listeners (ss -ulnp) ===\n");
        if ss_out.is_empty() {
            let netstat_out = Command::new("netstat").args(["-ulnp"]).output().ok()
                .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
            if netstat_out.is_empty() {
                out.push_str("  Neither 'ss' nor 'netstat' available.\n");
            } else {
                for line in netstat_out.lines().take(max_entries) {
                    out.push_str(&format!("  {}\n", line));
                }
            }
        } else {
            for line in ss_out.lines().take(max_entries) {
                out.push_str(&format!("  {}\n", line));
            }
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_gpo() -> Result<String, String> {
    let mut out = String::from("Host inspection: gpo\n\n");

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("gpresult")
            .args(["/r", "/scope", "computer"])
            .output()
            .ok();

        if let Some(o) = output {
            let stdout = String::from_utf8(o.stdout).unwrap_or_default();
            let stderr = String::from_utf8(o.stderr).unwrap_or_default();

            if stdout.contains("Applied Group Policy Objects") {
                out.push_str("=== Applied Group Policy Objects (Computer Scope) ===\n");
                let mut capture = false;
                for line in stdout.lines() {
                    if line.contains("Applied Group Policy Objects") {
                        capture = true;
                    } else if capture && line.contains("The following GPOs were not applied") {
                        break;
                    }
                    if capture && !line.trim().is_empty() {
                        out.push_str(&format!("  {}\n", line.trim()));
                    }
                }
            } else if stderr.contains("Access is denied") || stdout.contains("Access is denied") {
                out.push_str("Error: Access denied. Group Policy inspection requires Administrator elevation.\n");
            } else {
                out.push_str("No applied Group Policy Objects detected or insufficient permissions to query computer scope.\n");
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("Group Policy (GPO) is a Windows-only topic.\n");
    }

    Ok(out.trim_end().to_string())
}

fn inspect_certificates(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: certificates\n\n");

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = format!(
            "Get-ChildItem -Path Cert:\\LocalMachine\\My | Select-Object Subject, NotAfter, Thumbprint | Select-Object -First {} | ForEach-Object {{ \
                $days = ($_.NotAfter - (Get-Date)).Days; \
                $status = if ($days -lt 0) {{ \"[EXPIRED]\" }} else if ($days -lt 30) {{ \"[EXPIRING SOON ($days days)]\" }} else {{ \"\" }}; \
                \"  $($_.Subject) - Expires: $($_.NotAfter.ToString('yyyy-MM-dd')) $status (Thumb: $($_.Thumbprint.Substring(0,8))...)\" \
            }}", 
            max_entries
        );
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_cmd])
            .output()
            .ok();

        if let Some(o) = output {
            let stdout = String::from_utf8(o.stdout).unwrap_or_default();
            if !stdout.trim().is_empty() {
                out.push_str("=== Local Machine Certificates (Personal Store) ===\n");
                out.push_str(&stdout);
            } else {
                out.push_str("No certificates found in the Local Machine Personal store.\n");
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = max_entries;
        out.push_str("Host inspection: certificates (Linux/macOS)\n\n");
        // Check standard cert locations
        for path in ["/etc/ssl/certs", "/etc/pki/tls/certs"] {
            if Path::new(path).exists() {
                out.push_str(&format!("  Cert directory found: {}\n", path));
            }
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_integrity() -> Result<String, String> {
    let mut out = String::from("Host inspection: integrity\n\n");

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = "Get-ItemProperty 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Component Based Servicing' | Select-Object Corrupt, AutoRepairNeeded, LastRepairAttempted | ConvertTo-Json";
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_cmd])
            .output()
            .ok();

        if let Some(o) = output {
            let stdout = String::from_utf8(o.stdout).unwrap_or_default();
            if let Ok(val) = serde_json::from_str::<Value>(&stdout) {
                out.push_str("=== Windows Component Store Health (CBS) ===\n");
                let corrupt = val.get("Corrupt").and_then(|v| v.as_u64()).unwrap_or(0);
                let repair = val.get("AutoRepairNeeded").and_then(|v| v.as_u64()).unwrap_or(0);
                
                out.push_str(&format!("  Corruption Detected: {}\n", if corrupt != 0 { "YES (SFC/DISM recommended)" } else { "No" }));
                out.push_str(&format!("  Auto-Repair Needed: {}\n", if repair != 0 { "YES" } else { "No" }));
                
                if let Some(last) = val.get("LastRepairAttempted").and_then(|v| v.as_u64()) {
                    out.push_str(&format!("  Last Repair Attempt: (Raw code: {})\n", last));
                }
            } else {
                out.push_str("Could not retrieve CBS health from registry. System may be healthy or state is unknown.\n");
            }
        }

        if Path::new("C:\\Windows\\Logs\\CBS\\CBS.log").exists() {
            out.push_str("\nNote: Detailed integrity logs available at C:\\Windows\\Logs\\CBS\\CBS.log\n");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("System integrity check (Linux)\n\n");
        let pkg_check = Command::new("rpm").args(["-Va"]).output().or_else(|_| Command::new("dpkg").args(["--verify"]).output()).ok();
        if let Some(o) = pkg_check {
             out.push_str("  Package verification system active.\n");
             if o.status.success() {
                 out.push_str("  No major package integrity issues detected.\n");
             }
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_domain() -> Result<String, String> {
    let mut out = String::from("Host inspection: domain\n\n");

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = "Get-CimInstance Win32_ComputerSystem | Select-Object Name, Domain, PartOfDomain, Workgroup | ConvertTo-Json";
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_cmd])
            .output()
            .ok();

        if let Some(o) = output {
            let stdout = String::from_utf8(o.stdout).unwrap_or_default();
            if let Ok(val) = serde_json::from_str::<Value>(&stdout) {
                let part_of_domain = val.get("PartOfDomain").and_then(|v| v.as_bool()).unwrap_or(false);
                let domain = val.get("Domain").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let workgroup = val.get("Workgroup").and_then(|v| v.as_str()).unwrap_or("Unknown");

                out.push_str("=== Windows Domain / Workgroup Identity ===\n");
                out.push_str(&format!("  Join Status: {}\n", if part_of_domain { "DOMAIN JOINED" } else { "WORKGROUP" }));
                if part_of_domain {
                    out.push_str(&format!("  Active Directory Domain: {}\n", domain));
                } else {
                    out.push_str(&format!("  Workgroup Name: {}\n", workgroup));
                }

                if let Some(name) = val.get("Name").and_then(|v| v.as_str()) {
                    out.push_str(&format!("  NetBIOS Name: {}\n", name));
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let domainname = Command::new("domainname").output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        out.push_str("=== Linux Domain Identity ===\n");
        if !domainname.trim().is_empty() && domainname.trim() != "(none)" {
             out.push_str(&format!("  NIS/YP Domain: {}\n", domainname.trim()));
        } else {
             out.push_str("  No NIS domain configured.\n");
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_device_health() -> Result<String, String> {
    let mut out = String::from("Host inspection: device_health\n\n");

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = "Get-CimInstance Win32_PnPEntity | Where-Object { $_.ConfigManagerErrorCode -ne 0 } | Select-Object Name, Status, ConfigManagerErrorCode, Description | ForEach-Object { \"  [ERR:$($_.ConfigManagerErrorCode)] $($_.Name) ($($_.Status)) - $($_.Description)\" }";
        let output = Command::new("powershell").args(["-NoProfile", "-Command", ps_cmd])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        
        if output.trim().is_empty() {
            out.push_str("All PnP devices report as healthy (no ConfigManager errors detected).\n");
        } else {
            out.push_str("=== Malfunctioning Devices (Yellow Bangs) ===\n");
            out.push_str(&output);
            out.push_str("\nTip: Error codes 10 and 28 usually indicate missing or incompatible drivers.\n");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("Checking dmesg for hardware errors...\n");
        let dmesg = Command::new("dmesg").args(["--level=err,crit,alert"]).output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        if dmesg.is_empty() {
            out.push_str("  No critical hardware errors found in dmesg.\n");
        } else {
            out.push_str(&dmesg.lines().take(20).collect::<Vec<_>>().join("\n"));
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_drivers(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: drivers\n\n");

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = format!("Get-CimInstance Win32_SystemDriver | Select-Object Name, Description, State, Status | Select-Object -First {} | ForEach-Object {{ \"  $($_.Name): $($_.State) ($($_.Status)) - $($_.Description)\" }}", max_entries);
        let output = Command::new("powershell").args(["-NoProfile", "-Command", &ps_cmd])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        
        if output.trim().is_empty() {
            out.push_str("No drivers retrieved via WMI.\n");
        } else {
            out.push_str("=== Active System Drivers (CIM Snapshot) ===\n");
            out.push_str(&output);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("=== Loaded Kernel Modules (lsmod) ===\n");
        let lsmod = Command::new("lsmod").output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        out.push_str(&lsmod.lines().take(max_entries).collect::<Vec<_>>().join("\n"));
    }

    Ok(out.trim_end().to_string())
}

fn inspect_peripherals(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: peripherals\n\n");

    #[cfg(target_os = "windows")]
    {
        let _ = max_entries;
        out.push_str("=== USB Controllers & Hubs ===\n");
        let usb = Command::new("powershell").args(["-NoProfile", "-Command", "Get-CimInstance Win32_USBController | ForEach-Object { \"  $($_.Name) ($($_.Status))\" }"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        out.push_str(if usb.is_empty() { "  None detected.\n" } else { &usb });

        out.push_str("\n=== Input Devices (Keyboard/Pointer) ===\n");
        let kb = Command::new("powershell").args(["-NoProfile", "-Command", "Get-CimInstance Win32_Keyboard | ForEach-Object { \"  [KB] $($_.Name) ($($_.Status))\" }"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        let mouse = Command::new("powershell").args(["-NoProfile", "-Command", "Get-CimInstance Win32_PointingDevice | ForEach-Object { \"  [PTR] $($_.Name) ($($_.Status))\" }"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        out.push_str(&kb);
        out.push_str(&mouse);

        out.push_str("\n=== Connected Monitors (WMI) ===\n");
        let mon = Command::new("powershell").args(["-NoProfile", "-Command", "Get-CimInstance -Namespace root\\wmi -ClassName WmiMonitorBasicDisplayParams | ForEach-Object { \"  Display ($($_.Active ? 'Active' : 'Inactive'))\" }"])
            .output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        out.push_str(if mon.is_empty() { "  No active monitors identified via WMI.\n" } else { &mon });
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("=== Connected USB Devices (lsusb) ===\n");
        let lsusb = Command::new("lsusb").output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        out.push_str(&lsusb.lines().take(max_entries).collect::<Vec<_>>().join("\n"));
    }

    Ok(out.trim_end().to_string())
}

fn inspect_sessions(max_entries: usize) -> Result<String, String> {
    let mut out = String::from("Host inspection: sessions\n\n");

    #[cfg(target_os = "windows")]
    {
        let script = r#"Get-CimInstance Win32_LogonSession | ForEach-Object {
    "$($_.LogonId)|$($_.StartTime)|$($_.LogonType)|$($_.AuthenticationPackage)"
}"#;
        if let Ok(o) = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = text.lines().collect();
            if lines.is_empty() {
                out.push_str("No active logon sessions enumerated via WMI.\n");
            } else {
                out.push_str("=== Active Logon Sessions (WMI Snapshot) ===\n");
                for line in lines.iter().take(max_entries).filter(|l| !l.trim().is_empty()) {
                    let parts: Vec<&str> = line.trim().split('|').collect();
                    if parts.len() == 4 {
                        let logon_type = match parts[2] {
                            "2" => "Interactive",
                            "3" => "Network",
                            "4" => "Batch",
                            "5" => "Service",
                            "7" => "Unlock",
                            "8" => "NetworkCleartext",
                            "9" => "NewCredentials",
                            "10" => "RemoteInteractive",
                            "11" => "CachedInteractive",
                            _ => "Other",
                        };
                        out.push_str(&format!(
                            "- ID: {} | Type: {} | Started: {} | Auth: {}\n",
                            parts[0], logon_type, parts[1], parts[3]
                        ));
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("=== Logged-in Users (who) ===\n");
        let who = Command::new("who").output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
        out.push_str(&who.lines().take(max_entries).collect::<Vec<_>>().join("\n"));
    }

    Ok(out.trim_end().to_string())
}

async fn inspect_disk_benchmark(path: PathBuf) -> Result<String, String> {
    let mut out = String::from("Host inspection: disk_benchmark\n\n");
    let mut final_path = path;

    if !final_path.exists() {
        if let Ok(current_exe) = std::env::current_exe() {
            out.push_str(&format!(
                "Note: Requested target '{}' not found. Falling back to current binary for silicon-aware intensity report.\n",
                final_path.display()
            ));
            final_path = current_exe;
        } else {
            return Err(format!("Target not found: {}", final_path.display()));
        }
    }

    let target = if final_path.is_dir() {
        // Find a representative file to read
        let mut target_file = final_path.join("Cargo.toml");
        if !target_file.exists() {
            target_file = final_path.join("README.md");
        }
        if !target_file.exists() {
            return Err("Target path is a directory but no representative file (Cargo.toml/README.md) found for benchmarking.".to_string());
        }
        target_file
    } else {
        final_path
    };

    out.push_str(&format!("Target: {}\n", target.display()));
    out.push_str("Running diagnostic stress test (5s read-thrash + kernel counter trace)...\n\n");

    #[cfg(target_os = "windows")]
    {
        let script = format!(
            r#"
$target = "{}"
if (-not (Test-Path $target)) {{ "ERROR:Target not found"; exit }}

$diskQueue = @()
$readStats = @()
$startTime = Get-Date
$duration = 5

# Background reader job
$job = Start-Job -ScriptBlock {{
    param($t, $d)
    $stop = (Get-Date).AddSeconds($d)
    while ((Get-Date) -lt $stop) {{
        try {{ [System.IO.File]::ReadAllBytes($t) | Out-Null }} catch {{ }}
    }}
}} -ArgumentList $target, $duration

# Metrics collector loop
$stopTime = (Get-Date).AddSeconds($duration)
while ((Get-Date) -lt $stopTime) {{
    $q = Get-Counter '\PhysicalDisk(_Total)\Avg. Disk Queue Length' -ErrorAction SilentlyContinue
    if ($q) {{ $diskQueue += $q.CounterSamples[0].CookedValue }}
    
    $r = Get-Counter '\PhysicalDisk(_Total)\Disk Reads/sec' -ErrorAction SilentlyContinue
    if ($r) {{ $readStats += $r.CounterSamples[0].CookedValue }}
    
    Start-Sleep -Milliseconds 250
}}

Stop-Job $job
Receive-Job $job | Out-Null
Remove-Job $job

$avgQ = if ($diskQueue) {{ ($diskQueue | Measure-Object -Average).Average }} else {{ 0 }}
$maxQ = if ($diskQueue) {{ ($diskQueue | Measure-Object -Maximum).Maximum }} else {{ 0 }}
$avgR = if ($readStats) {{ ($readStats | Measure-Object -Average).Average }} else {{ 0 }}

"AVG_Q:$([math]::Round($avgQ, 4))|MAX_Q:$([math]::Round($maxQ, 4))|AVG_R:$([math]::Round($avgR, 2))"
"#,
            target.display()
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| format!("Benchmark failed: {e}"))?;

        let raw = String::from_utf8_lossy(&output.stdout);
        let text = raw.trim();

        if text.starts_with("ERROR") {
            return Err(text.to_string());
        }

        let mut lines = text.lines();
        if let Some(metrics_line) = lines.next() {
            let parts: Vec<&str> = metrics_line.split('|').collect();
            let mut avg_q = "unknown".to_string();
            let mut max_q = "unknown".to_string();
            let mut avg_r = "unknown".to_string();

            for p in parts {
                if let Some((k, v)) = p.split_once(':') {
                    match k {
                        "AVG_Q" => avg_q = v.to_string(),
                        "MAX_Q" => max_q = v.to_string(),
                        "AVG_R" => avg_r = v.to_string(),
                        _ => {}
                    }
                }
            }

            out.push_str("=== WORKSTATION INTENSITY REPORT ===\n");
            out.push_str(&format!("- Active Disk Queue (Avg): {}\n", avg_q));
            out.push_str(&format!("- Active Disk Queue (Max): {}\n", max_q));
            out.push_str(&format!("- Disk Throughput (Avg):  {} reads/sec\n", avg_r));
            out.push_str("\nVerdict: ");
            let q_num = avg_q.parse::<f64>().unwrap_or(0.0);
            if q_num > 1.0 {
                out.push_str("HIGH INTENSITY — the disk stack is saturated. Hardware bottleneck confirmed.");
            } else if q_num > 0.1 {
                out.push_str("MODERATE LOAD — significant I/O pressure detected.");
            } else {
                out.push_str("LIGHT LOAD — the hardware is handling this volume comfortably.");
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        out.push_str("Note: Native silicon benchmarking is currently optimized for Windows performance counters.\n");
        out.push_str("Generic disk load simulated.\n");
    }

    Ok(out)
}


