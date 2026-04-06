use serde_json::Value;

use crate::agent::config::{permission_for_shell, HematiteConfig, PermissionDecision, PermissionMode};
use crate::tools::RiskLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationSource {
    SystemAdminMode,
    ReadOnlyMode,
    YoloMode,
    McpExternal,
    SafePathBypass,
    ConfigAllow,
    ConfigAsk,
    ConfigDeny,
    ShellBlacklist,
    ShellRiskSafe,
    ShellRiskModerate,
    ShellRiskHigh,
    DefaultToolPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationDecision {
    Allow {
        source: AuthorizationSource,
    },
    Ask {
        source: AuthorizationSource,
        reason: String,
    },
    Deny {
        source: AuthorizationSource,
        reason: String,
    },
}

impl AuthorizationDecision {
    pub fn source(self) -> AuthorizationSource {
        match self {
            AuthorizationDecision::Allow { source }
            | AuthorizationDecision::Ask { source, .. }
            | AuthorizationDecision::Deny { source, .. } => source,
        }
    }
}

pub fn authorize_tool_call(
    name: &str,
    args: &Value,
    config: &HematiteConfig,
    yolo_flag: bool,
) -> AuthorizationDecision {
    if config.mode == PermissionMode::SystemAdmin {
        return AuthorizationDecision::Allow {
            source: AuthorizationSource::SystemAdminMode,
        };
    }

    if config.mode == PermissionMode::ReadOnly && is_destructive_tool(name) {
        return AuthorizationDecision::Deny {
            source: AuthorizationSource::ReadOnlyMode,
            reason: format!(
                "Action blocked: tool `{}` is forbidden in permission mode `{:?}`.",
                name, config.mode
            ),
        };
    }

    if yolo_flag {
        return AuthorizationDecision::Allow {
            source: AuthorizationSource::YoloMode,
        };
    }

    if name.starts_with("mcp__") {
        return AuthorizationDecision::Ask {
            source: AuthorizationSource::McpExternal,
            reason: format!(
                "External MCP tool `{}` requires explicit operator approval.",
                name
            ),
        };
    }

    if matches!(name, "write_file" | "edit_file") {
        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
            if is_path_safe(path) {
                return AuthorizationDecision::Allow {
                    source: AuthorizationSource::SafePathBypass,
                };
            }
        }
    }

    if name == "shell" {
        let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
        match permission_for_shell(cmd, config) {
            PermissionDecision::Allow => {
                return AuthorizationDecision::Allow {
                    source: AuthorizationSource::ConfigAllow,
                }
            }
            PermissionDecision::Ask => {
                return AuthorizationDecision::Ask {
                    source: AuthorizationSource::ConfigAsk,
                    reason: "Shell command requires approval by `.hematite/settings.json`."
                        .to_string(),
                }
            }
            PermissionDecision::Deny => {
                return AuthorizationDecision::Deny {
                    source: AuthorizationSource::ConfigDeny,
                    reason: "Action blocked: shell command denied by `.hematite/settings.json`."
                        .to_string(),
                }
            }
            PermissionDecision::UseRiskClassifier => {}
        }

        if let Err(e) = crate::tools::guard::bash_is_safe(cmd) {
            return AuthorizationDecision::Deny {
                source: AuthorizationSource::ShellBlacklist,
                reason: format!("Action blocked: {}", e),
            };
        }

        return match crate::tools::guard::classify_bash_risk(cmd) {
            RiskLevel::Safe => AuthorizationDecision::Allow {
                source: AuthorizationSource::ShellRiskSafe,
            },
            RiskLevel::Moderate => AuthorizationDecision::Ask {
                source: AuthorizationSource::ShellRiskModerate,
                reason: "Shell command classified as moderate risk and requires approval."
                    .to_string(),
            },
            RiskLevel::High => AuthorizationDecision::Ask {
                source: AuthorizationSource::ShellRiskHigh,
                reason: "Shell command classified as high risk and requires approval."
                    .to_string(),
            },
        };
    }

    AuthorizationDecision::Allow {
        source: AuthorizationSource::DefaultToolPolicy,
    }
}

fn is_destructive_tool(name: &str) -> bool {
    matches!(
        name,
        "write_file"
            | "edit_file"
            | "patch_hunk"
            | "shell"
            | "git_commit"
            | "git_push"
            | "git_remote"
            | "git_onboarding"
    ) || is_mcp_mutating_tool(name)
}

fn is_mcp_mutating_tool(name: &str) -> bool {
    if !name.starts_with("mcp__") {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    [
        "__edit",
        "__write",
        "__create",
        "__move",
        "__delete",
        "__remove",
        "__rename",
        "__replace",
        "__patch",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn is_path_safe(path: &str) -> bool {
    let p = path.to_lowercase();
    p.contains(".hematite/") || p.contains(".hematite\\") || p.contains("tmp/") || p.contains("tmp\\")
}
