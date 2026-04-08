use crate::agent::lsp::client::LspClient;
use serde_json::json;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

/// Orchestrates Language Servers for the agent.
pub struct LspManager {
    pub clients: HashMap<String, Arc<LspClient>>,
    pub workspace_root: PathBuf,
    pub opened_files: BTreeSet<PathBuf>,
}

impl LspManager {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            clients: HashMap::new(),
            workspace_root,
            opened_files: BTreeSet::new(),
        }
    }

    /// Discovers and starts necessary language servers.
    pub async fn start_servers(&mut self) -> Result<(), String> {
        // Start rust-analyzer if a Cargo.toml exists
        if self.workspace_root.join("Cargo.toml").exists() {
            self.start_server("rust", "rust-analyzer", &[]).await?;
        }

        // ── Stabilization ──
        // Give the Language Server a moment to index before the first tool call fires.
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        Ok(())
    }

    pub async fn start_server(
        &mut self,
        lang: &str,
        command: &str,
        args: &[String],
    ) -> Result<(), String> {
        let client =
            LspClient::spawn(command, args).map_err(|e| format!("LSP Spawn Fail: {}", e))?;
        let arc_client = Arc::new(client);

        // --- LAYER 1: LSP Handshake ---
        let params = json!({
            "processId": std::process::id(),
            "rootUri": format!("file:///{}", self.workspace_root.to_str().unwrap_or_default().replace("\\", "/")),
            "capabilities": {
                "textDocument": {
                    "definition": { "dynamicRegistration": false },
                    "references": { "dynamicRegistration": false },
                    "hover": { "dynamicRegistration": false },
                    "symbol": { "dynamicRegistration": false },
                    "rename": { "dynamicRegistration": false },
                    "publishDiagnostics": { "relatedInformation": true }
                },
                "workspace": {
                    "symbol": { "dynamicRegistration": false }
                }
            },
            "initializationOptions": null
        });

        match arc_client.call("initialize", params).await {
            Ok(_) => {
                let _ = arc_client.notify("initialized", json!({})).await;
                self.clients.insert(lang.to_string(), arc_client);
                Ok(())
            }
            Err(e) => Err(format!("LSP Handshake Fail ({}): {}", lang, e)),
        }
    }

    pub fn get_client(&self, lang: &str) -> Option<Arc<LspClient>> {
        self.clients.get(lang).cloned()
    }

    /// Helper to find the client for a file extension.
    pub fn get_client_for_path(&self, path: &str) -> Option<Arc<LspClient>> {
        let ext = std::path::Path::new(path).extension()?.to_str()?;
        match ext {
            "rs" => self.get_client("rust"),
            "ts" | "js" | "tsx" | "jsx" => self.get_client("typescript"),
            "py" => self.get_client("python"),
            _ => None,
        }
    }

    pub fn resolve_uri(&self, path: &str) -> String {
        let abs_path = if std::path::Path::new(path).is_absolute() {
            std::path::PathBuf::from(path)
        } else {
            self.workspace_root.join(path)
        };
        format!(
            "file:///{}",
            abs_path.to_str().unwrap_or_default().replace("\\", "/")
        )
    }

    pub async fn ensure_opened(&mut self, path: &str) -> Result<(), String> {
        let path_obj = if std::path::Path::new(path).is_absolute() {
            std::path::PathBuf::from(path)
        } else {
            self.workspace_root.join(path)
        };

        if self.opened_files.contains(&path_obj) {
            return Ok(());
        }

        let client = self
            .get_client_for_path(path)
            .ok_or_else(|| format!("No LSP client for {}", path))?;

        let content =
            std::fs::read_to_string(&path_obj).map_err(|e| format!("Read Fail: {}", e))?;

        let lang_id = match path_obj.extension().and_then(|e| e.to_str()) {
            Some("rs") => "rust",
            Some("py") => "python",
            Some("ts") | Some("js") => "typescript",
            _ => "text",
        };

        let params = json!({
            "textDocument": {
                "uri": self.resolve_uri(path),
                "languageId": lang_id,
                "version": 1,
                "text": content
            }
        });

        client.notify("textDocument/didOpen", params).await?;
        self.opened_files.insert(path_obj);
        Ok(())
    }
}
