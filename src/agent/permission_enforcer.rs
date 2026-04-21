use serde_json::Value;

use crate::agent::config::{
    permission_for_shell, HematiteConfig, PermissionDecision, PermissionMode,
};
use crate::agent::trust_resolver::{resolve_workspace_trust, WorkspaceTrustPolicy};
use crate::tools::RiskLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationSource {
    SystemAdminMode,
    ReadOnlyMode,
    YoloMode,
    WorkspaceTrusted,
    WorkspaceApprovalRequired,
    WorkspaceDenied,
    McpExternal,
    SafePathBypass,
    ConfigAllow,
    ConfigAsk,
    ConfigDeny,
    ShellBlacklist,
    ShellRiskSafe,
    ShellRiskModerate,
    ShellRiskHigh,
    StructuredWorkflowApproval,
    DefaultToolPolicy,
}

/// A structured security profile pairing an approval policy with a sandbox policy.
/// Ports the Principled Security patterns from Codex-RS.
#[derive(Debug, Clone, Copy)]
pub struct ApprovalPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub must_ask_all: bool,
    pub allow_mutations: bool,
}

impl ApprovalPreset {
    pub fn for_mode(mode: PermissionMode) -> Self {
        match mode {
            PermissionMode::ReadOnly => Self {
                id: "read-only",
                label: "Read Only",
                description: "Hematite can only read files. All mutations and shell commands are blocked.",
                must_ask_all: false,
                allow_mutations: false,
            },
            PermissionMode::Developer => Self {
                id: "developer",
                label: "Developer",
                description: "Hematite can read/edit files and run commands. Risky actions require approval.",
                must_ask_all: false,
                allow_mutations: true,
            },
            PermissionMode::SystemAdmin => Self {
                id: "power-user",
                label: "Power User",
                description: "Hematite has full access. Auto-approves most developer tools.",
                must_ask_all: false,
                allow_mutations: true,
            },
        }
    }
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
    let preset = ApprovalPreset::for_mode(config.mode);

    if preset.id == "power-user" {
        return AuthorizationDecision::Allow {
            source: AuthorizationSource::SystemAdminMode,
        };
    }

    if !preset.allow_mutations && is_destructive_tool(name) {
        return AuthorizationDecision::Deny {
            source: AuthorizationSource::ReadOnlyMode,
            reason: format!(
                "Action blocked: tool `{}` involves workspace mutations forbidden by the `{}` security preset.",
                name, preset.label
            ),
        };
    }

    if yolo_flag {
        return AuthorizationDecision::Allow {
            source: AuthorizationSource::YoloMode,
        };
    }

    let workspace_root = crate::tools::file_ops::workspace_root();
    let trust = resolve_workspace_trust(&workspace_root, &config.trust);
    if trust_sensitive_tool(name) {
        match trust.policy {
            WorkspaceTrustPolicy::Denied => {
                return AuthorizationDecision::Deny {
                    source: AuthorizationSource::WorkspaceDenied,
                    reason: format!(
                        "Action blocked: workspace `{}` is denied by trust policy{}.",
                        trust.workspace_display,
                        trust
                            .matched_root
                            .as_ref()
                            .map(|root| format!(" ({})", root))
                            .unwrap_or_default()
                    ),
                };
            }
            WorkspaceTrustPolicy::RequireApproval => {
                return AuthorizationDecision::Ask {
                    source: AuthorizationSource::WorkspaceApprovalRequired,
                    reason: format!(
                        "Workspace `{}` is not trust-allowlisted, so `{}` requires approval before Hematite performs destructive or external actions there.",
                        trust.workspace_display, name
                    ),
                };
            }
            WorkspaceTrustPolicy::Trusted => {}
        }
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

        // Auto-deny any shell call that looks like a host-inspection question,
        // IF we have a native topic to redirect it to.
        // validate_action_preconditions will auto-redirect it to inspect_host.
        if crate::agent::conversation::shell_looks_like_structured_host_inspection(cmd)
            && crate::agent::routing::preferred_host_inspection_topic(cmd).is_some()
        {
            return AuthorizationDecision::Deny {
                source: AuthorizationSource::ShellBlacklist,
                reason: "Action blocked: use inspect_host instead of shell for host-inspection questions.".to_string(),
            };
        }

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
                reason: "Shell command classified as high risk and requires approval.".to_string(),
            },
        };
    }

    if matches!(
        name,
        "run_hematite_maintainer_workflow" | "run_workspace_workflow"
    ) {
        return AuthorizationDecision::Ask {
            source: AuthorizationSource::StructuredWorkflowApproval,
            reason: structured_workflow_reason(name, args),
        };
    }

    AuthorizationDecision::Allow {
        source: if trust_sensitive_tool(name) {
            AuthorizationSource::WorkspaceTrusted
        } else {
            AuthorizationSource::DefaultToolPolicy
        },
    }
}

fn structured_workflow_reason(name: &str, args: &Value) -> String {
    if name == "run_workspace_workflow" {
        return match args.get("workflow").and_then(|v| v.as_str()).unwrap_or("") {
            "build" | "test" | "lint" | "fix" => {
                "Workspace workflow execution can build, test, or mutate the current project, so it requires approval."
                    .to_string()
            }
            "package_script" | "task" | "just" | "make" | "script_path" | "command" => {
                "Workspace script execution runs commands from the locked project root and may change files, installs, dev servers, or build artifacts, so it requires approval."
                    .to_string()
            }
            _ => {
                "Structured workspace workflow execution changes local state and requires approval."
                    .to_string()
            }
        };
    }

    match args.get("workflow").and_then(|v| v.as_str()).unwrap_or("") {
        "clean" => {
            "Repo cleanup changes build artifacts, local Hematite state, and possibly dist/ outputs, so it requires approval."
                .to_string()
        }
        "package_windows" => {
            "Windows packaging rebuilds release artifacts and may update the user PATH, so it requires approval."
                .to_string()
        }
        "release" => {
            "The release workflow can bump versions, commit, tag, push, build installers, and publish crates, so it requires approval."
                .to_string()
        }
        _ => {
            "Structured Hematite maintainer workflow execution changes local state and requires approval."
                .to_string()
        }
    }
}

fn is_destructive_tool(name: &str) -> bool {
    crate::agent::inference::tool_metadata_for_name(name).mutates_workspace
}

pub(crate) fn is_path_safe(path: &str) -> bool {
    let p = path.to_lowercase();
    p.contains(".hematite/")
        || p.contains(".hematite\\")
        || p.contains("tmp/")
        || p.contains("tmp\\")
}

fn trust_sensitive_tool(name: &str) -> bool {
    crate::agent::inference::tool_metadata_for_name(name).trust_sensitive
}
