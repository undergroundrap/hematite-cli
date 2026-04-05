use crate::agent::inference::{
    ChatMessage, InferenceEngine, InferenceEvent, MessageContent, ToolCallFn, ToolDefinition,
    ToolFunction,
};
// SystemPromptBuilder is no longer used — InferenceEngine::build_system_prompt() is canonical.
use crate::agent::compaction::{self, CompactionConfig};
use crate::ui::gpu_monitor::GpuState;

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

// ── Session persistence ───────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedSession {
    running_summary: Option<String>,
    #[serde(default)]
    session_memory: crate::agent::compaction::SessionMemory,
}

#[derive(Default)]
struct ActionGroundingState {
    turn_index: u64,
    observed_paths: std::collections::HashMap<String, u64>,
    last_verify_build_turn: Option<u64>,
    last_verify_build_ok: bool,
    code_changed_since_verify: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum WorkflowMode {
    #[default]
    Auto,
    Ask,
    Code,
    Architect,
    ReadOnly,
}

impl WorkflowMode {
    fn label(self) -> &'static str {
        match self {
            WorkflowMode::Auto => "AUTO",
            WorkflowMode::Ask => "ASK",
            WorkflowMode::Code => "CODE",
            WorkflowMode::Architect => "ARCHITECT",
            WorkflowMode::ReadOnly => "READ-ONLY",
        }
    }

    fn is_read_only(self) -> bool {
        matches!(self, WorkflowMode::Ask | WorkflowMode::Architect | WorkflowMode::ReadOnly)
    }
}

fn session_path() -> std::path::PathBuf {
    crate::tools::file_ops::workspace_root()
        .join(".hematite")
        .join("session.json")
}

fn load_session_data() -> (
    Option<String>,
    crate::agent::compaction::SessionMemory,
) {
    let path = session_path();
    if !path.exists() {
        return (None, crate::agent::compaction::SessionMemory::default());
    }
    let Ok(data) = std::fs::read_to_string(&path) else {
        return (None, crate::agent::compaction::SessionMemory::default());
    };
    let Ok(saved) = serde_json::from_str::<SavedSession>(&data) else {
        return (None, crate::agent::compaction::SessionMemory::default());
    };
    (saved.running_summary, saved.session_memory)
}

fn purge_task_files() {
    let root = crate::tools::file_ops::workspace_root();
    // Wipe Task/Plan/Walkthrough (layer 1-2)
    let _ = std::fs::remove_file(root.join(".hematite").join("TASK.md"));
    let _ = std::fs::remove_file(root.join(".hematite").join("PLAN.md"));
    let _ = std::fs::remove_file(root.join(".hematite").join("WALKTHROUGH.md"));
    let _ = std::fs::remove_file(root.join(".github").join("WALKTHROUGH.md"));
    let _ = std::fs::write(root.join(".hematite").join("TASK.md"), "");
    let _ = std::fs::write(root.join(".hematite").join("PLAN.md"), "");

    // Wipe DeepReflect summaries (layer 3)
    let mem_dir = root.join(".hematite").join("memories");
    if mem_dir.exists() {
        let _ = std::fs::remove_dir_all(&mem_dir);
        let _ = std::fs::create_dir_all(&mem_dir);
    }

    // Truncate Logs (layer 4)
    let log_dir = root.join(".hematite_logs");
    if log_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&log_dir) {
            for entry in entries.flatten() {
                let _ = std::fs::write(entry.path(), "");
            }
        }
    }
}

fn should_enable_grounded_trace_mode(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    let asks_trace = lower.contains("trace")
        || lower.contains("how does")
        || lower.contains("what are the main runtime subsystems")
        || lower.contains("how does a user message move")
        || lower.contains("separate normal assistant output")
        || lower.contains("session reset behavior")
        || lower.contains("file references")
        || lower.contains("event types")
        || lower.contains("channels");
    let read_only = lower.contains("read-only");
    let anti_guess = lower.contains("do not guess") || lower.contains("if you are unsure");
    asks_trace || read_only || anti_guess
}

fn should_enable_capability_mode(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("what can you do")
        || lower.contains("what are you capable")
        || lower.contains("can you make projects")
        || lower.contains("can you build projects")
        || lower.contains("do you know other coding languages")
        || lower.contains("other coding languages")
        || lower.contains("what languages")
        || lower.contains("can you use the internet")
        || lower.contains("internet research capabilities")
        || lower.contains("what tools do you have")
}

fn capability_question_requires_repo_inspection(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("this repo")
        || lower.contains("this repository")
        || lower.contains("codebase")
        || lower.contains("which files")
        || lower.contains("implementation")
        || lower.contains("in this project")
}

fn is_capability_probe_tool(name: &str) -> bool {
    matches!(
        name,
        "map_project"
            | "read_file"
            | "inspect_lines"
            | "list_files"
            | "grep_files"
            | "lsp_definitions"
            | "lsp_references"
            | "lsp_hover"
            | "lsp_search_symbol"
            | "lsp_get_diagnostics"
            | "trace_runtime_flow"
            | "auto_pin_context"
            | "list_pinned"
    )
}

fn should_answer_language_capability_directly(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    let asks_languages = lower.contains("other coding languages")
        || lower.contains("what languages")
        || lower.contains("know other languages");
    let asks_projects = lower.contains("capable of making projects")
        || lower.contains("can you make projects")
        || lower.contains("can you build projects");
    asks_languages && asks_projects
}

fn build_language_capability_answer() -> String {
    "Hematite itself is written in Rust, but it is not limited to that language. I can help with projects in Python, JavaScript, TypeScript, Go, C#, and other languages.\n\nI can help create projects by scaffolding files and directories, implementing features, editing code precisely, running the appropriate local build or test commands for the target stack, and iterating on the project structure as it grows. The main limits are the local model, the available tooling on this machine, and how much context fits cleanly in session.".to_string()
}

fn should_answer_session_memory_directly(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    (lower.contains("carry forward by default") || lower.contains("session memory should you carry forward"))
        && (lower.contains("restarted hematite")
            || lower.contains("restarted")
            || lower.contains("avoid carrying forward"))
}

fn build_session_memory_answer() -> String {
    "By default, Hematite should carry forward lightweight project and task signal, not full conversational residue.\n\nCarry forward: Vein-backed project memory, compact session summary, current task memory, working-set files, and explicit pinned context when it is still relevant.\n\nAvoid carrying forward: full chat history, stale reasoning chains, one-off conversational residue, and transient in-flight state from the previous turn.\n\nFor a local model, the right split is to save the project and the active task signal, not replay old dialogue unless you explicitly want to continue the same thread.".to_string()
}

fn should_answer_reasoning_split_directly(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    (lower.contains("reasoning output") || lower.contains("reasoning"))
        && (lower.contains("visible chat output")
            || lower.contains("visible chat")
            || lower.contains("chat output"))
}

fn build_reasoning_split_answer() -> String {
    "Hematite separates reasoning output from visible chat output so the operator sees a clean final answer while the system can still expose its internal reasoning state separately.\n\nVisible chat output is the user-facing reply that belongs in the main transcript. Reasoning output is routed to the SPECULAR side panel and related internal state so Hematite can show its thought process without polluting the main conversation.\n\nThat separation matters for three reasons: cleaner chat logs, easier debugging of agent behavior, and better control over modes like `/ask`, `/architect`, and read-only analysis where internal thinking should not be confused with the final reply.".to_string()
}

fn should_answer_workflow_modes_directly(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("/ask")
        && lower.contains("/code")
        && lower.contains("/architect")
        && lower.contains("/read-only")
        && lower.contains("/auto")
        && (lower.contains("difference") || lower.contains("differences") || lower.contains("what are"))
}

fn build_workflow_modes_answer() -> String {
    "/ask is sticky read-only analysis mode: inspect, explain, and answer without making changes.\n\n/code is sticky implementation mode: Hematite can edit, verify, and carry out coding work with the normal proof-before-action safeguards.\n\n/architect is sticky plan-first mode: inspect the repo, shape the solution, and produce the implementation approach before editing. It should not mutate code unless you explicitly ask to implement.\n\n/read-only is the hard no-mutation workflow: analysis only, no file edits, no mutating shell commands, and no commits.\n\n/auto returns Hematite to the default behavior where it chooses the narrowest effective path for the request.".to_string()
}

fn should_answer_verify_profiles_directly(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("verify_build")
        && lower.contains(".hematite/settings.json")
        && (lower.contains("build")
            || lower.contains("test")
            || lower.contains("lint")
            || lower.contains("fix")
            || lower.contains("verification commands"))
}

fn build_verify_profiles_answer() -> String {
    "When a project defines verify profiles in `.hematite/settings.json`, `verify_build` should treat those profile commands as the first source of truth.\n\nEach action stays separate: `build` runs the profile's build command, `test` runs the test command, `lint` runs the lint command, and `fix` runs the fix command. `verify_build` should not run all of them at once unless you call those actions separately.\n\nIf you pass an explicit profile, Hematite should use that profile or fail clearly if it does not exist. If the project defines a default profile, Hematite should use it when no explicit profile is given. Only when no profile is configured should Hematite fall back to stack-aware auto-detection.".to_string()
}

fn looks_like_mutation_request(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    [
        "fix ",
        "change ",
        "edit ",
        "modify ",
        "update ",
        "rename ",
        "refactor ",
        "patch ",
        "rewrite ",
        "implement ",
        "create a file",
        "create file",
        "add a file",
        "delete ",
        "remove ",
        "make the change",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn build_mode_redirect_answer(mode: WorkflowMode) -> String {
    match mode {
        WorkflowMode::Ask => "Workflow mode ASK is read-only. I can inspect the code, explain what should change, or review the target area, but I will not modify files here. Switch to `/code` to implement the change, or `/auto` to let Hematite choose.".to_string(),
        WorkflowMode::Architect => "Workflow mode ARCHITECT is plan-first. I can inspect the code and design the implementation approach, but I will not mutate files until you explicitly switch to `/code` or ask me to implement.".to_string(),
        WorkflowMode::ReadOnly => "Workflow mode READ-ONLY is a hard no-mutation mode. I can analyze, inspect, and explain, but I will not edit files, run mutating shell commands, or commit changes. Switch to `/code` or `/auto` if you want implementation.".to_string(),
        _ => "Switch to `/code` or `/auto` to allow implementation.".to_string(),
    }
}

fn parse_inline_workflow_prompt(user_input: &str) -> Option<(WorkflowMode, &str)> {
    let trimmed = user_input.trim();
    for (prefix, mode) in [
        ("/ask", WorkflowMode::Ask),
        ("/code", WorkflowMode::Code),
        ("/architect", WorkflowMode::Architect),
        ("/read-only", WorkflowMode::ReadOnly),
        ("/auto", WorkflowMode::Auto),
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let rest = rest.trim();
            if !rest.is_empty() {
                return Some((mode, rest));
            }
        }
    }
    None
}

fn should_enable_toolchain_mode(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("tooling discipline")
        || lower.contains("best read-only toolchain")
        || lower.contains("identify the best tools you actually have")
        || lower.contains("concrete read-only investigation plan")
        || lower.contains("do not execute the plan")
        || (lower.contains("which tools") && lower.contains("why"))
        || (lower.contains("available repo-inspection tools"))
        || (lower.contains("tool choice discipline"))
        || (lower.contains("what tools would you choose first"))
        || (lower.contains("when would you choose") && lower.contains("tool"))
}

fn should_answer_toolchain_directly(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    should_enable_toolchain_mode(user_input)
        && lower.contains("read-only")
        && (lower.contains("tooling discipline")
            || lower.contains("investigation plan")
            || lower.contains("best read-only toolchain")
            || lower.contains("tool choice discipline")
            || lower.contains("what tools would you choose first"))
}

// ── Tool catalogue ────────────────────────────────────────────────────────────

/// Returns the full set of tools exposed to the model.
pub fn get_tools() -> Vec<ToolDefinition> {
    let os = std::env::consts::OS;
    let mut tools = vec![
        make_tool(
            "shell",
            &format!("Execute a command in the host shell ({os}). \
                     Use this for building, testing, or system operations. \
                     Output is capped at 64KB. Prefer non-interactive commands."),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command to run"
                    },
                    "reason": {
                        "type": "string",
                        "description": "For risky shell calls, explain what this command is verifying or changing."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Optional timeout in seconds (default 60)"
                    }
                },
                "required": ["command"]
            }),
        ),
        make_tool(
            "map_project",
            "Compact architecture-aware map of the project structure, key configuration files, \
             likely entrypoints, and core owner files. Use this at the start of a task to gain \
             spatial awareness before deeper file reads or LSP inspection.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "focus": {
                        "type": "string",
                        "description": "Optional relative subpath to focus the map on instead of the whole workspace."
                    },
                    "include_symbols": {
                        "type": "boolean",
                        "description": "Whether to extract a small set of top symbols from core files. Defaults to true."
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Optional tree depth cap for the directory section. Defaults to 4 and is capped internally."
                    }
                }
            }),
        ),
        make_tool(
            "trace_runtime_flow",
            "Return an authoritative read-only trace of Hematite runtime flow. \
             Use this for architecture questions about keyboard input to final output, \
             reasoning/specular separation, startup wiring, runtime subsystems, or \
             session reset commands like /clear, /new, and /forget. Prefer this over guessing.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "enum": ["user_turn", "session_reset", "reasoning_split", "runtime_subsystems", "startup"],
                        "description": "Which verified runtime report to return"
                    },
                    "input": {
                        "type": "string",
                        "description": "Optional user input to label a normal user-turn trace"
                    },
                    "command": {
                        "type": "string",
                        "enum": ["/clear", "/new", "/forget", "all"],
                        "description": "Optional reset command when topic=session_reset"
                    }
                },
                "required": ["topic"]
            }),
        ),
        make_tool(
            "describe_toolchain",
            "Return an authoritative read-only description of Hematite's actual tool surface and investigation strategy. \
             Use this for tooling-discipline questions, best-tool selection, or read-only plans for tracing runtime behavior. \
             Prefer this over improvising tool names or investigation steps from memory.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "enum": ["read_only_codebase", "user_turn_plan", "voice_latency_plan", "all"],
                        "description": "Which authoritative toolchain report to return"
                    },
                    "question": {
                        "type": "string",
                        "description": "Optional user question to label or tailor the read-only investigation plan"
                    }
                }
            }),
        ),
        make_tool(
            "read_file",
            "Read the contents of a file. For large files, use 'offset' and 'limit' to navigate.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file, relative to the project root"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Starting line number (0-indexed)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of lines to read"
                    }
                },
                "required": ["path"]
            }),
        ),
        make_tool(
            "lsp_definitions",
            "Get the precise definition location (file:line:char) for a symbol at a specific position. \
             Use this to jump to function/struct source code accurately.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "0-indexed line" },
                    "character": { "type": "integer", "description": "0-indexed character" }
                },
                "required": ["path", "line", "character"]
            }),
        ),
        make_tool(
            "lsp_references",
            "Find all locations where a symbol is used across the entire workspace. \
             Use this to understand the impact of a refactor or discover internal API users.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "0-indexed line" },
                    "character": { "type": "integer", "description": "0-indexed character" }
                },
                "required": ["path", "line", "character"]
            }),
        ),
        make_tool(
            "lsp_hover",
            "Get hover information (documentation, function signature, type details) for a symbol. \
             Use this for rapid spatial awareness without opening every file.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "0-indexed line" },
                    "character": { "type": "integer", "description": "0-indexed character" }
                },
                "required": ["path", "line", "character"]
            }),
        ),
        make_tool(
            "lsp_rename_symbol",
            "Rename a symbol project-wide using the Language Server. Ensures all references are updated safely.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "line": { "type": "integer", "description": "0-indexed line" },
                    "character": { "type": "integer", "description": "0-indexed character" },
                    "new_name": { "type": "string", "description": "The new name for the symbol" }
                },
                "required": ["path", "line", "character", "new_name"]
            }),
        ),
        make_tool(
            "lsp_get_diagnostics",
            "Get a list of current compiler errors and warnings for a specific file. \
             Use this to verify your code compiles and and to find exactly where errors are located.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" }
                },
                "required": ["path"]
            }),
        ),
        make_tool(
            "vision_analyze",
            "Send an image file (screenshot, diagram, or UI mockup) to the multimodal vision model for technical analysis. \
             Use this to identify UI bugs, confirm visual states, or understand architectural diagrams.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute or relative path to the image file." },
                    "prompt": { "type": "string", "description": "The specific question or analysis request for the vision model." }
                },
                "required": ["path", "prompt"]
            }),
        ),
        make_tool(
            "patch_hunk",
            "Replace a specific line range [start_line, end_line] with new content. \
             This is the most precise way to edit code and avoids search string failures.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "start_line": { "type": "integer", "description": "Starting line (1-indexed)" },
                    "end_line": { "type": "integer", "description": "Ending line (inclusive)" },
                    "replacement": { "type": "string", "description": "The new content for this range" }
                },
                "required": ["path", "start_line", "end_line", "replacement"]
            }),
        ),
        make_tool(
            "multi_search_replace",
            "Replace multiple existing code blocks in a single file with new content. \
             Each hunk specifies an EXACT 'search' string and a 'replace' string. \
             The 'search' string MUST exactly match the existing file contents (including whitespace). \
             This is the safest and most reliable way to make multiple structural edits.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "hunks": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "search": { "type": "string", "description": "Exact existing text to find and replace" },
                                "replace": { "type": "string", "description": "The new replacement text" }
                            },
                            "required": ["search", "replace"]
                        }
                    }
                },
                "required": ["path", "hunks"]
            }),
        ),
        make_tool(
            "write_file",
            "Write content to a file, creating it (and any parent dirs) if needed. \
             Overwrites existing files.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "content": { "type": "string", "description": "Full file content to write" }
                },
                "required": ["path", "content"]
            }),
        ),
        make_tool(
            "research_web",
            "Perform a zero-cost technical search using DuckDuckGo. \
             Use this to find documentation, latest API changes, or solutions to complex errors \
             when your internal knowledge is insufficient. Returns snippets and URLs.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The technical search query" }
                },
                "required": ["query"]
            }),
        ),
        make_tool(
            "fetch_docs",
            "Fetch a URL and convert it to clean Markdown. Use this to 'read' the documentation \
             links found via research_web. This tool uses a proxy to bypass IP blocks.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The URL of the documentation to fetch" }
                },
                "required": ["url"]
            }),
        ),
        make_tool(
            "edit_file",
            "Edit a file by replacing an exact string with another. \
             The 'search' string does NOT need perfectly matching indentation (it is fuzzy), \
             but the non-whitespace text must match exactly. Use this for targeted edits.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "search": {
                        "type": "string",
                        "description": "The exact text to find (must match whitespace/indentation precisely)"
                    },
                    "replace": {
                        "type": "string",
                        "description": "The replacement text"
                    }
                },
                "required": ["path", "search", "replace"]
            }),
        ),
        make_tool(
            "auto_pin_context",
            "Select 1-3 core files to 'Lock' into high-fidelity memory. \
             Use this after map_project to ensure the most important architecture files \
             are always visible during complex refactorings.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "reason": { "type": "string" }
                },
                "required": ["paths", "reason"]
            }),
        ),
        make_tool(
            "list_pinned",
            "List all files currently pinned in the model's active context.",
            serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        ),
        make_tool(
            "list_files",
            "List files in a directory, optionally filtered by extension.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory to list (default: current dir)"
                    },
                    "extension": {
                        "type": "string",
                        "description": "Only return files with this extension, e.g. 'rs', 'toml' (no dot)"
                    }
                },
                "required": []
            }),
        ),
        make_tool(
            "grep_files",
            "Search file contents for a regex pattern. Supports context lines, files-only mode, \
             and pagination. Returns file:line:content format by default.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for (case-insensitive by default)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search (default: current dir)"
                    },
                    "extension": {
                        "type": "string",
                        "description": "Only search files with this extension, e.g. 'rs'"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["content", "files_only"],
                        "description": "'content' (default) returns matching lines; 'files_only' returns only filenames"
                    },
                    "context": {
                        "type": "integer",
                        "description": "Lines of context before AND after each match (like rg -C)"
                    },
                    "before": {
                        "type": "integer",
                        "description": "Lines of context before each match (overrides context)"
                    },
                    "after": {
                        "type": "integer",
                        "description": "Lines of context after each match (overrides context)"
                    },
                    "head_limit": {
                        "type": "integer",
                        "description": "Max hunks (or files in files_only) to return (default: 50)"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Skip first N hunks/files — for pagination (default: 0)"
                    }
                },
                "required": ["pattern"]
            }),
        ),
        make_tool(
            "git_commit",
            "Stage all changes (git add -A) and create a commit. You MUST use 'Conventional Commits' (e.g. 'feat: description').",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string", "description": "Commit message (Conventional Commit style)" }
                },
                "required": ["message"]
            }),
        ),
        make_tool(
            "git_push",
            "Push current branched changes to the remote origin. Requires an existing remote connection.",
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        make_tool(
            "git_remote",
            "View or manage git remotes. Use this for onboarding to GitHub/GitLab services.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "add", "remove"],
                        "description": "Operation to perform"
                    },
                    "name": { "type": "string", "description": "Remote name (e.g. origin)" },
                    "url": { "type": "string", "description": "Remote URL (for 'add' action)" }
                },
                "required": ["action"]
            }),
        ),
        make_tool(
            "git_onboarding",
            "High-level wizard to connect this repository to a remote host (GitHub/GitLab). \
             Handles adding the remote and performing the initial tracking push in one step.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The remote repository URL (HTTPS or SSH)" },
                    "name": { "type": "string", "description": "The remote name (default: origin)" },
                    "push": { "type": "boolean", "description": "Whether to perform an initial push to establish tracking (default: false)" }
                },
                "required": ["url"]
            }),
        ),
        make_tool(
            "verify_build",
            "Run project verification for build, test, lint, or fix workflows. \
             Prefer per-project verify profiles from `.hematite/settings.json`, and fall back to \
             auto-detected defaults when no profile is configured. Returns BUILD OK or BUILD FAILED \
             with command output. ALWAYS call this after scaffolding a new project or making structural changes.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["build", "test", "lint", "fix"],
                        "description": "Which verification action to run. Defaults to build."
                    },
                    "profile": {
                        "type": "string",
                        "description": "Optional named verify profile from `.hematite/settings.json`."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Optional timeout override for this verification run."
                    }
                }
            }),
        ),
        make_tool(
            "git_worktree",
            "Manage Git worktrees — isolated working directories on separate branches. \
             Use 'add' to create a safe sandbox for risky/experimental work, \
             'list' to see all worktrees, 'remove' to clean up, 'prune' to remove stale entries.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "add", "remove", "prune"],
                        "description": "Worktree operation to perform"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory path for the new worktree (required for add/remove)"
                    },
                    "branch": {
                        "type": "string",
                        "description": "Branch name for the worktree (add only; defaults to path basename)"
                    }
                },
                "required": ["action"]
            }),
        ),
        make_tool(
            "clarify",
            "Ask the user a clarifying question when you genuinely cannot proceed without \
             more information. Use this ONLY when you are blocked and cannot make a \
             reasonable assumption. Do NOT use it to ask permission — just act.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The specific question to ask the user"
                    }
                },
                "required": ["question"]
            }),
        ),
        make_tool(
            "manage_tasks",
            "Manage the persistent task ledger in .hematite/TASK.md. Use this to track long-term goals across restarts.",
            crate::tools::tasks::get_tasks_params(),
        ),
        make_tool(
            "maintain_plan",
            "Document the architectural strategy and session blueprint in .hematite/PLAN.md. Use this to maintain context across restarts.",
            crate::tools::plan::get_plan_params(),
        ),
        make_tool(
            "generate_walkthrough",
            "Generate a final session report in .hematite/WALKTHROUGH.md including achievements and verification results.",
            crate::tools::plan::get_walkthrough_params(),
        ),
        make_tool(
            "swarm",
            "Delegate high-volume parallel tasks to a swarm of background workers. \
             Use this for large-scale refactors, multi-file research, or parallel documentation updates. \
             You must provide a 'tasks' array where each task has an 'id', 'target' (file), and 'instruction'.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "tasks": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "target": { "type": "string", "description": "Target file or directory" },
                                "instruction": { "type": "string", "description": "Specific task for this worker" }
                            },
                            "required": ["id", "target", "instruction"]
                        }
                    },
                    "max_workers": {
                        "type": "integer",
                        "description": "Max parallel workers (default 3, auto-throttled by hardware)",
                        "default": 3
                    }
                },
                "required": ["tasks"]
            }),
        ),
    ];

    // ── Semantic Ignition: Specialized LSP Tools ───────────────
    let lsp_defs = crate::tools::lsp_tools::get_lsp_definitions();
    tools.push(make_tool(
        "lsp_search_symbol",
        "Find the location (file/line) of any function, struct, or variable in the entire project workspace. \
         This is the fastest 'Golden Path' for navigating to a symbol by name.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "The name of the symbol to find (e.g. 'initialize_mcp')" }
            },
            "required": ["query"]
        }),
    ));
    for def in lsp_defs {
        tools.push(ToolDefinition {
            tool_type: "function".into(),
            function: ToolFunction {
                name: def["name"].as_str().unwrap().into(),
                description: def["description"].as_str().unwrap().into(),
                parameters: def["parameters"].clone(),
            },
        });
    }

    tools
}

fn make_tool(name: &str, description: &str, parameters: Value) -> ToolDefinition {
    ToolDefinition {
        tool_type: "function".into(),
        function: ToolFunction {
            name: name.into(),
            description: description.into(),
            parameters,
        },
    }
}

// ── ConversationManager ───────────────────────────────────────────────────────

pub struct ConversationManager {
    /// Full conversation history in OpenAI format.
    pub history: Vec<ChatMessage>,
    pub engine: Arc<InferenceEngine>,
    pub tools: Vec<ToolDefinition>,
    pub mcp_manager: Arc<Mutex<crate::agent::mcp_manager::McpManager>>,
    pub professional: bool,
    pub brief: bool,
    pub snark: u8,
    pub chaos: u8,
    /// Model to use for simple read-only tasks (optional, user-supplied via --fast-model).
    pub fast_model: Option<String>,
    /// Model to use for complex write/build tasks (optional, user-supplied via --think-model).
    pub think_model: Option<String>,
    /// Files where whitespace auto-correction fired this session.
    pub correction_hints: Vec<String>,
    /// Running background summary of pruned older messages.
    pub running_summary: Option<String>,
    /// Live hardware telemetry handle.
    pub gpu_state: Arc<GpuState>,
    /// Local RAG memory — FTS5-indexed project source.
    pub vein: crate::memory::vein::Vein,
    /// Append-only session transcript logger.
    pub transcript: crate::agent::transcript::TranscriptLogger,
    /// Thread-safe cancellation signal for the current agent turn.
    pub cancel_token: Arc<std::sync::atomic::AtomicBool>,
    /// Shared Git remote state (for persistent connectivity checks).
    pub git_state: Arc<crate::agent::git_monitor::GitState>,
    /// Reasoning think-mode override. None = let model decide. Some(true) = force /think.
    /// Some(false) = force /no_think (fast mode, 3-5x quicker for simple tasks).
    pub think_mode: Option<bool>,
    workflow_mode: WorkflowMode,
    /// Layer 6: Dynamic Task Context (extracted during compaction)
    pub session_memory: crate::agent::compaction::SessionMemory,
    pub swarm_coordinator: Arc<crate::agent::swarm::SwarmCoordinator>,
    pub voice_manager: Arc<crate::ui::voice::VoiceManager>,
    pub lsp_manager: Arc<Mutex<crate::agent::lsp::manager::LspManager>>,
    /// Active reasoning summary extracted from the previous model turn (Gemma-4 Native).
    pub reasoning_history: Option<String>,
    /// Layer 8: Active Reference Pinning (Context Locked)
    pub pinned_files: Arc<Mutex<std::collections::HashMap<String, String>>>,
    /// Hard action-grounding state for proof-before-action checks.
    action_grounding: Arc<Mutex<ActionGroundingState>>,
}

impl ConversationManager {
    pub fn new(
        engine: Arc<InferenceEngine>,
        professional: bool,
        brief: bool,
        snark: u8,
        chaos: u8,
        fast_model: Option<String>,
        think_model: Option<String>,
        gpu_state: Arc<GpuState>,
        git_state: Arc<crate::agent::git_monitor::GitState>,
        swarm_coordinator: Arc<crate::agent::swarm::SwarmCoordinator>,
        voice_manager: Arc<crate::ui::voice::VoiceManager>,
    ) -> Self {
        let (saved_summary, saved_memory) = load_session_data();

        // Build the initial mcp_manager
        let mcp_manager = Arc::new(tokio::sync::Mutex::new(
            crate::agent::mcp_manager::McpManager::new(),
        ));

        // Build the initial system prompt using the canonical InferenceEngine path.
        let dynamic_instructions = engine.build_system_prompt(
            snark,
            chaos,
            brief,
            professional,
            &[],
            None,
            &[],
        );

        let history = vec![ChatMessage::system(&dynamic_instructions)];

        let vein_path = crate::tools::file_ops::workspace_root()
            .join(".hematite")
            .join("vein.db");
        let vein = crate::memory::vein::Vein::new(&vein_path)
            .unwrap_or_else(|_| crate::memory::vein::Vein::new(":memory:").unwrap());

        Self {
            history,
            engine,
            tools: get_tools(),
            mcp_manager,
            professional,
            brief,
            snark,
            chaos,
            fast_model,
            think_model,
            correction_hints: Vec::new(),
            running_summary: saved_summary,
            gpu_state,
            vein,
            transcript: crate::agent::transcript::TranscriptLogger::new(),
            cancel_token: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            git_state,
            think_mode: None,
            workflow_mode: WorkflowMode::Auto,
            session_memory: saved_memory,
            swarm_coordinator,
            voice_manager,
            lsp_manager: Arc::new(Mutex::new(crate::agent::lsp::manager::LspManager::new(
                crate::tools::file_ops::workspace_root(),
            ))),
            reasoning_history: None,
            pinned_files: Arc::new(Mutex::new(std::collections::HashMap::new())),
            action_grounding: Arc::new(Mutex::new(ActionGroundingState::default())),
        }
    }

    /// Index the project into The Vein. Call once after construction.
    /// Uses block_in_place so the tokio runtime thread isn't parked.
    pub fn initialize_vein(&mut self) -> usize {
        tokio::task::block_in_place(|| self.vein.index_project())
    }

    fn save_session(&self) {
        let path = session_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let saved = SavedSession {
            running_summary: self.running_summary.clone(),
            session_memory: self.session_memory.clone(),
        };
        if let Ok(json) = serde_json::to_string(&saved) {
            let _ = std::fs::write(&path, json);
        }
    }

    fn save_empty_session(&self) {
        let path = session_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let saved = SavedSession {
            running_summary: None,
            session_memory: crate::agent::compaction::SessionMemory::default(),
        };
        if let Ok(json) = serde_json::to_string(&saved) {
            let _ = std::fs::write(&path, json);
        }
    }

    fn refresh_session_memory(&mut self) {
        self.session_memory = compaction::extract_memory(&self.history);
    }

    fn append_session_handoff(&self, system_msg: &mut String) {
        let has_summary = self
            .running_summary
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_memory = self.session_memory.has_signal();

        if !has_summary && !has_memory {
            return;
        }

        system_msg.push_str(
            "\n\n# LIGHTWEIGHT SESSION HANDOFF\n\
             This is compact carry-over from earlier work on this machine.\n\
             Use it only when it helps the current request.\n\
             Prefer current repository state, pinned files, and fresh tool results over stale session memory.\n",
        );

        if has_memory {
            system_msg.push_str("\n## Active Task Memory\n");
            system_msg.push_str(&self.session_memory.to_prompt());
        }

        if let Some(summary) = self.running_summary.as_deref() {
            if !summary.trim().is_empty() {
                system_msg.push_str("\n## Compacted Session Summary\n");
                system_msg.push_str(summary);
                system_msg.push('\n');
            }
        }
    }

    fn set_workflow_mode(&mut self, mode: WorkflowMode) {
        self.workflow_mode = mode;
    }

    async fn begin_grounded_turn(&self) -> u64 {
        let mut state = self.action_grounding.lock().await;
        state.turn_index += 1;
        state.turn_index
    }

    async fn reset_action_grounding(&self) {
        let mut state = self.action_grounding.lock().await;
        *state = ActionGroundingState::default();
    }

    async fn record_read_observation(&self, path: &str) {
        let normalized = normalize_workspace_path(path);
        let mut state = self.action_grounding.lock().await;
        let turn = state.turn_index;
        state.observed_paths.insert(normalized, turn);
    }

    async fn record_verify_build_result(&self, ok: bool) {
        let mut state = self.action_grounding.lock().await;
        let turn = state.turn_index;
        state.last_verify_build_turn = Some(turn);
        state.last_verify_build_ok = ok;
        if ok {
            state.code_changed_since_verify = false;
        }
    }

    async fn record_successful_mutation(&self, path: Option<&str>) {
        let mut state = self.action_grounding.lock().await;
        state.code_changed_since_verify = match path {
            Some(p) => is_code_like_path(p),
            None => true,
        };
    }

    async fn validate_action_preconditions(
        &self,
        name: &str,
        args: &Value,
    ) -> Result<(), String> {
        if self.workflow_mode.is_read_only() && is_destructive_tool(name) {
            if name == "shell" {
                let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                let risk = crate::tools::guard::classify_bash_risk(command);
                if !matches!(risk, crate::tools::RiskLevel::Safe) {
                    return Err(format!(
                        "Action blocked: workflow mode `{}` is read-only for risky or mutating operations. Switch to `/code` or `/auto` before making changes.",
                        self.workflow_mode.label()
                    ));
                }
            } else {
                return Err(format!(
                    "Action blocked: workflow mode `{}` is read-only. Use `/code` to implement changes or `/auto` to leave mode selection to Hematite.",
                    self.workflow_mode.label()
                ));
            }
        }

        let normalized_target = action_target_path(name, args);
        if let Some(target) = normalized_target.as_deref() {
            let path_exists = std::path::Path::new(target).exists();
            if path_exists {
                let state = self.action_grounding.lock().await;
                let pinned = self.pinned_files.lock().await;
                let recent_observed = state
                    .observed_paths
                    .get(target)
                    .map(|turn| state.turn_index.saturating_sub(*turn) <= 3)
                    .unwrap_or(false);
                let pinned_match = pinned
                    .keys()
                    .any(|p| normalize_workspace_path(p) == target);
                drop(pinned);
                if !recent_observed && !pinned_match {
                    return Err(format!(
                        "Action blocked: `{}` on '{}' requires recent file evidence. Use `read_file` or `inspect_lines` on that path first, or pin the file into active context.",
                        name, target
                    ));
                }
            }
        }

        if is_mcp_mutating_tool(name) {
            return Err(format!(
                "Action blocked: `{}` is an external MCP mutation tool. For workspace file edits, prefer Hematite's built-in edit path (`read_file`/`inspect_lines` plus `patch_hunk`, `edit_file`, or `multi_search_replace`) unless the user explicitly requires MCP for that action.",
                name
            ));
        }

        if is_mcp_workspace_read_tool(name) {
            return Err(format!(
                "Action blocked: `{}` is an external MCP filesystem read tool. For local workspace inspection, prefer Hematite's built-in read path (`read_file`, `inspect_lines`, `list_files`, or `grep_files`) unless the user explicitly requires MCP for that action.",
                name
            ));
        }

        if name == "git_commit" || name == "git_push" {
            let state = self.action_grounding.lock().await;
            if state.code_changed_since_verify && !state.last_verify_build_ok {
                return Err(format!(
                    "Action blocked: `{}` requires a successful `verify_build` after the latest code edits. Run verification first so Hematite has proof that the tree is build-clean.",
                    name
                ));
            }
        }

        if name == "shell" {
            let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("").trim();
            let risk = crate::tools::guard::classify_bash_risk(command);
            if !matches!(risk, crate::tools::RiskLevel::Safe) && reason.is_empty() {
                return Err(
                    "Action blocked: risky `shell` calls require a concrete `reason` argument that explains what is being verified or changed."
                        .to_string(),
                );
            }
        }

        Ok(())
    }

    fn build_action_receipt(
        &self,
        name: &str,
        args: &Value,
        output: &str,
        is_error: bool,
    ) -> Option<ChatMessage> {
        if is_error || !is_destructive_tool(name) {
            return None;
        }

        let mut receipt = String::from("[ACTION RECEIPT]\n");
        receipt.push_str(&format!("- tool: {}\n", name));
        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
            receipt.push_str(&format!("- target: {}\n", path));
        }
        if name == "shell" {
            if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                receipt.push_str(&format!("- command: {}\n", command));
            }
            if let Some(reason) = args.get("reason").and_then(|v| v.as_str()) {
                if !reason.trim().is_empty() {
                    receipt.push_str(&format!("- reason: {}\n", reason.trim()));
                }
            }
        }
        let first_line = output.lines().next().unwrap_or(output).trim();
        receipt.push_str(&format!("- outcome: {}\n", first_line));
        Some(ChatMessage::system(&receipt))
    }

    fn replace_mcp_tool_definitions(&mut self, mcp_tools: &[crate::agent::mcp::McpTool]) {
        self.tools
            .retain(|tool| !tool.function.name.starts_with("mcp__"));
        self.tools
            .extend(mcp_tools.iter().map(|tool| ToolDefinition {
                tool_type: "function".into(),
                function: ToolFunction {
                    name: tool.name.clone(),
                    description: tool.description.clone().unwrap_or_default(),
                    parameters: tool.input_schema.clone(),
                },
            }));
    }

    async fn refresh_mcp_tools(
        &mut self,
    ) -> Result<Vec<crate::agent::mcp::McpTool>, Box<dyn std::error::Error + Send + Sync>> {
        let mcp_tools = {
            let mut mcp = self.mcp_manager.lock().await;
            match mcp.initialize_all().await {
                Ok(()) => mcp.discover_tools().await,
                Err(e) => {
                    drop(mcp);
                    self.replace_mcp_tool_definitions(&[]);
                    return Err(e.into());
                }
            }
        };

        self.replace_mcp_tool_definitions(&mcp_tools);
        Ok(mcp_tools)
    }

    /// Spawns and initializes all configured MCP servers, discovering their tools.
    pub async fn initialize_mcp(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = self.refresh_mcp_tools().await?;
        Ok(())
    }

    /// Run one user turn through the full agentic loop.
    ///
    /// Adds the user message, calls the model, executes any tools, and loops
    /// until the model produces a final text reply.  All progress is streamed
    /// as `InferenceEvent` values via `tx`.
    pub async fn run_turn(
        &mut self,
        user_input: &str,
        tx: mpsc::Sender<InferenceEvent>,
        yolo: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Reload config every turn (edits apply immediately, no restart needed).
        let config = crate::agent::config::load_config();
        let _turn_id = self.begin_grounded_turn().await;
        let _hook_runner = crate::agent::hooks::HookRunner::new(config.hooks.clone());
        let mcp_tools = match self.refresh_mcp_tools().await {
            Ok(tools) => tools,
            Err(e) => {
                let _ = tx
                    .send(InferenceEvent::Error(format!("MCP refresh failed: {}", e)))
                    .await;
                Vec::new()
            }
        };

        // Apply config model overrides (config takes precedence over CLI flags).
        let effective_fast = config
            .fast_model
            .clone()
            .or_else(|| self.fast_model.clone());
        let effective_think = config
            .think_model
            .clone()
            .or_else(|| self.think_model.clone());

        // ── /new: reset session ───────────────────────────────────────────────
        if user_input.trim() == "/new" {
            self.history.clear();
            self.reasoning_history = None;
            self.session_memory.clear();
            self.running_summary = None;
            self.correction_hints.clear();
            self.pinned_files.lock().await.clear();
            self.reset_action_grounding().await;
            purge_task_files();
            let _ = std::fs::remove_file(session_path());
            self.save_empty_session();
            for chunk in chunk_text("Session cleared. Fresh context.", 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        // ── /lsp: start language servers manually if needed ──────────────────
        if user_input.trim() == "/lsp" {
            let mut lsp = self.lsp_manager.lock().await;
            match lsp.start_servers().await {
                Ok(_) => {
                    let _ = tx
                        .send(InferenceEvent::MutedToken(
                            "LSP: Servers Initialized OK.".to_string(),
                        ))
                        .await;
                }
                Err(e) => {
                    let _ = tx
                        .send(InferenceEvent::Error(format!(
                            "LSP: Failed to start servers - {}",
                            e
                        )))
                        .await;
                }
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        // ── /forget: clear virtual memory, history, & physical ghosts ────────
        if user_input.trim() == "/forget" {
            self.history.clear();
            self.reasoning_history = None;
            self.session_memory.clear();
            self.running_summary = None; // Reset the context chain
            self.correction_hints.clear();
            self.pinned_files.lock().await.clear();
            self.reset_action_grounding().await;
            purge_task_files();
            let _ = std::fs::remove_file(session_path());
            self.save_empty_session();
            for chunk in chunk_text("Task Memory & History purged. Clean slate achieved.", 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/ask" {
            self.set_workflow_mode(WorkflowMode::Ask);
            for chunk in chunk_text(
                "Workflow mode: ASK. Stay read-only, explain, inspect, and answer without making changes.",
                8,
            ) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/code" {
            self.set_workflow_mode(WorkflowMode::Code);
            for chunk in chunk_text(
                "Workflow mode: CODE. Make changes when needed, but keep proof-before-action and verification discipline.",
                8,
            ) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/architect" {
            self.set_workflow_mode(WorkflowMode::Architect);
            for chunk in chunk_text(
                "Workflow mode: ARCHITECT. Plan, inspect, and shape the approach first. Do not mutate code unless the user explicitly asks to implement.",
                8,
            ) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/read-only" {
            self.set_workflow_mode(WorkflowMode::ReadOnly);
            for chunk in chunk_text(
                "Workflow mode: READ-ONLY. Analysis only. Do not modify files, run mutating shell commands, or commit changes.",
                8,
            ) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/auto" {
            self.set_workflow_mode(WorkflowMode::Auto);
            for chunk in chunk_text(
                "Workflow mode: AUTO. Hematite will choose the narrowest effective path for the request.",
                8,
            ) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        let mut effective_user_input = user_input.trim().to_string();
        if let Some((mode, rest)) = parse_inline_workflow_prompt(user_input) {
            self.set_workflow_mode(mode);
            effective_user_input = rest.to_string();
        }

        // ── /think / /no_think: reasoning budget toggle ──────────────────────
        if should_answer_language_capability_directly(&effective_user_input) {
            let response = build_language_capability_answer();
            self.history.push(ChatMessage::user(&effective_user_input));
            self.history.push(ChatMessage::assistant_text(&response));
            self.transcript.log_user(user_input);
            self.transcript.log_agent(&response);
            for chunk in chunk_text(&response, 8) {
                if !chunk.is_empty() {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
            }
            let _ = tx.send(InferenceEvent::Done).await;
            self.trim_history(80);
            self.refresh_session_memory();
            self.save_session();
            return Ok(());
        }

        if should_answer_session_memory_directly(&effective_user_input) {
            let response = build_session_memory_answer();
            self.history.push(ChatMessage::user(&effective_user_input));
            self.history.push(ChatMessage::assistant_text(&response));
            self.transcript.log_user(user_input);
            self.transcript.log_agent(&response);
            for chunk in chunk_text(&response, 8) {
                if !chunk.is_empty() {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
            }
            let _ = tx.send(InferenceEvent::Done).await;
            self.trim_history(80);
            self.refresh_session_memory();
            self.save_session();
            return Ok(());
        }

        if should_answer_reasoning_split_directly(&effective_user_input) {
            let response = build_reasoning_split_answer();
            self.history.push(ChatMessage::user(&effective_user_input));
            self.history.push(ChatMessage::assistant_text(&response));
            self.transcript.log_user(user_input);
            self.transcript.log_agent(&response);
            for chunk in chunk_text(&response, 8) {
                if !chunk.is_empty() {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
            }
            let _ = tx.send(InferenceEvent::Done).await;
            self.trim_history(80);
            self.refresh_session_memory();
            self.save_session();
            return Ok(());
        }

        if should_answer_workflow_modes_directly(&effective_user_input) {
            let response = build_workflow_modes_answer();
            self.history.push(ChatMessage::user(&effective_user_input));
            self.history.push(ChatMessage::assistant_text(&response));
            self.transcript.log_user(user_input);
            self.transcript.log_agent(&response);
            for chunk in chunk_text(&response, 8) {
                if !chunk.is_empty() {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
            }
            let _ = tx.send(InferenceEvent::Done).await;
            self.trim_history(80);
            self.refresh_session_memory();
            self.save_session();
            return Ok(());
        }

        if should_answer_verify_profiles_directly(&effective_user_input) {
            let response = build_verify_profiles_answer();
            self.history.push(ChatMessage::user(&effective_user_input));
            self.history.push(ChatMessage::assistant_text(&response));
            self.transcript.log_user(user_input);
            self.transcript.log_agent(&response);
            for chunk in chunk_text(&response, 8) {
                if !chunk.is_empty() {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
            }
            let _ = tx.send(InferenceEvent::Done).await;
            self.trim_history(80);
            self.refresh_session_memory();
            self.save_session();
            return Ok(());
        }

        if self.workflow_mode.is_read_only() && looks_like_mutation_request(&effective_user_input) {
            let response = build_mode_redirect_answer(self.workflow_mode);
            self.history.push(ChatMessage::user(&effective_user_input));
            self.history.push(ChatMessage::assistant_text(&response));
            self.transcript.log_user(user_input);
            self.transcript.log_agent(&response);
            for chunk in chunk_text(&response, 8) {
                if !chunk.is_empty() {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
            }
            let _ = tx.send(InferenceEvent::Done).await;
            self.trim_history(80);
            self.refresh_session_memory();
            self.save_session();
            return Ok(());
        }

        if should_answer_toolchain_directly(&effective_user_input) {
            let lower = effective_user_input.to_lowercase();
            let topic = if (lower.contains("voice output") || lower.contains("voice"))
                && (lower.contains("lag")
                    || lower.contains("behind visible text")
                    || lower.contains("latency"))
            {
                "voice_latency_plan"
            } else {
                "all"
            };
            let response = crate::tools::toolchain::describe_toolchain(&serde_json::json!({
                "topic": topic,
                "question": effective_user_input,
            }))
            .await
            .unwrap_or_else(|e| format!("Error: {}", e));
            self.history.push(ChatMessage::user(&effective_user_input));
            self.history.push(ChatMessage::assistant_text(&response));
            self.transcript.log_user(user_input);
            self.transcript.log_agent(&response);
            for chunk in chunk_text(&response, 8) {
                if !chunk.is_empty() {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
            }
            let _ = tx.send(InferenceEvent::Done).await;
            self.trim_history(80);
            self.refresh_session_memory();
            self.save_session();
            return Ok(());
        }

        if user_input.trim() == "/think" {
            self.think_mode = Some(true);
            for chunk in chunk_text("Think mode: ON — full chain-of-thought enabled.", 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }
        if user_input.trim() == "/no_think" {
            self.think_mode = Some(false);
            for chunk in chunk_text(
                "Think mode: OFF — fast mode enabled (no chain-of-thought).",
                8,
            ) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        // ── /pin: add file to active context ────────────────────────────────
        if user_input.trim_start().starts_with("/pin ") {
            let path = user_input.trim_start()[5..].trim();
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    self.pinned_files
                        .lock()
                        .await
                        .insert(path.to_string(), content);
                    let msg = format!(
                        "Pinned: {} — this file is now locked in model context.",
                        path
                    );
                    for chunk in chunk_text(&msg, 8) {
                        let _ = tx.send(InferenceEvent::Token(chunk)).await;
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(InferenceEvent::Error(format!(
                            "Failed to pin {}: {}",
                            path, e
                        )))
                        .await;
                }
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        // ── /unpin: remove file from active context ──────────────────────────
        if user_input.trim_start().starts_with("/unpin ") {
            let path = user_input.trim_start()[7..].trim();
            if self.pinned_files.lock().await.remove(path).is_some() {
                let msg = format!("Unpinned: {} — file removed from active context.", path);
                for chunk in chunk_text(&msg, 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
            } else {
                let _ = tx
                    .send(InferenceEvent::Error(format!(
                        "File {} was not pinned.",
                        path
                    )))
                    .await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        // ── Normal processing ───────────────────────────────────────────────

        // Ensure MCP is initialized and tools are discovered for this turn.
        let mut base_prompt = self.engine.build_system_prompt(
            self.snark,
            self.chaos,
            self.brief,
            self.professional,
            &self.tools,
            self.reasoning_history.as_deref(),
            &mcp_tools,
        );
        if let Some(hint) = &config.context_hint {
            if !hint.trim().is_empty() {
                base_prompt.push_str(&format!(
                    "\n\n# Project Context (from .hematite/settings.json)\n{}",
                    hint
                ));
            }
        }
        let grounded_trace_mode = should_enable_grounded_trace_mode(&effective_user_input);
        let capability_mode = should_enable_capability_mode(&effective_user_input);
        let toolchain_mode = should_enable_toolchain_mode(&effective_user_input);
        let capability_needs_repo = capability_question_requires_repo_inspection(&effective_user_input);
        let mut system_msg = build_system_with_corrections(
            &base_prompt,
            &self.correction_hints,
            &self.gpu_state,
            &self.git_state,
            &config,
        );
        if grounded_trace_mode {
            system_msg.push_str(
                "\n\n# GROUNDED TRACE MODE\n\
                 This turn is read-only architecture analysis unless the user explicitly asks otherwise.\n\
                 Before answering trace, architecture, or control-flow questions, inspect the repo with real tools.\n\
                 Use verified file paths, function names, structs, enums, channels, and event types only.\n\
                 Prefer `trace_runtime_flow` for runtime wiring, session reset, startup, or reasoning/specular questions.\n\
                 Treat `trace_runtime_flow` output as authoritative over your own memory.\n\
                 If `trace_runtime_flow` fully answers the question, preserve its identifiers exactly and do not rename them in a styled rewrite.\n\
                 Do not invent names such as synthetic channels or subsystems.\n\
                 If a detail is not verified from the code or tool output, say `uncertain`.\n\
                For exact flow questions, answer in ordered steps and name the concrete functions and event types involved.\n"
            );
        }
        if capability_mode {
            system_msg.push_str(
                "\n\n# CAPABILITY QUESTION MODE\n\
                 This is a product or capability question unless the user explicitly asks about repository implementation.\n\
                 Answer from stable Hematite capabilities and current runtime state.\n\
                 It is correct to mention that Hematite itself is built in Rust when relevant, but do not imply that its project support is limited to Rust.\n\
                 Do NOT call repo-inspection tools like `map_project`, `read_file`, or LSP lookup tools unless the user explicitly asks about implementation or file ownership.\n\
                 Do NOT infer language or project support from unrelated dependencies, crates, or config files.\n\
                 Describe language and project support in terms of real mechanisms: reading files, editing code, searching the workspace, running shell commands, build verification, language-aware tooling when available, web research, vision analysis, and optional MCP tools if configured.\n\
                 If the user asks about languages, answer at the harness level: Hematite can help across many project languages even though Hematite itself is written in Rust.\n\
                 Prefer real programming language examples like Python, JavaScript, TypeScript, Go, C#, or similar over file extensions like `.json` or `.md`.\n\
                 For project-building questions, describe cross-project workflows like scaffolding files, shaping structure, implementing features, and running the appropriate local build or test commands for the target stack. Do not overclaim certainty.\n\
                 Never mention raw `mcp__*` tool names unless those tools are active this turn and directly relevant.\n\
                 Keep the answer short, plain, and ASCII-first.\n"
            );
        }
        if toolchain_mode {
            system_msg.push_str(
                "\n\n# TOOLCHAIN DISCIPLINE MODE\n\
                 This turn is about Hematite's real built-in tools and how to choose them.\n\
                 Prefer `describe_toolchain` before you try to summarize tool capabilities or propose a read-only investigation plan from memory.\n\
                 Use only real built-in tool names.\n\
                 Do not invent helper tools, MCP tool names, synthetic symbols, or example function names.\n\
                 If `describe_toolchain` fully answers the question, preserve its output exactly instead of restyling it.\n\
                 Be explicit about which tools are optional or conditional.\n"
            );
        }

        // ── Inject Pinned Files (Context Locking) ───────────────────────────
        system_msg.push_str(&format!(
            "\n\n# WORKFLOW MODE\nCURRENT WORKFLOW: {}\n",
            self.workflow_mode.label()
        ));
        match self.workflow_mode {
            WorkflowMode::Auto => system_msg.push_str(
                "AUTO means choose the narrowest effective path for the request. Answer directly when stable product logic exists. Inspect before editing. Mutate only when the user is clearly asking for implementation.\n",
            ),
            WorkflowMode::Ask => system_msg.push_str(
                "ASK means analysis only. Stay read-only, inspect the repo, explain findings, and do not make changes unless the user explicitly switches modes.\n",
            ),
            WorkflowMode::Code => system_msg.push_str(
                "CODE means implementation is allowed when needed. Keep proof-before-action, verification, and edit precision discipline.\n",
            ),
            WorkflowMode::Architect => system_msg.push_str(
                "ARCHITECT means plan first. Inspect, reason, and produce a concrete implementation approach before editing. Do not mutate code unless the user explicitly asks to implement.\n",
            ),
            WorkflowMode::ReadOnly => system_msg.push_str(
                "READ-ONLY means analysis only. Do not modify files, run mutating shell commands, or commit changes.\n",
            ),
        }
        {
            let pinned = self.pinned_files.lock().await;
            if !pinned.is_empty() {
                system_msg.push_str("\n\n# ACTIVE CONTEXT (PINNED FILES)\n");
                system_msg.push_str("The following files are locked in your active memory for high-fidelity reference.\n\n");
                for (path, content) in pinned.iter() {
                    system_msg.push_str(&format!("## FILE: {}\n```\n{}\n```\n\n", path, content));
                }
            }
        }
        self.append_session_handoff(&mut system_msg);
        if self.history.is_empty() || self.history[0].role != "system" {
            self.history.insert(0, ChatMessage::system(&system_msg));
        } else {
            self.history[0] = ChatMessage::system(&system_msg);
        }

        // Ensure a clean state for the new turn.
        self.cancel_token
            .store(false, std::sync::atomic::Ordering::SeqCst);

        // [Official Gemma-4 Spec] Purge reasoning history for new user turns.
        // History from previous turns must not be fed back into the prompt to prevent duplication.
        self.reasoning_history = None;

        let user_content = match self.think_mode {
            Some(true) => format!("/think\n{}", effective_user_input),
            Some(false) => format!("/no_think\n{}", effective_user_input),
            None => effective_user_input.clone(),
        };
        self.history.push(ChatMessage::user(&user_content));
        self.transcript.log_user(user_input);

        // Incremental re-index: update any files that changed since last turn.
        tokio::task::block_in_place(|| self.vein.index_project());

        // Query The Vein for context relevant to this turn.
        // Results are injected as a system message just before the user message,
        // giving the model relevant code snippets without extra tool calls.
        let vein_context = self.build_vein_context(&effective_user_input);

        // Route: pick fast vs think model based on the complexity of this request.
        let routed_model =
            route_model(
                &effective_user_input,
                effective_fast.as_deref(),
                effective_think.as_deref(),
            )
            .map(|s| s.to_string());

        let mut loop_intervention: Option<String> = None;

        // Safety cap – never spin forever on a broken model.
        let max_iters = 25;
        let mut consecutive_errors = 0;
        let mut first_iter = true;
        let _called_this_turn: std::collections::HashSet<String> = std::collections::HashSet::new();
        // Track identical tool results within this turn to detect logical loops.
        let _result_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        // Track the count of identical (name, args) calls to detect infinite tool loops.
        let mut repeat_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        // Track the index of the message that started THIS turn, so compaction doesn't summarize it.
        let mut turn_anchor = self.history.len().saturating_sub(1);

        for _iter in 0..max_iters {
            let mut mutation_occurred = false;
            // Priority Check: External Cancellation (via Esc key in TUI)
            if self.cancel_token.load(std::sync::atomic::Ordering::SeqCst) {
                self.cancel_token
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                let _ = tx
                    .send(InferenceEvent::Thought("Turn cancelled by user.".into()))
                    .await;
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }

            // ── Intelligence Surge: Proactive Compaction Check ──────────────────────
            if self
                .compact_history_if_needed(&tx, Some(turn_anchor))
                .await?
            {
                // After compaction, history is [system, summary, turn_anchor, ...]
                // The new turn_anchor is index 2.
                turn_anchor = 2;
            }

            // On the first iteration inject Vein context; subsequent iters use plain slice
            // (tool results are now in history so Vein context would be redundant).
            let messages = if first_iter {
                first_iter = false;
                self.context_window_slice_with_vein(vein_context.as_deref())
            } else {
                self.context_window_slice()
            };

            // Use the canonical system prompt from history[0] which was built
            // by InferenceEngine::build_system_prompt() + build_system_with_corrections()
            // and includes GPU state, git context, permissions, and instruction files.
            let mut prompt_msgs = vec![self.history[0].clone()];
            if let Some(intervention) = loop_intervention.take() {
                prompt_msgs.push(ChatMessage::system(&intervention));
            }
            prompt_msgs.extend(messages);

            let (text, tool_calls, usage) = self
                .engine
                .call_with_tools(&prompt_msgs, &self.tools, routed_model.as_deref())
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;

            // Update TUI token counter with actual usage from LM Studio.
            if let Some(u) = usage {
                let _ = tx.send(InferenceEvent::UsageUpdate(u)).await;
            }

            // Treat empty tool_calls arrays (Some(vec![])) the same as None —
            // the model returned text only; an empty array causes an infinite loop.
            let tool_calls = tool_calls.filter(|c| !c.is_empty());

            if let Some(calls) = tool_calls {
                if capability_mode
                    && !capability_needs_repo
                    && calls.iter().all(|c| is_capability_probe_tool(&c.function.name))
                {
                    loop_intervention = Some(
                        "STOP. This is a stable capability question. Do not inspect the repository or call tools. \
                         Answer directly from verified Hematite capabilities, current runtime state, and the documented product boundary. \
                         Do not mention raw `mcp__*` names unless they are active and directly relevant."
                            .to_string(),
                    );
                    let _ = tx
                        .send(InferenceEvent::Thought(
                            "Capability mode: skipping unnecessary repo-inspection tools and answering directly."
                                .into(),
                        ))
                        .await;
                    continue;
                }

                // VOCAL AGENT: If the model provided reasoning alongside tools,
                // stream it to the SPECULAR panel now using the hardened extraction.
                let raw_content = text.as_deref().unwrap_or(" ");

                if let Some(thought) = crate::agent::inference::extract_think_block(raw_content) {
                    let _ = tx.send(InferenceEvent::Thought(thought.clone())).await;
                    // Reasoning is silent (hidden in SPECULAR only).
                    self.reasoning_history = Some(thought);
                }

                // [Gemma-4 Protocol] Keep raw content (including thoughts) during tool loops.
                // Thoughts are only stripped before the 'final' user turn.
                self.history.push(ChatMessage::assistant_tool_calls(
                    raw_content,
                    calls.clone(),
                ));

                // ── LAYER 4: Parallel Tool Orchestration (Batching) ────────────────────
                let mut results = Vec::new();

                // Partition tool calls: Parallel Read vs Serial Mutating
                let (parallel_calls, serial_calls): (Vec<_>, Vec<_>) = calls
                    .clone()
                    .into_iter()
                    .partition(|c| is_parallel_safe(&c.function.name));

                // 1. Concurrent Execution (ParallelRead)
                if !parallel_calls.is_empty() {
                    let mut tasks = Vec::new();
                    for call in parallel_calls {
                        let tx_clone = tx.clone();
                        let config_clone = config.clone();
                        // Carry the real call ID into the outcome
                        let call_with_id = call.clone();
                        tasks.push(self.process_tool_call(
                            call_with_id.function,
                            config_clone,
                            yolo,
                            tx_clone,
                            call_with_id.id,
                        ));
                    }
                    // Wait for all read-only tasks to complete simultaneously.
                    results.extend(futures::future::join_all(tasks).await);
                }

                // 2. Sequential Execution (SerialMutating)
                for call in serial_calls {
                    results.push(
                        self.process_tool_call(
                            call.function,
                            config.clone(),
                            yolo,
                            tx.clone(),
                            call.id,
                        )
                        .await,
                    );
                }

                // 3. Collate Messages into History & UI
                let mut authoritative_tool_output: Option<String> = None;
                let mut blocked_policy_output: Option<String> = None;
                for res in results {
                    let call_id = res.call_id.clone();
                    let tool_name = res.tool_name.clone();
                    let final_output = res.output.clone();
                    let is_error = res.is_error;

                    for msg in res.msg_results {
                        self.history.push(msg);
                    }

                    // Update State for Verification Loop
                    if tool_name == "patch_hunk" || tool_name == "write_file" {
                        mutation_occurred = true;
                    }

                    // Update Repeat Guard
                    let call_key = format!(
                        "{}:{}",
                        tool_name,
                        serde_json::to_string(&res.args).unwrap_or_default()
                    );
                    let repeat_count = repeat_counts.entry(call_key.clone()).or_insert(0);
                    *repeat_count += 1;

                    if is_error {
                        consecutive_errors += 1;
                    } else {
                        consecutive_errors = 0;
                    }

                    if consecutive_errors >= 3 {
                        loop_intervention = Some(
                            "CRITICAL: Repeated tool failures detected. You are likely stuck in a loop. \
                             STOP all tool calls immediately. Analyze why your previous 3 calls failed \
                             (check for hallucinations or invalid arguments) and ask the user for \
                             clarification if you cannot proceed.".to_string()
                        );
                    }

                    if consecutive_errors >= 4 {
                        let _ = tx
                            .send(InferenceEvent::Error(
                                "Hard termination: too many consecutive tool errors.".into(),
                            ))
                            .await;
                        return Ok(());
                    }

                    let _ = tx
                        .send(InferenceEvent::ToolCallResult {
                            id: call_id.clone(),
                            name: tool_name.clone(),
                            output: final_output.clone(),
                            is_error,
                        })
                        .await;

                    // Cap output before history
                    let capped = cap_output(&final_output, 8000);
                    self.history
                        .push(ChatMessage::tool_result(&call_id, &tool_name, &capped));

                    if !is_error
                        && ((grounded_trace_mode && tool_name == "trace_runtime_flow")
                            || (toolchain_mode && tool_name == "describe_toolchain"))
                    {
                        authoritative_tool_output = Some(final_output.clone());
                    }

                    if res.blocked_by_policy && blocked_policy_output.is_none() {
                        blocked_policy_output = Some(final_output.clone());
                    }

                    if *repeat_count >= 5 {
                        let _ = tx.send(InferenceEvent::Done).await;
                        return Ok(());
                    }
                }

                if let Some(blocked_output) = blocked_policy_output {
                    self.history.push(ChatMessage::assistant_text(&blocked_output));
                    self.transcript.log_agent(&blocked_output);

                    for chunk in chunk_text(&blocked_output, 8) {
                        if !chunk.is_empty() {
                            let _ = tx.send(InferenceEvent::Token(chunk)).await;
                        }
                    }

                    let _ = tx.send(InferenceEvent::Done).await;
                    break;
                }

                if let Some(tool_output) = authoritative_tool_output {
                    self.history.push(ChatMessage::assistant_text(&tool_output));
                    self.transcript.log_agent(&tool_output);

                    for chunk in chunk_text(&tool_output, 8) {
                        if !chunk.is_empty() {
                            let _ = tx.send(InferenceEvent::Token(chunk)).await;
                        }
                    }

                    let _ = tx.send(InferenceEvent::Done).await;
                    break;
                }

                // 4. Auto-Verification Loop (The Perfect Bake)
                if mutation_occurred && !yolo {
                    let _ = tx
                        .send(InferenceEvent::Thought(
                            "Self-Verification: Running 'cargo check' to ensure build integrity..."
                                .into(),
                        ))
                        .await;
                    let verify_res = self.auto_verify_build().await;
                    self.record_verify_build_result(verify_res.contains("BUILD SUCCESS")).await;
                    self.history.push(ChatMessage::system(&format!(
                        "\n# SYSTEM VERIFICATION\n{verify_res}"
                    )));
                    let _ = tx
                        .send(InferenceEvent::Thought(
                            "Verification turn injected into history.".into(),
                        ))
                        .await;
                }

                // Continue loop – the model will respond to the results.
                continue;
            } else if let Some(response_text) = text {
                // 1. Process and route the reasoning block to SPECULAR.
                if let Some(thought) = crate::agent::inference::extract_think_block(&response_text)
                {
                    let _ = tx.send(InferenceEvent::Thought(thought.clone())).await;
                    // Persist for history audit (stripped from next turn by Volatile Reasoning rule).
                    // This will be summarized in the next turn's system prompt.
                    self.reasoning_history = Some(thought);
                }

                // 2. Process and stream the final answer to the chat interface.
                let cleaned = crate::agent::inference::strip_think_blocks(&response_text);

                // [Hardened Interface] Strictly respect the stripper.
                // If it's empty, we stay silent in the chat area (reasoning is in SPECULAR).
                if cleaned.is_empty() {
                    let _ = tx.send(InferenceEvent::Done).await;
                    break;
                }

                self.history.push(ChatMessage::assistant_text(&cleaned));
                self.transcript.log_agent(&cleaned);

                // Send in smooth chunks for that professional UI feel.
                for chunk in chunk_text(&cleaned, 8) {
                    if !chunk.is_empty() {
                        let _ = tx.send(InferenceEvent::Token(chunk.clone())).await;
                    }
                }

                let _ = tx.send(InferenceEvent::Done).await;
                break;
            } else {
                let _ = tx
                    .send(InferenceEvent::Error(
                        "Model returned an empty response.".into(),
                    ))
                    .await;
                break;
            }
        }

        self.trim_history(80);
        self.refresh_session_memory();
        self.save_session();
        Ok(())
    }

    /// [Task Analyzer] Run 'cargo check' and return a concise summary for the model.
    async fn auto_verify_build(&self) -> String {
        match crate::tools::verify_build::execute(&serde_json::json!({ "action": "build" })).await {
            Ok(out) => "BUILD SUCCESS: Your changes are architecturally sound.\n\n".to_string() + &cap_output(&out, 2000),
            Err(e) => format!(
                "BUILD FAILURE: The build is currently broken. FIX THESE ERRORS IMMEDIATELY:\n\n{}",
                cap_output(&e, 2000)
            ),
        }
    }

    /// Triggers an LLM call to summarize old messages if history exceeds the VRAM character limit.
    /// Triggers the Deterministic Smart Compaction algorithm to shrink history while preserving context.
    /// Triggers the Recursive Context Compactor.
    async fn compact_history_if_needed(
        &mut self,
        tx: &mpsc::Sender<InferenceEvent>,
        anchor_index: Option<usize>,
    ) -> Result<bool, String> {
        let vram_ratio = self.gpu_state.ratio();
        let context_length = self.engine.context_length;
        let config = CompactionConfig::adaptive(context_length, vram_ratio);

        if !compaction::should_compact(&self.history, context_length, vram_ratio) {
            return Ok(false);
        }

        let _ = tx
            .send(InferenceEvent::Thought(format!(
                "Compaction: ctx={}k vram={:.0}% threshold={}k tokens — chaining summary...",
                context_length / 1000,
                vram_ratio * 100.0,
                config.max_estimated_tokens / 1000,
            )))
            .await;

        let result = compaction::compact_history(
            &self.history,
            self.running_summary.as_deref(),
            config,
            anchor_index,
        );

        self.history = result.messages;
        self.running_summary = result.summary;

        // Layer 6: Memory Synthesis (Task Context Persistence)
        self.session_memory = compaction::extract_memory(&self.history);

        // Jinja alignment: preserved slice may start with assistant/tool messages.
        // Strip any leading non-user messages so the first non-system message is always user.
        let first_non_sys = self
            .history
            .iter()
            .position(|m| m.role != "system")
            .unwrap_or(self.history.len());
        if first_non_sys < self.history.len() {
            if let Some(user_offset) = self.history[first_non_sys..]
                .iter()
                .position(|m| m.role == "user")
            {
                if user_offset > 0 {
                    self.history
                        .drain(first_non_sys..first_non_sys + user_offset);
                }
            }
        }

        let _ = tx
            .send(InferenceEvent::Thought(format!(
                "Memory Synthesis: Extracted context for task: '{}'. Working set: {} files.",
                self.session_memory.current_task,
                self.session_memory.working_set.len()
            )))
            .await;

        Ok(true)
    }

    /// Query The Vein for context relevant to the user's message.
    /// Returns a formatted system message string, or None if nothing useful found.
    fn build_vein_context(&self, query: &str) -> Option<String> {
        // Skip trivial / very short inputs.
        if query.trim().split_whitespace().count() < 3 {
            return None;
        }

        let results = self.vein.search_context(query, 4).ok()?;
        if results.is_empty() {
            return None;
        }

        let mut ctx = String::from(
            "# Relevant context from The Vein (auto-retrieved from your codebase)\n\
             Use this to answer without needing extra read_file calls where possible.\n\n",
        );

        let mut total = 0usize;
        const MAX_CTX_CHARS: usize = 3_000;

        for r in results {
            if total >= MAX_CTX_CHARS {
                break;
            }
            let snippet = if r.content.len() > 800 {
                format!("{}...", &r.content[..800])
            } else {
                r.content.clone()
            };
            ctx.push_str(&format!("--- {} ---\n{}\n\n", r.path, snippet));
            total += snippet.len() + r.path.len() + 10;
        }

        Some(ctx)
    }

    /// Returns the conversation history (WITHOUT the system prompt) for the context window.
    /// This ensures we don't have redundant system blocks and prevents Jinja crashes.
    fn context_window_slice(&self) -> Vec<ChatMessage> {
        let mut result = Vec::new();

        // Skip index 0 (the raw system message) and any stray system messages in history.
        if self.history.len() > 1 {
            for m in &self.history[1..] {
                if m.role == "system" {
                    continue;
                }

                let mut sanitized = m.clone();
                // DEEP SANITIZE: LM Studio Jinja templates for Qwen crash on truly empty content.
                if (m.role == "assistant" || m.role == "tool") && m.content.as_str().is_empty() {
                    sanitized.content = MessageContent::Text(" ".into());
                }
                result.push(sanitized);
            }
        }

        // Jinja Guard: The first message after the system prompt MUST be 'user'.
        // If not (e.g. because of compaction), we insert a tiny anchor.
        if !result.is_empty() && result[0].role != "user" {
            result.insert(0, ChatMessage::user("Continuing previous context..."));
        }

        result
    }

    /// Like context_window_slice but maintains history without system prompts.
    fn context_window_slice_with_vein(&self, _vein_context: Option<&str>) -> Vec<ChatMessage> {
        self.context_window_slice()
    }

    /// Build a deterministic smart summary of recent conversation history.
    #[allow(dead_code)]
    fn build_smart_summary(&self, messages: &[ChatMessage]) -> String {
        let mut lines = vec![
            "--- Context Summary ---".to_string(),
            format!("- Scope: {} messages compacted.", messages.len()),
        ];

        // 1. Key Files Referenced
        let mut files = std::collections::HashSet::new();
        for m in messages {
            for word in m.content.as_str().split_whitespace() {
                let word = word.trim_matches(|c: char| {
                    matches!(c, ',' | '.' | ':' | ';' | ')' | '(' | '"' | '\'' | '`')
                });
                if (word.contains('/') || word.contains('\\'))
                    && (word.ends_with(".rs")
                        || word.ends_with(".sh")
                        || word.ends_with(".toml")
                        || word.ends_with(".md"))
                {
                    files.insert(word.to_string());
                }
            }
        }
        if !files.is_empty() {
            let file_list: Vec<String> = files.into_iter().take(10).collect();
            lines.push(format!("- Key Files: {}", file_list.join(", ")));
        }

        // 2. Pending Work / Verbatim User Requests
        let mut recent_requests = Vec::new();
        for m in messages.iter().filter(|m| m.role == "user").rev().take(3) {
            let content_str = m.content.as_str();
            let truncated = if content_str.len() > 120 {
                let mut s: String = content_str.chars().take(117).collect();
                s.push_str("...");
                s
            } else {
                content_str.to_string()
            };
            recent_requests.push(truncated);
        }
        if !recent_requests.is_empty() {
            lines.push("- Recent User Requests:".to_string());
            for r in recent_requests.into_iter().rev() {
                lines.push(format!("  - {}", r));
            }
        }

        // 3. Compact Key Timeline
        lines.push("- Key Timeline:".to_string());
        for m in messages.iter().take(20) {
            // Keep the first 20 in sequence for the timeline
            let content_str = m.content.as_str();
            let content_preview = if content_str.len() > 100 {
                format!("{}...", &content_str[..97])
            } else if content_str.trim().is_empty() && !m.tool_calls.is_empty() {
                format!(
                    "Executing tools: {:?}",
                    m.tool_calls
                        .iter()
                        .map(|c| &c.function.name)
                        .collect::<Vec<_>>()
                )
            } else {
                content_str.to_string()
            };
            lines.push(format!(
                "  - {}: {}",
                m.role,
                content_preview.replace('\n', " ")
            ));
        }

        lines.join("\n")
    }

    /// Drop old turns from the middle of history.
    fn trim_history(&mut self, max_messages: usize) {
        if self.history.len() <= max_messages {
            return;
        }
        // Always keep [0] (system prompt).
        let excess = self.history.len() - max_messages;
        self.history.drain(1..=excess);
    }

    /// Performs an automated verification (e.g. cargo check) and returns the result if relevant.
    #[allow(dead_code)]
    async fn perform_auto_verify(&mut self) -> Option<String> {
        let root = crate::tools::file_ops::workspace_root();

        // Strategy: Only run if it's a Rust project (Cargo.toml exists).
        if root.join("Cargo.toml").exists() {
            let output = crate::tools::shell::execute(&serde_json::json!({
                "command": "cargo check --color never",
                "timeout_secs": 15
            }))
            .await;

            match output {
                Ok(out) => {
                    if out.contains("error:") || out.contains("warning:") {
                        return Some(out);
                    }
                }
                Err(e) => return Some(format!("Verification failed to run: {}", e)),
            }
        }
        None
    }

    /// P1: Attempt to fix malformed JSON tool arguments by asking the model to re-output them.
    async fn repair_tool_args(
        &self,
        tool_name: &str,
        bad_json: &str,
        tx: &mpsc::Sender<InferenceEvent>,
    ) -> Result<Value, String> {
        let _ = tx
            .send(InferenceEvent::Thought(format!(
                "Attempting to repair malformed JSON for '{}'...",
                tool_name
            )))
            .await;

        let prompt = format!(
            "The following JSON for tool '{}' is malformed and failed to parse:\n\n```json\n{}\n```\n\nOutput ONLY the corrected JSON string that fixes the syntax error (e.g. missing commas, unescaped quotes). Do NOT include markdown blocks or any other text.",
            tool_name, bad_json
        );

        let messages = vec![
            ChatMessage::system("You are a JSON repair tool. Output ONLY pure JSON."),
            ChatMessage::user(&prompt),
        ];

        // Use fast model for speed if available.
        let (text, _, _) = self
            .engine
            .call_with_tools(&messages, &[], self.fast_model.as_deref())
            .await
            .map_err(|e| e.to_string())?;

        let cleaned = text
            .unwrap_or_default()
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string();

        serde_json::from_str(&cleaned).map_err(|e| format!("Repair failed: {}", e))
    }

    /// P2: Run a fast validation step after file writes to check for subtle logic errors.
    async fn run_critic_check(
        &self,
        path: &str,
        content: &str,
        tx: &mpsc::Sender<InferenceEvent>,
    ) -> Option<String> {
        // Only run for source code files.
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        const CRITIC_EXTS: &[&str] = &["rs", "js", "ts", "py", "go", "c", "cpp"];
        if !CRITIC_EXTS.contains(&ext) {
            return None;
        }

        let _ = tx
            .send(InferenceEvent::Thought(format!(
                "CRITIC: Reviewing changes to '{}'...",
                path
            )))
            .await;

        let truncated = cap_output(content, 4000);

        let prompt = format!(
            "You are a Senior Security and Code Quality auditor. Review this file content for '{}' and identify any critical logic errors, security vulnerabilities, or missing error handling. Be extremely concise. If the code looks good, output 'PASS'.\n\n```{}\n{}\n```",
            path, ext, truncated
        );

        let messages = vec![
            ChatMessage::system("You are a technical critic. Identify ONLY critical issues. Output 'PASS' if none found."),
            ChatMessage::user(&prompt)
        ];

        let (text, _, _) = self
            .engine
            .call_with_tools(&messages, &[], self.fast_model.as_deref())
            .await
            .ok()?;

        let critique = text?.trim().to_string();
        if critique.to_uppercase().contains("PASS") || critique.is_empty() {
            None
        } else {
            Some(critique)
        }
    }
}

// ── Tool dispatcher ───────────────────────────────────────────────────────────

pub async fn dispatch_tool(name: &str, args: &Value) -> Result<String, String> {
    match name {
        "shell"       => crate::tools::shell::execute(args).await,
        "map_project" => crate::tools::project_map::map_project(args).await,
        "trace_runtime_flow" => crate::tools::runtime_trace::trace_runtime_flow(args).await,
        "describe_toolchain" => crate::tools::toolchain::describe_toolchain(args).await,
        "read_file"   => crate::tools::file_ops::read_file(args).await,
        "inspect_lines" => crate::tools::file_ops::inspect_lines(args).await,
        "write_file"  => crate::tools::file_ops::write_file(args).await,
        "edit_file"   => crate::tools::file_ops::edit_file(args).await,
        "patch_hunk"  => crate::tools::file_ops::patch_hunk(args).await,
        "multi_search_replace" => crate::tools::file_ops::multi_search_replace(args).await,
        "list_files"  => crate::tools::file_ops::list_files(args).await,
        "grep_files"  => crate::tools::file_ops::grep_files(args).await,
        "git_commit"  => crate::tools::git::execute(args).await,
        "git_push"    => crate::tools::git::execute_push(args).await,
        "git_remote"  => crate::tools::git::execute_remote(args).await,
        "git_onboarding" => crate::tools::git_onboarding::execute(args).await,
        "verify_build"  => crate::tools::verify_build::execute(args).await,
        "git_worktree"  => crate::tools::git::execute_worktree(args).await,
        "health"      => crate::tools::health::execute(args).await,
        "research_web"=> crate::tools::research::execute_search(args).await,
        "fetch_docs"  => crate::tools::research::execute_fetch(args).await,
        "manage_tasks" => crate::tools::tasks::manage_tasks(args).await,
        "maintain_plan" => crate::tools::plan::maintain_plan(args).await,
        "generate_walkthrough" => crate::tools::plan::generate_walkthrough(args).await,
        // clarify is handled specially in run_turn — it should never reach here,
        // but return a helpful string if it somehow does.
        "clarify"    => {
            let q = args.get("question").and_then(|v| v.as_str()).unwrap_or("?");
            Ok(format!("[clarify] {q}"))
        }
        "vision_analyze" => Err("Tool 'vision_analyze' must be dispatched by ConversationManager (it requires hardware engine access).".into()),
        other => {
            // HALLUCINATION GUARD: If the tool name contains a dot or a slash,
            // it's probably a path, not a tool. Redirect the model.
            if other.contains('.') || other.contains('/') || other.contains('\\') {
                Err(format!("'{}' is a PATH, not a tool. You correctly identified the location, but you MUST use `read_file` or `list_files` (internal) or `powershell` (external) to access it.", other))
            } else if other.to_lowercase() == "hematite" || other.to_lowercase() == "assistant" || other.to_lowercase() == "ai" {
                Err(format!("'{}' is YOUR IDENTITY, not a tool. Use list_files or read_file to explore the codebase.", other))
            } else if matches!(other.to_lowercase().as_str(), "thought" | "think" | "reasoning" | "thinking" | "internal") {
                Err(format!("'{}' is NOT a tool — it is a reasoning tag. Output your answer as plain text after your <think> block.", other))
            } else {
                Err(format!("Unknown tool: '{}'", other))
            }
        }
    }
}

impl ConversationManager {
    /// Checks if a tool call is authorized given the current configuration and mode.
    fn check_authorization(
        &self,
        name: &str,
        args: &serde_json::Value,
        config: &crate::agent::config::HematiteConfig,
        yolo_flag: bool,
    ) -> crate::agent::config::PermissionDecision {
        use crate::agent::config::{PermissionDecision, PermissionMode};

        // 1. System Admin Mode: Absolute Authority.
        if config.mode == PermissionMode::SystemAdmin {
            return PermissionDecision::Allow;
        }

        // 2. Read-Only Mode: Strict verification.
        if config.mode == PermissionMode::ReadOnly {
            if is_destructive_tool(name) {
                // Check if there's an explicit 'allow' override for this specific call (e.g. "git branch").
                if name == "shell" {
                    let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                    if matches!(
                        crate::agent::config::permission_for_shell(cmd, config),
                        PermissionDecision::Allow
                    ) {
                        return PermissionDecision::Allow;
                    }
                }
                return PermissionDecision::Deny;
            }
            return PermissionDecision::Allow;
        }

        // 3. Developer Mode (Default): Interactive safety.
        if yolo_flag {
            return PermissionDecision::Allow;
        }

        if requires_approval(name, args, config) {
            PermissionDecision::Ask
        } else {
            PermissionDecision::Allow
        }
    }

    /// Layer 4: Isolated tool execution logic. Does not mutate 'self' to allow parallelism.
    async fn process_tool_call(
        &self,
        call: ToolCallFn,
        config: crate::agent::config::HematiteConfig,
        yolo: bool,
        tx: mpsc::Sender<InferenceEvent>,
        real_id: String,
    ) -> ToolExecutionOutcome {
        let mut msg_results = Vec::new();

        // 1. Argument Parsing & Repair
        let args: Value = match serde_json::from_str(&call.arguments) {
            Ok(v) => v,
            Err(_) => {
                match self
                    .repair_tool_args(&call.name, &call.arguments, &tx)
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx
                            .send(InferenceEvent::Thought(format!(
                                "JSON Repair failed: {}",
                                e
                            )))
                            .await;
                        Value::Object(Default::default())
                    }
                }
            }
        };

        let display = format_tool_display(&call.name, &args);
        let precondition_result = self.validate_action_preconditions(&call.name, &args).await;
        let auth = self.check_authorization(&call.name, &args, &config, yolo);

        // 2. Permission Check
        let decision_result = match precondition_result {
            Err(e) => Err(e),
            Ok(_) => match auth {
            crate::agent::config::PermissionDecision::Allow => Ok(()),
            crate::agent::config::PermissionDecision::Ask => {
                let (approve_tx, approve_rx) = tokio::sync::oneshot::channel::<bool>();
                let _ = tx
                    .send(InferenceEvent::ApprovalRequired {
                        id: real_id.clone(),
                        name: call.name.clone(),
                        display: display.clone(),
                        responder: approve_tx,
                    })
                    .await;

                match approve_rx.await {
                    Ok(true) => Ok(()),
                    _ => Err("Declined by user".into()),
                }
            }
            crate::agent::config::PermissionDecision::Deny => Err(format!(
                "Access Denied: Tool '{}' is forbidden in current Permission Mode.",
                call.name
            )),
            _ => Err("Unauthorized".into()),
        }};
        let blocked_by_policy = matches!(&decision_result, Err(e) if e.starts_with("Action blocked:"));

        // 3. Execution (Local or MCP)
        let (output, is_error) = match decision_result {
            Err(e) => (format!("Error: {}", e), true),
            Ok(_) => {
                let _ = tx
                    .send(InferenceEvent::ToolCallStart {
                        id: real_id.clone(),
                        name: call.name.clone(),
                        args: display.clone(),
                    })
                    .await;

                let result = if call.name.starts_with("lsp_") {
                    let lsp = self.lsp_manager.clone();
                    let path = args
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let line = args.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let character =
                        args.get("character").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

                    match call.name.as_str() {
                        "lsp_definitions" => {
                            crate::tools::lsp_tools::lsp_definitions(lsp, path, line, character)
                                .await
                        }
                        "lsp_references" => {
                            crate::tools::lsp_tools::lsp_references(lsp, path, line, character)
                                .await
                        }
                        "lsp_hover" => {
                            crate::tools::lsp_tools::lsp_hover(lsp, path, line, character).await
                        }
                        "lsp_search_symbol" => {
                            let query = args
                                .get("query")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            crate::tools::lsp_tools::lsp_search_symbol(lsp, query).await
                        }
                        "lsp_rename_symbol" => {
                            let new_name = args
                                .get("new_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            crate::tools::lsp_tools::lsp_rename_symbol(
                                lsp, path, line, character, new_name,
                            )
                            .await
                        }
                        "lsp_get_diagnostics" => {
                            crate::tools::lsp_tools::lsp_get_diagnostics(lsp, path).await
                        }
                        _ => Err(format!("Unknown LSP tool: {}", call.name)),
                    }
                } else if call.name == "auto_pin_context" {
                    let pts = args.get("paths").and_then(|v| v.as_array());
                    let reason = args
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .unwrap_or("uninformed scoping");
                    if let Some(arr) = pts {
                        let mut pinned = Vec::new();
                        {
                            let mut guard = self.pinned_files.lock().await;
                            const MAX_PINNED_SIZE: u64 = 25 * 1024 * 1024; // 25MB Safety Valve

                            for v in arr.iter().take(3) {
                                if let Some(p) = v.as_str() {
                                    if let Ok(meta) = std::fs::metadata(p) {
                                        if meta.len() > MAX_PINNED_SIZE {
                                            let _ = tx.send(InferenceEvent::Thought(format!("[GUARD] Skipping {} - size ({} bytes) exceeds VRAM safety limit (25MB).", p, meta.len()))).await;
                                            continue;
                                        }
                                        if let Ok(content) = std::fs::read_to_string(p) {
                                            guard.insert(p.to_string(), content);
                                            pinned.push(p.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        let msg = format!(
                            "Autonomous Scoping: Locked {} in high-fidelity memory. Reason: {}",
                            pinned.join(", "),
                            reason
                        );
                        let _ = tx
                            .send(InferenceEvent::Thought(format!("[AUTO-PIN] {}", msg)))
                            .await;
                        Ok(msg)
                    } else {
                        Err("Missing 'paths' array for auto_pin_context.".to_string())
                    }
                } else if call.name == "list_pinned" {
                    let paths_msg = {
                        let pinned = self.pinned_files.lock().await;
                        if pinned.is_empty() {
                            "No files are currently pinned.".to_string()
                        } else {
                            let paths: Vec<_> = pinned.keys().cloned().collect();
                            format!(
                                "Currently pinned files in active memory:\n- {}",
                                paths.join("\n- ")
                            )
                        }
                    };
                    Ok(paths_msg)
                } else if call.name.starts_with("mcp__") {
                    let mut mcp = self.mcp_manager.lock().await;
                    match mcp.call_tool(&call.name, &args).await {
                        Ok(res) => Ok(res),
                        Err(e) => Err(e.to_string()),
                    }
                } else if call.name == "swarm" {
                    // ── Swarm Orchestration ──
                    let tasks_val = args.get("tasks").cloned().unwrap_or(Value::Array(vec![]));
                    let max_workers = args
                        .get("max_workers")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(3) as usize;

                    let mut task_objs = Vec::new();
                    if let Value::Array(arr) = tasks_val {
                        for v in arr {
                            let id = v
                                .get("id")
                                .and_then(|x| x.as_str())
                                .unwrap_or("?")
                                .to_string();
                            let target = v
                                .get("target")
                                .and_then(|x| x.as_str())
                                .unwrap_or("?")
                                .to_string();
                            let instruction = v
                                .get("instruction")
                                .and_then(|x| x.as_str())
                                .unwrap_or("?")
                                .to_string();
                            task_objs.push(crate::agent::parser::WorkerTask {
                                id,
                                target,
                                instruction,
                            });
                        }
                    }

                    if task_objs.is_empty() {
                        Err("No tasks provided for swarm.".to_string())
                    } else {
                        let (swarm_tx_internal, mut swarm_rx_internal) =
                            tokio::sync::mpsc::channel(32);
                        let tx_forwarder = tx.clone();

                        // Bridge SwarmMessage -> InferenceEvent
                        tokio::spawn(async move {
                            while let Some(msg) = swarm_rx_internal.recv().await {
                                match msg {
                                    crate::agent::swarm::SwarmMessage::Progress(id, p) => {
                                        let _ = tx_forwarder
                                            .send(InferenceEvent::Thought(format!(
                                                "Swarm [{}]: {}% complete",
                                                id, p
                                            )))
                                            .await;
                                    }
                                    crate::agent::swarm::SwarmMessage::ReviewRequest {
                                        worker_id,
                                        file_path,
                                        before: _,
                                        after: _,
                                        tx,
                                    } => {
                                        let (approve_tx, approve_rx) =
                                            tokio::sync::oneshot::channel::<bool>();
                                        let display = format!(
                                            "Swarm worker [{}]: Integrated changes into {:?}",
                                            worker_id, file_path
                                        );
                                        let _ = tx_forwarder
                                            .send(InferenceEvent::ApprovalRequired {
                                                id: format!("swarm_{}", worker_id),
                                                name: "swarm_apply".to_string(),
                                                display,
                                                responder: approve_tx,
                                            })
                                            .await;
                                        if let Ok(approved) = approve_rx.await {
                                            let response = if approved {
                                                crate::agent::swarm::ReviewResponse::Accept
                                            } else {
                                                crate::agent::swarm::ReviewResponse::Reject
                                            };
                                            let _ = tx.send(response);
                                        }
                                    }
                                    crate::agent::swarm::SwarmMessage::Done => {}
                                }
                            }
                        });

                        let coordinator = self.swarm_coordinator.clone();
                        match coordinator
                            .dispatch_swarm(task_objs, swarm_tx_internal, max_workers)
                            .await
                        {
                            Ok(_) => Ok(
                                "Swarm execution completed. Check files for integration results."
                                    .to_string(),
                            ),
                            Err(e) => Err(format!("Swarm failure: {}", e)),
                        }
                    }
                } else if call.name == "vision_analyze" {
                    crate::tools::vision::vision_analyze(&self.engine, &args).await
                } else {
                    dispatch_tool(&call.name, &args).await
                };

                match result {
                    Ok(o) => (o, false),
                    Err(e) => (format!("Error: {}", e), true),
                }
            }
        };

        // ── Session Economics ────────────────────────────────────────────────
        {
            if let Ok(mut econ) = self.engine.economics.lock() {
                econ.record_tool(&call.name, !is_error);
            }
        }

        if !is_error {
            if matches!(call.name.as_str(), "read_file" | "inspect_lines") {
                if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                    self.record_read_observation(path).await;
                }
            }

            if call.name == "verify_build" {
                let ok = output.contains("BUILD OK")
                    || output.contains("BUILD SUCCESS")
                    || output.contains("BUILD OKAY");
                self.record_verify_build_result(ok).await;
            }

            if matches!(
                call.name.as_str(),
                "write_file" | "edit_file" | "patch_hunk" | "multi_search_replace"
            ) || is_mcp_mutating_tool(&call.name)
            {
                self.record_successful_mutation(
                    action_target_path(&call.name, &args).as_deref(),
                )
                .await;
            }

            if let Some(receipt) = self.build_action_receipt(&call.name, &args, &output, is_error) {
                msg_results.push(receipt);
            }
        }

        // 4. Critic Check (Specular Tier 2)
        // Gated: Only run on code files with substantive content to avoid burning tokens
        // on trivial doc/config edits.
        if !is_error && (call.name == "edit_file" || call.name == "write_file") {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let ext = std::path::Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            const SKIP_EXTS: &[&str] = &[
                "md",
                "toml",
                "json",
                "txt",
                "yml",
                "yaml",
                "cfg",
                "csv",
                "lock",
                "gitignore",
            ];
            let line_count = content.lines().count();
            if !path.is_empty()
                && !content.is_empty()
                && !SKIP_EXTS.contains(&ext)
                && line_count >= 50
            {
                if let Some(critique) = self.run_critic_check(path, content, &tx).await {
                    msg_results.push(ChatMessage::system(&format!(
                        "[CRITIC REVIEW OF {}]\nIssues found:\n\n{}",
                        path, critique
                    )));
                }
            }
        }

        ToolExecutionOutcome {
            call_id: real_id,
            tool_name: call.name,
            args,
            output,
            is_error,
            blocked_by_policy,
            msg_results,
        }
    }
}

/// The result of an isolated tool execution.
/// Used to bridge Parallel/Serial execution back to the main history.
struct ToolExecutionOutcome {
    call_id: String,
    tool_name: String,
    args: Value,
    output: String,
    is_error: bool,
    blocked_by_policy: bool,
    msg_results: Vec<ChatMessage>,
}

/// Returns true if the tool can modify files or execute arbitrary shell commands.
fn is_destructive_tool(name: &str) -> bool {
    matches!(
        name,
        "write_file"
            | "edit_file"
            | "patch_hunk"
            | "shell"
            | "git_commit"
            | "git_push"
            | "git_remote"
            | "git_onboarding"
    ) || is_mcp_mutating_tool(name)
}

/// Returns true if the path is inside a "Safe Zone" (.hematite/ or tmp/)
/// where permission prompts are bypassed for internal bookkeeping.
fn is_path_safe(path: &str) -> bool {
    let p = path.to_lowercase();
    p.contains(".hematite/")
        || p.contains(".hematite\\")
        || p.contains("tmp/")
        || p.contains("tmp\\")
}

fn normalize_workspace_path(path: &str) -> String {
    let root = crate::tools::file_ops::workspace_root();
    let candidate = std::path::Path::new(path);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    joined
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase()
}

fn is_mcp_mutating_tool(name: &str) -> bool {
    if !name.starts_with("mcp__") {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    [
        "__edit",
        "__write",
        "__create",
        "__move",
        "__delete",
        "__remove",
        "__rename",
        "__replace",
        "__patch",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn is_mcp_workspace_read_tool(name: &str) -> bool {
    if !name.starts_with("mcp__filesystem__") {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    [
        "__read",
        "__list",
        "__search",
        "__get_file_info",
        "__stat",
        "__metadata",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn action_target_path(name: &str, args: &Value) -> Option<String> {
    match name {
        "write_file" | "edit_file" | "patch_hunk" | "multi_search_replace" => args
            .get("path")
            .and_then(|v| v.as_str())
            .map(normalize_workspace_path),
        _ if is_mcp_mutating_tool(name) => args
            .get("path")
            .or_else(|| args.get("target"))
            .or_else(|| args.get("target_path"))
            .or_else(|| args.get("destination"))
            .or_else(|| args.get("destination_path"))
            .or_else(|| args.get("source"))
            .or_else(|| args.get("source_path"))
            .or_else(|| args.get("from"))
            .and_then(|v| v.as_str())
            .map(normalize_workspace_path),
        _ => None,
    }
}

fn is_code_like_path(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "rs" | "js" | "ts" | "tsx" | "jsx" | "py" | "go" | "java" | "c" | "cpp" | "cc" | "h"
            | "hpp" | "cs" | "swift" | "kt" | "kts" | "rb" | "php"
    )
}

/// Returns true if this tool call should require explicit user approval in Developer mode.
fn requires_approval(
    name: &str,
    args: &Value,
    config: &crate::agent::config::HematiteConfig,
) -> bool {
    use crate::agent::config::{permission_for_shell, PermissionDecision};
    use crate::tools::RiskLevel;

    // MCP tools always ask — external servers are untrusted by default.
    if name.starts_with("mcp__") {
        return true;
    }

    // Layer 5: Safe Zone Bypass (Internal logs, memory, temp files)
    if name == "write_file" || name == "edit_file" {
        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
            if is_path_safe(path) {
                return false;
            }
        }
    }

    if name == "shell" {
        let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");

        // Config rules take priority over the risk classifier.
        match permission_for_shell(cmd, config) {
            PermissionDecision::Allow => return false,
            PermissionDecision::Deny | PermissionDecision::Ask => return true,
            PermissionDecision::UseRiskClassifier => {}
        }

        // Hard safety check (blacklisted paths/system dirs).
        if crate::tools::guard::bash_is_safe(cmd).is_err() {
            return true;
        }

        return match crate::tools::guard::classify_bash_risk(cmd) {
            RiskLevel::High => true,
            RiskLevel::Moderate => true, // We removed auto_approve_moderate for now to simplify
            RiskLevel::Safe => false,
        };
    }

    false
}

// ── Display helpers ───────────────────────────────────────────────────────────

pub fn format_tool_display(name: &str, args: &Value) -> String {
    let get = |key: &str| {
        args.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    match name {
        "shell" => format!("$ {}", get("command")),
        "map_project" => "map project architecture".to_string(),
        "trace_runtime_flow" => format!("trace runtime {}", get("topic")),
        "describe_toolchain" => format!("describe toolchain {}", get("topic")),
        _ => format!("{} {:?}", name, args),
    }
}

// ── Text utilities ────────────────────────────────────────────────────────────

// Moved strip_think_blocks to inference.rs

fn cap_output(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        text.to_string()
    } else {
        // Find the largest byte index <= max_bytes that is a valid char boundary.
        let mut split_at = max_bytes;
        while !text.is_char_boundary(split_at) && split_at > 0 {
            split_at -= 1;
        }
        format!(
            "{}\n... [output capped at {}B]",
            &text[..split_at],
            max_bytes
        )
    }
}

/// Split text into chunks of roughly `words_per_chunk` whitespace-separated tokens.
fn chunk_text(text: &str, words_per_chunk: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut count = 0;

    for ch in text.chars() {
        current.push(ch);
        if ch == ' ' || ch == '\n' {
            count += 1;
            if count >= words_per_chunk {
                chunks.push(current.clone());
                current.clear();
                count = 0;
            }
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn build_system_with_corrections(
    base: &str,
    hints: &[String],
    gpu: &Arc<GpuState>,
    git: &Arc<crate::agent::git_monitor::GitState>,
    config: &crate::agent::config::HematiteConfig,
) -> String {
    let mut system_msg = base.to_string();

    // Inject Permission Mode.
    system_msg.push_str("\n\n# Permission Mode\n");
    let mode_label = match config.mode {
        crate::agent::config::PermissionMode::ReadOnly => "READ-ONLY",
        crate::agent::config::PermissionMode::Developer => "DEVELOPER",
        crate::agent::config::PermissionMode::SystemAdmin => "SYSTEM-ADMIN (UNRESTRICTED)",
    };
    system_msg.push_str(&format!("CURRENT MODE: {}\n", mode_label));

    if config.mode == crate::agent::config::PermissionMode::ReadOnly {
        system_msg.push_str("PERMISSION: You are restricted to READ-ONLY access. Do NOT attempt to use write_file, edit_file, or shell for any modification. Focus entirely on analysis, indexing, and reporting.\n");
    } else {
        system_msg.push_str("PERMISSION: You have authority to modify code and execute tests with user oversight.\n");
    }

    // Inject live hardware status.
    let (used, total) = gpu.read();
    if total > 0 {
        system_msg.push_str("\n\n# Terminal Hardware Context\n");
        system_msg.push_str(&format!(
            "HOST GPU: {} | VRAM: {:.1}GB / {:.1}GB ({:.0}% used)\n",
            gpu.gpu_name(),
            used as f64 / 1024.0,
            total as f64 / 1024.0,
            gpu.ratio() * 100.0
        ));
        system_msg.push_str("Use this awareness to manage your context window responsibly.\n");
    }

    // Inject Git Repository context.
    system_msg.push_str("\n\n# Git Repository Context\n");
    let git_status_label = git.label();
    let git_url = git.url();
    system_msg.push_str(&format!(
        "REMOTE STATUS: {} | URL: {}\n",
        git_status_label, git_url
    ));

    // Live Snapshots (Status/Diff)
    let root = crate::tools::file_ops::workspace_root();
    if let Some(status_snapshot) = crate::agent::git_context::read_git_status(&root) {
        system_msg.push_str("\nGit status snapshot:\n");
        system_msg.push_str(&status_snapshot);
        system_msg.push_str("\n");
    }

    if let Some(diff_snapshot) = crate::agent::git_context::read_git_diff(&root, 2000) {
        system_msg.push_str("\nGit diff snapshot:\n");
        system_msg.push_str(&diff_snapshot);
        system_msg.push_str("\n");
    }

    if git_status_label == "NONE" {
        system_msg.push_str("\nONBOARDING: You noticed no remote is configured. Offer to help the user set up a remote (e.g. GitHub) if they haven't already.\n");
    } else if git_status_label == "BEHIND" {
        system_msg.push_str("\nSYNC: Local is behind remote. Suggest a pull if appropriate.\n");
    }

    // NOTE: Instruction files (CLAUDE.md, HEMATITE.md, etc.) are already injected
    // by InferenceEngine::build_system_prompt() via load_instruction_files().
    // Injecting them again here would double the token cost (~4K wasted per turn).

    if hints.is_empty() {
        return system_msg;
    }
    system_msg.push_str("\n\n# Formatting Corrections\n");
    system_msg.push_str("You previously failed formatting checks on these files. Ensure your whitespace/indentation perfectly matches the original file exactly on your next attempt:\n");
    for hint in hints {
        system_msg.push_str(&format!("- {}\n", hint));
    }
    system_msg
}

fn route_model<'a>(
    user_input: &str,
    fast_model: Option<&'a str>,
    think_model: Option<&'a str>,
) -> Option<&'a str> {
    let text = user_input.to_lowercase();
    let is_think = text.contains("refactor")
        || text.contains("rewrite")
        || text.contains("implement")
        || text.contains("create")
        || text.contains("fix")
        || text.contains("debug");
    let is_fast = text.contains("what")
        || text.contains("show")
        || text.contains("find")
        || text.contains("list")
        || text.contains("status");

    if is_think && think_model.is_some() {
        return think_model;
    } else if is_fast && fast_model.is_some() {
        return fast_model;
    }
    None
}

fn is_parallel_safe(name: &str) -> bool {
    matches!(
        name,
        "read_file"
            | "inspect_lines"
            | "list_files"
            | "grep_files"
            | "map_project"
            | "trace_runtime_flow"
            | "describe_toolchain"
            | "lsp_definitions"
            | "lsp_references"
            | "lsp_hover"
            | "vision_analyze"
            | "manage_tasks"
            | "research_web"
            | "fetch_docs"
    )
}
