//! Background GPU VRAM monitor.
//!
//! Spawns a Tokio task that polls `nvidia-smi` every few seconds and stores
//! the result in lock-free atomics so the TUI render loop can read it cheaply.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Shared GPU state — read by the TUI, written by the background poller.
#[derive(Debug)]
pub struct GpuState {
    /// VRAM used in MiB.
    pub used_mib: AtomicU32,
    /// VRAM total in MiB.
    pub total_mib: AtomicU32,
    /// GPU name (set once on first successful poll).
    pub name: std::sync::Mutex<String>,
}

impl GpuState {
    pub fn new() -> Self {
        Self {
            used_mib: AtomicU32::new(0),
            total_mib: AtomicU32::new(0),
            name: std::sync::Mutex::new("GPU".into()),
        }
    }

    /// Returns (used_mib, total_mib).
    pub fn read(&self) -> (u32, u32) {
        (
            self.used_mib.load(Ordering::Relaxed),
            self.total_mib.load(Ordering::Relaxed),
        )
    }

    /// Returns the ratio used/total, clamped to [0.0, 1.0].
    pub fn ratio(&self) -> f64 {
        let (used, total) = self.read();
        if total == 0 {
            return 0.0;
        }
        (used as f64 / total as f64).clamp(0.0, 1.0)
    }

    /// Returns a human-readable label like "7.5 GB / 12.0 GB".
    pub fn label(&self) -> String {
        let (used, total) = self.read();
        if total == 0 {
            return "N/A".into();
        }
        format!("{:.1} GB / {:.1} GB", used as f64 / 1024.0, total as f64 / 1024.0)
    }

    /// Returns the GPU name (e.g. "NVIDIA GeForce RTX 4070").
    pub fn gpu_name(&self) -> String {
        self.name.lock().unwrap().clone()
    }
}

/// Spawn the background polling task. Returns the shared state handle.
pub fn spawn_gpu_monitor() -> Arc<GpuState> {
    let state = Arc::new(GpuState::new());
    let bg = state.clone();

    tokio::spawn(async move {
        loop {
            if let Some((used, total, name)) = poll_nvidia_smi().await {
                bg.used_mib.store(used, Ordering::Relaxed);
                bg.total_mib.store(total, Ordering::Relaxed);
                if !name.is_empty() {
                    *bg.name.lock().unwrap() = name;
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });

    state
}

/// Call nvidia-smi and parse the CSV output.
async fn poll_nvidia_smi() -> Option<(u32, u32, String)> {
    let output = tokio::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.used,memory.total,name",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();
    let parts: Vec<&str> = line.splitn(3, ',').collect();
    if parts.len() < 2 {
        return None;
    }

    let used: u32 = parts[0].trim().parse().ok()?;
    let total: u32 = parts[1].trim().parse().ok()?;
    let name = parts.get(2).map(|s| s.trim().to_string()).unwrap_or_default();

    Some((used, total, name))
}
