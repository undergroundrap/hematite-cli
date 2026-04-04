use std::process::Command;
use serde_json::Value;

/// tool: git_onboarding
/// 
/// Action: Configures or updates a Git remote (usually 'origin') and optionally performs an initial push.
pub async fn execute(args: &Value) -> Result<String, String> {
    let url = args.get("url").and_then(|v| v.as_str()).ok_or("Missing 'url' argument")?;
    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("origin");
    let do_push = args.get("push").and_then(|v| v.as_bool()).unwrap_or(false);

    // Security Audit
    if !url.starts_with("https://") && !url.starts_with("git@") {
        return Err("Invalid remote URL. Must be HTTPS or SSH.".into());
    }

    // 1. Check if remote exists
    let check = Command::new("git").args(["remote", "get-url", name]).output().map_err(|e| e.to_string())?;
    
    if check.status.success() {
        // Already exists, update it
        let set_url = Command::new("git").args(["remote", "set-url", name, url]).output().map_err(|e| e.to_string())?;
        if !set_url.status.success() {
            return Err(format!("Failed to update remote: {}", String::from_utf8_lossy(&set_url.stderr)));
        }
    } else {
        // New remote
        let add = Command::new("git").args(["remote", "add", name, url]).output().map_err(|e| e.to_string())?;
        if !add.status.success() {
            return Err(format!("Failed to add remote: {}", String::from_utf8_lossy(&add.stderr)));
        }
    }

    let mut status = format!("Successfully configured remote '{}' to {}.", name, url);

    // 2. Optional initial push
    if do_push {
        let push = Command::new("git")
            .args(["push", "-u", name, "HEAD"])
            .output()
            .map_err(|e| e.to_string())?;
        
        if push.status.success() {
            status.push_str("\nInitial push complete. Branch tracking established.");
        } else {
            status.push_str(&format!("\nWarning: Push failed: {}. You may need to authenticate or handle branch conflicts manually.", String::from_utf8_lossy(&push.stderr)));
        }
    }

    Ok(status)
}
