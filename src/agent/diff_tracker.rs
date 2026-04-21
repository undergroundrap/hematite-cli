use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use similar::TextDiff;

/// Authoritative Turn Diff Tracker.
/// Enables Hematite to proactively capture workspace mutations and
/// generate high-precision unified diffs for human-in-the-loop verification.

pub struct TurnDiffTracker {
    /// Baseline snapshots: Path -> Original Content
    baselines: HashMap<PathBuf, Vec<u8>>,
}

impl TurnDiffTracker {
    pub fn new() -> Self {
        Self {
            baselines: HashMap::new(),
        }
    }

    /// Capture a baseline snapshot of a file if it hasn't been seen yet this turn.
    pub fn on_file_access(&mut self, path: &Path) {
        if !self.baselines.contains_key(path) {
            if path.exists() {
                if let Ok(content) = fs::read(path) {
                    self.baselines.insert(path.to_path_buf(), content);
                }
            } else {
                // For new files, the baseline is empty
                self.baselines.insert(path.to_path_buf(), Vec::new());
            }
        }
    }

    pub fn reset(&mut self) {
        self.baselines.clear();
    }

    /// Generate an aggregated unified diff of all modifications tracked this turn.
    pub fn generate_diff(&self) -> Result<String, String> {
        if self.baselines.is_empty() {
            return Ok(String::new());
        }

        let mut aggregated = String::new();
        let mut sorted_paths: Vec<_> = self.baselines.keys().collect();
        sorted_paths.sort();

        for path in sorted_paths {
            let original_bytes = self.baselines.get(path).unwrap();
            let current_bytes = fs::read(path).unwrap_or_default();

            if original_bytes == &current_bytes {
                continue;
            }

            let original_text = String::from_utf8_lossy(original_bytes);
            let current_text = String::from_utf8_lossy(&current_bytes);

            let diff = TextDiff::from_lines(&original_text, &current_text);
            let rel_path = path.to_string_lossy();
            
            let unified = diff.unified_diff()
                .header(&format!("a/{}", rel_path), &format!("b/{}", rel_path))
                .to_string();
            
            aggregated.push_str(&unified);
            aggregated.push('\n');
        }

        Ok(aggregated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_diff_generation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "original line\n").unwrap();

        let mut tracker = TurnDiffTracker::new();
        tracker.on_file_access(&file_path);

        fs::write(&file_path, "modified line\n").unwrap();

        let diff = tracker.generate_diff().expect("Should have a diff");
        assert!(diff.contains("-original line"));
        assert!(diff.contains("+modified line"));
    }
}
