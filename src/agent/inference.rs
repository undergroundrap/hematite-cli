use serde::Serialize;
use serde_json::Value;
use tokio::sync::{mpsc, Semaphore};

pub use crate::agent::economics::{SessionEconomics, ToolRecord};
pub use crate::agent::types::*;

// ── Engine ────────────────────────────────────────────────────────────────────

pub struct InferenceEngine {
    pub provider:
        std::sync::Arc<tokio::sync::RwLock<Box<dyn crate::agent::provider::ModelProvider>>>,
    pub cached_model: std::sync::Arc<std::sync::RwLock<String>>,
    pub cached_context: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    pub base_url: String,
    pub species: String,
    pub snark: u8,
    pub kv_semaphore: Semaphore,
    pub economics: std::sync::Arc<std::sync::Mutex<SessionEconomics>>,
    /// Optional model ID for worker-level tasks (Swarms / research).
    pub worker_model: Option<String>,
    /// Opt-in Gemma-native request shaping. Off by default.
    pub gemma_native_formatting: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Global cancellation token for hard-interrupting the inference stream.
    pub cancel_token: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub fn is_hematite_native_model(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    lower.contains("gemma-4") || lower.contains("gemma4")
}

fn should_use_native_formatting(engine: &InferenceEngine, model: &str) -> bool {
    is_hematite_native_model(model) && engine.gemma_native_formatting_enabled()
}

// ── OpenAI Tool Definition ────────────────────────────────────────────────────

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
        "trace_runtime_flow" => ToolMetadata {
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
        "inspect_host" => ToolMetadata {
            category: ToolCategory::Runtime,
            mutates_workspace: false,
            external_surface: false,
            trust_sensitive: false,
            read_only_friendly: true,
            plan_scope: false,
        },
        "resolve_host_issue" => ToolMetadata {
            category: ToolCategory::Runtime,
            mutates_workspace: true,
            external_surface: true,
            trust_sensitive: true,
            read_only_friendly: false,
            plan_scope: false,
        },
        "run_hematite_maintainer_workflow" => ToolMetadata {
            category: ToolCategory::Workflow,
            mutates_workspace: true,
            external_surface: false,
            trust_sensitive: true,
            read_only_friendly: false,
            plan_scope: false,
        },
        "run_workspace_workflow" => ToolMetadata {
            category: ToolCategory::Workflow,
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
            plan_scope: true,
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
// ── Message types migrated to types.rs ────────────────────────────────────────

// ── HTTP request / response shapes ───────────────────────────────────────────

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

// ── Events pushed to the TUI (migrated to types.rs) ──────────────────────────

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

        let base_url = {
            let trimmed = api_url.trim_end_matches('/');
            if let Some(scheme_end) = trimmed.find("://") {
                let after_scheme = &trimmed[scheme_end + 3..];
                if let Some(path_start) = after_scheme.find('/') {
                    format!(
                        "{}://{}",
                        &trimmed[..scheme_end],
                        &after_scheme[..path_start]
                    )
                } else {
                    trimmed.to_string()
                }
            } else {
                trimmed.to_string()
            }
        };

        let api_url_full = if api_url.ends_with("/chat/completions") {
            api_url
        } else if api_url.ends_with("/") {
            format!("{}chat/completions", api_url)
        } else {
            format!("{}/chat/completions", api_url)
        };

        let lms = crate::agent::lms::LmsHarness::new();
        let ollama_harness = crate::agent::ollama::OllamaHarness::new(&base_url);

        let provider = if base_url.contains("11434") {
            Box::new(crate::agent::provider::OllamaProvider {
                client: client.clone(),
                base_url: base_url.clone(),
                model: String::new(),
                ollama: ollama_harness,
            }) as Box<dyn crate::agent::provider::ModelProvider>
        } else {
            Box::new(crate::agent::provider::LmsProvider {
                client: client.clone(),
                api_url: api_url_full,
                base_url: base_url.clone(),
                model: String::new(),
                context_length: 32_768,
                lms,
            }) as Box<dyn crate::agent::provider::ModelProvider>
        };

        Ok(Self {
            provider: std::sync::Arc::new(tokio::sync::RwLock::new(provider)),
            cached_model: std::sync::Arc::new(std::sync::RwLock::new(String::new())),
            cached_context: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(32_768)),
            base_url: base_url.clone(),
            species: species.clone(),
            snark,
            kv_semaphore: Semaphore::new(3),
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

    pub async fn health_check(&self) -> bool {
        let p = self.provider.read().await;
        p.health_check().await
    }

    pub async fn provider_name(&self) -> String {
        let p = self.provider.read().await;
        p.name().to_string()
    }

    pub async fn get_loaded_model(&self) -> Option<String> {
        let p = self.provider.read().await;
        match p.detect_model().await {
            Ok(m) if m.is_empty() => Some("".to_string()),
            Ok(m) => Some(m),
            Err(_) => None,
        }
    }

    pub async fn get_embedding_model(&self) -> Option<String> {
        let p = self.provider.read().await;
        p.get_embedding_model().await
    }

    pub async fn load_model(&self, model_id: &str) -> Result<(), String> {
        let p = self.provider.read().await;
        p.load_model(model_id).await
    }

    pub async fn prewarm(&self) -> Result<(), String> {
        let p = self.provider.read().await;
        p.prewarm().await
    }

    pub async fn detect_context_length(&self) -> usize {
        let p = self.provider.read().await;
        p.detect_context_length().await
    }

    pub async fn set_runtime_profile(&self, model: &str, context_length: usize) {
        if let Ok(mut guard) = self.cached_model.write() {
            *guard = model.to_string();
        }
        self.cached_context
            .store(context_length, std::sync::atomic::Ordering::SeqCst);

        let mut p = self.provider.write().await;
        p.set_runtime_profile(model, context_length);
    }

    pub async fn refresh_runtime_profile(&self) -> Option<(String, usize, bool)> {
        let previous_model = self.current_model();
        let previous_context = self.current_context_length();

        let detected_model = match self.get_loaded_model().await {
            Some(m) if !m.is_empty() => m,
            Some(_) => "no model loaded".to_string(),
            None => previous_model.clone(),
        };

        let detected_context = self.detect_context_length().await;
        let effective_model = if detected_model.is_empty() {
            previous_model.clone()
        } else {
            detected_model
        };

        let changed = effective_model != previous_model || detected_context != previous_context;
        if changed {
            self.set_runtime_profile(&effective_model, detected_context)
                .await;
        }

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
        environment_summary: Option<&str>,
        mcp_tools: &[crate::agent::mcp::McpTool],
    ) -> String {
        let mut sys = self.build_system_prompt_legacy(
            snark,
            chaos,
            brief,
            professional,
            tools,
            reasoning_history,
            environment_summary,
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
        environment_summary: Option<&str>,
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
                                     - The running Hematite build is ");
        sys.push_str(&crate::hematite_version_display());
        sys.push_str(".\n\
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
        if let Some(summary) = environment_summary {
            sys.push_str("## HOST ENVIRONMENT\n");
            sys.push_str(summary);
            sys.push_str("\n\n");
        }

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
            if is_hematite_native_model(&current_model) {
                sys.push_str(
                    "Sovereign native note: prefer exact tool JSON with no extra prose when calling tools. \
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

        // Consolidated: All directives are now handled by the authoritative prompt.rs builder.
        sys.push_str("## TURN ADVISORY\n");
        if brief {
            sys.push_str("- BRIEF MODE: Respond with ONE concise sentence/block unless more code is required.\n");
        }
        sys.push_str("- INTERNAL REASONING: Plan your move in the thought channel first.\n");

        // Scaffolding protocol — enforces build validation after project creation.
        sys.push_str("\n## SCAFFOLDING PROTOCOL\n\
            2. ALWAYS call verify_build immediately after to confirm the project compiles/runs.\n\
            3. If verify_build fails, use `lsp_get_diagnostics` to find the exact line and error.\n\
            4. Fix all errors before declaring success.\n\n\
            ## PRE-FLIGHT SCOPING PROTOCOL\n\
            Before attempting any multi-file task or complex refactor:\n\
            1. Identify 1-3 core files (entry-points, central models, or types) that drive the logic.\n\
            2. Use `auto_pin_context` to keep those files in active context.\n\
            3. Only then proceed to deeper edits or research.\n\n\
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
        sys.push_str(&format!(
            "You are Hematite {}, a local coding harness working on the user's machine.\n",
            crate::hematite_version_display()
        ));
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
        if is_hematite_native_model(&current_model) {
            sys.push_str(
                "Sovereign native: use exact tool JSON. No extra prose in tool calls. \
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
        let mut sys = format!(
            "<|turn>system\nYou are Hematite {}, a local coding harness working on the user's machine.\n",
            crate::hematite_version_display()
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
            sys.push_str(&format!(
                "You are running on {}. Use the native Unix shell conventions.\n",
                os
            ));
        }
        if brief {
            sys.push_str("BRIEF MODE: answer in one concise sentence unless code is required.\n");
        }
        sys.push_str("<turn|>\n");
        sys
    }

    pub fn current_model(&self) -> String {
        self.cached_model
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn current_context_length(&self) -> usize {
        self.cached_context
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_compact_context_window(&self) -> bool {
        let len = self.current_context_length();
        len <= 16384
    }

    pub fn gemma_native_formatting_enabled(&self) -> bool {
        self.gemma_native_formatting
            .load(std::sync::atomic::Ordering::Relaxed)
    }

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

        let (res, model_name, prepared_messages) = {
            let p = self.provider.read().await;
            let model_name = model_override.unwrap_or(&p.current_model()).to_string();
            let prepared_messages = if should_use_native_formatting(self, &model_name) {
                prepare_gemma_native_messages(messages)
            } else {
                messages.to_vec()
            };
            if let Err(detail) = preflight_chat_request(
                &model_name,
                &prepared_messages,
                tools,
                self.current_context_length(),
            ) {
                return Err(format_runtime_failure_message(&detail));
            }
            let res = p
                .call_with_tools(&prepared_messages, tools, model_override)
                .await
                .map_err(|e| format_runtime_failure_message(&e))?;
            (res, model_name, prepared_messages)
        };

        if let Ok(mut econ) = self.economics.lock() {
            econ.input_tokens += res.usage.prompt_tokens;
            econ.output_tokens += res.usage.completion_tokens;
        }

        let mut content = res.content;
        let mut tool_calls = res.tool_calls;

        // Post-processing: Gemma 4 / thinking block extraction
        if let Some(text) = &content {
            if should_use_native_formatting(self, &model_name) {
                let native_calls = extract_native_tool_calls(text);
                if !native_calls.is_empty() {
                    let mut existing = tool_calls.unwrap_or_default();
                    existing.extend(native_calls);
                    tool_calls = Some(existing);

                    let stripped = strip_native_tool_call_text(text);
                    content = if stripped.trim().is_empty() {
                        None
                    } else {
                        Some(stripped)
                    };
                }
            }
        }

        // Normalization: Tool arguments
        if should_use_native_formatting(self, &model_name) {
            if let Some(calls) = tool_calls.as_mut() {
                for call in calls.iter_mut() {
                    normalize_tool_argument_value(
                        &call.function.name,
                        &mut call.function.arguments,
                    );
                }
            }
        }

        if should_use_native_formatting(self, &model_name)
            && content.is_none()
            && tool_calls.is_none()
            && !prepared_messages.is_empty()
        {
            return Err(format_runtime_failure_message(
                "model returned an empty response after native-format message preparation",
            ));
        }

        Ok((content, tool_calls, Some(res.usage), res.finish_reason))
    }

    // ── Streaming call (used for plain-text responses) ────────────────────────

    /// Stream a conversation (no tools). Emits Token/Done/Error events.
    pub async fn stream_messages(
        &self,
        messages: &[ChatMessage],
        tx: mpsc::Sender<InferenceEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let provider = self.provider.read().await;
        provider.stream(messages, tx).await
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
        let system =
            self.build_system_prompt(snark, chaos, brief, professional, &[], None, None, &[]);
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
        let model = self
            .worker_model
            .as_deref()
            .unwrap_or(current_model.as_str());
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
        _temp: f32,
        professional: bool,
        model: &str,
    ) -> Result<String, String> {
        let _permit = self
            .kv_semaphore
            .acquire()
            .await
            .map_err(|e| e.to_string())?;

        let system =
            self.build_system_prompt(self.snark, 50, false, professional, &[], None, None, &[]);
        let messages = vec![ChatMessage::system(&system), ChatMessage::user(prompt)];
        if let Err(detail) =
            preflight_chat_request(model, &messages, &[], self.current_context_length())
        {
            return Err(format_runtime_failure_message(&detail));
        }

        let p = self.provider.read().await;
        let res = p
            .call_with_tools(&messages, &[], Some(model))
            .await
            .map_err(|e| format_runtime_failure_message(&e))?;

        res.content
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

const IMAGE_PART_TOKEN_ESTIMATE: usize = 1024;

fn estimate_message_tokens(message: &ChatMessage) -> usize {
    let content_tokens = match &message.content {
        MessageContent::Text(s) => s.len() / 4 + 1,
        MessageContent::Parts(parts) => parts
            .iter()
            .map(|part| match part {
                ContentPart::Text { text } => text.len() / 4 + 1,
                // Image payloads are transported as data URLs, but their base64
                // length should not be treated like plain text context pressure.
                ContentPart::ImageUrl { .. } => IMAGE_PART_TOKEN_ESTIMATE,
            })
            .sum(),
    };
    let tool_tokens: usize = message
        .tool_calls
        .iter()
        .flatten()
        .map(|call| (call.function.name.len() + call.function.arguments.to_string().len()) / 4 + 4)
        .sum();
    content_tokens + tool_tokens + 6
}

pub fn estimate_message_batch_tokens(messages: &[ChatMessage]) -> usize {
    messages.iter().map(estimate_message_tokens).sum()
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
        estimate_message_batch_tokens(messages) + estimate_serialized_tokens(tools) + 32;
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
    // Fast-path: strip a stray </think> the model emits at the start when it skips
    // the opening tag (common with Qwen after tool calls). Strip it before the lower
    // allocation so it can't slip through any branch below.
    let text = {
        let t = text.trim_start();
        if t.to_lowercase().starts_with("</think>") {
            &t[8..]
        } else {
            text
        }
    };

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
    // Strip leading sentences like "The user asked..." or "I should present..."
    // if they appear before actual answer content.
    let naked_reasoning_phrases: &[&str] = &[
        "the user asked",
        "the user is asking",
        "the user wants",
        "i will structure",
        "i should provide",
        "i should give",
        "i should avoid",
        "i should note",
        "i should focus",
        "i should keep",
        "i should respond",
        "i should present",
        "i should display",
        "i should show",
        "i need to",
        "i can see from",
        "without being overly",
        "let me ",
        "necessary information in my identity",
        "was computed successfully",
        "computed successfully",
    ];
    let is_naked_reasoning = naked_reasoning_phrases.iter().any(|p| lower.contains(p));
    if is_naked_reasoning {
        let lines: Vec<&str> = text.lines().collect();
        if !lines.is_empty() {
            // Skip leading lines that are themselves reasoning prose or blank.
            // Stop skipping at the first line that looks like real answer content.
            let mut start_idx = 0;
            for (i, line) in lines.iter().enumerate() {
                let l = line.to_lowercase();
                let is_reasoning_line =
                    naked_reasoning_phrases.iter().any(|p| l.contains(p)) || l.trim().is_empty();
                if is_reasoning_line {
                    start_idx = i + 1;
                } else {
                    break;
                }
            }
            if start_idx < lines.len() {
                return lines[start_idx..]
                    .join("\n")
                    .trim()
                    .replace("\n\n\n", "\n\n")
                    .to_string();
            }
            // Entire response was reasoning prose — return empty.
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
        "</tool_call>",
        "<tool_call>",
        "</function>",
        "<function>",
        "</parameter>",
        "<parameter>",
        "</arguments>",
        "<arguments>",
        "</tool_use>",
        "<tool_use>",
        "</invoke>",
        "<invoke>",
        // Stray think/reasoning closing tags that leak after block extraction.
        "</think>",
        "</thought>",
        "</thinking>",
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

    // -- Format 1: Gemma 4 Native (call:name{args}) --
    let re_call = Regex::new(
        r#"(?s)<\|?tool_call\|?>\s*call:([A-Za-z_][A-Za-z0-9_]*)\{(.*?)\}(?:<\|?tool_call\|?>|\[END_TOOL_REQUEST\])"#
    ).unwrap();
    let re_arg = Regex::new(r#"(\w+):(?:<\|"\|>(.*?)<\|"\|>|([^,}]*))"#).unwrap();

    for cap in re_call.captures_iter(text) {
        let name = cap[1].to_string();
        let args_str = &cap[2];
        let mut arguments = serde_json::Map::new();

        for arg_cap in re_arg.captures_iter(args_str) {
            let key = arg_cap[1].to_string();
            let val_raw = arg_cap
                .get(2)
                .map(|m| m.as_str())
                .or_else(|| arg_cap.get(3).map(|m| m.as_str()))
                .unwrap_or("")
                .trim();
            let normalized_raw = normalize_string_arg(&val_raw.replace("\\\"", "\""));

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
                arguments: Value::Object(arguments),
            },
            index: None,
        });
    }

    // -- Format 2: XML (Qwen/Claude style) --
    let re_xml_call = Regex::new(
        r#"(?s)<tool_call>\s*<function=([A-Za-z_][A-Za-z0-9_]*)>(.*?)(?:</function>)?\s*</tool_call>"#
    ).unwrap();
    let re_xml_param =
        Regex::new(r#"(?s)<parameter=([A-Za-z_][A-Za-z0-9_]*)>(.*?)</parameter>"#).unwrap();

    for cap in re_xml_call.captures_iter(text) {
        let name = cap[1].to_string();
        let body = &cap[2];
        let mut arguments = serde_json::Map::new();

        for p_cap in re_xml_param.captures_iter(body) {
            let key = p_cap[1].to_string();
            let val_raw = p_cap[2].trim();
            let val = if val_raw == "true" {
                Value::Bool(true)
            } else if val_raw == "false" {
                Value::Bool(false)
            } else if let Ok(n) = val_raw.parse::<i64>() {
                Value::Number(n.into())
            } else if let Ok(n) = val_raw.parse::<u64>() {
                Value::Number(n.into())
            } else {
                Value::String(val_raw.to_string())
            };
            arguments.insert(key, val);
        }

        results.push(ToolCallResponse {
            id: format!("call_{}", rand::random::<u32>()),
            call_type: "function".to_string(),
            function: ToolCallFn {
                name,
                arguments: Value::Object(arguments),
            },
            index: None,
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

pub fn normalize_tool_argument_value(tool_name: &str, value: &mut Value) {
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
            let cleaned = strip_legacy_turn_wrappers(message.content.as_str())
                .trim()
                .to_string();
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
    // Format 1: Gemma 4 Native
    let re_call = Regex::new(
        r#"(?s)<\|?tool_call\|?>\s*call:[A-Za-z_][A-Za-z0-9_]*\{.*?\}(?:<\|?tool_call\|?>|\[END_TOOL_REQUEST\])"#
    ).unwrap();
    // Format 2: XML (Qwen/Claude style)
    let re_xml = Regex::new(r#"(?s)<tool_call>\s*<function=.*?>.*?</tool_call>"#).unwrap();
    let re_response =
        Regex::new(r#"(?s)<\|tool_response\|?>.*?(?:<\|tool_response\|?>|<tool_response\|>)"#)
            .unwrap();
    let without_calls = re_call.replace_all(text, "");
    let without_xml = re_xml.replace_all(without_calls.as_ref(), "");
    re_response
        .replace_all(without_xml.as_ref(), "")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_includes_running_hematite_version() {
        let engine = InferenceEngine::new(
            "http://localhost:1234/v1".to_string(),
            "strategist".to_string(),
            0,
        )
        .expect("engine");

        let system = engine.build_system_prompt(0, 50, false, true, &[], None, None, &[]);
        assert!(system.contains(crate::HEMATITE_VERSION));
    }

    #[test]
    fn extracts_gemma_native_tool_call_with_mixed_tool_call_tags() {
        let text = r#"<|channel>thought
Reading the next chunk.<channel|>The startup banner wording is likely defined within the UI drawing logic.
<|tool_call>call:read_file{limit:100,offset:100,path:\"src/ui/tui.rs\"}<tool_call|>"#;

        let calls = extract_native_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "read_file");

        let args: Value = calls[0].function.arguments.clone();
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
<channel|><|tool_call>call:list_files{extension:<|\"|>rs<|\"|>,path:<|\"|>src/<|\"|>}<tool_call|><|tool_response>thought
Mapped src.
<channel|><|tool_call>call:read_file{limit:100,offset:0,path:<|\"|>src/main.rs<|\"|>}<tool_call|><|tool_response>thought
Read main.
<channel|>"#;

        let calls = extract_native_tool_calls(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].function.name, "list_files");
        assert_eq!(calls[1].function.name, "read_file");

        let stripped = strip_native_tool_call_text(text);
        assert!(!stripped.contains("<|tool_call"));
        assert!(!stripped.contains("<|tool_response"));
        assert!(!stripped.contains("<tool_response|>"));
    }

    #[test]
    fn extracts_qwen_xml_tool_calls_from_reasoning() {
        let text = r#"Based on the project structure, I need to check the binary.
<tool_call>
<function=shell>
<parameter=command>
ls -la hematite.exe
</parameter>
<parameter=reason>
Check if the binary exists
</parameter>
</function>
</tool_call>"#;

        let calls = extract_native_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "shell");

        let args: Value = calls[0].function.arguments.clone();
        assert_eq!(
            args.get("command").and_then(|v| v.as_str()),
            Some("ls -la hematite.exe")
        );
        assert_eq!(
            args.get("reason").and_then(|v| v.as_str()),
            Some("Check if the binary exists")
        );

        let stripped = strip_native_tool_call_text(text);
        assert!(!stripped.contains("<tool_call>"));
        assert!(!stripped.contains("<function=shell>"));
    }
}
