use std::process::Command;
use crate::agent::config::HematiteConfig;
use tokio::time::{timeout, Duration};

/// Checks if SearXNG is responding at the configured URL.
pub async fn is_searx_responding(url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap_or_default();

    match timeout(Duration::from_millis(600), client.get(url).send()).await {
        Ok(Ok(resp)) => resp.status().is_success() || resp.status().as_u16() == 403, // 403 is fine, SearXNG might block generic UA but it's alive
        _ => false,
    }
}

/// Automatically boots SearXNG if it's offline and the user has auto-start enabled.
pub async fn boot_searx_if_needed(config: &HematiteConfig) {
    if !config.auto_start_searx {
        return;
    }

    let url = config.searx_url.as_deref().unwrap_or("http://localhost:8080");
    
    // Check if it's already alive.
    if is_searx_responding(url).await {
        return;
    }

    // It's offline. Try to boot it via the setup script.
    // We run it with powershell -ExecutionPolicy Bypass -File scripts/setup-searxng.ps1
    let script_path = "scripts/setup-searxng.ps1";
    if std::path::Path::new(script_path).exists() {
        let _ = Command::new("powershell")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(script_path)
            .spawn();
    }
}
