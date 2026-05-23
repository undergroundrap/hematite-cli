pub mod agent;

/// Serializes tests that call `std::env::set_current_dir` to prevent race conditions
/// in parallel test runs. Any test that mutates the process-wide cwd must hold this
/// lock for the duration of the directory change.
#[cfg(test)]
pub(crate) static TEST_CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub mod memory;
pub mod runtime;
pub mod telemetry;
pub mod tools;
pub mod ui;

pub const HEMATITE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const HEMATITE_AUTHOR: &str = "Ocean Bennett";
pub const HEMATITE_REPOSITORY_URL: &str = "https://github.com/undergroundrap/hematite-cli";
pub const HEMATITE_SHORT_DESCRIPTION: &str =
    "Local-first AI coding harness — Senior SysAdmin, Network Admin, Data Analyst, and Software Engineer in your terminal.";
const HEMATITE_GIT_COMMIT_SHORT_RAW: &str = env!("HEMATITE_GIT_COMMIT_SHORT");
const HEMATITE_GIT_EXACT_TAG_RAW: &str = env!("HEMATITE_GIT_EXACT_TAG");
const HEMATITE_GIT_DIRTY_RAW: &str = env!("HEMATITE_GIT_DIRTY");

pub fn hematite_git_commit_short() -> Option<&'static str> {
    #[allow(clippy::const_is_empty)]
    (!HEMATITE_GIT_COMMIT_SHORT_RAW.is_empty()).then_some(HEMATITE_GIT_COMMIT_SHORT_RAW)
}

pub fn hematite_git_exact_tag() -> Option<&'static str> {
    #[allow(clippy::const_is_empty)]
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

pub fn hematite_version() -> String {
    format!("v{}", HEMATITE_VERSION)
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
        "Hematite was created and is maintained by {}.\n\n{}\n\nThe running assistant uses a local model runtime, but Hematite itself is the local harness: the TUI, tool use, file editing, workflow control, host inspection, data analysis sandbox, voice integration, and workstation-assistant architecture.\n\nRepo: {}",
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
    about = "Hematite CLI - SysAdmin, Network Admin, Data Analyst, and Software Engineer in your terminal",
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

    // ── MCP Server ────────────────────────────────────────────────────────────
    #[arg(
        long,
        help_heading = "MCP Server",
        help = "Run as an MCP stdio server — exposes inspect_host to Claude Desktop, OpenClaw, Cursor, and any MCP-capable agent"
    )]
    pub mcp_server: bool,

    #[arg(
        long,
        help_heading = "MCP Server",
        help = "Enable edge redaction in MCP server mode — strips usernames, MACs, serial numbers, hostnames, and credentials before responses leave the machine"
    )]
    pub edge_redact: bool,

    #[arg(
        long,
        help_heading = "MCP Server",
        help = "Enable semantic edge redaction — routes inspect_host output through the local model for privacy-safe summarization before any data leaves the machine. Implies --edge-redact."
    )]
    pub semantic_redact: bool,

    #[arg(
        long,
        help_heading = "MCP Server",
        help = "Endpoint for --semantic-redact (default: same as --url). Point at a dedicated compact model on a different port."
    )]
    pub semantic_url: Option<String>,

    #[arg(
        long,
        help_heading = "MCP Server",
        help = "Model ID for --semantic-redact (e.g. bonsai-8b). Required when multiple models are loaded."
    )]
    pub semantic_model: Option<String>,

    // ── Headless Reports ──────────────────────────────────────────────────────
    #[arg(
        long,
        help_heading = "Headless Reports",
        help = "Run a headless diagnostic report and print to stdout — no TUI launched. Pipe to a file: hematite --report > health.md"
    )]
    pub report: bool,

    #[arg(
        long,
        help_heading = "Headless Reports",
        default_value = "md",
        help = "Output format: md (default), json, or html (self-contained, double-clickable)"
    )]
    pub report_format: String,

    #[arg(
        long,
        help_heading = "Headless Reports",
        help = "Staged triage — health_report then targeted follow-up inspections. Saves to .hematite/reports/. Add --open to launch."
    )]
    pub diagnose: bool,

    #[arg(
        long,
        help_heading = "Headless Reports",
        default_missing_value = "default",
        num_args = 0..=1,
        value_name = "PRESET",
        help = "IT-first-look triage. Optional preset: network, security, performance, storage, apps. Plain --triage runs health+security+connectivity+identity+updates."
    )]
    pub triage: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "ISSUE",
        help = "Targeted fix plan — keyword-matches your issue to the right inspect_host topics and saves a step-by-step plan. Example: hematite --fix \"PC running slow\""
    )]
    pub fix: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        help = "Open the saved report file immediately after writing (browser for HTML, editor for Markdown)"
    )]
    pub open: bool,

    #[arg(
        long,
        help_heading = "Headless Reports",
        help = "With --fix: preview which topics would be inspected without running any checks"
    )]
    pub dry_run: bool,

    #[arg(
        long,
        help_heading = "Headless Reports",
        help = "With --fix: offer to run safe auto-fixes after generating the plan (DNS flush, service restarts, clock sync, etc.)"
    )]
    pub execute: bool,

    #[arg(
        long,
        help_heading = "Headless Reports",
        help = "With --fix --execute: skip the Y/n prompt and apply auto-fixes immediately. Use in scripts and scheduled tasks."
    )]
    pub yes: bool,

    #[arg(
        long,
        help_heading = "Headless Reports",
        help = "Suppress output when the result is healthy (exit 0). Only prints when issues are found (exit 1). Use in scheduled tasks and scripts."
    )]
    pub quiet: bool,

    #[arg(
        long,
        help_heading = "Headless Reports",
        help = "Maintenance sweep — checks every safe auto-fix topic, skips what is healthy, runs what needs fixing, and verifies each fix resolved. No model required."
    )]
    pub fix_all: bool,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "LABEL",
        help = "With --fix-all: run only the named fix from the sweep. Example: hematite --fix-all --only \"Flush DNS Cache\". Use --fix-all --list to see all fix labels."
    )]
    pub only: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        help = "Copy output to clipboard after the command completes. Works with --triage, --diagnose, --fix, --fix-all, --inspect, and --query."
    )]
    pub clipboard: bool,

    #[arg(
        long,
        help_heading = "Headless Reports",
        help = "Show a native desktop notification when the command finishes. On alert pattern match with --watch, fires a notification instead of only ringing the bell. Windows 10/11 only."
    )]
    pub notify: bool,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "PATH",
        help = "Save report output to an explicit file path instead of the auto-dated .hematite/reports/ directory. Works with --triage, --diagnose, --fix, --fix-all, and --inspect."
    )]
    pub output: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        default_missing_value = "weekly",
        num_args = 0..=1,
        value_name = "CADENCE",
        help = "Register a Windows scheduled task for --triage. CADENCE: weekly (default), daily, remove, status. Combine with --fix-all to schedule the maintenance sweep instead."
    )]
    pub schedule: Option<String>,

    // ── Modelless Inspection ──────────────────────────────────────────────────
    #[arg(
        long,
        help_heading = "Modelless Inspection",
        help = "List all 128 available inspect_host topics by category. No model or TUI required."
    )]
    pub inventory: bool,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "TOPIC[,TOPIC2,...]",
        help = "Run any inspect_host topic directly to stdout. Comma-separate for multiple topics. Example: hematite --inspect wifi,latency,dns_cache"
    )]
    pub inspect: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "QUERY",
        help = "Natural-language query routed to the right inspect_host topics. Example: hematite --query \"why is my PC slow\""
    )]
    pub query: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "TOPIC[,TOPIC2,...]",
        help = "Continuously poll topic(s) every N seconds (see --watch-interval). Press Ctrl+C to stop. Example: hematite --watch resource_load,thermal"
    )]
    pub watch: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "SECONDS",
        default_value = "5",
        help = "Polling interval in seconds for --watch (default: 5)"
    )]
    pub watch_interval: u64,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "N",
        help = "With --watch: stop after N poll cycles instead of running until Ctrl+C. Example: hematite --watch resource_load --count 5"
    )]
    pub count: Option<u64>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "TOPIC[,TOPIC2,...]",
        help = "Take two snapshots separated by --diff-after seconds and show a colored diff. Example: hematite --diff processes --diff-after 60"
    )]
    pub diff: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "SECONDS",
        default_value = "30",
        help = "Seconds between snapshots for --diff (default: 30)"
    )]
    pub diff_after: u64,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "PATTERN",
        help = "With --watch: silent heartbeat when pattern is absent, bell + full output on match. Example: hematite --watch thermal --alert throttl"
    )]
    pub alert: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "PATTERN",
        help = "With --watch or --inspect: filter output to only lines containing PATTERN. Case-insensitive. Example: hematite --watch resource_load --field cpu"
    )]
    pub field: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "NAME",
        help = "With --inspect: save output to .hematite/snapshots/<name>.txt instead of printing. Example: hematite --inspect thermal --snapshot before-update"
    )]
    pub snapshot: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "NAME",
        help = "With --diff: load snapshot A from .hematite/snapshots/<name>.txt instead of running a live capture. Example: hematite --diff thermal --from before-update"
    )]
    pub from: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        help = "List saved snapshots in .hematite/snapshots/ with timestamps and sizes"
    )]
    pub snapshots: bool,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "NAME1,NAME2",
        help = "Diff two saved snapshots against each other without a live run. Example: hematite --compare before-update,after-update"
    )]
    pub compare: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "NAME",
        help = "Start a change audit session — takes a baseline snapshot of key system topics. Example: hematite --audit-start pre-patch"
    )]
    pub audit_start: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "NAME",
        help = "End a change audit session — re-runs the baseline topics and generates a diff report. Example: hematite --audit-end pre-patch"
    )]
    pub audit_end: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "TOPIC[,TOPIC2,...]",
        help = "Topics to capture for --audit-start (default: services,startup_items,ports,scheduled_tasks,shares,firewall_rules,processes,connections)"
    )]
    pub audit_topics: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "TOPIC:PATTERN",
        help = "Add a persistent alert rule. Format: TOPIC:PATTERN (e.g. thermal:throttl). Add --alert-rule-label to name it. Add --alert-rule-negate to fire when pattern is absent."
    )]
    pub alert_rule_add: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "NAME",
        help = "Label for the alert rule being added with --alert-rule-add."
    )]
    pub alert_rule_label: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        help = "With --alert-rule-add: fire when pattern is ABSENT (e.g. alert if antivirus is not running)."
    )]
    pub alert_rule_negate: bool,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        help = "List all saved alert rules."
    )]
    pub alert_rules: bool,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "ID",
        help = "Remove alert rule by ID (see --alert-rules for IDs)."
    )]
    pub alert_rule_remove: Option<u64>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        help = "Evaluate all saved alert rules against live machine data and fire toast notifications for matches. Add --schedule hourly|daily to automate."
    )]
    pub alert_rule_run: bool,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        help = "Take today's timeline snapshot (health_report, startup_items, ports, services). Skips if already captured today. Add --schedule daily to register a Task Scheduler task."
    )]
    pub timeline_capture: bool,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        help = "Show the machine state timeline — all captured daily entries with date, health grade, and summary."
    )]
    pub timeline: bool,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "DATE or DATE1,DATE2",
        help = "Diff timeline entries. Single date diffs against the previous entry; two dates diff each other. Example: hematite --timeline-diff 2025-05-10"
    )]
    pub timeline_diff: Option<String>,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        help = "Show an ASCII health grade trend chart from all captured timeline entries. Renders a bar chart, sparkline, and trajectory summary."
    )]
    pub timeline_trend: bool,

    #[arg(
        long,
        help_heading = "Modelless Inspection",
        value_name = "SYMPTOM",
        help = "Symptom-driven root-cause diagnosis — describe the problem in plain English. Runs all relevant topics and returns ranked probable causes with evidence. No model required. Example: hematite --diagnose-why \"PC is slow and freezing\""
    )]
    pub diagnose_why: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "FILE",
        help = "Statistical profiler — loads CSV/TSV/JSON/SQLite and prints a real computed column profile. No model required. Example: hematite --analyze data.csv"
    )]
    pub analyze: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "EXPR",
        help = "Evaluate a math or science expression locally — no model, no cloud. Supports arithmetic, trig, stats, physical constants, and percentages. Examples: hematite --compute \"sqrt(2)*pi\", hematite --compute \"15% of 89.99\", hematite --compute \"N_A * k_B\""
    )]
    pub compute: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "EXPR",
        help = "Unit conversion — instant, no model, no cloud. 15 categories: length, mass, time, area, volume, speed, force, pressure, energy, power, data, angle, frequency, illuminance, fuel economy, temperature. Examples: hematite --convert '5 km to miles'  '100 f to c'  '1 atm to Pa'  '60 mph to km/h'  '1 GiB to MB'  '1 kcal to J'  'list' (show all units)"
    )]
    pub convert: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "FILE",
        help = "Run a SQL query against a local data file (CSV, TSV, JSON, SQLite). The file is loaded as a table named 'data'. Pair with --sql to provide the query. Example: hematite --query-data employees.csv --sql \"SELECT department, COUNT(*) FROM data GROUP BY department\""
    )]
    pub query_data: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "QUERY",
        help = "SQL query to run against the file specified by --query-data. The table is always named 'data'. Example: --sql \"SELECT AVG(salary) FROM data WHERE department='Engineering'\""
    )]
    pub sql: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "FILE",
        help = "Generate a chart from a data file — no model, no cloud. Supports CSV, TSV, JSON, and SQLite. Uses matplotlib when available; falls back to a pure-Python SVG generator. Example: hematite --plot data.csv --plot-type histogram --plot-x age"
    )]
    pub plot: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "TYPE",
        default_value = "histogram",
        help = "Chart type for --plot: histogram, scatter, line, or bar. Default: histogram."
    )]
    pub plot_type: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "COLUMN",
        help = "X-axis column name for --plot. Auto-detected from numeric columns if omitted."
    )]
    pub plot_x: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "COLUMN",
        help = "Y-axis column name for --plot (scatter/line charts). Auto-detected if omitted."
    )]
    pub plot_y: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "TEXT",
        help = "Chart title for --plot (auto-generated if omitted)."
    )]
    pub plot_title: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "FILE",
        help = "Output SVG file path for --plot (default: <input>_plot.svg)."
    )]
    pub plot_output: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "QUERY",
        help = "Monte Carlo simulation — instant, no model. Modes: 'pi N' (estimate π), 'birthday N', 'dice 2d6 1000', 'ruin P START GOAL N', 'walk WALKS STEPS'. Example: hematite --simulate 'pi 1000000'  'dice 2d6+3 5000'  'birthday 30'"
    )]
    pub simulate: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Discrete Fourier Transform on a numeric column — finds dominant frequencies. Use --fourier-col COL, --fourier-top N, --fourier-rate Hz. Example: hematite --fourier signal.csv --fourier-col value --fourier-top 10 --fourier-rate 44100"
    )]
    pub fourier: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL",
        help = "Column to analyze with --fourier (auto-detected if omitted)."
    )]
    pub fourier_col: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "N",
        help = "Number of top frequency components to report for --fourier (default 10)."
    )]
    pub fourier_top: Option<usize>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "HZ",
        help = "Sample rate in Hz for --fourier (default 1.0 — reports normalized frequencies)."
    )]
    pub fourier_rate: Option<f64>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "k-Means clustering on numeric columns. Use --cluster-k N (default 3), --cluster-cols COL1,COL2,..., --cluster-output FILE. Example: hematite --cluster data.csv --cluster-k 4 --cluster-cols height,weight --cluster-output labeled.csv"
    )]
    pub cluster: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "N",
        help = "Number of clusters for --cluster (default 3)."
    )]
    pub cluster_k: Option<usize>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL1,COL2,...",
        help = "Feature columns for --cluster, comma-separated (default: all numeric)."
    )]
    pub cluster_cols: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Output CSV with cluster labels appended for --cluster."
    )]
    pub cluster_output: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Normalize/standardize numeric columns. Use --normalize-method minmax|zscore|robust, --normalize-cols COL1,COL2,..., --normalize-output FILE. Example: hematite --normalize data.csv --normalize-method zscore --normalize-output scaled.csv"
    )]
    pub normalize: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "METHOD",
        help = "Normalization method: minmax (default), zscore, robust."
    )]
    pub normalize_method: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL1,COL2,...",
        help = "Columns to normalize for --normalize, comma-separated (default: all numeric)."
    )]
    pub normalize_cols: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Output CSV with normalized values for --normalize."
    )]
    pub normalize_output: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Run PCA on a CSV/TSV file. Reports eigenvalues, variance explained per component, and top loadings. Example: hematite --pca data.csv --pca-components 3"
    )]
    pub pca: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "N",
        help = "Number of principal components to compute for --pca (default: 3)."
    )]
    pub pca_components: Option<usize>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL1,COL2,...",
        help = "Columns to include in --pca, comma-separated (default: all numeric)."
    )]
    pub pca_cols: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Output CSV with projected coordinates for --pca."
    )]
    pub pca_output: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Graph theory — parse an edge list and run BFS/DFS/Dijkstra/components/topo-sort. Example: hematite --graph 'shortest A D\\nA B 2\\nB D 3'"
    )]
    pub graph: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Symbolic calculus — differentiate, integrate, simplify, or evaluate. Example: hematite --symbolic 'diff x^3 + sin(x)'  or  'integrate 3*x^2'  or  'x^2+1 at x=5'"
    )]
    pub symbolic: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Financial math — NPV, IRR, loan amortization, compound interest, bond pricing, Black-Scholes. Example: hematite --finance 'loan 200000 6.5% 30'  or  'bs 100 100 5% 20% 1 call'"
    )]
    pub finance: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Propositional logic — truth table, SAT, tautology, CNF/DNF, equivalence, simplify. Example: hematite --logic 'A and (B or not C)'  or  'equiv A->B ; not A or B'"
    )]
    pub logic: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Signal processing (DSP) — DFT, convolution, cross-correlation, moving average, FIR filter design, waveform generation. No model, no cloud. Example: hematite --signal 'dft 1,0,-1,0'  or  'lowpass 0.1 31 hamming'  or  'gen sine 2 64'"
    )]
    pub signal: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Interpolation & curve fitting — linear, cubic spline, Lagrange polynomial, nearest-neighbor, with ASCII curve preview. Example: hematite --interpolate 'spline 0,0 1,1 2,4 3,9 at 1.5'  or  'linear 0,0 10,100 at 3,7'"
    )]
    pub interpolate: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Unit conversion — 14 categories, 130+ units. Length, mass, temperature, energy, digital storage, pressure, angle, and more. Example: hematite --units '100 km to miles'  or  '98.6 f to c'  or  '5 kg'  or  'list length'"
    )]
    pub units: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "ODE solver — solves ordinary differential equations using Euler, RK4, or adaptive RK45. Preset models: logistic, exponential, Lotka-Volterra, SIR. Example: hematite --ode 'dy/dt = -y  y0=1  t=5'  or  'logistic r=1 K=100 y0=5 t=10'"
    )]
    pub ode: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Numerical optimization — minimize/maximize 1D/2D functions, gradient descent, root finding. No model. Example: hematite --optimize 'min x^2-4*x+3 a=0 b=5'  or  'max sin(x) a=0 b=6.28'  or  'root x^3-2 a=0 b=2'"
    )]
    pub optimize: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "DATA",
        help = "Statistical hypothesis test — t-tests, chi-square, ANOVA, Mann-Whitney, Pearson, proportion z-test, confidence intervals. Provide comma-separated numbers or 'successes,n' for proportions. Example: hematite --hypothesis '2.1,2.8,3.2,2.5' --hypothesis-test one-t --hypothesis-mu 2.0"
    )]
    pub hypothesis: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "TYPE",
        help = "Test type for --hypothesis. Options: one-t two-t paired chi2 anova mannwhitney pearson proportion prop2 ci. Default: one-t."
    )]
    pub hypothesis_test: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "DATA",
        help = "Second group data for --hypothesis (two-t, paired, mannwhitney, pearson, prop2). Comma-separated numbers or 'successes,n'."
    )]
    pub hypothesis_group2: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "ALPHA",
        help = "Significance level for --hypothesis (default: 0.05)."
    )]
    pub hypothesis_alpha: Option<f64>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "MU",
        help = "Null hypothesis mean or proportion for --hypothesis one-t or proportion tests (default: 0.0)."
    )]
    pub hypothesis_mu: Option<f64>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Classification (k-NN or Naive Bayes) — train on labeled CSV, LOO cross-validate, predict new samples. Example: hematite --classify data.csv --classify-label species --classify-k 3"
    )]
    pub classify: Option<String>,
    #[arg(
        long,
        value_name = "COL",
        help = "Label column for --classify (default: last column)."
    )]
    pub classify_label: Option<String>,
    #[arg(
        long,
        value_name = "COL1,COL2,...",
        help = "Feature columns for --classify (default: all except label)."
    )]
    pub classify_cols: Option<String>,
    #[arg(
        long,
        value_name = "V1,V2,...",
        help = "Predict class for this comma-separated feature vector."
    )]
    pub classify_predict: Option<String>,
    #[arg(
        long,
        value_name = "N",
        help = "k neighbors for --classify k-NN (default: 3)."
    )]
    pub classify_k: Option<usize>,
    #[arg(
        long,
        value_name = "METHOD",
        help = "Algorithm for --classify: knn (default) or nb (Naive Bayes)."
    )]
    pub classify_method: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Polynomial curve fit — fit a degree-N polynomial to two CSV columns, compute R², RMSE, ASCII scatter+curve plot, and residual plot. Example: hematite --polyfit data.csv --polyfit-x age --polyfit-y salary --polyfit-degree 2"
    )]
    pub polyfit: Option<String>,
    #[arg(
        long,
        value_name = "COL",
        help = "X column for --polyfit (default: first column)."
    )]
    pub polyfit_x: Option<String>,
    #[arg(
        long,
        value_name = "COL",
        help = "Y column for --polyfit (default: last column)."
    )]
    pub polyfit_y: Option<String>,
    #[arg(
        long,
        value_name = "N",
        help = "Polynomial degree for --polyfit (default: 1 = linear, max: 10)."
    )]
    pub polyfit_degree: Option<usize>,
    #[arg(
        long,
        value_name = "X1,X2,...",
        help = "Predict y values for these x values with --polyfit."
    )]
    pub polyfit_predict: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Probability distribution calculator — instant, no model, no cloud. Distributions: normal, binomial, poisson, t (Student's), chi2, exponential, uniform, geometric. Operations: pdf/pmf, cdf, quantile, table, all. Examples: hematite --probability 'normal mean=0 sd=1 x=1.96' | hematite --probability 'binomial n=10 p=0.3 k=4' | hematite --probability 'poisson lambda=3 all' | hematite --probability 't df=9 x=2.262 cdf'"
    )]
    pub probability: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Bitwise calculator — instant, no model, no cloud. Inspect any integer in decimal/hex/binary/octal with full bit breakdown (popcount, parity, leading/trailing zeros, two's complement, byte decomposition). Operations: AND, OR, XOR, NOT, SHL, SHR, ROL, ROR. Also: IEEE 754 float bit-pattern analysis. Examples: hematite --bitwise '0xFF AND 0x3C' | hematite --bitwise 'NOT 0xAB' | hematite --bitwise '1 SHL 7' | hematite --bitwise 'ieee754 3.14159'"
    )]
    pub bitwise: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Set theory calculator — instant, no model, no cloud. Operations: union, intersection, difference, symmetric difference, power set, Cartesian product, subset/superset/disjoint checks. Elements can be numbers or strings. Examples: hematite --set '{1,2,3} union {3,4,5}' | hematite --set 'powerset {a,b,c}' | hematite --set 'cartesian {1,2} x {a,b,c}'"
    )]
    pub set: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Classical cipher encoder/decoder — instant, no model, no cloud. Ciphers: ROT13, Atbash, Caesar (with full brute-force table), Vigenère (encode/decode), Rail Fence (encode/decode), Columnar Transposition, Morse Code. Examples: hematite --cipher 'rot13 Hello World' | hematite --cipher 'caesar 13 Hello' | hematite --cipher 'vigenere encode KEY plaintext' | hematite --cipher 'morse encode Hello'"
    )]
    pub cipher: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TEXT",
        help = "Text statistics and readability analyzer — instant, no model, no cloud. Computes: character/word/sentence/paragraph counts, syllable count, average word length, Flesch Reading Ease, Flesch-Kincaid Grade Level, Gunning Fog Index, SMOG Index, Coleman-Liau Index, top-20 word frequency, letter frequency, longest words. Example: hematite --text-stats 'Paste or type any text here...'"
    )]
    pub text_stats: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "String distance metrics — instant, no model, no cloud. Computes: Levenshtein, Damerau-Levenshtein, Hamming, Jaro, Jaro-Winkler, LCS similarity, longest common substring. Separate the two strings with ' vs ' or ','. Examples: hematite --levenshtein 'kitten vs sitting' | hematite --levenshtein 'hello, helo'"
    )]
    pub levenshtein: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "NUMBER",
        help = "Number format converter — instant, no model, no cloud. Shows every representation of a number: thousands-separated decimal, scientific notation, engineering notation, SI prefix, hex/binary/octal (integers), English word form, log₁₀, ln, square root, reciprocal. Examples: hematite --number-format 1234567890 | hematite --number-format 6.022e23 | hematite --number-format 0xFF"
    )]
    pub number_format: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Sorting algorithm visualizer — instant, no model, no cloud. Shows step-by-step ASCII bar-chart visualization with comparison and swap counts. Algorithms: bubble, insertion, selection, merge, quick, heap. Pass all to compare all 6. Examples: hematite --sort-viz '5,3,8,1,9,2' | hematite --sort-viz 'bubble 5,3,8,1,9,2' | hematite --sort-viz 'merge 9,7,5,3,1'"
    )]
    pub sort_viz: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TEXT",
        help = "Checksum calculator — instant, no model, no cloud. Computes CRC-32, CRC-16 (CCITT), Adler-32, FNV-1a 32/64, DJB2, SDBM, XOR-8/16, and Sum-8/16/32 checksums for any string. Also shows hex dump, min/max/avg byte values. Examples: hematite --checksum 'Hello, World!' | hematite --checksum '123456789'"
    )]
    pub checksum: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "VALUE",
        help = "Validation toolkit — instant, no model, no cloud. Validates: Luhn algorithm (credit card numbers + card network detection), ISBN-10, ISBN-13/EAN-13, IBAN (all countries), UUID format and version. Shows check digit corrections for invalid values. Examples: hematite --validate '4532015112830366' | hematite --validate '978-0-306-40615-7' | hematite --validate 'GB82WEST12345698765432'"
    )]
    pub validate: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "NUMBERS",
        help = "Descriptive statistics — instant, no model, no cloud. Computes count, sum, min/max/range, mean, median, mode, population and sample variance/SD, CV, percentiles (P5–P99), IQR, skewness, excess kurtosis, Tukey outlier detection, and ASCII histogram. Examples: hematite --dstats '1,2,3,4,5' | hematite --dstats '10 20 30 40 50'"
    )]
    pub dstats: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Physics calculator — instant, no model, no cloud. Commands: kinematic (SUVAT 1D), projectile (2D range/height/time), force (F=ma), energy (KE+PE), momentum, work/power, wave (f↔λ, T, ω), snell (refraction), lens (thin lens/mirror), gas (ideal gas law PV=nRT), gravity (universal gravitation), pendulum (T=2π√L/g), circular (centripetal). All SI units. Examples: hematite --physics 'kinematic v0=0 a=9.8 t=3' | hematite --physics 'projectile v0=20 angle=45' | hematite --physics 'gas P=101325 n=1 T=273'"
    )]
    pub physics: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Chemistry calculator — instant, no model, no cloud. Pass a molecular formula for molar mass (H2O, C6H12O6, Ca(OH)2) or use commands: molarity (C=n/V), dilution (C1V1=C2V2), ph ([H⁺]↔pH↔pOH), buffer (Henderson-Hasselbalch), percent (mass percent of element in compound). Parses nested parentheses in formulas. Examples: hematite --chemistry 'H2O' | hematite --chemistry 'Ca(OH)2' | hematite --chemistry 'ph pH=3.5' | hematite --chemistry 'buffer pKa=4.75 A=0.1 HA=0.1'"
    )]
    pub chemistry: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Combinatorics calculator — instant, no model, no cloud. Commands: C n k (combinations), P n k (permutations), factorial n, derangement n (D(n)), catalan n (Catalan numbers), pascal n (row of Pascal's triangle), bell n (Bell numbers), stirling n k (Stirling 2nd kind), multinomial n k1,k2,... and partition n (integer partitions via Euler pentagonal recurrence). Examples: hematite --combinatorics 'C 10 3' | hematite --combinatorics 'catalan 7' | hematite --combinatorics 'bell 6'"
    )]
    pub combinatorics: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Geometry calculator — instant, no model, no cloud. Shapes: circle, triangle, rectangle, square, ellipse, polygon, sphere, cylinder, cone, box/cuboid. Also: distance/midpoint/slope, pythagorean, angle conversion (degrees/radians). Examples: hematite --geometry 'circle r=5' | hematite --geometry 'triangle a=3 b=4 c=5' | hematite --geometry 'sphere r=3' | hematite --geometry 'distance x1=0 y1=0 x2=3 y2=4'"
    )]
    pub geometry: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Electrical engineering calculator — instant, no model, no cloud. Supports SI prefixes (k/M/m/u/n/p). Commands: ohm (Ohm's law, solves for V/I/R/P), rc (time constant, cutoff freq, charge curve), rl (time constant, cutoff), lc (resonance), db (linear→dB), db2linear (dB→linear), freq (frequency↔wavelength), divider (voltage divider), energy (capacitor E=½CV², inductor E=½LI²). Examples: hematite --electrical 'ohm V=12 R=100' | hematite --electrical 'rc R=10k C=100u' | hematite --electrical 'divider Vin=12 R1=10k R2=4.7k'"
    )]
    pub electrical: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Percentage calculator — instant, no model, no cloud. Commands: 'X% of Y' (what is 15% of 350), 'X is what % of Y' (42 is what % of 280), 'change A to B' (percent change), 'X + Y%' / 'X - Y%' (increase/decrease by percent), 'markup cost pct%' (selling price + gross margin), 'discount price pct%' (final price after discount), 'tip bill pct%' (tip amount + total), 'split bill pct% N' (split N ways). Examples: hematite --percent '15% of 350' | hematite --percent 'change 80 to 95' | hematite --percent '350 + 15%' | hematite --percent 'tip 85 18%'"
    )]
    pub percent: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Complex number arithmetic — instant, no model, no cloud. Pass a complex number (3+4i) for full info (magnitude, argument, conjugate, polar form), or use operators: '(3+4i) + (1-2i)', '(3+4i) * (1-2i)', '(3+4i) / (1+2i)'. Commands: mag (|z|), arg (phase angle), conj (conjugate), polar re im (rect -> polar), rect r theta_deg (polar -> rect), pow z n (z^n via De Moivre), sqrt (including sqrt of negatives), euler theta (e^(i*theta) Euler's formula). Examples: hematite --complex '3+4i' | hematite --complex '(1+i) * (1-i)' | hematite --complex 'pow 1+i 8' | hematite --complex 'euler 3.14159'"
    )]
    pub complex: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Trigonometry calculator — instant, no model, no cloud. Pass an angle (bare number = degrees, add 'deg' or 'rad' suffix) for a full table: sin/cos/tan/cot/sec/csc + hyperbolic sinh/cosh/tanh. Inverse trig: 'asin 0.5', 'acos 0.866', 'atan 1', 'atan2 y x'. Hyperbolic: 'sinh 1.5'. Reference table: 'hyp' (all standard angles 0–360°). Examples: hematite --trig '45' | hematite --trig 'asin 0.5' | hematite --trig '0.785rad' | hematite --trig 'hyp'"
    )]
    pub trig: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Vector math — instant, no model, no cloud. 2D and 3D vectors in [x,y,z] format. Commands: dot (dot product + angle between), cross (cross product, works in 3D and 2D scalar), mag/magnitude (|v|), norm/normalize (unit vector), angle (angle between two vectors in degrees), add, sub (vector addition/subtraction), scale [v] s (scalar multiplication), proj [a] [b] (scalar + vector projection of a onto b). Examples: hematite --vector 'dot [1,2,3] [4,5,6]' | hematite --vector 'cross [1,0,0] [0,1,0]' | hematite --vector 'norm [3,4,0]'"
    )]
    pub vector: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Fraction arithmetic — instant, no model, no cloud. Operations: '1/2 + 1/3', '3/4 - 1/8', '2/3 * 3/5', '7/8 / 3/4' — results auto-simplified with decimal and mixed-number forms. Commands: simplify 12/18, lcd 1/3 1/4 1/6 (least common denominator), todec 3/7 (fraction → decimal, shows terminating/repeating), tofrac 0.625 (decimal → fraction via continued-fraction approximation), mixed 7/3 (improper → mixed number). Examples: hematite --fraction '1/2 + 1/3' | hematite --fraction 'tofrac 0.333' | hematite --fraction 'lcd 1/2 1/3 1/4'"
    )]
    pub fraction: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Date / time math — instant, no model, no cloud. Commands: date difference ('2025-01-15 to 2025-06-30' → days/weeks/months/business days), relative dates ('today + 90', '30 days from today', '90 days ago'), Unix timestamp conversion ('unix 1748000000' ↔ human date, 'toUnix 2025-06-30'), day-of-week info, and single-date profile. Examples: hematite --datetime '2025-01-15 to 2025-06-30' | hematite --datetime 'today + 90' | hematite --datetime 'unix 1748000000'"
    )]
    pub datetime: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Number theory toolkit — instant, no model, no cloud. Commands: prime (primality test), factor (prime factorization + divisor count + Euler phi), gcd a b (GCD + LCM + Bézout coefficients), lcm a b, phi n (Euler's totient), modinv a m (modular inverse via extended GCD), primes N (list all primes up to N via sieve), nextprime N. Also accepts a bare number for a full profile. Examples: hematite --nt 'prime 97' | hematite --nt 'factor 360' | hematite --nt 'modinv 3 11' | hematite --nt 'primes 100'"
    )]
    pub nt: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Health and body math — instant, no model, no cloud. Commands: bmi (w= kg or lb, h= m, cm, or 5ft10in), bmr (Mifflin-St Jeor formula — male/female, w=, h=, age=), tdee (BMR × activity level: sedentary/light/moderate/active/very — shows maintenance + cut/bulk targets), macros (protein/carb/fat breakdown for calories= and goal=maintain/muscle/cut/keto), ideal (ideal weight range for given height), water (daily water intake from body weight). Examples: hematite --health 'bmi w=70 h=1.75' | hematite --health 'tdee male w=80 h=180 age=30 activity=moderate' | hematite --health 'macros calories=2400 goal=muscle'"
    )]
    pub health: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "JSON toolkit — instant, no model, no cloud. Commands: format <json> (pretty-print), validate <json> (report valid/invalid), minify <json> (single line), keys <json> (list top-level keys), query <path> <json> (dot-path extraction, e.g. user.name or items[0].id), <json_a> --- <json_b> (diff two JSON blobs). Bare JSON is auto-formatted. Examples: hematite --json 'format {\"a\":1}' | hematite --json 'query user.name {\"user\":{\"name\":\"Alice\"}}' | hematite --json '{\"x\":1} --- {\"x\":2}'"
    )]
    pub json: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Regex toolkit — instant, no model, no cloud. Commands: test <pattern> <text> (find all matches, show capture groups), explain <pattern> (plain-English description of what the regex does), split <pattern> <text> (split text on pattern), replace <pattern> <replacement> <text> (replace all matches). Supports: . * + ? | () [] [^] {n,m} ^ $ \\d \\w \\s and their negations. Examples: hematite --regex 'test \\d+ foo 42 bar 99' | hematite --regex 'explain ^\\w+@\\w+\\.\\w+$' | hematite --regex 'replace \\s+ _ hello world'"
    )]
    pub regex: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "CSV toolkit — instant, no model, no Python. Accepts a file path or inline CSV. Commands: preview <file> (first 10 rows), head N <file> (first N rows), cols <file> (list column names), count <file> (row count), select <col1,col2> <file> (pick columns), filter <col> <op> <val> <file> (filter rows, ops: = != > < >= <=), sum <col> <file>, avg <col> <file>, groupby <col> <file> (group-by count), sort <col> [asc|desc] <file>. Examples: hematite --csv 'preview data.csv' | hematite --csv 'filter age > 30 data.csv' | hematite --csv 'groupby country data.csv'"
    )]
    pub csv: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "JWT decoder — offline, instant, no cloud (replaces jwt.io). Commands: <token> (auto-decode header + claims + expiry), decode <token>, claims <token> (payload only), header <token>. Flags expired tokens, shows iss/sub/aud/iat/exp/nbf as human-readable dates, reports signature algorithm. Never sends your token anywhere. Example: hematite --jwt 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0In0.sig'"
    )]
    pub jwt: Option<String>,

    #[arg(
        long = "url-tool",
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "URL toolkit — parse, encode, decode URLs offline. Commands: parse <url> (decompose scheme/host/port/path/query/fragment), encode <text> (percent-encode), decode <text> (percent-decode), params <url> (show query key=value pairs), build scheme=https host=x.com path=/api key=val (assemble URL). Bare URLs are auto-parsed. Examples: hematite --url-tool 'parse https://api.x.com/v1?foo=bar&page=2' | hematite --url-tool 'encode hello world' | hematite --url-tool 'decode hello%20world'"
    )]
    pub url_tool: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Cron expression toolkit — explain and compute next run times offline (replaces crontab.guru). Commands: <expr> (explain + next 5 runs), explain <expr> (plain-English description of 5-field cron), next <expr> (next 5 run times from now), next N <expr> (next N run times). Fields: minute hour day-of-month month day-of-week. Supports */step, ranges, lists, combinations. Examples: hematite --cron '* * * * *' | hematite --cron '0 9 * * 1-5' | hematite --cron 'next 10 */15 * * * *'"
    )]
    pub cron: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "IP address and subnet calculator — classify IPs and compute CIDR blocks offline. Commands: <ip> (classify: private/public/loopback/class), <ip>/<prefix> (subnet details: mask, broadcast, usable range, host count), contains <cidr> <ip> (test membership), range <ip1> <ip2> (count IPs, suggest CIDR), mask <prefix|dotted> (convert mask formats). Also handles IPv6 classification. Examples: hematite --ip 192.168.1.0/24 | hematite --ip 'contains 10.0.0.0/8 10.5.6.7' | hematite --ip 'range 192.168.1.1 192.168.1.254'"
    )]
    pub ip: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "UUID generator and inspector — offline replacement for uuid generator sites. Commands: v4 (generate one random UUID v4), v4 <N> (generate N UUIDs), nil (the all-zeros nil UUID), parse <uuid> (decode version, variant, timestamp for v1/v4), validate <uuid> (strict RFC 4122 format check). Examples: hematite --uuid v4 | hematite --uuid 'v4 5' | hematite --uuid 'parse 550e8400-e29b-41d4-a716-446655440000'"
    )]
    pub uuid: Option<String>,

    #[arg(
        long = "text-diff",
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Text diff — offline replacement for diffchecker.com. Pass two texts separated by ' vs '. Commands: '<text A> vs <text B>' (line diff), 'word: <text A> vs <text B>' (word-level diff), 'stats: <text A> vs <text B>' (summary counts only). Examples: hematite --text-diff 'hello world vs hello there' | hematite --text-diff 'word: foo bar baz vs foo qux baz'"
    )]
    pub text_diff: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Semantic version toolkit — offline replacement for semver.npmjs.com. Commands: parse <ver> (decode major/minor/patch/pre-release/build), compare <v1> <v2> (ordering), satisfies <version> <range> (npm/cargo range check: ^, ~, >=, <=, =), sort <v1> <v2> ... (order a list), bump <ver> major|minor|patch (next version), validate <ver> (strict semver check). Examples: hematite --semver 'parse 1.2.3-alpha.1' | hematite --semver 'satisfies 1.5.0 ^1.2.3' | hematite --semver 'bump 1.2.3 minor'"
    )]
    pub semver: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Unix timestamp converter — offline replacement for epoch.now.sh and unixtimestamp.com. Commands: (bare) = current timestamp; <unix-secs> = decode; <unix-ms> = auto-detect milliseconds; now + <N>d/h/m/s = relative future; now - <N>d/h/m/s = relative past; YYYY-MM-DD [HH:MM:SS] = date to Unix. Shows Unix (s/ms), ISO 8601, RFC 2822, human-readable UTC. Examples: hematite --timestamp | hematite --timestamp 1716220800 | hematite --timestamp '2024-05-20' | hematite --timestamp 'now + 7d'"
    )]
    pub timestamp: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "YAML validator, formatter, and key inspector — offline replacement for yamllint.com. Commands: (bare) / validate <yaml|file> (check validity + type + line count), format <yaml|file> (pretty-print), keys <yaml|file> (list top-level keys), get <yaml|file> <key.path> (fetch a value by dotted path). Examples: hematite --yaml 'key: value' | hematite --yaml 'validate config.yml' | hematite --yaml 'keys docker-compose.yml' | hematite --yaml 'get config.yml server.port'"
    )]
    pub yaml: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "ASCII table renderer — offline replacement for tableconvert.com. Auto-detects JSON arrays and CSV. Commands: <csv> (render CSV as ASCII table), <json-array> (render JSON array), markdown <input> (output as Markdown table), csv <json-array> (convert JSON to CSV). Examples: hematite --table 'name,age\\nAlice,30\\nBob,25' | hematite --table '[{\"name\":\"Alice\",\"age\":30}]' | hematite --table 'markdown name,score\\nA,95'"
    )]
    pub table: Option<String>,

    #[arg(
        long = "sql-fmt",
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "SQL formatter and keyword uppercaser — offline replacement for sqlbeautify.com / poorsql.com. Uppercases keywords, inserts newlines before clause starters (SELECT, FROM, WHERE, JOIN, GROUP BY, etc.), and indents continuation lines. Commands: <sql> (format), minify <sql> (collapse to one line), keywords (list all known keywords). Examples: hematite --sql-fmt 'select id,name from users where active=1' | hematite --sql-fmt 'minify SELECT id FROM t WHERE x=1'"
    )]
    pub sql_fmt: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "HTTP status code reference — offline replacement for httpstatuses.com. Commands: <code> (look up a code), <keyword> (search by phrase), list, list 4xx / list 5xx (range filter). Examples: hematite --http 404 | hematite --http 'server error' | hematite --http 'list 2xx'"
    )]
    pub http: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "MIME type reference — offline replacement for mime.io / mimeapplication.com. Bidirectional: extension→type and type→extensions. Commands: <.ext> or <ext>, <type/subtype>, <keyword>, list, list <category>. Examples: hematite --mime .json | hematite --mime 'application/pdf' | hematite --mime image | hematite --mime list"
    )]
    pub mime: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "XML toolkit — offline replacement for xmlvalidation.com / xmlformatter.com. Commands: validate <xml>, format <xml> (pretty-print), minify <xml>, get <tag> <xml> (extract element content). Examples: hematite --xml 'validate <root/>' | hematite --xml 'format <a><b/></a>' | hematite --xml 'get name <person><name>Alice</name></person>'"
    )]
    pub xml: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "TOML validator and toolkit — offline replacement for toml-lint.com. Commands: validate <toml|file>, keys <toml|file> (list key paths), get <key> <toml|file>, fmt <toml|file> (normalize spacing). Examples: hematite --toml 'validate [server]\\nport=8080' | hematite --toml 'keys Cargo.toml'"
    )]
    pub toml: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Network / CIDR calculator — offline replacement for subnet-calculator.com. Commands: <ip>/<prefix> (CIDR breakdown), <ip> (IP classification), contains <ip> <cidr>, split <cidr> /<prefix>. Examples: hematite --net 192.168.1.0/24 | hematite --net 10.0.0.5 | hematite --net 'split 10.0.0.0/16 /24'"
    )]
    pub net: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "ASCII table reference — offline replacement for asciitable.com. Commands: <decimal>, 0x<hex>, <char>, list, list printable, list control, <keyword>. Examples: hematite --ascii 65 | hematite --ascii 0x1B | hematite --ascii A | hematite --ascii newline"
    )]
    pub ascii: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Keyboard shortcut cheatsheets — instant offline reference for vim / vscode / tmux / git / bash / windows. Commands: <tool>, <tool> <filter>. Examples: hematite --kbd vim | hematite --kbd vscode | hematite --kbd 'vim search' | hematite --kbd 'git rebase'"
    )]
    pub kbd: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Date/duration arithmetic — offline replacement for timeanddate.com. Commands: today, age YYYY-MM-DD, YYYY-MM-DD to YYYY-MM-DD, 30 days ago, 90 days from now, N weeks from YYYY-MM-DD, 3600 in seconds, bare Unix timestamp decode. Date formats: YYYY-MM-DD | YYYY/MM/DD | DD/MM/YYYY | MM/DD/YYYY. Examples: hematite --duration today | hematite --duration 'age 1990-06-15' | hematite --duration '2024-01-01 to 2025-01-01' | hematite --duration '30 days ago' | hematite --duration 1715000000"
    )]
    pub duration: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Unicode sparkline and ASCII bar chart from comma-separated numbers — offline replacement for online chart tools. Commands: <values> = sparkline (▁▂▃▄▅▆▇█); bar <values> = horizontal bar chart; stats <values> = statistics summary + sparkline; normalize <values> = normalize to 0-1 first. Examples: hematite --spark '1,4,2,8,5,7' | hematite --spark 'bar 10,20,30,25,15' | hematite --spark 'stats 3,7,2,9,4,6'"
    )]
    pub spark: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Template variable substitution with {{key}} placeholders — offline replacement for online template engines. Separator between template and variables is |||. Multiple key=value pairs separated by commas or semicolons. Examples: hematite --template 'Hello {{name}}! ||| name=Alice' | hematite --template 'Dear {{first}} {{last}}, order #{{id}}. ||| first=Jane, last=Doe, id=42'"
    )]
    pub template: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "String escaping — offline replacement for online escape tools. Commands: json <text> = escape for JSON string; shell <text> = POSIX single-quote escaping; regex <text> = escape metacharacters; sql <text> = escape LIKE wildcards; unescape <text> = JSON unescape; bare input = all formats at once. Examples: hematite --escape 'json hello \"world\"' | hematite --escape 'regex (foo|bar).baz' | hematite --escape 'shell it'\"'\"'s fine'"
    )]
    pub escape: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Well-known port directory — offline replacement for port number Googling. Query by number (443), service name (postgres), or keyword (database). 'list' shows all ~80 entries. Examples: hematite --port 443 | hematite --port postgres | hematite --port redis | hematite --port list"
    )]
    pub port: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Unicode character inspector — offline replacement for unicode-table.com. Shows codepoint (U+XXXX), UTF-8 bytes, block, category, and HTML entity for every character. Supports U+XXXX direct lookup. Examples: hematite --chars 'Hello' | hematite --chars U+2014 | hematite --chars 'cafe\u{301}'"
    )]
    pub chars: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Timezone converter — offline replacement for timeanddate.com timezone tool. Commands: 'now in <zone>', '<time> <from> in <to>', single zone lookup, 'list'. Zones: UTC, EST, PST, JST, IST, CET, city names (tokyo, london, nyc, etc.). Examples: hematite --tz 'now in tokyo' | hematite --tz '3pm EST in JST' | hematite --tz '14:30 UTC in sydney' | hematite --tz list"
    )]
    pub tz: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "HTTP header reference — offline replacement for MDN header lookups. Shortcuts: cors (full CORS header set), security (security headers). Query by exact name, keyword, or category. Examples: hematite --headers cache-control | hematite --headers cors | hematite --headers security | hematite --headers etag | hematite --headers list"
    )]
    pub headers: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = ".gitignore template generator — offline replacement for gitignore.io. Generates .gitignore content for 20+ stacks. Combine multiple: 'node macos vscode'. Commands: list (all templates), or any stack name/alias. Examples: hematite --gitignore rust | hematite --gitignore 'python macos vscode' | hematite --gitignore node > .gitignore"
    )]
    pub gitignore: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Software license reference — offline replacement for choosealicense.com. Full text for MIT, BSD-2, BSD-3, ISC, Unlicense, WTFPL; summaries for Apache-2.0, GPL-2.0/3.0, LGPL, AGPL, MPL-2.0, CC0. Commands: list (comparison table) or a license name. Examples: hematite --license mit | hematite --license list | hematite --license apache | hematite --license gpl3"
    )]
    pub license: Option<String>,

    #[arg(
        long = "json-path",
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "JSON field extractor — offline jq replacement for simple lookups. Format: '<path> ||| <json>' or bare JSON to pretty-print. Path syntax: .field .arr[0] .a.b.c .users[0].name; commands: keys, type, length, pretty. Examples: hematite --json-path '.name ||| {\"name\":\"Alice\"}' | hematite --json-path 'keys ||| {\"a\":1,\"b\":2}'"
    )]
    pub json_path: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Markdown / CommonMark syntax reference — offline replacement for markdownguide.org. Sections: headings, emphasis, lists, links, images, code, tables, blockquotes, escape, footnotes, frontmatter, html, mermaid. Bare query shows all. Examples: hematite --markdown tables | hematite --markdown links | hematite --markdown mermaid"
    )]
    pub markdown: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Regex pattern library — offline replacement for regex101.com lookups. Named patterns: email, url, uuid, ipv4, ipv6, phone-us, date-iso, date-us, time, zip-us, postal-ca, credit-card, ssn, hex-color, slug, semver, jwt, mac-address, filename, path-unix, path-win, html-tag, whitespace, digits, word, hashtag, mention. Commands: syntax = regex syntax cheatsheet; all = every pattern; <name> = single pattern + grep/rg snippets. Examples: hematite --regex-ref email | hematite --regex-ref uuid | hematite --regex-ref syntax"
    )]
    pub regex_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Full 7-bit ASCII table — offline replacement for asciitable.com. Lookup by decimal (65), hex (0x41 or 41h), or character (A). Filter sections: control, printable, upper, lower, digits, letters, punct. Show all: hematite --ascii-table all. Examples: hematite --ascii-table 65 | hematite --ascii-table A | hematite --ascii-table control | hematite --ascii-table punct"
    )]
    pub ascii_table: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "TLS/SSL reference — offline replacement for SSL cheatsheets. Topics: handshake (TLS 1.2/1.3 flow), ciphers (recommended suites, deprecated suites), certificates (PEM/DER/PKCS formats, chain), openssl (command snippets), nginx (HTTPS server config), grades (Mozilla config levels, SSL Labs grading), hsts (security headers), errors (common TLS errors and fixes). Commands: all = print everything. Examples: hematite --ssl handshake | hematite --ssl openssl | hematite --ssl errors | hematite --ssl all"
    )]
    pub ssl: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TYPE",
        help = "Unique ID generator — offline replacement for uuidgenerator.net and similar tools. Types: uuid/uuid4, ulid (sortable), nanoid (URL-safe 21 chars), nanoid <N> (custom length), hex8, hex16, hex32, cuid2, xid (K-sortable). Generate all at once: all. Examples: hematite --id uuid | hematite --id ulid | hematite --id nanoid | hematite --id nanoid 32 | hematite --id all"
    )]
    pub id: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "HTTP status code reference — offline replacement for httpstatuses.com. Lookup by code (404), by name (not-found), or by category (4xx, 5xx, client, server, success, redirect). Commands: all = full list. Examples: hematite --http-status 404 | hematite --http-status 4xx | hematite --http-status service-unavailable"
    )]
    pub http_status: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Git command reference — offline replacement for daily git-scm.com lookups. Topics: init, config, stage, commit, branch, checkout, merge, rebase, remote, stash, log, reset, tag, diff, clean, bisect, worktree, aliases. Commands: all = everything. Examples: hematite --git-ref rebase | hematite --git-ref stash | hematite --git-ref reset"
    )]
    pub git_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "CSS named color reference — offline replacement for MDN color lookups. Lookup by name (cornflowerblue), by hex (#6495ED), or by category (red, green, blue, purple, pink, brown, gray, white). Shows hex, rgb(), and hsl() for every result. Examples: hematite --color-names tomato | hematite --color-names \"#FF6347\" | hematite --color-names blue"
    )]
    pub color_names: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Docker command reference — offline replacement for Docker docs daily lookups. Topics: build, run, container, exec, volumes, network, compose, registry, prune, dockerfile. Commands: all = everything. Examples: hematite --docker-ref run | hematite --docker-ref compose | hematite --docker-ref dockerfile"
    )]
    pub docker_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "SQL quick reference — offline ANSI/PostgreSQL/MySQL/SQLite cheatsheet. Topics: select, where, joins, aggregate, window, subquery, dml, ddl, explain, transactions, json. Commands: all = everything. Examples: hematite --sql-ref joins | hematite --sql-ref window | hematite --sql-ref all"
    )]
    pub sql_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Vim/Neovim cheatsheet — offline reference for modes, motions, editing, text-objects, search/replace, files/splits, macros, marks, and config. Commands: all = everything. Examples: hematite --vim motion | hematite --vim text-objects | hematite --vim macros"
    )]
    pub vim: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "curl command reference — offline cheatsheet covering basics, HTTP methods, headers, auth, TLS, file upload, proxy, cookies, and output formatting. Commands: all = everything. Examples: hematite --curl auth | hematite --curl upload | hematite --curl output"
    )]
    pub curl: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "jq filter reference — offline cheatsheet covering basics/flags, field access, transforms, strings, conditionals, reduce/add, and ready-to-use recipes. Commands: all = everything. Examples: hematite --jq transform | hematite --jq recipes | hematite --jq strings"
    )]
    pub jq: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "grep/ripgrep cheatsheet — offline reference covering flags, regex patterns, context, file filtering, ripgrep-specific features, and ready-to-use recipes. Topics: basics, patterns, context, files, ripgrep, advanced, one-liners. Commands: all = everything. Examples: hematite --grep patterns | hematite --grep ripgrep | hematite --grep one-liners"
    )]
    pub grep: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "sed stream editor reference — offline cheatsheet for substitution, addresses, delete, insert, transform, multiline, and advanced scripting. Topics: basics, substitute, address, delete, insert, transform, multiline, advanced. Commands: all = everything. Examples: hematite --sed substitute | hematite --sed multiline | hematite --sed advanced"
    )]
    pub sed: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "awk reference — offline cheatsheet for patterns, built-in variables, associative arrays, functions, I/O, and one-liners. Topics: basics, patterns, variables, arrays, functions, io, one-liners. Commands: all = everything. Examples: hematite --awk variables | hematite --awk arrays | hematite --awk one-liners"
    )]
    pub awk: Option<String>,

    #[arg(
        long = "ssh-ref",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "SSH client and server reference — offline cheatsheet for connecting, key management, ~/.ssh/config, tunnels/forwarding, scp/rsync, agent, options, and hardening. Topics: connect, keys, config, tunnel, scp-rsync, agent, options, hardening. Commands: all = everything. Examples: hematite --ssh-ref tunnel | hematite --ssh-ref hardening | hematite --ssh-ref config"
    )]
    pub ssh_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "tar command reference — offline cheatsheet for creating, extracting, listing, compression options (gzip/bzip2/xz/zstd), and advanced usage (incremental, pipes, split). Topics: basics, create, extract, compress, advanced. Commands: all = everything. Examples: hematite --tar create | hematite --tar compress | hematite --tar advanced"
    )]
    pub tar: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "find command reference — offline cheatsheet for name/path/type/time/size/permission filtering, -exec actions, -prune exclusions, and one-liner recipes. Topics: basics, by-type, by-time, by-size, by-perm, actions, prune, one-liners. Commands: all = everything. Examples: hematite --find actions | hematite --find by-time | hematite --find one-liners"
    )]
    pub find: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "systemd reference — offline cheatsheet for systemctl, journalctl, service management, timers, unit files, boot analysis, and targets. Topics: service, status, logs, analyze, timers, units, targets. Commands: all = everything. Examples: hematite --systemd logs | hematite --systemd timers | hematite --systemd units"
    )]
    pub systemd: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "GNU Make / Makefile reference — offline cheatsheet for variables, rules, pattern rules, functions, conditionals, and automatic variables. Topics: basics, variables, rules, patterns, functions, conditionals, special. Commands: all = everything. Examples: hematite --make variables | hematite --make patterns | hematite --make special"
    )]
    pub make: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "chmod / chown / umask reference — offline cheatsheet. Topics: basics, symbolic, special, chown, umask. Commands: all = everything. Examples: hematite --chmod basics | hematite --chmod symbolic | hematite --chmod umask"
    )]
    pub chmod: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "OpenSSL command reference — offline cheatsheet. Topics: keygen, certs, csr, inspect, convert, encrypt, digest, connect. Commands: all = everything. Examples: hematite --openssl keygen | hematite --openssl certs | hematite --openssl digest"
    )]
    pub openssl: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "nginx configuration reference — offline cheatsheet. Topics: commands, server-block, location, proxy, ssl-tls, static, rewrites. Commands: all = everything. Examples: hematite --nginx proxy | hematite --nginx ssl-tls | hematite --nginx location"
    )]
    pub nginx: Option<String>,

    #[arg(
        long = "bash-ref",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Bash scripting reference — offline cheatsheet. Topics: variables, arrays, conditionals, loops, functions, io, advanced. Commands: all = everything. Examples: hematite --bash-ref variables | hematite --bash-ref loops | hematite --bash-ref advanced"
    )]
    pub bash_ref: Option<String>,

    #[arg(
        long = "python-ref",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Python 3 reference — offline cheatsheet. Topics: builtins, strings, collections, comprehensions, functions, classes, async. Commands: all = everything. Examples: hematite --python-ref strings | hematite --python-ref async | hematite --python-ref comprehensions"
    )]
    pub python_ref: Option<String>,

    #[arg(
        long = "rust-ref",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Rust language reference — offline cheatsheet. Topics: ownership, types, traits, iterators, error, concurrency. Commands: all = everything. Examples: hematite --rust-ref ownership | hematite --rust-ref traits | hematite --rust-ref error"
    )]
    pub rust_ref: Option<String>,

    #[arg(
        long = "go-ref",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Go language reference — offline cheatsheet. Topics: basics, functions, slices, maps, interfaces, goroutines, errors. Commands: all = everything. Examples: hematite --go-ref goroutines | hematite --go-ref interfaces | hematite --go-ref slices"
    )]
    pub go_ref: Option<String>,

    #[arg(
        long = "js-ref",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "JavaScript (ES2022+) reference — offline cheatsheet. Topics: types, functions, arrays, objects, promises, modules, modern. Commands: all = everything. Examples: hematite --js-ref promises | hematite --js-ref modules | hematite --js-ref modern"
    )]
    pub js_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "kubectl / Kubernetes reference — offline cheatsheet. Topics: basics, pods, deployments, services, config, yaml, advanced. Commands: all = everything. Examples: hematite --kubectl pods | hematite --kubectl deployments | hematite --kubectl yaml"
    )]
    pub kubectl: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "tmux reference — offline cheatsheet. Topics: sessions, windows, panes, copy-mode, config, scripting. Commands: all = everything. Examples: hematite --tmux panes | hematite --tmux copy-mode | hematite --tmux config"
    )]
    pub tmux: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "PostgreSQL reference — offline cheatsheet. Topics: psql, tables, queries, admin, json, performance. Commands: all = everything. Examples: hematite --postgres queries | hematite --postgres json | hematite --postgres performance"
    )]
    pub postgres: Option<String>,

    #[arg(
        long = "ts-ref",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "TypeScript reference — offline cheatsheet. Topics: types, interfaces, generics, functions, utility, narrowing, config. Commands: all = everything. Examples: hematite --ts-ref generics | hematite --ts-ref utility | hematite --ts-ref narrowing"
    )]
    pub ts_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Ansible reference — offline cheatsheet. Topics: inventory, playbooks, modules, vars, roles, vault, cli. Commands: all = everything. Examples: hematite --ansible playbooks | hematite --ansible modules | hematite --ansible vault"
    )]
    pub ansible: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Terraform / OpenTofu reference — offline cheatsheet. Topics: workflow, hcl, variables, state, modules, expressions. Commands: all = everything. Examples: hematite --terraform workflow | hematite --terraform state | hematite --terraform expressions"
    )]
    pub terraform: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "npm / yarn / pnpm reference — offline cheatsheet. Topics: install, scripts, packages, config, workspaces. Commands: all = everything. Examples: hematite --npm install | hematite --npm scripts | hematite --npm workspaces"
    )]
    pub npm: Option<String>,

    #[arg(
        long = "git-adv",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Advanced Git reference — offline cheatsheet. Topics: rebase, stash, bisect, worktree, reflog, hooks. Commands: all = everything. Examples: hematite --git-adv rebase | hematite --git-adv stash | hematite --git-adv reflog"
    )]
    pub git_adv: Option<String>,

    #[arg(
        long = "docker-adv",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Advanced Docker reference — offline cheatsheet. Topics: dockerfile, networks, volumes, compose, buildkit, operations. Commands: all = everything. Examples: hematite --docker-adv dockerfile | hematite --docker-adv compose | hematite --docker-adv buildkit"
    )]
    pub docker_adv: Option<String>,

    #[arg(
        long = "systemd-adv",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Advanced systemd reference — offline cheatsheet. Topics: units, service, journal, timers, dropin, ctl. Commands: all = everything. Examples: hematite --systemd-adv service | hematite --systemd-adv timers | hematite --systemd-adv journal"
    )]
    pub systemd_adv: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "GNU Make / Makefile reference — offline cheatsheet. Topics: basics, variables, patterns, functions, recipes. Commands: all = everything. Examples: hematite --makefile variables | hematite --makefile patterns | hematite --makefile functions"
    )]
    pub makefile: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Jinja2 template reference — offline cheatsheet. Topics: syntax, control, filters, macros, inheritance. Commands: all = everything. Examples: hematite --jinja filters | hematite --jinja control | hematite --jinja inheritance"
    )]
    pub jinja: Option<String>,

    #[arg(
        long = "http-adv",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Advanced HTTP reference — offline cheatsheet. Topics: status, caching, cors, auth, headers, performance. Commands: all = everything. Examples: hematite --http-adv caching | hematite --http-adv cors | hematite --http-adv auth"
    )]
    pub http_adv: Option<String>,

    #[arg(
        long = "linux-adv",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Advanced Linux power-user reference — offline cheatsheet. Topics: processes, tracing, namespaces, sysctl, filesystem, networking. Commands: all = everything. Examples: hematite --linux-adv tracing | hematite --linux-adv sysctl | hematite --linux-adv namespaces"
    )]
    pub linux_adv: Option<String>,

    #[arg(
        long = "security-ref",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Security reference — OWASP, injection, TLS, secrets, JWT, scanning. Topics: owasp, injection, tls, secrets, jwt, scanning. Commands: all = everything. Examples: hematite --security-ref owasp | hematite --security-ref tls | hematite --security-ref jwt"
    )]
    pub security_ref: Option<String>,

    #[arg(
        long = "cloud-ref",
        help_heading = "Math & Science",
        value_name = "TOPIC",
        help = "Cloud CLI reference — AWS, GCP, Azure, IAM, managed K8s. Topics: aws, gcp, azure, iam, k8s-cloud. Commands: all = everything. Examples: hematite --cloud-ref aws | hematite --cloud-ref iam | hematite --cloud-ref gcp"
    )]
    pub cloud_ref: Option<String>,

    #[arg(
        long = "regex-adv",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Advanced regex reference — lookaround, groups, flavors, quantifiers, charclass, common patterns. Topics: lookaround, groups, flavors, quantifiers, charclass, patterns. Commands: all = everything. Examples: hematite --regex-adv lookahead | hematite --regex-adv groups | hematite --regex-adv patterns"
    )]
    pub regex_adv: Option<String>,

    #[arg(
        long = "sql-adv",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Advanced SQL reference — window functions, CTEs, indexes, EXPLAIN, transactions, JSONB. Topics: window, cte, indexes, explain, transactions, json. Commands: all = everything. Examples: hematite --sql-adv window | hematite --sql-adv cte | hematite --sql-adv indexes"
    )]
    pub sql_adv: Option<String>,

    #[arg(
        long = "vim-adv",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Advanced Vim / Neovim reference — registers, macros, ex commands, motions, folds, config. Topics: registers, macros, excommands, motions, folds, config. Commands: all = everything. Examples: hematite --vim-adv registers | hematite --vim-adv macros | hematite --vim-adv motions"
    )]
    pub vim_adv: Option<String>,

    #[arg(
        long = "python-data",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Python data toolkit — pandas, numpy, matplotlib, stdlib, data wrangling. Topics: pandas, numpy, stdlib, plotting, wrangling. Commands: all = everything. Examples: hematite --python-data pandas | hematite --python-data numpy | hematite --python-data wrangling"
    )]
    pub python_data: Option<String>,

    #[arg(
        long = "css-ref",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "CSS reference — selectors, flexbox, grid, animations, custom properties, responsive. Topics: selectors, flexbox, grid, animations, variables, responsive. Commands: all = everything. Examples: hematite --css-ref flexbox | hematite --css-ref grid | hematite --css-ref selectors"
    )]
    pub css_ref: Option<String>,

    #[arg(
        long = "rust-adv",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Advanced Rust reference — lifetimes, traits/generics, async/await, iterators, macros, error handling. Topics: lifetimes, traits, async, iterators, macros, errors. Commands: all = everything. Examples: hematite --rust-adv lifetimes | hematite --rust-adv async | hematite --rust-adv errors"
    )]
    pub rust_adv: Option<String>,

    #[arg(
        long = "algo-ref",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Algorithm and data structure reference — Big O cheatsheet, sorting, trees, graphs, DP, problem patterns. Topics: complexity, sorting, trees, graphs, dp, patterns. Commands: all = everything. Examples: hematite --algo-ref complexity | hematite --algo-ref dp | hematite --algo-ref patterns"
    )]
    pub algo_ref: Option<String>,

    #[arg(
        long = "oop-ref",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "OOP design patterns reference — GoF creational/structural/behavioral, SOLID, composition vs inheritance. Topics: creational, structural, behavioral, solid, composition, antipatterns. Commands: all = everything. Examples: hematite --oop-ref solid | hematite --oop-ref creational | hematite --oop-ref behavioral"
    )]
    pub oop_ref: Option<String>,

    #[arg(
        long = "typescript-adv",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "TypeScript advanced reference — generics, utility types, conditional/mapped types, decorators, modules. Topics: generics, utility, conditional, mapped, decorators, modules. Commands: all = everything. Examples: hematite --typescript-adv utility | hematite --typescript-adv conditional | hematite --typescript-adv all"
    )]
    pub typescript_adv: Option<String>,

    #[arg(
        long = "bash-adv",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Bash advanced reference — arrays, string manipulation, arithmetic, substitution, traps, patterns. Topics: arrays, strings, arithmetic, substitution, traps, patterns. Commands: all = everything. Examples: hematite --bash-adv arrays | hematite --bash-adv strings | hematite --bash-adv traps"
    )]
    pub bash_adv: Option<String>,

    #[arg(
        long = "network-ref",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Networking reference — OSI model, TCP/IP, subnetting, DNS, TLS, protocols. Topics: osi, tcp-ip, subnetting, dns, tls, protocols. Commands: all = everything. Examples: hematite --network-ref dns | hematite --network-ref tls | hematite --network-ref subnetting"
    )]
    pub network_ref: Option<String>,

    #[arg(
        long = "unicode-ref",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Unicode reference — encoding, code points, normalization, escape sequences, categories, BOM. Topics: encoding, codepoints, normalization, escapes, categories, bom. Commands: all = everything. Examples: hematite --unicode-ref encoding | hematite --unicode-ref normalization | hematite --unicode-ref bom"
    )]
    pub unicode_ref: Option<String>,

    #[arg(
        long = "regex-tester",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Regex reference — anchors, groups, quantifiers, character classes, flags, common patterns. Topics: anchors, groups, quantifiers, charclass, flags, patterns. Commands: all = everything. Examples: hematite --regex-tester anchors | hematite --regex-tester patterns | hematite --regex-tester groups"
    )]
    pub regex_tester: Option<String>,

    #[arg(
        long = "http-headers",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "HTTP headers reference — request, response, security, CORS, auth, caching. Topics: request, response, security, cors, auth, cache. Commands: all = everything. Examples: hematite --http-headers security | hematite --http-headers cors | hematite --http-headers cache"
    )]
    pub http_headers: Option<String>,

    #[arg(
        long = "crypto-ref",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Cryptography reference — symmetric, asymmetric, hashing, PKI, vulnerabilities, protocols. Topics: symmetric, asymmetric, hashing, pki, vulnerabilities, protocols. Commands: all = everything. Examples: hematite --crypto-ref hashing | hematite --crypto-ref asymmetric | hematite --crypto-ref pki"
    )]
    pub crypto_ref: Option<String>,

    #[arg(
        long = "devops-ref",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "DevOps reference — CI/CD, containers/k8s, IaC, monitoring, SRE, DevSecOps. Topics: cicd, containers, iac, monitoring, sre, security. Commands: all = everything. Examples: hematite --devops-ref cicd | hematite --devops-ref monitoring | hematite --devops-ref sre"
    )]
    pub devops_ref: Option<String>,

    #[arg(
        long = "linux-sys",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Linux SysAdmin reference — systemd/journalctl, processes/signals, kernel/sysctl, filesystem, network, security. Topics: systemctl, processes, kernel, filesystem, network, security. Commands: all = everything. Examples: hematite --linux-sys systemctl | hematite --linux-sys kernel | hematite --linux-sys security"
    )]
    pub linux_sys: Option<String>,

    #[arg(
        long = "api-design",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "API design reference — REST, OpenAPI/Swagger, versioning, error formats, GraphQL, rate limiting. Topics: rest, openapi, versioning, errors, graphql, ratelimit. Commands: all = everything. Examples: hematite --api-design rest | hematite --api-design openapi | hematite --api-design graphql"
    )]
    pub api_design: Option<String>,

    #[arg(
        long = "db-design",
        help_heading = "Developer Reference",
        value_name = "TOPIC",
        help = "Database design reference — normalization, indexes, ACID/transactions, distributed/CAP, NoSQL, migrations. Topics: normalization, indexes, transactions, distributed, nosql, migrations. Commands: all = everything. Examples: hematite --db-design normalization | hematite --db-design indexes | hematite --db-design nosql"
    )]
    pub db_design: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "Performance engineering reference — profiling, memory, benchmarking, web vitals, DB, optimization. Topics: profiling, memory, benchmarking, web, database, optimization. Commands: all = everything. Examples: hematite --perf-ref profiling | hematite --perf-ref memory | hematite --perf-ref benchmarking"
    )]
    pub perf_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "Docker Compose reference — services, networking, volumes, health checks, production, logging. Topics: basics, networking, health, production, logging, tips. Commands: all = everything. Examples: hematite --docker-compose basics | hematite --docker-compose health | hematite --docker-compose production"
    )]
    pub docker_compose: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "WebAssembly reference — WAT format, memory model, WASI, wasm-pack, wabt, component model. Topics: format, memory, wasi, wasm-pack, wabt, component. Commands: all = everything. Examples: hematite --wasm-ref format | hematite --wasm-ref wasi | hematite --wasm-ref wasm-pack"
    )]
    pub wasm_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "Web accessibility (a11y) reference — WCAG 2.1/2.2, ARIA, keyboard nav, contrast, screen readers, testing. Topics: wcag, aria, keyboard, contrast, screen-readers, testing. Commands: all = everything. Examples: hematite --accessibility aria | hematite --accessibility keyboard | hematite --accessibility contrast"
    )]
    pub accessibility: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "Kubernetes reference — pods, deployments, services, ingress, RBAC, troubleshooting, Helm. Topics: pods, services, config, rbac, troubleshoot, helm. Commands: all = everything. Examples: hematite --k8s-ref pods | hematite --k8s-ref rbac | hematite --k8s-ref troubleshoot"
    )]
    pub k8s_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "Observability reference — metrics/Prometheus, tracing/OpenTelemetry, logging, SLOs, dashboards, alerting. Topics: metrics, tracing, logging, slo, dashboards, alerting. Commands: all = everything. Examples: hematite --observability metrics | hematite --observability tracing | hematite --observability slo"
    )]
    pub observability: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "Advanced Terraform reference — modules, state management, workspaces, testing, patterns, CI/CD. Topics: modules, state, workspaces, testing, patterns, cicd. Commands: all = everything. Examples: hematite --terraform-adv modules | hematite --terraform-adv state | hematite --terraform-adv testing"
    )]
    pub terraform_adv: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "Security scanning reference — SAST, DAST, dependency scanning, container security, secrets detection, compliance. Topics: sast, dast, deps, containers, secrets, compliance. Commands: all = everything. Examples: hematite --security-scan sast | hematite --security-scan secrets | hematite --security-scan compliance"
    )]
    pub security_scan: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "Machine learning reference — fundamentals, models, feature engineering, training, evaluation, deployment/MLOps. Topics: fundamentals, models, features, training, evaluation, deployment. Commands: all = everything. Examples: hematite --ml-ref models | hematite --ml-ref training | hematite --ml-ref evaluation"
    )]
    pub ml_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "Rust design patterns — error handling, iterators, traits, async/concurrency, design patterns. Topics: errors, iterators, traits, async, concurrency, design. Commands: all = everything. Examples: hematite --rust-patterns errors | hematite --rust-patterns async | hematite --rust-patterns iterators"
    )]
    pub rust_patterns: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "Event-driven architecture reference — pub/sub, Kafka, event sourcing, CQRS, messaging, CDC, schema evolution. Topics: patterns, kafka, event-sourcing, messaging, cdc, schema. Commands: all = everything. Examples: hematite --event-driven kafka | hematite --event-driven event-sourcing | hematite --event-driven cdc"
    )]
    pub event_driven: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "TOPIC",
        help = "API gateway reference — patterns, products (Kong/Envoy/Istio), auth, service mesh, observability, design. Topics: patterns, products, service-mesh, auth, observability, design. Commands: all = everything. Examples: hematite --api-gateway patterns | hematite --api-gateway service-mesh | hematite --api-gateway auth"
    )]
    pub api_gateway: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "CI/CD reference — offline replacement for GitHub Actions/GitLab CI docs. Topics: concepts, github-actions, gitlab, pipelines, security, jenkins. Aliases: dora, gha, canary, blue-green, sast, jenkinsfile, trivy, and more. Examples: hematite --cicd-ref github-actions | hematite --cicd-ref canary | hematite --cicd-ref sast | hematite --cicd-ref all"
    )]
    pub cicd_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Design patterns reference — offline replacement for refactoring.guru. Topics: creational, structural, behavioral, concurrency, rust-idioms, architecture. Aliases: singleton, observer, builder-pattern, typestate, hexagonal, cqrs-pattern, and more. Examples: hematite --design-patterns creational | hematite --design-patterns observer | hematite --design-patterns hexagonal | hematite --design-patterns all"
    )]
    pub design_patterns: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Auth reference — offline replacement for auth0 docs, OIDC spec. Topics: oauth2, oidc, jwt, session, saml, security. Aliases: pkce, id-token, rs256, httponly, bcrypt, totp, webauthn, and more. Examples: hematite --auth-ref oauth2 | hematite --auth-ref pkce | hematite --auth-ref totp | hematite --auth-ref all"
    )]
    pub auth_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Linux kernel internals reference — offline replacement for kernel.org docs. Topics: syscalls, memory, namespaces, cgroups, ebpf, scheduler. Aliases: strace, mmap, oom-killer, cgroup-v2, bpftrace, cfs, io-uring, and more. Examples: hematite --linux-kernel syscalls | hematite --linux-kernel ebpf | hematite --linux-kernel cgroups | hematite --linux-kernel all"
    )]
    pub linux_kernel: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Advanced database reference — offline replacement for PostgreSQL/MySQL docs tabs. Topics: indexing, partitioning, replication, query-opt, transactions, maintenance. Aliases: btree, gin, explain, mvcc, pgbouncer, pitr, vacuum, patroni, and more. Examples: hematite --database-adv indexing | hematite --database-adv explain | hematite --database-adv mvcc | hematite --database-adv all"
    )]
    pub database_adv: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Advanced networking reference — offline replacement for Cisco/networking bookmarks. Topics: subnetting, routing, vlan, qos, nat, tunneling. Aliases: cidr, ospf, bgp, stp, dscp, ipsec, wireguard, vrf, and more. Examples: hematite --networking-adv subnetting | hematite --networking-adv ospf | hematite --networking-adv wireguard | hematite --networking-adv all"
    )]
    pub networking_adv: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Software testing reference — offline replacement for testing docs and blogs. Topics: strategy, unit, integration, e2e, performance, mocking. Aliases: tdd, bdd, playwright, k6, testcontainers, mockall, pact, snapshot, and more. Examples: hematite --testing-ref strategy | hematite --testing-ref playwright | hematite --testing-ref k6 | hematite --testing-ref all"
    )]
    pub testing_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Compiler internals reference — offline replacement for compiler books and LLVM docs. Topics: parsing, ast, ir, optimization, codegen, tools. Aliases: lexer, pratt, lalrpop, tree-sitter, mir, ssa, llvm-ir, cranelift, pgo, cargo-asm, and more. Examples: hematite --compiler-ref parsing | hematite --compiler-ref ir | hematite --compiler-ref optimization | hematite --compiler-ref all"
    )]
    pub compiler_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Monitoring & observability reference — offline replacement for Prometheus/Grafana docs. Topics: slo, prometheus, alerting, grafana, otel, logging. Aliases: sli, promql, alertmanager, loki, opentelemetry, structured-logging, and more. Examples: hematite --monitoring-ref slo | hematite --monitoring-ref prometheus | hematite --monitoring-ref otel | hematite --monitoring-ref all"
    )]
    pub monitoring_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Search engine reference — offline replacement for Elasticsearch docs. Topics: concepts, elasticsearch, vector, ranking, performance, design. Aliases: inverted-index, bm25, hnsw, faiss, qdrant, ltr, bulk-api, and more. Examples: hematite --search-ref concepts | hematite --search-ref vector | hematite --search-ref elasticsearch | hematite --search-ref all"
    )]
    pub search_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Network protocols reference — offline replacement for protocol docs. Topics: grpc, websocket, graphql, mqtt, http23, tls. Aliases: protobuf, grpc-gateway, dataloader, federation, qos, quic, mtls, and more. Examples: hematite --protocols-ref grpc | hematite --protocols-ref tls | hematite --protocols-ref graphql | hematite --protocols-ref all"
    )]
    pub protocols_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Container internals reference — offline replacement for Docker/containerd docs. Topics: namespaces, cgroups, oci, runtimes, build, security. Aliases: pid-namespace, cgroup-v2, overlayfs, runc, containerd, dockerfile, multi-stage, buildkit, seccomp, and more. Examples: hematite --container-ref namespaces | hematite --container-ref cgroups | hematite --container-ref build | hematite --container-ref all"
    )]
    pub container_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Regex engine reference — offline replacement for regex docs and regex101 for concepts. Topics: theory, syntax, advanced, rust-regex, performance, tools. Aliases: nfa, dfa, backtracking, lookahead, possessive-quantifiers, regex-crate, redos, and more. Examples: hematite --regex-engine theory | hematite --regex-engine syntax | hematite --regex-engine rust-regex | hematite --regex-engine all"
    )]
    pub regex_engine: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Git internals reference — offline replacement for Git docs and Pro Git book. Topics: objects, pack-files, plumbing, reflog, rewrite, advanced-ops. Aliases: blob, cat-file, git-gc, git-reflog, interactive-rebase, git-bisect, git-submodules, and more. Examples: hematite --git-internals objects | hematite --git-internals reflog | hematite --git-internals rewrite | hematite --git-internals all"
    )]
    pub git_internals: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Data serialization formats reference — offline replacement for format docs. Topics: binary, columnar, streaming, text, schema, compression. Aliases: msgpack, parquet, avro, json-format, yaml-format, toml-format, zstd, lz4, snappy, and more. Examples: hematite --data-formats binary | hematite --data-formats columnar | hematite --data-formats compression | hematite --data-formats all"
    )]
    pub data_formats: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Web performance reference — offline replacement for web.dev/performance. Topics: cwv, rendering, caching, cdn, bundling, images. Aliases: core-web-vitals, lcp, inp, cls, ttfb, critical-render-path, cache-control, service-worker-cache, bundle-splitting, tree-shaking, image-optimization, avif, webp, and more. Examples: hematite --web-perf cwv | hematite --web-perf caching | hematite --web-perf all"
    )]
    pub web_perf: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "SQL query tuning reference — offline replacement for use-the-index-luke.com. Topics: explain, indexes, statistics, optimizer, partitioning, advanced-sql. Aliases: execution-plan, btree-index, gin-index, partial-index, pg-stats, work-mem, range-partition, window-functions, cte, recursive-cte, lateral-join, and more. Examples: hematite --sql-tuning explain | hematite --sql-tuning indexes | hematite --sql-tuning all"
    )]
    pub sql_tuning: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Concurrency patterns reference — offline replacement for concurrency docs. Topics: primitives, atomics, channels, lock-free, async, patterns. Aliases: mutex, rwlock, semaphore, acquire-release, seqcst, cas-loop, channel-mpsc, backpressure, lock-free-queue, hazard-pointers, async-await, tokio-runtime, thread-pool, producer-consumer, rayon, and more. Examples: hematite --concurrency atomics | hematite --concurrency async | hematite --concurrency all"
    )]
    pub concurrency: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Cloud-native patterns reference — offline replacement for cloud docs. Topics: 12factor, patterns, service-mesh, observability, deployment, events. Aliases: twelve-factor, circuit-breaker, bulkhead, saga-pattern, cqrs-pattern, istio, envoy-proxy, prometheus-stack, distributed-tracing, blue-green, canary-deployment, gitops, event-sourcing, outbox-pattern, and more. Examples: hematite --cloud-native 12factor | hematite --cloud-native patterns | hematite --cloud-native all"
    )]
    pub cloud_native: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Regex pattern cookbook — offline replacement for regex cheat sheets. Topics: common, text, code, log, security, network. Aliases: validation, email-pattern, url-pattern, uuid-pattern, text-processing, duplicate-words, code-extraction, function-def, log-parsing, syslog-pattern, secrets-detection, aws-key, network-patterns, cidr-pattern, and more. Examples: hematite --regex-patterns common | hematite --regex-patterns log | hematite --regex-patterns all"
    )]
    pub regex_patterns: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "HTTP security reference — offline replacement for security header docs. Topics: csp, cors, hsts, headers, tls, cookies. Aliases: content-security-policy, nonce-csp, cross-origin-resource-sharing, preflight, strict-transport-security, hsts-preload, x-content-type-options, referrer-policy, permissions-policy, tls-config, cipher-suites, cookie-security, samesite, csrf-protection, and more. Examples: hematite --http-security csp | hematite --http-security cors | hematite --http-security all"
    )]
    pub http_security: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "gRPC and Protobuf reference — offline replacement for grpc.io docs. Topics: proto, services, codegen, interceptors, transport, tools. Aliases: protobuf, proto3, message-definition, service-definition, server-streaming, code-generation, protoc, tonic, buf-tool, grpc-middleware, grpc-metadata, grpc-tls, grpc-load-balancing, grpcurl, and more. Examples: hematite --grpc-ref proto | hematite --grpc-ref interceptors | hematite --grpc-ref all"
    )]
    pub grpc_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "WebAssembly runtime reference — offline replacement for webassembly.org docs. Topics: core, wasi, js, rust, components, perf. Aliases: wasm-concepts, binary-format, wasm-system-interface, wasi-preview2, wasi-http, wasm-javascript, instantiatestreaming, wasm-pack, wasm-bindgen, component-model, wit-idl, wasm-simd, wasm-threads, wasm-profiling, and more. Examples: hematite --wasm-runtime wasi | hematite --wasm-runtime rust | hematite --wasm-runtime all"
    )]
    pub wasm_runtime: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Linux performance tooling reference — offline replacement for perf-wiki. Topics: perf, ebpf, ftrace, flamegraph, memory, syscall. Aliases: perf-tool, perf-stat, bpftrace, bcc-tools, kprobe, opensnoop, kernel-tracer, function-graph, flame-graph, brendan-gregg, off-cpu, valgrind, asan-rust, strace, ltrace, and more. Examples: hematite --linux-perf ebpf | hematite --linux-perf flamegraph | hematite --linux-perf all"
    )]
    pub linux_perf: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Database migration reference — offline replacement for Flyway/Liquibase/Atlas docs. Topics: concepts, flyway, liquibase, atlas, patterns, rollback. Aliases: schema-migration, flyway-migrate, flyway-cli, liquibase-changeset, atlas-schema, zero-downtime-migration, expand-contract, concurrent-index, migration-rollback, transactional-ddl, and more. Examples: hematite --db-migrations flyway | hematite --db-migrations patterns | hematite --db-migrations all"
    )]
    pub db_migrations: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "OAuth 2.0 and OIDC reference — offline replacement for oauth.net docs. Topics: flows, tokens, oidc, security, providers, jwt. Aliases: authorization-code, pkce, client-credentials, refresh-token-flow, access-token, id-token, jwt-verification, openid-connect, oidc-scopes, discovery-endpoint, auth0, okta, azure-ad, jsonwebtoken, rs256, and more. Examples: hematite --oauth-ref flows | hematite --oauth-ref tokens | hematite --oauth-ref all"
    )]
    pub oauth_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Kubernetes security reference — offline replacement for k8s security docs. Topics: rbac, netpol, podsec, secrets, supply, audit. Aliases: kubernetes-rbac, rolebinding, serviceaccount-rbac, network-policy, default-deny, pod-security-standards, security-context, runasnonroot, encryption-at-rest, external-secrets, vault-agent, image-signing, cosign, falco, kube-bench, and more. Examples: hematite --k8s-security rbac | hematite --k8s-security podsec | hematite --k8s-security all"
    )]
    pub k8s_security: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "API versioning reference — offline replacement for REST versioning docs. Topics: strategies, semver, routing, graphql, hypermedia, docs. Aliases: url-versioning, header-versioning, breaking-changes, backwards-compatibility, contract-testing, hateoas, hal-format, openapi-changelog, and more. Examples: hematite --api-versioning strategies | hematite --api-versioning semver | hematite --api-versioning all"
    )]
    pub api_versioning: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Message queue reference — offline replacement for Kafka/RabbitMQ docs. Topics: kafka, rabbitmq, patterns, sqs, reliability, schema. Aliases: apache-kafka, kafka-topics, kafka-producer, kafka-consumer, rabbit-mq, amqp, dlx, outbox-pattern, saga-pattern, aws-sqs, sqs-fifo, avro-kafka, schema-registry, cloudevents, asyncapi, and more. Examples: hematite --message-queue kafka | hematite --message-queue rabbitmq | hematite --message-queue all"
    )]
    pub message_queue: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Caching reference — offline replacement for caching docs. Topics: strategies, redis, http, app, memcached, patterns. Aliases: cache-aside, write-through, write-behind, thundering-herd, redis-commands, redis-cluster, cache-control-header, etag-caching, cdn-caching, memoization, consistent-hashing-cache, xfetch, negative-caching, and more. Examples: hematite --caching-ref strategies | hematite --caching-ref redis | hematite --caching-ref all"
    )]
    pub caching_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Load testing reference — offline replacement for k6/Locust docs. Topics: k6, wrk, locust, metrics, scenarios, profiling. Aliases: k6-script, k6-stages, k6-thresholds, wrk-benchmark, vegeta, locustfile, load-test-metrics, percentiles, p99, coordinated-omission, amdahl-law, littles-law, smoke-test, stress-test, spike-test, soak-test, go-pprof, cargo-flamegraph-load, and more. Examples: hematite --load-testing k6 | hematite --load-testing metrics | hematite --load-testing all"
    )]
    pub load_testing: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Service mesh reference — offline replacement for Istio/Envoy/Linkerd docs. Topics: istio, envoy, linkerd, patterns, consul, dapr. Aliases: virtualservice, destinationrule, istioctl, envoy-proxy, xds-api, linkerd-inject, traffic-split, mesh-patterns, canary-mesh, spiffe, consul-connect, dapr-sidecar, dapr-state, and more. Examples: hematite --service-mesh istio | hematite --service-mesh patterns | hematite --service-mesh all"
    )]
    pub service_mesh: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Advanced observability reference — offline replacement for OTel/Jaeger/Grafana docs. Topics: otel, tracing, metrics, logging, alerting, dashboards. Aliases: opentelemetry, otlp, distributed-tracing, jaeger-tracing, slo-metrics-adv, error-budget, structured-logging, loki-logging, alertmanager, grafana-dashboards, use-method, red-method, and more. Examples: hematite --observability-adv otel | hematite --observability-adv tracing | hematite --observability-adv all"
    )]
    pub observability_adv: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Terminal tools reference — offline replacement for fzf/ripgrep/zsh docs. Topics: fzf, ripgrep, shell-tools, zsh, tmux-adv, wezterm. Aliases: fuzzy-finder, rg-search, modern-cli, zsh-config, zsh-plugins, tmux-advanced, tpm-plugins, terminal-emulator, starship-prompt, powerlevel10k, eza-ls, bat-cat, and more. Examples: hematite --terminal-tools fzf | hematite --terminal-tools zsh | hematite --terminal-tools all"
    )]
    pub terminal_tools: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Data pipeline reference — offline replacement for dbt/Airflow/Spark docs. Topics: dbt, airflow, spark, streaming, etl, lakehouse. Aliases: dbt-models, apache-airflow, pyspark, kafka-streams, apache-flink, etl-patterns, delta-lake, apache-iceberg, duckdb, data-quality, debezium-cdc, and more. Examples: hematite --data-pipeline dbt | hematite --data-pipeline spark | hematite --data-pipeline all"
    )]
    pub data_pipeline: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Security tools reference — offline replacement for OWASP and tool docs. Topics: nmap, web, sast, network, hardening, cloud-sec. Aliases: nmap-scan, owasp-zap, burp-suite, sqlmap, semgrep, trivy-scan, snyk, wireshark, metasploit, linux-hardening, cis-benchmark, aws-security, kube-bench, falco-security. Examples: hematite --security-tools nmap | hematite --security-tools owasp-zap | hematite --security-tools all"
    )]
    pub security_tools: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Advanced cryptography reference — offline replacement for crypto docs. Topics: tls, pki, jwt-adv, primitives, mtls. Aliases: tls-deep-dive, tls13, tls-handshake, lets-encrypt, acme-protocol, certbot, cert-manager, jwt-advanced, jwt-vulnerabilities, paseto, aes-gcm, ed25519, argon2, mutual-tls, client-certificates. Examples: hematite --crypto-adv tls | hematite --crypto-adv jwt-adv | hematite --crypto-adv all"
    )]
    pub crypto_adv: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Browser dev tools reference — offline replacement for MDN and DevTools docs. Topics: devtools, web-apis, pwa, websocket, performance. Aliases: chrome-devtools, core-web-vitals, lighthouse-audit, fetch-api, intersection-observer, service-workers, workbox, websocket-api, webrtc, critical-rendering-path, code-splitting. Examples: hematite --browser-dev devtools | hematite --browser-dev pwa | hematite --browser-dev all"
    )]
    pub browser_dev: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Rust async reference — offline replacement for Tokio and async-std docs. Topics: tokio, channels, streams, axum, patterns. Aliases: tokio-runtime, tokio-spawn, tokio-select, mpsc-channel, oneshot-channel, broadcast-channel, watch-channel, async-streams, tokio-stream, axum-framework, axum-extractors, async-trait, cancellation-token. Examples: hematite --rust-async tokio | hematite --rust-async channels | hematite --rust-async all"
    )]
    pub rust_async: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Regex visualizer & pattern library — offline replacement for regex101.com. Topics: syntax, breakdown, lookahead, performance, common. Aliases: regex-syntax, character-classes, quantifiers, pattern-breakdown, email-regex, catastrophic-backtracking, nfa-dfa, rust-regex, common-regex, validation-regex. Examples: hematite --regex-viz syntax | hematite --regex-viz breakdown | hematite --regex-viz all"
    )]
    pub regex_viz: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "SQL window functions reference — offline replacement for PostgreSQL/MySQL window docs. Topics: basics, ranking, offset, frames, practical. Aliases: over-clause, row-number, rank, dense-rank, lag-lead, rows-between, moving-average, gaps-islands, sessionization, window-frames. Examples: hematite --sql-window basics | hematite --sql-window ranking | hematite --sql-window all"
    )]
    pub sql_window: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "CSS Grid & Flexbox reference — offline replacement for css-tricks.com complete guides. Topics: grid, flexbox, layouts, responsive, tricks. Aliases: css-grid, css-flexbox, holy-grail, auto-fit, fr-unit, justify-content, media-queries, clamp, container-queries, subgrid, css-variables. Examples: hematite --css-grid grid | hematite --css-grid flexbox | hematite --css-grid all"
    )]
    pub css_grid: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Cloud cost optimization reference — offline replacement for AWS/GCP/Azure cost docs. Topics: aws, gcp, azure, finops, tools. Aliases: aws-savings-plans, gcp-cud, azure-reservations, finops, infracost, kubecost, rightsizing, spot-instances, tagging-strategy, unit-economics, cloud-custodian. Examples: hematite --cloud-cost aws | hematite --cloud-cost finops | hematite --cloud-cost all"
    )]
    pub cloud_cost: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "HTTP/2, HTTP/3, gRPC, server push & SSE reference — offline replacement for http2.github.io and RFC docs. Topics: http2, http3, grpc, push, sse. Aliases: h2, h3, quic, hpack, multiplexing, grpc-framing, grpc-status, server-push, 103-early-hints, sse-vs-websocket, websocket-h2. Examples: hematite --http2 h2 | hematite --http2 grpc | hematite --http2 all"
    )]
    pub http2: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "OAuth 2.0 / OIDC flows reference — offline replacement for oauth.net and OpenID docs. Topics: auth-code, client-creds, tokens, oidc, security. Aliases: pkce, code-challenge, authorization-code, client-credentials, machine-to-machine, device-flow, bearer-token, refresh-token, openid-connect, oidc-discovery, oauth2-vulnerabilities. Examples: hematite --oauth2-flow pkce | hematite --oauth2-flow tokens | hematite --oauth2-flow all"
    )]
    pub oauth2_flow: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Linux networking reference — offline replacement for iproute2 man pages and netfilter docs. Topics: ip, iptables, tc, bonding, tools. Aliases: ip-command, iproute2, iptables-ref, nftables, traffic-control, tc-netem, rate-limiting, vlan-linux, bonding-linux, ethtool, tcpdump-linux, iperf3. Examples: hematite --linux-net ip | hematite --linux-net iptables | hematite --linux-net all"
    )]
    pub linux_net: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Prometheus / Grafana / alerting reference — offline replacement for prometheus.io and grafana.com docs. Topics: promql, recording, grafana, exporters, alerting. Aliases: prometheus-query, rate-function, histogram-quantile, recording-rules, alert-rules, alertmanager-routing, grafana-ref, loki-ref, node-exporter, burn-rate, slo-alerting. Examples: hematite --prom-ref promql | hematite --prom-ref alerting | hematite --prom-ref all"
    )]
    pub prom_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Kubernetes advanced patterns — offline replacement for kubernetes.io advanced docs. Topics: scheduling, networking, storage, security, operators. Aliases: node-affinity, pod-affinity, taints-tolerations, k8s-networking, network-policy, pv-pvc, storageclass, pod-security-admission, rbac-k8s, crd-k8s, kubebuilder. Examples: hematite --k8s-adv scheduling | hematite --k8s-adv operators | hematite --k8s-adv all"
    )]
    pub k8s_adv: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Rust macros reference — offline replacement for Rust Reference macro chapters and proc-macro book. Topics: declarative, procedural, attribute, patterns, advanced. Aliases: macro-rules, proc-macro, derive-macro, syn-crate, quote-crate, attribute-macro, tt-muncher, cargo-expand, trybuild. Examples: hematite --rust-macros declarative | hematite --rust-macros procedural | hematite --rust-macros all"
    )]
    pub rust_macros: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Database internals reference — offline replacement for CMU lecture notes and use-the-index-luke.com. Topics: storage, indexes, transactions, replication, performance. Aliases: b-tree-internals, lsm-tree, mvcc-internals, index-types, isolation-levels, db-locking, postgres-replication, sharding-strategy, autovacuum-tuning, pg-stat-statements. Examples: hematite --db-internals storage | hematite --db-internals indexes | hematite --db-internals all"
    )]
    pub db_internals: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "OpenAPI / Swagger reference — offline replacement for spec.openapis.org and swagger.io docs. Topics: spec, paths, schemas, security, tooling. Aliases: openapi-structure, openapi-paths, openapi-schemas, openapi-security, swagger-ui, redoc, openapi-generator, spectral-linter, prism-mock, schemathesis. Examples: hematite --openapi-ref schemas | hematite --openapi-ref security | hematite --openapi-ref all"
    )]
    pub openapi_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "GraphQL reference — offline replacement for graphql.org/learn. Topics: queries, schema, subscriptions, resolvers, tooling. Aliases: graphql-queries, graphql-schema, graphql-mutations, graphql-fragments, graphql-subscriptions, graphql-resolvers, dataloader, apollo-server, apollo-client. Examples: hematite --graphql queries | hematite --graphql schema | hematite --graphql all"
    )]
    pub graphql: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "React reference — offline replacement for react.dev docs. Topics: hooks, patterns, state, rendering, nextjs. Aliases: react-hooks, usestate, useeffect, usememo, react-memo, tanstack-query, zustand, suspense-react, next-js, server-actions. Examples: hematite --react-ref hooks | hematite --react-ref state | hematite --react-ref all"
    )]
    pub react_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Node.js reference — offline replacement for nodejs.org docs. Topics: core, streams, http, workers, runtime. Aliases: node-fs, node-path, node-streams, eventemitter, node-http, node-fetch, worker-threads, node-cluster, node-esm, node-env. Examples: hematite --node-ref core | hematite --node-ref streams | hematite --node-ref all"
    )]
    pub node_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Java reference — offline replacement for docs.oracle.com. Topics: collections, streams, concurrency, modern, spring. Aliases: java-collections, java-streams, java-optional, virtual-threads, completablefuture, java-records, java-sealed, java21, spring-boot, spring-jpa. Examples: hematite --java-ref collections | hematite --java-ref modern | hematite --java-ref all"
    )]
    pub java_ref: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "Python advanced patterns — offline replacement for docs.python.org advanced topics. Topics: decorators, typing, async, dataclasses, patterns. Aliases: python-decorators, python-typing, python-async, python-dataclasses, python-patterns. Examples: hematite --python-adv decorators | hematite --python-adv async | hematite --python-adv all"
    )]
    pub python_adv: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "Go advanced patterns — offline replacement for go.dev docs. Topics: concurrency, generics, errors, modules, interfaces. Aliases: go-concurrency, go-generics, go-errors, go-modules, go-interfaces. Examples: hematite --go-adv concurrency | hematite --go-adv generics | hematite --go-adv all"
    )]
    pub go_adv: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "Vue 3 reference — offline replacement for vuejs.org docs. Topics: composition, reactivity, components, pinia, router. Aliases: vue-composition-api, vue-reactivity, pinia, vue-router. Examples: hematite --vue-ref composition | hematite --vue-ref pinia | hematite --vue-ref all"
    )]
    pub vue_ref: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "AWS reference — offline replacement for docs.aws.amazon.com. Topics: s3, ec2-iam, lambda, networking, rds-dynamo. Aliases: aws-s3, aws-ec2, aws-lambda, aws-vpc, aws-dynamodb. Examples: hematite --aws-ref s3 | hematite --aws-ref lambda | hematite --aws-ref all"
    )]
    pub aws_ref: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "Helm chart reference — offline replacement for helm.sh/docs. Topics: chart-structure, templating, values, hooks, releases. Aliases: helm-chart, helm-templating, helm-values, helm-hooks, helm-repo. Examples: hematite --helm-ref chart-structure | hematite --helm-ref templating | hematite --helm-ref all"
    )]
    pub helm_ref: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "Redis reference — offline replacement for redis.io/docs. Topics: data-types, commands, pub-sub, patterns, config. Aliases: redis-types, redis-commands, redis-pubsub, redis-streams, redis-patterns. Examples: hematite --redis-ref data-types | hematite --redis-ref patterns | hematite --redis-ref all"
    )]
    pub redis_ref: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "Browser Web APIs reference — offline replacement for MDN. Topics: fetch, websockets, service-workers, workers, storage. Aliases: fetch-api, websocket, service-worker, web-worker, indexeddb. Examples: hematite --web-apis fetch | hematite --web-apis websockets | hematite --web-apis all"
    )]
    pub web_apis: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "Linux container internals — offline reference for namespaces, cgroups, seccomp, podman, and image builds. Topics: namespaces, cgroups, seccomp, podman, container-build. Aliases: linux-namespaces, cgroups-v2, seccomp, podman, image-layers. Examples: hematite --linux-containers namespaces | hematite --linux-containers cgroups | hematite --linux-containers all"
    )]
    pub linux_containers: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "CI/CD advanced patterns — offline replacement for docs.github.com and docs.gitlab.com. Topics: github-actions, gitlab-ci, argocd, pipeline-patterns, tekton. Aliases: gha-matrix, gitlab-ci, argocd, cicd-patterns, tekton. Examples: hematite --ci-cd-adv github-actions | hematite --ci-cd-adv argocd | hematite --ci-cd-adv all"
    )]
    pub ci_cd_adv: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "PostgreSQL operations reference — offline replacement for postgresql.org/docs. Topics: explain, indexes, connections, replication, maintenance. Aliases: pg-explain, pg-indexes, pgbouncer, pg-replication, pg-vacuum. Examples: hematite --sql-ops explain | hematite --sql-ops indexes | hematite --sql-ops all"
    )]
    pub sql_ops: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "TypeScript advanced type patterns — offline replacement for typescriptlang.org/docs. Topics: mapped, conditional, advanced, decorators, modules. Aliases: ts-mapped-types, ts-conditional-types, ts-branded-types, ts-decorators, ts-module-augmentation. Examples: hematite --ts-patterns mapped | hematite --ts-patterns conditional | hematite --ts-patterns all"
    )]
    pub ts_patterns: Option<String>,

    #[arg(
        long,
        value_name = "QUERY",
        help = "Mobile web & PWA reference — offline replacement for MDN mobile docs. Topics: pwa, responsive, performance, touch, device-apis. Aliases: progressive-web-app, responsive-design, core-web-vitals, touch-events, geolocation. Examples: hematite --mobile-web pwa | hematite --mobile-web performance | hematite --mobile-web all"
    )]
    pub mobile_web: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Ansible reference — offline replacement for Ansible docs. Topics: playbooks, roles, vault, modules, advanced. Aliases: ansible-playbook, ansible-roles, ansible-vault, ansible-modules, ansible-loops. Examples: hematite --ansible-ref playbooks | hematite --ansible-ref vault | hematite --ansible-ref all"
    )]
    pub ansible_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Nginx reference — offline replacement for nginx.org docs. Topics: config, upstream, rate-limiting, tls, performance. Aliases: nginx-config, nginx-upstream, nginx-rate-limit, nginx-tls, nginx-performance. Examples: hematite --nginx-ref config | hematite --nginx-ref tls | hematite --nginx-ref all"
    )]
    pub nginx_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Protocol Buffers & gRPC reference — offline replacement for protobuf.dev. Topics: proto3, grpc, types, tooling, patterns. Aliases: proto3-syntax, grpc-service, proto-types, buf-tool, grpc-patterns. Examples: hematite --protobuf-ref proto3 | hematite --protobuf-ref grpc | hematite --protobuf-ref all"
    )]
    pub protobuf_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Rust embedded reference — offline replacement for embedded Rust docs. Topics: no-std, embedded-hal, rtic, defmt, debugging. Aliases: no_std, hal-traits, rtic-framework, defmt-logging, embedded-debug. Examples: hematite --rust-embedded no-std | hematite --rust-embedded rtic | hematite --rust-embedded all"
    )]
    pub rust_embedded: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Svelte & SvelteKit reference — offline replacement for svelte.dev. Topics: components, stores, sveltekit, ssr, advanced. Aliases: svelte-component, svelte-store, sveltekit-routing, sveltekit-ssr, svelte-transition. Examples: hematite --svelte-ref components | hematite --svelte-ref sveltekit | hematite --svelte-ref all"
    )]
    pub svelte_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Kafka advanced reference — offline replacement for Kafka docs. Topics: consumer-groups, compaction, streams, schema, operations. Aliases: kafka-consumer-group, log-compaction, kafka-streams, schema-registry, kafka-ops. Examples: hematite --kafka-adv consumer-groups | hematite --kafka-adv streams | hematite --kafka-adv all"
    )]
    pub kafka_adv: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Deno runtime reference — offline replacement for deno.land docs. Topics: runtime, apis, fresh, kv, testing. Aliases: deno-run, deno-permissions, fresh-framework, deno-kv, deno-test. Examples: hematite --deno-ref runtime | hematite --deno-ref kv | hematite --deno-ref all"
    )]
    pub deno_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "OpenTelemetry reference — offline replacement for opentelemetry.io docs. Topics: sdk, traces, metrics, logs, collector. Aliases: otel-sdk, otel-span, otel-counter, otel-logging, otel-collector. Examples: hematite --otel-ref traces | hematite --otel-ref collector | hematite --otel-ref all"
    )]
    pub otel_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Zig language reference — offline replacement for ziglang.org docs. Topics: basics, memory, structs, errors, comptime, build. Aliases: zig-basics, allocators, zig-structs, zig-errors, zig-comptime, zig-build. Examples: hematite --zig-ref basics | hematite --zig-ref comptime | hematite --zig-ref all"
    )]
    pub zig_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Redis advanced patterns — offline replacement for redis.io advanced docs. Topics: data-structures, lua, pubsub, cluster, patterns, persistence. Aliases: zset, scripting, pub-sub, redis-cluster, distributed-lock, rdb. Examples: hematite --redis-adv patterns | hematite --redis-adv lua | hematite --redis-adv all"
    )]
    pub redis_adv: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Nix package manager reference — offline replacement for nixos.org docs. Topics: basics, shells, flakes, nixos, derivations, home-manager. Aliases: nix-env, nix-shell, nix-flakes, nixos-config, mkderivation, home-nix. Examples: hematite --nix-ref flakes | hematite --nix-ref shells | hematite --nix-ref all"
    )]
    pub nix_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "GDB debugger reference — offline replacement for sourceware.org/gdb docs. Topics: basics, breakpoints, stepping, inspect, advanced, tui. Aliases: gdb-break, watch, backtrace, print, examine, remote, core-dump. Examples: hematite --gdb-ref breakpoints | hematite --gdb-ref inspect | hematite --gdb-ref all"
    )]
    pub gdb_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "CMake build system reference — offline replacement for cmake.org docs. Topics: basics, targets, find, variables, testing, modern. Aliases: cmakelists, cc-binary, find-package, fetchcontent, ctest, presets, toolchain. Examples: hematite --cmake-ref targets | hematite --cmake-ref find | hematite --cmake-ref all"
    )]
    pub cmake_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Bazel build system reference — offline replacement for bazel.build docs. Topics: basics, rules, deps, query, remote, starlark. Aliases: workspace, cc-library, bzlmod, bazel-query, remote-cache, macros, gazelle. Examples: hematite --bazel-ref rules | hematite --bazel-ref starlark | hematite --bazel-ref all"
    )]
    pub bazel_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Valgrind memory analysis reference — offline replacement for valgrind.org docs. Topics: memcheck, callgrind, massif, helgrind, sgcheck, asan. Aliases: leak-check, call-graph, heap-profiler, race-detection, sanitizers. Examples: hematite --valgrind-ref memcheck | hematite --valgrind-ref asan | hematite --valgrind-ref all"
    )]
    pub valgrind_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "Scrum/Agile reference card — offline replacement for scrumguides.org. Topics: roles, events, artifacts, backlog, metrics, scaling. Aliases: product-owner, sprint, sprint-planning, user-stories, velocity, nexus. Examples: hematite --scrum-ref events | hematite --scrum-ref backlog | hematite --scrum-ref all"
    )]
    pub scrum_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "PowerShell advanced scripting reference. Topics: objects, scripting, remoting, modules, regex, winapi. Aliases: ps-objects, pipeline, advanced-functions, ps-modules, ps-regex, com, cim. Examples: hematite --powershell-adv objects | hematite --powershell-adv remoting | hematite --powershell-adv all"
    )]
    pub powershell_adv: Option<String>,

    #[arg(
        long,
        help_heading = "Developer Toolkit",
        value_name = "QUERY",
        help = "JVM internals and tuning reference — offline replacement for oracle.com/jvm docs. Topics: memory, gc, jit, threads, profiling, flags. Aliases: jvm-memory, heap, g1gc, zgc, jfr, async-profiler, virtual-threads. Examples: hematite --jvm-ref gc | hematite --jvm-ref profiling | hematite --jvm-ref all"
    )]
    pub jvm_ref: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Lorem ipsum generator — offline replacement for lipsum.com. Commands: (bare) or <N> = N paragraphs (default 1); words <N> = N words; sentences <N> = N sentences. Examples: hematite --lorem | hematite --lorem 3 | hematite --lorem 'words 50' | hematite --lorem 'sentences 5'"
    )]
    pub lorem: Option<String>,

    #[arg(
        long,
        help_heading = "Math & Science",
        value_name = "QUERY",
        help = "Case converter — offline replacement for convertcase.net. Converts between camelCase, PascalCase, snake_case, kebab-case, SCREAMING_SNAKE_CASE, Title Case, dot.case, and more. Commands: camel, pascal, snake, kebab, screaming, title, upper, lower, dot, path, all. Bare input (no command) shows all at once. Examples: hematite --case 'hello world' | hematite --case 'snake getUserById' | hematite --case 'camel user_profile_data'"
    )]
    pub case: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "ELEMENT",
        help = "Look up a periodic table element — instant, no model, no cloud. Accepts symbol (H, Au), full name (Gold, Hydrogen), or atomic number (79). Shows atomic mass, category, period/group, electronegativity, and state at STP. Example: hematite --periodic Au"
    )]
    pub periodic: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "TARGET",
        help = "Compute MD5/SHA1/SHA256/SHA512 checksums of a file or text string. If TARGET is an existing file path, the file is hashed; otherwise the literal text is hashed. Pair with --hash-algo to select a single algorithm. Examples: hematite --hash installer.exe, hematite --hash \"hello world\""
    )]
    pub hash: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "ALGO",
        default_value = "all",
        help = "Hash algorithm for --hash: md5, sha1, sha256, sha512, or 'all' (default). Example: hematite --hash file.zip --hash-algo sha256"
    )]
    pub hash_algo: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "TEXT",
        help = "Encode text to a specified format. Pair with --codec (default: base64). Supported codecs: base64, hex, url, rot13, html, binary. Examples: hematite --encode \"hello world\", hematite --encode \"hello\" --codec hex"
    )]
    pub encode: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "TEXT",
        help = "Decode text from a specified format. Pair with --codec (default: base64). Supported codecs: base64, hex, url, rot13, html, binary. Examples: hematite --decode \"aGVsbG8gd29ybGQ=\", hematite --decode \"68656c6c6f\" --codec hex"
    )]
    pub decode: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "FORMAT",
        help = "Encoding format for --encode and --decode: base64 (default), hex, url, rot13, html, binary."
    )]
    pub codec: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "QUERY",
        help = "Search the built-in formula library — no model, no cloud. Pass a name, category, or keyword. Run --formula list to browse all entries. Examples: hematite --formula \"kinetic energy\", hematite --formula ohms, hematite --formula mechanics"
    )]
    pub formula: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "TYPE",
        help = "Generate cryptographically secure random values — no model, no cloud. Types: uuid  password  token  hex  urlsafe  pin  bytes  int  dice. Examples: hematite --random uuid, hematite --random password --length 24, hematite --random dice --random-args 2d6"
    )]
    pub random: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "N",
        help = "Length for --random password/token/pin/bytes generation. Default: 20 for passwords, 32 for tokens, 6 for PINs."
    )]
    pub length: Option<usize>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "ARGS",
        help = "Extra arguments for --random: dice notation (2d6, d20), int range (1 100), or custom charset for passwords."
    )]
    pub random_args: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "FILES",
        help = "Row-level diff of two data files — no model, no cloud. Pass comma-separated paths: file_a.csv,file_b.csv. Supports CSV, TSV, JSON, and SQLite. Pair with --diff-key to set the key column. Example: hematite --diff-data before.csv,after.csv"
    )]
    pub diff_data: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "COLUMN",
        help = "Key column for --diff-data row matching. Defaults to the first column. Example: --diff-key id"
    )]
    pub diff_key: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "FILE",
        help = "Descriptive statistics for numeric columns in a data file — no model, no cloud. Supports CSV, TSV, JSON, SQLite. Pair with --column to focus on one column. Example: hematite --describe sales.csv --column revenue"
    )]
    pub describe: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "NAME",
        help = "Column name to analyze with --stats. If omitted, all numeric columns are summarized."
    )]
    pub column: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "OP",
        help = "Matrix operation — no model, no cloud. OP: det  inv  transpose  multiply  solve  eigenvalues  rank  trace. Pass matrix as JSON: --matrix det --matrix-a '[[1,2],[3,4]]'. Example: hematite --matrix det --matrix-a '[[1,2],[3,4]]'"
    )]
    pub matrix: Option<String>,

    #[arg(
        long,
        value_name = "JSON",
        help = "Matrix A for --matrix, as a JSON array of rows: '[[1,2],[3,4]]'"
    )]
    pub matrix_a: Option<String>,

    #[arg(
        long,
        value_name = "JSON",
        help = "Matrix B for --matrix multiply or solve: '[[5],[6]]'"
    )]
    pub matrix_b: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "EQUATION",
        help = "Solve an equation numerically — no model, no cloud. Format: 'LHS = RHS' or expression = 0. Variable defaults to x. Supports sin/cos/sqrt/log/exp/pi/e. Example: hematite --solve 'x^2 - 4 = 0'  or  --solve '2*x + 3 = 11'"
    )]
    pub solve: Option<String>,

    #[arg(
        long,
        value_name = "VAR",
        help = "Variable name for --solve. Default: x. Example: --solve 't^2 = 16' --solve-var t"
    )]
    pub solve_var: Option<String>,

    #[arg(
        long,
        value_name = "LO,HI",
        help = "Search range for --solve as 'lo,hi'. Default: -1000,1000. Example: --solve-range '-100,100'"
    )]
    pub solve_range: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "FILE",
        help = "Fit a curve to two columns of data — no model, no cloud. Tries linear, polynomial, exponential, power, and log models and ranks by R². Pair with --fit-x, --fit-y, --fit-model. Example: hematite --curve-fit data.csv --fit-x time --fit-y temperature"
    )]
    pub curve_fit: Option<String>,

    #[arg(
        long,
        value_name = "COL",
        help = "X column for --curve-fit. Defaults to first numeric column."
    )]
    pub fit_x: Option<String>,

    #[arg(
        long,
        value_name = "COL",
        help = "Y column for --curve-fit. Defaults to second numeric column."
    )]
    pub fit_y: Option<String>,

    #[arg(
        long,
        value_name = "MODEL",
        help = "Model for --curve-fit: linear  poly2  poly3  exp  power  log  auto (default: auto, tries all)"
    )]
    pub fit_model: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "EXPR",
        help = "Numerically integrate an expression — no model, no cloud. Uses adaptive Simpson's rule. Pair with --from, --to, --int-var. Example: hematite --integrate 'sin(x)' --from 0 --to pi  or  --integrate 'x^2' --from 0 --to 3"
    )]
    pub integrate: Option<String>,

    #[arg(
        long,
        value_name = "N",
        help = "Lower bound for --integrate. Example: --int-from 0"
    )]
    pub int_from: Option<String>,

    #[arg(
        long,
        value_name = "N",
        help = "Upper bound for --integrate. Example: --int-to pi"
    )]
    pub int_to: Option<String>,

    #[arg(
        long,
        value_name = "VAR",
        help = "Integration variable for --integrate. Default: x."
    )]
    pub int_var: Option<String>,

    #[arg(
        long,
        value_name = "N",
        help = "Number of intervals for --integrate (default: 1000). Adaptive Simpson uses this as fallback."
    )]
    pub int_n: Option<usize>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "EXPR",
        help = "Numerically differentiate an expression — no model, no cloud. Uses 5-point stencil. Pair with --at and optionally --order. Example: hematite --differentiate 'x^3 + 2*x' --at 2  or  --differentiate 'sin(x)' --at 'pi/2'"
    )]
    pub differentiate: Option<String>,

    #[arg(
        long,
        value_name = "X",
        help = "Point at which to evaluate --differentiate or --solve. Example: --at 3.14"
    )]
    pub at: Option<String>,

    #[arg(
        long,
        value_name = "N",
        help = "Derivative order for --differentiate (1st, 2nd, 3rd, 4th). Default: 1."
    )]
    pub order: Option<u8>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "FILE",
        help = "AI-free data profile — type detection, missing values, ranges, outliers, and duplicate rows. No model, no cloud. Supports CSV, TSV, JSON, SQLite. Example: hematite --profile customers.csv"
    )]
    pub profile: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "N",
        help = "Prime number info — no model, no cloud. Primality test, factorization, divisors, Euler's φ, σ(n), nearest primes. Example: hematite --prime 97  or  --prime 360"
    )]
    pub prime: Option<u64>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "TYPE",
        help = "Generate a numeric sequence — no model, no cloud. Types: arithmetic  geometric  fibonacci  prime  square  triangular  cube  power2. Pair with --seq-count, --seq-start, --seq-step. Example: hematite --sequence fibonacci --seq-count 20"
    )]
    pub sequence: Option<String>,

    #[arg(
        long,
        value_name = "N",
        help = "Number of terms for --sequence (default: 10)."
    )]
    pub seq_count: Option<usize>,

    #[arg(
        long,
        value_name = "N",
        help = "Starting value for --sequence (default: 1)."
    )]
    pub seq_start: Option<f64>,

    #[arg(
        long,
        value_name = "N",
        help = "Step or ratio for --sequence (default: 1 for arithmetic, 2 for geometric)."
    )]
    pub seq_step: Option<f64>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "N K",
        help = "Combinations and permutations — no model, no cloud. Computes C(n,k) and P(n,k). Pass two integers separated by a space or comma. Example: hematite --choose '10 3'  or  --choose '52,5'"
    )]
    pub choose: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "EXPR",
        help = "Boolean truth table — no model, no cloud. Variables are single letters (A, B, C). Operators: AND OR NOT XOR NAND NOR (or ∧ ∨ ¬ ⊕). Example: hematite --truth-table '(A AND B) OR NOT C'"
    )]
    pub truth_table: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "A,B",
        help = "GCD and LCM of two integers — no model, no cloud. Example: hematite --gcd '48,18'  or  --gcd '360 252'"
    )]
    pub gcd: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "N or ROMAN",
        help = "Roman numeral conversion — no model, no cloud. Pass a number to encode or a Roman numeral to decode. Example: hematite --roman 2024  or  --roman MMXXIV"
    )]
    pub roman: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "N",
        help = "Number base conversion — no model, no cloud. Pair with --base-from and --base-to. Default: --base-from 10 --base-to 2. Example: hematite --base-convert 255 --base-to 16  or  --base-convert FF --base-from 16 --base-to 10"
    )]
    pub base_convert: Option<String>,

    #[arg(
        long,
        value_name = "N",
        help = "Source base for --base-convert (2–36). Default: 10."
    )]
    pub base_from: Option<u32>,

    #[arg(
        long,
        value_name = "N",
        help = "Target base for --base-convert (2–36). Default: 2."
    )]
    pub base_to: Option<u32>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "EXPR",
        help = "Date arithmetic and calendar info — no model, no cloud. Examples: hematite --date '2024-01-01 to 2024-12-31'  --date '2024-03-15 +90'  --date '2024-06-15'  --date 'unix 1700000000'"
    )]
    pub date: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "CIDR",
        help = "IPv4 subnet calculator — no model, no cloud. Pass a CIDR address. Example: hematite --subnet 192.168.1.0/24  or  --subnet 10.0.0.1/8"
    )]
    pub subnet: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "COLOR",
        help = "Color space conversion — no model, no cloud. Converts hex/RGB to HSL, HSV, CMYK, and WCAG luminance. Example: hematite --color '#ff8800'  or  --color 'rgb(255,136,0)'  or  --color '3f8'"
    )]
    pub color: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "FORMULA",
        help = "Molecular weight from a chemical formula — no model, no cloud. Supports nested groups: Ca(NO3)2, (NH4)2SO4. Example: hematite --mw H2O  or  --mw 'C6H12O6'"
    )]
    pub mw: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "NAME",
        help = "Physical constants lookup — no model, no cloud. Use 'list' to see all. Example: hematite --const c  --const planck  --const avogadro  --const list"
    )]
    pub r#const: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "QUERY",
        help = "Standard normal distribution — no model, no cloud. Modes: 'cdf X [mu sigma]'  'pdf X'  'inv P'  'between A B'  'table'. Example: hematite --normal 'cdf 1.96'  --normal 'inv 0.975'  --normal table"
    )]
    pub normal: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "EXPR",
        help = "2D/3D vector math — instant, no model. Ops: dot, cross, +, -, scalar*, mag, norm, angle, proj. Example: hematite --vectors '[1,2,3] dot [4,5,6]'  'mag [3,4]'  '[1,2,3] cross [0,0,1]'"
    )]
    pub vectors: Option<String>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "QUERY",
        help = "Number theory — instant, no model. Ops: extgcd, crt (Chinese Remainder Theorem), mobius, modinv, modpow, cf (continued fractions), goldbach, totient, jacobi. Example: hematite --number-theory 'modpow 3 10 1000'  'crt 2 3 3 5'  'goldbach 28'  'cf 355/113'  '42'"
    )]
    pub number_theory: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Percentile/quantile report for all numeric columns (or --percentile-col COL for a specific column). Example: hematite --percentile data.csv --percentile-col salary"
    )]
    pub percentile: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL",
        help = "Column to analyze with --percentile (default: all numeric columns)."
    )]
    pub percentile_col: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Pivot table — group rows by two columns and aggregate a value column. Use --pivot-row, --pivot-col, --pivot-val, --pivot-agg (count/sum/mean/min/max). Example: hematite --pivot sales.csv --pivot-row region --pivot-col quarter --pivot-val revenue --pivot-agg sum"
    )]
    pub pivot: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL",
        help = "Row grouping column for --pivot."
    )]
    pub pivot_row: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL",
        help = "Column grouping column for --pivot."
    )]
    pub pivot_col: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL",
        help = "Value column for --pivot aggregation."
    )]
    pub pivot_val: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "AGG",
        help = "Aggregation for --pivot: count (default), sum, mean, min, max."
    )]
    pub pivot_agg: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Multivariate OLS linear regression from a CSV/TSV/JSON/SQLite file. Use --regression-target to specify the dependent variable and --regression-predictors for a comma-separated list of independent variables. Example: hematite --regression data.csv --regression-target price --regression-predictors sqft,bedrooms,bathrooms"
    )]
    pub regression: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL",
        help = "Target (dependent) column for --regression (auto-detected if omitted)."
    )]
    pub regression_target: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL1,COL2,...",
        help = "Predictor (independent) columns for --regression, comma-separated (auto-detected if omitted)."
    )]
    pub regression_predictors: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Detect outliers using IQR (1.5× fence) and Z-score (|z|>3) in all numeric columns or a specific column. Use --outlier-col COL and --outlier-output FILE to save clean data. Example: hematite --outliers data.csv --outlier-col salary --outlier-output clean.csv"
    )]
    pub outliers: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL",
        help = "Column to analyze for --outliers (default: all numeric columns)."
    )]
    pub outlier_col: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Save clean data (outliers removed) to this CSV path."
    )]
    pub outlier_output: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Random-sample rows from a CSV/TSV/JSON/SQLite file. Use --sample-n or --sample-frac for size; --split for train/test; --sample-output DIR to save files. Example: hematite --sample data.csv --sample-n 200 --split 0.8 --sample-output out/"
    )]
    pub sample: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "N",
        help = "Number of rows to sample (default 100)."
    )]
    pub sample_n: Option<usize>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FRAC",
        help = "Fraction of rows to sample, e.g. 0.1 for 10%."
    )]
    pub sample_frac: Option<f64>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "SEED",
        help = "Random seed for reproducible sampling (default 42)."
    )]
    pub sample_seed: Option<u64>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FRAC",
        help = "Train/test split fraction, e.g. 0.8 saves 80% to train and 20% to test. Requires --sample-output."
    )]
    pub split: Option<f64>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "DIR",
        help = "Output directory for sampled files. If omitted, prints sample to stdout."
    )]
    pub sample_output: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Compute correlation matrix for all numeric columns in a file. Use --corr-method pearson|spearman. Example: hematite --correlation data.csv --corr-method spearman"
    )]
    pub correlation: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "METHOD",
        help = "Correlation method: pearson (default) or spearman."
    )]
    pub corr_method: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Time-series analysis: rolling mean, trend, peaks/valleys, sparkline. Example: hematite --timeseries sales.csv --ts-date date --ts-value revenue --ts-window 7"
    )]
    pub timeseries: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL",
        help = "Date column name for --timeseries (auto-detected if omitted)."
    )]
    pub ts_date: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "COL",
        help = "Value column name for --timeseries (auto-detected if omitted)."
    )]
    pub ts_value: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "N",
        help = "Rolling window size for --timeseries (default 7)."
    )]
    pub ts_window: Option<usize>,

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
