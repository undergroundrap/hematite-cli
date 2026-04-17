use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

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
        let today = chrono_lite_date();
        let session_file = log_dir.join(format!("{}.log", today));

        Self {
            log_dir,
            session_file,
        }
    }

    /// Appends a timestamped user turn to the daily log.
    pub fn log_user(&self, input: &str) {
        self.append(&format!("[USER] {}", input));
    }

    /// Appends a timestamped AI response to the daily log.
    pub fn log_agent(&self, output: &str) {
        // Truncate long responses to keep logs scannable for DeepReflect
        let truncated = if output.len() > 500 {
            format!("{}... [TRUNCATED {} bytes]", &output[..500], output.len())
        } else {
            output.to_string()
        };
        self.append(&format!("[AGENT] {}", truncated));
    }

    /// Appends a system event (Vigil tick, Swarm dispatch, etc.)
    pub fn log_system(&self, event: &str) {
        self.append(&format!("[SYSTEM] {}", event));
    }

    fn append(&self, line: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.session_file)
        {
            let _ = writeln!(file, "{}", line);
        }
    }
}

/// Lightweight date string without pulling in the full chrono crate.
/// Returns YYYY-MM-DD using the system clock.
#[allow(dead_code)]
fn chrono_lite_date() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Simple epoch-to-date conversion
    let days = now / 86400;
    let years = (days * 4 + 2) / 1461; // Approximate Gregorian
    let year = 1970 + years;
    let day_of_year = days - (years * 365 + years / 4);
    let month = day_of_year / 30 + 1;
    let day = day_of_year % 30 + 1;

    format!("{:04}-{:02}-{:02}", year, month.min(12), day.min(31))
}
