use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// LM Studio CLI Harness for automated lifecycle management.
/// Ports the "LMS Mastery" patterns from Codex-RS to ensure
/// Hematite can auto-start and auto-load models.
pub struct LmsHarness {
    pub binary_path: Option<PathBuf>,
}

impl LmsHarness {
    pub fn new() -> Self {
        Self {
            binary_path: Self::find_lms(),
        }
    }

    /// Locate the 'lms' binary in PATH or standard installation directories.
    fn find_lms() -> Option<PathBuf> {
        // 1. Try PATH via which
        if let Ok(path) = which::which("lms") {
            return Some(path);
        }

        // 2. Platform-specific fallbacks
        let home = if cfg!(windows) {
            std::env::var("USERPROFILE").ok()
        } else {
            std::env::var("HOME").ok()
        };

        if let Some(h) = home {
            let bin_name = if cfg!(windows) { "lms.exe" } else { "lms" };
            let fallback = PathBuf::from(h)
                .join(".lmstudio")
                .join("bin")
                .join(bin_name);
            if fallback.exists() {
                return Some(fallback);
            }
        }

        None
    }

    /// Check if the LM Studio server is responding on the expected port.
    pub async fn is_server_responding(&self, base_url: &str) -> bool {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(1000))
            .build()
            .unwrap_or_default();

        let url = format!("{}/models", base_url.trim_end_matches('/'));
        match client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Attempt to start the LM Studio server if it's not responding.
    pub fn ensure_server_running(&self) -> io::Result<()> {
        let Some(ref lms) = self.binary_path else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "lms CLI not found"));
        };

        // We run this detached/background-ish so it doesn't block Hematite startup.
        // LM Studio 'server start' is idempotent.
        let status = Command::new(lms)
            .args(["server", "start"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to start lms server",
            ));
        }

        Ok(())
    }

    /// Get a list of models currently known to LM Studio.
    pub fn list_models(&self) -> io::Result<Vec<String>> {
        let Some(ref lms) = self.binary_path else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "lms CLI not found"));
        };

        let output = Command::new(lms).args(["ls"]).output()?;

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to list models via lms",
            ));
        }

        let out_str = String::from_utf8_lossy(&output.stdout);
        let models = out_str
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with("NAME")) // Skip header
            .filter_map(|l| l.split_whitespace().next())
            .map(|s| s.to_string())
            .collect();

        Ok(models)
    }

    /// Get a list of models currently loaded in memory.
    pub fn list_loaded_models(&self) -> io::Result<Vec<String>> {
        let Some(ref lms) = self.binary_path else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "lms CLI not found"));
        };

        let output = Command::new(lms).args(["ps"]).output()?;

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to list loaded models via lms",
            ));
        }

        let out_str = String::from_utf8_lossy(&output.stdout);
        let models = out_str
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with("NAME"))
            .filter_map(|line| line.split_whitespace().next())
            .map(|value| value.to_string())
            .collect();

        Ok(models)
    }

    /// Load a specific model into the server.
    pub fn load_model(&self, model_id: &str) -> io::Result<()> {
        let Some(ref lms) = self.binary_path else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "lms CLI not found"));
        };

        let status = Command::new(lms)
            .args(["load", model_id])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to load model: {}", model_id),
            ));
        }

        Ok(())
    }

    /// Unload a specific model from the server.
    pub fn unload_model(&self, model_id: &str) -> io::Result<()> {
        let Some(ref lms) = self.binary_path else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "lms CLI not found"));
        };

        let status = Command::new(lms)
            .args(["unload", model_id])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to unload model: {}", model_id),
            ));
        }

        Ok(())
    }

    /// Unload all loaded models from the server.
    pub fn unload_all_models(&self) -> io::Result<()> {
        let Some(ref lms) = self.binary_path else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "lms CLI not found"));
        };

        let status = Command::new(lms)
            .args(["unload", "--all"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to unload all models",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lms_discovery() {
        let harness = LmsHarness::new();
        // We can't guarantee 'lms' is on the test machine, but we can verify the fallback path logic.
        if let Some(path) = harness.binary_path {
            assert!(path.exists());
        }
    }
}
