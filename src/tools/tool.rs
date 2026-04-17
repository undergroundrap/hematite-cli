use serde_json::Value;

#[allow(dead_code)]
pub trait HematiteTool {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn risk_level(&self, args: &serde_json::Value) -> RiskLevel;
    fn mutation_label(&self, args: &serde_json::Value) -> Option<String>;

    /// Estimates the context window impact before execution
    fn estimate_token_cost(&self, args: &Value) -> usize;

    /// Mandatory security intercept prior to any execution
    fn security_audit(&self, args: &Value) -> Result<(), String>;

    /// Executes the tool in a dry-run mode
    fn dry_run(&self, args: Value) -> Result<String, String>;

    /// Executes the tool for real, potentially hitting permission guards
    fn run(&self, args: Value) -> Result<String, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Safe,
    Moderate,
    High,
}
