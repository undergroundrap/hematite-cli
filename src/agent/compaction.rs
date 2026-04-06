use crate::agent::inference::ChatMessage;
use std::collections::{BTreeSet, HashSet};

/// Professional Compaction Configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionConfig {
    pub preserve_recent_messages: usize,
    /// Token threshold before compaction fires. Set dynamically via `adaptive()`.
    pub max_estimated_tokens: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            preserve_recent_messages: 10,
            max_estimated_tokens: 15_000,
        }
    }
}

impl CompactionConfig {
    /// Build a hardware-aware config that scales with the model's context window
    /// and current VRAM pressure.
    ///
    /// - `context_length`: tokens the loaded model can handle (from `/api/v0/models`)
    /// - `vram_ratio`: current VRAM usage 0.0–1.0 (from GpuState::ratio)
    ///
    /// Formula: threshold = ctx * 0.40 * (1 - vram * 0.5), clamped [4k, 60k].
    /// preserve_recent_messages scales with context: roughly 1 message per 3k tokens.
    pub fn adaptive(context_length: usize, vram_ratio: f64) -> Self {
        let vram = vram_ratio.clamp(0.0, 1.0);
        let effective = (context_length as f64 * 0.40 * (1.0 - vram * 0.5)) as usize;
        let max_estimated_tokens = effective.max(4_000).min(60_000);
        let preserve_recent_messages = (context_length / 3_000).clamp(8, 20);
        Self { preserve_recent_messages, max_estimated_tokens }
    }
}

pub struct CompactionResult {
    pub messages: Vec<ChatMessage>,
    pub summary: Option<String>,
}

const DEFAULT_MAX_SUMMARY_CHARS: usize = 1_400;
const DEFAULT_MAX_SUMMARY_LINES: usize = 28;
const DEFAULT_MAX_SUMMARY_LINE_CHARS: usize = 180;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryCompressionBudget {
    pub max_chars: usize,
    pub max_lines: usize,
    pub max_line_chars: usize,
}

impl Default for SummaryCompressionBudget {
    fn default() -> Self {
        Self {
            max_chars: DEFAULT_MAX_SUMMARY_CHARS,
            max_lines: DEFAULT_MAX_SUMMARY_LINES,
            max_line_chars: DEFAULT_MAX_SUMMARY_LINE_CHARS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryCompressionResult {
    pub summary: String,
    pub original_chars: usize,
    pub compressed_chars: usize,
    pub original_lines: usize,
    pub compressed_lines: usize,
    pub removed_duplicate_lines: usize,
    pub omitted_lines: usize,
    pub truncated: bool,
}

pub fn compress_summary(
    summary: &str,
    budget: SummaryCompressionBudget,
) -> SummaryCompressionResult {
    let original_chars = summary.chars().count();
    let original_lines = summary.lines().count();
    let normalized = normalize_summary_lines(summary, budget.max_line_chars);

    if normalized.lines.is_empty() || budget.max_chars == 0 || budget.max_lines == 0 {
        return SummaryCompressionResult {
            summary: String::new(),
            original_chars,
            compressed_chars: 0,
            original_lines,
            compressed_lines: 0,
            removed_duplicate_lines: normalized.removed_duplicate_lines,
            omitted_lines: normalized.lines.len(),
            truncated: original_chars > 0,
        };
    }

    let selected = select_summary_line_indexes(&normalized.lines, budget);
    let mut compressed_lines = selected
        .iter()
        .map(|index| normalized.lines[*index].clone())
        .collect::<Vec<_>>();
    if compressed_lines.is_empty() {
        compressed_lines.push(truncate_summary_line(
            &normalized.lines[0],
            budget.max_chars,
        ));
    }
    let omitted_lines = normalized
        .lines
        .len()
        .saturating_sub(compressed_lines.len());
    if omitted_lines > 0 {
        push_summary_line_with_budget(
            &mut compressed_lines,
            format!("- ... {omitted_lines} additional line(s) omitted."),
            budget,
        );
    }

    let compressed_summary = compressed_lines.join("\n");
    SummaryCompressionResult {
        summary: compressed_summary.clone(),
        original_chars,
        compressed_chars: compressed_summary.chars().count(),
        original_lines,
        compressed_lines: compressed_lines.len(),
        removed_duplicate_lines: normalized.removed_duplicate_lines,
        omitted_lines,
        truncated: compressed_summary != summary.trim(),
    }
}

pub fn compress_summary_text(summary: &str) -> String {
    compress_summary(summary, SummaryCompressionBudget::default()).summary
}

const COMPACT_PREAMBLE: &str = "## CONTEXT SUMMARY (RECURSIVE CHAIN)\n\
    This session is being continued from a previous conversation. The summary below covers the earlier portion.\n\n";
const COMPACT_INSTRUCTION: &str = "\n\nIMPORTANT: Resume directly from the last message. Do not recap or acknowledge this summary.";

/// Layer 6: Structured Session Memory.
/// Preserves the "Mission Context" across compactions.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SessionCheckpoint {
    pub state: String,
    pub summary: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SessionVerification {
    pub successful: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SessionCompactionLedger {
    pub count: u32,
    pub removed_message_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SessionMemory {
    pub current_task: String,
    pub working_set: std::collections::HashSet<String>,
    pub learnings: Vec<String>,
    #[serde(default)]
    pub current_plan: Option<crate::tools::plan::PlanHandoff>,
    #[serde(default)]
    pub last_checkpoint: Option<SessionCheckpoint>,
    #[serde(default)]
    pub last_blocker: Option<SessionCheckpoint>,
    #[serde(default)]
    pub last_recovery: Option<SessionCheckpoint>,
    #[serde(default)]
    pub last_verification: Option<SessionVerification>,
    #[serde(default)]
    pub last_compaction: Option<SessionCompactionLedger>,
}

impl SessionMemory {
    pub fn has_signal(&self) -> bool {
        let task = self.current_task.trim();
        (!task.is_empty() && task != "Ready for new mission.")
            || !self.working_set.is_empty()
            || !self.learnings.is_empty()
            || self.last_checkpoint.is_some()
            || self.last_blocker.is_some()
            || self.last_recovery.is_some()
            || self.last_verification.is_some()
            || self.last_compaction.is_some()
            || self
                .current_plan
                .as_ref()
                .map(|plan| plan.has_signal())
                .unwrap_or(false)
    }

    pub fn to_prompt(&self) -> String {
        let mut s = format!("- **Active Task**: {}\n", self.current_task);
        if let Some(plan) = &self.current_plan {
            if plan.has_signal() {
                s.push_str("- **Active Plan Handoff**:\n");
                s.push_str(&plan.to_prompt());
            }
        }
        if !self.working_set.is_empty() {
            let files: Vec<_> = self.working_set.iter().cloned().collect();
            s.push_str(&format!("- **Working Set**: {}\n", files.join(", ")));
        }
        if !self.learnings.is_empty() {
            s.push_str("- **Key Learnings**:\n");
            for l in &self.learnings {
                s.push_str(&format!("  - {l}\n"));
            }
        }
        if let Some(checkpoint) = &self.last_checkpoint {
            if checkpoint.summary.trim().is_empty() {
                s.push_str(&format!("- **Latest Checkpoint**: {}\n", checkpoint.state));
            } else {
                s.push_str(&format!(
                    "- **Latest Checkpoint**: {} - {}\n",
                    checkpoint.state, checkpoint.summary
                ));
            }
        }
        if let Some(blocker) = &self.last_blocker {
            if blocker.summary.trim().is_empty() {
                s.push_str(&format!("- **Latest Blocker**: {}\n", blocker.state));
            } else {
                s.push_str(&format!(
                    "- **Latest Blocker**: {} - {}\n",
                    blocker.state, blocker.summary
                ));
            }
        }
        if let Some(recovery) = &self.last_recovery {
            if recovery.summary.trim().is_empty() {
                s.push_str(&format!("- **Latest Recovery**: {}\n", recovery.state));
            } else {
                s.push_str(&format!(
                    "- **Latest Recovery**: {} - {}\n",
                    recovery.state, recovery.summary
                ));
            }
        }
        if let Some(verification) = &self.last_verification {
            let status = if verification.successful { "passed" } else { "failed" };
            s.push_str(&format!(
                "- **Latest Verification**: {} - {}\n",
                status, verification.summary
            ));
        }
        if let Some(compaction) = &self.last_compaction {
            s.push_str(&format!(
                "- **Latest Compaction**: pass {} removed {} message(s) - {}\n",
                compaction.count, compaction.removed_message_count, compaction.summary
            ));
        }
        s
    }

    pub fn inherit_runtime_ledger_from(&mut self, other: &Self) {
        self.last_checkpoint = other.last_checkpoint.clone();
        self.last_blocker = other.last_blocker.clone();
        self.last_recovery = other.last_recovery.clone();
        self.last_verification = other.last_verification.clone();
        self.last_compaction = other.last_compaction.clone();
    }

    pub fn record_checkpoint(&mut self, state: impl Into<String>, summary: impl Into<String>) {
        let checkpoint = SessionCheckpoint {
            state: state.into(),
            summary: summary.into(),
        };
        let state_name = checkpoint.state.as_str();
        if state_name == "recovering_provider" {
            self.last_recovery = Some(checkpoint.clone());
        }
        if state_name.starts_with("blocked_") {
            self.last_blocker = Some(checkpoint.clone());
        }
        self.last_checkpoint = Some(checkpoint);
    }

    pub fn record_verification(&mut self, successful: bool, summary: impl Into<String>) {
        self.last_verification = Some(SessionVerification {
            successful,
            summary: summary.into(),
        });
    }

    pub fn record_compaction(&mut self, removed_message_count: usize, summary: impl Into<String>) {
        let count = self
            .last_compaction
            .as_ref()
            .map_or(1, |entry| entry.count.saturating_add(1));
        self.last_compaction = Some(SessionCompactionLedger {
            count,
            removed_message_count,
            summary: summary.into(),
        });
    }

    pub fn clear(&mut self) {
        self.current_task = "Ready for new mission.".to_string();
        self.working_set.clear();
        self.learnings.clear();
        self.current_plan = None;
        self.last_checkpoint = None;
        self.last_blocker = None;
        self.last_recovery = None;
        self.last_verification = None;
        self.last_compaction = None;
    }
}

/// Returns true when history is large enough to warrant compaction.
/// Pass the model's context_length and current vram_ratio for adaptive thresholds.
pub fn should_compact(history: &[ChatMessage], context_length: usize, vram_ratio: f64) -> bool {
    let config = CompactionConfig::adaptive(context_length, vram_ratio);
    history.len().saturating_sub(1) > config.preserve_recent_messages + 5
        || estimate_compactable_tokens(history) > config.max_estimated_tokens
}

pub fn compact_history(
    history: &[ChatMessage],
    existing_summary: Option<&str>,
    config: CompactionConfig,
    // The index of the user message that started the CURRENT turn.
    // We must NEVER summarize past this index if we are in the middle of a turn.
    anchor_index: Option<usize>,
) -> CompactionResult {
    if history.len() <= config.preserve_recent_messages + 5 {
        return CompactionResult {
            messages: history.to_vec(),
            summary: existing_summary.map(|s| s.to_string()),
        };
    }

    // Triple-Slicer Strategy:
    // 1. [SYSTEM] (Index 0)
    // 2. [PAST TURNS] (Index 1 .. Anchor) -> Folded into summary.
    // 3. [ENTRY PROMPT] (Index Anchor) -> Kept verbatim for Jinja alignment.
    // 4. [MIDDLE OF TURN] (Index Anchor+1 .. End - Preserve) -> Folded into summary.
    // 5. [RECENT WORK] (End - Preserve .. End) -> Kept verbatim.

    // The anchor MUST be at least 1 (to avoid 1..0 slice panics) and 
    // capped at history.len() - 1.
    let anchor = anchor_index.unwrap_or(1).max(1).min(history.len() - 1);
    let keep_from = history.len().saturating_sub(config.preserve_recent_messages);
    
    let mut messages_to_summarize = Vec::new();
    let mut preserved_messages = Vec::new();

    // Preserve the Turn Entry User Prompt as the primary anchor.
    // Everything before it is permanently summarized.
    if anchor > 1 {
        messages_to_summarize.extend(history[1..anchor].iter().cloned());
    }
    preserved_messages.push(history[anchor].clone());

    // Evaluate the Middle of the Turn.
    if keep_from > anchor + 1 {
        // We have enough bulk in the current turn to justify a "Partial Turn" summary.
        messages_to_summarize.extend(history[anchor + 1..keep_from].iter().cloned());
        preserved_messages.extend(history[keep_from..].iter().cloned());
    } else {
        // Not enough bulk inside the turn yet; just preserve the rest.
        preserved_messages.extend(history[anchor + 1..].iter().cloned());
    }

    let new_summary_txt = build_technical_summary(&messages_to_summarize);
    let merged_summary = match existing_summary {
        Some(existing) => merge_summaries(existing, &new_summary_txt),
        None => new_summary_txt,
    };

    let summary_content = format!("{}{}{}", COMPACT_PREAMBLE, merged_summary, COMPACT_INSTRUCTION);
    let summary_msg = ChatMessage::system(&summary_content);

    let mut new_history = vec![history[0].clone()];
    new_history.push(summary_msg);
    new_history.extend(preserved_messages);

    CompactionResult {
        messages: new_history,
        summary: Some(merged_summary),
    }
}

/// Heuristic extraction of "The Mission" from a set of messages.
pub fn extract_memory(messages: &[ChatMessage]) -> SessionMemory {
    let mut mem = SessionMemory::default();
    
    // We only care about the MOST RECENT task boundary.
    // If we find multiple user messages, we only use the last one's intent
    // to avoid "Topic Pollution" (e.g. keeping Tokio context during a Ratatui research).
    let last_user_idx = messages.iter().rposition(|m| m.role == "user");
    
    if let Some(idx) = last_user_idx {
        let m = &messages[idx];
        let content_str = m.content.as_str();
        let limit = 250;
        mem.current_task = content_str.chars().take(limit).collect();
        if content_str.len() > limit { mem.current_task.push_str("..."); }

        // Smart Pivot: Only extract files/learnings from THIS turn's tool calls
        // if the turn has already started. This prevents "Ghost Files" from 
        // lingering in the working set.
        for turn_msg in &messages[idx..] {
            // Working Set (from Tool calls)
            for call in &turn_msg.tool_calls {
                if let Ok(args) = serde_json::from_str::<serde_json::Value>(&call.function.arguments) {
                    if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                        mem.working_set.insert(path.to_string());
                    }
                }
            }
            
            // Learnings
            if turn_msg.role == "tool" {
                let content_str = turn_msg.content.as_str();
                if content_str.contains("Error:") || content_str.contains("Finished") || content_str.contains("Complete") {
                    let lines: Vec<_> = content_str.lines().take(2).collect();
                    mem.learnings.push(lines.join(" "));
                }
            }
        }
    }

    // De-duplicate and cap learnings
    mem.learnings.dedup();
    if mem.learnings.len() > 5 {
        mem.learnings.remove(0);
    }

    mem
}

pub fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    messages.iter().map(|m| m.content.as_str().len() / 4 + 1).sum()
}

pub fn estimate_compactable_tokens(history: &[ChatMessage]) -> usize {
    if history.len() <= 1 {
        0
    } else {
        estimate_tokens(&history[1..])
    }
}

fn build_technical_summary(messages: &[ChatMessage]) -> String {
    let mut lines = vec![
        format!("- Scope: {} earlier turns compacted.", messages.len()),
    ];

    // 1. Extract Key Files
    let mut files = HashSet::new();
    let mut tools = HashSet::new();
    let mut requests = Vec::new();

    for m in messages {
        for word in m.content.as_str().split_whitespace() {
            let clean = word.trim_matches(|c: char| matches!(c, ',' | '.' | ':' | ';' | ')' | '(' | '"' | '\'' | '`'));
            if clean.contains('.') && (clean.contains('/') || clean.contains('\\')) {
                files.insert(clean.to_string());
            }
        }
        if m.role == "user" && !m.content.as_str().trim().is_empty() && requests.len() < 3 {
            requests.push(truncate_summary_line(
                &collapse_inline_whitespace(m.content.as_str()),
                120,
            ));
        }
        for call in &m.tool_calls {
            tools.insert(call.function.name.clone());
        }
    }
    if !files.is_empty() {
        let list: Vec<String> = files.into_iter().take(8).collect();
        lines.push(format!("- Key files referenced: {}.", list.join(", ")));
    }
    if !tools.is_empty() {
        let list: Vec<String> = tools.into_iter().take(8).collect();
        lines.push(format!("- Tools mentioned: {}.", list.join(", ")));
    }
    if !requests.is_empty() {
        lines.push("- Recent user requests:".to_string());
        for request in requests.into_iter().rev() {
            lines.push(format!("  - {}", request));
        }
    }

    // 2. Extract Timeline
    lines.push("- Newly compacted context:".to_string());
    for m in messages.iter().rev().take(6).rev() {
        let content_str = m.content.as_str();
        let preview = if content_str.len() > 100 {
            let mut s: String = content_str.chars().take(97).collect();
            s.push_str("...");
            s
        } else if content_str.is_empty() && !m.tool_calls.is_empty() {
            format!("Executing tools: {:?}", m.tool_calls.iter().map(|c| &c.function.name).collect::<Vec<_>>())
        } else {
            content_str.to_string()
        };
        lines.push(format!("  - {}: {}", m.role, preview.replace('\n', " ").trim()));
    }

    compress_summary_text(&lines.join("\n"))
}

fn merge_summaries(existing: &str, new: &str) -> String {
    compress_summary_text(&format!(
        "Conversation summary:\n- Previously compacted context:\n{}\n- Newly compacted context:\n{}",
        existing.trim(),
        new.trim()
    ))
}

#[derive(Debug, Default)]
struct NormalizedSummary {
    lines: Vec<String>,
    removed_duplicate_lines: usize,
}

fn normalize_summary_lines(summary: &str, max_line_chars: usize) -> NormalizedSummary {
    let mut seen = BTreeSet::new();
    let mut lines = Vec::new();
    let mut removed_duplicate_lines = 0;

    for raw_line in summary.lines() {
        let normalized = collapse_inline_whitespace(raw_line);
        if normalized.is_empty() {
            continue;
        }
        let truncated = truncate_summary_line(&normalized, max_line_chars);
        let dedupe_key = truncated.to_ascii_lowercase();
        if !seen.insert(dedupe_key) {
            removed_duplicate_lines += 1;
            continue;
        }
        lines.push(truncated);
    }

    NormalizedSummary {
        lines,
        removed_duplicate_lines,
    }
}

fn select_summary_line_indexes(lines: &[String], budget: SummaryCompressionBudget) -> Vec<usize> {
    let mut selected = BTreeSet::<usize>::new();

    for priority in 0..=3 {
        for (index, line) in lines.iter().enumerate() {
            if selected.contains(&index) || summary_line_priority(line) != priority {
                continue;
            }
            let candidate = selected
                .iter()
                .map(|selected_index| lines[*selected_index].as_str())
                .chain(std::iter::once(line.as_str()))
                .collect::<Vec<_>>();
            if candidate.len() > budget.max_lines {
                continue;
            }
            if joined_summary_char_count(&candidate) > budget.max_chars {
                continue;
            }
            selected.insert(index);
        }
    }

    selected.into_iter().collect()
}

fn push_summary_line_with_budget(
    lines: &mut Vec<String>,
    line: String,
    budget: SummaryCompressionBudget,
) {
    let candidate = lines
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(line.as_str()))
        .collect::<Vec<_>>();
    if candidate.len() <= budget.max_lines && joined_summary_char_count(&candidate) <= budget.max_chars {
        lines.push(line);
    }
}

fn joined_summary_char_count(lines: &[&str]) -> usize {
    lines.iter().map(|line| line.chars().count()).sum::<usize>() + lines.len().saturating_sub(1)
}

fn summary_line_priority(line: &str) -> usize {
    if line == "Conversation summary:" || is_core_summary_detail(line) {
        0
    } else if line.ends_with(':') {
        1
    } else if line.starts_with("- ") || line.starts_with("  - ") {
        2
    } else {
        3
    }
}

fn is_core_summary_detail(line: &str) -> bool {
    [
        "- Scope:",
        "- Key files referenced:",
        "- Tools mentioned:",
        "- Recent user requests:",
        "- Previously compacted context:",
        "- Newly compacted context:",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

fn collapse_inline_whitespace(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_summary_line(line: &str, max_chars: usize) -> String {
    if max_chars == 0 || line.chars().count() <= max_chars {
        return line.to_string();
    }
    if max_chars == 1 {
        return ".".to_string();
    }
    let mut truncated = line
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}
