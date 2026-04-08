use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpFraming {
    NewlineDelimited,
    ContentLength,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum JsonRpcId {
    Number(u64),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest<T = JsonValue> {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification<T = JsonValue> {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse<T = JsonValue> {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpListToolsResult {
    pub tools: Vec<McpTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCallToolResult {
    pub content: Vec<McpContent>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
}

pub struct McpProcess {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    framing: McpFraming,
    stderr_lines: Arc<Mutex<VecDeque<String>>>,
    _stderr_task: JoinHandle<()>,
}

impl McpProcess {
    pub fn spawn(
        command: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
    ) -> Result<Self> {
        Self::spawn_with_framing(command, args, env, McpFraming::NewlineDelimited)
    }

    pub fn spawn_with_framing(
        command: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
        framing: McpFraming,
    ) -> Result<Self> {
        let resolved_command =
            resolve_command_path(command).unwrap_or_else(|| PathBuf::from(command));
        let mut cmd = if is_cmd_wrapper(&resolved_command) {
            let mut wrapper = Command::new("cmd");
            wrapper.arg("/C").arg(&resolved_command);
            wrapper
        } else {
            Command::new(&resolved_command)
        };
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (k, v) in env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to capture stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("Failed to capture stderr"))?;
        let stderr_lines = Arc::new(Mutex::new(VecDeque::with_capacity(16)));
        let stderr_task = spawn_stderr_drain(stderr, Arc::clone(&stderr_lines));

        Ok(Self {
            _child: child,
            stdin,
            stdout: BufReader::new(stdout),
            framing,
            stderr_lines,
            _stderr_task: stderr_task,
        })
    }

    pub async fn request<P: Serialize, R: for<'de> Deserialize<'de>>(
        &mut self,
        id: u64,
        method: &str,
        params: Option<P>,
    ) -> Result<R> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(id),
            method: method.to_string(),
            params: params.map(serde_json::to_value).transpose()?,
        };

        self.write_message(&req).await?;

        loop {
            let payload = self.read_message_payload().await?;
            let value: JsonValue = serde_json::from_slice(&payload).map_err(|e| {
                anyhow!(
                    "Failed to parse MCP response: {}. Raw: {}",
                    e,
                    String::from_utf8_lossy(&payload)
                )
            })?;

            // Ignore notifications or server-initiated events while waiting for a response.
            if value.get("id").is_none() {
                continue;
            }

            let resp: JsonRpcResponse<R> = serde_json::from_value(value)
                .map_err(|e| anyhow!("Failed to decode MCP response: {}", e))?;

            if let Some(error) = resp.error {
                return Err(anyhow!("MCP Error ({}): {}", error.code, error.message));
            }

            return resp
                .result
                .ok_or_else(|| anyhow!("Missing result in MCP response"));
        }
    }

    pub async fn notify<P: Serialize>(&mut self, method: &str, params: Option<P>) -> Result<()> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: params.map(serde_json::to_value).transpose()?,
        };

        self.write_message(&notification).await
    }

    pub async fn initialize(&mut self, id: u64) -> Result<()> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "hematite", "version": "0.1.0" }
        });
        let _: JsonValue = self.request(id, "initialize", Some(params)).await?;
        self.notify("notifications/initialized", Some(serde_json::json!({})))
            .await?;
        Ok(())
    }

    pub async fn list_tools(&mut self, id: u64) -> Result<Vec<McpTool>> {
        let res: McpListToolsResult = self.request(id, "tools/list", None::<()>).await?;
        Ok(res.tools)
    }

    pub async fn call_tool(
        &mut self,
        id: u64,
        name: &str,
        arguments: JsonValue,
    ) -> Result<McpCallToolResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });
        self.request(id, "tools/call", Some(params)).await
    }

    pub async fn shutdown(mut self) {
        let _ = self._child.kill().await;
        self._stderr_task.abort();
    }

    pub fn stderr_summary(&self) -> Option<String> {
        let lines = self.stderr_lines.lock().ok()?;
        if lines.is_empty() {
            None
        } else {
            Some(lines.iter().cloned().collect::<Vec<_>>().join(" | "))
        }
    }

    async fn write_message<T: Serialize>(&mut self, message: &T) -> Result<()> {
        let payload = serde_json::to_vec(message)?;
        match self.framing {
            McpFraming::NewlineDelimited => {
                self.stdin.write_all(&payload).await?;
                self.stdin.write_all(b"\n").await?;
            }
            McpFraming::ContentLength => {
                let header = format!("Content-Length: {}\r\n\r\n", payload.len());
                self.stdin.write_all(header.as_bytes()).await?;
                self.stdin.write_all(&payload).await?;
            }
        }
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read_message_payload(&mut self) -> Result<Vec<u8>> {
        match self.framing {
            McpFraming::NewlineDelimited => {
                let mut line = String::new();
                self.stdout.read_line(&mut line).await?;
                if line.is_empty() {
                    return Err(anyhow!("MCP server closed connection unexpectedly"));
                }
                Ok(line.into_bytes())
            }
            McpFraming::ContentLength => {
                let mut first_line = String::new();
                self.stdout.read_line(&mut first_line).await?;
                if first_line.is_empty() {
                    return Err(anyhow!("MCP server closed connection unexpectedly"));
                }

                if !first_line.starts_with("Content-Length:") {
                    return Ok(first_line.into_bytes());
                }

                let content_length = first_line["Content-Length:".len()..]
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| anyhow!("Invalid MCP Content-Length header: {}", e))?;

                loop {
                    let mut header_line = String::new();
                    self.stdout.read_line(&mut header_line).await?;
                    if header_line.is_empty() {
                        return Err(anyhow!(
                            "MCP server closed connection while reading headers"
                        ));
                    }
                    if header_line == "\r\n" || header_line == "\n" {
                        break;
                    }
                }

                let mut payload = vec![0_u8; content_length];
                self.stdout.read_exact(&mut payload).await?;
                Ok(payload)
            }
        }
    }
}

fn spawn_stderr_drain(
    stderr: ChildStderr,
    stderr_lines: Arc<Mutex<VecDeque<String>>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    if let Ok(mut lines) = stderr_lines.lock() {
                        lines.push_back(trimmed.to_string());
                        while lines.len() > 20 {
                            lines.pop_front();
                        }
                    }
                }
            }
        }
    })
}

#[cfg(windows)]
fn resolve_command_path(command: &str) -> Option<PathBuf> {
    let candidate = PathBuf::from(command);
    let has_extension = Path::new(command).extension().is_some();
    if candidate.is_absolute() || command.contains('\\') || command.contains('/') {
        if !has_extension {
            for ext in [".exe", ".cmd", ".bat", ".com"] {
                let with_ext = PathBuf::from(format!("{command}{ext}"));
                if with_ext.exists() {
                    return Some(with_ext);
                }
            }
        }
        if candidate.exists() {
            return Some(candidate);
        }
        return None;
    }

    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        if !has_extension {
            for ext in [".exe", ".cmd", ".bat", ".com"] {
                let with_ext = dir.join(format!("{command}{ext}"));
                if with_ext.exists() {
                    return Some(with_ext);
                }
            }
        }
        let direct = dir.join(command);
        if direct.exists() {
            return Some(direct);
        }
    }

    None
}

#[cfg(not(windows))]
fn resolve_command_path(command: &str) -> Option<PathBuf> {
    Some(PathBuf::from(command))
}

#[cfg(windows)]
fn is_cmd_wrapper(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()),
        Some(ext) if ext == "cmd" || ext == "bat"
    )
}

#[cfg(not(windows))]
fn is_cmd_wrapper(_path: &Path) -> bool {
    false
}
