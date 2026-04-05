use crossterm::event::{self, Event, KeyCode, EventStream};
use futures::StreamExt;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Terminal,
};
use std::sync::{Arc, Mutex};
use crate::ui::gpu_monitor::GpuState;
use std::time::Instant;
use tokio::sync::mpsc::Receiver;
use crate::agent::specular::SpecularEvent;
use crate::agent::swarm::{SwarmMessage, ReviewResponse};
use super::modal_review::{ActiveReview, draw_diff_review};
use crate::agent::utils::{strip_ansi, CRLF_REGEX};
use walkdir::WalkDir;

// ── Approval modal state ──────────────────────────────────────────────────────

/// Holds a pending high-risk tool approval request.
/// The agent loop is blocked on `responder` until the user presses Y or N.
pub struct PendingApproval {
    pub display: String,
    pub tool_name: String,
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
    let candidates = [
        ("src/main.rs", "Active"),
        ("src/ui/tui.rs", "Active"),
        ("Cargo.toml", "Active"),
        ("./src", "Watching"),
    ];

    let mut files = Vec::new();
    for (path, status) in candidates {
        let joined = if path == "./src" {
            root.join("src")
        } else {
            root.join(path)
        };
        if joined.exists() {
            let size = std::fs::metadata(&joined).map(|m| m.len()).unwrap_or(0);
            files.push(ContextFile {
                path: path.to_string(),
                size,
                status: status.to_string(),
            });
        }
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
    pub user_input_tx: tokio::sync::mpsc::Sender<String>,
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
}

impl App {
    pub fn reset_active_context(&mut self) {
        self.active_context = default_active_context();
    }

    pub fn push_message(&mut self, speaker: &str, content: &str) {
        let filtered = filter_tui_noise(content);
        if filtered.is_empty() && !content.is_empty() { return; } // Completely suppressed noise

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
                if token.contains(' ') || token.contains('\n') || token.contains('.') || token.len() > 5 {
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
            "You"      => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            "Hematite" => Style::default().fg(rust).add_modifier(Modifier::BOLD),
            "Tool"     => Style::default().fg(Color::Cyan),
            _          => Style::default().fg(Color::DarkGray),
        };

        // Aggressive trim to avoid leading/trailing blank rows.
        let cleaned = crate::agent::inference::strip_think_blocks(content).trim().to_string();
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

            let label = if is_first { format!("{}: ", speaker) } else { "  ".to_string() };
            
            if speaker == "Tool" && (raw_line.starts_with("-") || raw_line.starts_with("+") || raw_line.starts_with("@@")) {
                let line_style = if raw_line.starts_with("-") {
                    Style::default().fg(Color::Red)
                } else if raw_line.starts_with("+") {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::DIM)
                };
                lines.push(Line::from(vec![
                    Span::raw("    "), // Deeper indent for diffs
                    Span::styled(raw_line.to_string(), line_style),
                ]));
            } else {
                let mut spans = vec![
                    Span::raw(" "),
                    Span::styled(label, style),
                ];
                // Render inline markdown for Hematite responses; plain text for others.
                if speaker == "Hematite" {
                    spans.extend(inline_markdown_core(raw_line));
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
            &self.input[pos+1..]
        } else {
            ""
        }.to_lowercase();
        
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
                    if matches.len() < 15 { // Show up to 15 at once
                        matches.push(path_str);
                    }
                }
            }
            if total_found > 100 { break; } // Safety cap for massive repos
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
        self.selected_suggestion = self.selected_suggestion.min(self.autocomplete_suggestions.len().saturating_sub(1));
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
                if (trimmed.starts_with("- [ ]") || trimmed.starts_with("- [/]")) && trimmed.len() > 6 {
                    self.current_objective = trimmed[6..].trim().to_string();
                    return;
                }
            }
        }
        self.current_objective = "Idle".into();
    }

    /// [Auto-Diagnostic] Copy full session transcript to clipboard.
    pub fn copy_transcript_to_clipboard(&self) {
        let mut history = self.messages_raw.iter()
            .map(|m| format!("[{}] {}\n", m.0, m.1))
            .collect::<String>();
        
        history.push_str("\nSession Stats\n");
        history.push_str(&format!("Tokens: {}\n", self.total_tokens));
        history.push_str(&format!("Cost: ${:.4}\n", self.current_session_cost));
        
        // Windows clip.exe — fast and zero dependencies.
        let mut child = std::process::Command::new("clip.exe")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn clip.exe");
        
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(history.as_bytes());
        }
        let _ = child.wait();
    }
}


// ── run_app ───────────────────────────────────────────────────────────────────

pub async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut specular_rx: Receiver<SpecularEvent>,
    mut agent_rx: Receiver<crate::agent::inference::InferenceEvent>,
    user_input_tx: tokio::sync::mpsc::Sender<String>,
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
            app.push_message("System", "🚨 HARDWARE GUARD: VRAM > 95%. Brief Mode auto-enabled to prevent crash.");
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
                        use crossterm::event::MouseEventKind;
                        let (width, _) = match terminal.size() {
                            Ok(s) => (s.width, s.height),
                            Err(_) => (80, 24),
                        };
                        let is_right_side = mouse.column as f64 > width as f64 * 0.65;

                        match mouse.kind {
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
                        if let Some(approval) = app.awaiting_approval.take() {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    app.push_message("System", &format!("Approved: {}", approval.display));
                                    let _ = approval.responder.send(true);
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') => {
                                    app.push_message("System", "Declined.");
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
                                    app.copy_transcript_to_clipboard();
                                    break;
                                }

                            KeyCode::Esc => {
                                // Silence current audio immediately
                                app.voice_manager.stop();
                                
                                // Always flag cancellation on Esc to block post-stop 'zombie tokens' from speech
                                app.cancel_token.store(true, std::sync::atomic::Ordering::SeqCst);
                                
                                if app.thinking || app.agent_running {
                                    app.copy_transcript_to_clipboard();
                                    app.push_message("System", "Cancellation requested. Logs copied to clipboard.");
                                }
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
                                let enabled = app.voice_manager.toggle();
                                app.push_message("System", &format!("Voice of Hematite: {}", if enabled { "VIBRANT" } else { "SILENCED" }));
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
                                                app.messages.clear();
                                                app.messages_raw.clear();
                                                app.last_reasoning.clear();
                                                app.current_thought.clear();
                                                app.specular_logs.clear();
                                                app.reset_active_context();
                                                app.current_objective = "Idle".into();
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
                                            "/copy" => {
                                                app.copy_transcript_to_clipboard();
                                                app.push_message("System", "Diagnostic transcript copied to clipboard.");
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/new" => {
                                                app.messages.clear();
                                                app.messages_raw.clear();
                                                app.last_reasoning.clear();
                                                app.current_thought.clear();
                                                app.specular_logs.clear();
                                                app.reset_active_context();
                                                app.current_objective = "Idle".into();
                                                app.push_message("You", "/new");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send("/new".to_string());
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/forget" => {
                                                app.messages.clear();
                                                app.messages_raw.clear();
                                                app.last_reasoning.clear();
                                                app.current_thought.clear();
                                                app.specular_logs.clear();
                                                app.reset_active_context();
                                                app.current_objective = "Idle".into();
                                                app.push_message("You", "/forget");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send("/forget".to_string());
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
                                                let _ = app.user_input_tx.try_send(outbound);
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/worktree" => {
                                                let sub = parts.get(1).copied().unwrap_or("");
                                                match sub {
                                                    "list" => {
                                                        app.push_message("You", "/worktree list");
                                                        app.agent_running = true;
                                                        let _ = app.user_input_tx.try_send(
                                                            "Call git_worktree with action=list".to_string()
                                                        );
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
                                                            let _ = app.user_input_tx.try_send(directive);
                                                        }
                                                    }
                                                    "remove" => {
                                                        let wt_path = parts.get(2).copied().unwrap_or("");
                                                        if wt_path.is_empty() {
                                                            app.push_message("System", "Usage: /worktree remove <path>");
                                                        } else {
                                                            app.push_message("You", &format!("/worktree remove {wt_path}"));
                                                            app.agent_running = true;
                                                            let _ = app.user_input_tx.try_send(
                                                                format!("Call git_worktree with action=remove path={wt_path}")
                                                            );
                                                        }
                                                    }
                                                    "prune" => {
                                                        app.push_message("You", "/worktree prune");
                                                        app.agent_running = true;
                                                        let _ = app.user_input_tx.try_send(
                                                            "Call git_worktree with action=prune".to_string()
                                                        );
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
                                                let _ = app.user_input_tx.try_send("/think".to_string());
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/no_think" => {
                                                app.think_mode = Some(false);
                                                app.push_message("You", "/no_think");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send("/no_think".to_string());
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/lsp" => {
                                                app.push_message("You", "/lsp");
                                                app.agent_running = true;
                                                let _ = app.user_input_tx.try_send("/lsp".to_string());
                                                app.history_idx = None;
                                                continue;
                                            }
                                            "/help" => {
                                                app.push_message("System",
                                                    "Hematite Commands:\n\
                                                     /auto             — (Flow) Let Hematite choose the narrowest effective workflow\n\
                                                     /ask [prompt]     — (Flow) Read-only analysis mode; optional inline prompt\n\
                                                     /code [prompt]    — (Flow) Explicit implementation mode; optional inline prompt\n\
                                                     /architect [prompt] — (Flow) Plan-first mode; optional inline prompt\n\
                                                     /read-only [prompt] — (Flow) Hard read-only mode; optional inline prompt\n\
                                                     /new              — (Reset) Clear history, memories, and task files\n\
                                                     /forget           — (Wipe) Nuclear pivot: reset history & active tasks\n\
                                                     /clear            — (UI) Clear dialogue display only\n\
                                                     /undo             — (Ghost) Revert last file change\n\
                                                     /diff             — (Git) Show session changes (--stat)\n\
                                                     /lsp              — (Logic) Start Language Servers (semantic intelligence)\n\
                                                     /swarm <text>     — (Swarm) Spawn parallel workers on a directive\n\
                                                     /worktree <cmd>   — (Isolated) Manage git worktrees (list|add|remove|prune)\n\
                                                     /think            — (Brain) Enable deep reasoning mode\n\
                                                     /no_think         — (Speed) Disable reasoning (3-5x faster responses)\n\
                                                     \nHotkeys:\n\
                                                     Ctrl+B — Toggle Brief Mode (minimal output)\n\
                                                     Ctrl+P — Toggle Professional Mode (strip personality)\n\
                                                     Ctrl+Y — Toggle Approvals Off (bypass safety approvals)\n\
                                                     Ctrl+S — Quick Swarm (hardcoded bootstrap)\n\
                                                     Ctrl+Z — Undo last edit\n\
                                                     Ctrl+Q/C — Quit session\n\
                                                     ESC    — Silence current playback\n\
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
                                                app.push_message("System", "Hematite v0.1.0- strategist");
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
                                    app.last_reasoning.clear(); 
                                    app.manual_scroll_offset = None;
                                    app.specular_auto_scroll = true;
                                    let tx = app.user_input_tx.clone();
                                    tokio::spawn(async move {
                                        let _ = tx.send(input_text).await;
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(Event::Paste(content))) => {
                        // Normalize pasted newlines into spaces so we don't accidentally submit 
                        // multiple lines or break the single-line input logic.
                        let normalized = content.replace("\r\n", " ").replace('\n', " ");
                        app.input.push_str(&normalized);
                        app.last_input_time = Instant::now();
                    }
                    _ => {}
                }
            }

            // ── Specular proactive watcher ────────────────────────────────────
            Some(specular_evt) = specular_rx.recv() => {
                match specular_evt {
                    SpecularEvent::SyntaxError { path, details } => {
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
                                let _ = tx.send(msg).await;
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
                        let display = format!("( )  {} {}", name, args);
                        app.push_message("Tool", &display);
                        // Track in active context
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
                        // Previews should ALWAYS be sanitized single-line summaries in the chat window.
                        let preview = first_n_chars(&output, 100);
                        app.push_message("Tool", &format!("{}  {} → {}", icon, name, preview));
                        
                        // If it was a read or write, we can extract the path from the app.active_context "Running" entries
                        // but it's simpler to just let Specular handle the indexing or update here if we had the path.
                        
                        // Remove "Running" tools from context list
                        app.active_context.retain(|f| f.path != name || f.status != "Running");
                        app.manual_scroll_offset = None;
                    }
                    InferenceEvent::ApprovalRequired { id: _, name, display, responder } => {
                        app.awaiting_approval = Some(PendingApproval {
                            display: display.clone(),
                            tool_name: name,
                            responder,
                        });
                        app.push_message("System", "[!]  Approval required (Press [Y] Approve or [N] Decline)");
                        app.push_message("System", &format!("Command: {}", display));
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
                        app.thinking = false;
                        app.agent_running = false;
                        if app.voice_manager.is_enabled() {
                            app.voice_manager.flush();
                        }
                        app.push_message("System", &format!("Error: {e}"));
                    }
                    InferenceEvent::TaskProgress { id, label, progress } => {
                        let nid = normalize_id(&id);
                        app.active_workers.insert(nid.clone(), progress);
                        app.worker_labels.insert(nid, label);
                    }
                    InferenceEvent::ModelDetected(id) => {
                        app.model_id = id;
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

    let input_len = app.input.len();
    let width = f.size().width.max(1) as usize;
    // We know the top[0] (Dialogue) is 65%. 
    // The input area width is roughly 65% of the total width minus borders.
    let approx_input_w = (width * 65 / 100).saturating_sub(4).max(1);
    let needed_lines = (input_len / approx_input_w) as u16 + 3;
    let input_height = needed_lines.clamp(3, 10);

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
            Style::default().fg(Color::Magenta).add_modifier(Modifier::DIM),
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
            Span::styled(format!(" TASK: {} ", objective_text), Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC)),
        ])
    } else { 
        Line::from(format!(" TASK: {} ", objective_text))
    };
    
    let core_para = Paragraph::new(core_lines.clone())
        .block(Block::default()
            .title(core_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)))
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
    let chat_area = Rect::new(top[0].x + 1, top[0].y, top[0].width.saturating_sub(2).max(1), top[0].height);
    f.render_widget(Clear, chat_area); 
    f.render_widget(core_para.scroll((scroll, 0)), chat_area);
    
    // Scrollbar: content_length = max_scroll+1 so position==max_scroll puts the
    // thumb flush at the bottom (position == content_length - 1).
    let mut scrollbar_state = ScrollbarState::new(max_scroll as usize + 1)
        .position(scroll as usize);
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
    let mut context_display = context_source.iter().map(|f| {
        let (icon, color) = match f.status.as_str() {
            "Running" => ("⚙️", Color::Cyan),
            "Dirty"   => ("📝", Color::Yellow),
            _         => ("📄", Color::Gray),
        };
        // Simple heuristic for "Tokens" (size / 4)
        let tokens = f.size / 4;
        ListItem::new(Line::from(vec![
            Span::styled(format!(" {} ", icon), Style::default().fg(color)),
            Span::styled(f.path.clone(), Style::default().fg(Color::White)),
            Span::styled(format!(" {}t ", tokens), Style::default().fg(Color::DarkGray)),
        ]))
    }).collect::<Vec<ListItem>>();

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
        v_lines.push(Line::from(vec![
            Span::styled(
                format!("[ {}{} ]", label, dots),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]));
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
            Style::default().fg(Color::White).add_modifier(Modifier::DIM),
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
            
            let id_prefix = if id == "AGENT" { "Agent: ".to_string() } else { format!("W{}: ", id) };
            
            v_lines.push(Line::from(vec![
                Span::styled(id_prefix, Style::default().fg(Color::Gray)),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(format!(" {} ", display_label), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}%", prog.min(100)), Style::default().fg(Color::DarkGray)),
            ]));
        }
        v_lines.push(Line::raw(""));
    }

    // Section: last completed turn's reasoning
    if !app.last_reasoning.is_empty() {
        v_lines.push(Line::from(vec![Span::styled(
            "── Logic Trace ──",
            Style::default().fg(Color::White).add_modifier(Modifier::DIM),
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
            Style::default().fg(Color::White).add_modifier(Modifier::DIM),
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
                Span::styled(log.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::DIM)),
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
    let mut v_scrollbar_state = ScrollbarState::new(v_max_scroll as usize + 1)
        .position(v_scroll as usize);
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
    let spark = match frame { 0 => "✧", 1 => "✦", _ => "✨" };
    let vigil = if app.brief_mode { "VIGIL:[ON]" } else { "VIGIL:[off]" };
    let yolo  = if app.yolo_mode  { " | APPROVALS: OFF" }     else { "" };

    let bar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12),  // NAME
            Constraint::Min(0),       // MODE
            Constraint::Length(16),  // REMOTE
            Constraint::Length(30),  // TOKENS
            Constraint::Length(30),  // VRAM (expanded to prevent clipping)
        ])
        .split(chunks[2]);

    let char_count: usize = app.messages_raw.iter().map(|(_, c)| c.len()).sum();
    let est_tokens = char_count / 3;
    let current_tokens = if app.total_tokens > 0 { app.total_tokens } else { est_tokens };
    let usage_text = format!("TOKENS: {:0>5} | TOTAL: ${:.4}", current_tokens, app.current_session_cost);

    let think_badge = match app.think_mode {
        Some(true)  => " [THINK]",
        Some(false) => " [FAST]",
        None        => "",
    };

    if app.professional {
        f.render_widget(Clear, bar_chunks[0]);
        f.render_widget(
            Paragraph::new(" Hematite").block(Block::default().borders(Borders::ALL)),
            bar_chunks[0],
        );
        f.render_widget(Clear, bar_chunks[1]);
        
        let voice_badge = if app.voice_manager.is_enabled() { " | VOICE: ON" } else { "" };
        f.render_widget(
            Paragraph::new(format!(" MODE: PROFESSIONAL | FLOW: {}{} | ERR: {:02}{}{}", app.workflow_mode, yolo, app.stats.debugging, think_badge, voice_badge))
                .block(Block::default().borders(Borders::ALL)),
            bar_chunks[1],
        );
    } else {
        f.render_widget(Clear, bar_chunks[0]);
        f.render_widget(
            Paragraph::new(format!(" {} Rusty", spark)).block(Block::default().borders(Borders::ALL)),
            bar_chunks[0],
        );
        f.render_widget(Clear, bar_chunks[1]);
        f.render_widget(
            Paragraph::new(format!("{}{}", vigil, think_badge))
                .block(Block::default().borders(Borders::ALL).fg(Color::Yellow)),
            bar_chunks[1],
        );
    }

    // ── Remote status indicator ──────────────────────────────────────────────
    let git_status = app.git_state.status();
    let git_label = app.git_state.label();
    let git_color = match git_status {
        crate::agent::git_monitor::GitRemoteStatus::Connected => Color::Green,
        crate::agent::git_monitor::GitRemoteStatus::NoRemote => Color::Yellow,
        crate::agent::git_monitor::GitRemoteStatus::Behind | crate::agent::git_monitor::GitRemoteStatus::Ahead => Color::Magenta,
        crate::agent::git_monitor::GitRemoteStatus::Diverged | crate::agent::git_monitor::GitRemoteStatus::Error => Color::Red,
        _ => Color::DarkGray,
    };
    
    f.render_widget(Clear, bar_chunks[2]);
    f.render_widget(
        Paragraph::new(format!(" REMOTE: {}", git_label))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(git_color)))
            .fg(git_color),
        bar_chunks[2],
    );

    f.render_widget(Clear, bar_chunks[3]);
    f.render_widget(
        Paragraph::new(usage_text).block(Block::default().borders(Borders::ALL).fg(Color::Cyan)).fg(Color::Cyan),
        bar_chunks[3],
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
    f.render_widget(Clear, bar_chunks[4]);
    f.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(format!(" {} ", gpu_name)))
            .gauge_style(Style::default().fg(gauge_color))
            .ratio(vram_ratio)
            .label(format!("  {}  ", vram_label)), // Added extra padding for visual excellence
        bar_chunks[4],
    );

    // ── Box 4: Input ──────────────────────────────────────────────────────────
    let input_style = if app.agent_running {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Rgb(120, 70, 50))
    };
    let input_hint = if app.agent_running {
        " Working… (wait for response)".to_string()
    } else {
        let voice_status = if app.voice_manager.is_enabled() { "ON" } else { "off" };
        format!(
            " [Enter] send · [^S] swarm · [^T] voice:{} · [ESC] sil · [^B] brief · [^Y] approvals · [^Q] quit (Len: {}) ",
            voice_status,
            app.input.len()
        )
    };
    let input_block = Block::default()
        .title(input_hint)
        .borders(Borders::ALL)
        .border_style(input_style)
        .style(Style::default().bg(Color::Rgb(40, 25, 15))); // Deeper soil rich background
    
    let inner_area = input_block.inner(chunks[1]);
    f.render_widget(Clear, chunks[1]);
    f.render_widget(input_block, chunks[1]);

    f.render_widget(
        Paragraph::new(app.input.as_str())
            .wrap(Wrap { trim: true }),
        inner_area,
    );

    // Hardware Cursor (Managed by terminal emulator for smooth asynchronous blink)
    // Hardware Cursor (Managed by terminal emulator for smooth asynchronous blink)
    // Always call set_cursor during standard operation to "park" the cursor safely in the input box,
    // preventing it from jittering to (0,0) (the top-left title) during modal reviews.
    if !app.agent_running {
        let text_w = app.input.len() as u16;
        let max_w = inner_area.width.saturating_sub(1);
        let cursor_x = inner_area.x + text_w.min(max_w);
        f.set_cursor(cursor_x, inner_area.y);
    }

    // ── High-risk approval modal ───────────────────────────────────────────────
    if let Some(approval) = &app.awaiting_approval {
        // Dynamic Height Modal: Use a Header/Body split to ensure instructions are never cut off.
        let area = centered_rect(75, 50, f.size());
        f.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Header: Title + Instructions
                Constraint::Min(0),    // Body: Tool + Command
            ])
            .split(area);

        // ── Modal Header ─────────────────────────────────────────────────────
        let header_text = vec![
            Line::from(Span::styled(
                " 🔒 HIGH-RISK OPERATION REQUESTED 🔒 ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "  [Y] Approve     [N] Decline ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
        ];
        f.render_widget(
            Paragraph::new(header_text)
                .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT).border_style(Style::default().fg(Color::Red)))
                .alignment(ratatui::layout::Alignment::Center),
            chunks[0],
        );

        // ── Modal Body ───────────────────────────────────────────────────────
        let body_text = vec![
            Line::from(Span::raw(format!(" Tool: {}", approval.tool_name))),
            Line::from(Span::styled(format!(" ❯ {}", approval.display), Style::default().fg(Color::Cyan))),
        ];
        f.render_widget(
            Paragraph::new(body_text)
                .block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT).border_style(Style::default().fg(Color::Red)))
                .wrap(Wrap { trim: true }),
            chunks[1],
        );
    }

    // ── Swarm diff review modal ────────────────────────────────────────────────
    if let Some(review) = &app.active_review {
        draw_diff_review(f, review);
    }

    // ── Autocomplete Hatch (Floating Popup) ──────────────────────────────────
    if app.show_autocomplete && !app.autocomplete_suggestions.is_empty() {
        let area = Rect {
            x: chunks[1].x + 2,
            y: chunks[1].y.saturating_sub(app.autocomplete_suggestions.len() as u16 + 2),
            width: chunks[1].width.saturating_sub(4),
            height: app.autocomplete_suggestions.len() as u16 + 2,
        };
        f.render_widget(Clear, area);
        
        let items: Vec<ListItem> = app.autocomplete_suggestions.iter().enumerate().map(|(i, s)| {
            let style = if i == app.selected_suggestion {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(format!(" 📄 {}", s)).style(style)
        }).collect();
        
        let hatch = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" @ RESOLVER (Matching: {}) ", app.autocomplete_filter)));
        f.render_widget(hatch, area);
        
        // Optional "More matches..." indicator
        if app.autocomplete_suggestions.len() >= 15 {
            let more_area = Rect {
                x: area.x + 2,
                y: area.y + area.height - 1,
                width: 20,
                height: 1,
            };
            f.render_widget(Paragraph::new("... (type to narrow) ").style(Style::default().fg(Color::DarkGray)), more_area);
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
        "Hematite: ", "HEMATITE: ", "Assistant: ", "assistant: ", 
        "Okay, ", "Hmm, ", "Wait, ", "Alright, ", "Got it, ", 
        "Certainly, ", "Sure, ", "Understood, "
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
        .replace("<thought>", "").replace("</thought>", "")
        .replace("<think>", "").replace("</think>", "");
    let trimmed = cleaned_owned.trim();
    if trimmed.is_empty() {
        return vec![];
    }

    // # Heading (all levels → bold white)
    for (prefix, indent) in &[("### ", "  "), ("## ", " "), ("# ", "")] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return vec![Line::from(vec![Span::styled(
                format!("{}{}", indent, rest),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )])];
        }
    }

    // > blockquote
    if let Some(rest) = trimmed.strip_prefix("> ").or_else(|| trimmed.strip_prefix(">")) {
        return vec![Line::from(vec![
            Span::styled("| ", Style::default().fg(Color::DarkGray)),
            Span::styled(rest.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::DIM)),
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
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
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
            if !before.is_empty() { spans.push(Span::raw(before.to_string())); }
            let after_open = &remaining[start + 2..];
            if let Some(end) = after_open.find("**") {
                spans.push(Span::styled(after_open[..end].to_string(),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
                remaining = &after_open[end + 2..];
                continue;
            }
        }
        if let Some(start) = remaining.find('`') {
            let before = &remaining[..start];
            if !before.is_empty() { spans.push(Span::raw(before.to_string())); }
            let after_open = &remaining[start + 1..];
            if let Some(end) = after_open.find('`') {
                spans.push(Span::styled(after_open[..end].to_string(),
                    Style::default().fg(Color::Yellow)));
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

fn draw_splash<B: Backend>(
    terminal: &mut Terminal<B>,
) -> Result<(), Box<dyn std::error::Error>> {
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

        // Total content height: logo(6) + spacer(1) + version(1) + author(1) + spacer(2) + prompt(1) = 12
        let content_height: u16 = 12;
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
        lines.push(Line::from(vec![
            Span::styled(
                format!("v{}", version),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        // Developer credit
        lines.push(Line::from(vec![
            Span::styled(
                "Developed by Ocean Bennett",
                Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
            ),
        ]));

        // Spacer
        lines.push(Line::raw(""));
        lines.push(Line::raw(""));

        // Prompt
        lines.push(Line::from(vec![
            Span::styled(
                "[ Press ENTER to initialize ]",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
            ),
        ]));

        let splash = Paragraph::new(lines)
            .alignment(ratatui::layout::Alignment::Center);

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
        if CRLF_REGEX.is_match(line) { continue; }
        // Strip git checkout/file update noise if it's too repetitive.
        if line.contains("Updating files:") && line.contains("%") { continue; }
        // Strip random terminal control characters that might have escaped.
        let sanitized: String = line.chars().filter(|c| !c.is_control() || *c == '\t').collect();
        if sanitized.trim().is_empty() && !line.trim().is_empty() { continue; }

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
