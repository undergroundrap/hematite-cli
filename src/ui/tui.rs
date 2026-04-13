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
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Terminal,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc::Receiver;
use walkdir::WalkDir;

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
    }

    pub fn clear_pending_attachments(&mut self) {
        self.attached_context = None;
        self.attached_image = None;
    }

    pub fn push_message(&mut self, speaker: &str, content: &str) {
        let filtered = filter_tui_noise(content);
        if filtered.is_empty() && !content.is_empty() {
            return;
        } // Completely suppressed noise

        self.messages_raw.push((speaker.to_string(), filtered));
        // Cap raw history to prevent UI lag.
        if self.messages_raw.len() > 100 {
            self.messages_raw.remove(0);
        }
        self.rebuild_formatted_messages();
        // Cap visual history.
        if self.messages.len() > 250 {
            let to_drain = self.messages.len() - 250;
            self.messages.drain(0..to_drain);
        }
    }

    pub fn update_last_message(&mut self, token: &str) {
        if let Some(last_raw) = self.messages_raw.last_mut() {
            if last_raw.0 == "Hematite" {
                last_raw.1.push_str(token);
                // Optimization: Only rebuild formatting on whitespace/newline or if specifically requested.
                // This prevents "shattering" the TUI during high-speed token streams.
                if token.contains(' ')
                    || token.contains('\n')
                    || token.contains('.')
                    || token.len() > 5
                {
                    self.rebuild_formatted_messages();
                }
            }
        }
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

    fn format_message(&self, speaker: &str, content: &str, _is_last: bool) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        // Hematite = rust iron-oxide brown; You = green; Tool = cyan
        let rust = Color::Rgb(180, 90, 50);
        let style = match speaker {
            "You" => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            "Hematite" => Style::default().fg(rust).add_modifier(Modifier::BOLD),
            "Tool" => Style::default().fg(Color::Cyan),
            _ => Style::default().fg(Color::DarkGray),
        };

        // Aggressive trim to avoid leading/trailing blank rows.
        let cleaned = crate::agent::inference::strip_think_blocks(content)
            .trim()
            .to_string();
        let cleaned = strip_ghost_prefix(&cleaned);

        let mut is_first = true;
        for raw_line in cleaned.lines() {
            // SPACING FIX:
            // If we have a sequence of blank lines, don't label them with "  ".
            // Only add labels to lines that have content OR are the very first line of the message.
            if !is_first && raw_line.trim().is_empty() {
                lines.push(Line::raw(""));
                continue;
            }

            let label = if is_first {
                format!("{}: ", speaker)
            } else {
                "  ".to_string()
            };

            // System messages with "+N -N" stat tokens get inline green/red coloring.
            if speaker == "System" && (raw_line.contains(" +") || raw_line.contains(" -")) {
                let mut spans: Vec<Span<'static>> =
                    vec![Span::raw(" "), Span::styled(label, style)];
                // Tokenise on whitespace, colouring +digits green, -digits red,
                // and file paths (containing '/' or '.') bright white.
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
                let line_style = if raw_line.starts_with("-") {
                    Style::default().fg(Color::Red)
                } else if raw_line.starts_with("+") {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::DIM)
                };
                lines.push(Line::from(vec![
                    Span::raw("    "), // Deeper indent for diffs
                    Span::styled(raw_line.to_string(), line_style),
                ]));
            } else {
                let mut spans = vec![Span::raw(" "), Span::styled(label, style)];
                // Render inline markdown for Hematite responses; plain text for others.
                // Code fence lines (``` or ```rust etc.) are rendered as plain dim text
                // rather than passed through inline_markdown_core, which would misparse
                // the backticks as inline code spans and garble the layout.
                if speaker == "Hematite" {
                    if raw_line.trim_start().starts_with("```") {
                        spans.push(Span::styled(
                            raw_line.to_string(),
                            Style::default().fg(Color::DarkGray),
                        ));
                    } else {
                        spans.extend(inline_markdown_core(raw_line));
                    }
                } else {
                    spans.push(Span::raw(raw_line.to_string()));
                }
                lines.push(Line::from(spans));
            }
            is_first = false;
        }

        lines
    }

    /// [Intelli-Hematite] Live scan of the workspace to populate autocomplete.
    /// Excludes common noisy directories like target, node_modules, .git.
    pub fn update_autocomplete(&mut self) {
        let root = crate::tools::file_ops::workspace_root();
        // Extract the fragment after the last '@'
        let query = if let Some(pos) = self.input.rfind('@') {
            &self.input[pos + 1..]
        } else {
            ""
        }
        .to_lowercase();

        self.autocomplete_filter = query.clone();

        let mut matches = Vec::new();
        let mut total_found = 0;

        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "target" && name != "node_modules"
            })
            .flatten()
        {
            if entry.file_type().is_file() {
                let path = entry.path().strip_prefix(&root).unwrap_or(entry.path());
                let path_str = path.to_string_lossy().to_string();
                if path_str.to_lowercase().contains(&query) {
                    total_found += 1;
                    if matches.len() < 15 {
                        // Show up to 15 at once
                        matches.push(path_str);
                    }
                }
            }
            if total_found > 100 {
                break;
            } // Safety cap for massive repos
        }

        // Prioritize: Move .rs and .md files to the top if they match
        matches.sort_by(|a, b| {
            let a_ext = a.split('.').last().unwrap_or("");
            let b_ext = b.split('.').last().unwrap_or("");
            let a_is_src = a_ext == "rs" || a_ext == "md";
            let b_is_src = b_ext == "rs" || b_ext == "md";
            b_is_src.cmp(&a_is_src)
        });

        self.autocomplete_suggestions = matches;
        self.selected_suggestion = self
            .selected_suggestion
            .min(self.autocomplete_suggestions.len().saturating_sub(1));
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
        let root = crate::tools::file_ops::workspace_root();
        let plan_path = root.join(".hematite").join("PLAN.md");
        if plan_path.exists() {
            if let Some(plan) = crate::tools::plan::load_plan_handoff() {
                if plan.has_signal() && !plan.goal.trim().is_empty() {
                    self.current_objective = plan.summary_line();
                    return;
                }
            }
        }
        let path = root.join(".hematite").join("TASK.md");
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
        let report_dir = std::path::PathBuf::from(".hematite/reports");
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

    pub fn copy_transcript_to_clipboard(&self) {
        let mut history = self
            .messages_raw
            .iter()
            .map(|m| format!("[{}] {}\n", m.0, m.1))
            .collect::<String>();

        history.push_str("\nSession Stats\n");
        history.push_str(&format!("Tokens: {}\n", self.total_tokens));
        history.push_str(&format!("Cost: ${:.4}\n", self.current_session_cost));

        copy_text_to_clipboard(&history);
    }

    pub fn copy_clean_transcript_to_clipboard(&self) {
        let mut history = self
            .messages_raw
            .iter()
            .filter(|(speaker, content)| !should_skip_transcript_copy_entry(speaker, content))
            .map(|m| format!("[{}] {}\n", m.0, m.1))
            .collect::<String>();

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

fn should_skip_transcript_copy_entry(speaker: &str, content: &str) -> bool {
    if speaker != "System" {
        return false;
    }

    content.starts_with("Hematite Commands:\n")
        || content.starts_with("Document note: `/attach`")
        || content == "Chat transcript copied to clipboard."
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
            Constraint::Length(3),
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

fn input_status_text(app: &App) -> String {
    let voice_status = if app.voice_manager.is_enabled() {
        "ON"
    } else {
        "OFF"
    };
    let approvals_status = if app.yolo_mode { "OFF" } else { "ON" };
    let doc_status = if app.attached_context.is_some() {
        "DOC"
    } else {
        "--"
    };
    let image_status = if app.attached_image.is_some() {
        "IMG"
    } else {
        "--"
    };
    if app.agent_running {
        format!(
            "pending:{}:{} | voice:{}",
            doc_status, image_status, voice_status
        )
    } else {
        format!(
            "pending:{}:{} | voice:{} | appr:{} | Len:{}",
            doc_status,
            image_status,
            voice_status,
            approvals_status,
            app.input.len()
        )
    }
}

fn visible_input_actions_for_title(app: &App, title_area: Rect) -> Vec<InputActionVisual> {
    let reserved = input_status_text(app).chars().count() as u16 + 3;
    let max_width = title_area.width.saturating_sub(reserved);
    visible_input_actions(app, max_width)
}

fn input_action_hitboxes(app: &App, title_area: Rect) -> Vec<(InputAction, u16, u16)> {
    let mut x = title_area.x;
    let mut out = Vec::new();
    for action in visible_input_actions_for_title(app, title_area) {
        let chip_width = action.label.chars().count() as u16 + 2;
        out.push((action.action, x, x + chip_width.saturating_sub(1)));
        x = x.saturating_add(chip_width + 1);
    }
    out
}

fn render_input_title(app: &App, title_area: Rect) -> Line<'static> {
    let mut spans = Vec::new();
    let actions = visible_input_actions_for_title(app, title_area);
    for (idx, action) in actions.into_iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw(" "));
        }
        let style = if app.hovered_input_action == Some(action.action) {
            action
                .style
                .bg(Color::Rgb(85, 48, 26))
                .add_modifier(Modifier::REVERSED)
        } else {
            action.style
        };
        spans.push(Span::styled(format!("[{}]", action.label), style));
    }
    let status = input_status_text(app);
    if !spans.is_empty() {
        spans.push(Span::raw(" | "));
    }
    spans.push(Span::styled(status, Style::default().fg(Color::DarkGray)));
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
    app.clear_pending_attachments();
    app.current_objective = "Idle".into();
}

fn request_stop(app: &mut App) {
    app.voice_manager.stop();
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
         /ask [prompt]     - (Flow) Read-only analysis mode; optional inline prompt\n\
         /code [prompt]    - (Flow) Explicit implementation mode; optional inline prompt\n\
         /architect [prompt] - (Flow) Plan-first mode; optional inline prompt\n\
         /implement-plan   - (Flow) Execute the saved architect handoff in /code\n\
         /read-only [prompt] - (Flow) Hard read-only mode; optional inline prompt\n\
           /new              - (Reset) Fresh task context; clear chat, pins, and task files\n\
           /forget           - (Wipe) Hard forget; purge saved memory and Vein index too\n\
         /vein-inspect     - (Vein) Inspect indexed memory, hot files, and active room bias\n\
         /workspace-profile - (Profile) Show the auto-generated workspace profile\n\
         /version          - (Build) Show the running Hematite version\n\
         /about            - (Info) Show author, repo, and product info\n\
         /vein-reset       - (Vein) Wipe the RAG index; rebuilds automatically on next turn\n\
           /clear            - (UI) Clear dialogue display only\n\
         /gemma-native [auto|on|off|status] - (Model) Auto/force/disable Gemma 4 native formatting\n\
         /runtime-refresh  - (Model) Re-read LM Studio model + CTX now\n\
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
         /attach <path>    - (Docs) Attach a PDF/markdown/txt file for next message (PDF best-effort)\n\
         /attach-pick      - (Docs) Open a file picker and attach a document\n\
         /image <path>     - (Vision) Attach an image for the next message\n\
         /image-pick       - (Vision) Open a file picker and attach an image\n\
         /detach           - (Context) Drop pending document/image attachments\n\
         /copy             - (Debug) Copy exact session transcript (includes help/system output)\n\
         /copy-last        - (Debug) Copy the latest Hematite reply only\n\
         /copy-clean       - (Debug) Copy chat transcript without help/debug boilerplate\n\
         /copy2            - (Debug) Copy SPECULAR log to clipboard (reasoning + events)\n\
         \nHotkeys:\n\
         Ctrl+B - Toggle Brief Mode (minimal output)\n\
         Ctrl+P - Toggle Professional Mode (strip personality)\n\
         Ctrl+O - Open document picker for next-turn context\n\
         Ctrl+I - Open image picker for next-turn vision context\n\
         Ctrl+Y - Toggle Approvals Off (bypass safety approvals)\n\
         Ctrl+S - Quick Swarm (hardcoded bootstrap)\n\
         Ctrl+Z - Undo last edit\n\
         Ctrl+Q/C - Quit session\n\
         ESC    - Silence current playback\n\
         \nStatus Legend:\n\
         LM    - LM Studio runtime health (`LIVE`, `RECV`, `WARN`, `CEIL`, `STALE`, `BOOT`)\n\
         VN    - Vein RAG status (`SEM`=semantic active, `FTS`=BM25 only, `--`=not indexed)\n\
         BUD   - Total prompt-budget pressure against the live context window\n\
         CMP   - History compaction pressure against Hematite's adaptive threshold\n\
         ERR   - Session error count (runtime, tool, or SPECULAR failures)\n\
         CTX   - Live context window currently reported by LM Studio\n\
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
           /new              — (Reset) Fresh task context; clear chat, pins, and task files\n\
           /forget           — (Wipe) Hard forget; purge saved memory and Vein index too\n\
           /vein-inspect     — (Vein) Inspect indexed memory, hot files, and active room bias\n\
           /workspace-profile — (Profile) Show the auto-generated workspace profile\n\
           /version          — (Build) Show the running Hematite version\n\
           /about            — (Info) Show author, repo, and product info\n\
           /vein-reset       — (Vein) Wipe the RAG index; rebuilds automatically on next turn\n\
           /clear            — (UI) Clear dialogue display only\n\
         /gemma-native [auto|on|off|status] — (Model) Auto/force/disable Gemma 4 native formatting\n\
         /runtime-refresh  — (Model) Re-read LM Studio model + CTX now\n\
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
         /attach <path>    — (Docs) Attach a PDF/markdown/txt file for next message\n\
         /attach-pick      — (Docs) Open a file picker and attach a document\n\
         /image <path>     — (Vision) Attach an image for the next message\n\
         /image-pick       — (Vision) Open a file picker and attach an image\n\
         /detach           — (Context) Drop pending document/image attachments\n\
         /copy             — (Debug) Copy session transcript to clipboard\n\
         /copy2            — (Debug) Copy SPECULAR log to clipboard (reasoning + events)\n\
         \nHotkeys:\n\
         Ctrl+B — Toggle Brief Mode (minimal output)\n\
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
         BUD   — Total prompt-budget pressure against the live context window\n\
         CMP   — History compaction pressure against Hematite's adaptive threshold\n\
         ERR   — Session error count (runtime, tool, or SPECULAR failures)\n\
         CTX   — Live context window currently reported by LM Studio\n\
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
        voice_loading_progress: 0.0,
        hardware_guard_enabled: true,
        session_start: std::time::SystemTime::now(),
        soul_name: soul.species.clone(),
        attached_context: None,
        attached_image: None,
        hovered_input_action: None,
    };

    // Initial placeholder — streaming will overwrite this with hardware diagnostics
    app.push_message("Hematite", "Initialising Engine & Hardware...");

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
                                }
                            }
                            MouseEventKind::ScrollUp => {
                                if is_right_side {
                                    // User scrolled up — disable auto-scroll so they can read.
                                    app.specular_auto_scroll = false;
                                    app.specular_scroll = app.specular_scroll.saturating_sub(3);
                                } else {
                                    let cur = app.manual_scroll_offset.unwrap_or(0);
                                    app.manual_scroll_offset = Some(cur.saturating_add(3));
                                }
                            }
                            MouseEventKind::ScrollDown => {
                                if is_right_side {
                                    app.specular_auto_scroll = false;
                                    app.specular_scroll = app.specular_scroll.saturating_add(3);
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
                                    let selected = &app.autocomplete_suggestions[app.selected_suggestion];
                                    if let Some(pos) = app.input.rfind('@') {
                                        app.input.truncate(pos + 1);
                                        app.input.push_str(selected);
                                        app.show_autocomplete = false;
                                    }
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
                                if app.show_autocomplete && !app.autocomplete_suggestions.is_empty() {
                                    let selected = &app.autocomplete_suggestions[app.selected_suggestion];
                                    if let Some(pos) = app.input.rfind('@') {
                                        app.input.truncate(pos + 1);
                                        app.input.push_str(selected);
                                        app.show_autocomplete = false;
                                        continue;
                                    }
                                }

                                if !app.input.is_empty() && !app.agent_running {
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
                                                let gemma_detected = crate::agent::inference::is_gemma4_model_name(&app.model_id);
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
                                            "/ask" | "/code" | "/architect" | "/read-only" | "/auto" => {
                                                let label = match cmd.as_str() {
                                                    "/ask" => "ASK",
                                                    "/code" => "CODE",
                                                    "/architect" => "ARCHITECT",
                                                    "/read-only" => "READ-ONLY",
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
                                                       /new              — (Reset) Fresh task context; clear chat, pins, and task files\n\
                                                       /forget           — (Wipe) Hard forget; purge saved memory and Vein index too\n\
                                                       /vein-inspect     — (Vein) Inspect indexed memory, hot files, and active room bias\n\
                                                       /workspace-profile — (Profile) Show the auto-generated workspace profile\n\
                                                       /version          — (Build) Show the running Hematite version\n\
                                                       /about            — (Info) Show author, repo, and product info\n\
                                                       /vein-reset       — (Vein) Wipe the RAG index; rebuilds automatically on next turn\n\
                                                       /clear            — (UI) Clear dialogue display only\n\
                                                     /gemma-native [auto|on|off|status] — (Model) Auto/force/disable Gemma 4 native formatting\n\
                                                     /runtime-refresh  — (Model) Re-read LM Studio model + CTX now\n\
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
                                                     Ctrl+B — Toggle Brief Mode (minimal output)\n\
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
                                    app.push_message("You", &input_text);
                                    app.agent_running = true;
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
                        app.thinking = true;
                        app.current_thought.push_str(&content);
                    }
                    InferenceEvent::VoiceStatus(msg) => {
                        app.push_message("System", &msg);
                    }
                    InferenceEvent::Token(ref token) | InferenceEvent::MutedToken(ref token) => {
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
                    InferenceEvent::ToolCallStart { name, args, .. } => {
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
                    InferenceEvent::ToolCallResult { id: _, name, output, is_error } => {
                        let icon = if is_error { "[x]" } else { "[v]" };
                        if is_error {
                            app.record_error();
                        }
                        // In chat mode, suppress tool results from main chat.
                        // Errors still show so the user knows something went wrong.
                        let preview = first_n_chars(&output, 100);
                        if app.workflow_mode != "CHAT" {
                            app.push_message("Tool", &format!("{}  {} → {}", icon, name, preview));
                        } else if is_error {
                            app.push_message("System", &format!("Tool error: {}", preview));
                        }

                        // If it was a read or write, we can extract the path from the app.active_context "Running" entries
                        // but it's simpler to just let Specular handle the indexing or update here if we had the path.

                        // Remove "Running" tools from context list
                        app.active_context.retain(|f| f.path != name || f.status != "Running");
                        app.manual_scroll_offset = None;
                    }
                    InferenceEvent::ApprovalRequired { id: _, name, display, diff, responder } => {
                        let is_diff = diff.is_some();
                        app.awaiting_approval = Some(PendingApproval {
                            display: display.clone(),
                            tool_name: name,
                            diff,
                            diff_scroll: 0,
                            responder,
                        });
                        if is_diff {
                            app.push_message("System", "[~]  Diff preview — [Y] Apply  [N] Skip");
                        } else {
                            app.push_message("System", "[!]  Approval required (Press [Y] Approve or [N] Decline)");
                            app.push_message("System", &format!("Command: {}", display));
                        }
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
                        if app.voice_manager.is_enabled() {
                            app.voice_manager.flush();
                        }
                        if !app.current_thought.is_empty() {
                            app.last_reasoning = app.current_thought.clone();
                        }
                        app.current_thought.clear();
                        app.specular_auto_scroll = true;
                        // Clear single-agent task bars on completion
                        app.active_workers.remove("AGENT");
                        app.worker_labels.remove("AGENT");
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
                    InferenceEvent::RuntimeProfile { model_id, context_length } => {
                        let was_no_model = app.model_id == "no model loaded";
                        let now_no_model = model_id == "no model loaded";
                        let changed = app.model_id != "detecting..."
                            && (app.model_id != model_id || app.context_length != context_length);
                        app.model_id = model_id.clone();
                        app.context_length = context_length;
                        app.last_runtime_profile_time = Instant::now();
                        if app.provider_state == ProviderRuntimeState::Booting {
                            app.provider_state = ProviderRuntimeState::Live;
                        }
                        if now_no_model && !was_no_model {
                            app.push_message(
                                "System",
                                "No coding model loaded. Load a model in LM Studio (e.g. Qwen/Qwen3.5-9B Q4_K_M) and start the server on port 1234. Optionally also load nomic-embed-text-v2 for semantic search.",
                            );
                        } else if changed && !now_no_model {
                            app.push_message(
                                "System",
                                &format!(
                                    "Runtime profile refreshed: Model {} | CTX {}",
                                    model_id, context_length
                                ),
                            );
                        }
                    }
                    InferenceEvent::EmbedProfile { model_id } => {
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
            Constraint::Length(3),
        ])
        .split(f.size());

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Length(45)]) // Fixed width sidebar prevents bleed
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

    let ctx_block = Block::default()
        .title(" ACTIVE CONTEXT ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    f.render_widget(Clear, side[0]);
    f.render_widget(List::new(context_display).block(ctx_block), side[0]);

    // Optional: Add a Gauge for total context if tokens were tracked accurately.
    // For now, let's just make the CONTEXT pane look high-density.

    // ── SPECULAR panel (Pane 2) ────────────────────────────────────────────────
    let v_title = if app.thinking || app.agent_running {
        format!(" SPECULAR [working] ")
    } else {
        " SPECULAR [Watching] ".to_string()
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
    if !app.last_reasoning.is_empty() {
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
            "── Events ──",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::DIM),
        )]));
        for log in &app.specular_logs {
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
                    log.to_string(),
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

    // ── Box 3: Status bar ─────────────────────────────────────────────────────
    let frame = app.tick_count % 3;
    let spark = match frame {
        0 => "✧",
        1 => "✦",
        _ => "✨",
    };
    let vigil = if app.brief_mode {
        "VIGIL:[ON]"
    } else {
        "VIGIL:[off]"
    };
    let yolo = if app.yolo_mode {
        " | APPROVALS: OFF"
    } else {
        ""
    };

    let bar_constraints = if app.professional {
        vec![
            Constraint::Min(0),     // MODE
            Constraint::Length(22), // LM + VN badge
            Constraint::Length(12), // BUD
            Constraint::Length(12), // CMP
            Constraint::Length(16), // REMOTE
            Constraint::Length(28), // TOKENS
            Constraint::Length(28), // VRAM
        ]
    } else {
        vec![
            Constraint::Length(12), // NAME
            Constraint::Min(0),     // MODE
            Constraint::Length(22), // LM + VN badge
            Constraint::Length(12), // BUD
            Constraint::Length(12), // CMP
            Constraint::Length(16), // REMOTE
            Constraint::Length(28), // TOKENS
            Constraint::Length(28), // VRAM
        ]
    };
    let bar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(bar_constraints)
        .split(chunks[2]);

    let char_count: usize = app.messages_raw.iter().map(|(_, c)| c.len()).sum();
    let est_tokens = char_count / 3;
    let current_tokens = if app.total_tokens > 0 {
        app.total_tokens
    } else {
        est_tokens
    };
    let usage_text = format!(
        "TOKENS: {:0>5} | TOTAL: ${:.4}",
        current_tokens, app.current_session_cost
    );
    let runtime_age = app.last_runtime_profile_time.elapsed();
    let (lm_label, lm_color) = if app.model_id == "no model loaded" {
        ("LM:NONE", Color::Red)
    } else if app.model_id == "detecting..." || app.context_length == 0 {
        ("LM:BOOT", Color::DarkGray)
    } else if app.provider_state == ProviderRuntimeState::Recovering {
        ("LM:RECV", Color::Cyan)
    } else if matches!(
        app.provider_state,
        ProviderRuntimeState::Degraded | ProviderRuntimeState::EmptyResponse
    ) {
        ("LM:WARN", Color::Red)
    } else if app.provider_state == ProviderRuntimeState::ContextWindow {
        ("LM:CEIL", Color::Yellow)
    } else if runtime_age > std::time::Duration::from_secs(12) {
        ("LM:STALE", Color::Yellow)
    } else {
        ("LM:LIVE", Color::Green)
    };
    let compaction_percent = app.compaction_percent.min(100);
    let compaction_label = if app.compaction_threshold_tokens == 0 {
        " CMP:  0%".to_string()
    } else {
        format!(" CMP:{:>3}%", compaction_percent)
    };
    let compaction_color = if app.compaction_threshold_tokens == 0 {
        Color::DarkGray
    } else if compaction_percent >= 85 {
        Color::Red
    } else if compaction_percent >= 60 {
        Color::Yellow
    } else {
        Color::Green
    };
    let prompt_percent = app.prompt_pressure_percent.min(100);
    let prompt_label = if app.prompt_estimated_total_tokens == 0 {
        " BUD:  0%".to_string()
    } else {
        format!(" BUD:{:>3}%", prompt_percent)
    };
    let prompt_color = if app.prompt_estimated_total_tokens == 0 {
        Color::DarkGray
    } else if prompt_percent >= 85 {
        Color::Red
    } else if prompt_percent >= 60 {
        Color::Yellow
    } else {
        Color::Green
    };

    let think_badge = match app.think_mode {
        Some(true) => " [THINK]",
        Some(false) => " [FAST]",
        None => "",
    };

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

    let (status_idx, lm_idx, bud_idx, cmp_idx, remote_idx, tokens_idx, vram_idx) =
        if app.professional {
            (0usize, 1usize, 2usize, 3usize, 4usize, 5usize, 6usize)
        } else {
            (1usize, 2usize, 3usize, 4usize, 5usize, 6usize, 7usize)
        };

    if app.professional {
        f.render_widget(Clear, bar_chunks[status_idx]);

        let voice_badge = if app.voice_manager.is_enabled() {
            " | VOICE:ON"
        } else {
            ""
        };
        f.render_widget(
            Paragraph::new(format!(
                " MODE:PRO | FLOW:{}{} | CTX:{} | ERR:{}{}{}",
                app.workflow_mode,
                yolo,
                app.context_length,
                app.stats.debugging,
                think_badge,
                voice_badge
            ))
            .block(Block::default().borders(Borders::ALL)),
            bar_chunks[status_idx],
        );
    } else {
        f.render_widget(Clear, bar_chunks[0]);
        f.render_widget(
            Paragraph::new(format!(" {} {}", spark, app.soul_name))
                .block(Block::default().borders(Borders::ALL)),
            bar_chunks[0],
        );
        f.render_widget(Clear, bar_chunks[status_idx]);
        f.render_widget(
            Paragraph::new(format!("{}{}", vigil, think_badge))
                .block(Block::default().borders(Borders::ALL).fg(Color::Yellow)),
            bar_chunks[status_idx],
        );
    }

    // ── Remote status indicator ──────────────────────────────────────────────
    let git_status = app.git_state.status();
    let git_label = app.git_state.label();
    let git_color = match git_status {
        crate::agent::git_monitor::GitRemoteStatus::Connected => Color::Green,
        crate::agent::git_monitor::GitRemoteStatus::NoRemote => Color::Yellow,
        crate::agent::git_monitor::GitRemoteStatus::Behind
        | crate::agent::git_monitor::GitRemoteStatus::Ahead => Color::Magenta,
        crate::agent::git_monitor::GitRemoteStatus::Diverged
        | crate::agent::git_monitor::GitRemoteStatus::Error => Color::Red,
        _ => Color::DarkGray,
    };

    f.render_widget(Clear, bar_chunks[lm_idx]);
    f.render_widget(
        Paragraph::new(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(format!(" {}", lm_label), Style::default().fg(lm_color)),
            ratatui::text::Span::raw(" | "),
            ratatui::text::Span::styled(vein_label, Style::default().fg(vein_color)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(lm_color)),
        ),
        bar_chunks[lm_idx],
    );

    f.render_widget(Clear, bar_chunks[bud_idx]);
    f.render_widget(
        Paragraph::new(prompt_label)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(prompt_color)),
            )
            .fg(prompt_color),
        bar_chunks[bud_idx],
    );

    f.render_widget(Clear, bar_chunks[cmp_idx]);
    f.render_widget(
        Paragraph::new(compaction_label)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(compaction_color)),
            )
            .fg(compaction_color),
        bar_chunks[cmp_idx],
    );

    f.render_widget(Clear, bar_chunks[remote_idx]);
    f.render_widget(
        Paragraph::new(format!(" REMOTE: {}", git_label))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(git_color)),
            )
            .fg(git_color),
        bar_chunks[remote_idx],
    );

    let usage_color = Color::Rgb(215, 125, 40);
    f.render_widget(Clear, bar_chunks[tokens_idx]);
    f.render_widget(
        Paragraph::new(usage_text)
            .block(Block::default().borders(Borders::ALL).fg(usage_color))
            .fg(usage_color),
        bar_chunks[tokens_idx],
    );

    // ── VRAM gauge (live from nvidia-smi poller) ─────────────────────────────
    let vram_ratio = app.gpu_state.ratio();
    let vram_label = app.gpu_state.label();
    let gpu_name = app.gpu_state.gpu_name();

    let gauge_color = if vram_ratio > 0.85 {
        Color::Red
    } else if vram_ratio > 0.60 {
        Color::Yellow
    } else {
        Color::Cyan
    };
    f.render_widget(Clear, bar_chunks[vram_idx]);
    f.render_widget(
        Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", gpu_name)),
            )
            .gauge_style(Style::default().fg(gauge_color))
            .ratio(vram_ratio)
            .label(format!("  {}  ", vram_label)), // Added extra padding for visual excellence
        bar_chunks[vram_idx],
    );

    // ── Box 4: Input ──────────────────────────────────────────────────────────
    let input_style = if app.agent_running {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Rgb(120, 70, 50))
    };
    let input_rect = chunks[1];
    let title_area = input_title_area(input_rect);
    let input_hint = render_input_title(app, title_area);
    let input_block = Block::default()
        .title(input_hint)
        .borders(Borders::ALL)
        .border_style(input_style)
        .style(Style::default().bg(Color::Rgb(40, 25, 15))); // Deeper soil rich background

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
        let (title_str, title_color) = if is_diff_preview {
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
            Line::from(Span::styled(
                if is_diff_preview {
                    "  [↑↓/jk/PgUp/PgDn] Scroll   [Y] Apply   [N] Skip "
                } else {
                    "  [Y] Approve     [N] Decline "
                },
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
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
        let border_color = if is_diff_preview {
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
                    format!(" {}", approval.display),
                    Style::default().fg(Color::Cyan),
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
                Line::from(Span::raw(format!(" Tool: {}", approval.tool_name))),
                Line::from(Span::styled(
                    format!(" ❯ {}", approval.display),
                    Style::default().fg(Color::Cyan),
                )),
            ];
            f.render_widget(
                Paragraph::new(body_text)
                    .block(
                        Block::default()
                            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                            .border_style(Style::default().fg(border_color)),
                    )
                    .wrap(Wrap { trim: true }),
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
    let rust_color = Color::Rgb(180, 90, 50);

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
