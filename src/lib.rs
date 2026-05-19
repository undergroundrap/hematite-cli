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
        help = "Convert between units or number bases — no model, no cloud. Supports length, mass, time, speed, energy, power, pressure, temperature, volume, area, digital storage, force, frequency, angle, and hex/binary/octal. Examples: hematite --convert \"100 km to miles\", hematite --convert \"72 F to C\", hematite --convert \"0xFF to decimal\""
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
