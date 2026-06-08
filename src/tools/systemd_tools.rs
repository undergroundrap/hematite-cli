use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else {
        "info".to_string()
    };
    match action.as_str() {
        "info" => info_action(args),
        "service" => service_action(args),
        "timer" => timer_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: info, service, timer, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("unit"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            "Missing 'text' — pass the systemd unit file content as a string".to_string()
        })
}

#[derive(Debug, Default)]
struct UnitFile {
    unit: Vec<(String, String)>,
    service: Vec<(String, String)>,
    timer: Vec<(String, String)>,
    socket: Vec<(String, String)>,
    install: Vec<(String, String)>,
    mount: Vec<(String, String)>,
    automount: Vec<(String, String)>,
    path: Vec<(String, String)>,
    sections: Vec<String>,
}

fn parse_unit(text: &str) -> UnitFile {
    let mut uf = UnitFile::default();
    let mut current_section = String::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            if !uf.sections.contains(&current_section) {
                uf.sections.push(current_section.clone());
            }
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_string();
            let val = line[eq + 1..].trim().to_string();
            match current_section.to_lowercase().as_str() {
                "unit" => uf.unit.push((key, val)),
                "service" => uf.service.push((key, val)),
                "timer" => uf.timer.push((key, val)),
                "socket" => uf.socket.push((key, val)),
                "install" => uf.install.push((key, val)),
                "mount" => uf.mount.push((key, val)),
                "automount" => uf.automount.push((key, val)),
                "path" => uf.path.push((key, val)),
                _ => {}
            }
        }
    }
    uf
}

fn get_kv<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .rev()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

fn get_all_kv<'a>(pairs: &'a [(String, String)], key: &str) -> Vec<&'a str> {
    pairs
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
        .collect()
}

fn detect_unit_type(uf: &UnitFile) -> &'static str {
    if !uf.service.is_empty() {
        "service"
    } else if !uf.timer.is_empty() {
        "timer"
    } else if !uf.socket.is_empty() {
        "socket"
    } else if !uf.mount.is_empty() {
        "mount"
    } else if !uf.path.is_empty() {
        "path"
    } else {
        "unit"
    }
}

fn explain_service_type(t: &str) -> &'static str {
    match t.to_lowercase().as_str() {
        "simple" => "starts immediately; ExecStart is the main process",
        "exec" => "similar to simple but waits for exec() to succeed before marking started",
        "forking" => "ExecStart forks a child; parent exits; systemd tracks child",
        "oneshot" => "runs once and exits; service stays 'active' after exit",
        "dbus" => "service signals readiness via D-Bus",
        "notify" => "service sends sd_notify(3) when ready",
        "notify-reload" => "like notify; supports sd_notify-based reloads",
        "idle" => "delayed start until active jobs finish",
        _ => "unknown type",
    }
}

fn info_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let uf = parse_unit(&text);

    let unit_type = detect_unit_type(&uf);
    let description = get_kv(&uf.unit, "Description").unwrap_or("(none)");
    let documentation = get_kv(&uf.unit, "Documentation");
    let after = get_all_kv(&uf.unit, "After");
    let requires = get_all_kv(&uf.unit, "Requires");
    let wants = get_all_kv(&uf.unit, "Wants");
    let wanted_by = get_all_kv(&uf.install, "WantedBy");
    let required_by = get_all_kv(&uf.install, "RequiredBy");

    let mut out = format!("Systemd Unit File\n{}\n\n", "=".repeat(44));
    out += &format!("Type:        {}\n", unit_type);
    out += &format!("Description: {}\n", description);
    if let Some(doc) = documentation {
        out += &format!("Docs:        {}\n", doc);
    }
    out += &format!("Sections:    {}\n\n", uf.sections.join(", "));

    if !after.is_empty() {
        out += &format!("[Unit] After:    {}\n", after.join(", "));
    }
    if !requires.is_empty() {
        out += &format!("[Unit] Requires: {}\n", requires.join(", "));
    }
    if !wants.is_empty() {
        out += &format!("[Unit] Wants:    {}\n", wants.join(", "));
    }

    if !uf.service.is_empty() {
        out += "\n[Service]\n";
        let stype = get_kv(&uf.service, "Type").unwrap_or("simple");
        out += &format!(
            "  Type:          {} — {}\n",
            stype,
            explain_service_type(stype)
        );
        if let Some(user) = get_kv(&uf.service, "User") {
            out += &format!("  User:          {}\n", user);
        }
        if let Some(group) = get_kv(&uf.service, "Group") {
            out += &format!("  Group:         {}\n", group);
        }
        if let Some(exec) = get_kv(&uf.service, "ExecStart") {
            out += &format!("  ExecStart:     {}\n", exec);
        }
        if let Some(exec) = get_kv(&uf.service, "ExecReload") {
            out += &format!("  ExecReload:    {}\n", exec);
        }
        if let Some(exec) = get_kv(&uf.service, "ExecStop") {
            out += &format!("  ExecStop:      {}\n", exec);
        }
        if let Some(r) = get_kv(&uf.service, "Restart") {
            out += &format!("  Restart:       {}\n", r);
        }
        if let Some(wd) = get_kv(&uf.service, "WorkingDirectory") {
            out += &format!("  WorkingDir:    {}\n", wd);
        }
        if let Some(env_file) = get_kv(&uf.service, "EnvironmentFile") {
            out += &format!("  EnvironmentFile: {}\n", env_file);
        }
    }

    if !uf.timer.is_empty() {
        out += "\n[Timer]\n";
        for key in &[
            "OnCalendar",
            "OnBootSec",
            "OnActiveSec",
            "OnUnitActiveSec",
            "Unit",
            "Persistent",
        ] {
            if let Some(v) = get_kv(&uf.timer, key) {
                out += &format!("  {:<20} {}\n", key, v);
            }
        }
    }

    if !uf.socket.is_empty() {
        out += "\n[Socket]\n";
        for key in &["ListenStream", "ListenDatagram", "Accept", "Service"] {
            if let Some(v) = get_kv(&uf.socket, key) {
                out += &format!("  {:<20} {}\n", key, v);
            }
        }
    }

    if !wanted_by.is_empty() || !required_by.is_empty() {
        out += "\n[Install]\n";
        if !wanted_by.is_empty() {
            out += &format!("  WantedBy:      {}\n", wanted_by.join(", "));
        }
        if !required_by.is_empty() {
            out += &format!("  RequiredBy:    {}\n", required_by.join(", "));
        }
    }

    Ok(out)
}

fn service_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let uf = parse_unit(&text);

    if uf.service.is_empty() {
        return Ok("No [Service] section found in unit file.\n".to_string());
    }

    let mut out = format!("Service Section\n{}\n\n", "=".repeat(44));

    let stype = get_kv(&uf.service, "Type").unwrap_or("simple");
    out += &format!("Type: {} — {}\n\n", stype, explain_service_type(stype));

    let exec_keys = [
        "ExecStartPre",
        "ExecStart",
        "ExecStartPost",
        "ExecReload",
        "ExecStop",
        "ExecStopPost",
    ];
    let has_exec = exec_keys.iter().any(|k| get_kv(&uf.service, k).is_some());
    if has_exec {
        out += "Exec:\n";
        for key in &exec_keys {
            if let Some(v) = get_kv(&uf.service, key) {
                out += &format!("  {:<18} {}\n", key, v);
            }
        }
        out += "\n";
    }

    let identity_keys = ["User", "Group", "DynamicUser", "SupplementaryGroups"];
    let has_identity = identity_keys
        .iter()
        .any(|k| get_kv(&uf.service, k).is_some());
    if has_identity {
        out += "Identity:\n";
        for key in &identity_keys {
            if let Some(v) = get_kv(&uf.service, key) {
                out += &format!("  {:<18} {}\n", key, v);
            }
        }
        out += "\n";
    }

    let restart_keys = [
        "Restart",
        "RestartSec",
        "StartLimitInterval",
        "StartLimitBurst",
    ];
    let has_restart = restart_keys
        .iter()
        .any(|k| get_kv(&uf.service, k).is_some());
    if has_restart {
        out += "Restart policy:\n";
        for key in &restart_keys {
            if let Some(v) = get_kv(&uf.service, key) {
                out += &format!("  {:<18} {}\n", key, v);
            }
        }
        out += "\n";
    }

    let env_keys: Vec<&str> = uf
        .service
        .iter()
        .filter(|(k, _)| {
            k.eq_ignore_ascii_case("Environment") || k.eq_ignore_ascii_case("EnvironmentFile")
        })
        .map(|(k, _)| k.as_str())
        .collect();
    if !env_keys.is_empty() {
        out += "Environment:\n";
        for (k, v) in &uf.service {
            let kl = k.to_lowercase();
            if kl == "environment" || kl == "environmentfile" {
                out += &format!("  {:<18} {}\n", k, v);
            }
        }
        out += "\n";
    }

    let security_keys = [
        "NoNewPrivileges",
        "PrivateTmp",
        "ProtectSystem",
        "ProtectHome",
        "ReadOnlyPaths",
        "InaccessiblePaths",
        "CapabilityBoundingSet",
        "AmbientCapabilities",
        "SecureBits",
    ];
    let has_security = security_keys
        .iter()
        .any(|k| get_kv(&uf.service, k).is_some());
    if has_security {
        out += "Security hardening:\n";
        for key in &security_keys {
            if let Some(v) = get_kv(&uf.service, key) {
                out += &format!("  {:<24} {}\n", key, v);
            }
        }
        out += "\n";
    }

    let misc_shown: std::collections::HashSet<&str> = exec_keys
        .iter()
        .chain(identity_keys.iter())
        .chain(restart_keys.iter())
        .chain(security_keys.iter())
        .chain(&["Environment", "EnvironmentFile", "Type"])
        .copied()
        .collect();

    let remaining: Vec<_> = uf
        .service
        .iter()
        .filter(|(k, _)| !misc_shown.iter().any(|s| s.eq_ignore_ascii_case(k)))
        .collect();
    if !remaining.is_empty() {
        out += "Other:\n";
        for (k, v) in &remaining {
            out += &format!("  {:<24} {}\n", k, v);
        }
    }

    Ok(out)
}

fn explain_calendar(cal: &str) -> String {
    match cal.to_lowercase().trim() {
        "daily" => "every day at midnight".to_string(),
        "weekly" => "every Monday at midnight".to_string(),
        "monthly" => "first day of every month at midnight".to_string(),
        "hourly" => "every hour at :00".to_string(),
        "minutely" => "every minute".to_string(),
        "annually" | "yearly" => "once a year (Jan 1 midnight)".to_string(),
        "quarterly" => "Jan 1, Apr 1, Jul 1, Oct 1 at midnight".to_string(),
        "semi-annually" => "Jan 1 and Jul 1 at midnight".to_string(),
        other => format!("expression: {}", other),
    }
}

fn timer_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let uf = parse_unit(&text);

    if uf.timer.is_empty() {
        return Ok("No [Timer] section found in unit file.\n".to_string());
    }

    let mut out = format!("Timer Section\n{}\n\n", "=".repeat(44));

    if let Some(cal) = get_kv(&uf.timer, "OnCalendar") {
        out += &format!("OnCalendar:      {}\n", cal);
        out += &format!("  ({})\n", explain_calendar(cal));
    }
    if let Some(v) = get_kv(&uf.timer, "OnBootSec") {
        out += &format!("OnBootSec:       {}\n", v);
    }
    if let Some(v) = get_kv(&uf.timer, "OnActiveSec") {
        out += &format!("OnActiveSec:     {}\n", v);
    }
    if let Some(v) = get_kv(&uf.timer, "OnUnitActiveSec") {
        out += &format!("OnUnitActiveSec: {}\n", v);
    }
    if let Some(v) = get_kv(&uf.timer, "OnUnitInactiveSec") {
        out += &format!("OnUnitInactiveSec: {}\n", v);
    }
    if let Some(v) = get_kv(&uf.timer, "AccuracySec") {
        out += &format!("AccuracySec:     {}\n", v);
    }
    if let Some(v) = get_kv(&uf.timer, "RandomizedDelaySec") {
        out += &format!("RandomizedDelay: {}\n", v);
    }
    let persistent = get_kv(&uf.timer, "Persistent").unwrap_or("no");
    out += &format!("Persistent:      {}\n", persistent);
    if let Some(unit) = get_kv(&uf.timer, "Unit") {
        out += &format!("Unit:            {}\n", unit);
    }
    if persistent.eq_ignore_ascii_case("yes") || persistent.eq_ignore_ascii_case("true") {
        out += "\n  Persistent=yes: if the timer missed a run (machine was off), it fires immediately on next start.\n";
    }

    if !uf.unit.is_empty() {
        let desc = get_kv(&uf.unit, "Description").unwrap_or("(none)");
        out += &format!("\n[Unit] Description: {}\n", desc);
    }
    if !uf.install.is_empty() {
        let wanted_by = get_all_kv(&uf.install, "WantedBy");
        if !wanted_by.is_empty() {
            out += &format!("[Install] WantedBy: {}\n", wanted_by.join(", "));
        }
    }

    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let uf = parse_unit(&text);
    let mut warnings: Vec<String> = Vec::new();

    let unit_type = detect_unit_type(&uf);

    if uf.unit.is_empty() {
        warnings.push(
            "No [Unit] section found — Description and dependency directives should live here"
                .to_string(),
        );
    } else if get_kv(&uf.unit, "Description").is_none() {
        warnings.push(
            "Missing Description in [Unit] — makes it hard to identify the service".to_string(),
        );
    }

    if unit_type == "service" {
        if get_kv(&uf.service, "ExecStart").is_none() {
            warnings.push(
                "Missing ExecStart in [Service] — required for all non-oneshot services"
                    .to_string(),
            );
        }

        let stype = get_kv(&uf.service, "Type").unwrap_or("simple");
        if stype.eq_ignore_ascii_case("forking") && get_kv(&uf.service, "PIDFile").is_none() {
            warnings.push(
                "Type=forking without PIDFile — systemd may track the wrong process".to_string(),
            );
        }

        if get_kv(&uf.service, "Restart").is_none() {
            warnings.push(
                "No Restart= directive — service will not auto-restart on failure".to_string(),
            );
        }

        let user = get_kv(&uf.service, "User");
        if user.is_none() {
            warnings.push(
                "No User= directive — service will run as root; prefer a dedicated non-root user"
                    .to_string(),
            );
        }

        if get_kv(&uf.service, "NoNewPrivileges").is_none() {
            warnings.push(
                "NoNewPrivileges=yes not set — consider adding for security hardening".to_string(),
            );
        }
        if get_kv(&uf.service, "PrivateTmp").is_none() {
            warnings.push(
                "PrivateTmp=yes not set — /tmp is shared; consider isolating with PrivateTmp=yes"
                    .to_string(),
            );
        }
    }

    if unit_type == "timer" {
        let has_trigger = get_kv(&uf.timer, "OnCalendar").is_some()
            || get_kv(&uf.timer, "OnBootSec").is_some()
            || get_kv(&uf.timer, "OnActiveSec").is_some()
            || get_kv(&uf.timer, "OnUnitActiveSec").is_some();
        if !has_trigger {
            warnings.push("Timer has no trigger directive (OnCalendar/OnBootSec/OnActiveSec/OnUnitActiveSec) — will never fire".to_string());
        }
    }

    if uf.install.is_empty() {
        warnings.push(
            "No [Install] section — unit cannot be enabled with `systemctl enable`".to_string(),
        );
    } else {
        let wanted_by = get_all_kv(&uf.install, "WantedBy");
        let required_by = get_all_kv(&uf.install, "RequiredBy");
        if wanted_by.is_empty() && required_by.is_empty() {
            warnings.push(
                "Empty [Install] section — add WantedBy=multi-user.target to enable at boot"
                    .to_string(),
            );
        }
    }

    let mut out = format!("Systemd Unit Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!("Unit type: {}\n", unit_type);
    out += &format!("Sections:  {}\n", uf.sections.join(", "));
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
