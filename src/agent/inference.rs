use tokio::sync::{mpsc, Semaphore};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::agent::economics::{SessionEconomics, ToolRecord};

// ── Engine ────────────────────────────────────────────────────────────────────

pub struct InferenceEngine {
    pub client: reqwest::Client,
    pub api_url: String,
    pub species: String,
    pub snark: u8,
    pub kv_semaphore: Semaphore,
    /// The model ID currently loaded in LM Studio (auto-detected on boot).
    pub model: String,
    /// Context window length in tokens (auto-detected from LM Studio, default 32768).
    pub context_length: usize,
    pub economics: std::sync::Arc<std::sync::Mutex<SessionEconomics>>,
    /// Optional model ID for worker-level tasks (Swarms / research).
    pub worker_model: Option<String>,
    /// Global cancellation token for hard-interrupting the inference stream.
    pub cancel_token: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

// ── OpenAI Tool Definition ────────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Serialize, Clone, Debug)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
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
        Self { role: "system".into(), content: MessageContent::Text(content.into()),
               tool_calls: Vec::new(), tool_call_id: None, name: None }
    }
    pub fn user(content: &str) -> Self {
        Self { role: "user".into(), content: MessageContent::Text(content.into()),
               tool_calls: Vec::new(), tool_call_id: None, name: None }
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
                ContentPart::ImageUrl { image_url: ImageUrlSource { url: image_url.into() } },
            ]),
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }
    }
    pub fn assistant_text(content: &str) -> Self {
        Self { role: "assistant".into(), content: MessageContent::Text(content.into()),
               tool_calls: Vec::new(), tool_call_id: None, name: None }
    }
    pub fn assistant_tool_calls(content: &str, calls: Vec<ToolCallResponse>) -> Self {
        Self { 
            role: "assistant".into(), 
            content: MessageContent::Text(content.into()), 
            tool_calls: calls, 
            tool_call_id: None, 
            name: None 
        }
    }
    pub fn tool_result(tool_call_id: &str, fn_name: &str, content: &str) -> Self {
        let native_resp = format!("<|tool_response>response:{}{}{}<tool_response|>", fn_name, "{", content);
        Self { role: "tool".into(), content: MessageContent::Text(native_resp),
               tool_calls: Vec::new(), tool_call_id: Some(tool_call_id.into()),
               name: Some(fn_name.into()) }
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
}

#[derive(Deserialize, Debug)]
struct ResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCallResponse>>,
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
    ToolCallStart { id: String, name: String, args: String },
    /// A tool call completed – show result in the TUI.
    ToolCallResult { id: String, name: String, output: String, is_error: bool },
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
    /// A generic task progress update (e.g. for single-agent tool execution).
    TaskProgress { id: String, label: String, progress: u8 },
    /// Real-time token usage update from the API.
    UsageUpdate(TokenUsage),
    /// The model ID detected on boot.
    ModelDetected(String),
}

// ── Engine implementation ─────────────────────────────────────────────────────

impl InferenceEngine {
    pub fn new(api_url: String, species: String, snark: u8) -> Result<Self, Box<dyn std::error::Error>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()?;

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
            species,
            snark,
            kv_semaphore: Semaphore::new(3),
            model: String::new(),
            context_length: 32_768, // Gemma-4 Sweet Spot (32K)
            economics: std::sync::Arc::new(std::sync::Mutex::new(SessionEconomics::new())),
            worker_model: None,
            cancel_token: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Returns true if LM Studio is reachable.
    pub async fn health_check(&self) -> bool {
        let url = "http://localhost:1234/v1/models";
        match self.client.get(url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Query /v1/models and return the first loaded model id.
    pub async fn get_loaded_model(&self) -> Option<String> {
        #[derive(Deserialize)]
        struct ModelList { data: Vec<ModelEntry> }
        #[derive(Deserialize)]
        struct ModelEntry { id: String }

        let resp = self.client
            .get("http://localhost:1234/v1/models")
            .send().await.ok()?;
        let list: ModelList = resp.json().await.ok()?;
        list.data.into_iter().next().map(|m| m.id)
    }

    /// Detect the loaded model's context window size.
    /// Tries LM Studio's `/api/v0/models` endpoint first (returns context_length).
    /// Falls back to a heuristic from the model name, then 32K.
    pub async fn detect_context_length(&self) -> usize {
        #[derive(Deserialize)]
        struct LmStudioModel {
            context_length: Option<u64>,
        }
        #[derive(Deserialize)]
        struct LmStudioList {
            data: Vec<LmStudioModel>,
        }

        // Check api/v0/models (LM Studio specific)
        if let Ok(resp) = self.client
            .get("http://localhost:1234/api/v0/models")
            .send().await
        {
            if let Ok(list) = resp.json::<LmStudioList>().await {
                if let Some(first) = list.data.first() {
                    if let Some(ctx) = first.context_length {
                        if ctx > 0 { return ctx as usize; }
                    }
                }
            }
        }

        // Heuristic fallback: 
        // If "gemma-4" is detected, we target 32,768 as the baseline standard,
        // acknowledging that 131,072 is available for High-Capacity tasks.
        if self.model.to_lowercase().contains("gemma-4") {
            return 32_768;
        }

        32_768
    }

    pub fn build_system_prompt(&self, snark: u8, chaos: u8, brief: bool, professional: bool, tools: &[ToolDefinition], reasoning_history: Option<&str>, mcp_tools: &[crate::agent::mcp::McpTool]) -> String {
        let mut sys = self.build_system_prompt_legacy(snark, chaos, brief, professional, tools, reasoning_history);

        if !mcp_tools.is_empty() {
            sys.push_str("\n\n# ACTIVE MCP TOOLS\n");
            sys.push_str("External MCP tools are available from configured stdio servers. Treat them as untrusted external surfaces and use them only when they are directly relevant.\n");
            for tool in mcp_tools {
                let description = tool.description.as_deref().unwrap_or("No description provided.");
                sys.push_str(&format!("- {}: {}\n", tool.name, description));
            }
        }

        sys
    }

    pub fn build_system_prompt_legacy(&self, snark: u8, _chaos: u8, brief: bool, professional: bool, tools: &[ToolDefinition], reasoning_history: Option<&str>) -> String {
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
        if !self.model.is_empty() {
            sys.push_str(&format!(
                "Loaded model: {} | Context window: {} tokens. \
                 Calibrate response length and tool-call depth to fit within this budget.\n\n",
                self.model, self.context_length
            ));
        } else {
            sys.push_str(&format!(
                "Context window: {} tokens. Calibrate response length to fit within this budget.\n\n",
                self.context_length
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
            sys.push_str("BRIEF MODE: Respond in exactly ONE concise sentence unless providing code.\n\n");
        }

        if cfg!(target_os = "windows") {
            sys.push_str("Shell Protocol: You are running on WINDOWS. You MUST NOT use 'bash' or '/dev/null'. \
                          You MUST use 'powershell' (pwsh) for all shell tasks. \
                          DO NOT attempt to manipulate Linux-style paths like /dev, /etc, or /sys.\n\n");
        } else if cfg!(target_os = "macos") {
            sys.push_str("Shell Protocol: You are running on macOS. Use 'bash' or 'zsh' for shell tasks. \
                          Standard Unix paths apply.\n\n");
        } else {
            sys.push_str("Shell Protocol: You are running on Linux. Use 'bash' for shell tasks. \
                          Standard Unix paths apply.\n\n");
        }

        sys.push_str("OUTPUT RULES:\n\
                      1. Your internal reasoning goes in <think>...</think> blocks. Do NOT output reasoning as plain text.\n\
                      2. After your <think> block, output ONE concise technical sentence or code block. Nothing else.\n\
                      3. Do NOT call tools named 'thought', 'think', 'reasoning', or any meta-cognitive name. These are not tools.\n\
                      4. NEGATIVE CONSTRAINT: Never use a string containing a dot (.), slash (/), or backslash (\\) as a tool name. Paths are NOT tools.\n\
                      5. NEGATIVE CONSTRAINT: Never use the name of a class, struct, or module as a tool name unless it is explicitly in the tool list.");

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
                let schema = serde_json::to_string(&tool.function.parameters).unwrap_or_else(|_| "{}".to_string());
                sys.push_str(&format!("<|tool>declaration:{}{}{}<tool|>\n", tool.function.name, "{", schema));
                sys.push_str(&format!("// {})\n", tool.function.description));
            }
        }

        sys
    }

    // ── Non-streaming call (used for agentic turns with tool support) ─────────

    /// Send messages to the model. Returns (text_content, tool_calls).
    /// Exactly one of the two will be Some on a successful response.
    pub async fn call_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        // Override the model ID for this call. None = use self.model.
        model_override: Option<&str>,
    ) -> Result<(Option<String>, Option<Vec<ToolCallResponse>>, Option<TokenUsage>), String> {
        let _permit = self.kv_semaphore.acquire().await.map_err(|e| e.to_string())?;

        let model = model_override.unwrap_or(&self.model).to_string();
        let filtered_tools = if cfg!(target_os = "windows") {
            tools.iter().filter(|t| t.function.name != "bash" && t.function.name != "sh").cloned().collect::<Vec<_>>()
        } else {
            tools.to_vec()
        };

        // Gemma-4 Protocol: Wrap all messages in turn delimiters to enforce native behavior.
        let wrapped_messages: Vec<ChatMessage> = messages.iter().map(|m| {
            let mut clone = m.clone();
            let current_text = m.content.as_str();
            // Don't double-wrap if already wrapped
            if !current_text.starts_with("<|turn>") {
                clone.content = MessageContent::Text(format!("<|turn>{}\n{}\n<turn|>", m.role, current_text));
            }
            clone
        }).collect();

        let request = ChatRequest {
            model,
            messages: wrapped_messages,
            temperature: 0.2,
            stream: false,
            tools: if filtered_tools.is_empty() { None } else { Some(filtered_tools) },
        };
        


        // Exponential backoff: retry up to 3× on 5xx / timeout / connect errors.
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

        let body: ChatResponse = res.json().await
            .map_err(|e| format!("Response parse error: {}", e))?;

        if let Some(usage) = &body.usage {
            let mut econ = self.economics.lock().unwrap();
            econ.input_tokens += usage.prompt_tokens;
            econ.output_tokens += usage.completion_tokens;
        }

        let choice = body.choices.into_iter().next()
            .ok_or_else(|| "Empty response from model".to_string())?;

        let mut tool_calls = choice.message.tool_calls;
        
        // Gemma-4 Fallback: If the model outputs native <|tool_call|> tags in the text content,
        // extract them and treat them as valid tool calls.
        if let Some(content) = &choice.message.content {
            let native_calls = extract_native_tool_calls(content);
            if !native_calls.is_empty() {
                let mut existing = tool_calls.unwrap_or_default();
                existing.extend(native_calls);
                tool_calls = Some(existing);
            }
        }

        Ok((choice.message.content, tool_calls, body.usage))
    }

    // ── Streaming call (used for plain-text responses) ────────────────────────

    /// Stream a conversation (no tools). Emits Token/Done/Error events.
    pub async fn stream_messages(
        &self,
        messages: &[ChatMessage],
        tx: mpsc::Sender<InferenceEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Gemma-4 Protocol: Wrap all messages in turn delimiters to enforce native behavior.
        let wrapped_messages: Vec<ChatMessage> = messages.iter().map(|m| {
            let mut clone = m.clone();
            let current_text = m.content.as_str();
            if !current_text.starts_with("<|turn>") {
                clone.content = MessageContent::Text(format!("<|turn>{}\n{}\n<turn|>", m.role, current_text));
            }
            clone
        }).collect();

        let request = ChatRequest {
            model: self.model.clone(),
            messages: wrapped_messages,
            temperature: 0.7,
            stream: true,
            tools: None,
        };

        let res = self.client.post(&self.api_url)
            .json(&request)
            .send()
            .await?;

        if !res.status().is_success() {
            let _ = tx.send(InferenceEvent::Error(format!("LM Studio: {}", res.status()))).await;
            return Ok(());
        }

        use futures::StreamExt;
        let mut byte_stream = res.bytes_stream();
        
        // [Collaborative Strategy] TokenBuffer refactor suggested by Hematite local agent.
        // Aggregates tokens to ensure coherent linguistic chunks for UI/Voice.
        let mut line_buffer = String::new();
        let mut content_buffer = String::new();
        let mut past_think = false;

        while let Some(item) = byte_stream.next().await {
            // Rapid hardware interrupt check
            if self.cancel_token.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }

            let chunk = item?;
            line_buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = line_buffer.find("\n\n") {
                let event_str = line_buffer.drain(..pos + 2).collect::<String>();
                let data_pos = match event_str.find("data: ") {
                    Some(p) => p,
                    None => continue,
                };
                
                let data = event_str[data_pos + 6..].trim();
                if data == "[DONE]" { break; }

                if let Ok(json) = serde_json::from_str::<Value>(data) {
                    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                        if content.is_empty() { continue; }

                        if !past_think {
                            let lc = content.to_lowercase();
                            let close = lc.find("<channel|>")
                                .map(|i| (i, "<channel|>".len()))
                                .or_else(|| lc.find("</think>").map(|i| (i, "</think>".len())));

                            if let Some((tag_start, tag_len)) = close {
                                // Flush any existing thought buffer
                                let before = &content[..tag_start];
                                content_buffer.push_str(before);
                                if !content_buffer.trim().is_empty() {
                                    let _ = tx.send(InferenceEvent::Thought(content_buffer.clone())).await;
                                }
                                content_buffer.clear();
                                
                                past_think = true;
                                let after = content[tag_start + tag_len..].trim_start_matches('\n');
                                content_buffer.push_str(after);
                            } else {
                                // Still in reasoning block
                                content_buffer.push_str(content);
                                // Heuristic: Flush thoughts on paragraph/sentence breaks for SPECULAR
                                if content_buffer.len() > 30 && (content.contains('\n') || content.contains('.')) {
                                    let _ = tx.send(InferenceEvent::Thought(content_buffer.clone())).await;
                                    content_buffer.clear();
                                }
                            }
                        } else {
                            // PAST THINK: final answer tokens.
                            // [Linguistic Buffering] Aggregate into content_buffer until a boundary is hit.
                            content_buffer.push_str(content);
                            let is_boundary = content.contains(' ') || content.contains('.') || content.contains('!') || content.contains('?');
                            
                            if content_buffer.len() > 10 && is_boundary {
                                let _ = tx.send(InferenceEvent::Token(content_buffer.clone())).await;
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
                let _ = tx.send(InferenceEvent::Token(content_buffer)).await;
            } else {
                let _ = tx.send(InferenceEvent::Thought(content_buffer)).await;
            }
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
        let messages = vec![
            ChatMessage::system(&system),
            ChatMessage::user(prompt),
        ];
        self.stream_messages(&messages, tx).await
    }

    // ── Swarm worker helpers (non-streaming) ──────────────────────────────────
    
    /// Runs a task using the `worker_model` if set, otherwise falls back to the main `model`.
    pub async fn generate_task_worker(&self, prompt: &str, professional: bool) -> Result<String, String> {
        let model = self.worker_model.as_deref().unwrap_or(&self.model);
        self.generate_task_with_model(prompt, 0.1, professional, model).await
    }

    pub async fn generate_task(&self, prompt: &str, professional: bool) -> Result<String, String> {
        self.generate_task_with_temp(prompt, 0.1, professional).await
    }

    pub async fn generate_task_with_temp(&self, prompt: &str, temp: f32, professional: bool) -> Result<String, String> {
        self.generate_task_with_model(prompt, temp, professional, &self.model).await
    }

    pub async fn generate_task_with_model(&self, prompt: &str, temp: f32, professional: bool, model: &str) -> Result<String, String> {
        let _permit = self.kv_semaphore.acquire().await.map_err(|e| e.to_string())?;

        let system = self.build_system_prompt(self.snark, 50, false, professional, &[], None, &[]);
        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage::system(&system),
                ChatMessage::user(prompt),
            ],
            temperature: temp,
            stream: false,
            tools: None,
        };

        let res = self.client.post(&self.api_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("LM Studio request failed: {}", e))?;

        let body: ChatResponse = res.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        body.choices.first()
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
        let total_chars: usize = turns.iter()
            .map(|m| m.content.as_str().len())
            .sum();
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

/// Walk from CWD up to 4 parent directories and collect instruction files.
/// Looks for CLAUDE.md, CLAUDE.local.md, and .hematite/instructions.md.
/// Deduplicates by content hash; truncates at 4KB per file, 12KB total.
fn load_instruction_files() -> String {
    use std::collections::HashSet;
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let Ok(cwd) = std::env::current_dir() else { return String::new() };
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
            if !path.exists() { continue; }
            let Ok(content) = std::fs::read_to_string(&path) else { continue };
            if content.trim().is_empty() { continue; }

            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            let h = hasher.finish();
            if !seen.insert(h) { continue; }

            let truncated = if content.len() > MAX_PER_FILE {
                format!("{}...[truncated]", &content[..MAX_PER_FILE])
            } else {
                content
            };

            if total_chars + truncated.len() > MAX_TOTAL { break; }
            total_chars += truncated.len();
            result.push_str(&format!("\n--- {} ---\n{}\n", path.display(), truncated));
        }
        match dir.parent().map(|p| p.to_owned()) {
            Some(p) => dir = p,
            None => break,
        }
    }

    if result.is_empty() { return String::new(); }
    format!("\n\n# Project Instructions\n{}", result)
}

pub fn extract_think_block(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    
    // Official Gemma-4 Native Tags
    let open_tag = "<|channel>thought";
    let close_tag = "<channel|>";

    let start_pos = lower.find(open_tag)?;
    let content_start = start_pos + open_tag.len();
    
    let close_pos = lower[content_start..].find(close_tag)
        .map(|p| content_start + p)
        .unwrap_or(text.len());

    let content = text[content_start..close_pos].trim();
    if content.is_empty() { None } else { Some(content.to_string()) }
}

pub fn strip_think_blocks(text: &str) -> String {
    let lower = text.to_lowercase();

    // Use the official Gemma-4 closing tag — answer is everything after it.
    if let Some(end) = lower.find("<channel|>").map(|i| i + "<channel|>".len()) {
        let answer = text[end..]
            .replace("<|channel>thought", "").replace("<channel|>", "");
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

    // [Gemma-4 Heuristic] If the model outputs 'naked' reasoning without tags:
    // Strip sentences like "The user asked..." or "I will structure the response..." 
    // if they appear before the first identifiable self-introduction or code block.
    if lower.contains("the user asked") || lower.contains("i will structure") || lower.contains("necessary information in my identity") {
        let lines: Vec<&str> = text.lines().collect();
        if !lines.is_empty() {
            // If the first line is pure reasoning, skip it and look for the first real content.
            for (i, line) in lines.iter().enumerate() {
                let l_line = line.to_lowercase();
                if l_line.contains("am hematite") || l_line.contains("my purpose is") || line.starts_with("#") || line.starts_with("```") {
                    return lines[i..].join("\n").trim().to_string();
                }
            }
        }
    }

    text.trim().replace("\n\n\n", "\n\n").to_string()
}
/// Extract native Gemma-4 <|tool_call|> tags from text.
/// Format: <|tool_call|>call:func_name{key:<|"|>value<|"|>, key2:value2}<tool_call|>
pub fn extract_native_tool_calls(text: &str) -> Vec<ToolCallResponse> {
    use regex::Regex;
    let mut results = Vec::new();
    
    // Regex to find the tool call block
    // Format: <|tool_call>call:func_name{args}<tool_call|>
    let re_call = Regex::new(r"<\|?tool_call\|?>call:(\w+)\{(.*?)\}<tool_call\|?>").unwrap();
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
            let val_raw = arg_cap.get(2).map(|m| m.as_str())
                .or_else(|| arg_cap.get(3).map(|m| m.as_str()))
                .unwrap_or("")
                .trim();
            
            // Try to parse as JSON types (bool, number), otherwise string
            let val = if val_raw == "true" { Value::Bool(true) }
                else if val_raw == "false" { Value::Bool(false) }
                else if let Ok(n) = val_raw.parse::<f64>() { 
                    serde_json::Number::from_f64(n).map(Value::Number).unwrap_or(Value::String(val_raw.into()))
                }
                else { Value::String(val_raw.replace("'\"", "").into()) };

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
