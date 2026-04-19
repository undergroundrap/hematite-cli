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
