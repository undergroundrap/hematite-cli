pub mod agent;
pub mod memory;
pub mod runtime;
pub mod telemetry;
pub mod tools;
pub mod ui;

pub const HEMATITE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const HEMATITE_AUTHOR: &str = "Ocean Bennett";
pub const HEMATITE_REPOSITORY_URL: &str = "https://github.com/undergroundrap/hematite-cli";
pub const HEMATITE_SHORT_DESCRIPTION: &str =
    "Local-first AI coding harness and workstation assistant for real developer workflows.";
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

pub fn hematite_about_report() -> String {
    [
        format!("Hematite v{}", HEMATITE_VERSION),
        format!("Build: {}", hematite_build_descriptor()),
        format!("Created and maintained by {}", HEMATITE_AUTHOR),
        HEMATITE_SHORT_DESCRIPTION.to_string(),
        format!("Repo: {}", HEMATITE_REPOSITORY_URL),
    ]
    .join("\n")
}

pub fn hematite_identity_answer() -> String {
    format!(
        "Hematite was created and is maintained by {}.\n\n{}\n\nThe running assistant uses a local model runtime, but Hematite itself is the local coding harness: the TUI, tool use, file editing, workflow control, voice integration, and workstation-assistant architecture.\n\nRepo: {}",
        HEMATITE_AUTHOR, HEMATITE_SHORT_DESCRIPTION, HEMATITE_REPOSITORY_URL
    )
}

// Standard imports for library users
pub use agent::config::HematiteConfig;
pub use agent::conversation::ConversationManager;
pub use agent::inference::InferenceEngine;

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    author,
    version,
    about = "Hematite CLI - Local-first AI coding harness and workstation assistant",
    long_about = None
)]
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

    #[arg(
        long,
        help = "Run as an MCP stdio server — exposes inspect_host to Claude Desktop, OpenClaw, Cursor, and any MCP-capable agent"
    )]
    pub mcp_server: bool,

    #[arg(
        long,
        help = "Enable edge redaction in MCP server mode — strips usernames, MACs, serial numbers, hostnames, and credentials before responses leave the machine"
    )]
    pub edge_redact: bool,

    #[arg(
        long,
        help = "Enable semantic edge redaction — routes inspect_host output through the local model for privacy-safe summarization before any data leaves the machine. Requires a local OpenAI-compatible runtime running. Implies --edge-redact."
    )]
    pub semantic_redact: bool,

    #[arg(
        long,
        help = "Endpoint for --semantic-redact (default: same as --url). Point at a dedicated compact model, e.g. Bonsai 8B on port 1235, while your main model stays on 1234."
    )]
    pub semantic_url: Option<String>,

    #[arg(
        long,
        help = "Model ID for --semantic-redact (e.g. bonsai-8b). Required when multiple models are loaded in the local runtime. Omit for single-model setups."
    )]
    pub semantic_model: Option<String>,

    #[arg(
        long,
        help = "Run a headless diagnostic report and print to stdout — no TUI launched. Pipe to a file: hematite --report > health.md"
    )]
    pub report: bool,

    #[arg(
        long,
        default_value = "md",
        help = "Output format for --report: 'md' (markdown, default), 'json', or 'html' (self-contained, double-clickable)"
    )]
    pub report_format: String,

    #[arg(
        long,
        help = "Run a full staged triage — no TUI, no model required. Saves diagnosis to .hematite/reports/ and prints the path. Add --open to launch the file immediately."
    )]
    pub diagnose: bool,

    #[arg(
        long,
        help = "IT-first-look triage — runs health, security, connectivity, identity, and update checks in one pass. No model required. Saves to .hematite/reports/triage-DATE. Add --open to launch immediately, --report-format html for a double-clickable report."
    )]
    pub triage: bool,

    #[arg(
        long,
        help = "After generating a --report, --diagnose, or --triage, open the saved file in the default application (browser for HTML, editor for Markdown)"
    )]
    pub open: bool,

    #[arg(long, hide = true)]
    pub pdf_extract_helper: Option<String>,

    #[arg(long, hide = true)]
    pub teleported_from: Option<String>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_report_contains_release_version() {
        let report = crate::hematite_version_report();
        assert!(report.contains(crate::HEMATITE_VERSION));
        assert!(report.contains("Build:"));
    }

    #[test]
    fn about_report_contains_author_and_repo() {
        let report = crate::hematite_about_report();
        assert!(report.contains(crate::HEMATITE_AUTHOR));
        assert!(report.contains(crate::HEMATITE_REPOSITORY_URL));
    }
}
