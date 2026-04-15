use crate::agent;
use crate::agent::conversation::{ConversationManager, UserTurn};
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
    pub user_input_tx: mpsc::Sender<UserTurn>,
    pub user_input_rx: mpsc::Receiver<UserTurn>,
}

pub struct RuntimeBundle {
    pub services: RuntimeServices,
    pub channels: RuntimeChannels,
    pub watcher_guard: RecommendedWatcher,
}

pub struct AgentLoopRuntime {
    pub user_input_rx: mpsc::Receiver<UserTurn>,
    pub agent_tx: mpsc::Sender<InferenceEvent>,
    pub services: RuntimeServices,
}

pub struct AgentLoopConfig {
    pub yolo: bool,
    pub professional: bool,
    pub brief: bool,
    pub snark: u8,
    pub chaos: u8,
    pub soul_personality: String,
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
        println!(
            "ERROR: LLM Provider not detected at {}",
            engine_raw.base_url
        );
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

    let (user_input_tx, user_input_rx) = mpsc::channel::<UserTurn>(32);
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

        let mut last_embed: Option<String> = None;

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

            // Poll embed model separately and notify on change.
            let current_embed = engine.get_embedding_model().await;
            if current_embed != last_embed {
                if agent_tx
                    .send(InferenceEvent::EmbedProfile {
                        model_id: current_embed.clone(),
                    })
                    .await
                    .is_err()
                {
                    break;
                }
                last_embed = current_embed;
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
        config.soul_personality,
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

    let workspace_root = crate::tools::file_ops::workspace_root();
    let _ = crate::agent::workspace_profile::ensure_workspace_profile(&workspace_root);

    // Send the startup greeting immediately — before MCP and Vein so it always
    // appears right away, even if vein indexing takes a while on first run.
    let gpu_name = gpu_state.gpu_name();
    let vram = gpu_state.label();
    let voice_cfg = crate::agent::config::load_config();
    let voice_status = format!(
        "Voice: {} | Speed: {}x | Volume: {}x",
        crate::agent::config::effective_voice(&voice_cfg),
        crate::agent::config::effective_voice_speed(&voice_cfg),
        crate::agent::config::effective_voice_volume(&voice_cfg),
    );
    let embed_status = match manager.engine.get_embedding_model().await {
        Some(id) => format!("Embed: {} (semantic search ready)", id),
        None => "Embed: none loaded (load nomic-embed-text-v2 for semantic search)".to_string(),
    };
    let workspace_root = crate::tools::file_ops::workspace_root();
    let docs_only_mode = !crate::tools::file_ops::is_project_workspace();
    let workspace_mode = if docs_only_mode {
        "docs-only"
    } else {
        "project"
    };
    let launched_from_home = home::home_dir()
        .and_then(|home| std::env::current_dir().ok().map(|cwd| cwd == home))
        .unwrap_or(false);
    let project_hint = if !docs_only_mode {
        String::new()
    } else if launched_from_home {
        "\nTip: you launched Hematite from your home directory. That is fine for workstation questions and docs-only memory, but for project-specific build, test, script, or repo work you should relaunch in the target project directory. `.hematite/docs/`, `.hematite/imports/`, and recent local session reports remain searchable in docs-only vein mode.".to_string()
    } else {
        "\nTip: source indexing is disabled outside a project folder. Launch Hematite in the target project directory for project-specific build, test, script, or repo work. `.hematite/docs/`, `.hematite/imports/`, and recent local session reports remain searchable in docs-only vein mode.".to_string()
    };
    let display_model = {
        let m = manager.engine.current_model();
        if m.is_empty() {
            "no chat model loaded".to_string()
        } else {
            m
        }
    };
    let greeting = format!(
        "Hematite {} Online | Model: {} | CTX: {} | GPU: {} | VRAM: {}\nEndpoint: {}\nWorkspace: {} ({})\n{}\n{}\n/ask · read-only analysis   /code · implement   /architect · plan-first   /chat · conversation\nRecovery: /undo · /new · /forget · /clear   |   /version · /about{}",
        crate::hematite_version_display(),
        display_model,
        manager.engine.current_context_length(),
        gpu_name,
        vram,
        format!("{}/v1", manager.engine.base_url),
        workspace_root.display(),
        workspace_mode,
        embed_status,
        voice_status,
        project_hint
    );
    let _ = agent_tx
        .send(InferenceEvent::MutedToken(format!("\n{}", greeting)))
        .await;

    if let Err(e) = manager.initialize_mcp(&agent_tx).await {
        let _ = agent_tx
            .send(InferenceEvent::Error(format!("MCP Init Failed: {}", e)))
            .await;
    }
    let indexed = manager.initialize_vein();
    manager.initialize_repo_map();
    let _ = agent_tx
        .send(InferenceEvent::VeinStatus {
            file_count: manager.vein.file_count(),
            embedded_count: manager.vein.embedded_chunk_count(),
            docs_only: docs_only_mode,
        })
        .await;
    let _ = agent_tx
        .send(InferenceEvent::Thought(format!(
            "The Vein: indexed {} files",
            indexed
        )))
        .await;

    // Show a compact resume line if a prior session left a checkpoint.
    if let Some(cp) = crate::agent::conversation::load_checkpoint() {
        let verify_tag = match cp.last_verify_ok {
            Some(true) => " | last verify: PASS",
            Some(false) => " | last verify: FAIL",
            None => "",
        };
        let files_tag = if cp.working_files.is_empty() {
            String::new()
        } else {
            format!(" | files: {}", cp.working_files.join(", "))
        };
        let goal_preview: String = cp.last_goal.chars().take(120).collect();
        let trail = if cp.last_goal.len() > 120 { "…" } else { "" };
        let resume_msg = format!(
            "Resumed: {} turn{}{}{} — last goal: \"{}{}\"",
            cp.turn_count,
            if cp.turn_count == 1 { "" } else { "s" },
            verify_tag,
            files_tag,
            goal_preview,
            trail,
        );
        let _ = agent_tx.send(InferenceEvent::Thought(resume_msg)).await;
    } else {
        let session_path = crate::tools::file_ops::workspace_root()
            .join(".hematite")
            .join("session.json");
        if !session_path.exists() {
            let first_run_msg = "\nWelcome to Hematite! I'm your local AI workstation assistant.\n\n\
                                 Since this is your first time here, what would you like to do?\n\
                                 - System Check: Wondering if your tools are working? Run `/health`\n\
                                 - Code: Ready to build something? Run `/architect Let's build a new feature`\n\
                                 - Setup: Need help configuring Git or the workspace? Run `/ask What should I set up first?`\n\
                                 - Help: Have a weird error? Type `/explain ` and paste it.\n\n\
                                 Just type \"hello\" to start a normal conversation!".to_string();
            let _ = agent_tx.send(InferenceEvent::Thought(first_run_msg)).await;

            // Create a minimal empty session struct so we don't show this again until they intentionally /forget
            let _ = std::fs::write(&session_path, "{\"turn_count\": 0}");
        }
    }

    let _ = agent_tx.send(InferenceEvent::Done).await;
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
        if let Err(e) = manager
            .run_turn(&input, agent_tx.clone(), config.yolo)
            .await
        {
            let _ = agent_tx.send(InferenceEvent::Error(e.to_string())).await;
            let _ = agent_tx.send(InferenceEvent::Done).await;
        }
    }
}
