use crate::agent;
use crate::agent::conversation::ConversationManager;
use crate::agent::git_monitor::GitState;
use crate::agent::inference::{InferenceEngine, InferenceEvent};
use crate::ui;
use crate::ui::gpu_monitor::GpuState;
use crate::ui::voice::VoiceManager;
use crate::CliCockpit;
use notify::RecommendedWatcher;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct RuntimeServices {
    pub engine: Arc<InferenceEngine>,
    pub gpu_state: Arc<GpuState>,
    pub git_state: Arc<GitState>,
    pub voice_manager: Arc<VoiceManager>,
    pub swarm_coordinator: Arc<agent::swarm::SwarmCoordinator>,
    pub cancel_token: Arc<std::sync::atomic::AtomicBool>,
}

pub struct RuntimeChannels {
    pub specular_rx: mpsc::Receiver<agent::specular::SpecularEvent>,
    pub agent_tx: mpsc::Sender<InferenceEvent>,
    pub agent_rx: mpsc::Receiver<InferenceEvent>,
    pub swarm_tx: mpsc::Sender<agent::swarm::SwarmMessage>,
    pub swarm_rx: mpsc::Receiver<agent::swarm::SwarmMessage>,
    pub user_input_tx: mpsc::Sender<String>,
    pub user_input_rx: mpsc::Receiver<String>,
}

pub struct RuntimeBundle {
    pub services: RuntimeServices,
    pub channels: RuntimeChannels,
    pub watcher_guard: RecommendedWatcher,
}

pub struct AgentLoopRuntime {
    pub user_input_rx: mpsc::Receiver<String>,
    pub agent_tx: mpsc::Sender<InferenceEvent>,
    pub services: RuntimeServices,
}

pub struct AgentLoopConfig {
    pub yolo: bool,
    pub professional: bool,
    pub brief: bool,
    pub snark: u8,
    pub chaos: u8,
    pub fast_model: Option<String>,
    pub think_model: Option<String>,
}

pub async fn build_runtime_bundle(
    cockpit: &CliCockpit,
    species: &str,
    snark: u8,
    professional: bool,
) -> Result<RuntimeBundle, Box<dyn std::error::Error>> {
    println!("Booting Hematite systems...");
    // settings.json api_url overrides the --url CLI flag so users don't need to retype it.
    let api_url = crate::agent::config::load_config()
        .api_url
        .unwrap_or_else(|| cockpit.url.clone());
    let mut engine_raw = InferenceEngine::new(api_url, species.to_string(), snark)?;
    let gpu_state = ui::gpu_monitor::spawn_gpu_monitor();
    let git_state = agent::git_monitor::spawn_git_monitor();

    if !engine_raw.health_check().await {
        println!("ERROR: LLM Provider not detected at {}", engine_raw.base_url);
        println!("Check if LM Studio (or your local server) is running and port mapped correctly.");
        std::process::exit(1);
    }

    let model_name = engine_raw.get_loaded_model().await;
    if let Some(name) = model_name {
        engine_raw.set_runtime_profile(&name, engine_raw.current_context_length());
    }
    let detected_context = engine_raw.detect_context_length().await;
    let detected_model = engine_raw.current_model();
    engine_raw.set_runtime_profile(&detected_model, detected_context);

    let (specular_tx, specular_rx) = mpsc::channel(32);
    let watcher_guard = agent::specular::spawn_watcher(specular_tx)?;

    let (agent_tx, agent_rx) = mpsc::channel::<InferenceEvent>(100);
    let (swarm_tx, swarm_rx) = mpsc::channel(32);
    let voice_manager = Arc::new(VoiceManager::new(agent_tx.clone()));

    if let Some(ref worker) = cockpit.fast_model {
        engine_raw.worker_model = Some(worker.clone());
    }

    let engine = Arc::new(engine_raw);
    let swarm_coordinator = Arc::new(agent::swarm::SwarmCoordinator::new(
        engine.clone(),
        gpu_state.clone(),
        cockpit.fast_model.clone(),
        professional,
    ));

    let (user_input_tx, user_input_rx) = mpsc::channel::<String>(32);
    let cancel_token = Arc::new(std::sync::atomic::AtomicBool::new(false));

    Ok(RuntimeBundle {
        services: RuntimeServices {
            engine,
            gpu_state,
            git_state,
            voice_manager,
            swarm_coordinator,
            cancel_token,
        },
        channels: RuntimeChannels {
            specular_rx,
            agent_tx,
            agent_rx,
            swarm_tx,
            swarm_rx,
            user_input_tx,
            user_input_rx,
        },
        watcher_guard,
    })
}

pub fn spawn_runtime_profile_sync(
    engine: Arc<InferenceEngine>,
    agent_tx: mpsc::Sender<InferenceEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Initial delay before the first background poll.
        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

        loop {
            let result = engine.refresh_runtime_profile().await;

            let Some((model_id, context_length, _changed)) = result else {
                if agent_tx.is_closed() {
                    break;
                }
                // LM Studio unreachable — back off; no need to hammer a closed server.
                tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
                continue;
            };

            // When no coding model is loaded, back off to reduce log noise in LM Studio.
            let poll_interval = if model_id == "no model loaded" {
                tokio::time::Duration::from_secs(12)
            } else {
                tokio::time::Duration::from_secs(4)
            };

            if agent_tx
                .send(InferenceEvent::RuntimeProfile {
                    model_id,
                    context_length,
                })
                .await
                .is_err()
            {
                break;
            }

            tokio::time::sleep(poll_interval).await;
        }
    })
}

pub async fn run_agent_loop(runtime: AgentLoopRuntime, config: AgentLoopConfig) {
    let AgentLoopRuntime {
        mut user_input_rx,
        agent_tx,
        services,
    } = runtime;
    let RuntimeServices {
        engine,
        gpu_state,
        git_state,
        voice_manager,
        swarm_coordinator,
        cancel_token,
    } = services;

    let mut manager = ConversationManager::new(
        engine,
        config.professional,
        config.brief,
        config.snark,
        config.chaos,
        config.fast_model,
        config.think_model,
        gpu_state.clone(),
        git_state,
        swarm_coordinator,
        voice_manager,
    );
    manager.cancel_token = cancel_token;

    let _ = agent_tx
        .send(InferenceEvent::RuntimeProfile {
            model_id: manager.engine.current_model(),
            context_length: manager.engine.current_context_length(),
        })
        .await;

    if let Err(e) = manager.initialize_mcp().await {
        let _ = agent_tx
            .send(InferenceEvent::Error(format!("MCP Init Failed: {}", e)))
            .await;
    }
    let indexed = manager.initialize_vein();
    let _ = agent_tx
        .send(InferenceEvent::Thought(format!("The Vein: indexed {} files", indexed)))
        .await;
    let _ = agent_tx.send(InferenceEvent::Done).await;

    let gpu_name = gpu_state.gpu_name();
    let vram = gpu_state.label();

    let embed_status = if manager.vein.embedded_chunk_count() > 0 {
        "Embed: nomic active (semantic search ready)"
    } else {
        "Embed: none loaded (BM25 only — load nomic-embed-text-v2 for semantic search)"
    };

    let voice_cfg = crate::agent::config::load_config();
    let voice_status = format!(
        "Voice: {} | Speed: {}x | Volume: {}x",
        crate::agent::config::effective_voice(&voice_cfg),
        crate::agent::config::effective_voice_speed(&voice_cfg),
        crate::agent::config::effective_voice_volume(&voice_cfg),
    );

    let greeting = format!(
        "Hematite Online | Model: {} | CTX: {} | GPU: {} | VRAM: {}\nEndpoint: {}\n{}\n{}",
        manager.engine.current_model(),
        manager.engine.current_context_length(),
        gpu_name,
        vram,
        format!("{}/v1", manager.engine.base_url),
        embed_status,
        voice_status
    );

    let _ = agent_tx
        .send(InferenceEvent::MutedToken(format!("\n{}", greeting)))
        .await;
    let startup_config = crate::agent::config::load_config();
    manager.engine.set_gemma_native_formatting(
        crate::agent::config::effective_gemma_native_formatting(
            &startup_config,
            &manager.engine.current_model(),
        ),
    );
    let startup_model = manager.engine.current_model();
    if crate::agent::inference::is_gemma4_model_name(&startup_model) {
        let mode = crate::agent::config::gemma_native_mode_label(&startup_config, &startup_model);
        let status = match mode {
            "on" => "Gemma 4 detected | Gemma Native Formatting: ON (forced)",
            "auto" => "Gemma 4 detected | Gemma Native Formatting: ON (auto)",
            _ => "Gemma 4 detected | Gemma Native Formatting: OFF (use /gemma-native auto|on)",
        };
        let _ = agent_tx
            .send(InferenceEvent::MutedToken(status.to_string()))
            .await;
    }

    while let Some(input) = user_input_rx.recv().await {
        if let Err(e) = manager.run_turn(&input, agent_tx.clone(), config.yolo).await {
            let _ = agent_tx.send(InferenceEvent::Error(e.to_string())).await;
            let _ = agent_tx.send(InferenceEvent::Done).await;
        }
    }
}