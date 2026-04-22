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

const MIN_RECOMMENDED_CODING_CONTEXT: usize = 8_192;

fn provider_help_hint(base_url: &str, provider_name: &str) -> String {
    if provider_name == "LM Studio" {
        format!(
            "Check if LM Studio is running on {}. If you prefer Ollama, set `api_url` to `{}` in `.hematite/settings.json`.",
            base_url,
            crate::agent::config::DEFAULT_OLLAMA_API_URL
        )
    } else if provider_name == "Ollama" {
        format!(
            "Check if Ollama is running on {} and that a chat model is available. If you prefer LM Studio, set `api_url` to `{}`.",
            base_url,
            crate::agent::config::DEFAULT_LM_STUDIO_API_URL
        )
    } else {
        format!(
            "Check if the configured provider is running on {} and that `.hematite/settings.json` points at the right endpoint.",
            base_url
        )
    }
}

pub fn session_endpoint_url(base_url: &str) -> String {
    format!("{}/v1", base_url.trim_end_matches('/'))
}

fn preferred_coding_model_target(
    config: &crate::agent::config::HematiteConfig,
    cockpit: &CliCockpit,
) -> Option<String> {
    crate::agent::config::preferred_coding_model(config)
        .or(cockpit.think_model.clone())
        .or(cockpit.fast_model.clone())
}

fn model_name_matches(current: &str, target: &str) -> bool {
    current.trim().eq_ignore_ascii_case(target.trim())
}

fn coding_runtime_budget_warning(
    provider_name: &str,
    model_name: &str,
    context_length: usize,
    preferred_model: Option<&str>,
) -> Option<String> {
    if model_name.trim().is_empty()
        || model_name.eq_ignore_ascii_case("no model loaded")
        || context_length >= MIN_RECOMMENDED_CODING_CONTEXT
    {
        return None;
    }

    let provider_label = if provider_name.is_empty() {
        "the active provider"
    } else {
        provider_name
    };
    let mut message = format!(
        "Warning: {} loaded `{}` with only {} tokens of live context. That is too small for normal coding, scaffold, or teleport-resume work.",
        provider_label, model_name, context_length
    );
    if let Some(target) = preferred_model.filter(|target| !model_name_matches(model_name, target)) {
        message.push_str(&format!(
            " Load your preferred coding model `{}` and rerun `/runtime refresh` before heavy implementation.",
            target
        ));
    } else {
        message.push_str(
            " Load a larger-context coding model before heavy implementation and rerun `/runtime refresh`.",
        );
    }
    Some(message)
}

fn provider_model_setup_hint(provider_name: &str) -> String {
    if provider_name == "Ollama" {
        format!(
            "Pull or run a chat model in Ollama, then keep `api_url` pointed at `{}`. If you want semantic search too, save an embedding model in `/embed prefer <id>` and Hematite can load it here as well.",
            crate::agent::config::DEFAULT_OLLAMA_API_URL
        )
    } else {
        format!(
            "Load a coding model in LM Studio and keep the local server on `{}`. Optionally also load an embedding model for semantic search.",
            crate::agent::config::DEFAULT_LM_STUDIO_API_URL
        )
    }
}

async fn provider_startup_guidance(provider_name: &str, endpoint: &str, has_model: bool) -> String {
    let mut lines = vec![format!("Provider setup: {} ({})", provider_name, endpoint)];
    if has_model {
        lines.push("Status: local runtime is reachable and a coding model is loaded.".to_string());
    } else {
        lines.push("Status: provider is reachable but no coding model is loaded yet.".to_string());
        lines.push(provider_model_setup_hint(provider_name));
    }
    if let Some((alt_name, alt_url)) = detect_alternative_provider(provider_name).await {
        lines.push(format!("Reachable alternative: {} ({})", alt_name, alt_url));
    }
    lines.push(
        "Use `/provider` after startup if you want to save a different runtime for future sessions."
            .to_string(),
    );
    lines.join("\n")
}

fn runtime_context_display(model: &str, context_length: usize) -> String {
    if model.trim().is_empty() || model == "no model loaded" || context_length == 0 {
        "none".to_string()
    } else {
        context_length.to_string()
    }
}

async fn print_provider_bootstrap_help(provider_name: &str, base_url: &str) {
    let endpoint = session_endpoint_url(base_url);
    println!("Quick setup path:");
    if provider_name == "Ollama" {
        println!("  1. Install Ollama: https://ollama.com/");
        println!("  2. Start Ollama and ensure `{}` is reachable.", endpoint);
        println!("  3. Pull a chat model, for example: `ollama pull qwen3.5:latest`");
        println!(
            "  4. Restart Hematite, or switch back to LM Studio with `api_url = \"{}\"`.",
            crate::agent::config::DEFAULT_LM_STUDIO_API_URL
        );
    } else {
        println!("  1. Install LM Studio: https://lmstudio.ai/");
        println!(
            "  2. Start the local server and ensure `{}` is reachable.",
            endpoint
        );
        println!("  3. Load a coding model such as `Qwen/Qwen3.5-9B Q4_K_M`.");
        println!("  4. Restart Hematite after the model is loaded.");
    }
    if let Some((alt_name, alt_url)) = detect_alternative_provider(provider_name).await {
        println!(
            "Reachable alternative detected: {} ({}). You can point Hematite there instead.",
            alt_name, alt_url
        );
    }
}

pub async fn detect_alternative_provider(active_provider: &str) -> Option<(String, String)> {
    match active_provider {
        "LM Studio" => {
            let ollama = crate::agent::ollama::OllamaHarness::new("http://localhost:11434");
            if ollama.is_reachable().await {
                Some((
                    "Ollama".to_string(),
                    crate::agent::config::DEFAULT_OLLAMA_API_URL.to_string(),
                ))
            } else {
                None
            }
        }
        "Ollama" => {
            let lms = crate::agent::lms::LmsHarness::new();
            if lms.is_server_responding("http://localhost:1234").await {
                Some((
                    "LM Studio".to_string(),
                    crate::agent::config::DEFAULT_LM_STUDIO_API_URL.to_string(),
                ))
            } else {
                None
            }
        }
        _ => {
            let lms = crate::agent::lms::LmsHarness::new();
            if lms.is_server_responding("http://localhost:1234").await {
                return Some((
                    "LM Studio".to_string(),
                    crate::agent::config::DEFAULT_LM_STUDIO_API_URL.to_string(),
                ));
            }
            let ollama = crate::agent::ollama::OllamaHarness::new("http://localhost:11434");
            if ollama.is_reachable().await {
                return Some((
                    "Ollama".to_string(),
                    crate::agent::config::DEFAULT_OLLAMA_API_URL.to_string(),
                ));
            }
            None
        }
    }
}

pub struct RuntimeServices {
    pub engine: Arc<InferenceEngine>,
    pub gpu_state: Arc<GpuState>,
    pub git_state: Arc<GitState>,
    pub voice_manager: Arc<VoiceManager>,
    pub swarm_coordinator: Arc<agent::swarm::SwarmCoordinator>,
    pub cancel_token: Arc<std::sync::atomic::AtomicBool>,
    pub searx_session: agent::searx_lifecycle::SearxRuntimeSession,
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
    let config = crate::agent::config::load_config();

    // Auto-boot SearXNG if enabled and offline.
    let searx_session = crate::agent::searx_lifecycle::boot_searx_if_needed(&config).await;

    // settings.json api_url overrides the --url CLI flag so users don't need to retype it.
    let api_url = crate::agent::config::effective_api_url(&config, &cockpit.url);
    let mut engine_raw = InferenceEngine::new(api_url, species.to_string(), snark)?;
    let provider_name = engine_raw.provider_name().await;
    let preferred_model = preferred_coding_model_target(&config, cockpit);
    let gpu_state = ui::gpu_monitor::spawn_gpu_monitor();
    let git_state = agent::git_monitor::spawn_git_monitor();

    if !engine_raw.health_check().await {
        println!(
            "ERROR: {} not detected at {}",
            provider_name, engine_raw.base_url
        );
        println!(
            "{}",
            provider_help_hint(&engine_raw.base_url, &provider_name)
        );
        print_provider_bootstrap_help(&provider_name, &engine_raw.base_url).await;
        std::process::exit(1);
    }

    let mut detected_model = engine_raw.get_loaded_model().await.unwrap_or_default();
    let mut detected_context = engine_raw.detect_context_length().await;
    let mut auto_loaded_coding_model = false;

    if detected_model.trim().is_empty() {
        let target = preferred_model
            .as_deref()
            .or(if provider_name == "LM Studio" {
                Some("gemma-4-9b-it")
            } else {
                None
            });
        if let Some(target) = target {
            println!(
                "Notice: No model loaded in {}. Attempting to auto-load `{}`...",
                provider_name, target
            );
            if let Err(e) = engine_raw.load_model(target).await {
                println!(
                    "Warning: Auto-load failed: {}. Please load a model manually in {}.",
                    e, provider_name
                );
            } else {
                auto_loaded_coding_model = true;
                detected_model = engine_raw.get_loaded_model().await.unwrap_or_default();
                detected_context = engine_raw.detect_context_length().await;
            }
        }
    }

    let effective_model = if detected_model.trim().is_empty() {
        "no model loaded".to_string()
    } else {
        detected_model.clone()
    };
    let effective_context = if effective_model == "no model loaded" {
        0
    } else {
        detected_context
    };
    engine_raw
        .set_runtime_profile(&effective_model, effective_context)
        .await;
    if let Some(warning) = coding_runtime_budget_warning(
        &provider_name,
        &effective_model,
        effective_context,
        preferred_model.as_deref(),
    ) {
        println!("{}", warning);
    }

    if auto_loaded_coding_model {
        if let Some(embed_target) = config.embed_model.as_deref() {
            let current_embed = engine_raw.get_embedding_model().await;
            let needs_embed = current_embed
                .as_deref()
                .map(|loaded| !model_name_matches(loaded, embed_target))
                .unwrap_or(true);
            if needs_embed {
                println!(
                    "Notice: preferred embed model `{}` is not loaded. Attempting to load it for semantic search...",
                    embed_target
                );
                if let Err(e) = engine_raw.load_embedding_model(embed_target).await {
                    println!(
                        "Warning: Preferred embed model auto-load failed: {}. Load `{}` manually or save a different `/embed prefer` target if you want semantic search.",
                        e, embed_target
                    );
                }
            }
        }
    }

    let (specular_tx, specular_rx) = mpsc::channel(32);
    let watcher_guard = agent::specular::spawn_watcher(specular_tx)?;

    let (agent_tx, agent_rx) = mpsc::channel::<InferenceEvent>(100);
    let (swarm_tx, swarm_rx) = mpsc::channel(32);
    let voice_manager = Arc::new(VoiceManager::new(agent_tx.clone()));

    if let Some(worker) = config
        .fast_model
        .clone()
        .or_else(|| cockpit.fast_model.clone())
    {
        engine_raw.worker_model = Some(worker);
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
            searx_session,
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
            let provider_name = engine.provider_name().await;

            // When no coding model is loaded, back off to reduce log noise in LM Studio.
            let poll_interval = if model_id == "no model loaded" {
                tokio::time::Duration::from_secs(12)
            } else {
                tokio::time::Duration::from_secs(4)
            };

            if agent_tx
                .send(InferenceEvent::RuntimeProfile {
                    provider_name,
                    endpoint: session_endpoint_url(&engine.base_url),
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
        searx_session,
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
            provider_name: manager.engine.provider_name().await,
            endpoint: session_endpoint_url(&manager.engine.base_url),
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
        None => {
            "Embed: none loaded (load a preferred embedding model for semantic search)".to_string()
        }
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
    let provider_name = manager.engine.provider_name().await;
    let startup_endpoint = session_endpoint_url(&manager.engine.base_url);
    let terminal_name = crate::ui::terminal::detect_terminal().label();
    let greeting = format!(
        "Hematite {} Online [{}] | Model: {} | CTX: {} | GPU: {} | VRAM: {}\nEndpoint: {}\nWorkspace: {} ({})\n{}\n{}\n/ask · read-only analysis   /code · implement   /architect · plan-first   /chat · conversation\nRecovery: /undo · /new · /forget · /clear   |   /version · /about{}",
        crate::hematite_version_display(),
        terminal_name,
        display_model,
        runtime_context_display(&display_model, manager.engine.current_context_length()),
        gpu_name,
        vram,
        startup_endpoint,
        workspace_root.display(),
        workspace_mode,
        embed_status,
        voice_status,
        project_hint
    );
    let greeting = greeting.replacen(
        " | Model:",
        &format!(" | Provider: {} | Model:", provider_name),
        1,
    );
    let _ = agent_tx
        .send(InferenceEvent::MutedToken(format!("\n{}", greeting)))
        .await;
    if let Some(summary) = searx_session.startup_summary.as_deref() {
        let _ = agent_tx
            .send(InferenceEvent::Thought(summary.to_string()))
            .await;
    }
    if display_model == "no chat model loaded" {
        let guidance = provider_startup_guidance(&provider_name, &startup_endpoint, false).await;
        let _ = agent_tx.send(InferenceEvent::Thought(guidance)).await;
    }

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
        let session_path = crate::tools::file_ops::hematite_dir().join("session.json");
        if !session_path.exists() {
            let first_run_msg = "\nWelcome to Hematite! I'm your local AI workstation assistant.\n\n\
                                 Since this is your first time here, what would you like to do?\n\
                                 - System Check: Wondering if your tools are working? Run `/health`\n\
                                 - Code: Ready to build something? Run `/architect Let's build a new feature`\n\
                                 - Setup: Need help configuring Git or the workspace? Run `/ask What should I set up first?`\n\
                                 - Help: Have a weird error? Type `/explain ` and paste it.\n\n\
                                 Just type \"hello\" to start a normal conversation!".to_string();
            let _ = agent_tx.send(InferenceEvent::Thought(first_run_msg)).await;
            let provider_setup = provider_startup_guidance(
                &provider_name,
                &startup_endpoint,
                display_model != "no chat model loaded",
            )
            .await;
            let _ = agent_tx.send(InferenceEvent::Thought(provider_setup)).await;

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
    if crate::agent::inference::is_hematite_native_model(&startup_model) {
        let mode = crate::agent::config::gemma_native_mode_label(&startup_config, &startup_model);
        let status = match mode {
            "on" => "Sovereign Engine detected | Native Turn-Formatting: ON (forced)",
            "auto" => "Sovereign Engine detected | Native Turn-Formatting: ON (auto)",
            _ => "Sovereign Engine detected | Native Turn-Formatting: OFF (use /gemma-native auto|on)",
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

#[cfg(test)]
mod tests {
    use super::{coding_runtime_budget_warning, model_name_matches, preferred_coding_model_target};
    use crate::agent::config::HematiteConfig;

    #[test]
    fn preferred_coding_model_uses_config_before_cli() {
        let mut config = HematiteConfig::default();
        config.think_model = Some("qwen-config".into());
        config.fast_model = Some("fast-config".into());
        let cockpit = crate::CliCockpit {
            yolo: false,
            swarm_size: 3,
            brief: false,
            reroll: None,
            rusty: false,
            stats: false,
            no_splash: false,
            fast_model: Some("fast-cli".into()),
            think_model: Some("think-cli".into()),
            url: "http://localhost:1234/v1".into(),
            mcp_server: false,
            edge_redact: false,
            semantic_redact: false,
            semantic_url: None,
            semantic_model: None,
            pdf_extract_helper: None,
            teleported_from: None,
        };

        assert_eq!(
            preferred_coding_model_target(&config, &cockpit),
            Some("qwen-config".to_string())
        );
    }

    #[test]
    fn model_name_matches_is_case_insensitive() {
        assert!(model_name_matches("Qwen/Qwen3.5-9B", "qwen/qwen3.5-9b"));
        assert!(!model_name_matches("bonsai-8b", "qwen/qwen3.5-9b"));
    }

    #[test]
    fn coding_runtime_budget_warning_flags_small_context() {
        let warning =
            coding_runtime_budget_warning("LM Studio", "bonsai-8b", 4096, Some("qwen/qwen3.5-9b"))
                .expect("warning expected");
        assert!(warning.contains("bonsai-8b"));
        assert!(warning.contains("4096"));
        assert!(warning.contains("qwen/qwen3.5-9b"));
    }
}
