use super::modal_review::{draw_diff_review, ActiveReview};
use crate::agent::conversation::{AttachedDocument, AttachedImage, UserTurn};
use crate::agent::inference::{McpRuntimeState, OperatorCheckpointState, ProviderRuntimeState};
use crate::agent::specular::SpecularEvent;
use crate::agent::swarm::{ReviewResponse, SwarmMessage};
use crate::agent::utils::{strip_ansi, CRLF_REGEX};
use crate::ui::gpu_monitor::GpuState;
use crossterm::event::{self, Event, EventStream, KeyCode};
use futures::StreamExt;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Terminal,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc::Receiver;
use walkdir::WalkDir;

fn provider_badge_prefix(provider_name: &str) -> &'static str {
    match provider_name {
        "LM Studio" => "LM",
        "Ollama" => "OL",
        _ => "AI",
    }
}

fn provider_state_label(state: ProviderRuntimeState) -> &'static str {
    match state {
        ProviderRuntimeState::Booting => "booting",
        ProviderRuntimeState::Live => "live",
        ProviderRuntimeState::Degraded => "degraded",
        ProviderRuntimeState::Recovering => "recovering",
        ProviderRuntimeState::EmptyResponse => "empty_response",
        ProviderRuntimeState::ContextWindow => "context_window",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeIssueKind {
    Healthy,
    Booting,
    Recovering,
    NoModel,
    Connectivity,
    EmptyResponse,
    ContextCeiling,
}

fn classify_runtime_issue(
    provider_state: ProviderRuntimeState,
    model_id: &str,
    context_length: usize,
    provider_summary: &str,
) -> RuntimeIssueKind {
    if provider_state == ProviderRuntimeState::ContextWindow {
        return RuntimeIssueKind::ContextCeiling;
    }
    if model_id.trim() == "no model loaded" {
        return RuntimeIssueKind::NoModel;
    }
    if provider_state == ProviderRuntimeState::EmptyResponse {
        return RuntimeIssueKind::EmptyResponse;
    }
    if provider_state == ProviderRuntimeState::Recovering {
        return RuntimeIssueKind::Recovering;
    }
    if provider_state == ProviderRuntimeState::Booting
        || model_id.trim().is_empty()
        || model_id.trim() == "detecting..."
        || context_length == 0
    {
        return RuntimeIssueKind::Booting;
    }
    if provider_state == ProviderRuntimeState::Degraded {
        let lower = provider_summary.to_ascii_lowercase();
        if lower.contains("empty reply") || lower.contains("empty response") {
            return RuntimeIssueKind::EmptyResponse;
        }
        if lower.contains("context ceiling") || lower.contains("context window") {
            return RuntimeIssueKind::ContextCeiling;
        }
        return RuntimeIssueKind::Connectivity;
    }
    RuntimeIssueKind::Healthy
}

fn runtime_issue_kind(app: &App) -> RuntimeIssueKind {
    classify_runtime_issue(
        app.provider_state,
        &app.model_id,
        app.context_length,
        &app.last_provider_summary,
    )
}

fn runtime_issue_label(issue: RuntimeIssueKind) -> &'static str {
    match issue {
        RuntimeIssueKind::Healthy => "healthy",
        RuntimeIssueKind::Booting => "booting",
        RuntimeIssueKind::Recovering => "recovering",
        RuntimeIssueKind::NoModel => "no_model",
        RuntimeIssueKind::Connectivity => "connectivity",
        RuntimeIssueKind::EmptyResponse => "empty_response",
        RuntimeIssueKind::ContextCeiling => "context_ceiling",
    }
}

fn runtime_issue_badge(issue: RuntimeIssueKind) -> (&'static str, Color) {
    match issue {
        RuntimeIssueKind::Healthy => ("OK", Color::Green),
        RuntimeIssueKind::Booting => ("WAIT", Color::DarkGray),
        RuntimeIssueKind::Recovering => ("RECV", Color::Cyan),
        RuntimeIssueKind::NoModel => ("MOD", Color::Red),
        RuntimeIssueKind::Connectivity => ("NET", Color::Red),
        RuntimeIssueKind::EmptyResponse => ("EMP", Color::Red),
        RuntimeIssueKind::ContextCeiling => ("CTX", Color::Yellow),
    }
}

fn mcp_state_label(state: McpRuntimeState) -> &'static str {
    match state {
        McpRuntimeState::Unconfigured => "unconfigured",
        McpRuntimeState::Healthy => "healthy",
        McpRuntimeState::Degraded => "degraded",
        McpRuntimeState::Failed => "failed",
    }
}

fn runtime_configured_endpoint() -> String {
    let config = crate::agent::config::load_config();
    config
        .api_url
        .clone()
        .unwrap_or_else(|| crate::agent::config::DEFAULT_LM_STUDIO_API_URL.to_string())
}

fn runtime_session_provider(app: &App) -> String {
    if app.provider_name.trim().is_empty() {
        "detecting".to_string()
    } else {
        app.provider_name.clone()
    }
}

fn runtime_session_endpoint(app: &App, configured_endpoint: &str) -> String {
    if app.provider_endpoint.trim().is_empty() {
        configured_endpoint.to_string()
    } else {
        app.provider_endpoint.clone()
    }
}

async fn format_provider_summary(app: &App) -> String {
    let config = crate::agent::config::load_config();
    let active_provider = runtime_session_provider(app);
    let active_endpoint = runtime_session_endpoint(
        app,
        &config.api_url.clone().unwrap_or_else(|| {
            crate::agent::config::default_api_url_for_provider(&active_provider).to_string()
        }),
    );
    let saved = config
        .api_url
        .as_ref()
        .map(|url| {
            format!(
                "{} ({})",
                crate::agent::config::provider_label_for_api_url(url),
                url
            )
        })
        .unwrap_or_else(|| {
            format!(
                "default LM Studio ({})",
                crate::agent::config::DEFAULT_LM_STUDIO_API_URL
            )
        });
    let alternative = crate::runtime::detect_alternative_provider(&active_provider)
        .await
        .map(|(name, url)| format!("Reachable alternative: {} ({})", name, url))
        .unwrap_or_else(|| "Reachable alternative: none detected".to_string());
    format!(
        "Active provider: {} | Session endpoint: {}\nSaved preference: {}\n{}\n\nUse /provider lmstudio, /provider ollama, /provider clear, or /provider <url>.\nProvider changes apply to new sessions; restart Hematite to switch this one.",
        active_provider, active_endpoint, saved, alternative
    )
}

fn runtime_fix_path(app: &App) -> String {
    let session_provider = runtime_session_provider(app);
    match runtime_issue_kind(app) {
        RuntimeIssueKind::NoModel => {
            if session_provider == "Ollama" {
                format!(
                    "Shortest fix: pull or run a chat model in Ollama, then keep `api_url` on `{}`. Hematite cannot safely auto-load that model for you here.",
                    crate::agent::config::DEFAULT_OLLAMA_API_URL
                )
            } else {
                format!(
                    "Shortest fix: load a coding model in LM Studio and keep the local server on `{}`. Hematite cannot safely auto-load that model for you here.",
                    crate::agent::config::DEFAULT_LM_STUDIO_API_URL
                )
            }
        }
        RuntimeIssueKind::ContextCeiling => {
            format!(
                "Shortest fix: narrow the request, let Hematite compact if needed, and run `/runtime fix` to refresh and re-check the active provider (`{}`).",
                session_provider
            )
        }
        RuntimeIssueKind::Connectivity | RuntimeIssueKind::Recovering => {
            format!(
                "Shortest fix: run `/runtime fix` to refresh and re-check the active provider (`{}`). If needed after that, use `/runtime provider <name>` and restart Hematite.",
                session_provider
            )
        }
        RuntimeIssueKind::EmptyResponse => {
            "Shortest fix: run `/runtime fix` to refresh the active runtime, then retry once with a narrower grounded request if the provider keeps answering empty.".to_string()
        }
        RuntimeIssueKind::Booting => {
            format!(
                "Shortest fix: wait for the active provider (`{}`) to stabilize, then run `/runtime fix` or `/runtime refresh` if detection stays stale.",
                session_provider
            )
        }
        RuntimeIssueKind::Healthy => {
            if app.embed_model_id.is_none() {
                "Shortest fix: optional only — load a preferred embedding model if you want semantic file search."
                    .to_string()
            } else {
                "Shortest fix: none — runtime is healthy.".to_string()
            }
        }
    }
}

async fn format_runtime_summary(app: &App) -> String {
    let config = crate::agent::config::load_config();
    let configured_endpoint = runtime_configured_endpoint();
    let configured_provider =
        crate::agent::config::provider_label_for_api_url(&configured_endpoint);
    let session_provider = runtime_session_provider(app);
    let session_endpoint = runtime_session_endpoint(app, &configured_endpoint);
    let issue = runtime_issue_kind(app);
    let coding_model = if app.model_id.trim().is_empty() {
        "detecting...".to_string()
    } else {
        app.model_id.clone()
    };
    let embed_status = match app.embed_model_id.as_deref() {
        Some(id) => format!("loaded ({})", id),
        None => "not loaded".to_string(),
    };
    let semantic_status = if app.embed_model_id.is_some() || app.vein_embedded_count > 0 {
        "ready"
    } else {
        "inactive"
    };
    let preferred_coding = crate::agent::config::preferred_coding_model(&config)
        .unwrap_or_else(|| "none saved".to_string());
    let preferred_embed = config
        .embed_model
        .clone()
        .unwrap_or_else(|| "none saved".to_string());
    let alternative = crate::runtime::detect_alternative_provider(&session_provider).await;
    let alternative_line = alternative
        .as_ref()
        .map(|(name, url)| format!("Reachable alternative: {} ({})", name, url))
        .unwrap_or_else(|| "Reachable alternative: none detected".to_string());
    let provider_controls = if session_provider == "Ollama" {
        "Provider controls: Ollama coding+embed load/unload is available here; `--ctx` maps to Ollama `num_ctx` for coding models."
    } else {
        "Provider controls: LM Studio coding+embed load/unload is available here; `--ctx` maps to LM Studio context length."
    };
    format!(
        "Configured provider: {} ({})\nSession provider: {} ({})\nProvider state: {}\nPrimary issue: {}\nCoding model: {}\nPreferred coding model: {}\nCTX: {}\nEmbedding model: {}\nPreferred embed model: {}\nSemantic search: {} | embedded chunks: {}\nMCP: {}\n{}\n{}\n{}\n\nTry: /runtime explain, /runtime fix, /model status, /model list loaded",
        configured_provider,
        configured_endpoint,
        session_provider,
        session_endpoint,
        provider_state_label(app.provider_state),
        runtime_issue_label(issue),
        coding_model,
        preferred_coding,
        app.context_length,
        embed_status,
        preferred_embed,
        semantic_status,
        app.vein_embedded_count,
        mcp_state_label(app.mcp_state),
        alternative_line,
        provider_controls,
        runtime_fix_path(app)
    )
}

async fn format_runtime_explanation(app: &App) -> String {
    let session_provider = runtime_session_provider(app);
    let issue = runtime_issue_kind(app);
    let coding_model = if app.model_id.trim().is_empty() {
        "detecting...".to_string()
    } else {
        app.model_id.clone()
    };
    let semantic = if app.embed_model_id.is_some() || app.vein_embedded_count > 0 {
        "semantic search is ready"
    } else {
        "semantic search is inactive"
    };
    let state_line = match app.provider_state {
        ProviderRuntimeState::Live => format!(
            "{} is live, Hematite sees model `{}`, and {}.",
            session_provider, coding_model, semantic
        ),
        ProviderRuntimeState::Booting => format!(
            "{} is still booting or being detected. Hematite has not stabilized the runtime view yet.",
            session_provider
        ),
        ProviderRuntimeState::Recovering => format!(
            "{} hit a runtime problem recently and Hematite is still trying to recover cleanly.",
            session_provider
        ),
        ProviderRuntimeState::Degraded => format!(
            "{} is reachable but degraded, so responses may fail or stall until the runtime is stable again.",
            session_provider
        ),
        ProviderRuntimeState::EmptyResponse => format!(
            "{} answered without useful content, which usually means the runtime needs attention even if the endpoint is still up.",
            session_provider
        ),
        ProviderRuntimeState::ContextWindow => format!(
            "{} hit its active context ceiling, so the problem is prompt budget rather than basic connectivity.",
            session_provider
        ),
    };
    let model_line = if coding_model == "no model loaded" {
        "No coding model is loaded right now, so Hematite cannot do real model work until one is available.".to_string()
    } else {
        format!("The current coding model is `{}`.", coding_model)
    };
    let alternative = crate::runtime::detect_alternative_provider(&session_provider)
        .await
        .map(|(name, url)| format!("A reachable alternative exists: {} ({}).", name, url))
        .unwrap_or_else(|| "No other reachable local runtime is currently detected.".to_string());
    format!(
        "Primary issue: {}\n{}\n{}\n{}\n{}",
        runtime_issue_label(issue),
        state_line,
        model_line,
        alternative,
        runtime_fix_path(app)
    )
}

async fn handle_runtime_fix(app: &mut App) {
    let session_provider = runtime_session_provider(app);
    let issue = runtime_issue_kind(app);
    let alternative = crate::runtime::detect_alternative_provider(&session_provider).await;

    if issue == RuntimeIssueKind::NoModel {
        let mut message = runtime_fix_path(app);
        if let Some((name, url)) = alternative {
            message.push_str(&format!(
                "\nReachable alternative: {} ({}). Hematite will not switch providers silently; use `/runtime provider {}` and restart if you want that runtime instead.",
                name,
                url,
                name.to_ascii_lowercase()
            ));
        }
        app.push_message("System", &message);
        app.history_idx = None;
        return;
    }

    if matches!(
        issue,
        RuntimeIssueKind::Booting
            | RuntimeIssueKind::Recovering
            | RuntimeIssueKind::Connectivity
            | RuntimeIssueKind::EmptyResponse
            | RuntimeIssueKind::ContextCeiling
    ) {
        let _ = app
            .user_input_tx
            .try_send(UserTurn::text("/runtime-refresh"));
        app.push_message("You", "/runtime fix");
        app.provider_state = ProviderRuntimeState::Recovering;
        app.agent_running = true;

        let mut message = format!(
            "Running the shortest safe fix now: refreshing the {} runtime profile and re-checking the active model/context window.",
            session_provider
        );
        if let Some((name, url)) = alternative {
            message.push_str(&format!(
                "\nReachable alternative: {} ({}). Hematite will stay on the current provider unless you explicitly switch with `/runtime provider {}` and restart.",
                name,
                url,
                name.to_ascii_lowercase()
            ));
        }
        app.push_message("System", &message);
        if issue == RuntimeIssueKind::EmptyResponse {
            if let Some(fallback) =
                build_runtime_fix_grounded_fallback(&app.recent_grounded_results)
            {
                app.push_message(
                    "System",
                    "The last turn already produced grounded tool output, so Hematite is surfacing a bounded fallback while the runtime refresh completes.",
                );
                app.push_message("Hematite", &fallback);
            } else {
                app.push_message(
                    "System",
                    "Runtime refresh requested successfully. The failed turn has no safe grounded fallback cached, so retry the turn once the runtime settles.",
                );
            }
        }
        app.history_idx = None;
        return;
    }

    if issue == RuntimeIssueKind::Healthy && app.embed_model_id.is_none() {
        app.push_message(
            "System",
            "Runtime is already healthy. The only missing piece is optional semantic search; load your preferred embedding model if you want embedding-backed file retrieval.",
        );
        app.history_idx = None;
        return;
    }

    app.push_message(
        "System",
        "Runtime is already healthy. `/runtime fix` has nothing safe to change right now.",
    );
    app.history_idx = None;
}

async fn handle_provider_command(app: &mut App, arg_text: String) {
    if arg_text.is_empty() || arg_text.eq_ignore_ascii_case("status") {
        app.push_message("System", &format_provider_summary(app).await);
        app.history_idx = None;
        return;
    }

    let lower = arg_text.to_ascii_lowercase();
    let result = match lower.as_str() {
        "lmstudio" | "lm" => {
            crate::agent::config::set_api_url_override(Some(
                crate::agent::config::DEFAULT_LM_STUDIO_API_URL,
            ))
            .map(|_| {
                format!(
                    "Saved provider preference: LM Studio ({}) in `.hematite/settings.json`.\nRestart Hematite to switch this session.",
                    crate::agent::config::DEFAULT_LM_STUDIO_API_URL
                )
            })
        }
        "ollama" | "ol" => {
            crate::agent::config::set_api_url_override(Some(
                crate::agent::config::DEFAULT_OLLAMA_API_URL,
            ))
            .map(|_| {
                format!(
                    "Saved provider preference: Ollama ({}) in `.hematite/settings.json`.\nRestart Hematite to switch this session.",
                    crate::agent::config::DEFAULT_OLLAMA_API_URL
                )
            })
        }
        "clear" | "default" => crate::agent::config::set_api_url_override(None).map(|_| {
            format!(
                "Cleared the saved provider override. New sessions will fall back to LM Studio ({}) unless `--url` overrides it.\nRestart Hematite to switch this session.",
                crate::agent::config::DEFAULT_LM_STUDIO_API_URL
            )
        }),
        _ if lower.starts_with("http://") || lower.starts_with("https://") => {
            crate::agent::config::set_api_url_override(Some(&arg_text)).map(|_| {
                format!(
                    "Saved provider endpoint override: {} ({}) in `.hematite/settings.json`.\nRestart Hematite to switch this session.",
                    crate::agent::config::provider_label_for_api_url(&arg_text),
                    arg_text
                )
            })
        }
        _ => Err("Usage: /provider [status|lmstudio|ollama|clear|http://host:port/v1]".to_string()),
    };

    match result {
        Ok(message) => app.push_message("System", &message),
        Err(error) => app.push_message("System", &error),
    }
    app.history_idx = None;
}

// ── Approval modal state ──────────────────────────────────────────────────────

/// Holds a pending high-risk tool approval request.
/// The agent loop is blocked on `responder` until the user presses Y or N.
pub struct PendingApproval {
    pub display: String,
    pub tool_name: String,
    /// Pre-formatted diff from `compute_*_diff`.  Lines starting with "- " are
    /// removals (red), "+ " are additions (green), "---" / "@@ " are headers.
    pub diff: Option<String>,
    /// Current scroll offset for the diff body (lines scrolled down).
    pub diff_scroll: u16,
    pub mutation_label: Option<String>,
    pub responder: tokio::sync::oneshot::Sender<bool>,
}

// ── App state ─────────────────────────────────────────────────────────────────

pub struct RustyStats {
    pub debugging: u32,
    pub wisdom: u16,
    pub patience: f32,
    pub chaos: u8,
    pub snark: u8,
}

use std::collections::HashMap;

#[derive(Clone)]
pub struct ContextFile {
    pub path: String,
    pub size: u64,
    pub status: String,
}

fn default_active_context() -> Vec<ContextFile> {
    let root = crate::tools::file_ops::workspace_root();

    // Detect the actual project entrypoints generically rather than
    // hardcoding Hematite's own file layout. Priority order: first match wins
    // for the "primary" slot, then the project manifest, then source root.
    let entrypoint_candidates = [
        "src/main.rs",
        "src/lib.rs",
        "src/index.ts",
        "src/index.js",
        "src/main.ts",
        "src/main.js",
        "src/main.py",
        "main.py",
        "main.go",
        "index.js",
        "index.ts",
        "app.py",
        "app.rs",
    ];
    let manifest_candidates = [
        "Cargo.toml",
        "package.json",
        "go.mod",
        "pyproject.toml",
        "setup.py",
        "composer.json",
        "pom.xml",
        "build.gradle",
    ];

    let mut files = Vec::new();

    // Primary entrypoint
    for path in &entrypoint_candidates {
        let joined = root.join(path);
        if joined.exists() {
            let size = std::fs::metadata(&joined).map(|m| m.len()).unwrap_or(0);
            files.push(ContextFile {
                path: path.to_string(),
                size,
                status: "Active".to_string(),
            });
            break;
        }
    }

    // Project manifest
    for path in &manifest_candidates {
        let joined = root.join(path);
        if joined.exists() {
            let size = std::fs::metadata(&joined).map(|m| m.len()).unwrap_or(0);
            files.push(ContextFile {
                path: path.to_string(),
                size,
                status: "Active".to_string(),
            });
            break;
        }
    }

    // Source root watcher
    let src = root.join("src");
    if src.exists() {
        let size = std::fs::metadata(&src).map(|m| m.len()).unwrap_or(0);
        files.push(ContextFile {
            path: "./src".to_string(),
            size,
            status: "Watching".to_string(),
        });
    }

    files
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SidebarMode {
    Hidden,
    Compact,
    Full,
}

fn sidebar_has_live_activity(app: &App) -> bool {
    app.agent_running
        || app.thinking
        || !app.active_workers.is_empty()
        || app.active_review.is_some()
        || app.awaiting_approval.is_some()
}

fn select_sidebar_mode(width: u16, brief_mode: bool, live_activity: bool) -> SidebarMode {
    if brief_mode || width < 100 {
        SidebarMode::Hidden
    } else if live_activity && width >= 145 {
        SidebarMode::Full
    } else {
        SidebarMode::Compact
    }
}

fn sidebar_mode(app: &App, width: u16) -> SidebarMode {
    select_sidebar_mode(width, app.brief_mode, sidebar_has_live_activity(app))
}

fn build_compact_sidebar_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let issue = runtime_issue_label(runtime_issue_kind(app));
    let provider = if app.provider_name.trim().is_empty() {
        "detecting".to_string()
    } else {
        app.provider_name.clone()
    };
    let model = if app.model_id.trim().is_empty() {
        "detecting...".to_string()
    } else {
        app.model_id.clone()
    };

    lines.push(Line::from(vec![
        Span::styled(" Runtime ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{} / {}", provider, issue),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" Model   ", Style::default().fg(Color::Gray)),
        Span::styled(model, Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" Flow    ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{} | CTX {}", app.workflow_mode, app.context_length),
            Style::default().fg(Color::White),
        ),
    ]));

    let context_source = if app.active_context.is_empty() {
        default_active_context()
    } else {
        app.active_context.clone()
    };
    if !context_source.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Files",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::DIM),
        )));
        for file in context_source.iter().take(3) {
            lines.push(Line::from(vec![
                Span::styled("· ", Style::default().fg(Color::DarkGray)),
                Span::styled(file.path.clone(), Style::default().fg(Color::White)),
            ]));
        }
    }

    let mut recent_events: Vec<String> = Vec::new();
    if sidebar_has_live_activity(app) {
        let label = if app.thinking { "Reasoning" } else { "Working" };
        let dots = ".".repeat((app.tick_count % 4) as usize + 1);
        recent_events.push(format!("{label}{dots}"));
    }
    recent_events.extend(app.specular_logs.iter().rev().take(4).cloned());
    if !recent_events.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Signals",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::DIM),
        )));
        for event in recent_events.into_iter().take(4) {
            lines.push(Line::from(vec![
                Span::styled("· ", Style::default().fg(Color::DarkGray)),
                Span::styled(event, Style::default().fg(Color::Gray)),
            ]));
        }
    }

    lines
}

fn sidebar_signal_rows(app: &App) -> Vec<(String, Color)> {
    let mut rows = Vec::new();
    if !app.last_operator_checkpoint_summary.trim().is_empty() {
        rows.push((
            format!(
                "STATE: {}",
                first_n_chars(&app.last_operator_checkpoint_summary, 96)
            ),
            Color::Yellow,
        ));
    }
    if !app.last_recovery_recipe_summary.trim().is_empty() {
        rows.push((
            format!(
                "RECOVERY: {}",
                first_n_chars(&app.last_recovery_recipe_summary, 96)
            ),
            Color::Cyan,
        ));
    }
    if !app.last_provider_summary.trim().is_empty() {
        rows.push((
            format!(
                "PROVIDER: {}",
                first_n_chars(&app.last_provider_summary, 96)
            ),
            Color::Gray,
        ));
    }
    if !app.last_mcp_summary.trim().is_empty() {
        rows.push((
            format!("MCP: {}", first_n_chars(&app.last_mcp_summary, 96)),
            Color::Gray,
        ));
    }
    rows
}

pub struct App {
    pub messages: Vec<Line<'static>>,
    pub messages_raw: Vec<(String, String)>, // Keep raw for reference or re-formatting if needed
    pub specular_logs: Vec<String>,
    pub brief_mode: bool,
    pub tick_count: u64,
    pub stats: RustyStats,
    pub yolo_mode: bool,
    /// Blocked waiting for user approval of a risky tool call.
    pub awaiting_approval: Option<PendingApproval>,
    pub active_workers: HashMap<String, u8>,
    pub worker_labels: HashMap<String, String>,
    pub active_review: Option<ActiveReview>,
    pub input: String,
    pub input_history: Vec<String>,
    pub history_idx: Option<usize>,
    pub thinking: bool,
    pub agent_running: bool,
    pub stop_requested: bool,
    pub current_thought: String,
    pub professional: bool,
    pub last_reasoning: String,
    pub active_context: Vec<ContextFile>,
    pub manual_scroll_offset: Option<u16>,
    /// Channel to send user messages to the agent task.
    pub user_input_tx: tokio::sync::mpsc::Sender<UserTurn>,
    pub specular_scroll: u16,
    /// When true the SPECULAR panel snaps to the bottom every frame.
    /// Set false when the user manually scrolls up; reset true on new turn / Done.
    pub specular_auto_scroll: bool,
    /// Shared GPU VRAM state (polled in background).
    pub gpu_state: Arc<GpuState>,
    /// Shared Git remote state (polled in background).
    pub git_state: Arc<crate::agent::git_monitor::GitState>,
    /// Track the last time a character or paste arrived to detect "fast streams" (pasting).
    pub last_input_time: std::time::Instant,
    pub cancel_token: Arc<std::sync::atomic::AtomicBool>,
    pub total_tokens: usize,
    pub current_session_cost: f64,
    pub model_id: String,
    pub context_length: usize,
    prompt_pressure_percent: u8,
    prompt_estimated_input_tokens: usize,
    prompt_reserved_output_tokens: usize,
    prompt_estimated_total_tokens: usize,
    compaction_percent: u8,
    compaction_estimated_tokens: usize,
    compaction_threshold_tokens: usize,
    /// Tracks the highest threshold crossed for compaction warnings (70, 90).
    /// Prevents re-firing the same warning every update tick.
    compaction_warned_level: u8,
    last_runtime_profile_time: Instant,
    vein_file_count: usize,
    vein_embedded_count: usize,
    vein_docs_only: bool,
    provider_name: String,
    provider_endpoint: String,
    embed_model_id: Option<String>,
    provider_state: ProviderRuntimeState,
    last_provider_summary: String,
    mcp_state: McpRuntimeState,
    last_mcp_summary: String,
    last_operator_checkpoint_state: OperatorCheckpointState,
    last_operator_checkpoint_summary: String,
    last_recovery_recipe_summary: String,
    /// Mirrors ConversationManager::think_mode for status bar display.
    /// None = auto, Some(true) = /think, Some(false) = /no_think.
    pub think_mode: Option<bool>,
    /// Sticky user-facing workflow mode.
    pub workflow_mode: String,
    /// [Autocomplete Hatch] List of matching project files.
    pub autocomplete_suggestions: Vec<String>,
    /// [Autocomplete Hatch] Index of the currently highlighted suggestion.
    pub selected_suggestion: usize,
    /// [Autocomplete Hatch] Whether the suggestions popup is visible.
    pub show_autocomplete: bool,
    /// [Autocomplete Hatch] The search fragment after the '@' symbol.
    pub autocomplete_filter: String,
    /// [Strategist] The currently active task from TASK.md.
    pub current_objective: String,
    /// [Voice of Hematite] Local TTS manager.
    pub voice_manager: Arc<crate::ui::voice::VoiceManager>,
    pub voice_loading: bool,
    pub voice_loading_progress: f64,
    /// [Autocomplete Hatch] True if the current scan is rooted in a sovereign folder.
    pub autocomplete_alias_active: bool,
    /// If false, the VRAM watchdog is silenced.
    pub hardware_guard_enabled: bool,
    /// Wall-clock time when this session started (for report timestamp).
    pub session_start: std::time::SystemTime,
    /// The current Rusty companion's species name — shown in the footer.
    pub soul_name: String,
    /// File attached via /attach — injected as context prefix on the next turn, then cleared.
    pub attached_context: Option<(String, String)>,
    pub attached_image: Option<AttachedImage>,
    hovered_input_action: Option<InputAction>,
    pub teleported_from: Option<String>,
    /// Numbered directory list from the last /ls call — used by /ls <N> to teleport.
    pub nav_list: Vec<std::path::PathBuf>,
    /// When true, all ApprovalRequired events are auto-approved for the rest of the session.
    /// Activated by pressing [A] ("Accept All") on any approval dialog.
    pub auto_approve_session: bool,
    /// Track when the current agentic task started for elapsed time rendering.
    pub task_start_time: Option<std::time::Instant>,
    /// Track live tool start times so timeline cards can show honest elapsed chips.
    pub tool_started_at: HashMap<String, std::time::Instant>,
    /// Successful grounded research/docs outputs from the current turn, used for
    /// bounded fallback recovery when the model returns empty content.
    pub recent_grounded_results: Vec<(String, String)>,
}

impl App {
    pub fn reset_active_context(&mut self) {
        self.active_context = default_active_context();
    }

    pub fn record_error(&mut self) {
        self.stats.debugging = self.stats.debugging.saturating_add(1);
    }

    pub fn reset_error_count(&mut self) {
        self.stats.debugging = 0;
    }

    pub fn reset_runtime_status_memory(&mut self) {
        self.last_provider_summary.clear();
        self.last_mcp_summary.clear();
        self.last_operator_checkpoint_summary.clear();
        self.last_operator_checkpoint_state = OperatorCheckpointState::Idle;
        self.last_recovery_recipe_summary.clear();
        self.embed_model_id = None;
    }

    pub fn clear_pending_attachments(&mut self) {
        self.attached_context = None;
        self.attached_image = None;
    }

    pub fn clear_grounded_recovery_cache(&mut self) {
        self.recent_grounded_results.clear();
    }

    pub fn push_message(&mut self, speaker: &str, content: &str) {
        let filtered = filter_tui_noise(content);
        if filtered.is_empty() && !content.is_empty() {
            return;
        } // Completely suppressed noise

        self.messages_raw.push((speaker.to_string(), filtered));
        // Cap raw history to prevent UI lag.
        if self.messages_raw.len() > 500 {
            self.messages_raw.remove(0);
        }
        self.rebuild_formatted_messages();
        // Cap visual history.
        if self.messages.len() > 8192 {
            let to_drain = self.messages.len() - 8192;
            self.messages.drain(0..to_drain);
        }
    }

    pub fn update_last_message(&mut self, token: &str) {
        if let Some(last_raw) = self.messages_raw.last_mut() {
            if last_raw.0 == "Hematite" {
                last_raw.1.push_str(token);
                // Explicitly treat the last assistant message as "dirty" and repaint
                // so the TUI can reliably snap to the newest Hematite message.
                self.rebuild_formatted_messages();
            }
        }
    }

    fn sync_task_start_time(&mut self) {
        self.task_start_time =
            synced_task_start_time(self.agent_running || self.thinking, self.task_start_time);
    }

    fn rebuild_formatted_messages(&mut self) {
        self.messages.clear();
        let total = self.messages_raw.len();
        for (i, (speaker, content)) in self.messages_raw.iter().enumerate() {
            let is_last = i == total - 1;
            let formatted = self.format_message(speaker, content, is_last);
            self.messages.extend(formatted);
            // Add a single blank line between messages for breathing room.
            // Never add this to the very last message so it remains flush with the bottom.
            if !is_last {
                self.messages.push(Line::raw(""));
            }
        }
    }

    fn header_spans(&self, speaker: &str, is_last: bool) -> Vec<Span<'static>> {
        let graphite = Color::Rgb(95, 95, 95);
        let steel = Color::Rgb(110, 110, 110);
        let ice = Color::Rgb(145, 205, 255);
        let slate = Color::Rgb(42, 42, 42);
        let pulse_on = self.tick_count % 2 == 0;

        match speaker {
            "You" => vec![
                Span::styled(" [", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "YOU",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("] ", Style::default().fg(Color::DarkGray)),
            ],
            "Hematite" => {
                let live_label = if is_last && (self.agent_running || self.thinking) {
                    if pulse_on {
                        "LIVE"
                    } else {
                        "FLOW"
                    }
                } else {
                    "HEMATITE"
                };
                vec![
                    Span::styled(" [", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        live_label,
                        Style::default()
                            .fg(if is_last { ice } else { steel })
                            .bg(slate)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("] ", Style::default().fg(Color::DarkGray)),
                ]
            }
            "System" => vec![
                Span::styled(" [", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "SYSTEM",
                    Style::default()
                        .fg(graphite)
                        .bg(Color::Rgb(28, 28, 28))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("] ", Style::default().fg(Color::DarkGray)),
            ],
            "Tool" => vec![
                Span::styled(" [", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "TOOLS",
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(Color::Rgb(28, 34, 38))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("] ", Style::default().fg(Color::DarkGray)),
            ],
            _ => vec![Span::styled(
                format!("[{}] ", speaker),
                Style::default().fg(graphite).add_modifier(Modifier::BOLD),
            )],
        }
    }

    fn tool_timeline_header(&self, label: &str, color: Color) -> Line<'static> {
        Line::from(vec![
            Span::styled("  o", Style::default().fg(Color::DarkGray)),
            Span::styled("----", Style::default().fg(Color::Rgb(52, 52, 52))),
            Span::styled(
                format!(" {} ", label),
                Style::default()
                    .fg(color)
                    .bg(Color::Rgb(28, 28, 28))
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    }

    fn tool_timeline_header_with_meta(
        &self,
        label: &str,
        color: Color,
        elapsed: Option<&str>,
    ) -> Line<'static> {
        let mut spans = vec![
            Span::styled("  o", Style::default().fg(Color::DarkGray)),
            Span::styled("----", Style::default().fg(Color::Rgb(52, 52, 52))),
            Span::styled(
                format!(" {} ", label),
                Style::default()
                    .fg(color)
                    .bg(Color::Rgb(28, 28, 28))
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        if let Some(elapsed) = elapsed.filter(|elapsed| !elapsed.trim().is_empty()) {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!(" {} ", elapsed),
                Style::default()
                    .fg(Color::Rgb(210, 210, 210))
                    .bg(Color::Rgb(36, 36, 36))
                    .add_modifier(Modifier::BOLD),
            ));
        }
        Line::from(spans)
    }

    fn format_message(&self, speaker: &str, content: &str, is_last: bool) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let cleaned_str = crate::agent::inference::strip_think_blocks(content);
        let trimmed = cleaned_str.trim();
        let cleaned = String::from(strip_ghost_prefix(trimmed));

        let mut is_first = true;
        let mut in_code_block = false;

        for raw_line in cleaned.lines() {
            let owned_line = String::from(raw_line);
            if !is_first && raw_line.trim().is_empty() {
                lines.push(Line::raw(""));
                continue;
            }

            if raw_line.trim_start().starts_with("```") {
                in_code_block = !in_code_block;
                let lang = raw_line
                    .trim_start()
                    .strip_prefix("```")
                    .unwrap_or("")
                    .trim();

                let (border, label) = if in_code_block {
                    (
                        " ┌── ",
                        format!(" {} ", if lang.is_empty() { "code" } else { lang }),
                    )
                } else {
                    (" └──", String::new())
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        border,
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::DIM),
                    ),
                    Span::styled(
                        label,
                        Style::default()
                            .fg(Color::Cyan)
                            .bg(Color::Rgb(40, 40, 40))
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                is_first = false;
                continue;
            }

            if in_code_block {
                lines.push(Line::from(vec![
                    Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(owned_line, Style::default().fg(Color::Rgb(200, 200, 160))),
                ]));
                is_first = false;
                continue;
            }

            if speaker == "System" && (raw_line.contains(" +") || raw_line.contains(" -")) {
                let mut spans: Vec<Span<'static>> = if is_first {
                    self.header_spans(speaker, is_last)
                } else {
                    vec![Span::raw("   ")]
                };
                for token in raw_line.split_whitespace() {
                    let is_add = token.starts_with('+')
                        && token.len() > 1
                        && token[1..].chars().all(|c| c.is_ascii_digit());
                    let is_rem = token.starts_with('-')
                        && token.len() > 1
                        && token[1..].chars().all(|c| c.is_ascii_digit());
                    let is_path =
                        (token.contains('/') || token.contains('\\') || token.contains('.'))
                            && !token.starts_with('+')
                            && !token.starts_with('-')
                            && !token.ends_with(':');
                    let span = if is_add {
                        Span::styled(
                            format!("{} ", token),
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else if is_rem {
                        Span::styled(
                            format!("{} ", token),
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        )
                    } else if is_path {
                        Span::styled(
                            format!("{} ", token),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::raw(format!("{} ", token))
                    };
                    spans.push(span);
                }
                lines.push(Line::from(spans));
                is_first = false;
                continue;
            }

            if speaker == "Tool"
                && (raw_line.starts_with("-")
                    || raw_line.starts_with("+")
                    || raw_line.starts_with("@@"))
            {
                let (line_style, gutter_style, sign) = if raw_line.starts_with("-") {
                    (
                        Style::default()
                            .fg(Color::Rgb(255, 200, 200))
                            .bg(Color::Rgb(60, 20, 20)),
                        Style::default().fg(Color::Red).bg(Color::Rgb(40, 15, 15)),
                        "-",
                    )
                } else if raw_line.starts_with("+") {
                    (
                        Style::default()
                            .fg(Color::Rgb(200, 255, 200))
                            .bg(Color::Rgb(20, 50, 30)),
                        Style::default().fg(Color::Green).bg(Color::Rgb(15, 30, 20)),
                        "+",
                    )
                } else {
                    (
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
                        Style::default().fg(Color::DarkGray),
                        "⋮",
                    )
                };

                let content = if raw_line.starts_with("@@") {
                    owned_line
                } else {
                    String::from(&raw_line[1..])
                };

                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", sign), gutter_style),
                    Span::styled(content, line_style),
                ]));
                is_first = false;
                continue;
            }
            if speaker == "Tool" {
                let border_style = Style::default().fg(Color::Rgb(60, 60, 60));

                if raw_line.starts_with("( )") {
                    lines.push(self.tool_timeline_header("REQUEST", Color::Cyan));
                    lines.push(Line::from(vec![
                        Span::styled("  | ", border_style),
                        Span::styled(
                            String::from(&raw_line[4..]),
                            Style::default().fg(Color::Rgb(155, 220, 255)),
                        ),
                    ]));
                } else if raw_line.starts_with("[v]") || raw_line.starts_with("[x]") {
                    let is_success = raw_line.starts_with("[v]");
                    let (status, color) = if is_success {
                        ("SUCCESS", Color::Green)
                    } else {
                        ("FAILED", Color::Red)
                    };

                    let payload = raw_line[4..].trim();
                    let (summary, preview) = if let Some((left, right)) = payload.split_once(" → ")
                    {
                        (left.trim(), Some(right))
                    } else {
                        (payload, None)
                    };
                    let (summary, elapsed) = extract_tool_elapsed_chip(summary);

                    lines.push(self.tool_timeline_header_with_meta(
                        status,
                        color,
                        elapsed.as_deref(),
                    ));
                    let mut detail_spans = vec![
                        Span::styled("  | ", border_style),
                        Span::styled(
                            summary,
                            Style::default().fg(if is_success {
                                Color::Rgb(145, 215, 145)
                            } else {
                                Color::Rgb(255, 175, 175)
                            }),
                        ),
                    ];
                    if let Some(preview) = preview {
                        detail_spans
                            .push(Span::styled(" → ", Style::default().fg(Color::DarkGray)));
                        detail_spans.push(Span::styled(
                            preview.to_string(),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                    lines.push(Line::from(detail_spans));
                } else if raw_line.starts_with("┌──") {
                    lines.push(Line::from(vec![
                        Span::styled(" ┌──", border_style),
                        Span::styled(
                            String::from(&raw_line[3..]),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                } else if raw_line.starts_with("└─") {
                    let status_color = if raw_line.contains("SUCCESS") {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    lines.push(Line::from(vec![
                        Span::styled(" └─", border_style),
                        Span::styled(
                            String::from(&raw_line[3..]),
                            Style::default()
                                .fg(status_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                } else if raw_line.starts_with("│") {
                    lines.push(Line::from(vec![
                        Span::styled(" │", border_style),
                        Span::styled(
                            String::from(&raw_line[1..]),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(" │ ", border_style),
                        Span::styled(owned_line, Style::default().fg(Color::DarkGray)),
                    ]));
                }
                is_first = false;
                continue;
            }

            let mut spans = if is_first {
                self.header_spans(speaker, is_last)
            } else {
                vec![Span::raw("   ")]
            };

            if speaker == "Hematite" {
                if is_first {
                    spans.push(Span::styled(" ", Style::default().fg(Color::DarkGray)));
                }
                spans.extend(inline_markdown_core(raw_line));
            } else {
                spans.push(Span::raw(owned_line));
            }
            lines.push(Line::from(spans));
            is_first = false;
        }

        lines
    }

    /// [Intelli-Hematite] Live scan of the workspace to populate autocomplete.
    /// Excludes common noisy directories like target, node_modules, .git.
    pub fn update_autocomplete(&mut self) {
        self.autocomplete_alias_active = false;
        let (scan_root, query) = if let Some(pos) = self.input.rfind('@') {
            let fragment = &self.input[pos + 1..];
            let upper = fragment.to_uppercase();

            // ── Path Alias Scan ──────────────────────────────────────────────
            // If the fragment starts with a known shortcut, jump the scan root.
            let mut resolved_root = crate::tools::file_ops::workspace_root();
            let mut final_query = fragment;

            let tokens = [
                "DESKTOP",
                "DOWNLOADS",
                "DOCUMENTS",
                "PICTURES",
                "VIDEOS",
                "MUSIC",
                "HOME",
            ];
            for token in tokens {
                if upper.starts_with(token) {
                    let candidate =
                        crate::tools::file_ops::resolve_candidate(&format!("@{}", token));
                    if candidate.exists() {
                        resolved_root = candidate;
                        self.autocomplete_alias_active = true;
                        // Strip the token from the query so we match files inside the target
                        if let Some(slash_pos) = fragment.find('/') {
                            final_query = &fragment[slash_pos + 1..];
                        } else {
                            final_query = ""; // Just browsing the token root
                        }
                        break;
                    }
                }
            }
            (resolved_root, final_query.to_lowercase())
        } else {
            (crate::tools::file_ops::workspace_root(), "".to_string())
        };

        self.autocomplete_filter = query.clone();
        let mut matches = Vec::new();
        let mut total_found = 0;

        // ── Noise Suppression List ───────────────────────────────────────────
        let noise = [
            "node_modules",
            "target",
            ".git",
            ".next",
            ".venv",
            "venv",
            "env",
            "bin",
            "obj",
            "dist",
            "vendor",
            "__pycache__",
            "AppData",
            "Local",
            "Roaming",
            "Application Data",
        ];

        for entry in WalkDir::new(&scan_root)
            .max_depth(4) // Prevent deep system dives
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && !noise.iter().any(|&n| name.eq_ignore_ascii_case(n))
            })
            .flatten()
        {
            let is_file = entry.file_type().is_file();
            let is_dir = entry.file_type().is_dir();

            if (is_file || is_dir) && entry.path() != scan_root {
                let path = entry
                    .path()
                    .strip_prefix(&scan_root)
                    .unwrap_or(entry.path());
                let mut path_str = path.to_string_lossy().to_string();

                if is_dir {
                    path_str.push('/');
                }

                if path_str.to_lowercase().contains(&query) || query.is_empty() {
                    total_found += 1;
                    if matches.len() < 15 {
                        matches.push(path_str);
                    }
                }
            }
            if total_found > 60 {
                break;
            } // Tighter safety cap
        }

        // Prioritize: Directories and source files (.rs, .md) at the top
        matches.sort_by(|a, b| {
            let a_is_dir = a.ends_with('/');
            let b_is_dir = b.ends_with('/');

            let a_ext = a.split('.').last().unwrap_or("");
            let b_ext = b.split('.').last().unwrap_or("");
            let a_is_src = a_ext == "rs" || a_ext == "md";
            let b_is_src = b_ext == "rs" || b_ext == "md";

            let a_score = if a_is_dir {
                2
            } else if a_is_src {
                1
            } else {
                0
            };
            let b_score = if b_is_dir {
                2
            } else if b_is_src {
                1
            } else {
                0
            };

            b_score.cmp(&a_score)
        });

        self.autocomplete_suggestions = matches;
        self.selected_suggestion = self
            .selected_suggestion
            .min(self.autocomplete_suggestions.len().saturating_sub(1));
    }

    /// [Intelli-Hematite] Applies an autocomplete selection back to the input bar.
    /// Implements Smart Splicing to handle path aliases (@DESKTOP/) vs global scans.
    pub fn apply_autocomplete_selection(&mut self, selection: &str) {
        if let Some(pos) = self.input.rfind('@') {
            if self.autocomplete_alias_active {
                // Splicing for @ALIAS/path
                // Truncate to the last slash AFTER the @ if it exists
                let after_at = &self.input[pos + 1..];
                if let Some(slash_pos) = after_at.rfind('/') {
                    self.input.truncate(pos + 1 + slash_pos + 1);
                } else {
                    // No slash yet, truncate to @ + 1
                    self.input.truncate(pos + 1);
                }
            } else {
                // Splicing for global scan: replace the @ entirely
                self.input.truncate(pos);
            }
            self.input.push_str(selection);
            self.show_autocomplete = false;
        }
    }

    /// [Intelli-Hematite] Update the context strategy deck with real file data.
    pub fn push_context_file(&mut self, path: String, status: String) {
        self.active_context.retain(|f| f.path != path);

        let root = crate::tools::file_ops::workspace_root();
        let full_path = root.join(&path);
        let size = std::fs::metadata(full_path).map(|m| m.len()).unwrap_or(0);

        self.active_context.push(ContextFile { path, size, status });

        if self.active_context.len() > 10 {
            self.active_context.remove(0);
        }
    }

    /// [Task Analyzer] Parse TASK.md to find the current active goal.
    pub fn update_objective(&mut self) {
        let hdir = crate::tools::file_ops::hematite_dir();
        let plan_path = hdir.join("PLAN.md");
        if plan_path.exists() {
            if let Some(plan) = crate::tools::plan::load_plan_handoff() {
                if plan.has_signal() && !plan.goal.trim().is_empty() {
                    self.current_objective = plan.summary_line();
                    return;
                }
            }
        }
        let path = hdir.join("TASK.md");
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let trimmed = line.trim();
                // Match "- [ ]" or "- [/]"
                if (trimmed.starts_with("- [ ]") || trimmed.starts_with("- [/]"))
                    && trimmed.len() > 6
                {
                    self.current_objective = trimmed[6..].trim().to_string();
                    return;
                }
            }
        }
        self.current_objective = "Idle".into();
    }

    /// [Auto-Diagnostic] Copy full session transcript to clipboard.
    pub fn copy_specular_to_clipboard(&self) {
        let mut out = String::from("=== SPECULAR LOG ===\n\n");

        if !self.last_reasoning.is_empty() {
            out.push_str("--- Last Reasoning Block ---\n");
            out.push_str(&self.last_reasoning);
            out.push_str("\n\n");
        }

        if !self.current_thought.is_empty() {
            out.push_str("--- In-Progress Reasoning ---\n");
            out.push_str(&self.current_thought);
            out.push_str("\n\n");
        }

        if !self.specular_logs.is_empty() {
            out.push_str("--- Specular Events ---\n");
            for entry in &self.specular_logs {
                out.push_str(entry);
                out.push('\n');
            }
            out.push('\n');
        }

        out.push_str(&format!(
            "Tokens: {} | Cost: ${:.4}\n",
            self.total_tokens, self.current_session_cost
        ));

        let mut child = std::process::Command::new("clip.exe")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn clip.exe");
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(out.as_bytes());
        }
        let _ = child.wait();
    }

    pub fn write_session_report(&self) {
        let report_dir = crate::tools::file_ops::hematite_dir().join("reports");
        if std::fs::create_dir_all(&report_dir).is_err() {
            return;
        }

        // Timestamp from session_start
        let start_secs = self
            .session_start
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Simple epoch → YYYY-MM-DD_HH-MM-SS (UTC)
        let secs_in_day = start_secs % 86400;
        let days = start_secs / 86400;
        let years_approx = (days * 4 + 2) / 1461;
        let year = 1970 + years_approx;
        let day_of_year = days - (years_approx * 365 + years_approx / 4);
        let month = (day_of_year / 30 + 1).min(12);
        let day = (day_of_year % 30 + 1).min(31);
        let hh = secs_in_day / 3600;
        let mm = (secs_in_day % 3600) / 60;
        let ss = secs_in_day % 60;
        let timestamp = format!(
            "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
            year, month, day, hh, mm, ss
        );

        let duration_secs = std::time::SystemTime::now()
            .duration_since(self.session_start)
            .unwrap_or_default()
            .as_secs();

        let report_path = report_dir.join(format!("session_{}.json", timestamp));

        let turns: Vec<serde_json::Value> = self
            .messages_raw
            .iter()
            .map(|(speaker, text)| serde_json::json!({ "speaker": speaker, "text": text }))
            .collect();

        let report = serde_json::json!({
            "session_start": timestamp,
            "duration_secs": duration_secs,
            "model": self.model_id,
            "context_length": self.context_length,
            "total_tokens": self.total_tokens,
            "estimated_cost_usd": self.current_session_cost,
            "turn_count": turns.len(),
            "transcript": turns,
        });

        if let Ok(json) = serde_json::to_string_pretty(&report) {
            let _ = std::fs::write(&report_path, json);
        }
    }

    fn transcript_snapshot_for_copy(&self) -> (Vec<(String, String)>, bool) {
        if !self.agent_running {
            return (self.messages_raw.clone(), false);
        }

        if let Some(last_user_idx) = self
            .messages_raw
            .iter()
            .rposition(|(speaker, _)| speaker == "You")
        {
            (
                self.messages_raw[..=last_user_idx].to_vec(),
                last_user_idx + 1 < self.messages_raw.len(),
            )
        } else {
            (Vec::new(), !self.messages_raw.is_empty())
        }
    }

    pub fn copy_transcript_to_clipboard(&self) {
        let (snapshot, omitted_inflight) = self.transcript_snapshot_for_copy();
        let mut history = snapshot
            .iter()
            .filter(|(speaker, content)| !should_skip_transcript_copy_entry(speaker, content))
            .map(|m| format!("[{}] {}\n", m.0, m.1))
            .collect::<String>();

        if omitted_inflight {
            history.push_str(
                "[System] Current turn is still in progress; in-flight Hematite output was omitted from this clipboard snapshot.\n",
            );
        }

        history.push_str("\nSession Stats\n");
        history.push_str(&format!("Tokens: {}\n", self.total_tokens));
        history.push_str(&format!("Cost: ${:.4}\n", self.current_session_cost));

        copy_text_to_clipboard(&history);
    }

    pub fn copy_clean_transcript_to_clipboard(&self) {
        let (snapshot, omitted_inflight) = self.transcript_snapshot_for_copy();
        let mut history = snapshot
            .iter()
            .filter(|(speaker, content)| !should_skip_transcript_copy_entry(speaker, content))
            .map(|m| format!("[{}] {}\n", m.0, m.1))
            .collect::<String>();

        if omitted_inflight {
            history.push_str(
                "[System] Current turn is still in progress; in-flight Hematite output was omitted from this clipboard snapshot.\n",
            );
        }

        history.push_str("\nSession Stats\n");
        history.push_str(&format!("Tokens: {}\n", self.total_tokens));
        history.push_str(&format!("Cost: ${:.4}\n", self.current_session_cost));

        copy_text_to_clipboard(&history);
    }

    pub fn copy_last_reply_to_clipboard(&self) -> bool {
        if let Some((speaker, content)) = self
            .messages_raw
            .iter()
            .rev()
            .find(|(speaker, content)| is_copyable_hematite_reply(speaker, content))
        {
            let cleaned = cleaned_copyable_reply_text(content);
            let payload = format!("[{}] {}", speaker, cleaned);
            copy_text_to_clipboard(&payload);
            true
        } else {
            false
        }
    }
}

fn should_accept_autocomplete_on_enter(alias_active: bool, filter: &str) -> bool {
    if alias_active && filter.trim().is_empty() {
        return false;
    }
    true
}

fn copy_text_to_clipboard(text: &str) {
    if copy_text_to_clipboard_powershell(text) {
        return;
    }

    // Fallback: Windows clip.exe is fast and dependency-free, but some
    // terminal/clipboard paths can mangle non-ASCII punctuation.
    let mut child = std::process::Command::new("clip.exe")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn clip.exe");

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(text.as_bytes());
    }
    let _ = child.wait();
}

fn synced_task_start_time(
    active: bool,
    current: Option<std::time::Instant>,
) -> Option<std::time::Instant> {
    match (active, current) {
        (true, None) => Some(std::time::Instant::now()),
        (false, Some(_)) => None,
        (_, existing) => existing,
    }
}

fn scroll_specular_up(app: &mut App, amount: u16) {
    app.specular_auto_scroll = false;
    app.specular_scroll = app.specular_scroll.saturating_sub(amount);
}

fn scroll_specular_down(app: &mut App, amount: u16) {
    app.specular_auto_scroll = false;
    app.specular_scroll = app.specular_scroll.saturating_add(amount);
}

fn follow_live_specular(app: &mut App) {
    app.specular_auto_scroll = true;
    app.specular_scroll = 0;
}

fn format_tool_elapsed(elapsed: std::time::Duration) -> String {
    if elapsed.as_millis() < 1_000 {
        format!("{}ms", elapsed.as_millis())
    } else {
        format!("{:.1}s", elapsed.as_secs_f64())
    }
}

fn extract_tool_elapsed_chip(summary: &str) -> (String, Option<String>) {
    let trimmed = summary.trim();
    if let Some((head, tail)) = trimmed.rsplit_once(" [") {
        if let Some(elapsed) = tail.strip_suffix(']') {
            if !elapsed.is_empty()
                && elapsed
                    .chars()
                    .all(|ch| ch.is_ascii_digit() || ch == '.' || ch == 'm' || ch == 's')
            {
                return (head.trim().to_string(), Some(elapsed.to_string()));
            }
        }
    }
    (trimmed.to_string(), None)
}

fn should_capture_grounded_tool_output(name: &str, is_error: bool) -> bool {
    !is_error && matches!(name, "research_web" | "fetch_docs")
}

fn looks_like_markup_payload(result: &str) -> bool {
    let lower = result
        .chars()
        .take(256)
        .collect::<String>()
        .to_ascii_lowercase();
    lower.contains("<!doctype")
        || lower.contains("<html")
        || lower.contains("<body")
        || lower.contains("<meta ")
}

fn build_runtime_fix_grounded_fallback(results: &[(String, String)]) -> Option<String> {
    if results.is_empty() {
        return None;
    }

    let mut sections = Vec::new();

    for (name, result) in results.iter().filter(|(name, _)| name == "research_web") {
        sections.push(format!(
            "[{}]\n{}",
            name,
            first_n_chars(result, 1800).trim()
        ));
    }

    if sections.is_empty() {
        for (name, result) in results
            .iter()
            .filter(|(name, result)| name == "fetch_docs" && !looks_like_markup_payload(result))
        {
            sections.push(format!(
                "[{}]\n{}",
                name,
                first_n_chars(result, 1600).trim()
            ));
        }
    }

    if sections.is_empty() {
        if let Some((name, result)) = results.last() {
            sections.push(format!(
                "[{}]\n{}",
                name,
                first_n_chars(result, 1200).trim()
            ));
        }
    }

    if sections.is_empty() {
        None
    } else {
        Some(format!(
            "The model returned empty content after grounded tool work. Hematite is surfacing the latest verified tool output directly.\n\n{}",
            sections.join("\n\n")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_runtime_fix_grounded_fallback, classify_runtime_issue, extract_tool_elapsed_chip,
        format_tool_elapsed, make_animated_sparkline_gauge, provider_badge_prefix,
        select_fitting_variant, select_sidebar_mode, should_accept_autocomplete_on_enter,
        synced_task_start_time, RuntimeIssueKind, SidebarMode,
    };
    use crate::agent::inference::ProviderRuntimeState;

    #[test]
    fn tool_elapsed_chip_extracts_cleanly_from_summary() {
        assert_eq!(
            extract_tool_elapsed_chip("research_web [842ms]"),
            ("research_web".to_string(), Some("842ms".to_string()))
        );
        assert_eq!(
            extract_tool_elapsed_chip("read_file"),
            ("read_file".to_string(), None)
        );
    }

    #[test]
    fn tool_elapsed_formats_compact_runtime_durations() {
        assert_eq!(
            format_tool_elapsed(std::time::Duration::from_millis(842)),
            "842ms"
        );
        assert_eq!(
            format_tool_elapsed(std::time::Duration::from_millis(1520)),
            "1.5s"
        );
    }

    #[test]
    fn enter_submits_bare_alias_root_instead_of_selecting_first_child() {
        assert!(!should_accept_autocomplete_on_enter(true, ""));
        assert!(!should_accept_autocomplete_on_enter(true, "   "));
    }

    #[test]
    fn enter_still_accepts_narrowed_alias_matches() {
        assert!(should_accept_autocomplete_on_enter(true, "web"));
        assert!(should_accept_autocomplete_on_enter(false, ""));
    }

    #[test]
    fn provider_badge_prefix_tracks_runtime_provider() {
        assert_eq!(provider_badge_prefix("LM Studio"), "LM");
        assert_eq!(provider_badge_prefix("Ollama"), "OL");
        assert_eq!(provider_badge_prefix("Other"), "AI");
    }

    #[test]
    fn runtime_issue_prefers_no_model_over_live_state() {
        assert_eq!(
            classify_runtime_issue(ProviderRuntimeState::Live, "no model loaded", 32000, ""),
            RuntimeIssueKind::NoModel
        );
    }

    #[test]
    fn runtime_issue_distinguishes_context_ceiling() {
        assert_eq!(
            classify_runtime_issue(
                ProviderRuntimeState::ContextWindow,
                "qwen/qwen3.5-9b",
                32000,
                "LM context ceiling hit."
            ),
            RuntimeIssueKind::ContextCeiling
        );
    }

    #[test]
    fn runtime_issue_maps_generic_degraded_state_to_connectivity_signal() {
        assert_eq!(
            classify_runtime_issue(
                ProviderRuntimeState::Degraded,
                "qwen/qwen3.5-9b",
                32000,
                "LM Studio degraded and did not recover cleanly; operator action is now required."
            ),
            RuntimeIssueKind::Connectivity
        );
    }

    #[test]
    fn sidebar_mode_hides_in_brief_or_narrow_layouts() {
        assert_eq!(select_sidebar_mode(99, false, true), SidebarMode::Hidden);
        assert_eq!(select_sidebar_mode(160, true, true), SidebarMode::Hidden);
    }

    #[test]
    fn sidebar_mode_only_uses_full_chrome_for_live_wide_sessions() {
        assert_eq!(select_sidebar_mode(130, false, false), SidebarMode::Compact);
        assert_eq!(select_sidebar_mode(130, false, true), SidebarMode::Compact);
        assert_eq!(select_sidebar_mode(160, false, true), SidebarMode::Full);
    }

    #[test]
    fn task_timer_starts_when_activity_begins() {
        assert!(synced_task_start_time(true, None).is_some());
    }

    #[test]
    fn task_timer_clears_when_activity_ends() {
        assert!(synced_task_start_time(false, Some(std::time::Instant::now())).is_none());
    }

    #[test]
    fn fitting_variant_picks_longest_string_that_fits() {
        let variants = vec![
            "this variant is too wide".to_string(),
            "fits nicely".to_string(),
            "tiny".to_string(),
        ];
        assert_eq!(select_fitting_variant(&variants, 12), "fits nicely");
        assert_eq!(select_fitting_variant(&variants, 4), "tiny");
    }

    #[test]
    fn animated_gauge_preserves_requested_width() {
        let gauge = make_animated_sparkline_gauge(0.42, 12, 7);
        assert_eq!(gauge.chars().count(), 12);
        assert!(gauge.contains('█') || gauge.contains('▓') || gauge.contains('▒'));
    }
    #[test]
    fn runtime_fix_grounded_fallback_prefers_search_results_over_html_fetch() {
        let fallback = build_runtime_fix_grounded_fallback(&[
            (
                "fetch_docs".to_string(),
                "<!doctype html><html><body>raw page shell</body></html>".to_string(),
            ),
            (
                "research_web".to_string(),
                "Search results for: uefn toolbelt\n1. GitHub repo\n2. Epic forum thread"
                    .to_string(),
            ),
        ])
        .expect("fallback");

        assert!(fallback.contains("Search results for: uefn toolbelt"));
        assert!(!fallback.contains("<!doctype html>"));
    }

    #[test]
    fn runtime_fix_grounded_fallback_returns_none_without_grounded_results() {
        assert!(build_runtime_fix_grounded_fallback(&[]).is_none());
    }
}

/// Capture the pixel rect of the current console window via a synchronous PowerShell call.
/// Returns (x, y, width, height) in screen pixels.
#[cfg(windows)]
fn get_console_pixel_rect() -> Option<(i32, i32, i32, i32)> {
    let script = concat!(
        "Add-Type -TypeDefinition '",
        "using System;using System.Runtime.InteropServices;",
        "public class WG{",
        "[DllImport(\"kernel32\")]public static extern IntPtr GetConsoleWindow();",
        "[DllImport(\"user32\")]public static extern bool GetWindowRect(IntPtr h,out RECT r);",
        "[StructLayout(LayoutKind.Sequential)]public struct RECT{public int L,T,R,B;}}",
        "';",
        "$h=[WG]::GetConsoleWindow();$r=New-Object WG+RECT;",
        "[WG]::GetWindowRect($h,[ref]$r)|Out-Null;",
        "Write-Output \"$($r.L) $($r.T) $($r.R-$r.L) $($r.B-$r.T)\""
    );
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let parts: Vec<i32> = s
        .split_whitespace()
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    if parts.len() >= 4 {
        Some((parts[0], parts[1], parts[2], parts[3]))
    } else {
        None
    }
}

/// Find the shell/tab process that should be closed after teleporting away from
/// the current session. In Windows Terminal we want the tab shell, not the
/// terminal host process itself.
#[cfg(windows)]
fn get_console_close_target_pid_sync() -> Option<u32> {
    let pid = std::process::id();
    let script = format!(
        r#"
$current = [uint32]{pid}
$seen = New-Object 'System.Collections.Generic.HashSet[uint32]'
$shell_pattern = '^(cmd|powershell|pwsh|bash|sh|wsl|ubuntu|debian|kali|arch)$'
$skip_pattern = '^(WindowsTerminal|wt|OpenConsole|conhost)$'
$fallback = $null
$found = $false
while ($current -gt 0 -and $seen.Add($current)) {{
    $proc = Get-CimInstance Win32_Process -Filter "ProcessId=$current" -ErrorAction SilentlyContinue
    if (-not $proc) {{ break }}
    $parent = [uint32]$proc.ParentProcessId
    if ($parent -le 0) {{ break }}
    $parent_proc = Get-Process -Id $parent -ErrorAction SilentlyContinue
    if ($parent_proc) {{
        $name = $parent_proc.ProcessName
        if ($name -match $shell_pattern) {{
            $found = $true
            Write-Output $parent
            break
        }}
        if (-not $fallback -and $name -notmatch $skip_pattern) {{
            $fallback = $parent
        }}
    }}
    $current = $parent
}}
if (-not $found -and $fallback) {{ Write-Output $fallback }}
"#
    );
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

/// Spawns a new detached terminal window pre-navigated to `path`, running Hematite.
/// - Writes a temp .bat file to avoid quoting issues with paths containing spaces
/// - Matches the current window's pixel size and position
/// - Skips the splash screen in the new session (`--no-splash`)
/// - Closes the originating shell/tab after Hematite exits without killing the
///   whole Windows Terminal host
#[cfg(windows)]
fn spawn_dive_in_terminal(path: &str) {
    let pid = std::process::id();
    let current_dir = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let close_target_pid = get_console_close_target_pid_sync().unwrap_or(0);
    let (px, py, pw, ph) = get_console_pixel_rect().unwrap_or((50, 50, 1100, 750));

    let bat_path = std::env::temp_dir().join("hematite_teleport.bat");
    let bat_content = format!(
        "@echo off\r\ncd /d \"{p}\"\r\nhematite --no-splash --teleported-from \"{o}\"\r\n",
        p = path.replace('"', ""),
        o = current_dir.replace('"', ""),
    );
    if std::fs::write(&bat_path, bat_content).is_err() {
        return;
    }
    let bat_str = bat_path.to_string_lossy().to_string();
    let bat_ps = bat_str.replace('\'', "''");

    let script = format!(
        r#"
Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices;
public class WM {{ [DllImport("user32")] public static extern bool MoveWindow(IntPtr h,int x,int y,int w,int ht,bool b); }}
'@
$proc = Start-Process cmd.exe -ArgumentList @('/k', '"{bat}"') -PassThru
$deadline = (Get-Date).AddSeconds(8)
while ((Get-Date) -lt $deadline -and $proc.MainWindowHandle -eq [IntPtr]::Zero) {{ Start-Sleep -Milliseconds 100 }}
if ($proc.MainWindowHandle -ne [IntPtr]::Zero) {{
    [WM]::MoveWindow($proc.MainWindowHandle, {px}, {py}, {pw}, {ph}, $true) | Out-Null
}}
Wait-Process -Id {pid} -ErrorAction SilentlyContinue
if ({close_pid} -gt 0) {{
    Stop-Process -Id {close_pid} -Force -ErrorAction SilentlyContinue
}}
"#,
        bat = bat_ps,
        px = px,
        py = py,
        pw = pw,
        ph = ph,
        pid = pid,
        close_pid = close_target_pid,
    );

    let _ = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .spawn();
}

#[cfg(not(windows))]
fn spawn_dive_in_terminal(_path: &str) {}

fn copy_text_to_clipboard_powershell(text: &str) -> bool {
    let temp_path = std::env::temp_dir().join(format!(
        "hematite-clipboard-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default()
    ));

    if std::fs::write(&temp_path, text.as_bytes()).is_err() {
        return false;
    }

    let escaped_path = temp_path.display().to_string().replace('\'', "''");
    let script = format!(
        "$t = Get-Content -LiteralPath '{}' -Raw -Encoding UTF8; Set-Clipboard -Value $t",
        escaped_path
    );

    let status = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status();

    let _ = std::fs::remove_file(&temp_path);

    matches!(status, Ok(code) if code.success())
}

fn is_immediate_local_command(input: &str) -> bool {
    matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "/copy" | "/copy-last" | "/copy-clean" | "/copy2"
    )
}

fn should_skip_transcript_copy_entry(speaker: &str, content: &str) -> bool {
    if speaker != "System" {
        return false;
    }

    content.starts_with("Hematite Commands:\n")
        || content.starts_with("Document note: `/attach`")
        || content == "Chat transcript copied to clipboard."
        || content == "Exact session transcript copied to clipboard (includes help/system output)."
        || content == "Clean chat transcript copied to clipboard (skips help/debug boilerplate)."
        || content == "Latest Hematite reply copied to clipboard."
        || content == "SPECULAR log copied to clipboard (reasoning + events)."
        || content == "Cancellation requested. Logs copied to clipboard."
}

fn is_copyable_hematite_reply(speaker: &str, content: &str) -> bool {
    if speaker != "Hematite" {
        return false;
    }

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed == "Initialising Engine & Hardware..."
        || trimmed == "Swarm engaged."
        || trimmed.starts_with("Hematite v")
        || trimmed.starts_with("Swarm analyzing: '")
        || trimmed.ends_with("Standing by for review...")
        || trimmed.ends_with("conflict - review required.")
        || trimmed.ends_with("conflict — review required.")
    {
        return false;
    }

    true
}

fn cleaned_copyable_reply_text(content: &str) -> String {
    let cleaned = content
        .replace("<thought>", "")
        .replace("</thought>", "")
        .replace("<think>", "")
        .replace("</think>", "");
    strip_ghost_prefix(cleaned.trim()).trim().to_string()
}

// ── run_app ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputAction {
    Stop,
    PickDocument,
    PickImage,
    Detach,
    New,
    Forget,
    Help,
}

#[derive(Clone)]
struct InputActionVisual {
    action: InputAction,
    label: String,
    style: Style,
}

#[derive(Clone, Copy)]
enum AttachmentPickerKind {
    Document,
    Image,
}

fn attach_document_from_path(app: &mut App, file_path: &str) {
    let p = std::path::Path::new(file_path);
    match crate::memory::vein::extract_document_text(p) {
        Ok(text) => {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(file_path)
                .to_string();
            let preview_len = text.len().min(200);
            // Rough token estimate: ~4 chars per token.
            let estimated_tokens = text.len() / 4;
            let ctx = app.context_length.max(1);
            let budget_pct = (estimated_tokens * 100) / ctx;
            let budget_note = if budget_pct >= 75 {
                format!(
                    "\nWarning: this document is ~{} tokens (~{}% of your {}k context). \
                     Very little room left for conversation. Consider /attach on a shorter excerpt.",
                    estimated_tokens, budget_pct, ctx / 1000
                )
            } else if budget_pct >= 40 {
                format!(
                    "\nNote: this document is ~{} tokens (~{}% of your {}k context).",
                    estimated_tokens,
                    budget_pct,
                    ctx / 1000
                )
            } else {
                String::new()
            };
            app.push_message(
                "System",
                &format!(
                    "Attached document: {} ({} chars) for the next message.\nPreview: {}...{}",
                    name,
                    text.len(),
                    &text[..preview_len],
                    budget_note,
                ),
            );
            app.attached_context = Some((name, text));
        }
        Err(e) => {
            app.push_message("System", &format!("Attach failed: {}", e));
        }
    }
}

fn attach_image_from_path(app: &mut App, file_path: &str) {
    let p = std::path::Path::new(file_path);
    match crate::tools::vision::encode_image_as_data_url(p) {
        Ok(_) => {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(file_path)
                .to_string();
            app.push_message(
                "System",
                &format!("Attached image: {} for the next message.", name),
            );
            app.attached_image = Some(AttachedImage {
                name,
                path: file_path.to_string(),
            });
        }
        Err(e) => {
            app.push_message("System", &format!("Image attach failed: {}", e));
        }
    }
}

fn is_document_path(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "pdf" | "md" | "markdown" | "txt" | "rst"
    )
}

fn is_image_path(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp"
    )
}

fn extract_pasted_path_candidates(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return out;
    }

    let mut in_quotes = false;
    let mut current = String::new();
    for ch in trimmed.chars() {
        if ch == '"' {
            if in_quotes && !current.trim().is_empty() {
                out.push(current.trim().to_string());
                current.clear();
            }
            in_quotes = !in_quotes;
            continue;
        }
        if in_quotes {
            current.push(ch);
        }
    }
    if !out.is_empty() {
        return out;
    }

    for line in trimmed.lines() {
        let candidate = line.trim().trim_matches('"').trim();
        if !candidate.is_empty() {
            out.push(candidate.to_string());
        }
    }

    if out.is_empty() {
        out.push(trimmed.trim_matches('"').to_string());
    }
    out
}

fn try_attach_from_paste(app: &mut App, content: &str) -> bool {
    let mut attached_doc = false;
    let mut attached_image = false;
    let mut ignored_supported = 0usize;

    for raw in extract_pasted_path_candidates(content) {
        let path = std::path::Path::new(&raw);
        if !path.exists() {
            continue;
        }
        if is_image_path(path) {
            if attached_image || app.attached_image.is_some() {
                ignored_supported += 1;
            } else {
                attach_image_from_path(app, &raw);
                attached_image = true;
            }
        } else if is_document_path(path) {
            if attached_doc || app.attached_context.is_some() {
                ignored_supported += 1;
            } else {
                attach_document_from_path(app, &raw);
                attached_doc = true;
            }
        }
    }

    if ignored_supported > 0 {
        app.push_message(
            "System",
            &format!(
                "Ignored {} extra dropped file(s). Hematite currently keeps one pending document and one pending image.",
                ignored_supported
            ),
        );
    }

    attached_doc || attached_image
}

fn compute_input_height(total_width: u16, input_len: usize) -> u16 {
    let width = total_width.max(1) as usize;
    let approx_input_w = (width * 65 / 100).saturating_sub(4).max(1);
    let needed_lines = (input_len / approx_input_w) as u16 + 3;
    needed_lines.clamp(3, 10)
}

fn input_rect_for_size(size: Rect, input_len: usize) -> Rect {
    let input_height = compute_input_height(size.width, input_len);
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(input_height),
            Constraint::Length(5), // Synced with 2-tier ui() for surgical mouse alignment
        ])
        .split(size)[1]
}

fn input_title_area(input_rect: Rect) -> Rect {
    Rect {
        x: input_rect.x.saturating_add(1),
        y: input_rect.y,
        width: input_rect.width.saturating_sub(2),
        height: 1,
    }
}

fn build_input_actions(app: &App) -> Vec<InputActionVisual> {
    let doc_label = if app.attached_context.is_some() {
        "Files*"
    } else {
        "Files"
    };
    let image_label = if app.attached_image.is_some() {
        "Image*"
    } else {
        "Image"
    };
    let detach_style = if app.attached_context.is_some() || app.attached_image.is_some() {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut actions = Vec::new();
    if app.agent_running {
        actions.push(InputActionVisual {
            action: InputAction::Stop,
            label: "Stop Esc".to_string(),
            style: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        });
    } else {
        actions.push(InputActionVisual {
            action: InputAction::New,
            label: "New".to_string(),
            style: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        });
        actions.push(InputActionVisual {
            action: InputAction::Forget,
            label: "Forget".to_string(),
            style: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        });
    }

    actions.push(InputActionVisual {
        action: InputAction::PickDocument,
        label: format!("{} ^O", doc_label),
        style: Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    });
    actions.push(InputActionVisual {
        action: InputAction::PickImage,
        label: format!("{} ^I", image_label),
        style: Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    });
    actions.push(InputActionVisual {
        action: InputAction::Detach,
        label: "Detach".to_string(),
        style: detach_style,
    });
    actions.push(InputActionVisual {
        action: InputAction::Help,
        label: "Help".to_string(),
        style: Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    });
    actions
}

fn visible_input_actions(app: &App, max_width: u16) -> Vec<InputActionVisual> {
    let mut used = 0u16;
    let mut visible = Vec::new();
    for action in build_input_actions(app) {
        let chip_width = action.label.chars().count() as u16 + 2;
        let gap = if visible.is_empty() { 0 } else { 1 };
        if used + gap + chip_width > max_width {
            break;
        }
        used += gap + chip_width;
        visible.push(action);
    }
    visible
}

fn input_status_variants(app: &App) -> Vec<String> {
    let voice_status = if app.voice_manager.is_enabled() {
        "ON"
    } else {
        "OFF"
    };
    let approvals_status = if app.yolo_mode { "OFF" } else { "ON" };
    let issue = runtime_issue_badge(runtime_issue_kind(app)).0;
    let flow = app.workflow_mode.to_uppercase();
    let attach_status = if app.attached_context.is_some() && app.attached_image.is_some() {
        "ATTACH:DOC+IMG"
    } else if app.attached_context.is_some() {
        "ATTACH:DOC"
    } else if app.attached_image.is_some() {
        "ATTACH:IMG"
    } else {
        "ATTACH:--"
    };
    if app.agent_running {
        vec![
            format!(
                "WORKING · ESC stops · FLOW:{} · RT:{} · VOICE:{}",
                flow, issue, voice_status
            ),
            format!("WORKING · RT:{} · VOICE:{}", issue, voice_status),
            format!("RT:{} · VOICE:{}", issue, voice_status),
            format!("RT:{}", issue),
        ]
    } else if app.input.trim().is_empty() {
        vec![
            format!(
                "READY · FLOW:{} · RT:{} · VOICE:{} · APPR:{}",
                flow, issue, voice_status, approvals_status
            ),
            format!("READY · FLOW:{} · RT:{}", flow, issue),
            format!("FLOW:{} · RT:{}", flow, issue),
            format!("RT:{}", issue),
        ]
    } else {
        let draft_len = app.input.len();
        vec![
            format!(
                "DRAFT:{} · FLOW:{} · RT:{} · {}",
                draft_len, flow, issue, attach_status
            ),
            format!("DRAFT:{} · RT:{} · {}", draft_len, issue, attach_status),
            format!("LEN:{} · RT:{}", draft_len, issue),
            format!("RT:{}", issue),
        ]
    }
}

fn make_sparkline_gauge(ratio: f64, width: usize) -> String {
    let filled = (ratio * width as f64).round() as usize;
    let mut s = String::with_capacity(width);
    for i in 0..width {
        if i < filled {
            s.push('▓');
        } else {
            s.push('░');
        }
    }
    s
}

fn make_animated_sparkline_gauge(ratio: f64, width: usize, tick_count: u64) -> String {
    let filled = (ratio.clamp(0.0, 1.0) * width as f64).round() as usize;
    let shimmer_idx = if filled > 0 {
        (tick_count as usize / 2) % filled.max(1)
    } else {
        0
    };
    let mut chars: Vec<char> = make_sparkline_gauge(ratio, width).chars().collect();
    for (i, ch) in chars.iter_mut().enumerate() {
        if i < filled {
            *ch = if i == shimmer_idx { '█' } else { '▓' };
        } else if i == filled && filled < width && ratio > 0.0 {
            *ch = '▒';
        } else {
            *ch = '░';
        }
    }
    chars.into_iter().collect()
}

fn select_fitting_variant(variants: &[String], width: u16) -> String {
    let max_width = width as usize;
    for variant in variants {
        if variant.chars().count() <= max_width {
            return variant.clone();
        }
    }
    variants.last().cloned().unwrap_or_default()
}

fn idle_footer_variants(app: &App) -> Vec<String> {
    let issue = runtime_issue_badge(runtime_issue_kind(app)).0;
    if issue != "OK" {
        return vec![
            format!(" /runtime fix • /runtime explain • RT:{} ", issue),
            format!(" /runtime fix • RT:{} ", issue),
            format!(" RT:{} ", issue),
        ];
    }

    let phase = (app.tick_count / 18) % 3;
    match phase {
        0 => vec![
            " [↑/↓] scroll • /help hints • /runtime status ".to_string(),
            " [↑/↓] scroll • /help hints ".to_string(),
            " /help ".to_string(),
        ],
        1 => vec![
            " /ask analyze • /architect plan • /code implement ".to_string(),
            " /ask • /architect • /code ".to_string(),
            " /code ".to_string(),
        ],
        _ => vec![
            " /provider status • /runtime refresh • /ls desktop ".to_string(),
            " /provider • /runtime refresh ".to_string(),
            " /runtime ".to_string(),
        ],
    }
}

fn running_footer_variants(app: &App, elapsed: &str, last_log: &str) -> Vec<String> {
    let worker_count = app.active_workers.len();
    let primary_caption = if worker_count > 0 {
        format!("{} workers • {}", worker_count, last_log)
    } else {
        last_log.to_string()
    };
    vec![
        primary_caption,
        last_log.to_string(),
        format!("{} • working", elapsed.trim()),
        "working".to_string(),
    ]
}

fn select_input_title_layout(app: &App, title_width: u16) -> (Vec<InputActionVisual>, String) {
    let action_total = build_input_actions(app).len();
    let mut best_actions = visible_input_actions(app, title_width);
    let mut best_status = String::new();
    for status in input_status_variants(app) {
        let reserved = status.chars().count() as u16 + 3;
        let actions = visible_input_actions(app, title_width.saturating_sub(reserved));
        let replace = actions.len() > best_actions.len()
            || (actions.len() == best_actions.len() && status.len() > best_status.len());
        if replace {
            best_actions = actions.clone();
            best_status = status.clone();
        }
        if actions.len() == action_total {
            return (actions, status);
        }
    }
    (best_actions, best_status)
}

fn input_action_hitboxes(app: &App, title_area: Rect) -> Vec<(InputAction, u16, u16)> {
    let mut x = title_area.x;
    let mut out = Vec::new();
    let (actions, _) = select_input_title_layout(app, title_area.width);
    for action in actions {
        let chip_width = action.label.chars().count() as u16 + 2; // " " + label + " "
        out.push((action.action, x, x + chip_width.saturating_sub(1)));
        x = x.saturating_add(chip_width + 1);
    }
    out
}

fn render_input_title<'a>(app: &'a App, area: Rect) -> Line<'a> {
    let mut spans = Vec::new();
    let (actions, status) = select_input_title_layout(app, area.width);
    for action in actions {
        let is_hovered = app.hovered_input_action == Some(action.action);
        let style = if is_hovered {
            Style::default()
                .bg(action.style.fg.unwrap_or(Color::Gray))
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            action.style
        };
        spans.push(Span::styled(format!(" {} ", action.label), style));
        spans.push(Span::raw(" "));
    }

    if !status.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(status, Style::default().fg(Color::DarkGray)));
    }
    Line::from(spans)
}

fn reset_visible_session_state(app: &mut App) {
    app.messages.clear();
    app.messages_raw.clear();
    app.last_reasoning.clear();
    app.current_thought.clear();
    app.specular_logs.clear();
    app.reset_error_count();
    app.reset_runtime_status_memory();
    app.reset_active_context();
    app.tool_started_at.clear();
    app.clear_grounded_recovery_cache();
    app.clear_pending_attachments();
    app.current_objective = "Idle".into();
}

fn request_stop(app: &mut App) {
    app.voice_manager.stop();
    if app.stop_requested {
        return;
    }
    app.stop_requested = true;
    app.cancel_token
        .store(true, std::sync::atomic::Ordering::SeqCst);
    if app.thinking || app.agent_running {
        app.write_session_report();
        app.copy_transcript_to_clipboard();
        app.push_message(
            "System",
            "Cancellation requested. Logs copied to clipboard.",
        );
    }
}

fn show_help_message(app: &mut App) {
    app.push_message(
        "System",
        "Hematite Commands:\n\
         /chat             - (Mode) Conversation mode - clean chat, no tool noise\n\
         /agent            - (Mode) Full coding harness + workstation mode - tools, file edits, builds, inspection\n\
         /reroll           - (Soul) Hatch a new companion mid-session\n\
         /auto             - (Flow) Let Hematite choose the narrowest effective workflow\n\
         /rules [view|edit]- (Meta) View status or edit local/shared project guidelines\n\
         /ask [prompt]     - (Flow) Read-only analysis mode; optional inline prompt\n\
         /code [prompt]    - (Flow) Explicit implementation mode; optional inline prompt\n\
         /architect [prompt] - (Flow) Plan-first mode; optional inline prompt\n\
         /implement-plan   - (Flow) Execute the saved architect handoff in /code\n\
         /read-only [prompt] - (Flow) Hard read-only mode; optional inline prompt\n\
         /teach [prompt]   - (Flow) Teacher mode; inspect machine then walk you through any admin task step-by-step\n\
         /new              - (Reset) Fresh task context; clear chat, pins, and task files\n\
         /forget           - (Wipe) Hard forget; purge saved memory and Vein index too\n\
         /cd <path>        - (Nav) Teleport to another directory and close this session; supports bare tokens like downloads, desktop, docs, home, temp, and ~, plus aliases like @DESKTOP/project\n\
         /ls [path|N]      - (Nav) List common locations or subdirectories; use /ls desktop, then /ls <N> to teleport to a numbered entry\n\
         /vein-inspect     - (Vein) Inspect indexed memory, hot files, and active room bias\n\
         /workspace-profile - (Profile) Show the auto-generated workspace profile\n\
         /rules            - (Rules) View behavioral guidelines (.hematite/rules.md)\n\
         /version          - (Build) Show the running Hematite version\n\
         /about            - (Info) Show author, repo, and product info\n\
         /vein-reset       - (Vein) Wipe the RAG index; rebuilds automatically on next turn\n\
         /clear            - (UI) Clear dialogue display only\n\
         /health           - (Diag) Run a synthesized plain-English system health report\n\
         /explain <text>   - (Help) Paste an error to get a non-technical breakdown\n\
         /gemma-native [auto|on|off|status] - (Model) Auto/force/disable Gemma 4 native formatting\n\
         /provider [status|lmstudio|ollama|clear|URL] - (Model) Show or save the active provider endpoint preference\n\
         /runtime          - (Model) Show the live runtime/provider/model/embed status and shortest fix path\n\
         /runtime fix      - (Model) Run the shortest safe runtime recovery step now\n\
         /runtime-refresh  - (Model) Re-read active provider model + CTX now\n\
         /model [status|list [available|loaded]|load <id> [--ctx N]|unload [id|current|all]|prefer <id>|clear] - (Model) Inspect, list, load, unload, or save the preferred coding model (`--ctx` uses LM Studio context length or Ollama `num_ctx`)\n\
         /embed [status|load <id>|unload [id|current]|prefer <id>|clear] - (Model) Inspect, load, unload, or save the preferred embed model\n\
         /undo             - (Ghost) Revert last file change\n\
         /diff             - (Git) Show session changes (--stat)\n\
         /lsp              - (Logic) Start Language Servers (semantic intelligence)\n\
         /swarm <text>     - (Swarm) Spawn parallel workers on a directive\n\
         /worktree <cmd>   - (Isolated) Manage git worktrees (list|add|remove|prune)\n\
         /think            - (Brain) Enable deep reasoning mode\n\
         /no_think         - (Speed) Disable reasoning (3-5x faster responses)\n\
         /voice            - (TTS) List all available voices\n\
         /voice N          - (TTS) Select voice by number\n\
         /read <text>      - (TTS) Speak text aloud directly, bypassing the model. ESC to stop.\n\
         /explain <text>   - (Plain English) Paste any error or output; Hematite explains it in plain English.\n\
         /health           - (SysAdmin) Run a full system health report (disk, RAM, tools, recent errors).\n\
         /attach <path>    - (Docs) Attach a PDF/markdown/txt file for next message (PDF best-effort)\n\
         /attach-pick      - (Docs) Open a file picker and attach a document\n\
         /image <path>     - (Vision) Attach an image for the next message\n\
         /image-pick       - (Vision) Open a file picker and attach an image\n\
         /detach           - (Context) Drop pending document/image attachments\n\
         /copy             - (Debug) Copy exact session transcript (includes help/system output)\n\
         /copy-last        - (Debug) Copy the latest Hematite reply only\n\
         /copy-clean       - (Debug) Copy chat transcript without help/debug boilerplate\n\
         /copy2            - (Debug) Copy the full SPECULAR rail to clipboard (reasoning + events)\n\
         \nHotkeys:\n\
         Ctrl+B - Toggle Brief Mode (minimal output; collapses side chrome)\n\
         Alt+↑/↓ - Scroll the SPECULAR rail by 3 lines\n\
         Alt+PgUp/PgDn - Scroll the SPECULAR rail by 10 lines\n\
         Alt+End - Snap SPECULAR back to live follow mode\n\
         Ctrl+P - Toggle Professional Mode (strip personality)\n\
         Ctrl+O - Open document picker for next-turn context\n\
         Ctrl+I - Open image picker for next-turn vision context\n\
         Ctrl+Y - Toggle Approvals Off (bypass safety approvals)\n\
         Ctrl+S - Quick Swarm (hardcoded bootstrap)\n\
         Ctrl+Z - Undo last edit\n\
         Ctrl+Q/C - Quit session\n\
         ESC    - Silence current playback\n\
         \nStatus Legend:\n\
         LM/OL - Provider runtime health (`LIVE`, `RECV`, `WARN`, `CEIL`, `STALE`, `BOOT`)\n\
         RT    - Primary runtime issue (`OK`, `MOD`, `NET`, `EMP`, `CTX`, `WAIT`)\n\
         VN    - Vein RAG status (`SEM`=semantic active, `FTS`=BM25 only, `--`=not indexed)\n\
         BUD   - Total prompt-budget pressure against the live context window\n\
         CMP   - History compaction pressure against Hematite's adaptive threshold\n\
         ERR   - Session error count (runtime, tool, or SPECULAR failures)\n\
         CTX   - Live context window currently reported by the provider\n\
         VOICE - Local speech output state\n\
         \nDocument note: `/attach` supports PDF/markdown/txt, but PDF parsing is best-effort by design so Hematite can stay a lightweight single-binary local coding harness and workstation assistant. If a PDF fails, export it to text/markdown or attach page images instead.\n\
         ",
    );
}

#[allow(dead_code)]
fn show_help_message_legacy(app: &mut App) {
    app.push_message("System",
        "Hematite Commands:\n\
         /chat             — (Mode) Conversation mode — clean chat, no tool noise\n\
         /agent            — (Mode) Full coding harness + workstation mode — tools, file edits, builds, inspection\n\
         /reroll           — (Soul) Hatch a new companion mid-session\n\
         /auto             — (Flow) Let Hematite choose the narrowest effective workflow\n\
         /ask [prompt]     — (Flow) Read-only analysis mode; optional inline prompt\n\
         /code [prompt]    — (Flow) Explicit implementation mode; optional inline prompt\n\
         /architect [prompt] — (Flow) Plan-first mode; optional inline prompt\n\
         /implement-plan   — (Flow) Execute the saved architect handoff in /code\n\
         /read-only [prompt] — (Flow) Hard read-only mode; optional inline prompt\n\
         /teach [prompt]   — (Flow) Teacher mode; inspect machine then walk you through any admin task step-by-step\n\
           /new              — (Reset) Fresh task context; clear chat, pins, and task files\n\
           /forget           — (Wipe) Hard forget; purge saved memory and Vein index too\n\
           /vein-inspect     — (Vein) Inspect indexed memory, hot files, and active room bias\n\
           /workspace-profile — (Profile) Show the auto-generated workspace profile\n\
           /rules            — (Rules) View behavioral guidelines (.hematite/rules.md)\n\
           /version          — (Build) Show the running Hematite version\n\
           /about            — (Info) Show author, repo, and product info\n\
           /vein-reset       — (Vein) Wipe the RAG index; rebuilds automatically on next turn\n\
           /clear            — (UI) Clear dialogue display only\n\
         /health           — (Diag) Run a synthesized plain-English system health report\n\
         /explain <text>   — (Help) Paste an error to get a non-technical breakdown\n\
         /gemma-native [auto|on|off|status] — (Model) Auto/force/disable Gemma 4 native formatting\n\
         /provider [status|lmstudio|ollama|clear|URL] — (Model) Show or save the active provider endpoint preference\n\
         /runtime          — (Model) Show the live runtime/provider/model/embed status and shortest fix path\n\
         /runtime fix      — (Model) Run the shortest safe runtime recovery step now\n\
         /runtime-refresh  — (Model) Re-read active provider model + CTX now\n\
         /model [status|list [available|loaded]|load <id> [--ctx N]|unload [id|current|all]|prefer <id>|clear] — (Model) Inspect, list, load, unload, or save the preferred coding model (`--ctx` uses LM Studio context length or Ollama `num_ctx`)\n\
         /embed [status|load <id>|unload [id|current]|prefer <id>|clear] — (Model) Inspect, load, unload, or save the preferred embed model\n\
         /undo             — (Ghost) Revert last file change\n\
         /diff             — (Git) Show session changes (--stat)\n\
         /lsp              — (Logic) Start Language Servers (semantic intelligence)\n\
         /swarm <text>     — (Swarm) Spawn parallel workers on a directive\n\
         /worktree <cmd>   — (Isolated) Manage git worktrees (list|add|remove|prune)\n\
         /think            — (Brain) Enable deep reasoning mode\n\
         /no_think         — (Speed) Disable reasoning (3-5x faster responses)\n\
         /voice            — (TTS) List all available voices\n\
         /voice N          — (TTS) Select voice by number\n\
         /read <text>      — (TTS) Speak text aloud directly, bypassing the model. ESC to stop.\n\
         /explain <text>   — (Plain English) Paste any error or output; Hematite explains it in plain English.\n\
         /health           — (SysAdmin) Run a full system health report (disk, RAM, tools, recent errors).\n\
         /attach <path>    — (Docs) Attach a PDF/markdown/txt file for next message\n\
         /attach-pick      — (Docs) Open a file picker and attach a document\n\
         /image <path>     — (Vision) Attach an image for the next message\n\
         /image-pick       — (Vision) Open a file picker and attach an image\n\
         /detach           — (Context) Drop pending document/image attachments\n\
         /copy             — (Debug) Copy session transcript to clipboard\n\
         /copy2            — (Debug) Copy the full SPECULAR rail to clipboard (reasoning + events)\n\
         \nHotkeys:\n\
         Ctrl+B — Toggle Brief Mode (minimal output; collapses side chrome)\n\
         Alt+↑/↓ — Scroll the SPECULAR rail by 3 lines\n\
         Alt+PgUp/PgDn — Scroll the SPECULAR rail by 10 lines\n\
         Alt+End — Snap SPECULAR back to live follow mode\n\
         Ctrl+P — Toggle Professional Mode (strip personality)\n\
         Ctrl+O — Open document picker for next-turn context\n\
         Ctrl+I — Open image picker for next-turn vision context\n\
         Ctrl+Y — Toggle Approvals Off (bypass safety approvals)\n\
         Ctrl+S — Quick Swarm (hardcoded bootstrap)\n\
         Ctrl+Z — Undo last edit\n\
         Ctrl+Q/C — Quit session\n\
         ESC    — Silence current playback\n\
         \nStatus Legend:\n\
         LM/OL — Provider runtime health (`LIVE`, `RECV`, `WARN`, `CEIL`, `STALE`, `BOOT`)\n\
         RT    — Primary runtime issue (`OK`, `MOD`, `NET`, `EMP`, `CTX`, `WAIT`)\n\
         VN    — Vein RAG status (`SEM`=semantic active, `FTS`=BM25 only, `--`=not indexed)\n\
         BUD   — Total prompt-budget pressure against the live context window\n\
         CMP   — History compaction pressure against Hematite's adaptive threshold\n\
         ERR   — Session error count (runtime, tool, or SPECULAR failures)\n\
         CTX   — Live context window currently reported by the provider\n\
         VOICE — Local speech output state\n\
         \nAssistant: Semantic Pathing (LSP), Vision Pass, Web Research, Swarm Synthesis"
    );
    app.push_message(
        "System",
        "Document note: `/attach` supports PDF/markdown/txt, but PDF parsing is best-effort by design so Hematite can stay a lightweight single-binary local coding harness and workstation assistant. If a PDF fails, export it to text/markdown or attach page images instead.",
    );
}

fn trigger_input_action(app: &mut App, action: InputAction) {
    match action {
        InputAction::Stop => request_stop(app),
        InputAction::PickDocument => match pick_attachment_path(AttachmentPickerKind::Document) {
            Ok(Some(path)) => attach_document_from_path(app, &path),
            Ok(None) => app.push_message("System", "Document picker cancelled."),
            Err(e) => app.push_message("System", &e),
        },
        InputAction::PickImage => match pick_attachment_path(AttachmentPickerKind::Image) {
            Ok(Some(path)) => attach_image_from_path(app, &path),
            Ok(None) => app.push_message("System", "Image picker cancelled."),
            Err(e) => app.push_message("System", &e),
        },
        InputAction::Detach => {
            app.clear_pending_attachments();
            app.push_message(
                "System",
                "Cleared pending document/image attachments for the next turn.",
            );
        }
        InputAction::New => {
            if !app.agent_running {
                reset_visible_session_state(app);
                app.push_message("You", "/new");
                app.agent_running = true;
                let _ = app.user_input_tx.try_send(UserTurn::text("/new"));
            }
        }
        InputAction::Forget => {
            if !app.agent_running {
                app.cancel_token
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                reset_visible_session_state(app);
                app.push_message("You", "/forget");
                app.agent_running = true;
                app.cancel_token
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                let _ = app.user_input_tx.try_send(UserTurn::text("/forget"));
            }
        }
        InputAction::Help => show_help_message(app),
    }
}

fn pick_attachment_path(kind: AttachmentPickerKind) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let (title, filter) = match kind {
            AttachmentPickerKind::Document => (
                "Attach document for the next Hematite turn",
                "Documents|*.pdf;*.md;*.markdown;*.txt;*.rst|All Files|*.*",
            ),
            AttachmentPickerKind::Image => (
                "Attach image for the next Hematite turn",
                "Images|*.png;*.jpg;*.jpeg;*.gif;*.webp|All Files|*.*",
            ),
        };
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms\n$dialog = New-Object System.Windows.Forms.OpenFileDialog\n$dialog.Title = '{title}'\n$dialog.Filter = '{filter}'\n$dialog.Multiselect = $false\nif ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ Write-Output $dialog.FileName }}"
        );
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-STA", "-Command", &script])
            .output()
            .map_err(|e| format!("File picker failed: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                "File picker did not complete successfully.".to_string()
            } else {
                format!("File picker failed: {}", stderr)
            });
        }
        let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if selected.is_empty() {
            Ok(None)
        } else {
            Ok(Some(selected))
        }
    }
    #[cfg(target_os = "macos")]
    {
        let prompt = match kind {
            AttachmentPickerKind::Document => "Choose a document for the next Hematite turn",
            AttachmentPickerKind::Image => "Choose an image for the next Hematite turn",
        };
        let script = format!("POSIX path of (choose file with prompt \"{}\")", prompt);
        let output = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| format!("File picker failed: {}", e))?;
        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if selected.is_empty() {
                Ok(None)
            } else {
                Ok(Some(selected))
            }
        } else {
            Ok(None)
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let title = match kind {
            AttachmentPickerKind::Document => "Attach document for the next Hematite turn",
            AttachmentPickerKind::Image => "Attach image for the next Hematite turn",
        };
        let output = std::process::Command::new("zenity")
            .args(["--file-selection", "--title", title])
            .output()
            .map_err(|e| format!("File picker failed: {}", e))?;
        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if selected.is_empty() {
                Ok(None)
            } else {
                Ok(Some(selected))
            }
        } else {
            Ok(None)
        }
    }
}

pub async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut specular_rx: Receiver<SpecularEvent>,
    mut agent_rx: Receiver<crate::agent::inference::InferenceEvent>,
    user_input_tx: tokio::sync::mpsc::Sender<UserTurn>,
    mut swarm_rx: Receiver<SwarmMessage>,
    swarm_tx: tokio::sync::mpsc::Sender<SwarmMessage>,
    swarm_coordinator: Arc<crate::agent::swarm::SwarmCoordinator>,
    last_interaction: Arc<Mutex<Instant>>,
    cockpit: crate::CliCockpit,
    soul: crate::ui::hatch::RustySoul,
    professional: bool,
    gpu_state: Arc<GpuState>,
    git_state: Arc<crate::agent::git_monitor::GitState>,
    cancel_token: Arc<std::sync::atomic::AtomicBool>,
    voice_manager: Arc<crate::ui::voice::VoiceManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App {
        messages: Vec::new(),
        messages_raw: Vec::new(),
        specular_logs: Vec::new(),
        brief_mode: cockpit.brief,
        tick_count: 0,
        stats: RustyStats {
            debugging: 0,
            wisdom: soul.wisdom,
            patience: 100.0,
            chaos: soul.chaos,
            snark: soul.snark,
        },
        yolo_mode: cockpit.yolo,
        awaiting_approval: None,
        active_workers: HashMap::new(),
        worker_labels: HashMap::new(),
        active_review: None,
        input: String::new(),
        input_history: Vec::new(),
        history_idx: None,
        thinking: false,
        agent_running: false,
        stop_requested: false,
        current_thought: String::new(),
        professional,
        last_reasoning: String::new(),
        active_context: default_active_context(),
        manual_scroll_offset: None,
        user_input_tx,
        specular_scroll: 0,
        specular_auto_scroll: true,
        gpu_state,
        git_state,
        last_input_time: Instant::now(),
        cancel_token,
        total_tokens: 0,
        current_session_cost: 0.0,
        model_id: "detecting...".to_string(),
        context_length: 0,
        prompt_pressure_percent: 0,
        prompt_estimated_input_tokens: 0,
        prompt_reserved_output_tokens: 0,
        prompt_estimated_total_tokens: 0,
        compaction_percent: 0,
        compaction_estimated_tokens: 0,
        compaction_threshold_tokens: 0,
        compaction_warned_level: 0,
        last_runtime_profile_time: Instant::now(),
        vein_file_count: 0,
        vein_embedded_count: 0,
        vein_docs_only: false,
        provider_name: "detecting".to_string(),
        provider_endpoint: String::new(),
        embed_model_id: None,
        provider_state: ProviderRuntimeState::Booting,
        last_provider_summary: String::new(),
        mcp_state: McpRuntimeState::Unconfigured,
        last_mcp_summary: String::new(),
        last_operator_checkpoint_state: OperatorCheckpointState::Idle,
        last_operator_checkpoint_summary: String::new(),
        last_recovery_recipe_summary: String::new(),
        think_mode: None,
        workflow_mode: "AUTO".into(),
        autocomplete_suggestions: Vec::new(),
        selected_suggestion: 0,
        show_autocomplete: false,
        autocomplete_filter: String::new(),
        current_objective: "Awaiting objective...".into(),
        voice_manager,
        voice_loading: false,
        voice_loading_progress: 1.0, // Pre-baked weights ready
        autocomplete_alias_active: false,
        hardware_guard_enabled: true,
        session_start: std::time::SystemTime::now(),
        soul_name: soul.species.clone(),
        attached_context: None,
        attached_image: None,
        hovered_input_action: None,
        teleported_from: cockpit.teleported_from.clone(),
        nav_list: Vec::new(),
        auto_approve_session: false,
        task_start_time: None,
        tool_started_at: HashMap::new(),
        recent_grounded_results: Vec::new(),
    };

    // Initial placeholder — streaming will overwrite this with hardware diagnostics
    app.push_message("Hematite", "Initialising Engine & Hardware...");

    if let Some(origin) = &app.teleported_from {
        app.push_message(
            "System",
            &format!(
                "Teleportation complete. You've arrived from {}. Hematite has launched this fresh session to ensure your original terminal remains clean and your context is grounded in this target workspace. What's our next move?",
                origin
            ),
        );
    }

    // ── Splash Screen ─────────────────────────────────────────────────────────
    // Blocking splash — user must press Enter to proceed.
    if !cockpit.no_splash {
        draw_splash(terminal)?;
        loop {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == event::KeyEventKind::Press
                    && matches!(key.code, KeyCode::Enter | KeyCode::Char(' '))
                {
                    break;
                }
            }
        }
    }

    if app.teleported_from.is_some()
        && crate::tools::plan::consume_teleport_resume_marker()
        && crate::tools::plan::load_plan_handoff().is_some()
    {
        app.workflow_mode = "CODE".into();
        app.thinking = true;
        app.agent_running = true;
        app.push_message(
            "System",
            "Teleport handoff detected in this project. Resuming from `.hematite/PLAN.md` automatically.",
        );
        app.push_message("You", "/implement-plan");
        let _ = app
            .user_input_tx
            .try_send(UserTurn::text("/implement-plan"));
    }

    let mut event_stream = EventStream::new();
    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(100));

    loop {
        // ── Hardware Watchdog ──
        let vram_ratio = app.gpu_state.ratio();
        if app.hardware_guard_enabled && vram_ratio > 0.95 && !app.brief_mode {
            app.brief_mode = true;
            app.push_message(
                "System",
                "🚨 HARDWARE GUARD: VRAM > 95%. Brief Mode auto-enabled to prevent crash.",
            );
        }

        app.sync_task_start_time();
        terminal.draw(|f| ui(f, &app))?;

        tokio::select! {
            _ = ticker.tick() => {
                // Increment voice loading progress (estimated 50s total load)
                if app.voice_loading && app.voice_loading_progress < 0.98 {
                    app.voice_loading_progress += 0.002;
                }

                let workers = app.active_workers.len() as u64;
                let advance = if workers > 0 { workers * 4 + 1 } else { 1 };
                // Scale advance to match new 100ms tick (formerly 500ms)
                // We keep animations consistent by only advancing tick_count every 5 ticks or scaling.
                // Let's just increment every tick but use a larger modulo in animations.
                app.tick_count = app.tick_count.wrapping_add(advance);
                app.update_objective();
            }

            // ── Keyboard / mouse input ────────────────────────────────────────
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(Event::Mouse(mouse))) => {
                        use crossterm::event::{MouseButton, MouseEventKind};
                        let (width, height) = match terminal.size() {
                            Ok(s) => (s.width, s.height),
                            Err(_) => (80, 24),
                        };
                        let is_right_side = mouse.column as f64 > width as f64 * 0.65;
                        let input_rect = input_rect_for_size(
                            Rect { x: 0, y: 0, width, height },
                            app.input.len(),
                        );
                        let title_area = input_title_area(input_rect);

                        match mouse.kind {
                            MouseEventKind::Moved => {
                                let hovered = if mouse.row == title_area.y
                                    && mouse.column >= title_area.x
                                    && mouse.column < title_area.x + title_area.width
                                {
                                    input_action_hitboxes(&app, title_area)
                                        .into_iter()
                                        .find_map(|(action, start, end)| {
                                            (mouse.column >= start && mouse.column <= end)
                                                .then_some(action)
                                        })
                                } else {
                                    None
                                };
                                app.hovered_input_action = hovered;
                            }
                            MouseEventKind::Down(MouseButton::Left) => {
                                if mouse.row == title_area.y
                                    && mouse.column >= title_area.x
                                    && mouse.column < title_area.x + title_area.width
                                {
                                    for (action, start, end) in input_action_hitboxes(&app, title_area) {
                                        if mouse.column >= start && mouse.column <= end {
                                            app.hovered_input_action = Some(action);
                                            trigger_input_action(&mut app, action);
                                            break;
                                        }
                                    }
                                } else {
                                    app.hovered_input_action = None;

                                    // Check Autocomplete Click
                                    if app.show_autocomplete && !app.autocomplete_suggestions.is_empty() {
                                        // The popup is rendered at chunks[1].y - (suggestions + 2)
                                        // Calculation must match ui() rendering logic exactly
                                        let items_len = app.autocomplete_suggestions.len();
                                        let popup_h = (items_len as u16 + 2).min(17); // 15 + borders
                                        let popup_y = input_rect.y.saturating_sub(popup_h);
                                        let popup_x = input_rect.x + 2;
                                        let popup_w = input_rect.width.saturating_sub(4);

                                        if mouse.row >= popup_y && mouse.row < popup_y + popup_h
                                            && mouse.column >= popup_x && mouse.column < popup_x + popup_w
                                        {
                                            // Clicked inside popup
                                            let mouse_relative_y = mouse.row.saturating_sub(popup_y + 1);
                                            if mouse_relative_y < items_len as u16 {
                                                let clicked_idx = mouse_relative_y as usize;
                                                let selected = &app.autocomplete_suggestions[clicked_idx].clone();
                                                app.apply_autocomplete_selection(selected);
                                            }
                                            continue; // Event handled
                                        }
                                    }
                                }
                            }
                            MouseEventKind::ScrollUp => {
                                if is_right_side {
                                    // User scrolled up — disable auto-scroll so they can read.
                                    scroll_specular_up(&mut app, 3);
                                } else {
                                    let cur = app.manual_scroll_offset.unwrap_or(0);
                                    app.manual_scroll_offset = Some(cur.saturating_add(3));
                                }
                            }
                            MouseEventKind::ScrollDown => {
                                if is_right_side {
                                    scroll_specular_down(&mut app, 3);
                                } else if let Some(cur) = app.manual_scroll_offset {
                                    app.manual_scroll_offset = if cur <= 3 { None } else { Some(cur - 3) };
                                }
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(Event::Key(key))) => {
                        if key.kind != event::KeyEventKind::Press { continue; }

                        // Update idle tracker for DeepReflect.
                        { *last_interaction.lock().unwrap() = Instant::now(); }

                        // ── Tier-2 Swarm diff review modal (exclusive lock) ───
                        if let Some(review) = app.active_review.take() {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    let _ = review.tx.send(ReviewResponse::Accept);
                                    app.push_message("System", &format!("Worker {} diff accepted.", review.worker_id));
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') => {
                                    let _ = review.tx.send(ReviewResponse::Reject);
                                    app.push_message("System", "Diff rejected.");
                                }
                                KeyCode::Char('r') | KeyCode::Char('R') => {
                                    let _ = review.tx.send(ReviewResponse::Retry);
                                    app.push_message("System", "Retrying synthesis…");
                                }
                                _ => { app.active_review = Some(review); }
                            }
                            continue;
                        }

                        // ── High-risk approval modal (exclusive lock) ─────────
                        if let Some(mut approval) = app.awaiting_approval.take() {
                            // Scroll keys — adjust offset and put approval back.
                            let scroll_handled = if approval.diff.is_some() {
                                let diff_lines = approval.diff.as_ref().map(|d| d.lines().count()).unwrap_or(0) as u16;
                                match key.code {
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        approval.diff_scroll = approval.diff_scroll.saturating_add(1).min(diff_lines.saturating_sub(1));
                                        true
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        approval.diff_scroll = approval.diff_scroll.saturating_sub(1);
                                        true
                                    }
                                    KeyCode::PageDown => {
                                        approval.diff_scroll = approval.diff_scroll.saturating_add(10).min(diff_lines.saturating_sub(1));
                                        true
                                    }
                                    KeyCode::PageUp => {
                                        approval.diff_scroll = approval.diff_scroll.saturating_sub(10);
                                        true
                                    }
                                    _ => false,
                                }
                            } else {
                                false
                            };
                            if scroll_handled {
                                app.awaiting_approval = Some(approval);
                                continue;
                            }
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    if let Some(ref diff) = approval.diff {
                                        let added = diff.lines().filter(|l| l.starts_with("+ ")).count();
                                        let removed = diff.lines().filter(|l| l.starts_with("- ")).count();
                                        app.push_message("System", &format!(
                                            "Applied: {} +{} -{}", approval.display, added, removed
                                        ));
                                    } else {
                                        app.push_message("System", &format!("Approved: {}", approval.display));
                                    }
                                    let _ = approval.responder.send(true);
                                }
                                KeyCode::Char('a') | KeyCode::Char('A') => {
                                    app.auto_approve_session = true;
                                    if let Some(ref diff) = approval.diff {
                                        let added = diff.lines().filter(|l| l.starts_with("+ ")).count();
                                        let removed = diff.lines().filter(|l| l.starts_with("- ")).count();
                                        app.push_message("System", &format!(
                                            "Applied: {} +{} -{}", approval.display, added, removed
                                        ));
                                    } else {
                                        app.push_message("System", &format!("Approved: {}", approval.display));
                                    }
                                    app.push_message("System", "🔓 FULL AUTONOMY — All mutations auto-approved for this session.");
                                    let _ = approval.responder.send(true);
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') => {
                                    if approval.diff.is_some() {
                                        app.push_message("System", "Edit skipped.");
                                    } else {
                                        app.push_message("System", "Declined.");
                                    }
                                    let _ = approval.responder.send(false);
                                }
                                _ => { app.awaiting_approval = Some(approval); }
                            }
                            continue;
                        }

                        // ── Normal key bindings ───────────────────────────────
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('c')
                                if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                    app.write_session_report();
                                    app.copy_transcript_to_clipboard();
                                    break;
                                }

                            KeyCode::Esc => {
                                request_stop(&mut app);
                            }

                            KeyCode::Char('b') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                app.brief_mode = !app.brief_mode;
                                // If the user manually toggles, silence the hardware guard for this session.
                                app.hardware_guard_enabled = false;
                                app.push_message("System", &format!("Hardware Guard {}: {}", if app.brief_mode { "ENFORCED" } else { "SILENCED" }, if app.brief_mode { "ON" } else { "OFF" }));
                            }
                            KeyCode::Char('p') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                app.professional = !app.professional;
                                app.push_message("System", &format!("Professional Harness: {}", if app.professional { "ACTIVE" } else { "DISABLED" }));
                            }
                            KeyCode::Char('y') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                app.yolo_mode = !app.yolo_mode;
                                app.push_message("System", &format!("Approvals Off: {}", if app.yolo_mode { "ON — all tools auto-approved" } else { "OFF" }));
                            }
                            KeyCode::Char('t') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                if !app.voice_manager.is_available() {
                                    app.push_message("System", "Voice is not available in this build. Use a packaged release for baked-in voice.");
                                } else {
                                    let enabled = app.voice_manager.toggle();
                                    app.push_message("System", &format!("Voice of Hematite: {}", if enabled { "VIBRANT" } else { "SILENCED" }));
                                }
                            }
                            KeyCode::Char('o') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                match pick_attachment_path(AttachmentPickerKind::Document) {
                                    Ok(Some(path)) => attach_document_from_path(&mut app, &path),
                                    Ok(None) => app.push_message("System", "Document picker cancelled."),
                                    Err(e) => app.push_message("System", &e),
                                }
                            }
                            KeyCode::Char('i') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                match pick_attachment_path(AttachmentPickerKind::Image) {
                                    Ok(Some(path)) => attach_image_from_path(&mut app, &path),
                                    Ok(None) => app.push_message("System", "Image picker cancelled."),
                                    Err(e) => app.push_message("System", &e),
                                }
                            }
                            KeyCode::Char('s') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                app.push_message("Hematite", "Swarm engaged.");
                                let swarm_tx_c = swarm_tx.clone();
                                let coord_c = swarm_coordinator.clone();
                                // Hardware-aware swarm: Limit workers if GPU is busy.
                                let max_workers = if app.gpu_state.ratio() > 0.70 { 1 } else { 3 };
                                if max_workers < 3 {
                                    app.push_message("System", "Hardware Guard: Limiting swarm to 1 worker due to GPU load.");
                                }

                                app.agent_running = true;
                                tokio::spawn(async move {
                                    let payload = r#"<worker_task id="1" target="src/ui/tui.rs">Implement Swarm Layout</worker_task>
<worker_task id="2" target="src/agent/swarm.rs">Build Scratchpad constraints</worker_task>
<worker_task id="3" target="docs">Update Readme</worker_task>"#;
                                    let tasks = crate::agent::parser::parse_master_spec(payload);
                                    let _ = coord_c.dispatch_swarm(tasks, swarm_tx_c, max_workers).await;
                                });
                            }
                            KeyCode::Char('z') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                match crate::tools::file_ops::pop_ghost_ledger() {
                                    Ok(msg) => {
                                        app.specular_logs.push(format!("GHOST: {}", msg));
                                        trim_vec(&mut app.specular_logs, 7);
                                        app.push_message("System", &msg);
                                    }
                                    Err(e) => {
                                        app.push_message("System", &format!("Undo failed: {}", e));
                                    }
                                }
                            }
                            KeyCode::Up
                                if key.modifiers.contains(event::KeyModifiers::ALT) =>
                            {
                                scroll_specular_up(&mut app, 3);
                            }
                            KeyCode::Down
                                if key.modifiers.contains(event::KeyModifiers::ALT) =>
                            {
                                scroll_specular_down(&mut app, 3);
                            }
                            KeyCode::PageUp
                                if key.modifiers.contains(event::KeyModifiers::ALT) =>
                            {
                                scroll_specular_up(&mut app, 10);
                            }
                            KeyCode::PageDown
                                if key.modifiers.contains(event::KeyModifiers::ALT) =>
                            {
                                scroll_specular_down(&mut app, 10);
                            }
                            KeyCode::End
                                if key.modifiers.contains(event::KeyModifiers::ALT) =>
                            {
                                follow_live_specular(&mut app);
                                app.push_message(
                                    "System",
                                    "SPECULAR snapped back to live follow mode.",
                                );
                            }
                            KeyCode::Up => {
                                if app.show_autocomplete && !app.autocomplete_suggestions.is_empty() {
                                    app.selected_suggestion = app.selected_suggestion.saturating_sub(1);
                                } else if app.manual_scroll_offset.is_some() {
                                    // Protect history: Use Up as a scroll fallback if already scrolling.
                                    let cur = app.manual_scroll_offset.unwrap();
                                    app.manual_scroll_offset = Some(cur.saturating_add(3));
                                } else if !app.input_history.is_empty() {
                                    // Only cycle history if we are at the bottom of the chat.
                                    let new_idx = match app.history_idx {
                                        None => app.input_history.len() - 1,
                                        Some(i) => i.saturating_sub(1),
                                    };
                                    app.history_idx = Some(new_idx);
                                    app.input = app.input_history[new_idx].clone();
                                }
                            }
                            KeyCode::Down => {
                                if app.show_autocomplete && !app.autocomplete_suggestions.is_empty() {
                                    app.selected_suggestion = (app.selected_suggestion + 1).min(app.autocomplete_suggestions.len().saturating_sub(1));
                                } else if let Some(off) = app.manual_scroll_offset {
                                    if off <= 3 { app.manual_scroll_offset = None; }
                                    else { app.manual_scroll_offset = Some(off.saturating_sub(3)); }
                                } else if let Some(i) = app.history_idx {
                                    if i + 1 < app.input_history.len() {
                                        app.history_idx = Some(i + 1);
                                        app.input = app.input_history[i + 1].clone();
                                    } else {
                                        app.history_idx = None;
                                        app.input.clear();
                                    }
                                }
                            }
                            KeyCode::PageUp => {
                                let cur = app.manual_scroll_offset.unwrap_or(0);
                                app.manual_scroll_offset = Some(cur.saturating_add(10));
                            }
                            KeyCode::PageDown => {
                                if let Some(off) = app.manual_scroll_offset {
                                    if off <= 10 { app.manual_scroll_offset = None; }
                                    else { app.manual_scroll_offset = Some(off.saturating_sub(10)); }
                                }
                            }
                            KeyCode::Tab => {
                                if app.show_autocomplete && !app.autocomplete_suggestions.is_empty() {
                                    let selected = app.autocomplete_suggestions[app.selected_suggestion].clone();
                                    app.apply_autocomplete_selection(&selected);
                                }
                            }
                            KeyCode::Char(c) => {
                                app.history_idx = None; // typing cancels history nav
                                app.input.push(c);
                                app.last_input_time = Instant::now();

                                if c == '@' {
                                    app.show_autocomplete = true;
                                    app.autocomplete_filter.clear();
                                    app.selected_suggestion = 0;
                                    app.update_autocomplete();
                                } else if app.show_autocomplete {
                                    app.autocomplete_filter.push(c);
                                    app.update_autocomplete();
                                }
                            }
                            KeyCode::Backspace => {
                                app.input.pop();
                                if app.show_autocomplete {
                                    if app.input.ends_with('@') || !app.input.contains('@') {
                                        app.show_autocomplete = false;
                                        app.autocomplete_filter.clear();
                                    } else {
                                        app.autocomplete_filter.pop();
                                        app.update_autocomplete();
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if app.show_autocomplete
                                    && !app.autocomplete_suggestions.is_empty()
                                    && should_accept_autocomplete_on_enter(
                                        app.autocomplete_alias_active,
                                        &app.autocomplete_filter,
                                    )
                                {
                                    let selected = app.autocomplete_suggestions[app.selected_suggestion].clone();
                                    app.apply_autocomplete_selection(&selected);
                                    continue;
                                }

                                if !app.input.is_empty()
                                    && (!app.agent_running
                                        || is_immediate_local_command(&app.input))
                                {
                                    // PASTE GUARD: If a newline arrives within 50ms of a character,
                                    // it's almost certainly part of a paste stream. Convert to space.
                                    if Instant::now().duration_since(app.last_input_time) < std::time::Duration::from_millis(50) {
                                        app.input.push(' ');
                                        app.last_input_time = Instant::now();
                                        continue;
                                    }

                                    let input_text = app.input.drain(..).collect::<String>();

                                    // ── Slash Command Processor ──────────────────────────
                                    if input_text.starts_with('/') {
                                        let parts: Vec<&str> = input_text.trim().split_whitespace().collect();
                                        let cmd = parts[0].to_lowercase();
                                        match cmd.as_str() {
                                            "/undo" => {
                                                match crate::tools::file_ops::pop_ghost_ledger() {
                                                    Ok(msg) => {
                                                        app.specular_logs.push(format!("GHOST: {}", msg));
                                                        trim_vec(&mut app.specular_logs, 7);
                                                        app.push_message("System", &msg);
                                                    }
                                                    Err(e) => {
                                                        app.push_message("System", &format!("Undo failed: {}", e));
                                                    }
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/clear" => {
                                                reset_visible_session_state(&mut app);
                                                app.push_message("System", "Dialogue buffer cleared.");
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/cd" => {
                                                if parts.len() < 2 {
                                                    app.push_message("System", "Usage: /cd <path>  — teleport to any directory. Supports bare tokens like downloads, desktop, docs, pictures, videos, music, home, temp, bare ~, aliases like @DESKTOP/project, plus .. and absolute paths. Tip: run /ls desktop first if you want a numbered picker.");
                                                    app.history_idx = None;
                                                    continue;
                                                }
                                                let raw = parts[1..].join(" ");
                                                let target = crate::tools::file_ops::resolve_candidate(&raw);
                                                if !target.exists() {
                                                    app.push_message("System", &format!("Directory not found: {}", target.display()));
                                                    app.history_idx = None;
                                                    continue;
                                                }
                                                if !target.is_dir() {
                                                    app.push_message("System", &format!("Not a directory: {}", target.display()));
                                                    app.history_idx = None;
                                                    continue;
                                                }
                                                let target_str = target.to_string_lossy().to_string();
                                                app.push_message("You", &format!("/cd {}", raw));
                                                app.push_message("System", &format!("Teleporting to {}...", target_str));
                                                app.push_message("System", "Launching new session. This terminal will close.");
                                                spawn_dive_in_terminal(&target_str);
                                                app.write_session_report();
                                                app.copy_transcript_to_clipboard();
                                                break;
                                            }
                                            "/ls" => {
                                                let base: std::path::PathBuf = if parts.len() >= 2 {
                                                    // /ls <path> or /ls <N>
                                                    let arg = parts[1..].join(" ");
                                                    if let Ok(n) = arg.trim().parse::<usize>() {
                                                        // /ls <N> — teleport to nav_list entry N
                                                        if n == 0 || n > app.nav_list.len() {
                                                            app.push_message("System", &format!("No entry {}. Run /ls first to see the list.", n));
                                                            app.history_idx = None;
                                                            continue;
                                                        }
                                                        let target = app.nav_list[n - 1].clone();
                                                        let target_str = target.to_string_lossy().to_string();
                                                        app.push_message("You", &format!("/ls {}", n));
                                                        app.push_message("System", &format!("Teleporting to {}...", target_str));
                                                        app.push_message("System", "Launching new session. This terminal will close.");
                                                        spawn_dive_in_terminal(&target_str);
                                                        app.write_session_report();
                                                        app.copy_transcript_to_clipboard();
                                                        break;
                                                    } else {
                                                        crate::tools::file_ops::resolve_candidate(&arg)
                                                    }
                                                } else {
                                                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                                                };

                                                // Build numbered nav list
                                                let mut entries: Vec<std::path::PathBuf> = Vec::new();
                                                let mut output = String::new();

                                                // Common locations (only when listing current/no-arg)
                                                let listing_base = parts.len() < 2;
                                                if listing_base {
                                                    let common: Vec<(&str, Option<std::path::PathBuf>)> = vec![
                                                        ("Desktop", dirs::desktop_dir()),
                                                        ("Downloads", dirs::download_dir()),
                                                        ("Documents", dirs::document_dir()),
                                                        ("Pictures", dirs::picture_dir()),
                                                        ("Home", dirs::home_dir()),
                                                    ];
                                                    let valid: Vec<_> = common.into_iter().filter_map(|(label, p)| p.map(|pb| (label, pb))).collect();
                                                    if !valid.is_empty() {
                                                        output.push_str("Common locations:\n");
                                                        for (label, pb) in &valid {
                                                            entries.push(pb.clone());
                                                            output.push_str(&format!("  {:>2}.  {:<12}  {}\n", entries.len(), label, pb.display()));
                                                        }
                                                    }
                                                }

                                                // Subdirectories of base path
                                                let cwd_label = if listing_base {
                                                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                                                } else {
                                                    base.clone()
                                                };
                                                if let Ok(read) = std::fs::read_dir(&cwd_label) {
                                                    let mut dirs_found: Vec<std::path::PathBuf> = read
                                                        .filter_map(|e| e.ok())
                                                        .filter(|e| e.path().is_dir())
                                                        .map(|e| e.path())
                                                        .collect();
                                                    dirs_found.sort();
                                                    if !dirs_found.is_empty() {
                                                        output.push_str(&format!("\n{}:\n", cwd_label.display()));
                                                        for pb in &dirs_found {
                                                            entries.push(pb.clone());
                                                            let name = pb.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                                                            output.push_str(&format!("  {:>2}.  {}\n", entries.len(), name));
                                                        }
                                                    }
                                                }

                                                if entries.is_empty() {
                                                    app.push_message("System", "No directories found.");
                                                } else {
                                                    output.push_str("\nType /ls <N> to teleport to that directory.");
                                                    app.nav_list = entries;
                                                    app.push_message("System", &output);
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/diff" => {
                                                app.push_message("System", "Fetching session diff...");
                                                let ws = crate::tools::file_ops::workspace_root();
                                                if crate::agent::git::is_git_repo(&ws) {
                                                    let output = std::process::Command::new("git")
                                                        .args(["diff", "--stat"])
                                                        .current_dir(ws)
                                                        .output();
                                                    if let Ok(out) = output {
                                                        let stat = String::from_utf8_lossy(&out.stdout).to_string();
                                                        app.push_message("System", if stat.is_empty() { "No changes detected." } else { &stat });
                                                    }
                                                } else {
                                                    app.push_message("System", "Not a git repository. Diff limited.");
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/vein-reset" => {
                                                app.vein_file_count = 0;
                                                app.vein_embedded_count = 0;
                                                app.push_message("You", "/vein-reset");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/vein-reset"));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/vein-inspect" => {
                                                app.push_message("You", "/vein-inspect");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/vein-inspect"));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/workspace-profile" => {
                                                app.push_message("You", "/workspace-profile");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/workspace-profile"));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/copy" => {
                                                app.copy_transcript_to_clipboard();
                                                app.push_message("System", "Exact session transcript copied to clipboard (includes help/system output).");
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/copy-last" => {
                                                if app.copy_last_reply_to_clipboard() {
                                                    app.push_message("System", "Latest Hematite reply copied to clipboard.");
                                                } else {
                                                    app.push_message("System", "No Hematite reply is available to copy yet.");
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/copy-clean" => {
                                                app.copy_clean_transcript_to_clipboard();
                                                app.push_message("System", "Clean chat transcript copied to clipboard (skips help/debug boilerplate).");
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/copy2" => {
                                                app.copy_specular_to_clipboard();
                                                app.push_message("System", "SPECULAR log copied to clipboard (reasoning + events).");
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/voice" => {
                                                use crate::ui::voice::VOICE_LIST;
                                                if let Some(arg) = parts.get(1) {
                                                    // /voice N — select by number
                                                    if let Ok(n) = arg.parse::<usize>() {
                                                        let idx = n.saturating_sub(1);
                                                        if let Some(&(id, label)) = VOICE_LIST.get(idx) {
                                                            app.voice_manager.set_voice(id);
                                                            let _ = crate::agent::config::set_voice(id);
                                                            app.push_message("System", &format!("Voice set to {} — {}", id, label));
                                                        } else {
                                                            app.push_message("System", &format!("Invalid voice number. Use /voice to list voices (1–{}).", VOICE_LIST.len()));
                                                        }
                                                    } else {
                                                        // /voice af_bella — select by name
                                                        if let Some(&(id, label)) = VOICE_LIST.iter().find(|&&(id, _)| id == *arg) {
                                                            app.voice_manager.set_voice(id);
                                                            let _ = crate::agent::config::set_voice(id);
                                                            app.push_message("System", &format!("Voice set to {} — {}", id, label));
                                                        } else {
                                                            app.push_message("System", &format!("Unknown voice '{}'. Use /voice to list voices.", arg));
                                                        }
                                                    }
                                                } else {
                                                    // /voice — list all voices
                                                    let current = app.voice_manager.current_voice_id();
                                                    let mut list = format!("Available voices (current: {}):\n", current);
                                                    for (i, &(id, label)) in VOICE_LIST.iter().enumerate() {
                                                        let marker = if id == current.as_str() { " ◀" } else { "" };
                                                        list.push_str(&format!("  {:>2}. {}{}\n", i + 1, label, marker));
                                                    }
                                                    list.push_str("\nUse /voice N or /voice <id> to select.");
                                                    app.push_message("System", &list);
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/read" => {
                                                let text = parts[1..].join(" ");
                                                if text.is_empty() {
                                                    app.push_message("System", "Usage: /read <text to speak>");
                                                } else if !app.voice_manager.is_available() {
                                                    app.push_message("System", "Voice is not available in this build. Use a packaged release for baked-in voice.");
                                                } else if !app.voice_manager.is_enabled() {
                                                    app.push_message("System", "Voice is off. Press Ctrl+T to enable, then /read again.");
                                                } else {
                                                    app.push_message("System", &format!("Reading {} words aloud. ESC to stop.", text.split_whitespace().count()));
                                                    app.voice_manager.speak(text.clone());
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/new" => {
                                                reset_visible_session_state(&mut app);
                                                app.push_message("You", "/new");
                                                app.agent_running = true;
                                                app.clear_pending_attachments();
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/new"));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/forget" => {
                                                // Cancel any running turn so /forget isn't queued behind retries.
                                                app.cancel_token.store(true, std::sync::atomic::Ordering::SeqCst);
                                                reset_visible_session_state(&mut app);
                                                app.push_message("You", "/forget");
                                                app.agent_running = true;
                                                app.cancel_token.store(false, std::sync::atomic::Ordering::SeqCst);
                                                app.clear_pending_attachments();
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/forget"));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/gemma-native" => {
                                                let sub = parts.get(1).copied().unwrap_or("status").to_ascii_lowercase();
                                                let gemma_detected = crate::agent::inference::is_hematite_native_model(&app.model_id);
                                                match sub.as_str() {
                                                    "auto" => {
                                                        match crate::agent::config::set_gemma_native_mode("auto") {
                                                            Ok(_) => {
                                                                if gemma_detected {
                                                                    app.push_message("System", "Gemma Native Formatting: AUTO. Gemma 4 will use native formatting automatically on the next turn.");
                                                                } else {
                                                                    app.push_message("System", "Gemma Native Formatting: AUTO in settings. It will activate automatically when a Gemma 4 model is loaded.");
                                                                }
                                                            }
                                                            Err(e) => app.push_message("System", &format!("Failed to update settings: {}", e)),
                                                        }
                                                    }
                                                    "on" => {
                                                        match crate::agent::config::set_gemma_native_mode("on") {
                                                            Ok(_) => {
                                                                if gemma_detected {
                                                                    app.push_message("System", "Gemma Native Formatting: ON (forced). It will apply on the next turn.");
                                                                } else {
                                                                    app.push_message("System", "Gemma Native Formatting: ON (forced) in settings. It will activate only when a Gemma 4 model is loaded.");
                                                                }
                                                            }
                                                            Err(e) => app.push_message("System", &format!("Failed to update settings: {}", e)),
                                                        }
                                                    }
                                                    "off" => {
                                                        match crate::agent::config::set_gemma_native_mode("off") {
                                                            Ok(_) => app.push_message("System", "Gemma Native Formatting: OFF."),
                                                            Err(e) => app.push_message("System", &format!("Failed to update settings: {}", e)),
                                                        }
                                                    }
                                                    _ => {
                                                        let config = crate::agent::config::load_config();
                                                        let mode = crate::agent::config::gemma_native_mode_label(&config, &app.model_id);
                                                        let enabled = match mode {
                                                            "on" => "ON (forced)",
                                                            "auto" => "ON (auto)",
                                                            "off" => "OFF",
                                                            _ => "INACTIVE",
                                                        };
                                                        let model_note = if gemma_detected {
                                                            "Gemma 4 detected."
                                                        } else {
                                                            "Current model is not Gemma 4."
                                                        };
                                                        app.push_message(
                                                            "System",
                                                            &format!(
                                                                "Gemma Native Formatting: {}. {} Usage: /gemma-native auto | on | off | status",
                                                                enabled, model_note
                                                            ),
                                                        );
                                                    }
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/chat" => {
                                                app.workflow_mode = "CHAT".into();
                                                app.push_message("System", "Chat mode — natural conversation, no agent scaffolding. Use /agent to return to the full harness, or /ask, /architect, or /code to jump straight into a narrower workflow.");
                                                app.history_idx = None;
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/chat"));
                                                continue;
                                            }
                                            "/reroll" => {
                                                app.history_idx = None;
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/reroll"));
                                                continue;
                                            }
                                            "/agent" => {
                                                app.workflow_mode = "AUTO".into();
                                                app.push_message("System", "Agent mode — full coding harness and workstation assistant active. Use /auto for normal behavior, /ask for read-only analysis, /architect for plan-first work, /code for implementation, or /chat for clean conversation.");
                                                app.history_idx = None;
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/agent"));
                                                continue;
                                            }
                                            "/implement-plan" => {
                                                app.workflow_mode = "CODE".into();
                                                app.push_message("You", "/implement-plan");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/implement-plan"));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/ask" | "/code" | "/architect" | "/read-only" | "/auto" | "/teach" => {
                                                let label = match cmd.as_str() {
                                                    "/ask" => "ASK",
                                                    "/code" => "CODE",
                                                    "/architect" => "ARCHITECT",
                                                    "/read-only" => "READ-ONLY",
                                                    "/teach" => "TEACH",
                                                    _ => "AUTO",
                                                };
                                                app.workflow_mode = label.to_string();
                                                let outbound = input_text.trim().to_string();
                                                app.push_message("You", &outbound);
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send(UserTurn::text(outbound));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/worktree" => {
                                                let sub = parts.get(1).copied().unwrap_or("");
                                                match sub {
                                                    "list" => {
                                                        app.push_message("You", "/worktree list");
                                                        app.agent_running = true;
                                                        let _ = app.user_input_tx.try_send(UserTurn::text(
                                                            "Call git_worktree with action=list"
                                                        ));
                                                    }
                                                    "add" => {
                                                        let wt_path = parts.get(2).copied().unwrap_or("");
                                                        let wt_branch = parts.get(3).copied().unwrap_or("");
                                                        if wt_path.is_empty() {
                                                            app.push_message("System", "Usage: /worktree add <path> [branch]");
                                                        } else {
                                                            app.push_message("You", &format!("/worktree add {wt_path}"));
                                                            app.agent_running = true;
                                                            let directive = if wt_branch.is_empty() {
                                                                format!("Call git_worktree with action=add path={wt_path}")
                                                            } else {
                                                                format!("Call git_worktree with action=add path={wt_path} branch={wt_branch}")
                                                            };
                                                            let _ = app.user_input_tx.try_send(UserTurn::text(directive));
                                                        }
                                                    }
                                                    "remove" => {
                                                        let wt_path = parts.get(2).copied().unwrap_or("");
                                                        if wt_path.is_empty() {
                                                            app.push_message("System", "Usage: /worktree remove <path>");
                                                        } else {
                                                            app.push_message("You", &format!("/worktree remove {wt_path}"));
                                                            app.agent_running = true;
                                                            let _ = app.user_input_tx.try_send(UserTurn::text(
                                                                format!("Call git_worktree with action=remove path={wt_path}")
                                                            ));
                                                        }
                                                    }
                                                    "prune" => {
                                                        app.push_message("You", "/worktree prune");
                                                        app.agent_running = true;
                                                        let _ = app.user_input_tx.try_send(UserTurn::text(
                                                            "Call git_worktree with action=prune"
                                                        ));
                                                    }
                                                    _ => {
                                                        app.push_message("System",
                                                            "Usage: /worktree list | add <path> [branch] | remove <path> | prune");
                                                    }
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/think" => {
                                                app.think_mode = Some(true);
                                                app.push_message("You", "/think");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/think"));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/no_think" => {
                                                app.think_mode = Some(false);
                                                app.push_message("You", "/no_think");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/no_think"));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/lsp" => {
                                                app.push_message("You", "/lsp");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/lsp"));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/runtime-refresh" => {
                                                app.push_message("You", "/runtime-refresh");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send(UserTurn::text("/runtime-refresh"));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/rules" => {
                                                let sub = parts.get(1).copied().unwrap_or("status").to_ascii_lowercase();
                                                let ws_root = crate::tools::file_ops::workspace_root();

                                                match sub.as_str() {
                                                    "view" => {
                                                        let mut combined = String::new();
                                                        let candidates = [
                                                            "CLAUDE.md",
                                                            ".claude.md",
                                                            "CLAUDE.local.md",
                                                            "HEMATITE.md",
                                                            ".hematite/rules.md",
                                                            ".hematite/rules.local.md",
                                                        ];
                                                        for cand in candidates {
                                                            let p = ws_root.join(cand);
                                                            if p.exists() {
                                                                if let Ok(c) = std::fs::read_to_string(&p) {
                                                                    combined.push_str(&format!("--- [{}] ---\n", cand));
                                                                    combined.push_str(&c);
                                                                    combined.push_str("\n\n");
                                                                }
                                                            }
                                                        }
                                                        if combined.is_empty() {
                                                            app.push_message("System", "No rule files found (CLAUDE.md, .hematite/rules.md, etc.).");
                                                        } else {
                                                            app.push_message("System", &format!("Current behavioral rules being injected:\n\n{}", combined));
                                                        }
                                                    }
                                                    "edit" => {
                                                        let which = parts.get(2).copied().unwrap_or("local").to_ascii_lowercase();
                                                        let target_file = if which == "shared" { "rules.md" } else { "rules.local.md" };
                                                        let target_path = crate::tools::file_ops::hematite_dir().join(target_file);

                                                        if !target_path.exists() {
                                                            if let Some(parent) = target_path.parent() {
                                                                let _ = std::fs::create_dir_all(parent);
                                                            }
                                                            let header = if which == "shared" { "# Project Rules (Shared)" } else { "# Local Guidelines (Private)" };
                                                            let _ = std::fs::write(&target_path, format!("{}\n\nAdd behavioral guidelines here for the agent to follow in this workspace.\n", header));
                                                        }

                                                        match crate::tools::file_ops::open_in_system_editor(&target_path) {
                                                            Ok(_) => app.push_message("System", &format!("Opening {} in system editor...", target_path.display())),
                                                            Err(e) => app.push_message("System", &format!("Failed to open editor: {}", e)),
                                                        }
                                                    }
                                                    _ => {
                                                        let mut status = "Behavioral Guidelines:\n".to_string();
                                                        let candidates = [
                                                            "CLAUDE.md",
                                                            ".claude.md",
                                                            "CLAUDE.local.md",
                                                            "HEMATITE.md",
                                                            ".hematite/rules.md",
                                                            ".hematite/rules.local.md",
                                                        ];
                                                        for cand in candidates {
                                                              let p = ws_root.join(cand);
                                                              let icon = if p.exists() { "[v]" } else { "[ ]" };
                                                              let label = if cand.contains(".local") || cand.ends_with(".local.md") { "(local override)" } else { "(shared asset)" };
                                                              status.push_str(&format!("  {} {:<25} {}\n", icon, cand, label));
                                                        }
                                                        status.push_str("\nUsage:\n  /rules view        - View combined rules\n  /rules edit        - Edit personal local rules (ignored by git)\n  /rules edit shared - Edit project-wide shared rules");
                                                        app.push_message("System", &status);
                                                    }
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/help" => {
                                                show_help_message(&mut app);
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/help-legacy-unused" => {
                                                app.push_message("System",
                                                    "Hematite Commands:\n\
                                                     /chat             — (Mode) Conversation mode — clean chat, no tool noise\n\
                                                     /agent            — (Mode) Full coding harness + workstation mode — tools, file edits, builds, inspection\n\
                                                     /reroll           — (Soul) Hatch a new companion mid-session\n\
                                                     /auto             — (Flow) Let Hematite choose the narrowest effective workflow\n\
                                                     /ask [prompt]     — (Flow) Read-only analysis mode; optional inline prompt\n\
                                                     /code [prompt]    — (Flow) Explicit implementation mode; optional inline prompt\n\
                                                     /architect [prompt] — (Flow) Plan-first mode; optional inline prompt\n\
                                                     /implement-plan   — (Flow) Execute the saved architect handoff in /code\n\
                                                     /read-only [prompt] — (Flow) Hard read-only mode; optional inline prompt\n\
                                                     /teach [prompt]   — (Flow) Teacher mode; inspect machine then walk you through any admin task step-by-step\n\
                                                       /new              — (Reset) Fresh task context; clear chat, pins, and task files\n\
                                                       /forget           — (Wipe) Hard forget; purge saved memory and Vein index too\n\
                                                       /vein-inspect     — (Vein) Inspect indexed memory, hot files, and active room bias\n\
                                                       /workspace-profile — (Profile) Show the auto-generated workspace profile\n\
                                                       /rules            — (Rules) View behavioral guidelines (.hematite/rules.md)\n\
                                                       /version          — (Build) Show the running Hematite version\n\
                                                       /about            — (Info) Show author, repo, and product info\n\
                                                       /vein-reset       — (Vein) Wipe the RAG index; rebuilds automatically on next turn\n\
                                                       /clear            — (UI) Clear dialogue display only\n\
                                                     /gemma-native [auto|on|off|status] — (Model) Auto/force/disable Gemma 4 native formatting\n\
                                                     /provider [status|lmstudio|ollama|clear|URL] — (Model) Show or save the active provider endpoint preference\n\
                                                     /runtime          — (Model) Show the live runtime/provider/model/embed status and shortest fix path\n\
                                                     /runtime fix      — (Model) Run the shortest safe runtime recovery step now\n\
                                                     /runtime-refresh  — (Model) Re-read active provider model + CTX now\n\
                                                     /model [status|list [available|loaded]|load <id> [--ctx N]|unload [id|current|all]|prefer <id>|clear] — (Model) Inspect, list, load, unload, or save the preferred coding model (`--ctx` uses LM Studio context length or Ollama `num_ctx`)\n\
                                                     /embed [status|load <id>|unload [id|current]|prefer <id>|clear] — (Model) Inspect, load, unload, or save the preferred embed model\n\
                                                     /undo             — (Ghost) Revert last file change\n\
                                                     /diff             — (Git) Show session changes (--stat)\n\
                                                     /lsp              — (Logic) Start Language Servers (semantic intelligence)\n\
                                                     /swarm <text>     — (Swarm) Spawn parallel workers on a directive\n\
                                                     /worktree <cmd>   — (Isolated) Manage git worktrees (list|add|remove|prune)\n\
                                                     /think            — (Brain) Enable deep reasoning mode\n\
                                                     /no_think         — (Speed) Disable reasoning (3-5x faster responses)\n\
                                                     /voice            — (TTS) List all available voices\n\
                                                     /voice N          — (TTS) Select voice by number\n\
                                                     /attach <path>    — (Docs) Attach a PDF/markdown/txt file for next message\n\
                                                     /attach-pick      — (Docs) Open a file picker and attach a document\n\
                                                     /image <path>     — (Vision) Attach an image for the next message\n\
                                                     /image-pick       — (Vision) Open a file picker and attach an image\n\
                                                     /detach           — (Context) Drop pending document/image attachments\n\
                                                     /copy             — (Debug) Copy session transcript to clipboard\n\
                                                     /copy2            — (Debug) Copy SPECULAR log to clipboard (reasoning + events)\n\
                                                     \nHotkeys:\n\
                                                     Ctrl+B — Toggle Brief Mode (minimal output; collapses side chrome)\n\
                                                     Ctrl+P — Toggle Professional Mode (strip personality)\n\
                                                     Ctrl+O — Open document picker for next-turn context\n\
                                                     Ctrl+I — Open image picker for next-turn vision context\n\
                                                     Ctrl+Y — Toggle Approvals Off (bypass safety approvals)\n\
                                                     Ctrl+S — Quick Swarm (hardcoded bootstrap)\n\
                                                     Ctrl+Z — Undo last edit\n\
                                                     Ctrl+Q/C — Quit session\n\
                                                     ESC    — Silence current playback\n\
                                                     \nStatus Legend:\n\
                                                     LM    — LM Studio runtime health (`LIVE`, `RECV`, `WARN`, `CEIL`, `STALE`, `BOOT`)\n\
                                                     VN    — Vein RAG status (`SEM`=semantic active, `FTS`=BM25 only, `--`=not indexed)\n\
                                                     BUD   â€” Total prompt-budget pressure against the live context window\n\
                                                     CMP   â€” History compaction pressure against Hematite's adaptive threshold\n\
                                                     ERR   â€” Session error count (runtime, tool, or SPECULAR failures)\n\
                                                     CTX   â€” Live context window currently reported by LM Studio\n\
                                                     VOICE â€” Local speech output state\n\
                                                     \nAssistant: Semantic Pathing (LSP), Vision Pass, Web Research, Swarm Synthesis"
                                                );
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/swarm" => {
                                                let directive = parts[1..].join(" ");
                                                if directive.is_empty() {
                                                    app.push_message("System", "Usage: /swarm <directive>");
                                                } else {
                                                    app.active_workers.clear(); // Fresh architecture for a fresh command
                                                    app.push_message("Hematite", &format!("Swarm analyzing: '{}'", directive));
                                                    let swarm_tx_c = swarm_tx.clone();
                                                    let coord_c = swarm_coordinator.clone();
                                                    let max_workers = if app.gpu_state.ratio() > 0.75 { 1 } else { 3 };
                                                    app.agent_running = true;
                                                    tokio::spawn(async move {
                                                        let payload = format!(r#"<worker_task id="1" target="src">Research {}</worker_task>
<worker_task id="2" target="src">Implement {}</worker_task>
<worker_task id="3" target="docs">Document {}</worker_task>"#, directive, directive, directive);
                                                        let tasks = crate::agent::parser::parse_master_spec(&payload);
                                                        let _ = coord_c.dispatch_swarm(tasks, swarm_tx_c, max_workers).await;
                                                    });
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/provider" => {
                                                let arg_text = parts[1..].join(" ").trim().to_string();
                                                handle_provider_command(&mut app, arg_text).await;
                                                continue;
                                            }
                                            "/runtime" => {
                                                let arg_text = parts[1..].join(" ").trim().to_string();
                                                let lower = arg_text.to_ascii_lowercase();
                                                match lower.as_str() {
                                                    "" | "status" => {
                                                        app.push_message(
                                                            "System",
                                                            &format_runtime_summary(&app).await,
                                                        );
                                                    }
                                                    "explain" => {
                                                        app.push_message(
                                                            "System",
                                                            &format_runtime_explanation(&app).await,
                                                        );
                                                    }
                                                    "refresh" => {
                                                        let _ = app
                                                            .user_input_tx
                                                            .try_send(UserTurn::text(
                                                                "/runtime-refresh",
                                                            ));
                                                        app.push_message("You", "/runtime refresh");
                                                        app.agent_running = true;
                                                    }
                                                    "fix" => {
                                                        handle_runtime_fix(&mut app).await;
                                                    }
                                                    _ if lower.starts_with("provider") => {
                                                        let provider_arg =
                                                            arg_text["provider".len()..].trim().to_string();
                                                        if provider_arg.is_empty() {
                                                            app.push_message(
                                                                "System",
                                                                "Usage: /runtime provider [status|lmstudio|ollama|clear|http://host:port/v1]",
                                                            );
                                                        } else {
                                                            handle_provider_command(&mut app, provider_arg)
                                                                .await;
                                                        }
                                                    }
                                                    _ => {
                                                        app.push_message(
                                                            "System",
                                                            "Usage: /runtime [status|explain|fix|refresh|provider ...]",
                                                        );
                                                    }
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/model" | "/embed" => {
                                                let outbound = input_text.clone();
                                                app.push_message("You", &outbound);
                                                app.agent_running = true;
                                                app.stop_requested = false;
                                                app.cancel_token.store(
                                                    false,
                                                    std::sync::atomic::Ordering::SeqCst,
                                                );
                                                app.last_reasoning.clear();
                                                app.manual_scroll_offset = None;
                                                app.specular_auto_scroll = true;
                                                let _ = app
                                                    .user_input_tx
                                                    .try_send(UserTurn::text(outbound));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/version" => {
                                                app.push_message(
                                                    "System",
                                                    &crate::hematite_version_report(),
                                                );
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/about" => {
                                                app.push_message(
                                                    "System",
                                                    &crate::hematite_about_report(),
                                                );
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/explain" => {
                                                let error_text = parts[1..].join(" ");
                                                if error_text.trim().is_empty() {
                                                    app.push_message("System", "Usage: /explain <error message or text>\n\nPaste any error, warning, or confusing message and Hematite will explain it in plain English — what it means, why it happened, and what to do about it.");
                                                } else {
                                                    let framed = format!(
                                                        "The user pasted the following error or message and needs a plain-English explanation. \
                                                         Explain what this means, why it happened, and what to do about it. \
                                                         Use simple, non-technical language. Avoid jargon. \
                                                         Structure your response as:\n\
                                                         1. What happened (one sentence)\n\
                                                         2. Why it happened\n\
                                                         3. How to fix it (step by step)\n\
                                                         4. How to prevent it next time (optional, if relevant)\n\n\
                                                         Error/message to explain:\n```\n{}\n```",
                                                        error_text
                                                    );
                                                    app.push_message("You", &format!("/explain {}", error_text));
                                                    app.agent_running = true;
                                                    let _ = app.user_input_tx.try_send(UserTurn::text(framed));
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/health" => {
                                                app.push_message("You", "/health");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send(UserTurn::text(
                                                    "Run inspect_host with topic=health_report. \
                                                     After getting the report, summarize it in plain English for a non-technical user. \
                                                     Use the tier labels (Needs fixing / Worth watching / Looking good) and \
                                                     give specific, actionable next steps for any items that need attention."
                                                ));
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/detach" => {
                                                app.clear_pending_attachments();
                                                app.push_message("System", "Cleared pending document/image attachments for the next turn.");
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/attach" => {
                                                let file_path = parts[1..].join(" ").trim().to_string();
                                                if file_path.is_empty() {
                                                    app.push_message("System", "Usage: /attach <path>  - attach a file (PDF, markdown, txt) as context for the next message.\nPDF parsing is best-effort for single-binary portability; scanned/image-only or oddly encoded PDFs may fail.\nUse /attach-pick for a file dialog. Drop reference docs in .hematite/docs/ to have them indexed permanently.");
                                                    app.history_idx = None;
                                                    continue;
                                                }
                                                if file_path.is_empty() {
                                                    app.push_message("System", "Usage: /attach <path>  — attach a file (PDF, markdown, txt) as context for the next message.\nUse /attach-pick for a file dialog. Drop reference docs in .hematite/docs/ to have them indexed permanently.");
                                                } else {
                                                    let p = std::path::Path::new(&file_path);
                                                    match crate::memory::vein::extract_document_text(p) {
                                                        Ok(text) => {
                                                            let name = p.file_name()
                                                                .and_then(|n| n.to_str())
                                                                .unwrap_or(&file_path)
                                                                .to_string();
                                                            let preview_len = text.len().min(200);
                                                            app.push_message("System", &format!(
                                                                "Attached: {} ({} chars) — will be injected as context on your next message.\nPreview: {}...",
                                                                name, text.len(), &text[..preview_len]
                                                            ));
                                                            app.attached_context = Some((name, text));
                                                        }
                                                        Err(e) => {
                                                            app.push_message("System", &format!("Attach failed: {}", e));
                                                        }
                                                    }
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/attach-pick" => {
                                                match pick_attachment_path(AttachmentPickerKind::Document) {
                                                    Ok(Some(path)) => attach_document_from_path(&mut app, &path),
                                                    Ok(None) => app.push_message("System", "Document picker cancelled."),
                                                    Err(e) => app.push_message("System", &e),
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/image" => {
                                                let file_path = parts[1..].join(" ").trim().to_string();
                                                if file_path.is_empty() {
                                                    app.push_message("System", "Usage: /image <path>  - attach an image (PNG/JPG/GIF/WebP) for the next message.\nUse /image-pick for a file dialog.");
                                                } else {
                                                    attach_image_from_path(&mut app, &file_path);
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/image-pick" => {
                                                match pick_attachment_path(AttachmentPickerKind::Image) {
                                                    Ok(Some(path)) => attach_image_from_path(&mut app, &path),
                                                    Ok(None) => app.push_message("System", "Image picker cancelled."),
                                                    Err(e) => app.push_message("System", &e),
                                                }
                                                app.history_idx = None;
                                                continue;
                                            }
                                            _ => {
                                                app.push_message("System", &format!("Unknown command: {}", cmd));
                                                app.history_idx = None;
                                                continue;
                                            }
                                        }
                                    }

                                    // Save to history (avoid consecutive duplicates).
                                    if app.input_history.last().map(|s| s.as_str()) != Some(&input_text) {
                                        app.input_history.push(input_text.clone());
                                        if app.input_history.len() > 50 {
                                            app.input_history.remove(0);
                                        }
                                    }
                                    app.history_idx = None;
                                    app.clear_grounded_recovery_cache();
                                    app.push_message("You", &input_text);
                                    app.agent_running = true;
                                    app.stop_requested = false;
                                    app.cancel_token.store(false, std::sync::atomic::Ordering::SeqCst);
                                    app.last_reasoning.clear();
                                    app.manual_scroll_offset = None;
                                    app.specular_auto_scroll = true;
                                    let tx = app.user_input_tx.clone();
                                    let outbound = UserTurn {
                                        text: input_text,
                                        attached_document: app.attached_context.take().map(|(name, content)| {
                                            AttachedDocument { name, content }
                                        }),
                                        attached_image: app.attached_image.take(),
                                    };
                                    tokio::spawn(async move {
                                        let _ = tx.send(outbound).await;
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(Event::Paste(content))) => {
                        if !try_attach_from_paste(&mut app, &content) {
                            // Normalize pasted newlines into spaces so we don't accidentally submit
                            // multiple lines or break the single-line input logic.
                            let normalized = content.replace("\r\n", " ").replace('\n', " ");
                            app.input.push_str(&normalized);
                            app.last_input_time = Instant::now();
                        }
                    }
                    _ => {}
                }
            }

            // ── Specular proactive watcher ────────────────────────────────────
            Some(specular_evt) = specular_rx.recv() => {
                match specular_evt {
                    SpecularEvent::SyntaxError { path, details } => {
                        app.record_error();
                        app.specular_logs.push(format!("ERROR: {:?}", path));
                        trim_vec(&mut app.specular_logs, 20);

                        // Only proactively suggest a fix if the user isn't actively typing.
                        let user_idle = {
                            let lock = last_interaction.lock().unwrap();
                            lock.elapsed() > std::time::Duration::from_secs(3)
                        };
                        if user_idle && !app.agent_running {
                            app.agent_running = true;
                            let tx = app.user_input_tx.clone();
                            let diag = details.clone();
                            tokio::spawn(async move {
                                let msg = format!(
                                    "<specular-build-fail>\n{}\n</specular-build-fail>\n\
                                     Fix the compiler error above.",
                                    diag
                                );
                                let _ = tx.send(UserTurn::text(msg)).await;
                            });
                        }
                    }
                    SpecularEvent::FileChanged(path) => {
                        app.stats.wisdom += 1;
                        app.stats.patience = (app.stats.patience - 0.5).max(0.0);
                        if app.stats.patience < 50.0 && !app.brief_mode {
                            app.brief_mode = true;
                            app.push_message("System", "Context saturation high — Brief Mode auto-enabled.");
                        }
                        let path_str = path.to_string_lossy().to_string();
                        app.specular_logs.push(format!("INDEX: {}", path_str));
                        app.push_context_file(path_str, "Active".into());
                        trim_vec(&mut app.specular_logs, 20);
                    }
                }
            }

            // ── Inference / agent events ──────────────────────────────────────
            Some(event) = agent_rx.recv() => {
                use crate::agent::inference::InferenceEvent;
                match event {
                    InferenceEvent::Thought(content) => {
                        if app.stop_requested {
                            continue;
                        }
                        app.thinking = true;
                        app.current_thought.push_str(&content);
                    }
                    InferenceEvent::VoiceStatus(msg) => {
                        if app.stop_requested {
                            continue;
                        }
                        app.push_message("System", &msg);
                    }
                    InferenceEvent::Token(ref token) | InferenceEvent::MutedToken(ref token) => {
                        if app.stop_requested {
                            continue;
                        }
                        let is_muted = matches!(event, InferenceEvent::MutedToken(_));
                        app.thinking = false;
                        if app.messages_raw.last().map(|(s, _)| s.as_str()) != Some("Hematite") {
                            app.push_message("Hematite", "");
                        }
                        app.update_last_message(token);
                        app.manual_scroll_offset = None;

                        // ONLY speak if not muted
                        if !is_muted && app.voice_manager.is_enabled() && !app.cancel_token.load(std::sync::atomic::Ordering::SeqCst) {
                            app.voice_manager.speak(token.clone());
                        }
                    }
                    InferenceEvent::ToolCallStart { id, name, args } => {
                        if app.stop_requested {
                            continue;
                        }
                        app.tool_started_at.insert(id, Instant::now());
                        // In chat mode, suppress tool noise from the main chat surface.
                        if app.workflow_mode != "CHAT" {
                            let display = format!("( )  {} {}", name, args);
                            app.push_message("Tool", &display);
                        }
                        // Always track in active context regardless of mode
                        app.active_context.push(ContextFile {
                            path: name.clone(),
                            size: 0,
                            status: "Running".into()
                        });
                        trim_vec_context(&mut app.active_context, 8);
                        app.manual_scroll_offset = None;
                    }
                    InferenceEvent::ToolCallResult { id, name, result, is_error } => {
                        if app.stop_requested {
                            continue;
                        }
                        if should_capture_grounded_tool_output(&name, is_error) {
                            app.recent_grounded_results.push((name.clone(), result.clone()));
                            if app.recent_grounded_results.len() > 4 {
                                app.recent_grounded_results.remove(0);
                            }
                        }
                        let icon = if is_error { "[x]" } else { "[v]" };
                        let elapsed_chip = app
                            .tool_started_at
                            .remove(&id)
                            .map(|started| format_tool_elapsed(started.elapsed()));
                        if is_error {
                            app.record_error();
                        }
                        // In chat mode, suppress tool results from main chat.
                        // Errors still show so the user knows something went wrong.
                        let preview = first_n_chars(&result, 100);
                        if app.workflow_mode != "CHAT" {
                            let display = if let Some(elapsed) = elapsed_chip.as_deref() {
                                format!("{}  {} [{}] ? {}", icon, name, elapsed, preview)
                            } else {
                                format!("{}  {} ? {}", icon, name, preview)
                            };
                            app.push_message("Tool", &display);
                        } else if is_error {
                            app.push_message("System", &format!("Tool error: {}", preview));
                        }

                        // If it was a read or write, we can extract the path from the app.active_context "Running" entries
                        // but it's simpler to just let Specular handle the indexing or update here if we had the path.

                        // Remove "Running" tools from context list
                        app.active_context.retain(|f| f.path != name || f.status != "Running");
                        app.manual_scroll_offset = None;
                    }
                    InferenceEvent::ApprovalRequired { id: _, name, display, diff, mutation_label, responder } => {
                        if app.stop_requested {
                            let _ = responder.send(false);
                            continue;
                        }
                        // Session-level auto-approve: skip dialog entirely.
                        if app.auto_approve_session {
                            if let Some(ref diff) = diff {
                                let added = diff.lines().filter(|l| l.starts_with("+ ")).count();
                                let removed = diff.lines().filter(|l| l.starts_with("- ")).count();
                                app.push_message("System", &format!(
                                    "Auto-approved: {} +{} -{}", display, added, removed
                                ));
                            } else {
                                app.push_message("System", &format!("Auto-approved: {}", display));
                            }
                            let _ = responder.send(true);
                            continue;
                        }
                        let is_diff = diff.is_some();
                        app.awaiting_approval = Some(PendingApproval {
                            display: display.clone(),
                            tool_name: name,
                            diff,
                            diff_scroll: 0,
                            mutation_label,
                            responder,
                        });
                        if is_diff {
                            app.push_message("System", "[~]  Diff preview — [Y] Apply  [N] Skip  [A] Accept All");
                        } else {
                            app.push_message("System", "[!]  Approval required — [Y] Approve  [N] Decline  [A] Accept All");
                            app.push_message("System", &format!("Command: {}", display));
                        }
                    }
                    InferenceEvent::TurnTiming { context_prep_ms, inference_ms, execution_ms } => {
                        app.specular_logs.push(format!(
                            "PROFILE: Prep {}ms | Eval {}ms | Exec {}ms",
                            context_prep_ms, inference_ms, execution_ms
                        ));
                        trim_vec(&mut app.specular_logs, 20);
                    }
                    InferenceEvent::UsageUpdate(usage) => {
                        app.total_tokens = usage.total_tokens;
                        // Calculate discounted cost for this turn.
                        let turn_cost = crate::agent::pricing::calculate_cost(&usage, &app.model_id);
                        app.current_session_cost += turn_cost;
                    }
                    InferenceEvent::Done => {
                        app.thinking = false;
                        app.agent_running = false;
                        app.stop_requested = false;
                        if app.voice_manager.is_enabled() {
                            app.voice_manager.flush();
                        }
                        if !app.current_thought.is_empty() {
                            app.last_reasoning = app.current_thought.clone();
                        }
                        app.current_thought.clear();
                        // Force one last repaint of the visible chat buffer in case the
                        // final streamed token chunk did not trigger the lightweight
                        // reformat heuristic in update_last_message().
                        app.rebuild_formatted_messages();
                        app.manual_scroll_offset = None;
                        app.specular_auto_scroll = true;
                        // Clear single-agent task bars on completion
                        app.active_workers.remove("AGENT");
                        app.worker_labels.remove("AGENT");
                    }
                    InferenceEvent::CopyDiveInCommand(path) => {
                        let command = format!("cd \"{}\" && hematite", path.replace('\\', "/"));
                        copy_text_to_clipboard(&command);
                        spawn_dive_in_terminal(&path);
                        app.push_message("System", &format!("Teleportation initiated: New terminal launched at {}", path));
                        app.push_message("System", "Teleportation complete. Closing original session to maintain workstation hygiene...");

                        // Self-Destruct Sequence: Graceful exit matching Ctrl+Q behavior
                        app.write_session_report();
                        app.copy_transcript_to_clipboard();
                        break;
                    }
                    InferenceEvent::ChainImplementPlan => {
                        app.push_message("You", "/implement-plan (Autonomous Handoff)");
                        app.manual_scroll_offset = None;
                    }
                    InferenceEvent::Error(e) => {
                        app.record_error();
                        app.thinking = false;
                        app.agent_running = false;
                        if app.voice_manager.is_enabled() {
                            app.voice_manager.flush();
                        }
                        app.push_message("System", &format!("Error: {e}"));
                    }
                    InferenceEvent::ProviderStatus { state, summary } => {
                        app.provider_state = state;
                        if !summary.trim().is_empty() && app.last_provider_summary != summary {
                            app.specular_logs.push(format!("PROVIDER: {}", summary));
                            trim_vec(&mut app.specular_logs, 20);
                            app.last_provider_summary = summary;
                        }
                    }
                    InferenceEvent::McpStatus { state, summary } => {
                        app.mcp_state = state;
                        if !summary.trim().is_empty() && app.last_mcp_summary != summary {
                            app.specular_logs.push(format!("MCP: {}", summary));
                            trim_vec(&mut app.specular_logs, 20);
                            app.last_mcp_summary = summary;
                        }
                    }
                    InferenceEvent::OperatorCheckpoint { state, summary } => {
                        app.last_operator_checkpoint_state = state;
                        if state == OperatorCheckpointState::Idle {
                            app.last_operator_checkpoint_summary.clear();
                        } else if !summary.trim().is_empty()
                            && app.last_operator_checkpoint_summary != summary
                        {
                            app.specular_logs.push(format!(
                                "STATE: {} - {}",
                                state.label(),
                                summary
                            ));
                            trim_vec(&mut app.specular_logs, 20);
                            app.last_operator_checkpoint_summary = summary;
                        }
                    }
                    InferenceEvent::RecoveryRecipe { summary } => {
                        if !summary.trim().is_empty()
                            && app.last_recovery_recipe_summary != summary
                        {
                            app.specular_logs.push(format!("RECOVERY: {}", summary));
                            trim_vec(&mut app.specular_logs, 20);
                            app.last_recovery_recipe_summary = summary;
                        }
                    }
                    InferenceEvent::CompactionPressure {
                        estimated_tokens,
                        threshold_tokens,
                        percent,
                    } => {
                        app.compaction_estimated_tokens = estimated_tokens;
                        app.compaction_threshold_tokens = threshold_tokens;
                        app.compaction_percent = percent;
                        // Fire a one-shot warning when crossing 70% or 90%.
                        // Reset warned_level to 0 when pressure drops back below 60%
                        // so warnings re-fire if context fills up again after a /new.
                        if percent < 60 {
                            app.compaction_warned_level = 0;
                        } else if percent >= 90 && app.compaction_warned_level < 90 {
                            app.compaction_warned_level = 90;
                            app.push_message(
                                "System",
                                "Context is 90% full. Use /new to reset history (project memory is preserved) or /forget to wipe everything.",
                            );
                        } else if percent >= 70 && app.compaction_warned_level < 70 {
                            app.compaction_warned_level = 70;
                            app.push_message(
                                "System",
                                &format!("Context at {}% — approaching the compaction threshold. Consider /new soon to keep responses sharp.", percent),
                            );
                        }
                    }
                    InferenceEvent::PromptPressure {
                        estimated_input_tokens,
                        reserved_output_tokens,
                        estimated_total_tokens,
                        context_length: _,
                        percent,
                    } => {
                        app.prompt_estimated_input_tokens = estimated_input_tokens;
                        app.prompt_reserved_output_tokens = reserved_output_tokens;
                        app.prompt_estimated_total_tokens = estimated_total_tokens;
                        app.prompt_pressure_percent = percent;
                    }
                    InferenceEvent::TaskProgress { id, label, progress } => {
                        let nid = normalize_id(&id);
                        app.active_workers.insert(nid.clone(), progress);
                        app.worker_labels.insert(nid, label);
                    }
                    InferenceEvent::RuntimeProfile {
                        provider_name,
                        endpoint,
                        model_id,
                        context_length,
                    } => {
                        let was_no_model = app.model_id == "no model loaded";
                        let now_no_model = model_id == "no model loaded";
                        let changed = app.model_id != "detecting..."
                            && (app.model_id != model_id || app.context_length != context_length);
                        let provider_changed = app.provider_name != provider_name;
                        app.provider_name = provider_name.clone();
                        app.provider_endpoint = endpoint.clone();
                        app.model_id = model_id.clone();
                        app.context_length = context_length;
                        app.last_runtime_profile_time = Instant::now();
                        if app.provider_state == ProviderRuntimeState::Booting {
                            app.provider_state = ProviderRuntimeState::Live;
                        }
                        if now_no_model && !was_no_model {
                            let mut guidance = if provider_name == "Ollama" {
                                "No coding model is currently available from Ollama. Pull or load a chat model in Ollama, then keep `api_url` pointed at `http://localhost:11434/v1`. If you also want semantic search, set `/embed prefer <id>` to an Ollama embedding model.".to_string()
                            } else {
                                "No coding model loaded. Load a model in LM Studio (e.g. Qwen/Qwen3.5-9B Q4_K_M) and start the server on port 1234. Optionally also load an embedding model for semantic search.".to_string()
                            };
                            if let Some((alt_name, alt_url)) =
                                crate::runtime::detect_alternative_provider(&provider_name).await
                            {
                                guidance.push_str(&format!(
                                    " Reachable alternative detected: {} ({}). Use `/provider {}` and restart Hematite if you want to switch.",
                                    alt_name,
                                    alt_url,
                                    alt_name.to_ascii_lowercase().replace(' ', "")
                                ));
                            }
                            app.push_message("System", &guidance);
                        } else if provider_changed && !now_no_model {
                            app.push_message(
                                "System",
                                &format!(
                                    "Provider detected: {} | Model {} | CTX {}",
                                    provider_name, model_id, context_length
                                ),
                            );
                        } else if changed && !now_no_model {
                            app.push_message(
                                "System",
                                &format!(
                                    "Runtime profile refreshed: {} | Model {} | CTX {}",
                                    provider_name, model_id, context_length
                                ),
                            );
                        }
                    }
                    InferenceEvent::EmbedProfile { model_id } => {
                        let changed = app.embed_model_id != model_id;
                        app.embed_model_id = model_id.clone();
                        if changed {
                            match model_id {
                                Some(id) => app.push_message(
                                    "System",
                                    &format!("Embed model loaded: {} (semantic search ready)", id),
                                ),
                                None => app.push_message(
                                    "System",
                                    "Embed model unloaded. Semantic search inactive.",
                                ),
                            }
                        }
                    }
                    InferenceEvent::VeinStatus { file_count, embedded_count, docs_only } => {
                        app.vein_file_count = file_count;
                        app.vein_embedded_count = embedded_count;
                        app.vein_docs_only = docs_only;
                    }
                    InferenceEvent::VeinContext { paths } => {
                        // Replace the default placeholder entries with what the
                        // Vein actually surfaced for this turn.
                        app.active_context.retain(|f| f.status == "Running");
                        for path in paths {
                            let root = crate::tools::file_ops::workspace_root();
                            let size = std::fs::metadata(root.join(&path))
                                .map(|m| m.len())
                                .unwrap_or(0);
                            if !app.active_context.iter().any(|f| f.path == path) {
                                app.active_context.push(ContextFile {
                                    path,
                                    size,
                                    status: "Vein".to_string(),
                                });
                            }
                        }
                        trim_vec_context(&mut app.active_context, 8);
                    }
                    InferenceEvent::SoulReroll { species, rarity, shiny, .. } => {
                        let shiny_tag = if shiny { " 🌟 SHINY" } else { "" };
                        app.soul_name = species.clone();
                        app.push_message(
                            "System",
                            &format!("[{}{}] {} has awakened.", rarity, shiny_tag, species),
                        );
                    }
                    InferenceEvent::ShellLine(line) => {
                        // Stream shell output into the SPECULAR side panel as it
                        // arrives so the operator sees live progress.
                        app.current_thought.push_str(&line);
                        app.current_thought.push('\n');
                    }
                }
            }

            // ── Swarm messages ────────────────────────────────────────────────
            Some(msg) = swarm_rx.recv() => {
                match msg {
                    SwarmMessage::Progress(worker_id, progress) => {
                        let nid = normalize_id(&worker_id);
                        app.active_workers.insert(nid.clone(), progress);
                        match progress {
                            102 => app.push_message("System", &format!("Worker {} architecture verified and applied.", nid)),
                            101 => { /* Handled by 102 terminal message */ },
                            100 => app.push_message("Hematite", &format!("Worker {} complete. Standing by for review...", nid)),
                            _ => {}
                        }
                    }
                    SwarmMessage::ReviewRequest { worker_id, file_path, before, after, tx } => {
                        app.push_message("Hematite", &format!("Worker {} conflict — review required.", worker_id));
                        app.active_review = Some(ActiveReview {
                            worker_id,
                            file_path: file_path.to_string_lossy().to_string(),
                            before,
                            after,
                            tx,
                        });
                    }
                    SwarmMessage::Done => {
                        app.agent_running = false;
                        // Workers now persist in SPECULAR until a new command is issued
                        app.push_message("System", "──────────────────────────────────────────────────────────");
                        app.push_message("System", " TASK COMPLETE: Swarm Synthesis Finalized ");
                        app.push_message("System", "──────────────────────────────────────────────────────────");
                    }
                }
            }
        }
    }
    Ok(())
}

// ── Render ────────────────────────────────────────────────────────────────────

fn ui(f: &mut ratatui::Frame, app: &App) {
    let size = f.size();
    if size.width < 60 || size.height < 10 {
        // Render a minimal wait message or just clear if area is too collapsed
        f.render_widget(Clear, size);
        return;
    }

    let input_height = compute_input_height(f.size().width, app.input.len());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(input_height),
            Constraint::Length(5), // Expanded to accommodate Multi-Tier Liquid Telemetry
        ])
        .split(f.size());

    let sidebar_mode = sidebar_mode(app, size.width);
    let sidebar_width = match sidebar_mode {
        SidebarMode::Hidden => 0,
        SidebarMode::Compact => 32,
        SidebarMode::Full => 45,
    };
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Length(sidebar_width)])
        .split(chunks[0]);

    // ── Box 1: Dialogue ───────────────────────────────────────────────────────
    let mut core_lines = app.messages.clone();

    // Show agent-running indicator as last line when active.
    if app.agent_running {
        let dots = ".".repeat((app.tick_count % 4) as usize + 1);
        core_lines.push(Line::from(Span::styled(
            format!(" Hematite is thinking{}", dots),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::DIM),
        )));
    }

    let (heart_color, core_icon) = if app.agent_running || !app.active_workers.is_empty() {
        let (r_base, g_base, b_base) = if !app.active_workers.is_empty() {
            (0, 200, 200) // Cyan pulse for swarm
        } else {
            (200, 0, 200) // Magenta pulse for thinking
        };

        let pulse = (app.tick_count % 50) as f64 / 50.0;
        let factor = (pulse * std::f64::consts::PI).sin().abs();
        let r = (r_base as f64 * factor) as u8;
        let g = (g_base as f64 * factor) as u8;
        let b = (b_base as f64 * factor) as u8;

        (Color::Rgb(r.max(60), g.max(60), b.max(60)), "•")
    } else {
        (Color::Rgb(80, 80, 80), "•") // Standby
    };

    let live_objective = if app.current_objective != "Idle" {
        app.current_objective.clone()
    } else if !app.active_workers.is_empty() {
        "Swarm active".to_string()
    } else if app.thinking {
        "Reasoning".to_string()
    } else if app.agent_running {
        "Working".to_string()
    } else {
        "Idle".to_string()
    };

    let objective_text = if live_objective.len() > 30 {
        format!("{}...", &live_objective[..27])
    } else {
        live_objective
    };

    let core_title = if app.professional {
        Line::from(vec![
            Span::styled(format!(" {} ", core_icon), Style::default().fg(heart_color)),
            Span::styled("HEMATITE ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!(" TASK: {} ", objective_text),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])
    } else {
        Line::from(format!(" TASK: {} ", objective_text))
    };

    let core_para = Paragraph::new(core_lines.clone())
        .block(
            Block::default()
                .title(core_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: true });

    // Enhanced Scroll calculation.
    let avail_h = top[0].height.saturating_sub(2);
    // Borders (2) + Scrollbar (1) + explicit Padding (1) = 4.
    let inner_w = top[0].width.saturating_sub(4).max(1);

    let mut total_lines: u16 = 0;
    for line in &core_lines {
        let line_w = line.width() as u16;
        if line_w == 0 {
            total_lines += 1;
        } else {
            // TUI SCROLL FIX:
            // Exact calculation: how many times does line_w fit into inner_w?
            // This matches Paragraph's internal Wrap logic closely.
            let wrapped = (line_w + inner_w - 1) / inner_w;
            total_lines += wrapped;
        }
    }

    let max_scroll = total_lines.saturating_sub(avail_h);
    let scroll = if let Some(off) = app.manual_scroll_offset {
        max_scroll.saturating_sub(off)
    } else {
        max_scroll
    };

    // Clear the outer chunk and the inner dialogue area to prevent ghosting from previous frames or background renders.
    f.render_widget(Clear, top[0]);

    // Create a sub-area for the dialogue with horizontal padding.
    let chat_area = Rect::new(
        top[0].x + 1,
        top[0].y,
        top[0].width.saturating_sub(2).max(1),
        top[0].height,
    );
    f.render_widget(Clear, chat_area);
    f.render_widget(core_para.scroll((scroll, 0)), chat_area);

    // Scrollbar: content_length = max_scroll+1 so position==max_scroll puts the
    // thumb flush at the bottom (position == content_length - 1).
    let mut scrollbar_state =
        ScrollbarState::new(max_scroll as usize + 1).position(scroll as usize);
    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        top[0],
        &mut scrollbar_state,
    );

    // ── Box 2: Side panel ─────────────────────────────────────────────────────
    if sidebar_mode == SidebarMode::Compact && top[1].width > 0 {
        let compact_title = if sidebar_has_live_activity(app) {
            " SIGNALS "
        } else {
            " SESSION "
        };
        let compact_para = Paragraph::new(build_compact_sidebar_lines(app))
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(compact_title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        f.render_widget(Clear, top[1]);
        f.render_widget(compact_para, top[1]);
    } else if sidebar_mode == SidebarMode::Full && top[1].width > 0 {
        let side = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // CONTEXT
                Constraint::Min(0),    // SPECULAR
            ])
            .split(top[1]);

        // Pane 1: Context (Nervous focus)
        let context_source = if app.active_context.is_empty() {
            default_active_context()
        } else {
            app.active_context.clone()
        };
        let mut context_display = context_source
            .iter()
            .map(|f| {
                let (icon, color) = match f.status.as_str() {
                    "Running" => ("⚙️", Color::Cyan),
                    "Dirty" => ("📝", Color::Yellow),
                    _ => ("📄", Color::Gray),
                };
                // Simple heuristic for "Tokens" (size / 4)
                let tokens = f.size / 4;
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", icon), Style::default().fg(color)),
                    Span::styled(f.path.clone(), Style::default().fg(Color::White)),
                    Span::styled(
                        format!(" {}t ", tokens),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect::<Vec<ListItem>>();

        if context_display.is_empty() {
            context_display = vec![ListItem::new(" (No active files)")];
        }

        let ctx_title = if sidebar_has_live_activity(app) {
            " LIVE CONTEXT "
        } else {
            " SESSION CONTEXT "
        };

        let ctx_block = Block::default()
            .title(ctx_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        f.render_widget(Clear, side[0]);
        f.render_widget(List::new(context_display).block(ctx_block), side[0]);

        // Optional: Add a Gauge for total context if tokens were tracked accurately.
        // For now, let's just make the CONTEXT pane look high-density.

        // ── SPECULAR panel (Pane 2) ────────────────────────────────────────────────
        let v_title = if app.thinking || app.agent_running {
            " HEMATITE SIGNALS [live] ".to_string()
        } else {
            " HEMATITE SIGNALS [watching] ".to_string()
        };

        f.render_widget(Clear, side[1]);

        let mut v_lines: Vec<Line<'static>> = Vec::new();

        // Section: live thought (bounded to last 300 chars to avoid wall-of-text)
        if app.thinking || app.agent_running {
            let dots = ".".repeat((app.tick_count % 4) as usize + 1);
            let label = if app.thinking { "REASONING" } else { "WORKING" };
            v_lines.push(Line::from(vec![Span::styled(
                format!("[ {}{} ]", label, dots),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]));
            // Show last 300 chars of current thought, split by line.
            let preview = if app.current_thought.chars().count() > 300 {
                app.current_thought
                    .chars()
                    .rev()
                    .take(300)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<String>()
            } else {
                app.current_thought.clone()
            };
            for raw in preview.lines() {
                let raw = raw.trim();
                if !raw.is_empty() {
                    v_lines.extend(render_markdown_line(raw));
                }
            }
            v_lines.push(Line::raw(""));
        } else {
            v_lines.push(Line::from(vec![
                Span::styled("• ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Waiting for the next turn. Runtime, MCP, and index signals stay visible here.",
                    Style::default().fg(Color::Gray),
                ),
            ]));
            v_lines.push(Line::raw(""));
        }

        let signal_rows = sidebar_signal_rows(app);
        if !signal_rows.is_empty() {
            let section_title = if app.thinking || app.agent_running {
                "-- Operator Signals --"
            } else {
                "-- Session Snapshot --"
            };
            v_lines.push(Line::from(vec![Span::styled(
                section_title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::DIM),
            )]));
            for (row, color) in signal_rows
                .iter()
                .take(if app.thinking || app.agent_running {
                    4
                } else {
                    3
                })
            {
                v_lines.push(Line::from(vec![
                    Span::styled("- ", Style::default().fg(Color::DarkGray)),
                    Span::styled(row.clone(), Style::default().fg(*color)),
                ]));
            }
            v_lines.push(Line::raw(""));
        }

        // Section: worker progress bars
        if !app.active_workers.is_empty() {
            v_lines.push(Line::from(vec![Span::styled(
                "── Task Progress ──",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::DIM),
            )]));

            let mut sorted_ids: Vec<_> = app.active_workers.keys().cloned().collect();
            sorted_ids.sort();

            for id in sorted_ids {
                let prog = app.active_workers[&id];
                let custom_label = app.worker_labels.get(&id).cloned();

                let (label, color) = match prog {
                    101..=102 => ("VERIFIED", Color::Green),
                    100 if !app.agent_running && id != "AGENT" => ("SKIPPED ", Color::DarkGray),
                    100 => ("REVIEW  ", Color::Magenta),
                    _ => ("WORKING ", Color::Yellow),
                };

                let display_label = custom_label.unwrap_or_else(|| label.to_string());
                let filled = (prog.min(100) / 10) as usize;
                let bar = "▓".repeat(filled) + &"░".repeat(10 - filled);

                let id_prefix = if id == "AGENT" {
                    "Agent: ".to_string()
                } else {
                    format!("W{}: ", id)
                };

                v_lines.push(Line::from(vec![
                    Span::styled(id_prefix, Style::default().fg(Color::Gray)),
                    Span::styled(bar, Style::default().fg(color)),
                    Span::styled(
                        format!(" {} ", display_label),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{}%", prog.min(100)),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
            v_lines.push(Line::raw(""));
        }

        // Section: last completed turn's reasoning
        if (app.thinking || app.agent_running) && !app.last_reasoning.is_empty() {
            v_lines.push(Line::from(vec![Span::styled(
                "── Logic Trace ──",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::DIM),
            )]));
            for raw in app.last_reasoning.lines() {
                v_lines.extend(render_markdown_line(raw));
            }
            v_lines.push(Line::raw(""));
        }

        // Section: specular event log
        if !app.specular_logs.is_empty() {
            v_lines.push(Line::from(vec![Span::styled(
                if app.thinking || app.agent_running {
                    "── Live Events ──"
                } else {
                    "── Recent Events ──"
                },
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::DIM),
            )]));
            let recent_logs: Vec<String> = if app.thinking || app.agent_running {
                app.specular_logs.iter().rev().take(8).cloned().collect()
            } else {
                app.specular_logs.iter().rev().take(5).cloned().collect()
            };
            for log in recent_logs.into_iter().rev() {
                let (icon, color) = if log.starts_with("ERROR") {
                    ("X ", Color::Red)
                } else if log.starts_with("INDEX") {
                    ("I ", Color::Cyan)
                } else if log.starts_with("GHOST") {
                    ("< ", Color::Magenta)
                } else {
                    ("- ", Color::Gray)
                };
                v_lines.push(Line::from(vec![
                    Span::styled(icon, Style::default().fg(color)),
                    Span::styled(
                        log,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::DIM),
                    ),
                ]));
            }
        }

        let v_total = v_lines.len() as u16;
        let v_avail = side[1].height.saturating_sub(2);
        let v_max_scroll = v_total.saturating_sub(v_avail);
        // If auto-scroll is active, always show the bottom. Otherwise respect the
        // user's manual position (clamped so we never scroll past the content end).
        let v_scroll = if app.specular_auto_scroll {
            v_max_scroll
        } else {
            app.specular_scroll.min(v_max_scroll)
        };

        let specular_para = Paragraph::new(v_lines)
            .wrap(Wrap { trim: true })
            .scroll((v_scroll, 0))
            .block(Block::default().title(v_title).borders(Borders::ALL));

        f.render_widget(specular_para, side[1]);

        // Scrollbar for SPECULAR
        let mut v_scrollbar_state =
            ScrollbarState::new(v_max_scroll as usize + 1).position(v_scroll as usize);
        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            side[1],
            &mut v_scrollbar_state,
        );
    }

    // ── Box 3: Status bar ─────────────────────────────────────────────────────
    let frame = app.tick_count % 3;
    let _spark = match frame {
        0 => "✧",
        1 => "✦",
        _ => "✨",
    };
    let _vigil = if app.brief_mode {
        "VIGIL:[ON]"
    } else {
        "VIGIL:[off]"
    };
    let _yolo = if app.yolo_mode {
        " | APPROVALS: OFF"
    } else {
        ""
    };

    let bar_constraints = vec![Constraint::Fill(1)];
    let bar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(bar_constraints)
        .split(chunks[2]);

    // ── Box 4: Logic Activity Row (Alive cues) ───────────────────────────────
    // We render this in the bottom-most area if active.
    let _footer_row_legacy = if app.agent_running || app.thinking {
        let elapsed = if let Some(start) = app.task_start_time {
            format!(" {:0>2}s ", start.elapsed().as_secs())
        } else {
            String::new()
        };
        let last_log = app
            .specular_logs
            .last()
            .map(|s| s.as_str())
            .unwrap_or("...");
        let spinner = match app.tick_count % 8 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            3 => "⠸",
            4 => "⠼",
            5 => "⠴",
            6 => "⠦",
            _ => "⠧",
        };

        Line::from(vec![
            Span::styled(
                format!(" {} ", spinner),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                elapsed,
                Style::default()
                    .bg(Color::Rgb(40, 40, 40))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ⬢ {}", last_log),
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ⬢ ", Style::default().fg(Color::Rgb(40, 40, 40))),
            Span::styled(
                " [↑/↓] scroll ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(" | ", Style::default().fg(Color::Rgb(30, 30, 30))),
            Span::styled(
                " /help hints ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            ),
        ])
    };

    let footer_row = {
        let footer_row_width = bar_chunks[0].width.saturating_sub(6);
        if app.agent_running || app.thinking {
            let elapsed = if let Some(start) = app.task_start_time {
                format!(" {:0>2}s ", start.elapsed().as_secs())
            } else {
                String::new()
            };
            let last_log = app
                .specular_logs
                .last()
                .map(|s| s.as_str())
                .unwrap_or("...");
            let spinner = match app.tick_count % 8 {
                0 => "⠋",
                1 => "⠙",
                2 => "⠹",
                3 => "⠸",
                4 => "⠼",
                5 => "⠴",
                6 => "⠦",
                _ => "⠧",
            };
            let footer_caption = select_fitting_variant(
                &running_footer_variants(app, &elapsed, last_log),
                footer_row_width,
            );

            Line::from(vec![
                Span::styled(
                    format!(" {} ", spinner),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    elapsed,
                    Style::default()
                        .bg(Color::Rgb(40, 40, 40))
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" ⬢ {}", footer_caption),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        } else {
            let idle_hint = select_fitting_variant(&idle_footer_variants(app), footer_row_width);
            Line::from(vec![
                Span::styled(" ⬢ ", Style::default().fg(Color::Rgb(40, 40, 40))),
                Span::styled(
                    idle_hint,
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                ),
            ])
        }
    };

    let runtime_age = app.last_runtime_profile_time.elapsed();
    let provider_prefix = provider_badge_prefix(&app.provider_name);
    let issue = runtime_issue_kind(app);
    let (issue_code, issue_color) = runtime_issue_badge(issue);
    let (lm_label, lm_color) = if issue == RuntimeIssueKind::NoModel {
        (format!("{provider_prefix}:NONE"), Color::Red)
    } else if issue == RuntimeIssueKind::Booting {
        (format!("{provider_prefix}:BOOT"), Color::DarkGray)
    } else if issue == RuntimeIssueKind::Recovering {
        (format!("{provider_prefix}:RECV"), Color::Cyan)
    } else if matches!(
        issue,
        RuntimeIssueKind::Connectivity | RuntimeIssueKind::EmptyResponse
    ) {
        (format!("{provider_prefix}:WARN"), Color::Red)
    } else if issue == RuntimeIssueKind::ContextCeiling {
        (format!("{provider_prefix}:CEIL"), Color::Yellow)
    } else if runtime_age > std::time::Duration::from_secs(12) {
        (format!("{provider_prefix}:STALE"), Color::Yellow)
    } else {
        (format!("{provider_prefix}:LIVE"), Color::Green)
    };
    let compaction_percent = app.compaction_percent.min(100);
    let _compaction_label = if app.compaction_threshold_tokens == 0 {
        " CMP:  0%".to_string()
    } else {
        format!(" CMP:{:>3}%", compaction_percent)
    };
    let _compaction_color = if app.compaction_threshold_tokens == 0 {
        Color::DarkGray
    } else if compaction_percent >= 85 {
        Color::Red
    } else if compaction_percent >= 60 {
        Color::Yellow
    } else {
        Color::Green
    };
    let prompt_percent = app.prompt_pressure_percent.min(100);
    let _prompt_label = if app.prompt_estimated_total_tokens == 0 {
        " BUD:  0%".to_string()
    } else {
        format!(" BUD:{:>3}%", prompt_percent)
    };
    let _prompt_color = if app.prompt_estimated_total_tokens == 0 {
        Color::DarkGray
    } else if prompt_percent >= 85 {
        Color::Red
    } else if prompt_percent >= 60 {
        Color::Yellow
    } else {
        Color::Green
    };

    let _think_badge = match app.think_mode {
        Some(true) => " [THINK]",
        Some(false) => " [FAST]",
        None => "",
    };

    // ── VRAM gauge (live from nvidia-smi poller) ─────────────────────────────
    let vram_ratio = app.gpu_state.ratio();
    let vram_label = app.gpu_state.label();
    let gpu_name = app.gpu_state.gpu_name();

    let (vein_label, vein_color) = if app.vein_docs_only {
        let color = if app.vein_embedded_count > 0 {
            Color::Green
        } else if app.vein_file_count > 0 {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        ("VN:DOC", color)
    } else if app.vein_file_count == 0 {
        ("VN:--", Color::DarkGray)
    } else if app.vein_embedded_count > 0 {
        ("VN:SEM", Color::Green)
    } else {
        ("VN:FTS", Color::Yellow)
    };

    let char_count: usize = app.messages_raw.iter().map(|(_, c)| c.len()).sum();
    let est_tokens = char_count / 3;
    let current_tokens = if app.total_tokens > 0 {
        app.total_tokens
    } else {
        est_tokens
    };
    let session_usage_text = format!(
        " TOKENS: {:0>5} | TOTAL: ${:.2} ",
        current_tokens, app.current_session_cost
    );

    // ── Single Liquid Status Bar ──────────────────────────────────────────
    f.render_widget(Clear, bar_chunks[0]);

    let usage_color = Color::Rgb(100, 100, 100);
    let ai_line = vec![
        Span::styled(
            format!(" {} ", lm_label),
            Style::default().fg(lm_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled("║ ", Style::default().fg(Color::Rgb(60, 60, 60))),
        Span::styled(format!("{} ", vein_label), Style::default().fg(vein_color)),
        Span::styled("│ ", Style::default().fg(Color::Rgb(40, 40, 40))),
        Span::styled(format!("{} ", issue_code), Style::default().fg(issue_color)),
        Span::styled("│ ", Style::default().fg(Color::Rgb(40, 40, 40))),
        Span::styled(
            format!("CTX:{} ", app.context_length),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("│ ", Style::default().fg(Color::Rgb(40, 40, 40))),
        Span::styled(
            format!("REMOTE:{} ", app.git_state.label()),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("│ ", Style::default().fg(Color::Rgb(40, 40, 40))),
        Span::styled(
            format!("BUD:{} CMP:{} ", prompt_percent, compaction_percent),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("│ ", Style::default().fg(Color::Rgb(40, 40, 40))),
        Span::styled(session_usage_text, Style::default().fg(usage_color)),
    ];

    let hardware_line = vec![
        Span::styled("   ⬢ ", Style::default().fg(Color::Rgb(60, 60, 60))), // Gray tint
        Span::styled(
            format!("{} ", gpu_name),
            Style::default()
                .fg(Color::Rgb(200, 200, 200))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("║ ", Style::default().fg(Color::Rgb(60, 60, 60))),
        Span::styled(
            format!(
                "[{}] ",
                make_animated_sparkline_gauge(vram_ratio, 12, app.tick_count)
            ),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!("{}% ", (vram_ratio * 100.0) as u8),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!("({})", vram_label),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        ),
    ];

    f.render_widget(
        Paragraph::new(vec![
            Line::from(ai_line),
            Line::from(hardware_line),
            footer_row,
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(60, 60, 60))),
        ),
        bar_chunks[0],
    );

    // ── Box 4: Input ──────────────────────────────────────────────────────────
    let input_border_color = if app.agent_running {
        Color::Rgb(60, 60, 60)
    } else {
        Color::Rgb(100, 100, 100) // High-focus gray glow
    };
    let input_rect = chunks[1];
    let title_area = input_title_area(input_rect);
    let input_hint = render_input_title(app, title_area);
    let input_block = Block::default()
        .title(input_hint)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(input_border_color))
        .style(Style::default().bg(Color::Rgb(25, 25, 25))); // Obsidian Dark Gray

    let inner_area = input_block.inner(input_rect);
    f.render_widget(Clear, input_rect);
    f.render_widget(input_block, input_rect);

    f.render_widget(
        Paragraph::new(app.input.as_str()).wrap(Wrap { trim: true }),
        inner_area,
    );

    // Hardware Cursor (Managed by terminal emulator for smooth asynchronous blink)
    // Hardware Cursor (Managed by terminal emulator for smooth asynchronous blink)
    // Always call set_cursor during standard operation to "park" the cursor safely in the input box,
    // preventing it from jittering to (0,0) (the top-left title) during modal reviews.
    if !app.agent_running && inner_area.height > 0 {
        let text_w = app.input.len() as u16;
        let max_w = inner_area.width.saturating_sub(1);
        let cursor_x = inner_area.x + text_w.min(max_w);
        f.set_cursor(cursor_x, inner_area.y);
    }

    // ── High-risk approval modal ───────────────────────────────────────────────
    if let Some(approval) = &app.awaiting_approval {
        let is_diff_preview = approval.diff.is_some();

        // Taller modal for diff preview so more lines are visible.
        let modal_h = if is_diff_preview { 70 } else { 50 };
        let area = centered_rect(80, modal_h, f.size());
        f.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Header: Title + Instructions
                Constraint::Min(0),    // Body: Tool + diff/command
            ])
            .split(area);

        // ── Modal Header ─────────────────────────────────────────────────────
        let (title_str, title_color) = if let Some(_) = &approval.mutation_label {
            (" MUTATION REQUESTED — AUTHORISE THE WORKFLOW ", Color::Cyan)
        } else if is_diff_preview {
            (" DIFF PREVIEW — REVIEW BEFORE APPLYING ", Color::Yellow)
        } else {
            (" HIGH-RISK OPERATION REQUESTED ", Color::Red)
        };
        let header_text = vec![
            Line::from(Span::styled(
                title_str,
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )),
            if is_diff_preview {
                Line::from(Span::styled(
                    "  [↑↓/jk/PgUp/PgDn] Scroll   [Y] Apply   [N] Skip   [A] Accept All ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(vec![
                    Span::styled(
                        "  [Y] Approve  ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "  [N] Decline  ",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "  [A] Accept All ",
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            },
        ];
        f.render_widget(
            Paragraph::new(header_text)
                .block(
                    Block::default()
                        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                        .border_style(Style::default().fg(title_color)),
                )
                .alignment(ratatui::layout::Alignment::Center),
            chunks[0],
        );

        // ── Modal Body ───────────────────────────────────────────────────────
        let border_color = if let Some(_) = &approval.mutation_label {
            Color::Cyan
        } else if is_diff_preview {
            Color::Yellow
        } else {
            Color::Red
        };
        if let Some(diff_text) = &approval.diff {
            // Render colored diff lines
            let added = diff_text.lines().filter(|l| l.starts_with("+ ")).count();
            let removed = diff_text.lines().filter(|l| l.starts_with("- ")).count();
            let mut body_lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    if let Some(label) = &approval.mutation_label {
                        format!(" INTENT: {}", label)
                    } else {
                        format!(" {}", approval.display)
                    },
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled(
                        format!(" +{}", added),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" -{}", removed),
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(Span::raw("")),
            ];
            for raw_line in diff_text.lines() {
                let styled = if raw_line.starts_with("+ ") {
                    Line::from(Span::styled(
                        format!(" {}", raw_line),
                        Style::default().fg(Color::Green),
                    ))
                } else if raw_line.starts_with("- ") {
                    Line::from(Span::styled(
                        format!(" {}", raw_line),
                        Style::default().fg(Color::Red),
                    ))
                } else if raw_line.starts_with("---") || raw_line.starts_with("@@ ") {
                    Line::from(Span::styled(
                        format!(" {}", raw_line),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(Span::raw(format!(" {}", raw_line)))
                };
                body_lines.push(styled);
            }
            f.render_widget(
                Paragraph::new(body_lines)
                    .block(
                        Block::default()
                            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                            .border_style(Style::default().fg(border_color)),
                    )
                    .scroll((approval.diff_scroll, 0)),
                chunks[1],
            );
        } else {
            let body_text = vec![
                Line::from(Span::raw("")),
                Line::from(Span::styled(
                    if let Some(label) = &approval.mutation_label {
                        format!(" INTENT: {}", label)
                    } else {
                        format!(" ACTION: {}", approval.display)
                    },
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::raw("")),
                Line::from(Span::styled(
                    format!("  Tool: {}", approval.tool_name),
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            if approval.mutation_label.is_some() {
                // For mutations, show the original display (e.g. path) as extra info
            }
            f.render_widget(
                Paragraph::new(body_text)
                    .block(
                        Block::default()
                            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                            .border_style(Style::default().fg(border_color)),
                    )
                    .alignment(ratatui::layout::Alignment::Center),
                chunks[1],
            );
        }
    }

    // ── Swarm diff review modal ────────────────────────────────────────────────
    if let Some(review) = &app.active_review {
        draw_diff_review(f, review);
    }

    // ── Autocomplete Hatch (Floating Popup) ──────────────────────────────────
    if app.show_autocomplete && !app.autocomplete_suggestions.is_empty() {
        let area = Rect {
            x: chunks[1].x + 2,
            y: chunks[1]
                .y
                .saturating_sub(app.autocomplete_suggestions.len() as u16 + 2),
            width: chunks[1].width.saturating_sub(4),
            height: app.autocomplete_suggestions.len() as u16 + 2,
        };
        f.render_widget(Clear, area);

        let items: Vec<ListItem> = app
            .autocomplete_suggestions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let style = if i == app.selected_suggestion {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                ListItem::new(format!(" 📄 {}", s)).style(style)
            })
            .collect();

        let hatch = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(
                    " @ RESOLVER (Matching: {}) ",
                    app.autocomplete_filter
                )),
        );
        f.render_widget(hatch, area);

        // Optional "More matches..." indicator
        if app.autocomplete_suggestions.len() >= 15 {
            let more_area = Rect {
                x: area.x + 2,
                y: area.y + area.height - 1,
                width: 20,
                height: 1,
            };
            f.render_widget(
                Paragraph::new("... (type to narrow) ").style(Style::default().fg(Color::DarkGray)),
                more_area,
            );
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}

fn strip_ghost_prefix(s: &str) -> &str {
    for prefix in &[
        "Hematite: ",
        "HEMATITE: ",
        "Assistant: ",
        "assistant: ",
        "Okay, ",
        "Hmm, ",
        "Wait, ",
        "Alright, ",
        "Got it, ",
        "Certainly, ",
        "Sure, ",
        "Understood, ",
    ] {
        if s.to_lowercase().starts_with(&prefix.to_lowercase()) {
            return &s[prefix.len()..];
        }
    }
    s
}

fn first_n_chars(s: &str, n: usize) -> String {
    let mut result = String::new();
    let mut count = 0;
    for c in s.chars() {
        if count >= n {
            result.push('…');
            break;
        }
        if c == '\n' || c == '\r' {
            result.push(' ');
        } else if !c.is_control() {
            result.push(c);
        }
        count += 1;
    }
    result
}

fn trim_vec_context(v: &mut Vec<ContextFile>, max: usize) {
    while v.len() > max {
        v.remove(0);
    }
}

fn trim_vec(v: &mut Vec<String>, max: usize) {
    while v.len() > max {
        v.remove(0);
    }
}

/// Minimal markdown → ratatui spans for the SPECULAR panel.
/// Handles: # headers, **bold**, `code`, - bullet, > blockquote, plain text.
fn render_markdown_line(raw: &str) -> Vec<Line<'static>> {
    // 1. Strip ANSI and control noise first to verify content.
    let cleaned_ansi = strip_ansi(raw);
    let trimmed = cleaned_ansi.trim();
    if trimmed.is_empty() {
        return vec![Line::raw("")];
    }

    // 2. Strip thought tags.
    let cleaned_owned = trimmed
        .replace("<thought>", "")
        .replace("</thought>", "")
        .replace("<think>", "")
        .replace("</think>", "");
    let trimmed = cleaned_owned.trim();
    if trimmed.is_empty() {
        return vec![];
    }

    // # Heading (all levels → bold white)
    for (prefix, indent) in &[("### ", "  "), ("## ", " "), ("# ", "")] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return vec![Line::from(vec![Span::styled(
                format!("{}{}", indent, rest),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )])];
        }
    }

    // > blockquote
    if let Some(rest) = trimmed
        .strip_prefix("> ")
        .or_else(|| trimmed.strip_prefix(">"))
    {
        return vec![Line::from(vec![
            Span::styled("| ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                rest.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::DIM),
            ),
        ])];
    }

    // - / * bullet
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let rest = &trimmed[2..];
        let mut spans = vec![Span::styled("* ", Style::default().fg(Color::Gray))];
        spans.extend(inline_markdown(rest));
        return vec![Line::from(spans)];
    }

    // Plain line with possible inline markdown
    let spans = inline_markdown(trimmed);
    vec![Line::from(spans)]
}

/// Inline markdown for The Core chat window (brighter palette than SPECULAR).
fn inline_markdown_core(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if let Some(start) = remaining.find("**") {
            let before = &remaining[..start];
            if !before.is_empty() {
                spans.push(Span::raw(before.to_string()));
            }
            let after_open = &remaining[start + 2..];
            if let Some(end) = after_open.find("**") {
                spans.push(Span::styled(
                    after_open[..end].to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
                remaining = &after_open[end + 2..];
                continue;
            }
        }
        if let Some(start) = remaining.find('`') {
            let before = &remaining[..start];
            if !before.is_empty() {
                spans.push(Span::raw(before.to_string()));
            }
            let after_open = &remaining[start + 1..];
            if let Some(end) = after_open.find('`') {
                spans.push(Span::styled(
                    after_open[..end].to_string(),
                    Style::default().fg(Color::Yellow),
                ));
                remaining = &after_open[end + 1..];
                continue;
            }
        }
        spans.push(Span::raw(remaining.to_string()));
        break;
    }
    spans
}

/// Parse inline `**bold**` and `` `code` `` — shared by SPECULAR and Core renderers.
fn inline_markdown(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if let Some(start) = remaining.find("**") {
            let before = &remaining[..start];
            if !before.is_empty() {
                spans.push(Span::raw(before.to_string()));
            }
            let after_open = &remaining[start + 2..];
            if let Some(end) = after_open.find("**") {
                spans.push(Span::styled(
                    after_open[..end].to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
                remaining = &after_open[end + 2..];
                continue;
            }
        }
        if let Some(start) = remaining.find('`') {
            let before = &remaining[..start];
            if !before.is_empty() {
                spans.push(Span::raw(before.to_string()));
            }
            let after_open = &remaining[start + 1..];
            if let Some(end) = after_open.find('`') {
                spans.push(Span::styled(
                    after_open[..end].to_string(),
                    Style::default().fg(Color::Yellow),
                ));
                remaining = &after_open[end + 1..];
                continue;
            }
        }
        spans.push(Span::raw(remaining.to_string()));
        break;
    }
    spans
}

// ── Splash Screen ─────────────────────────────────────────────────────────────

fn draw_splash<B: Backend>(terminal: &mut Terminal<B>) -> Result<(), Box<dyn std::error::Error>> {
    let rust_color = Color::Rgb(110, 110, 110);

    let logo_lines = vec![
        "██╗  ██╗███████╗███╗   ███╗ █████╗ ████████╗██╗████████╗███████╗",
        "██║  ██║██╔════╝████╗ ████║██╔══██╗╚══██╔══╝██║╚══██╔══╝██╔════╝",
        "███████║█████╗  ██╔████╔██║███████║   ██║   ██║   ██║   █████╗  ",
        "██╔══██║██╔══╝  ██║╚██╔╝██║██╔══██║   ██║   ██║   ██║   ██╔══╝  ",
        "██║  ██║███████╗██║ ╚═╝ ██║██║  ██║   ██║   ██║   ██║   ███████╗",
        "╚═╝  ╚═╝╚══════╝╚═╝     ╚═╝╚═╝  ╚═╝   ╚═╝   ╚═╝   ╚═╝   ╚══════╝",
    ];

    let version = env!("CARGO_PKG_VERSION");

    terminal.draw(|f| {
        let area = f.size();

        // Clear with a dark background
        f.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
            area,
        );

        // Total content height: logo(6) + spacer(1) + version(1) + tagline(1) + author(1) + spacer(2) + prompt(1) = 13
        let content_height: u16 = 13;
        let top_pad = area.height.saturating_sub(content_height) / 2;

        let mut lines: Vec<Line<'static>> = Vec::new();

        // Top padding
        for _ in 0..top_pad {
            lines.push(Line::raw(""));
        }

        // Logo lines — centered horizontally
        for logo_line in &logo_lines {
            lines.push(Line::from(Span::styled(
                logo_line.to_string(),
                Style::default().fg(rust_color).add_modifier(Modifier::BOLD),
            )));
        }

        // Spacer
        lines.push(Line::raw(""));

        // Version
        lines.push(Line::from(vec![Span::styled(
            format!("v{}", version),
            Style::default().fg(Color::DarkGray),
        )]));

        // Tagline
        lines.push(Line::from(vec![Span::styled(
            "Local AI coding harness + workstation assistant",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        )]));

        // Developer credit
        lines.push(Line::from(vec![Span::styled(
            "Developed by Ocean Bennett",
            Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
        )]));

        // Spacer
        lines.push(Line::raw(""));
        lines.push(Line::raw(""));

        // Prompt
        lines.push(Line::from(vec![
            Span::styled("[ ", Style::default().fg(rust_color)),
            Span::styled(
                "Press ENTER to start",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ]", Style::default().fg(rust_color)),
        ]));

        let splash = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);

        f.render_widget(splash, area);
    })?;

    Ok(())
}

fn normalize_id(id: &str) -> String {
    id.trim().to_uppercase()
}

fn filter_tui_noise(text: &str) -> String {
    // 1. First Pass: Strip ANSI escape codes that cause "shattering" in layout.
    let cleaned = strip_ansi(text);

    // 2. Second Pass: Filter heuristic noise.
    let mut lines = Vec::new();
    for line in cleaned.lines() {
        // Strip multi-line "LF replaced by CRLF" noise frequently emitted by git/shell on Windows.
        if CRLF_REGEX.is_match(line) {
            continue;
        }
        // Strip git checkout/file update noise if it's too repetitive.
        if line.contains("Updating files:") && line.contains("%") {
            continue;
        }
        // Strip random terminal control characters that might have escaped.
        let sanitized: String = line
            .chars()
            .filter(|c| !c.is_control() || *c == '\t')
            .collect();
        if sanitized.trim().is_empty() && !line.trim().is_empty() {
            continue;
        }

        lines.push(normalize_tui_text(&sanitized));
    }
    lines.join("\n").trim().to_string()
}

fn normalize_tui_text(text: &str) -> String {
    let mut normalized = text
        .replace("ΓÇö", "-")
        .replace("ΓÇô", "-")
        .replace("ΓÇª", "...")
        .replace("Γ£à", "[OK]")
        .replace("≡ƒ¢á∩╕Å", "")
        .replace("—", "-")
        .replace("–", "-")
        .replace("…", "...")
        .replace("•", "*")
        .replace("✅", "[OK]")
        .replace("🚨", "[!]");

    normalized = normalized
        .chars()
        .map(|c| match c {
            '\u{00A0}' => ' ',
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            c if c.is_ascii() || c == '\n' || c == '\t' => c,
            _ => ' ',
        })
        .collect();

    let mut compacted = String::with_capacity(normalized.len());
    let mut prev_space = false;
    for ch in normalized.chars() {
        if ch == ' ' {
            if !prev_space {
                compacted.push(ch);
            }
            prev_space = true;
        } else {
            compacted.push(ch);
            prev_space = false;
        }
    }

    compacted.trim().to_string()
}
