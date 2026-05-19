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

    #[arg(long, help_heading = "Headless Reports", value_name = "TEXT", help = "Chart title for --plot (auto-generated if omitted).")]
    pub plot_title: Option<String>,

    #[arg(long, help_heading = "Headless Reports", value_name = "FILE", help = "Output SVG file path for --plot (default: <input>_plot.svg).")]
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

    #[arg(long, help_heading = "Data Analysis", value_name = "COL", help = "Column to analyze with --fourier (auto-detected if omitted).")]
    pub fourier_col: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "N", help = "Number of top frequency components to report for --fourier (default 10).")]
    pub fourier_top: Option<usize>,

    #[arg(long, help_heading = "Data Analysis", value_name = "HZ", help = "Sample rate in Hz for --fourier (default 1.0 — reports normalized frequencies).")]
    pub fourier_rate: Option<f64>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "k-Means clustering on numeric columns. Use --cluster-k N (default 3), --cluster-cols COL1,COL2,..., --cluster-output FILE. Example: hematite --cluster data.csv --cluster-k 4 --cluster-cols height,weight --cluster-output labeled.csv"
    )]
    pub cluster: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "N", help = "Number of clusters for --cluster (default 3).")]
    pub cluster_k: Option<usize>,

    #[arg(long, help_heading = "Data Analysis", value_name = "COL1,COL2,...", help = "Feature columns for --cluster, comma-separated (default: all numeric).")]
    pub cluster_cols: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "FILE", help = "Output CSV with cluster labels appended for --cluster.")]
    pub cluster_output: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Normalize/standardize numeric columns. Use --normalize-method minmax|zscore|robust, --normalize-cols COL1,COL2,..., --normalize-output FILE. Example: hematite --normalize data.csv --normalize-method zscore --normalize-output scaled.csv"
    )]
    pub normalize: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "METHOD", help = "Normalization method: minmax (default), zscore, robust.")]
    pub normalize_method: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "COL1,COL2,...", help = "Columns to normalize for --normalize, comma-separated (default: all numeric).")]
    pub normalize_cols: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "FILE", help = "Output CSV with normalized values for --normalize.")]
    pub normalize_output: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "FILE", help = "Run PCA on a CSV/TSV file. Reports eigenvalues, variance explained per component, and top loadings. Example: hematite --pca data.csv --pca-components 3")]
    pub pca: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "N", help = "Number of principal components to compute for --pca (default: 3).")]
    pub pca_components: Option<usize>,

    #[arg(long, help_heading = "Data Analysis", value_name = "COL1,COL2,...", help = "Columns to include in --pca, comma-separated (default: all numeric).")]
    pub pca_cols: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "FILE", help = "Output CSV with projected coordinates for --pca.")]
    pub pca_output: Option<String>,

    #[arg(long, help_heading = "Math & Science", value_name = "QUERY", help = "Graph theory — parse an edge list and run BFS/DFS/Dijkstra/components/topo-sort. Example: hematite --graph 'shortest A D\\nA B 2\\nB D 3'")]
    pub graph: Option<String>,

    #[arg(long, help_heading = "Math & Science", value_name = "QUERY", help = "Symbolic calculus — differentiate, integrate, simplify, or evaluate. Example: hematite --symbolic 'diff x^3 + sin(x)'  or  'integrate 3*x^2'  or  'x^2+1 at x=5'")]
    pub symbolic: Option<String>,

    #[arg(long, help_heading = "Math & Science", value_name = "QUERY", help = "Financial math — NPV, IRR, loan amortization, compound interest, bond pricing, Black-Scholes. Example: hematite --finance 'loan 200000 6.5% 30'  or  'bs 100 100 5% 20% 1 call'")]
    pub finance: Option<String>,

    #[arg(long, help_heading = "Math & Science", value_name = "QUERY", help = "Propositional logic — truth table, SAT, tautology, CNF/DNF, equivalence, simplify. Example: hematite --logic 'A and (B or not C)'  or  'equiv A->B ; not A or B'")]
    pub logic: Option<String>,

    #[arg(long, help_heading = "Math & Science", value_name = "QUERY", help = "Signal processing (DSP) — DFT, convolution, cross-correlation, moving average, FIR filter design, waveform generation. No model, no cloud. Example: hematite --signal 'dft 1,0,-1,0'  or  'lowpass 0.1 31 hamming'  or  'gen sine 2 64'")]
    pub signal: Option<String>,

    #[arg(long, help_heading = "Math & Science", value_name = "QUERY", help = "Interpolation & curve fitting — linear, cubic spline, Lagrange polynomial, nearest-neighbor, with ASCII curve preview. Example: hematite --interpolate 'spline 0,0 1,1 2,4 3,9 at 1.5'  or  'linear 0,0 10,100 at 3,7'")]
    pub interpolate: Option<String>,

    #[arg(long, help_heading = "Math & Science", value_name = "QUERY", help = "Unit conversion — 14 categories, 130+ units. Length, mass, temperature, energy, digital storage, pressure, angle, and more. Example: hematite --units '100 km to miles'  or  '98.6 f to c'  or  '5 kg'  or  'list length'")]
    pub units: Option<String>,

    #[arg(long, help_heading = "Math & Science", value_name = "QUERY", help = "ODE solver — solves ordinary differential equations using Euler, RK4, or adaptive RK45. Preset models: logistic, exponential, Lotka-Volterra, SIR. Example: hematite --ode 'dy/dt = -y  y0=1  t=5'  or  'logistic r=1 K=100 y0=5 t=10'")]
    pub ode: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "DATA", help = "Statistical hypothesis test — t-tests, chi-square, ANOVA, Mann-Whitney, Pearson, proportion z-test, confidence intervals. Provide comma-separated numbers or 'successes,n' for proportions. Example: hematite --hypothesis '2.1,2.8,3.2,2.5' --hypothesis-test one-t --hypothesis-mu 2.0")]
    pub hypothesis: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "TYPE", help = "Test type for --hypothesis. Options: one-t two-t paired chi2 anova mannwhitney pearson proportion prop2 ci. Default: one-t.")]
    pub hypothesis_test: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "DATA", help = "Second group data for --hypothesis (two-t, paired, mannwhitney, pearson, prop2). Comma-separated numbers or 'successes,n'.")]
    pub hypothesis_group2: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "ALPHA", help = "Significance level for --hypothesis (default: 0.05).")]
    pub hypothesis_alpha: Option<f64>,

    #[arg(long, help_heading = "Data Analysis", value_name = "MU", help = "Null hypothesis mean or proportion for --hypothesis one-t or proportion tests (default: 0.0).")]
    pub hypothesis_mu: Option<f64>,

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

    #[arg(long, value_name = "COL", help = "X column for --curve-fit. Defaults to first numeric column.")]
    pub fit_x: Option<String>,

    #[arg(long, value_name = "COL", help = "Y column for --curve-fit. Defaults to second numeric column.")]
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

    #[arg(long, value_name = "N", help = "Lower bound for --integrate. Example: --int-from 0")]
    pub int_from: Option<String>,

    #[arg(long, value_name = "N", help = "Upper bound for --integrate. Example: --int-to pi")]
    pub int_to: Option<String>,

    #[arg(long, value_name = "VAR", help = "Integration variable for --integrate. Default: x.")]
    pub int_var: Option<String>,

    #[arg(long, value_name = "N", help = "Number of intervals for --integrate (default: 1000). Adaptive Simpson uses this as fallback.")]
    pub int_n: Option<usize>,

    #[arg(
        long,
        help_heading = "Headless Reports",
        value_name = "EXPR",
        help = "Numerically differentiate an expression — no model, no cloud. Uses 5-point stencil. Pair with --at and optionally --order. Example: hematite --differentiate 'x^3 + 2*x' --at 2  or  --differentiate 'sin(x)' --at 'pi/2'"
    )]
    pub differentiate: Option<String>,

    #[arg(long, value_name = "X", help = "Point at which to evaluate --differentiate or --solve. Example: --at 3.14")]
    pub at: Option<String>,

    #[arg(long, value_name = "N", help = "Derivative order for --differentiate (1st, 2nd, 3rd, 4th). Default: 1.")]
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

    #[arg(long, value_name = "N", help = "Number of terms for --sequence (default: 10).")]
    pub seq_count: Option<usize>,

    #[arg(long, value_name = "N", help = "Starting value for --sequence (default: 1).")]
    pub seq_start: Option<f64>,

    #[arg(long, value_name = "N", help = "Step or ratio for --sequence (default: 1 for arithmetic, 2 for geometric).")]
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

    #[arg(long, value_name = "N", help = "Source base for --base-convert (2–36). Default: 10.")]
    pub base_from: Option<u32>,

    #[arg(long, value_name = "N", help = "Target base for --base-convert (2–36). Default: 2.")]
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

    #[arg(long, help_heading = "Data Analysis", value_name = "COL", help = "Column to analyze with --percentile (default: all numeric columns).")]
    pub percentile_col: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Pivot table — group rows by two columns and aggregate a value column. Use --pivot-row, --pivot-col, --pivot-val, --pivot-agg (count/sum/mean/min/max). Example: hematite --pivot sales.csv --pivot-row region --pivot-col quarter --pivot-val revenue --pivot-agg sum"
    )]
    pub pivot: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "COL", help = "Row grouping column for --pivot.")]
    pub pivot_row: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "COL", help = "Column grouping column for --pivot.")]
    pub pivot_col: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "COL", help = "Value column for --pivot aggregation.")]
    pub pivot_val: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "AGG", help = "Aggregation for --pivot: count (default), sum, mean, min, max.")]
    pub pivot_agg: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Multivariate OLS linear regression from a CSV/TSV/JSON/SQLite file. Use --regression-target to specify the dependent variable and --regression-predictors for a comma-separated list of independent variables. Example: hematite --regression data.csv --regression-target price --regression-predictors sqft,bedrooms,bathrooms"
    )]
    pub regression: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "COL", help = "Target (dependent) column for --regression (auto-detected if omitted).")]
    pub regression_target: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "COL1,COL2,...", help = "Predictor (independent) columns for --regression, comma-separated (auto-detected if omitted).")]
    pub regression_predictors: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Detect outliers using IQR (1.5× fence) and Z-score (|z|>3) in all numeric columns or a specific column. Use --outlier-col COL and --outlier-output FILE to save clean data. Example: hematite --outliers data.csv --outlier-col salary --outlier-output clean.csv"
    )]
    pub outliers: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "COL", help = "Column to analyze for --outliers (default: all numeric columns).")]
    pub outlier_col: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "FILE", help = "Save clean data (outliers removed) to this CSV path.")]
    pub outlier_output: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Random-sample rows from a CSV/TSV/JSON/SQLite file. Use --sample-n or --sample-frac for size; --split for train/test; --sample-output DIR to save files. Example: hematite --sample data.csv --sample-n 200 --split 0.8 --sample-output out/"
    )]
    pub sample: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "N", help = "Number of rows to sample (default 100).")]
    pub sample_n: Option<usize>,

    #[arg(long, help_heading = "Data Analysis", value_name = "FRAC", help = "Fraction of rows to sample, e.g. 0.1 for 10%.")]
    pub sample_frac: Option<f64>,

    #[arg(long, help_heading = "Data Analysis", value_name = "SEED", help = "Random seed for reproducible sampling (default 42).")]
    pub sample_seed: Option<u64>,

    #[arg(long, help_heading = "Data Analysis", value_name = "FRAC", help = "Train/test split fraction, e.g. 0.8 saves 80% to train and 20% to test. Requires --sample-output.")]
    pub split: Option<f64>,

    #[arg(long, help_heading = "Data Analysis", value_name = "DIR", help = "Output directory for sampled files. If omitted, prints sample to stdout.")]
    pub sample_output: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Compute correlation matrix for all numeric columns in a file. Use --corr-method pearson|spearman. Example: hematite --correlation data.csv --corr-method spearman"
    )]
    pub correlation: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "METHOD", help = "Correlation method: pearson (default) or spearman.")]
    pub corr_method: Option<String>,

    #[arg(
        long,
        help_heading = "Data Analysis",
        value_name = "FILE",
        help = "Time-series analysis: rolling mean, trend, peaks/valleys, sparkline. Example: hematite --timeseries sales.csv --ts-date date --ts-value revenue --ts-window 7"
    )]
    pub timeseries: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "COL", help = "Date column name for --timeseries (auto-detected if omitted).")]
    pub ts_date: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "COL", help = "Value column name for --timeseries (auto-detected if omitted).")]
    pub ts_value: Option<String>,

    #[arg(long, help_heading = "Data Analysis", value_name = "N", help = "Rolling window size for --timeseries (default 7).")]
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
