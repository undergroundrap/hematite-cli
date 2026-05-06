use chrono::Local;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::agent::truncation::safe_head;

/// Persistent transcript logger for the DeepReflect engine.
/// Writes append-only session logs to `.hematite_logs/` so the
/// memory consolidation pass has real signal to synthesize from.
#[allow(dead_code)]
pub struct TranscriptLogger {
    log_dir: PathBuf,
    session_file: PathBuf,
}

#[allow(dead_code)]
impl TranscriptLogger {
    pub fn new() -> Self {
        let log_dir = crate::tools::file_ops::hematite_dir().join("logs");
        let _ = create_dir_all(&log_dir);

        // One file per calendar day — DeepReflect reads these during Phase 2 (Gather)
        let today = Local::now().format("%Y-%m-%d").to_string();
        let session_file = log_dir.join(format!("{}.log", today));

        Self {
            log_dir,
            session_file,
        }
    }

    /// Appends a timestamped user turn to the daily log.
    pub fn log_user(&self, input: &str) {
        self.append_two("[USER] ", input);
    }

    /// Appends a timestamped AI response to the daily log.
    pub fn log_agent(&self, output: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.session_file)
        {
            if output.len() > 500 {
                let _ = write!(file, "[AGENT] {}... [TRUNCATED {} bytes]\n", safe_head(output, 500), output.len());
            } else {
                let _ = write!(file, "[AGENT] {}\n", output);
            }
        }
    }

    /// Appends a system event (Vigil tick, Swarm dispatch, etc.)
    pub fn log_system(&self, event: &str) {
        self.append_two("[SYSTEM] ", event);
    }

    fn append_two(&self, prefix: &str, body: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.session_file)
        {
            let _ = write!(file, "{}{}\n", prefix, body);
        }
    }
}

