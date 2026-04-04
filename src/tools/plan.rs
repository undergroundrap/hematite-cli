use serde_json::{Value, json};
use std::fs;
use crate::tools::file_ops::workspace_root;

/// Manages a persistent mission plan for the agent in `.hematite/PLAN.md`.
pub async fn maintain_plan(args: &Value) -> Result<String, String> {
    let blueprint = args.get("blueprint").and_then(|v| v.as_str())
        .ok_or("maintain_plan: 'blueprint' (markdown text) required")?;
    let plan_path = workspace_root().join(".hematite").join("PLAN.md");

    fs::create_dir_all(plan_path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&plan_path, blueprint).map_err(|e| format!("Failed to write plan: {e}"))?;
    
    Ok(format!("Strategic Blueprint updated in .hematite/PLAN.md ({} bytes)", blueprint.len()))
}

/// Generates a final walkthrough report for the current session.
pub async fn generate_walkthrough(args: &Value) -> Result<String, String> {
    let summary = args.get("summary").and_then(|v| v.as_str())
        .ok_or("generate_walkthrough: 'summary' required")?;
    let path = workspace_root().join(".hematite").join("WALKTHROUGH.md");

    fs::write(&path, summary).map_err(|e| format!("Failed to save walkthrough: {e}"))?;
    
    Ok(format!("Walkthrough report saved to .hematite/WALKTHROUGH.md. Session complete!"))
}

pub fn get_plan_params() -> Value {
    json!({
        "type": "object",
        "properties": {
            "blueprint": {
                "type": "string",
                "description": "The full markdown content of the strategic blueprint."
            }
        },
        "required": ["blueprint"]
    })
}

pub fn get_walkthrough_params() -> Value {
    json!({
        "type": "object",
        "properties": {
            "summary": {
                "type": "string",
                "description": "The full markdown summary of accomplishments."
            }
        },
        "required": ["summary"]
    })
}
