use std::sync::Arc;
use tokio::task::JoinSet;
use super::inference::InferenceEngine;
use super::parser::{WorkerTask, Hunk};
use std::path::{Path, PathBuf};
use std::fs;
use tokio::sync::oneshot;

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

/// The Core parallel orchestrator locking background models to strict 12GB KV cache boundaries.
pub struct SwarmCoordinator {
    pub engine: Arc<InferenceEngine>,
    pub scratch_dir: PathBuf,
    pub worker_model: Option<String>,
    pub gpu_state: Arc<crate::ui::gpu_monitor::GpuState>,
    #[allow(dead_code)]
    pub professional: bool,
}

impl SwarmCoordinator {
    pub fn new(engine: Arc<InferenceEngine>, gpu_state: Arc<crate::ui::gpu_monitor::GpuState>, worker_model: Option<String>, professional: bool) -> Self {
        let root = crate::tools::file_ops::workspace_root();
        let hematite_dir = root.join(".hematite");
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
        
        Self { engine, scratch_dir, worker_model, gpu_state, professional }
    }

    /// Spawns parallel execution green-threads while respecting the hardware-aware limit.
    pub async fn dispatch_swarm(&self, tasks: Vec<WorkerTask>, progression_tx: tokio::sync::mpsc::Sender<SwarmMessage>, max_workers: usize) -> Result<(), String> {
        let mut join_set = JoinSet::new();

        // ── VRAM-Aware Throttling ──
        // If VRAM is > 85% used, we drop to Sequential Mode to prevent crashes.
        let vram_usage = self.gpu_state.ratio();
        let is_sequential = vram_usage > 0.85;
        
        if is_sequential {
            let _ = progression_tx.send(SwarmMessage::Progress("CPU/GPU GUARD".to_string(), 0)).await;
            let _ = progression_tx.send(SwarmMessage::Progress("LOW VRAM: Switching to Sequential Mode".to_string(), 1)).await;
        }

        for task in tasks.into_iter().take(max_workers) {
            let engine_clone = self.engine.clone();
            let tx_clone = progression_tx.clone();
            let scratch_path = self.scratch_dir.join(format!("worker_{}.diff", task.id));
            let worker_job = async move {
                // 1) Research
                let _ = tx_clone.send(SwarmMessage::Progress(task.id.clone(), 25)).await;
                
                // 2) Native Synthesis Gen (Batch context evaluation)
                let prompt = format!(
                    "TARGET: {}\nDIRECTIVE: {}\n\n[HEMATITE SYNTHESIS BAN]\nYou are explicitly forbidden from lazy delegation (e.g. saying 'based on worker findings'). You MUST execute a Synthesis Pass dynamically: 1) Read the actual findings. 2) Specify the concrete integration logic yourself. 3) Output code directly targeting the exact bounds.", 
                    task.target, task.instruction
                );
                
                // Use the generate_task_worker path which respects asymmetric model IDs
                if let Ok(res) = engine_clone.generate_task_worker(&prompt, true).await { 
                    let _ = tx_clone.send(SwarmMessage::Progress(task.id.clone(), 75)).await;
                    
                    // 3) Push directly into Scratchpad isolating original File Locks
                    let _ = std::fs::write(&scratch_path, res.clone());
                    let _ = tx_clone.send(SwarmMessage::Progress(task.id.clone(), 100)).await;

                    // 4) High-End Oversight: Trigger Human Review for EVERY successful generation
                    let target_path = PathBuf::from(task.target.clone());
                    let before = if target_path.is_file() {
                        std::fs::read_to_string(&target_path).unwrap_or_else(|_| "[Error reading context]".to_string())
                    } else {
                        format!("[SYNERGY: Exploring {}]", task.target)
                    };
                    
                    let (res_tx, res_rx) = oneshot::channel();
                    let _ = tx_clone.send(SwarmMessage::ReviewRequest {
                        worker_id: task.id.clone(),
                        file_path: target_path.clone(),
                        before,
                        after: res.clone(),
                        tx: res_tx,
                    }).await;

                    // Sync 2-Way Lock completely halting execution until Architect signs off
                    let _ = res_rx.await;
                }
            };

            if is_sequential {
                worker_job.await;
            } else {
                join_set.spawn(worker_job);
            }
        }

        // Orchestrator patiently waits natively evaluating background executions
        while let Some(_) = join_set.join_next().await {
            // Evaluates patches passively
        }
        
        let _ = progression_tx.send(SwarmMessage::Done).await;
        Ok(())
    }

    /// Evaluates compiled scratchpad chunks backwards utilizing Reverse sorting organically slicing VRAM limits natively!
    #[allow(dead_code)]
    pub async fn apply_patches_descending(
        &self,
        file_path: &Path, 
        mut hunks: Vec<Hunk>, 
        progression_tx: tokio::sync::mpsc::Sender<SwarmMessage>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut lines: Vec<String> = fs::read_to_string(file_path)?
            .lines()
            .map(|s| s.to_string())
            .collect();

        // The Golden Rule: Sort Descending natively
        hunks.sort_by_key(|h| h.sort_key());

        let mut i = 0;
        while i < hunks.len() {
            let current = &hunks[i];
            
            // Look ahead for overlaps (Conflicts)
            if i + 1 < hunks.len() && hunks[i+1].end_line >= current.start_line {
                // CONFLICT DETECTED: Tier 1 Synthesis Merge Pass targeting isolated context ranges
                let mut retry_count = 0u32;
                const MAX_CONFLICT_RETRIES: u32 = 3;
                loop {
                    if retry_count >= MAX_CONFLICT_RETRIES {
                        // Give up and skip both conflicting hunks.
                        i += 2;
                        break;
                    }
                    // Safety Net Context Expansion: Double the inference bounds on retry dynamically mapping logic
                    let padding: usize = 10 + (retry_count as usize * 10);
                    let conflict_start = hunks[i+1].start_line.saturating_sub(padding);
                    let conflict_end = (current.end_line + padding).min(lines.len());
                    let context = lines[conflict_start..conflict_end].join("\n");
                    
                    let prompt = if retry_count == 0 {
                        format!("CONFLICT in {}.\nContext:\n{}\n\nWorker {} wants: {}\nWorker {} wants: {}\nResolve these into one block.",
                        file_path.display(), context, current.worker_id, current.content, hunks[i+1].worker_id, hunks[i+1].content)
                    } else {
                        format!("CRITICAL: Your previous synthesis for this conflict was REJECTED by the human architect.\nThe merge you proposed was logically unsound.\nDO NOT REPEAT MISTAKES.\n\nCONFLICT in {}.\nContext:\n{}\n\nWorker {} wants: {}\nWorker {} wants: {}\nResolve these into one robust logical block.",
                        file_path.display(), context, current.worker_id, current.content, hunks[i+1].worker_id, hunks[i+1].content)
                    };

                    // Scale Chaos temperature natively explicitly to evade deterministic loops
                    let temp = if retry_count > 0 { 0.7 } else { 0.1 };
                    
                    let resolved_block = self.engine.generate_task_with_temp(&prompt, temp, true)
                        .await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::from(e) })?;
                    
                    // Cross the dimensional bound! Halt Background native evaluating physical Main interaction!
                    let (response_tx, response_rx) = oneshot::channel();
                    let _ = progression_tx.send(SwarmMessage::ReviewRequest {
                        worker_id: current.worker_id.clone(),
                        file_path: file_path.to_path_buf(),
                        before: context.clone(),
                        after: resolved_block.clone(),
                        tx: response_tx,
                    }).await;

                    // Sync 2-Way Lock completely halting Orchestrator 
                    match response_rx.await.unwrap_or(ReviewResponse::Reject) {
                        ReviewResponse::Accept => {
                            lines.splice(conflict_start..conflict_end, vec![resolved_block]);
                            i += 2;
                            break;
                        }
                        ReviewResponse::Retry => {
                            retry_count += 1;
                            continue; // Organically loops back utilizing dynamically expanded Context and Chaos Temps
                        }
                        ReviewResponse::Reject => {
                            i += 2; // Jump over merged hunk traces explicitly abandoning patch overlay
                            break;
                        }
                    }
                }
            } else {
                // Safe absolute hunk application preventing index drift identically
                let start_idx = current.start_line.saturating_sub(1);
                let end_idx = current.end_line.min(lines.len());
                let range = start_idx..end_idx;
                lines.splice(range, vec![current.content.clone()]);
                i += 1;
            }
        }

        fs::write(file_path, lines.join("\n"))?;
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
