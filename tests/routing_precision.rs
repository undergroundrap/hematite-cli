use hematite::agent::routing::{
    all_host_inspection_topics, mentions_commit_intent, mentions_symbol_search,
    preferred_host_inspection_topic, preferred_workspace_workflow,
};

#[test]
fn test_diagnostic_priority_collision_fix() {
    let input = "check for any pending reboot reasons";
    assert_eq!(
        preferred_host_inspection_topic(input),
        Some("pending_reboot")
    );
}

#[test]
fn test_multi_topic_detection_precision() {
    let input = "Give me a tiered health report of my workstation, then check for any pending reboot reasons and my current battery wear level.";
    let topics = all_host_inspection_topics(input);

    assert!(topics.contains(&"health_report"));
    assert!(topics.contains(&"pending_reboot"));
    assert!(topics.contains(&"battery"));
}

#[test]
fn test_folder_creation_routing() {
    let input = "Make a folder on my desktop named 'HematiteDev'";
    // This should NOT route to a host inspection topic, as it is a mutation action.
    assert_eq!(preferred_host_inspection_topic(input), None);
}

#[test]
fn test_lsp_symbol_routing() {
    let input = "Find where the 'initialize_mcp' function is defined";
    assert!(mentions_symbol_search(input));
    assert_eq!(preferred_workspace_workflow(input), Some("lsp_search"));
}

#[test]
fn test_commit_intent_routing() {
    let input = "Commit my progress to git";
    assert!(mentions_commit_intent(input));
    assert_eq!(preferred_workspace_workflow(input), Some("commit_workflow"));
}

#[test]
fn test_troubleshooting_priority_reordered() {
    // env_doctor should now be high priority.
    let input = "my environment is broken and I have some hardware errors";
    // "environment is broken" -> env_doctor. "hardware error" -> device_health.
    // env_doctor is now reordered higher.
    assert_eq!(preferred_host_inspection_topic(input), Some("env_doctor"));
}

#[test]
fn test_routing_detects_outlook_topic() {
    assert_eq!(
        preferred_host_inspection_topic("Check Outlook health on this machine."),
        Some("outlook")
    );
    assert_eq!(
        preferred_host_inspection_topic("Why is Outlook so slow or broken?"),
        Some("outlook")
    );
    assert_eq!(
        preferred_host_inspection_topic(
            "Audit Outlook profiles, OST/PST files, and add-in pressure."
        ),
        Some("outlook")
    );
}

#[test]
fn test_routing_outlook_in_multi_topic() {
    let topics = all_host_inspection_topics(
        "Why is Outlook crashing? Also check if the machine has any pending reboots.",
    );
    assert!(topics.contains(&"outlook"), "should detect outlook");
    assert!(
        topics.contains(&"pending_reboot"),
        "should detect pending_reboot"
    );
}

#[test]
fn test_routing_detects_teams_topic() {
    assert_eq!(
        preferred_host_inspection_topic("Check Teams health on this machine."),
        Some("teams")
    );
    assert_eq!(
        preferred_host_inspection_topic("Why is Microsoft Teams so slow or broken?"),
        Some("teams")
    );
    assert_eq!(
        preferred_host_inspection_topic("Audit Teams cache size and WebView2 dependency."),
        Some("teams")
    );
}

#[test]
fn test_routing_teams_does_not_match_nic_teaming() {
    assert_ne!(
        preferred_host_inspection_topic("Show NIC teaming configuration and LACP status."),
        Some("teams")
    );
}

#[test]
fn test_routing_teams_in_multi_topic() {
    let topics = all_host_inspection_topics(
        "Why is Teams crashing? Also check if the machine has any pending reboots.",
    );
    assert!(topics.contains(&"teams"), "should detect teams");
    assert!(
        topics.contains(&"pending_reboot"),
        "should detect pending_reboot"
    );
}

#[test]
fn test_routing_detects_windows_backup_topic() {
    assert_eq!(
        preferred_host_inspection_topic("Is this machine being backed up?"),
        Some("windows_backup")
    );
    assert_eq!(
        preferred_host_inspection_topic("Check Windows backup health and File History status."),
        Some("windows_backup")
    );
    assert_eq!(
        preferred_host_inspection_topic("Show me my System Restore points."),
        Some("windows_backup")
    );
}

#[test]
fn test_routing_windows_backup_in_multi_topic() {
    let topics = all_host_inspection_topics(
        "Check Windows backup health and also show me whether the disk is healthy.",
    );
    assert!(
        topics.contains(&"windows_backup"),
        "should detect windows_backup"
    );
    assert!(topics.contains(&"disk_health"), "should detect disk_health");
}

#[test]
fn test_routing_detects_hyperv_topic() {
    assert_eq!(
        preferred_host_inspection_topic("List all virtual machines on this machine."),
        Some("hyperv")
    );
    assert_eq!(
        preferred_host_inspection_topic("Check Hyper-V health and VM states."),
        Some("hyperv")
    );
    assert_eq!(
        preferred_host_inspection_topic("How much RAM are my running VMs using?"),
        Some("hyperv")
    );
}

#[test]
fn test_routing_hyperv_in_multi_topic() {
    let topics = all_host_inspection_topics(
        "Show me all running VMs and also check the system resource load.",
    );
    assert!(topics.contains(&"hyperv"), "should detect hyperv");
    assert!(
        topics.contains(&"resource_load"),
        "should detect resource_load"
    );
}

#[test]
fn test_routing_detects_app_crashes_topic() {
    assert_eq!(
        preferred_host_inspection_topic("What applications have been crashing on this machine?"),
        Some("app_crashes")
    );
    assert_eq!(
        preferred_host_inspection_topic("Show me application crash history."),
        Some("app_crashes")
    );
    assert_eq!(
        preferred_host_inspection_topic(
            "What is the faulting application name from the last crash?"
        ),
        Some("app_crashes")
    );
    assert_eq!(
        preferred_host_inspection_topic("What programs crashed recently on this machine?"),
        Some("app_crashes")
    );
    assert_eq!(
        preferred_host_inspection_topic("Which apps keep crashing on this machine?"),
        Some("app_crashes")
    );
}

#[test]
fn test_routing_app_crashes_in_multi_topic() {
    let topics =
        all_host_inspection_topics("Show application crashes and check system resource load.");
    assert!(topics.contains(&"app_crashes"), "should detect app_crashes");
    assert!(
        topics.contains(&"resource_load"),
        "should detect resource_load"
    );
}

// ── 0.8.0 wave topics ──────────────────────────────────────────────────────

#[test]
fn test_routing_detects_mdm_enrollment_topic() {
    assert_eq!(
        preferred_host_inspection_topic("Is this machine enrolled in Intune?"),
        Some("mdm_enrollment")
    );
    assert_eq!(
        preferred_host_inspection_topic("Check MDM enrollment state on this device."),
        Some("mdm_enrollment")
    );
    assert_eq!(
        preferred_host_inspection_topic("Show Autopilot and device enrollment status."),
        Some("mdm_enrollment")
    );
}

#[test]
fn test_routing_detects_storage_spaces_topic() {
    assert_eq!(
        preferred_host_inspection_topic("Check Windows Storage Spaces health."),
        Some("storage_spaces")
    );
    assert_eq!(
        preferred_host_inspection_topic("Show storage pool status and virtual disk health."),
        Some("storage_spaces")
    );
    assert_eq!(
        preferred_host_inspection_topic("Is my storage pool degraded?"),
        Some("storage_spaces")
    );
}

#[test]
fn test_routing_detects_defender_quarantine_topic() {
    assert_eq!(
        preferred_host_inspection_topic("Show Defender quarantine history."),
        Some("defender_quarantine")
    );
    assert_eq!(
        preferred_host_inspection_topic("What malware has Defender detected on this machine?"),
        Some("defender_quarantine")
    );
    assert_eq!(
        preferred_host_inspection_topic("Show threat history and detection log."),
        Some("defender_quarantine")
    );
}

#[test]
fn test_routing_detects_domain_health_topic() {
    assert_eq!(
        preferred_host_inspection_topic("Can this machine reach its domain controller?"),
        Some("domain_health")
    );
    assert_eq!(
        preferred_host_inspection_topic("Check DC connectivity and LDAP port reachability."),
        Some("domain_health")
    );
    assert_eq!(
        preferred_host_inspection_topic("Is Kerberos working and can the machine reach the DC?"),
        Some("domain_health")
    );
}

#[test]
fn test_routing_detects_service_dependencies_topic() {
    assert_eq!(
        preferred_host_inspection_topic("What services depend on the Print Spooler?"),
        Some("service_dependencies")
    );
    assert_eq!(
        preferred_host_inspection_topic("Show the service dependency graph."),
        Some("service_dependencies")
    );
    assert_eq!(
        preferred_host_inspection_topic("What will break if I stop this service?"),
        Some("service_dependencies")
    );
}

#[test]
fn test_routing_detects_wmi_health_topic() {
    assert_eq!(
        preferred_host_inspection_topic("Check WMI repository health."),
        Some("wmi_health")
    );
    assert_eq!(
        preferred_host_inspection_topic("Is the WMI repository corrupt?"),
        Some("wmi_health")
    );
    assert_eq!(
        preferred_host_inspection_topic("WMI is broken and tools keep failing."),
        Some("wmi_health")
    );
}

#[test]
fn test_routing_detects_local_security_policy_topic() {
    assert_eq!(
        preferred_host_inspection_topic("What is the local password policy on this machine?"),
        Some("local_security_policy")
    );
    assert_eq!(
        preferred_host_inspection_topic("Check account lockout threshold and policy."),
        Some("local_security_policy")
    );
    assert_eq!(
        preferred_host_inspection_topic("Show the NTLM authentication level."),
        Some("local_security_policy")
    );
}

#[test]
fn test_routing_detects_usb_history_topic() {
    assert_eq!(
        preferred_host_inspection_topic("Show USB device history for this machine."),
        Some("usb_history")
    );
    assert_eq!(
        preferred_host_inspection_topic("What USB storage devices have been connected?"),
        Some("usb_history")
    );
    assert_eq!(
        preferred_host_inspection_topic("USB forensic audit — what drives have been plugged in?"),
        Some("usb_history")
    );
}

#[test]
fn test_routing_detects_print_spooler_topic() {
    assert_eq!(
        preferred_host_inspection_topic("Check the Print Spooler service state."),
        Some("print_spooler")
    );
    assert_eq!(
        preferred_host_inspection_topic("Is this machine vulnerable to PrintNightmare?"),
        Some("print_spooler")
    );
    assert_eq!(
        preferred_host_inspection_topic("Show spooler service and printer queue."),
        Some("print_spooler")
    );
}

#[test]
fn test_routing_0_8_0_wave_multi_topic() {
    let topics = all_host_inspection_topics(
        "Check MDM enrollment, WMI repository health, and storage pool status.",
    );
    assert!(
        topics.contains(&"mdm_enrollment"),
        "should detect mdm_enrollment"
    );
    assert!(topics.contains(&"wmi_health"), "should detect wmi_health");
    assert!(
        topics.contains(&"storage_spaces"),
        "should detect storage_spaces"
    );
}
