pub mod agent;
pub mod tools;
pub mod memory;
pub mod ui;
pub mod telemetry;

// Standard imports for library users
pub use agent::conversation::ConversationManager;
pub use agent::inference::InferenceEngine;
pub use agent::config::HematiteConfig;

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Hematite CLI - Local AI Pair Programmer", long_about = None)]
pub struct CliCockpit {
    #[arg(long, help = "Bypasses the high-risk modal (Danger mode)")]
    pub yolo: bool,

    #[arg(long, default_value_t = 3, help = "Sets max parallel workers (default 3)")]
    pub swarm_size: usize,

    #[arg(long, help = "Forces the Vigil Brief Mode for concise, high-speed output")]
    pub brief: bool,

    #[arg(long, help = "Pass a custom salt to reroll the deterministic species hash")]
    pub reroll: Option<String>,

    #[arg(long, help = "Rusty Mode: Enables the Rusty personality system, snark, and companion features")]
    pub rusty: bool,

    #[arg(long, help = "Show Rusty stats and exit")]
    pub stats: bool,

    #[arg(long, help = "Skip the blocking splash screen and enter the TUI immediately")]
    pub no_splash: bool,

    #[arg(long, help = "Optional model ID for simple tasks (overrides auto-detect)")]
    pub fast_model: Option<String>,

    #[arg(long, help = "Optional model ID for complex tasks (overrides auto-detect)")]
    pub think_model: Option<String>,

    #[arg(long, default_value = "http://localhost:1234/v1", help = "The base URL for the OpenAI-compatible API")]
    pub url: String,
}
