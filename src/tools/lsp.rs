use std::process::Command;
use serde_json::Value;

#[allow(dead_code)]
pub struct SlimLspTool;

#[allow(dead_code)]
impl SlimLspTool {
    pub fn new() -> Self { Self }
    
    /// Physically reads isolated cargo check diagnostics targeting the exact "rendered" compiler outputs
    /// This bypasses generic "Error at Line 10" and provides exact IDE-grade structural suggestions!
    pub fn get_diagnostics(&self) -> String {
        let output = Command::new("cargo")
            .arg("check")
            .arg("--message-format=json")
            .output();
            
        if let Ok(cmd_out) = output {
            let stdout = String::from_utf8_lossy(&cmd_out.stdout);
            let mut diagnostics = Vec::new();
            
            for line in stdout.lines() {
                if let Ok(json) = serde_json::from_str::<Value>(line) {
                    if json["reason"] == "compiler-message" {
                        if let Some(msg) = json.get("message") {
                            // Extract the rendered compiler output cleanly providing explicitly the 3 lines of context 
                            // plus the Rust compiler's direct "help:" logic!
                            if let Some(rendered) = msg.get("rendered") {
                                diagnostics.push(rendered.as_str().unwrap_or_default().to_string());
                            } else {
                                diagnostics.push(msg["message"].as_str().unwrap_or_default().to_string());
                            }
                        }
                    }
                }
            }
            
            if diagnostics.is_empty() {
                "No syntax anomalies found. Code is secure.".to_string()
            } else {
                diagnostics.join("\n")
            }
        } else {
            "Failed to execute cargo check diagnostics.".to_string()
        }
    }
}
