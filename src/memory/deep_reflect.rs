/// DeepReflect v2 — idle-triggered session memory synthesis.
///
/// After 5 minutes of TUI inactivity, reads the day's transcript and calls
/// the local model to extract structured memories: files changed, decisions
/// made, patterns observed, next steps.
///
/// Outputs are written to `.hematite/memories/<YYYY-MM-DD>.md`.
/// These files are automatically injected into the system prompt at startup
/// so Hematite knows what you were working on when you come back.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::time::{sleep, Duration};

pub fn spawn_deep_reflect_system(
    last_interaction: Arc<Mutex<Instant>>,
    engine: Arc<crate::agent::inference::InferenceEngine>,
) {
    tokio::spawn(async move {
        let mut last_synthesized_hash: u64 = 0;

        loop {
            sleep(Duration::from_secs(60)).await;

            let idle = { last_interaction.lock().unwrap().elapsed() };
            if idle < Duration::from_secs(300) {
                continue;
            }

            let today = date_string();
            let log_path = crate::tools::file_ops::hematite_dir().join("logs").join(format!("{}.log", today));
            if !log_path.exists() {
                continue;
            }

            let Ok(log_content) = std::fs::read_to_string(&log_path) else {
                continue;
            };
            if log_content.trim().is_empty() {
                continue;
            }

            // Skip if we already synthesized this exact content.
            let hash = fast_hash(&log_content);
            if hash == last_synthesized_hash {
                continue;
            }

            // Cap transcript to avoid blowing the context window.
            let transcript_slice = if log_content.len() > 8_000 {
                &log_content[log_content.len() - 8_000..]
            } else {
                &log_content
            };

            let prompt = format!(
                "You are a memory synthesizer for a coding agent. Analyze this session transcript \
                 and extract the key information in structured form.\n\n\
                 SESSION TRANSCRIPT:\n{}\n\n\
                 Output ONLY this structure (no preamble, no explanation):\n\
                 ## Files Modified\n\
                 - list each file that was created, edited, or deleted\n\n\
                 ## Decisions Made\n\
                 - list key architectural or design decisions\n\n\
                 ## Patterns Observed\n\
                 - list any recurring issues, model behaviour patterns, or code patterns noted\n\n\
                 ## Next Steps\n\
                 - list any unfinished work, TODOs, or follow-up tasks mentioned\n\n\
                 Be concise. Maximum 250 words total.",
                transcript_slice
            );

            if let Ok(summary) = engine.generate_task(&prompt, true).await {
                let memory_dir = PathBuf::from(".hematite").join("memories");
                let _ = std::fs::create_dir_all(&memory_dir);
                let mem_file = memory_dir.join(format!("{}.md", today));

                let content = format!(
                    "# Session Memory — {}\n_Synthesized by DeepReflect after idle period_\n\n{}\n",
                    today, summary
                );
                let _ = std::fs::write(&mem_file, content);
                last_synthesized_hash = hash;
            }

            // Reset idle timer so we don't re-synthesize immediately.
            *last_interaction.lock().unwrap() = Instant::now();
        }
    });
}

/// Load recent memory files (last 3 days) to inject into the system prompt.
/// Returns a formatted string ready for system prompt injection, or empty string.
pub fn load_recent_memories() -> String {
    let memory_dir = PathBuf::from(".hematite").join("memories");
    if !memory_dir.exists() {
        return String::new();
    }

    // Get last 3 memory files sorted by name (date-named, so lexicographic = chronological).
    let mut files: Vec<_> = std::fs::read_dir(&memory_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .collect();
    files.sort_by_key(|e| e.file_name());
    files.reverse(); // newest first

    let mut result = String::new();
    let mut total = 0usize;
    const MAX_TOTAL: usize = 3_000;

    for entry in files.into_iter().take(3) {
        let Ok(content) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        if content.trim().is_empty() {
            continue;
        }
        let snippet = if content.len() > 1_000 {
            format!("{}...", &content[..1_000])
        } else {
            content
        };
        if total + snippet.len() > MAX_TOTAL {
            break;
        }
        total += snippet.len();
        result.push_str(&snippet);
        result.push('\n');
    }

    if result.is_empty() {
        return String::new();
    }
    format!("\n\n# Cross-Session Memory (DeepReflect)\n{}", result)
}

/// Fast non-cryptographic hash for change detection.
fn fast_hash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Returns today's date as YYYY-MM-DD using the system clock.
fn date_string() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Days since epoch
    let days = secs / 86_400;
    // Gregorian approximation (accurate for ~100 years from 1970)
    let mut year = 1970u64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let months = [
        31u64,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for days_in_month in &months {
        if remaining < *days_in_month {
            break;
        }
        remaining -= days_in_month;
        month += 1;
    }
    let day = remaining + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
