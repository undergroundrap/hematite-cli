use crate::tools::file_ops::hematite_dir;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

/// Manages a persistent TODO list for the agent in `.hematite/TASK.md`.
/// Actions: list, add, update, remove
pub async fn manage_tasks(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("list");
    let task_path = hematite_dir().join("TASK.md");

    match action {
        "list" => list_tasks(&task_path),
        "add" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or("manage_tasks: 'title' required for 'add'")?;
            add_task(&task_path, title)
        }
        "update" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_u64())
                .ok_or("manage_tasks: 'id' required for 'update'")? as usize;
            let status = args
                .get("status")
                .and_then(|v| v.as_str())
                .ok_or("manage_tasks: 'status' ([ ], [/], [x]) required for 'update'")?;
            update_task(&task_path, id, status)
        }
        "remove" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_u64())
                .ok_or("manage_tasks: 'id' required for 'remove'")? as usize;
            remove_task(&task_path, id)
        }
        _ => Err(format!("manage_tasks: unknown action '{action}'")),
    }
}

fn list_tasks(path: &PathBuf) -> Result<String, String> {
    if !path.exists() {
        return Ok("No task ledger found. Use 'add' to start tracking mission goals.".into());
    }
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read tasks: {e}"))?;
    Ok(format!(
        "--- TASK LEDGER (.hematite/TASK.md) ---\n\n{}",
        content
    ))
}

fn add_task(path: &PathBuf, title: &str) -> Result<String, String> {
    let mut tasks = if path.exists() {
        fs::read_to_string(path).unwrap_or_default()
    } else {
        String::new()
    };

    if !tasks.is_empty() && !tasks.ends_with('\n') {
        tasks.push('\n');
    }
    tasks.push_str(&format!("- [ ] {}\n", title));

    fs::create_dir_all(path.parent().expect("Invalid task path")).map_err(|e| e.to_string())?;
    fs::write(path, &tasks).map_err(|e| format!("Failed to write task: {e}"))?;

    Ok(format!("Added task: [ ] {}", title))
}

fn update_task(path: &PathBuf, id: usize, status: &str) -> Result<String, String> {
    if !path.exists() {
        return Err("Task ledger not found".into());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    if id < 1 || id > lines.len() {
        return Err(format!(
            "Invalid task ID {id}. Ledger has {} items.",
            lines.len()
        ));
    }

    let line = &mut lines[id - 1];
    if line.starts_with("- [") && line.len() >= 5 {
        // Update the [ ] status (index 3)
        let new_line = format!(
            "- [{}] {}",
            status.trim_matches(|c| c == '[' || c == ']' || c == ' '),
            &line[6..]
        );
        *line = new_line;
    } else {
        return Err("Target line is not a valid task format".into());
    }

    fs::write(path, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    Ok(format!("Updated task {id} to status [{}]", status))
}

fn remove_task(path: &PathBuf, id: usize) -> Result<String, String> {
    if !path.exists() {
        return Err("Task ledger not found".into());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    if id < 1 || id > lines.len() {
        return Err(format!("Invalid task ID {id}."));
    }

    let removed = lines.remove(id - 1);
    fs::write(path, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    Ok(format!("Removed task: {}", removed))
}

pub fn get_tasks_params() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "description": "The action to perform: 'list', 'add', 'update', 'remove'.",
                "enum": ["list", "add", "update", "remove"]
            },
            "id": {
                "type": "integer",
                "description": "The 1-based ID of the task to update or remove."
            },
            "title": {
                "type": "string",
                "description": "The description of the task (required for 'add')."
            },
            "status": {
                "type": "string",
                "description": "The status to set: '[ ]' (todo), '[/]' (in-progress), '[x]' (done).",
                "enum": [" ", "/", "x", "[ ]", "[/]", "[x]"]
            }
        },
        "required": ["action"]
    })
}
