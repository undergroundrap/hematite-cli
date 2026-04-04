use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tokio::process::Command;
use std::time::Duration;

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum GitRemoteStatus {
    Unknown = 0,
    NoRemote = 1,
    Connected = 2,
    Behind = 3,
    Ahead = 4,
    Diverged = 5,
    Error = 6,
}

pub struct GitState {
    /// Remote status represented as u8 for lock-free atomic access.
    pub remote_status: AtomicU8,
    /// Current remote origin URL.
    pub remote_url: std::sync::Mutex<String>,
}

impl GitState {
    pub fn new() -> Self {
        Self {
            remote_status: AtomicU8::new(GitRemoteStatus::Unknown as u8),
            remote_url: std::sync::Mutex::new("None".into()),
        }
    }

    pub fn status(&self) -> GitRemoteStatus {
        match self.remote_status.load(Ordering::Relaxed) {
            1 => GitRemoteStatus::NoRemote,
            2 => GitRemoteStatus::Connected,
            3 => GitRemoteStatus::Behind,
            4 => GitRemoteStatus::Ahead,
            5 => GitRemoteStatus::Diverged,
            6 => GitRemoteStatus::Error,
            _ => GitRemoteStatus::Unknown,
        }
    }

    pub fn label(&self) -> String {
        match self.status() {
            GitRemoteStatus::Unknown => "UNKNOWN".into(),
            GitRemoteStatus::NoRemote => "NONE".into(),
            GitRemoteStatus::Connected => "CONNECTED".into(),
            GitRemoteStatus::Behind => "BEHIND".into(),
            GitRemoteStatus::Ahead => "AHEAD".into(),
            GitRemoteStatus::Diverged => "OUT-OF-SYNC".into(),
            GitRemoteStatus::Error => "ERR".into(),
        }
    }

    pub fn url(&self) -> String {
        self.remote_url.lock().unwrap().clone()
    }
}

pub fn spawn_git_monitor() -> Arc<GitState> {
    let state = Arc::new(GitState::new());
    let bg = state.clone();

    tokio::spawn(async move {
        // Initial delay to avoid slowing down startup.
        tokio::time::sleep(Duration::from_secs(5)).await;

        loop {
            if let Some((status, url)) = check_git_status().await {
                bg.remote_status.store(status as u8, Ordering::Relaxed);
                if let Ok(mut u) = bg.remote_url.lock() {
                    *u = url;
                }
            }
            // Poll every 5 minutes (300s). Git operations are relatively expensive.
            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    });

    state
}

async fn check_git_status() -> Option<(GitRemoteStatus, String)> {
    // 1. Check if it's a git repo.
    let repo_check = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .await
        .ok()?;

    if !repo_check.status.success() {
        return Some((GitRemoteStatus::NoRemote, "Not a Repo".into()));
    }

    // 2. Check for remotes.
    let remote_check = Command::new("git")
        .args(["remote"])
        .output()
        .await
        .ok()?;

    let remotes = String::from_utf8_lossy(&remote_check.stdout).trim().to_string();
    if remotes.is_empty() {
        return Some((GitRemoteStatus::NoRemote, "None".into()));
    }

    // 3. Get remote URL (assume 'origin' but check first remote if origin missing).
    let primary_remote = if remotes.contains("origin") { "origin" } else { remotes.split_whitespace().next().unwrap_or("origin") };
    let url_check = Command::new("git")
        .args(["remote", "get-url", primary_remote])
        .output()
        .await
        .ok()?;
    let url = String::from_utf8_lossy(&url_check.stdout).trim().to_string();

    // 4. Fetch to check sync status (optional, but requested for "persistent check").
    // We do a "quiet" fetch.
    let _ = Command::new("git")
        .args(["fetch", "--quiet", primary_remote])
        .output()
        .await;

    // 5. Compare local and remote.
    let sync_check = Command::new("git")
        .args(["rev-list", "--left-right", "--count", "HEAD...HEAD@{u}"])
        .output()
        .await
        .ok()?;

    if sync_check.status.success() {
        let counts = String::from_utf8_lossy(&sync_check.stdout).trim().to_string();
        let parts: Vec<&str> = counts.split_whitespace().collect();
        if parts.len() == 2 {
            let ahead: u32 = parts[0].parse().unwrap_or(0);
            let behind: u32 = parts[1].parse().unwrap_or(0);

            if ahead > 0 && behind > 0 {
                return Some((GitRemoteStatus::Diverged, url));
            } else if ahead > 0 {
                return Some((GitRemoteStatus::Ahead, url));
            } else if behind > 0 {
                return Some((GitRemoteStatus::Behind, url));
            } else {
                return Some((GitRemoteStatus::Connected, url));
            }
        }
    }

    // If rev-list fails, it might mean there's no upstream branch set.
    Some((GitRemoteStatus::Connected, url))
}
