//! Background GPU VRAM monitor.
//!
//! Spawns a Tokio task that polls `nvidia-smi` every few seconds and stores
//! the result in lock-free atomics so the TUI render loop can read it cheaply.

use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

lazy_static! {
    /// Global access to GPU vitals for tool investigation (Zero-Shot Trends).
    pub static ref GLOBAL_GPU_STATE: Arc<GpuState> = Arc::new(GpuState::new());
}

/// Shared GPU state — read by the TUI/Agent, written by the background poller.
#[derive(Debug)]
pub struct GpuState {
    /// VRAM used in MiB.
    pub used_mib: AtomicU32,
    /// VRAM total in MiB.
    pub total_mib: AtomicU32,
    /// GPU name (set once on first successful poll).
    pub name: Mutex<String>,
    /// Recent history points (max 10).
    pub history: Mutex<VecDeque<HistoryPoint>>,
}

#[derive(Debug, Clone)]
pub struct HistoryPoint {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub used_mib: u32,
    pub temperature: u32,
    pub core_clock: u32,
    pub mem_clock: u32,
    pub power_draw: f32,
    pub fan_speed: u32,
    pub throttle_reasons: String,
}

impl GpuState {
    pub fn new() -> Self {
        Self {
            used_mib: AtomicU32::new(0),
            total_mib: AtomicU32::new(0),
            name: Mutex::new("GPU".into()),
            history: Mutex::new(VecDeque::with_capacity(10)),
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
        format!(
            "{:.1} GB / {:.1} GB",
            used as f64 / 1024.0,
            total as f64 / 1024.0
        )
    }

    /// Returns the GPU name (e.g. "NVIDIA GeForce RTX 4070").
    pub fn gpu_name(&self) -> String {
        self.name.lock().unwrap().clone()
    }
}

/// Spawn the background polling task. Returns the shared state handle.
pub fn spawn_gpu_monitor() -> Arc<GpuState> {
    let state = GLOBAL_GPU_STATE.clone();
    let bg = state.clone();

    tokio::spawn(async move {
        let mut poll_count = 0u64;
        loop {
            if let Some(metrics) = poll_nvidia_smi().await {
                bg.used_mib.store(metrics.used_mib, Ordering::Relaxed);
                bg.total_mib.store(metrics.total_mib, Ordering::Relaxed);
                if !metrics.name.is_empty() {
                    let mut name = bg.name.lock().unwrap();
                    if *name == "GPU" {
                        *name = metrics.name;
                    }
                }

                // Add to history every ~2 minutes (60 iterations @ 2s each)
                if poll_count % 60 == 0 {
                    let mut history = bg.history.lock().unwrap();
                    history.push_back(HistoryPoint {
                        timestamp: chrono::Local::now(),
                        used_mib: metrics.used_mib,
                        temperature: metrics.temperature,
                        core_clock: metrics.core_clock,
                        mem_clock: metrics.mem_clock,
                        power_draw: metrics.power_draw,
                        fan_speed: metrics.fan_speed,
                        throttle_reasons: metrics.throttle_reasons,
                    });
                    if history.len() > 10 {
                        history.pop_front();
                    }
                }
            }
            poll_count += 1;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });

    state
}

pub struct GpuMetrics {
    pub used_mib: u32,
    pub total_mib: u32,
    pub name: String,
    pub temperature: u32,
    pub core_clock: u32,
    pub mem_clock: u32,
    pub power_draw: f32,
    pub fan_speed: u32,
    pub throttle_reasons: String,
}

/// Call nvidia-smi and parse the CSV output.
async fn poll_nvidia_smi() -> Option<GpuMetrics> {
    let output = tokio::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.used,memory.total,name,temperature.gpu,clocks.current.graphics,clocks.current.memory,power.draw,fan.speed,clocks_throttle_reasons.active",
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
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if parts.len() < 9 {
        return None;
    }

    Some(GpuMetrics {
        used_mib: parts[0].parse().ok()?,
        total_mib: parts[1].parse().ok()?,
        name: parts[2].to_string(),
        temperature: parts[3].parse().ok()?,
        core_clock: parts[4].parse().ok()?,
        mem_clock: parts[5].parse().ok()?,
        power_draw: parts[6].parse().unwrap_or(0.0),
        fan_speed: parts[7].parse().unwrap_or(0),
        throttle_reasons: parts[8].to_string(),
    })
}
