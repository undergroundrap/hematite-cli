use hematite::agent::routing::{
    all_host_inspection_topics, mentions_commit_intent, mentions_symbol_search,
    preferred_host_inspection_topic, preferred_workspace_workflow,
};

#[test]
fn test_diagnostic_priority_collision_fix() {
    let input = "check for any pending reboot reasons";
    assert_eq!(preferred_host_inspection_topic(input), Some("pending_reboot"));
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
    assert_eq!(
        preferred_workspace_workflow(input),
        Some("commit_workflow")
    );
}

#[test]
fn test_troubleshooting_priority_reordered() {
    // env_doctor should now be high priority.
    let input = "my environment is broken and I have some hardware errors";
    // "environment is broken" -> env_doctor. "hardware error" -> device_health.
    // env_doctor is now reordered higher.
    assert_eq!(preferred_host_inspection_topic(input), Some("env_doctor"));
}
