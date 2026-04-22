use crate::agent::types::{
    ChatMessage, InferenceEvent, TokenUsage, ToolCallFn, ToolCallResponse, ToolDefinition,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCallResponse>>,
    pub usage: TokenUsage,
    pub finish_reason: Option<String>,
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn call_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        model_override: Option<&str>,
    ) -> Result<ProviderResponse, String>;

    async fn stream(
        &self,
        messages: &[ChatMessage],
        tx: mpsc::Sender<InferenceEvent>,
    ) -> Result<(), Box<dyn std::error::Error>>;

    async fn health_check(&self) -> bool;
    async fn detect_model(&self) -> Result<String, String>;
    async fn detect_context_length(&self) -> usize;
    async fn load_model(&self, model_id: &str) -> Result<(), String>;
    async fn prewarm(&self) -> Result<(), String>;

    async fn get_embedding_model(&self) -> Option<String>;

    fn name(&self) -> &str;
    fn current_model(&self) -> String;
    fn context_length(&self) -> usize;

    fn set_runtime_profile(&mut self, model: &str, context_length: usize);
}

pub struct LmsProvider {
    pub client: Client,
    pub api_url: String,
    pub base_url: String,
    pub model: String,
    pub context_length: usize,
    pub lms: crate::agent::lms::LmsHarness,
}

fn truncate_provider_error_body(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let compact: String = trimmed.chars().take(240).collect();
    if trimmed.chars().count() > 240 {
        format!("{}...", compact)
    } else {
        compact
    }
}

#[async_trait]
impl ModelProvider for LmsProvider {
    async fn call_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        model_override: Option<&str>,
    ) -> Result<ProviderResponse, String> {
        let model = model_override.unwrap_or(&self.model).to_string();
        let request = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": 0.2,
            "stream": false,
            "tools": if tools.is_empty() { None } else { Some(tools) },
        });

        let mut last_err = String::new();
        for attempt in 0..3u32 {
            match self.client.post(&self.api_url).json(&request).send().await {
                Ok(res) if res.status().is_success() => {
                    let body: Value = res
                        .json()
                        .await
                        .map_err(|e| format!("LMS parse error: {}", e))?;
                    let choice = body["choices"].get(0).ok_or("Empty choice from LMS")?;
                    let message = &choice["message"];
                    let content = message["content"].as_str().map(|s| s.to_string());
                    let tool_calls: Option<Vec<ToolCallResponse>> =
                        serde_json::from_value(message["tool_calls"].clone()).ok();
                    let usage: TokenUsage =
                        serde_json::from_value(body["usage"].clone()).unwrap_or_default();
                    let finish_reason = choice["finish_reason"].as_str().map(|s| s.to_string());
                    return Ok(ProviderResponse {
                        content,
                        tool_calls,
                        usage,
                        finish_reason,
                    });
                }
                Ok(res) => {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    let body_note = truncate_provider_error_body(&body);
                    last_err = if body_note.is_empty() {
                        format!("HTTP {}", status)
                    } else {
                        format!("HTTP {} | {}", status, body_note)
                    };
                }
                Err(e) => {
                    last_err = e.to_string();
                }
            }
            if attempt < 2 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
        Err(format!("LMS unreachable: {}", last_err))
    }

    async fn stream(
        &self,
        messages: &[ChatMessage],
        tx: mpsc::Sender<InferenceEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.2,
            "stream": true,
        });

        let res = self
            .client
            .post(&self.api_url)
            .json(&request)
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(format!("LMS stream error: {}", res.status()).into());
        }

        use futures::StreamExt;
        let mut stream = res.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        break;
                    }
                    if let Ok(v) = serde_json::from_str::<Value>(data) {
                        if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
                            let _ = tx.send(InferenceEvent::Token(delta.to_string())).await;
                        }
                    }
                }
            }
        }
        let _ = tx.send(InferenceEvent::Done).await;
        Ok(())
    }

    async fn health_check(&self) -> bool {
        if self.lms.is_server_responding(&self.base_url).await {
            return true;
        }
        if self.lms.binary_path.is_some() {
            let _ = self.lms.ensure_server_running();
            tokio::time::sleep(Duration::from_millis(1500)).await;
            return self.lms.is_server_responding(&self.base_url).await;
        }
        false
    }

    async fn detect_model(&self) -> Result<String, String> {
        let url = format!("{}/api/v0/models", self.base_url);
        if let Ok(res) = self.client.get(&url).send().await {
            if res.status().is_success() {
                let body: Value = res.json().await.map_err(|e| e.to_string())?;
                if let Some(data) = body["data"].as_array() {
                    for m in data {
                        if m["type"].as_str() == Some("chat")
                            && m["state"].as_str() == Some("loaded")
                        {
                            return Ok(m["id"].as_str().unwrap_or_default().to_string());
                        }
                    }
                }
            }
        }
        let url_v1 = format!("{}/v1/models", self.base_url);
        let resp_v1 = self
            .client
            .get(&url_v1)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body_v1: Value = resp_v1.json().await.map_err(|e| e.to_string())?;
        if let Some(data) = body_v1["data"].as_array() {
            if let Some(first) = data.iter().find(|m| {
                !m["id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains("embed")
            }) {
                return Ok(first["id"].as_str().unwrap_or_default().to_string());
            }
        }
        Ok(String::new())
    }

    async fn detect_context_length(&self) -> usize {
        let url = format!("{}/api/v0/models", self.base_url);
        if let Ok(res) = self.client.get(&url).send().await {
            if res.status().is_success() {
                let body: Value = res.json().await.unwrap_or_default();
                if let Some(data) = body["data"].as_array() {
                    for m in data {
                        if m["state"].as_str() == Some("loaded") {
                            if let Some(len) = m["loaded_context_length"].as_u64() {
                                return len as usize;
                            }
                            if let Some(len) = m["context_length"].as_u64() {
                                return len as usize;
                            }
                            if let Some(len) = m["max_context_length"].as_u64() {
                                return len as usize;
                            }
                        }
                    }
                }
            }
        }
        32768 // Fallback
    }

    async fn load_model(&self, model_id: &str) -> Result<(), String> {
        if self.lms.binary_path.is_some() {
            if self.lms.load_model(model_id).is_ok() {
                return Ok(());
            }
        }
        let payload = serde_json::json!({
            "model": model_id,
            "messages": [{"role": "system", "content": "System boot"}],
            "max_tokens": 1,
            "stream": false
        });
        match self.client.post(&self.api_url).json(&payload).send().await {
            Ok(res) if res.status().is_success() => Ok(()),
            _ => Err("Model load failed".into()),
        }
    }

    async fn prewarm(&self) -> Result<(), String> {
        let payload = serde_json::json!({
            "model": self.model,
            "messages": [{"role": "system", "content": "Hematite BootSequence"}],
            "max_tokens": 1,
            "stream": false
        });
        let _ = self.client.post(&self.api_url).json(&payload).send().await;
        Ok(())
    }

    async fn get_embedding_model(&self) -> Option<String> {
        let url = format!("{}/api/v0/models", self.base_url);
        if let Ok(res) = self.client.get(&url).send().await {
            if let Ok(body) = res.json::<Value>().await {
                if let Some(data) = body["data"].as_array() {
                    return data
                        .iter()
                        .find(|m| {
                            m["type"].as_str() == Some("embeddings")
                                && m["state"].as_str() == Some("loaded")
                        })
                        .map(|m| m["id"].as_str().unwrap_or_default().to_string());
                }
            }
        }
        None
    }

    fn name(&self) -> &str {
        "LM Studio"
    }
    fn current_model(&self) -> String {
        self.model.clone()
    }
    fn context_length(&self) -> usize {
        self.context_length
    }
    fn set_runtime_profile(&mut self, model: &str, context_length: usize) {
        self.model = model.to_string();
        self.context_length = context_length;
    }
}

pub struct OllamaProvider {
    pub client: Client,
    pub base_url: String,
    pub model: String,
    pub ollama: crate::agent::ollama::OllamaHarness,
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    async fn call_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        model_override: Option<&str>,
    ) -> Result<ProviderResponse, String> {
        let model = model_override.unwrap_or(&self.model).to_string();
        let url = format!("{}/api/chat", self.base_url);
        let request = serde_json::json!({
            "model": model, "messages": messages, "stream": false,
            "tools": if tools.is_empty() { None } else { Some(tools) },
        });
        let res = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("Ollama error: {}", res.status()));
        }
        let body: Value = res.json().await.map_err(|e| e.to_string())?;
        let message = &body["message"];
        let content = message["content"].as_str().map(|s| s.to_string());
        let tool_calls = if let Some(calls) = message["tool_calls"].as_array() {
            let mut mapped = Vec::new();
            for (i, c) in calls.iter().enumerate() {
                mapped.push(ToolCallResponse {
                    id: format!("call_{}", i),
                    call_type: "function".to_string(),
                    function: ToolCallFn {
                        name: c["function"]["name"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        arguments: c["function"]["arguments"].clone(),
                    },
                    index: Some(i as i32),
                });
            }
            Some(mapped)
        } else {
            None
        };
        let usage = TokenUsage {
            prompt_tokens: body["prompt_eval_count"].as_u64().unwrap_or(0) as usize,
            completion_tokens: body["eval_count"].as_u64().unwrap_or(0) as usize,
            ..Default::default()
        };
        Ok(ProviderResponse {
            content,
            tool_calls,
            usage,
            finish_reason: Some("stop".to_string()),
        })
    }

    async fn stream(
        &self,
        messages: &[ChatMessage],
        tx: mpsc::Sender<InferenceEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/api/chat", self.base_url);
        let request =
            serde_json::json!({ "model": self.model, "messages": messages, "stream": true });
        let res = self.client.post(&url).json(&request).send().await?;
        use futures::StreamExt;
        let mut stream = res.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if let Ok(v) = serde_json::from_slice::<Value>(&chunk) {
                if let Some(delta) = v["message"]["content"].as_str() {
                    let _ = tx.send(InferenceEvent::Token(delta.to_string())).await;
                }
                if v["done"].as_bool().unwrap_or(false) {
                    break;
                }
            }
        }
        let _ = tx.send(InferenceEvent::Done).await;
        Ok(())
    }

    async fn health_check(&self) -> bool {
        self.ollama.is_reachable().await
    }
    async fn detect_model(&self) -> Result<String, String> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body: Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(models) = body["models"].as_array() {
            if let Some(first) = models.first() {
                return Ok(first["name"].as_str().unwrap_or_default().to_string());
            }
        }
        Ok(String::new())
    }
    async fn detect_context_length(&self) -> usize {
        8192
    }
    async fn load_model(&self, _model_id: &str) -> Result<(), String> {
        Ok(())
    }
    async fn prewarm(&self) -> Result<(), String> {
        Ok(())
    }
    async fn get_embedding_model(&self) -> Option<String> {
        None
    }

    fn name(&self) -> &str {
        "Ollama"
    }
    fn current_model(&self) -> String {
        self.model.clone()
    }
    fn context_length(&self) -> usize {
        8192
    }
    fn set_runtime_profile(&mut self, model: &str, _context_length: usize) {
        self.model = model.to_string();
    }
}
