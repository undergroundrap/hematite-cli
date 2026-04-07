use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, Semaphore};

pub use crate::agent::economics::{SessionEconomics, ToolRecord};

// ── Engine ────────────────────────────────────────────────────────────────────

pub struct InferenceEngine {
    pub client: reqwest::Client,
    pub api_url: String,
    /// Root URL of the LLM provider (e.g. `http://localhost:1234`).
    /// All non-completions endpoints (models list, health, embeddings) are derived from this.
    pub base_url: String,
    pub species: String,
    pub snark: u8,
    pub kv_semaphore: Semaphore,
    /// The model ID currently loaded in LM Studio (auto-detected on boot).
    pub model: std::sync::RwLock<String>,
    /// Context window length in tokens (auto-detected from LM Studio, default 32768).
    pub context_length: std::sync::atomic::AtomicUsize,
    pub economics: std::sync::Arc<std::sync::Mutex<SessionEconomics>>,
    /// Optional model ID for worker-level tasks (Swarms / research).
    pub worker_model: Option<String>,
    /// Opt-in Gemma-native request shaping. Off by default.
    pub gemma_native_formatting: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Global cancellation token for hard-interrupting the inference stream.
    pub cancel_token: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub fn is_gemma4_model_name(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    lower.contains("gemma-4") || lower.contains("gemma4")
}

fn should_use_gemma_native_formatting(
    engine: &InferenceEngine,
    model: &str,
) -> bool {
    is_gemma4_model_name(model) && engine.gemma_native_formatting_enabled()
}

// ── OpenAI Tool Definition ────────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
    #[serde(skip_serializing, skip_deserializing)]
    pub metadata: ToolMetadata,
}

#[derive(Serialize, Clone, Debug)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolCategory {
    RepoRead,
    RepoWrite,
    Runtime,
    Architecture,
    Toolchain,
    Verification,
    Git,
    Research,
    Vision,
    Lsp,
    Workflow,
    External,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolMetadata {
    pub category: ToolCategory,
    pub mutates_workspace: bool,
    pub external_surface: bool,
    pub trust_sensitive: bool,
    pub read_only_friendly: bool,
    pub plan_scope: bool,
}

pub fn tool_metadata_for_name(name: &str) -> ToolMetadata {
    if name.starts_with("mcp__") {
        let lower = name.to_ascii_lowercase();
        let mutates_workspace = [
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
        .any(|needle| lower.contains(needle));
        return ToolMetadata {
            category: ToolCategory::External,
            mutates_workspace,
            external_surface: true,
            trust_sensitive: true,
            read_only_friendly: !mutates_workspace,
            plan_scope: false,
        };
    }

    match name {
        "read_file" | "inspect_lines" | "grep_files" | "list_files" => ToolMetadata {
            category: ToolCategory::RepoRead,
            mutates_workspace: false,
            external_surface: false,
            trust_sensitive: false,
            read_only_friendly: true,
            plan_scope: true,
        },
        "write_file" | "edit_file" | "patch_hunk" | "multi_search_replace" => ToolMetadata {
            category: ToolCategory::RepoWrite,
            mutates_workspace: true,
            external_surface: false,
            trust_sensitive: true,
            read_only_friendly: false,
            plan_scope: true,
        },
        "map_project" | "trace_runtime_flow" => ToolMetadata {
            category: ToolCategory::Architecture,
            mutates_workspace: false,
            external_surface: false,
            trust_sensitive: false,
            read_only_friendly: true,
            plan_scope: false,
        },
        "describe_toolchain" => ToolMetadata {
            category: ToolCategory::Toolchain,
            mutates_workspace: false,
            external_surface: false,
            trust_sensitive: false,
            read_only_friendly: true,
            plan_scope: false,
        },
        "shell" => ToolMetadata {
            category: ToolCategory::Runtime,
            mutates_workspace: true,
            external_surface: false,
            trust_sensitive: true,
            read_only_friendly: false,
            plan_scope: false,
        },
        "verify_build" => ToolMetadata {
            category: ToolCategory::Verification,
            mutates_workspace: false,
            external_surface: false,
            trust_sensitive: false,
            read_only_friendly: true,
            plan_scope: false,
        },
        "git_commit" | "git_push" | "git_remote" | "git_onboarding" | "git_worktree" => {
            ToolMetadata {
                category: ToolCategory::Git,
                mutates_workspace: true,
                external_surface: false,
                trust_sensitive: true,
                read_only_friendly: false,
                plan_scope: false,
            }
        }
        "research_web" | "fetch_docs" => ToolMetadata {
            category: ToolCategory::Research,
            mutates_workspace: false,
            external_surface: false,
            trust_sensitive: false,
            read_only_friendly: true,
            plan_scope: false,
        },
        "vision_analyze" => ToolMetadata {
            category: ToolCategory::Vision,
            mutates_workspace: false,
            external_surface: false,
            trust_sensitive: false,
            read_only_friendly: true,
            plan_scope: false,
        },
        "lsp_definitions"
        | "lsp_references"
        | "lsp_hover"
        | "lsp_rename_symbol"
        | "lsp_get_diagnostics"
        | "lsp_search_symbol" => ToolMetadata {
            category: ToolCategory::Lsp,
            mutates_workspace: false,
            external_surface: false,
            trust_sensitive: false,
            read_only_friendly: true,
            plan_scope: false,
        },
        "auto_pin_context" | "list_pinned" | "clarify" => ToolMetadata {
            category: ToolCategory::Workflow,
            mutates_workspace: false,
            external_surface: false,
            trust_sensitive: false,
            read_only_friendly: true,
            plan_scope: true,
        },
        "manage_tasks" => ToolMetadata {
            category: ToolCategory::Workflow,
            mutates_workspace: false,
            external_surface: false,
            trust_sensitive: false,
            read_only_friendly: true,
            plan_scope: false,
        },
        _ => ToolMetadata {
            category: ToolCategory::Other,
            mutates_workspace: false,
            external_surface: false,
            trust_sensitive: false,
            read_only_friendly: true,
            plan_scope: false,
        },
    }
}

// ── Message types ─────────────────────────────────────────────────────────────

/// OpenAI-compatible chat message. Content can be a string (legacy) or a
/// Vec of ContentPart (multimodal).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    /// Support both simple string content and complex multi-part content (Vision).
    pub content: MessageContent,
    /// Assistant messages may have tool calls. Default to empty vec, not null.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCallResponse>,
    /// Tool message references the original call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Tool message name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlSource },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImageUrlSource {
    pub url: String,
}

impl Default for MessageContent {
    fn default() -> Self {
        MessageContent::Text(String::new())
    }
}

impl MessageContent {
    pub fn as_str(&self) -> &str {
        match self {
            MessageContent::Text(s) => s,
            MessageContent::Parts(parts) => {
                for part in parts {
                    if let ContentPart::Text { text } = part {
                        return text;
                    }
                }
                ""
            }
        }
    }
}

impl ChatMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".into(),
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }
    }
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".into(),
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }
    }
    pub fn user_with_image(text: &str, image_url: &str) -> Self {
        let mut text_parts = text.to_string();
        if !text_parts.contains("<|image|>") {
            text_parts.push_str(" <|image|>");
        }
        Self {
            role: "user".into(),
            content: MessageContent::Parts(vec![
                ContentPart::Text { text: text_parts },
                ContentPart::ImageUrl {
                    image_url: ImageUrlSource {
                        url: image_url.into(),
                    },
                },
            ]),
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }
    }
    pub fn assistant_text(content: &str) -> Self {
        Self {
            role: "assistant".into(),
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }
    }
    pub fn assistant_tool_calls(content: &str, calls: Vec<ToolCallResponse>) -> Self {
        Self {
            role: "assistant".into(),
            content: MessageContent::Text(content.into()),
            tool_calls: calls,
            tool_call_id: None,
            name: None,
        }
    }
    pub fn tool_result(tool_call_id: &str, fn_name: &str, content: &str) -> Self {
        Self::tool_result_for_model(tool_call_id, fn_name, content, "")
    }

    /// Build a tool result message, applying Gemma 4 native markup only when the
    /// loaded model is actually a Gemma 4 model.
    pub fn tool_result_for_model(
        tool_call_id: &str,
        fn_name: &str,
        content: &str,
        model: &str,
    ) -> Self {
        let body = if is_gemma4_model_name(model) {
            format!(
                "<|tool_response>response:{}{}{}<tool_response|>",
                fn_name, "{", content
            )
        } else {
            content.to_string()
        };
        Self {
            role: "tool".into(),
            content: MessageContent::Text(body),
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
            name: Some(fn_name.into()),
        }
    }
}

// ── Tool call as returned by the model ───────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCallResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFn,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCallFn {
    pub name: String,
    /// JSON-encoded arguments string (as returned by the API).
    pub arguments: String,
}

// ── HTTP request / response shapes ───────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<ResponseChoice>,
    usage: Option<TokenUsage>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    #[serde(default)]
    pub prompt_cache_hit_tokens: usize,
    #[serde(default)]
    pub cache_read_input_tokens: usize,
}

#[derive(Deserialize, Debug)]
struct ResponseChoice {
    message: ResponseMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCallResponse>>,
}

const MIN_RESERVED_OUTPUT_TOKENS: usize = 1024;
const MAX_RESERVED_OUTPUT_TOKENS: usize = 4096;

fn is_tiny_context_window(context_length: usize) -> bool {
    context_length <= 8_192
}

fn is_compact_context_window(context_length: usize) -> bool {
    context_length > 8_192 && context_length <= 49_152
}

pub fn is_compact_context_window_pub(context_length: usize) -> bool {
    is_compact_context_window(context_length)
}

fn is_provider_context_limit_detail(lower: &str) -> bool {
    (lower.contains("n_keep") && lower.contains("n_ctx"))
        || lower.contains("context length")
        || lower.contains("keep from the initial prompt")
        || lower.contains("prompt is greater than the context length")
        || lower.contains("exceeds the context window")
}

fn classify_runtime_failure_tag(detail: &str) -> &'static str {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("context_window_blocked")
        || lower.contains("context ceiling reached")
        || lower.contains("exceeds the")
        || is_provider_context_limit_detail(&lower)
    {
        "context_window"
    } else if lower.contains("empty response from model")
        || lower.contains("model returned an empty response")
    {
        "empty_model_response"
    } else if lower.contains("action blocked:")
        || lower.contains("access denied")
        || lower.contains("declined by user")
    {
        "tool_policy_blocked"
    } else {
        "provider_degraded"
    }
}

fn runtime_failure_guidance(tag: &str) -> &'static str {
    match tag {
        "context_window" => {
            "Narrow the request, compact the session, or preserve grounded tool output instead of restyling it. If LM Studio reports a smaller live n_ctx than Hematite expected, reload or re-detect the model budget before retrying."
        }
        "empty_model_response" => {
            "Retry once automatically, then narrow the turn or restart LM Studio if the model keeps returning nothing."
        }
        "tool_policy_blocked" => {
            "Stay inside the allowed workflow or switch modes before retrying."
        }
        _ => "Retry once automatically, then narrow the turn or restart LM Studio if it persists.",
    }
}

fn format_runtime_failure_message(detail: &str) -> String {
    let tag = classify_runtime_failure_tag(detail);
    format!(
        "[failure:{}] {} Detail: {}",
        tag,
        runtime_failure_guidance(tag),
        detail.trim()
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRuntimeState {
    Booting,
    Live,
    Recovering,
    Degraded,
    ContextWindow,
    EmptyResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpRuntimeState {
    Unconfigured,
    Healthy,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorCheckpointState {
    Idle,
    RecoveringProvider,
    BudgetReduced,
    HistoryCompacted,
    BlockedContextWindow,
    BlockedPolicy,
    BlockedRecentFileEvidence,
    BlockedExactLineWindow,
    BlockedToolLoop,
    BlockedVerification,
}

impl OperatorCheckpointState {
    pub fn label(self) -> &'static str {
        match self {
            OperatorCheckpointState::Idle => "idle",
            OperatorCheckpointState::RecoveringProvider => "recovering_provider",
            OperatorCheckpointState::BudgetReduced => "budget_reduced",
            OperatorCheckpointState::HistoryCompacted => "history_compacted",
            OperatorCheckpointState::BlockedContextWindow => "blocked_context_window",
            OperatorCheckpointState::BlockedPolicy => "blocked_policy",
            OperatorCheckpointState::BlockedRecentFileEvidence => {
                "blocked_recent_file_evidence"
            }
            OperatorCheckpointState::BlockedExactLineWindow => "blocked_exact_line_window",
            OperatorCheckpointState::BlockedToolLoop => "blocked_tool_loop",
            OperatorCheckpointState::BlockedVerification => "blocked_verification",
        }
    }
}

fn provider_state_for_failure_tag(tag: &str) -> ProviderRuntimeState {
    match tag {
        "context_window" => ProviderRuntimeState::ContextWindow,
        "empty_model_response" => ProviderRuntimeState::EmptyResponse,
        _ => ProviderRuntimeState::Degraded,
    }
}

fn compact_runtime_failure_summary(tag: &str, detail: &str) -> String {
    match tag {
        "context_window" => {
            "LM Studio context ceiling hit; narrow the turn or refresh the live runtime budget."
                .to_string()
        }
        "empty_model_response" => {
            "LM Studio returned an empty reply; Hematite will retry once before surfacing a failure."
                .to_string()
        }
        "tool_policy_blocked" => {
            "A blocked tool path was rejected; stay inside the allowed workflow before retrying."
                .to_string()
        }
        _ => {
            let mut excerpt = detail
                .split_whitespace()
                .take(12)
                .collect::<Vec<_>>()
                .join(" ");
            if excerpt.len() > 110 {
                excerpt.truncate(110);
                excerpt.push_str("...");
            }
            if excerpt.is_empty() {
                "LM Studio degraded; Hematite will retry once before surfacing a failure."
                    .to_string()
            } else {
                format!("LM Studio degraded: {}", excerpt)
            }
        }
    }
}

// ── Events pushed to the TUI ──────────────────────────────────────────────────

#[derive(Debug)]
pub enum InferenceEvent {
    /// A text token to append to the current assistant message.
    Token(String),
    /// A text token to be displayed on screen but NOT spoken (e.g. startup greeting).
    MutedToken(String),
    /// Internal model reasoning (shown in side panel, not dialogue).
    Thought(String),
    /// Critical diagnostic feedback from the voice synthesis engine.
    VoiceStatus(String),
    /// A tool call is starting – show a status line in the TUI.
    ToolCallStart {
        id: String,
        name: String,
        args: String,
    },
    /// A tool call completed – show result in the TUI.
    ToolCallResult {
        id: String,
        name: String,
        output: String,
        is_error: bool,
    },
    /// A risky tool requires explicit user approval.
    /// The TUI must send `true` (approved) or `false` (rejected) via `responder`.
    ApprovalRequired {
        id: String,
        name: String,
        display: String,
        responder: tokio::sync::oneshot::Sender<bool>,
    },
    /// The current agent turn is complete.
    Done,
    /// An error occurred during inference.
    Error(String),
    /// Compact provider/runtime state for the operator surface.
    ProviderStatus {
        state: ProviderRuntimeState,
        summary: String,
    },
    /// Typed operator checkpoint/blocker state for SPECULAR and recovery UIs.
    OperatorCheckpoint {
        state: OperatorCheckpointState,
        summary: String,
    },
    /// Typed recovery recipe summary for operator/debug surfaces.
    RecoveryRecipe {
        summary: String,
    },
    /// Compact MCP/runtime server health for the operator surface.
    McpStatus {
        state: McpRuntimeState,
        summary: String,
    },
    /// Current compaction pressure against the adaptive threshold.
    CompactionPressure {
        estimated_tokens: usize,
        threshold_tokens: usize,
        percent: u8,
    },
    /// Current total prompt-budget pressure against the live context window.
    PromptPressure {
        estimated_input_tokens: usize,
        reserved_output_tokens: usize,
        estimated_total_tokens: usize,
        context_length: usize,
        percent: u8,
    },
    /// A generic task progress update (e.g. for single-agent tool execution).
    TaskProgress {
        id: String,
        label: String,
        progress: u8,
    },
    /// Real-time token usage update from the API.
    UsageUpdate(TokenUsage),
    /// The current runtime profile detected from LM Studio.
    RuntimeProfile { model_id: String, context_length: usize },
    /// Vein index status after each incremental re-index.
    VeinStatus { file_count: usize, embedded_count: usize },
    /// File paths the Vein surfaced as relevant to the current turn.
    /// Used to populate ACTIVE CONTEXT with retrieval results.
    VeinContext { paths: Vec<String> },
}

// ── Engine implementation ─────────────────────────────────────────────────────

impl InferenceEngine {
    pub fn new(
        api_url: String,
        species: String,
        snark: u8,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()?;

        // Extract http://host:port as the base for all non-completions endpoints.
        let base_url = {
            let trimmed = api_url.trim_end_matches('/');
            if let Some(scheme_end) = trimmed.find("://") {
                let after_scheme = &trimmed[scheme_end + 3..];
                if let Some(path_start) = after_scheme.find('/') {
                    format!("{}://{}", &trimmed[..scheme_end], &after_scheme[..path_start])
                } else {
                    trimmed.to_string()
                }
            } else {
                trimmed.to_string()
            }
        };

        let api_url = if api_url.ends_with("/chat/completions") {
            api_url
        } else if api_url.ends_with("/") {
            format!("{}chat/completions", api_url)
        } else {
            format!("{}/chat/completions", api_url)
        };

        Ok(Self {
            client,
            api_url,
            base_url,
            species,
            snark,
            kv_semaphore: Semaphore::new(3),
            model: std::sync::RwLock::new(String::new()),
            context_length: std::sync::atomic::AtomicUsize::new(32_768), // Gemma-4 Sweet Spot (32K)
            economics: std::sync::Arc::new(std::sync::Mutex::new(SessionEconomics::new())),
            worker_model: None,
            gemma_native_formatting: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            cancel_token: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    pub fn set_gemma_native_formatting(&self, enabled: bool) {
        self.gemma_native_formatting
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn gemma_native_formatting_enabled(&self) -> bool {
        self.gemma_native_formatting
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn current_model(&self) -> String {
        self.model
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn current_context_length(&self) -> usize {
        self.context_length
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn set_runtime_profile(&self, model: &str, context_length: usize) {
        if let Ok(mut guard) = self.model.write() {
            *guard = model.to_string();
        }
        self.context_length
            .store(context_length, std::sync::atomic::Ordering::SeqCst);
    }

    /// Returns true if LM Studio is reachable.
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/v1/models", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Query /v1/models and return the first loaded model id.
    pub async fn get_loaded_model(&self) -> Option<String> {
        #[derive(Deserialize)]
        struct ModelList {
            data: Vec<ModelEntry>,
        }
        #[derive(Deserialize)]
        struct ModelEntry {
            id: String,
            #[serde(rename = "type", default)]
            model_type: String,
        }

        let resp = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .send()
            .await
            .ok()?;
        let list: ModelList = resp.json().await.ok()?;
        // Skip embedding models — they are not coding agents and should never
        // be selected as the active model even if they are the only one loaded.
        // Return Some("") when LM Studio is reachable but no coding model is loaded
        // so callers can distinguish "offline" (None) from "no coding model" (Some("")).
        Some(
            list.data
                .into_iter()
                .find(|m| m.model_type != "embeddings")
                .map(|m| m.id)
                .unwrap_or_default(),
        )
    }

    /// Detect the loaded model's context window size.
    /// Tries LM Studio's `/api/v0/models` endpoint first and prefers the loaded
    /// model's live `loaded_context_length`, then falls back to older
    /// `context_length` / `max_context_length` style fields.
    /// Falls back to a heuristic from the model name, then 32K.
    pub async fn detect_context_length(&self) -> usize {
        #[derive(Deserialize)]
        struct LmStudioModel {
            id: Option<String>,
            #[serde(rename = "type", default)]
            model_type: String,
            state: Option<String>,
            loaded_context_length: Option<u64>,
            context_length: Option<u64>,
            max_context_length: Option<u64>,
        }
        #[derive(Deserialize)]
        struct LmStudioList {
            data: Vec<LmStudioModel>,
        }

        // Check api/v0/models (LM Studio specific)
        if let Ok(resp) = self
            .client
            .get(format!("{}/api/v0/models", self.base_url))
            .send()
            .await
        {
            if let Ok(list) = resp.json::<LmStudioList>().await {
                let target_model = self.current_model().to_ascii_lowercase();
                // Never select embedding models for context-length detection.
                let non_embed = |m: &&LmStudioModel| m.model_type != "embeddings";
                let loaded = list
                    .data
                    .iter()
                    .find(|m| {
                        non_embed(m)
                            && m.state.as_deref() == Some("loaded")
                            && m.id
                                .as_deref()
                                .map(|id| id.eq_ignore_ascii_case(&target_model))
                                .unwrap_or(false)
                    })
                    .or_else(|| list.data.iter().find(|m| non_embed(m) && m.state.as_deref() == Some("loaded")))
                    .or_else(|| {
                        list.data.iter().find(|m| {
                            non_embed(m) && m.id.as_deref()
                                .map(|id| id.eq_ignore_ascii_case(&target_model))
                                .unwrap_or(false)
                        })
                    })
                    .or_else(|| list.data.iter().find(|m| non_embed(m)));

                if let Some(model) = loaded {
                    if let Some(ctx) = model.loaded_context_length {
                        if ctx > 0 {
                            return ctx as usize;
                        }
                    }
                    if let Some(ctx) = model.context_length {
                        if ctx > 0 {
                            return ctx as usize;
                        }
                    }
                    if let Some(ctx) = model.max_context_length {
                        if ctx > 0 && ctx <= 32_768 {
                            return ctx as usize;
                        }
                    }
                }
            }
        }

        // Heuristic fallback:
        // If "gemma-4" is detected, we target 32,768 as the baseline standard,
        // acknowledging that 131,072 is available for High-Capacity tasks.
        if self.current_model().to_lowercase().contains("gemma-4") {
            return 32_768;
        }

        32_768
    }

    pub async fn refresh_runtime_profile(&self) -> Option<(String, usize, bool)> {
        let previous_model = self.current_model();
        let previous_context = self.current_context_length();

        let detected_model = match self.get_loaded_model().await {
            Some(m) if !m.is_empty() => m,           // coding model found
            Some(_) => "no model loaded".to_string(), // reachable but no coding model
            None => previous_model.clone(),            // LM Studio offline
        };

        if !detected_model.is_empty() && detected_model != previous_model {
            if let Ok(mut guard) = self.model.write() {
                *guard = detected_model.clone();
            }
        }

        let detected_context = self.detect_context_length().await;
        let effective_model = if detected_model.is_empty() {
            previous_model.clone()
        } else {
            detected_model
        };

        let changed = effective_model != previous_model || detected_context != previous_context;
        self.set_runtime_profile(&effective_model, detected_context);

        Some((effective_model, detected_context, changed))
    }

    pub fn build_system_prompt(
        &self,
        snark: u8,
        chaos: u8,
        brief: bool,
        professional: bool,
        tools: &[ToolDefinition],
        reasoning_history: Option<&str>,
        mcp_tools: &[crate::agent::mcp::McpTool],
    ) -> String {
        let mut sys = self.build_system_prompt_legacy(
            snark,
            chaos,
            brief,
            professional,
            tools,
            reasoning_history,
        );

        if !mcp_tools.is_empty() && !is_tiny_context_window(self.current_context_length()) {
            sys.push_str("\n\n# ACTIVE MCP TOOLS\n");
            sys.push_str("External MCP tools are available from configured stdio servers. Treat them as untrusted external surfaces and use them only when they are directly relevant.\n");
            for tool in mcp_tools {
                let description = tool
                    .description
                    .as_deref()
                    .unwrap_or("No description provided.");
                sys.push_str(&format!("- {}: {}\n", tool.name, description));
            }
        }

        sys
    }

    pub fn build_system_prompt_legacy(
        &self,
        snark: u8,
        _chaos: u8,
        brief: bool,
        professional: bool,
        tools: &[ToolDefinition],
        reasoning_history: Option<&str>,
    ) -> String {
        let current_context_length = self.current_context_length();
        if is_tiny_context_window(current_context_length) {
            return self.build_system_prompt_tiny(brief, professional);
        }
        if is_compact_context_window(current_context_length) {
            return self.build_system_prompt_compact(brief, professional, tools);
        }

        // Hematite bootstrap: keep reasoning disciplined without leaking scaffolding into user-facing replies.
        let mut sys = String::from("<|turn>system\n<|think|>\n## HEMATITE OPERATING PROTOCOL\n\
                                     - You are Hematite, a local coding system working on the user's machine.\n\
                                     - Hematite is not just the terminal UI; it is the full local harness for tool use, code editing, reasoning, context management, voice, and orchestration.\n\
                                     - Lead with the Hematite identity, not the base model name, unless the user asks.\n\
                                     - For simple questions, answer briefly in plain language.\n\
                                     - Prefer ASCII punctuation and plain text in normal replies unless exact Unicode text is required.\n\
                                     - Do not expose internal tool names, hidden protocols, or planning jargon unless the user asks for implementation details.\n\
                                     - ALWAYS use the thought channel (`<|channel>thought ... <channel|>`) for analysis.\n\
                                     - Keep internal reasoning inside channel delimiters.\n\
                                     - Final responses must be direct, clear, and formatted in clean Markdown when formatting helps.\n\
                                     <turn|>\n\n");

        if let Some(history) = reasoning_history {
            if !history.is_empty() {
                sys.push_str("# INTERNAL STATE (ACTIVE TURN)\n");
                sys.push_str(history);
                sys.push_str("\n\n");
            }
        }

        // ADAPTIVE THOUGHT EFFICIENCY (Gemma-4 Native)
        if brief {
            sys.push_str("# ADAPTIVE THOUGHT EFFICIENCY: LOW\n\
                          - Core directive: Think efficiently. Avoid redundant internal derivation.\n\
                          - Depth: Surface-level verification only.\n\n");
        } else {
            sys.push_str("# ADAPTIVE THOUGHT EFFICIENCY: HIGH\n\
                          - Core directive: Think in depth when the task needs it. Explore edge cases and architectural implications.\n\
                          - Depth: Full multi-step derivation required.\n\n");
        }

        // IDENTITY & ENVIRONMENT
        let os = std::env::consts::OS;
        if professional {
            sys.push_str(&format!(
                "You are Hematite, a local coding system running on {}. \
                 The TUI is one interface layer, not your whole identity. \
                 Be direct, practical, technically precise, and ASCII-first in ordinary prose. \
                 Skip filler and keep the focus on the work.\n",
                os
            ));
        } else {
            sys.push_str(&format!(
                "You are Hematite, a [{}] local AI coding system (Snark: {}/100) running on the user's hardware on {}. \
                 The terminal UI is only one surface of the system. \
                 Be direct, efficient, technical, and ASCII-first in ordinary prose. \
                 When the user asks who you are, describe Hematite as the local coding harness and agent, not merely the TUI.\n",
                self.species, snark, os
            ));
        }

        // Inject loaded model and context window so the model knows its own budget.
        let current_model = self.current_model();
        if !current_model.is_empty() {
            sys.push_str(&format!(
                "Loaded model: {} | Context window: {} tokens. \
                 Calibrate response length and tool-call depth to fit within this budget.\n\n",
                current_model, current_context_length
            ));
            if is_gemma4_model_name(&current_model) {
                sys.push_str(
                    "Gemma 4 native note: prefer exact tool JSON with no extra prose when calling tools. \
                     Do not wrap `path`, `extension`, or other string arguments in extra quote layers. \
                     For `grep_files`, provide the raw regex pattern without surrounding slash delimiters.\n\n",
                );
            }
        } else {
            sys.push_str(&format!(
                "Context window: {} tokens. Calibrate response length to fit within this budget.\n\n",
                current_context_length
            ));
        }

        // PROTOCOL & TOOLS
        let shell_desc = if cfg!(target_os = "windows") {
            "[EXTERNAL SHELL]: `powershell` (Windows).\n\
             - Use ONLY for builds, tests, or file migrations. \n\
             - You MUST use the `powershell` tool directly. \n\
             - NEVER attempt to use `bash`, `sh`, or `/dev/null` on this system. \n\n"
        } else {
            "[EXTERNAL SHELL]: `bash` (Unix).\n\
             - Use ONLY for builds, tests, or file migrations. \n\
             - NEVER wrap bash in other shells. \n\n"
        };

        sys.push_str("You distinguish strictly between [INTERNAL TOOLS] and [EXTERNAL SHELL].\n\n\
                      [INTERNAL TOOLS]: `list_files`, `grep_files`, `read_file`, `edit_file`, `write_file`.\n\
                      - These are the ONLY way to explore and modify code. \n\
                      - NEVER attempt to run these as shell commands (e.g. `bash $ grep_files` is FORBIDDEN).\n\n");
        sys.push_str(shell_desc);

        // ANTI-LOOPING & SELF-AUDIT
        sys.push_str("ANTI-LOOPING: If a tool returns (no output) or 'not recognized' in a shell, pivot to a different internal tool. \n\
                      SELF-AUDIT: If you see your own command echoed back as the result, the shell failed; pivot to an internal tool immediately.\n\n");

        if brief {
            sys.push_str(
                "BRIEF MODE: Respond in exactly ONE concise sentence unless providing code.\n\n",
            );
        }

        if cfg!(target_os = "windows") {
            sys.push_str("Shell Protocol: You are running on WINDOWS. You MUST NOT use 'bash' or '/dev/null'. \
                          You MUST use 'powershell' (pwsh) for all shell tasks. \
                          DO NOT attempt to manipulate Linux-style paths like /dev, /etc, or /sys.\n\n");
        } else if cfg!(target_os = "macos") {
            sys.push_str(
                "Shell Protocol: You are running on macOS. Use 'bash' or 'zsh' for shell tasks. \
                          Standard Unix paths apply.\n\n",
            );
        } else {
            sys.push_str(
                "Shell Protocol: You are running on Linux. Use 'bash' for shell tasks. \
                          Standard Unix paths apply.\n\n",
            );
        }

        sys.push_str("OUTPUT RULES:\n\
                      1. Your internal reasoning goes in <think>...</think> blocks. Do NOT output reasoning as plain text.\n\
                      2. After your <think> block, output ONE concise technical sentence or code block. Nothing else.\n\
                      3. Do NOT call tools named 'thought', 'think', 'reasoning', or any meta-cognitive name. These are not tools.\n\
                      4. NEGATIVE CONSTRAINT: Never use a string containing a dot (.), slash (/), or backslash (\\) as a tool name. Paths are NOT tools.\n\
                      5. NEGATIVE CONSTRAINT: Never use the name of a class, struct, or module as a tool name unless it is explicitly in the tool list.\n\
                      6. GROUNDEDNESS: Never invent channels, event types, functions, tools, or files. If a detail is not verified from the repo or tool output, say `uncertain`.\n\
                      7. TRACE QUESTIONS: For architecture or control-flow questions, prefer verified file and function names over high-level summaries.\n\
                      8. If `trace_runtime_flow` fully answers the runtime question, preserve its identifiers exactly. Do not restyle or rename symbols from that tool output.\n\
                      9. For generic capability questions, answer from stable Hematite capabilities. Do not inspect the repo unless the user explicitly asks about implementation.\n\
                      10. Never infer language support, project support, or internet capability from unrelated crates or config files.\n\
                      11. It is fine to say Hematite itself is written in Rust when relevant, but do not imply that capability is limited to Rust projects.\n\
                      12. For language questions, answer at the harness level: file operations, shell, build verification, language-aware tooling when available, and multi-language project work.\n\
                      13. Prefer real programming language examples like Python, JavaScript, TypeScript, Go, and C# over file extensions when answering language questions.\n\
                      14. For project-building questions, talk about scaffolding, implementation, builds, tests, and iteration across different stacks instead of defaulting to a Rust-only example like `cargo build`.\n\
                      15. Never mention raw `mcp__*` tool names unless those tools are active this turn and directly relevant.\n\
                      16. For tooling-discipline or best-tool-selection questions, prefer `describe_toolchain` over improvising the tool surface from memory.\n\
                      17. If `describe_toolchain` fully answers the tooling question, preserve its tool names and investigation order exactly.\n\
                      18. PROOF BEFORE ACTION: Before editing an existing file, gather recent evidence with `read_file` or `inspect_lines` on that path or keep it pinned in active context.\n\
                      19. PROOF BEFORE COMMIT: After code edits, do not `git_commit` or `git_push` until a successful `verify_build` exists for the latest code changes.\n\
                      20. RISKY SHELL DISCIPLINE: Risky `shell` calls must include a concrete `reason` argument explaining what is being verified or changed.\n\
                      21. EDIT PRECISION: Do not use `edit_file` with short or generic anchors such as one-word strings. Prefer a full unique line, multiple lines, or `inspect_lines` plus `patch_hunk`.\n\
                      22. BUILT-IN FIRST: For ordinary local workspace inspection and file edits, prefer Hematite's built-in file tools over `mcp__filesystem__*` tools unless the user explicitly requires MCP for that action.");

        // Scaffolding protocol — enforces build validation after project creation.
        sys.push_str("\n## SCAFFOLDING PROTOCOL\n\
            2. ALWAYS call verify_build immediately after to confirm the project compiles/runs.\n\
            3. If verify_build fails, use `lsp_get_diagnostics` to find the exact line and error.\n\
            4. Fix all errors before declaring success.\n\n\
            ## PRE-FLIGHT SCOPING PROTOCOL\n\
            Before attempting any multi-file task or complex refactor:\n\
            1. Use `map_project` to understand the project structure.\n\
            2. Identify 1-3 core files (entry-points, central models, or types) that drive the logic.\n\
            3. Use `auto_pin_context` to keep those files in active context.\n\
            4. Only then proceed to deeper edits or research.\n\n\
            ## REFACTORING PROTOCOL\n\
            When modifying existing code or renaming symbols:\n\
            1. Use `lsp_rename_symbol` for all variable/function renames to ensure project-wide safety.\n\
            2. After any significant edit, call `lsp_get_diagnostics` on the affected files.\n\
            3. If errors are found, you MUST fix them. Do not wait for the user to point them out.\n\n");

        // Inject CLAUDE.md / instruction files from the project directory.
        sys.push_str(&load_instruction_files());

        // Inject cross-session memories synthesized by DeepReflect.
        sys.push_str(&crate::memory::deep_reflect::load_recent_memories());

        // Native Gemma-4 Tool Declarations
        if !tools.is_empty() {
            sys.push_str("\n\n# NATIVE TOOL DECLARATIONS\n");
            for tool in tools {
                let schema = serde_json::to_string(&tool.function.parameters)
                    .unwrap_or_else(|_| "{}".to_string());
                sys.push_str(&format!(
                    "<|tool>declaration:{}{}{}<tool|>\n",
                    tool.function.name, "{", schema
                ));
                sys.push_str(&format!("// {})\n", tool.function.description));
            }
        }

        sys
    }

    fn build_system_prompt_compact(
        &self,
        brief: bool,
        professional: bool,
        tools: &[ToolDefinition],
    ) -> String {
        // Compact tier: fits in 16k context. Keeps tool names + one-line descriptions
        // but skips full JSON schemas, verbose protocol sections, and CLAUDE.md injection.
        let current_model = self.current_model();
        let current_context_length = self.current_context_length();
        let os = std::env::consts::OS;

        let mut sys = String::from("<|turn>system\n<|think|>\n");
        sys.push_str("You are Hematite, a local coding harness working on the user's machine.\n");
        if professional {
            sys.push_str("Be direct, technical, concise, and ASCII-first.\n");
        } else {
            sys.push_str(&format!(
                "You are a [{}] local AI coding system. Be direct, concise, and technical.\n",
                self.species
            ));
        }
        sys.push_str(&format!(
            "Model: {} | Context: {} tokens. Keep turns focused.\n",
            current_model, current_context_length
        ));
        if is_gemma4_model_name(&current_model) {
            sys.push_str(
                "Gemma 4: use exact tool JSON. No extra prose in tool calls. \
                 Raw regex patterns in grep_files, no slash delimiters.\n",
            );
        }
        if cfg!(target_os = "windows") {
            sys.push_str(&format!(
                "OS: {}. Use PowerShell for shell. Never bash or /dev/null.\n",
                os
            ));
        } else {
            sys.push_str(&format!("OS: {}. Use native Unix shell.\n", os));
        }
        if brief {
            sys.push_str("BRIEF MODE: one concise sentence unless code is required.\n");
        }

        sys.push_str(
            "\nCORE RULES:\n\
             - Read before editing: use `read_file` or `inspect_lines` on a file before mutating it.\n\
             - Verify after edits: run `verify_build` after code changes, before committing.\n\
             - One tool at a time. Do not batch unrelated tool calls.\n\
             - Do not invent tool names, file paths, or symbols not confirmed by tool output.\n\
             - Built-in tools first: prefer `read_file`, `edit_file`, `grep_files` over MCP filesystem tools.\n\
             - STARTUP/UI CHANGES: read the owner file first, make one focused edit, then run `verify_build`.\n",
        );

        if !tools.is_empty() {
            sys.push_str("\n# AVAILABLE TOOLS\n");
            for tool in tools {
                let desc: String = tool.function.description.chars().take(120).collect();
                sys.push_str(&format!("- {}: {}\n", tool.function.name, desc));
            }
        }

        sys.push_str("<turn|>\n");
        sys
    }

    fn build_system_prompt_tiny(&self, brief: bool, professional: bool) -> String {
        let current_model = self.current_model();
        let current_context_length = self.current_context_length();
        let os = std::env::consts::OS;
        let mut sys = String::from(
            "<|turn>system\nYou are Hematite, a local coding harness working on the user's machine.\n",
        );
        if professional {
            sys.push_str("Be direct, technical, concise, and ASCII-first.\n");
        } else {
            sys.push_str(&format!(
                "You are a [{}] local AI coding system. Be direct, concise, and technical.\n",
                self.species
            ));
        }
        if !current_model.is_empty() {
            sys.push_str(&format!(
                "Loaded model: {} | Context window: {} tokens.\n",
                current_model, current_context_length
            ));
        } else {
            sys.push_str(&format!(
                "Context window: {} tokens.\n",
                current_context_length
            ));
        }
        sys.push_str("Tiny-context mode is active. Keep turns short. Prefer final answers over long analysis. Only use tools when necessary.\n");
        sys.push_str("Use built-in workspace tools for local inspection and edits. Do not invent tools, files, channels, or symbols.\n");
        sys.push_str("Before editing an existing file, gather recent file evidence first. After code edits, verify before commit.\n");
        if cfg!(target_os = "windows") {
            sys.push_str(&format!(
                "You are running on {}. Use PowerShell for shell work. Do not assume bash or /dev/null.\n",
                os
            ));
        } else {
            sys.push_str(&format!("You are running on {}. Use the native Unix shell conventions.\n", os));
        }
        if brief {
            sys.push_str("BRIEF MODE: answer in one concise sentence unless code is required.\n");
        }
        if is_gemma4_model_name(&current_model) {
            sys.push_str("Gemma 4 note: use exact tool JSON with no extra prose when calling tools.\n");
        }
        sys.push_str("<turn|>\n");
        sys
    }

    // ── Non-streaming call (used for agentic turns with tool support) ─────────

    /// Send messages to the model. Returns (text_content, tool_calls).
    /// Exactly one of the two will be Some on a successful response.
    pub async fn call_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        // Override the model ID for this call. None = use the live runtime model.
        model_override: Option<&str>,
    ) -> Result<
        (
            Option<String>,
            Option<Vec<ToolCallResponse>>,
            Option<TokenUsage>,
            Option<String>,
        ),
        String,
    > {
        let _permit = self
            .kv_semaphore
            .acquire()
            .await
            .map_err(|e| e.to_string())?;

        let current_model = self.current_model();
        let model = model_override.unwrap_or(current_model.as_str()).to_string();
        let filtered_tools = if cfg!(target_os = "windows") {
            tools
                .iter()
                .filter(|t| t.function.name != "bash" && t.function.name != "sh")
                .cloned()
                .collect::<Vec<_>>()
        } else {
            tools.to_vec()
        };

        let request_messages = if should_use_gemma_native_formatting(self, &model) {
            prepare_gemma_native_messages(messages)
        } else {
            messages.to_vec()
        };

        // In compact context windows, restrict tools to the core coding set.
        // Full schemas for 36+ tools add 10k+ tokens via the model's chat template (e.g. Gemma 4).
        // Sending a small core set keeps schemas available for structured tool-call dispatch
        // while staying within the 16k budget.
        const COMPACT_CORE_TOOLS: &[&str] = &[
            "read_file", "inspect_lines", "edit_file", "write_file",
            "grep_files", "list_files", "verify_build", "shell", "map_project",
        ];
        let effective_tools = if is_compact_context_window(self.current_context_length()) {
            let core: Vec<_> = filtered_tools
                .iter()
                .filter(|t| COMPACT_CORE_TOOLS.contains(&t.function.name.as_str()))
                .cloned()
                .collect();
            if core.is_empty() { None } else { Some(core) }
        } else if filtered_tools.is_empty() {
            None
        } else {
            Some(filtered_tools)
        };

        let request = ChatRequest {
            model: model.clone(),
            messages: request_messages,
            temperature: 0.2,
            stream: false,
            tools: effective_tools,
        };

        // Exponential backoff: retry up to 3× on 5xx / timeout / connect errors.
        preflight_chat_request(
            &model,
            &request.messages,
            request.tools.as_deref().unwrap_or(&[]),
            self.current_context_length(),
        )?;

        let mut last_err = String::new();
        let mut response_opt: Option<reqwest::Response> = None;
        for attempt in 0..3u32 {
            match self.client.post(&self.api_url).json(&request).send().await {
                Ok(res) if res.status().is_success() => {
                    response_opt = Some(res);
                    break;
                }
                Ok(res) if res.status().as_u16() >= 500 => {
                    last_err = format!("LM Studio error {}", res.status());
                }
                Ok(res) => {
                    // 4xx — don't retry
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    let preview = &body[..body.len().min(300)];
                    return Err(format!("LM Studio error {}: {}", status, preview));
                }
                Err(e) if e.is_timeout() || e.is_connect() => {
                    last_err = format!("Request failed: {}", e);
                }
                Err(e) => return Err(format!("Request failed: {}", e)),
            }
            if attempt < 2 {
                let delay = std::time::Duration::from_millis(500 * (1u64 << attempt));
                tokio::time::sleep(delay.min(std::time::Duration::from_secs(4))).await;
            }
        }
        let res = response_opt
            .ok_or_else(|| format!("LM Studio unreachable after 3 attempts: {}", last_err))?;

        let body: ChatResponse = res
            .json()
            .await
            .map_err(|e| format!("Response parse error: {}", e))?;

        if let Some(usage) = &body.usage {
            let mut econ = self.economics.lock().unwrap();
            econ.input_tokens += usage.prompt_tokens;
            econ.output_tokens += usage.completion_tokens;
        }

        let choice = body
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| "Empty response from model".to_string())?;

        let finish_reason = choice.finish_reason;
        let mut tool_calls = choice.message.tool_calls;
        let mut content = choice.message.content;

        // Gemma-4 Fallback: If the model outputs native <|tool_call|> tags in the text content,
        // extract them and treat them as valid tool calls.
        if let Some(raw_content) = &content {
            let native_calls = extract_native_tool_calls(raw_content);
            if !native_calls.is_empty() {
                let mut existing = tool_calls.unwrap_or_default();
                existing.extend(native_calls);
                tool_calls = Some(existing);
                let stripped = strip_native_tool_call_text(raw_content);
                content = if stripped.trim().is_empty() {
                    None
                } else {
                    Some(stripped)
                };
            }
        }

        if is_gemma4_model_name(&model) {
            if let Some(calls) = tool_calls.as_mut() {
                for call in calls.iter_mut() {
                    call.function.arguments =
                        normalize_tool_argument_string(&call.function.name, &call.function.arguments);
                }
            }
        }

        Ok((content, tool_calls, body.usage, finish_reason))
    }

    // ── Streaming call (used for plain-text responses) ────────────────────────

    /// Stream a conversation (no tools). Emits Token/Done/Error events.
    pub async fn stream_messages(
        &self,
        messages: &[ChatMessage],
        tx: mpsc::Sender<InferenceEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let current_model = self.current_model();
        let request_messages = if should_use_gemma_native_formatting(self, &current_model) {
            prepare_gemma_native_messages(messages)
        } else {
            messages
                .iter()
                .map(|m| {
                    let mut clone = m.clone();
                    let current_text = m.content.as_str();
                    if !current_text.starts_with("<|turn>") {
                        clone.content = MessageContent::Text(format!(
                            "<|turn>{}\n{}\n<turn|>",
                            m.role, current_text
                        ));
                    }
                    clone
                })
                .collect()
        };

        let request = ChatRequest {
            model: current_model.clone(),
            messages: request_messages,
            temperature: 0.7,
            stream: true,
            tools: None,
        };

        if let Err(e) = preflight_chat_request(&current_model, &request.messages, &[], self.current_context_length()) {
            let tag = classify_runtime_failure_tag(&e);
            let _ = tx
                .send(InferenceEvent::ProviderStatus {
                    state: provider_state_for_failure_tag(tag),
                    summary: compact_runtime_failure_summary(tag, &e),
                })
                .await;
            let _ = tx
                .send(InferenceEvent::Error(format_runtime_failure_message(&e)))
                .await;
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        let mut last_err = String::new();
        let mut response_opt: Option<reqwest::Response> = None;
        for attempt in 0..2u32 {
            match self.client.post(&self.api_url).json(&request).send().await {
                Ok(res) if res.status().is_success() => {
                    response_opt = Some(res);
                    break;
                }
                Ok(res) if res.status().as_u16() >= 500 => {
                    last_err = format!("LM Studio error {}", res.status());
                }
                Ok(res) => {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    let preview = &body[..body.len().min(300)];
                    let detail = format!("LM Studio error {}: {}", status, preview);
                    let tag = classify_runtime_failure_tag(&detail);
                    let _ = tx
                        .send(InferenceEvent::ProviderStatus {
                            state: provider_state_for_failure_tag(tag),
                            summary: compact_runtime_failure_summary(tag, &detail),
                        })
                        .await;
                    let _ = tx
                        .send(InferenceEvent::Error(format_runtime_failure_message(&detail)))
                        .await;
                    let _ = tx.send(InferenceEvent::Done).await;
                    return Ok(());
                }
                Err(e) if e.is_timeout() || e.is_connect() => {
                    last_err = format!("Request failed: {}", e);
                }
                Err(e) => {
                    let detail = format!("Request failed: {}", e);
                    let tag = classify_runtime_failure_tag(&detail);
                    let _ = tx
                        .send(InferenceEvent::ProviderStatus {
                            state: provider_state_for_failure_tag(tag),
                            summary: compact_runtime_failure_summary(tag, &detail),
                        })
                        .await;
                    let _ = tx
                        .send(InferenceEvent::Error(format_runtime_failure_message(&detail)))
                        .await;
                    let _ = tx.send(InferenceEvent::Done).await;
                    return Ok(());
                }
            }
            if attempt < 1 {
                let _ = tx
                    .send(InferenceEvent::ProviderStatus {
                        state: ProviderRuntimeState::Recovering,
                        summary: "LM Studio degraded during stream startup; retrying once.".into(),
                    })
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
        let Some(res) = response_opt else {
            let detail = format!("LM Studio unreachable after 2 attempts: {}", last_err);
            let tag = classify_runtime_failure_tag(&detail);
            let _ = tx
                .send(InferenceEvent::ProviderStatus {
                    state: provider_state_for_failure_tag(tag),
                    summary: compact_runtime_failure_summary(tag, &detail),
                })
                .await;
            let _ = tx
                .send(InferenceEvent::Error(format_runtime_failure_message(&detail)))
                .await;
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        };

        use futures::StreamExt;
        let mut byte_stream = res.bytes_stream();

        // [Collaborative Strategy] TokenBuffer refactor suggested by Hematite local agent.
        // Aggregates tokens to ensure coherent linguistic chunks for UI/Voice.
        let mut line_buffer = String::new();
        let mut content_buffer = String::new();
        let mut past_think = false;
        let mut emitted_any_content = false;
        let mut emitted_live_status = false;

        while let Some(item) = byte_stream.next().await {
            // Rapid hardware interrupt check
            if self.cancel_token.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }

            let chunk = match item {
                Ok(chunk) => chunk,
                Err(e) => {
                    let detail = format!("Request failed: {}", e);
                    let tag = classify_runtime_failure_tag(&detail);
                    let _ = tx
                        .send(InferenceEvent::ProviderStatus {
                            state: provider_state_for_failure_tag(tag),
                            summary: compact_runtime_failure_summary(tag, &detail),
                        })
                        .await;
                    let _ = tx
                        .send(InferenceEvent::Error(format_runtime_failure_message(&detail)))
                        .await;
                    let _ = tx.send(InferenceEvent::Done).await;
                    return Ok(());
                }
            };
            line_buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = line_buffer.find("\n\n") {
                let event_str = line_buffer.drain(..pos + 2).collect::<String>();
                let data_pos = match event_str.find("data: ") {
                    Some(p) => p,
                    None => continue,
                };

                let data = event_str[data_pos + 6..].trim();
                if data == "[DONE]" {
                    break;
                }

                if let Ok(json) = serde_json::from_str::<Value>(data) {
                    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                        if content.is_empty() {
                            continue;
                        }

                        if !past_think {
                            let lc = content.to_lowercase();
                            let close = lc
                                .find("<channel|>")
                                .map(|i| (i, "<channel|>".len()))
                                .or_else(|| lc.find("</think>").map(|i| (i, "</think>".len())));

                            if let Some((tag_start, tag_len)) = close {
                                // Flush any existing thought buffer
                                let before = &content[..tag_start];
                                content_buffer.push_str(before);
                                if !content_buffer.trim().is_empty() {
                                    let _ = tx
                                        .send(InferenceEvent::Thought(content_buffer.clone()))
                                        .await;
                                    emitted_any_content = true;
                                }
                                content_buffer.clear();

                                past_think = true;
                                let after = content[tag_start + tag_len..].trim_start_matches('\n');
                                content_buffer.push_str(after);
                            } else {
                                // Still in reasoning block
                                content_buffer.push_str(content);
                                // Heuristic: Flush thoughts on paragraph/sentence breaks for SPECULAR
                                if content_buffer.len() > 30
                                    && (content.contains('\n') || content.contains('.'))
                                {
                                    let _ = tx
                                        .send(InferenceEvent::Thought(content_buffer.clone()))
                                        .await;
                                    emitted_any_content = true;
                                    content_buffer.clear();
                                }
                            }
                        } else {
                            // PAST THINK: final answer tokens.
                            // [Linguistic Buffering] Aggregate into content_buffer until a boundary is hit.
                            content_buffer.push_str(content);
                            let is_boundary = content.contains(' ')
                                || content.contains('.')
                                || content.contains('!')
                                || content.contains('?');

                            if content_buffer.len() > 10 && is_boundary {
                                if !emitted_live_status {
                                    let _ = tx
                                        .send(InferenceEvent::ProviderStatus {
                                            state: ProviderRuntimeState::Live,
                                            summary: String::new(),
                                        })
                                        .await;
                                    emitted_live_status = true;
                                }
                                let _ =
                                    tx.send(InferenceEvent::Token(content_buffer.clone())).await;
                                emitted_any_content = true;
                                content_buffer.clear();
                            }
                        }
                    }
                }
            }
        }

        // Final Flush
        if !content_buffer.is_empty() {
            if past_think {
                if !emitted_live_status {
                    let _ = tx
                        .send(InferenceEvent::ProviderStatus {
                            state: ProviderRuntimeState::Live,
                            summary: String::new(),
                        })
                        .await;
                }
                let _ = tx.send(InferenceEvent::Token(content_buffer)).await;
            } else {
                let _ = tx.send(InferenceEvent::Thought(content_buffer)).await;
            }
            emitted_any_content = true;
        }

        if !emitted_any_content {
            let _ = tx
                .send(InferenceEvent::ProviderStatus {
                    state: ProviderRuntimeState::EmptyResponse,
                    summary: compact_runtime_failure_summary(
                        "empty_model_response",
                        "Empty response from model",
                    ),
                })
                .await;
            let _ = tx
                .send(InferenceEvent::Error(format_runtime_failure_message(
                    "Empty response from model",
                )))
                .await;
            let _ = tx.send(InferenceEvent::Done).await;
            return Ok(());
        }

        let _ = tx.send(InferenceEvent::Done).await;
        Ok(())
    }

    /// Single-turn streaming (legacy helper used by startup sequence).
    pub async fn stream_generation(
        &self,
        prompt: &str,
        snark: u8,
        chaos: u8,
        brief: bool,
        professional: bool,
        tx: mpsc::Sender<InferenceEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let system = self.build_system_prompt(snark, chaos, brief, professional, &[], None, &[]);
        let messages = vec![ChatMessage::system(&system), ChatMessage::user(prompt)];
        self.stream_messages(&messages, tx).await
    }

    // ── Swarm worker helpers (non-streaming) ──────────────────────────────────

    /// Runs a task using the `worker_model` if set, otherwise falls back to the main `model`.
    pub async fn generate_task_worker(
        &self,
        prompt: &str,
        professional: bool,
    ) -> Result<String, String> {
        let current_model = self.current_model();
        let model = self.worker_model.as_deref().unwrap_or(current_model.as_str());
        self.generate_task_with_model(prompt, 0.1, professional, model)
            .await
    }

    pub async fn generate_task(&self, prompt: &str, professional: bool) -> Result<String, String> {
        self.generate_task_with_temp(prompt, 0.1, professional)
            .await
    }

    pub async fn generate_task_with_temp(
        &self,
        prompt: &str,
        temp: f32,
        professional: bool,
    ) -> Result<String, String> {
        let current_model = self.current_model();
        self.generate_task_with_model(prompt, temp, professional, &current_model)
            .await
    }

    pub async fn generate_task_with_model(
        &self,
        prompt: &str,
        temp: f32,
        professional: bool,
        model: &str,
    ) -> Result<String, String> {
        let _permit = self
            .kv_semaphore
            .acquire()
            .await
            .map_err(|e| e.to_string())?;

        let system = self.build_system_prompt(self.snark, 50, false, professional, &[], None, &[]);
        let request_messages = if should_use_gemma_native_formatting(self, model) {
            prepare_gemma_native_messages(&[ChatMessage::system(&system), ChatMessage::user(prompt)])
        } else {
            vec![ChatMessage::system(&system), ChatMessage::user(prompt)]
        };
        let request = ChatRequest {
            model: model.to_string(),
            messages: request_messages,
            temperature: temp,
            stream: false,
            tools: None,
        };

        preflight_chat_request(model, &request.messages, &[], self.current_context_length())?;

        let res = self
            .client
            .post(&self.api_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("LM Studio request failed: {}", e))?;

        let body: ChatResponse = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        body.choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| "Empty response from model".to_string())
    }

    // ── History management ────────────────────────────────────────────────────

    /// Prune middle turns when context grows too large, keeping system + recent N.
    #[allow(dead_code)]
    pub fn snip_history(
        &self,
        turns: &[ChatMessage],
        max_tokens_estimate: usize,
        keep_recent: usize,
    ) -> Vec<ChatMessage> {
        let total_chars: usize = turns.iter().map(|m| m.content.as_str().len()).sum();
        if total_chars / 4 <= max_tokens_estimate {
            return turns.to_vec();
        }
        let keep = keep_recent.min(turns.len());
        let mut snipped = vec![turns[0].clone()];
        if turns.len() > keep + 1 {
            snipped.push(ChatMessage::system(&format!(
                "[CONTEXT SNIPPED: {} earlier turns pruned to preserve VRAM]",
                turns.len() - keep - 1
            )));
            snipped.extend_from_slice(&turns[turns.len() - keep..]);
        } else {
            snipped = turns.to_vec();
        }
        snipped
    }
}

fn estimate_serialized_tokens<T: Serialize + ?Sized>(value: &T) -> usize {
    serde_json::to_vec(value)
        .ok()
        .map_or(0, |bytes| bytes.len() / 4 + 1)
}

fn reserved_output_tokens(context_length: usize) -> usize {
    let proportional = (context_length / 8).max(MIN_RESERVED_OUTPUT_TOKENS);
    proportional.min(MAX_RESERVED_OUTPUT_TOKENS)
}

pub fn estimate_prompt_pressure(
    messages: &[ChatMessage],
    tools: &[ToolDefinition],
    context_length: usize,
) -> (usize, usize, usize, u8) {
    let estimated_input_tokens =
        estimate_serialized_tokens(messages) + estimate_serialized_tokens(tools) + 32;
    let reserved_output = reserved_output_tokens(context_length);
    let estimated_total = estimated_input_tokens.saturating_add(reserved_output);
    let percent = if context_length == 0 {
        0
    } else {
        ((estimated_total.saturating_mul(100)) / context_length).min(100) as u8
    };
    (
        estimated_input_tokens,
        reserved_output,
        estimated_total,
        percent,
    )
}

fn preflight_chat_request(
    model: &str,
    messages: &[ChatMessage],
    tools: &[ToolDefinition],
    context_length: usize,
) -> Result<(), String> {
    let (estimated_input_tokens, reserved_output, estimated_total, _) =
        estimate_prompt_pressure(messages, tools, context_length);

    if estimated_total > context_length {
        return Err(format!(
            "context_window_blocked for {}: estimated input {} + reserved output {} = {} tokens exceeds the {}-token context window; narrow the request, compact the session, or preserve grounded tool output instead of restyling it.",
            model, estimated_input_tokens, reserved_output, estimated_total, context_length
        ));
    }

    Ok(())
}

/// Walk from CWD up to 4 parent directories and collect instruction files.
/// Looks for CLAUDE.md, CLAUDE.local.md, and .hematite/instructions.md.
/// Deduplicates by content hash; truncates at 4KB per file, 12KB total.
fn load_instruction_files() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::collections::HashSet;
    use std::hash::{Hash, Hasher};

    let Ok(cwd) = std::env::current_dir() else {
        return String::new();
    };
    let mut result = String::new();
    let mut seen: HashSet<u64> = HashSet::new();
    let mut total_chars: usize = 0;
    const MAX_TOTAL: usize = 12_000;
    const MAX_PER_FILE: usize = 4_000;

    let candidates = ["CLAUDE.md", "CLAUDE.local.md", ".hematite/instructions.md"];

    let mut dir = cwd.clone();
    for _ in 0..4 {
        for name in &candidates {
            let path = dir.join(name);
            if !path.exists() {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            if content.trim().is_empty() {
                continue;
            }

            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            let h = hasher.finish();
            if !seen.insert(h) {
                continue;
            }

            let truncated = if content.len() > MAX_PER_FILE {
                format!("{}...[truncated]", &content[..MAX_PER_FILE])
            } else {
                content
            };

            if total_chars + truncated.len() > MAX_TOTAL {
                break;
            }
            total_chars += truncated.len();
            result.push_str(&format!("\n--- {} ---\n{}\n", path.display(), truncated));
        }
        match dir.parent().map(|p| p.to_owned()) {
            Some(p) => dir = p,
            None => break,
        }
    }

    if result.is_empty() {
        return String::new();
    }
    format!("\n\n# Project Instructions\n{}", result)
}

pub fn extract_think_block(text: &str) -> Option<String> {
    let lower = text.to_lowercase();

    // Official Gemma-4 Native Tags
    let open_tag = "<|channel>thought";
    let close_tag = "<channel|>";

    let start_pos = lower.find(open_tag)?;
    let content_start = start_pos + open_tag.len();

    let close_pos = lower[content_start..]
        .find(close_tag)
        .map(|p| content_start + p)
        .unwrap_or(text.len());

    let content = text[content_start..close_pos].trim();
    if content.is_empty() {
        None
    } else {
        Some(content.to_string())
    }
}

pub fn strip_think_blocks(text: &str) -> String {
    let lower = text.to_lowercase();

    // Use the official Gemma-4 closing tag — answer is everything after it.
    if let Some(end) = lower.find("<channel|>").map(|i| i + "<channel|>".len()) {
        let answer = text[end..]
            .replace("<|channel>thought", "")
            .replace("<channel|>", "");
        return answer.trim().replace("\n\n\n", "\n\n").to_string();
    }

    // No closing tag — if there's an unclosed opening tag, discard everything before and during it.
    let first_open = [
        lower.find("<|channel>thought"), // Prioritize Gemma-4 native
        lower.find("<think>"),
        lower.find("<thought>"),
        lower.find("<|think|>"),
    ]
    .iter()
    .filter_map(|&x| x)
    .min();

    if let Some(start) = first_open {
        if start > 0 {
            return text[..start].trim().replace("\n\n\n", "\n\n").to_string();
        }
        return String::new();
    }

    // If the model outputs 'naked' reasoning without tags:
    // Strip sentences like "The user asked..." or "I will structure the response..."
    // if they appear before the first identifiable self-introduction or code block.
    let is_naked_reasoning = lower.contains("the user asked")
        || lower.contains("the user is asking")
        || lower.contains("the user wants")
        || lower.contains("i will structure")
        || lower.contains("i should provide")
        || lower.contains("i can see from")
        || lower.contains("necessary information in my identity");
    if is_naked_reasoning {
        let lines: Vec<&str> = text.lines().collect();
        if !lines.is_empty() {
            // Find the first line that looks like real answer content.
            for (i, line) in lines.iter().enumerate() {
                let l_line = line.to_lowercase();
                if l_line.contains("am hematite")
                    || l_line.contains("my purpose is")
                    || line.starts_with("#")
                    || line.starts_with("```")
                {
                    return lines[i..].join("\n").trim().to_string();
                }
            }
            // No real content found after reasoning — the whole response is just
            // internal monologue (e.g. reasoning + abandoned tool call). Return empty.
            return String::new();
        }
    }

    // Strip leaked XML tool-call fragments that Qwen sometimes emits when it
    // abandons a tool call mid-generation (e.g. </parameter></function></tool_call>).
    let cleaned = strip_xml_tool_call_artifacts(text);
    cleaned.trim().replace("\n\n\n", "\n\n").to_string()
}

/// Remove stray XML tool-call closing/opening tags that local models occasionally
/// leak into visible output when they start-then-abandon a tool call.
fn strip_xml_tool_call_artifacts(text: &str) -> String {
    // Tags to remove (both open and close forms, case-insensitive).
    const XML_ARTIFACTS: &[&str] = &[
        "</tool_call>", "<tool_call>",
        "</function>",  "<function>",
        "</parameter>", "<parameter>",
        "</arguments>", "<arguments>",
        "</tool_use>",  "<tool_use>",
        "</invoke>",    "<invoke>",
        // Stray think/reasoning closing tags that leak after block extraction.
        "</think>", "</thought>", "</thinking>",
    ];
    let mut out = text.to_string();
    for tag in XML_ARTIFACTS {
        // Case-insensitive replace
        while let Some(pos) = out.to_lowercase().find(&tag.to_lowercase()) {
            out.drain(pos..pos + tag.len());
        }
    }
    // Collapse any blank lines left behind
    out
}

/// Extract native Gemma-4 <|tool_call|> tags from text.
/// Format: <|tool_call|>call:func_name{key:<|"|>value<|"|>, key2:value2}<tool_call|>
pub fn extract_native_tool_calls(text: &str) -> Vec<ToolCallResponse> {
    use regex::Regex;
    let mut results = Vec::new();

    // Regex to find the tool call block
    // Formats supported:
    // <|tool_call|>call:func_name{args}<tool_call|>
    // <|tool_call>call:func_name{args}[END_TOOL_REQUEST]
    // <|tool_call>call:func_name{args}<tool_call|>
    let re_call = Regex::new(
        r#"(?s)<\|?tool_call\|?>\s*call:([A-Za-z_][A-Za-z0-9_]*)\{(.*?)\}(?:<\|?tool_call\|?>|\[END_TOOL_REQUEST\])"#
    ).unwrap();
    // Regex to find arguments inside the braces
    // Handles <|"|> wrappers and plain values
    let re_arg = Regex::new(r#"(\w+):(?:<\|"\|>(.*?)<\|"\|>|([^,}]*))"#).unwrap();

    for cap in re_call.captures_iter(text) {
        let name = cap[1].to_string();
        let args_str = &cap[2];
        let mut arguments = serde_json::Map::new();

        for arg_cap in re_arg.captures_iter(args_str) {
            let key = arg_cap[1].to_string();
            // arg_cap[2] is the <|"|> wrapped value, arg_cap[3] is the plain value
            let val_raw = arg_cap
                .get(2)
                .map(|m| m.as_str())
                .or_else(|| arg_cap.get(3).map(|m| m.as_str()))
                .unwrap_or("")
                .trim();
            let normalized_raw = normalize_string_arg(&val_raw.replace("\\\"", "\""));

            // Try to parse as JSON types (bool, number), otherwise string
            let val = if normalized_raw == "true" {
                Value::Bool(true)
            } else if normalized_raw == "false" {
                Value::Bool(false)
            } else if let Ok(n) = normalized_raw.parse::<i64>() {
                Value::Number(n.into())
            } else if let Ok(n) = normalized_raw.parse::<u64>() {
                Value::Number(n.into())
            } else if let Ok(n) = normalized_raw.parse::<f64>() {
                serde_json::Number::from_f64(n)
                    .map(Value::Number)
                    .unwrap_or(Value::String(normalized_raw.clone()))
            } else {
                Value::String(normalized_raw)
            };

            arguments.insert(key, val);
        }

        results.push(ToolCallResponse {
            id: format!("call_{}", rand::random::<u32>()),
            call_type: "function".to_string(),
            function: ToolCallFn {
                name,
                arguments: Value::Object(arguments).to_string(),
            },
        });
    }

    results
}

pub fn normalize_tool_argument_string(tool_name: &str, raw: &str) -> String {
    let trimmed = raw.trim();
    let candidate = unwrap_json_string_once(trimmed).unwrap_or_else(|| trimmed.to_string());

    let mut value = match serde_json::from_str::<Value>(&candidate) {
        Ok(v) => v,
        Err(_) => return candidate,
    };
    normalize_tool_argument_value(tool_name, &mut value);
    value.to_string()
}

fn normalize_tool_argument_value(tool_name: &str, value: &mut Value) {
    match value {
        Value::String(s) => *s = normalize_string_arg(s),
        Value::Array(items) => {
            for item in items {
                normalize_tool_argument_value(tool_name, item);
            }
        }
        Value::Object(map) => {
            for val in map.values_mut() {
                normalize_tool_argument_value(tool_name, val);
            }
            if tool_name == "grep_files" {
                if let Some(Value::String(pattern)) = map.get_mut("pattern") {
                    *pattern = normalize_regex_pattern(pattern);
                }
            }
            for key in ["path", "extension", "query", "command", "reason"] {
                if let Some(Value::String(s)) = map.get_mut(key) {
                    *s = normalize_string_arg(s);
                }
            }
        }
        _ => {}
    }
}

fn unwrap_json_string_once(input: &str) -> Option<String> {
    if input.len() < 2 {
        return None;
    }
    let first = input.chars().next()?;
    let last = input.chars().last()?;
    if !matches!((first, last), ('"', '"') | ('\'', '\'') | ('`', '`')) {
        return None;
    }
    let inner = &input[1..input.len() - 1];
    let unescaped = inner.replace("\\\"", "\"").replace("\\\\", "\\");
    Some(unescaped.trim().to_string())
}

fn normalize_string_arg(input: &str) -> String {
    let mut out = input.trim().to_string();
    while out.len() >= 2 {
        let mut changed = false;
        for (start, end) in [("\"", "\""), ("'", "'"), ("`", "`")] {
            if out.starts_with(start) && out.ends_with(end) {
                out = out[start.len()..out.len() - end.len()].trim().to_string();
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    out
}

fn normalize_regex_pattern(input: &str) -> String {
    let out = normalize_string_arg(input);
    if out.len() >= 2 && out.starts_with('/') && out.ends_with('/') {
        out[1..out.len() - 1].to_string()
    } else {
        out
    }
}

fn prepare_gemma_native_messages(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    let mut system_blocks = Vec::new();
    let mut prepared = Vec::new();
    let mut seeded = false;

    for message in messages {
        if message.role == "system" {
            let cleaned = strip_legacy_turn_wrappers(message.content.as_str()).trim().to_string();
            if !cleaned.is_empty() {
                system_blocks.push(cleaned);
            }
            continue;
        }

        let mut clone = message.clone();
        clone.content = MessageContent::Text(strip_legacy_turn_wrappers(message.content.as_str()));

        if !seeded && message.role == "user" {
            let mut merged = String::new();
            if !system_blocks.is_empty() {
                merged.push_str("System instructions for this turn:\n");
                merged.push_str(&system_blocks.join("\n\n"));
                merged.push_str("\n\n");
            }
            merged.push_str(clone.content.as_str());
            clone.content = MessageContent::Text(merged);
            seeded = true;
        }

        prepared.push(clone);
    }

    if !seeded && !system_blocks.is_empty() {
        prepared.insert(
            0,
            ChatMessage::user(&format!(
                "System instructions for this turn:\n{}",
                system_blocks.join("\n\n")
            )),
        );
    }

    prepared
}

fn strip_legacy_turn_wrappers(text: &str) -> String {
    text.replace("<|turn>system\n", "")
        .replace("<|turn>user\n", "")
        .replace("<|turn>assistant\n", "")
        .replace("<|turn>tool\n", "")
        .replace("<turn|>", "")
        .trim()
        .to_string()
}

pub fn strip_native_tool_call_text(text: &str) -> String {
    use regex::Regex;
    let re_call = Regex::new(
        r#"(?s)<\|?tool_call\|?>\s*call:[A-Za-z_][A-Za-z0-9_]*\{.*?\}(?:<\|?tool_call\|?>|\[END_TOOL_REQUEST\])"#
    ).unwrap();
    let re_response = Regex::new(
        r#"(?s)<\|tool_response\|?>.*?(?:<\|tool_response\|?>|<tool_response\|>)"#
    ).unwrap();
    let without_calls = re_call.replace_all(text, "");
    re_response
        .replace_all(without_calls.as_ref(), "")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_gemma_native_tool_call_with_mixed_tool_call_tags() {
        let text = r#"<|channel>thought
Reading the next chunk.<channel|>The startup banner wording is likely defined within the UI drawing logic.
<|tool_call>call:read_file{limit:100,offset:100,path:\"src/ui/tui.rs\"}<tool_call|>"#;

        let calls = extract_native_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "read_file");

        let args: Value = serde_json::from_str(&calls[0].function.arguments).unwrap();
        assert_eq!(args.get("limit").and_then(|v| v.as_i64()), Some(100));
        assert_eq!(args.get("offset").and_then(|v| v.as_i64()), Some(100));
        assert_eq!(
            args.get("path").and_then(|v| v.as_str()),
            Some("src/ui/tui.rs")
        );

        let stripped = strip_native_tool_call_text(text);
        assert!(!stripped.contains("<|tool_call"));
        assert!(!stripped.contains("<tool_call|>"));
    }

    #[test]
    fn strips_hallucinated_tool_responses_from_native_tool_transcript() {
        let text = r#"<|channel>thought
Planning.
<channel|><|tool_call>call:map_project{focus:<|\"|>src/<|\"|>,include_symbols:true}<tool_call|><|tool_response>thought
Mapped src.
<channel|><|tool_call>call:read_file{limit:100,offset:0,path:<|\"|>src/main.rs<|\"|>}<tool_call|><|tool_response>thought
Read main.
<channel|>"#;

        let calls = extract_native_tool_calls(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].function.name, "map_project");
        assert_eq!(calls[1].function.name, "read_file");

        let stripped = strip_native_tool_call_text(text);
        assert!(!stripped.contains("<|tool_call"));
        assert!(!stripped.contains("<|tool_response"));
        assert!(!stripped.contains("<tool_response|>"));
    }
}
