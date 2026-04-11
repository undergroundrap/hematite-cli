pub mod agent;
pub mod memory;
pub mod runtime;
pub mod telemetry;
pub mod tools;
pub mod ui;

pub const HEMATITE_VERSION: &str = env!("CARGO_PKG_VERSION");
const HEMATITE_GIT_COMMIT_SHORT_RAW: &str = env!("HEMATITE_GIT_COMMIT_SHORT");
const HEMATITE_GIT_EXACT_TAG_RAW: &str = env!("HEMATITE_GIT_EXACT_TAG");
const HEMATITE_GIT_DIRTY_RAW: &str = env!("HEMATITE_GIT_DIRTY");

pub fn hematite_git_commit_short() -> Option<&'static str> {
    (!HEMATITE_GIT_COMMIT_SHORT_RAW.is_empty()).then_some(HEMATITE_GIT_COMMIT_SHORT_RAW)
}

pub fn hematite_git_exact_tag() -> Option<&'static str> {
    (!HEMATITE_GIT_EXACT_TAG_RAW.is_empty()).then_some(HEMATITE_GIT_EXACT_TAG_RAW)
}

pub fn hematite_git_dirty() -> bool {
    HEMATITE_GIT_DIRTY_RAW.eq_ignore_ascii_case("true")
}

pub fn hematite_build_descriptor() -> String {
    let release_tag = format!("v{}", HEMATITE_VERSION);
    let exact_release = matches!(hematite_git_exact_tag(), Some(tag) if tag == release_tag);

    if exact_release && !hematite_git_dirty() {
        "release".to_string()
    } else {
        match (hematite_git_commit_short(), hematite_git_dirty()) {
            (Some(commit), true) => format!("dev+{}-dirty", commit),
            (Some(commit), false) => format!("dev+{}", commit),
            (None, true) => "dev-dirty".to_string(),
            (None, false) => "dev".to_string(),
        }
    }
}

pub fn hematite_version_display() -> String {
    format!("v{} [{}]", HEMATITE_VERSION, hematite_build_descriptor())
}

pub fn hematite_version_report() -> String {
    let mut lines = vec![
        format!("Hematite v{}", HEMATITE_VERSION),
        format!("Build: {}", hematite_build_descriptor()),
    ];
    if let Some(commit) = hematite_git_commit_short() {
        lines.push(format!("Commit: {}", commit));
    }
    lines.push(format!(
        "Built from a dirty worktree: {}",
        if hematite_git_dirty() { "yes" } else { "no" }
    ));
    lines.push(format!(
        "Exact release tag at build time: {}",
        hematite_git_exact_tag().unwrap_or("none")
    ));
    lines.join("\n")
}

// Standard imports for library users
pub use agent::config::HematiteConfig;
pub use agent::conversation::ConversationManager;
pub use agent::inference::InferenceEngine;

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Hematite CLI - Local AI Pair Programmer", long_about = None)]
pub struct CliCockpit {
    #[arg(long, help = "Bypasses the high-risk modal (Danger mode)")]
    pub yolo: bool,

    #[arg(
        long,
        default_value_t = 3,
        help = "Sets max parallel workers (default 3)"
    )]
    pub swarm_size: usize,

    #[arg(
        long,
        help = "Forces the Vigil Brief Mode for concise, high-speed output"
    )]
    pub brief: bool,

    #[arg(
        long,
        help = "Pass a custom salt to reroll the deterministic species hash"
    )]
    pub reroll: Option<String>,

    #[arg(
        long,
        help = "Rusty Mode: Enables the Rusty personality system, snark, and companion features"
    )]
    pub rusty: bool,

    #[arg(long, help = "Show Rusty stats and exit")]
    pub stats: bool,

    #[arg(
        long,
        help = "Skip the blocking splash screen and enter the TUI immediately"
    )]
    pub no_splash: bool,

    #[arg(
        long,
        help = "Optional model ID for simple tasks (overrides auto-detect)"
    )]
    pub fast_model: Option<String>,

    #[arg(
        long,
        help = "Optional model ID for complex tasks (overrides auto-detect)"
    )]
    pub think_model: Option<String>,

    #[arg(
        long,
        default_value = "http://localhost:1234/v1",
        help = "The base URL for the OpenAI-compatible API"
    )]
    pub url: String,

    #[arg(long, hide = true)]
    pub pdf_extract_helper: Option<String>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_report_contains_release_version() {
        let report = crate::hematite_version_report();
        assert!(report.contains(crate::HEMATITE_VERSION));
        assert!(report.contains("Build:"));
    }
}
