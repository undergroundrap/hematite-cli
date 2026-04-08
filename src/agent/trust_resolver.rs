use std::path::{Path, PathBuf};

use crate::agent::config::WorkspaceTrustConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceTrustPolicy {
    Trusted,
    RequireApproval,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceTrustSource {
    DefaultWorkspace,
    Allowlist,
    UnknownWorkspace,
    Denylist,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceTrustDecision {
    pub policy: WorkspaceTrustPolicy,
    pub source: WorkspaceTrustSource,
    pub workspace_display: String,
    pub matched_root: Option<String>,
    pub reason: Option<String>,
}

pub fn resolve_workspace_trust(
    workspace_root: &Path,
    config: &WorkspaceTrustConfig,
) -> WorkspaceTrustDecision {
    let workspace = normalize_path(workspace_root);
    let workspace_display = workspace.to_string_lossy().replace('\\', "/");

    let denied_roots = resolve_roots(workspace_root, &config.deny);
    if let Some(root) = denied_roots
        .iter()
        .find(|root| path_matches(&workspace, root))
    {
        let matched = root.to_string_lossy().replace('\\', "/");
        return WorkspaceTrustDecision {
            policy: WorkspaceTrustPolicy::Denied,
            source: WorkspaceTrustSource::Denylist,
            workspace_display,
            matched_root: Some(matched.clone()),
            reason: Some(format!("workspace matches denied trust root: {}", matched)),
        };
    }

    let allow_roots = resolve_roots(workspace_root, &config.allow);
    if let Some(root) = allow_roots
        .iter()
        .find(|root| path_matches(&workspace, root))
    {
        let matched = root.to_string_lossy().replace('\\', "/");
        let source = if root == &workspace {
            WorkspaceTrustSource::DefaultWorkspace
        } else {
            WorkspaceTrustSource::Allowlist
        };
        return WorkspaceTrustDecision {
            policy: WorkspaceTrustPolicy::Trusted,
            source,
            workspace_display,
            matched_root: Some(matched),
            reason: None,
        };
    }

    WorkspaceTrustDecision {
        policy: WorkspaceTrustPolicy::RequireApproval,
        source: WorkspaceTrustSource::UnknownWorkspace,
        workspace_display,
        matched_root: None,
        reason: Some(
            "workspace is not trust-allowlisted, so destructive or external actions require approval."
                .to_string(),
        ),
    }
}

fn resolve_roots(workspace_root: &Path, configured_roots: &[String]) -> Vec<PathBuf> {
    configured_roots
        .iter()
        .map(|root| {
            let path = Path::new(root);
            if path.is_absolute() {
                normalize_path(path)
            } else {
                normalize_path(&workspace_root.join(path))
            }
        })
        .collect()
}

fn path_matches(candidate: &Path, root: &Path) -> bool {
    candidate == root || candidate.starts_with(root)
}

fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::{resolve_workspace_trust, WorkspaceTrustPolicy, WorkspaceTrustSource};
    use crate::agent::config::WorkspaceTrustConfig;
    use std::path::PathBuf;

    #[test]
    fn trusts_current_workspace_by_default() {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let decision = resolve_workspace_trust(&root, &WorkspaceTrustConfig::default());
        assert_eq!(decision.policy, WorkspaceTrustPolicy::Trusted);
        assert_eq!(decision.source, WorkspaceTrustSource::DefaultWorkspace);
    }

    #[test]
    fn denied_root_takes_precedence() {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let config = WorkspaceTrustConfig {
            allow: vec![".".to_string()],
            deny: vec![root.to_string_lossy().to_string()],
        };
        let decision = resolve_workspace_trust(&root, &config);
        assert_eq!(decision.policy, WorkspaceTrustPolicy::Denied);
        assert_eq!(decision.source, WorkspaceTrustSource::Denylist);
    }

    #[test]
    fn unknown_workspace_requires_approval() {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let config = WorkspaceTrustConfig {
            allow: vec!["./somewhere-else".to_string()],
            deny: Vec::new(),
        };
        let decision = resolve_workspace_trust(&root, &config);
        assert_eq!(decision.policy, WorkspaceTrustPolicy::RequireApproval);
        assert_eq!(decision.source, WorkspaceTrustSource::UnknownWorkspace);
    }
}
