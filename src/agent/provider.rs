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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderModelKind {
    Any,
    Coding,
    Embed,
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
    async fn load_model_with_context(
        &self,
        model_id: &str,
        context_length: Option<usize>,
    ) -> Result<(), String>;
    async fn load_embedding_model(&self, model_id: &str) -> Result<(), String>;
    async fn list_models(
        &self,
        kind: ProviderModelKind,
        loaded_only: bool,
    ) -> Result<Vec<String>, String>;
    async fn unload_model(&self, model_id: Option<&str>, all: bool) -> Result<String, String>;
    async fn unload_embedding_model(&self, model_id: Option<&str>) -> Result<String, String>;
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

fn lms_message_to_json(message: &ChatMessage) -> Value {
    let content = match &message.content {
        crate::agent::types::MessageContent::Text(text) => Value::String(text.clone()),
        crate::agent::types::MessageContent::Parts(parts) => serde_json::to_value(parts)
            .unwrap_or_else(|_| Value::String(message.content.as_str().to_string())),
    };

    match message.role.as_str() {
        "assistant" => {
            let mut base = serde_json::json!({
                "role": "assistant",
                "content": content,
            });
            if let Some(calls) = &message.tool_calls {
                let tool_calls: Vec<Value> = calls
                    .iter()
                    .map(|call| {
                        let arguments = if call.function.arguments.is_string() {
                            call.function.arguments.clone()
                        } else {
                            Value::String(call.function.arguments.to_string())
                        };
                        serde_json::json!({
                            "id": call.id,
                            "type": call.call_type,
                            "function": {
                                "name": call.function.name,
                                "arguments": arguments,
                            }
                        })
                    })
                    .collect();
                if let Some(obj) = base.as_object_mut() {
                    obj.insert("tool_calls".to_string(), Value::Array(tool_calls));
                }
            }
            base
        }
        "tool" => serde_json::json!({
            "role": "tool",
            "content": content,
            "tool_call_id": message.tool_call_id.clone().unwrap_or_default(),
        }),
        _ => serde_json::json!({
            "role": message.role,
            "content": content,
        }),
    }
}

fn lms_messages_payload(messages: &[ChatMessage]) -> Vec<Value> {
    messages.iter().map(lms_message_to_json).collect()
}

fn push_unique_model(models: &mut Vec<String>, candidate: &str) {
    let trimmed = candidate.trim();
    if !trimmed.is_empty() && !models.iter().any(|existing| existing == trimmed) {
        models.push(trimmed.to_string());
    }
}

fn matches_lms_model_kind(kind: ProviderModelKind, raw_type: &str) -> bool {
    match kind {
        ProviderModelKind::Any => true,
        ProviderModelKind::Coding => raw_type != "embedding" && raw_type != "embeddings",
        ProviderModelKind::Embed => raw_type == "embedding" || raw_type == "embeddings",
    }
}

fn looks_like_embedding_model_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("embed")
        || lower.contains("embedding")
        || lower.contains("minilm")
        || lower.contains("bge")
        || lower.contains("e5")
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
        let payload_messages = lms_messages_payload(messages);
        let request = serde_json::json!({
            "model": model,
            "messages": payload_messages,
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
        let base = self.base_url.trim_end_matches('/').trim_end_matches("/v1");
        let url = format!("{}/api/v0/models", base);
        if let Ok(res) = self.client.get(&url).send().await {
            if res.status().is_success() {
                let body: Value = res.json().await.map_err(|e| e.to_string())?;
                if let Some(data) = body["data"].as_array() {
                    for m in data {
                        let m_type = m["type"].as_str().unwrap_or_default();
                        if (m_type == "chat" || m_type == "vlm" || m_type == "llm")
                            && m["state"].as_str() == Some("loaded")
                        {
                            return Ok(m["id"].as_str().unwrap_or_default().to_string());
                        }
                    }
                }
            }
        }
        let url_v1 = format!("{}/v1/models", base);
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
        let base = self.base_url.trim_end_matches('/').trim_end_matches("/v1");
        let url = format!("{}/api/v0/models", base);
        if let Ok(res) = self.client.get(&url).send().await {
            if res.status().is_success() {
                let body: Value = res.json().await.unwrap_or_default();
                if let Some(data) = body["data"].as_array() {
                    for m in data {
                        let m_type = m["type"].as_str().unwrap_or_default();
                        if (m_type == "chat" || m_type == "vlm" || m_type == "llm")
                            && m["state"].as_str() == Some("loaded")
                        {
                            // Try multiple possible field names and nested locations
                            let fields = [
                                "loaded_context_length",
                                "context_length",
                                "max_context_length",
                                "contextLength",
                            ];

                            // Check top-level first
                            for field in fields {
                                if let Some(val) = m.get(field) {
                                    if let Some(len) = val.as_u64() {
                                        return len as usize;
                                    }
                                    if let Some(s) = val.as_str() {
                                        if let Ok(len) = s.parse::<usize>() {
                                            return len;
                                        }
                                    }
                                }
                            }

                            // Check "stats" object
                            if let Some(stats) = m.get("stats") {
                                for field in fields {
                                    if let Some(val) = stats.get(field) {
                                        if let Some(len) = val.as_u64() {
                                            return len as usize;
                                        }
                                        if let Some(s) = val.as_str() {
                                            if let Ok(len) = s.parse::<usize>() {
                                                return len;
                                            }
                                        }
                                    }
                                }
                            }

                            // Check "config" object
                            if let Some(config) = m.get("config") {
                                for field in fields {
                                    if let Some(val) = config.get(field) {
                                        if let Some(len) = val.as_u64() {
                                            return len as usize;
                                        }
                                        if let Some(s) = val.as_str() {
                                            if let Ok(len) = s.parse::<usize>() {
                                                return len;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        0
    }

    async fn load_model(&self, model_id: &str) -> Result<(), String> {
        self.load_model_with_context(model_id, None).await
    }

    async fn load_model_with_context(
        &self,
        model_id: &str,
        context_length: Option<usize>,
    ) -> Result<(), String> {
        let mut payload = serde_json::json!({ "model": model_id });
        if let Some(ctx) = context_length {
            payload["context_length"] = serde_json::json!(ctx);
        }

        let load_url = format!("{}/api/v1/models/load", self.base_url);
        if let Ok(res) = self.client.post(&load_url).json(&payload).send().await {
            if res.status().is_success() {
                return Ok(());
            }
            let body = res.text().await.unwrap_or_default();
            let body_note = truncate_provider_error_body(&body);
            if !body_note.is_empty() {
                return Err(format!("Model load failed: {}", body_note));
            }
        }

        if context_length.is_none()
            && self.lms.binary_path.is_some()
            && self.lms.load_model(model_id).is_ok()
        {
            return Ok(());
        }

        let payload = serde_json::json!({
            "model": model_id,
            "messages": [{"role": "system", "content": "System boot"}],
            "max_tokens": 1,
            "stream": false
        });
        match self.client.post(&self.api_url).json(&payload).send().await {
            Ok(res) if res.status().is_success() => Ok(()),
            Ok(res) => Err(format!("Model load failed: HTTP {}", res.status())),
            Err(e) => Err(format!("Model load failed: {}", e)),
        }
    }

    async fn load_embedding_model(&self, model_id: &str) -> Result<(), String> {
        self.load_model(model_id).await
    }

    async fn list_models(
        &self,
        kind: ProviderModelKind,
        loaded_only: bool,
    ) -> Result<Vec<String>, String> {
        let mut models = Vec::new();

        if loaded_only {
            let url = format!("{}/api/v0/models", self.base_url);
            if let Ok(res) = self.client.get(&url).send().await {
                if res.status().is_success() {
                    let body: Value = res.json().await.map_err(|e| e.to_string())?;
                    if let Some(data) = body["data"].as_array() {
                        for model in data {
                            if model["state"].as_str() != Some("loaded") {
                                continue;
                            }
                            let raw_type = model["type"].as_str().unwrap_or_default();
                            if !matches_lms_model_kind(kind, raw_type) {
                                continue;
                            }
                            if let Some(id) = model["id"].as_str() {
                                push_unique_model(&mut models, id);
                            }
                        }
                    }
                }
            }

            if models.is_empty()
                && self.lms.binary_path.is_some()
                && kind != ProviderModelKind::Embed
            {
                if let Ok(cli_models) = self.lms.list_loaded_models() {
                    for model in cli_models {
                        push_unique_model(&mut models, &model);
                    }
                }
            }
            return Ok(models);
        }

        let url = format!("{}/api/v1/models", self.base_url);
        if let Ok(res) = self.client.get(&url).send().await {
            if res.status().is_success() {
                let body: Value = res.json().await.map_err(|e| e.to_string())?;
                if let Some(data) = body["data"].as_array() {
                    for model in data {
                        let raw_type = model["type"].as_str().unwrap_or_default();
                        if !matches_lms_model_kind(kind, raw_type) {
                            continue;
                        }
                        if let Some(id) = model["id"].as_str() {
                            push_unique_model(&mut models, id);
                        }
                    }
                }
            }
        }

        if models.is_empty() && self.lms.binary_path.is_some() && kind != ProviderModelKind::Embed {
            if let Ok(cli_models) = self.lms.list_models() {
                for model in cli_models {
                    push_unique_model(&mut models, &model);
                }
            }
        }

        Ok(models)
    }

    async fn unload_model(&self, model_id: Option<&str>, all: bool) -> Result<String, String> {
        if all {
            let loaded = self.list_models(ProviderModelKind::Any, true).await?;
            if loaded.is_empty() {
                return Ok("No LM Studio models are currently loaded.".to_string());
            }

            if self.lms.binary_path.is_some() && self.lms.unload_all_models().is_ok() {
                return Ok(format!("Unloaded {} LM Studio model(s).", loaded.len()));
            }

            let unload_url = format!("{}/api/v1/models/unload", self.base_url);
            let mut unloaded = 0usize;
            let mut failures = Vec::new();
            for instance_id in loaded {
                match self
                    .client
                    .post(&unload_url)
                    .json(&serde_json::json!({ "instance_id": instance_id }))
                    .send()
                    .await
                {
                    Ok(res) if res.status().is_success() => unloaded += 1,
                    Ok(res) => failures.push(format!("{} ({})", instance_id, res.status())),
                    Err(e) => failures.push(format!("{} ({})", instance_id, e)),
                }
            }
            if failures.is_empty() {
                return Ok(format!("Unloaded {} LM Studio model(s).", unloaded));
            }
            return Err(format!(
                "Unloaded {} LM Studio model(s), but some unloads failed: {}",
                unloaded,
                failures.join(", ")
            ));
        }

        let target = model_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Missing model ID to unload.".to_string())?;

        let unload_url = format!("{}/api/v1/models/unload", self.base_url);
        match self
            .client
            .post(&unload_url)
            .json(&serde_json::json!({ "instance_id": target }))
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => {
                Ok(format!("Unloaded LM Studio model `{}`.", target))
            }
            Ok(res) => {
                let status = res.status();
                let body = res.text().await.unwrap_or_default();
                let body_note = truncate_provider_error_body(&body);
                if self.lms.binary_path.is_some() && self.lms.unload_model(target).is_ok() {
                    Ok(format!("Unloaded LM Studio model `{}`.", target))
                } else if body_note.is_empty() {
                    Err(format!("LM Studio unload failed: HTTP {}", status))
                } else {
                    Err(format!(
                        "LM Studio unload failed: HTTP {} | {}",
                        status, body_note
                    ))
                }
            }
            Err(err) => {
                if self.lms.binary_path.is_some() && self.lms.unload_model(target).is_ok() {
                    Ok(format!("Unloaded LM Studio model `{}`.", target))
                } else {
                    Err(format!("LM Studio unload failed: {}", err))
                }
            }
        }
    }

    async fn unload_embedding_model(&self, model_id: Option<&str>) -> Result<String, String> {
        self.unload_model(model_id, false).await
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
    pub context_length: usize,
    pub embed_model: std::sync::Arc<std::sync::RwLock<Option<String>>>,
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
        let running_url = format!("{}/api/ps", self.base_url);
        if let Ok(resp) = self.client.get(&running_url).send().await {
            let body: Value = resp.json().await.map_err(|e| e.to_string())?;
            if let Some(models) = body["models"].as_array() {
                if let Some(first) = models.first() {
                    let name = first["name"]
                        .as_str()
                        .or_else(|| first["model"].as_str())
                        .unwrap_or_default();
                    return Ok(name.to_string());
                }
                return Ok(String::new());
            }
        }

        if !self.model.trim().is_empty() {
            return Ok(self.model.clone());
        }

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
        let running_url = format!("{}/api/ps", self.base_url);
        if let Ok(resp) = self.client.get(&running_url).send().await {
            if let Ok(body) = resp.json::<Value>().await {
                if let Some(models) = body["models"].as_array() {
                    if let Some(first) = models.first() {
                        if let Some(context_length) = first["context_length"].as_u64() {
                            return context_length as usize;
                        }
                    }
                }
            }
        }
        self.context_length
    }
    async fn load_model(&self, _model_id: &str) -> Result<(), String> {
        self.load_model_with_context(_model_id, None).await
    }
    async fn load_model_with_context(
        &self,
        model_id: &str,
        context_length: Option<usize>,
    ) -> Result<(), String> {
        if !self.ollama.has_model(model_id).await? {
            return Err(format!(
                "Ollama model `{}` is not pulled locally. Run `ollama pull {}` first.",
                model_id, model_id
            ));
        }
        let url = format!("{}/api/generate", self.base_url);
        let request = serde_json::json!({
            "model": model_id,
            "prompt": "Hematite runtime warmup",
            "stream": false,
            "keep_alive": "30m",
            "options": {
                "num_ctx": context_length.unwrap_or(self.context_length.max(4096))
            }
        });
        let res = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = res.text().await.unwrap_or_default();
            let body_note = truncate_provider_error_body(&body);
            if body_note.is_empty() {
                Err(format!("Ollama load failed: HTTP {}", status))
            } else {
                Err(format!(
                    "Ollama load failed: HTTP {} | {}",
                    status, body_note
                ))
            }
        }
    }
    async fn load_embedding_model(&self, model_id: &str) -> Result<(), String> {
        if !self.ollama.has_model(model_id).await? {
            return Err(format!(
                "Ollama embedding model `{}` is not pulled locally. Run `ollama pull {}` first.",
                model_id, model_id
            ));
        }
        let url = format!("{}/api/embed", self.base_url);
        let request = serde_json::json!({
            "model": model_id,
            "input": "search_document: Hematite semantic search warmup",
            "keep_alive": "30m"
        });
        let res = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            let body_note = truncate_provider_error_body(&body);
            return if body_note.is_empty() {
                Err(format!("Ollama embed load failed: HTTP {}", status))
            } else {
                Err(format!(
                    "Ollama embed load failed: HTTP {} | {}",
                    status, body_note
                ))
            };
        }
        if let Ok(mut guard) = self.embed_model.write() {
            *guard = Some(model_id.to_string());
        }
        Ok(())
    }
    async fn list_models(
        &self,
        kind: ProviderModelKind,
        loaded_only: bool,
    ) -> Result<Vec<String>, String> {
        let url = if loaded_only {
            format!("{}/api/ps", self.base_url)
        } else {
            format!("{}/api/tags", self.base_url)
        };
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body: Value = resp.json().await.map_err(|e| e.to_string())?;
        let mut models = Vec::new();
        if let Some(entries) = body["models"].as_array() {
            for entry in entries {
                let name = entry["name"]
                    .as_str()
                    .or_else(|| entry["model"].as_str())
                    .unwrap_or_default();
                if kind == ProviderModelKind::Embed && !looks_like_embedding_model_name(name) {
                    continue;
                }
                if kind == ProviderModelKind::Coding && looks_like_embedding_model_name(name) {
                    continue;
                }
                push_unique_model(&mut models, name);
            }
        }
        if loaded_only && kind == ProviderModelKind::Embed {
            if let Ok(guard) = self.embed_model.read() {
                if let Some(model) = guard.as_deref() {
                    push_unique_model(&mut models, model);
                }
            }
        }
        Ok(models)
    }
    async fn unload_model(&self, model_id: Option<&str>, all: bool) -> Result<String, String> {
        let targets = if all {
            self.list_models(ProviderModelKind::Coding, true).await?
        } else {
            vec![model_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "Missing model ID to unload.".to_string())?
                .to_string()]
        };

        if targets.is_empty() {
            return Ok("No Ollama models are currently loaded.".to_string());
        }

        let url = format!("{}/api/generate", self.base_url);
        let mut unloaded = 0usize;
        let mut failures = Vec::new();
        for target in targets {
            let request = serde_json::json!({
                "model": target,
                "prompt": "",
                "stream": false,
                "keep_alive": 0
            });
            match self.client.post(&url).json(&request).send().await {
                Ok(res) if res.status().is_success() => unloaded += 1,
                Ok(res) => failures.push(format!("{} ({})", target, res.status())),
                Err(e) => failures.push(format!("{} ({})", target, e)),
            }
        }

        if failures.is_empty() {
            return Ok(if all {
                format!("Unloaded {} Ollama model(s).", unloaded)
            } else {
                format!("Unloaded Ollama model `{}`.", model_id.unwrap_or_default())
            });
        }

        Err(format!(
            "Unloaded {} Ollama model(s), but some unloads failed: {}",
            unloaded,
            failures.join(", ")
        ))
    }
    async fn unload_embedding_model(&self, model_id: Option<&str>) -> Result<String, String> {
        let target = match model_id {
            Some(explicit) if !explicit.trim().is_empty() => explicit.trim().to_string(),
            _ => self
                .get_embedding_model()
                .await
                .ok_or_else(|| "No Ollama embedding model is currently loaded.".to_string())?,
        };
        let url = format!("{}/api/embed", self.base_url);
        let request = serde_json::json!({
            "model": target,
            "input": "search_document: Hematite semantic search warmup",
            "keep_alive": 0
        });
        let res = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if res.status().is_success() {
            if let Ok(mut guard) = self.embed_model.write() {
                if guard.as_deref() == Some(target.as_str()) {
                    *guard = None;
                }
            }
            Ok(format!("Unloaded Ollama embedding model `{}`.", target))
        } else {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            let body_note = truncate_provider_error_body(&body);
            if body_note.is_empty() {
                Err(format!("Ollama embed unload failed: HTTP {}", status))
            } else {
                Err(format!(
                    "Ollama embed unload failed: HTTP {} | {}",
                    status, body_note
                ))
            }
        }
    }
    async fn prewarm(&self) -> Result<(), String> {
        Ok(())
    }
    async fn get_embedding_model(&self) -> Option<String> {
        if let Ok(guard) = self.embed_model.read() {
            if let Some(model) = guard.as_ref() {
                return Some(model.clone());
            }
        }

        let url = format!("{}/api/ps", self.base_url);
        if let Ok(res) = self.client.get(&url).send().await {
            if let Ok(body) = res.json::<Value>().await {
                if let Some(entries) = body["models"].as_array() {
                    for entry in entries {
                        let name = entry["name"]
                            .as_str()
                            .or_else(|| entry["model"].as_str())
                            .unwrap_or_default();
                        if looks_like_embedding_model_name(name) {
                            return Some(name.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    fn name(&self) -> &str {
        "Ollama"
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

#[cfg(test)]
mod tests {
    use super::{
        lms_messages_payload, looks_like_embedding_model_name, matches_lms_model_kind,
        ProviderModelKind,
    };
    use crate::agent::types::{ChatMessage, ToolCallFn, ToolCallResponse};
    use serde_json::json;

    #[test]
    fn lms_payload_stringifies_assistant_tool_arguments() {
        let messages = vec![ChatMessage::assistant_tool_calls(
            "",
            vec![ToolCallResponse {
                id: "call_1".to_string(),
                call_type: "function".to_string(),
                function: ToolCallFn {
                    name: "read_file".to_string(),
                    arguments: json!({"path":"index.html"}),
                },
                index: None,
            }],
        )];

        let payload = lms_messages_payload(&messages);
        let args = &payload[0]["tool_calls"][0]["function"]["arguments"];
        assert!(args.is_string());
        assert_eq!(
            args.as_str().unwrap_or_default(),
            "{\"path\":\"index.html\"}"
        );
    }

    #[test]
    fn lms_model_kind_matching_distinguishes_embedding_models() {
        assert!(matches_lms_model_kind(ProviderModelKind::Coding, "chat"));
        assert!(matches_lms_model_kind(
            ProviderModelKind::Embed,
            "embeddings"
        ));
        assert!(!matches_lms_model_kind(
            ProviderModelKind::Coding,
            "embeddings"
        ));
        assert!(!matches_lms_model_kind(ProviderModelKind::Embed, "chat"));
    }

    #[test]
    fn embedding_name_heuristic_catches_common_ollama_embed_models() {
        assert!(looks_like_embedding_model_name("embeddinggemma"));
        assert!(looks_like_embedding_model_name("qwen3-embedding"));
        assert!(looks_like_embedding_model_name("all-minilm"));
        assert!(!looks_like_embedding_model_name("qwen3.5:latest"));
    }
}
