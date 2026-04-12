use serde_json::{json, Value};

pub fn get_scoping_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "auto_pin_context",
            "description": "Select 1-3 core files to 'Lock' into high-fidelity memory. \
                             Use this to ensure the most important architecture files \
                             are always visible during complex refactorings.",
            "parameters": {
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of relative paths to pin (max 3)."
                    },
                    "reason": {
                        "type": "string",
                        "description": "Brief explanation of why these files are the project's 'Core' for the current task."
                    }
                },
                "required": ["paths", "reason"]
            }
        }),
        json!({
            "name": "list_pinned",
            "description": "List all files currently pinned in the model's active context.",
            "parameters": {
                "type": "object",
                "properties": {}
            }
        }),
    ]
}
