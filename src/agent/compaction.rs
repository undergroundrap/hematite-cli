use crate::agent::inference::ChatMessage;

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

const COMPACT_PREAMBLE: &str = "## CONTEXT SUMMARY (RECURSIVE CHAIN)\n\
    This session is being continued from a previous conversation. The summary below covers the earlier portion.\n\n";
const COMPACT_INSTRUCTION: &str = "\n\nIMPORTANT: Resume directly from the last message. Do not recap or acknowledge this summary.";

/// Layer 6: Structured Session Memory.
/// Preserves the "Mission Context" across compactions.
#[derive(Debug, Clone, Default)]
pub struct SessionMemory {
    pub current_task: String,
    pub working_set: std::collections::HashSet<String>,
    pub learnings: Vec<String>,
}

impl SessionMemory {
    pub fn to_prompt(&self) -> String {
        let mut s = format!("- **Active Task**: {}\n", self.current_task);
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
        s
    }

    pub fn clear(&mut self) {
        self.current_task = "Ready for new mission.".to_string();
        self.working_set.clear();
        self.learnings.clear();
    }
}

/// Returns true when history is large enough to warrant compaction.
/// Pass the model's context_length and current vram_ratio for adaptive thresholds.
pub fn should_compact(history: &[ChatMessage], context_length: usize, vram_ratio: f64) -> bool {
    let config = CompactionConfig::adaptive(context_length, vram_ratio);
    history.len() > config.preserve_recent_messages + 5
        || estimate_tokens(history) > config.max_estimated_tokens
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

fn build_technical_summary(messages: &[ChatMessage]) -> String {
    let mut lines = vec![
        format!("- Scope: {} earlier turns compacted.", messages.len()),
    ];

    // 1. Extract Key Files
    let mut files = std::collections::HashSet::new();
    for m in messages {
        for word in m.content.as_str().split_whitespace() {
            let clean = word.trim_matches(|c: char| matches!(c, ',' | '.' | ':' | ';' | ')' | '(' | '"' | '\'' | '`'));
            if clean.contains('.') && (clean.contains('/') || clean.contains('\\')) {
                files.insert(clean.to_string());
            }
        }
    }
    if !files.is_empty() {
        let list: Vec<String> = files.into_iter().take(8).collect();
        lines.push(format!("- Files Active: {}.", list.join(", ")));
    }

    // 2. Extract Timeline
    lines.push("- Technical Timeline:".to_string());
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

    lines.join("\n")
}

fn merge_summaries(existing: &str, new: &str) -> String {
    format!(
        "### Previously Compacted Context\n{}\n\n### Newly Compacted Context\n{}",
        existing,
        new
    )
}
