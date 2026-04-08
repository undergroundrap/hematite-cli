use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex};

/// LSP JSON-RPC Request object
#[derive(Serialize)]
pub struct LspRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Value,
}

/// A robust, async-first LSP client for Hematite-CLI.
pub struct LspClient {
    #[allow(dead_code)]
    child: Child,
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
    pub next_id: Arc<std::sync::atomic::AtomicU64>,
    /// Layer 9: Diagnostic Storage (Pinned to URI)
    pub diagnostics: Arc<Mutex<HashMap<String, Value>>>,
}

impl LspClient {
    pub fn spawn(
        command: &str,
        args: &[String],
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;

        let pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let next_id = Arc::new(std::sync::atomic::AtomicU64::new(1));
        let diagnostics: Arc<Mutex<HashMap<String, Value>>> = Arc::new(Mutex::new(HashMap::new()));

        let pending_requests_clone = pending_requests.clone();
        let diagnostics_clone = diagnostics.clone();

        // Background thread to read LSP stdout
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                let n = match reader.read_line(&mut line).await {
                    Ok(n) => n,
                    Err(_) => break,
                };
                if n == 0 {
                    break;
                }

                if line.starts_with("Content-Length: ") {
                    let len: usize = line["Content-Length: ".len()..].trim().parse().unwrap_or(0);
                    line.clear();
                    let _ = reader.read_line(&mut line).await;

                    let mut body = vec![0u8; len];
                    if let Err(_) = reader.read_exact(&mut body).await {
                        break;
                    }

                    if let Ok(json_body) = serde_json::from_slice::<Value>(&body) {
                        if let Some(id) = json_body.get("id").and_then(|v| v.as_u64()) {
                            let mut pending = pending_requests_clone.lock().await;
                            if let Some(tx) = pending.remove(&id) {
                                if let Some(err) = json_body.get("error") {
                                    let _ = tx.send(Err(err.to_string()));
                                } else {
                                    let result =
                                        json_body.get("result").cloned().unwrap_or(Value::Null);
                                    let _ = tx.send(Ok(result));
                                }
                            }
                        } else if let Some(method) =
                            json_body.get("method").and_then(|v| v.as_str())
                        {
                            // This is a notification
                            if method == "textDocument/publishDiagnostics" {
                                if let Some(params) = json_body.get("params") {
                                    if let Some(uri) = params.get("uri").and_then(|v| v.as_str()) {
                                        let mut diags = diagnostics_clone.lock().await;
                                        diags.insert(
                                            uri.to_string(),
                                            params
                                                .get("diagnostics")
                                                .cloned()
                                                .unwrap_or(Value::Null),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            child,
            stdin: Arc::new(Mutex::new(stdin)),
            pending_requests,
            next_id,
            diagnostics,
        })
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id, tx);
        }

        let request = LspRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let body = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        {
            let mut stdin = self.stdin.lock().await;
            if let Err(e) = stdin.write_all(header.as_bytes()).await {
                return Err(format!("LSP Stdin Header Fail: {}", e));
            }
            if let Err(e) = stdin.write_all(body.as_bytes()).await {
                return Err(format!("LSP Stdin Body Fail: {}", e));
            }
            if let Err(e) = stdin.flush().await {
                return Err(format!("LSP Stdin Flush Fail: {}", e));
            }
        }

        rx.await
            .map_err(|_| "LSP Response Channel Closed".to_string())?
    }

    /// Sends an LSP notification (no response expected).
    pub async fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let body = serde_json::to_string(&notification).map_err(|e| e.to_string())?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        {
            let mut stdin = self.stdin.lock().await;
            let _ = stdin.write_all(header.as_bytes()).await;
            let _ = stdin.write_all(body.as_bytes()).await;
            let _ = stdin.flush().await;
        }
        Ok(())
    }
}
