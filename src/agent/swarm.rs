use super::inference::InferenceEngine;
use super::parser::WorkerTask;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::task::JoinSet;

pub enum ReviewResponse {
    Accept,
    Reject,
    Retry,
}

pub enum SwarmMessage {
    Progress(String, u8),
    ReviewRequest {
        worker_id: String,
        file_path: PathBuf,
        before: String,
        after: String,
        tx: oneshot::Sender<ReviewResponse>,
    },
    Done,
}

/// Parallel worker orchestrator. Dispatches tasks to background inference threads
/// with VRAM-aware throttling and a human-review gate before applying any output.
pub struct SwarmCoordinator {
    pub engine: Arc<InferenceEngine>,
    pub scratch_dir: PathBuf,
    pub worker_model: Option<String>,
    pub gpu_state: Arc<crate::ui::gpu_monitor::GpuState>,
    #[allow(dead_code)]
    pub professional: bool,
}

impl SwarmCoordinator {
    pub fn new(
        engine: Arc<InferenceEngine>,
        gpu_state: Arc<crate::ui::gpu_monitor::GpuState>,
        worker_model: Option<String>,
        professional: bool,
    ) -> Self {
        let root = crate::tools::file_ops::workspace_root();
        let hematite_dir = crate::tools::file_ops::hematite_dir();
        let scratch_dir = hematite_dir.join("scratch");

        let gitignore_path = root.join(".gitignore");
        if gitignore_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&gitignore_path) {
                if !content.contains(".hematite") {
                    let mut new_content = content;
                    if !new_content.ends_with('\n') {
                        new_content.push('\n');
                    }
                    new_content.push_str(".hematite/\n");
                    let _ = std::fs::write(&gitignore_path, new_content);
                }
            }
        }

        if !hematite_dir.exists() {
            let _ = std::fs::create_dir_all(&hematite_dir);
        }
        if !scratch_dir.exists() {
            let _ = std::fs::create_dir_all(&scratch_dir);
        }

        Self {
            engine,
            scratch_dir,
            worker_model,
            gpu_state,
            professional,
        }
    }

    /// Spawns parallel execution green-threads while respecting the hardware-aware limit.
    pub async fn dispatch_swarm(
        &self,
        tasks: Vec<WorkerTask>,
        progression_tx: tokio::sync::mpsc::Sender<SwarmMessage>,
        max_workers: usize,
    ) -> Result<(), String> {
        let mut join_set = JoinSet::new();

        // ── VRAM-Aware Throttling ──
        // If VRAM is > 85% used, we drop to Sequential Mode to prevent crashes.
        let vram_usage = self.gpu_state.ratio();
        let is_sequential = vram_usage > 0.85;

        if is_sequential {
            let _ = progression_tx
                .send(SwarmMessage::Progress("CPU/GPU GUARD".to_string(), 0))
                .await;
            let _ = progression_tx
                .send(SwarmMessage::Progress(
                    "LOW VRAM: Switching to Sequential Mode".to_string(),
                    1,
                ))
                .await;
        }

        for task in tasks.into_iter().take(max_workers) {
            let engine_clone = self.engine.clone();
            let tx_clone = progression_tx.clone();
            let scratch_path = self.scratch_dir.join(format!("worker_{}.diff", task.id));
            let worker_job = async move {
                // 1) Research
                let _ = tx_clone
                    .send(SwarmMessage::Progress(task.id.clone(), 25))
                    .await;

                // 2) Native Synthesis Gen (Batch context evaluation)
                let prompt = format!(
                    "TARGET: {}\nDIRECTIVE: {}\n\n[HEMATITE SYNTHESIS BAN]\nYou are explicitly forbidden from lazy delegation (e.g. saying 'based on worker findings'). You MUST execute a Synthesis Pass dynamically: 1) Read the actual findings. 2) Specify the concrete integration logic yourself. 3) Output code directly targeting the exact bounds.", 
                    task.target, task.instruction
                );

                // Use the generate_task_worker path which respects asymmetric model IDs
                match engine_clone.generate_task_worker(&prompt, true).await {
                    Ok(res) => {
                        let _ = tx_clone
                            .send(SwarmMessage::Progress(task.id.clone(), 75))
                            .await;

                        // 3) Push directly into Scratchpad isolating original File Locks
                        let _ = std::fs::write(&scratch_path, res.clone());
                        let _ = tx_clone
                            .send(SwarmMessage::Progress(task.id.clone(), 100))
                            .await;

                        // 4) High-End Oversight: Trigger Human Review for EVERY successful generation
                        let target_path = PathBuf::from(task.target.clone());
                        let before = if target_path.is_file() {
                            std::fs::read_to_string(&target_path)
                                .unwrap_or_else(|_| "[Error reading context]".to_string())
                        } else {
                            format!("[SYNERGY: Exploring {}]", task.target)
                        };

                        let (res_tx, res_rx) = oneshot::channel();
                        let _ = tx_clone
                            .send(SwarmMessage::ReviewRequest {
                                worker_id: task.id.clone(),
                                file_path: target_path.clone(),
                                before,
                                after: res.clone(),
                                tx: res_tx,
                            })
                            .await;

                        // Block until the operator accepts or rejects the diff.
                        let _ = res_rx.await;
                    }
                    Err(e) => {
                        let _ = tx_clone
                            .send(SwarmMessage::Progress(
                                format!("worker {} failed: {e}", task.id),
                                0,
                            ))
                            .await;
                    }
                }
            };

            if is_sequential {
                worker_job.await;
            } else {
                join_set.spawn(worker_job);
            }
        }

        while join_set.join_next().await.is_some() {}

        let _ = progression_tx.send(SwarmMessage::Done).await;
        Ok(())
    }
}

impl Drop for SwarmCoordinator {
    fn drop(&mut self) {
        // Emergency Cleanup: Wipe the scratchpad contents.
        // This fires on normal exit, Ctrl+C (via tokio's signal handler), or panic unwind.
        if self.scratch_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&self.scratch_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        let _ = std::fs::remove_file(p);
                    }
                }
            }
        }
        eprintln!("[Hematite] Swarm shutdown complete. Scratchpad wiped.");
    }
}
