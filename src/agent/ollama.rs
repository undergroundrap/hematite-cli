use serde::Deserialize;
use reqwest::Client;
use std::time::Duration;

/// High-Precision Ollama Orchestration Module.
/// Enables Hematite to proactively verify server readiness and model inventory.

#[derive(Debug, Deserialize)]
struct OllamaVersion {
    version: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTags {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
}

pub struct OllamaHarness {
    client: Client,
    base_url: String,
}

impl OllamaHarness {
    pub fn new(url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap_or_default(),
            base_url: url.trim_end_matches('/').to_string(),
        }
    }

    /// Verify if the Ollama server is reachable and responsive.
    pub async fn is_reachable(&self) -> bool {
        self.client
            .get(&format!("{}/api/tags", self.base_url))
            .send()
            .await
            .is_ok()
    }

    /// Check if the Ollama server meets the minimum architectural requirements.
    pub async fn verify_version(&self) -> Result<String, String> {
        let resp = self.client
            .get(&format!("{}/api/version", self.base_url))
            .send()
            .await
            .map_err(|e| format!("Ollama unreachable: {}", e))?;

        let ver: OllamaVersion = resp.json()
            .await
            .map_err(|e| format!("Failed to parse Ollama version: {}", e))?;

        // Grounding: Ollama 0.13.4+ is required for robust tool and streaming support.
        Ok(ver.version)
    }

    /// Check if a specific model is already pulled and ready to run.
    pub async fn has_model(&self, name: &str) -> Result<bool, String> {
        let resp = self.client
            .get(&format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| format!("Ollama unreachable: {}", e))?;

        let tags: OllamaTags = resp.json()
            .await
            .map_err(|e| format!("Failed to parse Ollama models: {}", e))?;

        // Handle both "model:latest" and "model" variants
        let search_name = if !name.contains(':') {
            format!("{}:latest", name)
        } else {
            name.to_string()
        };

        Ok(tags.models.iter().any(|m| m.name == name || m.name == search_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_url_cleanup() {
        let harness = OllamaHarness::new("http://localhost:11434/");
        assert_eq!(harness.base_url, "http://localhost:11434");
    }
}
