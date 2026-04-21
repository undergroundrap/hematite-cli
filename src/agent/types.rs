use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Role ──────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

// ── Message Content ───────────────────────────────────────────────────────────

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

// ── Chat Message ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: MessageContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".into(),
            content: MessageContent::Text(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".into(),
            content: MessageContent::Text(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }
    pub fn user_with_image(content: &str, image_url: &str) -> Self {
        Self {
            role: "user".into(),
            content: MessageContent::Parts(vec![
                ContentPart::Text {
                    text: content.into(),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrlSource {
                        url: image_url.into(),
                    },
                },
            ]),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }
    pub fn assistant_text(content: &str) -> Self {
        Self {
            role: "assistant".into(),
            content: MessageContent::Text(content.into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }
    pub fn assistant_tool_calls(content: &str, calls: Vec<ToolCallResponse>) -> Self {
        Self {
            role: "assistant".into(),
            content: MessageContent::Text(content.into()),
            tool_calls: Some(calls),
            tool_call_id: None,
            name: None,
        }
    }
    pub fn tool_result(tool_call_id: &str, fn_name: &str, content: &str) -> Self {
        Self {
            role: "tool".into(),
            content: MessageContent::Text(content.to_string()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            name: Some(fn_name.into()),
        }
    }
    pub fn tool_result_for_model(id: &str, name: &str, result: &str, _model: &str) -> Self {
        Self::tool_result(id, name, result)
    }
}

// ── Tool Call ─────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCallResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCallFn {
    pub name: String,
    #[serde(deserialize_with = "deserialize_arguments")]
    pub arguments: Value,
}

fn deserialize_arguments<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Value = serde::Deserialize::deserialize(deserializer)?;
    if let Value::String(s) = &v {
        if let Ok(parsed) = serde_json::from_str(s) {
            return Ok(parsed);
        }
    }
    Ok(v)
}

// ── Tool Definition ───────────────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
    #[serde(skip_serializing, skip_deserializing)]
    pub metadata: ToolMetadata,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub category: ToolCategory,
    pub mutates_workspace: bool,
    pub external_surface: bool,
    pub trust_sensitive: bool,
    pub read_only_friendly: bool,
    pub plan_scope: bool,
}

// ── Token Usage ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    #[serde(default)]
    pub prompt_cache_hit_tokens: usize,
    #[serde(default)]
    pub cache_read_input_tokens: usize,
}

// ── State Enums ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderRuntimeState {
    Booting,
    Live,
    Degraded,
    Recovering,
    EmptyResponse,
    ContextWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::RecoveringProvider => "recovering_provider",
            Self::BudgetReduced => "budget_reduced",
            Self::HistoryCompacted => "history_compacted",
            Self::BlockedContextWindow => "blocked_context_window",
            Self::BlockedPolicy => "blocked_policy",
            Self::BlockedRecentFileEvidence => "blocked_recent_file_evidence",
            Self::BlockedExactLineWindow => "blocked_exact_line_window",
            Self::BlockedToolLoop => "blocked_tool_loop",
            Self::BlockedVerification => "blocked_verification",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpRuntimeState {
    Unconfigured,
    Healthy,
    Degraded,
    Failed,
}

// ── Inference Event ───────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum InferenceEvent {
    Token(String),
    MutedToken(String),
    Thought(String),
    VoiceStatus(String),
    ToolCallStart {
        id: String,
        name: String,
        args: String,
    },
    ToolCallResult {
        id: String,
        name: String,
        result: String,
        is_error: bool,
    },
    ApprovalRequired {
        id: String,
        name: String,
        display: String,
        diff: Option<String>,
        mutation_label: Option<String>,
        responder: tokio::sync::oneshot::Sender<bool>,
    },
    Done,
    ChainImplementPlan,
    Error(String),
    ProviderStatus {
        state: ProviderRuntimeState,
        summary: String,
    },
    OperatorCheckpoint {
        state: OperatorCheckpointState,
        summary: String,
    },
    RecoveryRecipe {
        summary: String,
    },
    McpStatus {
        state: McpRuntimeState,
        summary: String,
    },
    CompactionPressure {
        estimated_tokens: usize,
        threshold_tokens: usize,
        percent: u8,
    },
    PromptPressure {
        estimated_input_tokens: usize,
        reserved_output_tokens: usize,
        estimated_total_tokens: usize,
        context_length: usize,
        percent: u8,
    },
    TaskProgress {
        id: String,
        label: String,
        progress: u8,
    },
    UsageUpdate(TokenUsage),
    RuntimeProfile {
        provider_name: String,
        model_id: String,
        context_length: usize,
    },
    TurnTiming {
        context_prep_ms: u128,
        inference_ms: u128,
        execution_ms: u128,
    },
    VeinStatus {
        file_count: usize,
        embedded_count: usize,
        docs_only: bool,
    },
    VeinContext {
        paths: Vec<String>,
    },
    SoulReroll {
        species: String,
        rarity: String,
        shiny: bool,
        personality: String,
    },
    CopyDiveInCommand(String),
    EmbedProfile {
        model_id: Option<String>,
    },
    ShellLine(String),
}
