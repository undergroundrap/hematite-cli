use Hematite_CLI::agent::conversation::ConversationManager;
use Hematite_CLI::agent::inference::InferenceEngine;
use Hematite_CLI::agent::config::HematiteConfig;
use Hematite_CLI::ui::gpu_monitor::GpuState;
use Hematite_CLI::agent::git_monitor::{GitState, GitRemoteStatus};
use serde_json::json;
use std::sync::Arc;
use std::fs;
use std::path::PathBuf;

#[tokio::test]
async fn test_sandbox_env_isolation() {
    let args = json!({
        // Use PowerShell syntax for environment variables on Windows
        "command": "echo $env:HOME" 
    });
    
    let result = Hematite_CLI::tools::shell::execute(&args).await.expect("Shell execution failed");
    
    // The output should contain our redirected sandbox path.
    // We trim to avoid issues with newlines and check for the signature substring.
    let trimmed_result = result.trim();
    assert!(trimmed_result.contains(".hematite\\sandbox") || trimmed_result.contains(".hematite/sandbox"), 
            "HOME was not redirected to sandbox. Got: {}", trimmed_result);
}

#[tokio::test]
async fn test_gpu_monitor_logic() {
    let state = GpuState::new();
    // Initially should be 0
    let (used, total) = state.read();
    assert_eq!(used, 0);
    assert_eq!(total, 0);
    assert_eq!(state.ratio(), 0.0);
    assert_eq!(state.label(), "N/A");

    // Mock some data (using atomics internally)
    state.used_mib.store(4096, std::sync::atomic::Ordering::Relaxed);
    state.total_mib.store(8192, std::sync::atomic::Ordering::Relaxed);
    
    assert_eq!(state.read(), (4096, 8192));
    assert_eq!(state.ratio(), 0.5);
    assert_eq!(state.label(), "4.0 GB / 8.0 GB");
}

#[tokio::test]
async fn test_git_monitor_initial_state() {
    let state = GitState::new();
    assert_eq!(state.status(), GitRemoteStatus::Unknown);
    assert_eq!(state.label(), "UNKNOWN");
    assert_eq!(state.url(), "None");
}

#[tokio::test]
async fn test_mission_control_task_parsing() {
    let root = PathBuf::from(".");
    let hematite_dir = root.join(".hematite");
    if !hematite_dir.exists() {
        fs::create_dir_all(&hematite_dir).unwrap();
    }
    let task_file = hematite_dir.join("TASK_TEST.md");
    
    // Write a mock task
    let mock_task = "# Objective: Implement Sovereign Diagnostics\n\n- [ ] Task 1";
    fs::write(&task_file, mock_task).unwrap();
    
    // Simulate the parsing logic used in tui.rs
    let content = fs::read_to_string(&task_file).unwrap_or_default();
    let objective = content.lines()
        .find(|l| l.starts_with("# Objective:"))
        .map(|l| l.replace("# Objective:", "").trim().to_string())
        .unwrap_or_else(|| "Standby".to_string());
        
    assert_eq!(objective, "Implement Sovereign Diagnostics");
    
    // Cleanup
    fs::remove_file(task_file).ok();
}

#[tokio::test]
async fn test_shell_timeout_kill() {
    let args = json!({
        "command": "ping 127.0.0.1 -n 10", 
        "timeout_ms": 200
    });
    
    let result = Hematite_CLI::tools::shell::execute(&args).await;
    
    assert!(result.is_err(), "Command should have timed out");
}

