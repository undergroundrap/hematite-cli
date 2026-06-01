use std::fmt::Write as _;

use crate::agent::architecture_summary::{
    build_architecture_overview_answer, prune_architecture_trace_batch,
    prune_authoritative_tool_batch, prune_read_only_context_bloat_batch,
    prune_redirected_shell_batch, summarize_runtime_trace_output,
};
use crate::agent::direct_answers::{
    build_about_answer, build_architect_session_reset_plan, build_authorization_policy_answer,
    build_gemma_native_answer, build_gemma_native_settings_answer, build_help_answer,
    build_identity_answer, build_inspect_inventory, build_language_capability_answer,
    build_mcp_lifecycle_answer, build_product_surface_answer, build_reasoning_split_answer,
    build_recovery_recipes_answer, build_session_memory_answer,
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
    is_scaffold_request, looks_like_mutation_request, needs_ansi_tools, needs_archive_tools,
    needs_ascii_chart_tools, needs_ascii_tools, needs_asn1_tools, needs_base_tools,
    needs_bencode_tools, needs_bin_pack_tools, needs_binary_tools, needs_calc_tools,
    needs_cbor_tools, needs_changelog_gen, needs_changelog_tools, needs_char_tools,
    needs_checksum_tools, needs_cipher_tools, needs_code_metrics, needs_color_tools,
    needs_computation_sandbox, needs_crash_debug, needs_cron_tools, needs_csp_tools,
    needs_css_tools, needs_csv_tools, needs_data_gen_tools, needs_date_tools,
    needs_dependency_audit, needs_diff_tools, needs_dns_tools, needs_docker_compose_tools,
    needs_docker_ops, needs_dockerfile_tools, needs_dotenv_tools, needs_duration_tools,
    needs_elf_tools, needs_email_tools, needs_encode_tools, needs_env_diff, needs_env_schema_tools,
    needs_file_tree_tools, needs_find_tools, needs_format, needs_fraction_tools, needs_geo_tools,
    needs_geometry_tools, needs_github_actions_tools, needs_github_ops, needs_gitignore_tools,
    needs_glob_tools, needs_graph_tools, needs_graphql_tools, needs_graphviz_tools,
    needs_grep_tools, needs_har_tools, needs_hash_tools, needs_hex_tools, needs_html_tools,
    needs_http_parse_tools, needs_http_request, needs_http_status_tools, needs_ical_tools,
    needs_id_tools, needs_ini_tools, needs_interval_tools, needs_ip_tools, needs_jq_tools,
    needs_json_tools, needs_jsonl_tools, needs_jsonschema_tools, needs_jwt_tools, needs_k8s_tools,
    needs_keyval_tools, needs_leb128_tools, needs_license_tools, needs_line_tools,
    needs_lint_check, needs_lock_file_tools, needs_log_parse_tools, needs_make_tools,
    needs_markdown_tools, needs_matrix_tools, needs_mermaid_tools, needs_mime_tools,
    needs_money_tools, needs_msgpack_tools, needs_nato_tools, needs_net_lookup_tools,
    needs_network_header_tools, needs_nginx_conf_tools, needs_number_sequence_tools,
    needs_number_theory_tools, needs_number_tools, needs_number_words_tools, needs_openapi_tools,
    needs_package_json_tools, needs_password_gen, needs_path_tools, needs_pem_tools,
    needs_plist_tools, needs_port_check, needs_printf_tools, needs_proto_tools, needs_regex_tools,
    needs_robots_txt_tools, needs_rss_tools, needs_scientific_compute, needs_secret_scan,
    needs_semver_tools, needs_sitemap_tools, needs_size_tools, needs_sql_format_tools,
    needs_sql_migrate_tools, needs_sql_tools, needs_sqlite_tools, needs_ssh_config_tools,
    needs_stat_tools, needs_string_metric_tools, needs_systemd_tools, needs_table_tools,
    needs_tar_tools, needs_template_gen, needs_template_tools, needs_terraform_tools,
    needs_test_run, needs_text_extract_tools, needs_text_tools, needs_time_zone_tools,
    needs_tlv_tools, needs_todo_tools, needs_token_tools, needs_toml_tools, needs_totp_tools,
    needs_unicode_tools, needs_unit_tools, needs_url_tools, needs_uuid_gen, needs_validate_tools,
    needs_vcf_tools, needs_wasm_tools, needs_word_tools, needs_xml_tools, needs_yaml_tools,
    preferred_host_inspection_topic, preferred_maintainer_workflow, preferred_workspace_workflow,
    DirectAnswerKind, QueryIntentClass,
};
use crate::agent::tool_registry::dispatch_builtin_tool;
use crate::agent::truncation::safe_head;
use crate::agent::types::{
    ChatMessage, InferenceEvent, MessageContent, OperatorCheckpointState, ProviderRuntimeState,
    ToolCallFn, ToolDefinition, ToolFunction,
};
// SystemPromptBuilder is no longer used — InferenceEngine::build_system_prompt() is canonical.
use crate::agent::compaction::{self, CompactionConfig};
use crate::agent::report_export::{
    fix_issue_categories, generate_fix_plan_markdown, generate_triage_report_markdown,
};
use crate::tools::host_inspect::inspect_host;
use crate::ui::gpu_monitor::GpuState;

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
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

#[derive(serde::Serialize, serde::Deserialize, Default)]
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
    working_files.sort_unstable();
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
    if let Some(plan) = crate::tools::plan::load_plan_handoff() {
        saved.session_memory.current_plan = Some(plan);
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

fn backtick_join(paths: &[String]) -> String {
    let cap = paths.iter().map(|p| p.len() + 2).sum::<usize>() + paths.len().saturating_sub(1) * 2;
    let mut s = String::with_capacity(cap);
    for (i, p) in paths.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push('`');
        s.push_str(p);
        s.push('`');
    }
    s
}

fn build_force_plan_mutation_prompt(
    progress: TaskChecklistProgress,
    target_files: &[String],
) -> String {
    let targets = if target_files.is_empty() {
        "the saved target files".to_string()
    } else {
        backtick_join(target_files)
    };
    format!(
        "You completed an implementation pass without mutating any target files, but `.hematite/TASK.md` still has {} unchecked item(s). This is not done. Read `.hematite/TASK.md`, inspect {}, and make a concrete implementation edit now. Do not summarize. If you still cannot mutate safely after grounding yourself in those files, surface exactly one concrete blocker.",
        progress.remaining, targets
    )
}

fn build_current_plan_scope_recovery_prompt(target_files: &[String]) -> String {
    let targets = if target_files.is_empty() {
        "the saved target files".to_string()
    } else {
        backtick_join(target_files)
    };
    format!(
        "STOP. You just tried to read or inspect something outside the saved current-plan targets. Stay inside {} only. Read `.hematite/TASK.md` or inspect one saved target file, then make progress there. Do not branch into unrelated files or docs/exec-plans paths.",
        targets
    )
}

fn build_task_ledger_closeout_prompt(
    progress: TaskChecklistProgress,
    target_files: &[String],
) -> String {
    let targets = if target_files.is_empty() {
        "the saved target files".to_string()
    } else {
        backtick_join(target_files)
    };
    format!(
        "The deliverable files were already mutated, but `.hematite/TASK.md` still has {} unchecked item(s). This is not summary time yet. Read `.hematite/TASK.md`, verify the completed work in {}, then update the checklist to mark the finished items `[x]`. If needed, also write `.hematite/WALKTHROUGH.md`. Do not summarize until the task ledger reflects reality.",
        progress.remaining, targets
    )
}

fn should_suppress_recoverable_tool_result(
    blocked_by_policy: bool,
    recoverable_policy_intervention: bool,
) -> bool {
    blocked_by_policy && recoverable_policy_intervention
}

fn is_sovereign_scaffold_plan(plan: &crate::tools::plan::PlanHandoff) -> bool {
    plan.goal
        .to_ascii_lowercase()
        .contains("sovereign scaffold task")
}

fn target_files_materialized(target_files: &[String]) -> bool {
    if target_files.is_empty() {
        return false;
    }
    target_files.iter().all(|path| {
        let file = std::path::Path::new(path);
        std::fs::metadata(file)
            .map(|meta| meta.is_file() && meta.len() > 0)
            .unwrap_or(false)
    })
}

fn mark_all_task_ledger_items_complete() -> Result<TaskChecklistProgress, String> {
    let path = task_status_path();
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read task ledger for closeout: {e}"))?;
    let mut updated = String::with_capacity(content.len());
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("- [ ]") {
            let indent_len = line.len().saturating_sub(trimmed.len());
            let indent = &line[..indent_len];
            updated.push_str(indent);
            updated.push_str(&line[indent_len..].replacen("- [ ]", "- [x]", 1));
        } else if trimmed.starts_with("* [ ]") {
            let indent_len = line.len().saturating_sub(trimmed.len());
            let indent = &line[..indent_len];
            updated.push_str(indent);
            updated.push_str(&line[indent_len..].replacen("* [ ]", "* [x]", 1));
        } else if trimmed.starts_with("+ [ ]") {
            let indent_len = line.len().saturating_sub(trimmed.len());
            let indent = &line[..indent_len];
            updated.push_str(indent);
            updated.push_str(&line[indent_len..].replacen("+ [ ]", "+ [x]", 1));
        } else {
            updated.push_str(line);
        }
        updated.push('\n');
    }
    std::fs::write(&path, updated)
        .map_err(|e| format!("Failed to update task ledger during closeout: {e}"))?;
    read_task_checklist_progress().ok_or_else(|| "Task ledger closeout re-read failed.".to_string())
}

fn write_minimal_walkthrough(summary: &str) -> Result<(), String> {
    let path = crate::tools::file_ops::hematite_dir().join("WALKTHROUGH.md");
    std::fs::write(&path, summary)
        .map_err(|e| format!("Failed to write walkthrough during closeout: {e}"))
}

fn deterministic_sovereign_closeout_summary(
    plan: &crate::tools::plan::PlanHandoff,
    target_files: &[String],
) -> String {
    let targets = backtick_join(target_files);
    format!(
        "## Summary: Sovereign Scaffold Task Complete\n\n### What Was Built\nImplemented the sovereign scaffold deliverable in {}.\n\n### What Was Verified\n- Deliverable files exist and are non-empty\n- `.hematite/TASK.md` was updated to reflect completion\n- `.hematite/WALKTHROUGH.md` was written for session closeout\n\n### Plan Goal\n{}\n",
        targets,
        plan.goal.trim()
    )
}

fn maybe_deterministic_sovereign_closeout(
    plan: Option<&crate::tools::plan::PlanHandoff>,
    mutation_occurred: bool,
) -> Option<String> {
    let plan = plan?;
    if !mutation_occurred || !is_sovereign_scaffold_plan(plan) {
        return None;
    }
    if !target_files_materialized(&plan.target_files) {
        return None;
    }
    let progress = mark_all_task_ledger_items_complete().ok()?;
    if progress.remaining != 0 {
        return None;
    }
    let summary = deterministic_sovereign_closeout_summary(plan, &plan.target_files);
    let _ = write_minimal_walkthrough(&summary);
    Some(summary)
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
        let path_str = raw.trim_end_matches([',', '.', ':', ';', '!', '?']);
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
fn compact_stale_reads(history: &mut [ChatMessage], path: &str) {
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
    let mut lines = String::with_capacity(max_lines * 60);
    for (i, line) in content.lines().enumerate().take(max_lines) {
        if i > 0 {
            lines.push('\n');
        }
        let _ = write!(lines, "{:>4}  {}", i + 1, line);
    }
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
    let mut prefixes = Vec::with_capacity(2);
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

fn should_use_turn_scoped_investigation_mode(
    workflow_mode: WorkflowMode,
    primary_class: QueryIntentClass,
) -> bool {
    workflow_mode == WorkflowMode::Auto && primary_class == QueryIntentClass::Research
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
    paths.sort_unstable();
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
    let mentions_site_shape = lower.contains("website")
        || lower.contains("landing page")
        || lower.contains("web page")
        || lower.contains("html website")
        || lower.contains("html site")
        || lower.contains("single index.html")
        || lower.contains("index.html")
        || lower.contains("single file html")
        || lower.contains("single-file html")
        || lower.contains("single html file");
    mentions_site_shape
        && (lower.contains("html")
            || lower.contains("css")
            || lower.contains("javascript")
            || lower.contains("js")
            || lower.contains("index.html")
            || !lower.contains("react"))
}

fn prefers_single_file_html_site(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    lower.contains("single index.html")
        || lower.contains("index.html")
        || lower.contains("single file html")
        || lower.contains("single-file html")
        || lower.contains("single html file")
}

fn sanitize_project_folder_name(raw: &str) -> String {
    let trimmed = raw
        .trim()
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '`' | '.' | ',' | ':' | ';'));
    let mut out = String::with_capacity(trimmed.len());
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
        if !prefers_single_file_html_site(user_input) {
            targets.insert("style.css".to_string());
            targets.insert("script.js".to_string());
        }
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
        if prefers_single_file_html_site(user_input) {
            steps.insert(
                1,
                "Keep the deliverable to a single `index.html` file with inline structure/content that explains the research clearly and reads well on desktop and mobile."
                    .to_string(),
            );
            "Open and inspect `index.html` in this root, confirm the page is coherent, self-contained, and responsive without relying on extra front-end files or repo-root workflows.".to_string()
        } else {
            steps.insert(
                1,
                "Make sure index.html, style.css, and script.js stay linked correctly and that the layout remains responsive on desktop and mobile.".to_string(),
            );
            "Open and inspect the generated front-end files in this root, confirm cross-file links are valid, and verify the page is coherent and responsive without using repo-root workflows.".to_string()
        }
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
    let mut word_iter = lower.split_whitespace();
    let first = match word_iter.next() {
        Some(w) => w,
        None => return false,
    };

    // Single pass: accumulate total count and stop-word hits.
    let stop_words = [
        "the", "a", "an", "on", "my", "your", "for", "with", "into", "onto",
    ];
    let mut stop_count = usize::from(stop_words.contains(&first));
    let mut total = 1usize;
    for word in word_iter {
        total += 1;
        if stop_words.contains(&word) {
            stop_count += 1;
        }
    }

    // 1. Sentences starting with conversational phrases
    if [
        "make", "create", "i", "can", "please", "we", "let's", "go", "execute", "run", "how",
    ]
    .contains(&first)
        && total >= 3
    {
        return true;
    }

    // 2. Presence of English stop-words that are rare in CLI commands
    if stop_count >= 2 {
        return true;
    }

    // 3. Lack of common CLI separators if many words exist
    if total >= 5
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
    pub pinned_files: Arc<RwLock<std::collections::HashMap<String, String>>>,
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
    /// Skill body explicitly loaded via `/skill <name>` — injected once then cleared.
    pending_skill_inject: Option<String>,
    /// Recent shell command history — loaded once at session start, injected into system prompt.
    shell_history_block: Option<String>,
    /// Error context loaded by `/fix` — injected as a focused intervention on the next turn.
    pending_fix_context: Option<String>,
    /// Last turn's context budget ledger — re-surfaced by /budget.
    last_turn_budget: Option<crate::agent::economics::TurnBudget>,
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
            let _ = writeln!(out, "[{}]", room);
            for file in files {
                let _ = writeln!(
                    out,
                    "- {} [{} edit{}]",
                    file.path,
                    file.heat,
                    if file.heat == 1 { "" } else { "s" }
                );
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
        let percent = (estimated_tokens.saturating_mul(100))
            .checked_div(config.max_estimated_tokens)
            .unwrap_or(0)
            .min(100) as u8;

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
                let _ = write!(
                    summary,
                    " | reachable alternative: {} ({})",
                    alt_name, alt_url
                );
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
        let mut lines = String::with_capacity(models.len() * 40);
        for (idx, model) in models.iter().enumerate() {
            if idx > 0 {
                lines.push('\n');
            }
            let _ = write!(lines, "{}. {}", idx + 1, model);
        }
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

    #[allow(clippy::too_many_arguments)]
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
            pinned_files: Arc::new(RwLock::new(std::collections::HashMap::new())),
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
            pending_skill_inject: None,
            shell_history_block: crate::agent::shell_history::load_shell_history_block(),
            pending_fix_context: None,
            last_turn_budget: None,
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
            let _ = crate::tools::file_ops::safe_write(&path, json.as_bytes());
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
            let _ = crate::tools::file_ops::safe_write(&path, json.as_bytes());
        }
    }

    fn refresh_session_memory(&mut self) {
        let current_plan = self.session_memory.current_plan.take();
        let last_checkpoint = self.session_memory.last_checkpoint.take();
        let last_blocker = self.session_memory.last_blocker.take();
        let last_recovery = self.session_memory.last_recovery.take();
        let last_verification = self.session_memory.last_verification.take();
        let last_compaction = self.session_memory.last_compaction.take();
        self.session_memory = compaction::extract_memory(&self.history);
        self.session_memory.current_plan = current_plan;
        self.session_memory.last_checkpoint = last_checkpoint;
        self.session_memory.last_blocker = last_blocker;
        self.session_memory.last_recovery = last_recovery;
        self.session_memory.last_verification = last_verification;
        self.session_memory.last_compaction = last_compaction;
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
        let plan = crate::tools::plan::parse_plan_handoff(response)?;
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
                .trim_end_matches([',', '.', ':', ';', '!', '?']);
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
                        let allowed = backtick_join(&allowed_paths);
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
                let pinned = self.pinned_files.read().await;
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
                    let files = backtick_join(&state.last_failed_build_paths);
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
                    if let Some(obj) = redirect_args.as_object_mut() {
                        if let Some(identity) = extract_dns_lookup_target_from_shell(command) {
                            obj.insert("name".to_string(), serde_json::Value::String(identity));
                        }
                        if let Some(record_type) = extract_dns_record_type_from_shell(command) {
                            obj.insert(
                                "type".to_string(),
                                serde_json::Value::String(record_type.to_string()),
                            );
                        }
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
                        for (i, part) in command.split_whitespace().enumerate() {
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
                        if let Some(obj) = redirect_args.as_object_mut() {
                            obj.insert(
                                "name_filter".to_string(),
                                serde_json::Value::String(identity),
                            );
                        }
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
        let _ = writeln!(receipt, "- tool: {}", name);
        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
            let _ = writeln!(receipt, "- target: {}", path);
        }
        if name == "shell" {
            if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                let _ = writeln!(receipt, "- command: {}", command);
            }
            if let Some(reason) = args.get("reason").and_then(|v| v.as_str()) {
                if !reason.trim().is_empty() {
                    let _ = writeln!(receipt, "- reason: {}", reason.trim());
                }
            }
        }
        let first_line = output.lines().next().unwrap_or(output).trim();
        let _ = writeln!(receipt, "- outcome: {}", first_line);
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

        // ── Deterministic IT Lane: 0-model remediation ────────────────────────
        if user_input.starts_with("/triage") || user_input == "/health" {
            let preset = if user_input.starts_with("/triage") {
                user_input.strip_prefix("/triage").unwrap_or("").trim()
            } else {
                ""
            };
            let preset = if preset.is_empty() { "default" } else { preset };
            let _ = tx
                .send(InferenceEvent::Thought(
                    "Running deterministic IT triage...".into(),
                ))
                .await;
            let report = generate_triage_report_markdown(preset).await;
            for chunk in chunk_text(&report, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.starts_with("/fix") {
            let issue = user_input.strip_prefix("/fix").unwrap_or("").trim();
            if issue.is_empty() || issue == "list" || issue == "help" {
                let mut list = "Supported issue categories:\n\n".to_string();
                for (cat, keywords) in fix_issue_categories() {
                    let _ = writeln!(list, "  {:<22} {}", cat, keywords);
                }
                for chunk in chunk_text(&list, 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }
            let _ = tx
                .send(InferenceEvent::Thought(format!(
                    "Generating fix plan for '{}'...",
                    issue
                )))
                .await;
            let plan = generate_fix_plan_markdown(issue).await;
            for chunk in chunk_text(&plan, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.starts_with("/inspect") {
            let topic = user_input.strip_prefix("/inspect").unwrap_or("").trim();
            if topic.is_empty() {
                for chunk in chunk_text(&build_inspect_inventory(), 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }
            let _ = tx
                .send(InferenceEvent::Thought(format!(
                    "Inspecting host topic: {}...",
                    topic
                )))
                .await;
            let args = serde_json::json!({"topic": topic});
            let output = inspect_host(&args)
                .await
                .unwrap_or_else(|e| format!("Error: {}", e));
            for chunk in chunk_text(&output, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.starts_with("/query") {
            let q = user_input.strip_prefix("/query").unwrap_or("").trim();
            if q.is_empty() {
                for chunk in chunk_text(
                    "Usage: /query <natural language question>\n\
                     Example: /query why is my PC slow\n\
                     Routes your question to the right inspect_host topics and runs them without a model.",
                    8,
                ) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }
            let detected = all_host_inspection_topics(q);
            let topics: Vec<&str> = if !detected.is_empty() {
                detected
            } else {
                match preferred_host_inspection_topic(q) {
                    Some(t) => vec![t],
                    None => vec!["summary"],
                }
            };
            let _ = tx
                .send(InferenceEvent::Thought(format!(
                    "Query routed to {} topic(s): {}",
                    topics.len(),
                    topics.join(", ")
                )))
                .await;
            let total = topics.len();
            let mut out = String::new();
            for (i, topic) in topics.iter().enumerate() {
                let _ = tx
                    .send(InferenceEvent::Thought(format!(
                        "  [{}/{}] inspecting {}...",
                        i + 1,
                        total,
                        topic
                    )))
                    .await;
                let args = serde_json::json!({"topic": topic});
                let result = inspect_host(&args)
                    .await
                    .unwrap_or_else(|e| format!("Error: {}", e));
                if total > 1 {
                    out.push_str(&format!("─── {} ───\n", topic));
                }
                out.push_str(result.trim_end());
                out.push('\n');
                if total > 1 {
                    out.push('\n');
                }
            }
            for chunk in chunk_text(&out, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        // ── Fast-path reset commands: handled locally, no network I/O needed ──
        if user_input.trim() == "/new" {
            self.history.clear();
            self.reasoning_history = None;
            self.session_memory.clear();
            self.running_summary = None;
            self.correction_hints.clear();
            self.pinned_files.write().await.clear();
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
            self.pinned_files.write().await.clear();
            self.reset_action_grounding().await;
            reset_task_files();
            crate::agent::tasks::clear();
            purge_persistent_memory();
            tokio::task::block_in_place(|| self.vein.reset());
            let _ = std::fs::remove_file(session_path());
            self.save_empty_session();
            self.emit_compaction_pressure(&tx).await;
            self.emit_prompt_pressure_idle(&tx).await;
            for chunk in chunk_text(
                "Hard forget complete. Chat history, saved memory, task files, task list, and the Vein index were purged.",
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
            let workspace_root = crate::tools::file_ops::workspace_root();
            let report = {
                let mut combined = String::with_capacity(
                    crate::agent::instructions::PROJECT_GUIDANCE_FILES.len() * 512,
                );
                for name in crate::agent::instructions::PROJECT_GUIDANCE_FILES {
                    let path =
                        crate::agent::instructions::resolve_guidance_path(&workspace_root, name);
                    if !path.exists() {
                        continue;
                    }
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            let _ = write!(combined, "## {}\n\n{}\n\n", name, content.trim());
                        }
                        Err(e) => {
                            let _ = write!(
                                combined,
                                "## {}\n\nError reading {}: {}\n\n",
                                name,
                                path.display(),
                                e
                            );
                        }
                    }
                }
                if combined.is_empty() {
                    "No project guidance files found.\n\nRecognized files: `CLAUDE.md`, `SKILLS.md`, `SKILL.md`, `HEMATITE.md`, `.hematite/rules.md`, `.hematite/rules.local.md`, and `.hematite/instructions.md`.\n\nCreate one of those files to inject workspace-specific guidance on the next turn.".to_string()
                } else {
                    format!(
                        "## Project Guidance\n\n{}---\nTo update shared rules, open `.hematite/rules.md`. To add workspace-specific recipes or conventions, use `SKILLS.md` or `SKILL.md` in the workspace root. Changes take effect on the next turn.",
                        combined
                    )
                }
            };
            for chunk in chunk_text(&report, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/skills" {
            let workspace_root = crate::tools::file_ops::workspace_root();
            let config = crate::agent::config::load_config();
            let discovery =
                crate::agent::instructions::discover_agent_skills(&workspace_root, &config.trust);
            let report = crate::agent::instructions::render_skills_report(&discovery);
            for chunk in chunk_text(&report, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        // /skill <name> — explicitly load a skill's full body for the next turn.
        if let Some(skill_name) = user_input
            .trim()
            .strip_prefix("/skill ")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let workspace_root = crate::tools::file_ops::workspace_root();
            let config = crate::agent::config::load_config();
            let discovery =
                crate::agent::instructions::discover_agent_skills(&workspace_root, &config.trust);
            let name_lower = skill_name.to_lowercase();
            if let Some(skill) = discovery
                .skills
                .iter()
                .find(|s| s.name.to_lowercase() == name_lower)
            {
                if skill.body.is_empty() {
                    let msg = format!(
                        "Skill `{}` found but its SKILL.md has no body — add instructions after the frontmatter.",
                        skill.name
                    );
                    for chunk in chunk_text(&msg, 8) {
                        let _ = tx.send(InferenceEvent::Token(chunk)).await;
                    }
                } else {
                    self.pending_skill_inject =
                        Some(format!("## Skill: {}\n{}", skill.name, skill.body));
                    let msg = format!(
                        "Skill `{}` loaded — instructions will be active for the next turn.",
                        skill.name
                    );
                    for chunk in chunk_text(&msg, 8) {
                        let _ = tx.send(InferenceEvent::Token(chunk)).await;
                    }
                }
            } else {
                let available: Vec<&str> =
                    discovery.skills.iter().map(|s| s.name.as_str()).collect();
                let msg = if available.is_empty() {
                    format!(
                        "No skill named `{}` found. No skills are currently discovered.",
                        skill_name
                    )
                } else {
                    format!(
                        "No skill named `{}` found. Available: {}",
                        skill_name,
                        available.join(", ")
                    )
                };
                for chunk in chunk_text(&msg, 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        // /skill new <name> — scaffold a SKILL.md skeleton in .agents/skills/<name>/
        if let Some(new_name) = user_input
            .trim()
            .strip_prefix("/skill new ")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let slug = new_name
                .to_lowercase()
                .replace(' ', "-")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect::<String>();
            let skill_dir = crate::tools::file_ops::workspace_root()
                .join(".agents")
                .join("skills")
                .join(&slug);
            let skill_path = skill_dir.join("SKILL.md");
            let msg = if skill_path.exists() {
                format!(
                    "Skill `{}` already exists at `{}`.",
                    slug,
                    skill_path.display()
                )
            } else {
                match std::fs::create_dir_all(&skill_dir) {
                    Err(e) => format!("Failed to create skill directory: {}", e),
                    Ok(()) => {
                        let template = format!(
                            "---\nname: {slug}\ndescription: Describe when this skill should activate.\ntriggers: \"\"\n---\n\n## When to use\n\nDescribe the problem or context this skill addresses.\n\n## Instructions\n\n1. Step one.\n2. Step two.\n3. Step three.\n\n## Notes\n\n- Any caveats or edge cases.\n"
                        );
                        match std::fs::write(&skill_path, template) {
                            Ok(()) => format!(
                                "Created `{}` — edit the description, triggers, and instructions, then use `/skill {}` to load it.",
                                skill_path.display(),
                                slug
                            ),
                            Err(e) => format!("Failed to write SKILL.md: {}", e),
                        }
                    }
                }
            };
            for chunk in chunk_text(&msg, 8) {
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

        if user_input.trim() == "/compact" {
            let context_length = self.engine.current_context_length();
            let vram_ratio = self.gpu_state.ratio();
            let config = compaction::CompactionConfig::adaptive(context_length, vram_ratio);
            let before_len = self.history.len();
            let estimated_tokens = compaction::estimate_compactable_tokens(&self.history);
            let result = compaction::compact_history(
                &self.history,
                self.running_summary.as_deref(),
                config,
                None,
            );
            let removed = before_len.saturating_sub(result.messages.len());
            self.history = result.messages;
            self.running_summary = result.summary;
            let last_checkpoint = self.session_memory.last_checkpoint.take();
            let last_blocker = self.session_memory.last_blocker.take();
            let last_recovery = self.session_memory.last_recovery.take();
            let last_verification = self.session_memory.last_verification.take();
            let last_compaction = self.session_memory.last_compaction.take();
            self.session_memory = compaction::extract_memory(&self.history);
            self.session_memory.last_checkpoint = last_checkpoint;
            self.session_memory.last_blocker = last_blocker;
            self.session_memory.last_recovery = last_recovery;
            self.session_memory.last_verification = last_verification;
            self.session_memory.last_compaction = last_compaction;
            self.session_memory.record_compaction(
                removed,
                format!(
                    "Manual /compact: task '{}', {} file(s) in working set.",
                    self.session_memory.current_task,
                    self.session_memory.working_set.len()
                ),
            );
            self.emit_compaction_pressure(&tx).await;
            let after_tokens = compaction::estimate_compactable_tokens(&self.history);
            let msg = format!(
                "History compacted. {} message(s) summarized, ~{} tokens freed. \
                 Remaining: ~{} tokens. Active task: \"{}\".",
                removed,
                estimated_tokens.saturating_sub(after_tokens),
                after_tokens,
                self.session_memory.current_task,
            );
            for chunk in chunk_text(&msg, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        if user_input.trim() == "/budget" {
            let msg = match &self.last_turn_budget {
                Some(b) => b.render(),
                None => "No turn budget recorded yet — run a prompt first.".to_string(),
            };
            for chunk in chunk_text(&msg, 8) {
                let _ = tx.send(InferenceEvent::Token(chunk)).await;
            }
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        // ── /task commands ───────────────────────────────────────────────────────
        {
            let trimmed = user_input.trim();

            // /task or /task list — show current tasks
            if trimmed == "/task" || trimmed == "/task list" {
                let tasks = crate::agent::tasks::load();
                let report = crate::agent::tasks::render_list(&tasks);
                for chunk in chunk_text(&report, 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }

            // /task add <text>
            if let Some(text) = trimmed
                .strip_prefix("/task add ")
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let tasks = crate::agent::tasks::add(text);
                let added = tasks
                    .iter()
                    .find(|t| t.text == text.trim())
                    .map(|t| t.id)
                    .unwrap_or(0);
                let msg = format!("Task {} added: {}", added, text.trim());
                for chunk in chunk_text(&msg, 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }

            // /task done <N>
            if let Some(n_str) = trimmed.strip_prefix("/task done ").map(str::trim) {
                let msg = match n_str.parse::<usize>() {
                    Ok(n) => match crate::agent::tasks::mark_done(n) {
                        Ok(tasks) => {
                            let task = tasks.iter().find(|t| t.id == n);
                            format!(
                                "Task {} marked done: {}",
                                n,
                                task.map(|t| t.text.as_str()).unwrap_or("")
                            )
                        }
                        Err(e) => e,
                    },
                    Err(_) => "Usage: /task done <number>  (e.g. `/task done 2`)".to_string(),
                };
                for chunk in chunk_text(&msg, 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }

            // /task remove <N>
            if let Some(n_str) = trimmed.strip_prefix("/task remove ").map(str::trim) {
                let msg = match n_str.parse::<usize>() {
                    Ok(n) => match crate::agent::tasks::remove(n) {
                        Ok(_) => format!("Task {} removed.", n),
                        Err(e) => e,
                    },
                    Err(_) => "Usage: /task remove <number>  (e.g. `/task remove 3`)".to_string(),
                };
                for chunk in chunk_text(&msg, 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }

            // /task clear
            if trimmed == "/task clear" {
                crate::agent::tasks::clear();
                for chunk in chunk_text("All tasks cleared.", 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }
        }

        // ── GitHub slash commands (harness-driven, no model) ─────────────────
        {
            let trimmed = user_input.trim();

            // /pr [--draft] [title]
            if trimmed == "/pr" || trimmed.starts_with("/pr ") {
                let rest = trimmed.strip_prefix("/pr").unwrap_or("").trim();
                let draft = rest.contains("--draft");
                let title_part = rest.trim_start_matches("--draft").trim();
                let title = if title_part.is_empty() {
                    None
                } else {
                    Some(title_part)
                };
                let msg = match crate::tools::github::create_pr_from_context(title, draft) {
                    Ok(out) => out,
                    Err(e) => format!("PR creation failed: {}", e),
                };
                for chunk in chunk_text(&msg, 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }

            // /ci
            if trimmed == "/ci" {
                let msg = match crate::tools::github::ci_status_current() {
                    Ok(out) if out.trim().is_empty() => {
                        "No CI runs found for this branch. Push to GitHub and trigger a workflow first.".to_string()
                    }
                    Ok(out) => format!("## CI Status\n\n```\n{}\n```", out.trim()),
                    Err(e) => format!("CI status failed: {}", e),
                };
                for chunk in chunk_text(&msg, 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }

            // /issue
            if trimmed == "/issue" || trimmed.starts_with("/issue ") {
                let rest = trimmed.strip_prefix("/issue").unwrap_or("").trim();
                let args = if rest.is_empty() {
                    serde_json::json!({ "action": "issue_list", "limit": 10 })
                } else if let Ok(n) = rest.parse::<u64>() {
                    serde_json::json!({ "action": "issue_view", "number": n })
                } else {
                    serde_json::json!({ "action": "issue_list", "limit": 10, "state": rest })
                };
                let msg = match crate::tools::github::execute(&args).await {
                    Ok(out) if out.trim().is_empty() => "No issues found.".to_string(),
                    Ok(out) => format!("## Issues\n\n```\n{}\n```", out.trim()),
                    Err(e) => format!("Issue lookup failed: {}", e),
                };
                for chunk in chunk_text(&msg, 8) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                let _ = tx.send(InferenceEvent::Done).await;
                return Ok(());
            }
        }

        // ── /fix — run verify_build now, load error into next-turn intervention ──
        if user_input.trim() == "/fix" || user_input.trim() == "/fix --test" {
            let action = if user_input.trim() == "/fix --test" {
                "test"
            } else {
                "build"
            };
            let _ = tx
                .send(InferenceEvent::Thought(format!(
                    "Running verify_build({action}) to capture current error state..."
                )))
                .await;
            let result =
                crate::tools::verify_build::execute(&serde_json::json!({ "action": action })).await;
            let (ok, output) = match result {
                Ok(out) => (true, out),
                Err(e) => (false, e),
            };
            if ok {
                for chunk in chunk_text(
                    &format!(
                        "Build is clean — nothing to fix.\n\n```\n{}\n```",
                        output.trim()
                    ),
                    8,
                ) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
            } else {
                // Stream the error so the user sees it.
                let capped: String = output.chars().take(3000).collect();
                for chunk in chunk_text(
                    &format!(
                        "Build failed. Fix context loaded — send any message to start fixing.\n\n```\n{}\n```",
                        capped.trim()
                    ),
                    8,
                ) {
                    let _ = tx.send(InferenceEvent::Token(chunk)).await;
                }
                self.pending_fix_context = Some(capped);
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
                        let _ = write!(
                            message,
                            " Reachable alternative detected: {} ({})",
                            alt_name, alt_url
                        );
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
                let _ = write!(message, " Current plan: {plan}.");
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
                let _ = write!(message, " Existing plan: {plan}.");
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

        // Seamless Search Handover: investigation mode is turn-scoped in AUTO.
        if should_use_turn_scoped_investigation_mode(self.workflow_mode, intent.primary_class) {
            let _ = tx
                .send(InferenceEvent::Thought(
                    "Seamless search detected: using investigation mode for this turn...".into(),
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
                        let mut combined = Vec::with_capacity(topics.len());
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
                DirectAnswerKind::Help => {
                    let response = build_help_answer();
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
                        .write()
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
            if self.pinned_files.write().await.remove(path).is_some() {
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
                    let _ = write!(
                        base_prompt,
                        "\n\n# Project Context (from .hematite/settings.json)\n{}",
                        hint
                    );
                }
            }
            if let Some(profile_block) = crate::agent::workspace_profile::profile_prompt_block(
                &crate::tools::file_ops::workspace_root(),
            ) {
                let _ = write!(base_prompt, "\n\n{}", profile_block);
            }
            if let Some(strategy_block) =
                crate::agent::workspace_profile::profile_strategy_prompt_block(
                    &crate::tools::file_ops::workspace_root(),
                )
            {
                let _ = write!(base_prompt, "\n\n{}", strategy_block);
            }
            // L1: inject hot-files block if available (persists across sessions via vein.db).
            if let Some(ref l1) = self.l1_context {
                let _ = write!(base_prompt, "\n\n{}", l1);
            }
            if let Some(ref repo_map_block) = self.repo_map {
                let _ = write!(base_prompt, "\n\n{}", repo_map_block);
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
        let research_mode = (capability_needs_repo || !capability_mode)
            && intent.direct_answer.is_none()
            && intent.primary_class == QueryIntentClass::Research;
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
        let _ = write!(
            system_msg,
            "\n\n# WORKFLOW MODE\nCURRENT WORKFLOW: {}\n",
            self.workflow_mode.label()
        );
        if tiny_context_mode {
            system_msg
                .push_str("Use the narrowest safe behavior for this mode. Keep the turn short.\n");
        }
        if !tiny_context_mode && self.workflow_mode == WorkflowMode::Architect {
            system_msg.push_str("\n\n# ARCHITECT HANDOFF CONTRACT\n");
            system_msg.push_str(architect_handoff_contract());
            system_msg.push('\n');
        }
        if !tiny_context_mode && is_scaffold_request(&effective_user_input) {
            system_msg.push_str(scaffold_protocol());
        }
        if !tiny_context_mode {
            let workspace_root = crate::tools::file_ops::workspace_root();
            let skill_discovery =
                crate::agent::instructions::discover_agent_skills(&workspace_root, &config.trust);
            if let Some(bodies) = crate::agent::instructions::render_active_skill_bodies(
                &skill_discovery,
                &effective_user_input,
                8_000,
            ) {
                let _ = write!(system_msg, "\n\n{}", bodies);
            }
            // Inject any explicitly force-loaded skill from /skill <name>, then clear it.
            if let Some(forced_body) = self.pending_skill_inject.take() {
                let _ = write!(
                    system_msg,
                    "\n\n# Active Skill Instructions\n\n{}",
                    forced_body
                );
            }
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
                        let _ = writeln!(system_msg, "- {}", path);
                    }
                }
            }
        }
        if !tiny_context_mode {
            let pinned = self.pinned_files.read().await;
            if !pinned.is_empty() {
                system_msg.push_str("\n\n# ACTIVE CONTEXT (PINNED FILES)\n");
                system_msg.push_str("The following files are locked in your active memory for prioritized reference.\n\n");
                for (path, content) in pinned.iter() {
                    let _ = write!(system_msg, "## FILE: {}\n```\n{}\n```\n\n", path, content);
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
                        let mut s = String::with_capacity(50 * 80);
                        for (i, line) in content.lines().take(50).enumerate() {
                            if i > 0 {
                                s.push('\n');
                            }
                            s.push_str(line);
                        }
                        s + "\n... (truncated)"
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

        // ── Inject user task list ────────────────────────────────────────────
        if !tiny_context_mode {
            let tasks = crate::agent::tasks::load();
            if let Some(block) = crate::agent::tasks::render_prompt_block(&tasks) {
                final_system_msg.push_str("\n\n");
                final_system_msg.push_str(&block);
            }
        }

        // ── Inject shell history (once per session, non-chat modes) ──────────
        if !tiny_context_mode && !self.workflow_mode.is_chat() {
            if let Some(ref block) = self.shell_history_block {
                final_system_msg.push_str("\n\n");
                final_system_msg.push_str(block);
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

                let mut tool_calls = Vec::with_capacity(topics.len());
                let mut tool_msgs = Vec::with_capacity(topics.len());

                for topic in &topics {
                    let call_id = format!("prerun_{topic}");
                    let mut args_val =
                        host_inspection_args_from_prompt(topic, &effective_user_input);
                    if let Some(obj) = args_val.as_object_mut() {
                        obj.insert("max_entries".to_string(), Value::from(20));
                    }
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
        if loop_intervention.is_none() {
            if let Some(fix_ctx) = self.pending_fix_context.take() {
                loop_intervention = Some(format!(
                    "FIX MODE — The build is currently failing. Fix ONLY the error below. \
                     Do not refactor, add features, or touch unrelated code. \
                     After each edit call `verify_build` to check if the error is resolved. \
                     Stop as soon as the build is green.\n\n\
                     ## Current Build Error\n```\n{}\n```",
                    fix_ctx.trim()
                ));
            }
        }

        if loop_intervention.is_none() && needs_github_ops(&effective_user_input) {
            loop_intervention = Some(
                "GITHUB TOOL NOTICE: This query is about GitHub (PRs, issues, CI runs, or checks). \
                 Use the `github_ops` tool — never call `gh` via `shell`. \
                 For a quick overview, try `/pr` (PR status), `/ci` (CI status), or `/issue` (issues). \
                 The model should call `github_ops` with the appropriate `action` field."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_computation_sandbox(&effective_user_input) {
            loop_intervention = Some(
                "COMPUTATION INTEGRITY NOTICE: This query requires a real numeric result. \
                 You MUST NOT answer from training-data memory — that is a hallucination. \
                 TOOL SELECTION: \
                 • Use `run_code` for direct computation: arithmetic, percentages, unit conversion, \
                   date math, statistics on given numbers, hashes. \
                   Pass `language: \"python\"` for Python; omit or pass `language: \"javascript\"` for JS/Deno. \
                 • Use `scientific_compute` for: symbolic algebra/calculus (mode: \"symbolic\"), \
                   dimensional unit safety (mode: \"units\"), Big-O auditing (mode: \"complexity\"), \
                   SQL/Python analysis of a CSV/JSON/SQLite file (mode: \"dataset\"). \
                 RULE: every number in your response must come from tool output, not your weights. \
                 Write the code, run it, show the result."
                    .to_string(),
            );
        }

        // ── Crash Debug Routing: steer model toward run_with_backtrace for panic/crash queries ──
        if loop_intervention.is_none() && needs_crash_debug(&effective_user_input) {
            loop_intervention = Some(
                "CRASH DEBUG NOTICE: This query involves a runtime crash, panic, or segfault. \
                 Use `run_with_backtrace` instead of `shell` — it sets RUST_BACKTRACE=full automatically \
                 and returns a structured crash report with filtered stack trace. \
                 Example: run_with_backtrace(command: \"./target/debug/myapp [args]\") \
                 Do NOT use `shell` for crash investigation — you will lose the backtrace."
                    .to_string(),
            );
        }

        // ── Format Routing: steer model toward format_code instead of raw shell cargo fmt ──
        if loop_intervention.is_none() && needs_format(&effective_user_input) {
            loop_intervention = Some(
                "FORMAT NOTICE: Use the `format_code` tool — not `shell cargo fmt` / `rustfmt`. \
                 `format_code` auto-detects the workspace type (Rust/Node/Python), applies the right \
                 formatter, and returns a list of files that were changed. \
                 Use check=true to preview what needs reformatting without writing. \
                 Use path=<file> to format a single Rust file."
                    .to_string(),
            );
        }

        // ── Lint Routing: steer model toward lint_code instead of raw shell cargo clippy ──
        if loop_intervention.is_none() && needs_lint_check(&effective_user_input) {
            loop_intervention = Some(
                "LINT NOTICE: Use the `lint_code` tool — not `shell cargo clippy`. \
                 `lint_code` gives structured results: file:line, lint code, message, and fix suggestion. \
                 To apply all machine-fixable lints automatically, call lint_code(fix=true). \
                 If the working tree has uncommitted changes, also pass allow_dirty=true."
                    .to_string(),
            );
        }

        // ── Test Run Routing: steer model toward run_tests instead of raw shell cargo test ──
        if loop_intervention.is_none() && needs_test_run(&effective_user_input) {
            loop_intervention = Some(
                "TEST RUN NOTICE: Use the `run_tests` tool — not `shell` with `cargo test` / `pytest` / `npm test`. \
                 `run_tests` gives you structured pass/fail counts and extracted failure blocks automatically. \
                 Use the `filter` arg to run a specific test by name. \
                 Example: run_tests(filter: \"test_my_function\") or run_tests() for the full suite."
                    .to_string(),
            );
        }

        // ── HTTP Request Routing: steer model toward http_request instead of raw shell curl ──
        if loop_intervention.is_none() && needs_http_request(&effective_user_input) {
            loop_intervention = Some(
                "HTTP REQUEST NOTICE: Use the `http_request` tool — not `shell curl`. \
                 `http_request` gives you structured output: status code, key headers, and JSON body \
                 auto-pretty-printed. Supports GET/POST/PUT/DELETE/PATCH, Bearer token, Basic auth, \
                 and custom headers. Example: http_request(url: \"https://api.example.com/v1/items\", \
                 method: \"GET\", bearer_token: \"<token>\")."
                    .to_string(),
            );
        }

        // ── Docker Compose File Parsing: steer model toward docker_compose_tools ──
        if loop_intervention.is_none() && needs_docker_compose_tools(&effective_user_input) {
            loop_intervention = Some(
                "DOCKER COMPOSE NOTICE: Use the `docker_compose_tools` tool to parse and analyze docker-compose.yml files. \
                 Actions: services (default — summary of all services with image, ports, restart, depends_on), \
                 inspect (full detail for one service; pass 'service'), \
                 ports (all host:container port mappings across services), \
                 volumes (named volumes and bind mounts per service), \
                 networks (network definitions and service membership), \
                 env (environment variables per service; secrets redacted; optional 'service' filter), \
                 validate (check for missing image/build, undefined depends_on targets, privileged mode). \
                 Pass the compose file content as 'text'. \
                 Example: docker_compose_tools(action: 'services', text: '...') or \
                 docker_compose_tools(action: 'inspect', text: '...', service: 'api') or \
                 docker_compose_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_dockerfile_tools(&effective_user_input) {
            loop_intervention = Some(
                "DOCKERFILE NOTICE: Use the `dockerfile_tools` tool to parse, inspect, and validate Dockerfiles. \
                 Pass the Dockerfile content as 'text'. \
                 Actions: info (default — base image and tag per stage, exposed ports, labels, WORKDIR, USER, CMD, instruction counts), \
                 layers (all instructions in order with type and content), \
                 validate (check for: latest tag, running as root, ADD instead of COPY, curl|sh pipe, secrets in ENV/ARG, missing CMD/ENTRYPOINT, no HEALTHCHECK). \
                 Example: dockerfile_tools(action: 'info', text: '...') or \
                 dockerfile_tools(action: 'validate', text: '...') or \
                 dockerfile_tools(action: 'layers', text: '...')."
                    .to_string(),
            );
        }

        // ── Docker Routing: steer model toward docker_ops instead of raw shell docker ──
        if loop_intervention.is_none() && needs_docker_ops(&effective_user_input) {
            loop_intervention = Some(
                "DOCKER NOTICE: Use the `docker_ops` tool — not `shell docker`. \
                 `docker_ops` covers: ps, ps-all, logs, start, stop, restart, rm, images, pull, \
                 inspect, build, exec, stats, compose-ps, compose-up, compose-down. \
                 Example: docker_ops(action: \"ps\") or docker_ops(action: \"logs\", container: \"my-app\", tail: 50)."
                    .to_string(),
            );
        }

        // ── Secret Scan Routing: steer model toward secret_scanner ──
        if loop_intervention.is_none() && needs_secret_scan(&effective_user_input) {
            loop_intervention = Some(
                "SECRET SCAN NOTICE: Use the `secret_scanner` tool to search for committed secrets. \
                 It detects AWS keys, GitHub tokens, Stripe keys, Slack webhooks, private key blocks, \
                 database URLs, bearer tokens, password literals, and more across all text files. \
                 Binary files, lock files, and obvious placeholders are automatically skipped. \
                 Results are grouped by file with line numbers and a redacted snippet. \
                 Example: secret_scanner() to scan the entire workspace, or \
                 secret_scanner(path: \"src\") to scan a subdirectory."
                    .to_string(),
            );
        }

        // ── Diff Tools Routing: steer model toward diff_tools ──
        if loop_intervention.is_none() && needs_diff_tools(&effective_user_input) {
            loop_intervention = Some(
                "DIFF NOTICE: Use the `diff_tools` tool for comparing or patching text/files. \
                 Actions: compare (unified diff with context lines), patch (generate a .patch file), \
                 apply (apply a unified patch to a base), word-diff (inline [+added]/[-removed] tokens), \
                 stat (lines added/deleted/unchanged + similarity %). \
                 Provide text inline via 'text_a'/'text_b' or file paths via 'file_a'/'file_b'. \
                 Example: diff_tools(action: \"compare\", file_a: \"old.json\", file_b: \"new.json\", context: 5)."
                    .to_string(),
            );
        }

        // ── Regex Tools Routing: steer model toward regex_tools ──
        if loop_intervention.is_none() && needs_regex_tools(&effective_user_input) {
            loop_intervention = Some(
                "REGEX NOTICE: Use the `regex_tools` tool for regex work. \
                 Actions: test (match/no-match with excerpts), extract (all matches or named groups), \
                 replace (with optional limit), split (partition text), explain (plain-English breakdown), \
                 named-groups (extract named captures). \
                 Flags: case_insensitive, multiline, dot_all. \
                 Example: regex_tools(action: \"test\", pattern: \"\\\\d+\", text: \"abc 123\") or \
                 regex_tools(action: \"explain\", pattern: \"^(?P<year>\\\\d{4})-(?P<month>\\\\d{2})\")."
                    .to_string(),
            );
        }

        // ── YAML Tools Routing: steer model toward yaml_tools ──
        if loop_intervention.is_none() && needs_yaml_tools(&effective_user_input) {
            loop_intervention = Some(
                "YAML NOTICE: Use the `yaml_tools` tool for YAML work. \
                 Actions: validate, format, get (dot-path query like 'metadata.name'), keys, \
                 to-json, from-json, merge, diff. \
                 Provide inline YAML via 'yaml' arg or a file path via 'file' arg. \
                 Example: yaml_tools(action: \"get\", file: \"k8s/deploy.yaml\", path: \"spec.replicas\")."
                    .to_string(),
            );
        }

        // ── CSV Tools Routing: steer model toward csv_tools ──
        if loop_intervention.is_none() && needs_csv_tools(&effective_user_input) {
            loop_intervention = Some(
                "CSV NOTICE: Use the `csv_tools` tool for CSV data. \
                 Actions: read (table view), head (first N rows), columns, stats, filter, sort, \
                 to-json, to-markdown, count. \
                 Provide inline CSV via 'csv' arg or a file path via 'file' arg. \
                 Example: csv_tools(action: \"stats\", file: \"data.csv\") or \
                 csv_tools(action: \"filter\", file: \"data.csv\", column: \"status\", op: \"eq\", value: \"active\")."
                    .to_string(),
            );
        }

        // ── Encode Tools Routing: steer model toward encode_tools ──
        if loop_intervention.is_none() && needs_encode_tools(&effective_user_input) {
            loop_intervention = Some(
                "ENCODE NOTICE: Use the `encode_tools` tool for encoding/decoding. \
                 Actions: base64-encode, base64-decode, url-encode, url-decode, hex-encode, hex-decode, \
                 jwt-decode, html-encode, html-decode. \
                 All actions take an 'input' field. base64 actions also accept 'url_safe: true'. \
                 Example: encode_tools(action: \"base64-encode\", input: \"Hello, World!\") or \
                 encode_tools(action: \"jwt-decode\", input: \"eyJ...\")."
                    .to_string(),
            );
        }

        // ── Hash Tools Routing: steer model toward hash_tools ──
        if loop_intervention.is_none() && needs_hash_tools(&effective_user_input) {
            loop_intervention = Some(
                "HASH NOTICE: Use the `hash_tools` tool for cryptographic hashing. \
                 Actions: sha256 (default), sha512, md5, hmac-sha256 (requires 'key'), all (runs all at once). \
                 Provide the data via 'input' (inline string) or 'file' (path). \
                 Example: hash_tools(action: \"sha256\", input: \"Hello, World!\") or \
                 hash_tools(action: \"all\", file: \"src/main.rs\") or \
                 hash_tools(action: \"hmac-sha256\", input: \"data\", key: \"secret\")."
                    .to_string(),
            );
        }

        // ── TOML Tools Routing: steer model toward toml_tools ──
        if loop_intervention.is_none() && needs_toml_tools(&effective_user_input) {
            loop_intervention = Some(
                "TOML NOTICE: Use the `toml_tools` tool for TOML work. \
                 Actions: validate, format, get (dot-path query like 'package.name'), keys, to-json, from-json. \
                 Provide inline TOML via 'toml' arg or a file path via 'file' arg. \
                 Example: toml_tools(action: \"get\", file: \"Cargo.toml\", path: \"package.version\") or \
                 toml_tools(action: \"to-json\", file: \"config.toml\")."
                    .to_string(),
            );
        }

        // ── Text Tools Routing: steer model toward text_tools ──
        if loop_intervention.is_none() && needs_text_tools(&effective_user_input) {
            loop_intervention = Some(
                "TEXT NOTICE: Use the `text_tools` tool for text transformation. \
                 Case actions: to-snake, to-camel, to-pascal, to-kebab, to-screaming, to-title, to-lower, to-upper. \
                 Other actions: slugify, count (word/line/char stats), truncate (with 'max' and optional 'ellipsis'), \
                 pad (with 'width' and 'align': left/right/center), wrap (with 'width'), \
                 repeat (with 'n' and optional 'sep'), reverse, \
                 lines (with optional 'sort', 'dedupe', 'filter_empty' booleans). \
                 All actions take an 'input' field. \
                 Example: text_tools(action: \"to-snake\", input: \"MyClassName\") or \
                 text_tools(action: \"count\", input: \"some text here\")."
                    .to_string(),
            );
        }

        // ── Date Tools Routing: steer model toward date_tools ──
        if loop_intervention.is_none() && needs_date_tools(&effective_user_input) {
            loop_intervention = Some(
                "DATE NOTICE: Use the `date_tools` tool for date/time work. \
                 Actions: now (current time in UTC/local/ISO/epoch/week), \
                 parse (parse any date string), format (reformat with strftime pattern), \
                 add (add days/weeks/months/years/hours/minutes), \
                 diff (duration between two dates via 'from'/'to' fields), \
                 timestamp (date → Unix epoch), from-timestamp (epoch → human date, auto-detects ms), \
                 relative ('3 days ago' / 'in 2 hours'), weekday (weekday name + ISO week). \
                 Example: date_tools(action: \"diff\", from: \"2024-01-01\", to: \"2024-12-31\") or \
                 date_tools(action: \"add\", input: \"2024-06-15\", months: 3)."
                    .to_string(),
            );
        }

        // ── Number Tools Routing: steer model toward number_tools ──
        if loop_intervention.is_none() && needs_number_tools(&effective_user_input) {
            loop_intervention = Some(
                "NUMBER NOTICE: Use the `number_tools` tool for number conversion and math. \
                 Actions: convert (base conversion — omit 'to' to show all bases at once; \
                 accepts 0x/0b/0o prefixes), format (thousands separators, scientific, engineering, SI), \
                 roman (int → Roman numeral), from-roman (Roman → int), \
                 si (show with SI prefix like k/M/G), factors (prime factorization + primality), \
                 gcd (GCD + LCM via 'a' and 'b' fields), clamp (clamp 'value' to 'min'/'max'). \
                 Example: number_tools(action: \"convert\", input: \"255\") or \
                 number_tools(action: \"factors\", input: 360)."
                    .to_string(),
            );
        }

        // ── UUID Gen Routing: steer model toward uuid_gen ──
        if loop_intervention.is_none() && needs_uuid_gen(&effective_user_input) {
            loop_intervention = Some(
                "UUID NOTICE: Use the `uuid_gen` tool for UUID generation and validation. \
                 Actions: generate (default — single UUID v4 with metadata), \
                 validate (check format, decode version/variant), \
                 nil (return the all-zeros nil UUID), \
                 bulk (generate N UUIDs at once, up to 100 — pass 'n' field). \
                 All actions accept 'upper: true' for uppercase output. \
                 Example: uuid_gen(action: \"generate\") or uuid_gen(action: \"bulk\", n: 10)."
                    .to_string(),
            );
        }

        // ── Cron Tools Routing: steer model toward cron_tools ──
        if loop_intervention.is_none() && needs_cron_tools(&effective_user_input) {
            loop_intervention = Some(
                "CRON NOTICE: Use the `cron_tools` tool for cron expression work. \
                 Actions: explain (field-by-field breakdown of any cron expression), \
                 validate (check if an expression is valid), \
                 next (list the next N run times from now — pass 'n' for count, default 5), \
                 describe (one-line plain-English summary). \
                 Pass the expression via 'expression' or 'input'. \
                 Example: cron_tools(action: \"explain\", expression: \"0 */6 * * *\") or \
                 cron_tools(action: \"next\", expression: \"0 9 * * 1\", n: 5)."
                    .to_string(),
            );
        }

        // ── IP Tools Routing: steer model toward ip_tools ──
        if loop_intervention.is_none() && needs_ip_tools(&effective_user_input) {
            loop_intervention = Some(
                "IP NOTICE: Use the `ip_tools` tool for IP address and CIDR calculations. \
                 Actions: info (parse an IP address — class, type, binary, decimal, hex), \
                 cidr (CIDR breakdown — pass '192.168.1.0/24' style input: network, broadcast, range, usable hosts), \
                 contains (check if an IP is in a CIDR network — pass 'ip' and 'cidr' fields), \
                 convert (convert between IPv4 decimal/hex/binary formats), \
                 subnet (given IP + mask, show network info). \
                 Example: ip_tools(action: \"cidr\", input: \"10.0.0.0/8\") or \
                 ip_tools(action: \"contains\", ip: \"192.168.1.50\", cidr: \"192.168.1.0/24\")."
                    .to_string(),
            );
        }

        // ── Color Tools Routing: steer model toward color_tools ──
        if loop_intervention.is_none() && needs_color_tools(&effective_user_input) {
            loop_intervention = Some(
                "COLOR NOTICE: Use the `color_tools` tool for color conversion and analysis. \
                 Actions: info (full breakdown — hex, RGB, HSL, luminance, WCAG contrast on white/black), \
                 convert (any format → hex + RGB + HSL), \
                 contrast (WCAG contrast ratio between two colors — pass 'color1' and 'color2'), \
                 mix (blend two colors — pass 'color1', 'color2', optional 'ratio' 0.0–1.0), \
                 lighten / darken (adjust lightness by 'amount' percent), \
                 palette (complementary, triadic, analogous, lighter/darker variants). \
                 Accepts: #RRGGBB, #RGB, rgb(R,G,B), hsl(H,S%,L%), or CSS color names. \
                 Example: color_tools(action: \"contrast\", color1: \"#FFFFFF\", color2: \"#0066CC\")."
                    .to_string(),
            );
        }

        // ── SemVer Tools Routing: steer model toward semver_tools ──
        if loop_intervention.is_none() && needs_semver_tools(&effective_user_input) {
            loop_intervention = Some(
                "SEMVER NOTICE: Use the `semver_tools` tool for semantic version work. \
                 Actions: parse (break a version into major/minor/patch/pre-release/build meta), \
                 compare (compare two versions — pass 'a' and 'b'), \
                 bump (increment a version — pass 'input' and 'part': major/minor/patch/premajor/preminor/prepatch), \
                 validate (check if a string is valid semver), \
                 satisfies (check if a version matches a range like '^1.2.3' or '>=2.0.0 <3.0.0' — pass 'version' and 'range'), \
                 sort (sort an array of versions — pass 'versions' array and optional 'order': asc/desc). \
                 Example: semver_tools(action: \"satisfies\", version: \"1.5.3\", range: \"^1.2.0\") or \
                 semver_tools(action: \"bump\", input: \"1.4.2\", part: \"minor\")."
                    .to_string(),
            );
        }

        // ── Password Gen Routing: steer model toward password_gen ──
        if loop_intervention.is_none() && needs_password_gen(&effective_user_input) {
            loop_intervention = Some(
                "PASSWORD NOTICE: Use the `password_gen` tool for secure password generation and analysis. \
                 Actions: generate (default — random password; options: 'length' (default 16), \
                 'upper'/'lower'/'digits'/'symbols' booleans, 'no_ambiguous', 'count' for multiple), \
                 passphrase (word-based — options: 'words' (default 4), 'separator', 'capitalize', 'number', 'count'), \
                 strength (analyze password strength — pass 'input'), \
                 pin (numeric PIN — options: 'length' (default 6), 'count'). \
                 Example: password_gen(action: \"generate\", length: 20, symbols: true) or \
                 password_gen(action: \"passphrase\", words: 5)."
                    .to_string(),
            );
        }

        // ── JWT Tools Routing: steer model toward jwt_tools ──
        if loop_intervention.is_none() && needs_jwt_tools(&effective_user_input) {
            loop_intervention = Some(
                "JWT NOTICE: Use the `jwt_tools` tool for JWT decode, verification, and signing. \
                 Actions: decode (decode header + claims without signature check — pass 'token'), \
                 verify (verify HS256/HS384/HS512 HMAC signature — pass 'token' and 'secret'), \
                 sign (create a new JWT — pass 'claims' object, 'secret', optional 'algorithm' default HS256), \
                 inspect (expiry/validity summary without secret — pass 'token'). \
                 Example: jwt_tools(action: \"verify\", token: \"eyJ...\", secret: \"my-secret\") or \
                 jwt_tools(action: \"sign\", claims: {\"sub\": \"user123\", \"exp\": 9999999999}, secret: \"key\")."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_k8s_tools(&effective_user_input) {
            loop_intervention = Some(
                "KUBERNETES NOTICE: Use the `k8s_tools` tool to parse, inspect, and validate Kubernetes manifests (Deployment, Service, Pod, StatefulSet, DaemonSet, Job, CronJob, Ingress, ConfigMap). \
                 Pass the manifest YAML content as 'text'. \
                 Actions: info (default — kind, apiVersion, name, namespace, labels, replicas, selector, port list, container summary), \
                 containers (detailed per-container breakdown: image, ports, resource requests/limits, env vars, volume mounts, liveness/readiness probes, security context), \
                 volumes (all volume types with source details: ConfigMap, Secret, PVC, HostPath, EmptyDir, NFS), \
                 validate (checks: missing kind/apiVersion/name, image latest tag, missing resource limits, privileged containers, running as root, missing liveness/readiness probes, hostPath volumes, hostNetwork, single replica). \
                 Example: k8s_tools(action: 'info', text: '...') or \
                 k8s_tools(action: 'containers', text: '...') or \
                 k8s_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        // ── XML Tools Routing: steer model toward xml_tools ──
        if loop_intervention.is_none() && needs_xml_tools(&effective_user_input) {
            loop_intervention = Some(
                "XML NOTICE: Use the `xml_tools` tool for XML parsing, formatting, and conversion. \
                 Actions: validate (default — parse and summarize root element, depth, child count), \
                 format (pretty-print with 2-space indentation), \
                 get (navigate to a specific element via dot-path like 'project.build' or 'deps.dep[2]' — pass 'path'), \
                 keys (list immediate children/attributes of an element — pass optional 'path'), \
                 to-json (convert the XML document to JSON with @ prefix for attributes, #text for content), \
                 query (find all elements matching a tag name anywhere in the document — pass 'tag'). \
                 Pass 'xml' for inline XML or 'file' for a file path. \
                 Example: xml_tools(action: \"to-json\", file: \"pom.xml\") or \
                 xml_tools(action: \"query\", xml: \"<root>...</root>\", tag: \"dependency\")."
                    .to_string(),
            );
        }

        // ── Archive Tools Routing: steer model toward archive_tools ──
        if loop_intervention.is_none() && needs_asn1_tools(&effective_user_input) {
            loop_intervention = Some(
                "ASN.1 NOTICE: Use the `asn1_tools` tool to parse and inspect ASN.1 DER/BER encoded binary data \
                 without external utilities. Used in X.509 certificates, PKCS#8/PKCS#12 keys, SNMP, and LDAP. \
                 Actions: parse (default — decode DER/BER structure as an indented tag/length/value tree), \
                 oid (look up an OID number to its name — 200+ well-known OIDs covered; pass 'oid' field), \
                 decode_cert (X.509 certificate quick summary — subject, issuer, validity, serial, algorithms), \
                 info (tag class/number/constructed flag and byte structure at root level). \
                 Pass 'hex' (hex-encoded DER bytes) or 'file' (path to .der/.cer/.crt/.p8 file). \
                 Example: asn1_tools(action: 'oid', oid: '2.5.4.3') or \
                 asn1_tools(action: 'parse', hex: '3082...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_archive_tools(&effective_user_input) {
            loop_intervention = Some(
                "ARCHIVE NOTICE: Use the `archive_tools` tool to inspect and read zip archives without external tools. \
                 Actions: list (default — tabular listing of all entries with name, size, method; supports 'max' and 'filter'), \
                 info (overall archive statistics — file count, total size, compression ratio), \
                 inspect (detailed metadata for a specific entry — pass 'entry' with the entry name), \
                 extract (read a specific text entry as a string — pass 'entry'; limited to 1 MB text files). \
                 Pass 'file' with the path to the .zip, .jar, .whl, .vsix, .apk, or other zip-format archive. \
                 Example: archive_tools(action: \"list\", file: \"app.jar\") or \
                 archive_tools(action: \"extract\", file: \"dist.zip\", entry: \"README.md\")."
                    .to_string(),
            );
        }

        // ── SQLite Tools Routing: steer model toward sqlite_tools ──
        if loop_intervention.is_none() && needs_sqlite_tools(&effective_user_input) {
            loop_intervention = Some(
                "SQLITE NOTICE: Use the `sqlite_tools` tool to inspect and query SQLite databases in read-only mode. \
                 Actions: tables (default — list all tables with row counts, plus views and indexes), \
                 schema (show CREATE SQL and column info; pass 'table' to scope to one table), \
                 query (run a SELECT/EXPLAIN/WITH/PRAGMA statement; pass 'sql'; max 100 rows, use 'limit' to override), \
                 info (database file metadata — page size, encoding, journal mode, SQLite version), \
                 export (dump a table as CSV or JSON; pass 'table' and optionally 'format' and 'limit'). \
                 Pass 'file' with the path to the .sqlite or .db file. Only read-only SQL is allowed — \
                 INSERT/UPDATE/DELETE/DROP/CREATE are blocked. \
                 Example: sqlite_tools(action: \"tables\", file: \"app.db\") or \
                 sqlite_tools(action: \"query\", file: \"data.sqlite\", sql: \"SELECT * FROM users LIMIT 10\")."
                    .to_string(),
            );
        }

        // ── Markdown Tools Routing: steer model toward markdown_tools ──
        if loop_intervention.is_none() && needs_markdown_tools(&effective_user_input) {
            loop_intervention = Some(
                "MARKDOWN NOTICE: Use the `markdown_tools` tool to parse and analyze Markdown documents without external tools. \
                 Actions: toc (generate a table of contents with anchor links; 'depth' limits heading levels), \
                 stats (word count, reading time, heading/code/link/image/table/blockquote counts), \
                 extract (extract specific elements; pass 'what' = headings | code | links | images; 'lang' filters code by language), \
                 links (list all hyperlinks and images with text and URL), \
                 to-html (render Markdown to HTML; 'wrap: true' for a full HTML document with optional 'title'), \
                 strip (remove all Markdown formatting and return plain text). \
                 Pass 'text' for inline Markdown or 'file' for a .md file path. \
                 Example: markdown_tools(action: \"toc\", file: \"README.md\") or \
                 markdown_tools(action: \"stats\", text: \"# Hello\\nThis is **bold**.\")."
                    .to_string(),
            );
        }

        // ── URL Tools Routing: steer model toward url_tools ──
        if loop_intervention.is_none() && needs_url_tools(&effective_user_input) {
            loop_intervention = Some(
                "URL NOTICE: Use the `url_tools` tool for URL parsing, building, and manipulation without external utilities. \
                 Actions: parse (default — break a URL into scheme, host, port, path, query params, fragment), \
                 build (construct a URL from 'scheme', 'host', 'path', optional 'port'/'query'/'params'/'fragment'), \
                 params (list, set, or remove query parameters; pass 'op': list | set | remove, and 'key'/'value'), \
                 encode (percent-encode a string; 'component: true' for strict component encoding), \
                 decode (percent-decode a string), \
                 normalize (lowercase scheme/host, resolve dot segments), \
                 validate (check if a URL is valid and flag common issues). \
                 Pass 'url' with the URL string. \
                 Example: url_tools(action: \"parse\", url: \"https://api.example.com/v2/search?q=rust&page=2\") or \
                 url_tools(action: \"params\", url: \"https://example.com/?a=1&b=2\", op: \"set\", key: \"b\", value: \"99\")."
                    .to_string(),
            );
        }

        // ── Line Tools Routing: steer model toward line_tools ──
        if loop_intervention.is_none() && needs_line_tools(&effective_user_input) {
            loop_intervention = Some(
                "LINE NOTICE: Use the `line_tools` tool for line-based text processing without external utilities. \
                 Actions: grep (default — filter lines matching a pattern; 'pattern' required; 'regex: true' for regex; 'invert: true' for non-matching; 'ignore_case: true'), \
                 head (first N lines; 'n' default 10), \
                 tail (last N lines; 'n' default 10), \
                 sort (sort lines; 'numeric: true' for numeric sort; 'reverse: true'; 'ignore_case: true'; 'unique: true' to deduplicate after sort), \
                 unique (remove duplicate lines preserving order; 'count: true' to show frequency; 'sorted: true' to rank by frequency), \
                 count (line/word/character/byte counts), \
                 slice (extract lines from 'from' to 'to' — 1-based line numbers), \
                 number (add line numbers; 'start' and 'step' args), \
                 join (join all lines into one string; 'sep' sets separator, default ', '), \
                 replace (find-and-replace across all lines; 'from' and 'to' required; 'regex: true'; 'limit' for max replacements), \
                 cut (extract one field per line by delimiter; 'field' is 1-based, 'd'/'delimiter' default tab). \
                 Pass 'text' for inline text or 'file' for a file path. \
                 Example: line_tools(action: \"grep\", file: \"app.log\", pattern: \"ERROR\") or \
                 line_tools(action: \"sort\", text: \"banana\\napple\\ncherry\", unique: true)."
                    .to_string(),
            );
        }

        // ── Path Tools Routing: steer model toward path_tools ──
        if loop_intervention.is_none() && needs_path_tools(&effective_user_input) {
            loop_intervention = Some(
                "PATH NOTICE: Use the `path_tools` tool for path parsing and manipulation without external utilities. \
                 Actions: parse (default — split a path into parent, filename, stem, extension, and components), \
                 join (join path segments; 'base' + 'parts' array, or 'paths' array), \
                 normalize (resolve . and .. segments logically without touching the filesystem), \
                 relative (compute relative path from 'from' to 'to'), \
                 basename (filename with extension), \
                 stem (filename without extension), \
                 extension (current extension; optionally pass 'replace' to swap it), \
                 is-absolute (check if a path is absolute or relative). \
                 Pass 'path' for the path string. \
                 Example: path_tools(action: \"parse\", path: \"src/tools/mod.rs\") or \
                 path_tools(action: \"relative\", from: \"/a/b\", to: \"/a/c/d\")."
                    .to_string(),
            );
        }

        // ── Table Tools Routing: steer model toward table_tools ──
        if loop_intervention.is_none() && needs_table_tools(&effective_user_input) {
            loop_intervention = Some(
                "TABLE NOTICE: Use the `table_tools` tool to format tabular data as ASCII or markdown tables without external utilities. \
                 Actions: format (default — format 'rows' (2D array) + optional 'headers' as a table), \
                 from-csv (parse CSV text from 'text' or 'csv' and render as table; 'header: true' by default), \
                 from-json (format a JSON array of objects or 2D array as a table; pass 'json' or 'text'), \
                 to-markdown (render any input as a GitHub-flavored markdown table), \
                 transpose (flip rows and columns; pass 'rows' 2D array and optional 'headers'). \
                 Style options: 'simple' (default — spaces + dashes), 'bordered' (| boxes), 'markdown'. \
                 Example: table_tools(action: \"format\", headers: [\"Name\",\"Score\"], rows: [[\"Alice\",\"95\"],[\"Bob\",\"87\"]]) or \
                 table_tools(action: \"from-csv\", text: \"name,age\\nAlice,30\\nBob,25\")."
                    .to_string(),
            );
        }

        // ── Hex Tools Routing: steer model toward hex_tools ──
        if loop_intervention.is_none() && needs_hex_tools(&effective_user_input) {
            loop_intervention = Some(
                "HEX NOTICE: Use the `hex_tools` tool for hex dump, binary analysis, and hex encoding/decoding without external utilities. \
                 Actions: dump (default — xxd-style hex dump; 'width' bytes per row, default 16; 'limit' max bytes, default 4096), \
                 strings (extract printable ASCII strings from binary data; 'min' minimum length, default 4), \
                 bytes (byte frequency histogram, null count, high-byte count, Shannon entropy), \
                 analyze (magic byte file type detection + entropy estimate), \
                 to-hex (encode bytes or text as hex string; 'sep' separator, 'upper: true' for uppercase), \
                 from-hex (decode a hex string back to bytes or text). \
                 Pass 'file' for a file path, 'hex' for an existing hex string, or 'text'/'input' for UTF-8 text. \
                 Example: hex_tools(action: \"dump\", file: \"binary.bin\") or \
                 hex_tools(action: \"to-hex\", text: \"Hello\")."
                    .to_string(),
            );
        }

        // ── INI Tools Routing: steer model toward ini_tools ──
        if loop_intervention.is_none() && needs_ini_tools(&effective_user_input) {
            loop_intervention = Some(
                "INI NOTICE: Use the `ini_tools` tool to parse, query, and convert INI/config files without external utilities. \
                 Actions: parse (default — display all sections and key-value pairs), \
                 get (retrieve a specific value; pass 'key' as 'section.key' dot notation or separate 'section' + 'key' args), \
                 sections (list all section names with key counts), \
                 keys (list all keys in a section; pass 'section' to scope), \
                 validate (check for duplicate keys, duplicate sections, empty sections), \
                 to-json (convert the INI document to a JSON object), \
                 to-toml (convert the INI document to TOML format). \
                 Pass 'text' or 'ini' for inline INI text, or 'file' for a file path. \
                 Example: ini_tools(action: \"get\", file: \"config.ini\", key: \"database.host\") or \
                 ini_tools(action: \"to-json\", text: \"[server]\\nport=8080\")."
                    .to_string(),
            );
        }

        // ── Duration Tools Routing: steer model toward duration_tools ──
        if loop_intervention.is_none() && needs_duration_tools(&effective_user_input) {
            loop_intervention = Some(
                "DURATION NOTICE: Use the `duration_tools` tool to parse, humanize, convert, and add time durations. \
                 Actions: parse (default — break any duration into years/days/hours/minutes/seconds), \
                 humanize (convert seconds to readable text; 'style: compact' for short form like '1h 30m'), \
                 convert (express as seconds/minutes/hours/days/weeks; pass 'to' for a specific unit), \
                 add (sum two durations via 'a'/'b' or sum an array via 'durations'). \
                 Input formats: '1h 30m 45s', '90 minutes', '5400' (seconds), '1:30:45' (HH:MM:SS), 'PT1H30M45S' (ISO 8601). \
                 Example: duration_tools(action: \"parse\", duration: \"1h 30m\") or \
                 duration_tools(action: \"humanize\", duration: \"5400\", style: \"compact\")."
                    .to_string(),
            );
        }

        // ── Dotenv Tools Routing: steer model toward dotenv_tools ──
        if loop_intervention.is_none() && needs_dotenv_tools(&effective_user_input) {
            loop_intervention = Some(
                "DOTENV NOTICE: Use the `dotenv_tools` tool to parse, validate, convert, and merge .env files without external utilities. \
                 Actions: parse (default — display all key-value pairs with line numbers; 'show_values: false' to redact), \
                 validate (check key names, quote balance, duplicate keys), \
                 get (retrieve a specific key's value; pass 'key'), \
                 list (show key names only, no values), \
                 to-json (convert to JSON object), \
                 to-shell (generate export/SET commands; 'shell: powershell' or 'shell: cmd'), \
                 merge (overlay one .env on another; pass 'base' and 'overlay' text — overlay wins on conflict). \
                 Pass 'text' for inline .env content or 'file' for a file path. \
                 Example: dotenv_tools(action: \"parse\", file: \".env\") or \
                 dotenv_tools(action: \"merge\", base: \"KEY=a\", overlay: \"KEY=b\\nNEW=c\")."
                    .to_string(),
            );
        }

        // ── ANSI Tools Routing: steer model toward ansi_tools ──
        if loop_intervention.is_none() && needs_ansi_tools(&effective_user_input) {
            loop_intervention = Some(
                "ANSI NOTICE: Use the `ansi_tools` tool to process ANSI/VT100 escape codes without external utilities. \
                 Actions: strip (default — remove all ANSI escape sequences, output plain text), \
                 colorize (wrap text in ANSI SGR codes; pass 'fg'/'bg' color name, 'style' or array of styles), \
                 length (print visible character count, excluding ANSI escape sequences), \
                 parse (identify and describe all ANSI sequences found in input). \
                 Colors: black, red, green, yellow, blue, magenta, cyan, white, bright_red, bright_green, etc. \
                 Styles: bold, dim, italic, underline, blink, reverse, strikethrough. \
                 Example: ansi_tools(action: \"strip\", text: \"\\x1b[31mHello\\x1b[0m\") or \
                 ansi_tools(action: \"colorize\", text: \"Warning!\", fg: \"yellow\", style: \"bold\")."
                    .to_string(),
            );
        }

        // ── Template Tools Routing: steer model toward template_tools ──
        if loop_intervention.is_none() && needs_template_tools(&effective_user_input) {
            loop_intervention = Some(
                "TEMPLATE NOTICE: Use the `template_tools` tool to render {{VAR}} placeholder templates without external utilities. \
                 Actions: render (default — substitute {{VAR}} and {{VAR|default}} placeholders using 'vars' object; \
                   'strict: true' to error on undefined vars), \
                 list (list all unique {{VAR}} placeholder names found in the template), \
                 validate (check for unbalanced braces and undefined variables given 'vars'), \
                 preview (show each placeholder with DEFINED/MISSING status + rendered preview with [MISSING:VAR] markers). \
                 Pass 'template' (or 'text'/'file') for the template string. Pass 'vars' as a JSON object of substitutions. \
                 Example: template_tools(action: \"render\", template: \"Hello {{NAME}}!\", vars: {\"NAME\": \"World\"}) or \
                 template_tools(action: \"list\", template: \"{{HOST}}:{{PORT|8080}}/{{PATH}}\")."
                    .to_string(),
            );
        }

        // ── Char Tools Routing: steer model toward char_tools ──
        if loop_intervention.is_none() && needs_char_tools(&effective_user_input) {
            loop_intervention = Some(
                "CHAR NOTICE: Use the `char_tools` tool for Unicode character inspection without external utilities. \
                 Actions: info (default — full Unicode info for a char or string: codepoint, block, category, decimal/hex/octal/binary representations), \
                 codepoint (char → U+XXXX or provide 'codepoint' number/U+XXXX string → char), \
                 escape (escape non-printable or non-ASCII chars; 'style: unicode' for \\u{XXXXX} (default), 'json' for \\uXXXX, 'hex' for \\xXX), \
                 unescape (decode \\u{XXXXX}, \\uXXXX, \\xXX sequences back to chars), \
                 check (test character properties: is_ascii, is_alphabetic, is_numeric, is_alphanumeric, is_uppercase, is_lowercase, is_whitespace, is_control). \
                 Pass 'input' or 'text' for string input. For codepoint action: pass 'codepoint' for reverse lookup."
                    .to_string(),
            );
        }

        // ── Stat Tools Routing: steer model toward stat_tools ──
        if loop_intervention.is_none() && needs_stat_tools(&effective_user_input) {
            loop_intervention = Some(
                "STAT NOTICE: Use the `stat_tools` tool for statistical analysis on number arrays without external utilities. \
                 Actions: describe (default — count/sum/min/max/range/mean/median/stddev/variance/Q1/Q3/IQR), \
                 histogram (ASCII bar chart; 'bins' for bin count, 'width' for bar width), \
                 percentile (compute percentiles; 'p' for custom list like [25, 50, 75, 90, 99] or single value), \
                 mode (most frequent values with frequency counts), \
                 outliers (find values beyond N stddevs; 'threshold' for sigma cutoff (default 2.0); 'method: iqr' for IQR fences), \
                 zscore (normalize each value to its z-score), \
                 correlate (Pearson r between two series; pass 'a' and 'b' as arrays). \
                 Pass 'numbers' as a JSON array or 'data' as a comma/newline-delimited string. \
                 Example: stat_tools(action: \"describe\", numbers: [1, 2, 3, 4, 5]) or \
                 stat_tools(action: \"outliers\", numbers: [...], threshold: 2.5)."
                    .to_string(),
            );
        }

        // ── RSS Tools Routing: steer model toward rss_tools ──
        if loop_intervention.is_none() && needs_rss_tools(&effective_user_input) {
            loop_intervention = Some(
                "RSS NOTICE: Use the `rss_tools` tool to parse RSS 2.0 and Atom 1.0 feeds without external utilities. \
                 Actions: list (default — show all entries with title/date/author/link/description snippet; 'limit' to cap count), \
                 info (feed metadata — type, title, description, language, generator, last updated, author list), \
                 links (extract all entry hyperlinks with titles), \
                 search (filter entries by 'query' or 'q' — matches title, description, and author). \
                 Pass 'text'/'xml'/'rss' for inline feed content or 'file' for a path to an .xml or .rss file. \
                 Example: rss_tools(action: \"list\", file: \"feed.xml\") or \
                 rss_tools(action: \"search\", text: \"...\", query: \"python\")."
                    .to_string(),
            );
        }

        // ── KeyVal Tools Routing: steer model toward keyval_tools ──
        if loop_intervention.is_none() && needs_keyval_tools(&effective_user_input) {
            loop_intervention = Some(
                "KEYVAL NOTICE: Use the `keyval_tools` tool to set, get, list, and delete persistent key-value pairs \
                 stored in `.hematite/kv.json` (or `~/.hematite/kv.json` outside a project). \
                 Actions: set (store a value; 'key' + 'value' — value can be any JSON type), \
                 get (retrieve a value by 'key'), \
                 list (show all keys and values; optional 'prefix' to filter), \
                 delete (remove a key), \
                 clear (wipe all keys or all keys matching 'prefix'), \
                 keys (list key names only). \
                 Use 'namespace'/'ns' to prefix all keys (e.g. ns: 'build' → key 'build:version'). \
                 Example: keyval_tools(action: \"set\", key: \"api_version\", value: \"v2\") or \
                 keyval_tools(action: \"list\")."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_net_lookup_tools(&effective_user_input) {
            loop_intervention = Some(
                "NET LOOKUP NOTICE: Use the `net_lookup_tools` tool to look up well-known TCP/UDP ports, \
                 service names, and IANA IP protocol numbers — no shell commands needed. \
                 Actions: port (look up a port number → service/protocol), \
                 service (look up a service name → port numbers), \
                 search (fuzzy search across service names and descriptions), \
                 protocol (look up an IP protocol number or name; omit args to list all). \
                 Example: net_lookup_tools(action: \"port\", port: 443) or \
                 net_lookup_tools(action: \"search\", query: \"database\") or \
                 net_lookup_tools(action: \"protocol\", number: 6)."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_money_tools(&effective_user_input) {
            loop_intervention = Some(
                "MONEY NOTICE: Use the `money_tools` tool for financial calculations — no external libraries needed. \
                 Actions: compound_interest (principal, rate %, periods, n compounds/yr), \
                 loan (principal, annual_rate %, term_months → monthly payment + amortization summary), \
                 apr_to_apy (convert APR to APY given compounds per year), \
                 discount (original price + % off → sale price and savings), \
                 percent_of (what percent is A of B, or what is X% of N), \
                 format_currency (format a number with currency symbol and thousands separators), \
                 tip (bill amount + tip % + optional split → per-person total), \
                 split_bill (total + people + optional tip % → per-person share). \
                 Example: money_tools(action: \"loan\", principal: 250000, annual_rate: 6.5, term_months: 360) or \
                 money_tools(action: \"compound_interest\", principal: 10000, rate: 5, periods: 10, n: 12)."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_size_tools(&effective_user_input) {
            loop_intervention = Some(
                "SIZE NOTICE: Use the `size_tools` tool to convert, parse, and compare data sizes without shell commands. \
                 Actions: convert (default — bytes ↔ KB/MB/GB/TB/KiB/MiB/GiB/TiB; optional 'to' for a specific unit), \
                 parse (resolve a size string to bytes + human-readable forms), \
                 format (auto/decimal/binary human-readable label), \
                 compare (compare two sizes and show ratio and difference), \
                 bandwidth (estimate transfer time at a given speed, or compute speed from size+time; \
                   omit 'speed'/'time' for a table of common connection speeds). \
                 Input 'size'/'input'/'value': '1.5 GB', '512 MiB', '2048', '100 Mbps'. \
                 Example: size_tools(action: 'convert', size: '1.5 GB') or \
                 size_tools(action: 'bandwidth', size: '4 GB', speed: '100 Mbps')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_validate_tools(&effective_user_input) {
            loop_intervention = Some(
                "VALIDATE NOTICE: Use the `validate_tools` tool to validate common data formats without external utilities. \
                 Actions (or use 'auto' to detect type automatically): \
                 email, ipv4, ipv6, cidr, mac, url, credit_card (Luhn check), isbn (ISBN-10/13), \
                 uuid (RFC 4122 version/variant), phone (NANP US/CA or E.164 international), \
                 semver (SemVer 2.0), hex_color (#RGB / #RRGGBB / #RGBA / #RRGGBBAA). \
                 Pass 'value'/'input'/'text' with the string to validate. \
                 Example: validate_tools(action: 'email', value: 'user@example.com') or \
                 validate_tools(action: 'cidr', value: '192.168.1.0/24') or \
                 validate_tools(action: 'auto', value: '2.0.0-alpha.1')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_token_tools(&effective_user_input) {
            loop_intervention = Some(
                "TOKEN NOTICE: Use the `token_tools` tool to estimate LLM token counts without external utilities. \
                 Actions: estimate (default — chars/4 + words*1.3 heuristics with context window fill bars for 4K/8K/32K/128K), \
                 budget (fill % for a specific context window — pass 'context_size' in tokens, default 8192), \
                 compare (token cost difference between two texts — pass 'a' and 'b'), \
                 truncate (cut text to approximately N tokens — pass 'text' and 'max_tokens', default 1000). \
                 Example: token_tools(action: 'estimate', text: '...') or \
                 token_tools(action: 'budget', text: '...', context_size: 4096)."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_mime_tools(&effective_user_input) {
            loop_intervention = Some(
                "MIME NOTICE: Use the `mime_tools` tool to look up MIME types by extension without external utilities. \
                 Actions: from_ext (default — file extension to MIME type and category; pass 'ext' like 'js', '.ts', or 'image.json'), \
                 from_mime (MIME type string to file extensions; pass 'mime' like 'image/png'), \
                 search (fuzzy search on extension or MIME type string; pass 'query'), \
                 category (list all types in a category — text/image/audio/video/application/font; omit for summary). \
                 Example: mime_tools(action: 'from_ext', ext: 'pdf') or \
                 mime_tools(action: 'search', query: 'audio') or \
                 mime_tools(action: 'category', category: 'image')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_http_status_tools(&effective_user_input) {
            loop_intervention = Some(
                "HTTP STATUS NOTICE: Use the `http_status_tools` tool to look up HTTP status codes without external utilities. \
                 Actions: lookup (default — code number to reason and description; pass 'code' like 404), \
                 search (keyword search in reason and description; pass 'query'), \
                 category (list codes in a category — 1xx/2xx/3xx/4xx/5xx; omit for summary), \
                 list (all codes or filtered by 'category'). \
                 Example: http_status_tools(action: 'lookup', code: 429) or \
                 http_status_tools(action: 'category', category: '4xx') or \
                 http_status_tools(action: 'search', query: 'redirect')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_http_parse_tools(&effective_user_input) {
            loop_intervention = Some(
                "HTTP PARSE NOTICE: Use the `http_parse_tools` tool to parse raw HTTP/1.1 request and response messages without external utilities. \
                 Actions: parse/auto (default — auto-detect and parse), \
                 request (force-parse as HTTP request — method, path, query params, headers, body), \
                 response (force-parse as HTTP response — status code, headers, content analysis), \
                 headers (all headers with annotations and security header check), \
                 cookies (parse Cookie: and Set-Cookie: with security flag analysis), \
                 auth (analyze Authorization:, WWW-Authenticate:, and API key headers). \
                 Input: 'text'/'http'/'message' for inline HTTP text, or 'file' for a file path. \
                 Example: http_parse_tools(action: 'parse', text: 'GET / HTTP/1.1\\nHost: example.com') or \
                 http_parse_tools(action: 'auth', text: 'Authorization: Bearer eyJ...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_jq_tools(&effective_user_input) {
            loop_intervention = Some(
                "JQ TOOLS NOTICE: Use the `jq_tools` tool for jq-style JSON path queries and filters without external utilities. \
                 Actions: query (default — evaluate a path expression; pass 'path' or 'q'), \
                 keys (list object keys or array indices at 'path'), \
                 values (list object values or array elements at 'path'), \
                 flatten (flatten nested arrays; optional 'depth'), \
                 map (extract a 'field' from each element of an array), \
                 filter (keep array elements where 'field' equals 'value', 'contains', 'gt', 'lt', or 'exists'), \
                 count (count elements/keys at 'path'), \
                 type (show JSON type and size at 'path'). \
                 Path syntax: '.' identity, '.field', '.a.b[0]', '.items[]' iterate, '.a, .b' multi-path, '.arr | sort' pipe builtins. \
                 Input: 'json' (inline JSON) or 'file' (path to JSON file). \
                 Example: jq_tools(action: 'query', json: '[...]', path: '.[0].name') or \
                 jq_tools(action: 'filter', file: 'data.json', field: 'age', gt: 30) or \
                 jq_tools(action: 'query', json: '...', path: '.items | sort')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_glob_tools(&effective_user_input) {
            loop_intervention = Some(
                "GLOB NOTICE: Use the `glob_tools` tool to test, filter, explain, and convert glob patterns without external utilities. \
                 Actions: match (test if a single path matches a pattern; pass 'pattern' and 'path'), \
                 filter (filter a list of paths; pass 'pattern' and 'paths' as array or newline string), \
                 explain (tokenize and describe each component of a pattern; pass 'pattern'), \
                 convert (show the equivalent regex; pass 'pattern'). \
                 Glob syntax: ** matches any depth, * matches one segment, ? matches one char, [!abc] negates. \
                 Example: glob_tools(action: 'match', pattern: '**/*.rs', path: 'src/tools/mod.rs') or \
                 glob_tools(action: 'explain', pattern: 'src/**/*.{ts,tsx}')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_graph_tools(&effective_user_input) {
            loop_intervention = Some(
                "GRAPH TOOLS NOTICE: Use the `graph_tools` tool for graph algorithm operations without external utilities. \
                 Actions: info (default — graph summary: node/edge counts, density, degree distribution; pass 'nodes' array and 'edges' array), \
                 bfs (breadth-first search from a start node; pass 'start'), \
                 dfs (depth-first search from a start node; pass 'start'), \
                 shortest (Dijkstra's shortest path; pass 'start' and 'end'), \
                 topo (topological sort via Kahn's algorithm; detects cycles), \
                 cycles (detect cycles; works on directed and undirected graphs), \
                 components (connected components for undirected; SCCs via Kosaraju's for directed). \
                 Pass 'directed: true' for directed graphs (default: undirected). \
                 Edges format: array of objects {from, to, weight?} or arrays [from, to, weight?]. \
                 Example: graph_tools(action: 'shortest', nodes: ['A','B','C','D'], edges: [{from:'A',to:'B',weight:2},{from:'B',to:'D',weight:3},{from:'A',to:'C',weight:1},{from:'C',to:'D',weight:5}], start: 'A', end: 'D') or \
                 graph_tools(action: 'topo', nodes: ['build','test','deploy'], edges: [{from:'build',to:'test'},{from:'test',to:'deploy'}], directed: true)."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_matrix_tools(&effective_user_input) {
            loop_intervention = Some(
                "MATRIX TOOLS NOTICE: Use the `matrix_tools` tool for linear algebra operations without external utilities. \
                 Actions: info (default — shape, rank, trace, determinant, min/max/mean; pass 'matrix'), \
                 multiply (matrix multiplication; pass 'a' and 'b' as 2D arrays), \
                 transpose (flip rows and columns; pass 'matrix'), \
                 determinant (det(A); pass 'matrix' — must be square), \
                 inverse (A⁻¹; pass 'matrix' — must be square and invertible), \
                 solve (solve Ax=b via Gaussian elimination; pass 'matrix' for A and 'vector' for b), \
                 rank (matrix rank via row reduction; pass 'matrix'). \
                 Matrix format: JSON array of arrays e.g. [[1,2],[3,4]]. \
                 Example: matrix_tools(action: 'solve', matrix: [[2,1,-1],[−3,−1,2],[−2,1,2]], vector: [8,-11,-3]) or \
                 matrix_tools(action: 'determinant', matrix: [[1,2,3],[4,5,6],[7,8,9]])."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_har_tools(&effective_user_input) {
            loop_intervention = Some(
                "HAR TOOLS NOTICE: Use the `har_tools` tool to parse and analyze HTTP Archive (.har) files without external utilities. \
                 Actions: summary (default — entry count, domains, errors, total time/size, status distribution, MIME types), \
                 entries (tabular list of all requests; pass 'limit' to cap rows, default 25), \
                 slowest (top N slowest requests with timing breakdown DNS/connect/SSL/send/wait/receive; pass 'n', default 10), \
                 errors (filter 4xx/5xx/network-error entries only), \
                 domains (per-domain request count, total time, total size — sorted by slowest), \
                 search (filter entries by URL substring; pass 'query' or 'q'). \
                 Input: pass 'har' with a parsed HAR JSON object, 'json'/'text' with a HAR JSON string, or 'file' with a path to a .har file. \
                 Example: har_tools(action: 'slowest', file: 'network.har', n: 5) or \
                 har_tools(action: 'errors', file: 'network.har') or \
                 har_tools(action: 'search', file: 'network.har', query: 'api/v2')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_ical_tools(&effective_user_input) {
            loop_intervention = Some(
                "ICAL TOOLS NOTICE: Use the `ical_tools` tool to parse and inspect iCalendar (.ics) files without external utilities. \
                 Actions: parse (default — list all VEVENT and VTODO components with title, dates, location, status), \
                 events (same as parse but scoped to VEVENT only), \
                 todos (list VTODO items with due date, status, and priority), \
                 info (calendar-level metadata: iCal version, producer, calendar name, timezone, component counts), \
                 search (filter events/todos by keyword; pass 'query' or 'q'). \
                 Input: pass 'text'/'ical'/'ics' with iCalendar text content, or 'file' with a path to a .ics file. \
                 Example: ical_tools(action: 'parse', file: 'calendar.ics') or \
                 ical_tools(action: 'search', file: 'calendar.ics', query: 'sprint') or \
                 ical_tools(action: 'info', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_graphviz_tools(&effective_user_input) {
            loop_intervention = Some(
                "GRAPHVIZ TOOLS NOTICE: Use the `graphviz_tools` tool to generate and parse DOT language graph descriptions without external utilities. \
                 Actions: generate (default — generate DOT output from 'nodes' array and 'edges' array; pass 'directed: true' for digraph; 'name', 'rankdir' options), \
                 parse (parse DOT source and extract node/edge lists; pass 'dot' or 'text' with the DOT source), \
                 flowchart (generate a sequential flowchart from a 'steps' array with optional 'start'/'end' labels), \
                 tree (generate a tree DOT from 'root' + 'children' array of {label, children?} objects or strings). \
                 Nodes: array of strings or {id, label} objects. \
                 Edges: array of {from, to, label?} objects or [from, to, label?] arrays. \
                 Output includes the DOT source and render commands (dot -Tpng, dot -Tsvg). \
                 Example: graphviz_tools(action: 'generate', directed: true, nodes: ['A','B','C'], edges: [{from:'A',to:'B'},{from:'B',to:'C'}]) or \
                 graphviz_tools(action: 'flowchart', steps: ['Input','Validate','Process','Output'])."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_mermaid_tools(&effective_user_input) {
            loop_intervention = Some(
                "MERMAID TOOLS NOTICE: Use the `mermaid_tools` tool to generate Mermaid.js diagram syntax without external utilities. \
                 Actions: flowchart (default — flowchart from 'nodes'+'edges' or 'steps'; 'direction': TD/LR/RL/BT), \
                 sequence (sequence diagram from 'messages': [{from, to, text, type?}]; types: sync/async/lost), \
                 class (class diagram from 'classes': [{name, fields?, methods?, relationships?}]), \
                 gantt (Gantt chart from 'sections': [{name, tasks: [{name, start, duration, status?}]}]; 'title', 'date_format'), \
                 pie (pie chart from 'data': {\"label\": value} object; 'title'), \
                 er (ER diagram from 'entities': [{name, attributes?}] and 'relationships': [{left, right, cardinality, label}]). \
                 Output is a fenced ```mermaid code block ready to paste into GitHub, GitLab, Notion, Obsidian, or mermaid.live. \
                 Example: mermaid_tools(action: 'flowchart', steps: ['Start','Process','End']) or \
                 mermaid_tools(action: 'sequence', messages: [{from:'Client',to:'Server',text:'GET /api'},{from:'Server',to:'Client',text:'200 OK',type:'async'}]) or \
                 mermaid_tools(action: 'pie', title: 'Traffic', data: {\"API\": 45, \"Web\": 35, \"Mobile\": 20})."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_log_parse_tools(&effective_user_input) {
            loop_intervention = Some(
                "LOG PARSE NOTICE: Use the `log_parse_tools` tool to parse and analyze structured log lines without external utilities. \
                 Actions: parse (default — auto-detect format and parse fields from each line; pass 'text'), \
                 detect (identify the log format — JSON Lines / key=value / Apache / Syslog; pass 'text'), \
                 filter (keep only lines where a field matches a value; pass 'text', 'field', 'value'), \
                 stats (aggregate counts by a field; pass 'text' and optional 'field'). \
                 Supported formats: JSON Lines, key=value, Apache Common/Combined, Syslog. \
                 Pass 'format' to override auto-detection: json, kv, apache, combined, syslog. \
                 Example: log_parse_tools(action: 'parse', text: '...') or \
                 log_parse_tools(action: 'filter', text: '...', field: 'status', value: '500')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_csp_tools(&effective_user_input) {
            loop_intervention = Some(
                "CSP NOTICE: Use the `csp_tools` tool to parse, explain, validate, and build Content Security Policy headers without external utilities. \
                 Actions: parse (default — break header into directives with source descriptions; pass 'header'), \
                 explain (plain-English summary of what each directive permits; pass 'header'), \
                 validate (check for unsafe sources, missing directives, deprecated syntax; pass 'header'), \
                 build (generate a CSP from a directives object or preset — 'preset': strict/moderate/api). \
                 Pass 'header' with the raw CSP value (strips 'Content-Security-Policy:' prefix automatically). \
                 Example: csp_tools(action: 'parse', header: \"default-src 'self'; script-src 'nonce-xyz'\") or \
                 csp_tools(action: 'build', preset: 'strict')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_robots_txt_tools(&effective_user_input) {
            loop_intervention = Some(
                "ROBOTS.TXT NOTICE: Use the `robots_txt_tools` tool to parse, check, validate, and summarize robots.txt files without external utilities. \
                 Actions: parse (default — show all user-agent blocks with Allow/Disallow rules; pass 'text'), \
                 check (test whether a specific path is allowed or blocked; pass 'text', 'url' or 'path', optional 'agent' for user-agent; defaults to '*'), \
                 validate (check for unknown directives, paths without leading slash, missing wildcard block, Disallow: /), \
                 summary (table view of all blocks with rule counts and crawl-delay). \
                 Example: robots_txt_tools(action: 'check', text: '...', path: '/admin/', agent: 'Googlebot') or \
                 robots_txt_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_sitemap_tools(&effective_user_input) {
            loop_intervention = Some(
                "SITEMAP NOTICE: Use the `sitemap_tools` tool to parse, search, and analyze sitemap.xml files without external utilities. \
                 Actions: parse (default — list all URLs with lastmod/changefreq/priority; 'max' to limit shown, default 20), \
                 search (filter URLs containing a query string; pass 'query'), \
                 stats (URL count, lastmod/changefreq/priority coverage, distribution), \
                 list (all URLs or filtered by prefix; pass optional 'filter'). \
                 Handles both urlset (standard sitemap) and sitemapindex (sitemap of sitemaps). \
                 Example: sitemap_tools(action: 'parse', xml: '...') or \
                 sitemap_tools(action: 'search', xml: '...', query: '/blog/') or \
                 sitemap_tools(action: 'stats', xml: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_github_actions_tools(&effective_user_input) {
            loop_intervention = Some(
                "GITHUB ACTIONS NOTICE: Use the `github_actions_tools` tool to parse, inspect, and validate GitHub Actions workflow YAML. \
                 Pass the workflow YAML content as 'text'. \
                 Actions: info (default — workflow name, triggers, and per-job summary), \
                 jobs (detailed job listing with runs-on, step count, needs, matrix, and env vars), \
                 steps (all steps per job with uses/run/if; optional 'job' filter for a specific job), \
                 triggers (full trigger detail including branches/tags/paths filters, cron schedules, workflow_dispatch inputs), \
                 validate (checks: missing 'on' triggers, missing runs-on, undefined needs references, steps without uses/run, missing top-level permissions). \
                 Example: github_actions_tools(action: 'info', text: '...') or \
                 github_actions_tools(action: 'steps', text: '...', job: 'build') or \
                 github_actions_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_gitignore_tools(&effective_user_input) {
            loop_intervention = Some(
                "GITIGNORE NOTICE: Use the `gitignore_tools` tool to parse, check, generate, and explain .gitignore files. \
                 Actions: parse (default — list all patterns grouped by comment sections with counts), \
                 check (test if a file path is IGNORED or NOT IGNORED; pass 'path'), \
                 generate (produce a standard .gitignore for a language; pass 'language': rust/node/python/go/java/dotnet/react/docker), \
                 explain (plain-English description of each pattern — scope, glob semantics, negation). \
                 Example: gitignore_tools(action: 'check', text: '...', path: 'dist/bundle.js') or \
                 gitignore_tools(action: 'generate', language: 'rust')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_license_tools(&effective_user_input) {
            loop_intervention = Some(
                "LICENSE NOTICE: Use the `license_tools` tool to look up, detect, compare, and list software licenses. \
                 Actions: info (default — full details for a named license: SPDX ID, category, copyleft, patent grant, conditions, permissions, limitations; pass 'license'), \
                 detect (identify the license from file text; pass 'text'), \
                 compare (side-by-side comparison of two licenses; pass 'a' and 'b'), \
                 list (all 14 supported licenses grouped by category; optional 'category' filter). \
                 Supports: MIT, Apache-2.0, GPL-2.0, GPL-3.0, LGPL-2.1, LGPL-3.0, MPL-2.0, AGPL-3.0, BSD-2-Clause, BSD-3-Clause, ISC, Unlicense, CC0-1.0, EUPL-1.2. \
                 Example: license_tools(action: 'info', license: 'MIT') or \
                 license_tools(action: 'compare', a: 'MIT', b: 'GPL-3.0') or \
                 license_tools(action: 'detect', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_make_tools(&effective_user_input) {
            loop_intervention = Some(
                "MAKEFILE NOTICE: Use the `make_tools` tool to parse, inspect, and analyze Makefiles without external utilities. \
                 Actions: list (default — all targets with deps and phony flag), \
                 explain (full detail for one target — dependencies and commands; pass 'target'), \
                 deps (dependency graph for all targets or a specific target; pass optional 'target'), \
                 vars (all variable assignments with operator and value). \
                 Example: make_tools(action: 'list', text: '...') or \
                 make_tools(action: 'explain', text: '...', target: 'build') or \
                 make_tools(action: 'vars', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_changelog_tools(&effective_user_input) {
            loop_intervention = Some(
                "CHANGELOG NOTICE: Use the `changelog_tools` tool to parse, query, and validate CHANGELOG.md files (Keep a Changelog format). \
                 Actions: list (default — all releases with version/date/section counts), \
                 get (full body of a specific version; pass 'version'), \
                 latest (full body of the most recent non-Unreleased release), \
                 validate (check Keep a Changelog compliance — Unreleased section, dates, standard section names, YANKED releases). \
                 Example: changelog_tools(action: 'list', text: '...') or \
                 changelog_tools(action: 'get', text: '...', version: '1.2.0') or \
                 changelog_tools(action: 'latest', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_ssh_config_tools(&effective_user_input) {
            loop_intervention = Some(
                "SSH CONFIG NOTICE: Use the `ssh_config_tools` tool to parse, query, explain, and validate ~/.ssh/config files. \
                 Actions: list (default — summary of all host blocks with HostName/User/Port/IdentityFile/ProxyJump), \
                 get (all options for a specific host; pass 'host'), \
                 explain (plain-English explanation of every option; optional 'host' filter), \
                 validate (check for duplicate patterns, StrictHostKeyChecking=no warnings, relative IdentityFile paths). \
                 Pass the config content as 'text'. \
                 Example: ssh_config_tools(action: 'list', text: '...') or \
                 ssh_config_tools(action: 'get', text: '...', host: 'myserver') or \
                 ssh_config_tools(action: 'explain', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_systemd_tools(&effective_user_input) {
            loop_intervention = Some(
                "SYSTEMD UNIT NOTICE: Use the `systemd_tools` tool to parse, inspect, and validate systemd unit files (.service/.timer/.socket). \
                 Pass the unit file content as 'text'. \
                 Actions: info (default — unit type, description, [Unit]/[Service]/[Timer]/[Socket]/[Install] summary), \
                 service (detailed [Service] section breakdown: exec commands, identity, restart policy, environment, security hardening), \
                 timer (timer triggers with human-readable schedule explanations for OnCalendar/OnBootSec/OnUnitActiveSec, Persistent flag), \
                 validate (warn on missing Description, missing ExecStart, Type=forking without PIDFile, no Restart=, running as root, missing security directives, missing [Install] section). \
                 Example: systemd_tools(action: 'info', text: '...') or \
                 systemd_tools(action: 'service', text: '...') or \
                 systemd_tools(action: 'timer', text: '...') or \
                 systemd_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_nginx_conf_tools(&effective_user_input) {
            loop_intervention = Some(
                "NGINX CONFIG NOTICE: Use the `nginx_conf_tools` tool to parse, inspect, and validate nginx.conf files. \
                 Actions: list (default — all server blocks with server_name, listen, root/proxy, SSL state, location count), \
                 inspect (full detail for one server including all directives and location blocks; pass 'server' as server_name or index), \
                 locations (all location blocks with proxy_pass/root/alias targets; optional 'server' filter), \
                 directives (global and http-block directives plus upstream definitions), \
                 validate (warn on missing server_name, SSL without certificate, proxy without Host header, multiple default servers). \
                 Pass the config file content as 'text'. \
                 Example: nginx_conf_tools(action: 'list', text: '...') or \
                 nginx_conf_tools(action: 'inspect', text: '...', server: 'example.com') or \
                 nginx_conf_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_openapi_tools(&effective_user_input) {
            loop_intervention = Some(
                "OPENAPI NOTICE: Use the `openapi_tools` tool to parse, query, search, and validate OpenAPI 3.x / Swagger 2.x specs. \
                 Actions: info (default — title, version, description, servers, endpoint/schema counts, tags, auth schemes), \
                 endpoints (list all paths+methods with summary, operationId, tags, deprecated flag; pass 'tag' to filter), \
                 schemas (list schema/definition names with types, properties, required flags; pass 'schema' to filter), \
                 search (filter endpoints by path, summary, operationId, tag, or HTTP method; pass 'query'), \
                 validate (check for missing info section, no endpoints, missing summaries/operationIds, duplicate operationIds, deprecated endpoints). \
                 Pass the spec content (YAML or JSON) as 'text'. \
                 Example: openapi_tools(action: 'info', text: '...') or \
                 openapi_tools(action: 'endpoints', text: '...', tag: 'users') or \
                 openapi_tools(action: 'search', text: '...', query: 'POST') or \
                 openapi_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_terraform_tools(&effective_user_input) {
            loop_intervention = Some(
                "TERRAFORM NOTICE: Use the `terraform_tools` tool to parse, inspect, and validate Terraform HCL files. \
                 Pass the HCL content as 'text'. \
                 Actions: info (default — required_version, provider list with source/version, block counts for resource/data/module/variable/output/local), \
                 resources (list all resource blocks with type, name, and key attributes like ami/instance_type/name/location), \
                 variables (list all variable blocks with type, description, default or '(required)', SENSITIVE flag), \
                 outputs (list all output blocks with value expression and SENSITIVE flag), \
                 validate (warn on missing required_version, permissive provider versions, hardcoded credentials, sensitive outputs/variables without sensitive=true). \
                 Example: terraform_tools(action: 'info', text: '...') or \
                 terraform_tools(action: 'resources', text: '...') or \
                 terraform_tools(action: 'variables', text: '...') or \
                 terraform_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_package_json_tools(&effective_user_input) {
            loop_intervention = Some(
                "PACKAGE.JSON NOTICE: Use the `package_json_tools` tool to parse, inspect, and validate package.json files. \
                 Pass the JSON content as 'text'. \
                 Actions: info (default — name, version, description, license, author, main/module/types, engine requirements, script/dep/devDep counts, keywords, repository), \
                 scripts (list all npm scripts with their command strings; pass 'filter' to narrow), \
                 deps (list dependencies by section: prod/dev/peer/optional with version ranges and URL-dep/wildcard flags; pass 'kind' to filter), \
                 validate (check for missing name/version/description/license, no engines field, wildcard dep versions, http:// deps, missing test/build scripts, no files whitelist, duplicate deps). \
                 Example: package_json_tools(action: 'info', text: '...') or \
                 package_json_tools(action: 'scripts', text: '...') or \
                 package_json_tools(action: 'deps', text: '...', kind: 'dev') or \
                 package_json_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_sql_tools(&effective_user_input) {
            loop_intervention = Some(
                "SQL NOTICE: Use the `sql_tools` tool to parse, explain, and validate SQL statements without external utilities. \
                 Pass the SQL content as 'text' (also 'sql'/'query'/'content'/'input'). \
                 Actions: parse (default — count statements by type, list each with referenced tables, join count, and subquery flag), \
                 tables (extract CREATE TABLE definitions with column names, types, NOT NULL/PK/FK flags, and foreign key relationships), \
                 explain (plain-English description of what each statement does — reads, writes, joins, filters, ordering), \
                 validate (warn on SELECT *, DELETE/UPDATE without WHERE, DROP TABLE without IF EXISTS, implicit cross joins, NOT IN with NULL risk, LIKE with leading wildcard, CREATE TABLE missing PK). \
                 Example: sql_tools(action: 'parse', text: '...') or \
                 sql_tools(action: 'tables', text: '...') or \
                 sql_tools(action: 'explain', text: '...') or \
                 sql_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_proto_tools(&effective_user_input) {
            loop_intervention = Some(
                "PROTOBUF NOTICE: Use the `proto_tools` tool to parse, inspect, and validate Protocol Buffer (.proto) files. \
                 Pass the .proto content as 'text' (also 'proto'/'content'/'input'). \
                 Actions: info (default — syntax version, package, imports, file options, message/enum/service counts), \
                 messages (detailed message and enum listing with field names, types, field numbers, labels, and inline options), \
                 services (all service definitions with RPC method signatures, streaming flags, and unary/streaming classification), \
                 validate (check: unrecognised syntax, missing package, empty messages, duplicate field numbers, field number 0 or in reserved range 19000-19999, proto2 required fields, proto3 enum first value not 0, empty services). \
                 Example: proto_tools(action: 'info', text: '...') or \
                 proto_tools(action: 'messages', text: '...') or \
                 proto_tools(action: 'services', text: '...') or \
                 proto_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_pem_tools(&effective_user_input) {
            loop_intervention = Some(
                "PEM CERTIFICATE NOTICE: Use the `pem_tools` tool to inspect, decode, and validate PEM-encoded \
                 certificates, certificate chains, and private keys without external utilities. \
                 Pass the PEM content as 'text' (also 'pem'/'content'/'input'). \
                 Actions: info (default — per-block type, certificate subject/issuer/validity/SANs/key info and expiry countdown), \
                 chain (ordered chain display with issuer→subject linkage verification and chain completeness check), \
                 validate (check: expired certs, expiring within 30 days, self-signed leaf, SHA-1/MD5 signatures, \
                 RSA < 2048 bits, missing SANs on leaf v3 cert, private key bundled with cert, chain out of order). \
                 Example: pem_tools(action: 'info', text: '...') or \
                 pem_tools(action: 'chain', text: '...') or \
                 pem_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_env_schema_tools(&effective_user_input) {
            loop_intervention = Some(
                "ENV SCHEMA NOTICE: Use the `env_schema_tools` tool to validate a .env file against a .env.example \
                 schema — check for missing keys, extra keys, and empty required values. \
                 Actions: validate (default — compare .env against .env.example, report VALID/INVALID with per-key findings), \
                 diff (keys present in .env.example but absent from .env), \
                 required (list which keys in .env.example are required vs optional with default placeholders), \
                 info (overview of both files — key count, coverage percentage, optional vs required breakdown). \
                 Pass 'example' (.env.example content) and 'env' (.env content); or 'example_file'/'env_file' for file paths. \
                 Example: env_schema_tools(action: 'validate', example: '...', env: '...') or \
                 env_schema_tools(action: 'diff', example: '...', env: '...') or \
                 env_schema_tools(action: 'required', example: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_graphql_tools(&effective_user_input) {
            loop_intervention = Some(
                "GRAPHQL NOTICE: Use the `graphql_tools` tool to parse, inspect, and validate GraphQL schemas and \
                 query documents without external utilities. Pass content as 'text' (also 'schema'/'query'/'graphql'/'content'/'input'). \
                 Actions: info (default — document kind, type/interface/input/enum/union/scalar counts, operations, schema entry points), \
                 types (list all type definitions with fields and args; optional 'filter' by name), \
                 queries (list all operations and fragments with top-level field names), \
                 validate (checks: missing Query root, empty types/interfaces/enums/unions, input fields using output types, \
                 fields referencing undefined types, duplicate type names, operations selecting no fields). \
                 Example: graphql_tools(action: 'info', text: '...') or \
                 graphql_tools(action: 'types', text: '...') or \
                 graphql_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_sql_migrate_tools(&effective_user_input) {
            loop_intervention = Some(
                "SQL MIGRATION NOTICE: Use the `sql_migrate_tools` tool to analyze, risk-score, and validate SQL \
                 migration files without external utilities. Pass content as 'text' (also 'sql'/'migration'/'content'/'input'). \
                 Actions: analyze (default — per-statement risk rating SAFE/LOW/MEDIUM/HIGH/CRITICAL with actionable notes), \
                 risk (show only medium/high/critical risk operations), \
                 ops (operation type summary — what kinds of statements the migration contains), \
                 validate (transaction wrapping check, destructive operations summary, concurrent index in transaction detection). \
                 Example: sql_migrate_tools(action: 'analyze', text: '...') or \
                 sql_migrate_tools(action: 'risk', text: '...') or \
                 sql_migrate_tools(action: 'validate', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_base_tools(&effective_user_input) {
            loop_intervention = Some(
                "BASE ENCODING NOTICE: Use the `base_tools` tool for extended base encoding and decoding \
                 without external utilities. Actions: encode (default — show all encodings or specify 'encoding': \
                 base16/base32/base58/base85; pass 'input'), \
                 decode (convert encoded string back to bytes; requires 'encoding': base16/base32/base58/base85; \
                 shows UTF-8 and hex representations), \
                 identify (guess the encoding of a string from character set and length heuristics). \
                 Example: base_tools(action: 'encode', input: 'hello world', encoding: 'base58') or \
                 base_tools(action: 'decode', input: 'StV1DL6CwTryKyV', encoding: 'base58')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_lock_file_tools(&effective_user_input) {
            loop_intervention = Some(
                "LOCK FILE NOTICE: Use the `lock_file_tools` tool to parse and analyze dependency lock files \
                 without external utilities. Supports Cargo.lock (cargo), package-lock.json (npm), yarn.lock (yarn), \
                 and poetry.lock (poetry). Pass content as 'text' or a file path as 'file'; optionally pass 'format' \
                 to override auto-detection. Actions: info (default — package count, duplicates, format metadata), \
                 list (all packages sorted by name; pass 'limit' to cap), \
                 search (find packages by name substring; pass 'query'), \
                 duplicates (packages appearing at multiple versions — the root cause of bundle bloat). \
                 Example: lock_file_tools(action: 'info', file: 'Cargo.lock') or \
                 lock_file_tools(action: 'duplicates', file: 'package-lock.json')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_binary_tools(&effective_user_input) {
            loop_intervention = Some(
                "BINARY TOOLS NOTICE: Use the `binary_tools` tool for bit manipulation and bitfield analysis \
                 without external utilities. Actions: info (default — decimal/hex/octal/binary, popcount, parity, \
                 Gray code, leading/trailing zeros, IEEE 754 float view; pass 'value' as integer or 0x/0b/0o string, \
                 optional 'width' in bits), \
                 flags (show each bit position with SET/clear state; optional 'names' array for named flags), \
                 pack (assemble fields into a packed integer; 'fields' array of {value, bits, name?} objects, \
                 MSB first), \
                 unpack (extract fields from a packed integer; 'value' + 'layout' array of {bits, name?} objects), \
                 ops (compute AND/OR/XOR/NOT/NAND/NOR/XNOR, shifts, rotates, popcount, Gray code, mask/set/clear/toggle; \
                 'value' for A, optional 'b' for B, optional 'shift' count). \
                 Example: binary_tools(action: 'flags', value: '0xFF', names: ['bit7','bit6','bit5','bit4','bit3','bit2','bit1','bit0'])"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_ascii_tools(&effective_user_input) {
            loop_intervention = Some(
                "ASCII TOOLS NOTICE: Use the `ascii_tools` tool to generate ASCII art, boxes, bars, tables, \
                 and trees without external utilities. Actions: \
                 banner (default — block-letter ASCII art from text; pass 'text', max 30 chars), \
                 box (draw a Unicode box around text lines; 'text', optional 'style': single/double/rounded/heavy/ascii, \
                 optional 'padding'), \
                 bar (render a progress/fill bar; 'value', optional 'max' default 100, 'width', \
                 'style': block/hash/equals/shade/circle/dot, 'label'), \
                 table (render a formatted table; 'headers' string array, 'rows' 2D array, \
                 optional 'style': single/double/heavy/rounded), \
                 tree (render a directory-style tree; pass 'root' + 'nodes' array of {label, children?} objects, \
                 or 'text' as an indented outline with 'root' as root label). \
                 Example: ascii_tools(action: 'box', text: 'Hello!\\nWorld', style: 'double')"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_time_zone_tools(&effective_user_input) {
            loop_intervention = Some(
                "TIME ZONE TOOLS NOTICE: Use the `time_zone_tools` tool for timezone conversions and world clock \
                 queries without external utilities. Actions: \
                 convert (default — convert a datetime from one timezone to another; 'datetime' as ISO 8601 or \
                 'YYYY-MM-DD HH:MM:SS', 'from' source timezone, 'to' target timezone; e.g. \
                 time_zone_tools(action:'convert', datetime:'2024-06-15 14:30:00', from:'EST', to:'JST')), \
                 list (list all supported timezone names and their UTC offsets; optional 'filter' to search), \
                 offset (get UTC offset for a timezone name; 'tz' field), \
                 world_clock (show current time in multiple timezones; 'zones' array of timezone names). \
                 Accepts named zones (UTC, EST, PST, GMT, IST, JST, etc.) and numeric offsets (+05:30, -07:00). \
                 Example: time_zone_tools(action:'world_clock', zones:['UTC','EST','PST','IST','JST'])"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_word_tools(&effective_user_input) {
            loop_intervention = Some(
                "WORD TOOLS NOTICE: Use the `word_tools` tool for word frequency analysis, anagram detection, \
                 phonetic matching, and syllable counting without external utilities. Actions: \
                 frequency (default — word frequency table with percentages; 'text', optional 'top' N, \
                 'stop_words' bool to filter common words), \
                 anagram (check if two words/phrases are anagrams; 'a' and 'b' fields), \
                 soundex (Soundex phonetic code for a word or list; 'word', 'words' array, or 'text'; \
                 groups phonetically similar words), \
                 palindrome (check if text is a palindrome; 'text', optional 'strict' bool; finds longest \
                 palindromic substring), \
                 syllables (syllable count per word with Flesch-Kincaid grade estimate; 'text'). \
                 Example: word_tools(action:'anagram', a:'listen', b:'silent')"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_string_metric_tools(&effective_user_input) {
            loop_intervention = Some(
                "STRING METRIC TOOLS NOTICE: Use the `string_metric_tools` tool for string distance and similarity \
                 calculations without external utilities. Actions: \
                 levenshtein (default — edit distance between 'a' and 'b'; optional 'case_sensitive' bool), \
                 damerau (Damerau-Levenshtein with transposition support; 'a' and 'b'), \
                 jaro (Jaro similarity score; 'a' and 'b'), \
                 jaro_winkler (Jaro-Winkler with prefix boost — good for name matching; 'a' and 'b'), \
                 hamming (Hamming distance for equal-length strings; 'a' and 'b'), \
                 lcs (Longest Common Subsequence length and string; 'a' and 'b'), \
                 similarity (all metrics in one table with average; 'a' and 'b'), \
                 fuzzy (rank 'candidates' list by similarity to 'query'; optional 'threshold' 0.0-1.0, 'top' N). \
                 Example: string_metric_tools(action:'similarity', a:'Robert', b:'Rupert') or \
                 string_metric_tools(action:'fuzzy', query:'helo', candidates:['hello','world','help'])"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_calc_tools(&effective_user_input) {
            loop_intervention = Some(
                "CALC TOOLS NOTICE: Use the `calc_tools` tool to evaluate mathematical expressions and generate \
                 sequences without needing run_code. Actions: \
                 eval (default — evaluate an infix expression; 'expr' field; supports +/-/*/÷/^/% operators, \
                 parentheses, constants pi/e/tau/phi, and functions: sqrt, abs, sin, cos, tan, log, ln, exp, \
                 floor, ceil, round, factorial, gcd, lcm, min, max, avg, sum, pow, clamp, choose; \
                 optional 'vars' object for variable substitution), \
                 rpn (Reverse Polish Notation calculator; 'expr' as space-separated tokens; supports +/-/*/^/%/sqrt/abs/neg/dup/swap/drop), \
                 variables (multi-statement session with assignments; 'statements' array like ['x=5', 'y=x*2', 'z=x+y']), \
                 sequence (generate N terms of a numeric sequence; 'expr' using 'n' as index; 'start'/'step'/'count'). \
                 Example: calc_tools(expr:'2^10 + factorial(5)') or \
                 calc_tools(action:'sequence', expr:'n^2 + 1', count:10)"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_cipher_tools(&effective_user_input) {
            loop_intervention = Some(
                "CIPHER TOOLS NOTICE: Use the `cipher_tools` tool for classical cipher encoding, decoding, and \
                 frequency analysis without external utilities. Actions: \
                 rot13 (default — ROT13 encode/decode; 'text'), \
                 caesar (shift cipher; 'text', 'shift' N, optional 'decode: true'; shows brute-force table for short texts), \
                 vigenere (key-based polyalphabetic cipher; 'text', 'key', optional 'decode: true'), \
                 atbash (A↔Z reversal; 'text'), \
                 rail_fence (transposition cipher; 'text', 'rails' N (default 3), optional 'decode: true'; shows rail diagram), \
                 analyze (letter frequency, Index of Coincidence, Caesar break attempt; 'text'). \
                 Example: cipher_tools(action:'caesar', text:'HELLO', shift:13) or \
                 cipher_tools(action:'analyze', text:'KHOOR ZRUOG')"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_nato_tools(&effective_user_input) {
            loop_intervention = Some(
                "NATO/MORSE TOOLS NOTICE: Use the `nato_tools` tool for NATO phonetic alphabet and Morse code \
                 encoding/decoding without external utilities. Actions: \
                 nato (default — text → NATO phonetic words; 'text'; e.g. \"H\" → \"Hotel\"), \
                 from_nato (NATO words → text; 'text' containing NATO words like \"Alpha Bravo\"), \
                 morse ('text' → Morse code using dots/dashes/slashes; auto-detects encode vs decode; \
                 or pass 'decode: true' to force decode mode), \
                 spell (spell out text with NATO and character labels; 'text', optional 'separator'). \
                 Example: nato_tools(action:'nato', text:'SOS') or \
                 nato_tools(action:'morse', text:'... --- ...')"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_geo_tools(&effective_user_input) {
            loop_intervention = Some(
                "GEO TOOLS NOTICE: Use the `geo_tools` tool for geographic coordinate calculations \
                 without external utilities. Actions: \
                 distance (default — Haversine great-circle distance between two lat/lng points; \
                 'lat1', 'lng1', 'lat2', 'lng2'; returns km/miles/nautical miles and bearing), \
                 bearing ('lat1','lng1','lat2','lng2' — initial and back bearing in degrees + compass point), \
                 midpoint (geographic centroid; 'lat1'/'lng1'/'lat2'/'lng2' or 'points' array of [lat,lng] pairs), \
                 dms (decimal ↔ DMS conversion; decimal mode: 'lat'+'lng'; DMS mode: 'lat_d'/'lat_m'/'lat_s'/'lat_dir' + 'lng_d'/'lng_m'/'lng_s'/'lng_dir'), \
                 bbox (bounding box for a set of points; 'points' array → N/S/E/W bounds, center, width/height km), \
                 destination (project a point at distance+bearing; 'lat','lng','distance' km,'bearing' degrees). \
                 Example: geo_tools(action:'distance', lat1:51.5074, lng1:-0.1278, lat2:48.8566, lng2:2.3522)"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_data_gen_tools(&effective_user_input) {
            loop_intervention = Some(
                "DATA GEN TOOLS NOTICE: Use the `data_gen_tools` tool to generate test/mock data \
                 without external utilities. Actions: \
                 lorem (default — Lorem ipsum text; 'count' and 'unit': words/sentences/paragraphs; optional 'seed'), \
                 name (random person names; 'count'; optional 'seed'), \
                 email (random emails; 'count'; optional 'domain' to fix the domain; optional 'seed'), \
                 numbers (random numbers in range; 'count', 'min', 'max'; 'float: true' for decimals; optional 'separator'), \
                 dates (random dates in range; 'count', 'from' YYYY-MM-DD, 'to' YYYY-MM-DD; 'format': iso/us/eu/long), \
                 id (sequential or random IDs; 'count', 'kind': seq/hex/uuid; 'prefix', 'start', 'pad' for seq mode). \
                 Example: data_gen_tools(action:'lorem', count:3, unit:'paragraphs') or \
                 data_gen_tools(action:'name', count:10)"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_unit_tools(&effective_user_input) {
            loop_intervention = Some(
                "UNIT TOOLS NOTICE: Use the `unit_tools` tool for unit conversion without external utilities. \
                 Actions: \
                 convert (default — convert a value between units; 'value' (number), 'from' (unit name/symbol), \
                 optional 'to' (target unit) — omit 'to' to show all conversions in the same category), \
                 list (show all supported units; optional 'category' to filter), \
                 categories (show all 13 categories with unit counts). \
                 13 categories: length, mass, temperature, area, volume, speed, energy, power, pressure, \
                 time, angle, fuel, frequency. Temperature conversion is affine (handles Fahrenheit/Kelvin/Rankine). \
                 Examples: unit_tools(action:'convert', value:100, from:'km', to:'miles') or \
                 unit_tools(action:'convert', value:98.6, from:'fahrenheit', to:'celsius') or \
                 unit_tools(action:'list', category:'energy')"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_geometry_tools(&effective_user_input) {
            loop_intervention = Some(
                "GEOMETRY TOOLS NOTICE: Use the `geometry_tools` tool for geometric calculations without external utilities. \
                 Actions: \
                 area (default — 'shape' + dimensions → area; shapes: rectangle/width+height, square/side, \
                 circle/radius, ellipse/a+b, triangle/base+height or a+b+c sides, trapezoid/a+b+height, \
                 parallelogram/base+height, rhombus/d1+d2, regular_polygon/sides+side_length, sector/radius+angle), \
                 volume ('shape' + dimensions → volume and surface area; shapes: cube/side, rectangular_prism/width+height+depth, \
                 sphere/radius, hemisphere/radius, cylinder/radius+height, cone/radius+height, pyramid/base_area+height, \
                 torus/major_radius+minor_radius), \
                 perimeter (same shapes as area), \
                 triangle (comprehensive triangle solver; 'a','b','c' for SSS or 'a','b','angle_c' for SAS — \
                 returns all angles, area, perimeter, inradius, circumradius, type), \
                 circle (comprehensive circle math; provide any one of: radius, diameter, circumference, area; \
                 optional 'angle' in degrees for arc length, sector area, chord length). \
                 Examples: geometry_tools(action:'area', shape:'circle', radius:5) or \
                 geometry_tools(action:'triangle', a:3, b:4, c:5) or \
                 geometry_tools(action:'circle', diameter:10, angle:90)"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_checksum_tools(&effective_user_input) {
            loop_intervention = Some(
                "CHECKSUM TOOLS NOTICE: Use the `checksum_tools` tool for computing checksums without external utilities. \
                 Actions: \
                 all (default — compute all checksums in one call: CRC-8, CRC-16/MODBUS, CRC-32 IEEE, Adler-32, Fletcher-16), \
                 crc8 (CRC-8 polynomial 0x07), crc16 (CRC-16/MODBUS 0xA001 reflected), \
                 crc32 (CRC-32 IEEE 802.3 — same as zip/gzip), \
                 adler32 (Adler-32: fast A+B rolling sum used in zlib), \
                 fletcher16 (Fletcher-16 checksum). \
                 Input: 'text' for a string, or 'hex' for raw bytes as a hex string. \
                 Output includes decimal, hex, and binary representations. \
                 Examples: checksum_tools(action:'all', text:'Hello') or \
                 checksum_tools(action:'crc32', hex:'DEADBEEF')"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_id_tools(&effective_user_input) {
            loop_intervention = Some(
                "ID TOOLS NOTICE: Use the `id_tools` tool to generate and decode time-sortable IDs without external utilities. \
                 Actions: \
                 ulid (default — Universally Unique Lexicographically Sortable Identifier; 26-char Crockford base32; \
                 'count' for bulk; optional 'seed' for reproducibility), \
                 nanoid (URL-safe compact random ID; 'size' chars default 21; optional 'alphabet' for custom chars; 'count'), \
                 snowflake (Twitter/Discord-style 64-bit numeric ID; 41-bit ms timestamp + 10-bit machine + 12-bit seq; \
                 'machine_id' optional; 'count'), \
                 decode (inspect a ULID or Snowflake — extracts embedded timestamp, machine ID, random part; pass 'id'). \
                 Examples: id_tools(action:'ulid', count:5) or \
                 id_tools(action:'nanoid', size:12, count:3) or \
                 id_tools(action:'decode', id:'01ARZ3NDEKTSV4RRFFQ69G5FAV')"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_jsonl_tools(&effective_user_input) {
            loop_intervention = Some(
                "JSONL NOTICE: Use the `jsonl_tools` tool to process JSONL (JSON Lines / NDJSON) data \
                 without external utilities. Each line is a separate JSON object. \
                 Actions: parse (default — display records with index, pretty-printed; 'limit' to cap), \
                 filter (keep records where a field matches a value; 'field' dot-path + 'value' + 'op': eq/ne/gt/lt/gte/lte/contains/exists/missing), \
                 map (extract one field from every record; 'field'), \
                 aggregate (count/sum/avg/min/max/distinct on a field; 'field' + 'agg'), \
                 keys (union of all keys across all records with type distribution), \
                 stats (record count, key coverage %, null rate, type distribution per key), \
                 to_csv (convert records to CSV), \
                 group (group by a field value with count bar chart; 'field'), \
                 sort (sort by a field; 'field'; optional 'order': asc/desc). \
                 Pass 'text'/'jsonl' (inline content) or 'file' (path to .jsonl/.ndjson file). \
                 Example: jsonl_tools(action: 'filter', text: '...', field: 'status', value: 'error') or \
                 jsonl_tools(action: 'aggregate', file: 'events.jsonl', field: 'duration', agg: 'avg')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_json_tools(&effective_user_input) {
            loop_intervention = Some(
                "JSON TOOLS NOTICE: Use the `json_tools` tool to query, transform, and analyze JSON \
                 without needing jq or external utilities. Actions: \
                 pretty (default — pretty-print with indentation), \
                 compact (minify), \
                 keys (list top-level keys), \
                 get (dot-path navigation e.g. 'user.address.city' or 'items[0].id'), \
                 filter (field equality/comparison e.g. field:'status', op:'eq', value:'active'), \
                 pluck (extract a field from every object in an array), \
                 flatten (flatten nested objects one level), \
                 count, sort, unique, merge, diff, validate, \
                 schema (infer recursive type structure), \
                 stats (min/max/mean/median/stddev for numeric fields), \
                 to-csv (convert array of objects to CSV). \
                 Pass 'json' for inline JSON or 'file' for a file path. \
                 Example: json_tools(action:'get', json:'{...}', path:'user.name') or \
                 json_tools(action:'filter', json:'[...]', field:'age', op:'gt', value:30)"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_code_metrics(&effective_user_input) {
            loop_intervention = Some(
                "CODE METRICS NOTICE: Use the `code_metrics` tool to measure lines of code, comment density, \
                 TODO/FIXME counts, language breakdown, and test coverage ratio without external tools. \
                 Pass 'path' for a directory or file (defaults to workspace root). \
                 Output includes: total lines / blank / comment / code counts, comment density %, \
                 TODO+FIXME count, top 10 largest files, language breakdown by extension, \
                 and test file ratio as a coverage proxy. \
                 Example: code_metrics() for the whole workspace, code_metrics(path:'src') for a subdirectory."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_dependency_audit(&effective_user_input) {
            loop_intervention = Some(
                "DEPENDENCY AUDIT NOTICE: Use the `dependency_audit` tool to audit Cargo.toml, package.json, \
                 requirements.txt/pyproject.toml, and go.mod for pinning issues, wildcard versions, deprecated packages, \
                 missing lock files, and outdated major versions. No network required. \
                 Pass 'path' for the project root (defaults to workspace root). \
                 Output: per-manifest findings (pinning warnings, wildcard versions, missing lock file) \
                 and recommendations for cargo audit/npm audit/safety for CVE scanning. \
                 Example: dependency_audit() for the workspace or dependency_audit(path:'my-project')"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_env_diff(&effective_user_input) {
            loop_intervention = Some(
                "ENV DIFF NOTICE: Use the `env_diff` tool to compare two .env files or a .env file against \
                 the live process environment. Findings: additions (+), removals (-), changed values (~) — \
                 secret values are automatically redacted. \
                 Pass 'file_a' and 'file_b' for two files, or 'file_a' alone to compare against the process env. \
                 With no arguments, auto-detects .env/.env.local pairs in the workspace root. \
                 Example: env_diff(file_a:'.env', file_b:'.env.production') or env_diff(file_a:'.env')"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_template_gen(&effective_user_input) {
            loop_intervention = Some(
                "TEMPLATE GEN NOTICE: Use the `template_gen` tool to generate 23 built-in project scaffolding files \
                 without external utilities. Pass 'template' to select one; pass template='list' to see all. \
                 Templates: dockerfile-node, dockerfile-python, dockerfile-rust, dockerfile-go (multi-stage), \
                 ci-github-node, ci-github-python, ci-github-rust, ci-github-go (Actions workflows), \
                 gitignore-node, gitignore-python, gitignore-rust, gitignore-go, \
                 docker-compose, makefile, env-example, pre-commit, editorconfig, \
                 dependabot, codeowners, pr-template, issue-bug, issue-feature. \
                 Supports variable substitution via 'project_name', 'port', and language version fields. \
                 Example: template_gen(template:'dockerfile-rust', project_name:'my-api', port:'8080') or \
                 template_gen(template:'ci-github-rust') to scaffold a full CI pipeline."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_changelog_gen(&effective_user_input) {
            loop_intervention = Some(
                "CHANGELOG GEN NOTICE: Use the `changelog_gen` tool to generate a Markdown changelog \
                 from git commit history grouped by conventional commit type (feat/fix/perf/refactor/docs/test/chore). \
                 Optional 'from'/'to' version tags to scope the range, 'title' for the version heading, \
                 and up to 500 commits processed. Scopes rendered in bold, short hash appended per entry. \
                 Example: changelog_gen(from:'v0.11.0', title:'v0.12.0') or \
                 changelog_gen() to generate from all commits."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_port_check(&effective_user_input) {
            loop_intervention = Some(
                "PORT CHECK NOTICE: Use the `port_check` tool to test TCP port reachability with a configurable timeout. \
                 Pass 'host' and 'port' (required). Optional 'timeout_ms' (default 3000ms). \
                 Annotates 40+ well-known ports (PostgreSQL=5432, Redis=6379, MySQL=3306, SSH=22, HTTPS=443, \
                 LM Studio=1234, Ollama=11434, Jupyter=8888, RDP=3389, etc.). \
                 Returns OPEN or CLOSED/FILTERED with actionable hints for closed ports. \
                 Example: port_check(host:'localhost', port:5432) or port_check(host:'192.168.1.1', port:22)"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_scientific_compute(&effective_user_input) {
            loop_intervention = Some(
                "SCIENTIFIC COMPUTE NOTICE: Use the `scientific_compute` tool for physics and chemistry calculations \
                 using fundamental constants and well-known formulas. No external libraries required. \
                 Covers: physical constants (speed of light, Planck, Boltzmann, Avogadro, electron charge/mass, etc.), \
                 mechanics (kinetic/potential energy, work, momentum, gravitational force), \
                 electromagnetism (Ohm's law, Coulomb's law, capacitance, inductance), \
                 thermodynamics (ideal gas law, heat, Stefan-Boltzmann radiation), \
                 waves (frequency/wavelength/period, photon energy), \
                 chemistry (molar mass, stoichiometry, pH). \
                 Example: scientific_compute(formula:'kinetic_energy', mass:2.0, velocity:5.0) or \
                 scientific_compute(formula:'ohms_law', voltage:12.0, resistance:4.0)"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_fraction_tools(&effective_user_input) {
            loop_intervention = Some(
                "FRACTION TOOLS NOTICE: Use the `fraction_tools` tool for fraction arithmetic and conversions \
                 without external utilities. Actions: \
                 simplify (default — reduce fraction to lowest terms; 'fraction' e.g. \"6/9\" or 'numerator'/'denominator'), \
                 add/sub/mul/div (arithmetic on two fractions; 'a' and 'b' fields e.g. \"1/4\"), \
                 convert ('fraction' string → decimal/percent/mixed OR 'decimal' number → fraction via continued fractions), \
                 compare ('a'/'b' or 'fractions' array — ordering and difference), \
                 series ('type': harmonic/egyptian/farey; 'terms' for harmonic, 'fraction' for egyptian, 'n' for farey). \
                 Example: fraction_tools(action:'add', a:'1/3', b:'2/5') or \
                 fraction_tools(action:'convert', decimal:0.3333333)"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_number_theory_tools(&effective_user_input) {
            loop_intervention = Some(
                "NUMBER THEORY NOTICE: Use the `number_theory_tools` tool for pure number-theory calculations \
                 without external utilities. Actions: \
                 factor (default — prime factorization with all divisors and divisor sum; 'n'), \
                 primes ('limit' for sieve up to N, 'nth' for Nth prime, 'test' for primality check), \
                 gcd/lcm ('a'/'b' or 'numbers' array; gcd shows Bézout coefficients), \
                 totient (Euler phi function; 'n'), \
                 modpow (fast modular exponentiation; 'base', 'exp', 'modulus'), \
                 modinv (modular inverse; 'a', 'modulus'), \
                 collatz (Collatz sequence to 1; 'n'), \
                 fibonacci (first N numbers via 'n'; 'nth' for specific index; 'test' for membership check), \
                 perfect (classify as perfect/abundant/deficient; 'n' or 'limit' for range scan). \
                 Example: number_theory_tools(action:'factor', n:360) or \
                 number_theory_tools(action:'primes', limit:100)"
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_dns_tools(&effective_user_input) {
            loop_intervention = Some(
                "DNS TOOLS NOTICE: Use the `dns_tools` tool to parse, filter, validate, and explain \
                 BIND-format DNS zone files without external utilities. \
                 Actions: parse (default — all records grouped by type; NAME/TTL/TYPE/DATA table), \
                 records (filter by record type; pass 'type' like 'MX', 'TXT', 'A', 'NS'), \
                 validate (check for missing SOA/NS, CNAME at apex, MX-to-CNAME, duplicate records, SPF count), \
                 explain (plain-English breakdown per record type; SPF/DMARC/DKIM/CAA decoded). \
                 Pass 'text'/'zone' for inline zone content or 'file' for a path. \
                 Example: dns_tools(text: '...zone...') or dns_tools(action: 'records', text: '...', type: 'MX') or \
                 dns_tools(action: 'validate', file: '/etc/bind/db.example.com')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_css_tools(&effective_user_input) {
            loop_intervention = Some(
                "CSS TOOLS NOTICE: Use the `css_tools` tool to parse, validate, extract variables, \
                 compute statistics, and minify CSS without external utilities. \
                 Actions: parse (default — all selectors with line number, property count, and key declarations; \
                 at-rule summary), \
                 validate (duplicate selectors, empty blocks, duplicate properties, !important overuse, \
                 vendor prefix without standard, deep selectors, bad hex colors, high z-index, unknown pseudo-elements), \
                 vars (defined -- custom properties with values; var() usage counts; undefined variable warnings), \
                 stats (totals, at-rule breakdown, top-10 properties, selector complexity, colors, file size), \
                 minify (strip comments, collapse whitespace; shows size reduction). \
                 Pass 'text'/'css' for inline CSS or 'file' for a path. \
                 Example: css_tools(text: '...') or css_tools(action: 'validate', file: 'styles.css') or \
                 css_tools(action: 'stats', file: 'app.css')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_plist_tools(&effective_user_input) {
            loop_intervention = Some(
                "PLIST NOTICE: Use the `plist_tools` tool to parse, inspect, validate, and convert \
                 Apple Property List (plist) XML files without external utilities. \
                 Actions: parse (default — human-readable indented tree; highlights bundle ID, version, ATS, permissions), \
                 get (navigate to any value by dot-path like 'NSAppTransportSecurity.NSAllowsArbitraryLoads'), \
                 keys (list top-level or nested keys with type and value preview), \
                 validate (check for missing required keys, NSAllowsArbitraryLoads=true, missing UsageDescription), \
                 to-json (convert plist to pretty-printed JSON). \
                 Pass 'file' for a .plist path or 'text'/'plist'/'xml' for inline plist XML. \
                 Example: plist_tools(action: 'parse', file: 'Info.plist') or \
                 plist_tools(action: 'get', file: 'Info.plist', path: 'CFBundleVersion') or \
                 plist_tools(action: 'validate', file: 'MyApp/Info.plist')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_bencode_tools(&effective_user_input) {
            loop_intervention = Some(
                "BENCODE NOTICE: Use the `bencode_tools` tool to decode and inspect BitTorrent bencode \
                 format and .torrent files without external utilities. \
                 Actions: decode (default — human-readable indented tree with type annotations), \
                 info (torrent summary: name, file count, total size, piece size, tracker, creator, creation date), \
                 files (list all files with path, size, and cumulative offset), \
                 trackers (all tracker URLs grouped by tier with UDP/HTTP/HTTPS distinction). \
                 Pass 'file' for a .torrent path, 'hex' for hex-encoded bencode bytes, or 'text' for raw bencode. \
                 Example: bencode_tools(action: 'info', file: 'download.torrent') or \
                 bencode_tools(action: 'files', file: 'archive.torrent') or \
                 bencode_tools(action: 'trackers', file: 'movie.torrent')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_printf_tools(&effective_user_input) {
            loop_intervention = Some(
                "PRINTF NOTICE: Use the `printf_tools` tool to analyze, simulate, validate, or convert \
                 C-style printf format strings without external utilities. \
                 Actions: explain (default — parse all format specifiers with type/width/precision/flags and meanings; warns on %n), \
                 simulate (render the format string with provided args JSON array), \
                 validate (check for %n, unknown specifiers, arg count mismatches, null bytes), \
                 convert (translate to Python %, Python f-string, Rust format!, Go fmt.Sprintf, JavaScript template literal). \
                 Pass 'format' for the format string; 'args' as a JSON array for simulate. \
                 Example: printf_tools(action: 'explain', format: '%-10s %5.2f') or \
                 printf_tools(action: 'simulate', format: 'Hello %s, you are %d years old', args: ['Alice', 30]) or \
                 printf_tools(action: 'convert', format: '%05.2f')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_ascii_chart_tools(&effective_user_input) {
            loop_intervention = Some(
                "ASCII CHART NOTICE: Use the `ascii_chart_tools` tool to render charts directly in the terminal \
                 from numeric data without external utilities. \
                 Actions: bar (default — vertical bar chart with labels), line (line/time-series chart), \
                 scatter (XY scatter plot from x+y arrays or [[x,y]] pairs), \
                 sparkline (compact one-row Unicode sparkline ▁▂▃▄▅▆▇█), hbar (alias for bar). \
                 Pass 'data' as a JSON number array or comma-separated string; add 'labels', 'title', 'width', 'height'. \
                 Example: ascii_chart_tools(action: 'bar', data: [10,25,15,40,30], labels: ['Mon','Tue','Wed','Thu','Fri'], title: 'Daily Sales') or \
                 ascii_chart_tools(action: 'sparkline', data: [1,3,2,8,4,6,5]) or \
                 ascii_chart_tools(action: 'scatter', x: [1,2,3,4], y: [1,4,9,16])."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_sql_format_tools(&effective_user_input) {
            loop_intervention = Some(
                "SQL FORMAT NOTICE: Use the `sql_format_tools` tool to format, minify, split, or extract from SQL statements \
                 without external utilities. \
                 Actions: format (default — pretty-print with configurable indentation and keyword casing; 'indent' and 'uppercase' options), \
                 minify (compact SQL — strip whitespace and comments; reports size reduction %), \
                 split (split multi-statement SQL on semicolons into individual statement blocks), \
                 extract (extract 'tables', 'columns', 'aliases', or 'comments' from the SQL; pass 'what' arg). \
                 Pass 'sql' for inline SQL text or 'file' for a path to a .sql file. \
                 Example: sql_format_tools(action: 'format', sql: 'select id,name from users where active=1') or \
                 sql_format_tools(action: 'minify', sql: '  SELECT  *  FROM  users') or \
                 sql_format_tools(action: 'extract', sql: '...', what: 'tables') or \
                 sql_format_tools(action: 'split', file: 'migrations/001.sql')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_totp_tools(&effective_user_input) {
            loop_intervention = Some(
                "TOTP NOTICE: Use the `totp_tools` tool to generate, verify, and inspect TOTP/HOTP one-time passwords \
                 without external utilities or cloud calls. \
                 Actions: generate (default — current TOTP code from base32 secret; shows code, validity window, ±1 context codes), \
                 verify (check a user-provided code against the secret; accepts ±1 window for clock drift), \
                 hotp (generate HMAC-based OTP codes from a counter; 'count' to see multiple), \
                 info (explain TOTP/HOTP parameters or parse an otpauth:// URI), \
                 qr (generate the otpauth:// URI for QR code scanning in authenticator apps). \
                 Pass 'secret' as the base32-encoded secret (from the app setup QR code). \
                 Example: totp_tools(secret: 'JBSWY3DPEHPK3PXP') or \
                 totp_tools(action: 'verify', secret: 'JBSWY3DPEHPK3PXP', code: '123456') or \
                 totp_tools(action: 'qr', secret: 'JBSWY3DPEHPK3PXP', issuer: 'MyApp', label: 'user@example.com')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_tar_tools(&effective_user_input) {
            loop_intervention = Some(
                "TAR NOTICE: Use the `tar_tools` tool to inspect uncompressed TAR archives without external utilities. \
                 Actions: list (default — table of all entries with permissions/size/date/type), \
                 info (archive statistics: file/dir/symlink counts, total content size, owner list, date range), \
                 find (filter entries by name substring; pass 'query'), \
                 extract (read a specific text entry; pass 'entry' with the entry name; limited to 512 KB). \
                 Pass 'file' with the path to the .tar archive. \
                 For .tar.gz / .tgz / .tar.bz2 / .tar.xz — tool reports the compression type and the correct shell command. \
                 Example: tar_tools(file: 'archive.tar') or \
                 tar_tools(action: 'find', file: 'archive.tar', query: '.rs') or \
                 tar_tools(action: 'extract', file: 'archive.tar', entry: 'project/README.md')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_email_tools(&effective_user_input) {
            loop_intervention = Some(
                "EMAIL NOTICE: Use the `email_tools` tool to parse and analyze RFC 2822 email files (.eml) without external utilities. \
                 Actions: parse (default — key headers summary + body preview; decodes RFC 2047 encoded words), \
                 headers (all headers in a table; pass 'name' to retrieve a specific header; pass 'filter' to narrow), \
                 structure (MIME part tree — content types, encodings, attachment listing), \
                 trace (delivery chain from Received: headers — hop servers, IPs, timestamps). \
                 Pass 'file' (path to .eml file) or 'text' (raw email string). \
                 Example: email_tools(file: 'message.eml') or \
                 email_tools(action: 'headers', file: 'message.eml', name: 'Subject') or \
                 email_tools(action: 'trace', text: raw_email_string) or \
                 email_tools(action: 'structure', file: 'message.eml')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_cbor_tools(&effective_user_input) {
            loop_intervention = Some(
                "CBOR NOTICE: Use the `cbor_tools` tool to decode and inspect CBOR (Concise Binary Object Representation) data without external utilities. \
                 Actions: decode (default — human-readable decoded value with type labels and tag annotations), \
                 info (root type, total bytes, array/map length, key list, type distribution), \
                 annotate (hex dump with per-byte CBOR major-type labels). \
                 Accepts 'hex' (hex-encoded bytes), 'base64' (base64/base64url), or 'file' path. \
                 Automatically annotates known tags (tag 0=datetime, tag 1=epoch, tag 37=uuid, tag 55799=self-described CBOR) \
                 and emits hints for WebAuthn AttestationObject / COSE Key structures. \
                 Example: cbor_tools(hex: 'a2616101616202') or cbor_tools(file: 'payload.cbor') or cbor_tools(action: 'annotate', hex: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_msgpack_tools(&effective_user_input) {
            loop_intervention = Some(
                "MSGPACK NOTICE: Use the `msgpack_tools` tool to decode and inspect MessagePack binary data without external utilities. \
                 Actions: decode (default — human-readable decoded value), \
                 info (root type, total bytes, array/map length, string key list, type distribution), \
                 annotate (hex dump with per-byte MessagePack format-byte labels: fixint, fixmap, fixarray, fixstr, uint8/16/32/64, int8..., float32/64, bin8..., ext types). \
                 Accepts 'hex' (hex-encoded bytes), 'base64' (base64/base64url), or 'file' path. \
                 Automatically decodes Timestamp ext type (-1) for 4-byte and 8-byte forms. \
                 Example: msgpack_tools(hex: '82a3666f6f01a362617202') or msgpack_tools(file: 'data.msgpack') or msgpack_tools(action: 'annotate', hex: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_wasm_tools(&effective_user_input) {
            loop_intervention = Some(
                "WASM NOTICE: Use the `wasm_tools` tool to inspect WebAssembly binary (.wasm) files without external utilities. \
                 Actions: info (default — magic, version, section count, import/export summary), \
                 sections (all sections with id, name, size, and offset), \
                 imports (all imported functions, tables, memories, globals with module and name), \
                 exports (all exported symbols with kind and index). \
                 Pass 'file' (path to .wasm file) or 'hex' (hex-encoded WASM bytes). \
                 Example: wasm_tools(file: 'module.wasm') or \
                 wasm_tools(action: 'imports', file: 'module.wasm') or \
                 wasm_tools(action: 'exports', file: 'module.wasm')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_jsonschema_tools(&effective_user_input) {
            loop_intervention = Some(
                "JSON SCHEMA NOTICE: Use the `jsonschema_tools` tool to inspect and validate JSON Schema documents. \
                 Actions: info (default — schema metadata: $schema, title, type, required count, property count, combiners), \
                 properties (list all properties with type, required flag, and description), \
                 refs (list all $ref, $defs/definitions, and $id anchors), \
                 validate (validate a JSON instance against the schema — reports errors with JSON Pointer paths). \
                 Pass 'schema' (inline JSON or file path) or 'schema_file'. For validate also pass 'instance' or 'instance_file'. \
                 Example: jsonschema_tools(schema_file: 'schema.json') or \
                 jsonschema_tools(action: 'validate', schema_file: 'schema.json', instance_file: 'data.json') or \
                 jsonschema_tools(action: 'properties', schema: '{\"type\":\"object\",...}')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_html_tools(&effective_user_input) {
            loop_intervention = Some(
                "HTML NOTICE: Use the `html_tools` tool to parse and analyze HTML documents without external utilities. \
                 Actions: parse (default — overview: title, meta description, element counts, heading structure), \
                 links (all hyperlinks with href, anchor text, and rel type), \
                 images (all img tags with src, alt, and dimensions; flags missing alt), \
                 forms (form elements with method, action, and input fields), \
                 tables (table structure with row/column counts and cell preview), \
                 scripts (external and inline script tags), \
                 validate (accessibility and SEO checks: doctype, charset, lang, title, alt, viewport, h1), \
                 text (strip all HTML tags to plain text), \
                 stats (element counts, max nesting depth, top tags). \
                 Pass 'html' (inline HTML) or 'file' (path to .html/.htm file). \
                 Example: html_tools(file: 'index.html') or html_tools(action: 'links', file: 'page.html') or \
                 html_tools(action: 'validate', html: '<html>...</html>')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_vcf_tools(&effective_user_input) {
            loop_intervention = Some(
                "VCF NOTICE: Use the `vcf_tools` tool to parse and analyze vCard contact files (.vcf) without external utilities. \
                 Actions: parse (default — full contact detail: name, org, title, emails, phones, addresses, URLs, birthday, categories), \
                 list (summary table — name, primary email, primary phone), \
                 search (filter contacts by keyword across all fields; pass 'query'), \
                 to_json (JSON array of all contacts with structured fields), \
                 to_csv (CSV export with standard contact columns). \
                 Supports vCard 2.1, 3.0, and 4.0. Handles line unfolding, property parameters (TYPE=WORK,CELL), and structured names. \
                 Pass 'vcf' (inline vCard content) or 'file' (path to .vcf file). \
                 Example: vcf_tools(file: 'contacts.vcf') or vcf_tools(action: 'to_csv', file: 'contacts.vcf') or \
                 vcf_tools(action: 'search', file: 'contacts.vcf', query: 'smith')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_network_header_tools(&effective_user_input) {
            loop_intervention = Some(
                "NETWORK HEADER NOTICE: Use the `network_header_tools` tool to parse and decode raw network protocol headers \
                 from hex bytes without external tools. \
                 Actions: parse (auto-detect protocol from bytes), ipv4 (decode IPv4 header with checksum verification), \
                 ipv6 (decode IPv6 header with next-header chain), tcp (decode TCP header with flag breakdown), \
                 udp (decode UDP header), icmp (decode ICMP/ICMPv6 type/code), ethernet (decode Ethernet II frame header). \
                 Pass 'hex' with the raw header bytes (spaces and colons are ignored). \
                 Example: network_header_tools(hex: '45 00 00 3c ...') or network_header_tools(action: 'tcp', hex: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_tlv_tools(&effective_user_input) {
            loop_intervention = Some(
                "TLV NOTICE: Use the `tlv_tools` tool to parse, decode, and build Type-Length-Value encoded binary data \
                 without external tools. \
                 Actions: parse (generic TLV with configurable type_size/length_size/endian), \
                 ber (ASN.1 BER/DER — variable-length tag and length, with type name decoding), \
                 dhcp (DHCP options per RFC 2132 with known option names and value formatting), \
                 wifi (802.11 information elements with SSID/rate/RSN decoding), \
                 build (assemble TLV bytes from a JSON items spec). \
                 Pass 'hex' with raw bytes. \
                 Example: tlv_tools(hex: 'a2616101616202') or tlv_tools(action: 'ber', hex: '300a...') or \
                 tlv_tools(action: 'dhcp', hex: '3501013604...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_bin_pack_tools(&effective_user_input) {
            loop_intervention = Some(
                "BIN PACK NOTICE: Use the `bin_pack_tools` tool to pack and unpack binary data using \
                 struct-style format strings without external utilities. \
                 Actions: pack (values → hex bytes), unpack (hex bytes → typed field values), \
                 describe (explain each field in a format string), size (total byte size of the format). \
                 Format string: optional endian prefix < (little) or > (big, default), then field specifiers \
                 b/B (int8/uint8), h/H (int16/uint16), i/I (int32/uint32), q/Q (int64/uint64), \
                 f (float32), d (float64), s (length-prefixed string), x (pad byte). Repeat counts allowed: '4B'. \
                 Example: bin_pack_tools(format: '<HI', action: 'pack', values: [42, 1000]) or \
                 bin_pack_tools(format: '>BHI', action: 'unpack', hex: '01 00 2a 00 00 03 e8')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_elf_tools(&effective_user_input) {
            loop_intervention = Some(
                "ELF NOTICE: Use the `elf_tools` tool to inspect ELF binary files (Linux executables, \
                 shared libraries .so, object files .o, kernel modules .ko) without readelf or objdump. \
                 Actions: info (default — ELF class/endian/type/machine/entry point), \
                 segments (program headers — type, flags, virtual address, size), \
                 sections (section headers — name, type, flags, offset, size), \
                 symbols (symbol table with bind/type/section), \
                 dynamic (shared library dependencies, SONAME, RPATH, DT_NEEDED libraries). \
                 Pass 'file' with the path to the ELF binary or 'hex' with raw ELF bytes. \
                 Example: elf_tools(file: '/usr/bin/ls') or elf_tools(action: 'dynamic', file: 'libfoo.so')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_leb128_tools(&effective_user_input) {
            loop_intervention = Some(
                "LEB128 NOTICE: Use the `leb128_tools` tool to encode, decode, and analyze \
                 LEB128 variable-length integers (ULEB128 unsigned and SLEB128 signed). \
                 Actions: encode (integer → LEB128 bytes), decode (bytes → integer + byte count), \
                 analyze (byte-by-byte bit-field breakdown), multi (batch encode array or decode stream), \
                 explain (verbose bit-level walkthrough of each group). \
                 Used in WASM, DWARF debug info, protobuf, and Android DEX. \
                 Example: leb128_tools(action: 'encode', value: 624485) or \
                 leb128_tools(action: 'decode', hex: 'e5 8e 26')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_unicode_tools(&effective_user_input) {
            loop_intervention = Some(
                "UNICODE NOTICE: Use the `unicode_tools` tool to analyze Unicode text. \
                 Actions: analyze (per-character codepoint/category/script/UTF-8 bytes), \
                 scripts (script distribution table), blocks (Unicode block distribution), \
                 bidi (RTL detection + Trojan Source CVE-2021-42574 bidi control char risk), \
                 confusables (homoglyph/lookalike detection — Cyrillic/Greek ASCII lookalikes), \
                 encoding (UTF-8/UTF-16/UTF-32 byte sequences per character), \
                 normalize (NFC/NFD normalization status — combining marks vs precomposed). \
                 Example: unicode_tools(action: 'bidi', text: '...') or \
                 unicode_tools(action: 'confusables', text: 'pаypal') or \
                 unicode_tools(text: 'Hello 世界')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_todo_tools(&effective_user_input) {
            loop_intervention = Some(
                "TODO SCAN NOTICE: Use the `todo_tools` tool to scan source files for annotated comments. \
                 Actions: scan (default — find all TODO/FIXME/HACK/XXX/NOTE/DEPRECATED/BUG/OPTIMIZE/WORKAROUND/TEMP/KLUDGE/NB annotations grouped by label), \
                 stats (count per label with bar chart), list (flat chronological list of all findings), \
                 filter (show only a specific label; pass 'label'), files (top N files by annotation count). \
                 Example: todo_tools() or todo_tools(action: 'filter', label: 'FIXME') or \
                 todo_tools(action: 'stats', path: 'src/')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_grep_tools(&effective_user_input) {
            loop_intervention = Some(
                "GREP NOTICE: Use the `grep_tools` tool to search files for patterns without external utilities. \
                 Actions: search (default — matching lines with file:line context), count (matches per file), \
                 files (list files with any match), matches (flat list with capture groups). \
                 Options: case_insensitive, fixed (literal string), whole_word, before/after (context lines), invert. \
                 Example: grep_tools(pattern: 'fn execute', path: 'src/') or \
                 grep_tools(pattern: 'TODO', action: 'count') or \
                 grep_tools(pattern: 'error', case_insensitive: true, after: 2)."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_file_tree_tools(&effective_user_input) {
            loop_intervention = Some(
                "FILE TREE NOTICE: Use the `file_tree_tools` tool to generate visual directory trees without the `tree` command. \
                 Actions: tree (default — ASCII directory tree with depth limit), flat (sorted file listing), \
                 stats (file/dir/size breakdown by extension), json (structured JSON tree), sizes (largest files ranked). \
                 Options: path, depth (default 4), show_hidden, extensions (filter), limit. \
                 Example: file_tree_tools() or file_tree_tools(path: 'src/', depth: 3) or \
                 file_tree_tools(action: 'stats', path: '.') or file_tree_tools(action: 'sizes')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_find_tools(&effective_user_input) {
            loop_intervention = Some(
                "FIND NOTICE: Use the `find_tools` tool to find files matching criteria without the `find` command. \
                 Actions: list (default — matching paths with size/age), count (how many match), \
                 sizes (size summary), recent (sorted newest first). \
                 Filters: name (glob or substring), ext (extension), type (file/dir/all), \
                 min_size/max_size (bytes), newer_than/older_than (days), depth, show_hidden. \
                 Example: find_tools(name: '*.rs') or find_tools(ext: 'json', newer_than: 7) or \
                 find_tools(action: 'recent', newer_than: 3) or find_tools(min_size: 1048576, action: 'sizes')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_text_extract_tools(&effective_user_input) {
            loop_intervention = Some(
                "TEXT EXTRACT NOTICE: Use the `text_extract_tools` tool to extract structured entities from unstructured text. \
                 Actions: emails, urls, ips (IPv4 and IPv6), phones (US/international), dates (ISO/US/EU), \
                 uuids, hashes (MD5/SHA-1/SHA-256), all (default — every entity type at once), custom (user regex). \
                 Each action returns deduplicated results with occurrence counts. \
                 Options: unique (default true), limit (max per type), case_insensitive (for custom). \
                 Example: text_extract_tools(text: '...') or text_extract_tools(action: 'emails', text: '...') or \
                 text_extract_tools(action: 'custom', pattern: 'API_[A-Z0-9]+', text: '...')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_interval_tools(&effective_user_input) {
            loop_intervention = Some(
                "INTERVAL NOTICE: Use the `interval_tools` tool to work with date intervals and schedules without external utilities. \
                 Actions: overlap (do two intervals overlap — shows overlap range and duration), \
                 contains (is a date within an interval), union (merge overlapping intervals from a list), \
                 intersect (find intersection of two intervals), duration (time between two dates in full breakdown), \
                 schedule (generate N recurring dates from a start with a step like '1d', '2w', '1m', '6h', '30min'). \
                 Accepts ISO 8601 dates (YYYY-MM-DD) and datetimes (YYYY-MM-DDTHH:MM:SS). \
                 Example: interval_tools(action: 'overlap', start: '2024-01-01', end: '2024-06-30', start2: '2024-04-01', end2: '2024-12-31') or \
                 interval_tools(action: 'schedule', start: '2024-01-01', step: '2w', count: 12) or \
                 interval_tools(action: 'duration', start: '2023-03-15', end: '2024-09-01')."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_number_sequence_tools(&effective_user_input) {
            loop_intervention = Some(
                "NUMBER SEQUENCE NOTICE: Use the `number_sequence_tools` tool to analyze and extend numeric sequences without external utilities. \
                 Actions: detect (identify pattern — arithmetic, geometric, Fibonacci-like, polynomial, squares, cubes, triangular, power-of-2, primes, constant), \
                 continue (extend the sequence by N more terms), \
                 diff (show Newton's forward difference table to reveal polynomial patterns), \
                 stats (min/max/mean/sum/growth rate). \
                 Example: number_sequence_tools(numbers: [1, 4, 9, 16, 25]) or \
                 number_sequence_tools(action: 'continue', data: '2, 4, 8, 16, 32', n: 6)."
                    .to_string(),
            );
        }

        if loop_intervention.is_none() && needs_number_words_tools(&effective_user_input) {
            loop_intervention = Some(
                "NUMBER WORDS NOTICE: Use the `number_words_tools` tool to convert numbers to/from English words without external utilities. \
                 Actions: to_words (1234 → 'one thousand two hundred thirty-four'), \
                 to_ordinal (42 → 'forty-second'), \
                 from_words ('one hundred twenty-three' → 123), \
                 currency (123.45 → 'one hundred twenty-three dollars and forty-five cents'; supports dollar/euro/pound), \
                 digits (spell each digit: 123 → 'one two three'), \
                 roman (integer ↔ Roman numeral — pass 'number' to encode, 'text' to decode). \
                 Example: number_words_tools(number: 1000000) or \
                 number_words_tools(action: 'to_ordinal', number: 21) or \
                 number_words_tools(action: 'from_words', text: 'three hundred and forty-five')."
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
            std::collections::HashMap::with_capacity(8);
        let mut completed_tool_cache: std::collections::HashMap<String, CachedToolResult> =
            std::collections::HashMap::with_capacity(8);
        let mut successful_read_targets: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(8);
        // (path, offset) pairs — catches repeated reads at the same non-zero offset.
        let mut successful_read_regions: std::collections::HashSet<(String, u64)> =
            std::collections::HashSet::with_capacity(8);
        let mut successful_grep_targets: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(8);
        let mut no_match_grep_targets: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(8);
        let mut broad_grep_targets: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(8);
        let mut sovereign_task_root: Option<String> = None;
        let mut sovereign_scaffold_targets: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        let mut turn_mutated_paths: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        let mut mutation_counts_by_path: std::collections::HashMap<String, usize> =
            std::collections::HashMap::with_capacity(4);
        let mut frontend_polish_intervention_emitted = false;
        let mut visible_closeout_emitted = false;

        // Track the index of the message that started THIS turn, so compaction doesn't summarize it.
        let mut turn_anchor = self.history.len().saturating_sub(1);

        // ── Pre-turn compaction (Codex-style: PreTurn phase) ────────────────
        // If context is already overloaded before inference starts, compact now.
        // This prevents the model from seeing a 90%+ full prompt on the first call.
        {
            let context_length = self.engine.current_context_length();
            let vram_ratio = self.gpu_state.ratio();
            if compaction::should_compact(&self.history, context_length, vram_ratio) {
                let _ = tx
                    .send(InferenceEvent::Thought(
                        "Pre-turn compaction: context pressure detected — compacting history before inference.".into(),
                    ))
                    .await;
                if self
                    .compact_history_if_needed(&tx, Some(turn_anchor))
                    .await?
                {
                    // After compaction, history is [system, summary, user, ...].
                    // Recalculate the anchor so the in-loop compaction doesn't misfire.
                    turn_anchor = self
                        .history
                        .iter()
                        .rposition(|m| m.role == "user")
                        .unwrap_or(self.history.len().saturating_sub(1));
                }
            }
        }

        // Prevent Windows from sleeping during inference/tool execution.
        // Dropped automatically when the turn completes.
        let _sleep_guard = crate::ui::sleep_inhibitor::SleepInhibitor::acquire();

        // ── Context budget ledger — snapshot tokens before this turn ────────
        let (budget_input_start, budget_output_start) = {
            let econ = self
                .engine
                .economics
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            (econ.input_tokens, econ.output_tokens)
        };
        // Estimate existing history size before this turn (excludes system prompt).
        let budget_history_est: usize = self
            .history
            .iter()
            .take(turn_anchor)
            .map(crate::agent::inference::estimate_message_tokens)
            .sum();
        // Accumulates per-tool result costs (chars / 4) during the turn.
        let mut budget_tool_costs: Vec<crate::agent::economics::ToolCost> = Vec::with_capacity(8);

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

            let explicit_search_synthesis = explicit_search_request
                && grounded_research_results.is_some()
                && turn_tools.is_empty();

            let call_result = if explicit_search_synthesis {
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(20),
                    self.engine
                        .call_with_tools(&prompt_msgs, &turn_tools, routed_model.as_deref()),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => Err(
                        "explicit_search_synthesis_timeout: grounded research summary took too long to complete"
                            .to_string(),
                    ),
                }
            } else {
                self.engine
                    .call_with_tools(&prompt_msgs, &turn_tools, routed_model.as_deref())
                    .await
            };

            let (mut text, mut tool_calls, usage, finish_reason) = match call_result {
                Ok(result) => result,
                Err(e) => {
                    if explicit_search_synthesis
                        && (e.contains("explicit_search_synthesis_timeout")
                            || e.contains("provider_degraded")
                            || e.contains("empty response"))
                    {
                        if let Some(results) = grounded_research_results.as_deref() {
                            let response = build_research_provider_fallback(results);
                            self.history.push(ChatMessage::assistant_text(&response));
                            self.transcript.log_agent(&response);
                            let _ = tx
                                .send(InferenceEvent::Thought(
                                    "Search synthesis stalled; returning a grounded fallback summary from the fetched results."
                                        .into(),
                                ))
                                .await;
                            for chunk in chunk_text(&response, 8) {
                                let _ = tx.send(InferenceEvent::Token(chunk)).await;
                            }
                            let _ = tx.send(InferenceEvent::Done).await;
                            return Ok(());
                        }
                    }

                    let class = classify_runtime_failure(&e);
                    if should_retry_runtime_failure(class)
                        && self.recovery_context.consume_transient_retry()
                    {
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
                let mut results = Vec::with_capacity(calls.len());
                let gemma4_model =
                    crate::agent::inference::is_hematite_native_model(&self.engine.current_model());
                let latest_user_prompt = self.latest_user_prompt();
                let mut seen_call_keys = std::collections::HashSet::new();
                let mut deduped_calls = Vec::with_capacity(calls.len());
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
                                guard.on_file_access(std::path::Path::new(&path));
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
                                let mut err_results = Vec::with_capacity(calls.len());
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
                    let mut tasks = Vec::with_capacity(parallel_calls.len());
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
                        context_prep_ms,
                        inference_ms,
                        execution_ms,
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
                                // Re-index the just-edited file immediately so RAG results
                                // within the same turn reflect the new content, not the
                                // stale chunks from turn start.
                                if let Ok(meta) = std::fs::metadata(path) {
                                    if let Ok(mtime) = meta.modified() {
                                        let mtime_secs = mtime
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs()
                                            as i64;
                                        if let Ok(content) = std::fs::read_to_string(path) {
                                            let _ = tokio::task::block_in_place(|| {
                                                self.vein.index_document(path, mtime_secs, &content)
                                            });
                                        }
                                    }
                                }
                                self.l1_context = self.vein.l1_context();
                                // Compact stale read_file results for this path — the file
                                // just changed so old content is wrong and wastes context.
                                compact_stale_reads(&mut self.history, path);
                            }
                            // Refresh repo map so PageRank accounts for the new edit.
                            self.refresh_repo_map();
                            // Source changed — any cached build result is now stale.
                            crate::tools::verify_build::invalidate_build_cache();
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

                    if !should_suppress_recoverable_tool_result(
                        res.blocked_by_policy,
                        recoverable_policy_intervention.is_some(),
                    ) {
                        let _ = tx
                            .send(InferenceEvent::ToolCallResult {
                                id: call_id.clone(),
                                name: tool_name.clone(),
                                result: final_output.clone(),
                                is_error,
                            })
                            .await;
                    }

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
                    budget_tool_costs.push(crate::agent::economics::ToolCost {
                        name: tool_name.clone(),
                        tokens: capped.len() / 4,
                    });

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
                        && final_output
                            .contains("current-plan execution is locked to the saved target files")
                        && recoverable_policy_intervention.is_none()
                    {
                        let target_files = self
                            .session_memory
                            .current_plan
                            .as_ref()
                            .map(|plan| plan.target_files.clone())
                            .unwrap_or_default();
                        recoverable_policy_intervention =
                            Some(build_current_plan_scope_recovery_prompt(&target_files));
                        recoverable_policy_recipe = Some(RecoveryScenario::CurrentPlanScopeBlocked);
                        recoverable_policy_checkpoint = Some((
                            OperatorCheckpointState::BlockedPolicy,
                            format!(
                                "Current-plan execution blocked off-target path access via `{}`.",
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
                        context_prep_ms,
                        inference_ms,
                        execution_ms,
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
                    if let Some(summary) = maybe_deterministic_sovereign_closeout(
                        self.session_memory.current_plan.as_ref(),
                        mutation_occurred,
                    ) {
                        self.history.push(ChatMessage::assistant_text(&summary));
                        self.transcript.log_agent(&summary);
                        for chunk in chunk_text(&summary, 8) {
                            let _ = tx.send(InferenceEvent::Token(chunk)).await;
                        }
                        let _ = tx.send(InferenceEvent::Done).await;
                        return Ok(());
                    }

                    let last_was_tool = self
                        .history
                        .last()
                        .map(|m| m.role == "tool")
                        .unwrap_or(false);
                    if last_was_tool {
                        let fallback = "[Proof successful. See tool output above for results.]";
                        self.history.push(ChatMessage::assistant_text(fallback));
                        self.transcript.log_agent(fallback);
                        for chunk in chunk_text(fallback, 8) {
                            let _ = tx.send(InferenceEvent::Token(chunk)).await;
                        }
                        let _ = tx.send(InferenceEvent::Done).await;
                        return Ok(());
                    }

                    self.emit_runtime_failure(
                        &tx,
                        RuntimeFailureClass::EmptyModelResponse,
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

                if implement_current_plan
                    && mutation_occurred
                    && matches!(class, RuntimeFailureClass::EmptyModelResponse)
                {
                    if let Some(summary) = maybe_deterministic_sovereign_closeout(
                        self.session_memory.current_plan.as_ref(),
                        mutation_occurred,
                    ) {
                        self.history.push(ChatMessage::assistant_text(&summary));
                        self.transcript.log_agent(&summary);
                        for chunk in chunk_text(&summary, 8) {
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

        if implement_current_plan
            && !visible_closeout_emitted
            && turn_mutated_paths.is_empty()
            && current_plan_pass == 1
        {
            if let Some(progress) = task_progress_after.filter(|progress| progress.has_open_items())
            {
                let target_files = self
                    .session_memory
                    .current_plan
                    .as_ref()
                    .map(|plan| plan.target_files.clone())
                    .unwrap_or_default();
                let _ = tx
                    .send(InferenceEvent::Thought(
                        "No target files were mutated during the first current-plan pass. Forcing one grounded implementation retry before allowing summary mode."
                            .to_string(),
                    ))
                    .await;
                let synthetic_turn = UserTurn {
                    text: build_force_plan_mutation_prompt(progress, &target_files),
                    attached_document: None,
                    attached_image: None,
                };
                return Box::pin(self.run_turn(&synthetic_turn, tx.clone(), false)).await;
            }
        }

        if implement_current_plan
            && !visible_closeout_emitted
            && !turn_mutated_paths.is_empty()
            && current_plan_pass <= 2
        {
            if let (Some(before), Some(after)) = (task_progress_before, task_progress_after) {
                if after.has_open_items()
                    && after.remaining == before.remaining
                    && after.completed == before.completed
                {
                    let target_files = self
                        .session_memory
                        .current_plan
                        .as_ref()
                        .map(|plan| plan.target_files.clone())
                        .unwrap_or_default();
                    let _ = tx
                        .send(InferenceEvent::Thought(
                            "Implementation mutated target files, but the task ledger did not advance. Forcing one closeout pass to update `.hematite/TASK.md` before summary mode."
                                .to_string(),
                        ))
                        .await;
                    let synthetic_turn = UserTurn {
                        text: build_task_ledger_closeout_prompt(after, &target_files),
                        attached_document: None,
                        attached_image: None,
                    };
                    return Box::pin(self.run_turn(&synthetic_turn, tx.clone(), false)).await;
                }
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

        // ── Context budget ledger ────────────────────────────────────────────
        {
            let (input_end, output_end) = {
                let econ = self
                    .engine
                    .economics
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                (econ.input_tokens, econ.output_tokens)
            };
            let context_pct = {
                let ctx_len = self.engine.current_context_length();
                let total = input_end.saturating_sub(budget_input_start)
                    + output_end.saturating_sub(budget_output_start);
                (total * 100).checked_div(ctx_len).unwrap_or(0).min(100) as u8
            };
            // Collapse duplicate tool names into summed costs (insertion order preserved).
            let mut tool_costs: Vec<crate::agent::economics::ToolCost> =
                Vec::with_capacity(budget_tool_costs.len());
            for tc in &budget_tool_costs {
                if let Some(existing) = tool_costs.iter_mut().find(|e| e.name == tc.name) {
                    existing.tokens += tc.tokens;
                } else {
                    tool_costs.push(crate::agent::economics::ToolCost {
                        name: tc.name.clone(),
                        tokens: tc.tokens,
                    });
                }
            }
            let budget = crate::agent::economics::TurnBudget {
                input_tokens: input_end.saturating_sub(budget_input_start),
                output_tokens: output_end.saturating_sub(budget_output_start),
                history_est: budget_history_est,
                tool_costs,
                context_pct,
            };
            let _ = tx.send(InferenceEvent::Thought(budget.render())).await;
            self.last_turn_budget = Some(budget);
        }

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

        let mut sections = Vec::with_capacity(4);
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
                            if crate::tools::workspace_workflow::run_workspace_workflow(&start_args)
                                .await
                                .is_ok()
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
        let last_checkpoint = self.session_memory.last_checkpoint.take();
        let last_blocker = self.session_memory.last_blocker.take();
        let last_recovery = self.session_memory.last_recovery.take();
        let last_verification = self.session_memory.last_verification.take();
        let last_compaction = self.session_memory.last_compaction.take();
        self.session_memory = compaction::extract_memory(&self.history);
        self.session_memory.last_checkpoint = last_checkpoint;
        self.session_memory.last_blocker = last_blocker;
        self.session_memory.last_recovery = last_recovery;
        self.session_memory.last_verification = last_verification;
        self.session_memory.last_compaction = last_compaction;
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
        if query.split_whitespace().count() < 3 {
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
        let mut paths: Vec<String> = Vec::with_capacity(results.len());

        let mut total = 0usize;
        const MAX_CTX_CHARS: usize = 1_500;

        for r in results {
            if total >= MAX_CTX_CHARS {
                break;
            }
            let snippet = if r.content.len() > 500 {
                format!("{}...", safe_head(&r.content, 500))
            } else {
                r.content.clone()
            };
            let _ = write!(ctx, "--- {} ---\n{}\n\n", r.path, snippet);
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
        let mut result = Vec::with_capacity(self.history.len().saturating_sub(1));

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
        let mut result = Vec::with_capacity(self.history.len().saturating_sub(start_idx.max(1)));

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
    use std::sync::OnceLock;
    static RE_VERSION: OnceLock<regex::Regex> = OnceLock::new();
    static RE_BUMP: OnceLock<regex::Regex> = OnceLock::new();
    let re = match flag {
        "-Version" => RE_VERSION.get_or_init(|| {
            regex::Regex::new(r#"(?i)-Version\s+['"]?([^'" \r\n]+)['"]?"#).expect("valid")
        }),
        "-Bump" => RE_BUMP.get_or_init(|| {
            regex::Regex::new(r#"(?i)-Bump\s+['"]?([^'" \r\n]+)['"]?"#).expect("valid")
        }),
        other => {
            let pattern = format!(r#"(?i){}\s+['"]?([^'" \r\n]+)['"]?"#, regex::escape(other));
            return regex::Regex::new(&pattern).ok().and_then(|re| {
                re.captures(command)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
            });
        }
    };
    re.captures(command)?.get(1).map(|m| m.as_str().to_string())
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
        .trim_end_matches([':', '.'])
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

fn dns_quoted_re() -> &'static regex::Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r#"['"]([^'"]+)['"]"#).expect("valid"))
}

fn extract_dns_lookup_target_from_shell(command: &str) -> Option<String> {
    use std::sync::OnceLock;
    static RE1: OnceLock<regex::Regex> = OnceLock::new();
    static RE2: OnceLock<regex::Regex> = OnceLock::new();
    static RE3: OnceLock<regex::Regex> = OnceLock::new();
    let re1 = RE1.get_or_init(|| {
        regex::Regex::new(r#"(?i)-name\s+['"]?([^'"\s;()]+)['"]?"#).expect("valid")
    });
    let re2 = RE2.get_or_init(|| {
        regex::Regex::new(r#"(?i)(?:gethostaddresses|gethostentry)\s*\(\s*['"]([^'"]+)['"]\s*\)"#)
            .expect("valid")
    });
    let re3 = RE3.get_or_init(|| {
        regex::Regex::new(
            r#"(?i)\b(?:resolve-dnsname|nslookup|host|dig)\s+['"]?([^'"\s;()]+)['"]?"#,
        )
        .expect("valid")
    });
    for re in [re1, re2, re3] {
        if let Some(value) = re
            .captures(command)
            .and_then(|captures| captures.get(1).map(|m| clean_shell_dns_token(m.as_str())))
            .filter(|value| looks_like_dns_target(value))
        {
            return Some(value);
        }
    }

    let quoted = dns_quoted_re();
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
    let quoted = dns_quoted_re();
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
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r"(?i)\bevent(?:\s*_?\s*id)?\s*[:#]?\s*(\d{2,5})\b").expect("valid")
    });
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
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let lower = text.to_ascii_lowercase();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r"(?i)\b(?:last|past)\s+(\d{1,3})\s*(hour|hours|hr|hrs)\b")
            .expect("valid")
    });
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
    if let Some(obj) = args.as_object_mut() {
        if topic == "dns_lookup" {
            if let Some(name) = extract_dns_lookup_target_from_text(prompt) {
                obj.insert("name".to_string(), Value::String(name));
            }
            let record_type = extract_dns_record_type_from_text(prompt).unwrap_or("A");
            obj.insert("type".to_string(), Value::String(record_type.to_string()));
        } else if topic == "event_query" {
            if let Some(event_id) = extract_event_query_event_id_from_text(prompt) {
                obj.insert(
                    "event_id".to_string(),
                    Value::Number(serde_json::Number::from(event_id)),
                );
            }
            if let Some(log_name) = extract_event_query_log_from_text(prompt) {
                obj.insert("log".to_string(), Value::String(log_name.to_string()));
            }
            if let Some(level) = extract_event_query_level_from_text(prompt) {
                obj.insert("level".to_string(), Value::String(level.to_string()));
            }
            if let Some(hours) = extract_event_query_hours_from_text(prompt) {
                obj.insert(
                    "hours".to_string(),
                    Value::Number(serde_json::Number::from(hours)),
                );
            }
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
            use std::sync::OnceLock;
            static SEMVER_RE: OnceLock<regex::Regex> = OnceLock::new();
            let version = SEMVER_RE
                .get_or_init(|| regex::Regex::new(r#"(?i)\b(\d+\.\d+\.\d+)\b"#).expect("valid"))
                .captures(prompt)
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
        let mut msg_results = Vec::with_capacity(2);
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
                .and_then(|m| m.content.as_str().split('\n').next_back()),
        );
        call.name = normalized_name;
        let last_user_prompt = self
            .history
            .iter()
            .rev()
            .find(|message| message.role == "user")
            .map(|message| message.content.as_str());
        rewrite_host_tool_call(&mut call.name, &mut args, last_user_prompt);
        if self
            .plan_execution_active
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            let fallback_target = self
                .session_memory
                .current_plan
                .as_ref()
                .and_then(|plan| plan.target_files.first().map(String::as_str));
            let explicit_query = last_user_prompt.and_then(extract_explicit_web_search_query);
            if let Some((repaired_args, note)) = repaired_plan_tool_args(
                &call.name,
                &args,
                std::path::Path::new(".hematite/TASK.md").exists(),
                fallback_target,
                explicit_query.as_deref(),
            ) {
                args = repaired_args;
                let _ = tx.send(InferenceEvent::Thought(note)).await;
            }
        }

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
                        let mut pinned = Vec::with_capacity(arr.len().min(3));
                        {
                            let mut guard = self.pinned_files.write().await;
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
                        let pinned = self.pinned_files.read().await;
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
                        task_objs.reserve(arr.len());
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
                } else if call.name == "refactor_rename"
                    && !args
                        .get("dry_run")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true)
                    && !yolo
                {
                    // ── Refactor rename live-apply gate ───────────────────────
                    // Run a dry-run pass to generate the full per-file preview,
                    // then surface it in the approval modal so the operator sees
                    // every line that will change before committing.
                    let mut preview_args = args.clone();
                    preview_args["dry_run"] = serde_json::json!(true);
                    let preview = crate::tools::refactor::execute_rename(&preview_args).await;
                    match preview {
                        Ok(diff_text) => {
                            let old = args.get("old_name").and_then(|v| v.as_str()).unwrap_or("?");
                            let new = args.get("new_name").and_then(|v| v.as_str()).unwrap_or("?");
                            let (appr_tx, appr_rx) = tokio::sync::oneshot::channel::<bool>();
                            let mutation_label =
                                crate::agent::tool_registry::get_mutation_label(&call.name, &args);
                            let _ = tx
                                .send(InferenceEvent::ApprovalRequired {
                                    id: real_id.clone(),
                                    name: call.name.clone(),
                                    display: format!("Rename: {old} → {new}"),
                                    diff: Some(diff_text),
                                    mutation_label,
                                    responder: appr_tx,
                                })
                                .await;
                            match appr_rx.await {
                                Ok(true) => crate::tools::refactor::execute_rename(&args).await,
                                _ => Err("Rename declined by user.".into()),
                            }
                        }
                        Err(e) => {
                            // Preview failed — still run with the full apply so the error surfaces.
                            Err(format!("refactor_rename preview failed: {e}"))
                        }
                    }
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
                } else if call.name == "vein_search" {
                    // Direct Vein query — returns relevant code/session chunks from the
                    // local RAG index without waiting for the next turn's pre-retrieval pass.
                    let query = args
                        .get("query")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim();
                    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(8) as usize;
                    if query.is_empty() {
                        Err("vein_search: missing required 'query'".to_string())
                    } else {
                        let capped_limit = limit.min(20);
                        let results = tokio::task::block_in_place(|| {
                            self.vein.search_context(query, capped_limit)
                        });
                        match results {
                            Ok(hits) if hits.is_empty() => Ok(format!(
                                "vein_search: no results for \"{query}\" \
                                 (index may be empty or embedding model not loaded)"
                            )),
                            Ok(hits) => {
                                let mut out = format!(
                                    "VEIN SEARCH: \"{query}\" — {} result(s)\n\n",
                                    hits.len()
                                );
                                for (i, r) in hits.iter().enumerate() {
                                    out.push_str(&format!(
                                        "── [{i}] {} (room: {}, score: {:.2}) ──\n{}\n\n",
                                        r.path,
                                        r.room,
                                        r.score,
                                        r.content.trim()
                                    ));
                                }
                                Ok(out)
                            }
                            Err(e) => Err(format!("vein_search: index error: {e}")),
                        }
                    }
                } else {
                    dispatch_tool(&call.name, &args, &config, budget_tokens).await
                };

                match result {
                    Ok(o) => (o, false),
                    Err(e) => {
                        // Auto-enrich verify_build failures with structured error list.
                        // Runs cargo_errors as a quick secondary pass so the model sees
                        // file:line [Exxxx]: message entries without needing an extra turn.
                        let enriched = if call.name == "verify_build" && e.contains("BUILD FAILED")
                        {
                            let structured = crate::tools::build_errors::execute(
                                &serde_json::json!({"tests": e.contains(":test")}),
                            )
                            .await;
                            match structured {
                                Ok(s) if s.contains("ERRORS") => {
                                    format!("STRUCTURED ERRORS:\n{s}\n\nFULL OUTPUT:\nError: {e}")
                                }
                                _ => format!("Error: {e}"),
                            }
                        } else {
                            format!("Error: {e}")
                        };
                        (enriched, true)
                    }
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
    let get = |key: &str| -> &str { args.get(key).and_then(|v| v.as_str()).unwrap_or("") };
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
                format!("{}... (truncated)", safe_head(&rep, 100))
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
    let mut tool_indices: Vec<usize> = {
        let mut v = Vec::with_capacity(prompt_msgs.len());
        v.extend(
            prompt_msgs
                .iter()
                .enumerate()
                .filter_map(|(idx, msg)| (msg.role == "tool").then_some(idx)),
        );
        v
    };
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
    tool_indices.clear();
    tool_indices.extend(
        prompt_msgs
            .iter()
            .enumerate()
            .filter_map(|(idx, msg)| (msg.role == "tool").then_some(idx)),
    );
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
    let avg_word = 6usize;
    let mut chunks = Vec::with_capacity(text.len() / (words_per_chunk * avg_word).max(1) + 1);
    let mut current = String::with_capacity(words_per_chunk * avg_word);
    let mut count = 0;

    for ch in text.chars() {
        current.push(ch);
        if ch == ' ' || ch == '\n' {
            count += 1;
            if count >= words_per_chunk {
                chunks.push(std::mem::take(&mut current));
                current = String::with_capacity(words_per_chunk * avg_word);
                count = 0;
            }
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn repaired_plan_tool_args(
    tool_name: &str,
    args: &Value,
    task_file_exists: bool,
    fallback_target: Option<&str>,
    explicit_query: Option<&str>,
) -> Option<(Value, String)> {
    match tool_name {
        "read_file" | "inspect_lines" => {
            let has_path = args
                .as_object()
                .and_then(|map| map.get("path"))
                .and_then(|v| v.as_str())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if has_path {
                return None;
            }

            let target = if task_file_exists {
                Some(".hematite/TASK.md")
            } else {
                fallback_target
            }?;
            let mut repaired = if args.is_object() {
                args.clone()
            } else {
                Value::Object(serde_json::Map::new())
            };
            let map = repaired.as_object_mut()?;
            map.insert("path".to_string(), Value::String(target.to_string()));
            Some((
                repaired,
                format!(
                    "Recovered malformed `{}` call during current-plan execution by grounding it to `{}`.",
                    tool_name, target
                ),
            ))
        }
        "research_web" => {
            let has_query = args
                .as_object()
                .and_then(|map| map.get("query"))
                .and_then(|v| v.as_str())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if has_query {
                return None;
            }
            let query = explicit_query?.trim();
            if query.is_empty() {
                return None;
            }
            let mut repaired = if args.is_object() {
                args.clone()
            } else {
                Value::Object(serde_json::Map::new())
            };
            let map = repaired.as_object_mut()?;
            map.insert("query".to_string(), Value::String(query.to_string()));
            Some((
                repaired,
                format!(
                    "Recovered malformed `research_web` call during current-plan execution by restoring query `{}`.",
                    query
                ),
            ))
        }
        _ => None,
    }
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
    let _ = writeln!(system_msg, "CURRENT MODE: {}", mode_label);

    if config.mode == crate::agent::config::PermissionMode::ReadOnly {
        system_msg.push_str("PERMISSION: You are restricted to READ-ONLY access. Do NOT attempt to use write_file, edit_file, or shell for any modification. Focus entirely on analysis, indexing, and reporting.\n");
    } else {
        system_msg.push_str("PERMISSION: You have authority to modify code and execute tests with user oversight.\n");
    }

    // Inject live hardware status.
    let (used, total) = gpu.read();
    if total > 0 {
        system_msg.push_str("\n\n# Terminal Hardware Context\n");
        let _ = writeln!(
            system_msg,
            "HOST GPU: {} | VRAM: {:.1}GB / {:.1}GB ({:.0}% used)",
            gpu.gpu_name(),
            used as f64 / 1024.0,
            total as f64 / 1024.0,
            gpu.ratio() * 100.0
        );
        system_msg.push_str("Use this awareness to manage your context window responsibly.\n");
    }

    // Inject Git Repository context.
    system_msg.push_str("\n\n# Git Repository Context\n");
    let git_status_label = git.label();
    let git_url = git.url();
    let _ = writeln!(
        system_msg,
        "REMOTE STATUS: {} | URL: {}",
        git_status_label, git_url
    );

    // Live Snapshots (Status/Diff)
    let root = crate::tools::file_ops::workspace_root();
    if let Some(status_snapshot) = crate::agent::git_context::read_git_status(&root) {
        system_msg.push_str("\nGit status snapshot:\n");
        system_msg.push_str(&status_snapshot);
        system_msg.push('\n');
    }

    if let Some(diff_snapshot) = crate::agent::git_context::read_git_diff(&root, 2000) {
        system_msg.push_str("\nGit diff snapshot:\n");
        system_msg.push_str(&diff_snapshot);
        system_msg.push('\n');
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
        let _ = writeln!(system_msg, "- {}", hint);
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
    fn auto_research_handover_is_turn_scoped_only() {
        assert!(should_use_turn_scoped_investigation_mode(
            WorkflowMode::Auto,
            QueryIntentClass::Research
        ));
        assert!(!should_use_turn_scoped_investigation_mode(
            WorkflowMode::Ask,
            QueryIntentClass::Research
        ));
        assert!(!should_use_turn_scoped_investigation_mode(
            WorkflowMode::Auto,
            QueryIntentClass::RepoArchitecture
        ));
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
    fn single_file_html_sovereign_targets_only_index() {
        let targets = default_sovereign_scaffold_targets(
            "google uefn toolbelt then make a folder on my desktop called yourtask and inside it create a single index.html that explains what you found",
        );

        assert!(targets.contains("index.html"));
        assert!(!targets.contains("style.css"));
        assert!(!targets.contains("script.js"));
    }

    #[test]
    fn single_file_html_handoff_verification_mentions_self_contained_index() {
        let mut targets = std::collections::BTreeSet::new();
        targets.insert("index.html".to_string());
        let plan = build_sovereign_scaffold_handoff(
            "google uefn toolbelt then make a folder on my desktop called yourtask and inside it create a single index.html that explains what you found",
            &targets,
        );

        assert!(plan.verification.contains("index.html"));
        assert!(plan.verification.contains("self-contained"));
        assert!(plan
            .ordered_steps
            .iter()
            .any(|step| step.contains("single `index.html` file")));
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

        // Use ends_with instead of contains(&normalize_workspace_path(...)) to avoid a
        // race condition: normalize_workspace_path reads current_dir(), which concurrent
        // tests that call set_current_dir() can change between the two call sites.
        assert!(allowed.iter().any(|p| p.ends_with("/src/main.rs")));
        assert!(allowed
            .iter()
            .any(|path| path.ends_with("/.hematite/task.md")));
        assert!(allowed
            .iter()
            .any(|path| path.ends_with("/.hematite/plan.md")));
    }

    #[test]
    fn repaired_plan_tool_args_recovers_empty_read_to_task_ledger() {
        let args = serde_json::json!({});
        let (repaired, note) =
            repaired_plan_tool_args("read_file", &args, true, Some("index.html"), None).unwrap();

        assert_eq!(
            repaired.get("path").and_then(|v| v.as_str()),
            Some(".hematite/TASK.md")
        );
        assert!(note.contains(".hematite/TASK.md"));
    }

    #[test]
    fn repaired_plan_tool_args_recovers_empty_research_query() {
        let args = serde_json::json!({});
        let (repaired, note) = repaired_plan_tool_args(
            "research_web",
            &args,
            true,
            Some("index.html"),
            Some("uefn toolbelt"),
        )
        .unwrap();

        assert_eq!(
            repaired.get("query").and_then(|v| v.as_str()),
            Some("uefn toolbelt")
        );
        assert!(note.contains("uefn toolbelt"));
    }

    #[test]
    fn repaired_plan_tool_args_recovers_non_object_read_call() {
        let args = serde_json::json!("");
        let (repaired, _) =
            repaired_plan_tool_args("read_file", &args, true, Some("index.html"), None).unwrap();

        assert_eq!(
            repaired.get("path").and_then(|v| v.as_str()),
            Some(".hematite/TASK.md")
        );
    }

    #[test]
    fn force_plan_mutation_prompt_names_target_files() {
        let prompt = build_force_plan_mutation_prompt(
            TaskChecklistProgress {
                total: 5,
                completed: 0,
                remaining: 5,
            },
            &["index.html".to_string()],
        );

        assert!(prompt.contains(".hematite/TASK.md"));
        assert!(prompt.contains("`index.html`"));
        assert!(prompt.contains("Do not summarize"));
    }

    #[test]
    fn current_plan_scope_recovery_prompt_names_saved_targets() {
        let prompt = build_current_plan_scope_recovery_prompt(&["index.html".to_string()]);

        assert!(prompt.contains("`index.html`"));
        assert!(prompt.contains(".hematite/TASK.md"));
        assert!(prompt.contains("Do not branch into unrelated files"));
    }

    #[test]
    fn task_ledger_closeout_prompt_demands_checklist_update() {
        let prompt = build_task_ledger_closeout_prompt(
            TaskChecklistProgress {
                total: 5,
                completed: 0,
                remaining: 5,
            },
            &["index.html".to_string()],
        );

        assert!(prompt.contains(".hematite/TASK.md"));
        assert!(prompt.contains("`index.html`"));
        assert!(prompt.contains("Do not summarize"));
        assert!(prompt.contains("`[x]`"));
    }

    #[test]
    fn suppresses_recoverable_blocked_tool_result_only_when_redirect_exists() {
        assert!(should_suppress_recoverable_tool_result(true, true));
        assert!(!should_suppress_recoverable_tool_result(true, false));
        assert!(!should_suppress_recoverable_tool_result(false, true));
    }

    #[test]
    fn sovereign_closeout_detects_materialized_targets() {
        let _cwd_lock = crate::TEST_CWD_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let previous = env!("CARGO_MANIFEST_DIR");
        std::env::set_current_dir(temp.path()).unwrap();
        std::fs::write("index.html", "<html>ok</html>").unwrap();

        assert!(target_files_materialized(&["index.html".to_string()]));

        std::env::set_current_dir(previous).unwrap();
    }

    #[test]
    fn deterministic_sovereign_closeout_returns_summary_when_targets_exist() {
        let _cwd_lock = crate::TEST_CWD_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let previous = env!("CARGO_MANIFEST_DIR");
        std::env::set_current_dir(temp.path()).unwrap();
        std::fs::create_dir_all(".hematite").unwrap();
        std::fs::write("index.html", "<html>ok</html>").unwrap();
        std::fs::write(".hematite/TASK.md", "# Task Ledger\n\n- [ ] Build index\n").unwrap();
        std::fs::write(".hematite/WALKTHROUGH.md", "").unwrap();

        let plan = crate::tools::plan::PlanHandoff {
            goal: "Continue the sovereign scaffold task in this new project root".to_string(),
            target_files: vec!["index.html".to_string()],
            ordered_steps: vec!["Build index".to_string()],
            verification: "Open index.html".to_string(),
            risks: vec![],
            open_questions: vec![],
        };

        let summary = maybe_deterministic_sovereign_closeout(Some(&plan), true).unwrap();
        let task = std::fs::read_to_string(".hematite/TASK.md").unwrap();

        std::env::set_current_dir(previous).unwrap();

        assert!(summary.contains("Sovereign Scaffold Task Complete"));
        assert!(task.contains("- [x] Build index"));
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
