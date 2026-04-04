use std::process::Command;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
}

impl HookEvent {
    fn as_str(self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRunResult {
    pub denied: bool,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeHookConfig {
    #[serde(default)]
    pub pre_tool_use: Vec<String>,
    #[serde(default)]
    pub post_tool_use: Vec<String>,
}

pub struct HookRunner {
    config: RuntimeHookConfig,
}

impl HookRunner {
    pub fn new(config: RuntimeHookConfig) -> Self {
        Self { config }
    }

    pub fn run_pre_tool_use(&self, tool_name: &str, tool_input: &Value) -> HookRunResult {
        self.run_commands(
            HookEvent::PreToolUse,
            &self.config.pre_tool_use,
            tool_name,
            tool_input,
            None,
            false,
        )
    }

    pub fn run_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: &Value,
        tool_output: &str,
        is_error: bool,
    ) -> HookRunResult {
        self.run_commands(
            HookEvent::PostToolUse,
            &self.config.post_tool_use,
            tool_name,
            tool_input,
            Some(tool_output),
            is_error,
        )
    }

    fn run_commands(
        &self,
        event: HookEvent,
        commands: &[String],
        tool_name: &str,
        tool_input: &Value,
        tool_output: Option<&str>,
        is_error: bool,
    ) -> HookRunResult {
        let mut messages = Vec::new();
        let mut denied = false;

        for command_str in commands {
            let mut cmd = if cfg!(windows) {
                let mut c = Command::new("cmd");
                c.arg("/C").arg(command_str);
                c
            } else {
                let mut c = Command::new("sh");
                c.arg("-c").arg(command_str);
                c
            };

            cmd.env("HEMATITE_HOOK_EVENT", event.as_str());
            cmd.env("HEMATITE_TOOL_NAME", tool_name);
            cmd.env("HEMATITE_TOOL_INPUT", tool_input.to_string());
            cmd.env("HEMATITE_TOOL_ERROR", if is_error { "1" } else { "0" });
            if let Some(out) = tool_output {
                cmd.env("HEMATITE_TOOL_OUTPUT", out);
            }

            match cmd.output() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !stdout.is_empty() {
                        messages.push(stdout);
                    }
                    
                    // Exit code 2 means "DENY" — hook explicitly blocks the tool call
                    if output.status.code() == Some(2) {
                        denied = true;
                        break;
                    }
                }
                Err(e) => {
                    messages.push(format!("Hook failed to start: {}", e));
                }
            }
        }

        HookRunResult { denied, messages }
    }
}
