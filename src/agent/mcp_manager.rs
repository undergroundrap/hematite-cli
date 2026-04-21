use crate::agent::mcp::*;
use crate::agent::types::McpRuntimeState;
use crate::tools::file_ops::hematite_dir;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct McpConfig {
    #[serde(rename = "mcpServers")]
    pub servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}

pub struct McpManager {
    pub connections: HashMap<String, McpProcess>,
    pub tool_map: HashMap<String, String>, // qualified_name -> server_name
    pub discovered_tools: Vec<McpTool>,
    pub active_config_signature: Option<String>,
    pub configured_servers: usize,
    pub startup_errors: Vec<String>,
    pub discovery_errors: Vec<String>,
    pub next_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRuntimeReport {
    pub state: McpRuntimeState,
    pub configured_servers: usize,
    pub connected_servers: usize,
    pub active_tools: usize,
    pub error_count: usize,
    pub summary: String,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            tool_map: HashMap::new(),
            discovered_tools: Vec::new(),
            active_config_signature: None,
            configured_servers: 0,
            startup_errors: Vec::new(),
            discovery_errors: Vec::new(),
            next_id: 1,
        }
    }

    pub async fn initialize_all(&mut self) -> Result<()> {
        let config = self.load_mcp_config();
        self.configured_servers = config.servers.len();
        let signature = self.config_signature(&config);
        let all_connected = self.connections.len() == config.servers.len();
        if self.active_config_signature.as_deref() == Some(signature.as_str())
            && (all_connected || config.servers.is_empty())
        {
            return Ok(());
        }

        self.shutdown_all().await;
        self.tool_map.clear();
        self.discovered_tools.clear();
        self.startup_errors.clear();
        self.discovery_errors.clear();
        self.active_config_signature = Some(signature);

        for (name, cfg) in config.servers {
            let args = cfg.args.clone().unwrap_or_default();
            let env = cfg.env.clone().unwrap_or_default();

            match self
                .spawn_and_initialize_server(&cfg.command, &args, &env)
                .await
            {
                Ok(proc) => {
                    self.connections.insert(name.clone(), proc);
                }
                Err(e) => {
                    self.startup_errors.push(format!("{}: {}", name, e));
                }
            }
        }
        Ok(())
    }

    pub fn load_mcp_config(&self) -> McpConfig {
        let mut config = McpConfig::default();

        // 1. Load GLOBAL config (~/.hematite/mcp_servers.json)
        if let Some(mut global_path) = home::home_dir() {
            global_path.push(".hematite");
            global_path.push("mcp_servers.json");
            if let Ok(global_cfg) = self.read_mcp_file(&global_path) {
                self.merge_configs(&mut config, global_cfg);
            }
        }

        // 2. Load LOCAL config (.hematite/mcp_servers.json in workspace)
        let local_path = hematite_dir().join("mcp_servers.json");
        if let Ok(local_cfg) = self.read_mcp_file(&local_path) {
            self.merge_configs(&mut config, local_cfg);
        }

        config
    }

    fn read_mcp_file(&self, path: &Path) -> Result<McpConfig> {
        let data = std::fs::read_to_string(path)?;
        let config: McpConfig = serde_json::from_str(&data)?;
        Ok(config)
    }

    fn merge_configs(&self, base: &mut McpConfig, new: McpConfig) {
        for (name, server) in new.servers {
            base.servers.insert(name, server);
        }
    }

    fn config_signature(&self, config: &McpConfig) -> String {
        let mut servers: Vec<_> = config.servers.iter().collect();
        servers.sort_by(|a, b| a.0.cmp(b.0));

        let mut signature = String::new();
        for (name, server) in servers {
            signature.push_str(name);
            signature.push('|');
            signature.push_str(&server.command);
            signature.push('|');

            if let Some(args) = &server.args {
                for arg in args {
                    signature.push_str(arg);
                    signature.push('\u{1f}');
                }
            }
            signature.push('|');

            let mut env_pairs = server
                .env
                .as_ref()
                .map(|env| env.iter().collect::<Vec<_>>())
                .unwrap_or_default();
            env_pairs.sort_by(|a, b| a.0.cmp(b.0));
            for (key, value) in env_pairs {
                signature.push_str(key);
                signature.push('=');
                signature.push_str(value);
                signature.push(';');
            }
            signature.push('\n');
        }

        signature
    }

    async fn shutdown_all(&mut self) {
        let connections = std::mem::take(&mut self.connections);
        for (_, proc) in connections {
            proc.shutdown().await;
        }
    }

    async fn spawn_and_initialize_server(
        &mut self,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<McpProcess> {
        let mut last_error = None;

        for framing in [McpFraming::NewlineDelimited, McpFraming::ContentLength] {
            let mut proc = McpProcess::spawn_with_framing(command, args, env, framing)?;
            let init_result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                proc.initialize(self.next_id),
            )
            .await;

            match init_result {
                Ok(Ok(())) => {
                    self.next_id += 1;
                    return Ok(proc);
                }
                Ok(Err(err)) => {
                    last_error = Some(Self::format_mcp_init_error(&proc, err.to_string()));
                    proc.shutdown().await;
                }
                Err(_) => {
                    last_error = Some(Self::format_mcp_init_error(
                        &proc,
                        "initialize timed out after 5s".to_string(),
                    ));
                    proc.shutdown().await;
                }
            }
        }

        Err(anyhow!(last_error.unwrap_or_else(|| {
            "server did not complete initialize using newline or content-length framing".to_string()
        })))
    }

    fn format_mcp_init_error(proc: &McpProcess, base_error: String) -> String {
        match proc.stderr_summary() {
            Some(stderr) => format!("{base_error}; stderr: {stderr}"),
            None => base_error,
        }
    }

    pub async fn discover_tools(&mut self) -> Vec<McpTool> {
        if !self.discovered_tools.is_empty() {
            return self.discovered_tools.clone();
        }

        let mut all_tools = Vec::new();
        self.tool_map.clear();
        self.discovery_errors.clear();
        let server_names: Vec<String> = self.connections.keys().cloned().collect();

        for name in server_names {
            if let Some(proc) = self.connections.get_mut(&name) {
                match proc.list_tools(self.next_id).await {
                    Ok(tools) => {
                        self.next_id += 1;
                        for mut tool in tools {
                            let original_name = tool.name.clone();
                            // Prefix to avoid collisions
                            tool.name = format!("mcp__{}__{}", name, original_name);
                            self.tool_map.insert(tool.name.clone(), name.clone());
                            all_tools.push(tool);
                        }
                    }
                    Err(e) => {
                        self.discovery_errors.push(format!("{}: {}", name, e));
                    }
                }
            }
        }
        self.discovered_tools = all_tools.clone();
        all_tools
    }

    pub async fn call_tool(&mut self, full_name: &str, args: &JsonValue) -> Result<String> {
        let server_name = self
            .tool_map
            .get(full_name)
            .ok_or_else(|| anyhow!("Unknown MCP tool: {}", full_name))?;
        let proc = self
            .connections
            .get_mut(server_name)
            .ok_or_else(|| anyhow!("Server not connected: {}", server_name))?;

        // Strip prefix to get original name
        let prefix = format!("mcp__{}__", server_name);
        let original_name = full_name.strip_prefix(&prefix).unwrap_or(full_name);

        let result = proc
            .call_tool(self.next_id, original_name, args.clone())
            .await?;
        self.next_id += 1;

        let mut output = String::new();
        for content in result.content {
            match content {
                McpContent::Text { text } => output.push_str(&text),
                McpContent::Image { .. } => {
                    output.push_str("\n[Image Data Not Supported in TUI]\n")
                }
            }
        }

        if result.is_error.unwrap_or(false) {
            Err(anyhow!(output))
        } else {
            // VRAM Guard: Truncate massive outputs to protect the local context window.
            if output.len() > 2500 {
                output.truncate(2500);
                output.push_str("\n\n[Output Truncated by Hematite for VRAM Safety]");
            }
            Ok(output)
        }
    }

    pub fn runtime_report(&self) -> McpRuntimeReport {
        let first_error = self
            .startup_errors
            .first()
            .or_else(|| self.discovery_errors.first())
            .map(String::as_str);
        runtime_report_from_snapshot(
            self.configured_servers,
            self.connections.len(),
            self.discovered_tools.len(),
            self.startup_errors.len() + self.discovery_errors.len(),
            first_error,
        )
    }
}

fn runtime_report_from_snapshot(
    configured_servers: usize,
    connected_servers: usize,
    active_tools: usize,
    error_count: usize,
    first_error: Option<&str>,
) -> McpRuntimeReport {
    let state = if configured_servers == 0 {
        McpRuntimeState::Unconfigured
    } else if connected_servers == 0 {
        McpRuntimeState::Failed
    } else if error_count > 0 {
        McpRuntimeState::Degraded
    } else {
        McpRuntimeState::Healthy
    };

    let detail = summarize_runtime_error(first_error);

    let summary = match state {
        McpRuntimeState::Unconfigured => "No MCP servers configured.".to_string(),
        McpRuntimeState::Healthy => format!(
            "MCP healthy: {}/{} servers connected; {} tools active.",
            connected_servers, configured_servers, active_tools
        ),
        McpRuntimeState::Degraded => format!(
            "MCP degraded: {}/{} servers connected; {} tools active; {} startup/discovery issue(s){}",
            connected_servers, configured_servers, active_tools, error_count, detail
        ),
        McpRuntimeState::Failed => format!(
            "MCP failed: 0/{} servers connected; {} startup/discovery issue(s){}",
            configured_servers, error_count, detail
        ),
    };

    McpRuntimeReport {
        state,
        configured_servers,
        connected_servers,
        active_tools,
        error_count,
        summary,
    }
}

fn summarize_runtime_error(first_error: Option<&str>) -> String {
    let Some(error) = first_error.map(str::trim).filter(|value| !value.is_empty()) else {
        return ".".to_string();
    };

    const MAX_CHARS: usize = 160;
    let mut truncated = error.chars().take(MAX_CHARS).collect::<String>();
    if error.chars().count() > MAX_CHARS {
        truncated.push_str("...");
    }
    format!(" First issue: {truncated}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_report_marks_unconfigured_when_no_servers_exist() {
        let report = runtime_report_from_snapshot(0, 0, 0, 0, None);
        assert_eq!(report.state, McpRuntimeState::Unconfigured);
        assert!(report.summary.contains("No MCP servers configured"));
    }

    #[test]
    fn runtime_report_marks_failed_when_servers_exist_but_none_connect() {
        let report = runtime_report_from_snapshot(2, 0, 0, 2, Some("filesystem: spawn failed"));
        assert_eq!(report.state, McpRuntimeState::Failed);
        assert!(report.summary.contains("0/2"));
        assert!(report.summary.contains("filesystem: spawn failed"));
    }

    #[test]
    fn runtime_report_marks_degraded_when_some_servers_or_discovery_steps_fail() {
        let report =
            runtime_report_from_snapshot(2, 1, 3, 1, Some("filesystem: tools/list failed"));
        assert_eq!(report.state, McpRuntimeState::Degraded);
        assert!(report.summary.contains("1/2"));
        assert!(report.summary.contains("tools/list failed"));
    }

    #[test]
    fn runtime_report_marks_healthy_when_all_servers_connect_without_errors() {
        let report = runtime_report_from_snapshot(2, 2, 5, 0, None);
        assert_eq!(report.state, McpRuntimeState::Healthy);
        assert!(report.summary.contains("5 tools active"));
    }
}
