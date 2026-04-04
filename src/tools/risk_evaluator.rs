use super::tool::RiskLevel;

/// Local Risk Evaluator — a fast, deterministic triage pass that classifies
/// Swarm worker actions by risk level. LOW risk actions auto-approve silently,
/// MODERATE actions log a warning, and HIGH risk actions trigger the Red Modal.
///
/// This eliminates the UX bottleneck of prompting the user for every single
/// file read while still enforcing hard safety gates on destructive operations.
#[allow(dead_code)]
pub struct RiskEvaluator;

#[allow(dead_code)]
impl RiskEvaluator {
    /// Classifies a tool invocation based on its name and arguments.
    /// Returns the RiskLevel which determines whether the action
    /// auto-approves, warns, or blocks behind the Red Modal.
    pub fn classify(tool_name: &str, args_preview: &str) -> RiskLevel {
        // HIGH RISK: Anything that can destroy data, push to remotes,
        // or modify system-level configuration. Always blocks.
        let high_risk_patterns = [
            "rm ", "rm -", "del ", "rmdir",
            "git push", "git reset --hard",
            "format-volume", "diskpart",
            "system32", "C:\\Windows",
            "shutdown", "taskkill",
            ".env", ".ssh", ".gitconfig",
            "curl ", "wget ", "Invoke-WebRequest",
            "chmod 777", "sudo ",
        ];

        for pattern in &high_risk_patterns {
            if args_preview.contains(pattern) {
                return RiskLevel::High;
            }
        }

        // HIGH RISK: Tool-level classification for inherently destructive tools
        match tool_name {
            "BashTool" | "PowerShellTool" => {
                // Bash is MODERATE by default — it could do anything.
                // But specific args above escalate it to HIGH.
                return RiskLevel::Moderate;
            }
            "FileWriteTool" | "FileEditTool" => {
                return RiskLevel::Moderate;
            }
            "git_commit" => {
                return RiskLevel::High;
            }
            _ => {}
        }

        // SAFE: Read-only operations that can never modify state.
        let safe_tools = [
            "FileReadTool", "GlobTool", "GrepTool",
            "SlimLspTool", "ToolSearchTool",
        ];

        if safe_tools.contains(&tool_name) {
            return RiskLevel::Safe;
        }

        // Default: anything unknown is MODERATE — log but don't block
        RiskLevel::Moderate
    }

    /// Returns true if the action can be auto-approved without user input.
    pub fn can_auto_approve(level: RiskLevel, yolo_mode: bool) -> bool {
        match level {
            RiskLevel::Safe => true,
            RiskLevel::Moderate => yolo_mode, // Only auto-approve in YOLO mode
            RiskLevel::High => false,         // NEVER auto-approve, even in YOLO
        }
    }
}
