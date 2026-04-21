use lazy_static::lazy_static;
use reqwest::header::USER_AGENT;
use serde_json::Value;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

lazy_static! {
    /// Rate limit: 2 seconds between search calls to prevent local IP blocking.
    static ref LAST_SEARCH_CALL: Mutex<Option<Instant>> = Mutex::new(None);
}

/// tool: research_web
///
/// Perform a zero-cost technical search using SearXNG (if configured) or DuckDuckGo Lite.
/// Returns snippets and titles from technical search results.
pub async fn execute_search(args: &Value, searx_url: Option<String>) -> Result<String, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: 'query'".to_string())?;

    // 1. First Attempt: Original Query
    let results = perform_search(query, searx_url.as_deref()).await?;
    if !results.is_empty() && !results.contains("No search results found") {
        return Ok(results);
    }

    // 2. Fallback: Simplified Query if needed
    let tier2 = query
        .replace("2024", "")
        .replace("2025", "")
        .replace("2026", "")
        .replace("crate", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if tier2 != query {
        let second_results = perform_search(&tier2, searx_url.as_deref()).await?;
        if !second_results.is_empty() && !second_results.contains("No search results found") {
            return Ok(second_results);
        }
    }

    Ok(
        "No search results found. All web content was safely sanitized. Try a broader search term."
            .to_string(),
    )
}

/// Proactively strip JSON-like structures and tool-call patterns from web content.
/// This prevents 'Prompt Injection' where a website tries to trick the agent into running commands.
fn sanitize_web_content(text: &str) -> String {
    text.replace("{", " (")
        .replace("}", ") ")
        .replace("[", " (")
        .replace("]", ") ")
        .replace("\"", "'")
        .replace("<script", "[BLOCKED SCRIPT]")
}

async fn perform_search(query: &str, searx_url: Option<&str>) -> Result<String, String> {
    // 1. Try Local SearXNG if configured OR auto-detect on default port (8080)
    let effective_url = searx_url.unwrap_or("http://localhost:8080");

    match perform_searx_search(query, effective_url).await {
        Ok(results) if !results.is_empty() => return Ok(results),
        _ => {
            // Silently fall back to Jina if SearXNG is unreachable or empty.
            // Note: perform_searx_search has its own timeout to prevent blocking.
        }
    }

    // 2. Respect Rate Limits (even for proxy, to be a good citizen)
    let sleep_duration = {
        if let Ok(last_call) = LAST_SEARCH_CALL.lock() {
            last_call.and_then(|instant| {
                let elapsed = instant.elapsed();
                if elapsed < Duration::from_secs(3) {
                    Some(Duration::from_secs(3) - elapsed)
                } else {
                    None
                }
            })
        } else {
            None
        }
    };
    if let Some(duration) = sleep_duration {
        tokio::time::sleep(duration).await;
    }
    if let Ok(mut last_call) = LAST_SEARCH_CALL.lock() {
        *last_call = Some(Instant::now());
    }

    // 3. Construct Jina Search Proxy URL
    // s.jina.ai converts search results into clean markdown for agents.
    let encoded = percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC);
    let search_url = format!("https://s.jina.ai/{}", encoded);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let mut request = client.get(&search_url)
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

    // 3.5 Optional: Inject Jina API Key if available in environment
    if let Ok(key) = std::env::var("JINA_API_KEY") {
        request = request.header("Authorization", format!("Bearer {}", key));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to connect to search proxy: {e}"))?;

    let markdown = response
        .text()
        .await
        .map_err(|e| format!("Failed to read search response: {e}"))?;

    // 4. Safety First: Detect HTML/Captcha leaks and sanitize content.
    if markdown.trim().starts_with("<!doctype html") || markdown.contains("<html") {
        return Err("Search proxy returned raw HTML (possibly a rate limit or captcha). Falling back to internal reasoning.".into());
    }

    Ok(format!("[Source: Jina Search Proxy]\n\n{}", sanitize_web_content(&markdown)))
}

async fn perform_searx_search(query: &str, base_url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build SearXNG client: {e}"))?;

    // Base URL should not have trailing slash for consistency
    let base = base_url.trim_end_matches('/');
    let search_url = format!("{}/search?q={}&format=json", base, urlencoding::encode(query));

    let response = client
        .get(&search_url)
        .header(USER_AGENT, "Hematite-CLI/0.6.0")
        .send()
        .await
        .map_err(|e| format!("SearXNG connection failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("SearXNG returned error: {}", response.status()));
    }

    let json: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse SearXNG JSON: {e}"))?;

    let mut output = String::new();
    output.push_str("[Source: SearXNG (Local/Auto-Detected)]\n\n");
    output.push_str(&format!("# Search results for: {}\n\n", query));

    if let Some(results) = json.get("results").and_then(|r| r.as_array()) {
        for (i, res) in results.iter().take(10).enumerate() {
            let title = res.get("title").and_then(|v| v.as_str()).unwrap_or("No Title");
            let url = res.get("url").and_then(|v| v.as_str()).unwrap_or("#");
            let content = res.get("content").and_then(|v| v.as_str()).unwrap_or("");

            output.push_str(&format!(
                "### {}. [{}]({})\n{}\n\n",
                i + 1,
                title,
                url,
                sanitize_web_content(content)
            ));
        }
    }

    if output.len() < 50 {
        return Ok(String::new());
    }

    Ok(output)
}

/// tool: fetch_docs
///
/// Fetch any URL and convert it into clean, agent-ready Markdown using the Jina Reader proxy.
/// This prevents local IP blocking and ensures structured context for documentation.
pub async fn execute_fetch(args: &Value) -> Result<String, String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: 'url'".to_string())?;

    // Prefix with Jina Reader - it handles the rendering and markdown conversion for us.
    let proxy_url = format!("https://r.jina.ai/{}", url);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let mut request = client.get(&proxy_url)
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

    // 2.5 Optional: Inject Jina API Key if available in environment
    if let Ok(key) = std::env::var("JINA_API_KEY") {
        request = request.header("Authorization", format!("Bearer {}", key));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to connect to documentation proxy: {e}"))?;

    let markdown = response
        .text()
        .await
        .map_err(|e| format!("Failed to read documentation body: {e}"))?;

    Ok(markdown)
}
