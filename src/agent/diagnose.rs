/// Staged triage engine for /diagnose. Phase 1 harness, Phase 2 agent synthesis.

/// Parse health_report text and return determined follow-up inspect_host topics.
pub fn triage_follow_up_topics(health_output: &str) -> Vec<&'static str> {
    let lower = health_output.to_ascii_lowercase();
    let mut topics: Vec<&'static str> = Vec::new();

    let action_required = lower.contains("action required");
    let worth_a_look = lower.contains("worth a look");
    if !action_required && !worth_a_look {
        return topics; // ALL GOOD — no follow-up needed
    }

    if lower.contains("[!]") && (lower.contains("disk") || lower.contains("drive")) {
        topics.push("storage");
        topics.push("disk_health");
    } else if lower.contains("[-]") && (lower.contains("disk") || lower.contains("drive")) {
        topics.push("storage");
    }

    if (lower.contains("[!]") || lower.contains("[-]")) && lower.contains("ram") {
        topics.push("resource_load");
        topics.push("processes");
    }

    if lower.contains("critical") || lower.contains("error event") {
        if lower.contains("event") {
            topics.push("log_check");
        }
    }

    if lower.contains("[!]") && lower.contains("service") {
        topics.push("services");
    } else if lower.contains("[-]") && lower.contains("service") {
        topics.push("services");
    }

    if (lower.contains("[!]") || lower.contains("[-]"))
        && (lower.contains("defender") || lower.contains("firewall") || lower.contains("security"))
    {
        topics.push("security");
    }

    if lower.contains("[!]") && lower.contains("internet connectivity") {
        topics.push("connectivity");
        topics.push("network");
    } else if lower.contains("[-]") && lower.contains("internet connectivity") {
        topics.push("connectivity");
    }

    if lower.contains("pending reboot") {
        topics.push("pending_reboot");
    }

    if (lower.contains("[!]") || lower.contains("[-]"))
        && (lower.contains("thermal") || lower.contains("°c"))
    {
        topics.push("thermal");
        topics.push("overclocker");
    }

    let mut seen = std::collections::HashSet::new();
    topics.retain(|t| seen.insert(*t));

    topics
}

/// Examine the combined output from a --fix phase-1 topic run and return
/// additional topics to drill into. Caps at 3 to keep the command fast.
pub fn fix_follow_up_topics(
    combined_output: &str,
    already_ran: &[&str],
) -> Vec<(&'static str, &'static str)> {
    let lower = combined_output.to_ascii_lowercase();
    let ran: std::collections::HashSet<&str> = already_ran.iter().copied().collect();
    let mut candidates: Vec<(&'static str, &'static str)> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    macro_rules! add {
        ($topic:expr, $label:expr, $cond:expr) => {
            if $cond && !ran.contains($topic) && seen.insert($topic) {
                candidates.push(($topic, $label));
            }
        };
    }

    add!(
        "processes",
        "Top Processes",
        lower.contains("very high") && (lower.contains("cpu") || lower.contains("processor"))
    );

    add!(
        "cpu_power",
        "CPU Power",
        lower.contains("throttling") || (lower.contains("very high") && lower.contains("°c"))
    );

    add!(
        "app_crashes",
        "Application Crashes",
        lower.contains("very high") && lower.contains("memory")
    );

    add!(
        "shadow_copies",
        "Shadow Copies",
        (lower.contains("unhealthy") || lower.contains("predictive failure"))
            && lower.contains("disk")
    );

    add!(
        "log_check",
        "Event Log",
        lower.contains("unexpected shutdown")
            || lower.contains("kernel: critical")
            || lower.contains("stop error")
    );

    add!(
        "services",
        "Services",
        lower.contains("critical/error event")
            || lower.contains("error events in windows event log")
    );

    add!(
        "wifi",
        "Wi-Fi",
        lower.contains("unreachable") && !lower.contains("reachable: yes")
    );

    add!(
        "connectivity",
        "Connectivity",
        lower.contains("dns resolution: failed") || lower.contains("dns: failed")
    );

    add!(
        "defender_quarantine",
        "Defender Quarantine",
        lower.contains("real-time protection: disabled") || lower.contains("threat detected")
    );

    add!(
        "identity_auth",
        "Identity & Auth",
        (lower.contains("teams") || lower.contains("outlook"))
            && (lower.contains("sign-in fail")
                || lower.contains("auth fail")
                || lower.contains("token broker"))
    );

    add!(
        "credentials",
        "Credentials",
        lower.contains("token broker: not running")
            || lower.contains("wam: not running")
            || lower.contains("aad broker plugin: not found")
    );

    candidates.truncate(3);
    candidates
}

/// Build the agent instruction for phase 2 of /diagnose.
pub fn build_diagnose_instruction(health_output: &str, follow_up_topics: &[&str]) -> String {
    if follow_up_topics.is_empty() {
        return format!(
            "DIAGNOSE MODE — triage complete.\n\n\
             Health report:\n{}\n\n\
             The machine is in good health. Summarize the key findings for the user \
             in 2-3 sentences and confirm no action is needed.",
            health_output
        );
    }

    let topic_list = follow_up_topics
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{}. inspect_host(topic=\"{}\")", i + 1, t))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "DIAGNOSE MODE — harness triage identified {} area(s) to investigate.\n\n\
         Health report (already run by harness):\n{}\n\n\
         PROTOCOL — follow this exactly:\n\
         Call each topic below in order:\n{}\n\n\
         After all calls complete:\n\
         - Write a numbered fix plan grounded in the tool output\n\
         - Lead with the most critical issue first\n\
         - Every step must reference specific data from the results (exact path, count, service name, etc.)\n\
         - No generic advice — only steps that address what the tools actually found\n\
         - If a finding needs a restart or elevated privileges, say so explicitly",
        follow_up_topics.len(),
        health_output,
        topic_list
    )
}
