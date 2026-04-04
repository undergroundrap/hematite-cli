// Hematite: Frontier Precision Active.
use hematite::{agent, ui};
use hematite::agent::inference::{InferenceEngine, InferenceEvent};
use hematite::agent::conversation::ConversationManager;
use hematite::CliCockpit;

use tokio::sync::mpsc;
use std::sync::Arc;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use clap::Parser;

async fn run_agent_task(
    mut user_input_rx: mpsc::Receiver<String>,
    agent_tx: mpsc::Sender<InferenceEvent>,
    engine: Arc<InferenceEngine>,
    yolo: bool,
    professional: bool,
    brief: bool,
    snark: u8,
    chaos: u8,
    fast_model: Option<String>,
    think_model: Option<String>,
    gpu_state: Arc<ui::gpu_monitor::GpuState>,
    git_state: Arc<agent::git_monitor::GitState>,
    swarm_coordinator: Arc<agent::swarm::SwarmCoordinator>,
    cancel_token: Arc<std::sync::atomic::AtomicBool>,
    voice_manager: Arc<ui::voice::VoiceManager>,
) {
    let mut manager = ConversationManager::new(
        engine,
        professional,
        brief,
        snark,
        chaos,
        fast_model,
        think_model,
        gpu_state.clone(),
        git_state,
        swarm_coordinator,
        voice_manager,
    );
    manager.cancel_token = cancel_token;
    
    let _ = agent_tx.send(InferenceEvent::ModelDetected(manager.engine.model.clone())).await;

    if let Err(e) = manager.initialize_mcp().await {
        let _ = agent_tx.send(InferenceEvent::Error(format!("MCP Init Failed: {}", e))).await;
    }
    let indexed = manager.initialize_vein();
    let _ = agent_tx.send(InferenceEvent::Thought(format!("The Vein: indexed {} files", indexed))).await;
    let _ = agent_tx.send(InferenceEvent::Done).await;

    let gpu_name = gpu_state.gpu_name();
    let vram = gpu_state.label();

    let greeting = format!(
        "Hematite Online | Model: {} | CTX: {} | GPU: {} | VRAM: {}",
        manager.engine.model, manager.engine.context_length, gpu_name, vram
    );

    let _ = agent_tx.send(InferenceEvent::MutedToken(format!("\n{}", greeting))).await;

    while let Some(input) = user_input_rx.recv().await {
        if let Err(e) = manager.run_turn(&input, agent_tx.clone(), yolo).await {
            let _ = agent_tx.send(InferenceEvent::Error(e.to_string())).await;
            let _ = agent_tx.send(InferenceEvent::Done).await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cockpit = CliCockpit::parse();
    let local_soul = ui::hatch::generate_soul(cockpit.reroll.clone());

    if cockpit.stats {
        println!("Species: {} | Wisdom: {} | Chaos: {}", local_soul.species, local_soul.wisdom, local_soul.chaos);
        return Ok(());
    }

    println!("Booting Hematite systems...");
    let mut engine_raw = InferenceEngine::new(cockpit.url.clone(), local_soul.species.clone(), local_soul.snark)?;
    let gpu_state = ui::gpu_monitor::spawn_gpu_monitor();
    let git_state = agent::git_monitor::spawn_git_monitor();

    if !engine_raw.health_check().await {
        println!("ERROR: LLM Provider not detected at {}", cockpit.url);
        println!("Check if LM Studio (or your local server) is running and port mapped correctly.");
        std::process::exit(1);
    }

    let model_name = engine_raw.get_loaded_model().await;
    if let Some(name) = model_name { engine_raw.model = name; }
    engine_raw.context_length = engine_raw.detect_context_length().await;

    let (specular_tx, specular_rx) = mpsc::channel(32);
    let _watcher_guard = agent::specular::spawn_watcher(specular_tx)?;

    let (agent_tx, agent_rx) = mpsc::channel::<InferenceEvent>(100);
    let (swarm_tx, swarm_rx) = mpsc::channel(32);
    let voice_manager = Arc::new(ui::voice::VoiceManager::new(agent_tx.clone()));

    if let Some(ref worker) = cockpit.fast_model {
        engine_raw.worker_model = Some(worker.clone());
    }

    let inference_singleton = Arc::new(engine_raw);
    let swarm_coordinator = Arc::new(agent::swarm::SwarmCoordinator::new(
        inference_singleton.clone(),
        gpu_state.clone(),
        cockpit.fast_model.clone(),
        !cockpit.rusty,
    ));

    let (user_input_tx, user_input_rx) = mpsc::channel::<String>(32);
    let cancel_token = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let tui_cancel_token = cancel_token.clone();

    tokio::spawn(run_agent_task(
        user_input_rx,
        agent_tx.clone(),
        inference_singleton.clone(),
        cockpit.yolo,
        !cockpit.rusty,
        cockpit.brief,
        local_soul.snark,
        local_soul.chaos,
        cockpit.fast_model.clone(),
        cockpit.think_model.clone(),
        gpu_state.clone(),
        git_state.clone(),
        swarm_coordinator.clone(),
        cancel_token,
        voice_manager.clone(),
    ));

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    std::io::stdout().execute(EnterAlternateScreen)?;
    std::io::stdout().execute(crossterm::event::EnableMouseCapture)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let _app_result = ui::tui::run_app(
        &mut terminal,
        specular_rx,
        agent_rx,
        user_input_tx,
        swarm_rx,
        swarm_tx,
        swarm_coordinator,
        Arc::new(std::sync::Mutex::new(std::time::Instant::now())),
        cockpit.clone(),
        local_soul,
        !cockpit.rusty,
        gpu_state,
        git_state,
        tui_cancel_token,
        voice_manager,
    ).await;

    disable_raw_mode()?;
    std::io::stdout().execute(crossterm::event::DisableMouseCapture)?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
