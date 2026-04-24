use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HematiteTask {
    pub id: usize,
    pub text: String,
    pub done: bool,
}

fn store_path() -> PathBuf {
    crate::tools::file_ops::hematite_dir().join("tasks.json")
}

pub fn load() -> Vec<HematiteTask> {
    let path = store_path();
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save(tasks: &[HematiteTask]) {
    let path = store_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(tasks) {
        let _ = std::fs::write(&path, json);
    }
}

pub fn add(text: &str) -> Vec<HematiteTask> {
    let mut tasks = load();
    let next_id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    tasks.push(HematiteTask {
        id: next_id,
        text: text.trim().to_string(),
        done: false,
    });
    save(&tasks);
    tasks
}

pub fn mark_done(n: usize) -> Result<Vec<HematiteTask>, String> {
    let mut tasks = load();
    let task = tasks
        .iter_mut()
        .find(|t| t.id == n)
        .ok_or_else(|| format!("No task with id {}.", n))?;
    task.done = true;
    save(&tasks);
    Ok(tasks)
}

pub fn remove(n: usize) -> Result<Vec<HematiteTask>, String> {
    let mut tasks = load();
    let before = tasks.len();
    tasks.retain(|t| t.id != n);
    if tasks.len() == before {
        return Err(format!("No task with id {}.", n));
    }
    save(&tasks);
    Ok(tasks)
}

pub fn clear() {
    save(&[]);
}

/// Compact block injected into the system prompt every turn when tasks exist.
pub fn render_prompt_block(tasks: &[HematiteTask]) -> Option<String> {
    if tasks.is_empty() {
        return None;
    }
    let mut out = String::from("## Active Task List\n");
    for t in tasks {
        let mark = if t.done { "[x]" } else { "[ ]" };
        out.push_str(&format!("{} {}. {}\n", mark, t.id, t.text));
    }
    out.push_str(
        "\nWhen you complete a task, let the user know and suggest running `/task done <N>`.",
    );
    Some(out)
}

/// Human-readable list for `/task list` output.
pub fn render_list(tasks: &[HematiteTask]) -> String {
    if tasks.is_empty() {
        return "No tasks. Use `/task add <text>` to add one.".to_string();
    }
    let mut out = String::from("## Task List\n\n");
    for t in tasks {
        let mark = if t.done { "[x]" } else { "[ ]" };
        out.push_str(&format!("{} **{}**. {}\n", mark, t.id, t.text));
    }
    out
}
