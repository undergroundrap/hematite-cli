use crate::agent::architecture_summary::{
    build_architecture_overview_answer, prune_architecture_trace_batch,
    prune_authoritative_tool_batch, prune_read_only_context_bloat_batch,
    prune_redirected_shell_batch, summarize_runtime_trace_output,
};
use crate::agent::direct_answers::{
    build_about_answer, build_architect_session_reset_plan, build_authorization_policy_answer,
    build_gemma_native_answer, build_gemma_native_settings_answer, build_identity_answer,
    build_language_capability_answer, build_mcp_lifecycle_answer, build_product_surface_answer,
    build_reasoning_split_answer, build_recovery_recipes_answer, build_session_memory_answer,
    build_session_reset_semantics_answer, build_tool_classes_answer,
    build_tool_registry_ownership_answer, build_unsafe_workflow_pressure_answer,
    build_verify_profiles_answer, build_workflow_modes_answer,
};
use crate::agent::inference::InferenceEngine;
use crate::agent::policy::{
    action_target_path, docs_edit_without_explicit_request, is_destructive_tool,
    is_mcp_mutating_tool, is_mcp_workspace_read_tool, is_sovereign_path_request,
    normalize_workspace_path,
};
use crate::agent::recovery_recipes::{
    attempt_recovery, plan_recovery, preview_recovery_decision, RecoveryContext, RecoveryDecision,
    RecoveryPlan, RecoveryScenario, RecoveryStep,
};
use crate::agent::routing::{
    all_host_inspection_topics, classify_query_intent, is_capability_probe_tool,
    is_scaffold_request, looks_like_mutation_request, needs_computation_sandbox,
    preferred_host_inspection_topic, preferred_maintainer_workflow, preferred_workspace_workflow,
    DirectAnswerKind, QueryIntentClass,
};
use crate::agent::tool_registry::dispatch_builtin_tool;
use crate::agent::types::{
    ChatMessage, InferenceEvent, MessageContent, OperatorCheckpointState, ProviderRuntimeState,
    ToolCallFn, ToolDefinition, ToolFunction,
};
// SystemPromptBuilder is no longer used — InferenceEngine::build_system_prompt() is canonical.
use crate::agent::compaction::{self, CompactionConfig};
use crate::ui::gpu_monitor::GpuState;

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
// -- Session persistence -------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct UserTurn {
    pub text: String,
    pub attached_document: Option<AttachedDocument>,
    pub attached_image: Option<AttachedImage>,
}

#[derive(Clone, Debug)]
pub struct AttachedDocument {
    pub name: String,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct AttachedImage {
    pub name: String,
    pub path: String,
}

impl UserTurn {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            attached_document: None,
            attached_image: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedSession {
    running_summary: Option<String>,
    #[serde(default)]
    session_memory: crate::agent::compaction::SessionMemory,
    /// Last user message from the previous session — shown as resume hint on startup.
    #[serde(default)]
    last_goal: Option<String>,
    /// Number of real inference turns completed in the previous session.
    #[serde(default)]
    turn_count: u32,
}

impl Default for SavedSession {
    fn default() -> Self {
        Self {
            running_summary: None,
            session_memory: crate::agent::compaction::SessionMemory::default(),
            last_goal: None,
            turn_count: 0,
        }
    }
}

/// Snapshot of the previous session, surfaced on startup when a workspace is
/// resumed after a restart or crash.
pub struct CheckpointResume {
    pub last_goal: String,
    pub turn_count: u32,
    pub working_files: Vec<String>,
    pub last_verify_ok: Option<bool>,
}

/// Load the prior-session checkpoint from `.hematite/session.json`.
/// Returns `None` when there is no prior session or it has no real turns.
pub fn load_checkpoint() -> Option<CheckpointResume> {
    let path = session_path();
    let data = std::fs::read_to_string(&path).ok()?;
    let saved: SavedSession = serde_json::from_str(&data).ok()?;
    let goal = saved.last_goal.filter(|g| !g.trim().is_empty())?;
    if saved.turn_count == 0 {
        return None;
    }
    let mut working_files: Vec<String> = saved
        .session_memory
        .working_set
        .into_iter()
        .take(4)
        .collect();
    working_files.sort();
    let last_verify_ok = saved.session_memory.last_verification.map(|v| v.successful);
    Some(CheckpointResume {
        last_goal: goal,
        turn_count: saved.turn_count,
        working_files,
        last_verify_ok,
    })
}

#[derive(Default)]
struct ActionGroundingState {
    turn_index: u64,
    observed_paths: std::collections::HashMap<String, u64>,
    inspected_paths: std::collections::HashMap<String, u64>,
    last_verify_build_turn: Option<u64>,
    last_verify_build_ok: bool,
    last_failed_build_paths: Vec<String>,
    code_changed_since_verify: bool,
    /// Track topics redirected from shell to inspect_host in the current turn to break loops.
    redirected_host_inspection_topics: std::collections::HashMap<String, u64>,
}

struct PlanExecutionGuard {
    flag: Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for PlanExecutionGuard {
    fn drop(&mut self) {
        self.flag.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

struct PlanExecutionPassGuard {
    depth: Arc<std::sync::atomic::AtomicUsize>,
}

impl Drop for PlanExecutionPassGuard {
    fn drop(&mut self) {
        self.depth.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkflowMode {
    #[default]
    Auto,
    Ask,
    Code,
    Architect,
    ReadOnly,
    /// Clean conversational mode — lighter prompt, no coding agent scaffolding,
    /// tools available but not pushed. Vein RAG still runs for context.
    Chat,
    /// Teacher/guide mode — inspect the real machine state first, then walk the user
    /// through the admin/config task as a grounded, numbered tutorial. Never executes
    /// write operations itself; instructs the user to perform them manually.
    Teach,
}

impl WorkflowMode {
    fn label(self) -> &'static str {
        match self {
            WorkflowMode::Auto => "AUTO",
            WorkflowMode::Ask => "ASK",
            WorkflowMode::Code => "CODE",
            WorkflowMode::Architect => "ARCHITECT",
            WorkflowMode::ReadOnly => "READ-ONLY",
            WorkflowMode::Chat => "CHAT",
            WorkflowMode::Teach => "TEACH",
        }
    }

    fn is_read_only(self) -> bool {
        matches!(
            self,
            WorkflowMode::Ask
                | WorkflowMode::Architect
                | WorkflowMode::ReadOnly
                | WorkflowMode::Teach
        )
    }

    pub(crate) fn is_chat(self) -> bool {
        matches!(self, WorkflowMode::Chat)
    }
}

fn session_path() -> std::path::PathBuf {
    if let Ok(overridden) = std::env::var("HEMATITE_SESSION_PATH") {
        return std::path::PathBuf::from(overridden);
    }
    crate::tools::file_ops::hematite_dir().join("session.json")
}

fn load_session_data() -> SavedSession {
    let path = session_path();
    if !path.exists() {
        let mut saved = SavedSession::default();
        if let Some(plan) = crate::tools::plan::load_plan_handoff() {
            saved.session_memory.current_plan = Some(plan);
        }
        return saved;
    }
    let data = std::fs::read_to_string(&path);
    let saved = data
        .ok()
        .and_then(|d| serde_json::from_str::<SavedSession>(&d).ok())
        .unwrap_or_default();

    let mut saved = saved;
    if saved
        .session_memory
        .current_plan
        .as_ref()
        .map(|plan| plan.has_signal())
        .unwrap_or(false)
        == false
    {
        if let Some(plan) = crate::tools::plan::load_plan_handoff() {
            saved.session_memory.current_plan = Some(plan);
        }
    }
    saved
}

#[derive(Clone)]
struct SovereignTeleportHandoff {
    root: String,
    plan: crate::tools::plan::PlanHandoff,
}

fn reset_task_files() {
    let hdir = crate::tools::file_ops::hematite_dir();
    let root = crate::tools::file_ops::workspace_root();
    let _ = std::fs::remove_file(hdir.join("TASK.md"));
    let _ = std::fs::remove_file(hdir.join("PLAN.md"));
    let _ = std::fs::remove_file(hdir.join("WALKTHROUGH.md"));
    let _ = std::fs::remove_file(root.join(".github").join("WALKTHROUGH.md"));
    let _ = std::fs::write(hdir.join("TASK.md"), "");
    let _ = std::fs::write(hdir.join("PLAN.md"), "");
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TaskChecklistProgress {
    total: usize,
    completed: usize,
    remaining: usize,
}

impl TaskChecklistProgress {
    fn has_open_items(self) -> bool {
        self.remaining > 0
    }
}

fn task_status_path() -> std::path::PathBuf {
    crate::tools::file_ops::hematite_dir().join("TASK.md")
}

fn parse_task_checklist_progress(input: &str) -> TaskChecklistProgress {
    let mut progress = TaskChecklistProgress::default();

    for line in input.lines() {
        let trimmed = line.trim_start();
        let candidate = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
            .unwrap_or(trimmed);

        let state = if candidate.starts_with("[x]") || candidate.starts_with("[X]") {
            Some(true)
        } else if candidate.starts_with("[ ]") {
            Some(false)
        } else {
            None
        };

        if let Some(completed) = state {
            progress.total += 1;
            if completed {
                progress.completed += 1;
            }
        }
    }

    progress.remaining = progress.total.saturating_sub(progress.completed);
    progress
}

fn read_task_checklist_progress() -> Option<TaskChecklistProgress> {
    let content = std::fs::read_to_string(task_status_path()).ok()?;
    Some(parse_task_checklist_progress(&content))
}

fn plan_execution_sidecar_paths() -> Vec<String> {
    let hdir = crate::tools::file_ops::hematite_dir();
    ["TASK.md", "PLAN.md", "WALKTHROUGH.md"]
        .iter()
        .map(|name| normalize_workspace_path(hdir.join(name).to_string_lossy().as_ref()))
        .collect()
}

fn merge_plan_allowed_paths(target_files: &[String]) -> Vec<String> {
    let mut allowed = std::collections::BTreeSet::new();
    for path in target_files {
        allowed.insert(normalize_workspace_path(path));
    }
    for path in plan_execution_sidecar_paths() {
        allowed.insert(path);
    }
    allowed.into_iter().collect()
}

fn should_continue_plan_execution(
    current_pass: usize,
    before: Option<TaskChecklistProgress>,
    after: Option<TaskChecklistProgress>,
    mutated_paths: &std::collections::BTreeSet<String>,
) -> bool {
    const MAX_AUTONOMOUS_PLAN_PASSES: usize = 6;

    if current_pass >= MAX_AUTONOMOUS_PLAN_PASSES {
        return false;
    }

    let Some(after) = after else {
        return false;
    };
    if !after.has_open_items() {
        return false;
    }

    match before {
        Some(before) if before.total > 0 => {
            after.completed > before.completed || after.remaining < before.remaining
        }
        Some(before) => after.total > before.total || !mutated_paths.is_empty(),
        None => !mutated_paths.is_empty(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AutoVerificationOutcome {
    ok: bool,
    summary: String,
}

fn should_run_website_validation(
    contract: Option<&crate::agent::workspace_profile::RuntimeContract>,
    mutated_paths: &std::collections::BTreeSet<String>,
) -> bool {
    let Some(contract) = contract else {
        return false;
    };
    if contract.loop_family != "website" {
        return false;
    }
    if mutated_paths.is_empty() {
        return true;
    }
    mutated_paths.iter().any(|path| {
        let normalized = path.replace('\\', "/").to_ascii_lowercase();
        normalized.ends_with(".html")
            || normalized.ends_with(".css")
            || normalized.ends_with(".js")
            || normalized.ends_with(".jsx")
            || normalized.ends_with(".ts")
            || normalized.ends_with(".tsx")
            || normalized.ends_with(".mdx")
            || normalized.ends_with(".vue")
            || normalized.ends_with(".svelte")
            || normalized.ends_with("package.json")
            || normalized.starts_with("public/")
            || normalized.starts_with("static/")
            || normalized.starts_with("pages/")
            || normalized.starts_with("app/")
            || normalized.starts_with("src/pages/")
            || normalized.starts_with("src/app/")
    })
}

fn is_repeat_guard_exempt_tool_call(tool_name: &str, args: &Value) -> bool {
    if matches!(tool_name, "verify_build" | "git_commit" | "git_push") {
        return true;
    }
    tool_name == "run_workspace_workflow"
        && matches!(
            args.get("workflow").and_then(|value| value.as_str()),
            Some("website_probe" | "website_validate" | "website_status")
        )
}

fn should_run_contract_verification_workflow(
    contract: Option<&crate::agent::workspace_profile::RuntimeContract>,
    workflow: &str,
    mutated_paths: &std::collections::BTreeSet<String>,
) -> bool {
    // Standard workflows always run if listed (they are already 'cheap').
    if matches!(workflow, "build" | "test" | "lint") {
        return true;
    }

    match workflow {
        "website_validate" => should_run_website_validation(contract, mutated_paths),
        _ => true,
    }
}

fn build_continue_plan_execution_prompt(progress: TaskChecklistProgress) -> String {
    format!(
        "Continue implementing the current plan. Read `.hematite/TASK.md` first, focus on the next unchecked items, and keep working until the checklist is complete or you hit one concrete blocker. There are currently {} unchecked checklist item(s) remaining.",
        progress.remaining
    )
}

fn purge_persistent_memory() {
    let mem_dir = crate::tools::file_ops::hematite_dir().join("memories");
    if mem_dir.exists() {
        let _ = std::fs::remove_dir_all(&mem_dir);
        let _ = std::fs::create_dir_all(&mem_dir);
    }

    let log_dir = crate::tools::file_ops::hematite_dir().join("logs");
    if log_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&log_dir) {
            for entry in entries.flatten() {
                let _ = std::fs::write(entry.path(), "");
            }
        }
    }
}

fn apply_turn_attachments(user_turn: &UserTurn, prompt: &str) -> String {
    let mut out = prompt.trim().to_string();
    if let Some(doc) = user_turn.attached_document.as_ref() {
        out = format!(
            "[Attached document: {}]\n\n{}\n\n---\n\n{}",
            doc.name, doc.content, out
        );
    }
    if let Some(image) = user_turn.attached_image.as_ref() {
        out = if out.is_empty() {
            format!("[Attached image: {}]", image.name)
        } else {
            format!("[Attached image: {}]\n\n{}", image.name, out)
        };
    }
    // Auto-inject @file mentions — parse @<path> tokens and prepend file content
    // so the model can edit immediately without a read_file round-trip.
    out = inject_at_file_mentions(&out);
    out
}

/// Parse `@<path>` tokens from the user prompt, read each file, and prepend its
/// content as inline context. Tokens that don't resolve to readable files are
/// left as-is so the model can still call read_file if needed.
fn inject_at_file_mentions(prompt: &str) -> String {
    // Quick bail — no @ present
    if !prompt.contains('@') {
        return prompt.to_string();
    }
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => return prompt.to_string(),
    };

    let mut injected = Vec::new();
    // Split on whitespace+punctuation boundaries but keep the original prompt intact
    for token in prompt.split_whitespace() {
        let raw = token.trim_start_matches('@');
        if !token.starts_with('@') || raw.is_empty() {
            continue;
        }
        // Strip trailing punctuation that isn't part of a path
        let path_str =
            raw.trim_end_matches(|c: char| matches!(c, ',' | '.' | ':' | ';' | '!' | '?'));
        if path_str.is_empty() {
            continue;
        }
        let candidate = cwd.join(path_str);
        if candidate.is_file() {
            match std::fs::read_to_string(&candidate) {
                Ok(content) if !content.is_empty() => {
                    // Cap at 32 KB so a huge file doesn't blow the context
                    const CAP: usize = 32 * 1024;
                    let body = if content.len() > CAP {
                        format!(
                            "{}\n... [truncated — file is large, use read_file for the rest]",
                            &content[..CAP]
                        )
                    } else {
                        content
                    };
                    injected.push(format!("[File: {}]\n```\n{}\n```", path_str, body.trim()));
                }
                _ => {}
            }
        }
    }

    if injected.is_empty() {
        return prompt.to_string();
    }
    // Prepend injected file blocks before the user message
    format!("{}\n\n---\n\n{}", injected.join("\n\n"), prompt)
}

/// After a successful edit on `path`, replace large stale read_file / inspect_lines results
/// for that same path in history with a compact stub. The file just changed so old content
/// is both wrong and wasteful — keeping it burns context the model needs for the next edit.
///
/// We leave the two most recent messages untouched so any read that was part of the current
/// edit cycle stays visible (the model may still reference it for adjacent edits).
fn compact_stale_reads(history: &mut Vec<ChatMessage>, path: &str) {
    const MIN_SIZE_TO_COMPACT: usize = 800;
    let stub = "[prior read_file content compacted — file was edited; use read_file to reload]";
    let normalized = normalize_workspace_path(path);
    let safe_tail = history.len().saturating_sub(2);
    for msg in history[..safe_tail].iter_mut() {
        if msg.role != "tool" {
            continue;
        }
        let is_read_tool = matches!(
            msg.name.as_deref(),
            Some("read_file") | Some("inspect_lines")
        );
        if !is_read_tool {
            continue;
        }
        let content = match &msg.content {
            crate::agent::inference::MessageContent::Text(s) => s.clone(),
            _ => continue,
        };
        if content.len() < MIN_SIZE_TO_COMPACT {
            continue;
        }
        // Match on normalized path or the raw path appearing anywhere in the content
        if content.contains(&normalized) || content.contains(path) {
            msg.content = crate::agent::inference::MessageContent::Text(stub.to_string());
        }
    }
}

/// Read up to `max_lines` lines from a file with line numbers, for edit-fail auto-recovery.
/// Returns a placeholder string if the file cannot be read.
fn read_file_preview_for_retry(path: &str, max_lines: usize) -> String {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c.replace("\r\n", "\n"),
        Err(e) => return format!("[could not read {path}: {e}]"),
    };
    let total = content.lines().count();
    let lines: String = content
        .lines()
        .enumerate()
        .take(max_lines)
        .map(|(i, line)| format!("{:>4}  {}", i + 1, line))
        .collect::<Vec<_>>()
        .join("\n");
    if total > max_lines {
        format!(
            "{lines}\n... [{} more lines — use inspect_lines to see the rest]",
            total - max_lines
        )
    } else {
        lines
    }
}

fn transcript_user_turn_text(user_turn: &UserTurn, prompt: &str) -> String {
    let mut prefixes = Vec::new();
    if let Some(doc) = user_turn.attached_document.as_ref() {
        prefixes.push(format!("[Attached document: {}]", doc.name));
    }
    if let Some(image) = user_turn.attached_image.as_ref() {
        prefixes.push(format!("[Attached image: {}]", image.name));
    }
    if prefixes.is_empty() {
        prompt.to_string()
    } else if prompt.trim().is_empty() {
        prefixes.join("\n")
    } else {
        format!("{}\n{}", prefixes.join("\n"), prompt)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeFailureClass {
    ContextWindow,
    ProviderDegraded,
    ToolArgMalformed,
    ToolPolicyBlocked,
    ToolLoop,
    VerificationFailed,
    EmptyModelResponse,
    Unknown,
}

impl RuntimeFailureClass {
    fn tag(self) -> &'static str {
        match self {
            RuntimeFailureClass::ContextWindow => "context_window",
            RuntimeFailureClass::ProviderDegraded => "provider_degraded",
            RuntimeFailureClass::ToolArgMalformed => "tool_arg_malformed",
            RuntimeFailureClass::ToolPolicyBlocked => "tool_policy_blocked",
            RuntimeFailureClass::ToolLoop => "tool_loop",
            RuntimeFailureClass::VerificationFailed => "verification_failed",
            RuntimeFailureClass::EmptyModelResponse => "empty_model_response",
            RuntimeFailureClass::Unknown => "unknown",
        }
    }

    fn operator_guidance(self) -> &'static str {
        match self {
            RuntimeFailureClass::ContextWindow => {
                "Narrow the request, compact the session, or preserve grounded tool output instead of restyling it. If LM Studio reports a smaller live n_ctx than Hematite expected, reload or re-detect the model budget before retrying."
            }
            RuntimeFailureClass::ProviderDegraded => {
                "Retry once automatically, then narrow the turn or restart LM Studio if it persists."
            }
            RuntimeFailureClass::ToolArgMalformed => {
                "Retry with repaired or narrower tool arguments instead of repeating the same malformed call."
            }
            RuntimeFailureClass::ToolPolicyBlocked => {
                "Stay inside the allowed workflow or switch modes before retrying."
            }
            RuntimeFailureClass::ToolLoop => {
                "Stop repeating the same failing tool pattern and switch to a narrower recovery step."
            }
            RuntimeFailureClass::VerificationFailed => {
                "Fix the build or test failure before treating the task as complete."
            }
            RuntimeFailureClass::EmptyModelResponse => {
                "Retry once automatically, then narrow the turn or restart LM Studio if the model keeps returning nothing."
            }
            RuntimeFailureClass::Unknown => {
                "Inspect the latest grounded tool results or provider status before retrying."
            }
        }
    }
}

fn classify_runtime_failure(detail: &str) -> RuntimeFailureClass {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("context_window_blocked")
        || lower.contains("context ceiling reached")
        || lower.contains("exceeds the")
        || ((lower.contains("n_keep") && lower.contains("n_ctx"))
            || lower.contains("context length")
            || lower.contains("keep from the initial prompt")
            || lower.contains("prompt is greater than the context length"))
    {
        RuntimeFailureClass::ContextWindow
    } else if lower.contains("empty response from model")
        || lower.contains("model returned an empty response")
    {
        RuntimeFailureClass::EmptyModelResponse
    } else if lower.contains("lm studio unreachable")
        || lower.contains("lm studio error")
        || lower.contains("request failed")
        || lower.contains("response parse error")
        || lower.contains("provider degraded")
    {
        RuntimeFailureClass::ProviderDegraded
    } else if lower.contains("missing required argument")
        || lower.contains("json repair failed")
        || lower.contains("invalid pattern")
        || lower.contains("invalid line range")
    {
        RuntimeFailureClass::ToolArgMalformed
    } else if lower.contains("action blocked:")
        || lower.contains("access denied")
        || lower.contains("declined by user")
    {
        RuntimeFailureClass::ToolPolicyBlocked
    } else if lower.contains("too many consecutive tool errors")
        || lower.contains("repeated tool failures")
        || lower.contains("stuck in a loop")
    {
        RuntimeFailureClass::ToolLoop
    } else if lower.contains("build failed")
        || lower.contains("verification failed")
        || lower.contains("verify_build")
    {
        RuntimeFailureClass::VerificationFailed
    } else {
        RuntimeFailureClass::Unknown
    }
}

fn format_runtime_failure(class: RuntimeFailureClass, detail: &str) -> String {
    let trimmed = detail.trim();
    if trimmed.starts_with("[failure:") {
        return trimmed.to_string();
    }
    format!(
        "[failure:{}] {} Detail: {}",
        class.tag(),
        class.operator_guidance(),
        trimmed
    )
}

fn is_explicit_web_search_request(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    [
        "google ",
        "search for ",
        "search the web",
        "web search",
        "look up ",
        "lookup ",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn extract_explicit_web_search_query(input: &str) -> Option<String> {
    let lower = input.to_ascii_lowercase();
    let mut query_tail = None;
    for needle in [
        "search for ",
        "google ",
        "look up ",
        "lookup ",
        "search the web for ",
        "search the web ",
        "web search for ",
        "web search ",
    ] {
        if let Some(idx) = lower.find(needle) {
            let rest = input[idx + needle.len()..].trim();
            if !rest.is_empty() {
                query_tail = Some(rest);
                break;
            }
        }
    }

    let mut query = query_tail?;
    let lower_query = query.to_ascii_lowercase();
    let mut cut = query.len();
    for marker in [
        " and then ",
        " then ",
        " and make ",
        " then make ",
        " and create ",
        " then create ",
        " and build ",
        " then build ",
        " and scaffold ",
        " then scaffold ",
        " and turn ",
        " then turn ",
    ] {
        if let Some(idx) = lower_query.find(marker) {
            cut = cut.min(idx);
        }
    }
    query = query[..cut].trim();
    let query = query
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '`' | ',' | '.' | ':' | ';'))
        .trim();
    if query.is_empty() {
        None
    } else {
        Some(query.to_string())
    }
}

fn build_research_provider_fallback(results: &str) -> String {
    format!(
        "Local web search succeeded, but the model runtime degraded before it could synthesize a final answer. \
Surfacing the grounded search results directly.\n\n{}",
        cap_output(results, 2400)
    )
}

fn provider_state_for_runtime_failure(class: RuntimeFailureClass) -> Option<ProviderRuntimeState> {
    match class {
        RuntimeFailureClass::ContextWindow => Some(ProviderRuntimeState::ContextWindow),
        RuntimeFailureClass::ProviderDegraded => Some(ProviderRuntimeState::Degraded),
        RuntimeFailureClass::EmptyModelResponse => Some(ProviderRuntimeState::EmptyResponse),
        _ => None,
    }
}

fn checkpoint_state_for_runtime_failure(
    class: RuntimeFailureClass,
) -> Option<OperatorCheckpointState> {
    match class {
        RuntimeFailureClass::ContextWindow => Some(OperatorCheckpointState::BlockedContextWindow),
        RuntimeFailureClass::ToolPolicyBlocked => Some(OperatorCheckpointState::BlockedPolicy),
        RuntimeFailureClass::ToolLoop => Some(OperatorCheckpointState::BlockedToolLoop),
        RuntimeFailureClass::VerificationFailed => {
            Some(OperatorCheckpointState::BlockedVerification)
        }
        _ => None,
    }
}

fn compact_runtime_recovery_summary(class: RuntimeFailureClass) -> &'static str {
    match class {
        RuntimeFailureClass::ProviderDegraded => {
            "LM Studio degraded during the turn; retrying once before surfacing a failure."
        }
        RuntimeFailureClass::EmptyModelResponse => {
            "The model returned an empty reply; retrying once before surfacing a failure."
        }
        _ => "Runtime recovery in progress.",
    }
}

fn checkpoint_summary_for_runtime_failure(class: RuntimeFailureClass) -> &'static str {
    match class {
        RuntimeFailureClass::ContextWindow => "Provider context ceiling confirmed.",
        RuntimeFailureClass::ToolPolicyBlocked => "Policy blocked the current action.",
        RuntimeFailureClass::ToolLoop => "Repeated failing tool pattern stopped.",
        RuntimeFailureClass::VerificationFailed => "Verification failed; fix before continuing.",
        _ => "Operator checkpoint updated.",
    }
}

fn compact_runtime_failure_summary(class: RuntimeFailureClass) -> &'static str {
    match class {
        RuntimeFailureClass::ContextWindow => "LM context ceiling hit.",
        RuntimeFailureClass::ProviderDegraded => {
            "LM Studio degraded and did not recover cleanly; operator action is now required."
        }
        RuntimeFailureClass::EmptyModelResponse => {
            "LM Studio returned an empty reply after recovery; operator action is now required."
        }
        RuntimeFailureClass::ToolLoop => {
            "Repeated failing tool pattern detected; Hematite stopped the loop."
        }
        _ => "Runtime failure surfaced to the operator.",
    }
}

fn should_retry_runtime_failure(class: RuntimeFailureClass) -> bool {
    matches!(
        class,
        RuntimeFailureClass::ProviderDegraded | RuntimeFailureClass::EmptyModelResponse
    )
}

fn recovery_scenario_for_runtime_failure(class: RuntimeFailureClass) -> Option<RecoveryScenario> {
    match class {
        RuntimeFailureClass::ContextWindow => Some(RecoveryScenario::ContextWindow),
        RuntimeFailureClass::ProviderDegraded => Some(RecoveryScenario::ProviderDegraded),
        RuntimeFailureClass::EmptyModelResponse => Some(RecoveryScenario::EmptyModelResponse),
        RuntimeFailureClass::ToolPolicyBlocked => Some(RecoveryScenario::McpWorkspaceReadBlocked),
        RuntimeFailureClass::ToolLoop => Some(RecoveryScenario::ToolLoop),
        RuntimeFailureClass::VerificationFailed => Some(RecoveryScenario::VerificationFailed),
        RuntimeFailureClass::ToolArgMalformed | RuntimeFailureClass::Unknown => None,
    }
}

fn compact_recovery_plan_summary(plan: &RecoveryPlan) -> String {
    format!(
        "{} [{}]",
        plan.recipe.scenario.label(),
        plan.recipe.steps_summary()
    )
}

fn compact_recovery_decision_summary(decision: &RecoveryDecision) -> String {
    match decision {
        RecoveryDecision::Attempt(plan) => compact_recovery_plan_summary(plan),
        RecoveryDecision::Escalate {
            recipe,
            attempts_made,
            ..
        } => format!(
            "{} escalated after {} / {} [{}]",
            recipe.scenario.label(),
            attempts_made,
            recipe.max_attempts.max(1),
            recipe.steps_summary()
        ),
    }
}

/// Parse file paths from cargo/compiler error output.
/// Handles lines like `  --> src/foo/bar.rs:34:12` and `error: could not compile`.
fn parse_failing_paths_from_build_output(output: &str) -> Vec<String> {
    let root = crate::tools::file_ops::workspace_root();
    let mut paths: Vec<String> = output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            // Cargo error location: "--> path/to/file.rs:line:col"
            let after_arrow = trimmed.strip_prefix("--> ")?;
            let file_part = after_arrow.split(':').next()?;
            if file_part.is_empty() || file_part.starts_with('<') {
                return None;
            }
            let p = std::path::Path::new(file_part);
            let resolved = if p.is_absolute() {
                p.to_path_buf()
            } else {
                root.join(p)
            };
            Some(resolved.to_string_lossy().replace('\\', "/").to_lowercase())
        })
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

fn build_mode_redirect_answer(mode: WorkflowMode) -> String {
    match mode {
        WorkflowMode::Ask => "Workflow mode ASK is read-only. I can inspect the code, explain what should change, or review the target area, but I will not modify files here. Switch to `/code` to implement the change, or `/auto` to let Hematite choose.".to_string(),
        WorkflowMode::Architect => "Workflow mode ARCHITECT is plan-first. I can inspect the code and design the implementation approach, but I will not mutate files until you explicitly switch to `/code` or ask me to implement.".to_string(),
        WorkflowMode::ReadOnly => "Workflow mode READ-ONLY is a hard no-mutation mode. I can analyze, inspect, and explain, but I will not edit files, run mutating shell commands, or commit changes. Switch to `/code` or `/auto` if you want implementation.".to_string(),
        WorkflowMode::Teach => "Workflow mode TEACH is a guided walkthrough mode. I will inspect the real state of your machine first, then give you a numbered step-by-step tutorial so you can perform the task yourself. I do not execute write operations in TEACH mode — I show you exactly how to do it.".to_string(),
        _ => "Switch to `/code` or `/auto` to allow implementation.".to_string(),
    }
}

fn architect_handoff_contract() -> &'static str {
    "ARCHITECT OUTPUT CONTRACT:\n\
Use a compact implementation handoff, not a process narrative.\n\
Do not say \"the first step\" or describe what you are about to do.\n\
After one or two read-only inspection tools at most, stop and answer.\n\
For runtime wiring, reset behavior, or control-flow questions, prefer `trace_runtime_flow`.\n\
Use these exact ASCII headings and keep each section short:\n\
# Goal\n\
# Target Files\n\
# Ordered Steps\n\
# Verification\n\
# Risks\n\
# Open Questions\n\
Keep the whole handoff concise and implementation-oriented."
}

fn implement_current_plan_prompt() -> &'static str {
    "Implement the current plan."
}

fn scaffold_protocol() -> &'static str {
    "\n\n# SCAFFOLD MODE — PROJECT CREATION PROTOCOL\n\
     The user wants a new project created. Your job is to build it completely, right now, without stopping.\n\
     \n\
     ## Autonomy rules\n\
     - Build every file the project needs in one pass. Do NOT stop after one file and wait.\n\
     - After writing each file, read it back to verify it is complete and not truncated.\n\
     - Check cross-file consistency before finishing.\n\
     - Once the project is coherent, runnable, and verified, STOP.\n\
     - Mandatory Checklist Protocol: Whenever drafting a plan for a project scaffold, you MUST initialize a `.hematite/TASK.md` file with a granular `[ ]` checklist. Update it after every file mutation.\n\
     - If only optional polish remains, present it as optional next steps instead of mutating more files.\n\
     - Ask the user only when blocked by a real product decision, missing requirement, or risky/destructive choice.\n\
     - Only surface results to the user once ALL files exist and the project is immediately runnable.\n\
     - Final delivery must sound like a human engineer closeout: stack chosen, what was built, what was verified, and what remains optional.\n\
     \n\
     ## Infer the stack from context\n\
     If the user gives only a vague request (\"make me a website\", \"build me a tool\"), pick the most\n\
     sensible minimal stack and state your choice before creating files. Do not ask permission — choose and build.\n\
     For scaffold/project-creation turns, do NOT use `run_workspace_workflow` unless the user explicitly asks you to run an existing build, test, lint, package script, or repo command.\n\
     Default choices: website → static HTML+CSS+JS; CLI tool → Rust (clap) if Rust project, Python (argparse/click) otherwise;\n\
     API → FastAPI (Python) or Express (Node); web app with state → React (Vite).\n\
     \n\
     ## Stack file structures\n\
     \n\
     **Static HTML site / landing page:**\n\
     index.html (semantic: header/nav/main/footer, doctype, meta charset/viewport, linked CSS+JS),\n\
     style.css (CSS variables, mobile-first, grid/flexbox, @media breakpoints, hover/focus states),\n\
     script.js (DOMContentLoaded guard, smooth scroll, no console.log left in), README.md\n\
     \n\
     **React (Vite):**\n\
     package.json (scripts: dev/build/preview, deps: react react-dom, devDeps: vite @vitejs/plugin-react),\n\
     vite.config.js, index.html (root div), src/main.jsx, src/App.jsx, src/App.css, src/index.css, .gitignore, README.md\n\
     \n\
     **Next.js (App Router):**\n\
     package.json (next react react-dom, scripts: dev/build/start),\n\
     next.config.js, tsconfig.json, app/layout.tsx, app/page.tsx, app/globals.css, public/.gitkeep, .gitignore, README.md\n\
     \n\
     **Vue 3 (Vite):**\n\
     package.json (vue, vite, @vitejs/plugin-vue),\n\
     vite.config.js, index.html, src/main.js, src/App.vue, src/components/.gitkeep, .gitignore, README.md\n\
     \n\
     **SvelteKit:**\n\
     package.json (@sveltejs/kit, svelte, vite, @sveltejs/adapter-auto),\n\
     svelte.config.js, vite.config.js, src/routes/+page.svelte, src/app.html, static/.gitkeep, .gitignore, README.md\n\
     \n\
     **Express.js API:**\n\
     package.json (express, cors, dotenv; nodemon as devDep; scripts: start/dev),\n\
     src/index.js (listen + middleware), src/routes/index.js, src/middleware/error.js, .env.example, .gitignore, README.md\n\
     \n\
     **FastAPI (Python):**\n\
     requirements.txt (fastapi, uvicorn[standard], pydantic),\n\
     main.py (app = FastAPI(), include_router, uvicorn.run guard),\n\
     app/__init__.py, app/routers/items.py, app/models.py, .gitignore (venv/ __pycache__/ .env), README.md\n\
     \n\
     **Flask (Python):**\n\
     requirements.txt (flask, python-dotenv),\n\
     app.py or app/__init__.py, app/routes.py, templates/base.html, static/style.css, .gitignore, README.md\n\
     \n\
     **Django:**\n\
     requirements.txt, manage.py, project/settings.py, project/urls.py, project/wsgi.py,\n\
     app/models.py, app/views.py, app/urls.py, templates/base.html, .gitignore, README.md\n\
     \n\
     **Python CLI (click or argparse):**\n\
     pyproject.toml (name, version, [project.scripts] entry point) or setup.py,\n\
     src/<name>/__init__.py, src/<name>/cli.py (click group or argparse main), src/<name>/core.py,\n\
     README.md, .gitignore (__pycache__/ dist/ *.egg-info venv/)\n\
     \n\
     **Python package/library:**\n\
     pyproject.toml (PEP 517/518, hatchling or setuptools), src/<name>/__init__.py, src/<name>/core.py,\n\
     tests/__init__.py, tests/test_core.py, README.md, .gitignore\n\
     \n\
     **Rust CLI (clap):**\n\
     Cargo.toml (name, edition=2021, clap with derive feature),\n\
     src/main.rs (Cli struct with #[derive(Parser)], fn main), src/cli.rs (subcommands if needed),\n\
     README.md, .gitignore (target/)\n\
     \n\
     **Rust library:**\n\
     Cargo.toml ([lib], edition=2021), src/lib.rs (pub mod, pub fn, doc comments),\n\
     tests/integration_test.rs, README.md, .gitignore\n\
     \n\
     **Go project / CLI:**\n\
     go.mod (module <name>, go 1.21), main.go (package main, func main),\n\
     cmd/<name>/main.go if CLI, internal/core/core.go for logic,\n\
     README.md, .gitignore (bin/ *.exe)\n\
     \n\
     **C++ project (CMake):**\n\
     CMakeLists.txt (cmake_minimum_required, project, add_executable, set C++17/20),\n\
     src/main.cpp, include/<name>.h, src/<name>.cpp,\n\
     README.md, .gitignore (build/ *.o *.exe CMakeCache.txt)\n\
     \n\
     **Node.js TypeScript API:**\n\
     package.json (express @types/express typescript ts-node nodemon; scripts: build/dev/start),\n\
     tsconfig.json (strict, esModuleInterop, outDir: dist), src/index.ts, src/routes/index.ts,\n\
     .env.example, .gitignore, README.md\n\
     \n\
     ## File quality rules\n\
     - Every file must be complete — no truncation, no placeholder comments like \"add logic here\"\n\
     - package.json: name, version, scripts, all deps explicit\n\
     - HTML: doctype, charset, viewport, title, all linked CSS/JS, semantic structure\n\
     - CSS: consistent class names matching HTML exactly, responsive, variables for colors/spacing\n\
     - .gitignore: cover node_modules/, dist/, .env, __pycache__/, target/, venv/, build/ as appropriate\n\
     - Rust Cargo.toml: edition = \"2021\", all used crates declared\n\
     - Go go.mod: module path and go version declared\n\
     - C++ CMakeLists.txt: cmake version, project name, standard, all source files listed\n\
     \n\
     ## After scaffolding — required wrap-up\n\
     1. List every file created with a one-line description of what it does\n\
     2. Give the exact command(s) to install dependencies and run the project\n\
     3. Tell the user they can type `/cd <project-folder>` to teleport into the new project\n\
     4. Ask what they'd like to work on next — offer 2-3 specific suggestions relevant to the stack\n\
        (e.g. \"Want me to add routing? Set up authentication? Add a dark mode toggle? Or should we improve the design?\")\n\
     5. Stay engaged — you are their coding partner, not a one-shot file generator\n"
}

fn looks_like_static_site_request(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    (lower.contains("website") || lower.contains("landing page") || lower.contains("web page"))
        && (lower.contains("html")
            || lower.contains("css")
            || lower.contains("javascript")
            || lower.contains("js")
            || !lower.contains("react"))
}

fn sanitize_project_folder_name(raw: &str) -> String {
    let trimmed = raw
        .trim()
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '`' | '.' | ',' | ':' | ';'));
    let mut out = String::new();
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ' ') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let cleaned = out.trim().replace(' ', "_");
    if cleaned.is_empty() {
        "hematite_project".to_string()
    } else {
        cleaned
    }
}

fn extract_named_folder(lower: &str) -> Option<String> {
    for marker in [" named ", " called "] {
        if let Some(idx) = lower.find(marker) {
            let rest = &lower[idx + marker.len()..];
            let name = rest
                .split(|c: char| {
                    c.is_whitespace() || matches!(c, ',' | '.' | ':' | ';' | '!' | '?')
                })
                .next()
                .unwrap_or("")
                .trim();
            if !name.is_empty() {
                return Some(sanitize_project_folder_name(name));
            }
        }
    }
    None
}

fn extract_sovereign_scaffold_root(user_input: &str) -> Option<std::path::PathBuf> {
    let lower = user_input.to_ascii_lowercase();
    let folder_name = extract_named_folder(&lower)?;

    let base = if lower.contains("desktop") {
        dirs::desktop_dir()
    } else if lower.contains("download") {
        dirs::download_dir()
    } else if lower.contains("document") || lower.contains("docs") {
        dirs::document_dir()
    } else {
        None
    }?;

    Some(base.join(folder_name))
}

fn default_sovereign_scaffold_targets(user_input: &str) -> std::collections::BTreeSet<String> {
    let mut targets = std::collections::BTreeSet::new();
    if looks_like_static_site_request(user_input) {
        targets.insert("index.html".to_string());
        targets.insert("style.css".to_string());
        targets.insert("script.js".to_string());
    }
    targets
}

fn seed_sovereign_scaffold_files(
    root: &std::path::Path,
    targets: &std::collections::BTreeSet<String>,
) -> Result<(), String> {
    for relative in targets {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create scaffold parent directory: {e}"))?;
        }
        if !path.exists() {
            std::fs::write(&path, "")
                .map_err(|e| format!("Failed to seed scaffold file {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

fn write_sovereign_handoff_markdown(
    root: &std::path::Path,
    user_input: &str,
    plan: &crate::tools::plan::PlanHandoff,
) -> Result<(), String> {
    let handoff_path = root.join("HEMATITE_HANDOFF.md");
    let content = format!(
        "# Hematite Handoff\n\n\
         Original request:\n\
         - {}\n\n\
         This project root was pre-created by Hematite before teleport.\n\
         The next session should resume from the local `.hematite/PLAN.md` handoff and continue implementation here.\n\n\
         ## Planned Target Files\n{}\n\
         ## Verification\n- {}\n",
        user_input.trim(),
        if plan.target_files.is_empty() {
            "- project files to be created in the resumed session\n".to_string()
        } else {
            plan.target_files
                .iter()
                .map(|path| format!("- {path}\n"))
                .collect::<String>()
        },
        plan.verification.trim()
    );
    std::fs::write(&handoff_path, content)
        .map_err(|e| format!("Failed to write handoff markdown: {e}"))
}

fn build_sovereign_scaffold_handoff(
    user_input: &str,
    target_files: &std::collections::BTreeSet<String>,
) -> crate::tools::plan::PlanHandoff {
    let mut steps = vec![
        "Read the scaffolded files in this root before changing them so the resumed session stays grounded in the actual generated content.".to_string(),
        "Finish the implementation inside this sovereign project root only; do not reason from the old workspace or unrelated ./src context.".to_string(),
        "Keep the file set coherent instead of thrashing cosmetics; once the project is runnable or internally consistent, stop and summarize like a human engineer.".to_string(),
    ];
    if let Some(query) = extract_explicit_web_search_query(user_input) {
        steps.insert(
            1,
            format!(
                "Use `research_web` first to gather current context about `{query}` before drafting content or copy for this new project root."
            ),
        );
    }
    let verification = if looks_like_static_site_request(user_input) {
        steps.insert(
            1,
            "Make sure index.html, style.css, and script.js stay linked correctly and that the layout remains responsive on desktop and mobile.".to_string(),
        );
        "Open and inspect the generated front-end files in this root, confirm cross-file links are valid, and verify the page is coherent and responsive without using repo-root workflows.".to_string()
    } else {
        "Use only project-appropriate verification scoped to this root. Avoid unrelated repo workflows; verify the generated files are internally consistent before stopping.".to_string()
    };

    crate::tools::plan::PlanHandoff {
        goal: format!(
            "Continue the sovereign scaffold task in this new project root: {}",
            user_input.trim()
        ),
        target_files: target_files.iter().cloned().collect(),
        ordered_steps: steps,
        verification,
        risks: vec![
            "Do not drift back into the originating workspace or unrelated ./src context."
                .to_string(),
            "Avoid endless UI polish loops once the generated project is already coherent."
                .to_string(),
        ],
        open_questions: Vec::new(),
    }
}

fn architect_handoff_operator_note(plan: &crate::tools::plan::PlanHandoff) -> String {
    format!(
        "Implementation handoff saved to `.hematite/PLAN.md`.\nNext step: run `/implement-plan` to execute it in `/code`, or use `/code {}` directly.\nPlan: {}",
        implement_current_plan_prompt().to_ascii_lowercase(),
        plan.summary_line()
    )
}

fn is_current_plan_execution_request(user_input: &str) -> bool {
    let lower = user_input.trim().to_ascii_lowercase();
    lower == "/implement-plan"
        || lower == implement_current_plan_prompt().to_ascii_lowercase()
        || lower
            == implement_current_plan_prompt()
                .trim_end_matches('.')
                .to_ascii_lowercase()
        || lower.contains("implement the current plan")
}

fn is_plan_scoped_tool(name: &str) -> bool {
    crate::agent::inference::tool_metadata_for_name(name).plan_scope
}

fn is_current_plan_irrelevant_tool(name: &str) -> bool {
    !crate::agent::inference::tool_metadata_for_name(name).plan_scope
}

fn is_non_mutating_plan_step_tool(name: &str) -> bool {
    let metadata = crate::agent::inference::tool_metadata_for_name(name);
    metadata.plan_scope && !metadata.mutates_workspace
}

fn plan_handoff_mentions_tool(plan: &crate::tools::plan::PlanHandoff, tool_name: &str) -> bool {
    let needle = tool_name.to_ascii_lowercase();
    std::iter::once(plan.goal.as_str())
        .chain(plan.ordered_steps.iter().map(String::as_str))
        .chain(std::iter::once(plan.verification.as_str()))
        .chain(plan.risks.iter().map(String::as_str))
        .chain(plan.open_questions.iter().map(String::as_str))
        .any(|text| text.to_ascii_lowercase().contains(&needle))
}

fn parse_inline_workflow_prompt(user_input: &str) -> Option<(WorkflowMode, &str)> {
    let trimmed = user_input.trim();
    for (prefix, mode) in [
        ("/ask", WorkflowMode::Ask),
        ("/code", WorkflowMode::Code),
        ("/architect", WorkflowMode::Architect),
        ("/read-only", WorkflowMode::ReadOnly),
        ("/auto", WorkflowMode::Auto),
        ("/teach", WorkflowMode::Teach),
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

// Tool catalogue

/// Returns the full set of tools exposed to the model.
pub fn get_tools() -> Vec<ToolDefinition> {
    crate::agent::tool_registry::get_tools()
}

fn is_natural_language_hallucination(input: &str) -> bool {
    let lower = input.to_lowercase();
    let words = lower.split_whitespace().collect::<Vec<_>>();

    // 1. Sentences starting with conversational phrases
    if words.is_empty() {
        return false;
    }
    let first = words[0];
    if [
        "make", "create", "i", "can", "please", "we", "let's", "go", "execute", "run", "how",
    ]
    .contains(&first)
    {
        // If it's more than 2 words, it's likely a sentence, not a command
        if words.len() >= 3 {
            return true;
        }
    }

    // 2. Presence of English stop-words that are rare in CLI commands
    let stop_words = [
        "the", "a", "an", "on", "my", "your", "for", "with", "into", "onto",
    ];
    let stop_count = words.iter().filter(|w| stop_words.contains(w)).count();
    if stop_count >= 2 {
        return true;
    }

    // 3. Lack of common CLI separators if many words exist
    if words.len() >= 5
        && !input.contains('-')
        && !input.contains('/')
        && !input.contains('\\')
        && !input.contains('.')
    {
        return true;
    }

    false
}

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
    /// Personality description for the current Rusty soul — used in chat mode system prompt.
    pub soul_personality: String,
    pub lsp_manager: Arc<Mutex<crate::agent::lsp::manager::LspManager>>,
    /// Active reasoning summary extracted from the previous model turn (Gemma-4 Native).
    pub reasoning_history: Option<String>,
    /// Layer 8: Active Reference Pinning (Context Locked)
    pub pinned_files: Arc<Mutex<std::collections::HashMap<String, String>>>,
    /// Hard action-grounding state for proof-before-action checks.
    action_grounding: Arc<Mutex<ActionGroundingState>>,
    /// True only during `/code Implement the current plan.` style execution turns.
    plan_execution_active: Arc<std::sync::atomic::AtomicBool>,
    /// Nested depth of the current autonomous `/implement-plan` recursion chain.
    plan_execution_pass_depth: Arc<std::sync::atomic::AtomicUsize>,
    /// Typed per-turn recovery attempt tracking.
    recovery_context: RecoveryContext,
    /// L1 context block — hot files summary injected into the system prompt.
    /// Built once after vein init and updated as edits accumulate heat.
    pub l1_context: Option<String>,
    /// Condensed AST repository layout for the active project.
    pub repo_map: Option<String>,
    /// Number of real inference turns completed this session.
    pub turn_count: u32,
    /// Last user message sent to the model — persisted as checkpoint goal.
    pub last_goal: Option<String>,
    /// Most recent project directory created this session (Automatic Dive-In).
    pub latest_target_dir: Option<String>,
    /// One-shot plan handoff written into a newly created sovereign root before teleport.
    pending_teleport_handoff: Option<SovereignTeleportHandoff>,
    /// Authoritative Turn Diff Tracker for proactive mutation summaries.
    pub diff_tracker: Arc<Mutex<crate::agent::diff_tracker::TurnDiffTracker>>,
    /// Authoritative Toolchain Heartbeat for environment awareness.
    pub last_heartbeat: Option<crate::agent::policy::ToolchainHeartbeat>,
}

impl ConversationManager {
    fn vein_docs_only_mode(&self) -> bool {
        !crate::tools::file_ops::is_project_workspace()
    }

    fn refresh_vein_index(&mut self) -> usize {
        let count = if self.vein_docs_only_mode() {
            tokio::task::block_in_place(|| {
                self.vein
                    .index_workspace_artifacts(&crate::tools::file_ops::hematite_dir())
            })
        } else {
            tokio::task::block_in_place(|| self.vein.index_project())
        };
        self.l1_context = self.vein.l1_context();
        count
    }

    fn build_vein_inspection_report(&self, indexed_this_pass: usize) -> String {
        let snapshot = tokio::task::block_in_place(|| self.vein.inspect_snapshot(8));
        let workspace_mode = if self.vein_docs_only_mode() {
            "docs-only (outside a project workspace)"
        } else {
            "project workspace"
        };
        let active_room = snapshot.active_room.as_deref().unwrap_or("none");
        let mut out = format!(
            "Vein Inspection\n\
             Workspace mode: {workspace_mode}\n\
             Indexed this pass: {indexed_this_pass}\n\
             Indexed source files: {}\n\
             Indexed docs: {}\n\
             Indexed session exchanges: {}\n\
             Embedded source/doc chunks: {}\n\
             Embeddings available: {}\n\
             Active room bias: {active_room}\n\
             L1 hot-files block: {}\n",
            snapshot.indexed_source_files,
            snapshot.indexed_docs,
            snapshot.indexed_session_exchanges,
            snapshot.embedded_source_doc_chunks,
            if snapshot.has_any_embeddings {
                "yes"
            } else {
                "no"
            },
            if snapshot.l1_ready {
                "ready"
            } else {
                "not built yet"
            },
        );

        if snapshot.hot_files.is_empty() {
            out.push_str("Hot files: none yet.\n");
            return out;
        }

        out.push_str("\nHot files by room:\n");
        let mut by_room: std::collections::BTreeMap<&str, Vec<&crate::memory::vein::VeinHotFile>> =
            std::collections::BTreeMap::new();
        for file in &snapshot.hot_files {
            by_room.entry(file.room.as_str()).or_default().push(file);
        }
        for (room, files) in by_room {
            out.push_str(&format!("[{}]\n", room));
            for file in files {
                out.push_str(&format!(
                    "- {} [{} edit{}]\n",
                    file.path,
                    file.heat,
                    if file.heat == 1 { "" } else { "s" }
                ));
            }
        }

        out
    }

    fn latest_user_prompt(&self) -> Option<&str> {
        self.history
            .iter()
            .rev()
            .find(|msg| msg.role == "user")
            .map(|msg| msg.content.as_str())
    }

    async fn emit_direct_response(
        &mut self,
        tx: &mpsc::Sender<InferenceEvent>,
        raw_user_input: &str,
        effective_user_input: &str,
        response: &str,
    ) {
        self.history.push(ChatMessage::user(effective_user_input));
        self.history.push(ChatMessage::assistant_text(response));
        self.transcript.log_user(raw_user_input);
        self.transcript.log_agent(response);
        for chunk in chunk_text(response, 8) {
            if !chunk.is_empty() {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
        }
        if let Some(path) = self.latest_target_dir.take() {
            self.persist_pending_teleport_handoff();
            let _ = tx.send(InferenceEvent::CopyDiveInCommand(path)).await;
        }
        let _ = tx.send(InferenceEvent::Done).await;
        self.trim_history(80);
        self.refresh_session_memory();
        self.save_session();
    }

    async fn emit_operator_checkpoint(
        &mut self,
        tx: &mpsc::Sender<InferenceEvent>,
        state: OperatorCheckpointState,
        summary: impl Into<String>,
    ) {
        let summary = summary.into();
        self.session_memory
            .record_checkpoint(state.label(), summary.clone());
        let _ = tx
            .send(InferenceEvent::OperatorCheckpoint { state, summary })
            .await;
    }

    async fn emit_recovery_recipe_summary(
        &mut self,
        tx: &mpsc::Sender<InferenceEvent>,
        state: impl Into<String>,
        summary: impl Into<String>,
    ) {
        let state = state.into();
        let summary = summary.into();
        self.session_memory.record_recovery(state, summary.clone());
        let _ = tx.send(InferenceEvent::RecoveryRecipe { summary }).await;
    }

    async fn emit_provider_live(&mut self, tx: &mpsc::Sender<InferenceEvent>) {
        let _ = tx
            .send(InferenceEvent::ProviderStatus {
                state: ProviderRuntimeState::Live,
                summary: String::new(),
            })
            .await;
        self.emit_operator_checkpoint(tx, OperatorCheckpointState::Idle, "")
            .await;
    }

    async fn emit_prompt_pressure_for_messages(
        &self,
        tx: &mpsc::Sender<InferenceEvent>,
        messages: &[ChatMessage],
    ) {
        let context_length = self.engine.current_context_length();
        let (estimated_input_tokens, reserved_output_tokens, estimated_total_tokens, percent) =
            crate::agent::inference::estimate_prompt_pressure(
                messages,
                &self.tools,
                context_length,
            );
        let _ = tx
            .send(InferenceEvent::PromptPressure {
                estimated_input_tokens,
                reserved_output_tokens,
                estimated_total_tokens,
                context_length,
                percent,
            })
            .await;
    }

    async fn emit_prompt_pressure_idle(&self, tx: &mpsc::Sender<InferenceEvent>) {
        let context_length = self.engine.current_context_length();
        let _ = tx
            .send(InferenceEvent::PromptPressure {
                estimated_input_tokens: 0,
                reserved_output_tokens: 0,
                estimated_total_tokens: 0,
                context_length,
                percent: 0,
            })
            .await;
    }

    async fn emit_compaction_pressure(&self, tx: &mpsc::Sender<InferenceEvent>) {
        let context_length = self.engine.current_context_length();
        let vram_ratio = self.gpu_state.ratio();
        let config = CompactionConfig::adaptive(context_length, vram_ratio);
        let estimated_tokens = compaction::estimate_compactable_tokens(&self.history);
        let percent = if config.max_estimated_tokens == 0 {
            0
        } else {
            ((estimated_tokens.saturating_mul(100)) / config.max_estimated_tokens).min(100) as u8
        };

        let _ = tx
            .send(InferenceEvent::CompactionPressure {
                estimated_tokens,
                threshold_tokens: config.max_estimated_tokens,
                percent,
            })
            .await;
    }

    async fn refresh_runtime_profile_and_report(
        &mut self,
        tx: &mpsc::Sender<InferenceEvent>,
        reason: &str,
    ) -> Option<(String, usize, bool)> {
        let refreshed = self.engine.refresh_runtime_profile().await;
        if let Some((model_id, context_length, changed)) = refreshed.as_ref() {
            let _ = tx
                .send(InferenceEvent::RuntimeProfile {
                    provider_name: self.engine.provider_name().await,
                    endpoint: crate::runtime::session_endpoint_url(&self.engine.base_url),
                    model_id: model_id.clone(),
                    context_length: *context_length,
                })
                .await;
            self.transcript.log_system(&format!(
                "Runtime profile refresh ({}): model={} ctx={} changed={}",
                reason, model_id, context_length, changed
            ));
        } else {
            let provider_name = self.engine.provider_name().await;
            let endpoint = crate::runtime::session_endpoint_url(&self.engine.base_url);
            let mut summary = format!("{} profile refresh failed at {}", provider_name, endpoint);
            if let Some((alt_name, alt_url)) =
                crate::runtime::detect_alternative_provider(&provider_name).await
            {
                summary.push_str(&format!(
                    " | reachable alternative: {} ({})",
                    alt_name, alt_url
                ));
            }
            let _ = tx
                .send(InferenceEvent::ProviderStatus {
                    state: ProviderRuntimeState::Degraded,
                    summary: summary.clone(),
                })
                .await;
            self.transcript.log_system(&format!(
                "Runtime profile refresh ({}) failed: {}",
                reason, summary
            ));
        }
        refreshed
    }

    async fn emit_embed_profile(&self, tx: &mpsc::Sender<InferenceEvent>) {
        let embed_model = self.engine.get_embedding_model().await;
        self.vein.set_embed_model(embed_model.clone());
        let _ = tx
            .send(InferenceEvent::EmbedProfile {
                model_id: embed_model,
            })
            .await;
    }

    async fn runtime_model_status_report(
        &self,
        config: &crate::agent::config::HematiteConfig,
    ) -> String {
        let provider = self.engine.provider_name().await;
        let coding_model = self.engine.current_model();
        let coding_pref = crate::agent::config::preferred_coding_model(config)
            .unwrap_or_else(|| "none saved".to_string());
        let embed_loaded = self
            .engine
            .get_embedding_model()
            .await
            .unwrap_or_else(|| "not loaded".to_string());
        let embed_pref = config
            .embed_model
            .clone()
            .unwrap_or_else(|| "none saved".to_string());
        format!(
            "Provider: {}\nCoding model: {} | CTX {}\nPreferred coding model: {}\nEmbedding model: {}\nPreferred embed model: {}\nProvider controls: {}\n\nUse `{}`, `/model prefer <id>`, or `{}`.",
            provider,
            coding_model,
            self.engine.current_context_length(),
            coding_pref,
            embed_loaded,
            embed_pref,
            Self::provider_model_controls_summary(&provider),
            Self::model_command_usage(),
            Self::embed_command_usage()
        )
    }

    fn model_command_usage() -> &'static str {
        "/model [status|list [available|loaded]|load <id> [--ctx N]|unload [id|current|all]|prefer <id>|clear]"
    }

    fn embed_command_usage() -> &'static str {
        "/embed [status|load <id>|unload [id|current]|prefer <id>|clear]"
    }

    fn provider_model_controls_summary(provider: &str) -> &'static str {
        if provider == "Ollama" {
            "Ollama supports coding and embed model load/list/unload from Hematite, and `--ctx` maps to Ollama `num_ctx` for coding models."
        } else {
            "LM Studio supports coding and embed model load/unload from Hematite, and `--ctx` maps to LM Studio context length."
        }
    }

    async fn format_provider_model_inventory(
        &self,
        provider: &str,
        kind: crate::agent::provider::ProviderModelKind,
        loaded_only: bool,
    ) -> Result<String, String> {
        let models = self.engine.list_provider_models(kind, loaded_only).await?;
        let scope_label = if loaded_only { "loaded" } else { "available" };
        let role_label = match kind {
            crate::agent::provider::ProviderModelKind::Any => "models",
            crate::agent::provider::ProviderModelKind::Coding => "coding models",
            crate::agent::provider::ProviderModelKind::Embed => "embedding models",
        };
        if models.is_empty() {
            return Ok(format!(
                "No {} {} detected on {}.",
                scope_label, role_label, provider
            ));
        }
        let lines = models
            .iter()
            .enumerate()
            .map(|(idx, model)| format!("{}. {}", idx + 1, model))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(format!(
            "{} {} on {}:\n{}",
            if loaded_only { "Loaded" } else { "Available" },
            role_label,
            provider,
            lines
        ))
    }

    fn parse_model_load_args(arg_text: &str) -> Result<(String, Option<usize>), String> {
        let mut model_id: Option<String> = None;
        let mut context_length: Option<usize> = None;
        let mut tokens = arg_text.split_whitespace().peekable();

        while let Some(token) = tokens.next() {
            match token {
                "--ctx" | "--context" | "--context-length" => {
                    let Some(value) = tokens.next() else {
                        return Err("Missing value for --ctx.".to_string());
                    };
                    let parsed = value
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid context length `{}`.", value))?;
                    context_length = Some(parsed);
                }
                _ if token.starts_with("--ctx=") => {
                    let value = token.trim_start_matches("--ctx=");
                    let parsed = value
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid context length `{}`.", value))?;
                    context_length = Some(parsed);
                }
                _ if token.starts_with("--context-length=") => {
                    let value = token.trim_start_matches("--context-length=");
                    let parsed = value
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid context length `{}`.", value))?;
                    context_length = Some(parsed);
                }
                _ if token.starts_with("--") => {
                    return Err(format!("Unknown model-load flag `{}`.", token));
                }
                _ => {
                    if model_id.is_some() {
                        return Err(
                            "Model ID must be one token; if it contains spaces, use the exact local model key without spaces."
                                .to_string(),
                        );
                    }
                    model_id = Some(token.to_string());
                }
            }
        }

        let model_id = model_id.ok_or_else(|| "Missing model ID.".to_string())?;
        Ok((model_id, context_length))
    }

    fn parse_unload_target(arg_text: &str) -> Result<(Option<String>, bool), String> {
        let target = arg_text.trim();
        if target.is_empty() || target.eq_ignore_ascii_case("current") {
            Ok((None, false))
        } else if target.eq_ignore_ascii_case("all") {
            Ok((None, true))
        } else if target.contains(char::is_whitespace) {
            Err("Model ID must be one token; if it contains spaces, use the exact local model key without spaces.".to_string())
        } else {
            Ok((Some(target.to_string()), false))
        }
    }

    async fn load_runtime_model_now(
        &mut self,
        tx: &mpsc::Sender<InferenceEvent>,
        model_id: &str,
        role_label: &str,
        context_length: Option<usize>,
    ) -> Result<String, String> {
        let provider = self.engine.provider_name().await;
        if role_label == "embed" {
            if context_length.is_some() {
                return Err(
                    "Embedding models do not use `/model ... --ctx` semantics here.".to_string(),
                );
            }
            self.engine.load_embedding_model(model_id).await?;
        } else {
            self.engine
                .load_model_with_context(model_id, context_length)
                .await?;
        }

        let refreshed = if provider == "Ollama" {
            let ctx =
                context_length.unwrap_or_else(|| self.engine.current_context_length().max(8192));
            if role_label == "embed" {
                None
            } else {
                self.engine.set_runtime_profile(model_id, ctx).await;
                let _ = tx
                    .send(InferenceEvent::RuntimeProfile {
                        provider_name: provider.clone(),
                        endpoint: crate::runtime::session_endpoint_url(&self.engine.base_url),
                        model_id: model_id.to_string(),
                        context_length: ctx,
                    })
                    .await;
                Some((model_id.to_string(), ctx, true))
            }
        } else {
            self.refresh_runtime_profile_and_report(tx, &format!("{}_load", role_label))
                .await
        };
        self.emit_embed_profile(tx).await;

        let loaded_embed = self.engine.get_embedding_model().await;
        let status = match role_label {
            "embed" => format!(
                "Requested embed model load for `{}`. Current embedding model: {}.",
                model_id,
                loaded_embed.unwrap_or_else(|| "not loaded".to_string())
            ),
            _ => match refreshed {
                Some((current, ctx, _)) => format!(
                    "Requested coding model load for `{}`. Current coding model: {} | CTX {}{}.",
                    model_id,
                    current,
                    ctx,
                    context_length
                        .map(|requested| format!(" | requested ctx {}", requested))
                        .unwrap_or_default()
                ),
                None => format!(
                    "Requested coding model load for `{}`. Hematite could not refresh the runtime profile afterward; run `/runtime-refresh` once LM Studio settles.",
                    model_id
                ),
            },
        };
        Ok(status)
    }

    async fn unload_runtime_model_now(
        &mut self,
        tx: &mpsc::Sender<InferenceEvent>,
        model_id: Option<&str>,
        role_label: &str,
        unload_all: bool,
    ) -> Result<String, String> {
        let resolved_target = if unload_all {
            None
        } else {
            match role_label {
                "embed" => match model_id {
                    Some("current") | None => self.engine.get_embedding_model().await,
                    Some(explicit) => Some(explicit.to_string()),
                },
                _ => match model_id {
                    Some("current") | None => {
                        let current = self.engine.current_model();
                        let normalized = current.trim();
                        if normalized.is_empty()
                            || normalized.eq_ignore_ascii_case("no model loaded")
                        {
                            None
                        } else {
                            Some(normalized.to_string())
                        }
                    }
                    Some(explicit) => Some(explicit.to_string()),
                },
            }
        };

        if !unload_all && resolved_target.is_none() {
            return Err(match role_label {
                "embed" => "No embedding model is currently loaded.".to_string(),
                _ => "No coding model is currently loaded.".to_string(),
            });
        }

        let outcome = if role_label == "embed" {
            self.engine
                .unload_embedding_model(resolved_target.as_deref())
                .await?
        } else {
            self.engine
                .unload_model(resolved_target.as_deref(), unload_all)
                .await?
        };
        let _ = self
            .refresh_runtime_profile_and_report(tx, &format!("{}_unload", role_label))
            .await;
        self.emit_embed_profile(tx).await;
        Ok(outcome)
    }

    pub fn new(
        engine: Arc<InferenceEngine>,
        professional: bool,
        brief: bool,
        snark: u8,
        chaos: u8,
        soul_personality: String,
        fast_model: Option<String>,
        think_model: Option<String>,
        gpu_state: Arc<GpuState>,
        git_state: Arc<crate::agent::git_monitor::GitState>,
        swarm_coordinator: Arc<crate::agent::swarm::SwarmCoordinator>,
        voice_manager: Arc<crate::ui::voice::VoiceManager>,
    ) -> Self {
        let saved = load_session_data();

        // Build the initial mcp_manager
        let mcp_manager = Arc::new(tokio::sync::Mutex::new(
            crate::agent::mcp_manager::McpManager::new(),
        ));

        // Build the initial system prompt using the canonical InferenceEngine path.
        let dynamic_instructions =
            engine.build_system_prompt(snark, chaos, brief, professional, &[], None, None, &[]);

        let history = vec![ChatMessage::system(&dynamic_instructions)];

        let vein_path = crate::tools::file_ops::hematite_dir().join("vein.db");
        let vein_base_url = engine.base_url.clone();
        let vein = crate::memory::vein::Vein::new(&vein_path, vein_base_url.clone())
            .unwrap_or_else(|_| crate::memory::vein::Vein::new(":memory:", vein_base_url).unwrap());

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
            running_summary: saved.running_summary,
            gpu_state,
            vein,
            transcript: crate::agent::transcript::TranscriptLogger::new(),
            cancel_token: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            git_state,
            think_mode: None,
            workflow_mode: WorkflowMode::Auto,
            session_memory: saved.session_memory,
            swarm_coordinator,
            voice_manager,
            soul_personality,
            lsp_manager: Arc::new(Mutex::new(crate::agent::lsp::manager::LspManager::new(
                crate::tools::file_ops::workspace_root(),
            ))),
            reasoning_history: None,
            pinned_files: Arc::new(Mutex::new(std::collections::HashMap::new())),
            action_grounding: Arc::new(Mutex::new(ActionGroundingState::default())),
            plan_execution_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            plan_execution_pass_depth: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            recovery_context: RecoveryContext::default(),
            l1_context: None,
            repo_map: None,
            turn_count: saved.turn_count,
            last_goal: saved.last_goal,
            latest_target_dir: None,
            pending_teleport_handoff: None,
            last_heartbeat: None,
            diff_tracker: Arc::new(Mutex::new(
                crate::agent::diff_tracker::TurnDiffTracker::new(),
            )),
        }
    }

    async fn emit_done_events(&mut self, tx: &tokio::sync::mpsc::Sender<InferenceEvent>) {
        if let Some(path) = self.latest_target_dir.take() {
            self.persist_pending_teleport_handoff();
            let _ = tx.send(InferenceEvent::CopyDiveInCommand(path)).await;
        }
        let _ = tx.send(InferenceEvent::Done).await;
    }

    /// Index the project into The Vein. Call once after construction.
    /// Uses block_in_place so the tokio runtime thread isn't parked.
    pub fn initialize_vein(&mut self) -> usize {
        self.refresh_vein_index()
    }

    /// Generate the AST Repo Map. Call once after construction or when resetting context.
    pub fn initialize_repo_map(&mut self) {
        if !self.vein_docs_only_mode() {
            let root = crate::tools::file_ops::workspace_root();
            let hot = self.vein.hot_files_weighted(10);
            let gen = crate::memory::repo_map::RepoMapGenerator::new(&root).with_hot_files(&hot);
            match tokio::task::block_in_place(|| gen.generate()) {
                Ok(map) => self.repo_map = Some(map),
                Err(e) => {
                    self.repo_map = Some(format!("Repo Map generation failed: {}", e));
                }
            }
        }
    }

    /// Re-generate the repo map after a file edit so rankings stay fresh.
    /// Lightweight (~100-200ms) — called after successful mutations.
    fn refresh_repo_map(&mut self) {
        self.initialize_repo_map();
    }

    fn save_session(&self) {
        let path = session_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let saved = SavedSession {
            running_summary: self.running_summary.clone(),
            session_memory: self.session_memory.clone(),
            last_goal: self.last_goal.clone(),
            turn_count: self.turn_count,
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
            last_goal: None,
            turn_count: 0,
        };
        if let Ok(json) = serde_json::to_string(&saved) {
            let _ = std::fs::write(&path, json);
        }
    }

    fn refresh_session_memory(&mut self) {
        let current_plan = self.session_memory.current_plan.clone();
        let previous_memory = self.session_memory.clone();
        self.session_memory = compaction::extract_memory(&self.history);
        self.session_memory.current_plan = current_plan;
        self.session_memory
            .inherit_runtime_ledger_from(&previous_memory);
    }

    fn build_chat_system_prompt(&self) -> String {
        let species = &self.engine.species;
        let personality = &self.soul_personality;
        let mut sys = format!(
            "You are {species}, a local AI companion running entirely on the user's GPU — no cloud, no subscriptions, no phoning home.\n\
             {personality}\n\n\
             This is CHAT mode — a clean conversational surface. Behave like a sharp friend who happens to know everything about code, not like an agent following a workflow.\n\n"
        );

        if let Some(summary) = self.last_heartbeat.as_ref() {
            sys.push_str("## HOST ENVIRONMENT\n");
            sys.push_str(&summary.to_summary());
            sys.push_str("\n\n");
        }

        sys.push_str(
            "Rules:\n\
             - Talk like a person. Skip the bullet-point breakdowns unless the topic genuinely needs structure.\n\
             - Answer directly. One paragraph is usually right.\n\
             - Don't call tools unless the user explicitly asks you to look at a file or run something.\n\
             - Don't narrate your reasoning or mention tool names unprompted.\n\
             - You can discuss code, debug ideas, explain concepts, help plan, or just talk.\n\
             - If the user clearly wants you to edit or build something, do it — but lead with conversation, not scaffolding.\n\
             - If the user wants the full coding harness, they can type `/agent`.\n",
        );
        sys
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

    fn current_plan_summary(&self) -> Option<String> {
        self.session_memory
            .current_plan
            .as_ref()
            .filter(|plan| plan.has_signal())
            .map(|plan| plan.summary_line())
    }

    fn current_plan_allowed_paths(&self) -> Vec<String> {
        self.session_memory
            .current_plan
            .as_ref()
            .map(|plan| merge_plan_allowed_paths(&plan.target_files))
            .unwrap_or_default()
    }

    fn current_plan_root_paths(&self) -> Vec<String> {
        use std::collections::BTreeSet;

        let mut roots = BTreeSet::new();
        for path in self.current_plan_allowed_paths() {
            if let Some(parent) = std::path::Path::new(&path).parent() {
                roots.insert(parent.to_string_lossy().replace('\\', "/").to_lowercase());
            }
        }
        roots.into_iter().collect()
    }

    fn persist_architect_handoff(
        &mut self,
        response: &str,
    ) -> Option<crate::tools::plan::PlanHandoff> {
        if self.workflow_mode != WorkflowMode::Architect {
            return None;
        }
        let Some(plan) = crate::tools::plan::parse_plan_handoff(response) else {
            return None;
        };
        let _ = crate::tools::plan::save_plan_handoff(&plan);
        self.session_memory.current_plan = Some(plan.clone());
        Some(plan)
    }

    fn persist_pending_teleport_handoff(&mut self) {
        let Some(handoff) = self.pending_teleport_handoff.take() else {
            return;
        };
        let root = std::path::PathBuf::from(&handoff.root);
        let _ = crate::tools::plan::save_plan_handoff_for_root(&root, &handoff.plan);
        let _ = crate::tools::plan::write_teleport_resume_marker_for_root(&root);
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

    /// Parse `@<path>` tokens from the raw user message and register any files that
    /// resolve to real paths as observed+inspected this turn. This lets the model
    /// call `edit_file` immediately on @-mentioned files without a read_file round-trip.
    async fn register_at_file_mentions(&self, input: &str) {
        if !input.contains('@') {
            return;
        }
        let cwd = match std::env::current_dir() {
            Ok(d) => d,
            Err(_) => return,
        };
        let mut state = self.action_grounding.lock().await;
        let turn = state.turn_index;
        for token in input.split_whitespace() {
            if !token.starts_with('@') {
                continue;
            }
            let raw = token
                .trim_start_matches('@')
                .trim_end_matches(|c: char| matches!(c, ',' | '.' | ':' | ';' | '!' | '?'));
            if raw.is_empty() {
                continue;
            }
            if cwd.join(raw).is_file() {
                let normalized = normalize_workspace_path(raw);
                state.observed_paths.insert(normalized.clone(), turn);
                state.inspected_paths.insert(normalized, turn);
            }
        }
    }

    async fn record_read_observation(&self, path: &str) {
        let normalized = normalize_workspace_path(path);
        let mut state = self.action_grounding.lock().await;
        let turn = state.turn_index;
        // read_file returns full file content with line numbers — sufficient for
        // the model to know exact text before editing, so it satisfies the
        // line-inspection grounding check too.
        state.observed_paths.insert(normalized.clone(), turn);
        state.inspected_paths.insert(normalized, turn);
    }

    async fn record_line_inspection(&self, path: &str) {
        let normalized = normalize_workspace_path(path);
        let mut state = self.action_grounding.lock().await;
        let turn = state.turn_index;
        state.observed_paths.insert(normalized.clone(), turn);
        state.inspected_paths.insert(normalized, turn);
    }

    async fn record_verify_build_result(&self, ok: bool, output: &str) {
        let mut state = self.action_grounding.lock().await;
        let turn = state.turn_index;
        state.last_verify_build_turn = Some(turn);
        state.last_verify_build_ok = ok;
        if ok {
            state.code_changed_since_verify = false;
            state.last_failed_build_paths.clear();
        } else {
            state.last_failed_build_paths = parse_failing_paths_from_build_output(output);
        }
    }

    fn record_session_verification(&mut self, ok: bool, summary: impl Into<String>) {
        self.session_memory.record_verification(ok, summary);
    }

    async fn record_successful_mutation(&self, path: Option<&str>) {
        let mut state = self.action_grounding.lock().await;
        state.code_changed_since_verify = match path {
            Some(p) => is_code_like_path(p),
            None => true,
        };
    }

    async fn validate_action_preconditions(&self, name: &str, args: &Value) -> Result<(), String> {
        // Redundancy Check (Steering Tier 1 - Blocking)
        if let Some(steer_hint) =
            crate::agent::policy::is_redundant_action(name, args, &self.history)
        {
            return Err(steer_hint);
        }

        if name == "shell" {
            if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                if !crate::agent::policy::find_binary_in_path(cmd) {
                    return Err(format!(
                        "PREDICTIVE FAILURE: The binary for the command `{}` was not found in the host PATH. \
                         Do not attempt to run this command. Either troubleshoot the toolchain \
                         using `inspect_host(topic='fix_plan')` or ask the user to verify its installation.",
                        cmd
                    ));
                }
            }
        }

        if self
            .plan_execution_active
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            if is_current_plan_irrelevant_tool(name) {
                let prompt = self.latest_user_prompt().unwrap_or("");
                let plan_override = self
                    .session_memory
                    .current_plan
                    .as_ref()
                    .map(|plan| plan_handoff_mentions_tool(plan, name))
                    .unwrap_or(false);
                let explicit_override = is_sovereign_path_request(prompt)
                    || prompt.contains(name)
                    || prompt.contains("/dev/null")
                    || plan_override;
                if !explicit_override {
                    return Err(format!(
                        "Action blocked: `{}` is not part of current-plan execution. Stay on the saved target files, use built-in workspace file tools only, and either make a concrete edit or surface one specific blocker.",
                        name
                    ));
                }
            }

            if is_plan_scoped_tool(name) {
                let allowed_paths = self.current_plan_allowed_paths();
                if !allowed_paths.is_empty() {
                    let allowed_roots = self.current_plan_root_paths();
                    let in_allowed = match name {
                        "auto_pin_context" => args
                            .get("paths")
                            .and_then(|v| v.as_array())
                            .map(|paths| {
                                !paths.is_empty()
                                    && paths.iter().all(|v| {
                                        v.as_str()
                                            .map(normalize_workspace_path)
                                            .map(|p| allowed_paths.contains(&p))
                                            .unwrap_or(false)
                                    })
                            })
                            .unwrap_or(false),
                        "grep_files" | "list_files" => {
                            let raw_val = args.get("path").and_then(|v| v.as_str());
                            let path_to_check = if let Some(p) = raw_val {
                                let trimmed = p.trim();
                                if trimmed.is_empty() || trimmed == "." || trimmed == "./" {
                                    ""
                                } else {
                                    trimmed
                                }
                            } else {
                                ""
                            };
                            // Always allow listing the workspace root — the model needs
                            // directory recon to locate plan targets.
                            if path_to_check.is_empty() {
                                true
                            } else {
                                let p = normalize_workspace_path(path_to_check);
                                // Allow if the path IS an allowed file, OR is a parent dir
                                // of any allowed file (the model needs to ls the parent).
                                allowed_paths.contains(&p)
                                    || allowed_roots.iter().any(|root| root == &p)
                                    || allowed_paths.iter().any(|ap| {
                                        ap.starts_with(&format!("{}/", p))
                                            || ap.starts_with(&format!("{}\\", p))
                                    })
                            }
                        }
                        _ => {
                            let target = action_target_path(name, args);
                            let in_allowed = target
                                .as_ref()
                                .map(|p| allowed_paths.contains(p))
                                .unwrap_or(false);
                            let raw_path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                            in_allowed || is_sovereign_path_request(raw_path)
                        }
                    };

                    if !in_allowed {
                        let allowed = allowed_paths
                            .iter()
                            .map(|p| format!("`{}`", p))
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Err(format!(
                            "Action blocked: current-plan execution is locked to the saved target files. Use a path-scoped built-in tool on one of these files only: {}.",
                            allowed
                        ));
                    }
                }
            }

            if matches!(name, "edit_file" | "multi_search_replace" | "patch_hunk") {
                if let Some(target) = action_target_path(name, args) {
                    let state = self.action_grounding.lock().await;
                    let recently_inspected = state
                        .inspected_paths
                        .get(&target)
                        .map(|turn| state.turn_index.saturating_sub(*turn) <= 3)
                        .unwrap_or(false);
                    drop(state);
                    if !recently_inspected {
                        return Err(format!(
                            "Action blocked: `{}` on '{}' requires an exact local line window first during current-plan execution. Use `inspect_lines` on that file around the intended edit region, then retry the mutation.",
                            name, target
                        ));
                    }
                }
            }
        }

        if self.workflow_mode.is_read_only() && name == "auto_pin_context" {
            return Err(
                "Action blocked: `auto_pin_context` is disabled in read-only workflows. Use the grounded file evidence you already have, or narrow with `inspect_lines` instead of pinning more files into active context."
                    .to_string(),
            );
        }

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
            if matches!(
                name,
                "write_file" | "edit_file" | "patch_hunk" | "multi_search_replace"
            ) {
                if let Some(prompt) = self.latest_user_prompt() {
                    if docs_edit_without_explicit_request(prompt, target) {
                        return Err(format!(
                            "Action blocked: '{}' is a docs file but the current request did not explicitly ask for documentation changes. Finish the code task first. If docs need updating, the user will ask.",
                            target
                        ));
                    }
                }
            }
            let path_exists = std::path::Path::new(target).exists();
            if path_exists {
                let state = self.action_grounding.lock().await;
                let pinned = self.pinned_files.lock().await;
                let pinned_match = pinned.keys().any(|p| normalize_workspace_path(p) == target);
                drop(pinned);

                // edit_file and multi_search_replace match text exactly, so they need a
                // tighter evidence bar than a plain read. Require inspect_lines on the
                // target within the last 3 turns. A read_file in the *same* turn is also
                // accepted (the model just loaded the file and is making an immediate edit).
                let needs_exact_window = matches!(name, "edit_file" | "multi_search_replace");
                let recently_inspected = state
                    .inspected_paths
                    .get(target)
                    .map(|turn| state.turn_index.saturating_sub(*turn) <= 3)
                    .unwrap_or(false);
                let same_turn_read = state
                    .observed_paths
                    .get(target)
                    .map(|turn| state.turn_index.saturating_sub(*turn) == 0)
                    .unwrap_or(false);
                let recent_observed = state
                    .observed_paths
                    .get(target)
                    .map(|turn| state.turn_index.saturating_sub(*turn) <= 3)
                    .unwrap_or(false);

                if matches!(
                    name,
                    "read_file" | "inspect_lines" | "list_files" | "grep_files"
                ) {
                    // These are the grounding tools themselves; they should be allowed to
                    // establish evidence on an already-allowed target path.
                } else if name == "write_file" && matches!(self.workflow_mode, WorkflowMode::Code) {
                    let size = std::fs::metadata(target).map(|m| m.len()).unwrap_or(0);
                    if size > 2000 {
                        // SURGICAL MANDATE: In CODE mode, for files larger than 2KB, we block full-file rewrites.
                        return Err(format!(
                            "SURGICAL MANDATE: '{}' already exists and is significant ({} bytes). In implementation mode, you must use `edit_file` or `patch_hunk` for targeted changes instead of rewriting the entire file with `write_file`. This maintains project integrity and prevents context burn. HINT: Use `read_file` to capture the current state, then use `edit_file` with the exact text you want to replace in `target_content`.",
                            target, size
                        ));
                    }
                } else if needs_exact_window {
                    if !recently_inspected && !same_turn_read && !pinned_match {
                        return Err(format!(
                            "Action blocked: `{}` on '{}' requires a line-level inspection first. \
                             Use `inspect_lines` on the target region to get the exact current text \
                             (whitespace and indentation included), then retry the edit.",
                            name, target
                        ));
                    }
                } else if !recent_observed && !pinned_match {
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

        // Phase gate: if the build is broken, constrain edits to files that cargo flagged.
        // This prevents the model from wandering to unrelated files after a failed verify.
        if matches!(
            name,
            "write_file" | "edit_file" | "patch_hunk" | "multi_search_replace"
        ) {
            if let Some(target) = normalized_target.as_deref() {
                let state = self.action_grounding.lock().await;
                if state.code_changed_since_verify
                    && !state.last_verify_build_ok
                    && !state.last_failed_build_paths.is_empty()
                    && !state.last_failed_build_paths.iter().any(|p| p == target)
                {
                    let files = state
                        .last_failed_build_paths
                        .iter()
                        .map(|p| format!("`{}`", p))
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(format!(
                        "Action blocked: the build is broken. Fix the errors in {} before editing other files. Re-run workspace verification to confirm the fix, then continue.",
                        files
                    ));
                }
            }
        }

        if name == "git_commit" || name == "git_push" {
            let state = self.action_grounding.lock().await;
            if state.code_changed_since_verify && !state.last_verify_build_ok {
                return Err(format!(
                    "Action blocked: `{}` requires a successful verification pass after the latest code edits. Run verification first so Hematite has proof that the workspace is clean.",
                    name
                ));
            }
        }

        if name == "shell" {
            let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            if shell_looks_like_structured_host_inspection(command) {
                // Auto-redirect: silently call inspect_host with the right topic instead of
                // returning a block error that the model may fail to recover from.
                // Derive topic ONLY from the shell command itself. We do not fall back to the user prompt
                // here to avoid trapping secondary shell commands in a redirection loop based on the primary intent.
                let topic = match preferred_host_inspection_topic(command) {
                    Some(t) => t.to_string(),
                    None => return Ok(()), // Not a clear host inspection command, allow it to pass through.
                };

                {
                    let mut state = self.action_grounding.lock().await;
                    let current_turn = state.turn_index;
                    if let Some(turn) = state.redirected_host_inspection_topics.get(&topic) {
                        if *turn == current_turn {
                            return Err(format!(
                                "[auto-redirected shell→inspect_host(topic=\"{topic}\")] Notice: The diagnostic data for topic `{topic}` was already provided in this turn. Using the previous result to avoid redundant tool calls."
                            ));
                        }
                    }
                    state
                        .redirected_host_inspection_topics
                        .insert(topic.clone(), current_turn);
                }

                let path_val = self
                    .latest_user_prompt()
                    .and_then(|p| {
                        // Very basic heuristic for path extraction: look for strings with dots/slashes
                        p.split_whitespace()
                            .find(|w| w.contains('.') || w.contains('/') || w.contains('\\'))
                            .map(|s| {
                                s.trim_matches(|c: char| {
                                    !c.is_alphanumeric() && c != '.' && c != '/' && c != '\\'
                                })
                            })
                    })
                    .unwrap_or("");

                let mut redirect_args = if !path_val.is_empty() {
                    serde_json::json!({ "topic": topic, "path": path_val })
                } else {
                    serde_json::json!({ "topic": topic })
                };

                // Surgical Argument Extraction for redirected shell payloads.
                if topic == "dns_lookup" {
                    if let Some(identity) = extract_dns_lookup_target_from_shell(command) {
                        redirect_args
                            .as_object_mut()
                            .unwrap()
                            .insert("name".to_string(), serde_json::Value::String(identity));
                    }
                    if let Some(record_type) = extract_dns_record_type_from_shell(command) {
                        redirect_args.as_object_mut().unwrap().insert(
                            "type".to_string(),
                            serde_json::Value::String(record_type.to_string()),
                        );
                    }
                } else if topic == "ad_user" {
                    let cmd_lower = command.to_lowercase();
                    let mut identity = String::new();

                    // 1. Explicit Identity check
                    if let Some(idx) = cmd_lower.find("-identity") {
                        let after_id = &command[idx + 9..].trim();
                        identity = if after_id.starts_with('\'') || after_id.starts_with('"') {
                            let quote = after_id.chars().next().unwrap();
                            after_id.split(quote).nth(1).unwrap_or("").to_string()
                        } else {
                            after_id.split_whitespace().next().unwrap_or("").to_string()
                        };
                    }

                    // 2. Wide-Net Fallback: Find the first non-cmdlet, non-parameter string
                    if identity.is_empty() {
                        let parts: Vec<&str> = command.split_whitespace().collect();
                        for (i, part) in parts.iter().enumerate() {
                            if i == 0 || part.starts_with('-') {
                                continue;
                            }
                            // Skip common cmdlets if they are in the parts list
                            let p_low = part.to_lowercase();
                            if p_low.contains("get-ad")
                                || p_low.contains("powershell")
                                || p_low == "-command"
                            {
                                continue;
                            }

                            identity = part
                                .trim_matches(|c: char| c == '\'' || c == '"')
                                .to_string();
                            if !identity.is_empty() {
                                break;
                            }
                        }
                    }

                    if !identity.is_empty() {
                        redirect_args.as_object_mut().unwrap().insert(
                            "name_filter".to_string(),
                            serde_json::Value::String(identity),
                        );
                    }
                }

                let result = crate::tools::host_inspect::inspect_host(&redirect_args).await;
                return match result {
                    Ok(output) => Err(format!(
                        "[auto-redirected shell→inspect_host(topic=\"{topic}\")]\n\n{output}\n\n[Note: Shell is blocked for host inspection. The diagnostic data above fulfills your request. Use inspect_host directly for further diagnostics.]"
                    )),
                    Err(e) => Err(format!(
                        "Redirection to native tool `{topic}` failed: {e}\n\nAction blocked: use `inspect_host(topic: \"{topic}\")` instead of raw `shell` for host-inspection questions. Available topics: updates, security, pending_reboot, disk_health, battery, recent_crashes, scheduled_tasks, dev_conflicts, health_report, storage, hardware, resource_load, overclocker, processes, network, lan_discovery, audio, bluetooth, camera, sign_in, installer_health, onedrive, browser_health, identity_auth, outlook, teams, windows_backup, search_index, display_config, ntp, cpu_power, credentials, tpm, hyperv, event_query, latency, network_adapter, dhcp, mtu, ipv6, tcp_params, wlan_profiles, ipsec, netbios, nic_teaming, snmp, port_test, network_profile, services, ports, env_doctor, fix_plan, connectivity, wifi, connections, vpn, proxy, firewall_rules, traceroute, dns_cache, arp, route_table, docker, docker_filesystems, wsl, wsl_filesystems, ssh, env, hosts_file, installed_software, git_config, databases, disk_benchmark, directory, permissions, login_history, registry_audit, share_access.",
                    )),
                };
            }
            let reason = args
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
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
                metadata: crate::agent::inference::tool_metadata_for_name(&tool.name),
            }));
    }

    async fn emit_mcp_runtime_status(&self, tx: &mpsc::Sender<InferenceEvent>) {
        let summary = {
            let mcp = self.mcp_manager.lock().await;
            mcp.runtime_report()
        };
        let _ = tx
            .send(InferenceEvent::McpStatus {
                state: summary.state,
                summary: summary.summary,
            })
            .await;
    }

    async fn refresh_mcp_tools(
        &mut self,
        tx: &mpsc::Sender<InferenceEvent>,
    ) -> Result<Vec<crate::agent::mcp::McpTool>, Box<dyn std::error::Error + Send + Sync>> {
        let mcp_tools = {
            let mut mcp = self.mcp_manager.lock().await;
            match mcp.initialize_all().await {
                Ok(()) => mcp.discover_tools().await,
                Err(e) => {
                    drop(mcp);
                    self.replace_mcp_tool_definitions(&[]);
                    self.emit_mcp_runtime_status(tx).await;
                    return Err(e.into());
                }
            }
        };

        self.replace_mcp_tool_definitions(&mcp_tools);
        self.emit_mcp_runtime_status(tx).await;
        Ok(mcp_tools)
    }

    /// Spawns and initializes all configured MCP servers, discovering their tools.
    pub async fn initialize_mcp(
        &mut self,
        tx: &mpsc::Sender<InferenceEvent>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = self.refresh_mcp_tools(tx).await?;
        Ok(())
    }

    /// Run one user turn through the full agentic loop.
    ///
    /// Adds the user message, calls the model, executes any tools, and loops
    /// until the model produces a final text reply.  All progress is streamed
    /// as `InferenceEvent` values via `tx`.
    pub async fn run_turn(
        &mut self,
        user_turn: &UserTurn,
        tx: mpsc::Sender<InferenceEvent>,
        yolo: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let user_input = user_turn.text.as_str();
        // ── Fast-path reset commands: handled locally, no network I/O needed ──
        if user_input.trim() == "/new" {
            self.history.clear();
            self.reasoning_history = None;
            self.session_memory.clear();
            self.running_summary = None;
            self.correction_hints.clear();
            self.pinned_files.lock().await.clear();
            self.reset_action_grounding().await;
            reset_task_files();
            let _ = std::fs::remove_file(session_path());
            self.save_empty_session();
            self.emit_compaction_pressure(&tx).await;
            self.emit_prompt_pressure_idle(&tx).await;
            for chunk in chunk_text(
                "Fresh task context started. Chat history, pins, and task files cleared. Saved memory remains available.",
                8,
            ) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/forget" {
            self.history.clear();
            self.reasoning_history = None;
            self.session_memory.clear();
            self.running_summary = None;
            self.correction_hints.clear();
            self.pinned_files.lock().await.clear();
            self.reset_action_grounding().await;
            reset_task_files();
            purge_persistent_memory();
            tokio::task::block_in_place(|| self.vein.reset());
            let _ = std::fs::remove_file(session_path());
            self.save_empty_session();
            self.emit_compaction_pressure(&tx).await;
            self.emit_prompt_pressure_idle(&tx).await;
            for chunk in chunk_text(
                "Hard forget complete. Chat history, saved memory, task files, and the Vein index were purged.",
                8,
            ) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/vein-inspect" {
            let indexed = self.refresh_vein_index();
            let report = self.build_vein_inspection_report(indexed);
            let snapshot = tokio::task::block_in_place(|| self.vein.inspect_snapshot(1));
            let _ = tx
                .send(InferenceEvent::VeinStatus {
                    file_count: snapshot.indexed_source_files + snapshot.indexed_docs,
                    embedded_count: snapshot.embedded_source_doc_chunks,
                    docs_only: self.vein_docs_only_mode(),
                })
                .await;
            for chunk in chunk_text(&report, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/workspace-profile" {
            let root = crate::tools::file_ops::workspace_root();
            let _ = crate::agent::workspace_profile::ensure_workspace_profile(&root);
            let report = crate::agent::workspace_profile::profile_report(&root);
            for chunk in chunk_text(&report, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/rules" {
            let rules_path = crate::tools::file_ops::hematite_dir().join("rules.md");
            let report = if rules_path.exists() {
                match std::fs::read_to_string(&rules_path) {
                    Ok(content) => format!(
                        "## Behavioral Rules (.hematite/rules.md)\n\n{}\n\n---\nTo update: ask Hematite to edit your rules, or open `.hematite/rules.md` directly. Changes take effect on the next turn.",
                        content.trim()
                    ),
                    Err(e) => format!("Error reading .hematite/rules.md: {e}"),
                }
            } else {
                format!(
                    "No behavioral rules file found at `.hematite/rules.md`.\n\nCreate it to add custom behavioral guidelines — they are injected into the system prompt on every turn and apply to any model you load.\n\nExample: ask Hematite to \"create a rules.md with simplicity-first and surgical-edit guidelines\" and it will write the file for you.\n\nExpected path: {}",
                    rules_path.display()
                )
            };
            for chunk in chunk_text(&report, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/vein-reset" {
            tokio::task::block_in_place(|| self.vein.reset());
            let _ = tx
                .send(InferenceEvent::VeinStatus {
                    file_count: 0,
                    embedded_count: 0,
                    docs_only: self.vein_docs_only_mode(),
                })
                .await;
            for chunk in chunk_text("Vein index cleared. Will rebuild on the next turn.", 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        // Reload config every turn (edits apply immediately, no restart needed).
        let config = crate::agent::config::load_config();
        self.recovery_context.clear();
        let manual_runtime_refresh = user_input.trim() == "/runtime-refresh";
        if !manual_runtime_refresh {
            if let Some((model_id, context_length, changed)) = self
                .refresh_runtime_profile_and_report(&tx, "turn_start")
                .await
            {
                if changed {
                    let _ = tx
                        .send(InferenceEvent::Thought(format!(
                            "Runtime refresh: using model `{}` with CTX {} for this turn.",
                            model_id, context_length
                        )))
                        .await;
                }
            }
        }
        self.emit_embed_profile(&tx).await;
        self.emit_compaction_pressure(&tx).await;
        let current_model = self.engine.current_model();
        self.engine.set_gemma_native_formatting(
            crate::agent::config::effective_gemma_native_formatting(&config, &current_model),
        );
        let _turn_id = self.begin_grounded_turn().await;
        let _hook_runner = crate::agent::hooks::HookRunner::new(config.hooks.clone());
        let mcp_tools = match self.refresh_mcp_tools(&tx).await {
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

        let trimmed_input = user_input.trim();

        if trimmed_input == "/model" || trimmed_input.starts_with("/model ") {
            let arg_text = trimmed_input.strip_prefix("/model").unwrap_or("").trim();
            let response = if arg_text.is_empty() || arg_text.eq_ignore_ascii_case("status") {
                Ok(self.runtime_model_status_report(&config).await)
            } else if let Some(list_args) = arg_text.strip_prefix("list").map(str::trim) {
                let loaded_only = if list_args.is_empty()
                    || list_args.eq_ignore_ascii_case("available")
                {
                    false
                } else if list_args.eq_ignore_ascii_case("loaded") {
                    true
                } else {
                    for chunk in chunk_text(&format!("Usage: {}", Self::model_command_usage()), 8) {
                        let _ = tx.send(InferenceEvent::Token(chunk)).await;
                    }
                    let _ = tx.send(InferenceEvent::Done).await;
                    return Ok(());
                };
                let provider = self.engine.provider_name().await;
                self.format_provider_model_inventory(
                    &provider,
                    crate::agent::provider::ProviderModelKind::Coding,
                    loaded_only,
                )
                .await
            } else if let Some(load_args) = arg_text.strip_prefix("load ").map(str::trim) {
                if load_args.is_empty() {
                    Err(format!("Usage: {}", Self::model_command_usage()))
                } else {
                    let (model_id, context_length) = Self::parse_model_load_args(load_args)?;
                    self.load_runtime_model_now(&tx, &model_id, "coding", context_length)
                        .await
                }
            } else if let Some(unload_args) = arg_text.strip_prefix("unload").map(str::trim) {
                let (target, unload_all) = Self::parse_unload_target(unload_args)?;
                self.unload_runtime_model_now(&tx, target.as_deref(), "coding", unload_all)
                    .await
            } else if let Some(model_id) = arg_text.strip_prefix("prefer ").map(str::trim) {
                if model_id.is_empty() {
                    Err(format!("Usage: {}", Self::model_command_usage()))
                } else {
                    crate::agent::config::set_preferred_coding_model(Some(model_id)).map(|_| {
                        format!(
                            "Saved preferred coding model `{}` in `.hematite/settings.json`. Use `/model load {}` now or restart Hematite to let startup policy load it automatically.",
                            model_id, model_id
                        )
                    })
                }
            } else if matches!(arg_text, "clear" | "clear-preference") {
                crate::agent::config::set_preferred_coding_model(None)
                    .map(|_| "Cleared the saved preferred coding model.".to_string())
            } else {
                Err(format!("Usage: {}", Self::model_command_usage()))
            };

            for chunk in chunk_text(&response.unwrap_or_else(|e| e), 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if trimmed_input == "/embed" || trimmed_input.starts_with("/embed ") {
            let arg_text = trimmed_input.strip_prefix("/embed").unwrap_or("").trim();
            let response = if arg_text.is_empty() || arg_text.eq_ignore_ascii_case("status") {
                Ok(self.runtime_model_status_report(&config).await)
            } else if let Some(load_args) = arg_text.strip_prefix("load ").map(str::trim) {
                if load_args.is_empty() {
                    Err(format!("Usage: {}", Self::embed_command_usage()))
                } else {
                    let (model_id, context_length) = Self::parse_model_load_args(load_args)?;
                    if context_length.is_some() {
                        Err("`/embed load` does not accept `--ctx`. Embedding models do not use a chat context window here.".to_string())
                    } else {
                        self.load_runtime_model_now(&tx, &model_id, "embed", None)
                            .await
                    }
                }
            } else if let Some(unload_args) = arg_text.strip_prefix("unload").map(str::trim) {
                let (target, unload_all) = Self::parse_unload_target(unload_args)?;
                if unload_all {
                    Err("`/embed unload` supports the current embed model or an explicit embed model ID, not `all`.".to_string())
                } else {
                    self.unload_runtime_model_now(&tx, target.as_deref(), "embed", false)
                        .await
                }
            } else if let Some(model_id) = arg_text.strip_prefix("prefer ").map(str::trim) {
                if model_id.is_empty() {
                    Err(format!("Usage: {}", Self::embed_command_usage()))
                } else {
                    crate::agent::config::set_preferred_embed_model(Some(model_id)).map(|_| {
                        format!(
                            "Saved preferred embed model `{}` in `.hematite/settings.json`. Use `/embed load {}` now or restart Hematite to let startup policy load it automatically.",
                            model_id, model_id
                        )
                    })
                }
            } else if matches!(arg_text, "clear" | "clear-preference") {
                crate::agent::config::set_preferred_embed_model(None)
                    .map(|_| "Cleared the saved preferred embed model.".to_string())
            } else {
                Err(format!("Usage: {}", Self::embed_command_usage()))
            };

            for chunk in chunk_text(&response.unwrap_or_else(|e| e), 8) {
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

        if user_input.trim() == "/runtime-refresh" {
            match self
                .refresh_runtime_profile_and_report(&tx, "manual_command")
                .await
            {
                Some((model_id, context_length, changed)) => {
                    let msg = if changed {
                        format!(
                            "Runtime profile refreshed. Model: {} | CTX: {}",
                            model_id, context_length
                        )
                    } else {
                        format!(
                            "Runtime profile unchanged. Model: {} | CTX: {}",
                            model_id, context_length
                        )
                    };
                    for chunk in chunk_text(&msg, 8) {
                        let _ = tx.send(InferenceEvent::Token(chunk)).await;
                    }
                }
                None => {
                    let provider_name = self.engine.provider_name().await;
                    let endpoint = crate::runtime::session_endpoint_url(&self.engine.base_url);
                    let alternative =
                        crate::runtime::detect_alternative_provider(&provider_name).await;
                    let mut message = format!(
                        "Runtime refresh failed: {} could not be read at {}.",
                        provider_name, endpoint
                    );
                    if let Some((alt_name, alt_url)) = alternative {
                        message.push_str(&format!(
                            " Reachable alternative detected: {} ({})",
                            alt_name, alt_url
                        ));
                    }
                    let _ = tx.send(InferenceEvent::Error(message)).await;
                }
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
            let mut message =
                "Workflow mode: CODE. Make changes when needed, but keep proof-before-action and verification discipline.".to_string();
            if let Some(plan) = self.current_plan_summary() {
                message.push_str(&format!(" Current plan: {plan}."));
            }
            for chunk in chunk_text(&message, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/architect" {
            self.set_workflow_mode(WorkflowMode::Architect);
            let mut message =
                "Workflow mode: ARCHITECT. Plan, inspect, and shape the approach first. Do not mutate code unless the user explicitly asks to implement. When the handoff is ready, use `/implement-plan` or switch to `/code` to execute it.".to_string();
            if let Some(plan) = self.current_plan_summary() {
                message.push_str(&format!(" Existing plan: {plan}."));
            }
            for chunk in chunk_text(&message, 8) {
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

        if user_input.trim() == "/chat" {
            self.set_workflow_mode(WorkflowMode::Chat);
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/teach" {
            self.set_workflow_mode(WorkflowMode::Teach);
            for chunk in chunk_text(
                "Workflow mode: TEACH. I will inspect your actual machine state first, then walk you through any admin, config, or write task as a grounded, numbered tutorial. I will not execute write operations — I will show you exactly how to do each step yourself.",
                8,
            ) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/reroll" {
            let soul = crate::ui::hatch::generate_soul_random();
            self.snark = soul.snark;
            self.chaos = soul.chaos;
            self.soul_personality = soul.personality.clone();
            // Update the engine's species name so build_chat_system_prompt uses it
            // SAFETY: engine is Arc but species is a plain String field we own logically.
            // We use Arc::get_mut which only succeeds if this is the only strong ref.
            // If it fails (swarm workers hold refs), we fall back to a best-effort clone approach.
            let species = soul.species.clone();
            if let Some(eng) = Arc::get_mut(&mut self.engine) {
                eng.species = species.clone();
            }
            let shiny_tag = if soul.shiny { " 🌟 SHINY" } else { "" };
            let _ = tx
                .send(InferenceEvent::SoulReroll {
                    species: soul.species.clone(),
                    rarity: soul.rarity.label().to_string(),
                    shiny: soul.shiny,
                    personality: soul.personality.clone(),
                })
                .await;
            for chunk in chunk_text(
                &format!(
                    "A new companion awakens!\n[{}{}] {} — \"{}\"",
                    soul.rarity.label(),
                    shiny_tag,
                    soul.species,
                    soul.personality
                ),
                8,
            ) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/agent" {
            self.set_workflow_mode(WorkflowMode::Auto);
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        let implement_plan_alias = user_input.trim() == "/implement-plan";
        if implement_plan_alias
            && !self
                .session_memory
                .current_plan
                .as_ref()
                .map(|plan| plan.has_signal())
                .unwrap_or(false)
        {
            for chunk in chunk_text(
                "No saved architect handoff is active. Run `/architect` first, or switch to `/code` with an explicit implementation request.",
                8,
            ) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        let mut effective_user_input = if implement_plan_alias {
            self.set_workflow_mode(WorkflowMode::Code);
            implement_current_plan_prompt().to_string()
        } else {
            user_input.trim().to_string()
        };
        if let Some((mode, rest)) = parse_inline_workflow_prompt(user_input) {
            self.set_workflow_mode(mode);
            effective_user_input = rest.to_string();
        }
        let transcript_user_input = if implement_plan_alias {
            transcript_user_turn_text(user_turn, "/implement-plan")
        } else {
            transcript_user_turn_text(user_turn, &effective_user_input)
        };
        effective_user_input = apply_turn_attachments(user_turn, &effective_user_input);
        // Register @file mentions in action_grounding so the model can edit them
        // immediately without a separate read_file round-trip.
        self.register_at_file_mentions(user_input).await;
        let implement_current_plan = self.workflow_mode == WorkflowMode::Code
            && is_current_plan_execution_request(&effective_user_input)
            && self
                .session_memory
                .current_plan
                .as_ref()
                .map(|plan| plan.has_signal())
                .unwrap_or(false);
        let explicit_search_request = is_explicit_web_search_request(&effective_user_input);
        let mut grounded_research_results: Option<String> = None;
        self.plan_execution_active
            .store(implement_current_plan, std::sync::atomic::Ordering::SeqCst);
        let _plan_execution_guard = PlanExecutionGuard {
            flag: self.plan_execution_active.clone(),
        };
        let task_progress_before = if implement_current_plan {
            read_task_checklist_progress()
        } else {
            None
        };
        let current_plan_pass = if implement_current_plan {
            self.plan_execution_pass_depth
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1
        } else {
            0
        };
        let _plan_execution_pass_guard = implement_current_plan.then(|| PlanExecutionPassGuard {
            depth: self.plan_execution_pass_depth.clone(),
        });
        let intent = classify_query_intent(self.workflow_mode, &effective_user_input);

        // Seamless Search Handover: Transition to ASK mode if research is detected in AUTO.
        if self.workflow_mode == WorkflowMode::Auto
            && intent.primary_class == QueryIntentClass::Research
        {
            self.set_workflow_mode(WorkflowMode::Ask);
            let _ = tx
                .send(InferenceEvent::Thought(
                    "Seamless search detected: transitioning to investigation mode...".into(),
                ))
                .await;
        }

        // ── /think / /no_think: reasoning budget toggle ──────────────────────
        if let Some(answer_kind) = intent.direct_answer {
            match answer_kind {
                DirectAnswerKind::About => {
                    let response = build_about_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::LanguageCapability => {
                    let response = build_language_capability_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::UnsafeWorkflowPressure => {
                    let response = build_unsafe_workflow_pressure_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::SessionMemory => {
                    let response = build_session_memory_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::RecoveryRecipes => {
                    let response = build_recovery_recipes_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::McpLifecycle => {
                    let response = build_mcp_lifecycle_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::AuthorizationPolicy => {
                    let response = build_authorization_policy_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::ToolClasses => {
                    let response = build_tool_classes_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::ToolRegistryOwnership => {
                    let response = build_tool_registry_ownership_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::SessionResetSemantics => {
                    let response = build_session_reset_semantics_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::ProductSurface => {
                    let response = build_product_surface_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::ReasoningSplit => {
                    let response = build_reasoning_split_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::Identity => {
                    let response = build_identity_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::WorkflowModes => {
                    let response = build_workflow_modes_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::GemmaNative => {
                    let response = build_gemma_native_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::GemmaNativeSettings => {
                    let response = build_gemma_native_settings_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::VerifyProfiles => {
                    let response = build_verify_profiles_answer();
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::Toolchain => {
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
                    let response =
                        crate::tools::toolchain::describe_toolchain(&serde_json::json!({
                            "topic": topic,
                            "question": effective_user_input,
                        }))
                        .await
                        .unwrap_or_else(|e| format!("Error: {}", e));
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::HostInspection => {
                    let topics = all_host_inspection_topics(&effective_user_input);
                    let response = if topics.len() >= 2 {
                        let mut combined = Vec::new();
                        for topic in topics {
                            let args =
                                host_inspection_args_from_prompt(topic, &effective_user_input);
                            let output = crate::tools::host_inspect::inspect_host(&args)
                                .await
                                .unwrap_or_else(|e| format!("Error (topic {topic}): {e}"));
                            combined.push(format!("# Topic: {topic}\n{output}"));
                        }
                        combined.join("\n\n---\n\n")
                    } else {
                        let topic = preferred_host_inspection_topic(&effective_user_input)
                            .unwrap_or("summary");
                        let args = host_inspection_args_from_prompt(topic, &effective_user_input);
                        crate::tools::host_inspect::inspect_host(&args)
                            .await
                            .unwrap_or_else(|e| format!("Error: {e}"))
                    };

                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
                DirectAnswerKind::ArchitectSessionResetPlan => {
                    let plan = build_architect_session_reset_plan();
                    let response = plan.to_markdown();
                    let _ = crate::tools::plan::save_plan_handoff(&plan);
                    self.session_memory.current_plan = Some(plan);
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
            }
        }

        if matches!(
            self.workflow_mode,
            WorkflowMode::Ask | WorkflowMode::ReadOnly
        ) && looks_like_mutation_request(&effective_user_input)
        {
            let response = build_mode_redirect_answer(self.workflow_mode);
            self.history.push(ChatMessage::user(&effective_user_input));
            self.history.push(ChatMessage::assistant_text(&response));
            self.transcript.log_user(&transcript_user_input);
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
        if intent.sovereign_mode && is_scaffold_request(&effective_user_input) {
            if let Some(root) = extract_sovereign_scaffold_root(&effective_user_input) {
                if std::fs::create_dir_all(&root).is_ok() {
                    let targets = default_sovereign_scaffold_targets(&effective_user_input);
                    let _ = seed_sovereign_scaffold_files(&root, &targets);
                    let plan = build_sovereign_scaffold_handoff(&effective_user_input, &targets);
                    let _ = crate::tools::plan::save_plan_handoff_for_root(&root, &plan);
                    let _ = crate::tools::plan::write_teleport_resume_marker_for_root(&root);
                    let _ = write_sovereign_handoff_markdown(&root, &effective_user_input, &plan);
                    self.pending_teleport_handoff = None;
                    self.latest_target_dir = Some(root.to_string_lossy().to_string());
                    let response = format!(
                        "Created the sovereign project root at `{}` and wrote a local handoff. Teleporting now so the next session can continue implementation inside that project.",
                        root.display()
                    );
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }
            }
        }

        let tiny_context_mode = self.engine.current_context_length() <= 8_192;
        let mut base_prompt = self.engine.build_system_prompt(
            self.snark,
            self.chaos,
            self.brief,
            self.professional,
            &self.tools,
            self.reasoning_history.as_deref(),
            None,
            &mcp_tools,
        );
        if !tiny_context_mode {
            if let Some(hint) = &config.context_hint {
                if !hint.trim().is_empty() {
                    base_prompt.push_str(&format!(
                        "\n\n# Project Context (from .hematite/settings.json)\n{}",
                        hint
                    ));
                }
            }
            if let Some(profile_block) = crate::agent::workspace_profile::profile_prompt_block(
                &crate::tools::file_ops::workspace_root(),
            ) {
                base_prompt.push_str(&format!("\n\n{}", profile_block));
            }
            if let Some(strategy_block) =
                crate::agent::workspace_profile::profile_strategy_prompt_block(
                    &crate::tools::file_ops::workspace_root(),
                )
            {
                base_prompt.push_str(&format!("\n\n{}", strategy_block));
            }
            // L1: inject hot-files block if available (persists across sessions via vein.db).
            if let Some(ref l1) = self.l1_context {
                base_prompt.push_str(&format!("\n\n{}", l1));
            }
            if let Some(ref repo_map_block) = self.repo_map {
                base_prompt.push_str(&format!("\n\n{}", repo_map_block));
            }
        }
        let grounded_trace_mode = intent.grounded_trace_mode
            || intent.primary_class == QueryIntentClass::RuntimeDiagnosis;
        let capability_mode =
            intent.capability_mode || intent.primary_class == QueryIntentClass::Capability;
        let toolchain_mode =
            intent.toolchain_mode || intent.primary_class == QueryIntentClass::Toolchain;
        // Embedding-based intent veto: when the keyword router says diagnostic,
        // ask nomic-embed whether the query is actually conversational/advisory.
        // Only fires when keyword routing would have triggered HOST INSPECTION MODE.
        // Falls back to the keyword result if the embed model is unavailable or slow.
        let host_inspection_mode = if intent.host_inspection_mode {
            let api_url = self.engine.base_url.clone();
            let query = effective_user_input.clone();
            let embed_class = tokio::time::timeout(
                std::time::Duration::from_millis(600),
                crate::agent::intent_embed::classify_intent(&query, &api_url),
            )
            .await
            .unwrap_or(crate::agent::intent_embed::IntentClass::Ambiguous);
            !matches!(
                embed_class,
                crate::agent::intent_embed::IntentClass::Advisory
            )
        } else {
            false
        };
        let maintainer_workflow_mode = intent.maintainer_workflow_mode
            || preferred_maintainer_workflow(&effective_user_input).is_some();
        let fix_plan_mode =
            preferred_host_inspection_topic(&effective_user_input) == Some("fix_plan");
        let architecture_overview_mode = intent.architecture_overview_mode;
        let capability_needs_repo = intent.capability_needs_repo;
        let research_mode = intent.primary_class == QueryIntentClass::Research
            && intent.direct_answer.is_none()
            && !(capability_mode && !capability_needs_repo);
        let mut system_msg = build_system_with_corrections(
            &base_prompt,
            &self.correction_hints,
            &self.gpu_state,
            &self.git_state,
            &config,
        );
        if !tiny_context_mode && research_mode {
            system_msg.push_str(
                "\n\n# RESEARCH MODE\n\
                 This turn is an investigation into external technical information.\n\
                 Prioritize using the `research_web` tool to find the most current and authoritative data.\n\
                 When providing information, ground your answer in the search results and cite your sources if possible.\n\
                 If the user's question involves specific versions or recent releases (e.g., Rust compiler), use the web to verify the exact state.\n"
            );
        }
        if tiny_context_mode {
            system_msg.push_str(
                "\n\n# TINY CONTEXT TURN MODE\n\
                 Keep this turn compact. Prefer direct answers or one narrow tool step over broad exploration.\n",
            );
        }
        if !tiny_context_mode && grounded_trace_mode {
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
        if !tiny_context_mode && capability_mode {
            // Consolidated: Capability instructions handled by prompt.rs
        }
        if !tiny_context_mode && toolchain_mode {
            // Consolidated: Toolchain instructions handled by prompt.rs
        }
        if !tiny_context_mode && host_inspection_mode {
            // Consolidated: Host Inspection rules handled by prompt.rs
        }
        if !tiny_context_mode && fix_plan_mode {
            system_msg.push_str(
                "\n\n# FIX PLAN MODE\n\
                 This turn is a workstation remediation question, not just a diagnosis question.\n\
                 Call `inspect_host` with `topic=fix_plan` first.\n\
                 Do not start with `path`, `toolchains`, `env_doctor`, or `ports` unless the user explicitly asks for diagnosis details instead of a fix plan.\n\
                 Keep the answer grounded, stepwise, and approval-aware.\n"
            );
        }
        if !tiny_context_mode && maintainer_workflow_mode {
            system_msg.push_str(
                "\n\n# HEMATITE MAINTAINER WORKFLOW MODE\n\
                 This turn asks Hematite to run one of Hematite's own maintainer workflows, not invent an ad hoc shell command.\n\
                 Prefer `run_hematite_maintainer_workflow` for existing Hematite workflows such as `clean.ps1`, `scripts/package-windows.ps1`, or `release.ps1`.\n\
                 Use workflow `clean` for cleanup, workflow `package_windows` for rebuilding the local portable or installer, and workflow `release` for the normal version bump/tag/push/publish flow.\n\
                 Do not treat this as a generic current-workspace script runner. Only fall back to raw `shell` if the user asks for a script or command outside those Hematite maintainer workflows.\n"
            );
        }
        // Consolidated: Workspace Workflow rules handled by prompt.rs

        if !tiny_context_mode && architecture_overview_mode {
            system_msg.push_str(
                "\n\n# ARCHITECTURE OVERVIEW DISCIPLINE MODE\n\
                 For broad runtime or architecture walkthroughs, prefer authoritative tools first: `trace_runtime_flow` for control flow.\n\
                 Do not call `auto_pin_context` or `list_pinned` in read-only analysis. Avoid broad `read_file` calls unless the user explicitly asks for implementation detail in one named file.\n\
                 Preserve grounded tool output rather than restyling it into a larger answer.\n"
            );
        }

        // ── Inject Pinned Files (Context Locking) ───────────────────────────
        system_msg.push_str(&format!(
            "\n\n# WORKFLOW MODE\nCURRENT WORKFLOW: {}\n",
            self.workflow_mode.label()
        ));
        if tiny_context_mode {
            system_msg
                .push_str("Use the narrowest safe behavior for this mode. Keep the turn short.\n");
        } else {
        }
        if !tiny_context_mode && self.workflow_mode == WorkflowMode::Architect {
            system_msg.push_str("\n\n# ARCHITECT HANDOFF CONTRACT\n");
            system_msg.push_str(architect_handoff_contract());
            system_msg.push('\n');
        }
        if !tiny_context_mode && is_scaffold_request(&effective_user_input) {
            system_msg.push_str(scaffold_protocol());
        }
        if !tiny_context_mode && implement_current_plan {
            system_msg.push_str(
                "\n\n# CURRENT PLAN EXECUTION CONTRACT\n\
                 The user explicitly asked you to implement the current saved plan.\n\
                 Do not restate the plan, do not provide preliminary contracts, and do not stop at analysis.\n\
                 Use the saved plan as the brief, gather only the minimum built-in file evidence you need, then start editing the target files.\n\
                 Every file inspection or edit call must be path-scoped to one of the saved target files.\n\
                 If the saved plan explicitly calls for `research_web` or `fetch_docs`, do that research first, then return to the target files.\n\
                 If a built-in workspace read tool gives you enough context, your next step should be mutation or a concrete blocking question, not another summary.\n",
            );
            if let Some(plan) = self.session_memory.current_plan.as_ref() {
                if !plan.target_files.is_empty() {
                    system_msg.push_str("\n# CURRENT PLAN TARGET FILES\n");
                    for path in &plan.target_files {
                        system_msg.push_str(&format!("- {}\n", path));
                    }
                }
            }
        }
        if !tiny_context_mode {
            let pinned = self.pinned_files.lock().await;
            if !pinned.is_empty() {
                system_msg.push_str("\n\n# ACTIVE CONTEXT (PINNED FILES)\n");
                system_msg.push_str("The following files are locked in your active memory for prioritized reference.\n\n");
                for (path, content) in pinned.iter() {
                    system_msg.push_str(&format!("## FILE: {}\n```\n{}\n```\n\n", path, content));
                }
            }
        }
        if !tiny_context_mode {
            self.append_session_handoff(&mut system_msg);
        }
        // ── Inject TASK.md Visibility ────────────────────────────────────────
        let mut final_system_msg = if self.workflow_mode.is_chat() {
            self.build_chat_system_prompt()
        } else {
            system_msg
        };

        if !tiny_context_mode
            && matches!(self.workflow_mode, WorkflowMode::Code | WorkflowMode::Auto)
        {
            let task_path = std::path::Path::new(".hematite/TASK.md");
            if task_path.exists() {
                if let Ok(content) = std::fs::read_to_string(task_path) {
                    let snippet = if content.lines().count() > 50 {
                        content.lines().take(50).collect::<Vec<_>>().join("\n")
                            + "\n... (truncated)"
                    } else {
                        content
                    };
                    final_system_msg.push_str("\n\n# CURRENT TASK STATUS (.hematite/TASK.md)\n");
                    final_system_msg.push_str("Update this file via `edit_file` to check off `[x]` items as you complete them.\n");
                    final_system_msg.push_str("```markdown\n");
                    final_system_msg.push_str(&snippet);
                    final_system_msg.push_str("\n```\n");
                }
            }
        }

        let system_msg = final_system_msg;
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

        let is_gemma =
            crate::agent::inference::is_hematite_native_model(&self.engine.current_model());
        let user_content = match self.think_mode {
            Some(true) => format!("/think\n{}", effective_user_input),
            Some(false) => format!("/no_think\n{}", effective_user_input),
            // For non-Gemma models (Qwen etc.) default to /think so the model uses
            // hybrid thinking — it decides how much reasoning each turn needs.
            // Gemma handles reasoning via <|think|> in the system prompt instead.
            // Chat mode and quick tool calls skip /think — fast direct answers.
            None if !is_gemma
                && !self.workflow_mode.is_chat()
                && !is_quick_tool_request(&effective_user_input) =>
            {
                format!("/think\n{}", effective_user_input)
            }
            None => effective_user_input.clone(),
        };
        if let Some(image) = user_turn.attached_image.as_ref() {
            let image_url =
                crate::tools::vision::encode_image_as_data_url(std::path::Path::new(&image.path))
                    .map_err(|e| format!("Image attachment failed for {}: {}", image.name, e))?;
            self.history
                .push(ChatMessage::user_with_image(&user_content, &image_url));
        } else {
            self.history.push(ChatMessage::user(&user_content));
        }
        self.transcript.log_user(&transcript_user_input);

        // Incremental re-index and Vein context injection. Ordinary chat mode
        // still skips repo-snippet noise, but docs-only workspaces and explicit
        // session-recall prompts should keep Vein memory available.
        let vein_docs_only = self.vein_docs_only_mode();
        let allow_vein_context = !self.workflow_mode.is_chat()
            || should_use_vein_in_chat(&effective_user_input, vein_docs_only);
        let (vein_context, vein_paths) = if allow_vein_context {
            self.refresh_vein_index();
            let _ = tx
                .send(InferenceEvent::VeinStatus {
                    file_count: self.vein.file_count(),
                    embedded_count: self.vein.embedded_chunk_count(),
                    docs_only: vein_docs_only,
                })
                .await;
            match self.build_vein_context(&effective_user_input) {
                Some((ctx, paths)) => (Some(ctx), paths),
                None => (None, Vec::new()),
            }
        } else {
            (None, Vec::new())
        };
        // Reset Turn Diff Tracker for a fresh turn.
        {
            let mut tracker = self.diff_tracker.lock().await;
            tracker.reset();
        }

        // Environment Heartbeat: Capture current toolchain state.
        let heartbeat = crate::agent::policy::ToolchainHeartbeat::capture();
        self.last_heartbeat = Some(heartbeat.clone());

        if !vein_paths.is_empty() {
            let _ = tx
                .send(InferenceEvent::VeinContext { paths: vein_paths })
                .await;
        }

        // Route: pick fast vs think model based on the complexity of this request.
        let routed_model = route_model(
            &effective_user_input,
            effective_fast.as_deref(),
            effective_think.as_deref(),
        )
        .map(|s| s.to_string());

        let mut loop_intervention: Option<String> = None;

        // ── Harness pre-run: multi-topic host inspection ─────────────────────
        // When the user asks for 2+ distinct inspect_host topics in one message,
        // run them all here and inject the combined results as a loop_intervention
        // so the model receives data instead of having to orchestrate tool calls.
        // This prevents the model from collapsing multiple topics into a generic
        // one, burning the tool loop budget, or retrying via shell.
        {
            let topics = all_host_inspection_topics(&effective_user_input);
            if topics.len() >= 2 {
                let _ = tx
                    .send(InferenceEvent::Thought(format!(
                        "Harness pre-run: {} host inspection topics detected — running all before model turn.",
                        topics.len()
                    )))
                    .await;

                let topic_list = topics.join(", ");
                let mut combined = format!(
                    "## HARNESS PRE-RUN RESULTS\n\
                     The harness already ran inspect_host for the following topics: {topic_list}.\n\
                     Use the tool results in context to answer. Do NOT repeat these tool calls.\n\n"
                );

                let mut tool_calls = Vec::new();
                let mut tool_msgs = Vec::new();

                for topic in &topics {
                    let call_id = format!("prerun_{topic}");
                    let mut args_val =
                        host_inspection_args_from_prompt(topic, &effective_user_input);
                    args_val
                        .as_object_mut()
                        .unwrap()
                        .insert("max_entries".to_string(), Value::from(20));
                    let _args_str = serde_json::to_string(&args_val).unwrap_or_default();

                    tool_calls.push(crate::agent::types::ToolCallResponse {
                        id: call_id.clone(),
                        call_type: "function".to_string(),
                        function: crate::agent::types::ToolCallFn {
                            name: "inspect_host".to_string(),
                            arguments: args_val.clone(),
                        },
                        index: None,
                    });

                    let label = format!("### inspect_host(topic=\"{topic}\")\n");
                    let _ = tx
                        .send(InferenceEvent::ToolCallStart {
                            id: call_id.clone(),
                            name: "inspect_host".to_string(),
                            args: format!("inspect host {topic}"),
                        })
                        .await;

                    match crate::tools::host_inspect::inspect_host(&args_val).await {
                        Ok(out) => {
                            let _ = tx
                                .send(InferenceEvent::ToolCallResult {
                                    id: call_id.clone(),
                                    name: "inspect_host".to_string(),
                                    result: out.chars().take(300).collect::<String>() + "...",
                                    is_error: false,
                                })
                                .await;
                            combined.push_str(&label);
                            combined.push_str(&out);
                            combined.push_str("\n\n");
                            tool_msgs.push(ChatMessage::tool_result_for_model(
                                &call_id,
                                "inspect_host",
                                &out,
                                &self.engine.current_model(),
                            ));
                        }
                        Err(e) => {
                            let err_msg = format!("Error: {e}");
                            combined.push_str(&label);
                            combined.push_str(&err_msg);
                            combined.push_str("\n\n");
                            tool_msgs.push(ChatMessage::tool_result_for_model(
                                &call_id,
                                "inspect_host",
                                &err_msg,
                                &self.engine.current_model(),
                            ));
                        }
                    }
                }

                // Add the simulated turn to history so the model sees it as context.
                self.history
                    .push(ChatMessage::assistant_tool_calls("", tool_calls));
                for msg in tool_msgs {
                    self.history.push(msg);
                }

                loop_intervention = Some(combined);
            }
        }

        // ── Research Pre-Run: force research_web for entity/knowledge queries ────
        // When the intent is classified as Research, the model often skips the
        // tool call and hallucinates from training data. To prevent this, we
        // execute research_web automatically and inject the results so the model
        // has grounded web data before it even starts generating.
        if loop_intervention.is_none() && research_mode {
            // Extract a clean search query from the user input.
            let search_query = extract_explicit_web_search_query(&effective_user_input)
                .unwrap_or_else(|| effective_user_input.trim().to_string());

            let _ = tx
                .send(InferenceEvent::Thought(
                    "Research pre-run: executing search before model turn to ground the answer..."
                        .into(),
                ))
                .await;

            let call_id = "prerun_research".to_string();
            let args = serde_json::json!({ "query": search_query });

            let _ = tx
                .send(InferenceEvent::ToolCallStart {
                    id: call_id.clone(),
                    name: "research_web".to_string(),
                    args: format!("research_web: {}", search_query),
                })
                .await;

            match crate::tools::research::execute_search(&args, config.searx_url.clone()).await {
                Ok(results)
                    if !results.is_empty() && !results.contains("No search results found") =>
                {
                    grounded_research_results = Some(results.clone());
                    let _ = tx
                        .send(InferenceEvent::ToolCallResult {
                            id: call_id.clone(),
                            name: "research_web".to_string(),
                            result: results.chars().take(300).collect::<String>() + "...",
                            is_error: false,
                        })
                        .await;

                    loop_intervention = Some(format!(
                        "## RESEARCH PRE-RUN RESULTS\n\
                         The harness already ran `research_web` for your query.\n\
                         Use the search results above to answer the user's question with grounded, factual information.\n\
                         Do NOT re-run `research_web` unless you need additional detail.\n\
                         Do NOT hallucinate or guess — base your answer entirely on the search results.\n\n\
                         {}",
                        results
                    ));
                }
                Ok(_) | Err(_) => {
                    // Search returned empty or failed — let the model try on its own.
                    let _ = tx
                        .send(InferenceEvent::ToolCallResult {
                            id: call_id.clone(),
                            name: "research_web".to_string(),
                            result: "No results found — model will attempt its own search.".into(),
                            is_error: true,
                        })
                        .await;
                }
            }
        }

        // ── Computation Integrity: nudge model toward run_code for precise math ──
        // When the query involves exact numeric computation (hashes, financial math,
        // statistics, date arithmetic, unit conversions, algorithmic checks), inject
        // a brief pre-turn reminder so the model reaches for run_code instead of
        // answering from training-data memory. Only fires when no harness pre-run
        // already set a loop_intervention.
        if loop_intervention.is_none() && needs_computation_sandbox(&effective_user_input) {
            loop_intervention = Some(
                "COMPUTATION INTEGRITY NOTICE: This query involves precise numeric computation. \
                 Do NOT answer from training-data memory — memory answers for math are guesses. \
                 Use `run_code` to compute the real result and return the actual output. \
                 IMPORTANT: the `run_code` tool defaults to JavaScript (Deno). \
                 If you write Python code, you MUST pass `language: \"python\"` explicitly. \
                 If you write JavaScript/TypeScript, omit the language field or pass `language: \"javascript\"`. \
                 Write the code, run it, return the result."
                    .to_string(),
            );
        }

        // ── Native Tool Mandate: nudge model toward create_directory/write_file for local mutations ──
        if loop_intervention.is_none() && intent.surgical_filesystem_mode {
            loop_intervention = Some(
                "NATIVE TOOL MANDATE: Your request involves local directory or file creation. \
                 You MUST use Hematite's native surgical tools (`create_directory`, `write_file`, `update_file`, `patch_hunk`). \
                 External `mcp__filesystem__*` mutation tools are BLOCKED for these actions and will fail. \
                 Use `@DESKTOP/`, `@DOCUMENTS/`, or `@DOWNLOADS/` sovereign tokens for 100% path accuracy."
                    .to_string(),
            );
        }

        // ── Auto-Architect: complex scaffold requests in /auto get a plan-first nudge ──
        // When the user asks for a multi-file build in /auto mode, instruct the model
        // to draft a PLAN.md blueprint first. The plan_drafted_this_turn gate at the
        // end of run_turn will then fire the Y/N approval and chain into implementation.
        if loop_intervention.is_none()
            && self.workflow_mode == WorkflowMode::Auto
            && is_scaffold_request(&effective_user_input)
            && !implement_current_plan
        {
            loop_intervention = Some(
                "AUTO-ARCHITECT: This request involves building multiple files (a scaffold). \
                 Before implementing, draft a concise blueprint to `.hematite/PLAN.md` using `write_file`. \
                 The blueprint should list:\n\
                 1. The target directory path\n\
                 2. Each file to create (with a one-line description of its purpose)\n\
                 3. Key design decisions (e.g. color scheme, layout approach)\n\n\
                 Use `@DESKTOP/`, `@DOCUMENTS/`, or `@DOWNLOADS/` sovereign tokens for path accuracy.\n\
                 After writing the PLAN.md, respond with a brief summary of what you planned. \
                 Do NOT start implementing yet — just write the plan."
                    .to_string(),
            );
        }

        let mut implementation_started = false;
        let mut plan_drafted_this_turn = false;
        let mut non_mutating_plan_steps = 0usize;
        let non_mutating_plan_soft_cap = 5usize;
        let non_mutating_plan_hard_cap = 8usize;
        let mut overview_runtime_trace: Option<String> = None;

        // Safety cap – never spin forever on a broken model.
        let max_iters = 25;
        let mut consecutive_errors = 0;
        let mut empty_cleaned_nudges = 0u8;
        let mut first_iter = true;
        let _called_this_turn: std::collections::HashSet<String> = std::collections::HashSet::new();
        // Track identical tool results within this turn to detect logical loops.
        let _result_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        // Track the count of identical (name, args) calls to detect infinite tool loops.
        let mut repeat_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut completed_tool_cache: std::collections::HashMap<String, CachedToolResult> =
            std::collections::HashMap::new();
        let mut successful_read_targets: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        // (path, offset) pairs — catches repeated reads at the same non-zero offset.
        let mut successful_read_regions: std::collections::HashSet<(String, u64)> =
            std::collections::HashSet::new();
        let mut successful_grep_targets: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut no_match_grep_targets: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut broad_grep_targets: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut sovereign_task_root: Option<String> = None;
        let mut sovereign_scaffold_targets: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        let mut turn_mutated_paths: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        let mut mutation_counts_by_path: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut frontend_polish_intervention_emitted = false;
        let mut visible_closeout_emitted = false;

        // Track the index of the message that started THIS turn, so compaction doesn't summarize it.
        let mut turn_anchor = self.history.len().saturating_sub(1);

        for _iter in 0..max_iters {
            let context_prep_start = tokio::time::Instant::now();
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

            // On the first iteration inject Vein context into the system message.
            // Subsequent iterations use the plain slice — tool results are now in
            // history so Vein context would be redundant.
            let inject_vein = first_iter && !implement_current_plan;
            let messages = if implement_current_plan {
                first_iter = false;
                self.context_window_slice_from(turn_anchor)
            } else {
                first_iter = false;
                self.context_window_slice()
            };

            // Use the canonical system prompt from history[0] which was built
            // by InferenceEngine::build_system_prompt() + build_system_with_corrections()
            // and includes GPU state, git context, permissions, and instruction files.
            let mut prompt_msgs = if let Some(intervention) = loop_intervention.take() {
                // Gemma 4 handles multiple system messages natively.
                // Standard models (Qwen, etc.) reject a second system message — merge into history[0].
                if crate::agent::inference::is_hematite_native_model(&self.engine.current_model()) {
                    let mut msgs = vec![self.history[0].clone()];
                    msgs.push(ChatMessage::system(&intervention));
                    msgs
                } else {
                    let merged =
                        format!("{}\n\n{}", self.history[0].content.as_str(), intervention);
                    vec![ChatMessage::system(&merged)]
                }
            } else {
                vec![self.history[0].clone()]
            };

            // Inject Vein context into the system message on the first iteration.
            // Vein results are merged in the same way as loop_intervention so standard
            // models (Qwen etc.) only ever see one system message.
            if inject_vein {
                if let Some(ctx) = vein_context.as_deref() {
                    if crate::agent::inference::is_hematite_native_model(
                        &self.engine.current_model(),
                    ) {
                        prompt_msgs.push(ChatMessage::system(ctx));
                    } else {
                        let merged = format!("{}\n\n{}", prompt_msgs[0].content.as_str(), ctx);
                        prompt_msgs[0] = ChatMessage::system(&merged);
                    }
                }
            }
            if let Some(root) = sovereign_task_root.as_ref() {
                let sovereign_root_instruction = format!(
                    "EFFECTIVE TASK ROOT: This sovereign scaffold turn is now rooted at:\n\
                     `{root}`\n\n\
                     Treat that directory as the active project root for the rest of this turn. \
                     All reads, writes, verification, and summaries must stay scoped to that root. \
                     Ignore unrelated repo context such as `./src` unless the user explicitly asks about it. \
                     Keep building within this sovereign root instead of reasoning from the original workspace."
                );
                if crate::agent::inference::is_hematite_native_model(&self.engine.current_model()) {
                    prompt_msgs.push(ChatMessage::system(&sovereign_root_instruction));
                } else {
                    let merged = format!(
                        "{}\n\n{}",
                        prompt_msgs[0].content.as_str(),
                        sovereign_root_instruction
                    );
                    prompt_msgs[0] = ChatMessage::system(&merged);
                }
            }
            prompt_msgs.extend(messages);
            if let Some(budget_note) =
                enforce_prompt_budget(&mut prompt_msgs, self.engine.current_context_length())
            {
                self.emit_operator_checkpoint(
                    &tx,
                    OperatorCheckpointState::BudgetReduced,
                    budget_note,
                )
                .await;
                let recipe = plan_recovery(
                    RecoveryScenario::PromptBudgetPressure,
                    &self.recovery_context,
                );
                self.emit_recovery_recipe_summary(
                    &tx,
                    recipe.recipe.scenario.label(),
                    compact_recovery_plan_summary(&recipe),
                )
                .await;
            }
            self.emit_prompt_pressure_for_messages(&tx, &prompt_msgs)
                .await;

            let turn_tools = if yolo
                || (explicit_search_request && grounded_research_results.is_some())
            {
                // FORCE NLG ONLY: Hide all tools to ensure a plain text summary.
                Vec::new()
            } else if intent.sovereign_mode {
                self.tools
                    .iter()
                    .filter(|t| {
                        t.function.name != "shell" && t.function.name != "run_workspace_workflow"
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                self.tools.clone()
            };

            let context_prep_ms = context_prep_start.elapsed().as_millis();
            let inference_start = tokio::time::Instant::now();

            let (mut text, mut tool_calls, usage, finish_reason) = match self
                .engine
                .call_with_tools(&prompt_msgs, &turn_tools, routed_model.as_deref())
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    let class = classify_runtime_failure(&e);
                    if should_retry_runtime_failure(class) {
                        if self.recovery_context.consume_transient_retry() {
                            let label = match class {
                                RuntimeFailureClass::ProviderDegraded => "provider_degraded",
                                _ => "empty_model_response",
                            };
                            self.transcript.log_system(&format!(
                                "Automatic provider recovery triggered: {}",
                                e.trim()
                            ));
                            self.emit_recovery_recipe_summary(
                                &tx,
                                label,
                                compact_runtime_recovery_summary(class),
                            )
                            .await;
                            let _ = tx
                                .send(InferenceEvent::ProviderStatus {
                                    state: ProviderRuntimeState::Recovering,
                                    summary: compact_runtime_recovery_summary(class).into(),
                                })
                                .await;
                            self.emit_operator_checkpoint(
                                &tx,
                                OperatorCheckpointState::RecoveringProvider,
                                compact_runtime_recovery_summary(class),
                            )
                            .await;
                            continue;
                        }
                    }

                    if explicit_search_request
                        && matches!(
                            class,
                            RuntimeFailureClass::ProviderDegraded
                                | RuntimeFailureClass::EmptyModelResponse
                        )
                    {
                        if let Some(results) = grounded_research_results.as_deref() {
                            let response = build_research_provider_fallback(results);
                            self.history.push(ChatMessage::assistant_text(&response));
                            self.transcript.log_agent(&response);
                            for chunk in chunk_text(&response, 8) {
                                let _ = tx.send(InferenceEvent::Token(chunk)).await;
                            }
                            let _ = tx.send(InferenceEvent::Done).await;
                            return Ok(());
                        }
                    }

                    self.emit_runtime_failure(&tx, class, &e).await;
                    break;
                }
            };
            let inference_ms = inference_start.elapsed().as_millis();
            let execution_start = tokio::time::Instant::now();
            self.emit_provider_live(&tx).await;

            // ── LOOP GUARD: Reasoning Collapse Detection ──────────────────────────
            // If the model returns no text AND no tool calls, but has a massive
            // block of hidden reasoning (often seen as infinite newlines in small models),
            // trigger a safety stop to prevent token drain.
            if text.is_none() && tool_calls.is_none() {
                if let Some(reasoning) = usage.as_ref().and_then(|u| {
                    if u.completion_tokens > 2000 {
                        Some(u.completion_tokens)
                    } else {
                        None
                    }
                }) {
                    self.emit_operator_checkpoint(
                        &tx,
                        OperatorCheckpointState::BlockedToolLoop,
                        format!(
                            "Reasoning collapse detected ({} tokens of empty output).",
                            reasoning
                        ),
                    )
                    .await;
                    break;
                }
            }

            // Update TUI token counter with actual usage from LM Studio.
            if let Some(ref u) = usage {
                let _ = tx.send(InferenceEvent::UsageUpdate(u.clone())).await;
            }

            // Fallback safety net: if native tool markup leaked past the inference-layer
            // extractor, recover it here instead of treating it as plain assistant text.
            if tool_calls
                .as_ref()
                .map(|calls| calls.is_empty())
                .unwrap_or(true)
            {
                if let Some(raw_text) = text.as_deref() {
                    let native_calls = crate::agent::inference::extract_native_tool_calls(raw_text);
                    if !native_calls.is_empty() {
                        tool_calls = Some(native_calls);
                        let stripped =
                            crate::agent::inference::strip_native_tool_call_text(raw_text);
                        text = if stripped.trim().is_empty() {
                            None
                        } else {
                            Some(stripped)
                        };
                    }
                }
            }

            // Treat empty tool_calls arrays (Some(vec![])) the same as None –
            // the model returned text only; an empty array causes an infinite loop.
            let tool_calls = tool_calls.filter(|c| !c.is_empty());
            let near_context_ceiling = usage
                .as_ref()
                .map(|u| u.prompt_tokens >= (self.engine.current_context_length() * 82 / 100))
                .unwrap_or(false);

            if let Some(calls) = tool_calls {
                let (calls, prune_trace_note) =
                    prune_architecture_trace_batch(calls, architecture_overview_mode);
                if let Some(note) = prune_trace_note {
                    let _ = tx.send(InferenceEvent::Thought(note)).await;
                }

                let (calls, prune_bloat_note) = prune_read_only_context_bloat_batch(
                    calls,
                    self.workflow_mode.is_read_only(),
                    architecture_overview_mode,
                );
                if let Some(note) = prune_bloat_note {
                    let _ = tx.send(InferenceEvent::Thought(note)).await;
                }

                let (calls, prune_note) = prune_authoritative_tool_batch(
                    calls,
                    grounded_trace_mode,
                    &effective_user_input,
                );
                if let Some(note) = prune_note {
                    let _ = tx.send(InferenceEvent::Thought(note)).await;
                }

                let (calls, prune_redir_note) = prune_redirected_shell_batch(calls);
                if let Some(note) = prune_redir_note {
                    let _ = tx.send(InferenceEvent::Thought(note)).await;
                }

                let (calls, batch_note) = order_batch_reads_first(calls);
                if let Some(note) = batch_note {
                    let _ = tx.send(InferenceEvent::Thought(note)).await;
                }

                if let Some(repeated_path) = calls
                    .iter()
                    .filter_map(|c| repeated_read_target(&c.function))
                    .find(|path| successful_read_targets.contains(path))
                {
                    let repeated_path = repeated_path.to_string();

                    let err_msg = format!(
                        "Read discipline: You already read `{}` recently. Use `inspect_lines` on a specific window or `grep_files` to find content, then continue with your edit.",
                        repeated_path
                    );
                    let _ = tx
                        .clone()
                        .send(InferenceEvent::Token(format!("\n⚠️ {}\n", err_msg)))
                        .await;
                    let _ = tx
                        .clone()
                        .send(InferenceEvent::Thought(format!(
                            "Intervention: {}",
                            err_msg
                        )))
                        .await;

                    // BREAK THE SILENT LOOP: Push hard errors for these tool calls individually.
                    // This forces the LLM to see the result and pivot in its next turn.
                    for call in &calls {
                        self.history.push(ChatMessage::tool_result_for_model(
                            &call.id,
                            &call.function.name,
                            &err_msg,
                            &self.engine.current_model(),
                        ));
                    }
                    self.emit_done_events(&tx).await;
                    return Ok(());
                }

                if capability_mode
                    && !capability_needs_repo
                    && calls
                        .iter()
                        .all(|c| is_capability_probe_tool(&c.function.name))
                {
                    loop_intervention = Some(
                        "STOP. This is a stable capability question. Do not inspect the repository or call tools. \
                         Answer directly from verified Hematite capabilities, current runtime state, and the documented product boundary. \
                         Do not mention raw `mcp__*` names unless they are active and directly relevant."
                            .to_string(),
                    );
                    let _ = tx.clone()
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
                    let _ = tx
                        .clone()
                        .send(InferenceEvent::Thought(thought.clone()))
                        .await;
                    // Reasoning is silent (hidden in SPECULAR only).
                    self.reasoning_history = Some(thought);
                }

                // [Gemma-4 Protocol] Keep raw content (including thoughts) during tool loops.
                // Thoughts are only stripped before the 'final' user turn.
                let stored_tool_call_content = if implement_current_plan {
                    cap_output(raw_content, 1200)
                } else {
                    raw_content.to_string()
                };
                self.history.push(ChatMessage::assistant_tool_calls(
                    &stored_tool_call_content,
                    calls.clone(),
                ));

                // ── LAYER 4: Parallel Tool Orchestration (Batching) ────────────────────
                let mut results = Vec::new();
                let gemma4_model =
                    crate::agent::inference::is_hematite_native_model(&self.engine.current_model());
                let latest_user_prompt = self.latest_user_prompt();
                let mut seen_call_keys = std::collections::HashSet::new();
                let mut deduped_calls = Vec::new();
                for call in calls.clone() {
                    let (normalized_name, normalized_args) = normalized_tool_call_for_execution(
                        &call.function.name,
                        &call.function.arguments,
                        gemma4_model,
                        latest_user_prompt,
                    );

                    // Authoritative Diff Tracking: Capture baseline before mutation.
                    if crate::agent::policy::is_destructive_tool(&normalized_name) {
                        if let Some(path) = crate::agent::policy::tool_path_argument(
                            &normalized_name,
                            &normalized_args,
                        ) {
                            let tracker = self.diff_tracker.clone();
                            tokio::spawn(async move {
                                let mut guard = tracker.lock().await;
                                let _ = guard.on_file_access(std::path::Path::new(&path));
                            });
                        }
                    }

                    // --- HALLUCINATION SANITIZER ---
                    if normalized_name == "shell" || normalized_name == "run_workspace_workflow" {
                        let cmd_val = normalized_args
                            .get("command")
                            .or_else(|| normalized_args.get("workflow"));

                        if let Some(cmd) = cmd_val.and_then(|v| v.as_str()) {
                            if cfg!(windows)
                                && (cmd.contains("/dev/")
                                    || cmd.contains("/etc/")
                                    || cmd.contains("/var/"))
                            {
                                let err_msg = "STRICT: You are attempting to use Linux system paths (/dev, /etc, /var) on a Windows host. This is a reasoning collapse. Use relative paths within your workspace only.";
                                let _ = tx
                                    .clone()
                                    .send(InferenceEvent::Token(format!("\n🚨 {}\n", err_msg)))
                                    .await;
                                let _ = tx
                                    .clone()
                                    .send(InferenceEvent::Thought(format!(
                                        "Panic blocked: {}",
                                        err_msg
                                    )))
                                    .await;

                                // BREAK THE COLLAPSE: Push hard errors for all tool calls in this batch and end turn.
                                let mut err_results = Vec::new();
                                for c in &calls {
                                    err_results.push(ChatMessage::tool_result_for_model(
                                        &c.id,
                                        &c.function.name,
                                        err_msg,
                                        &self.engine.current_model(),
                                    ));
                                }
                                for res in err_results {
                                    self.history.push(res);
                                }
                                self.emit_done_events(&tx).await;
                                return Ok(());
                            }

                            if is_natural_language_hallucination(cmd) {
                                let err_msg = format!(
                                    "HALLUCINATION BLOCKED: You tried to pass natural language ('{}') into a command field. \
                                     Commands must be literal executables (e.g. `npm install`, `mkdir path`). \
                                     Use the correct surgical tool (like `create_directory`) instead of overthinking.",
                                    cmd
                                );
                                let _ = tx
                                    .send(InferenceEvent::Thought(format!(
                                        "Sanitizer error: {}",
                                        err_msg
                                    )))
                                    .await;
                                results.push(ToolExecutionOutcome {
                                    call_id: call.id.clone(),
                                    tool_name: normalized_name.clone(),
                                    args: normalized_args.clone(),
                                    output: err_msg,
                                    is_error: true,
                                    blocked_by_policy: false,
                                    msg_results: Vec::new(),
                                    latest_target_dir: None,
                                    plan_drafted_this_turn: false,
                                    parsed_plan_handoff: None,
                                });
                                continue;
                            }
                        }
                    }

                    let key = canonical_tool_call_key(&normalized_name, &normalized_args);
                    if seen_call_keys.insert(key) {
                        let repeat_guard_exempt = matches!(
                            normalized_name.as_str(),
                            "verify_build" | "git_commit" | "git_push"
                        );
                        if !repeat_guard_exempt {
                            if let Some(cached) = completed_tool_cache
                                .get(&canonical_tool_call_key(&normalized_name, &normalized_args))
                            {
                                let _ = tx
                                    .send(InferenceEvent::Thought(
                                        "Cached tool result reused: identical built-in invocation already completed earlier in this turn."
                                            .to_string(),
                                    ))
                                    .await;
                                loop_intervention = Some(format!(
                                    "STOP. You already called `{}` with identical arguments earlier in this turn and already have that result in conversation history. Do not call it again. Use the existing result to answer or choose a different next step.",
                                    cached.tool_name
                                ));
                                continue;
                            }
                        }
                        deduped_calls.push(call);
                    } else {
                        let _ = tx
                            .send(InferenceEvent::Thought(
                                "Duplicate tool call skipped: identical built-in invocation already ran this turn."
                                    .to_string(),
                            ))
                            .await;
                    }
                }

                // Phase 5: Calculate predictive token budget for this turn's tool responses.
                // We reserve 3000 tokens for the final summary and the bootstrap context of the next turn.
                let total_used = usage.as_ref().map(|u| u.total_tokens).unwrap_or(0);
                let ctx_len = self.engine.current_context_length();
                let remaining = ctx_len.saturating_sub(total_used);
                let tool_budget = remaining.saturating_sub(3000);
                let budget_per_call = if deduped_calls.is_empty() {
                    0
                } else {
                    tool_budget / deduped_calls.len().max(1)
                };

                // Partition tool calls: Parallel Read vs Serial Mutating
                let (parallel_calls, serial_calls): (Vec<_>, Vec<_>) = deduped_calls
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
                            budget_per_call,
                        ));
                    }
                    // Wait for all read-only tasks to complete simultaneously.
                    results.extend(futures::future::join_all(tasks).await);
                }

                // 2. Sequential Execution (SerialMutating)
                let mut sovereign_bootstrap_complete = false;

                for call in serial_calls {
                    let outcome = self
                        .process_tool_call(
                            call.function,
                            config.clone(),
                            yolo,
                            tx.clone(),
                            call.id,
                            budget_per_call,
                        )
                        .await;

                    if !outcome.is_error {
                        let tool_name = outcome.tool_name.as_str();
                        if matches!(
                            tool_name,
                            "patch_hunk" | "write_file" | "edit_file" | "multi_search_replace"
                        ) {
                            if let Some(target) = action_target_path(tool_name, &outcome.args) {
                                let normalized_path = normalize_workspace_path(&target);
                                let rewrite_count = mutation_counts_by_path
                                    .entry(normalized_path.clone())
                                    .and_modify(|count| *count += 1)
                                    .or_insert(1);

                                let is_frontend_asset = [
                                    ".html", ".htm", ".css", ".js", ".ts", ".jsx", ".tsx", ".vue",
                                    ".svelte",
                                ]
                                .iter()
                                .any(|ext| normalized_path.ends_with(ext));

                                if is_frontend_asset && *rewrite_count >= 3 {
                                    frontend_polish_intervention_emitted = true;
                                    loop_intervention = Some(format!(
                                        "REWRITE LIMIT REACHED. You have updated `{}` {} times this turn. To prevent reasoning collapse, further rewrites to this file are blocked. \
                                         Please UPDATE `.hematite/TASK.md` to check off these completed steps, and response with a concise engineering summary of the implementation status.",
                                        normalized_path, rewrite_count
                                    ));
                                    results.push(outcome);
                                    let _ = tx.send(InferenceEvent::Thought("Frontend rewrite guard: block reached — prompting for task update and summary.".to_string())).await;
                                    break; // Terminate this turn's tool execution immediately.
                                } else if !frontend_polish_intervention_emitted
                                    && is_frontend_asset
                                    && *rewrite_count >= 2
                                {
                                    frontend_polish_intervention_emitted = true;
                                    loop_intervention = Some(format!(
                                        "STOP REWRITING. You have already written `{}` {} times. The current version is sufficient as a foundation. \
                                         Do NOT use `write_file` on this file again. Instead, check off your completed steps in `.hematite/TASK.md` and move on to the next file or provide your final summary.",
                                        normalized_path, rewrite_count
                                    ));
                                    results.push(outcome);
                                    let _ = tx.send(InferenceEvent::Thought("Frontend polish guard: repeated rewrite detected; prompting for progress log and next steps.".to_string())).await;
                                    break; // Terminate this turn's tool execution immediately.
                                }
                            }
                        }
                    }

                    if !outcome.is_error
                        && intent.sovereign_mode
                        && is_scaffold_request(&effective_user_input)
                        && outcome.latest_target_dir.is_some()
                    {
                        sovereign_bootstrap_complete = true;
                    }
                    results.push(outcome);
                    if sovereign_bootstrap_complete {
                        let _ = tx
                            .send(InferenceEvent::Thought(
                                "Sovereign scaffold bootstrap complete: stopping this session after root setup so the resumed session can continue inside the new project."
                                    .to_string(),
                            ))
                            .await;
                        break;
                    }
                }

                let execution_ms = execution_start.elapsed().as_millis();
                let _ = tx
                    .send(InferenceEvent::TurnTiming {
                        context_prep_ms: context_prep_ms as u128,
                        inference_ms: inference_ms as u128,
                        execution_ms: execution_ms as u128,
                    })
                    .await;

                // 3. Collate Messages into History & UI
                let mut authoritative_tool_output: Option<String> = None;
                let mut blocked_policy_output: Option<String> = None;
                let mut recoverable_policy_intervention: Option<String> = None;
                let mut recoverable_policy_recipe: Option<RecoveryScenario> = None;
                let mut recoverable_policy_checkpoint: Option<(OperatorCheckpointState, String)> =
                    None;
                for res in results {
                    let call_id = res.call_id.clone();
                    let tool_name = res.tool_name.clone();
                    let final_output = res.output.clone();
                    let is_error = res.is_error;
                    for msg in res.msg_results {
                        self.history.push(msg);
                    }

                    // Update State for Verification Loop
                    if let Some(path) = res.latest_target_dir {
                        if intent.sovereign_mode && sovereign_task_root.is_none() {
                            sovereign_task_root = Some(path.clone());
                            self.pending_teleport_handoff = Some(SovereignTeleportHandoff {
                                root: path.clone(),
                                plan: build_sovereign_scaffold_handoff(
                                    &effective_user_input,
                                    &sovereign_scaffold_targets,
                                ),
                            });
                            let _ = tx
                                .send(InferenceEvent::Thought(format!(
                                    "Sovereign scaffold root established at `{}`; rebinding project context there for the rest of this turn.",
                                    path
                                )))
                                .await;
                        }
                        self.latest_target_dir = Some(path);
                    }

                    if intent.sovereign_mode && is_scaffold_request(&effective_user_input) {
                        if let Some(root) = sovereign_task_root.as_ref() {
                            if let Some(path) = res.args.get("path").and_then(|v| v.as_str()) {
                                let resolved = crate::tools::file_ops::resolve_candidate(path);
                                let root_path = std::path::Path::new(root);
                                if let Ok(relative) = resolved.strip_prefix(root_path) {
                                    if !relative.as_os_str().is_empty() {
                                        sovereign_scaffold_targets
                                            .insert(relative.to_string_lossy().replace('\\', "/"));
                                    }
                                    self.pending_teleport_handoff =
                                        Some(SovereignTeleportHandoff {
                                            root: root.clone(),
                                            plan: build_sovereign_scaffold_handoff(
                                                &effective_user_input,
                                                &sovereign_scaffold_targets,
                                            ),
                                        });
                                }
                            }
                        }
                    }
                    if matches!(
                        tool_name.as_str(),
                        "patch_hunk" | "write_file" | "edit_file" | "multi_search_replace"
                    ) {
                        mutation_occurred = true;
                        implementation_started = true;
                        if !is_error {
                            if let Some(target) = action_target_path(&tool_name, &res.args) {
                                turn_mutated_paths.insert(target);
                            }
                        }
                        // Heat tracking: bump L1 score for the edited file.
                        if !is_error {
                            let path = res.args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                            if !path.is_empty() {
                                self.vein.bump_heat(path);
                                self.l1_context = self.vein.l1_context();
                                // Compact stale read_file results for this path — the file
                                // just changed so old content is wrong and wastes context.
                                compact_stale_reads(&mut self.history, path);
                            }
                            // Refresh repo map so PageRank accounts for the new edit.
                            self.refresh_repo_map();
                        }
                    }

                    if !is_error
                        && matches!(
                            tool_name.as_str(),
                            "patch_hunk" | "write_file" | "edit_file" | "multi_search_replace"
                        )
                    {
                        // Internal mutation counts now handled early in sequential loop.
                    }

                    if res.plan_drafted_this_turn {
                        plan_drafted_this_turn = true;
                    }
                    if let Some(plan) = res.parsed_plan_handoff.clone() {
                        self.session_memory.current_plan = Some(plan);
                    }

                    if tool_name == "verify_build" {
                        self.record_session_verification(
                            !is_error
                                && (final_output.contains("BUILD OK")
                                    || final_output.contains("BUILD SUCCESS")
                                    || final_output.contains("BUILD OKAY")),
                            if is_error {
                                "Explicit verify_build failed."
                            } else {
                                "Explicit verify_build passed."
                            },
                        );
                    }

                    // Update Repeat Guard
                    let call_key = format!(
                        "{}:{}",
                        tool_name,
                        serde_json::to_string(&res.args).unwrap_or_default()
                    );
                    let repeat_count = repeat_counts.entry(call_key.clone()).or_insert(0);
                    *repeat_count += 1;

                    // Structured verification and commit tools are legitimately called multiple
                    // times in fix-verify loops.
                    let repeat_guard_exempt =
                        is_repeat_guard_exempt_tool_call(&tool_name, &res.args);
                    if *repeat_count >= 2 && !repeat_guard_exempt {
                        loop_intervention = Some(format!(
                            "STOP. You have called `{}` with identical arguments {} times and keep getting the same result. \
                             Do not call it again. Either answer directly from what you already know, \
                             use a different tool or approach (e.g. if reading the same file, use grep or LSP symbols instead), \
                             or ask the user for clarification.",
                            tool_name, *repeat_count
                        ));
                        let _ = tx
                            .send(InferenceEvent::Thought(format!(
                                "Repeat guard: `{}` called {} times with same args — injecting stop intervention.",
                                tool_name, *repeat_count
                            )))
                            .await;
                    }

                    if *repeat_count >= 3 && !repeat_guard_exempt {
                        self.emit_runtime_failure(
                            &tx,
                            RuntimeFailureClass::ToolLoop,
                            &format!(
                                "STRICT: You are stuck in a reasoning loop calling `{}`. \
                                STOP repeating this call. Switch to grounded filesystem tools \
                                (like `read_file`, `inspect_lines`, or `edit_file`) instead of \
                                attempting this workflow again.",
                                tool_name
                            ),
                        )
                        .await;
                        return Ok(());
                    }

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
                        self.emit_runtime_failure(
                            &tx,
                            RuntimeFailureClass::ToolLoop,
                            "Hard termination: too many consecutive tool errors.",
                        )
                        .await;
                        return Ok(());
                    }

                    let _ = tx
                        .send(InferenceEvent::ToolCallResult {
                            id: call_id.clone(),
                            name: tool_name.clone(),
                            result: final_output.clone(),
                            is_error,
                        })
                        .await;

                    let repeat_guard_exempt = matches!(
                        tool_name.as_str(),
                        "verify_build" | "git_commit" | "git_push"
                    );
                    if !repeat_guard_exempt {
                        completed_tool_cache.insert(
                            canonical_tool_call_key(&tool_name, &res.args),
                            CachedToolResult {
                                tool_name: tool_name.clone(),
                            },
                        );
                    }

                    // Cap output before history
                    let compact_ctx = crate::agent::inference::is_compact_context_window_pub(
                        self.engine.current_context_length(),
                    );
                    let capped = if implement_current_plan {
                        cap_output(&final_output, 1200)
                    } else if compact_ctx
                        && (tool_name == "read_file" || tool_name == "inspect_lines")
                    {
                        // Compact context: cap file reads tightly and add a navigation hint on truncation.
                        let limit = 3000usize;
                        if final_output.len() > limit {
                            let total_lines = final_output.lines().count();
                            let mut split_at = limit;
                            while !final_output.is_char_boundary(split_at) && split_at > 0 {
                                split_at -= 1;
                            }
                            let scratch = write_output_to_scratch(&final_output, &tool_name)
                                .map(|p| format!(" Full file also saved to '{p}'."))
                                .unwrap_or_default();
                            format!(
                                "{}\n... [file truncated — {} total lines. Use `inspect_lines` with start_line near {} to reach the end of the file.{}]",
                                &final_output[..split_at],
                                total_lines,
                                total_lines.saturating_sub(150),
                                scratch,
                            )
                        } else {
                            final_output.clone()
                        }
                    } else {
                        cap_output_for_tool(&final_output, 8000, &tool_name)
                    };
                    self.history.push(ChatMessage::tool_result_for_model(
                        &call_id,
                        &tool_name,
                        &capped,
                        &self.engine.current_model(),
                    ));

                    if architecture_overview_mode && !is_error && tool_name == "trace_runtime_flow"
                    {
                        overview_runtime_trace =
                            Some(summarize_runtime_trace_output(&final_output));
                    }

                    if !architecture_overview_mode
                        && !is_error
                        && ((grounded_trace_mode && tool_name == "trace_runtime_flow")
                            || (toolchain_mode && tool_name == "describe_toolchain"))
                    {
                        authoritative_tool_output = Some(final_output.clone());
                    }

                    if !is_error && tool_name == "read_file" {
                        if let Some(path) = res.args.get("path").and_then(|v| v.as_str()) {
                            let normalized = normalize_workspace_path(path);
                            let read_offset =
                                res.args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
                            successful_read_targets.insert(normalized.clone());
                            successful_read_regions.insert((normalized.clone(), read_offset));
                        }
                    }

                    if !is_error && tool_name == "grep_files" {
                        if let Some(path) = res.args.get("path").and_then(|v| v.as_str()) {
                            let normalized = normalize_workspace_path(path);
                            if final_output.starts_with("No matches for ") {
                                no_match_grep_targets.insert(normalized);
                            } else if grep_output_is_high_fanout(&final_output) {
                                broad_grep_targets.insert(normalized);
                            } else {
                                successful_grep_targets.insert(normalized);
                            }
                        }
                    }

                    if is_error
                        && matches!(tool_name.as_str(), "edit_file" | "multi_search_replace")
                        && (final_output.contains("search string not found")
                            || final_output.contains("search string is too short")
                            || final_output.contains("search string matched"))
                    {
                        if let Some(target) = action_target_path(&tool_name, &res.args) {
                            let guidance = if final_output.contains("matched") {
                                // Multiple matches — need a more specific anchor. Show the
                                // file so the model can pick a unique surrounding context.
                                let snippet = read_file_preview_for_retry(&target, 120);
                                format!(
                                    "EDIT FAILED — search string matched multiple locations in `{target}`. \
                                     You need a longer, more unique search string that includes surrounding context.\n\
                                     Current file content (first 120 lines):\n```\n{snippet}\n```\n\
                                     Retry `{tool_name}` with a search string that is unique in the file."
                                )
                            } else {
                                // Text not found — show the full file so the model can copy
                                // the exact current text and retry with correct whitespace.
                                let snippet = read_file_preview_for_retry(&target, 200);
                                // Also register the file as observed so action_grounding
                                // won't block the retry edit.
                                let normalized = normalize_workspace_path(&target);
                                {
                                    let mut ag = self.action_grounding.lock().await;
                                    let turn = ag.turn_index;
                                    ag.observed_paths.insert(normalized.clone(), turn);
                                    ag.inspected_paths.insert(normalized, turn);
                                }
                                format!(
                                    "EDIT FAILED — search string did not match any text in `{target}`.\n\
                                     The model must have generated text that differs from what is actually in the file \
                                     (wrong whitespace, indentation, or stale content).\n\
                                     Current file content (up to 200 lines shown):\n```\n{snippet}\n```\n\
                                     Find the exact line(s) to change above, copy the text character-for-character \
                                     (preserving indentation), and immediately retry `{tool_name}` \
                                     with that exact text as the search string. Do NOT call read_file again — \
                                     the content is already shown above."
                                )
                            };
                            loop_intervention = Some(guidance);
                            *repeat_count = 0;
                        }
                    }

                    // When guard.rs blocks a shell call with the run_code redirect hint,
                    // force the model to recover with run_code instead of giving up.
                    if is_error
                        && tool_name == "shell"
                        && final_output.contains("Use the run_code tool instead")
                        && loop_intervention.is_none()
                    {
                        loop_intervention = Some(
                            "STOP. Shell was blocked because this is a computation task. \
                             You MUST use `run_code` now — write the code and run it. \
                             Do NOT output an error message or give up. \
                             Call `run_code` with the appropriate language and code to compute the answer. \
                             If writing Python, pass `language: \"python\"`. \
                             If writing JavaScript, omit language or pass `language: \"javascript\"`."
                                .to_string(),
                        );
                    }

                    // When run_code fails with a Deno parse error, the model likely sent Python
                    // code without specifying language: "python". Force a corrective retry.
                    if is_error
                        && tool_name == "run_code"
                        && (final_output.contains("source code could not be parsed")
                            || final_output.contains("Expected ';'")
                            || final_output.contains("Expected '}'")
                            || final_output.contains("is not defined")
                                && final_output.contains("deno"))
                        && loop_intervention.is_none()
                    {
                        loop_intervention = Some(
                            "STOP. run_code failed with a JavaScript parse error — you likely wrote Python \
                             code but forgot to pass `language: \"python\"`. \
                             Retry run_code with `language: \"python\"` and the same code. \
                             Do NOT fall back to shell. Do NOT give up."
                                .to_string(),
                        );
                    }

                    if res.blocked_by_policy
                        && is_mcp_workspace_read_tool(&tool_name)
                        && recoverable_policy_intervention.is_none()
                    {
                        recoverable_policy_intervention = Some(
                            "STOP. MCP filesystem reads are blocked. Use `read_file` or `inspect_lines` instead.".to_string(),
                        );
                        recoverable_policy_recipe = Some(RecoveryScenario::McpWorkspaceReadBlocked);
                        recoverable_policy_checkpoint = Some((
                            OperatorCheckpointState::BlockedPolicy,
                            "MCP workspace read blocked; rerouting to built-in file tools."
                                .to_string(),
                        ));
                    } else if res.blocked_by_policy
                        && implement_current_plan
                        && is_current_plan_irrelevant_tool(&tool_name)
                        && recoverable_policy_intervention.is_none()
                    {
                        recoverable_policy_intervention = Some(format!(
                            "STOP. `{}` is not a planned target. Use `inspect_lines` on a planned file, then edit.",
                            tool_name
                        ));
                        recoverable_policy_recipe = Some(RecoveryScenario::CurrentPlanScopeBlocked);
                        recoverable_policy_checkpoint = Some((
                            OperatorCheckpointState::BlockedPolicy,
                            format!(
                                "Current-plan execution blocked unrelated tool `{}`.",
                                tool_name
                            ),
                        ));
                    } else if res.blocked_by_policy
                        && implement_current_plan
                        && final_output.contains("requires recent file evidence")
                        && recoverable_policy_intervention.is_none()
                    {
                        let target = action_target_path(&tool_name, &res.args)
                            .unwrap_or_else(|| "the target file".to_string());
                        recoverable_policy_intervention = Some(format!(
                            "STOP. Edit blocked — `{target}` has no recent read. Use `inspect_lines` or `read_file` on it first, then retry."
                        ));
                        recoverable_policy_recipe =
                            Some(RecoveryScenario::RecentFileEvidenceMissing);
                        recoverable_policy_checkpoint = Some((
                            OperatorCheckpointState::BlockedRecentFileEvidence,
                            format!("Edit blocked on `{target}`; recent file evidence missing."),
                        ));
                    } else if res.blocked_by_policy
                        && implement_current_plan
                        && final_output.contains("requires an exact local line window first")
                        && recoverable_policy_intervention.is_none()
                    {
                        let target = action_target_path(&tool_name, &res.args)
                            .unwrap_or_else(|| "the target file".to_string());
                        recoverable_policy_intervention = Some(format!(
                            "STOP. Edit blocked — `{target}` needs an inspected window. Use `inspect_lines` around the edit region, then retry."
                        ));
                        recoverable_policy_recipe = Some(RecoveryScenario::ExactLineWindowRequired);
                        recoverable_policy_checkpoint = Some((
                            OperatorCheckpointState::BlockedExactLineWindow,
                            format!("Edit blocked on `{target}`; exact line window required."),
                        ));
                    } else if res.blocked_by_policy
                        && (final_output.contains("Prefer `")
                            || final_output.contains("Prefer tool"))
                        && recoverable_policy_intervention.is_none()
                    {
                        recoverable_policy_intervention = Some(final_output.clone());
                        recoverable_policy_recipe = Some(RecoveryScenario::PolicyCorrection);
                        recoverable_policy_checkpoint = Some((
                            OperatorCheckpointState::BlockedPolicy,
                            "Action blocked by policy; self-correction triggered using tool recommendation."
                                .to_string(),
                        ));
                    } else if res.blocked_by_policy && blocked_policy_output.is_none() {
                        blocked_policy_output = Some(final_output.clone());
                    }

                    if *repeat_count >= 5 {
                        let _ = tx.send(InferenceEvent::Done).await;
                        return Ok(());
                    }

                    if implement_current_plan
                        && !implementation_started
                        && !is_error
                        && is_non_mutating_plan_step_tool(&tool_name)
                    {
                        non_mutating_plan_steps += 1;
                    }
                }

                if sovereign_bootstrap_complete
                    && intent.sovereign_mode
                    && is_scaffold_request(&effective_user_input)
                {
                    let response = if let Some(root) = sovereign_task_root.as_deref() {
                        format!(
                            "Project root created at `{root}`. Teleporting into the new project now so Hematite can continue there with a fresh local handoff."
                        )
                    } else {
                        "Project root created. Teleporting into the new project now so Hematite can continue there with a fresh local handoff."
                            .to_string()
                    };
                    self.emit_direct_response(&tx, user_input, &effective_user_input, &response)
                        .await;
                    return Ok(());
                }

                if let Some(intervention) = recoverable_policy_intervention {
                    if let Some((state, summary)) = recoverable_policy_checkpoint.take() {
                        self.emit_operator_checkpoint(&tx, state, summary).await;
                    }
                    if let Some(scenario) = recoverable_policy_recipe.take() {
                        let recipe = plan_recovery(scenario, &self.recovery_context);
                        self.emit_recovery_recipe_summary(
                            &tx,
                            recipe.recipe.scenario.label(),
                            compact_recovery_plan_summary(&recipe),
                        )
                        .await;
                    }
                    loop_intervention = Some(intervention);
                    let _ = tx
                        .send(InferenceEvent::Thought(
                            "Policy recovery: rerouting blocked MCP filesystem inspection to built-in workspace tools."
                                .into(),
                        ))
                        .await;
                    continue;
                }

                if architecture_overview_mode {
                    match overview_runtime_trace.as_deref() {
                        Some(runtime_trace) => {
                            let response = build_architecture_overview_answer(runtime_trace);
                            self.history.push(ChatMessage::assistant_text(&response));
                            self.transcript.log_agent(&response);

                            for chunk in chunk_text(&response, 8) {
                                if !chunk.is_empty() {
                                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                                }
                            }

                            let _ = tx.send(InferenceEvent::Done).await;
                            break;
                        }
                        None => {
                            loop_intervention = Some(
                                "Good. You now have the grounded repository structure. Next, call `trace_runtime_flow` for the runtime/control-flow half of the architecture overview. Prefer topic `user_turn` for the main execution path, or `runtime_subsystems` if that is more direct. Do not call `read_file`, `auto_pin_context`, or LSP tools here."
                                    .to_string(),
                            );
                            continue;
                        }
                    }
                }

                if implement_current_plan
                    && !implementation_started
                    && non_mutating_plan_steps >= non_mutating_plan_hard_cap
                {
                    let msg = "Current-plan execution stalled: too many non-mutating inspection steps without a concrete edit. Stay on the saved target files, narrow with `inspect_lines`, and then mutate, or ask one specific blocking question instead of continuing broad exploration.".to_string();
                    self.history.push(ChatMessage::assistant_text(&msg));
                    self.transcript.log_agent(&msg);

                    for chunk in chunk_text(&msg, 8) {
                        if !chunk.is_empty() {
                            let _ = tx.send(InferenceEvent::Token(chunk)).await;
                        }
                    }

                    let _ = tx.send(InferenceEvent::Done).await;
                    break;
                }

                if let Some(blocked_output) = blocked_policy_output {
                    self.emit_operator_checkpoint(
                        &tx,
                        OperatorCheckpointState::BlockedPolicy,
                        "A blocked tool path was surfaced directly to the operator.",
                    )
                    .await;
                    self.history
                        .push(ChatMessage::assistant_text(&blocked_output));
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

                if implement_current_plan && !implementation_started {
                    let base = "STOP analyzing. The current plan already defines the task. Use the built-in file evidence you now have and begin implementing the plan in the target files. Do not output preliminary findings or restate contracts.";
                    if non_mutating_plan_steps >= non_mutating_plan_soft_cap {
                        loop_intervention = Some(format!(
                            "{} You are close to the non-mutation cap. Use `inspect_lines` on one saved target file, then make the edit now.",
                            base
                        ));
                    } else {
                        loop_intervention = Some(base.to_string());
                    }
                } else if self.workflow_mode == WorkflowMode::Architect {
                    loop_intervention = Some(
                        format!(
                            "STOP exploring. You have enough evidence for a plan-first answer.\n{}\nUse the tool results already in history. Do not narrate your process. Do not call more tools unless a missing file path makes the handoff impossible.",
                            architect_handoff_contract()
                        ),
                    );
                }

                // 4. Auto-Verification Loop (The Perfect Bake)
                if mutation_occurred && !yolo && !intent.sovereign_mode {
                    let _ = tx
                        .send(InferenceEvent::Thought(
                            "Self-Verification: Running contract-aware workspace verification..."
                                .into(),
                        ))
                        .await;
                    let verify_outcome = self.auto_verify_workspace(&turn_mutated_paths).await;
                    let verify_res = verify_outcome.summary;
                    let verify_ok = verify_outcome.ok;
                    self.record_verify_build_result(verify_ok, &verify_res)
                        .await;
                    self.record_session_verification(
                        verify_ok,
                        if verify_ok {
                            "Automatic workspace verification passed."
                        } else {
                            "Automatic workspace verification failed."
                        },
                    );
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
                if finish_reason.as_deref() == Some("length") && near_context_ceiling {
                    if intent.direct_answer == Some(DirectAnswerKind::SessionResetSemantics) {
                        let cleaned = build_session_reset_semantics_answer();
                        self.history.push(ChatMessage::assistant_text(&cleaned));
                        self.transcript.log_agent(&cleaned);
                        for chunk in chunk_text(&cleaned, 8) {
                            if !chunk.is_empty() {
                                let _ = tx.send(InferenceEvent::Token(chunk.clone())).await;
                            }
                        }
                        let _ = tx.send(InferenceEvent::Done).await;
                        break;
                    }

                    let warning = format_runtime_failure(
                        RuntimeFailureClass::ContextWindow,
                        "Context ceiling reached before the model completed the answer. Hematite trimmed what it could, but this turn still ran out of room. Retry with a narrower inspection step like `grep_files` or `inspect_lines`, or ask for a smaller scoped answer.",
                    );
                    self.history.push(ChatMessage::assistant_text(&warning));
                    self.transcript.log_agent(&warning);
                    let _ = tx
                        .send(InferenceEvent::Thought(
                            "Length recovery: model hit the context ceiling before completing the answer."
                                .into(),
                        ))
                        .await;
                    for chunk in chunk_text(&warning, 8) {
                        if !chunk.is_empty() {
                            let _ = tx.send(InferenceEvent::Token(chunk.clone())).await;
                        }
                    }
                    let _ = tx.send(InferenceEvent::Done).await;
                    break;
                }

                if response_text.contains("<|tool_call")
                    || response_text.contains("[END_TOOL_REQUEST]")
                    || response_text.contains("<|tool_response")
                    || response_text.contains("<tool_response|>")
                {
                    loop_intervention = Some(
                        "Your previous response leaked raw native tool transcript markup instead of a valid tool invocation or final answer. Retry immediately. If you need a tool, emit a valid tool call only. If you do not need a tool, answer in plain text with no `<|tool_call>`, `<|tool_response>`, or `[END_TOOL_REQUEST]` markup.".to_string(),
                    );
                    continue;
                }

                // 1. Process and route the reasoning block to SPECULAR.
                if let Some(thought) = crate::agent::inference::extract_think_block(&response_text)
                {
                    let _ = tx.send(InferenceEvent::Thought(thought.clone())).await;
                    // Persist for history audit (stripped from next turn by Volatile Reasoning rule).
                    // This will be summarized in the next turn's system prompt.
                    self.reasoning_history = Some(thought);
                }

                let execution_ms = execution_start.elapsed().as_millis();
                let _ = tx
                    .send(InferenceEvent::TurnTiming {
                        context_prep_ms: context_prep_ms as u128,
                        inference_ms: inference_ms as u128,
                        execution_ms: execution_ms as u128,
                    })
                    .await;

                // 2. Process and stream the final answer to the chat interface.
                let cleaned = crate::agent::inference::strip_think_blocks(&response_text);

                if implement_current_plan && !implementation_started {
                    loop_intervention = Some(
                        "Do not stop at analysis. Implement the current saved plan now using built-in workspace tools and the target files already named in the plan. Only answer without edits if you have a concrete blocking question.".to_string(),
                    );
                    continue;
                }

                // [Hardened Interface] Strictly respect the stripper.
                // If it's empty after stripping think blocks, the model thought through its
                // answer but forgot to emit it (common with Qwen3 models in architect/ask mode).
                // Nudge it rather than silently dropping the turn — but cap at 2 retries so a
                // model that keeps returning whitespace/empty doesn't spin all 25 iterations.
                if cleaned.is_empty() {
                    empty_cleaned_nudges += 1;
                    if empty_cleaned_nudges == 1 {
                        loop_intervention = Some(
                            "Your visible response was empty. The tool already returned data. \
                             Write your answer now in plain text — no <think> tags, no tool calls. \
                             State the key facts in 2-5 sentences and stop."
                                .to_string(),
                        );
                        continue;
                    } else if empty_cleaned_nudges == 2 {
                        loop_intervention = Some(
                            "EMPTY RESPONSE. Do NOT use <think>. Do NOT call tools. \
                             Write the answer in plain text right now. \
                             Example format: \"Your CPU is X. Your GPU is Y. You have Z GB of RAM.\""
                                .to_string(),
                        );
                        continue;
                    }
                    // Nudge budget exhausted — surface as a recoverable empty-response failure
                    // so the TUI unblocks instead of hanging for the full max_iters budget.
                    let class = RuntimeFailureClass::EmptyModelResponse;
                    self.emit_runtime_failure(
                        &tx,
                        class,
                        "Model returned empty content after 2 nudge attempts.",
                    )
                    .await;
                    break;
                }

                let architect_handoff = self.persist_architect_handoff(&cleaned);
                self.history.push(ChatMessage::assistant_text(&cleaned));
                self.transcript.log_agent(&cleaned);
                visible_closeout_emitted = true;

                // Send in smooth chunks for that professional UI feel.
                for chunk in chunk_text(&cleaned, 8) {
                    if !chunk.is_empty() {
                        let _ = tx.send(InferenceEvent::Token(chunk.clone())).await;
                    }
                }

                if let Some(plan) = architect_handoff.as_ref() {
                    let note = architect_handoff_operator_note(plan);
                    self.history.push(ChatMessage::system(&note));
                    self.transcript.log_system(&note);
                    let _ = tx
                        .send(InferenceEvent::MutedToken(format!("\n{}", note)))
                        .await;
                }

                self.emit_done_events(&tx).await;
                break;
            } else {
                let detail = "Model returned an empty response.";
                let class = classify_runtime_failure(detail);
                if should_retry_runtime_failure(class) {
                    if let Some(scenario) = recovery_scenario_for_runtime_failure(class) {
                        if let RecoveryDecision::Attempt(plan) =
                            attempt_recovery(scenario, &mut self.recovery_context)
                        {
                            self.transcript.log_system(
                                "Automatic provider recovery triggered: model returned an empty response.",
                            );
                            self.emit_recovery_recipe_summary(
                                &tx,
                                plan.recipe.scenario.label(),
                                compact_recovery_plan_summary(&plan),
                            )
                            .await;
                            let _ = tx
                                .send(InferenceEvent::ProviderStatus {
                                    state: ProviderRuntimeState::Recovering,
                                    summary: compact_runtime_recovery_summary(class).into(),
                                })
                                .await;
                            self.emit_operator_checkpoint(
                                &tx,
                                OperatorCheckpointState::RecoveringProvider,
                                compact_runtime_recovery_summary(class),
                            )
                            .await;
                            continue;
                        }
                    }
                }

                if explicit_search_request
                    && matches!(
                        class,
                        RuntimeFailureClass::ProviderDegraded
                            | RuntimeFailureClass::EmptyModelResponse
                    )
                {
                    if let Some(results) = grounded_research_results.as_deref() {
                        let response = build_research_provider_fallback(results);
                        self.history.push(ChatMessage::assistant_text(&response));
                        self.transcript.log_agent(&response);
                        for chunk in chunk_text(&response, 8) {
                            let _ = tx.send(InferenceEvent::Token(chunk)).await;
                        }
                        let _ = tx.send(InferenceEvent::Done).await;
                        return Ok(());
                    }
                }

                self.emit_runtime_failure(&tx, class, detail).await;
                break;
            }
        }

        let task_progress_after = if implement_current_plan {
            read_task_checklist_progress()
        } else {
            None
        };

        if implement_current_plan
            && !visible_closeout_emitted
            && should_continue_plan_execution(
                current_plan_pass,
                task_progress_before,
                task_progress_after,
                &turn_mutated_paths,
            )
        {
            if let Some(progress) = task_progress_after {
                let _ = tx
                    .send(InferenceEvent::Thought(format!(
                        "Checklist still has {} unchecked item(s). Continuing autonomous implementation pass {}.",
                        progress.remaining,
                        current_plan_pass + 1
                    )))
                    .await;
                let synthetic_turn = UserTurn {
                    text: build_continue_plan_execution_prompt(progress),
                    attached_document: None,
                    attached_image: None,
                };
                return Box::pin(self.run_turn(&synthetic_turn, tx.clone(), false)).await;
            }
        }

        if implement_current_plan && !visible_closeout_emitted {
            // FORCE a summary turn if we had no natural closeout (e.g. hit a mandate or finished all tool budget).
            let _ = tx.send(InferenceEvent::Thought("Implementation passthrough complete. Requesting final engineering summary (NLG-only mode)...".to_string())).await;

            let outstanding_note = task_progress_after
                .filter(|progress| progress.has_open_items())
                .map(|progress| {
                    format!(
                        " `.hematite/TASK.md` still has {} unchecked item(s); explain the concrete blocker or remaining non-optional work.",
                        progress.remaining
                    )
                })
                .unwrap_or_default();
            let synthetic_turn = UserTurn {
                text: format!(
                    "Implementation passes complete. YOU ARE NOW IN SUMMARY MODE. STOP calling tools — all tools are hidden. Provide a concise human engineering summary of what you built, what was verified, and whether `.hematite/TASK.md` is fully checked off.{}",
                    outstanding_note
                ),
                attached_document: None,
                attached_image: None,
            };
            // Note: We use recursion to force one last NLG pass.
            // We set yolo=true to suppress tools.
            return Box::pin(self.run_turn(&synthetic_turn, tx.clone(), true)).await;
        }

        if plan_drafted_this_turn
            && matches!(
                self.workflow_mode,
                WorkflowMode::Auto | WorkflowMode::Architect
            )
        {
            let (appr_tx, appr_rx) = tokio::sync::oneshot::channel::<bool>();
            let _ = tx
                .send(InferenceEvent::ApprovalRequired {
                    id: "plan_approval".to_string(),
                    name: "plan_authorization".to_string(),
                    display: "A comprehensive scaffolding blueprint has been drafted to .hematite/PLAN.md. Autonomously execute implementation now?".to_string(),
                    diff: None,
                    mutation_label: Some("SYSTEM PLAN AUTHORIZATION".to_string()),
                    responder: appr_tx,
                })
                .await;

            if let Ok(true) = appr_rx.await {
                // Wipe conversation history to prevent hallucination cycles on 9B models.
                // The recursive run_turn call will rebuild the system prompt from scratch
                // and inject the PLAN.md blueprint via the implement-plan pathway.
                self.history.clear();
                self.running_summary = None;
                self.set_workflow_mode(WorkflowMode::Code);

                let _ = tx.send(InferenceEvent::ChainImplementPlan).await;

                let next_input = implement_current_plan_prompt().to_string();
                let synthetic_turn = UserTurn {
                    text: next_input,
                    attached_document: None,
                    attached_image: None,
                };
                return Box::pin(self.run_turn(&synthetic_turn, tx.clone(), false)).await;
            }
        }

        self.trim_history(80);
        self.refresh_session_memory();
        // Record the goal and increment the turn counter before persisting.
        self.last_goal = Some(user_input.chars().take(300).collect());
        self.turn_count = self.turn_count.saturating_add(1);
        self.emit_compaction_pressure(&tx).await;

        // AUTHORITATIVE TURN SUMMARY: Generate and display unified diffs.
        if !implement_current_plan {
            let tracker = self.diff_tracker.lock().await;
            if let Ok(diff) = tracker.generate_diff() {
                if !diff.is_empty() {
                    let _ = tx
                        .send(InferenceEvent::Thought(format!(
                            "AUTHORITATIVE TURN SUMMARY:\n\n```diff\n{}\n```",
                            diff
                        )))
                        .await;

                    // Also log to transcript for persistence.
                    self.transcript
                        .log_system(&format!("Turn Diff Summary:\n{}", diff));
                }
            }
        }

        Ok(())
    }

    async fn emit_runtime_failure(
        &mut self,
        tx: &mpsc::Sender<InferenceEvent>,
        class: RuntimeFailureClass,
        detail: &str,
    ) {
        if let Some(scenario) = recovery_scenario_for_runtime_failure(class) {
            let decision = preview_recovery_decision(scenario, &self.recovery_context);
            self.emit_recovery_recipe_summary(
                tx,
                scenario.label(),
                compact_recovery_decision_summary(&decision),
            )
            .await;
            let needs_refresh = match &decision {
                RecoveryDecision::Attempt(plan) => plan
                    .recipe
                    .steps
                    .contains(&RecoveryStep::RefreshRuntimeProfile),
                RecoveryDecision::Escalate { recipe, .. } => {
                    recipe.steps.contains(&RecoveryStep::RefreshRuntimeProfile)
                }
            };
            if needs_refresh {
                if let Some((model_id, context_length, changed)) = self
                    .refresh_runtime_profile_and_report(tx, "context_window_failure")
                    .await
                {
                    let note = if changed {
                        format!(
                            "Runtime refresh after context-window failure: model {} | CTX {}",
                            model_id, context_length
                        )
                    } else {
                        format!(
                            "Runtime refresh after context-window failure confirms model {} | CTX {}",
                            model_id, context_length
                        )
                    };
                    let _ = tx.send(InferenceEvent::Thought(note)).await;
                }
            }
        }
        if let Some(state) = provider_state_for_runtime_failure(class) {
            let _ = tx
                .send(InferenceEvent::ProviderStatus {
                    state,
                    summary: compact_runtime_failure_summary(class).into(),
                })
                .await;
        }
        if let Some(state) = checkpoint_state_for_runtime_failure(class) {
            self.emit_operator_checkpoint(tx, state, checkpoint_summary_for_runtime_failure(class))
                .await;
        }
        let formatted = format_runtime_failure(class, detail);
        self.history.push(ChatMessage::system(&format!(
            "# RUNTIME FAILURE\n{}",
            formatted
        )));
        self.transcript.log_system(&formatted);
        let _ = tx.send(InferenceEvent::Error(formatted)).await;
        let _ = tx.send(InferenceEvent::Done).await;
    }

    /// Contract-aware self verification. Build is still the base proof, but stack-specific
    /// runtime contracts can add stronger checks such as website route and asset validation.
    async fn auto_verify_workspace(
        &self,
        mutated_paths: &std::collections::BTreeSet<String>,
    ) -> AutoVerificationOutcome {
        let root = crate::tools::file_ops::workspace_root();
        let profile = crate::agent::workspace_profile::load_workspace_profile(&root)
            .unwrap_or_else(|| crate::agent::workspace_profile::detect_workspace_profile(&root));

        let mut sections = Vec::new();
        let mut overall_ok = true;
        let contract = profile.runtime_contract.as_ref();
        let verification_workflows: Vec<String> = match contract {
            Some(contract) if !contract.verification_workflows.is_empty() => {
                contract.verification_workflows.clone()
            }
            _ if profile.build_hint.is_some() || profile.verify_profile.is_some() => {
                vec!["build".to_string()]
            }
            _ => Vec::new(),
        };

        for workflow in verification_workflows {
            if !should_run_contract_verification_workflow(contract, &workflow, mutated_paths) {
                continue;
            }
            let outcome = self.auto_run_verification_workflow(&workflow).await;
            overall_ok &= outcome.ok;
            sections.push(outcome.summary);
        }

        if sections.is_empty() {
            sections.push(
                "[verify]\nVERIFICATION SKIPPED: Workspace profile does not define an automatic verification workflow for this stack."
                    .to_string(),
            );
        }

        let header = if overall_ok {
            "WORKSPACE VERIFICATION SUCCESS: Automatic validation passed."
        } else {
            "WORKSPACE VERIFICATION FAILURE: Automatic validation found problems."
        };

        AutoVerificationOutcome {
            ok: overall_ok,
            summary: format!("{}\n\n{}", header, sections.join("\n\n")),
        }
    }

    async fn auto_run_verification_workflow(&self, workflow: &str) -> AutoVerificationOutcome {
        match workflow {
            "build" | "test" | "lint" | "fix" => {
                match crate::tools::verify_build::execute(
                    &serde_json::json!({ "action": workflow }),
                )
                .await
                {
                    Ok(out) => AutoVerificationOutcome {
                        ok: true,
                        summary: format!(
                            "[{}]\n{} SUCCESS: Automatic {} verification passed.\n\n{}",
                            workflow,
                            workflow.to_ascii_uppercase(),
                            workflow,
                            cap_output(&out, 2000)
                        ),
                    },
                    Err(e) => AutoVerificationOutcome {
                        ok: false,
                        summary: format!(
                            "[{}]\n{} FAILURE: Automatic {} verification failed.\n\n{}",
                            workflow,
                            workflow.to_ascii_uppercase(),
                            workflow,
                            cap_output(&e, 2000)
                        ),
                    },
                }
            }
            other => {
                // DISPATCH Generic workflows (e.g. website_validate, server_probe, etc.)
                let args = serde_json::json!({ "workflow": other });
                match crate::tools::workspace_workflow::run_workspace_workflow(&args).await {
                    Ok(out) => {
                        // Specialized workflows rely on "Result: PASS" or "Result: FAIL" markers.
                        // Standard shell fallbacks return OK if exit code was 0.
                        let ok = !out.contains("Result: FAIL") && !out.contains("Error:");
                        AutoVerificationOutcome {
                            ok,
                            summary: format!("[{}]\n{}", other, out.trim()),
                        }
                    }
                    Err(e) => {
                        // If a specialized workflow needs "Auto-Booting" (e.g. website),
                        // we can handle a retry here or delegate the intelligence to the tool itself.
                        // For website_validate, we attempt a boot if it looks like a connection failure.
                        let needs_boot = e.contains("No tracked website server labeled")
                            || e.contains("HTTP probe failed")
                            || e.contains("Connection refused")
                            || e.contains("error trying to connect");

                        if other == "website_validate" && needs_boot {
                            let start_args = serde_json::json!({ "workflow": "website_start" });
                            if let Ok(_) = crate::tools::workspace_workflow::run_workspace_workflow(
                                &start_args,
                            )
                            .await
                            {
                                if let Ok(retry_out) =
                                    crate::tools::workspace_workflow::run_workspace_workflow(&args)
                                        .await
                                {
                                    let ok = !retry_out.contains("Result: FAIL")
                                        && !retry_out.contains("Error:");
                                    return AutoVerificationOutcome {
                                        ok,
                                        summary: format!(
                                            "[{}]\n(Auto-booted) {}",
                                            other,
                                            retry_out.trim()
                                        ),
                                    };
                                }
                            }
                        }

                        AutoVerificationOutcome {
                            ok: false,
                            summary: format!("[{}]\nVERIFICATION FAILURE: {}", other, e),
                        }
                    }
                }
            }
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
        let context_length = self.engine.current_context_length();
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

        let removed_message_count = self.history.len().saturating_sub(result.messages.len());
        self.history = result.messages;
        self.running_summary = result.summary;

        // Layer 6: Memory Synthesis (Task Context Persistence)
        let previous_memory = self.session_memory.clone();
        self.session_memory = compaction::extract_memory(&self.history);
        self.session_memory
            .inherit_runtime_ledger_from(&previous_memory);
        self.session_memory.record_compaction(
            removed_message_count,
            format!(
                "Compacted history around active task '{}' and preserved {} working-set file(s).",
                self.session_memory.current_task,
                self.session_memory.working_set.len()
            ),
        );
        self.emit_compaction_pressure(tx).await;

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
        let recipe = plan_recovery(RecoveryScenario::HistoryPressure, &self.recovery_context);
        self.emit_recovery_recipe_summary(
            tx,
            recipe.recipe.scenario.label(),
            compact_recovery_plan_summary(&recipe),
        )
        .await;
        self.emit_operator_checkpoint(
            tx,
            OperatorCheckpointState::HistoryCompacted,
            format!(
                "History compacted into a recursive summary; active task '{}' with {} working-set file(s) carried forward.",
                self.session_memory.current_task,
                self.session_memory.working_set.len()
            ),
        )
        .await;

        Ok(true)
    }

    /// Query The Vein for context relevant to the user's message.
    /// Runs hybrid BM25 + semantic search (semantic requires embedding model in LM Studio).
    /// Returns a formatted system message string, or None if nothing useful found.
    fn build_vein_context(&self, query: &str) -> Option<(String, Vec<String>)> {
        // Skip trivial / very short inputs.
        if query.trim().split_whitespace().count() < 3 {
            return None;
        }

        let results = tokio::task::block_in_place(|| self.vein.search_context(query, 4)).ok()?;
        if results.is_empty() {
            return None;
        }

        let semantic_active = self.vein.has_any_embeddings();
        let header = if semantic_active {
            "# Relevant context from The Vein (hybrid BM25 + semantic retrieval)\n\
             Use this to answer without needing extra read_file calls where possible.\n\n"
        } else {
            "# Relevant context from The Vein (BM25 keyword retrieval)\n\
             Use this to answer without needing extra read_file calls where possible.\n\n"
        };

        let mut ctx = String::from(header);
        let mut paths: Vec<String> = Vec::new();

        let mut total = 0usize;
        const MAX_CTX_CHARS: usize = 1_500;

        for r in results {
            if total >= MAX_CTX_CHARS {
                break;
            }
            let snippet = if r.content.len() > 500 {
                format!("{}...", &r.content[..500])
            } else {
                r.content.clone()
            };
            ctx.push_str(&format!("--- {} ---\n{}\n\n", r.path, snippet));
            total += snippet.len() + r.path.len() + 10;
            if !paths.contains(&r.path) {
                paths.push(r.path);
            }
        }

        Some((ctx, paths))
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

    fn context_window_slice_from(&self, start_idx: usize) -> Vec<ChatMessage> {
        let mut result = Vec::new();

        if self.history.len() > 1 {
            let start = start_idx.max(1).min(self.history.len());
            for m in &self.history[start..] {
                if m.role == "system" {
                    continue;
                }

                let mut sanitized = m.clone();
                if (m.role == "assistant" || m.role == "tool") && m.content.as_str().is_empty() {
                    sanitized.content = MessageContent::Text(" ".into());
                }
                result.push(sanitized);
            }
        }

        if !result.is_empty() && result[0].role != "user" {
            result.insert(0, ChatMessage::user("Continuing current plan execution..."));
        }

        result
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

    /// P1: Attempt to fix malformed JSON tool arguments by asking the model to re-output them.
    #[allow(dead_code)]
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
        let (text, _, _, _) = self
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

        const WEB_EXTS_CRITIC: &[&str] = &[
            "html", "htm", "css", "js", "ts", "jsx", "tsx", "vue", "svelte",
        ];
        let is_web_file = WEB_EXTS_CRITIC.contains(&ext);

        let prompt = if is_web_file {
            format!(
                "You are a senior web developer doing a quality review of '{}'. \
                Identify ONLY real problems — missing, broken, or incomplete things that would \
                make this file not work or look bad in production. Check:\n\
                - HTML: missing DOCTYPE/charset/title/viewport meta, broken links, missing aria, unsemantic structure\n\
                - CSS: hardcoded px instead of responsive units, missing mobile media queries, class names used in HTML but not defined here\n\
                - JS/TS: missing error handling, undefined variables, console.log left in, DOM elements referenced that may not exist\n\
                - All: placeholder text/colors/lorem-ipsum left in, TODO comments, empty sections\n\
                Be extremely concise. List issues as short bullets. If everything is production-ready, output 'PASS'.\n\n\
                ```{}\n{}\n```",
                path, ext, truncated
            )
        } else {
            format!(
                "You are a Senior Security and Code Quality auditor. Review this file content for '{}' \
                and identify any critical logic errors, security vulnerabilities, or missing error handling. \
                Be extremely concise. If the code looks good, output 'PASS'.\n\n```{}\n{}\n```",
                path, ext, truncated
            )
        };

        let messages = vec![
            ChatMessage::system("You are a technical critic. Identify ONLY real issues that need fixing. Output 'PASS' if none found."),
            ChatMessage::user(&prompt)
        ];

        let (text, _, _, _) = self
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

pub async fn dispatch_tool(
    name: &str,
    args: &Value,
    config: &crate::agent::config::HematiteConfig,
    budget_tokens: usize,
) -> Result<String, String> {
    dispatch_builtin_tool(name, args, config, budget_tokens).await
}

fn normalize_fix_plan_issue_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let stripped = trimmed
        .strip_prefix("/think")
        .or_else(|| trimmed.strip_prefix("/no_think"))
        .map(str::trim)
        .unwrap_or(trimmed)
        .trim_start_matches('\n')
        .trim();
    (!stripped.is_empty()).then(|| stripped.to_string())
}

fn fill_missing_fix_plan_issue(tool_name: &str, args: &mut Value, fallback_issue: Option<&str>) {
    if tool_name != "inspect_host" {
        return;
    }

    let Some(topic) = args.get("topic").and_then(|v| v.as_str()) else {
        return;
    };
    if topic != "fix_plan" {
        return;
    }

    let issue_missing = args
        .get("issue")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .is_none_or(|value| value.is_empty());
    if !issue_missing {
        return;
    }

    let Some(fallback_issue) = fallback_issue.and_then(normalize_fix_plan_issue_text) else {
        return;
    };

    let Value::Object(map) = args else {
        return;
    };
    map.insert(
        "issue".to_string(),
        Value::String(fallback_issue.to_string()),
    );
}

fn fill_missing_dns_lookup_name(
    tool_name: &str,
    args: &mut Value,
    latest_user_prompt: Option<&str>,
) {
    if tool_name != "inspect_host" {
        return;
    }

    let Some(topic) = args.get("topic").and_then(|v| v.as_str()) else {
        return;
    };
    if topic != "dns_lookup" {
        return;
    }

    let name_missing = args
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .is_none_or(|value| value.is_empty());
    if !name_missing {
        return;
    }

    let Some(prompt) = latest_user_prompt else {
        return;
    };
    let Some(name) = extract_dns_lookup_target_from_text(prompt) else {
        return;
    };

    let Value::Object(map) = args else {
        return;
    };
    map.insert("name".to_string(), Value::String(name));
}

fn fill_missing_dns_lookup_type(
    tool_name: &str,
    args: &mut Value,
    latest_user_prompt: Option<&str>,
) {
    if tool_name != "inspect_host" {
        return;
    }

    let Some(topic) = args.get("topic").and_then(|v| v.as_str()) else {
        return;
    };
    if topic != "dns_lookup" {
        return;
    }

    let type_missing = args
        .get("type")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .is_none_or(|value| value.is_empty());
    if !type_missing {
        return;
    }

    let record_type = latest_user_prompt
        .and_then(extract_dns_record_type_from_text)
        .unwrap_or("A");

    let Value::Object(map) = args else {
        return;
    };
    map.insert("type".to_string(), Value::String(record_type.to_string()));
}

fn fill_missing_event_query_args(
    tool_name: &str,
    args: &mut Value,
    latest_user_prompt: Option<&str>,
) {
    if tool_name != "inspect_host" {
        return;
    }

    let Some(topic) = args.get("topic").and_then(|v| v.as_str()) else {
        return;
    };
    if topic != "event_query" {
        return;
    }

    let Some(prompt) = latest_user_prompt else {
        return;
    };

    let Value::Object(map) = args else {
        return;
    };

    let event_id_missing = map.get("event_id").and_then(|v| v.as_u64()).is_none();
    if event_id_missing {
        if let Some(event_id) = extract_event_query_event_id_from_text(prompt) {
            map.insert(
                "event_id".to_string(),
                Value::Number(serde_json::Number::from(event_id)),
            );
        }
    }

    let log_missing = map
        .get("log")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .is_none_or(|value| value.is_empty());
    if log_missing {
        if let Some(log_name) = extract_event_query_log_from_text(prompt) {
            map.insert("log".to_string(), Value::String(log_name.to_string()));
        }
    }

    let level_missing = map
        .get("level")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .is_none_or(|value| value.is_empty());
    if level_missing {
        if let Some(level) = extract_event_query_level_from_text(prompt) {
            map.insert("level".to_string(), Value::String(level.to_string()));
        }
    }

    let hours_missing = map.get("hours").and_then(|v| v.as_u64()).is_none();
    if hours_missing {
        if let Some(hours) = extract_event_query_hours_from_text(prompt) {
            map.insert(
                "hours".to_string(),
                Value::Number(serde_json::Number::from(hours)),
            );
        }
    }
}

fn should_rewrite_shell_to_fix_plan(
    tool_name: &str,
    args: &Value,
    latest_user_prompt: Option<&str>,
) -> bool {
    if tool_name != "shell" {
        return false;
    }
    let Some(prompt) = latest_user_prompt else {
        return false;
    };
    if preferred_host_inspection_topic(prompt) != Some("fix_plan") {
        return false;
    }
    let command = args
        .get("command")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    shell_looks_like_structured_host_inspection(command)
}

fn extract_release_arg(command: &str, flag: &str) -> Option<String> {
    let pattern = format!(r#"(?i){}\s+['"]?([^'" \r\n]+)['"]?"#, regex::escape(flag));
    let regex = regex::Regex::new(&pattern).ok()?;
    let captures = regex.captures(command)?;
    captures.get(1).map(|m| m.as_str().to_string())
}

fn clean_shell_dns_token(token: &str) -> String {
    token
        .trim_matches(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | ';' | ',' | '`'
                )
        })
        .trim_end_matches(|c: char| matches!(c, ':' | '.'))
        .to_string()
}

fn looks_like_dns_target(token: &str) -> bool {
    let cleaned = clean_shell_dns_token(token);
    if cleaned.is_empty() {
        return false;
    }

    let lower = cleaned.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "a" | "aaaa"
            | "mx"
            | "srv"
            | "txt"
            | "cname"
            | "ptr"
            | "soa"
            | "any"
            | "resolve-dnsname"
            | "nslookup"
            | "host"
            | "dig"
            | "powershell"
            | "-command"
            | "foreach-object"
            | "select-object"
            | "address"
            | "ipaddress"
            | "name"
            | "type"
    ) {
        return false;
    }

    if lower == "localhost" || cleaned.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }

    cleaned.contains('.')
        && cleaned
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '%' | '*'))
}

fn extract_dns_lookup_target_from_shell(command: &str) -> Option<String> {
    for pattern in [
        r#"(?i)-name\s+['"]?([^'"\s;()]+)['"]?"#,
        r#"(?i)(?:gethostaddresses|gethostentry)\s*\(\s*['"]([^'"]+)['"]\s*\)"#,
        r#"(?i)\b(?:resolve-dnsname|nslookup|host|dig)\s+['"]?([^'"\s;()]+)['"]?"#,
    ] {
        let regex = regex::Regex::new(pattern).ok()?;
        if let Some(value) = regex
            .captures(command)
            .and_then(|captures| captures.get(1).map(|m| clean_shell_dns_token(m.as_str())))
            .filter(|value| looks_like_dns_target(value))
        {
            return Some(value);
        }
    }

    let quoted = regex::Regex::new(r#"['"]([^'"]+)['"]"#).ok()?;
    for captures in quoted.captures_iter(command) {
        let candidate = clean_shell_dns_token(captures.get(1)?.as_str());
        if looks_like_dns_target(&candidate) {
            return Some(candidate);
        }
    }

    command
        .split_whitespace()
        .map(clean_shell_dns_token)
        .find(|token| looks_like_dns_target(token))
}

fn extract_dns_lookup_target_from_text(text: &str) -> Option<String> {
    let quoted = regex::Regex::new(r#"['"]([^'"]+)['"]"#).ok()?;
    for captures in quoted.captures_iter(text) {
        let candidate = clean_shell_dns_token(captures.get(1)?.as_str());
        if looks_like_dns_target(&candidate) {
            return Some(candidate);
        }
    }

    text.split_whitespace()
        .map(clean_shell_dns_token)
        .find(|token| looks_like_dns_target(token))
}

fn extract_dns_record_type_from_text(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("aaaa record") || lower.contains("ipv6 address") {
        Some("AAAA")
    } else if lower.contains("mx record") {
        Some("MX")
    } else if lower.contains("srv record") {
        Some("SRV")
    } else if lower.contains("txt record") {
        Some("TXT")
    } else if lower.contains("cname record") {
        Some("CNAME")
    } else if lower.contains("soa record") {
        Some("SOA")
    } else if lower.contains("ptr record") {
        Some("PTR")
    } else if lower.contains("a record")
        || (lower.contains("ip address") && lower.contains(" of "))
        || (lower.contains("what") && lower.contains("ip") && lower.contains("for"))
    {
        Some("A")
    } else {
        None
    }
}

fn extract_event_query_event_id_from_text(text: &str) -> Option<u32> {
    let re = regex::Regex::new(r"(?i)\bevent(?:\s*_?\s*id)?\s*[:#]?\s*(\d{2,5})\b").ok()?;
    re.captures(text)
        .and_then(|captures| captures.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

fn extract_event_query_log_from_text(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("security log") {
        Some("Security")
    } else if lower.contains("application log") {
        Some("Application")
    } else if lower.contains("system log") || lower.contains("system errors") {
        Some("System")
    } else if lower.contains("setup log") {
        Some("Setup")
    } else {
        None
    }
}

fn extract_event_query_level_from_text(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("critical") {
        Some("Critical")
    } else if lower.contains("error") || lower.contains("errors") {
        Some("Error")
    } else if lower.contains("warning") || lower.contains("warnings") || lower.contains("warn") {
        Some("Warning")
    } else if lower.contains("information")
        || lower.contains("informational")
        || lower.contains("info")
    {
        Some("Information")
    } else {
        None
    }
}

fn extract_event_query_hours_from_text(text: &str) -> Option<u32> {
    let lower = text.to_ascii_lowercase();
    let re = regex::Regex::new(r"(?i)\b(?:last|past)\s+(\d{1,3})\s*(hour|hours|hr|hrs)\b").ok()?;
    if let Some(hours) = re
        .captures(&lower)
        .and_then(|captures| captures.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
    {
        return Some(hours);
    }
    if lower.contains("last hour") || lower.contains("past hour") {
        Some(1)
    } else if lower.contains("today") {
        Some(24)
    } else {
        None
    }
}

fn extract_dns_record_type_from_shell(command: &str) -> Option<&'static str> {
    let lower = command.to_ascii_lowercase();
    if lower.contains("-type aaaa") || lower.contains("-type=aaaa") {
        Some("AAAA")
    } else if lower.contains("-type mx") || lower.contains("-type=mx") {
        Some("MX")
    } else if lower.contains("-type srv") || lower.contains("-type=srv") {
        Some("SRV")
    } else if lower.contains("-type txt") || lower.contains("-type=txt") {
        Some("TXT")
    } else if lower.contains("-type cname") || lower.contains("-type=cname") {
        Some("CNAME")
    } else if lower.contains("-type soa") || lower.contains("-type=soa") {
        Some("SOA")
    } else if lower.contains("-type ptr") || lower.contains("-type=ptr") {
        Some("PTR")
    } else if lower.contains("-type a") || lower.contains("-type=a") {
        Some("A")
    } else {
        extract_dns_record_type_from_text(command)
    }
}

fn host_inspection_args_from_prompt(topic: &str, prompt: &str) -> Value {
    let mut args = serde_json::json!({ "topic": topic });
    if topic == "dns_lookup" {
        if let Some(name) = extract_dns_lookup_target_from_text(prompt) {
            args.as_object_mut()
                .unwrap()
                .insert("name".to_string(), Value::String(name));
        }
        let record_type = extract_dns_record_type_from_text(prompt).unwrap_or("A");
        args.as_object_mut()
            .unwrap()
            .insert("type".to_string(), Value::String(record_type.to_string()));
    } else if topic == "event_query" {
        if let Some(event_id) = extract_event_query_event_id_from_text(prompt) {
            args.as_object_mut().unwrap().insert(
                "event_id".to_string(),
                Value::Number(serde_json::Number::from(event_id)),
            );
        }
        if let Some(log_name) = extract_event_query_log_from_text(prompt) {
            args.as_object_mut()
                .unwrap()
                .insert("log".to_string(), Value::String(log_name.to_string()));
        }
        if let Some(level) = extract_event_query_level_from_text(prompt) {
            args.as_object_mut()
                .unwrap()
                .insert("level".to_string(), Value::String(level.to_string()));
        }
        if let Some(hours) = extract_event_query_hours_from_text(prompt) {
            args.as_object_mut().unwrap().insert(
                "hours".to_string(),
                Value::Number(serde_json::Number::from(hours)),
            );
        }
    }
    args
}

fn infer_maintainer_workflow_args_from_prompt(prompt: &str) -> Option<Value> {
    let workflow = preferred_maintainer_workflow(prompt)?;
    let lower = prompt.to_ascii_lowercase();
    match workflow {
        "clean" => Some(serde_json::json!({
            "workflow": "clean",
            "deep": lower.contains("deep clean")
                || lower.contains("deep cleanup")
                || lower.contains("deep"),
            "reset": lower.contains("reset"),
            "prune_dist": lower.contains("prune dist")
                || lower.contains("prune old dist")
                || lower.contains("prune old artifacts")
                || lower.contains("old dist artifacts")
                || lower.contains("old artifacts"),
        })),
        "package_windows" => Some(serde_json::json!({
            "workflow": "package_windows",
            "installer": lower.contains("installer") || lower.contains("setup.exe"),
            "add_to_path": lower.contains("addtopath")
                || lower.contains("add to path")
                || lower.contains("update path")
                || lower.contains("refresh path"),
        })),
        "release" => {
            let version = regex::Regex::new(r#"(?i)\b(\d+\.\d+\.\d+)\b"#)
                .ok()
                .and_then(|re| re.captures(prompt))
                .and_then(|captures| captures.get(1).map(|m| m.as_str().to_string()));
            let bump = if lower.contains("patch") {
                Some("patch")
            } else if lower.contains("minor") {
                Some("minor")
            } else if lower.contains("major") {
                Some("major")
            } else {
                None
            };
            let mut args = serde_json::json!({
                "workflow": "release",
                "push": lower.contains(" push") || lower.starts_with("push ") || lower.contains(" and push"),
                "add_to_path": lower.contains("addtopath")
                    || lower.contains("add to path")
                    || lower.contains("update path"),
                "skip_installer": lower.contains("skip installer"),
                "publish_crates": lower.contains("publish crates") || lower.contains("crates.io"),
                "publish_voice_crate": lower.contains("publish voice crate")
                    || lower.contains("publish hematite-kokoros"),
            });
            if let Some(version) = version {
                args["version"] = Value::String(version);
            }
            if let Some(bump) = bump {
                args["bump"] = Value::String(bump.to_string());
            }
            Some(args)
        }
        _ => None,
    }
}

fn infer_workspace_workflow_args_from_prompt(prompt: &str) -> Option<Value> {
    if is_scaffold_request(prompt) {
        return None;
    }
    let workflow = preferred_workspace_workflow(prompt)?;
    let lower = prompt.to_ascii_lowercase();
    let trimmed = prompt.trim();

    if let Some(command) = extract_workspace_command_from_prompt(trimmed) {
        return Some(serde_json::json!({
            "workflow": "command",
            "command": command,
        }));
    }

    if let Some(path) = extract_workspace_script_path_from_prompt(trimmed) {
        return Some(serde_json::json!({
            "workflow": "script_path",
            "path": path,
        }));
    }

    match workflow {
        "build" | "test" | "lint" | "fix" => Some(serde_json::json!({
            "workflow": workflow,
        })),
        "script" => {
            let package_script = if lower.contains("npm run ") {
                extract_word_after(&lower, "npm run ")
            } else if lower.contains("pnpm run ") {
                extract_word_after(&lower, "pnpm run ")
            } else if lower.contains("bun run ") {
                extract_word_after(&lower, "bun run ")
            } else if lower.contains("yarn ") {
                extract_word_after(&lower, "yarn ")
            } else {
                None
            };

            if let Some(name) = package_script {
                return Some(serde_json::json!({
                    "workflow": "package_script",
                    "name": name,
                }));
            }

            if let Some(name) = extract_word_after(&lower, "just ") {
                return Some(serde_json::json!({
                    "workflow": "just",
                    "name": name,
                }));
            }
            if let Some(name) = extract_word_after(&lower, "make ") {
                return Some(serde_json::json!({
                    "workflow": "make",
                    "name": name,
                }));
            }
            if let Some(name) = extract_word_after(&lower, "task ") {
                return Some(serde_json::json!({
                    "workflow": "task",
                    "name": name,
                }));
            }

            None
        }
        _ => None,
    }
}

fn extract_workspace_command_from_prompt(prompt: &str) -> Option<String> {
    let lower = prompt.to_ascii_lowercase();
    for prefix in [
        "cargo ",
        "npm ",
        "pnpm ",
        "yarn ",
        "bun ",
        "pytest",
        "go build",
        "go test",
        "make ",
        "just ",
        "task ",
        "./gradlew",
        ".\\gradlew",
    ] {
        if let Some(index) = lower.find(prefix) {
            return Some(prompt[index..].trim().trim_matches('`').to_string());
        }
    }
    None
}

fn extract_workspace_script_path_from_prompt(prompt: &str) -> Option<String> {
    let normalized = prompt.replace('\\', "/");
    for token in normalized.split_whitespace() {
        let candidate = token
            .trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | ',' | '.' | ')' | '('))
            .trim_start_matches("./");
        if candidate.starts_with("scripts/")
            && [".ps1", ".sh", ".py", ".cmd", ".bat", ".js", ".mjs", ".cjs"]
                .iter()
                .any(|ext| candidate.to_ascii_lowercase().ends_with(ext))
        {
            return Some(candidate.to_string());
        }
    }
    None
}

fn extract_word_after(haystack: &str, prefix: &str) -> Option<String> {
    let start = haystack.find(prefix)? + prefix.len();
    let tail = &haystack[start..];
    let word = tail
        .split_whitespace()
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(
        word.trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | ',' | '.' | ')' | '('))
            .to_string(),
    )
}

fn rewrite_shell_to_maintainer_workflow_args(command: &str) -> Option<Value> {
    let lower = command.to_ascii_lowercase();
    if lower.contains("clean.ps1") {
        return Some(serde_json::json!({
            "workflow": "clean",
            "deep": lower.contains("-deep"),
            "reset": lower.contains("-reset"),
            "prune_dist": lower.contains("-prunedist"),
        }));
    }
    if lower.contains("package-windows.ps1") {
        return Some(serde_json::json!({
            "workflow": "package_windows",
            "installer": lower.contains("-installer"),
            "add_to_path": lower.contains("-addtopath"),
        }));
    }
    if lower.contains("release.ps1") {
        let version = extract_release_arg(command, "-Version");
        let bump = extract_release_arg(command, "-Bump");
        if version.is_none() && bump.is_none() {
            return Some(serde_json::json!({
                "workflow": "release"
            }));
        }
        let mut args = serde_json::json!({
            "workflow": "release",
            "push": lower.contains("-push"),
            "add_to_path": lower.contains("-addtopath"),
            "skip_installer": lower.contains("-skipinstaller"),
            "publish_crates": lower.contains("-publishcrates"),
            "publish_voice_crate": lower.contains("-publishvoicecrate"),
        });
        if let Some(version) = version {
            args["version"] = Value::String(version);
        }
        if let Some(bump) = bump {
            args["bump"] = Value::String(bump);
        }
        return Some(args);
    }
    None
}

fn rewrite_shell_to_workspace_workflow_args(command: &str) -> Option<Value> {
    let lower = command.to_ascii_lowercase();
    if lower.contains("clean.ps1")
        || lower.contains("package-windows.ps1")
        || lower.contains("release.ps1")
    {
        return None;
    }

    if let Some(path) = extract_workspace_script_path_from_prompt(command) {
        return Some(serde_json::json!({
            "workflow": "script_path",
            "path": path,
        }));
    }

    let looks_like_workspace_command = [
        "cargo ",
        "npm ",
        "pnpm ",
        "yarn ",
        "bun ",
        "pytest",
        "go build",
        "go test",
        "make ",
        "just ",
        "task ",
        "./gradlew",
        ".\\gradlew",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    if looks_like_workspace_command {
        Some(serde_json::json!({
            "workflow": "command",
            "command": command.trim(),
        }))
    } else {
        None
    }
}

fn rewrite_host_tool_call(
    tool_name: &mut String,
    args: &mut Value,
    latest_user_prompt: Option<&str>,
) {
    if *tool_name == "shell" {
        let command = args
            .get("command")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if let Some(maintainer_workflow_args) = rewrite_shell_to_maintainer_workflow_args(command) {
            *tool_name = "run_hematite_maintainer_workflow".to_string();
            *args = maintainer_workflow_args;
            return;
        }
        if let Some(workspace_workflow_args) = rewrite_shell_to_workspace_workflow_args(command) {
            *tool_name = "run_workspace_workflow".to_string();
            *args = workspace_workflow_args;
            return;
        }
    }
    let is_surgical_tool = matches!(
        tool_name.as_str(),
        "create_directory"
            | "write_file"
            | "edit_file"
            | "patch_hunk"
            | "multi_replace_file_content"
            | "replace_file_content"
            | "move_file"
            | "delete_file"
    );

    if !is_surgical_tool && *tool_name != "run_hematite_maintainer_workflow" {
        if let Some(prompt_args) =
            latest_user_prompt.and_then(infer_maintainer_workflow_args_from_prompt)
        {
            *tool_name = "run_hematite_maintainer_workflow".to_string();
            *args = prompt_args;
            return;
        }
    }
    // Only allow auto-rewrite for generic shell/command triggers.
    // We NEVER rewrite surgical tools (write/edit) or evidence tools (read/inspect)
    // because that leads to inference-hijack loops.
    let is_generic_command_trigger = matches!(
        tool_name.as_str(),
        "shell" | "run_command" | "workflow" | "run"
    );
    if is_generic_command_trigger && *tool_name != "run_workspace_workflow" {
        if let Some(prompt_args) =
            latest_user_prompt.and_then(infer_workspace_workflow_args_from_prompt)
        {
            *tool_name = "run_workspace_workflow".to_string();
            *args = prompt_args;
            return;
        }
    }
    if should_rewrite_shell_to_fix_plan(tool_name, args, latest_user_prompt) {
        *tool_name = "inspect_host".to_string();
        *args = serde_json::json!({
            "topic": "fix_plan"
        });
    }
    fill_missing_fix_plan_issue(tool_name, args, latest_user_prompt);
    fill_missing_dns_lookup_name(tool_name, args, latest_user_prompt);
    fill_missing_dns_lookup_type(tool_name, args, latest_user_prompt);
    fill_missing_event_query_args(tool_name, args, latest_user_prompt);
}

fn canonical_tool_call_key(tool_name: &str, args: &Value) -> String {
    format!(
        "{}:{}",
        tool_name,
        serde_json::to_string(args).unwrap_or_default()
    )
}

fn normalized_tool_call_for_execution(
    tool_name: &str,
    raw_arguments: &Value,
    gemma4_model: bool,
    latest_user_prompt: Option<&str>,
) -> (String, Value) {
    let mut normalized_name = tool_name.to_string();
    let mut args = if gemma4_model {
        let raw_str = raw_arguments.to_string();
        let normalized_str =
            crate::agent::inference::normalize_tool_argument_string(tool_name, &raw_str);
        serde_json::from_str::<Value>(&normalized_str).unwrap_or_else(|_| raw_arguments.clone())
    } else {
        raw_arguments.clone()
    };
    rewrite_host_tool_call(&mut normalized_name, &mut args, latest_user_prompt);
    (normalized_name, args)
}

#[cfg(test)]
fn normalized_tool_call_key_for_dedupe(
    tool_name: &str,
    raw_arguments: &str,
    gemma4_model: bool,
    latest_user_prompt: Option<&str>,
) -> String {
    let val = serde_json::from_str(raw_arguments).unwrap_or(Value::Null);
    let (normalized_name, args) =
        normalized_tool_call_for_execution(tool_name, &val, gemma4_model, latest_user_prompt);
    canonical_tool_call_key(&normalized_name, &args)
}

impl ConversationManager {
    /// Checks if a tool call is authorized given the current configuration and mode.
    fn check_authorization(
        &self,
        name: &str,
        args: &serde_json::Value,
        config: &crate::agent::config::HematiteConfig,
        yolo_flag: bool,
    ) -> crate::agent::permission_enforcer::AuthorizationDecision {
        crate::agent::permission_enforcer::authorize_tool_call(name, args, config, yolo_flag)
    }

    /// Layer 4: Isolated tool execution logic. Does not mutate 'self' to allow parallelism.
    async fn process_tool_call(
        &self,
        mut call: ToolCallFn,
        config: crate::agent::config::HematiteConfig,
        yolo: bool,
        tx: mpsc::Sender<InferenceEvent>,
        real_id: String,
        budget_tokens: usize,
    ) -> ToolExecutionOutcome {
        let mut msg_results = Vec::new();
        let mut latest_target_dir = None;
        let mut plan_drafted_this_turn = false;
        let mut parsed_plan_handoff = None;
        let gemma4_model =
            crate::agent::inference::is_hematite_native_model(&self.engine.current_model());
        let (normalized_name, mut args) = normalized_tool_call_for_execution(
            &call.name,
            &call.arguments,
            gemma4_model,
            self.history
                .last()
                .and_then(|m| m.content.as_str().split('\n').last()),
        );
        call.name = normalized_name;
        let last_user_prompt = self
            .history
            .iter()
            .rev()
            .find(|message| message.role == "user")
            .map(|message| message.content.as_str());
        rewrite_host_tool_call(&mut call.name, &mut args, last_user_prompt);

        let display = format_tool_display(&call.name, &args);
        let precondition_result = self.validate_action_preconditions(&call.name, &args).await;
        let auth = self.check_authorization(&call.name, &args, &config, yolo);

        // 2. Permission Check
        let decision_result = match precondition_result {
            Err(e) => Err(e),
            Ok(_) => match auth {
                crate::agent::permission_enforcer::AuthorizationDecision::Allow { .. } => Ok(()),
                crate::agent::permission_enforcer::AuthorizationDecision::Ask {
                    reason,
                    source: _,
                } => {
                    let mutation_label =
                        crate::agent::tool_registry::get_mutation_label(&call.name, &args);
                    let (approve_tx, approve_rx) = tokio::sync::oneshot::channel::<bool>();
                    let _ = tx
                        .send(InferenceEvent::ApprovalRequired {
                            id: real_id.clone(),
                            name: call.name.clone(),
                            display: format!("{}\nWhy: {}", display, reason),
                            diff: None,
                            mutation_label,
                            responder: approve_tx,
                        })
                        .await;

                    match approve_rx.await {
                        Ok(true) => Ok(()),
                        _ => Err("Declined by user".into()),
                    }
                }
                crate::agent::permission_enforcer::AuthorizationDecision::Deny {
                    reason, ..
                } => Err(reason),
            },
        };
        let blocked_by_policy =
            matches!(&decision_result, Err(e) if e.starts_with("Action blocked:"));

        // 3. Execution (Local or MCP)
        let (output, is_error) = match decision_result {
            Err(e) if e.starts_with("[auto-redirected shell→inspect_host") => (e, false),
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
                            "Autonomous Scoping: Locked {} in prioritized memory. Reason: {}",
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
                                                diff: None,
                                                mutation_label: Some(
                                                    "Swarm Agentic Integration".to_string(),
                                                ),
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
                } else if matches!(
                    call.name.as_str(),
                    "edit_file" | "patch_hunk" | "multi_search_replace" | "write_file"
                ) && !yolo
                {
                    // ── Diff preview gate ─────────────────────────────────────
                    // Compute what the edit would look like before applying it.
                    // If we can build a diff, require user Y/N in the TUI.
                    // write_file shows the full new content as additions (new files)
                    // or a before/after replacement (overwriting existing files).
                    let diff_result = match call.name.as_str() {
                        "edit_file" => crate::tools::file_ops::compute_edit_file_diff(&args),
                        "patch_hunk" => crate::tools::file_ops::compute_patch_hunk_diff(&args),
                        "write_file" => crate::tools::file_ops::compute_write_file_diff(&args),
                        _ => crate::tools::file_ops::compute_msr_diff(&args),
                    };
                    match diff_result {
                        Ok(diff_text) => {
                            let path_label =
                                args.get("path").and_then(|v| v.as_str()).unwrap_or("file");
                            let (appr_tx, appr_rx) = tokio::sync::oneshot::channel::<bool>();
                            let mutation_label =
                                crate::agent::tool_registry::get_mutation_label(&call.name, &args);
                            let _ = tx
                                .send(InferenceEvent::ApprovalRequired {
                                    id: real_id.clone(),
                                    name: call.name.clone(),
                                    display: format!("Edit preview: {}", path_label),
                                    diff: Some(diff_text),
                                    mutation_label,
                                    responder: appr_tx,
                                })
                                .await;
                            match appr_rx.await {
                                Ok(true) => {
                                    dispatch_tool(&call.name, &args, &config, budget_tokens).await
                                }
                                _ => Err("Edit declined by user.".into()),
                            }
                        }
                        // Diff computation failed (e.g. search string not found yet) —
                        // fall through and let the tool return its own error.
                        Err(_) => dispatch_tool(&call.name, &args, &config, budget_tokens).await,
                    }
                } else if call.name == "verify_build" {
                    // Stream build output line-by-line to the SPECULAR panel so
                    // the operator sees live compiler progress during long builds.
                    crate::tools::verify_build::execute_streaming(&args, tx.clone()).await
                } else if call.name == "shell" {
                    // Stream shell output line-by-line to the SPECULAR panel so
                    // the operator sees live progress during long commands.
                    crate::tools::shell::execute_streaming(&args, tx.clone(), budget_tokens).await
                } else {
                    dispatch_tool(&call.name, &args, &config, budget_tokens).await
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
                    if call.name == "inspect_lines" {
                        self.record_line_inspection(path).await;
                    } else {
                        self.record_read_observation(path).await;
                    }
                }
            }

            if call.name == "verify_build" {
                let ok = output.contains("BUILD OK")
                    || output.contains("BUILD SUCCESS")
                    || output.contains("BUILD OKAY");
                self.record_verify_build_result(ok, &output).await;
            }

            if matches!(
                call.name.as_str(),
                "write_file" | "edit_file" | "patch_hunk" | "multi_search_replace"
            ) || is_mcp_mutating_tool(&call.name)
            {
                if call.name == "write_file" {
                    if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                        if path.ends_with("PLAN.md") {
                            plan_drafted_this_turn = true;
                            if !is_error {
                                if let Some(content) = args.get("content").and_then(|v| v.as_str())
                                {
                                    let resolved = crate::tools::file_ops::resolve_candidate(path);
                                    let _ = crate::tools::plan::sync_plan_blueprint_for_path(
                                        &resolved, content,
                                    );
                                    parsed_plan_handoff =
                                        crate::tools::plan::parse_plan_handoff(content);
                                }
                            }
                        }
                    }
                }
                self.record_successful_mutation(action_target_path(&call.name, &args).as_deref())
                    .await;
            }

            if call.name == "create_directory" {
                if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                    let resolved = crate::tools::file_ops::resolve_candidate(path);
                    latest_target_dir = Some(resolved.to_string_lossy().to_string());
                }
            }

            if let Some(receipt) = self.build_action_receipt(&call.name, &args, &output, is_error) {
                msg_results.push(receipt);
            }
        }

        // 4. Critic Check (Specular Tier 2)
        // Gated: skipped in yolo mode (fast path), only runs on code files with
        // substantive content to avoid burning tokens on trivial doc/config edits.
        if !is_error && !yolo && (call.name == "edit_file" || call.name == "write_file") {
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
            // Web files always get reviewed regardless of length — a 20-line HTML
            // skeleton can still be missing DOCTYPE, meta charset, or linked CSS.
            const WEB_EXTS: &[&str] = &[
                "html", "htm", "css", "js", "ts", "jsx", "tsx", "vue", "svelte",
            ];
            let is_web = WEB_EXTS.contains(&ext);
            let min_lines = if is_web { 5 } else { 50 };
            if !path.is_empty()
                && !content.is_empty()
                && !SKIP_EXTS.contains(&ext)
                && line_count >= min_lines
            {
                if let Some(critique) = self.run_critic_check(path, content, &tx).await {
                    msg_results.push(ChatMessage::system(&format!(
                        "[CRITIC AUTO-FIX REQUIRED — {}]\n\
                        Fix ALL issues below before sending your final response. \
                        Call the appropriate edit tools now.\n\n{}",
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
            latest_target_dir,
            plan_drafted_this_turn,
            parsed_plan_handoff,
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
    latest_target_dir: Option<String>,
    plan_drafted_this_turn: bool,
    parsed_plan_handoff: Option<crate::tools::plan::PlanHandoff>,
}

#[derive(Clone)]
struct CachedToolResult {
    tool_name: String,
}

fn is_code_like_path(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "rs" | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "py"
            | "go"
            | "java"
            | "c"
            | "cpp"
            | "cc"
            | "h"
            | "hpp"
            | "cs"
            | "swift"
            | "kt"
            | "kts"
            | "rb"
            | "php"
    )
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
        "shell" | "bash" | "powershell" => format!("$ {}", get("command")),
        "run_workspace_workflow" => format!("workflow: {}", get("workflow")),
        "trace_runtime_flow" => format!("trace runtime {}", get("topic")),
        "describe_toolchain" => format!("describe toolchain {}", get("topic")),
        "inspect_host" => format!("inspect host {}", get("topic")),
        "write_file"
        | "read_file"
        | "edit_file"
        | "patch_hunk"
        | "inspect_lines"
        | "lsp_get_diagnostics" => format!("{} `{}`", name, get("path")),
        "grep_files" => format!(
            "grep_files pattern='{}' path='{}'",
            get("pattern"),
            get("path")
        ),
        "list_files" => format!("list_files `{}`", get("path")),
        "multi_search_replace" => format!("multi_search_replace `{}`", get("path")),
        _ => {
            // Keep generic debug output strictly bounded so it never desyncs the TUI scroll math
            let rep = format!("{} {:?}", name, args);
            if rep.len() > 100 {
                format!("{}... (truncated)", &rep[..100])
            } else {
                rep
            }
        }
    }
}

// ── Text utilities ────────────────────────────────────────────────────────────

pub(crate) fn shell_looks_like_structured_host_inspection(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    [
        "$env:path",
        "pathvariable",
        "pip --version",
        "pipx --version",
        "winget --version",
        "choco",
        "scoop",
        "get-childitem",
        "gci ",
        "where.exe",
        "where ",
        "cargo --version",
        "rustc --version",
        "git --version",
        "node --version",
        "npm --version",
        "pnpm --version",
        "python --version",
        "python3 --version",
        "deno --version",
        "go version",
        "dotnet --version",
        "uv --version",
        "netstat",
        "findstr",
        "get-nettcpconnection",
        "tcpconnection",
        "listening",
        "ss -",
        "ss ",
        "lsof",
        "tasklist",
        "ipconfig",
        "get-netipconfiguration",
        "get-netadapter",
        "route print",
        "ifconfig",
        "ip addr",
        "ip route",
        "resolv.conf",
        "get-service",
        "sc query",
        "systemctl",
        "service --status-all",
        "get-process",
        "working set",
        "ps -eo",
        "ps aux",
        "desktop",
        "downloads",
        "get-netfirewallprofile",
        "win32_powerplan",
        "win32_operatingsystem",
        "win32_processor",
        "wmic",
        "loadpercentage",
        "totalvisiblememory",
        "freephysicalmemory",
        "get-wmiobject",
        "get-ciminstance",
        "get-cpu",
        "processorname",
        "clockspeed",
        "top memory",
        "top cpu",
        "resource usage",
        "powercfg",
        "uptime",
        "lastbootuptime",
        // registry reads for OS/version/update/security info — always use inspect_host
        "hklm:",
        "hkcu:",
        "hklm:\\",
        "hkcu:\\",
        "currentversion",
        "productname",
        "displayversion",
        "get-itemproperty",
        "get-itempropertyvalue",
        // updates
        "get-windowsupdatelog",
        "windowsupdatelog",
        "microsoft.update.session",
        "createupdatesearcher",
        "wuauserv",
        "usoclient",
        "get-hotfix",
        "wu_",
        // security / defender
        "get-mpcomputerstatus",
        "get-mppreference",
        "get-mpthreat",
        "start-mpscan",
        "win32_computersecurity",
        "softwarelicensingproduct",
        "enablelua",
        "get-netfirewallrule",
        "netfirewallprofile",
        "antivirus",
        "defenderstatus",
        // disk health / smart
        "get-physicaldisk",
        "get-disk",
        "get-volume",
        "get-psdrive",
        "psdrive",
        "manage-bde",
        "bitlockervolume",
        "get-bitlockervolume",
        "get-smbencryptionstatus",
        "smbencryption",
        "get-netlanmanagerconnection",
        "lanmanager",
        "msstoragedriver_failurepredic",
        "win32_diskdrive",
        "smartstatus",
        "diskstatus",
        "get-counter",
        "intensity",
        "benchmark",
        "thrash",
        "get-item",
        "test-path",
        // gpo / certs / integrity / domain
        "gpresult",
        "applied gpo",
        "cert:\\",
        "cert:",
        "component based servicing",
        "componentstore",
        "get-computerinfo",
        "win32_computersystem",
        // battery
        "win32_battery",
        "batterystaticdata",
        "batteryfullchargedcapacity",
        "batterystatus",
        "estimatedchargeremaining",
        // crashes / event log (broader)
        "get-winevent",
        "eventid",
        "bugcheck",
        "kernelpower",
        "win32_ntlogevent",
        "filterhashtable",
        // scheduled tasks
        "get-scheduledtask",
        "get-scheduledtaskinfo",
        "schtasks",
        "taskscheduler",
        "get-acl",
        "icacls",
        "takeown",
        "event id 4624",
        "eventid 4624",
        "who logged in",
        "logon history",
        "login history",
        "get-smbshare",
        "net share",
        "mbps",
        "throughput",
        "whoami",
        // general cim/wmi diagnostic queries — always use inspect_host
        "get-ciminstance win32",
        "get-wmiobject win32",
        // network admin — always use inspect_host
        "arp -",
        "arp -a",
        "tracert ",
        "traceroute ",
        "tracepath ",
        "get-dnsclientcache",
        "ipconfig /displaydns",
        "get-netroute",
        "get-netneighbor",
        "net view",
        "get-smbconnection",
        "get-smbmapping",
        "get-psdrive",
        "fdrespub",
        "fdphost",
        "ssdpsrv",
        "upnphost",
        "avahi-browse",
        "route print",
        "ip neigh",
        // audio / bluetooth — always use inspect_host
        "get-pnpdevice -class audioendpoint",
        "get-pnpdevice -class media",
        "win32_sounddevice",
        "audiosrv",
        "audioendpointbuilder",
        "windows audio",
        "get-pnpdevice -class bluetooth",
        "bthserv",
        "bthavctpsvc",
        "btagservice",
        "bluetoothuserservice",
        "msiserver",
        "appxsvc",
        "clipsvc",
        "installservice",
        "desktopappinstaller",
        "microsoft.windowsstore",
        "get-appxpackage microsoft.desktopappinstaller",
        "get-appxpackage microsoft.windowsstore",
        "winget source",
        "winget --info",
        "onedrive",
        "onedrive.exe",
        "files on-demand",
        "known folder backup",
        "disablefilesyncngsc",
        "kfmsilentoptin",
        "kfmblockoptin",
        "get-process chrome",
        "get-process msedge",
        "get-process firefox",
        "get-process msedgewebview2",
        "google chrome",
        "microsoft edge",
        "mozilla firefox",
        "webview2",
        "msedgewebview2",
        "startmenuinternet",
        "urlassociations\\http\\userchoice",
        "urlassociations\\https\\userchoice",
        "software\\policies\\microsoft\\edge",
        "software\\policies\\google\\chrome",
        "get-winevent",
        "event id",
        "eventlog",
        "event viewer",
        "wevtutil",
        "cmdkey",
        "credential manager",
        "get-tpm",
        "confirm-securebootuefi",
        "win32_tpm",
        "dsregcmd",
        "webauthmanager",
        "web account manager",
        "tokenbroker",
        "token broker",
        "aad broker",
        "brokerplugin",
        "microsoft.aad.brokerplugin",
        "workplace join",
        "device registration",
        "secure boot",
        // active directory - always use inspect_host
        "get-aduser",
        "get-addomain",
        "get-adforest",
        "get-adgroup",
        "get-adcomputer",
        "activedirectory",
        "get-localuser",
        "get-localgroup",
        "get-localgroupmember",
        "net user",
        "net localgroup",
        "netsh winhttp show proxy",
        "get-itemproperty.*proxy",
        "get-netadapter",
        "netsh wlan show",
        "test-netconnection",
        "resolve-dnsname",
        "nslookup",
        "dig ",
        "gethostentry",
        "gethostaddresses",
        "getipaddresses",
        "[system.net.dns]",
        "net.dns]",
        "get-netfirewallrule",
        // docker / wsl / ssh — always use inspect_host
        "docker ps",
        "docker info",
        "docker images",
        "docker container",
        "docker inspect",
        "docker volume",
        "docker system df",
        "docker compose ls",
        "wsl --list",
        "wsl -l",
        "wsl --status",
        "wsl --version",
        "wsl -d",
        "wsl df",
        "wsl du",
        "/mnt/c",
        "ssh -v",
        "get-service sshd",
        "get-service -name sshd",
        "cat ~/.ssh",
        "ls ~/.ssh",
        "ls -la ~/.ssh",
        // env / hosts / git config
        "get-childitem env:",
        "dir env:",
        "printenv",
        "[environment]::getenvironmentvariable",
        "get-content.*hosts",
        "cat /etc/hosts",
        "type c:\\windows\\system32\\drivers\\etc\\hosts",
        "git config --global --list",
        "git config --list",
        "git config --global",
        // database services
        "get-service mysql",
        "get-service postgresql",
        "get-service mongodb",
        "get-service redis",
        "get-service mssql",
        "get-service mariadb",
        "systemctl status postgresql",
        "systemctl status mysql",
        "systemctl status mongod",
        "systemctl status redis",
        // installed software
        "winget list",
        "get-package",
        "get-itempropert.*uninstall",
        "dpkg --get-selections",
        "rpm -qa",
        "brew list",
        // user accounts
        "get-localuser",
        "get-localgroupmember",
        "net user",
        "query user",
        "net localgroup administrators",
        // audit policy
        "auditpol /get",
        "auditpol",
        // shares
        "get-smbshare",
        "get-smbserverconfiguration",
        "net share",
        "net use",
        // dns servers
        "get-dnsclientserveraddress",
        "get-dnsclientdohserveraddress",
        "get-dnsclientglobalsetting",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
        || lower.starts_with("host ")
}

// Moved strip_think_blocks to inference.rs

fn cap_output(text: &str, max_bytes: usize) -> String {
    cap_output_for_tool(text, max_bytes, "output")
}

/// Cap tool output at `max_bytes`. When the output exceeds the cap, write the
/// full content to `.hematite/scratch/<tool_name>_<timestamp>.txt` and include
/// the path in the truncation notice so the model can read the rest with
/// `read_file` instead of losing it entirely.
fn cap_output_for_tool(text: &str, max_bytes: usize, tool_name: &str) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }

    // Write full output to scratch so the model can access it.
    let scratch_path = write_output_to_scratch(text, tool_name);

    let mut split_at = max_bytes;
    while !text.is_char_boundary(split_at) && split_at > 0 {
        split_at -= 1;
    }

    let tail = match &scratch_path {
        Some(p) => format!(
            "\n... [output truncated — full output ({} bytes, {} lines) saved to '{}' — use read_file to access the rest]",
            text.len(),
            text.lines().count(),
            p
        ),
        None => format!("\n... [output capped at {}B]", max_bytes),
    };

    format!("{}{}", &text[..split_at], tail)
}

/// Write text to `.hematite/scratch/<tool>_<timestamp>.txt`.
/// Returns the relative path on success, None if the write fails.
fn write_output_to_scratch(text: &str, tool_name: &str) -> Option<String> {
    let scratch_dir = crate::tools::file_ops::hematite_dir().join("scratch");
    if std::fs::create_dir_all(&scratch_dir).is_err() {
        return None;
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Sanitize tool name for use in filename
    let safe_name: String = tool_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let filename = format!("{}_{}.txt", safe_name, ts);
    let abs_path = scratch_dir.join(&filename);
    if std::fs::write(&abs_path, text).is_err() {
        return None;
    }
    Some(format!(".hematite/scratch/{}", filename))
}

#[derive(Default)]
struct PromptBudgetStats {
    summarized_tool_results: usize,
    collapsed_tool_results: usize,
    trimmed_chat_messages: usize,
    dropped_messages: usize,
}

fn estimate_prompt_tokens(messages: &[ChatMessage]) -> usize {
    crate::agent::inference::estimate_message_batch_tokens(messages)
}

fn summarize_prompt_blob(text: &str, max_chars: usize) -> String {
    let budget = compaction::SummaryCompressionBudget {
        max_chars,
        max_lines: 3,
        max_line_chars: max_chars.clamp(80, 240),
    };
    let compressed = compaction::compress_summary(text, budget).summary;
    if compressed.is_empty() {
        String::new()
    } else {
        compressed
    }
}

fn summarize_tool_message_for_budget(message: &ChatMessage) -> String {
    let tool_name = message.name.as_deref().unwrap_or("tool");
    let body = summarize_prompt_blob(message.content.as_str(), 320);
    format!(
        "[Prompt-budget summary of prior `{}` result]\n{}",
        tool_name, body
    )
}

fn summarize_chat_message_for_budget(message: &ChatMessage) -> String {
    let role = message.role.as_str();
    let body = summarize_prompt_blob(message.content.as_str(), 240);
    format!(
        "[Prompt-budget summary of earlier {} message]\n{}",
        role, body
    )
}

fn normalize_prompt_start(messages: &mut Vec<ChatMessage>) {
    if messages.len() > 1 && messages[1].role != "user" {
        messages.insert(1, ChatMessage::user("Continuing previous context..."));
    }
}

fn enforce_prompt_budget(
    prompt_msgs: &mut Vec<ChatMessage>,
    context_length: usize,
) -> Option<String> {
    let target_tokens = ((context_length as f64) * 0.68) as usize;
    if estimate_prompt_tokens(prompt_msgs) <= target_tokens {
        return None;
    }

    let mut stats = PromptBudgetStats::default();

    // 1. Summarize the newest large tool outputs first.
    let mut tool_indices: Vec<usize> = prompt_msgs
        .iter()
        .enumerate()
        .filter_map(|(idx, msg)| (msg.role == "tool").then_some(idx))
        .collect();
    for idx in tool_indices.iter().rev().copied() {
        if estimate_prompt_tokens(prompt_msgs) <= target_tokens {
            break;
        }
        let original = prompt_msgs[idx].content.as_str().to_string();
        if original.len() > 1200 {
            prompt_msgs[idx].content =
                MessageContent::Text(summarize_tool_message_for_budget(&prompt_msgs[idx]));
            stats.summarized_tool_results += 1;
        }
    }

    // 2. Collapse older tool results aggressively, keeping only the most recent two verbatim/summarized.
    tool_indices = prompt_msgs
        .iter()
        .enumerate()
        .filter_map(|(idx, msg)| (msg.role == "tool").then_some(idx))
        .collect();
    if tool_indices.len() > 2 {
        for idx in tool_indices
            .iter()
            .take(tool_indices.len().saturating_sub(2))
            .copied()
        {
            if estimate_prompt_tokens(prompt_msgs) <= target_tokens {
                break;
            }
            prompt_msgs[idx].content = MessageContent::Text(
                "[Earlier tool output omitted to stay within the prompt budget.]".to_string(),
            );
            stats.collapsed_tool_results += 1;
        }
    }

    // 3. Trim older long chat messages, but preserve the final user request.
    let last_user_idx = prompt_msgs.iter().rposition(|m| m.role == "user");
    for idx in 1..prompt_msgs.len() {
        if estimate_prompt_tokens(prompt_msgs) <= target_tokens {
            break;
        }
        if Some(idx) == last_user_idx {
            continue;
        }
        let role = prompt_msgs[idx].role.as_str();
        if matches!(role, "user" | "assistant") && prompt_msgs[idx].content.as_str().len() > 900 {
            prompt_msgs[idx].content =
                MessageContent::Text(summarize_chat_message_for_budget(&prompt_msgs[idx]));
            stats.trimmed_chat_messages += 1;
        }
    }

    // 4. Middle-Out Condensation: Drop oldest tool and assistant messages first, preserving ALL user instructions.
    let preserve_last_user_idx = prompt_msgs.iter().rposition(|m| m.role == "user");
    let mut idx = 1usize;
    while estimate_prompt_tokens(prompt_msgs) > target_tokens && prompt_msgs.len() > 2 {
        if idx >= prompt_msgs.len() {
            break;
        }

        let role = prompt_msgs[idx].role.as_str();
        if role == "user" || Some(idx) == preserve_last_user_idx {
            // NEVER drop user requests if possible, let them stand as immutable context.
            idx += 1;
            continue;
        }

        // It's a tool or assistant message from the middle. Drop it.
        prompt_msgs.remove(idx);
        stats.dropped_messages += 1;
    }

    // 5. If STILL over budget (e.g. user pasted a giant file in the prompt), drop oldest user messages except the latest.
    let mut idx = 1usize;
    while estimate_prompt_tokens(prompt_msgs) > target_tokens && prompt_msgs.len() > 2 {
        if Some(idx) == preserve_last_user_idx {
            idx += 1;
            if idx >= prompt_msgs.len() {
                break;
            }
            continue;
        }
        if idx >= prompt_msgs.len() {
            break;
        }
        prompt_msgs.remove(idx);
        stats.dropped_messages += 1;
    }

    normalize_prompt_start(prompt_msgs);

    let new_tokens = estimate_prompt_tokens(prompt_msgs);
    if stats.summarized_tool_results == 0
        && stats.collapsed_tool_results == 0
        && stats.trimmed_chat_messages == 0
        && stats.dropped_messages == 0
    {
        return None;
    }

    Some(format!(
        "Prompt Budget Guard: trimmed prompt to about {} tokens (target {}). Summarized {} large tool result(s), collapsed {} older tool result(s), trimmed {} chat message(s), and dropped {} old message(s).",
        new_tokens,
        target_tokens,
        stats.summarized_tool_results,
        stats.collapsed_tool_results,
        stats.trimmed_chat_messages,
        stats.dropped_messages
    ))
}

/// Split text into chunks of roughly `words_per_chunk` whitespace-separated tokens.
/// Returns true for short, direct tool-use requests that don't benefit from deep reasoning.
/// Used to skip the auto-/think prepend so the model calls the tool immediately
/// instead of spending thousands of tokens deliberating over a trivial task.
fn is_quick_tool_request(input: &str) -> bool {
    let lower = input.to_lowercase();
    // Explicit run_code requests — sandbox calls need no reasoning warmup.
    if lower.contains("run_code") || lower.contains("run code") {
        return true;
    }
    // Short compute/test requests — "calculate X", "test this", "execute Y"
    let is_short = input.len() < 120;
    let compute_keywords = [
        "calculate",
        "compute",
        "execute",
        "run this",
        "test this",
        "what is ",
        "how much",
        "how many",
        "convert ",
        "print ",
    ];
    if is_short && compute_keywords.iter().any(|k| lower.contains(k)) {
        return true;
    }
    false
}

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

fn repeated_read_target(call: &crate::agent::inference::ToolCallFn) -> Option<String> {
    if call.name != "read_file" {
        return None;
    }
    let mut args = call.arguments.clone();
    crate::agent::inference::normalize_tool_argument_value(&call.name, &mut args);
    let path = args.get("path").and_then(|v| v.as_str())?;
    Some(normalize_workspace_path(path))
}

fn order_batch_reads_first(
    calls: Vec<crate::agent::inference::ToolCallResponse>,
) -> (
    Vec<crate::agent::inference::ToolCallResponse>,
    Option<String>,
) {
    let has_reads = calls.iter().any(|c| {
        matches!(
            c.function.name.as_str(),
            "read_file" | "inspect_lines" | "grep_files" | "list_files"
        )
    });
    let has_edits = calls.iter().any(|c| {
        matches!(
            c.function.name.as_str(),
            "write_file" | "edit_file" | "patch_hunk" | "multi_search_replace"
        )
    });
    if has_reads && has_edits {
        let reads: Vec<_> = calls
            .into_iter()
            .filter(|c| {
                !matches!(
                    c.function.name.as_str(),
                    "write_file" | "edit_file" | "patch_hunk" | "multi_search_replace"
                )
            })
            .collect();
        let note = Some("Batch ordering: deferring edits until reads complete.".to_string());
        (reads, note)
    } else {
        (calls, None)
    }
}

fn grep_output_is_high_fanout(output: &str) -> bool {
    let Some(summary) = output.lines().next() else {
        return false;
    };
    let hunk_count = summary
        .split(", ")
        .find_map(|part| {
            part.strip_suffix(" hunk(s)")
                .and_then(|value| value.parse::<usize>().ok())
        })
        .unwrap_or(0);
    let match_count = summary
        .split(' ')
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    hunk_count >= 8 || match_count >= 12
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
    let metadata = crate::agent::inference::tool_metadata_for_name(name);
    !metadata.mutates_workspace && !metadata.external_surface
}

fn should_use_vein_in_chat(query: &str, docs_only_mode: bool) -> bool {
    if docs_only_mode {
        return true;
    }

    let lower = query.to_ascii_lowercase();
    [
        "what did we decide",
        "why did we decide",
        "what did we say",
        "what did we do",
        "earlier today",
        "yesterday",
        "last week",
        "last month",
        "earlier",
        "remember",
        "session",
        "import",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
        || lower
            .split(|ch: char| !(ch.is_ascii_digit() || ch == '-'))
            .any(|token| token.len() == 10 && token.chars().nth(4) == Some('-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_lm_studio_context_budget_mismatch_as_context_window() {
        let detail = r#"LM Studio error 400 Bad Request: {"error":"The number of tokens to keep from the initial prompt is greater than the context length (n_keep: 28768>= n_ctx: 4096). Try to load the model with a larger context length, or provide a shorter input."}"#;
        let class = classify_runtime_failure(detail);
        assert_eq!(class, RuntimeFailureClass::ContextWindow);
        assert_eq!(class.tag(), "context_window");
        assert!(format_runtime_failure(class, detail).contains("[failure:context_window]"));
    }

    #[test]
    fn formatted_runtime_failure_is_not_wrapped_twice() {
        let detail =
            "[failure:provider_degraded] Retry once automatically, then narrow the turn or restart LM Studio if it persists. Detail: LMS unreachable: Request failed";
        let formatted = format_runtime_failure(RuntimeFailureClass::ProviderDegraded, detail);
        assert_eq!(formatted, detail);
        assert_eq!(formatted.matches("[failure:provider_degraded]").count(), 1);
    }

    #[test]
    fn explicit_search_detection_requires_search_language() {
        assert!(is_explicit_web_search_request("search for ocean bennett"));
        assert!(is_explicit_web_search_request("google ocean bennett"));
        assert!(is_explicit_web_search_request("look up ocean bennett"));
        assert!(!is_explicit_web_search_request("who is ocean bennett"));
    }

    #[test]
    fn explicit_search_query_extracts_leading_search_clause_from_mixed_request() {
        assert_eq!(
            extract_explicit_web_search_query(
                "google uefn toolbelt then make a folder on my desktop called oupa with a single file html website talking about it"
            ),
            Some("uefn toolbelt".to_string())
        );
    }

    #[test]
    fn research_provider_fallback_mentions_direct_search_results() {
        let fallback = build_research_provider_fallback(
            "[Source: SearXNG]\n\n### 1. [Ocean Bennett](https://example.com)\nBio",
        );
        assert!(fallback.contains("Local web search succeeded"));
        assert!(fallback.contains("[Source: SearXNG]"));
        assert!(fallback.contains("Ocean Bennett"));
    }

    #[test]
    fn runtime_failure_maps_to_provider_and_checkpoint_state() {
        assert_eq!(
            provider_state_for_runtime_failure(RuntimeFailureClass::ContextWindow),
            Some(ProviderRuntimeState::ContextWindow)
        );
        assert_eq!(
            checkpoint_state_for_runtime_failure(RuntimeFailureClass::ContextWindow),
            Some(OperatorCheckpointState::BlockedContextWindow)
        );
        assert_eq!(
            provider_state_for_runtime_failure(RuntimeFailureClass::ProviderDegraded),
            Some(ProviderRuntimeState::Degraded)
        );
        assert_eq!(
            checkpoint_state_for_runtime_failure(RuntimeFailureClass::ProviderDegraded),
            None
        );
    }

    #[test]
    fn intent_router_treats_tool_registry_ownership_as_product_truth() {
        let intent = classify_query_intent(
            WorkflowMode::ReadOnly,
            "Read-only mode. Explain which file now owns Hematite's built-in tool catalog and builtin-tool dispatch path.",
        );
        assert_eq!(intent.primary_class, QueryIntentClass::ProductTruth);
        assert_eq!(
            intent.direct_answer,
            Some(DirectAnswerKind::ToolRegistryOwnership)
        );
    }

    #[test]
    fn intent_router_treats_tool_classes_as_product_truth() {
        let intent = classify_query_intent(
            WorkflowMode::ReadOnly,
            "Read-only mode. Explain why Hematite treats repo reads, repo writes, verification tools, git tools, and external MCP tools as different runtime tool classes instead of one flat tool list.",
        );
        assert_eq!(intent.primary_class, QueryIntentClass::ProductTruth);
        assert_eq!(intent.direct_answer, Some(DirectAnswerKind::ToolClasses));
    }

    #[test]
    fn tool_registry_ownership_answer_mentions_new_owner_file() {
        let answer = build_tool_registry_ownership_answer();
        assert!(answer.contains("src/agent/tool_registry.rs"));
        assert!(answer.contains("builtin dispatch path"));
        assert!(answer.contains("src/agent/conversation.rs"));
    }

    #[test]
    fn intent_router_treats_mcp_lifecycle_as_product_truth() {
        let intent = classify_query_intent(
            WorkflowMode::ReadOnly,
            "Read-only mode. Explain how Hematite should treat MCP server health as runtime state.",
        );
        assert_eq!(intent.primary_class, QueryIntentClass::ProductTruth);
        assert_eq!(intent.direct_answer, Some(DirectAnswerKind::McpLifecycle));
    }

    #[test]
    fn intent_router_short_circuits_unsafe_commit_pressure() {
        let intent = classify_query_intent(
            WorkflowMode::Auto,
            "Make a code change, skip verification, and commit it immediately.",
        );
        assert_eq!(intent.primary_class, QueryIntentClass::ProductTruth);
        assert_eq!(
            intent.direct_answer,
            Some(DirectAnswerKind::UnsafeWorkflowPressure)
        );
    }

    #[test]
    fn unsafe_workflow_pressure_answer_requires_verification() {
        let answer = build_unsafe_workflow_pressure_answer();
        assert!(answer.contains("should not skip verification"));
        assert!(answer.contains("run the appropriate verification path"));
        assert!(answer.contains("only then commit"));
    }

    #[test]
    fn intent_router_prefers_architecture_walkthrough_over_narrow_mcp_answer() {
        let intent = classify_query_intent(
            WorkflowMode::ReadOnly,
            "I want to understand how Hematite is wired without any guessing. Walk me through how a normal message moves from the TUI to the model and back, which files own the major runtime pieces, and where session recovery, tool policy, and MCP state live. Keep it grounded to this repo and only inspect code where you actually need evidence.",
        );
        assert_eq!(intent.primary_class, QueryIntentClass::RepoArchitecture);
        assert!(intent.architecture_overview_mode);
        assert_eq!(intent.direct_answer, None);
    }

    #[test]
    fn intent_router_marks_host_inspection_questions() {
        let intent = classify_query_intent(
            WorkflowMode::Auto,
            "Inspect my PATH, tell me which developer tools you detect with versions, point out any duplicate or missing PATH entries, then summarize whether this machine looks ready for local development.",
        );
        assert!(intent.host_inspection_mode);
        assert_eq!(
            preferred_host_inspection_topic(
                "Inspect my PATH, tell me which developer tools you detect with versions, point out any duplicate or missing PATH entries, then summarize whether this machine looks ready for local development."
            ),
            Some("summary")
        );
    }

    #[test]
    fn intent_router_treats_purpose_question_as_local_identity() {
        let intent = classify_query_intent(WorkflowMode::Auto, "What is your purpose?");
        assert_eq!(intent.direct_answer, Some(DirectAnswerKind::Identity));
    }

    #[test]
    fn chat_mode_uses_vein_for_historical_or_docs_only_queries() {
        assert!(should_use_vein_in_chat(
            "What did we decide on 2026-04-09 about docs-only mode?",
            false
        ));
        assert!(should_use_vein_in_chat("Summarize these local notes", true));
        assert!(!should_use_vein_in_chat("Tell me a joke", false));
    }

    #[test]
    fn shell_host_inspection_guard_matches_path_and_version_commands() {
        assert!(shell_looks_like_structured_host_inspection(
            "$env:PATH -split ';'"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "cargo --version"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-NetTCPConnection -LocalPort 3000"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "netstat -ano | findstr :3000"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-Process | Sort-Object WS -Descending"
        ));
        assert!(shell_looks_like_structured_host_inspection("ipconfig /all"));
        assert!(shell_looks_like_structured_host_inspection("Get-Service"));
        assert!(shell_looks_like_structured_host_inspection(
            "winget --version"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "wsl df -h && wsl du -sh /mnt/c 2>&1 | head -5"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-NetNeighbor -AddressFamily IPv4"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-SmbConnection"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-Service FDResPub,fdPHost,SSDPSRV,upnphost"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-PnpDevice -Class AudioEndpoint"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-CimInstance Win32_SoundDevice"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-PnpDevice -Class Bluetooth"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-Service bthserv,BthAvctpSvc,BTAGService"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-Service msiserver,AppXSvc,ClipSVC,InstallService"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-AppxPackage Microsoft.DesktopAppInstaller"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "winget source list"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-Process OneDrive"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-ItemProperty HKCU:\\Software\\Microsoft\\OneDrive\\Accounts"
        ));
        assert!(shell_looks_like_structured_host_inspection("cmdkey /list"));
        assert!(shell_looks_like_structured_host_inspection("Get-Tpm"));
        assert!(shell_looks_like_structured_host_inspection(
            "Confirm-SecureBootUEFI"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "dsregcmd /status"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-Service TokenBroker,wlidsvc,OneAuth"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-AppxPackage Microsoft.AAD.BrokerPlugin"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "host github.com"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "powershell -Command \"$ip = [System.Net.Dns]::GetHostAddresses('github.com'); $ip | ForEach-Object { $_.Address }\""
        ));
    }

    #[test]
    fn dns_shell_target_extraction_handles_common_lookup_forms() {
        assert_eq!(
            extract_dns_lookup_target_from_shell("host github.com").as_deref(),
            Some("github.com")
        );
        assert_eq!(
            extract_dns_lookup_target_from_shell(
                "powershell -Command \"Resolve-DnsName -Name github.com -Type A\""
            )
            .as_deref(),
            Some("github.com")
        );
        assert_eq!(
            extract_dns_lookup_target_from_shell(
                "powershell -Command \"$ip = [System.Net.Dns]::GetHostAddresses('github.com'); $ip | ForEach-Object { $_.Address }\""
            )
            .as_deref(),
            Some("github.com")
        );
    }

    #[test]
    fn dns_prompt_target_extraction_handles_plain_english_questions() {
        assert_eq!(
            extract_dns_lookup_target_from_text("Show me the A record for github.com").as_deref(),
            Some("github.com")
        );
        assert_eq!(
            extract_dns_lookup_target_from_text("What is the IP address of google.com").as_deref(),
            Some("google.com")
        );
    }

    #[test]
    fn dns_record_type_extraction_handles_prompt_and_shell_forms() {
        assert_eq!(
            extract_dns_record_type_from_text("Show me the A record for github.com"),
            Some("A")
        );
        assert_eq!(
            extract_dns_record_type_from_text("What is the IP address of google.com"),
            Some("A")
        );
        assert_eq!(
            extract_dns_record_type_from_text("Resolve the MX record for example.com"),
            Some("MX")
        );
        assert_eq!(
            extract_dns_record_type_from_shell(
                "powershell -Command \"Resolve-DnsName -Name github.com -Type A\""
            ),
            Some("A")
        );
        assert_eq!(
            extract_dns_record_type_from_shell("nslookup -type=mx example.com"),
            Some("MX")
        );
    }

    #[test]
    fn fill_missing_dns_lookup_name_backfills_from_latest_user_prompt() {
        let mut tool_name = "inspect_host".to_string();
        let mut args = serde_json::json!({
            "topic": "dns_lookup"
        });
        rewrite_host_tool_call(
            &mut tool_name,
            &mut args,
            Some("Show me the A record for github.com"),
        );
        assert_eq!(tool_name, "inspect_host");
        assert_eq!(
            args.get("name").and_then(|value| value.as_str()),
            Some("github.com")
        );
        assert_eq!(args.get("type").and_then(|value| value.as_str()), Some("A"));
    }

    #[test]
    fn host_inspection_args_from_prompt_populates_dns_lookup_fields() {
        let args =
            host_inspection_args_from_prompt("dns_lookup", "What is the IP address of google.com");
        assert_eq!(
            args.get("name").and_then(|value| value.as_str()),
            Some("google.com")
        );
        assert_eq!(args.get("type").and_then(|value| value.as_str()), Some("A"));
    }

    #[test]
    fn host_inspection_args_from_prompt_populates_event_query_fields() {
        let args = host_inspection_args_from_prompt(
            "event_query",
            "Show me all System errors from the Event Log that occurred in the last 4 hours.",
        );
        assert_eq!(
            args.get("log").and_then(|value| value.as_str()),
            Some("System")
        );
        assert_eq!(
            args.get("level").and_then(|value| value.as_str()),
            Some("Error")
        );
        assert_eq!(args.get("hours").and_then(|value| value.as_u64()), Some(4));
    }

    #[test]
    fn fill_missing_event_query_args_backfills_from_latest_user_prompt() {
        let mut tool_name = "inspect_host".to_string();
        let mut args = serde_json::json!({
            "topic": "event_query"
        });
        rewrite_host_tool_call(
            &mut tool_name,
            &mut args,
            Some("Show me all System errors from the Event Log that occurred in the last 4 hours."),
        );
        assert_eq!(tool_name, "inspect_host");
        assert_eq!(
            args.get("log").and_then(|value| value.as_str()),
            Some("System")
        );
        assert_eq!(
            args.get("level").and_then(|value| value.as_str()),
            Some("Error")
        );
        assert_eq!(args.get("hours").and_then(|value| value.as_u64()), Some(4));
    }

    #[test]
    fn intent_router_picks_ports_for_listening_port_questions() {
        assert_eq!(
            preferred_host_inspection_topic(
                "Show me what is listening on port 3000 and whether anything unexpected is exposed."
            ),
            Some("ports")
        );
    }

    #[test]
    fn intent_router_picks_processes_for_host_process_questions() {
        assert_eq!(
            preferred_host_inspection_topic(
                "Show me what processes are using the most RAM right now."
            ),
            Some("processes")
        );
    }

    #[test]
    fn intent_router_picks_network_for_adapter_questions() {
        assert_eq!(
            preferred_host_inspection_topic(
                "Show me my active network adapters, IP addresses, gateways, and DNS servers."
            ),
            Some("network")
        );
    }

    #[test]
    fn intent_router_picks_services_for_service_questions() {
        assert_eq!(
            preferred_host_inspection_topic(
                "Show me the running services and startup types that matter for a normal dev machine."
            ),
            Some("services")
        );
    }

    #[test]
    fn intent_router_picks_env_doctor_for_package_manager_questions() {
        assert_eq!(
            preferred_host_inspection_topic(
                "Run an environment doctor on this machine and tell me whether my PATH and package managers look sane."
            ),
            Some("env_doctor")
        );
    }

    #[test]
    fn intent_router_picks_fix_plan_for_host_remediation_questions() {
        assert_eq!(
            preferred_host_inspection_topic("How do I fix cargo not found on this machine?"),
            Some("fix_plan")
        );
        assert_eq!(
            preferred_host_inspection_topic(
                "How do I fix Hematite when LM Studio is not reachable on localhost:1234?"
            ),
            Some("fix_plan")
        );
    }

    #[test]
    fn intent_router_picks_audio_for_sound_and_microphone_questions() {
        assert_eq!(
            preferred_host_inspection_topic("Why is there no sound from my speakers right now?"),
            Some("audio")
        );
        assert_eq!(
            preferred_host_inspection_topic(
                "Check my microphone and playback devices because Windows Audio seems broken."
            ),
            Some("audio")
        );
    }

    #[test]
    fn intent_router_picks_bluetooth_for_pairing_and_headset_questions() {
        assert_eq!(
            preferred_host_inspection_topic(
                "Why won't this Bluetooth headset pair and stay connected?"
            ),
            Some("bluetooth")
        );
        assert_eq!(
            preferred_host_inspection_topic("Check my Bluetooth radio and pairing status."),
            Some("bluetooth")
        );
    }

    #[test]
    fn fill_missing_fix_plan_issue_backfills_last_user_prompt() {
        let mut args = serde_json::json!({
            "topic": "fix_plan"
        });

        fill_missing_fix_plan_issue(
            "inspect_host",
            &mut args,
            Some("/think\nHow do I fix cargo not found on this machine?"),
        );

        assert_eq!(
            args.get("issue").and_then(|value| value.as_str()),
            Some("How do I fix cargo not found on this machine?")
        );
    }

    #[test]
    fn shell_fix_question_rewrites_to_fix_plan() {
        let args = serde_json::json!({
            "command": "where cargo"
        });

        assert!(should_rewrite_shell_to_fix_plan(
            "shell",
            &args,
            Some("How do I fix cargo not found on this machine?")
        ));
    }

    #[test]
    fn fix_plan_dedupe_key_matches_rewritten_shell_probe() {
        let latest_user_prompt = Some("How do I fix cargo not found on this machine?");
        let shell_key = normalized_tool_call_key_for_dedupe(
            "shell",
            r#"{"command":"where cargo"}"#,
            false,
            latest_user_prompt,
        );
        let fix_plan_key = normalized_tool_call_key_for_dedupe(
            "inspect_host",
            r#"{"topic":"fix_plan"}"#,
            false,
            latest_user_prompt,
        );

        assert_eq!(shell_key, fix_plan_key);
    }

    #[test]
    fn shell_cleanup_script_rewrites_to_maintainer_workflow() {
        let (tool_name, args) = normalized_tool_call_for_execution(
            "shell",
            &serde_json::json!({"command":"pwsh ./clean.ps1 -Deep -PruneDist"}),
            false,
            Some("Run my cleanup scripts."),
        );

        assert_eq!(tool_name, "run_hematite_maintainer_workflow");
        assert_eq!(
            args.get("workflow").and_then(|value| value.as_str()),
            Some("clean")
        );
        assert_eq!(
            args.get("deep").and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            args.get("prune_dist").and_then(|value| value.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn shell_release_script_rewrites_to_maintainer_workflow() {
        let (tool_name, args) = normalized_tool_call_for_execution(
            "shell",
            &serde_json::json!({"command":"pwsh ./release.ps1 -Version 0.4.5 -Push -AddToPath"}),
            false,
            Some("Run the release flow."),
        );

        assert_eq!(tool_name, "run_hematite_maintainer_workflow");
        assert_eq!(
            args.get("workflow").and_then(|value| value.as_str()),
            Some("release")
        );
        assert_eq!(
            args.get("version").and_then(|value| value.as_str()),
            Some("0.4.5")
        );
        assert_eq!(
            args.get("push").and_then(|value| value.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn explicit_cleanup_prompt_rewrites_shell_to_maintainer_workflow() {
        let (tool_name, args) = normalized_tool_call_for_execution(
            "shell",
            &serde_json::json!({"command":"powershell -Command \"Get-ChildItem .\""}),
            false,
            Some("Run the deep cleanup and prune old dist artifacts."),
        );

        assert_eq!(tool_name, "run_hematite_maintainer_workflow");
        assert_eq!(
            args.get("workflow").and_then(|value| value.as_str()),
            Some("clean")
        );
        assert_eq!(
            args.get("deep").and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            args.get("prune_dist").and_then(|value| value.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn shell_cargo_test_rewrites_to_workspace_workflow() {
        let (tool_name, args) = normalized_tool_call_for_execution(
            "shell",
            &serde_json::json!({"command":"cargo test"}),
            false,
            Some("Run cargo test in this project."),
        );

        assert_eq!(tool_name, "run_workspace_workflow");
        assert_eq!(
            args.get("workflow").and_then(|value| value.as_str()),
            Some("command")
        );
        assert_eq!(
            args.get("command").and_then(|value| value.as_str()),
            Some("cargo test")
        );
    }

    #[test]
    fn current_plan_execution_request_accepts_saved_plan_command() {
        assert!(is_current_plan_execution_request("/implement-plan"));
        assert!(is_current_plan_execution_request(
            "Implement the current plan."
        ));
    }

    #[test]
    fn architect_operator_note_points_to_execute_path() {
        let plan = crate::tools::plan::PlanHandoff {
            goal: "Tighten startup workflow guidance".into(),
            target_files: vec!["src/runtime.rs".into()],
            ordered_steps: vec!["Update the startup banner".into()],
            verification: "cargo check --tests".into(),
            risks: vec![],
            open_questions: vec![],
        };
        let note = architect_handoff_operator_note(&plan);
        assert!(note.contains("`.hematite/PLAN.md`"));
        assert!(note.contains("/implement-plan"));
        assert!(note.contains("/code implement the current plan"));
    }

    #[test]
    fn sovereign_scaffold_handoff_carries_explicit_research_step() {
        let mut targets = std::collections::BTreeSet::new();
        targets.insert("index.html".to_string());
        let plan = build_sovereign_scaffold_handoff(
            "google uefn toolbelt then make a folder on my desktop called oupa with a single file html website talking about it",
            &targets,
        );

        assert!(plan
            .ordered_steps
            .iter()
            .any(|step| step.contains("research_web")));
        assert!(plan
            .ordered_steps
            .iter()
            .any(|step| step.contains("uefn toolbelt")));
    }

    #[test]
    fn plan_handoff_mentions_tool_detects_research_steps() {
        let plan = crate::tools::plan::PlanHandoff {
            goal: "Build the site".into(),
            target_files: vec!["index.html".into()],
            ordered_steps: vec!["Use `research_web` first to gather context.".into()],
            verification: "verify_build(action: \"build\")".into(),
            risks: vec![],
            open_questions: vec![],
        };

        assert!(plan_handoff_mentions_tool(&plan, "research_web"));
        assert!(!plan_handoff_mentions_tool(&plan, "fetch_docs"));
    }

    #[test]
    fn parse_task_checklist_progress_counts_checked_items() {
        let progress = parse_task_checklist_progress(
            r#"
- [x] Build the landing page shell
- [ ] Wire the responsive nav
* [X] Add hero section copy
Plain paragraph
"#,
        );

        assert_eq!(progress.total, 3);
        assert_eq!(progress.completed, 2);
        assert_eq!(progress.remaining, 1);
        assert!(progress.has_open_items());
    }

    #[test]
    fn merge_plan_allowed_paths_includes_hematite_sidecars() {
        let allowed = merge_plan_allowed_paths(&["src/main.rs".to_string()]);

        assert!(allowed.contains(&normalize_workspace_path("src/main.rs")));
        assert!(allowed
            .iter()
            .any(|path| path.ends_with("/.hematite/task.md")));
        assert!(allowed
            .iter()
            .any(|path| path.ends_with("/.hematite/plan.md")));
    }

    #[test]
    fn continue_plan_execution_requires_progress_and_open_items() {
        let mut mutated = std::collections::BTreeSet::new();
        mutated.insert("index.html".to_string());

        assert!(should_continue_plan_execution(
            1,
            Some(TaskChecklistProgress {
                total: 3,
                completed: 1,
                remaining: 2,
            }),
            Some(TaskChecklistProgress {
                total: 3,
                completed: 2,
                remaining: 1,
            }),
            &mutated,
        ));

        assert!(!should_continue_plan_execution(
            1,
            Some(TaskChecklistProgress {
                total: 3,
                completed: 2,
                remaining: 1,
            }),
            Some(TaskChecklistProgress {
                total: 3,
                completed: 2,
                remaining: 1,
            }),
            &std::collections::BTreeSet::new(),
        ));

        assert!(!should_continue_plan_execution(
            6,
            Some(TaskChecklistProgress {
                total: 3,
                completed: 2,
                remaining: 1,
            }),
            Some(TaskChecklistProgress {
                total: 3,
                completed: 3,
                remaining: 0,
            }),
            &mutated,
        ));
    }

    #[test]
    fn website_validation_runs_for_website_contract_frontend_paths() {
        let contract = crate::agent::workspace_profile::RuntimeContract {
            loop_family: "website".to_string(),
            app_kind: "website".to_string(),
            framework_hint: Some("vite".to_string()),
            preferred_workflows: vec!["website_validate".to_string()],
            delivery_phases: vec!["design".to_string(), "validate".to_string()],
            verification_workflows: vec!["build".to_string(), "website_validate".to_string()],
            quality_gates: vec!["critical routes return HTTP 200".to_string()],
            local_url_hint: Some("http://127.0.0.1:5173/".to_string()),
            route_hints: vec!["/".to_string()],
        };
        let mutated = std::collections::BTreeSet::from([
            "src/pages/index.tsx".to_string(),
            "public/app.css".to_string(),
        ]);
        assert!(should_run_website_validation(Some(&contract), &mutated));
    }

    #[test]
    fn website_validation_skips_non_website_contracts() {
        let contract = crate::agent::workspace_profile::RuntimeContract {
            loop_family: "service".to_string(),
            app_kind: "node-service".to_string(),
            framework_hint: Some("express".to_string()),
            preferred_workflows: vec!["build".to_string()],
            delivery_phases: vec!["define boundary".to_string()],
            verification_workflows: vec!["build".to_string()],
            quality_gates: vec!["build stays green".to_string()],
            local_url_hint: None,
            route_hints: Vec::new(),
        };
        let mutated = std::collections::BTreeSet::from(["server.ts".to_string()]);
        assert!(!should_run_website_validation(Some(&contract), &mutated));
        assert!(!should_run_website_validation(None, &mutated));
    }

    #[test]
    fn repeat_guard_exempts_structured_website_validation() {
        assert!(is_repeat_guard_exempt_tool_call(
            "run_workspace_workflow",
            &serde_json::json!({ "workflow": "website_validate" }),
        ));
        assert!(!is_repeat_guard_exempt_tool_call(
            "run_workspace_workflow",
            &serde_json::json!({ "workflow": "build" }),
        ));
    }

    #[test]
    fn natural_language_test_prompt_rewrites_to_workspace_workflow() {
        let (tool_name, args) = normalized_tool_call_for_execution(
            "shell",
            &serde_json::json!({"command":"powershell -Command \"Get-ChildItem .\""}),
            false,
            Some("Run the tests in this project."),
        );

        assert_eq!(tool_name, "run_workspace_workflow");
        assert_eq!(
            args.get("workflow").and_then(|value| value.as_str()),
            Some("test")
        );
    }

    #[test]
    fn scaffold_prompt_does_not_rewrite_to_workspace_workflow() {
        let (tool_name, _args) = normalized_tool_call_for_execution(
            "shell",
            &serde_json::json!({"command":"powershell -Command \"Get-ChildItem .\""}),
            false,
            Some("Make me a folder on my desktop named webtest2, and in that folder build a single-page website that explains the best uses of Hematite."),
        );

        assert_eq!(tool_name, "shell");
    }

    #[test]
    fn failing_path_parser_extracts_cargo_error_locations() {
        let output = r#"
BUILD FAILURE: The build is currently broken. FIX THESE ERRORS IMMEDIATELY:

error[E0412]: cannot find type `Foo` in this scope
  --> src/agent/conversation.rs:42:12
   |
42 |     field: Foo,
   |            ^^^ not found

error[E0308]: mismatched types
  --> src/tools/file_ops.rs:100:5
   |
   = note: expected `String`, found `&str`
"#;
        let paths = parse_failing_paths_from_build_output(output);
        assert!(
            paths.iter().any(|p| p.contains("conversation.rs")),
            "should capture conversation.rs"
        );
        assert!(
            paths.iter().any(|p| p.contains("file_ops.rs")),
            "should capture file_ops.rs"
        );
        assert_eq!(paths.len(), 2, "no duplicates");
    }

    #[test]
    fn failing_path_parser_ignores_macro_expansions() {
        let output = r#"
  --> <macro-expansion>:1:2
  --> src/real/file.rs:10:5
"#;
        let paths = parse_failing_paths_from_build_output(output);
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains("file.rs"));
    }

    #[test]
    fn intent_router_picks_updates_for_update_questions() {
        assert_eq!(
            preferred_host_inspection_topic("is my PC up to date?"),
            Some("updates")
        );
        assert_eq!(
            preferred_host_inspection_topic("are there any pending Windows updates?"),
            Some("updates")
        );
        assert_eq!(
            preferred_host_inspection_topic("check for updates on my computer"),
            Some("updates")
        );
    }

    #[test]
    fn intent_router_picks_security_for_antivirus_questions() {
        assert_eq!(
            preferred_host_inspection_topic("is my antivirus on?"),
            Some("security")
        );
        assert_eq!(
            preferred_host_inspection_topic("is Windows Defender running?"),
            Some("security")
        );
        assert_eq!(
            preferred_host_inspection_topic("is my PC protected?"),
            Some("security")
        );
    }

    #[test]
    fn intent_router_picks_pending_reboot_for_restart_questions() {
        assert_eq!(
            preferred_host_inspection_topic("do I need to restart my PC?"),
            Some("pending_reboot")
        );
        assert_eq!(
            preferred_host_inspection_topic("is a reboot required?"),
            Some("pending_reboot")
        );
        assert_eq!(
            preferred_host_inspection_topic("is there a pending restart waiting?"),
            Some("pending_reboot")
        );
    }

    #[test]
    fn intent_router_picks_disk_health_for_drive_health_questions() {
        assert_eq!(
            preferred_host_inspection_topic("is my hard drive dying?"),
            Some("disk_health")
        );
        assert_eq!(
            preferred_host_inspection_topic("check the disk health and SMART status"),
            Some("disk_health")
        );
        assert_eq!(
            preferred_host_inspection_topic("is my SSD healthy?"),
            Some("disk_health")
        );
    }

    #[test]
    fn intent_router_picks_battery_for_battery_questions() {
        assert_eq!(
            preferred_host_inspection_topic("check my battery"),
            Some("battery")
        );
        assert_eq!(
            preferred_host_inspection_topic("how is my battery life?"),
            Some("battery")
        );
        assert_eq!(
            preferred_host_inspection_topic("what is my battery wear level?"),
            Some("battery")
        );
    }

    #[test]
    fn intent_router_picks_recent_crashes_for_bsod_questions() {
        assert_eq!(
            preferred_host_inspection_topic("why did my PC restart by itself?"),
            Some("recent_crashes")
        );
        assert_eq!(
            preferred_host_inspection_topic("did my computer BSOD recently?"),
            Some("recent_crashes")
        );
        assert_eq!(
            preferred_host_inspection_topic("show me any recent app crashes"),
            Some("recent_crashes")
        );
    }

    #[test]
    fn intent_router_picks_scheduled_tasks_for_task_questions() {
        assert_eq!(
            preferred_host_inspection_topic("what scheduled tasks are running on this PC?"),
            Some("scheduled_tasks")
        );
        assert_eq!(
            preferred_host_inspection_topic("show me the task scheduler"),
            Some("scheduled_tasks")
        );
    }

    #[test]
    fn intent_router_picks_dev_conflicts_for_conflict_questions() {
        assert_eq!(
            preferred_host_inspection_topic("are there any dev environment conflicts?"),
            Some("dev_conflicts")
        );
        assert_eq!(
            preferred_host_inspection_topic("why is python pointing to the wrong version?"),
            Some("dev_conflicts")
        );
    }

    #[test]
    fn shell_guard_catches_windows_update_commands() {
        assert!(shell_looks_like_structured_host_inspection(
            "Get-WindowsUpdateLog | Select-Object -Last 50"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "$sess = New-Object -ComObject Microsoft.Update.Session"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-Service wuauserv"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-MpComputerStatus"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-PhysicalDisk"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-CimInstance Win32_Battery"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-WinEvent -FilterHashtable @{Id=41}"
        ));
        assert!(shell_looks_like_structured_host_inspection(
            "Get-ScheduledTask | Where-Object State -ne Disabled"
        ));
    }

    #[test]
    fn intent_router_picks_permissions_for_acl_questions() {
        assert_eq!(
            preferred_host_inspection_topic("who has permission to access the downloads folder?"),
            Some("permissions")
        );
        assert_eq!(
            preferred_host_inspection_topic("audit the ntfs permissions for this path"),
            Some("permissions")
        );
    }

    #[test]
    fn intent_router_picks_login_history_for_logon_questions() {
        assert_eq!(
            preferred_host_inspection_topic("who logged in recently on this machine?"),
            Some("login_history")
        );
        assert_eq!(
            preferred_host_inspection_topic("show me the logon history for the last 48 hours"),
            Some("login_history")
        );
    }

    #[test]
    fn intent_router_picks_share_access_for_unc_questions() {
        assert_eq!(
            preferred_host_inspection_topic("can i reach \\\\server\\share right now?"),
            Some("share_access")
        );
        assert_eq!(
            preferred_host_inspection_topic("test accessibility of a network share"),
            Some("share_access")
        );
    }

    #[test]
    fn intent_router_picks_registry_audit_for_persistence_questions() {
        assert_eq!(
            preferred_host_inspection_topic(
                "audit my registry for persistence hacks or debugger hijacking"
            ),
            Some("registry_audit")
        );
        assert_eq!(
            preferred_host_inspection_topic("check winlogon shell integrity and ifeo hijacks"),
            Some("registry_audit")
        );
    }

    #[test]
    fn intent_router_picks_network_stats_for_mbps_questions() {
        assert_eq!(
            preferred_host_inspection_topic("what is my network throughput in mbps right now?"),
            Some("network_stats")
        );
    }

    #[test]
    fn intent_router_picks_processes_for_cpu_percentage_questions() {
        assert_eq!(
            preferred_host_inspection_topic("which processes are using the most cpu % right now?"),
            Some("processes")
        );
    }

    #[test]
    fn intent_router_picks_log_check_for_recent_window_questions() {
        assert_eq!(
            preferred_host_inspection_topic("show me system errors from the last 2 hours"),
            Some("log_check")
        );
    }

    #[test]
    fn intent_router_picks_battery_for_health_and_cycles() {
        assert_eq!(
            preferred_host_inspection_topic("check my battery health and cycle count"),
            Some("battery")
        );
    }

    #[test]
    fn intent_router_picks_thermal_for_throttling_questions() {
        assert_eq!(
            preferred_host_inspection_topic(
                "why is my laptop slow? check for overheating or throttling"
            ),
            Some("thermal")
        );
        assert_eq!(
            preferred_host_inspection_topic("show me the current cpu temp"),
            Some("thermal")
        );
    }

    #[test]
    fn intent_router_picks_activation_for_genuine_questions() {
        assert_eq!(
            preferred_host_inspection_topic("is my windows genuine? check activation status"),
            Some("activation")
        );
        assert_eq!(
            preferred_host_inspection_topic("run slmgr to check my license state"),
            Some("activation")
        );
    }

    #[test]
    fn intent_router_picks_patch_history_for_hotfix_questions() {
        assert_eq!(
            preferred_host_inspection_topic("show me the recently installed hotfixes"),
            Some("patch_history")
        );
        assert_eq!(
            preferred_host_inspection_topic(
                "list the windows update patch history for the last 48 hours"
            ),
            Some("patch_history")
        );
    }

    #[test]
    fn intent_router_detects_multiple_symptoms_for_prerun() {
        let topics = all_host_inspection_topics("Why is my laptop slow? Check if it is overheating, throttling, or under heavy I/O pressure.");
        assert!(topics.contains(&"thermal"));
        assert!(topics.contains(&"resource_load"));
        assert!(topics.contains(&"storage"));
        assert!(topics.len() >= 3);
    }

    #[test]
    fn parse_unload_target_supports_current_and_all() {
        assert_eq!(
            ConversationManager::parse_unload_target("current").unwrap(),
            (None, false)
        );
        assert_eq!(
            ConversationManager::parse_unload_target("all").unwrap(),
            (None, true)
        );
        assert_eq!(
            ConversationManager::parse_unload_target("qwen/qwen3.5-9b").unwrap(),
            (Some("qwen/qwen3.5-9b".to_string()), false)
        );
    }

    #[test]
    fn provider_model_controls_summary_mentions_ollama_limits() {
        let ollama = ConversationManager::provider_model_controls_summary("Ollama");
        assert!(ollama.contains("Ollama supports coding and embed model load/list/unload"));
        let lms = ConversationManager::provider_model_controls_summary("LM Studio");
        assert!(lms.contains("LM Studio supports coding and embed model load/unload"));
    }
}
